//! Pad escape planning for dense SMD components (BGA, QFP).
//!
//! Generates short escape traces and vias from densely-packed SMD pads on the
//! top copper layer to inner layers, creating routable access points before
//! the PathFinder negotiation loop starts.
//!
//! # Algorithm
//!
//! For each SMD pad on a component that has fewer than `min_access_threshold`
//! free neighbours, the planner:
//! 1. Computes the escape direction (away from the component center, quantized
//!    to 4 cardinal directions).
//! 2. Selects a target inner layer via round-robin per component.
//! 3. Walks up to `max_escape_mm / resolution_mm` cells in the escape
//!    direction, stopping at the first cell that is free on both source and
//!    target layers.
//! 4. If all four cardinal directions are blocked, skips the pad.
//!
//! The resulting [`EscapePlan`] is applied to the obstacle maps before access
//! points are computed, so the escape via positions become the effective
//! routing start/goal points for the detailed A* router.

use autopcb_ir::PcbIr;
use autopcb_routes::{LayerId, NetId};

use crate::config::EscapeConfig;
use crate::obstacles::ObstacleMap;
use crate::workspace::GridConfig;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A pre-routed escape segment for a single SMD pad.
///
/// Describes the short trace from the pad to a via location on an inner layer.
#[derive(Debug, Clone, PartialEq)]
pub struct EscapeRoute {
    /// Net the pad belongs to.
    pub net_id: NetId,
    /// Grid cell of the pad that this escape route serves.
    pub pad_cell: (u32, u32),
    /// Copper layer the pad lives on (source of the escape trace).
    pub source_layer: LayerId,
    /// Inner layer the via lands on (destination of the escape).
    pub target_layer: LayerId,
    /// Grid cells occupied by the escape trace on the source layer
    /// (not including the pad cell itself).
    pub trace_cells: Vec<(u32, u32)>,
    /// Grid cell where the via is placed (last cell of the escape trace).
    pub via_cell: (u32, u32),
}

/// The complete set of pre-planned escape routes for a board.
#[derive(Debug, Clone, Default)]
pub struct EscapePlan {
    pub routes: Vec<EscapeRoute>,
}

// ---------------------------------------------------------------------------
// plan_escapes
// ---------------------------------------------------------------------------

/// Plan escape routes for all qualifying SMD pads in the IR.
///
/// Returns an [`EscapePlan`] that should be applied to the obstacle maps
/// (via [`apply_escapes`]) before access points are computed.
pub fn plan_escapes(
    ir: &PcbIr,
    grid: &GridConfig,
    obstacle_maps: &[ObstacleMap],
    layer_count: usize,
    config: &EscapeConfig,
) -> EscapePlan {
    if !config.enabled {
        return EscapePlan::default();
    }

    // Need at least 3 copper layers (top + ≥1 inner + bottom) to escape to
    // an inner layer.
    if layer_count <= 2 {
        return EscapePlan::default();
    }

    let max_steps = ((config.max_escape_mm / grid.resolution_mm).ceil() as u32).max(1);

    let mut routes: Vec<EscapeRoute> = Vec::new();

    for (_comp_id, comp) in ir.components.iter() {
        // Round-robin inner layer selector per component.
        // Inner layers are indices 1..layer_count-1.
        let inner_layer_count = layer_count - 2;
        let mut layer_cursor: usize = 0;

        for pad in &comp.pads {
            // Skip through-hole pads — they already span all layers.
            if pad.is_through_hole {
                continue;
            }
            // Skip unassigned pads — no net to route.
            let net_id = match pad.net {
                Some(n) => NetId(n.raw()),
                None => continue,
            };

            // Only consider pads on a single source layer (SMD).
            let source_ir_layer = match pad.layer_set.first() {
                Some(&l) => l,
                None => continue,
            };
            let source_layer = LayerId(source_ir_layer.raw() as u16);
            let source_map_idx = source_ir_layer.raw() as usize;

            if source_map_idx >= obstacle_maps.len() {
                continue;
            }

            let (pad_gx, pad_gy) = grid.to_grid(pad.world_position);

            // Count free access cells around the pad on its source layer.
            let free_count = count_free_neighbours(
                pad_gx,
                pad_gy,
                &obstacle_maps[source_map_idx],
                grid,
            );
            if free_count >= config.min_access_threshold {
                // Pad has enough access already.
                continue;
            }

            // Determine escape direction: normalized vector from component
            // center to pad, quantized to 4 cardinal directions.
            let directions = escape_directions(comp.position, pad.world_position);

            // Choose target inner layer (round-robin).
            let inner_idx = 1 + (layer_cursor % inner_layer_count);
            layer_cursor += 1;
            let target_layer = LayerId(inner_idx as u16);
            let target_map_idx = inner_idx;

            if target_map_idx >= obstacle_maps.len() {
                continue;
            }

            // Try escape directions in order until one succeeds.
            let mut found: Option<EscapeRoute> = None;
            'dir: for &(dx, dy) in &directions {
                let mut trace_cells: Vec<(u32, u32)> = Vec::new();
                let mut cx = pad_gx as i64;
                let mut cy = pad_gy as i64;

                for _ in 0..max_steps {
                    cx += dx as i64;
                    cy += dy as i64;
                    if cx < 0 || cy < 0 {
                        continue 'dir;
                    }
                    let gcx = cx as u32;
                    let gcy = cy as u32;
                    if !grid.in_bounds(gcx, gcy) {
                        continue 'dir;
                    }
                    // Cell must be free on both source and target layers.
                    if obstacle_maps[source_map_idx].is_blocked(gcx, gcy)
                        || obstacle_maps[target_map_idx].is_blocked(gcx, gcy)
                    {
                        // Blocked — this direction is a dead end.
                        continue 'dir;
                    }
                    trace_cells.push((gcx, gcy));
                    // Require at least min_escape_mm distance before placing via.
                    let dist_mm =
                        trace_cells.len() as f64 * grid.resolution_mm;
                    if dist_mm >= config.min_escape_mm {
                        let via_cell = (gcx, gcy);
                        found = Some(EscapeRoute {
                            net_id,
                            pad_cell: (pad_gx, pad_gy),
                            source_layer,
                            target_layer,
                            trace_cells,
                            via_cell,
                        });
                        break 'dir;
                    }
                }
            }

            if let Some(route) = found {
                routes.push(route);
            }
        }
    }

    EscapePlan { routes }
}

// ---------------------------------------------------------------------------
// apply_escapes
// ---------------------------------------------------------------------------

/// Apply a pre-planned escape to the obstacle maps.
///
/// Marks trace cells as blocked on the source layer and the via cell as
/// blocked on both source and target layers. The `inflate` margin expands
/// each blocked region by that many cells in each direction.
pub fn apply_escapes(
    plan: &EscapePlan,
    _grid: &GridConfig,
    obstacle_maps: &mut [ObstacleMap],
    inflate: u32,
) {
    for route in &plan.routes {
        let src_idx = route.source_layer.raw() as usize;
        let tgt_idx = route.target_layer.raw() as usize;

        if src_idx >= obstacle_maps.len() || tgt_idx >= obstacle_maps.len() {
            continue;
        }

        // Mark trace cells on the source layer.
        for &(gx, gy) in &route.trace_cells {
            mark_with_inflate(&mut obstacle_maps[src_idx], gx, gy, inflate);
        }

        // Mark via cell on both source and target layers.
        let (vx, vy) = route.via_cell;
        mark_with_inflate(&mut obstacle_maps[src_idx], vx, vy, inflate);
        mark_with_inflate(&mut obstacle_maps[tgt_idx], vx, vy, inflate);
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Count free (unblocked, in-bounds) 8-neighbour cells of `(gx, gy)` on the
/// given obstacle map.
fn count_free_neighbours(
    gx: u32,
    gy: u32,
    map: &ObstacleMap,
    grid: &GridConfig,
) -> usize {
    const OFFSETS: [(i32, i32); 8] = [
        (-1, 0), (1, 0), (0, -1), (0, 1),
        (-1, -1), (1, -1), (-1, 1), (1, 1),
    ];
    let mut count = 0usize;
    for (dx, dy) in OFFSETS {
        let nx = gx as i64 + dx as i64;
        let ny = gy as i64 + dy as i64;
        if nx < 0 || ny < 0 {
            continue;
        }
        let nx = nx as u32;
        let ny = ny as u32;
        if grid.in_bounds(nx, ny) && !map.is_blocked(nx, ny) {
            count += 1;
        }
    }
    count
}

/// Compute a prioritized list of (dx, dy) escape directions for a pad.
///
/// The primary direction is the vector from the component center to the pad,
/// quantized to one of the 4 cardinal directions.  The remaining 3 cardinal
/// directions follow in order (90°, 180°, 270° rotations).
fn escape_directions(
    component_center: autopcb_ir::types::PointMm,
    pad_position: autopcb_ir::types::PointMm,
) -> [(i32, i32); 4] {
    let dx = pad_position.x - component_center.x;
    let dy = pad_position.y - component_center.y;

    // Quantize to primary cardinal direction.
    let primary: (i32, i32) = if dx.abs() >= dy.abs() {
        if dx >= 0.0 { (1, 0) } else { (-1, 0) }
    } else {
        if dy >= 0.0 { (0, 1) } else { (0, -1) }
    };

    // Generate 4 candidates: primary, +90°, 180°, -90°.
    let r90 = rotate_cw(primary);
    let r180 = rotate_cw(r90);
    let r270 = rotate_cw(r180);
    [primary, r90, r180, r270]
}

/// Rotate a cardinal direction 90° clockwise.
#[inline]
fn rotate_cw((dx, dy): (i32, i32)) -> (i32, i32) {
    (dy, -dx)
}

/// Mark a grid cell and an optional inflation margin as blocked.
fn mark_with_inflate(map: &mut ObstacleMap, gx: u32, gy: u32, inflate: u32) {
    let min_gx = gx.saturating_sub(inflate);
    let min_gy = gy.saturating_sub(inflate);
    let max_gx = gx.saturating_add(inflate);
    let max_gy = gy.saturating_add(inflate);
    map.mark_rect_blocked(min_gx, min_gy, max_gx, max_gy);
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
        handles::{ComponentId, IdMap, LayerId as IrLayerId, PadId},
        layer_stack::{IrCopperLayer, IrLayerStack, PreferredDirection},
        types::{BoardSide, BoundingBoxMm, PointMm},
        IrBoardGeometry,
    };

    fn make_grid(board_max: f64, resolution: f64) -> GridConfig {
        let width_cells = (board_max / resolution).ceil() as u32 + 1;
        let height_cells = (board_max / resolution).ceil() as u32 + 1;
        GridConfig {
            resolution_mm: resolution,
            width_cells,
            height_cells,
            origin: PointMm::new(0.0, 0.0),
        }
    }

    fn make_obstacle_maps(grid: &GridConfig, layer_count: usize) -> Vec<ObstacleMap> {
        (0..layer_count)
            .map(|_| ObstacleMap::new(grid.width_cells, grid.height_cells))
            .collect()
    }

    fn make_copper_layers(n: usize) -> Vec<IrCopperLayer> {
        (0..n)
            .map(|i| IrCopperLayer {
                id: IrLayerId::from(i as u32),
                name: format!("Layer{i}"),
                is_top: i == 0,
                is_bottom: i == n - 1,
                preferred_direction: Some(PreferredDirection::Any),
            })
            .collect()
    }

    fn make_smd_pad(id: u32, world_pos: PointMm, net_raw: u32, layer_id: u32) -> IrComponentPad {
        IrComponentPad {
            id: PadId::from(id),
            name: format!("{id}"),
            local_position: world_pos,
            world_position: world_pos,
            net: Some(autopcb_ir::handles::NetId::from(net_raw)),
            shape: PadShapeInfo {
                kind: PadShapeKind::Round,
                size_x: 0.3,
                size_y: 0.3,
                rotation: 0.0,
            },
            is_through_hole: false,
            hole_size_mm: 0.0,
            swap_id_pin: None,
            swap_id_part: None,
            layer_set: vec![IrLayerId::from(layer_id)],
        }
    }

    fn make_through_hole_pad(id: u32, world_pos: PointMm) -> IrComponentPad {
        IrComponentPad {
            id: PadId::from(id),
            name: format!("{id}"),
            local_position: world_pos,
            world_position: world_pos,
            net: Some(autopcb_ir::handles::NetId::from(0)),
            shape: PadShapeInfo {
                kind: PadShapeKind::Round,
                size_x: 1.0,
                size_y: 1.0,
                rotation: 0.0,
            },
            is_through_hole: true,
            hole_size_mm: 0.8,
            swap_id_pin: None,
            swap_id_part: None,
            layer_set: vec![IrLayerId::from(0), IrLayerId::from(1)],
        }
    }

    fn make_component(id: u32, pos: PointMm, pads: Vec<IrComponentPad>) -> IrComponent {
        IrComponent {
            id: ComponentId::from(id),
            designator: format!("U{id}"),
            pattern: "BGA".into(),
            value: "".into(),
            position: pos,
            rotation: 0.0,
            side: BoardSide::Top,
            local_bounds: BoundingBoxMm::new(PointMm::new(-5.0, -5.0), PointMm::new(5.0, 5.0)),
            world_bounds: BoundingBoxMm::new(
                PointMm::new(pos.x - 5.0, pos.y - 5.0),
                PointMm::new(pos.x + 5.0, pos.y + 5.0),
            ),
            pads,
        }
    }

    fn make_ir(components: Vec<IrComponent>, layer_count: usize) -> autopcb_ir::PcbIr {
        let board_max = 50.0;
        let copper_layers = make_copper_layers(layer_count);
        let mut comp_map: IdMap<ComponentId, IrComponent> = IdMap::new();
        for comp in components {
            comp_map.push(comp);
        }
        autopcb_ir::PcbIr {
            board: IrBoardGeometry {
                outline: vec![
                    PointMm::new(0.0, 0.0),
                    PointMm::new(board_max, 0.0),
                    PointMm::new(board_max, board_max),
                    PointMm::new(0.0, board_max),
                ],
                cutouts: vec![],
                bounds: BoundingBoxMm::new(
                    PointMm::new(0.0, 0.0),
                    PointMm::new(board_max, board_max),
                ),
                keepouts: vec![],
            },
            layer_stack: IrLayerStack {
                copper_layers,
                copper_layer_count: layer_count,
            },
            components: comp_map,
            nets: IdMap::new(),
            rules: IdMap::new(),
            free_copper: FreeCopperGeometry::default(),
            polygons: IdMap::new(),
            texts: IdMap::new(),
            regions: IdMap::new(),
            component_bodies: IdMap::new(),
        }
    }

    // -----------------------------------------------------------------------
    // Test: through-hole only board → empty plan
    // -----------------------------------------------------------------------

    #[test]
    fn plan_escapes_empty_for_through_hole_only() {
        let pos = PointMm::new(25.0, 25.0);
        let pad = make_through_hole_pad(0, pos);
        let comp = make_component(0, pos, vec![pad]);
        let ir = make_ir(vec![comp], 4);
        let grid = make_grid(50.0, 0.5);
        let maps = make_obstacle_maps(&grid, 4);
        let config = EscapeConfig::default();
        let plan = plan_escapes(&ir, &grid, &maps, 4, &config);
        assert!(
            plan.routes.is_empty(),
            "through-hole pads must not generate escape routes"
        );
    }

    // -----------------------------------------------------------------------
    // Test: 2-layer board → empty plan (no inner layers)
    // -----------------------------------------------------------------------

    #[test]
    fn plan_escapes_empty_for_two_layer_board() {
        let pos = PointMm::new(25.0, 25.0);
        let pad = make_smd_pad(0, pos, 1, 0);
        let comp = make_component(0, pos, vec![pad]);
        let ir = make_ir(vec![comp], 2);
        let grid = make_grid(50.0, 0.5);
        let maps = make_obstacle_maps(&grid, 2);
        let config = EscapeConfig::default();
        let plan = plan_escapes(&ir, &grid, &maps, 2, &config);
        assert!(
            plan.routes.is_empty(),
            "2-layer board has no inner layers; escape plan must be empty"
        );
    }

    // -----------------------------------------------------------------------
    // Test: dense SMD component with blocked neighbours → routes generated
    // -----------------------------------------------------------------------

    #[test]
    fn plan_escapes_generates_routes_for_dense_smd() {
        // Place a component at center with one pad to the right.
        // Block all 8 neighbours of the pad on layer 0 to force escape planning.
        let center = PointMm::new(25.0, 25.0);
        // Pad at (30, 25) — offset from center so escape direction is rightward.
        let pad_pos = PointMm::new(30.0, 25.0);
        let pad = make_smd_pad(0, pad_pos, 1, 0);
        let comp = make_component(0, center, vec![pad]);
        let ir = make_ir(vec![comp], 4);

        let grid = make_grid(50.0, 0.5);
        let mut maps = make_obstacle_maps(&grid, 4);

        // Block 6 of the 8 neighbours of the pad cell on layer 0 so that
        // free_count (= 2) < min_access_threshold (= 3), triggering escape
        // planning.  Leave the rightward cell and one other unblocked so the
        // escape walk can find a path in the +x direction.  We do NOT block
        // (pgx+1, pgy) so the escape trace can exit there.
        let (pgx, pgy) = grid.to_grid(pad_pos);
        let blocked_offsets: [(i32, i32); 6] = [
            (-1, 0), (0, -1), (0, 1),
            (-1, -1), (1, -1), (-1, 1),
        ];
        for (dx, dy) in blocked_offsets {
            let nx = (pgx as i64 + dx as i64) as u32;
            let ny = (pgy as i64 + dy as i64) as u32;
            maps[0].set_blocked(nx, ny, true);
        }

        let config = EscapeConfig {
            enabled: true,
            min_escape_mm: 0.5,
            max_escape_mm: 3.0,
            min_access_threshold: 3,
        };
        let plan = plan_escapes(&ir, &grid, &maps, 4, &config);

        assert!(
            !plan.routes.is_empty(),
            "dense SMD pad with blocked neighbours should produce escape routes"
        );

        let route = &plan.routes[0];
        assert_eq!(route.source_layer, LayerId(0), "escape must start on top layer");
        assert!(
            route.target_layer.raw() > 0 && route.target_layer.raw() < 3,
            "escape must target an inner layer"
        );
        assert!(!route.trace_cells.is_empty(), "escape trace must have at least one cell");
    }

    // -----------------------------------------------------------------------
    // Test: escape direction computation
    // -----------------------------------------------------------------------

    #[test]
    fn escape_direction_rightward_pad() {
        let center = PointMm::new(0.0, 0.0);
        let pad_right = PointMm::new(5.0, 0.0);
        let dirs = escape_directions(center, pad_right);
        // Primary direction must be rightward (1, 0).
        assert_eq!(dirs[0], (1, 0), "pad to the right → primary direction (1,0)");
        // Remaining are 90° rotations.
        assert_eq!(dirs[1], rotate_cw(dirs[0]));
        assert_eq!(dirs[2], rotate_cw(dirs[1]));
        assert_eq!(dirs[3], rotate_cw(dirs[2]));
    }

    #[test]
    fn escape_direction_upward_pad() {
        let center = PointMm::new(0.0, 0.0);
        let pad_up = PointMm::new(0.0, 5.0);
        let dirs = escape_directions(center, pad_up);
        assert_eq!(dirs[0], (0, 1), "pad above → primary direction (0,1)");
    }

    #[test]
    fn escape_direction_diagonal_quantized() {
        let center = PointMm::new(0.0, 0.0);
        // dx=3, dy=4 → dy > dx, so primary should be (0,1).
        let pad = PointMm::new(3.0, 4.0);
        let dirs = escape_directions(center, pad);
        assert_eq!(dirs[0], (0, 1), "dominant dy→ primary (0,1)");
    }

    // -----------------------------------------------------------------------
    // Test: disabled config → empty plan
    // -----------------------------------------------------------------------

    #[test]
    fn plan_escapes_disabled_returns_empty() {
        let pos = PointMm::new(25.0, 25.0);
        let pad = make_smd_pad(0, pos, 1, 0);
        let comp = make_component(0, pos, vec![pad]);
        let ir = make_ir(vec![comp], 4);
        let grid = make_grid(50.0, 0.5);
        let maps = make_obstacle_maps(&grid, 4);
        let config = EscapeConfig {
            enabled: false,
            ..EscapeConfig::default()
        };
        let plan = plan_escapes(&ir, &grid, &maps, 4, &config);
        assert!(plan.routes.is_empty(), "disabled escape planning must return empty plan");
    }

    // -----------------------------------------------------------------------
    // Test: apply_escapes blocks correct cells
    // -----------------------------------------------------------------------

    #[test]
    fn apply_escapes_marks_cells_blocked() {
        let grid = make_grid(20.0, 1.0);
        let mut maps = make_obstacle_maps(&grid, 4);

        let plan = EscapePlan {
            routes: vec![EscapeRoute {
                net_id: NetId(1),
                pad_cell: (4, 5),
                source_layer: LayerId(0),
                target_layer: LayerId(1),
                trace_cells: vec![(5, 5), (6, 5)],
                via_cell: (6, 5),
            }],
        };

        apply_escapes(&plan, &grid, &mut maps, 0);

        // Trace cells blocked on source layer.
        assert!(maps[0].is_blocked(5, 5), "trace cell (5,5) on layer 0 must be blocked");
        assert!(maps[0].is_blocked(6, 5), "via cell (6,5) on layer 0 must be blocked");
        // Via cell also blocked on target layer.
        assert!(maps[1].is_blocked(6, 5), "via cell (6,5) on layer 1 must be blocked");
        // Other cells unaffected.
        assert!(!maps[0].is_blocked(4, 5), "cell (4,5) must be unblocked");
        assert!(!maps[2].is_blocked(6, 5), "layer 2 must be unaffected");
    }
}
