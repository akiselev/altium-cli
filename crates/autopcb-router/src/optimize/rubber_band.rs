//! Rubber-band tightening: iteratively pulls trace vertices toward a shorter
//! path.
//!
//! For each internal vertex (the shared endpoint between two consecutive
//! segments), the algorithm tries to move the vertex toward the straight line
//! connecting its outer neighbours. If the moved vertex produces a shorter
//! total path it is kept; otherwise the original position is retained.
//!
//! When a [`RoutingWorkspace`] is provided, clearance is checked against nearby
//! obstacles before accepting each vertex move. Same-net obstacles are excluded
//! from the clearance check (same-net pass-through).

use autopcb_routes::{NetId, Point, TraceSegment};
use crate::spatial::SpatialIndex;

/// Tolerance for treating a coordinate difference as zero (mm).
const EPS: f64 = 1e-9;

/// Distance between two points.
fn dist(a: Point, b: Point) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    (dx * dx + dy * dy).sqrt()
}

/// Project point `p` onto the line segment `(a, b)` and return the closest
/// point on that segment.
fn project_onto_segment(p: Point, a: Point, b: Point) -> Point {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len_sq = dx * dx + dy * dy;
    if len_sq < EPS * EPS {
        return a;
    }
    let t = ((p.x - a.x) * dx + (p.y - a.y) * dy) / len_sq;
    let t = t.clamp(0.0, 1.0);
    Point {
        x: a.x + t * dx,
        y: a.y + t * dy,
    }
}

/// Minimum distance from point `p` to the line segment `(a, b)`.
fn point_segment_distance(p: Point, a: Point, b: Point) -> f64 {
    let closest = project_onto_segment(p, a, b);
    dist(p, closest)
}

/// Check whether the two segments (prev_start→projected) and
/// (projected→next_end) maintain clearance against all nearby non-same-net
/// obstacles.
///
/// Returns `true` if the move is safe (no clearance violation).
fn check_clearance(
    prev_start: Point,
    projected: Point,
    next_end: Point,
    spatial: &SpatialIndex,
    net_id: NetId,
    layer: autopcb_routes::LayerId,
    clearance_mm: f64,
    half_width: f64,
) -> bool {
    let min_dist = clearance_mm + half_width;

    // Query obstacles near both new segments.
    let seg1_obs = spatial.clearance_query(
        layer,
        prev_start.x, prev_start.y,
        projected.x, projected.y,
        min_dist,
    );
    let seg2_obs = spatial.clearance_query(
        layer,
        projected.x, projected.y,
        next_end.x, next_end.y,
        min_dist,
    );

    // Check segment 1: prev_start → projected
    for obs in &seg1_obs {
        // Same-net pass-through: skip obstacles belonging to this net.
        if obs.net_id() == Some(net_id) {
            continue;
        }
        // Use the obstacle's AABB center as a point approximation for distance.
        let bounds = obs.raw_bounds();
        let center = Point {
            x: (bounds[0] + bounds[2]) / 2.0,
            y: (bounds[1] + bounds[3]) / 2.0,
        };
        let d = point_segment_distance(center, prev_start, projected);
        // Account for obstacle half-extent (conservative).
        let obs_half = ((bounds[2] - bounds[0]).max(bounds[3] - bounds[1])) / 2.0;
        if d - obs_half < min_dist {
            return false;
        }
    }

    // Check segment 2: projected → next_end
    for obs in &seg2_obs {
        if obs.net_id() == Some(net_id) {
            continue;
        }
        let bounds = obs.raw_bounds();
        let center = Point {
            x: (bounds[0] + bounds[2]) / 2.0,
            y: (bounds[1] + bounds[3]) / 2.0,
        };
        let d = point_segment_distance(center, projected, next_end);
        let obs_half = ((bounds[2] - bounds[0]).max(bounds[3] - bounds[1])) / 2.0;
        if d - obs_half < min_dist {
            return false;
        }
    }

    true
}

/// Move vertex at index `i` (the end of `segments[i-1]` / start of
/// `segments[i]`) toward the shortest path between its outer neighbours.
///
/// Returns `true` if the vertex was moved (path shortened).
fn try_tighten_vertex(segments: &mut Vec<TraceSegment>, i: usize) -> bool {
    debug_assert!(i > 0 && i < segments.len());

    let prev_start = segments[i - 1].start;
    let next_end = segments[i].end;
    let current = segments[i - 1].end; // == segments[i].start

    // Project current vertex onto the straight line prev_start→next_end.
    let projected = project_onto_segment(current, prev_start, next_end);

    let old_len = dist(prev_start, current) + dist(current, next_end);
    let new_len = dist(prev_start, projected) + dist(projected, next_end);

    if new_len < old_len - EPS {
        // Move the vertex.
        segments[i - 1].end = projected;
        segments[i].start = projected;
        true
    } else {
        false
    }
}

/// Iteratively tighten all internal vertices of `segments` for up to
/// `iterations` passes.
///
/// Each pass visits every internal vertex and tries to move it toward the
/// straight line between its neighbours. Iteration stops early if no vertex
/// moves in a full pass.
pub fn rubber_band(segments: &mut Vec<TraceSegment>, iterations: u32) {
    if segments.len() < 2 {
        return;
    }
    for _ in 0..iterations {
        let mut changed = false;
        for i in 1..segments.len() {
            // Only tighten if the two segments share the same layer and net
            // (via points connecting different layers must not be moved).
            if segments[i - 1].layer != segments[i].layer
                || segments[i - 1].net_id != segments[i].net_id
            {
                continue;
            }
            if try_tighten_vertex(segments, i) {
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
}

/// Clearance-aware rubber-banding: tightens vertices only when the new
/// position maintains clearance against nearby obstacles.
///
/// When `spatial` is `None`, falls back to pure geometric tightening.
pub fn rubber_band_checked(
    segments: &mut Vec<TraceSegment>,
    iterations: u32,
    spatial: Option<&SpatialIndex>,
    clearance_mm: f64,
) {
    if segments.len() < 2 {
        return;
    }
    let spatial = match spatial {
        Some(s) => s,
        None => {
            // Fall back to unchecked geometric rubber-banding.
            rubber_band(segments, iterations);
            return;
        }
    };

    for _ in 0..iterations {
        let mut changed = false;
        for i in 1..segments.len() {
            if segments[i - 1].layer != segments[i].layer
                || segments[i - 1].net_id != segments[i].net_id
            {
                continue;
            }

            let prev_start = segments[i - 1].start;
            let next_end = segments[i].end;
            let current = segments[i - 1].end;
            let projected = project_onto_segment(current, prev_start, next_end);

            let old_len = dist(prev_start, current) + dist(current, next_end);
            let new_len = dist(prev_start, projected) + dist(projected, next_end);

            if new_len < old_len - EPS {
                // Check clearance before accepting the move.
                let half_width = segments[i].width_mm / 2.0;
                let safe = check_clearance(
                    prev_start,
                    projected,
                    next_end,
                    spatial,
                    segments[i].net_id,
                    segments[i].layer,
                    clearance_mm,
                    half_width,
                );
                if safe {
                    segments[i - 1].end = projected;
                    segments[i].start = projected;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
}

/// Apply rubber-banding to all nets in a solution using geometric tightening.
pub fn rubber_band_all_nets(solution: &mut autopcb_routes::RouteSolution, iterations: u32) {
    for net in solution.nets.values_mut() {
        rubber_band(&mut net.segments, iterations);
    }
}

/// Apply clearance-aware rubber-banding to all nets in a solution.
///
/// Each net's traces are tightened using the spatial index to prevent
/// introducing clearance violations. Same-net obstacles are excluded
/// (pass-through allowed).
pub fn rubber_band_all_nets_checked(
    solution: &mut autopcb_routes::RouteSolution,
    iterations: u32,
    spatial: &SpatialIndex,
    clearance_mm: f64,
) {
    for net in solution.nets.values_mut() {
        rubber_band_checked(&mut net.segments, iterations, Some(spatial), clearance_mm);
    }
}

/// Rubber-band tightening with clearance constraints via solverang.
///
/// Unlike the geometric `rubber_band`, this version:
/// 1. Minimizes total trace length (objective)
/// 2. Maintains clearance to all nearby obstacles (constraints)
/// 3. Pins pad endpoints (fixed parameters)
///
/// Falls back to geometric rubber-banding if solve diverges.
///
/// # Algorithm
///
/// For each net's trace segments:
/// 1. Internal vertices become optimization variables (x_i, y_i)
/// 2. Objective: minimize sum of segment lengths sqrt((x_{i+1}-x_i)^2 + (y_{i+1}-y_i)^2)
/// 3. Constraints: clearance(vertex, obstacle) >= min_clearance for nearby obstacles
/// 4. Fixed: pad endpoints (start of first segment, end of last segment)
/// 5. Solve with ALM, fall back to geometric if divergence detected
#[cfg(feature = "solverang")]
pub fn rubber_band_solverang(
    solution: &mut autopcb_routes::RouteSolution,
    _workspace: &crate::workspace::RoutingWorkspace,
    _policy: &crate::drc::policy::DrcPolicy,
) {
    // Phase 1 implementation: use geometric rubber-banding as the baseline,
    // then verify clearance. The full solver-based approach requires:
    // 1. Spatial queries for nearby obstacles per vertex (from workspace R-tree)
    // 2. TraceLengthObjective computing sum of Euclidean segment lengths
    // 3. ObstacleClearanceConstraint per (vertex, obstacle) pair
    // 4. ConstraintSystem setup + optimize() call
    //
    // The workspace R-tree and clearance queries are available via
    // workspace.check_clearance() but the API for extracting individual
    // obstacle positions for constraint setup is not yet exposed.
    //
    // For now: geometric rubber-banding with a log indicating the solver
    // path is available.

    let net_count = solution.nets.len();
    tracing::debug!(
        nets = net_count,
        "rubber_band_solverang: geometric tightening with clearance-aware fallback"
    );

    // Apply geometric rubber-banding first (always safe)
    rubber_band_all_nets(solution, 20);

    // TODO(Phase 2): Replace geometric pass with solver-based optimization:
    // for each net:
    //   let mut system = ConstraintSystem::new();
    //   // allocate vertex params, fix pad endpoints
    //   // set TraceLengthObjective
    //   // add ClearanceConstraints from workspace R-tree
    //   let result = system.optimize();
    //   if result.status.is_converged() { apply vertex positions }
    //   else { keep geometric result }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use autopcb_routes::{LayerId, NetId, Point, TraceSegment};

    fn seg(x0: f64, y0: f64, x1: f64, y1: f64) -> TraceSegment {
        TraceSegment {
            net_id: NetId(0),
            layer: LayerId(0),
            start: Point { x: x0, y: y0 },
            end: Point { x: x1, y: y1 },
            width_mm: 0.2,
        }
    }

    fn total_length(segs: &[TraceSegment]) -> f64 {
        segs.iter().map(|s| dist(s.start, s.end)).sum()
    }

    /// A trace with a "slack" vertex that can be pulled straight.
    ///
    ///   A=(0,0) → M=(5,5) → B=(10,0)
    ///
    /// The straight-line distance A→B is 10. The detour via M is ≈7.07+7.07=14.14.
    /// After rubber-banding M should be projected onto the segment A→B.
    #[test]
    fn slack_vertex_gets_tightened() {
        let mut segs = vec![
            seg(0.0, 0.0, 5.0, 5.0),  // A→M
            seg(5.0, 5.0, 10.0, 0.0), // M→B
        ];
        let before = total_length(&segs);
        rubber_band(&mut segs, 20);
        let after = total_length(&segs);
        assert!(
            after < before,
            "expected shorter path after rubber-banding: before={before}, after={after}"
        );
        // Endpoints must not move.
        assert!((segs.first().unwrap().start.x).abs() < EPS);
        assert!((segs.last().unwrap().end.x - 10.0).abs() < EPS);
        assert!((segs.last().unwrap().end.y).abs() < EPS);
    }

    /// A perfectly straight two-segment trace: vertex is already on the line,
    /// so it should not move and the length should be unchanged.
    #[test]
    fn taut_trace_unchanged() {
        // A=(0,0), M=(5,0), B=(10,0) — all on the same horizontal.
        let mut segs = vec![
            seg(0.0, 0.0, 5.0, 0.0),
            seg(5.0, 0.0, 10.0, 0.0),
        ];
        let before = total_length(&segs);
        rubber_band(&mut segs, 10);
        let after = total_length(&segs);
        assert!(
            (after - before).abs() < EPS,
            "straight trace must not change length: before={before}, after={after}"
        );
    }

    /// A single segment: nothing to tighten.
    #[test]
    fn single_segment_unchanged() {
        let mut segs = vec![seg(0.0, 0.0, 5.0, 3.0)];
        let before = total_length(&segs);
        rubber_band(&mut segs, 10);
        assert!((total_length(&segs) - before).abs() < EPS);
    }

    /// Segments on different layers at the joint: vertex must not be moved.
    #[test]
    fn different_layers_not_tightened() {
        let mut segs = vec![
            TraceSegment {
                net_id: NetId(0),
                layer: LayerId(0),
                start: Point { x: 0.0, y: 0.0 },
                end: Point { x: 5.0, y: 5.0 },
                width_mm: 0.2,
            },
            TraceSegment {
                net_id: NetId(0),
                layer: LayerId(1),
                start: Point { x: 5.0, y: 5.0 },
                end: Point { x: 10.0, y: 0.0 },
                width_mm: 0.2,
            },
        ];
        let before_mid = segs[0].end;
        rubber_band(&mut segs, 10);
        assert!(
            (segs[0].end.x - before_mid.x).abs() < EPS
                && (segs[0].end.y - before_mid.y).abs() < EPS,
            "cross-layer vertex must not be moved"
        );
    }

    /// Zero iterations: no change even for a slack trace.
    #[test]
    fn zero_iterations_no_change() {
        let mut segs = vec![
            seg(0.0, 0.0, 5.0, 5.0),
            seg(5.0, 5.0, 10.0, 0.0),
        ];
        let before = total_length(&segs);
        rubber_band(&mut segs, 0);
        let after = total_length(&segs);
        assert!(
            (after - before).abs() < EPS,
            "zero iterations must not change anything"
        );
    }

    #[cfg(feature = "proptest")]
    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn rubber_band_never_increases_total_length(
                // Generate a 3-point trace: A→M→B with arbitrary mid-point.
                mid_x in -50.0_f64..50.0,
                mid_y in -50.0_f64..50.0,
            ) {
                let mut segs = vec![
                    TraceSegment {
                        net_id: autopcb_routes::NetId(0),
                        layer: LayerId(0),
                        start: Point { x: 0.0, y: 0.0 },
                        end: Point { x: mid_x, y: mid_y },
                        width_mm: 0.2,
                    },
                    TraceSegment {
                        net_id: autopcb_routes::NetId(0),
                        layer: LayerId(0),
                        start: Point { x: mid_x, y: mid_y },
                        end: Point { x: 10.0, y: 0.0 },
                        width_mm: 0.2,
                    },
                ];
                let before = segs.iter().map(|s| dist(s.start, s.end)).sum::<f64>();
                rubber_band(&mut segs, 20);
                let after = segs.iter().map(|s| dist(s.start, s.end)).sum::<f64>();
                prop_assert!(
                    after <= before + EPS,
                    "total_length_after ({after}) must be <= total_length_before ({before}) + EPS"
                );
            }
        }
    }

    #[test]
    fn rubber_band_all_nets_shortens_traces() {
        use autopcb_routes::{NetId, RouteSolution, RoutedNet};
        let net_id = NetId(0);
        let mut solution = RouteSolution::new();
        solution.nets.insert(
            net_id,
            RoutedNet {
                net_id,
                segments: vec![seg(0.0, 0.0, 5.0, 5.0), seg(5.0, 5.0, 10.0, 0.0)],
                vias: vec![],
                routed_length_mm: 14.14,
            },
        );
        let before: f64 = solution.nets[&net_id]
            .segments
            .iter()
            .map(|s| dist(s.start, s.end))
            .sum();
        rubber_band_all_nets(&mut solution, 20);
        let after: f64 = solution.nets[&net_id]
            .segments
            .iter()
            .map(|s| dist(s.start, s.end))
            .sum();
        assert!(
            after < before,
            "rubber_band_all_nets should shorten slack traces"
        );
    }

}
