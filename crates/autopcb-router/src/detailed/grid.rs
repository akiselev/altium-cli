//! 3D routing grid: `GridNode { x, y, layer }` and `GridRouter` A* implementation.
//!
//! Supports 4-way (cardinal) and 8-way (diagonal) movement plus layer
//! transitions via vias. Uses `pathfinding::directed::astar::astar` with
//! `ordered_float::OrderedFloat<f64>` costs.
//!
//! # Successor generation
//!
//! For each node during A*:
//! 1. Generate same-layer neighbours (4-way or 8-way, filtered by `is_blocked`).
//! 2. Generate via transitions to every other allowed layer (filtered by
//!    `is_blocked` on the target layer).
//! 3. Apply direction-bias penalty to each neighbour cost.
//! 4. If `history_costs` is provided, add the linearised history value to the
//!    cost of each successor (PathFinder M7 integration hook).
//!
//! # History linearisation
//!
//! `index = x * (grid_height * layer_count) + y * layer_count + layer.raw() as usize`
//!
//! as specified in the plan invariants section.

use std::hash::Hash;

use autopcb_ir::handles::LayerId as IrLayerId;
use autopcb_ir::layer_stack::PreferredDirection;
use autopcb_routes::{LayerId, NetId, Point, RoutedVia, TraceSegment};
use ordered_float::OrderedFloat;
use pathfinding::directed::astar::astar;

use crate::config::MovementStyle;
use crate::workspace::{GridConfig, RoutingWorkspace};
use crate::RoutingError;

use super::astar::{direction_penalty, heuristic};
use super::via_cost::ViaCostModel;

// ---------------------------------------------------------------------------
// GridNode
// ---------------------------------------------------------------------------

/// A node in the 3D routing search space: one grid cell on one layer.
///
/// `Hash + Eq` are required by `pathfinding`'s visited-set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridNode {
    /// Grid column (x direction).
    pub x: u32,
    /// Grid row (y direction).
    pub y: u32,
    /// Copper layer.
    pub layer: LayerId,
}

// ---------------------------------------------------------------------------
// PathSegment
// ---------------------------------------------------------------------------

/// A routed segment of a path: either a same-layer move or a via transition.
///
/// Same-layer: `start.layer == end.layer`, the path traverses grid cells.
/// Via: `start.layer != end.layer`, a via is placed at `start` to reach `end`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathSegment {
    pub start: GridNode,
    pub end: GridNode,
}

// ---------------------------------------------------------------------------
// DetailedRouter trait
// ---------------------------------------------------------------------------

/// Trait implemented by all detailed routing backends (grid, shape).
///
/// `history_costs` is an optional slice for PathFinder integration (M7).
/// If `Some`, the linearized cost at each node is added during A* successor
/// expansion to guide rip-up/reroute convergence.
pub trait DetailedRouter {
    fn route_subnet(
        &self,
        workspace: &RoutingWorkspace,
        subnet: &crate::global::steiner::Subnet,
        net_id: NetId,
        history_costs: Option<&[f64]>,
    ) -> Result<Vec<PathSegment>, RoutingError>;
}

// ---------------------------------------------------------------------------
// GridRouter
// ---------------------------------------------------------------------------

/// Grid-based A* router implementing `DetailedRouter`.
///
/// Holds the via cost model and references the `RoutingConfig` movement style
/// and allowed layers via the `RoutingWorkspace`.
#[derive(Debug, Clone)]
pub struct GridRouter {
    pub via_cost: ViaCostModel,
    pub movement: MovementStyle,
}

impl GridRouter {
    pub fn new(via_cost: ViaCostModel, movement: MovementStyle) -> Self {
        GridRouter { via_cost, movement }
    }
}

impl DetailedRouter for GridRouter {
    fn route_subnet(
        &self,
        workspace: &RoutingWorkspace,
        subnet: &crate::global::steiner::Subnet,
        net_id: NetId,
        history_costs: Option<&[f64]>,
    ) -> Result<Vec<PathSegment>, RoutingError> {
        route_subnet_astar(self, workspace, subnet, net_id, history_costs)
    }
}

// ---------------------------------------------------------------------------
// Coordinate helpers
// ---------------------------------------------------------------------------

/// Convert a `routes::LayerId` to `autopcb_ir::handles::LayerId` for
/// `workspace.is_blocked()`.
fn to_ir_layer(layer: LayerId) -> IrLayerId {
    IrLayerId::from(layer.raw() as u32)
}

/// Linearize a `GridNode` position for indexing into the history-cost slice.
///
/// Formula (from plan invariants):
/// `x * (grid_height * layer_count) + y * layer_count + layer`
fn linearize(node: GridNode, grid: &GridConfig, layer_count: usize) -> usize {
    let h = grid.height_cells as usize;
    node.x as usize * (h * layer_count)
        + node.y as usize * layer_count
        + node.layer.raw() as usize
}

// ---------------------------------------------------------------------------
// Successor generation
// ---------------------------------------------------------------------------

/// Cardinal (4-way) movement deltas: right, left, up, down.
const CARDINAL: &[(i32, i32, f64)] = &[(1, 0, 1.0), (-1, 0, 1.0), (0, 1, 1.0), (0, -1, 1.0)];

/// Diagonal (8-way) movement deltas (additional 4 diagonals at cost √2).
const DIAGONAL: &[(i32, i32, f64)] = &[
    (1, 1, std::f64::consts::SQRT_2),
    (1, -1, std::f64::consts::SQRT_2),
    (-1, 1, std::f64::consts::SQRT_2),
    (-1, -1, std::f64::consts::SQRT_2),
];

/// Return the `PreferredDirection` for `layer` from the IR layer stack, or
/// `None` if the layer is not found.
fn preferred_direction(workspace: &RoutingWorkspace, layer: LayerId) -> Option<PreferredDirection> {
    // workspace doesn't hold a direct reference to the IR, but the
    // allowed_layers + direction info is in the policy. We fall through
    // to None for now; M5 will wire up layer-direction from the full IR.
    // The workspace doesn't currently expose the IR layer stack directly,
    // so we query via the policy's all_copper_layers.
    // Direction info requires the IR layer_stack — not yet on the workspace.
    // Return None (no preference) so the penalty is 1.0 everywhere.
    // TODO (M5 wiring): expose preferred_direction via policy or workspace.
    let _ = (workspace, layer);
    None
}

/// Generate successors for `node` during A*:
/// 1. Same-layer moves (4-way or 8-way).
/// 2. Via transitions to every other allowed layer.
fn successors(
    node: GridNode,
    workspace: &RoutingWorkspace,
    net_id: NetId,
    via_cost: &ViaCostModel,
    movement: MovementStyle,
    history_costs: Option<&[f64]>,
    allowed_layers: &[LayerId],
) -> Vec<(GridNode, OrderedFloat<f64>)> {
    let grid = &workspace.grid;
    let layer_count = workspace.layer_count;
    let preferred = preferred_direction(workspace, node.layer);

    let mut result = Vec::new();

    // Helper: add the history cost if provided.
    let history_cost = |n: GridNode| -> f64 {
        history_costs
            .map(|h| {
                let idx = linearize(n, grid, layer_count);
                if idx < h.len() { h[idx] } else { 0.0 }
            })
            .unwrap_or(0.0)
    };

    // --- Same-layer moves ---
    let moves: &[(i32, i32, f64)] = match movement {
        MovementStyle::FourWay => CARDINAL,
        MovementStyle::EightWay => {
            // We'll combine both; use a local buffer approach below.
            CARDINAL
        }
    };

    let process_move = |dx: i32, dy: i32, base_cost: f64, result: &mut Vec<_>| {
        let nx = node.x as i64 + dx as i64;
        let ny = node.y as i64 + dy as i64;
        if nx < 0 || ny < 0 {
            return;
        }
        let nx = nx as u32;
        let ny = ny as u32;
        if !grid.in_bounds(nx, ny) {
            return;
        }
        let neighbour = GridNode { x: nx, y: ny, layer: node.layer };
        if workspace.is_blocked(to_ir_layer(node.layer), nx, ny, Some(net_id)) {
            return;
        }
        let penalty = direction_penalty(dx, dy, preferred);
        let cost = base_cost * penalty + history_cost(neighbour);
        result.push((neighbour, OrderedFloat(cost)));
    };

    for &(dx, dy, cost) in moves {
        process_move(dx, dy, cost, &mut result);
    }

    if movement == MovementStyle::EightWay {
        for &(dx, dy, cost) in DIAGONAL {
            process_move(dx, dy, cost, &mut result);
        }
    }

    // --- Via transitions ---
    let net_class: Option<&str> = None; // net class lookup deferred to M9 wiring
    let via_c = via_cost.cost(net_class);

    for &target_layer in allowed_layers {
        if target_layer == node.layer {
            continue;
        }
        // Via placed at current (x, y) — must be unblocked on target layer.
        if workspace.is_blocked(to_ir_layer(target_layer), node.x, node.y, Some(net_id)) {
            continue;
        }
        let via_node = GridNode { x: node.x, y: node.y, layer: target_layer };
        let cost = via_c + history_cost(via_node);
        result.push((via_node, OrderedFloat(cost)));
    }

    result
}

// ---------------------------------------------------------------------------
// Core A* routing function
// ---------------------------------------------------------------------------

fn route_subnet_astar(
    router: &GridRouter,
    workspace: &RoutingWorkspace,
    subnet: &crate::global::steiner::Subnet,
    net_id: NetId,
    history_costs: Option<&[f64]>,
) -> Result<Vec<PathSegment>, RoutingError> {
    let grid = &workspace.grid;

    // Convert source/target mm coordinates to grid cells.
    let (sx, sy) = grid.to_grid(subnet.source);
    let (tx, ty) = grid.to_grid(subnet.target);

    // Determine allowed layers from policy.
    let allowed_layers = workspace.policy.allowed_layers(net_id);

    if allowed_layers.is_empty() {
        return Err(RoutingError::NoPath {
            net_id,
            reason: "no allowed layers for this net".to_string(),
        });
    }

    // Choose start/goal layers: prefer subnet hints, fall back to first allowed.
    let start_layer = subnet
        .source_layer
        .filter(|l| allowed_layers.contains(l))
        .unwrap_or(allowed_layers[0]);
    let goal_layer = subnet
        .target_layer
        .filter(|l| allowed_layers.contains(l))
        .unwrap_or(allowed_layers[0]);

    let start = GridNode { x: sx, y: sy, layer: start_layer };
    let goal = GridNode { x: tx, y: ty, layer: goal_layer };

    // Fast exit for trivial case: start == goal.
    if start == goal {
        return Ok(vec![]);
    }

    let min_via_cost = router.via_cost.cost(None);

    let result = astar(
        &start,
        |node| {
            successors(
                *node,
                workspace,
                net_id,
                &router.via_cost,
                router.movement,
                history_costs,
                &allowed_layers,
            )
        },
        |node| OrderedFloat(heuristic(*node, goal, min_via_cost)),
        |node| node.x == goal.x && node.y == goal.y && node.layer == goal.layer,
    );

    match result {
        None => Err(RoutingError::NoPath {
            net_id,
            reason: format!(
                "A* found no path from ({sx},{sy},layer {}) to ({tx},{ty},layer {})",
                start_layer.raw(),
                goal_layer.raw(),
            ),
        }),
        Some((path, _cost)) => {
            // Convert the node sequence to PathSegments.
            let segments = node_sequence_to_segments(&path);
            Ok(segments)
        }
    }
}

/// Convert a sequence of `GridNode`s returned by A* into `PathSegment`s.
///
/// Consecutive nodes are grouped into segments; each segment records its
/// start and end node.
fn node_sequence_to_segments(path: &[GridNode]) -> Vec<PathSegment> {
    if path.len() < 2 {
        return Vec::new();
    }
    path.windows(2)
        .map(|w| PathSegment { start: w[0], end: w[1] })
        .collect()
}

// ---------------------------------------------------------------------------
// route_subnet_to_traces
// ---------------------------------------------------------------------------

/// Convert a grid-space path (produced by `route_subnet`) into mm-space
/// `TraceSegment`s and `RoutedVia`s.
///
/// Consecutive same-layer segments are merged into a single `TraceSegment`.
/// Layer transitions (via nodes) become `RoutedVia` entries.
///
/// `width_mm` is applied to every `TraceSegment`.
pub fn route_subnet_to_traces(
    path_segments: &[PathSegment],
    grid: &GridConfig,
    net_id: NetId,
    width_mm: f64,
) -> (Vec<TraceSegment>, Vec<RoutedVia>) {
    let mut traces: Vec<TraceSegment> = Vec::new();
    let mut vias: Vec<RoutedVia> = Vec::new();

    // Reconstruct the full node sequence from PathSegments.
    if path_segments.is_empty() {
        return (traces, vias);
    }

    // Walk segments: detect layer transitions (vias) vs same-layer traces.
    // For efficiency we accumulate same-layer runs.
    let mut current_layer = path_segments[0].start.layer;
    let mut run_start_mm = grid.to_mm(path_segments[0].start.x, path_segments[0].start.y);

    for seg in path_segments {
        if seg.start.layer != seg.end.layer {
            // Via transition: finalise any open run, then emit a via.
            let seg_start_mm = grid.to_mm(seg.start.x, seg.start.y);
            let seg_end_mm = grid.to_mm(seg.end.x, seg.end.y);

            // Close the current trace run from run_start → via position.
            if run_start_mm.x != seg_start_mm.x || run_start_mm.y != seg_start_mm.y {
                traces.push(TraceSegment {
                    net_id,
                    layer: current_layer,
                    start: Point { x: run_start_mm.x, y: run_start_mm.y },
                    end: Point { x: seg_start_mm.x, y: seg_start_mm.y },
                    width_mm,
                });
            }

            // Emit the via.
            vias.push(RoutedVia {
                net_id,
                position: Point { x: seg_start_mm.x, y: seg_start_mm.y },
                from_layer: seg.start.layer,
                to_layer: seg.end.layer,
                drill_mm: 0.3,
                annular_ring_mm: 0.1,
            });

            current_layer = seg.end.layer;
            run_start_mm = seg_end_mm;
        }
        // else: same-layer move — continue the run.
    }

    // Emit the final trace run.
    let last = path_segments.last().unwrap();
    let last_end_mm = grid.to_mm(last.end.x, last.end.y);
    if run_start_mm.x != last_end_mm.x || run_start_mm.y != last_end_mm.y {
        traces.push(TraceSegment {
            net_id,
            layer: current_layer,
            start: Point { x: run_start_mm.x, y: run_start_mm.y },
            end: Point { x: last_end_mm.x, y: last_end_mm.y },
            width_mm,
        });
    }

    (traces, vias)
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
        layer_stack::{IrCopperLayer, IrLayerStack, PreferredDirection as IrPreferredDir},
        rule::{IrDesignRule, IrRuleParams},
        types::{BoardSide, BoundingBoxMm, PointMm},
        IrBoardGeometry,
    };
    use altium_format_types::pcb::RuleKind;
    use autopcb_routes::{LayerId, NetId};

    use crate::config::RoutingConfig;
    use crate::global::steiner::Subnet;
    use crate::workspace::build_workspace;
    use crate::detailed::via_cost::ViaCostModel;

    // -----------------------------------------------------------------------
    // Test helpers
    // -----------------------------------------------------------------------

    fn two_layer_ir(board_max: f64) -> autopcb_ir::PcbIr {
        autopcb_ir::PcbIr {
            board: IrBoardGeometry {
                outline: vec![
                    PointMm::new(0.0, 0.0),
                    PointMm::new(board_max, 0.0),
                    PointMm::new(board_max, board_max),
                    PointMm::new(0.0, board_max),
                ],
                cutouts: vec![],
                bounds: BoundingBoxMm::new(PointMm::new(0.0, 0.0), PointMm::new(board_max, board_max)),
                keepouts: vec![],
            },
            layer_stack: IrLayerStack {
                copper_layers: vec![
                    IrCopperLayer {
                        id: IrLayerId::from(0),
                        name: "Top Layer".into(),
                        is_top: true,
                        is_bottom: false,
                        preferred_direction: Some(IrPreferredDir::Any),
                    },
                    IrCopperLayer {
                        id: IrLayerId::from(1),
                        name: "Bottom Layer".into(),
                        is_top: false,
                        is_bottom: true,
                        preferred_direction: Some(IrPreferredDir::Any),
                    },
                ],
                copper_layer_count: 2,
            },
            components: IdMap::new(),
            nets: IdMap::new(),
            rules: IdMap::new(),
            free_copper: FreeCopperGeometry::default(),
            polygons: IdMap::new(),
            texts: IdMap::new(),
            regions: IdMap::new(),
            component_bodies: IdMap::new(),
        }
    }

    fn simple_config() -> RoutingConfig {
        let mut cfg = RoutingConfig::default();
        cfg.grid_resolution_mm = 1.0;
        cfg
    }

    fn make_router() -> GridRouter {
        GridRouter::new(ViaCostModel::default(), MovementStyle::FourWay)
    }

    fn make_subnet(sx: f64, sy: f64, tx: f64, ty: f64, net_id: NetId) -> Subnet {
        Subnet {
            source: PointMm::new(sx, sy),
            target: PointMm::new(tx, ty),
            net_id,
            source_layer: Some(LayerId(0)),
            target_layer: Some(LayerId(0)),
            region_path: vec![],
        }
    }

    // -----------------------------------------------------------------------
    // Straight path on empty grid
    // -----------------------------------------------------------------------

    #[test]
    fn straight_path_on_empty_grid() {
        let ir = two_layer_ir(20.0);
        let config = simple_config();
        let ws = build_workspace(&ir, &config).expect("build_workspace failed");
        let router = make_router();

        // Route from (2,2) to (7,2) on top layer — straight line.
        let net_id = NetId(0);
        let subnet = make_subnet(2.0, 2.0, 7.0, 2.0, net_id);

        let path = router
            .route_subnet(&ws, &subnet, net_id, None)
            .expect("should find a path");

        assert!(!path.is_empty(), "expected non-empty path");

        // All segments on top layer (no vias).
        for seg in &path {
            assert_eq!(seg.start.layer, LayerId(0), "all segments should be on layer 0");
            assert_eq!(seg.end.layer, LayerId(0), "all segments should end on layer 0");
        }
    }

    // -----------------------------------------------------------------------
    // Path around single obstacle
    // -----------------------------------------------------------------------

    #[test]
    fn path_around_single_obstacle() {
        let mut ir = two_layer_ir(20.0);

        // Add a pad obstacle at grid cell (5, 5) on layer 0.
        // Pad radius = 0.5mm, so at 1mm/cell resolution it blocks cell (5,5).
        let pad = IrComponentPad {
            id: PadId::from(0),
            name: "1".into(),
            local_position: PointMm::new(0.0, 0.0),
            world_position: PointMm::new(5.5, 5.5), // center of cell (5,5)
            net: None,
            shape: PadShapeInfo {
                kind: PadShapeKind::Round,
                size_x: 0.8,
                size_y: 0.8,
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
            value: "".into(),
            position: PointMm::new(5.5, 5.5),
            rotation: 0.0,
            side: BoardSide::Top,
            local_bounds: BoundingBoxMm::new(PointMm::new(-0.4, -0.4), PointMm::new(0.4, 0.4)),
            world_bounds: BoundingBoxMm::new(PointMm::new(5.1, 5.1), PointMm::new(5.9, 5.9)),
            pads: vec![pad],
        };
        ir.components.push(comp);

        let config = simple_config();
        let ws = build_workspace(&ir, &config).expect("build_workspace failed");
        let router = make_router();

        let net_id = NetId(0);
        // Route from (2,5) to (9,5) — the obstacle is at (5,5).
        let subnet = make_subnet(2.5, 5.5, 9.5, 5.5, net_id);

        let path = router
            .route_subnet(&ws, &subnet, net_id, None)
            .expect("router should find a path around the obstacle");

        assert!(!path.is_empty(), "expected non-empty path");
    }

    // -----------------------------------------------------------------------
    // Blocked path returns NoPath error
    // -----------------------------------------------------------------------

    #[test]
    fn blocked_path_returns_no_path() {
        let mut ir = two_layer_ir(10.0);

        // Add a keepout that completely blocks the path.
        ir.board.keepouts.push(autopcb_ir::IrKeepoutZone {
            outline: vec![
                PointMm::new(0.0, 4.0),
                PointMm::new(10.0, 4.0),
                PointMm::new(10.0, 7.0),
                PointMm::new(0.0, 7.0),
            ],
            layer_name: None,
        });

        let config = simple_config();
        let ws = build_workspace(&ir, &config).expect("build_workspace");
        let router = make_router();

        let net_id = NetId(0);
        // Try to route from (2,2) to (2,9) — must cross the blocked band.
        let subnet = Subnet {
            source: PointMm::new(2.0, 2.0),
            target: PointMm::new(2.0, 9.0),
            net_id,
            source_layer: Some(LayerId(0)),
            target_layer: Some(LayerId(0)),
            region_path: vec![],
        };

        let result = router.route_subnet(&ws, &subnet, net_id, None);
        assert!(
            matches!(result, Err(RoutingError::NoPath { .. })),
            "expected NoPath error for fully blocked route, got {:?}",
            result
        );
    }

    // -----------------------------------------------------------------------
    // Multi-layer route with via
    // -----------------------------------------------------------------------

    #[test]
    fn multi_layer_route_includes_via() {
        let ir = two_layer_ir(20.0);
        let config = simple_config();
        let ws = build_workspace(&ir, &config).expect("build_workspace failed");

        let via_cost = ViaCostModel {
            base: 5.0,
            si_penalty: 0.0,
            overrides: Default::default(),
        };
        let router = GridRouter::new(via_cost, MovementStyle::FourWay);

        let net_id = NetId(0);
        // Source on layer 0, target on layer 1.
        let subnet = Subnet {
            source: PointMm::new(2.0, 2.0),
            target: PointMm::new(8.0, 2.0),
            net_id,
            source_layer: Some(LayerId(0)),
            target_layer: Some(LayerId(1)),
            region_path: vec![],
        };

        let path = router
            .route_subnet(&ws, &subnet, net_id, None)
            .expect("should find multi-layer path");

        // Check that the path includes a layer transition.
        let has_via = path.iter().any(|seg| seg.start.layer != seg.end.layer);
        assert!(has_via, "multi-layer route should contain a via transition");

        // Verify the final segment ends on layer 1.
        if let Some(last) = path.last() {
            assert_eq!(
                last.end.layer,
                LayerId(1),
                "final segment should end on target layer 1"
            );
        }
    }

    // -----------------------------------------------------------------------
    // 8-way movement produces diagonal segments
    // -----------------------------------------------------------------------

    #[test]
    fn eight_way_movement_finds_diagonal_path() {
        let ir = two_layer_ir(20.0);
        let config = {
            let mut cfg = simple_config();
            cfg.movement = MovementStyle::EightWay;
            cfg
        };
        let ws = build_workspace(&ir, &config).expect("build_workspace failed");

        let via_cost = ViaCostModel::default();
        let router = GridRouter::new(via_cost, MovementStyle::EightWay);

        let net_id = NetId(0);
        // Diagonal path from (2,2) to (7,7).
        let subnet = make_subnet(2.0, 2.0, 7.0, 7.0, net_id);

        let path = router
            .route_subnet(&ws, &subnet, net_id, None)
            .expect("should find diagonal path");

        // An 8-way diagonal path should have at most 5 segments (vs 10 for 4-way L-shape).
        // The path from (2,2) to (7,7) is 5 diagonal steps in the best case.
        assert!(
            !path.is_empty(),
            "8-way router should find a path"
        );

        // Check that at least some diagonal moves exist (dx != 0 AND dy != 0).
        let has_diagonal = path.iter().any(|seg| {
            let dx = seg.end.x as i64 - seg.start.x as i64;
            let dy = seg.end.y as i64 - seg.start.y as i64;
            dx != 0 && dy != 0
        });
        assert!(
            has_diagonal,
            "8-way routing should produce diagonal segments"
        );
    }

    // -----------------------------------------------------------------------
    // route_subnet_to_traces
    // -----------------------------------------------------------------------

    #[test]
    fn route_subnet_to_traces_single_layer() {
        let bounds = BoundingBoxMm::new(PointMm::new(0.0, 0.0), PointMm::new(10.0, 10.0));
        let grid = crate::workspace::GridConfig {
            resolution_mm: 1.0,
            width_cells: 11,
            height_cells: 11,
            origin: PointMm::new(0.0, 0.0),
        };
        let net_id = NetId(1);
        let segments = vec![
            PathSegment {
                start: GridNode { x: 0, y: 0, layer: LayerId(0) },
                end: GridNode { x: 1, y: 0, layer: LayerId(0) },
            },
            PathSegment {
                start: GridNode { x: 1, y: 0, layer: LayerId(0) },
                end: GridNode { x: 2, y: 0, layer: LayerId(0) },
            },
        ];
        let (traces, vias) = route_subnet_to_traces(&segments, &grid, net_id, 0.2);
        assert!(vias.is_empty(), "no via expected for single-layer path");
        assert!(!traces.is_empty(), "expected at least one trace segment");
        // All traces on layer 0.
        for t in &traces {
            assert_eq!(t.layer, LayerId(0));
            assert_eq!(t.net_id, net_id);
            assert!((t.width_mm - 0.2).abs() < f64::EPSILON);
        }
        let _ = bounds;
    }

    #[test]
    fn route_subnet_to_traces_with_via() {
        let grid = crate::workspace::GridConfig {
            resolution_mm: 1.0,
            width_cells: 11,
            height_cells: 11,
            origin: PointMm::new(0.0, 0.0),
        };
        let net_id = NetId(2);
        let segments = vec![
            PathSegment {
                start: GridNode { x: 0, y: 0, layer: LayerId(0) },
                end: GridNode { x: 2, y: 0, layer: LayerId(0) },
            },
            // Via transition at (2,0).
            PathSegment {
                start: GridNode { x: 2, y: 0, layer: LayerId(0) },
                end: GridNode { x: 2, y: 0, layer: LayerId(1) },
            },
            PathSegment {
                start: GridNode { x: 2, y: 0, layer: LayerId(1) },
                end: GridNode { x: 4, y: 0, layer: LayerId(1) },
            },
        ];
        let (traces, vias) = route_subnet_to_traces(&segments, &grid, net_id, 0.15);
        assert_eq!(vias.len(), 1, "expected 1 via");
        assert_eq!(vias[0].from_layer, LayerId(0));
        assert_eq!(vias[0].to_layer, LayerId(1));
        assert!(!traces.is_empty(), "expected trace segments");
    }
}
