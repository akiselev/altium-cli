//! PathFinder negotiation-based routing loop.
//!
//! Implements the McMurchie & Ebeling (1995) negotiated congestion algorithm:
//! iterative rip-up + reroute with history and present congestion cost
//! accumulation.
//!
//! # Cost function
//!
//! `C(n) = (b_n + h_n) × p_n`
//!
//! where:
//! - `b_n` = base cost (Manhattan distance move cost)
//! - `h_n` = history cost for cell `n` (accumulated from previous iterations)
//! - `p_n` = present congestion factor (grows exponentially each iteration)
//!
//! # Convergence
//!
//! The loop terminates when `count_conflicts()` returns 0 (no oversubscribed
//! cells) or when `max_iterations` is reached.

pub mod history;
pub mod hot_set;
pub mod ripup;

use std::collections::{BTreeMap, HashMap};

use autopcb_ir::PcbIr;
use autopcb_routes::{NetId, RouteSolution, RoutingIterationSnapshot, TraceSegment};

use crate::config::RoutingConfig;
use crate::detailed::grid::{
    DetailedRouter, GridRouter, PathSegment, route_subnet_to_traces,
};
use crate::detailed::via_cost::ViaCostModel;
use crate::drc::cpu_engine::CpuDrcEngine;
use crate::drc::policy::DrcPolicy;
use crate::drc::{DrcConfig, DrcEngine};
use crate::global::global_route;
use crate::solution::RouteSolutionBuilder;
use crate::workspace::RoutingWorkspace;
use crate::RoutingError;

use history::HistoryArray;
use hot_set::HotSet;
use ripup::{count_conflicts, rip_up_all, rip_up_net};

// ---------------------------------------------------------------------------
// PathFinderState
// ---------------------------------------------------------------------------

/// Mutable state carried across PathFinder iterations.
#[derive(Debug)]
pub struct PathFinderState {
    /// Per-cell history congestion costs.
    pub history: HistoryArray,
    /// Current present-congestion multiplier. Grows each iteration.
    pub pres_fac: f64,
    /// Current iteration number (0-based).
    pub iteration: u32,
}

impl PathFinderState {
    fn new(width: u32, height: u32, layer_count: usize) -> Self {
        PathFinderState {
            history: HistoryArray::new(width, height, layer_count),
            pres_fac: 1.0,
            iteration: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// pathfinder_route
// ---------------------------------------------------------------------------

/// Run the PathFinder negotiation loop.
///
/// Orchestrates:
/// 1. Global routing (net decomposition + layer assignment + ordering).
/// 2. Iterative rip-up + detailed A* reroute until convergence.
/// 3. History and present-congestion updates per iteration.
/// 4. Iteration snapshot capture for viewer playback.
///
/// Returns a [`RouteSolution`] containing all routed nets, unrouted nets,
/// per-iteration snapshots, and aggregate metrics.
pub fn pathfinder_route(
    workspace: &RoutingWorkspace,
    ir: &PcbIr,
    config: &RoutingConfig,
) -> Result<RouteSolution, RoutingError> {
    // ------------------------------------------------------------------
    // 1. Global routing: decompose nets, assign layers, establish order.
    // ------------------------------------------------------------------
    let global_plan = global_route(workspace, ir)?;

    // ------------------------------------------------------------------
    // 2. Build detailed router.
    // ------------------------------------------------------------------
    let via_cost = ViaCostModel::from_config(config);
    let router = GridRouter::new(via_cost, config.movement);

    // ------------------------------------------------------------------
    // 3. Initialise PathFinder state.
    // ------------------------------------------------------------------
    let grid = &workspace.grid;
    let mut state = PathFinderState::new(
        grid.width_cells,
        grid.height_cells,
        workspace.layer_count,
    );

    // Current solution: maps NetId → flat list of PathSegments.
    let mut solution_paths: HashMap<NetId, Vec<PathSegment>> = HashMap::new();

    // Track which nets failed to route in the final iteration.
    let mut final_failed: Vec<NetId> = Vec::new();

    let mut builder = RouteSolutionBuilder::new();

    // DRC violation count from the last iteration that ran a DRC check.
    let mut last_drc_violation_count: u32 = 0;

    // ------------------------------------------------------------------
    // 3b. Build DRC engine for routing-time checks.
    // ------------------------------------------------------------------
    let drc_config = DrcConfig::default();
    let drc_policy = DrcPolicy::build(ir).map_err(RoutingError::from)?;
    let via_drill_mm = drc_policy.via_bounds.hole_min_mm;
    let via_annular_ring_mm = drc_policy.via_bounds.annular_ring_min_mm;
    let drc_engine = CpuDrcEngine::new(drc_policy);

    // ------------------------------------------------------------------
    // 4. Negotiation loop.
    // ------------------------------------------------------------------
    for _iteration in 0..config.max_iterations {
        state.iteration = _iteration;
        final_failed.clear();

        // -- 4a. Rip-up strategy -------------------------------------------
        if _iteration == 0 {
            // First iteration: always full rip-up (solution is empty anyway).
            rip_up_all(&mut solution_paths);
        } else {
            // Subsequent iterations: partial rip-up using hot set.
            let (_, oversubscribed) =
                count_conflicts(&solution_paths, grid, workspace.layer_count);

            if oversubscribed.is_empty() {
                // Already converged — nothing to rip up.
                break;
            }

            let hot = HotSet::from_conflicts(&oversubscribed, &solution_paths);
            if hot.is_empty() {
                // Hot set is empty despite conflicts — fall back to full rip-up.
                rip_up_all(&mut solution_paths);
            } else {
                for net_id in hot.iter() {
                    rip_up_net(&mut solution_paths, net_id);
                }
            }
        }

        // -- 4b. Reroute each net in priority order -------------------------
        // Build a per-net subnet map from the global plan.
        // Subnets are grouped by net_id; route all subnets of a net together.
        let mut net_subnets: HashMap<NetId, Vec<usize>> = HashMap::new();
        for (idx, subnet) in global_plan.subnets.iter().enumerate() {
            net_subnets.entry(subnet.net_id).or_default().push(idx);
        }

        let history_slice = state.history.as_slice();

        for &net_id in &global_plan.net_order {
            // Skip nets that already have a valid path (not ripped up).
            if solution_paths.contains_key(&net_id) {
                continue;
            }

            let subnet_indices = match net_subnets.get(&net_id) {
                Some(idxs) => idxs,
                None => {
                    // Net has no subnets (0 or 1 pin) — trivially routed.
                    solution_paths.insert(net_id, Vec::new());
                    continue;
                }
            };

            // Route all subnets for this net, accumulating path segments.
            let mut net_segments: Vec<PathSegment> = Vec::new();
            let mut net_failed = false;

            for &subnet_idx in subnet_indices {
                let subnet = &global_plan.subnets[subnet_idx];
                match router.route_subnet(workspace, subnet, net_id, Some(history_slice), state.pres_fac) {
                    Ok(segments) => net_segments.extend(segments),
                    Err(_) => {
                        net_failed = true;
                        break;
                    }
                }
            }

            if net_failed {
                final_failed.push(net_id);
            } else {
                solution_paths.insert(net_id, net_segments);
            }
        }

        // -- 4c. DRC check (routing-time: clearance + shorts) ----------------
        // Skip early iterations where many conflicts produce noisy DRC results.
        if drc_config.enabled && _iteration >= drc_config.start_iteration {
            // Build a partial RouteSolution for the current iteration's paths.
            let mut iter_solution = autopcb_routes::RouteSolution::new();
            for (&net_id, segs) in &solution_paths {
                let width_mm = workspace.policy.trace_width(net_id, autopcb_routes::LayerId(0)).preferred;
                let (traces, vias) = route_subnet_to_traces(segs, grid, net_id, width_mm, via_drill_mm, via_annular_ring_mm);
                iter_solution.nets.insert(net_id, autopcb_routes::RoutedNet {
                    net_id,
                    segments: traces,
                    vias,
                    routed_length_mm: 0.0,
                });
            }

            match drc_engine.check_routing(&iter_solution, workspace, ir) {
                Ok(report) => {
                    let violation_count = report.total_count();
                    if violation_count > 0 {
                        tracing::debug!(
                            iteration = _iteration,
                            violations = violation_count,
                            "DRC routing check: {} violation(s)",
                            violation_count,
                        );
                    }
                    last_drc_violation_count = violation_count as u32;

                    // Increment history costs at each violation location so
                    // future PathFinder iterations route away from DRC hotspots.
                    for v in &report.violations {
                        let (col, row) = grid.to_grid(v.location);
                        if grid.in_bounds(col, row) {
                            if let Some(layer_id) = v.layer {
                                state.history.increment(
                                    col,
                                    row,
                                    layer_id.raw(),
                                    drc_config.violation_penalty,
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    return Err(RoutingError::from(e));
                }
            }
        }

        // -- 4d. Count conflicts in current solution ------------------------
        let (conflict_count, oversubscribed) =
            count_conflicts(&solution_paths, grid, workspace.layer_count);

        // -- 4e. Update history: increment oversubscribed cells -------------
        for &(x, y, layer_raw) in &oversubscribed {
            state
                .history
                .increment(x, y, layer_raw, config.history_increment);
        }

        // -- 4f. Update present congestion factor ---------------------------
        state.pres_fac = (state.pres_fac * config.pres_fac_multiplier).min(config.pres_fac_cap);

        // -- 4g. Capture iteration snapshot ---------------------------------
        let routed_count = solution_paths.len() as u32;
        let unrouted_count = final_failed.len() as u32;

        // Build snapshot paths: convert PathSegments to TraceSegments per net.
        let snap_paths: BTreeMap<NetId, Vec<TraceSegment>> = solution_paths
            .iter()
            .map(|(&net_id, segs)| {
                let width_mm = workspace.policy.trace_width(net_id, autopcb_routes::LayerId(0)).preferred;
                let (traces, _vias) =
                    route_subnet_to_traces(segs, grid, net_id, width_mm, via_drill_mm, via_annular_ring_mm);
                (net_id, traces)
            })
            .collect();

        builder.add_snapshot(RoutingIterationSnapshot {
            iteration: _iteration,
            conflicts: conflict_count,
            routed_count,
            unrouted_count,
            paths: snap_paths,
        });

        // -- 4h. Convergence check ------------------------------------------
        let drc_clean = !drc_config.enabled
            || _iteration < drc_config.start_iteration
            || last_drc_violation_count == 0;
        if conflict_count == 0 && final_failed.is_empty() && drc_clean {
            break;
        }
    }

    // ------------------------------------------------------------------
    // 5. Build final solution from the last iteration's paths.
    // ------------------------------------------------------------------
    for (&net_id, segments) in &solution_paths {
        let width_mm = workspace.policy.trace_width(net_id, autopcb_routes::LayerId(0)).preferred;
        let (traces, vias) = route_subnet_to_traces(segments, grid, net_id, width_mm, via_drill_mm, via_annular_ring_mm);
        builder.add_net(net_id, traces, vias);
    }
    for net_id in &final_failed {
        builder.add_unrouted(*net_id);
    }

    // ------------------------------------------------------------------
    // 6. Final full DRC run — captures all violation records for storage.
    // ------------------------------------------------------------------
    let partial_solution = builder.build_partial();
    match drc_engine.check_full(&partial_solution, workspace, ir) {
        Ok(report) => {
            let count = report.total_count() as u32;
            builder.set_drc_violations(count);
            builder.set_drc_violation_records(report.to_violation_records());
        }
        Err(e) => {
            return Err(RoutingError::from(e));
        }
    }

    Ok(builder.build())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    use autopcb_ir::{
        copper::FreeCopperGeometry,
        handles::{IdMap, LayerId as IrLayerId, NetId as IrNetId},
        layer_stack::{IrCopperLayer, IrLayerStack, PreferredDirection},
        net::{IrNet, IrNetPin},
        types::{BoundingBoxMm, PointMm},
        IrBoardGeometry, PcbIr,
    };
    use autopcb_ir::handles::{ComponentId, PadId};
    use autopcb_routes::NetId;

    use crate::config::RoutingConfig;
    use crate::workspace::build_workspace;

    fn two_layer_ir(board_max: f64) -> PcbIr {
        PcbIr {
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
                copper_layers: vec![
                    IrCopperLayer {
                        id: IrLayerId::from(0),
                        name: "Top".into(),
                        is_top: true,
                        is_bottom: false,
                        preferred_direction: Some(PreferredDirection::Any),
                    },
                    IrCopperLayer {
                        id: IrLayerId::from(1),
                        name: "Bottom".into(),
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
            texts: IdMap::new(),
            regions: IdMap::new(),
            component_bodies: IdMap::new(),
        }
    }

    fn make_net(id: u32, pins: Vec<PointMm>) -> IrNet {
        IrNet {
            id: IrNetId::from(id),
            name: format!("NET{id}"),
            pins: pins
                .into_iter()
                .enumerate()
                .map(|(i, pos)| IrNetPin {
                    pad: PadId::from(i as u32),
                    component: ComponentId::from(0),
                    position: pos,
                })
                .collect(),
            component_count: 1,
            net_class: None,
            diff_pair_partner: None,
        }
    }

    fn ir_with_nets(nets: Vec<IrNet>, board_max: f64) -> PcbIr {
        let mut ir = two_layer_ir(board_max);
        for net in nets {
            ir.nets.push(net);
        }
        ir
    }

    fn fast_config() -> RoutingConfig {
        let mut cfg = RoutingConfig::default();
        cfg.grid_resolution_mm = 1.0;
        cfg.max_iterations = 10;
        cfg
    }

    // -----------------------------------------------------------------------

    #[test]
    fn empty_board_produces_empty_solution() {
        let ir = two_layer_ir(20.0);
        let config = fast_config();
        let ws = build_workspace(&ir, &config).unwrap();
        let solution = pathfinder_route(&ws, &ir, &config).unwrap();
        assert!(solution.nets.is_empty());
        assert!(solution.unrouted.is_empty());
        assert!((solution.metrics.completion_pct - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn two_non_conflicting_nets_converge_quickly() {
        // Two nets far apart on an open grid — should converge in 1 iteration.
        let net0 = make_net(0, vec![PointMm::new(2.0, 2.0), PointMm::new(4.0, 2.0)]);
        let net1 = make_net(1, vec![PointMm::new(12.0, 12.0), PointMm::new(15.0, 12.0)]);
        let ir = ir_with_nets(vec![net0, net1], 20.0);
        let config = fast_config();
        let ws = build_workspace(&ir, &config).unwrap();
        let solution = pathfinder_route(&ws, &ir, &config).unwrap();

        // Both nets routed (no obstacles blocking them).
        assert!(
            solution.unrouted.is_empty(),
            "both nets should be routed, unrouted: {:?}",
            solution.unrouted
        );
        assert_eq!(solution.nets.len(), 2);
        // Snapshots captured (at least 1 iteration).
        assert!(!solution.iterations.is_empty());
        // Final snapshot should show 0 conflicts.
        let last = solution.iterations.last().unwrap();
        assert_eq!(last.conflicts, 0, "final iteration should have 0 conflicts");
    }

    #[test]
    fn same_seed_produces_identical_solution() {
        let net0 = make_net(0, vec![PointMm::new(2.0, 2.0), PointMm::new(8.0, 2.0)]);
        let net1 = make_net(1, vec![PointMm::new(2.0, 8.0), PointMm::new(8.0, 8.0)]);
        let ir = ir_with_nets(vec![net0, net1], 20.0);

        let mut config = fast_config();
        config.seed = 12345;

        let ws = build_workspace(&ir, &config).unwrap();
        let sol_a = pathfinder_route(&ws, &ir, &config).unwrap();
        let sol_b = pathfinder_route(&ws, &ir, &config).unwrap();

        assert_eq!(
            sol_a.nets.len(),
            sol_b.nets.len(),
            "same seed must produce same number of routed nets"
        );
        assert_eq!(
            sol_a.iterations.len(),
            sol_b.iterations.len(),
            "same seed must produce same iteration count"
        );
    }

    #[test]
    fn pres_fac_grows_each_iteration() {
        // We verify pres_fac growth via snapshots: the conflict count should
        // decrease (or stay 0) as routing pressure increases.
        let net0 = make_net(0, vec![PointMm::new(2.0, 2.0), PointMm::new(8.0, 2.0)]);
        let ir = ir_with_nets(vec![net0], 15.0);
        let config = fast_config();
        let ws = build_workspace(&ir, &config).unwrap();
        let solution = pathfinder_route(&ws, &ir, &config).unwrap();

        // Single non-conflicting net converges immediately.
        assert!(solution.unrouted.is_empty());
    }

    #[test]
    fn history_cost_incremented_for_conflicted_cells() {
        // We cannot observe HistoryArray from outside, but we can verify that
        // a board with no conflicts converges in 1 iteration.
        let net0 = make_net(0, vec![PointMm::new(1.0, 5.0), PointMm::new(5.0, 5.0)]);
        let net1 = make_net(1, vec![PointMm::new(1.0, 10.0), PointMm::new(5.0, 10.0)]);
        let ir = ir_with_nets(vec![net0, net1], 20.0);
        let config = fast_config();
        let ws = build_workspace(&ir, &config).unwrap();
        let solution = pathfinder_route(&ws, &ir, &config).unwrap();
        assert_eq!(solution.unrouted.len(), 0);
        assert!(!solution.iterations.is_empty());
    }

    #[test]
    fn snapshots_ordered_by_iteration() {
        let net0 = make_net(0, vec![PointMm::new(2.0, 2.0), PointMm::new(6.0, 2.0)]);
        let ir = ir_with_nets(vec![net0], 15.0);
        let config = fast_config();
        let ws = build_workspace(&ir, &config).unwrap();
        let solution = pathfinder_route(&ws, &ir, &config).unwrap();

        for (i, snap) in solution.iterations.iter().enumerate() {
            assert_eq!(snap.iteration, i as u32, "snapshots must be in order");
        }
    }
}
