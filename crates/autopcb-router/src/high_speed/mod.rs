//! High-speed routing: differential-pair and bus routing.
//!
//! Handles coupled/semi-coupled diff-pair routing with gap and skew
//! enforcement, and provides the `BusRouter` placeholder for future parallel
//! bus routing.

pub mod bus;
pub mod diff_pair;

pub use bus::BusRouter;
pub use diff_pair::DiffPairOptimizer;

use autopcb_routes::RouteSolution;

use crate::workspace::RoutingWorkspace;
use crate::RoutingError;

/// Run high-speed post-processing passes on `solution`.
///
/// For each net that has a diff-pair partner (as determined by the workspace
/// routing policy), [`DiffPairOptimizer::optimize_pair`] is called to enforce
/// the gap constraint between the paired traces.
///
/// Matched-length group processing will be added in a future milestone.
pub fn optimize_high_speed(
    workspace: &RoutingWorkspace,
    solution: &mut RouteSolution,
) -> Result<(), RoutingError> {
    // Collect net IDs present in the solution so we can iterate without
    // holding an immutable borrow on `solution` while also mutating it.
    let net_ids: Vec<autopcb_routes::NetId> = solution.nets.keys().copied().collect();

    // Track which nets have already been processed as part of a pair so that
    // we don't process each pair twice.
    let mut processed = std::collections::HashSet::new();

    for net_id in &net_ids {
        if processed.contains(net_id) {
            continue;
        }

        if let Some(config) = workspace.policy.diff_pair_config(*net_id) {
            // Find the partner net: scan for a net whose diff_pair_config
            // points back to net_id.  For now we use a simple heuristic:
            // find the other net in the solution that also has a diff-pair
            // config.  A complete implementation would look up the partner
            // from the IR, but the IR is not available in this post-processing
            // context — the policy already encoded the pairing at workspace
            // build time.
            //
            // Heuristic: pick the next net in sorted order that also has a
            // diff-pair config and has not yet been processed.
            let partner = net_ids
                .iter()
                .find(|&&candidate| {
                    candidate != *net_id
                        && !processed.contains(&candidate)
                        && workspace.policy.diff_pair_config(candidate).is_some()
                })
                .copied();

            if let Some(partner_id) = partner {
                DiffPairOptimizer::optimize_pair(solution, *net_id, partner_id, &config);
                processed.insert(*net_id);
                processed.insert(partner_id);
            }
        }
    }

    Ok(())
}
