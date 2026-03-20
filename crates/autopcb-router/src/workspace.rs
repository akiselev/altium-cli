//! `RoutingWorkspace` — derived routing state built from `PcbIr` + `RoutingConfig`.
//!
//! [`build_workspace`] is the sole entry point.  It:
//!
//! 1. Calls [`crate::rules::build_policy`] to derive a [`RoutingPolicy`].
//! 2. Computes a [`GridConfig`] from the board bounding box and grid resolution.
//! 3. Iterates all pads, keepouts, pre-routed tracks/vias, and the board edge
//!    to populate an [`rstar::RTree`] (via [`SpatialIndex::build`]) and
//!    per-layer [`ObstacleMap`] bitmaps.
//! 4. Computes [`AccessPoint`]s for every pad: the 8 grid cells adjacent to
//!    the pad center cell, filtered to those that are within bounds and not
//!    blocked at construction time.
//!
//! # Clearance inflation
//!
//! When blocking a pad or pre-routed segment in the bitmap, the blocked region
//! is expanded by the global clearance converted to grid cells:
//! `inflate_cells = ceil(clearance_mm / resolution_mm)`.
//!
//! # Board-edge blocking
//!
//! All grid cells that fall outside the board bounding box (i.e. with world
//! coordinates outside `IrBoardGeometry::bounds`) are marked blocked on every
//! layer.  This is an approximation for non-rectangular boards; the R-tree
//! edge entries and DRC provide the precise constraint.
//!
//! # LayerId conversion
//!
//! `autopcb_ir::LayerId` is `u32`; `autopcb_routes::LayerId` is `u16`.
//! Conversion: `routes::LayerId(ir_id.raw() as u16)` guarded by
//! `debug_assert!(ir_id.raw() <= u16::MAX as u32)`.
//!
//! # Vec indexing for per-layer maps
//!
//! `obstacle_maps` is a flat `Vec<ObstacleMap>` with one entry per copper
//! layer.  It is indexed by `ir_layer.raw() as usize` (the raw u32 from
//! `autopcb_ir::LayerId`).  This matches the plan decision:
//! > "Per-layer collections use Vec not BTreeMap … indexed by
//! > `layer.raw() as usize`"

use std::collections::HashMap;

use autopcb_ir::handles::{ComponentId, LayerId as IrLayerId, NetId as IrNetId, PadId};
use autopcb_ir::types::{BoundingBoxMm, PointMm};
use autopcb_ir::PcbIr;
use autopcb_routes::{LayerId, NetId};

use crate::config::RoutingConfig;
use crate::obstacles::{AccessPoint, ObstacleMap};
use crate::rules::{build_policy, RoutingPolicy};
use crate::spatial::{ObstacleEntry, SpatialIndex};
use crate::RoutingError;

// ---------------------------------------------------------------------------
// GridConfig
// ---------------------------------------------------------------------------

/// Configuration for the routing grid (mm-based origin + cell size).
#[derive(Debug, Clone, Copy)]
pub struct GridConfig {
    /// Size of each grid cell in mm.
    pub resolution_mm: f64,
    /// Number of grid columns (x direction).
    pub width_cells: u32,
    /// Number of grid rows (y direction).
    pub height_cells: u32,
    /// World coordinate of the grid origin (min corner of the board bounding box).
    pub origin: PointMm,
}

impl GridConfig {
    /// Convert a world point (mm) to grid coordinates `(gx, gy)`.
    ///
    /// Clamps negative results to 0; does **not** clamp to the upper bound.
    /// Use [`Self::in_bounds`] to validate.
    pub fn to_grid(&self, point: PointMm) -> (u32, u32) {
        let fx = (point.x - self.origin.x) / self.resolution_mm;
        let fy = (point.y - self.origin.y) / self.resolution_mm;
        let gx = fx.max(0.0).floor() as u32;
        let gy = fy.max(0.0).floor() as u32;
        (gx, gy)
    }

    /// Convert grid coordinates to the world-space mm center of the cell.
    pub fn to_mm(&self, gx: u32, gy: u32) -> PointMm {
        PointMm {
            x: self.origin.x + (gx as f64 + 0.5) * self.resolution_mm,
            y: self.origin.y + (gy as f64 + 0.5) * self.resolution_mm,
        }
    }

    /// Returns `true` if `(gx, gy)` is a valid grid cell.
    pub fn in_bounds(&self, gx: u32, gy: u32) -> bool {
        gx < self.width_cells && gy < self.height_cells
    }

    /// Number of grid cells to inflate an obstacle by for a given clearance.
    pub fn inflate_cells(&self, clearance_mm: f64) -> u32 {
        (clearance_mm / self.resolution_mm).ceil() as u32
    }

    /// Build a `GridConfig` from a board bounding box and grid resolution.
    fn from_bounds(bounds: BoundingBoxMm, resolution_mm: f64) -> Result<GridConfig, RoutingError> {
        if resolution_mm <= 0.0 {
            return Err(RoutingError::InvalidConfig(format!(
                "grid_resolution_mm must be positive, got {resolution_mm}"
            )));
        }
        let w = bounds.width();
        let h = bounds.height();
        if w <= 0.0 || h <= 0.0 {
            return Err(RoutingError::WorkspaceBuildError(format!(
                "board bounding box has zero or negative extent: {w}×{h} mm"
            )));
        }
        let width_cells = (w / resolution_mm).ceil() as u32 + 1;
        let height_cells = (h / resolution_mm).ceil() as u32 + 1;
        Ok(GridConfig {
            resolution_mm,
            width_cells,
            height_cells,
            origin: bounds.min,
        })
    }
}

// ---------------------------------------------------------------------------
// PadKey
// ---------------------------------------------------------------------------

/// Identifies a specific pad on a specific component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PadKey {
    pub component_id: ComponentId,
    pub pad_id: PadId,
}

// ---------------------------------------------------------------------------
// RoutingWorkspace
// ---------------------------------------------------------------------------

/// Complete derived routing state for one routing invocation.
///
/// Built fresh from `PcbIr` + `RoutingConfig` via [`build_workspace`]; never
/// persisted between invocations.
#[derive(Debug)]
pub struct RoutingWorkspace {
    /// Routing policy derived from IR design rules and config.
    pub policy: RoutingPolicy,
    /// R-tree over all fixed obstacles (pads, keepouts, board edge,
    /// pre-routed traces).
    pub spatial_index: SpatialIndex,
    /// Per-layer obstacle bitmaps indexed by `ir_layer_id.raw() as usize`.
    /// Length == `layer_count`.
    pub obstacle_maps: Vec<ObstacleMap>,
    /// Grid configuration (origin, resolution, dimensions).
    pub grid: GridConfig,
    /// Access points keyed by pad identity.
    pub pin_accesses: HashMap<PadKey, Vec<AccessPoint>>,
    /// Number of copper layers (== `obstacle_maps.len()`).
    pub layer_count: usize,
}

impl RoutingWorkspace {
    /// Returns `true` if grid cell `(gx, gy)` on `layer` is blocked for a
    /// router trying to place a trace belonging to `net_id`.
    ///
    /// A cell is considered **unblocked** (pass-through) if it is occupied
    /// only by an obstacle that belongs to the **same** net as the router query
    /// (same-net pass-through).  Any other occupant blocks the cell.
    ///
    /// Out-of-bounds cells are always treated as blocked.
    pub fn is_blocked(&self, layer: IrLayerId, gx: u32, gy: u32, net_id: Option<NetId>) -> bool {
        let idx = layer.raw() as usize;
        if idx >= self.obstacle_maps.len() {
            return true;
        }
        if !self.grid.in_bounds(gx, gy) {
            return true;
        }
        if !self.obstacle_maps[idx].is_blocked(gx, gy) {
            return false;
        }
        // Cell is marked blocked in the bitmap.  Check if it is blocked
        // *only* by obstacles that belong to the same net — if so, allow
        // pass-through.
        if let Some(query_net) = net_id {
            let cell_mm = self.grid.to_mm(gx, gy);
            let r = self.grid.resolution_mm / 2.0;
            let candidates = self.spatial_index.query_rect([
                cell_mm.x - r,
                cell_mm.y - r,
                cell_mm.x + r,
                cell_mm.y + r,
            ]);
            // If every obstacle touching this cell is same-net, allow pass-through.
            let all_same_net = candidates.iter().all(|obs| {
                obs.net_id()
                    .map_or(false, |obs_net| obs_net == query_net)
            });
            if all_same_net && !candidates.is_empty() {
                return false;
            }
        }
        true
    }

    /// Return access points for the pad identified by `key`.
    pub fn pin_accesses(&self, key: PadKey) -> &[AccessPoint] {
        self.pin_accesses
            .get(&key)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Spatial clearance query: obstacles within `clearance` mm of a segment
    /// on the given layer.
    pub fn clearance_query(
        &self,
        layer: LayerId,
        start: PointMm,
        end: PointMm,
        clearance: f64,
    ) -> Vec<&ObstacleEntry> {
        self.spatial_index
            .clearance_query(layer, start.x, start.y, end.x, end.y, clearance)
    }
}

// ---------------------------------------------------------------------------
// build_workspace
// ---------------------------------------------------------------------------

/// Convert an `autopcb_ir::NetId` to `autopcb_routes::NetId`.
fn to_routes_net(id: IrNetId) -> NetId {
    NetId(id.raw())
}

/// Convert an `autopcb_ir::LayerId` to `autopcb_routes::LayerId`.
fn to_routes_layer(id: IrLayerId) -> LayerId {
    debug_assert!(
        id.raw() <= u16::MAX as u32,
        "LayerId({}) overflows u16",
        id.raw()
    );
    LayerId(id.raw() as u16)
}

/// Build a [`RoutingWorkspace`] from a board IR and routing config.
///
/// This is the main construction entry point called by [`crate::build_workspace`].
pub fn build_workspace(
    ir: &PcbIr,
    config: &RoutingConfig,
) -> Result<RoutingWorkspace, RoutingError> {
    // ------------------------------------------------------------------
    // 1. Routing policy
    // ------------------------------------------------------------------
    let policy = build_policy(ir, config)?;

    // ------------------------------------------------------------------
    // 2. Grid config
    // ------------------------------------------------------------------
    let grid = GridConfig::from_bounds(ir.board.bounds, config.grid_resolution_mm)?;

    // ------------------------------------------------------------------
    // 3. Allocate per-layer obstacle maps
    // ------------------------------------------------------------------
    let layer_count = ir.layer_stack.copper_layers.len();
    let mut obstacle_maps: Vec<ObstacleMap> = ir
        .layer_stack
        .copper_layers
        .iter()
        .map(|_| ObstacleMap::new(grid.width_cells, grid.height_cells))
        .collect();

    // ------------------------------------------------------------------
    // 4. Global clearance in grid cells (for bitmap inflation)
    // ------------------------------------------------------------------
    // Use a sentinel net pair for the global clearance query — since all
    // pads currently use the same global clearance this is sufficient.
    let sentinel = NetId(u32::MAX);
    let clearance_mm = policy.clearance(sentinel, sentinel);
    let inflate = grid.inflate_cells(clearance_mm);

    // ------------------------------------------------------------------
    // 5. Accumulate obstacle entries
    // ------------------------------------------------------------------
    let mut entries: Vec<ObstacleEntry> = Vec::new();

    // 5a. Board edge — mark all cells outside the board bounding box.
    //     For convex-outline approximation we only mark the 4 outside regions
    //     (left, right, bottom, top strips) as blocked on all layers.
    mark_board_edge_blocked(&ir.board.bounds, &grid, &mut obstacle_maps, &mut entries);

    // 5b. Component pads
    mark_pads(ir, &grid, &policy, clearance_mm, inflate, &mut obstacle_maps, &mut entries);

    // 5c. Keepout zones
    mark_keepouts(ir, &grid, inflate, &mut obstacle_maps, &mut entries);

    // 5d. Pre-routed tracks and vias (locked)
    mark_pre_routed(ir, &grid, inflate, &mut obstacle_maps, &mut entries);

    // ------------------------------------------------------------------
    // 6. Build spatial index
    // ------------------------------------------------------------------
    let spatial_index = SpatialIndex::build(entries);

    // ------------------------------------------------------------------
    // 7. Compute pad access points
    // ------------------------------------------------------------------
    let pin_accesses = compute_access_points(ir, &grid, &obstacle_maps, layer_count);

    Ok(RoutingWorkspace {
        policy,
        spatial_index,
        obstacle_maps,
        grid,
        pin_accesses,
        layer_count,
    })
}

// ---------------------------------------------------------------------------
// Board-edge blocking
// ---------------------------------------------------------------------------

/// Mark all grid cells that fall outside the board bounding box as blocked on
/// all layers, and add a single BoardEdge R-tree entry for the whole boundary.
fn mark_board_edge_blocked(
    bounds: &BoundingBoxMm,
    grid: &GridConfig,
    maps: &mut [ObstacleMap],
    entries: &mut Vec<ObstacleEntry>,
) {
    // Add a thin BoardEdge R-tree entry along each side of the bounding box.
    // We use a 1mm-thick band for the envelope; DRC enforces the exact outline.
    let eps = 1.0_f64;
    entries.push(ObstacleEntry::board_edge(
        bounds.min.x - eps,
        bounds.min.y - eps,
        bounds.max.x + eps,
        bounds.min.y,
    ));
    entries.push(ObstacleEntry::board_edge(
        bounds.min.x - eps,
        bounds.max.y,
        bounds.max.x + eps,
        bounds.max.y + eps,
    ));
    entries.push(ObstacleEntry::board_edge(
        bounds.min.x - eps,
        bounds.min.y,
        bounds.min.x,
        bounds.max.y,
    ));
    entries.push(ObstacleEntry::board_edge(
        bounds.max.x,
        bounds.min.y,
        bounds.max.x + eps,
        bounds.max.y,
    ));

    // Mark grid cells that lie entirely outside the board bounding box.
    for gy in 0..grid.height_cells {
        for gx in 0..grid.width_cells {
            let cell_center = grid.to_mm(gx, gy);
            if !bounds.contains(&cell_center) {
                for map in maps.iter_mut() {
                    map.set_blocked(gx, gy, true);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Pad marking
// ---------------------------------------------------------------------------

fn mark_pads(
    ir: &PcbIr,
    grid: &GridConfig,
    policy: &RoutingPolicy,
    clearance_mm: f64,
    inflate: u32,
    maps: &mut [ObstacleMap],
    entries: &mut Vec<ObstacleEntry>,
) {
    for (_comp_id, comp) in ir.components.iter() {
        for pad in &comp.pads {
            let net_id = pad.net.map(to_routes_net);
            let cx = pad.world_position.x;
            let cy = pad.world_position.y;

            // Effective pad radius: half the max(size_x, size_y) + clearance.
            let pad_radius = (pad.shape.size_x.max(pad.shape.size_y) / 2.0).max(0.0);
            let inflated_radius = pad_radius + clearance_mm;

            // R-tree entry (once per pad, not per layer — the bbox is 2D).
            if let Some(&first_layer) = pad.layer_set.first() {
                entries.push(ObstacleEntry::pad(
                    cx - inflated_radius,
                    cy - inflated_radius,
                    cx + inflated_radius,
                    cy + inflated_radius,
                    net_id,
                    first_layer,
                ));
            }

            // Bitmap: one entry per layer the pad occupies.
            let (gcx, gcy) = grid.to_grid(pad.world_position);
            let radius_cells = (pad_radius / grid.resolution_mm).ceil() as u32 + inflate;

            for &ir_layer in &pad.layer_set {
                let idx = ir_layer.raw() as usize;
                if idx < maps.len() {
                    // Use circle blocking for pads.
                    maps[idx].mark_circle_blocked(gcx, gcy, radius_cells);
                }
                // Additional R-tree entry per extra layer (for same-net query).
                if pad.layer_set.len() > 1 {
                    entries.push(ObstacleEntry::pad(
                        cx - inflated_radius,
                        cy - inflated_radius,
                        cx + inflated_radius,
                        cy + inflated_radius,
                        net_id,
                        ir_layer,
                    ));
                }
            }
        }
    }
    let _ = policy; // policy available for future per-net clearance queries
}

// ---------------------------------------------------------------------------
// Keepout marking
// ---------------------------------------------------------------------------

fn mark_keepouts(
    ir: &PcbIr,
    grid: &GridConfig,
    inflate: u32,
    maps: &mut [ObstacleMap],
    entries: &mut Vec<ObstacleEntry>,
) {
    for keepout in &ir.board.keepouts {
        // Compute bounding box of the keepout polygon.
        let bb = match autopcb_ir::types::BoundingBoxMm::from_points(&keepout.outline) {
            Some(bb) => bb,
            None => continue, // empty outline, skip
        };

        // Resolve layer restriction.
        let layer_opt: Option<LayerId> = keepout.layer_name.as_deref().and_then(|name| {
            ir.layer_stack
                .copper_layers
                .iter()
                .find(|l| l.name == name)
                .map(|l| to_routes_layer(l.id))
        });

        entries.push(ObstacleEntry::keepout(
            bb.min.x,
            bb.min.y,
            bb.max.x,
            bb.max.y,
            layer_opt,
        ));

        // Bitmap: map the keepout bounding box to grid cells with inflation.
        let (min_gx, min_gy) = grid.to_grid(bb.min);
        let (max_gx, max_gy) = grid.to_grid(bb.max);
        let min_gx = min_gx.saturating_sub(inflate);
        let min_gy = min_gy.saturating_sub(inflate);
        let max_gx = max_gx.saturating_add(inflate);
        let max_gy = max_gy.saturating_add(inflate);

        if let Some(routes_layer) = layer_opt {
            // Single-layer keepout.
            // Find the IR layer index by matching the routes layer id.
            for layer in &ir.layer_stack.copper_layers {
                if to_routes_layer(layer.id) == routes_layer {
                    let idx = layer.id.raw() as usize;
                    if idx < maps.len() {
                        maps[idx].mark_rect_blocked(min_gx, min_gy, max_gx, max_gy);
                    }
                }
            }
        } else {
            // All-layer keepout.
            for map in maps.iter_mut() {
                map.mark_rect_blocked(min_gx, min_gy, max_gx, max_gy);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Pre-routed copper marking
// ---------------------------------------------------------------------------

fn mark_pre_routed(
    ir: &PcbIr,
    grid: &GridConfig,
    inflate: u32,
    maps: &mut [ObstacleMap],
    entries: &mut Vec<ObstacleEntry>,
) {
    // Tracks locked or pre-routed.
    for track in &ir.free_copper.tracks {
        if !track.locked && !track.pre_routed {
            continue;
        }
        let net_id = track.net.map(to_routes_net);
        let hw = track.width_mm / 2.0;

        let min_x = track.start.x.min(track.end.x) - hw;
        let min_y = track.start.y.min(track.end.y) - hw;
        let max_x = track.start.x.max(track.end.x) + hw;
        let max_y = track.start.y.max(track.end.y) + hw;

        entries.push(ObstacleEntry::pre_routed_track(
            min_x, min_y, max_x, max_y, net_id, track.layer,
        ));

        // Bitmap.
        let (min_gx, min_gy) = grid.to_grid(PointMm::new(min_x, min_y));
        let (max_gx, max_gy) = grid.to_grid(PointMm::new(max_x, max_y));
        let min_gx = min_gx.saturating_sub(inflate);
        let min_gy = min_gy.saturating_sub(inflate);
        let max_gx = max_gx.saturating_add(inflate);
        let max_gy = max_gy.saturating_add(inflate);

        let idx = track.layer.raw() as usize;
        if idx < maps.len() {
            maps[idx].mark_rect_blocked(min_gx, min_gy, max_gx, max_gy);
        }
    }

    // Vias locked or pre-routed.
    for via in &ir.free_copper.vias {
        if !via.locked && !via.pre_routed {
            continue;
        }
        let net_id = via.net.map(to_routes_net);
        let r = via.diameter_mm / 2.0;
        let cx = via.position.x;
        let cy = via.position.y;

        entries.push(ObstacleEntry::pre_routed_via(
            cx - r,
            cy - r,
            cx + r,
            cy + r,
            net_id,
            via.from_layer,
            via.to_layer,
        ));

        // Bitmap: mark on all layers the via spans.
        let (gcx, gcy) = grid.to_grid(via.position);
        let radius_cells = (r / grid.resolution_mm).ceil() as u32 + inflate;

        // Mark all copper layers between from_layer and to_layer.
        let from_idx = via.from_layer.raw() as usize;
        let to_idx = via.to_layer.raw() as usize;
        let lo = from_idx.min(to_idx);
        let hi = from_idx.max(to_idx);
        for idx in lo..=hi {
            if idx < maps.len() {
                maps[idx].mark_circle_blocked(gcx, gcy, radius_cells);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Access point computation
// ---------------------------------------------------------------------------

/// 8-directional neighbour offsets.
const NEIGHBOUR_OFFSETS: [(i32, i32); 8] = [
    (-1, 0), (1, 0), (0, -1), (0, 1),
    (-1, -1), (1, -1), (-1, 1), (1, 1),
];

/// Compute access points for every pad in the IR.
fn compute_access_points(
    ir: &PcbIr,
    grid: &GridConfig,
    maps: &[ObstacleMap],
    _layer_count: usize,
) -> HashMap<PadKey, Vec<AccessPoint>> {
    let mut result: HashMap<PadKey, Vec<AccessPoint>> = HashMap::new();

    for (comp_id, comp) in ir.components.iter() {
        for pad in &comp.pads {
            let key = PadKey {
                component_id: comp_id,
                pad_id: pad.id,
            };
            let (gcx, gcy) = grid.to_grid(pad.world_position);
            let mut access: Vec<AccessPoint> = Vec::new();

            for &ir_layer in &pad.layer_set {
                let map_idx = ir_layer.raw() as usize;
                let layer = to_routes_layer(ir_layer);
                let map = match maps.get(map_idx) {
                    Some(m) => m,
                    None => continue,
                };

                for (dx, dy) in NEIGHBOUR_OFFSETS {
                    let ngx = gcx as i64 + dx as i64;
                    let ngy = gcy as i64 + dy as i64;
                    if ngx < 0 || ngy < 0 {
                        continue;
                    }
                    let ngx = ngx as u32;
                    let ngy = ngy as u32;
                    if grid.in_bounds(ngx, ngy) && !map.is_blocked(ngx, ngy) {
                        access.push(AccessPoint {
                            gx: ngx,
                            gy: ngy,
                            layer,
                        });
                    }
                }
            }

            result.insert(key, access);
        }
    }

    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use autopcb_ir::{
        component::{IrComponent, IrComponentPad, PadShapeInfo, PadShapeKind},
        copper::FreeCopperGeometry,
        handles::{ComponentId, IdMap, LayerId as IrLayerId, PadId, RuleId},
        layer_stack::{IrCopperLayer, IrLayerStack, PreferredDirection},
        rule::{IrDesignRule, IrRuleParams},
        types::{BoardSide, BoundingBoxMm, PointMm},
        IrBoardGeometry,
    };
    use altium_format_types::pcb::RuleKind;

    fn two_layer_ir(
        board_min: PointMm,
        board_max: PointMm,
    ) -> PcbIr {
        PcbIr {
            board: IrBoardGeometry {
                outline: vec![
                    board_min,
                    PointMm::new(board_max.x, board_min.y),
                    board_max,
                    PointMm::new(board_min.x, board_max.y),
                ],
                cutouts: vec![],
                bounds: BoundingBoxMm::new(board_min, board_max),
                keepouts: vec![],
            },
            layer_stack: IrLayerStack {
                copper_layers: vec![
                    IrCopperLayer {
                        id: IrLayerId::from(0),
                        name: "Top Layer".into(),
                        is_top: true,
                        is_bottom: false,
                        preferred_direction: Some(PreferredDirection::Any),
                    },
                    IrCopperLayer {
                        id: IrLayerId::from(1),
                        name: "Bottom Layer".into(),
                        is_top: false,
                        is_bottom: true,
                        preferred_direction: Some(PreferredDirection::Any),
                    },
                ],
                copper_layer_count: 2,
            },
            components: IdMap::new(),
            nets: IdMap::new(),
            rules: IdMap::new(),
            free_copper: FreeCopperGeometry::default(),
            polygons: IdMap::new(),
        }
    }

    fn empty_config() -> RoutingConfig {
        RoutingConfig::default()
    }

    // -----------------------------------------------------------------------
    // GridConfig tests
    // -----------------------------------------------------------------------

    #[test]
    fn grid_config_to_grid_roundtrip_approximate() {
        let bounds = BoundingBoxMm::new(PointMm::new(0.0, 0.0), PointMm::new(10.0, 10.0));
        let grid = GridConfig::from_bounds(bounds, 0.1).unwrap();

        // to_grid then to_mm should return within one cell of the original.
        let original = PointMm::new(5.0, 7.0);
        let (gx, gy) = grid.to_grid(original);
        let recovered = grid.to_mm(gx, gy);
        assert!(
            (recovered.x - original.x).abs() < grid.resolution_mm,
            "x roundtrip error too large"
        );
        assert!(
            (recovered.y - original.y).abs() < grid.resolution_mm,
            "y roundtrip error too large"
        );
    }

    #[test]
    fn grid_config_in_bounds() {
        let bounds = BoundingBoxMm::new(PointMm::new(0.0, 0.0), PointMm::new(1.0, 1.0));
        let grid = GridConfig::from_bounds(bounds, 0.5).unwrap();
        // width_cells = ceil(1.0/0.5)+1 = 3, height_cells = 3
        assert!(grid.in_bounds(0, 0));
        assert!(grid.in_bounds(grid.width_cells - 1, grid.height_cells - 1));
        assert!(!grid.in_bounds(grid.width_cells, 0));
        assert!(!grid.in_bounds(0, grid.height_cells));
    }

    #[test]
    fn grid_config_to_grid_clamps_negative() {
        let bounds = BoundingBoxMm::new(PointMm::new(5.0, 5.0), PointMm::new(10.0, 10.0));
        let grid = GridConfig::from_bounds(bounds, 0.1).unwrap();
        // A point below the origin should clamp to (0, 0).
        let (gx, gy) = grid.to_grid(PointMm::new(0.0, 0.0));
        assert_eq!((gx, gy), (0, 0));
    }

    #[test]
    fn grid_config_invalid_resolution_returns_error() {
        let bounds = BoundingBoxMm::new(PointMm::new(0.0, 0.0), PointMm::new(10.0, 10.0));
        assert!(GridConfig::from_bounds(bounds, 0.0).is_err());
        assert!(GridConfig::from_bounds(bounds, -0.1).is_err());
    }

    // -----------------------------------------------------------------------
    // build_workspace from empty IR
    // -----------------------------------------------------------------------

    #[test]
    fn build_workspace_empty_ir_succeeds() {
        let ir = two_layer_ir(PointMm::new(0.0, 0.0), PointMm::new(100.0, 100.0));
        let config = empty_config();
        let ws = build_workspace(&ir, &config).expect("build_workspace failed");
        assert_eq!(ws.layer_count, 2);
    }

    // -----------------------------------------------------------------------
    // Pad obstacle inflation
    // -----------------------------------------------------------------------

    #[test]
    fn pad_at_known_position_blocks_nearby_cells() {
        let mut ir = two_layer_ir(PointMm::new(0.0, 0.0), PointMm::new(20.0, 20.0));

        // Add a clearance rule: 0.5 mm.
        let rule_id = ir.rules.push(IrDesignRule {
            id: RuleId::from(0),
            name: "clearance".into(),
            kind: RuleKind::Clearance,
            priority: 1,
            enabled: true,
            params: IrRuleParams::Clearance { gap_mm: 0.5 },
        });
        ir.rules[rule_id].id = rule_id;

        // Add a component with a 1 mm round pad at (10.0, 10.0) on top layer.
        let pad = IrComponentPad {
            id: PadId::from(0),
            name: "1".into(),
            local_position: PointMm::new(0.0, 0.0),
            world_position: PointMm::new(10.0, 10.0),
            net: None,
            shape: PadShapeInfo {
                kind: PadShapeKind::Round,
                size_x: 1.0,
                size_y: 1.0,
                rotation: 0.0,
            },
            is_through_hole: false,
            hole_size_mm: 0.0,
            swap_id_pin: None,
            swap_id_part: None,
            layer_set: vec![IrLayerId::from(0)],
        };
        let comp = IrComponent {
            id: ComponentId::from(0),
            designator: "R1".into(),
            pattern: "0603".into(),
            value: "10k".into(),
            position: PointMm::new(10.0, 10.0),
            rotation: 0.0,
            side: BoardSide::Top,
            local_bounds: BoundingBoxMm::new(
                PointMm::new(-0.5, -0.5),
                PointMm::new(0.5, 0.5),
            ),
            world_bounds: BoundingBoxMm::new(
                PointMm::new(9.5, 9.5),
                PointMm::new(10.5, 10.5),
            ),
            pads: vec![pad],
        };
        ir.components.push(comp);

        let mut config = empty_config();
        config.grid_resolution_mm = 0.1;

        let ws = build_workspace(&ir, &config).expect("build_workspace failed");

        // Pad center at (10,10), pad radius = 0.5mm, clearance = 0.5mm.
        // Inflated radius ≈ (0.5+0.5)/0.1 = 10 cells.
        // Grid cell for (10.0, 10.0): gx=100, gy=100.
        // The pad center cell itself should be blocked.
        let layer0 = IrLayerId::from(0);
        let (gcx, gcy) = ws.grid.to_grid(PointMm::new(10.0, 10.0));
        assert!(
            ws.is_blocked(layer0, gcx, gcy, None),
            "pad center cell must be blocked"
        );

        // A cell at (5.0, 5.0) (far from pad) should be unblocked.
        let (gfx, gfy) = ws.grid.to_grid(PointMm::new(5.0, 5.0));
        assert!(
            !ws.is_blocked(layer0, gfx, gfy, None),
            "far cell should be unblocked"
        );
    }

    // -----------------------------------------------------------------------
    // Keepout blocking
    // -----------------------------------------------------------------------

    #[test]
    fn keepout_region_blocks_cells() {
        use autopcb_ir::IrKeepoutZone;

        let mut ir = two_layer_ir(PointMm::new(0.0, 0.0), PointMm::new(50.0, 50.0));

        // Keepout from (10,10) to (20,20) on all layers.
        ir.board.keepouts.push(IrKeepoutZone {
            outline: vec![
                PointMm::new(10.0, 10.0),
                PointMm::new(20.0, 10.0),
                PointMm::new(20.0, 20.0),
                PointMm::new(10.0, 20.0),
            ],
            layer_name: None,
        });

        let mut config = empty_config();
        config.grid_resolution_mm = 1.0;

        let ws = build_workspace(&ir, &config).expect("build_workspace failed");
        let layer0 = IrLayerId::from(0);

        // Center of keepout (15,15) should be blocked.
        let (gx, gy) = ws.grid.to_grid(PointMm::new(15.0, 15.0));
        assert!(
            ws.is_blocked(layer0, gx, gy, None),
            "keepout center must be blocked"
        );

        // Far outside keepout (1,1) should not be blocked (board edge excluded).
        let (gx2, gy2) = ws.grid.to_grid(PointMm::new(5.0, 5.0));
        assert!(
            !ws.is_blocked(layer0, gx2, gy2, None),
            "outside keepout should be unblocked"
        );
    }
}
