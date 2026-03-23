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
use crate::pathfinder::history::EdgeHistoryMap;
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
///
/// `present_usage` is an optional slice of per-cell usage counts for the
/// current iteration (rebuilt from scratch each iteration).
///
/// `pres_fac` scales the present-usage congestion penalty.
///
/// `hist_weight` scales the history congestion penalty.
pub trait DetailedRouter {
    fn route_subnet(
        &self,
        workspace: &RoutingWorkspace,
        subnet: &crate::global::steiner::Subnet,
        net_id: NetId,
        history_costs: Option<&[f64]>,
        present_usage: Option<&[u16]>,
        pres_fac: f64,
        hist_weight: f64,
    ) -> Result<Vec<PathSegment>, RoutingError>;
}

// ---------------------------------------------------------------------------
// RoiBounds
// ---------------------------------------------------------------------------

/// Bounding box for Region of Interest filtering in A*.
#[derive(Debug, Clone, Copy)]
struct RoiBounds {
    min_x: u32,
    max_x: u32,
    min_y: u32,
    max_y: u32,
}

impl RoiBounds {
    /// Compute ROI from start/goal expanded by `radius`, clamped to grid bounds.
    fn from_endpoints(start: GridNode, goal: GridNode, radius: u32, grid: &GridConfig) -> Self {
        let min_x = start.x.min(goal.x).saturating_sub(radius);
        let min_y = start.y.min(goal.y).saturating_sub(radius);
        let max_x = (start.x.max(goal.x) + radius).min(grid.width_cells.saturating_sub(1));
        let max_y = (start.y.max(goal.y) + radius).min(grid.height_cells.saturating_sub(1));
        RoiBounds { min_x, max_x, min_y, max_y }
    }

    /// Returns true if (x, y) is within the ROI bounds.
    #[inline]
    fn contains(&self, x: u32, y: u32) -> bool {
        x >= self.min_x && x <= self.max_x && y >= self.min_y && y <= self.max_y
    }
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
    pub roi_initial_radius: u32,
    pub roi_retry_multiplier: u32,
}

impl GridRouter {
    pub fn new(
        via_cost: ViaCostModel,
        movement: MovementStyle,
        roi_initial_radius: u32,
        roi_retry_multiplier: u32,
    ) -> Self {
        GridRouter { via_cost, movement, roi_initial_radius, roi_retry_multiplier }
    }

    /// Route a subnet with optional corridor and congestion grid for bias.
    pub fn route_subnet_with_corridor(
        &self,
        workspace: &RoutingWorkspace,
        subnet: &crate::global::steiner::Subnet,
        net_id: NetId,
        history_costs: Option<&[f64]>,
        present_usage: Option<&[u16]>,
        pres_fac: f64,
        hist_weight: f64,
        corridor: Option<&std::collections::HashSet<u32>>,
        congestion_grid: Option<&crate::global::congestion::GlobalRoutingGrid>,
        edge_history: Option<&EdgeHistoryMap>,
    ) -> Result<Vec<PathSegment>, RoutingError> {
        route_subnet_astar_inner(
            self,
            workspace,
            subnet,
            net_id,
            history_costs,
            present_usage,
            pres_fac,
            hist_weight,
            corridor,
            congestion_grid,
            edge_history,
        )
    }
}

impl DetailedRouter for GridRouter {
    fn route_subnet(
        &self,
        workspace: &RoutingWorkspace,
        subnet: &crate::global::steiner::Subnet,
        net_id: NetId,
        history_costs: Option<&[f64]>,
        present_usage: Option<&[u16]>,
        pres_fac: f64,
        hist_weight: f64,
    ) -> Result<Vec<PathSegment>, RoutingError> {
        route_subnet_astar(
            self,
            workspace,
            subnet,
            net_id,
            history_costs,
            present_usage,
            pres_fac,
            hist_weight,
        )
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

/// Return the `PreferredDirection` for `layer` from the workspace layer directions.
fn preferred_direction(workspace: &RoutingWorkspace, layer: LayerId) -> Option<PreferredDirection> {
    let idx = layer.raw() as usize;
    workspace.layer_directions.get(idx).copied().flatten()
}

/// Combine base cost, history penalty, and present-usage penalty into a
/// single scalar edge cost.
#[inline]
fn apply_costs(base: f64, history: f64, usage: f64) -> f64 {
    base + history + usage
}

/// Generate successors for `node` during A*:
/// 1. Same-layer moves (4-way or 8-way).
/// 2. Via transitions to every other allowed layer.
#[allow(clippy::too_many_arguments)]
fn successors(
    node: GridNode,
    workspace: &RoutingWorkspace,
    net_id: NetId,
    via_cost: &ViaCostModel,
    movement: MovementStyle,
    history_costs: Option<&[f64]>,
    present_usage: Option<&[u16]>,
    pres_fac: f64,
    hist_weight: f64,
    allowed_layers: &[LayerId],
    roi: Option<&RoiBounds>,
    corridor: Option<&std::collections::HashSet<u32>>,
    congestion_grid: Option<&crate::global::congestion::GlobalRoutingGrid>,
    edge_history: Option<&EdgeHistoryMap>,
) -> Vec<(GridNode, OrderedFloat<f64>)> {
    let grid = &workspace.grid;
    let layer_count = workspace.layer_count;
    let preferred = preferred_direction(workspace, node.layer);

    let mut result = Vec::new();

    // Helper: add the weighted history cost if provided.
    let history_cost = |n: GridNode| -> f64 {
        history_costs
            .map(|h| {
                let idx = linearize(n, grid, layer_count);
                if idx < h.len() { hist_weight * h[idx] } else { 0.0 }
            })
            .unwrap_or(0.0)
    };

    // Helper: add the present-usage congestion cost if provided.
    let usage_cost = |n: GridNode| -> f64 {
        present_usage
            .map(|u| {
                let idx = linearize(n, grid, layer_count);
                if idx < u.len() {
                    pres_fac * (u[idx].saturating_sub(1)) as f64
                } else {
                    0.0
                }
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
        // ROI bounds check — skipped before is_blocked to avoid expensive spatial lookups.
        if let Some(roi) = roi {
            if !roi.contains(nx, ny) {
                return;
            }
        }
        let neighbour = GridNode { x: nx, y: ny, layer: node.layer };
        if workspace.is_blocked(to_ir_layer(node.layer), nx, ny, Some(net_id)) {
            return;
        }
        let penalty = direction_penalty(dx, dy, preferred);
        let corridor_penalty = if let (Some(corridor), Some(cg)) = (corridor, congestion_grid) {
            let coarse_cell = cg.cell_id_for_fine(nx, ny, grid);
            if corridor.contains(&coarse_cell.0) { 1.0 } else { 1.5 }
        } else {
            1.0
        };
        let history = history_cost(neighbour);
        let usage = usage_cost(neighbour);
        let edge_hist = edge_history
            .map(|eh| {
                let a = (node.x, node.y, node.layer.raw());
                let b = (nx, ny, node.layer.raw());
                let key = if a <= b { (a, b) } else { (b, a) };
                eh.get(&key)
            })
            .unwrap_or(0.0);
        let cost = apply_costs(base_cost * penalty * corridor_penalty, history, usage)
            + hist_weight * edge_hist;
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
    let net_class: Option<&str> = workspace.policy.net_class(net_id);
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
        let corridor_penalty = if let (Some(corridor), Some(cg)) = (corridor, congestion_grid) {
            let coarse_cell = cg.cell_id_for_fine(node.x, node.y, grid);
            if corridor.contains(&coarse_cell.0) { 1.0 } else { 1.5 }
        } else {
            1.0
        };
        let history = history_cost(via_node);
        let usage = usage_cost(via_node);
        let edge_hist = edge_history
            .map(|eh| {
                let a = (node.x, node.y, node.layer.raw());
                let b = (node.x, node.y, target_layer.raw());
                let key = if a <= b { (a, b) } else { (b, a) };
                eh.get(&key)
            })
            .unwrap_or(0.0);
        let cost = apply_costs(via_c * corridor_penalty, history, usage)
            + hist_weight * edge_hist;
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
    present_usage: Option<&[u16]>,
    pres_fac: f64,
    hist_weight: f64,
) -> Result<Vec<PathSegment>, RoutingError> {
    route_subnet_astar_inner(
        router,
        workspace,
        subnet,
        net_id,
        history_costs,
        present_usage,
        pres_fac,
        hist_weight,
        None,
        None,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn route_subnet_astar_inner(
    router: &GridRouter,
    workspace: &RoutingWorkspace,
    subnet: &crate::global::steiner::Subnet,
    net_id: NetId,
    history_costs: Option<&[f64]>,
    present_usage: Option<&[u16]>,
    pres_fac: f64,
    hist_weight: f64,
    corridor: Option<&std::collections::HashSet<u32>>,
    congestion_grid: Option<&crate::global::congestion::GlobalRoutingGrid>,
    edge_history: Option<&EdgeHistoryMap>,
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

    // Apply escape routing for source.
    let (sx, sy, start_layer) =
        if let Some(escape) = workspace.escape_for_position(subnet.source, start_layer) {
            let (ex, ey) = escape.via_cell;
            (ex, ey, escape.target_layer)
        } else {
            (sx, sy, start_layer)
        };

    // Apply escape routing for target.
    let (tx, ty, goal_layer) =
        if let Some(escape) = workspace.escape_for_position(subnet.target, goal_layer) {
            let (ex, ey) = escape.via_cell;
            (ex, ey, escape.target_layer)
        } else {
            (tx, ty, goal_layer)
        };

    let start = GridNode { x: sx, y: sy, layer: start_layer };
    let goal = GridNode { x: tx, y: ty, layer: goal_layer };

    // Fast exit for trivial case: start == goal.
    if start == goal {
        return Ok(vec![]);
    }

    let min_via_cost = router.via_cost.cost(None);

    // Build the ordered list of ROI radii to attempt.
    // radius=None means full grid (no ROI restriction).
    let radii: Vec<Option<u32>> = if router.roi_initial_radius > 0 {
        vec![
            Some(router.roi_initial_radius),
            Some(router.roi_initial_radius * router.roi_retry_multiplier),
            None,
        ]
    } else {
        vec![None]
    };

    for radius_opt in &radii {
        let roi = radius_opt.map(|r| RoiBounds::from_endpoints(start, goal, r, grid));
        let roi_ref = roi.as_ref();

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
                    present_usage,
                    pres_fac,
                    hist_weight,
                    &allowed_layers,
                    roi_ref,
                    corridor,
                    congestion_grid,
                    edge_history,
                )
            },
            |node| OrderedFloat(heuristic(*node, goal, min_via_cost)),
            |node| node.x == goal.x && node.y == goal.y && node.layer == goal.layer,
        );

        if let Some((path, _cost)) = result {
            return Ok(node_sequence_to_segments(&path));
        }
    }

    Err(RoutingError::NoPath {
        net_id,
        reason: format!(
            "A* found no path from ({sx},{sy},layer {}) to ({tx},{ty},layer {})",
            start_layer.raw(),
            goal_layer.raw(),
        ),
    })
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
    via_drill_mm: f64,
    via_annular_ring_mm: f64,
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
                drill_mm: via_drill_mm,
                annular_ring_mm: via_annular_ring_mm,
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
        GridRouter::new(ViaCostModel::default(), MovementStyle::FourWay, 0, 0)
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
            .route_subnet(&ws, &subnet, net_id, None, None, 1.0, 1.0)
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
            .route_subnet(&ws, &subnet, net_id, None, None, 1.0, 1.0)
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

        let result = router.route_subnet(&ws, &subnet, net_id, None, None, 1.0, 1.0);
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
        let router = GridRouter::new(via_cost, MovementStyle::FourWay, 0, 0);

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
            .route_subnet(&ws, &subnet, net_id, None, None, 1.0, 1.0)
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
        let router = GridRouter::new(via_cost, MovementStyle::EightWay, 0, 0);

        let net_id = NetId(0);
        // Diagonal path from (2,2) to (7,7).
        let subnet = make_subnet(2.0, 2.0, 7.0, 7.0, net_id);

        let path = router
            .route_subnet(&ws, &subnet, net_id, None, None, 1.0, 1.0)
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
        let (traces, vias) = route_subnet_to_traces(&segments, &grid, net_id, 0.2, 0.3, 0.1);
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

    // -----------------------------------------------------------------------
    // ROI disabled (roi_initial_radius=0) still routes correctly
    // -----------------------------------------------------------------------

    #[test]
    fn roi_disabled_routes_correctly() {
        let ir = two_layer_ir(20.0);
        let config = simple_config();
        let ws = build_workspace(&ir, &config).expect("build_workspace failed");
        // roi_initial_radius=0 disables ROI entirely — full grid used.
        let router = GridRouter::new(ViaCostModel::default(), MovementStyle::FourWay, 0, 0);

        let net_id = NetId(0);
        let subnet = make_subnet(1.0, 1.0, 15.0, 1.0, net_id);

        let path = router
            .route_subnet(&ws, &subnet, net_id, None, None, 1.0, 1.0)
            .expect("should find a path with ROI disabled");

        assert!(!path.is_empty(), "ROI-disabled router should produce a non-empty path");
    }

    // -----------------------------------------------------------------------
    // Very small ROI forces fallback to full grid
    // -----------------------------------------------------------------------

    #[test]
    fn small_roi_falls_back_to_full_grid() {
        let ir = two_layer_ir(20.0);
        let config = simple_config();
        let ws = build_workspace(&ir, &config).expect("build_workspace failed");
        // roi_initial_radius=1 is too small to span the 10-cell path; multiplier=1
        // means the second attempt is the same size; third attempt is full grid.
        let router = GridRouter::new(ViaCostModel::default(), MovementStyle::FourWay, 1, 1);

        let net_id = NetId(0);
        // Route a long path that won't fit in a radius-1 or radius-1 ROI.
        let subnet = make_subnet(0.0, 0.0, 18.0, 0.0, net_id);

        let path = router
            .route_subnet(&ws, &subnet, net_id, None, None, 1.0, 1.0)
            .expect("should find a path after falling back to full grid");

        assert!(!path.is_empty(), "fallback to full grid should produce a non-empty path");
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
        let (traces, vias) = route_subnet_to_traces(&segments, &grid, net_id, 0.15, 0.3, 0.1);
        assert_eq!(vias.len(), 1, "expected 1 via");
        assert_eq!(vias[0].from_layer, LayerId(0));
        assert_eq!(vias[0].to_layer, LayerId(1));
        assert!(!traces.is_empty(), "expected trace segments");
    }
}
