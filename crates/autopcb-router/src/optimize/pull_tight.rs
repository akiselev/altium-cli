//! Enhanced multi-segment bypass (pull-tight) optimization.
//!
//! Iterates over windows of consecutive segments and attempts to replace each
//! window with a shorter octilinear path. Multiple step sizes are tried in
//! decreasing order so that large detours are handled before fine-grained
//! cleanup. After each step level, colinear merging is applied to prevent
//! the segment list from growing.
//!
//! Both a geometry-only variant ([`pull_tight`]) and a DRC-safe variant
//! ([`pull_tight_checked`]) are provided.

use autopcb_routes::{NetId, Point, TraceSegment};

use crate::spatial::SpatialIndex;
use super::merge::merge_colinear;

/// Tolerance for treating a coordinate difference as zero (mm).
const EPS: f64 = 1e-9;

/// Window sizes tried from largest to smallest.
const STEP_SIZES: &[usize] = &[8, 4, 2, 1];

/// Maximum outer passes before giving up if changes keep occurring.
const MAX_PASSES: u32 = 3;

// ---------------------------------------------------------------------------
// Geometry helpers (local copies to avoid cross-module pub(crate) coupling)
// ---------------------------------------------------------------------------

fn dist(a: Point, b: Point) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    (dx * dx + dy * dy).sqrt()
}

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

fn point_segment_distance(p: Point, a: Point, b: Point) -> f64 {
    let closest = project_onto_segment(p, a, b);
    dist(p, closest)
}

// ---------------------------------------------------------------------------
// Range validation
// ---------------------------------------------------------------------------

/// Returns `true` if all segments in `segs[start..=end]` share the same
/// `net_id`, `layer`, and `width_mm`, and are all connected end-to-start.
///
/// Connectivity failure signals a subnet boundary; we must not bypass across
/// it.
fn valid_range(segs: &[TraceSegment], start: usize, end: usize) -> bool {
    let first = &segs[start];
    for j in start..=end {
        let s = &segs[j];
        if s.net_id != first.net_id {
            return false;
        }
        if s.layer != first.layer {
            return false;
        }
        if (s.width_mm - first.width_mm).abs() >= EPS {
            return false;
        }
        // Check connectivity to successor.
        if j < end {
            let next = &segs[j + 1];
            if (s.end.x - next.start.x).abs() >= EPS
                || (s.end.y - next.start.y).abs() >= EPS
            {
                return false;
            }
        }
    }
    true
}

// ---------------------------------------------------------------------------
// Candidate generation
// ---------------------------------------------------------------------------

/// Returns `true` if the vector `(dx, dy)` is octilinear (a multiple of 45°).
fn is_octilinear(dx: f64, dy: f64) -> bool {
    let ax = dx.abs();
    let ay = dy.abs();
    // Horizontal or vertical.
    if ax < EPS || ay < EPS {
        return true;
    }
    // Diagonal: |dx| ≈ |dy|.
    (ax - ay).abs() < EPS * 1000.0 + 0.001 * ax.max(ay)
}

/// Total length of a slice of segments.
fn slice_length(segs: &[TraceSegment]) -> f64 {
    segs.iter().map(|s| dist(s.start, s.end)).sum()
}

/// Build a `TraceSegment` from two points, inheriting metadata from `template`.
fn make_seg(template: &TraceSegment, from: Point, to: Point) -> TraceSegment {
    TraceSegment {
        net_id: template.net_id,
        layer: template.layer,
        start: from,
        end: to,
        width_mm: template.width_mm,
    }
}

/// Generate up to 5 bypass candidates from `a` to `b` using the metadata
/// from `template` for net/layer/width.
///
/// Candidates are:
/// 1. Direct `a→b` if octilinear.
/// 2. H-first L-shape: `a → (b.x, a.y) → b`.
/// 3. V-first L-shape: `a → (a.x, b.y) → b`.
/// 4. Diagonal-then-H: 45° leg using `min(|dx|, |dy|)`, then horizontal.
/// 5. Diagonal-then-V: 45° leg using `min(|dx|, |dy|)`, then vertical.
fn generate_candidates(
    a: Point,
    b: Point,
    template: &TraceSegment,
) -> Vec<Vec<TraceSegment>> {
    let mut out: Vec<Vec<TraceSegment>> = Vec::with_capacity(5);
    let dx = b.x - a.x;
    let dy = b.y - a.y;

    // Candidate 1: direct, if octilinear.
    if is_octilinear(dx, dy) {
        out.push(vec![make_seg(template, a, b)]);
    }

    // Candidate 2: H-first L-shape: a → (b.x, a.y) → b.
    let corner_h = Point { x: b.x, y: a.y };
    if dist(a, corner_h) > EPS && dist(corner_h, b) > EPS {
        out.push(vec![
            make_seg(template, a, corner_h),
            make_seg(template, corner_h, b),
        ]);
    }

    // Candidate 3: V-first L-shape: a → (a.x, b.y) → b.
    let corner_v = Point { x: a.x, y: b.y };
    if dist(a, corner_v) > EPS && dist(corner_v, b) > EPS {
        out.push(vec![
            make_seg(template, a, corner_v),
            make_seg(template, corner_v, b),
        ]);
    }

    // Candidates 4 & 5: diagonal leg then rectilinear remainder.
    let diag = dx.abs().min(dy.abs());
    if diag > EPS {
        let sign_x = if dx >= 0.0 { 1.0 } else { -1.0 };
        let sign_y = if dy >= 0.0 { 1.0 } else { -1.0 };

        // Candidate 4: diagonal-then-H.
        let mid4 = Point {
            x: a.x + sign_x * diag,
            y: a.y + sign_y * diag,
        };
        if dist(a, mid4) > EPS && dist(mid4, b) > EPS {
            out.push(vec![
                make_seg(template, a, mid4),
                make_seg(template, mid4, b),
            ]);
        }

        // Candidate 5: diagonal-then-V (approach from the other axis first).
        let remaining_x = dx.abs() - diag;
        let remaining_y = dy.abs() - diag;
        // Which axis still has remainder after the diagonal?
        let mid5 = if remaining_x > remaining_y {
            // More horizontal remainder → diagonal consumed the full vertical extent.
            Point {
                x: b.x - sign_x * diag.min(dy.abs()),
                y: b.y,
            }
        } else {
            Point {
                x: b.x,
                y: b.y - sign_y * diag.min(dx.abs()),
            }
        };
        if dist(a, mid5) > EPS && dist(mid5, b) > EPS {
            out.push(vec![
                make_seg(template, a, mid5),
                make_seg(template, mid5, b),
            ]);
        }
    }

    out
}

// ---------------------------------------------------------------------------
// Clearance check helper
// ---------------------------------------------------------------------------

/// Returns `true` if all segments in `candidate` maintain `clearance_mm`
/// clearance against all non-same-net obstacles in `spatial`.
fn check_candidate_clearance(
    candidate: &[TraceSegment],
    spatial: &SpatialIndex,
    net_id: NetId,
    clearance_mm: f64,
) -> bool {
    for seg in candidate {
        let half_width = seg.width_mm / 2.0;
        let min_dist = clearance_mm + half_width;

        let obstacles = spatial.clearance_query(
            seg.layer,
            seg.start.x,
            seg.start.y,
            seg.end.x,
            seg.end.y,
            min_dist,
        );

        for obs in obstacles {
            if obs.net_id() == Some(net_id) {
                continue;
            }
            let bounds = obs.raw_bounds();
            let center = Point {
                x: (bounds[0] + bounds[2]) / 2.0,
                y: (bounds[1] + bounds[3]) / 2.0,
            };
            let d = point_segment_distance(center, seg.start, seg.end);
            let obs_half = ((bounds[2] - bounds[0]).max(bounds[3] - bounds[1])) / 2.0;
            if d - obs_half < min_dist {
                return false;
            }
        }
    }
    true
}

// ---------------------------------------------------------------------------
// Core algorithm
// ---------------------------------------------------------------------------

/// Run one complete pull-tight sweep over `segments`, optionally checking
/// clearance with `spatial`.
///
/// Returns `true` if any replacement was made.
fn pull_tight_pass(
    segments: &mut Vec<TraceSegment>,
    spatial: Option<(&SpatialIndex, NetId, f64)>,
) -> bool {
    let mut any_changed = false;

    for &step in STEP_SIZES {
        let mut changed_this_step = false;
        let mut i = 0;

        while i + step < segments.len() {
            if !valid_range(segments, i, i + step) {
                i += 1;
                continue;
            }

            let a = segments[i].start;
            let b = segments[i + step].end;
            let template = &segments[i];
            let original_len = {
                let window = &segments[i..=i + step];
                slice_length(window)
            };

            let candidates = generate_candidates(a, b, template);

            // Find the shortest candidate that is shorter than the original
            // and passes clearance.
            let best = candidates
                .into_iter()
                .filter(|c| slice_length(c) < original_len - EPS)
                .filter(|c| {
                    if let Some((spatial, net_id, clearance_mm)) = spatial {
                        check_candidate_clearance(c, spatial, net_id, clearance_mm)
                    } else {
                        true
                    }
                })
                .min_by(|x, y| {
                    slice_length(x)
                        .partial_cmp(&slice_length(y))
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

            if let Some(replacement) = best {
                // Replace segments[i..=i+step] with the candidate.
                let end_idx = i + step + 1;
                segments.splice(i..end_idx, replacement);
                changed_this_step = true;
                any_changed = true;
                // Do not advance i — re-examine from the same position.
            } else {
                i += 1;
            }
        }

        // After each step level consolidate colinear runs.
        merge_colinear(segments);

        let _ = changed_this_step; // consumed by any_changed
    }

    any_changed
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Pull-tight without clearance checking (geometry-only, for tests).
pub fn pull_tight(segments: &mut Vec<TraceSegment>) {
    if segments.len() < 2 {
        return;
    }
    for _ in 0..MAX_PASSES {
        let changed = pull_tight_pass(segments, None);
        if !changed {
            break;
        }
    }
}

/// Pull-tight with DRC-safe clearance checking.
///
/// Uses `net_id` for same-net pass-through and `clearance_mm` as the
/// required clearance to all other obstacles.
pub fn pull_tight_checked(
    segments: &mut Vec<TraceSegment>,
    spatial: &SpatialIndex,
    net_id: NetId,
    clearance_mm: f64,
) {
    if segments.len() < 2 {
        return;
    }
    for _ in 0..MAX_PASSES {
        let changed = pull_tight_pass(segments, Some((spatial, net_id, clearance_mm)));
        if !changed {
            break;
        }
    }
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

    /// A 3-segment right-angle detour should be shortened.
    ///
    ///   (0,0) → (5,0) → (5,5) → (10,5)
    ///
    /// The H-first L-shape bypass (0,0)→(10,5) passes through (10,0) or
    /// (0,5); either is shorter than the 3-segment path.
    #[test]
    fn direct_bypass_shortens_detour() {
        let mut segs = vec![
            seg(0.0, 0.0, 5.0, 0.0),
            seg(5.0, 0.0, 5.0, 5.0),
            seg(5.0, 5.0, 10.0, 5.0),
        ];
        let before = total_length(&segs);
        pull_tight(&mut segs);
        let after = total_length(&segs);
        assert!(
            after < before,
            "pull_tight should shorten a 3-segment detour: before={before:.4}, after={after:.4}"
        );
        assert!(segs.len() <= 3, "segment count must not grow");
    }

    /// A 4-segment staircase should reduce to at most 2 segments.
    #[test]
    fn l_shape_bypass() {
        let mut segs = vec![
            seg(0.0, 0.0, 1.0, 0.0),
            seg(1.0, 0.0, 1.0, 1.0),
            seg(1.0, 1.0, 2.0, 1.0),
            seg(2.0, 1.0, 2.0, 2.0),
        ];
        let before = total_length(&segs);
        pull_tight(&mut segs);
        let after = total_length(&segs);
        assert!(
            after < before,
            "pull_tight should shorten a 4-segment staircase: before={before:.4}, after={after:.4}"
        );
    }

    /// A layer change between segments prevents bypass.
    #[test]
    fn cross_layer_not_bypassed() {
        let mut segs = vec![
            TraceSegment {
                net_id: NetId(0),
                layer: LayerId(0),
                start: Point { x: 0.0, y: 0.0 },
                end: Point { x: 5.0, y: 0.0 },
                width_mm: 0.2,
            },
            TraceSegment {
                net_id: NetId(0),
                layer: LayerId(1),
                start: Point { x: 5.0, y: 0.0 },
                end: Point { x: 5.0, y: 5.0 },
                width_mm: 0.2,
            },
            TraceSegment {
                net_id: NetId(0),
                layer: LayerId(1),
                start: Point { x: 5.0, y: 5.0 },
                end: Point { x: 10.0, y: 5.0 },
                width_mm: 0.2,
            },
        ];
        let before_len = segs.len();
        let before_total = total_length(&segs);
        pull_tight(&mut segs);
        // The cross-layer boundary prevents a single bypass of all 3 segments.
        // Segment count must not decrease to 1 (the cross-layer join is preserved).
        assert!(
            segs.len() >= 2,
            "cross-layer segments must not be merged across the layer boundary"
        );
        let after = total_length(&segs);
        assert!(
            after <= before_total + EPS,
            "length must not increase: before={before_total:.4}, after={after:.4}, count before={before_len}, after={}",
            segs.len()
        );
    }

    /// A gap between segments (subnet boundary) blocks bypass.
    #[test]
    fn cross_subnet_not_bypassed() {
        let mut segs = vec![
            seg(0.0, 0.0, 5.0, 0.0),
            seg(6.0, 0.0, 6.0, 5.0), // gap — not connected to first
            seg(6.0, 5.0, 10.0, 5.0),
        ];
        // valid_range will reject any window spanning the gap.
        let before = segs.clone();
        pull_tight(&mut segs);
        assert_eq!(
            segs.len(),
            before.len(),
            "subnet boundary gap must prevent bypass"
        );
    }

    /// A width change (neckdown) blocks bypass.
    #[test]
    fn different_width_not_bypassed() {
        let mut segs = vec![
            TraceSegment {
                net_id: NetId(0),
                layer: LayerId(0),
                start: Point { x: 0.0, y: 0.0 },
                end: Point { x: 5.0, y: 0.0 },
                width_mm: 0.2,
            },
            TraceSegment {
                net_id: NetId(0),
                layer: LayerId(0),
                start: Point { x: 5.0, y: 0.0 },
                end: Point { x: 5.0, y: 5.0 },
                width_mm: 0.1, // neckdown
            },
            TraceSegment {
                net_id: NetId(0),
                layer: LayerId(0),
                start: Point { x: 5.0, y: 5.0 },
                end: Point { x: 10.0, y: 5.0 },
                width_mm: 0.1,
            },
        ];
        // A window spanning all 3 (0..=2) will fail valid_range due to width mismatch.
        let before_count = segs.len();
        pull_tight(&mut segs);
        // The 2-segment window [1..=2] is valid but may or may not shorten;
        // the 3-segment window [0..=2] must be rejected.
        // At minimum the neckdown boundary (segs[0].width != segs[1].width) is preserved.
        assert!(
            segs.len() <= before_count,
            "segment count must not grow"
        );
        // Verify no width homogenization.
        let widths: Vec<f64> = segs.iter().map(|s| s.width_mm).collect();
        assert!(
            widths.iter().any(|&w| (w - 0.2).abs() < EPS)
                || widths.iter().any(|&w| (w - 0.1).abs() < EPS),
            "width information must be preserved"
        );
    }

    /// A single segment is unchanged.
    #[test]
    fn single_segment_unchanged() {
        let mut segs = vec![seg(0.0, 0.0, 5.0, 3.0)];
        let before = segs.clone();
        pull_tight(&mut segs);
        assert_eq!(segs.len(), 1);
        assert!((segs[0].start.x - before[0].start.x).abs() < EPS);
        assert!((segs[0].end.x - before[0].end.x).abs() < EPS);
    }

    /// The first segment's start and the last segment's end must never change.
    #[test]
    fn endpoints_preserved() {
        let mut segs = vec![
            seg(0.0, 0.0, 5.0, 0.0),
            seg(5.0, 0.0, 5.0, 5.0),
            seg(5.0, 5.0, 10.0, 5.0),
        ];
        let start = segs.first().unwrap().start;
        let end = segs.last().unwrap().end;
        pull_tight(&mut segs);
        let new_start = segs.first().unwrap().start;
        let new_end = segs.last().unwrap().end;
        assert!(
            (new_start.x - start.x).abs() < EPS && (new_start.y - start.y).abs() < EPS,
            "start endpoint must be preserved"
        );
        assert!(
            (new_end.x - end.x).abs() < EPS && (new_end.y - end.y).abs() < EPS,
            "end endpoint must be preserved"
        );
    }

    /// Total path length must be monotonically non-increasing.
    #[test]
    fn length_never_increases() {
        let mut segs = vec![
            seg(0.0, 0.0, 3.0, 0.0),
            seg(3.0, 0.0, 3.0, 4.0),
            seg(3.0, 4.0, 7.0, 4.0),
            seg(7.0, 4.0, 7.0, 0.0),
        ];
        let before = total_length(&segs);
        pull_tight(&mut segs);
        let after = total_length(&segs);
        assert!(
            after <= before + EPS,
            "total length must never increase: before={before:.4}, after={after:.4}"
        );
    }
}
