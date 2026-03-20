//! Differential-pair DRC: length skew, gap, and width-matching checks.
//!
//! WHY this check exists: differential pairs must have matching lengths so
//! that the positive and negative signals arrive simultaneously at the
//! receiver.  Any skew causes inter-symbol interference and degrades signal
//! integrity.  The `max_uncoupled_length_mm` in the policy bounds how much
//! of each trace may be routed outside the coupled region.  Gap enforcement
//! ensures pairs are tightly coupled; width matching ensures equal impedance
//! on both conductors.
//!
//! For each pair identified via `IrNet::diff_pair_partner` the check:
//! 1. Flags skew exceeding `max_uncoupled_length_mm`.
//! 2. Flags average width mismatch between the two nets (`DiffPairWidthMismatch`).
//! 3. For each segment in net_a, finds the nearest segment in net_b on the
//!    same layer, computes center-to-center distance, subtracts half the sum
//!    of the two widths to get the gap, and flags if gap is outside
//!    `[gap_mm, max_gap_mm]` (`DiffPairGapViolation`).

use altium_format_types::pcb::RuleKind;
use autopcb_ir::{types::PointMm, PcbIr};
use autopcb_routes::{NetId, RouteSolution};

use super::{net_length_mm, net_midpoint, DrcObject, DrcViolation, DrcViolationKind};
use crate::drc::policy::DrcPolicy;

/// Average width of all segments for a net.  Returns 0.0 if the net has no segments.
fn net_avg_width(solution: &RouteSolution, net_id: NetId) -> f64 {
    solution
        .nets
        .get(&net_id)
        .and_then(|rn| {
            if rn.segments.is_empty() {
                None
            } else {
                let sum: f64 = rn.segments.iter().map(|s| s.width_mm).sum();
                Some(sum / rn.segments.len() as f64)
            }
        })
        .unwrap_or(0.0)
}

/// Midpoint of a segment, as `PointMm`.
fn seg_midpoint_mm(s: &autopcb_routes::TraceSegment) -> PointMm {
    PointMm {
        x: (s.start.x + s.end.x) / 2.0,
        y: (s.start.y + s.end.y) / 2.0,
    }
}

/// Minimum distance between the centerlines of two segments.
///
/// Uses the segment-to-segment distance from the clearance module.
/// For parallel diff-pair traces, this equals the perpendicular distance.
fn segment_min_distance(a: &autopcb_routes::TraceSegment, b: &autopcb_routes::TraceSegment) -> f64 {
    super::clearance::segment_to_segment_distance(a.start, a.end, b.start, b.end)
}

/// Check differential-pair constraints.
///
/// For each pair of nets linked by `IrNet::diff_pair_partner`, this function:
/// 1. Computes the routed length of both nets and checks skew against
///    `policy.diff_pair.max_uncoupled_length_mm`.
/// 2. Compares average segment width of both nets; flags `DiffPairWidthMismatch`
///    if the difference exceeds 1 % of the larger width.
/// 3. For each segment in net_a, finds the nearest segment in net_b on the same
///    layer; computes center-to-center distance minus half the summed widths to
///    obtain the gap; flags `DiffPairGapViolation` if gap < `gap_mm` or
///    gap > `max_gap_mm`.
///
/// Each pair is processed exactly once (canonical order by raw index).
pub fn check_diff_pairs(
    solution: &RouteSolution,
    policy: &DrcPolicy,
    ir: &PcbIr,
) -> Vec<DrcViolation> {
    let dp_constraint = match policy.diff_pair {
        Some(c) => c,
        None => return Vec::new(),
    };

    let mut violations = Vec::new();
    // Track which pairs we have already processed so each pair is flagged once.
    let mut visited = std::collections::BTreeSet::new();

    for (_ir_net_id, ir_net) in ir.nets.iter() {
        let partner_ir_id = match ir_net.diff_pair_partner {
            Some(p) => p,
            None => continue,
        };

        let this_raw = ir_net.id.raw();
        let partner_raw = partner_ir_id.raw();

        // Process in canonical order (smaller raw index first).
        let key = (this_raw.min(partner_raw), this_raw.max(partner_raw));
        if !visited.insert(key) {
            continue;
        }

        let net_a = NetId(this_raw);
        let net_b = NetId(partner_raw);

        let len_a = net_length_mm(solution, net_a);
        let len_b = net_length_mm(solution, net_b);
        let skew = (len_a - len_b).abs();

        if skew > dp_constraint.max_uncoupled_length_mm {
            let (short_id, long_id) = if len_a <= len_b { (net_a, net_b) } else { (net_b, net_a) };

            let obj_a = solution
                .nets
                .get(&short_id)
                .and_then(|rn| rn.segments.first())
                .map(|s| DrcObject::Segment(s.clone()))
                .unwrap_or_else(|| DrcObject::Pad {
                    component: String::new(),
                    pad: String::new(),
                    position: net_midpoint(solution, short_id),
                });

            let obj_b = solution
                .nets
                .get(&long_id)
                .and_then(|rn| rn.segments.first())
                .map(|s| DrcObject::Segment(s.clone()))
                .unwrap_or_else(|| DrcObject::Pad {
                    component: String::new(),
                    pad: String::new(),
                    position: net_midpoint(solution, long_id),
                });

            violations.push(DrcViolation {
                kind: DrcViolationKind::DiffPairSkew,
                rule_kind: RuleKind::DifferentialPairsRouting,
                rule_name: "DifferentialPairsRouting".to_string(),
                object_a: obj_a,
                object_b: Some(obj_b),
                location: net_midpoint(solution, short_id),
                layer: None,
                actual_mm: skew,
                required_mm: dp_constraint.max_uncoupled_length_mm,
            });
        }

        // Width matching: flag if average widths differ by more than 1 % of the
        // larger value (or by more than 1 µm as an absolute floor).
        let avg_w_a = net_avg_width(solution, net_a);
        let avg_w_b = net_avg_width(solution, net_b);
        if avg_w_a > 0.0 && avg_w_b > 0.0 {
            let width_diff = (avg_w_a - avg_w_b).abs();
            let threshold = (avg_w_a.max(avg_w_b) * 0.01).max(1e-6);
            if width_diff > threshold {
                let loc = net_midpoint(solution, net_a);
                let obj_a = solution
                    .nets
                    .get(&net_a)
                    .and_then(|rn| rn.segments.first())
                    .map(|s| DrcObject::Segment(s.clone()))
                    .unwrap_or_else(|| DrcObject::Pad {
                        component: String::new(),
                        pad: String::new(),
                        position: loc,
                    });
                let obj_b = solution
                    .nets
                    .get(&net_b)
                    .and_then(|rn| rn.segments.first())
                    .map(|s| DrcObject::Segment(s.clone()))
                    .unwrap_or_else(|| DrcObject::Pad {
                        component: String::new(),
                        pad: String::new(),
                        position: net_midpoint(solution, net_b),
                    });
                violations.push(DrcViolation {
                    kind: DrcViolationKind::DiffPairWidthMismatch,
                    rule_kind: RuleKind::DifferentialPairsRouting,
                    rule_name: "DifferentialPairsRouting".to_string(),
                    object_a: obj_a,
                    object_b: Some(obj_b),
                    location: loc,
                    layer: None,
                    actual_mm: width_diff,
                    required_mm: 0.0,
                });
            }
        }

        // Gap check: for each segment in net_a, find the nearest segment in net_b
        // on the same layer and check the edge-to-edge gap.
        if let Some(rn_a) = solution.nets.get(&net_a) {
            if let Some(rn_b) = solution.nets.get(&net_b) {
                for seg_a in &rn_a.segments {
                    // Restrict to segments on the same layer.
                    let same_layer: Vec<&autopcb_routes::TraceSegment> = rn_b
                        .segments
                        .iter()
                        .filter(|sb| sb.layer == seg_a.layer)
                        .collect();
                    if same_layer.is_empty() {
                        continue;
                    }
                    // Find nearest segment by center-to-center distance.
                    let nearest = same_layer
                        .iter()
                        .min_by(|sa, sb| {
                            segment_min_distance(seg_a, sa)
                                .partial_cmp(&segment_min_distance(seg_a, sb))
                                .unwrap()
                        })
                        .unwrap();
                    let center_dist = segment_min_distance(seg_a, nearest);
                    let gap = center_dist - (seg_a.width_mm + nearest.width_mm) / 2.0;
                    if gap < dp_constraint.gap_mm || gap > dp_constraint.max_gap_mm {
                        let loc = seg_midpoint_mm(seg_a);
                        violations.push(DrcViolation {
                            kind: DrcViolationKind::DiffPairGapViolation,
                            rule_kind: RuleKind::DifferentialPairsRouting,
                            rule_name: "DifferentialPairsRouting".to_string(),
                            object_a: DrcObject::Segment(seg_a.clone()),
                            object_b: Some(DrcObject::Segment((*nearest).clone())),
                            location: loc,
                            layer: Some(seg_a.layer),
                            actual_mm: gap,
                            required_mm: dp_constraint.gap_mm,
                        });
                    }
                }
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
    use autopcb_ir::{
        handles::{IdMap, LayerId as IrLayerId, NetId as IrNetId, RuleId},
        layer_stack::{IrCopperLayer, IrLayerStack, PreferredDirection},
        net::IrNet,
        rule::{IrDesignRule, IrRuleParams},
        types::{BoundingBoxMm, PointMm},
        IrBoardGeometry, PcbIr,
    };
    use autopcb_routes::{LayerId, NetId, RoutedNet, RouteSolution, TraceSegment};
    use autopcb_routes::Point;
    use altium_format_types::pcb::RuleKind as AltRuleKind;

    fn empty_ir() -> PcbIr {
        PcbIr {
            board: IrBoardGeometry {
                outline: vec![],
                cutouts: vec![],
                bounds: BoundingBoxMm {
                    min: PointMm { x: 0.0, y: 0.0 },
                    max: PointMm { x: 100.0, y: 100.0 },
                },
                keepouts: vec![],
            },
            layer_stack: IrLayerStack {
                copper_layers: vec![IrCopperLayer {
                    id: IrLayerId::from(0u32),
                    name: "Top Layer".into(),
                    is_top: true,
                    is_bottom: false,
                    preferred_direction: Some(PreferredDirection::Any),
                }],
                copper_layer_count: 1,
            },
            components: IdMap::new(),
            nets: IdMap::new(),
            rules: IdMap::new(),
            free_copper: Default::default(),
            polygons: IdMap::new(),
        }
    }

    fn add_rule(ir: &mut PcbIr, kind: AltRuleKind, priority: i32, params: IrRuleParams) {
        let id = ir.rules.push(IrDesignRule {
            id: RuleId::from(0u32),
            name: "dp_rule".into(),
            kind,
            priority,
            enabled: true,
            params,
        });
        ir.rules[id].id = id;
    }

    /// Add two paired nets to the IR; return (pos_id, neg_id).
    fn add_diff_pair(ir: &mut PcbIr) -> (IrNetId, IrNetId) {
        let pos_id = ir.nets.push(IrNet {
            id: IrNetId::from(0u32),
            name: "DP+".into(),
            pins: vec![],
            component_count: 0,
            net_class: None,
            diff_pair_partner: None, // filled in below
        });
        ir.nets[pos_id].id = pos_id;

        let neg_id = ir.nets.push(IrNet {
            id: IrNetId::from(0u32),
            name: "DP-".into(),
            pins: vec![],
            component_count: 0,
            net_class: None,
            diff_pair_partner: None,
        });
        ir.nets[neg_id].id = neg_id;

        ir.nets[pos_id].diff_pair_partner = Some(neg_id);
        ir.nets[neg_id].diff_pair_partner = Some(pos_id);

        (pos_id, neg_id)
    }

    fn make_segment(net_id: NetId, length: f64, y_offset: f64) -> TraceSegment {
        TraceSegment {
            net_id,
            layer: LayerId(0),
            start: Point { x: 0.0, y: y_offset },
            end: Point { x: length, y: y_offset },
            width_mm: 0.1,
        }
    }

    fn solution_with_lengths(pos_ir: IrNetId, pos_len: f64, neg_ir: IrNetId, neg_len: f64) -> RouteSolution {
        let mut solution = RouteSolution::new();
        // Offset the negative net by 0.3 mm so center-to-center = 0.3 mm, gap = 0.2 mm,
        // which is within [gap_mm=0.1, max_gap_mm=0.3] and avoids spurious gap violations.
        for (ir_id, len, y) in [(pos_ir, pos_len, 0.0f64), (neg_ir, neg_len, 0.3f64)] {
            let net_id = NetId(ir_id.raw());
            let seg = make_segment(net_id, len, y);
            solution.nets.insert(
                net_id,
                RoutedNet {
                    net_id,
                    segments: vec![seg],
                    vias: vec![],
                    routed_length_mm: len,
                },
            );
        }
        solution
    }

    #[test]
    fn paired_nets_similar_lengths_no_violation() {
        let mut ir = empty_ir();
        add_rule(
            &mut ir,
            AltRuleKind::DifferentialPairsRouting,
            1,
            IrRuleParams::DiffPairsRouting {
                gap_mm: 0.1,
                max_gap_mm: 0.3,
                max_uncoupled_length_mm: 5.0,
            },
        );
        let (pos, neg) = add_diff_pair(&mut ir);
        let policy = DrcPolicy::build(&ir).unwrap();
        // Lengths 100 and 103: skew = 3, max_uncoupled = 5 → pass.
        let solution = solution_with_lengths(pos, 100.0, neg, 103.0);

        let violations = check_diff_pairs(&solution, &policy, &ir);
        assert!(violations.is_empty(), "small skew should not violate: {:?}", violations);
    }

    #[test]
    fn paired_nets_large_skew_violation() {
        let mut ir = empty_ir();
        add_rule(
            &mut ir,
            AltRuleKind::DifferentialPairsRouting,
            1,
            IrRuleParams::DiffPairsRouting {
                gap_mm: 0.1,
                max_gap_mm: 0.3,
                max_uncoupled_length_mm: 2.0,
            },
        );
        let (pos, neg) = add_diff_pair(&mut ir);
        let policy = DrcPolicy::build(&ir).unwrap();
        // Lengths 100 and 110: skew = 10, max_uncoupled = 2 → violation.
        let solution = solution_with_lengths(pos, 100.0, neg, 110.0);

        let violations = check_diff_pairs(&solution, &policy, &ir);
        assert_eq!(violations.len(), 1, "expected 1 DiffPairSkew violation");
        let v = &violations[0];
        assert_eq!(v.kind, DrcViolationKind::DiffPairSkew);
        assert_eq!(v.rule_kind, AltRuleKind::DifferentialPairsRouting);
        assert!((v.actual_mm - 10.0).abs() < 1e-6, "skew should be 10mm");
        assert!((v.required_mm - 2.0).abs() < 1e-6, "max_uncoupled should be 2mm");
    }

    #[test]
    fn no_diff_pair_policy_no_violations() {
        let mut ir = empty_ir();
        let (pos, neg) = add_diff_pair(&mut ir);
        // No DiffPairsRouting rule → policy.diff_pair = None → skip check.
        let policy = DrcPolicy::build(&ir).unwrap();
        let solution = solution_with_lengths(pos, 100.0, neg, 200.0);

        let violations = check_diff_pairs(&solution, &policy, &ir);
        assert!(violations.is_empty(), "no diff-pair policy should produce no violations");
    }

    /// Build a solution where both paired nets have the same length but different widths.
    fn solution_with_widths(
        pos_ir: IrNetId,
        w_pos: f64,
        neg_ir: IrNetId,
        w_neg: f64,
    ) -> RouteSolution {
        let mut solution = RouteSolution::new();
        for (ir_id, w) in [(pos_ir, w_pos), (neg_ir, w_neg)] {
            let net_id = NetId(ir_id.raw());
            let seg = TraceSegment {
                net_id,
                layer: LayerId(0),
                start: Point { x: 0.0, y: 0.0 },
                end: Point { x: 10.0, y: 0.0 },
                width_mm: w,
            };
            solution.nets.insert(
                net_id,
                RoutedNet {
                    net_id,
                    segments: vec![seg],
                    vias: vec![],
                    routed_length_mm: 10.0,
                },
            );
        }
        solution
    }

    /// Build a solution where both nets run at fixed y-offsets (parallel, same layer).
    fn solution_with_parallel_segments(
        pos_ir: IrNetId,
        y_pos: f64,
        w_pos: f64,
        neg_ir: IrNetId,
        y_neg: f64,
        w_neg: f64,
    ) -> RouteSolution {
        let mut solution = RouteSolution::new();
        for (ir_id, y, w) in [(pos_ir, y_pos, w_pos), (neg_ir, y_neg, w_neg)] {
            let net_id = NetId(ir_id.raw());
            let seg = TraceSegment {
                net_id,
                layer: LayerId(0),
                start: Point { x: 0.0, y },
                end: Point { x: 10.0, y },
                width_mm: w,
            };
            solution.nets.insert(
                net_id,
                RoutedNet {
                    net_id,
                    segments: vec![seg],
                    vias: vec![],
                    routed_length_mm: 10.0,
                },
            );
        }
        solution
    }

    #[test]
    fn width_mismatch_emits_diff_pair_width_mismatch() {
        let mut ir = empty_ir();
        add_rule(
            &mut ir,
            AltRuleKind::DifferentialPairsRouting,
            1,
            IrRuleParams::DiffPairsRouting {
                gap_mm: 0.05,
                max_gap_mm: 1.0,
                max_uncoupled_length_mm: 50.0,
            },
        );
        let (pos, neg) = add_diff_pair(&mut ir);
        let policy = DrcPolicy::build(&ir).unwrap();
        // net_a width = 0.1 mm, net_b width = 0.2 mm → 100 % mismatch → violation.
        let solution = solution_with_widths(pos, 0.1, neg, 0.2);

        let violations = check_diff_pairs(&solution, &policy, &ir);
        let width_viols: Vec<_> = violations
            .iter()
            .filter(|v| v.kind == DrcViolationKind::DiffPairWidthMismatch)
            .collect();
        assert_eq!(width_viols.len(), 1, "expected 1 DiffPairWidthMismatch, got {:?}", violations);
        assert!((width_viols[0].actual_mm - 0.1).abs() < 1e-6,
            "width diff should be 0.1 mm, got {}", width_viols[0].actual_mm);
    }

    #[test]
    fn equal_widths_no_width_mismatch() {
        let mut ir = empty_ir();
        add_rule(
            &mut ir,
            AltRuleKind::DifferentialPairsRouting,
            1,
            IrRuleParams::DiffPairsRouting {
                gap_mm: 0.05,
                max_gap_mm: 1.0,
                max_uncoupled_length_mm: 50.0,
            },
        );
        let (pos, neg) = add_diff_pair(&mut ir);
        let policy = DrcPolicy::build(&ir).unwrap();
        let solution = solution_with_widths(pos, 0.1, neg, 0.1);

        let violations = check_diff_pairs(&solution, &policy, &ir);
        let width_viols: Vec<_> = violations
            .iter()
            .filter(|v| v.kind == DrcViolationKind::DiffPairWidthMismatch)
            .collect();
        assert!(width_viols.is_empty(), "equal widths should not produce width-mismatch: {:?}", violations);
    }

    #[test]
    fn gap_too_small_emits_diff_pair_gap_violation() {
        let mut ir = empty_ir();
        // gap_mm = 0.2 mm required; segments are separated by only 0.05 mm edge-to-edge.
        add_rule(
            &mut ir,
            AltRuleKind::DifferentialPairsRouting,
            1,
            IrRuleParams::DiffPairsRouting {
                gap_mm: 0.2,
                max_gap_mm: 1.0,
                max_uncoupled_length_mm: 50.0,
            },
        );
        let (pos, neg) = add_diff_pair(&mut ir);
        let policy = DrcPolicy::build(&ir).unwrap();
        // Both traces width 0.1 mm; center-to-center = 0.15 mm → gap = 0.15 - 0.1 = 0.05 mm < 0.2.
        let solution = solution_with_parallel_segments(pos, 0.0, 0.1, neg, 0.15, 0.1);

        let violations = check_diff_pairs(&solution, &policy, &ir);
        let gap_viols: Vec<_> = violations
            .iter()
            .filter(|v| v.kind == DrcViolationKind::DiffPairGapViolation)
            .collect();
        assert!(!gap_viols.is_empty(), "expected DiffPairGapViolation, got {:?}", violations);
        assert!(gap_viols[0].actual_mm < 0.2,
            "reported gap should be below min_gap, got {}", gap_viols[0].actual_mm);
    }

    #[test]
    fn gap_too_large_emits_diff_pair_gap_violation() {
        let mut ir = empty_ir();
        // max_gap_mm = 0.3 mm; traces are 2.0 mm apart edge-to-edge.
        add_rule(
            &mut ir,
            AltRuleKind::DifferentialPairsRouting,
            1,
            IrRuleParams::DiffPairsRouting {
                gap_mm: 0.1,
                max_gap_mm: 0.3,
                max_uncoupled_length_mm: 50.0,
            },
        );
        let (pos, neg) = add_diff_pair(&mut ir);
        let policy = DrcPolicy::build(&ir).unwrap();
        // Both traces width 0.1 mm; center-to-center = 2.1 mm → gap = 2.1 - 0.1 = 2.0 mm > 0.3.
        let solution = solution_with_parallel_segments(pos, 0.0, 0.1, neg, 2.1, 0.1);

        let violations = check_diff_pairs(&solution, &policy, &ir);
        let gap_viols: Vec<_> = violations
            .iter()
            .filter(|v| v.kind == DrcViolationKind::DiffPairGapViolation)
            .collect();
        assert!(!gap_viols.is_empty(), "expected DiffPairGapViolation for oversized gap, got {:?}", violations);
        assert!(gap_viols[0].actual_mm > 0.3,
            "reported gap should exceed max_gap, got {}", gap_viols[0].actual_mm);
    }

    #[test]
    fn gap_within_bounds_no_gap_violation() {
        let mut ir = empty_ir();
        add_rule(
            &mut ir,
            AltRuleKind::DifferentialPairsRouting,
            1,
            IrRuleParams::DiffPairsRouting {
                gap_mm: 0.1,
                max_gap_mm: 0.5,
                max_uncoupled_length_mm: 50.0,
            },
        );
        let (pos, neg) = add_diff_pair(&mut ir);
        let policy = DrcPolicy::build(&ir).unwrap();
        // Both width 0.1 mm; center-to-center = 0.3 mm → gap = 0.3 - 0.1 = 0.2 mm (within [0.1, 0.5]).
        let solution = solution_with_parallel_segments(pos, 0.0, 0.1, neg, 0.3, 0.1);

        let violations = check_diff_pairs(&solution, &policy, &ir);
        let gap_viols: Vec<_> = violations
            .iter()
            .filter(|v| v.kind == DrcViolationKind::DiffPairGapViolation)
            .collect();
        assert!(gap_viols.is_empty(), "gap within bounds should not violate: {:?}", violations);
    }

    #[test]
    fn pair_checked_only_once() {
        let mut ir = empty_ir();
        add_rule(
            &mut ir,
            AltRuleKind::DifferentialPairsRouting,
            1,
            IrRuleParams::DiffPairsRouting {
                gap_mm: 0.1,
                max_gap_mm: 0.3,
                max_uncoupled_length_mm: 1.0,
            },
        );
        let (pos, neg) = add_diff_pair(&mut ir);
        let policy = DrcPolicy::build(&ir).unwrap();
        // Both nets point to each other; ensure we don't get duplicate violations.
        let solution = solution_with_lengths(pos, 100.0, neg, 110.0);

        let violations = check_diff_pairs(&solution, &policy, &ir);
        assert_eq!(violations.len(), 1, "pair should be reported exactly once");
    }
}
