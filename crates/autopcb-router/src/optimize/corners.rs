//! Corner-style conversion: applies `CornerStyle` (45-degree or rounded)
//! to right-angle bends in routed traces.
//!
//! For `FortyFiveDegree`: a right-angle bend between two consecutive
//! axis-aligned segments is replaced by a chamfer — the bend vertex is split
//! into two 45° segments that cut across the corner.
//!
//! For `RoundedCorner`: the corner is approximated by a short sequence of
//! small diagonal segments (a chord approximation of the arc).
//!
//! Segments that are not axis-aligned, or that do not form a 90° angle with
//! their neighbour, are left unchanged.

use autopcb_routes::{Point, TraceSegment};

use crate::config::CornerStyle;

/// Tolerance for treating a coordinate difference as zero (mm).
const EPS: f64 = 1e-9;

/// Chamfer size as a fraction of the shorter leg at each corner.
/// Setting this to 0.25 means we cut 25% off each leg.
const CHAMFER_FRACTION: f64 = 0.25;

/// Number of chord segments used to approximate a rounded corner arc.
const ROUND_STEPS: usize = 3;

/// Returns `true` if `s` is purely horizontal (dy ≈ 0).
fn is_horizontal(s: &TraceSegment) -> bool {
    (s.end.y - s.start.y).abs() < EPS
}

/// Returns `true` if `s` is purely vertical (dx ≈ 0).
fn is_vertical(s: &TraceSegment) -> bool {
    (s.end.x - s.start.x).abs() < EPS
}

/// Returns `true` if `a` ends exactly where `b` starts.
fn connected(a: &TraceSegment, b: &TraceSegment) -> bool {
    (a.end.x - b.start.x).abs() < EPS && (a.end.y - b.start.y).abs() < EPS
}

/// Returns `true` if `a` and `b` form a right-angle bend that can be chamfered:
/// - connected end-to-start
/// - same layer and net
/// - one horizontal and one vertical
fn is_right_angle(a: &TraceSegment, b: &TraceSegment) -> bool {
    if a.layer != b.layer || a.net_id != b.net_id {
        return false;
    }
    if !connected(a, b) {
        return false;
    }
    (is_horizontal(a) && is_vertical(b)) || (is_vertical(a) && is_horizontal(b))
}

/// Compute the chamfer point on segment `s` at distance `dist` from `s.end`,
/// moving back toward `s.start`.
fn chamfer_point_from_end(s: &TraceSegment, dist: f64) -> Point {
    let dx = s.end.x - s.start.x;
    let dy = s.end.y - s.start.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < EPS {
        return s.end;
    }
    let t = 1.0 - dist / len;
    let t = t.clamp(0.0, 1.0);
    Point {
        x: s.start.x + t * dx,
        y: s.start.y + t * dy,
    }
}

/// Compute the chamfer point on segment `s` at distance `dist` from `s.start`.
fn chamfer_point_from_start(s: &TraceSegment, dist: f64) -> Point {
    let dx = s.end.x - s.start.x;
    let dy = s.end.y - s.start.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < EPS {
        return s.start;
    }
    let t = (dist / len).clamp(0.0, 1.0);
    Point {
        x: s.start.x + t * dx,
        y: s.start.y + t * dy,
    }
}

/// Emit a chamfered 45° replacement for the right-angle bend at the junction
/// of `a` (ending at vertex) and `b` (starting at vertex).
///
/// Returns up to 3 segments:
/// - shortened `a` (from `a.start` to chamfer-start)
/// - diagonal chamfer (from chamfer-start to chamfer-end)
/// - shortened `b` (from chamfer-end to `b.end`)
fn emit_chamfer(a: &TraceSegment, b: &TraceSegment) -> Vec<TraceSegment> {
    let len_a = {
        let dx = a.end.x - a.start.x;
        let dy = a.end.y - a.start.y;
        (dx * dx + dy * dy).sqrt()
    };
    let len_b = {
        let dx = b.end.x - b.start.x;
        let dy = b.end.y - b.start.y;
        (dx * dx + dy * dy).sqrt()
    };
    let chamfer = (len_a.min(len_b) * CHAMFER_FRACTION).max(EPS);

    let p0 = chamfer_point_from_end(a, chamfer);   // new end of shortened a
    let p1 = chamfer_point_from_start(b, chamfer); // new start of shortened b

    let mut out = Vec::with_capacity(3);

    // Shortened a (only emit if non-degenerate).
    if (p0.x - a.start.x).abs() > EPS || (p0.y - a.start.y).abs() > EPS {
        out.push(TraceSegment {
            net_id: a.net_id,
            layer: a.layer,
            start: a.start,
            end: p0,
            width_mm: a.width_mm,
        });
    }

    // Diagonal chamfer.
    out.push(TraceSegment {
        net_id: a.net_id,
        layer: a.layer,
        start: p0,
        end: p1,
        width_mm: a.width_mm,
    });

    // Shortened b (only emit if non-degenerate).
    if (b.end.x - p1.x).abs() > EPS || (b.end.y - p1.y).abs() > EPS {
        out.push(TraceSegment {
            net_id: b.net_id,
            layer: b.layer,
            start: p1,
            end: b.end,
            width_mm: b.width_mm,
        });
    }

    out
}

/// Emit a rounded-corner approximation (chord segments along an arc).
///
/// Uses the same chamfer points as `emit_chamfer` but interpolates `ROUND_STEPS`
/// intermediate points along the arc between them.
fn emit_rounded(a: &TraceSegment, b: &TraceSegment) -> Vec<TraceSegment> {
    let len_a = {
        let dx = a.end.x - a.start.x;
        let dy = a.end.y - a.start.y;
        (dx * dx + dy * dy).sqrt()
    };
    let len_b = {
        let dx = b.end.x - b.start.x;
        let dy = b.end.y - b.start.y;
        (dx * dx + dy * dy).sqrt()
    };
    let chamfer = (len_a.min(len_b) * CHAMFER_FRACTION).max(EPS);

    let p_start = chamfer_point_from_end(a, chamfer);
    let p_end = chamfer_point_from_start(b, chamfer);

    // Centre of the arc: the bend vertex (a.end == b.start).
    let cx = a.end.x;
    let cy = a.end.y;

    // Generate ROUND_STEPS intermediate chord points along the arc from
    // p_start → vertex → p_end.  We parameterise by angle.
    let angle_start = (p_start.y - cy).atan2(p_start.x - cx);
    let angle_end = (p_end.y - cy).atan2(p_end.x - cx);

    // Angular span (always the short way around the corner).
    let mut span = angle_end - angle_start;
    while span > std::f64::consts::PI {
        span -= 2.0 * std::f64::consts::PI;
    }
    while span < -std::f64::consts::PI {
        span += 2.0 * std::f64::consts::PI;
    }

    let radius = (p_start.x - cx)
        .hypot(p_start.y - cy)
        .max((p_end.x - cx).hypot(p_end.y - cy));

    let n = ROUND_STEPS + 1; // number of intervals
    let mut points = Vec::with_capacity(n + 1);
    points.push(p_start);
    for i in 1..n {
        let t = i as f64 / n as f64;
        let angle = angle_start + t * span;
        points.push(Point {
            x: cx + radius * angle.cos(),
            y: cy + radius * angle.sin(),
        });
    }
    points.push(p_end);

    let mut out: Vec<TraceSegment> = Vec::with_capacity(2 + points.len());

    // Shortened a.
    if (p_start.x - a.start.x).abs() > EPS || (p_start.y - a.start.y).abs() > EPS {
        out.push(TraceSegment {
            net_id: a.net_id,
            layer: a.layer,
            start: a.start,
            end: p_start,
            width_mm: a.width_mm,
        });
    }

    // Arc chord segments.
    for w in points.windows(2) {
        out.push(TraceSegment {
            net_id: a.net_id,
            layer: a.layer,
            start: w[0],
            end: w[1],
            width_mm: a.width_mm,
        });
    }

    // Shortened b.
    if (b.end.x - p_end.x).abs() > EPS || (b.end.y - p_end.y).abs() > EPS {
        out.push(TraceSegment {
            net_id: b.net_id,
            layer: b.layer,
            start: p_end,
            end: b.end,
            width_mm: b.width_mm,
        });
    }

    out
}

/// Apply `style` to all right-angle bends in `segments`.
///
/// For `FortyFiveDegree`: chamfers each right-angle bend with a 45° diagonal.
/// For `RoundedCorner`: approximates each bend with short chord segments.
///
/// Segments that are not axis-aligned or that do not form 90° bends are
/// passed through unchanged.
pub fn convert_corners(segments: &mut Vec<TraceSegment>, style: CornerStyle) {
    if segments.len() < 2 {
        return;
    }

    let mut result: Vec<TraceSegment> = Vec::with_capacity(segments.len() * 2);
    let mut i = 0;

    while i < segments.len() {
        if i + 1 < segments.len() && is_right_angle(&segments[i], &segments[i + 1]) {
            let replacement = match style {
                CornerStyle::FortyFiveDegree => emit_chamfer(&segments[i], &segments[i + 1]),
                CornerStyle::RoundedCorner => emit_rounded(&segments[i], &segments[i + 1]),
            };
            result.extend(replacement);
            i += 2;
        } else {
            result.push(segments[i].clone());
            i += 1;
        }
    }

    *segments = result;
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

    /// A right-angle H→V bend converted to 45° produces more segments
    /// (the original 2 become 3: shortened_H + diagonal + shortened_V).
    #[test]
    fn right_angle_to_45_produces_chamfer() {
        let mut segs = vec![
            seg(0.0, 0.0, 4.0, 0.0), // H, length 4
            seg(4.0, 0.0, 4.0, 4.0), // V, length 4
        ];
        convert_corners(&mut segs, CornerStyle::FortyFiveDegree);
        // Must have more than 2 segments (chamfer inserted).
        assert!(
            segs.len() > 2,
            "expected more than 2 segments after chamfer, got {}",
            segs.len()
        );
        // Start and end preserved.
        assert!((segs.first().unwrap().start.x).abs() < EPS);
        assert!((segs.first().unwrap().start.y).abs() < EPS);
        assert!((segs.last().unwrap().end.x - 4.0).abs() < EPS);
        assert!((segs.last().unwrap().end.y - 4.0).abs() < EPS);
    }

    /// A straight segment (no corner): unchanged.
    #[test]
    fn straight_segment_unchanged() {
        let mut segs = vec![seg(0.0, 0.0, 5.0, 0.0)];
        let original_len = segs.len();
        convert_corners(&mut segs, CornerStyle::FortyFiveDegree);
        assert_eq!(segs.len(), original_len, "straight segment must be unchanged");
    }

    /// Two segments that are both horizontal (no bend): unchanged.
    #[test]
    fn two_horizontal_no_bend_unchanged() {
        let mut segs = vec![
            seg(0.0, 0.0, 2.0, 0.0),
            seg(2.0, 0.0, 5.0, 0.0),
        ];
        convert_corners(&mut segs, CornerStyle::FortyFiveDegree);
        // Both horizontal → no right-angle → unchanged.
        assert_eq!(segs.len(), 2, "two collinear segments must not be modified");
    }

    /// Rounded corner produces more segments than FortyFiveDegree (arc chords).
    #[test]
    fn rounded_corner_produces_arc_segments() {
        let mut segs_45 = vec![
            seg(0.0, 0.0, 4.0, 0.0),
            seg(4.0, 0.0, 4.0, 4.0),
        ];
        let mut segs_round = segs_45.clone();

        convert_corners(&mut segs_45, CornerStyle::FortyFiveDegree);
        convert_corners(&mut segs_round, CornerStyle::RoundedCorner);

        // Rounded must produce at least as many segments as 45°.
        assert!(
            segs_round.len() >= segs_45.len(),
            "rounded ({}) should produce >= segments as 45° ({})",
            segs_round.len(),
            segs_45.len()
        );

        // Endpoints preserved for rounded too.
        assert!((segs_round.first().unwrap().start.x).abs() < EPS);
        assert!((segs_round.last().unwrap().end.y - 4.0).abs() < EPS);
    }

    /// Segments on different layers: not converted.
    #[test]
    fn different_layers_not_converted() {
        let mut segs = vec![
            TraceSegment {
                net_id: NetId(0),
                layer: LayerId(0),
                start: Point { x: 0.0, y: 0.0 },
                end: Point { x: 3.0, y: 0.0 },
                width_mm: 0.2,
            },
            TraceSegment {
                net_id: NetId(0),
                layer: LayerId(1),
                start: Point { x: 3.0, y: 0.0 },
                end: Point { x: 3.0, y: 3.0 },
                width_mm: 0.2,
            },
        ];
        convert_corners(&mut segs, CornerStyle::FortyFiveDegree);
        assert_eq!(segs.len(), 2, "different-layer pair must not be converted");
    }
}
