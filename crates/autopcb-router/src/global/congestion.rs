//! Coarse congestion grid: per-cell capacity estimation and demand tracking.
//!
//! # Capacity estimation
//!
//! Each cell's routing capacity is:
//! ```text
//! capacity = (cell_size_mm - total_obstacle_width_in_cell) / (trace_width_mm + clearance_mm)
//! ```
//! where `total_obstacle_width_in_cell` is derived from the fraction of the
//! cell that is blocked in the fine-grid obstacle bitmap.
//!
//! # Congestion ratio
//!
//! `congestion_ratio = demand / capacity`.  Cells with ratio > 1.0 are
//! oversubscribed and will drive PathFinder rip-up decisions.
//!
//! # Cell size
//!
//! Cell size is `cell_size_multiplier × (trace_width + clearance)`, clamped
//! to at least one fine-grid cell.  Typical multiplier: 5–10.

use crate::workspace::{GridConfig, RoutingWorkspace};

use super::steiner::CellId;

// ---------------------------------------------------------------------------
// CongestionCell
// ---------------------------------------------------------------------------

/// Per-cell congestion state.
#[derive(Debug, Clone, Copy)]
pub struct CongestionCell {
    /// Maximum number of traces that fit through this cell.
    pub capacity: f64,
    /// Current routing demand routed through this cell.
    pub demand: f64,
}

impl CongestionCell {
    /// Congestion ratio (demand / capacity).  Returns `f64::INFINITY` when
    /// capacity is zero.
    pub fn congestion_ratio(&self) -> f64 {
        if self.capacity <= 0.0 {
            return f64::INFINITY;
        }
        self.demand / self.capacity
    }

    /// Returns `true` when demand exceeds capacity.
    pub fn is_congested(&self) -> bool {
        self.demand > self.capacity
    }
}

// ---------------------------------------------------------------------------
// GlobalRoutingGrid
// ---------------------------------------------------------------------------

/// Coarse grid for global routing congestion estimation.
///
/// Each cell covers a square region of size `cell_size_mm × cell_size_mm` in
/// world coordinates.  The grid origin and dimensions are derived from the
/// fine routing grid.
#[derive(Debug, Clone)]
pub struct GlobalRoutingGrid {
    /// Flat cell array in row-major order: `index = row * cols + col`.
    pub cells: Vec<CongestionCell>,
    /// Number of rows (y direction).
    pub rows: u32,
    /// Number of columns (x direction).
    pub cols: u32,
    /// Cell size in mm.
    pub cell_size_mm: f64,
}

impl GlobalRoutingGrid {
    /// Build a congestion grid from a `RoutingWorkspace`.
    ///
    /// `cell_size_multiplier` scales the fine grid resolution to the coarse
    /// cell size.  Typical values: 5–10.  The capacity of each cell is
    /// estimated from the fine-grid obstacle density: blocked fine cells reduce
    /// available routing capacity.
    pub fn from_workspace(workspace: &RoutingWorkspace, cell_size_multiplier: u32) -> Self {
        let fine = &workspace.grid;
        let multiplier = cell_size_multiplier.max(1) as f64;

        // Coarse cell size in mm.
        let cell_size_mm = fine.resolution_mm * multiplier;

        // Coarse grid dimensions: ceiling division.
        let cols = (fine.width_cells as f64 / multiplier).ceil() as u32;
        let rows = (fine.height_cells as f64 / multiplier).ceil() as u32;

        let total = (rows as usize).saturating_mul(cols as usize);
        let mut cells = Vec::with_capacity(total);

        // Estimate capacity for each coarse cell from obstacle density on all
        // layers.  For each coarse cell we count blocked fine cells across all
        // layers and compute average obstacle fraction.
        let layer_count = workspace.obstacle_maps.len().max(1);
        let fine_per_coarse = (multiplier as u32).pow(2); // fine cells per coarse cell

        for cr in 0..rows {
            for cc in 0..cols {
                // Fine-grid range covered by this coarse cell.
                let fine_col_start = cc * cell_size_multiplier;
                let fine_row_start = cr * cell_size_multiplier;
                let fine_col_end =
                    (fine_col_start + cell_size_multiplier).min(fine.width_cells);
                let fine_row_end =
                    (fine_row_start + cell_size_multiplier).min(fine.height_cells);

                // Count blocked fine cells across all layers.
                let mut blocked_count: u64 = 0;
                let mut total_fine: u64 = 0;
                for map in &workspace.obstacle_maps {
                    for gy in fine_row_start..fine_row_end {
                        for gx in fine_col_start..fine_col_end {
                            total_fine += 1;
                            if map.is_blocked(gx, gy) {
                                blocked_count += 1;
                            }
                        }
                    }
                }

                // Average obstacle fraction across all layers.
                let obstacle_fraction = if total_fine > 0 {
                    blocked_count as f64 / (total_fine as f64 * layer_count as f64)
                } else {
                    0.0
                };

                // Capacity: (cell_size - obstacle_width) / (trace + clearance).
                // We use global policy defaults for trace_width and clearance.
                // Obtain from the policy via a sentinel net pair.
                let clearance_mm = workspace
                    .policy
                    .clearance(autopcb_routes::NetId(u32::MAX), autopcb_routes::NetId(u32::MAX));
                let trace_width_mm = workspace
                    .policy
                    .trace_width(
                        autopcb_routes::NetId(u32::MAX),
                        autopcb_routes::LayerId(0),
                    )
                    .preferred;
                let pitch = trace_width_mm + clearance_mm;
                let available_mm = cell_size_mm * (1.0 - obstacle_fraction);
                let capacity = if pitch > 0.0 {
                    (available_mm / pitch).max(0.0) * layer_count as f64
                } else {
                    0.0
                };

                let _ = fine_per_coarse; // used conceptually via fine_col/row ranges
                cells.push(CongestionCell {
                    capacity,
                    demand: 0.0,
                });
            }
        }

        GlobalRoutingGrid {
            cells,
            rows,
            cols,
            cell_size_mm,
        }
    }

    /// Returns the cell index for coarse grid coordinates `(col, row)`.
    /// Returns `None` if out of bounds.
    pub fn cell_index(&self, col: u32, row: u32) -> Option<usize> {
        if col < self.cols && row < self.rows {
            Some(row as usize * self.cols as usize + col as usize)
        } else {
            None
        }
    }

    /// Returns the `CellId` for the coarse cell that contains the fine-grid
    /// cell `(fine_gx, fine_gy)` given the fine grid configuration.
    pub fn cell_id_for_fine(&self, fine_gx: u32, fine_gy: u32, fine: &GridConfig) -> CellId {
        let multiplier = (self.cell_size_mm / fine.resolution_mm).round() as u32;
        let col = fine_gx / multiplier;
        let row = fine_gy / multiplier;
        let col = col.min(self.cols.saturating_sub(1));
        let row = row.min(self.rows.saturating_sub(1));
        CellId(row * self.cols + col)
    }

    /// Increment the demand of `cell_id` by `amount`.
    ///
    /// Silently ignores out-of-range `CellId` values.
    pub fn add_demand(&mut self, cell_id: CellId, amount: f64) {
        let idx = cell_id.0 as usize;
        if idx < self.cells.len() {
            self.cells[idx].demand += amount;
        }
    }

    /// Returns `true` if the cell's demand exceeds its capacity.
    ///
    /// Out-of-range cells return `false` (safe default).
    pub fn is_congested(&self, cell_id: CellId) -> bool {
        let idx = cell_id.0 as usize;
        self.cells.get(idx).map_or(false, |c| c.is_congested())
    }

    /// Clear all cell demands to zero for the next routing iteration.
    pub fn reset_demand(&mut self) {
        for cell in &mut self.cells {
            cell.demand = 0.0;
        }
    }

    /// Returns the congestion ratio for `cell_id`.
    ///
    /// Returns `0.0` for out-of-range cells (safe default).
    pub fn congestion_ratio(&self, cell_id: CellId) -> f64 {
        let idx = cell_id.0 as usize;
        self.cells
            .get(idx)
            .map_or(0.0, |c| c.congestion_ratio())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal `GlobalRoutingGrid` directly for unit tests, bypassing
    /// workspace construction.
    fn make_grid(rows: u32, cols: u32, capacity: f64) -> GlobalRoutingGrid {
        let total = (rows as usize) * (cols as usize);
        GlobalRoutingGrid {
            cells: vec![
                CongestionCell {
                    capacity,
                    demand: 0.0,
                };
                total
            ],
            rows,
            cols,
            cell_size_mm: 1.0,
        }
    }

    #[test]
    fn empty_grid_all_cells_zero_demand_positive_capacity() {
        let grid = make_grid(4, 4, 5.0);
        for row in 0..4 {
            for col in 0..4 {
                let idx = row * 4 + col;
                let cell = &grid.cells[idx];
                assert!(
                    (cell.demand - 0.0).abs() < f64::EPSILON,
                    "demand should be 0 at ({col},{row})"
                );
                assert!(
                    cell.capacity > 0.0,
                    "capacity should be positive at ({col},{row})"
                );
            }
        }
    }

    #[test]
    fn add_demand_increments_cell() {
        let mut grid = make_grid(3, 3, 10.0);
        let cell = CellId(4); // center of 3×3 grid
        grid.add_demand(cell, 2.5);
        assert!(
            (grid.cells[4].demand - 2.5).abs() < f64::EPSILON,
            "demand should be 2.5"
        );
        grid.add_demand(cell, 1.0);
        assert!(
            (grid.cells[4].demand - 3.5).abs() < f64::EPSILON,
            "demand should be 3.5 after second add"
        );
    }

    #[test]
    fn add_demand_out_of_range_is_no_op() {
        let mut grid = make_grid(2, 2, 5.0);
        // Cell index 100 is out of range — should not panic.
        grid.add_demand(CellId(100), 99.0);
        // All in-bounds cells remain unchanged.
        for cell in &grid.cells {
            assert!((cell.demand - 0.0).abs() < f64::EPSILON);
        }
    }

    #[test]
    fn is_congested_false_when_demand_below_capacity() {
        let mut grid = make_grid(2, 2, 5.0);
        let cell = CellId(0);
        grid.add_demand(cell, 4.9);
        assert!(!grid.is_congested(cell), "demand < capacity → not congested");
    }

    #[test]
    fn is_congested_true_when_demand_exceeds_capacity() {
        let mut grid = make_grid(2, 2, 5.0);
        let cell = CellId(0);
        grid.add_demand(cell, 5.1);
        assert!(grid.is_congested(cell), "demand > capacity → congested");
    }

    #[test]
    fn is_congested_out_of_range_returns_false() {
        let grid = make_grid(2, 2, 5.0);
        assert!(!grid.is_congested(CellId(999)));
    }

    #[test]
    fn reset_demand_clears_all_cells() {
        let mut grid = make_grid(3, 3, 10.0);
        // Add demand to all cells.
        for i in 0..(3 * 3) {
            grid.add_demand(CellId(i as u32), 7.0);
        }
        // Verify demand was added.
        for cell in &grid.cells {
            assert!((cell.demand - 7.0).abs() < f64::EPSILON);
        }
        grid.reset_demand();
        for cell in &grid.cells {
            assert!(
                (cell.demand - 0.0).abs() < f64::EPSILON,
                "demand should be 0 after reset"
            );
        }
    }

    #[test]
    fn congestion_ratio_zero_when_no_demand() {
        let grid = make_grid(2, 2, 4.0);
        let ratio = grid.congestion_ratio(CellId(0));
        assert!((ratio - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn congestion_ratio_infinity_when_capacity_zero() {
        let grid = GlobalRoutingGrid {
            cells: vec![CongestionCell {
                capacity: 0.0,
                demand: 1.0,
            }],
            rows: 1,
            cols: 1,
            cell_size_mm: 1.0,
        };
        assert!(
            grid.congestion_ratio(CellId(0)).is_infinite(),
            "zero-capacity cell should have infinite congestion ratio"
        );
    }
}
