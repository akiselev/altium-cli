//! Per-layer obstacle bitmaps and occupancy queries.
//!
//! Uses `bitvec`-backed per-layer grids for O(1) blocked-cell lookups during
//! A* traversal.
//!
//! # Grid layout
//!
//! Cells are linearized in row-major order: `index = gy * width + gx`.
//! All grid coordinates are `(u32, u32)` with `gx` in `[0, width)` and
//! `gy` in `[0, height)`.
//!
//! # Access points
//!
//! An `AccessPoint` is a routable grid cell adjacent to a pad.  It carries
//! its own `LayerId` so that through-hole pads can expose access points on
//! multiple layers.

use bitvec::prelude::{BitVec, Lsb0};

use autopcb_routes::LayerId;

// ---------------------------------------------------------------------------
// AccessPoint
// ---------------------------------------------------------------------------

/// A routable grid cell adjacent to a pad.
///
/// `layer` uses `autopcb_routes::LayerId` because access points are
/// consumed by the detailed router which works in route-space layer IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccessPoint {
    /// Grid column (x).
    pub gx: u32,
    /// Grid row (y).
    pub gy: u32,
    /// Layer the access point is on.
    pub layer: LayerId,
}

// ---------------------------------------------------------------------------
// ObstacleMap
// ---------------------------------------------------------------------------

/// Single-layer obstacle bitmap.
///
/// Wraps a flat `BitVec` grid of `width × height` cells. Each cell is either
/// blocked (`true`) or free (`false`). Out-of-bounds queries always return
/// `false` (unblocked) to avoid panics in border-cell pathfinding code.
#[derive(Debug, Clone)]
pub struct ObstacleMap {
    /// Flat bit array, row-major order: `index = gy * width + gx`.
    blocked: BitVec<usize, Lsb0>,
    /// Number of columns (x extent of the grid).
    pub width: u32,
    /// Number of rows (y extent of the grid).
    pub height: u32,
}

impl ObstacleMap {
    /// Create an empty (all-unblocked) obstacle map of the given dimensions.
    pub fn new(width: u32, height: u32) -> Self {
        let cells = (width as usize).saturating_mul(height as usize);
        ObstacleMap {
            blocked: BitVec::repeat(false, cells),
            width,
            height,
        }
    }

    /// Returns `true` if the cell at `(gx, gy)` is blocked.
    ///
    /// Out-of-bounds coordinates return `false` (safe default).
    pub fn is_blocked(&self, gx: u32, gy: u32) -> bool {
        if gx >= self.width || gy >= self.height {
            return false;
        }
        let idx = gy as usize * self.width as usize + gx as usize;
        self.blocked[idx]
    }

    /// Set or clear the blocked state of cell `(gx, gy)`.
    ///
    /// Out-of-bounds coordinates are silently ignored.
    pub fn set_blocked(&mut self, gx: u32, gy: u32, blocked: bool) {
        if gx >= self.width || gy >= self.height {
            return;
        }
        let idx = gy as usize * self.width as usize + gx as usize;
        self.blocked.set(idx, blocked);
    }

    /// Block all cells in the rectangle `[min_gx, max_gx] × [min_gy, max_gy]`
    /// (inclusive on all sides). Coordinates are clamped to grid bounds.
    pub fn mark_rect_blocked(&mut self, min_gx: u32, min_gy: u32, max_gx: u32, max_gy: u32) {
        let min_gx = min_gx.min(self.width.saturating_sub(1));
        let min_gy = min_gy.min(self.height.saturating_sub(1));
        let max_gx = max_gx.min(self.width.saturating_sub(1));
        let max_gy = max_gy.min(self.height.saturating_sub(1));

        for gy in min_gy..=max_gy {
            for gx in min_gx..=max_gx {
                let idx = gy as usize * self.width as usize + gx as usize;
                self.blocked.set(idx, true);
            }
        }
    }

    /// Block all cells within `radius_cells` (inclusive, Euclidean) of the
    /// center cell `(cx, cy)`.  Used for circular pad footprints.
    ///
    /// All coordinates are clamped to grid bounds.
    pub fn mark_circle_blocked(&mut self, cx: u32, cy: u32, radius_cells: u32) {
        let r = radius_cells as i64;
        // Bounding box of the circle.
        let min_gx = (cx as i64 - r).max(0) as u32;
        let min_gy = (cy as i64 - r).max(0) as u32;
        let max_gx = ((cx as i64 + r) as u32).min(self.width.saturating_sub(1));
        let max_gy = ((cy as i64 + r) as u32).min(self.height.saturating_sub(1));

        let r2 = (radius_cells as i64) * (radius_cells as i64);
        for gy in min_gy..=max_gy {
            for gx in min_gx..=max_gx {
                let dx = gx as i64 - cx as i64;
                let dy = gy as i64 - cy as i64;
                if dx * dx + dy * dy <= r2 {
                    let idx = gy as usize * self.width as usize + gx as usize;
                    self.blocked.set(idx, true);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_map_all_unblocked() {
        let map = ObstacleMap::new(10, 8);
        for gy in 0..8 {
            for gx in 0..10 {
                assert!(
                    !map.is_blocked(gx, gy),
                    "expected unblocked at ({gx}, {gy})"
                );
            }
        }
    }

    #[test]
    fn set_blocked_roundtrip() {
        let mut map = ObstacleMap::new(5, 5);
        assert!(!map.is_blocked(2, 3));
        map.set_blocked(2, 3, true);
        assert!(map.is_blocked(2, 3));
        map.set_blocked(2, 3, false);
        assert!(!map.is_blocked(2, 3));
    }

    #[test]
    fn mark_rect_blocked_correct_region() {
        let mut map = ObstacleMap::new(10, 10);
        map.mark_rect_blocked(2, 3, 5, 6);

        for gy in 0u32..10 {
            for gx in 0u32..10 {
                let expected = gx >= 2 && gx <= 5 && gy >= 3 && gy <= 6;
                assert_eq!(
                    map.is_blocked(gx, gy),
                    expected,
                    "mismatch at ({gx}, {gy})"
                );
            }
        }
    }

    #[test]
    fn mark_circle_blocked_approximate() {
        let mut map = ObstacleMap::new(20, 20);
        // Circle of radius 3 centered at (10, 10).
        map.mark_circle_blocked(10, 10, 3);

        // Center must be blocked.
        assert!(map.is_blocked(10, 10), "center must be blocked");

        // Corners of the bounding box (radius 3) should NOT be blocked because
        // they are farther than radius from center (distance = 3√2 ≈ 4.24 > 3).
        assert!(
            !map.is_blocked(7, 7),
            "corner (7,7) should not be blocked by r=3 circle at (10,10)"
        );

        // A cell at exactly radius distance (e.g. (10, 13)) should be blocked.
        assert!(map.is_blocked(10, 13), "(10,13) should be blocked");
        assert!(map.is_blocked(10, 7), "(10,7) should be blocked");
    }

    #[test]
    fn out_of_bounds_does_not_panic_returns_false() {
        let map = ObstacleMap::new(5, 5);
        // Access well beyond bounds.
        assert!(!map.is_blocked(100, 100));
        assert!(!map.is_blocked(u32::MAX, u32::MAX));
    }

    #[test]
    fn set_blocked_out_of_bounds_is_no_op() {
        let mut map = ObstacleMap::new(3, 3);
        // Should not panic.
        map.set_blocked(99, 99, true);
        // Nothing in bounds should be affected.
        for gy in 0..3 {
            for gx in 0..3 {
                assert!(!map.is_blocked(gx, gy));
            }
        }
    }

    #[test]
    fn mark_rect_blocked_clamped_to_bounds() {
        let mut map = ObstacleMap::new(4, 4);
        // Rectangle extends beyond grid — should not panic.
        map.mark_rect_blocked(2, 2, 100, 100);
        assert!(map.is_blocked(3, 3), "in-bounds corner must be blocked");
        assert!(!map.is_blocked(0, 0), "unaffected corner must be free");
    }
}
