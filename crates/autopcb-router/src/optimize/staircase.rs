//! Staircase elimination: converts consecutive H-V bend pairs into single
//! 45-degree diagonal segments.
//!
//! A "staircase" is a pair of consecutive segments where one is horizontal
//! and the next is vertical (or vice versa), forming a right-angle step that
//! can be replaced by a single diagonal segment without changing the endpoints.
//!
//! The replacement is valid only when both segments share the same layer and
//! net. The resulting diagonal has length equal to the hypotenuse of the
//! right-angle step.

use autopcb_routes::TraceSegment;

/// Tolerance for treating a coordinate difference as zero (mm).
const EPS: f64 = 1e-9;

/// Returns `true` if segment `s` is horizontal (dy ≈ 0) or vertical (dx ≈ 0)
/// and the two values indicating which.
fn classify(s: &TraceSegment) -> (bool, bool) {
    let dx = (s.end.x - s.start.x).abs();
    let dy = (s.end.y - s.start.y).abs();
    (dx < EPS, dy < EPS)
}

/// Returns `true` if `a` ends where `b` starts (within EPS).
fn connected(a: &TraceSegment, b: &TraceSegment) -> bool {
    (a.end.x - b.start.x).abs() < EPS && (a.end.y - b.start.y).abs() < EPS
}

/// Returns `true` if `a` and `b` form a staircase pair: one is horizontal and
/// the other is vertical (or vice versa), and they are connected end-to-start.
/// Both must be on the same layer and belong to the same net.
fn is_staircase_pair(a: &TraceSegment, b: &TraceSegment) -> bool {
    if a.layer != b.layer || a.net_id != b.net_id {
        return false;
    }
    if !connected(a, b) {
        return false;
    }
    let (a_vert, a_horiz) = classify(a);
    let (b_vert, b_horiz) = classify(b);
    // One must be purely horizontal and the other purely vertical.
    let a_hv = a_vert ^ a_horiz; // exactly one axis is near-zero ↔ axis-aligned
    let b_hv = b_vert ^ b_horiz;
    if !a_hv || !b_hv {
        return false;
    }
    // One horizontal, one vertical.
    let a_is_horiz = !a_vert; // dy≈0 → horizontal
    let b_is_horiz = !b_vert;
    a_is_horiz != b_is_horiz // different axes ↔ staircase
}

/// Merge a staircase pair `(a, b)` into a single diagonal `TraceSegment`.
///
/// The resulting segment shares `a.start` and `b.end`, and width/net/layer
/// from `a`.
fn merge_staircase(a: &TraceSegment, b: &TraceSegment) -> TraceSegment {
    TraceSegment {
        net_id: a.net_id,
        layer: a.layer,
        start: a.start,
        end: b.end,
        width_mm: a.width_mm,
    }
}

/// Scan `segments` for consecutive staircase pairs and replace each pair with
/// a single diagonal segment.
///
/// Operates in-place. Multiple passes are run until no further reductions are
/// possible (to handle chains of staircases).
pub fn eliminate_staircases(segments: &mut Vec<TraceSegment>) {
    loop {
        let before = segments.len();
        let mut result: Vec<TraceSegment> = Vec::with_capacity(segments.len());
        let mut i = 0;
        while i < segments.len() {
            if i + 1 < segments.len() && is_staircase_pair(&segments[i], &segments[i + 1]) {
                result.push(merge_staircase(&segments[i], &segments[i + 1]));
                i += 2;
            } else {
                result.push(segments[i].clone());
                i += 1;
            }
        }
        *segments = result;
        if segments.len() == before {
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

    /// Helper: total length of a segment list.
    fn total_length(segs: &[TraceSegment]) -> f64 {
        segs.iter()
            .map(|s| {
                let dx = s.end.x - s.start.x;
                let dy = s.end.y - s.start.y;
                (dx * dx + dy * dy).sqrt()
            })
            .sum()
    }

    /// 3-segment staircase H→V→H: should compress.
    ///
    /// Input:  (0,0)→(1,0)  horizontal
    ///         (1,0)→(1,1)  vertical
    ///         (1,1)→(2,1)  horizontal
    ///
    /// First pass merges segs 0+1 into diagonal (0,0)→(1,1).
    /// Second pass: diagonal is not axis-aligned so it won't pair with seg 2.
    /// End result: 2 segments (diagonal + last horizontal).
    #[test]
    fn staircase_h_v_h_compresses() {
        let mut segs = vec![
            seg(0.0, 0.0, 1.0, 0.0), // H
            seg(1.0, 0.0, 1.0, 1.0), // V
            seg(1.0, 1.0, 2.0, 1.0), // H
        ];
        eliminate_staircases(&mut segs);
        // Must be fewer segments than original.
        assert!(
            segs.len() < 3,
            "expected fewer than 3 segments, got {}",
            segs.len()
        );
        // Endpoints preserved.
        let first = &segs[0];
        let last = segs.last().unwrap();
        assert!((first.start.x - 0.0).abs() < EPS, "start.x");
        assert!((first.start.y - 0.0).abs() < EPS, "start.y");
        assert!((last.end.x - 2.0).abs() < EPS, "end.x");
        assert!((last.end.y - 1.0).abs() < EPS, "end.y");
    }

    /// A single straight horizontal segment: unchanged.
    #[test]
    fn straight_line_unchanged() {
        let mut segs = vec![seg(0.0, 0.0, 5.0, 0.0)];
        let original = segs.clone();
        eliminate_staircases(&mut segs);
        assert_eq!(segs.len(), original.len());
        assert!((segs[0].end.x - 5.0).abs() < EPS);
    }

    /// A single diagonal segment (neither H nor V): unchanged.
    #[test]
    fn diagonal_unchanged() {
        let mut segs = vec![seg(0.0, 0.0, 3.0, 4.0)];
        let original_len = segs.len();
        eliminate_staircases(&mut segs);
        assert_eq!(segs.len(), original_len);
    }

    /// V→H pair: a single staircase is merged into one diagonal.
    #[test]
    fn staircase_v_h_merges_to_diagonal() {
        let mut segs = vec![
            seg(0.0, 0.0, 0.0, 2.0), // V
            seg(0.0, 2.0, 3.0, 2.0), // H
        ];
        eliminate_staircases(&mut segs);
        assert_eq!(segs.len(), 1, "expected 1 diagonal, got {}", segs.len());
        let d = &segs[0];
        assert!((d.start.x - 0.0).abs() < EPS);
        assert!((d.start.y - 0.0).abs() < EPS);
        assert!((d.end.x - 3.0).abs() < EPS);
        assert!((d.end.y - 2.0).abs() < EPS);
    }

    /// Segments on different layers must not be merged.
    #[test]
    fn different_layers_not_merged() {
        let mut segs = vec![
            TraceSegment {
                net_id: NetId(0),
                layer: LayerId(0),
                start: Point { x: 0.0, y: 0.0 },
                end: Point { x: 1.0, y: 0.0 },
                width_mm: 0.2,
            },
            TraceSegment {
                net_id: NetId(0),
                layer: LayerId(1),
                start: Point { x: 1.0, y: 0.0 },
                end: Point { x: 1.0, y: 1.0 },
                width_mm: 0.2,
            },
        ];
        eliminate_staircases(&mut segs);
        assert_eq!(segs.len(), 2, "different-layer pair must not merge");
    }

    /// Total endpoint-to-endpoint displacement is preserved after elimination.
    #[test]
    fn endpoints_preserved_after_elimination() {
        let mut segs = vec![
            seg(0.0, 0.0, 2.0, 0.0), // H
            seg(2.0, 0.0, 2.0, 3.0), // V
        ];
        eliminate_staircases(&mut segs);
        assert_eq!(segs.len(), 1);
        let d = &segs[0];
        assert!((d.start.x).abs() < EPS);
        assert!((d.start.y).abs() < EPS);
        assert!((d.end.x - 2.0).abs() < EPS);
        assert!((d.end.y - 3.0).abs() < EPS);
    }

    /// The merged diagonal must have length = sqrt(dx²+dy²).
    #[test]
    fn diagonal_length_correct() {
        let mut segs = vec![
            seg(0.0, 0.0, 3.0, 0.0), // dx=3
            seg(3.0, 0.0, 3.0, 4.0), // dy=4
        ];
        // Expected: 3-4-5 triangle → diagonal length = 5
        let before_len = total_length(&segs);
        eliminate_staircases(&mut segs);
        assert_eq!(segs.len(), 1);
        let after_len = total_length(&segs);
        // Diagonal (5) is shorter than sum of legs (3+4=7).
        assert!(
            after_len < before_len,
            "diagonal shorter than sum of legs: {after_len} >= {before_len}"
        );
        assert!((after_len - 5.0).abs() < 1e-6, "expected 5.0, got {after_len}");
    }
}
