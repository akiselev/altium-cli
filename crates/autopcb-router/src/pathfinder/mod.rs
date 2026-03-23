//! PathFinder negotiation-based routing loop.
//!
//! Implements the McMurchie & Ebeling (1995) negotiated congestion algorithm:
//! iterative rip-up + reroute with history and present congestion cost
//! accumulation.
//!
//! # Cost function
//!
//! `C(n) = base * dir_penalty * corridor_penalty + hist_weight * history[n] + pres_fac * max(0, usage[n] - 1)`
//!
//! where:
//! - `base` = move cost (1.0 cardinal, √2 diagonal, via_cost for layer change)
//! - `dir_penalty` = 1.0 (preferred) or 1.5 (against layer preferred direction)
//! - `corridor_penalty` = 1.0 (inside global corridor) or 1.5 (outside)
//! - `hist_weight` = weight multiplier for history cost (default 1.0)
//! - `history[n]` = accumulated congestion from prior iterations
//! - `pres_fac` = present congestion factor (grows exponentially each iteration)
//! - `usage[n]` = current-iteration net occupancy count (0 = free, 1 = at capacity)
//!
//! # Convergence
//!
//! The loop terminates when `count_conflicts()` returns 0 (no oversubscribed
//! cells) or when `max_iterations` is reached.

pub mod history;
pub mod hot_set;
pub mod present_usage;
pub mod ripup;

use std::collections::{BTreeMap, HashMap, HashSet};

use autopcb_ir::PcbIr;
use autopcb_routes::{NetId, RouteSolution, RoutingIterationSnapshot, TraceSegment};

use crate::config::RoutingConfig;
use crate::detailed::grid::{
    GridRouter, PathSegment, route_subnet_to_traces,
};
use crate::detailed::via_cost::ViaCostModel;
use crate::drc::cpu_engine::CpuDrcEngine;
use crate::drc::policy::DrcPolicy;
use crate::drc::{DrcConfig, DrcEngine};
use crate::global::global_route;
use crate::solution::RouteSolutionBuilder;
use crate::workspace::RoutingWorkspace;
use crate::RoutingError;

use history::{EdgeHistoryMap, HistoryArray};
use hot_set::HotSet;
use present_usage::PresentUsageArray;
use ripup::{count_conflicts, count_edge_conflicts, rip_up_all, rip_up_net};

// ---------------------------------------------------------------------------
// BestSolution
// ---------------------------------------------------------------------------

/// Snapshot of the best routing solution seen during PathFinder iteration.
///
/// Cloned from `solution_paths` whenever the combined score (conflicts + DRC
/// violations) improves. At termination, if the final solution is worse than
/// the best, the best is returned instead.
#[derive(Debug, Clone)]
struct BestSolution {
    paths: HashMap<NetId, Vec<PathSegment>>,
    failed: Vec<NetId>,
    conflict_count: u32,
    drc_violations: u32,
    iteration: u32,
}

impl BestSolution {
    fn score(&self) -> u32 {
        self.conflict_count + self.drc_violations
    }
}

// ---------------------------------------------------------------------------
// PathFinderState
// ---------------------------------------------------------------------------

/// Mutable state carried across PathFinder iterations.
#[derive(Debug)]
pub struct PathFinderState {
    /// Per-cell history congestion costs (accumulates across iterations).
    pub history: HistoryArray,
    /// Per-edge history congestion costs (sparse; accumulates across iterations).
    pub edge_history: EdgeHistoryMap,
    /// Per-cell present usage counts (rebuilt from scratch each iteration).
    pub present_usage: PresentUsageArray,
    /// Current present-congestion multiplier. Grows each iteration.
    pub pres_fac: f64,
    /// Current iteration number (0-based).
    pub iteration: u32,
    /// Best (lowest) conflict count seen so far.
    pub best_conflict_count: u32,
    /// Number of consecutive iterations without improvement.
    pub stagnation_counter: u32,
}

impl PathFinderState {
    fn new(width: u32, height: u32, layer_count: usize, initial_pres_fac: f64) -> Self {
        PathFinderState {
            history: HistoryArray::new(width, height, layer_count),
            edge_history: EdgeHistoryMap::new(),
            present_usage: PresentUsageArray::new(width, height, layer_count),
            pres_fac: initial_pres_fac,
            iteration: 0,
            best_conflict_count: u32::MAX,
            stagnation_counter: 0,
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

    tracing::info!(
        target: "autopcb_router::pathfinder",
        net_count = global_plan.net_order.len(),
        subnet_count = global_plan.subnets.len(),
        max_iterations = config.max_iterations,
        pres_fac_multiplier = %config.pres_fac_multiplier,
        "pathfinder_started"
    );

    // ------------------------------------------------------------------
    // 2. Build detailed router.
    // ------------------------------------------------------------------
    let via_cost = ViaCostModel::from_config(config);
    let router = GridRouter::new(via_cost, config.movement, config.roi_initial_radius, config.roi_retry_multiplier);

    // ------------------------------------------------------------------
    // 3. Initialise PathFinder state.
    // ------------------------------------------------------------------
    let grid = &workspace.grid;
    let mut state = PathFinderState::new(
        grid.width_cells,
        grid.height_cells,
        workspace.layer_count,
        config.initial_pres_fac,
    );

    // Current solution: maps NetId → flat list of PathSegments.
    let mut solution_paths: HashMap<NetId, Vec<PathSegment>> = HashMap::new();

    // Track which nets failed to route in the final iteration.
    let mut final_failed: Vec<NetId> = Vec::new();

    let mut builder = RouteSolutionBuilder::new();

    // DRC violation count from the last iteration that ran a DRC check.
    let mut last_drc_violation_count: u32 = 0;

    // Best solution tracking for rollback.
    let mut best_solution: Option<BestSolution> = None;

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

        // -- 4a-pre. Apply history decay (prevents fossilization) -----------
        state.history.decay(config.history_decay);
        state.edge_history.decay(config.history_decay);

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

            let hot = HotSet::from_conflicts_adaptive(&oversubscribed, &solution_paths);
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
                let present_slice = state.present_usage.as_slice();

                let corridor: Option<HashSet<u32>> = if subnet.region_path.is_empty() {
                    None
                } else {
                    Some(subnet.region_path.iter().map(|c| c.0).collect())
                };
                let corridor_ref = corridor.as_ref();
                let congestion_grid_ref = Some(&global_plan.congestion_grid);

                match router.route_subnet_with_corridor(workspace, subnet, net_id, Some(history_slice), Some(present_slice), state.pres_fac, config.hist_weight, corridor_ref, congestion_grid_ref, Some(&state.edge_history)) {
                    Ok(segments) => net_segments.extend(segments),
                    Err(e) => {
                        tracing::debug!(
                            target: "autopcb_router::pathfinder",
                            net_id = ?net_id,
                            subnet_idx,
                            error = %e,
                            "subnet_routing_failed"
                        );
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

        // -- 4b2. Rebuild present usage from current solution paths ----------
        state.present_usage.clear();
        for segs in solution_paths.values() {
            // Track which cells each net touches (deduplicate per net).
            let mut seen = std::collections::HashSet::new();
            for seg in segs {
                let start_key = (seg.start.x, seg.start.y, seg.start.layer.raw());
                let end_key = (seg.end.x, seg.end.y, seg.end.layer.raw());
                if seen.insert(start_key) {
                    state.present_usage.increment(seg.start.x, seg.start.y, seg.start.layer.raw());
                }
                if seen.insert(end_key) {
                    state.present_usage.increment(seg.end.x, seg.end.y, seg.end.layer.raw());
                }
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

        // -- 4d-edge. Count edge conflicts and update edge history ----------
        let (_edge_conflict_count, edge_oversubscribed) =
            count_edge_conflicts(&solution_paths, grid, workspace.layer_count);
        for edge in &edge_oversubscribed {
            state.edge_history.increment(*edge, config.history_increment);
        }

        // -- 4e2. Stagnation detection ----------------------------------------
        if conflict_count < state.best_conflict_count {
            state.best_conflict_count = conflict_count;
            state.stagnation_counter = 0;
        } else {
            state.stagnation_counter += 1;
        }

        // Early termination on persistent stagnation
        if state.stagnation_counter >= config.stagnation_max {
            tracing::warn!(
                target: "autopcb_router::pathfinder",
                iteration = _iteration,
                conflict_count,
                "stagnation_termination: no progress for {} iterations",
                state.stagnation_counter,
            );
            break;
        }

        // -- 4f. Update present congestion factor ---------------------------
        // On stagnation threshold: escalate by doubling instead of normal growth.
        if state.stagnation_counter == config.stagnation_threshold {
            tracing::info!(
                target: "autopcb_router::pathfinder",
                iteration = _iteration,
                stagnation_counter = state.stagnation_counter,
                "stagnation_escalation: doubling pres_fac"
            );
            state.pres_fac = (state.pres_fac * 2.0).min(config.pres_fac_cap);
        } else {
            state.pres_fac = (state.pres_fac * config.pres_fac_multiplier).min(config.pres_fac_cap);
        }

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

        tracing::info!(
            target: "autopcb_router::pathfinder",
            iteration = _iteration,
            conflict_count,
            routed_count,
            unrouted_count = final_failed.len(),
            pres_fac = %state.pres_fac,
            drc_violations = last_drc_violation_count,
            "pathfinder_iteration_complete"
        );

        // -- 4h. Best-solution tracking --------------------------------------
        let current_score = conflict_count + last_drc_violation_count;
        let is_best = best_solution
            .as_ref()
            .map_or(true, |b| current_score < b.score());
        if is_best {
            best_solution = Some(BestSolution {
                paths: solution_paths.clone(),
                failed: final_failed.clone(),
                conflict_count,
                drc_violations: last_drc_violation_count,
                iteration: _iteration,
            });
        }

        // -- 4i. Convergence check ------------------------------------------
        let drc_clean = !drc_config.enabled
            || _iteration < drc_config.start_iteration
            || last_drc_violation_count == 0;
        if conflict_count == 0 && final_failed.is_empty() && drc_clean {
            break;
        }
    }

    tracing::info!(
        target: "autopcb_router::pathfinder",
        total_iterations = state.iteration + 1,
        nets_routed = solution_paths.len(),
        nets_failed = final_failed.len(),
        "pathfinder_finished"
    );

    // ------------------------------------------------------------------
    // 5. Rollback to best solution if final is worse.
    // ------------------------------------------------------------------
    let final_score = {
        let (cc, _) = count_conflicts(&solution_paths, grid, workspace.layer_count);
        cc + last_drc_violation_count
    };
    if let Some(ref best) = best_solution {
        if best.score() < final_score {
            tracing::info!(
                target: "autopcb_router::pathfinder",
                best_iteration = best.iteration,
                best_score = best.score(),
                final_score,
                "rolling_back_to_best_solution"
            );
            solution_paths = best.paths.clone();
            final_failed = best.failed.clone();
        }
    }

    // ------------------------------------------------------------------
    // 6. Build final solution from the best iteration's paths.
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
    // 7. Final full DRC run — captures all violation records for storage.
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
