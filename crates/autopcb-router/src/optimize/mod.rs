//! Post-route trace optimization passes.
//!
//! Applied in order after PathFinder convergence:
//!
//! 1. **Colinear merge** — consecutive same-direction segments collapsed into one.
//! 2. **Staircase elimination** — consecutive H-V bends replaced by diagonals.
//! 3. **Pull-tight** — multi-segment bypass toward shorter octilinear paths.
//! 4. **Corner conversion** — right-angle bends converted to 45° chamfers or
//!    rounded approximations per the per-net `CornerStyle`.
//! 5. **Final merge** — clean up any colinear artifacts from earlier passes.
//!
//! Serpentine insertion is available as a standalone pass via
//! [`serpentine::insert_serpentine`] and is not run automatically because it
//! requires per-net target-length annotations from the routing spec.

pub mod corners;
pub mod merge;
pub mod pull_tight;
pub mod rubber_band;
pub mod serpentine;
pub mod staircase;

use autopcb_routes::{NetId, RouteSolution};

use crate::workspace::RoutingWorkspace;
use crate::RoutingError;

/// Run all post-route optimization passes on `solution` in order.
///
/// Passes applied (per-net):
/// 1. [`merge::merge_colinear`]
/// 2. [`staircase::eliminate_staircases`]
/// 3. [`pull_tight::pull_tight_checked`]
/// 4. [`corners::convert_corners`] — style taken from workspace policy
/// 5. [`merge::merge_colinear`] — final cleanup
pub fn optimize_solution(
    workspace: &RoutingWorkspace,
    solution: &mut RouteSolution,
) -> Result<(), RoutingError> {
    for (net_id, routed_net) in solution.nets.iter_mut() {
        let segs = &mut routed_net.segments;
        let clearance_mm = workspace.policy.clearance(*net_id, NetId(u32::MAX));

        // Pass 1: merge colinear (clean PathFinder artifacts).
        merge::merge_colinear(segs);

        // Pass 2: staircase elimination.
        staircase::eliminate_staircases(segs);

        // Pass 3: pull-tight (replaces rubber_band).
        pull_tight::pull_tight_checked(
            segs,
            &workspace.spatial_index,
            *net_id,
            clearance_mm,
        );

        // Pass 4: corner-style conversion.
        let style = workspace.policy.corner_style(*net_id);
        corners::convert_corners(segs, style);

        // Pass 5: final merge (clean up corner/pull-tight artifacts).
        merge::merge_colinear(segs);
    }

    Ok(())
}
