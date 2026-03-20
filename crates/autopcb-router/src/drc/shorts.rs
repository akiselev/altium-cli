//! Short circuit detection: overlapping segments from different nets on the
//! same layer.
//!
//! Two segments from different nets are a short when their physical copper
//! bodies overlap. The simplified model: if the distance between centerlines
//! is less than `(width_a + width_b) / 2`, the copper edges overlap and the
//! traces are shorted.

use autopcb_ir::types::PointMm;
use autopcb_routes::{RouteSolution, TraceSegment};
use altium_format_types::pcb::RuleKind;

use super::{DrcObject, DrcViolation, DrcViolationKind};
use super::clearance::segment_to_segment_distance;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Detect short circuits in `solution`.
///
/// Returns one `DrcViolation` per pair of segments from different nets whose
/// copper bodies overlap on the same layer.
pub fn check_shorts(solution: &RouteSolution) -> Vec<DrcViolation> {
    let mut violations = Vec::new();

    let all_segments: Vec<&TraceSegment> = solution
        .nets
        .values()
        .flat_map(|n| n.segments.iter())
        .collect();

    for (i, seg_a) in all_segments.iter().enumerate() {
        for seg_b in all_segments.iter().skip(i + 1) {
            if seg_a.layer != seg_b.layer {
                continue;
            }
            if seg_a.net_id == seg_b.net_id {
                // Same net — touching/overlapping is allowed.
                continue;
            }
            // Copper bodies overlap when centerline distance < sum of half-widths.
            let center_dist = segment_to_segment_distance(
                seg_a.start, seg_a.end,
                seg_b.start, seg_b.end,
            );
            let min_separation = (seg_a.width_mm + seg_b.width_mm) / 2.0;
            if center_dist < min_separation {
                let loc_x = (seg_a.start.x + seg_a.end.x) / 2.0;
                let loc_y = (seg_a.start.y + seg_a.end.y) / 2.0;
                violations.push(DrcViolation {
                    kind: DrcViolationKind::ShortCircuit,
                    rule_kind: RuleKind::ShortCircuit,
                    rule_name: "ShortCircuit".into(),
                    object_a: DrcObject::Segment((*seg_a).clone()),
                    object_b: Some(DrcObject::Segment((*seg_b).clone())),
                    location: PointMm { x: loc_x, y: loc_y },
                    layer: Some(seg_a.layer),
                    // actual_mm: how much the copper bodies overlap (negative = overlap).
                    actual_mm: center_dist - min_separation,
                    // required_mm: 0.0 — the bodies must not overlap at all.
                    required_mm: 0.0,
                });
            }
        }
    }

    violations
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use autopcb_routes::{LayerId, NetId, Point, RoutedNet, RouteSolution, TraceSegment};

    fn make_solution(segs: Vec<TraceSegment>) -> RouteSolution {
        let mut solution = RouteSolution::new();
        let mut by_net: std::collections::BTreeMap<NetId, Vec<TraceSegment>> =
            std::collections::BTreeMap::new();
        for s in segs {
            by_net.entry(s.net_id).or_default().push(s);
        }
        for (net_id, segments) in by_net {
            solution.nets.insert(net_id, RoutedNet {
                net_id,
                segments,
                vias: vec![],
                routed_length_mm: 0.0,
            });
        }
        solution
    }

    /// Two perpendicular crossing traces from different nets → short detected.
    #[test]
    fn crossing_different_nets_short_detected() {
        let layer = LayerId(0);
        // Horizontal segment, net 0.
        let seg_a = TraceSegment {
            net_id: NetId(0),
            layer,
            start: Point { x: -1.0, y: 0.0 },
            end: Point { x: 1.0, y: 0.0 },
            width_mm: 0.2,
        };
        // Vertical segment, net 1, crossing at origin.
        let seg_b = TraceSegment {
            net_id: NetId(1),
            layer,
            start: Point { x: 0.0, y: -1.0 },
            end: Point { x: 0.0, y: 1.0 },
            width_mm: 0.2,
        };
        let solution = make_solution(vec![seg_a, seg_b]);
        let violations = check_shorts(&solution);
        assert_eq!(violations.len(), 1, "expected one short circuit violation");
        assert_eq!(violations[0].kind, DrcViolationKind::ShortCircuit);
        // actual_mm should be negative (copper bodies overlap).
        assert!(violations[0].actual_mm < 0.0,
            "overlapping segments should have negative actual_mm, got {}",
            violations[0].actual_mm);
    }

    /// Same-net segments that overlap → no short violation.
    #[test]
    fn same_net_overlap_no_violation() {
        let layer = LayerId(0);
        let seg_a = TraceSegment {
            net_id: NetId(0),
            layer,
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 1.0, y: 0.0 },
            width_mm: 0.2,
        };
        // Overlapping same-net segment.
        let seg_b = TraceSegment {
            net_id: NetId(0),
            layer,
            start: Point { x: 0.5, y: 0.0 },
            end: Point { x: 1.5, y: 0.0 },
            width_mm: 0.2,
        };
        let solution = make_solution(vec![seg_a, seg_b]);
        let violations = check_shorts(&solution);
        assert!(violations.is_empty(), "same-net overlap must not produce shorts");
    }

    /// Two different-net segments on different layers → no short.
    #[test]
    fn different_layers_no_violation() {
        let seg_a = TraceSegment {
            net_id: NetId(0),
            layer: LayerId(0),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 1.0, y: 0.0 },
            width_mm: 0.2,
        };
        let seg_b = TraceSegment {
            net_id: NetId(1),
            layer: LayerId(1), // different layer
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 1.0, y: 0.0 },
            width_mm: 0.2,
        };
        let solution = make_solution(vec![seg_a, seg_b]);
        let violations = check_shorts(&solution);
        assert!(violations.is_empty(), "different-layer segments must not produce shorts");
    }

    /// Two parallel traces from different nets well apart → no short.
    #[test]
    fn well_separated_traces_no_short() {
        let layer = LayerId(0);
        let seg_a = TraceSegment {
            net_id: NetId(0),
            layer,
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 1.0, y: 0.0 },
            width_mm: 0.1,
        };
        // 1.0 mm apart — much more than (0.1+0.1)/2 = 0.1 mm.
        let seg_b = TraceSegment {
            net_id: NetId(1),
            layer,
            start: Point { x: 0.0, y: 1.0 },
            end: Point { x: 1.0, y: 1.0 },
            width_mm: 0.1,
        };
        let solution = make_solution(vec![seg_a, seg_b]);
        let violations = check_shorts(&solution);
        assert!(violations.is_empty(), "well-separated traces must not produce shorts");
    }
}
