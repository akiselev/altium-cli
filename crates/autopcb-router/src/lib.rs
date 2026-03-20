//! `autopcb-router` — pure-algorithm PCB autorouter.
//!
//! Receives [`autopcb_ir::PcbIr`] (produced by the spec compiler) and
//! [`config::RoutingConfig`], and returns a [`autopcb_routes::RouteSolution`]
//! that is serialized to a `.routes` file and imported back into `pcbdoc-spec`.
//!
//! **Derived-state-only policy**: this crate defines NO board IR types. All
//! geometry, component, net, pad, and rule types come from `autopcb-ir`. The
//! types defined here are transient optimization structures (grids, obstacle
//! maps, congestion arrays, pathfinder state) that are built fresh from
//! `PcbIr` + `RoutingConfig` for each routing invocation and never persisted.
//!
//! **PcbDoc is never a direct input.** Everything flows through
//! `pcbdoc-spec` → `PcbIr` → `autopcb-router`.

#![allow(dead_code)]

pub mod coopt;
pub mod config;
pub mod detailed;
pub mod drc;
pub mod global;
pub mod high_speed;
pub mod obstacles;
pub mod optimize;
pub mod pathfinder;
pub mod pipeline;
pub mod rules;
pub mod solution;
pub mod spatial;
pub mod workspace;

pub use coopt::{congestion_oracle, extract_bottlenecks, Bottleneck, CongestionGrid};
pub use config::RoutingConfig;
pub use obstacles::AccessPoint;
pub use rules::{build_policy, DiffPairConfig, RoutingPolicy, ViaTemplate, WidthConstraint};
pub use workspace::{GridConfig, RoutingWorkspace};

use autopcb_ir::PcbIr;
use autopcb_routes::{NetId, RouteSolution, RoutedNet};
use thiserror::Error;

pub use solution::RouteSolutionBuilder;

/// Errors produced by the router.
#[derive(Debug, Error)]
pub enum RoutingError {
    /// Workspace construction failed (e.g. malformed IR, unsupported board geometry).
    #[error("workspace build error: {0}")]
    WorkspaceBuildError(String),

    /// General routing failure.
    #[error("routing failed: {0}")]
    RoutingFailed(String),

    /// No path could be found for a specific net.
    #[error("no path for net {net_id:?}: {reason}")]
    NoPath { net_id: NetId, reason: String },

    /// Invalid or inconsistent routing configuration.
    #[error("invalid config: {0}")]
    InvalidConfig(String),

    /// A design-rule kind was encountered that the router does not support.
    /// Fail-fast per CLAUDE.md — never silently skip unsupported rules.
    #[error("unsupported rule kind: {kind}")]
    UnsupportedRule { kind: String },
}

/// Build a [`RoutingWorkspace`] from a board IR and routing config.
///
/// The workspace encapsulates all derived state: R-tree spatial index,
/// per-layer obstacle bitmaps, coarse global routing grid, and pin access
/// points. It is built fresh for each invocation — no persistent state.
pub fn build_workspace(
    ir: &PcbIr,
    config: &RoutingConfig,
) -> Result<RoutingWorkspace, RoutingError> {
    workspace::build_workspace(ir, config)
}

/// Route a single net within an already-built workspace.
///
/// Runs the detailed A* router for all subnets of `net_id` (derived from the
/// board IR via global routing) and returns the assembled `RoutedNet` without
/// modifying other nets in the workspace.
///
/// `config` is required to parameterise the via cost model and movement style.
pub fn route_single_net(
    workspace: &RoutingWorkspace,
    ir: &PcbIr,
    config: &RoutingConfig,
    net_id: NetId,
) -> Result<RoutedNet, RoutingError> {
    use detailed::grid::{DetailedRouter, GridRouter, route_subnet_to_traces};
    use detailed::via_cost::ViaCostModel;
    use global::global_route;

    let plan = global_route(workspace, ir)?;
    let via_cost = ViaCostModel::from_config(config);
    let router = GridRouter::new(via_cost, config.movement);

    let mut all_traces = Vec::new();
    let mut all_vias = Vec::new();

    for subnet in plan.subnets.iter().filter(|s| s.net_id == net_id) {
        let segments = router.route_subnet(workspace, subnet, net_id, None)?;
        let width_mm = workspace
            .policy
            .trace_width(net_id, autopcb_routes::LayerId(0))
            .preferred;
        let (traces, vias) =
            route_subnet_to_traces(&segments, &workspace.grid, net_id, width_mm);
        all_traces.extend(traces);
        all_vias.extend(vias);
    }

    let routed_length_mm: f64 = all_traces
        .iter()
        .map(|s| {
            let dx = s.end.x - s.start.x;
            let dy = s.end.y - s.start.y;
            (dx * dx + dy * dy).sqrt()
        })
        .sum();

    Ok(RoutedNet {
        net_id,
        segments: all_traces,
        vias: all_vias,
        routed_length_mm,
    })
}

/// Route the entire board using PathFinder negotiation.
///
/// Orchestrates: global routing → detailed routing → PathFinder rip-up /
/// reroute loop → convergence check. Returns a complete [`RouteSolution`]
/// including iteration snapshots for viewer playback.
pub fn route_board(
    workspace: &RoutingWorkspace,
    ir: &PcbIr,
    config: &RoutingConfig,
) -> Result<RouteSolution, RoutingError> {
    pathfinder::pathfinder_route(workspace, ir, config)
}

/// Apply post-route optimization passes to a solution.
///
/// Passes are applied in order: staircase elimination, corner conversion,
/// and rubber-banding. Serpentine insertion for matched-length constraints is
/// available via [`optimize::serpentine::insert_serpentine`] and must be
/// triggered explicitly with per-net target-length parameters.
pub fn optimize_solution(
    workspace: &RoutingWorkspace,
    solution: &mut RouteSolution,
) -> Result<(), RoutingError> {
    optimize::optimize_solution(workspace, solution)
}
