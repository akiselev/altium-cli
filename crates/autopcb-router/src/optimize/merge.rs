//! Colinear segment merging: collapses consecutive same-direction segments
//! into a single segment, removing micro-jogs and PathFinder artifacts.
//!
//! Two adjacent segments can be merged when they:
//! - belong to the same net and layer,
//! - have the same width (neckdown boundaries are preserved),
//! - are connected end-to-start (subnet boundary detection), and
//! - point in the same direction (colinear within `COLINEAR_TOLERANCE`).

use autopcb_routes::TraceSegment;

/// Tolerance for treating a coordinate difference as zero (mm).
const EPS: f64 = 1e-9;

/// Maximum |sin θ| between two direction vectors that is still considered
/// colinear (~0.115°).
const COLINEAR_TOLERANCE: f64 = 0.002;

/// Returns `true` if segment `b` can be merged into segment `a` by extending
/// `a.end` to `b.end`.
fn can_merge(a: &TraceSegment, b: &TraceSegment) -> bool {
    // Must share net, layer, and width.
    if a.net_id != b.net_id {
        return false;
    }
    if a.layer != b.layer {
        return false;
    }
    if (a.width_mm - b.width_mm).abs() >= EPS {
        return false;
    }
    // Must be connected end-to-start (subnet boundary guard).
    if (a.end.x - b.start.x).abs() >= EPS || (a.end.y - b.start.y).abs() >= EPS {
        return false;
    }
    // Colinearity check using normalized direction vectors.
    let adx = a.end.x - a.start.x;
    let ady = a.end.y - a.start.y;
    let a_len = (adx * adx + ady * ady).sqrt();

    let bdx = b.end.x - b.start.x;
    let bdy = b.end.y - b.start.y;
    let b_len = (bdx * bdx + bdy * bdy).sqrt();

    // Zero-length segments are trivially colinear; absorbing them is a no-op.
    if a_len < EPS || b_len < EPS {
        return true;
    }

    // Normalize and compute |cross product| = |sin θ|.
    let au = (adx / a_len, ady / a_len);
    let bu = (bdx / b_len, bdy / b_len);
    let cross = au.0 * bu.1 - au.1 * bu.0;
    cross.abs() < COLINEAR_TOLERANCE
}

/// Merge all runs of colinear, connected, same-net/layer/width segments in
/// `segments` into single segments.
///
/// Operates in-place. A single pass is sufficient because the output is
/// already fully merged.
pub fn merge_colinear(segments: &mut Vec<TraceSegment>) {
    if segments.len() < 2 {
        return;
    }

    let mut result: Vec<TraceSegment> = Vec::with_capacity(segments.len());
    result.push(segments[0].clone());

    for seg in &segments[1..] {
        let last = result.last_mut().unwrap();
        if can_merge(last, seg) {
            // Extend the last segment to absorb this one.
            last.end = seg.end;
        } else {
            result.push(seg.clone());
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

    #[test]
    fn two_colinear_horizontal_merged() {
        let mut segs = vec![
            seg(0.0, 0.0, 5.0, 0.0),
            seg(5.0, 0.0, 10.0, 0.0),
        ];
        merge_colinear(&mut segs);
        assert_eq!(segs.len(), 1, "two colinear horizontal segments must merge");
        assert!((segs[0].start.x).abs() < EPS);
        assert!((segs[0].end.x - 10.0).abs() < EPS);
        assert!((segs[0].end.y).abs() < EPS);
    }

    #[test]
    fn two_colinear_diagonal_merged() {
        let mut segs = vec![
            seg(0.0, 0.0, 3.0, 3.0),
            seg(3.0, 3.0, 6.0, 6.0),
        ];
        merge_colinear(&mut segs);
        assert_eq!(segs.len(), 1, "two colinear diagonal segments must merge");
        assert!((segs[0].start.x).abs() < EPS);
        assert!((segs[0].end.x - 6.0).abs() < EPS);
        assert!((segs[0].end.y - 6.0).abs() < EPS);
    }

    #[test]
    fn perpendicular_not_merged() {
        let mut segs = vec![
            seg(0.0, 0.0, 5.0, 0.0), // horizontal
            seg(5.0, 0.0, 5.0, 5.0), // vertical
        ];
        merge_colinear(&mut segs);
        assert_eq!(segs.len(), 2, "perpendicular segments must not merge");
    }

    #[test]
    fn different_width_not_merged() {
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
                end: Point { x: 10.0, y: 0.0 },
                width_mm: 0.4,
            },
        ];
        merge_colinear(&mut segs);
        assert_eq!(segs.len(), 2, "different-width segments must not merge (neckdown preserved)");
    }

    #[test]
    fn different_layer_not_merged() {
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
                end: Point { x: 10.0, y: 0.0 },
                width_mm: 0.2,
            },
        ];
        merge_colinear(&mut segs);
        assert_eq!(segs.len(), 2, "different-layer segments must not merge");
    }

    #[test]
    fn disconnected_not_merged() {
        // Gap between segments — subnet boundary.
        let mut segs = vec![
            seg(0.0, 0.0, 5.0, 0.0),
            seg(6.0, 0.0, 11.0, 0.0), // does not connect to the first
        ];
        merge_colinear(&mut segs);
        assert_eq!(segs.len(), 2, "disconnected segments (subnet boundary) must not merge");
    }

    #[test]
    fn chain_of_three_merged() {
        let mut segs = vec![
            seg(0.0, 0.0, 3.0, 0.0),
            seg(3.0, 0.0, 7.0, 0.0),
            seg(7.0, 0.0, 10.0, 0.0),
        ];
        merge_colinear(&mut segs);
        assert_eq!(segs.len(), 1, "three colinear segments must merge into one");
        assert!((segs[0].start.x).abs() < EPS);
        assert!((segs[0].end.x - 10.0).abs() < EPS);
    }

    #[test]
    fn empty_and_single_unchanged() {
        let mut empty: Vec<TraceSegment> = vec![];
        merge_colinear(&mut empty);
        assert!(empty.is_empty());

        let mut single = vec![seg(0.0, 0.0, 5.0, 5.0)];
        merge_colinear(&mut single);
        assert_eq!(single.len(), 1);
    }

    #[test]
    fn zero_length_absorbed() {
        // A zero-length segment between two colinear segments should be absorbed.
        let mut segs = vec![
            seg(0.0, 0.0, 5.0, 0.0),
            seg(5.0, 0.0, 5.0, 0.0), // zero-length
            seg(5.0, 0.0, 10.0, 0.0),
        ];
        merge_colinear(&mut segs);
        assert_eq!(segs.len(), 1, "zero-length segment should be absorbed by merge");
        assert!((segs[0].start.x).abs() < EPS);
        assert!((segs[0].end.x - 10.0).abs() < EPS);
    }
}
