//! High-speed routing: differential-pair and bus routing.
//!
//! Handles coupled/semi-coupled diff-pair routing with gap and skew
//! enforcement, and provides the `BusRouter` placeholder for future parallel
//! bus routing.

pub mod bus;
pub mod diff_pair;

pub use bus::BusRouter;
pub use diff_pair::{CenterlineExpander, DiffPairOptimizer};

use autopcb_routes::{LayerId, RouteSolution};

use crate::workspace::RoutingWorkspace;
use crate::RoutingError;

/// Run high-speed post-processing passes on `solution`.
///
/// For each diff-pair primary net, expand its centerline path into two
/// physical traces (primary + partner) offset by the configured gap.
/// The secondary net should NOT be independently routed — it is derived
/// entirely from the primary's centerline geometry.
///
/// Uses [`CenterlineExpander::expand_pair`] for the geometry and the
/// explicit partner map in [`RoutingPolicy::diff_pair_partner`] for pairing.
pub fn optimize_high_speed(
    workspace: &RoutingWorkspace,
    solution: &mut RouteSolution,
) -> Result<(), RoutingError> {
    let net_ids: Vec<autopcb_routes::NetId> = solution.nets.keys().copied().collect();
    let mut processed = std::collections::HashSet::new();

    for &net_id in &net_ids {
        if processed.contains(&net_id) {
            continue;
        }

        // Only process primary diff-pair nets.
        if !workspace.policy.is_diff_pair_primary(net_id) {
            continue;
        }

        let partner_id = match workspace.policy.diff_pair_partner(net_id) {
            Some(p) => p,
            None => continue,
        };

        let config = match workspace.policy.diff_pair_config(net_id) {
            Some(c) => c,
            None => continue,
        };

        let width = workspace.policy.trace_width(net_id, LayerId(0)).preferred;

        // Get the primary net's centerline segments.
        let (centerline_segments, centerline_vias) = match solution.nets.get(&net_id) {
            Some(n) => (n.segments.clone(), n.vias.clone()),
            None => continue,
        };

        let (new_primary, new_partner) = CenterlineExpander::expand_pair(
            &centerline_segments,
            &centerline_vias,
            net_id,
            partner_id,
            &config,
            width,
        );

        tracing::info!(
            target: "autopcb_router::high_speed",
            primary = ?net_id,
            partner = ?partner_id,
            gap = config.gap,
            primary_segments = new_primary.segments.len(),
            partner_segments = new_partner.segments.len(),
            "expanded diff-pair centerline"
        );

        // Replace primary and insert partner.
        solution.nets.insert(net_id, new_primary);
        solution.nets.insert(partner_id, new_partner);

        // Remove partner from unrouted list if present.
        solution.unrouted.retain(|&id| id != partner_id);

        processed.insert(net_id);
        processed.insert(partner_id);
    }

    Ok(())
}
