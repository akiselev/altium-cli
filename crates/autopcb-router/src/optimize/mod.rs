//! Post-route trace optimization passes.
//!
//! Applied in order after PathFinder convergence:
//!
//! 1. **Staircase elimination** — consecutive H-V bends replaced by diagonals.
//! 2. **Corner conversion** — right-angle bends converted to 45° chamfers or
//!    rounded approximations per the per-net `CornerStyle`.
//! 3. **Rubber-banding** — internal vertices pulled toward a shorter path.
//!
//! Serpentine insertion is available as a standalone pass via
//! [`serpentine::insert_serpentine`] and is not run automatically because it
//! requires per-net target-length annotations from the routing spec.

pub mod corners;
pub mod rubber_band;
pub mod serpentine;
pub mod staircase;

use autopcb_routes::RouteSolution;

use crate::workspace::RoutingWorkspace;
use crate::RoutingError;

/// Default number of rubber-band iterations applied per net.
const RUBBER_BAND_ITERATIONS: u32 = 5;

/// Run all post-route optimization passes on `solution` in order.
///
/// Passes applied (per-net):
/// 1. [`staircase::eliminate_staircases`]
/// 2. [`corners::convert_corners`] — style taken from workspace policy
/// 3. [`rubber_band::rubber_band`]
pub fn optimize_solution(
    workspace: &RoutingWorkspace,
    solution: &mut RouteSolution,
) -> Result<(), RoutingError> {
    for (net_id, routed_net) in solution.nets.iter_mut() {
        let segs = &mut routed_net.segments;

        // Pass 1: staircase elimination.
        staircase::eliminate_staircases(segs);

        // Pass 2: corner-style conversion.
        let style = workspace.policy.corner_style(*net_id);
        corners::convert_corners(segs, style);

        // Pass 3: rubber-banding.
        rubber_band::rubber_band(segs, RUBBER_BAND_ITERATIONS);
    }

    Ok(())
}
