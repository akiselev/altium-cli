//! Length DRC: net length min/max and matched-length group tolerance.
//!
//! WHY this check exists: signal integrity and timing constraints require
//! traces to stay within length bounds (e.g. impedance-controlled traces
//! must not be too long, high-speed buses must be length-matched).
//!
//! Matched-length check: when `policy.matched_length` is Some, we find the
//! spread (max − min) across all nets that have routed segments and flag any
//! pair whose length difference exceeds the tolerance.  This is a simplified
//! global check — a future refinement would scope matched-length rules to
//! named groups extracted from the IR.
//!
//! Per-net length bounds: when `policy.length_constraints` contains an entry
//! for `None` (the default class), every routed net is checked against
//! `[min_mm, max_mm]` and flagged with `LengthBelowMinimum` or
//! `LengthAboveMaximum` respectively.

use altium_format_types::pcb::RuleKind;
use autopcb_routes::{NetId, RouteSolution};

use super::{net_length_mm, net_midpoint, DrcObject, DrcViolation, DrcViolationKind};
use crate::drc::policy::DrcPolicy;

/// Check net lengths against the policy.
///
/// Checks:
/// 1. Matched-length spread across all routed nets when `policy.matched_length` is Some.
/// 2. Per-net min/max bounds from `policy.length_constraints` (key `None` = default class).
pub fn check_lengths(solution: &RouteSolution, policy: &DrcPolicy) -> Vec<DrcViolation> {
    let mut violations = Vec::new();

    // Collect lengths of all nets that have at least one routed segment.
    let routed_lengths: std::collections::BTreeMap<NetId, f64> = solution
        .nets
        .iter()
        .filter(|(_, rn)| !rn.segments.is_empty())
        .map(|(&net_id, _)| (net_id, net_length_mm(solution, net_id)))
        .collect();

    if routed_lengths.is_empty() {
        return violations;
    }

    // Matched-length check: flag pairs whose length delta exceeds tolerance.
    if let Some(ml) = policy.matched_length {
        let max_len = routed_lengths
            .values()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let min_len = routed_lengths
            .values()
            .copied()
            .fold(f64::INFINITY, f64::min);

        let spread = max_len - min_len;
        if spread > ml.tolerance_mm {
            // Find the shortest net — it is the one that needs lengthening.
            let (&short_net_id, _) = routed_lengths
                .iter()
                .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                .unwrap();
            let (&long_net_id, _) = routed_lengths
                .iter()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                .unwrap();

            violations.push(DrcViolation {
                kind: DrcViolationKind::MatchedLengthOutOfTolerance,
                rule_kind: RuleKind::MatchedLengths,
                rule_name: "MatchedLengths".to_string(),
                object_a: DrcObject::Segment(
                    solution.nets[&short_net_id].segments[0].clone(),
                ),
                object_b: Some(DrcObject::Segment(
                    solution.nets[&long_net_id].segments[0].clone(),
                )),
                location: net_midpoint(solution, short_net_id),
                layer: None,
                // actual: spread between the two extreme nets
                actual_mm: spread,
                // required: tolerance from policy
                required_mm: ml.tolerance_mm,
            });
        }
    }

    // Per-net min/max length check.
    if let Some(constraint) = policy.length_constraints.get(&None) {
        for (&net_id, &length) in &routed_lengths {
            if length < constraint.min_mm {
                let loc = net_midpoint(solution, net_id);
                violations.push(DrcViolation {
                    kind: DrcViolationKind::NetLengthBelowMinimum,
                    rule_kind: RuleKind::Length,
                    rule_name: "Length".to_string(),
                    object_a: DrcObject::Segment(
                        solution.nets[&net_id].segments[0].clone(),
                    ),
                    object_b: None,
                    location: loc,
                    layer: None,
                    actual_mm: length,
                    required_mm: constraint.min_mm,
                });
            }
            if length > constraint.max_mm {
                let loc = net_midpoint(solution, net_id);
                violations.push(DrcViolation {
                    kind: DrcViolationKind::NetLengthAboveMaximum,
                    rule_kind: RuleKind::Length,
                    rule_name: "Length".to_string(),
                    object_a: DrcObject::Segment(
                        solution.nets[&net_id].segments[0].clone(),
                    ),
                    object_b: None,
                    location: loc,
                    layer: None,
                    actual_mm: length,
                    required_mm: constraint.max_mm,
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
    use autopcb_ir::{
        handles::{IdMap, LayerId as IrLayerId, RuleId},
        layer_stack::{IrCopperLayer, IrLayerStack, PreferredDirection},
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
            name: "test_rule".into(),
            kind,
            priority,
            enabled: true,
            params,
        });
        ir.rules[id].id = id;
    }

    /// Build a solution where net 0 has length `len_a` and net 1 has length `len_b`.
    fn two_net_solution(len_a: f64, len_b: f64) -> RouteSolution {
        let mut solution = RouteSolution::new();

        for (i, len) in [(0u32, len_a), (1u32, len_b)] {
            let net_id = NetId(i);
            let seg = TraceSegment {
                net_id,
                layer: LayerId(0),
                start: Point { x: 0.0, y: 0.0 },
                end: Point { x: len, y: 0.0 },
                width_mm: 0.2,
            };
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

    /// Build a solution with a single net of the given length.
    fn single_net_solution(len: f64) -> RouteSolution {
        let mut solution = RouteSolution::new();
        let net_id = NetId(0);
        let seg = TraceSegment {
            net_id,
            layer: LayerId(0),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: len, y: 0.0 },
            width_mm: 0.2,
        };
        solution.nets.insert(
            net_id,
            RoutedNet {
                net_id,
                segments: vec![seg],
                vias: vec![],
                routed_length_mm: len,
            },
        );
        solution
    }

    #[test]
    fn matched_length_within_tolerance_no_violation() {
        let mut ir = empty_ir();
        add_rule(
            &mut ir,
            AltRuleKind::MatchedLengths,
            1,
            IrRuleParams::MatchedLengths { tolerance_mm: 10.0 },
        );
        let policy = DrcPolicy::build(&ir).unwrap();
        // Lengths 100 and 105: spread = 5, tolerance = 10 → pass.
        let solution = two_net_solution(100.0, 105.0);

        let violations = check_lengths(&solution, &policy);
        assert!(violations.is_empty(), "spread within tolerance should not violate");
    }

    #[test]
    fn matched_length_exceeds_tolerance_violation() {
        let mut ir = empty_ir();
        add_rule(
            &mut ir,
            AltRuleKind::MatchedLengths,
            1,
            IrRuleParams::MatchedLengths { tolerance_mm: 5.0 },
        );
        let policy = DrcPolicy::build(&ir).unwrap();
        // Lengths 100 and 120: spread = 20, tolerance = 5 → violation.
        let solution = two_net_solution(100.0, 120.0);

        let violations = check_lengths(&solution, &policy);
        assert_eq!(violations.len(), 1, "expected 1 MatchedLengthExceeded violation");
        let v = &violations[0];
        assert_eq!(v.kind, DrcViolationKind::MatchedLengthOutOfTolerance);
        assert_eq!(v.rule_kind, AltRuleKind::MatchedLengths);
        // actual_mm is the spread; required_mm is the tolerance.
        assert!(
            (v.actual_mm - 20.0).abs() < 1e-6,
            "spread should be 20mm, got {}",
            v.actual_mm
        );
        assert!(
            (v.required_mm - 5.0).abs() < 1e-6,
            "tolerance should be 5mm, got {}",
            v.required_mm
        );
    }

    #[test]
    fn no_matched_length_policy_no_violation() {
        let ir = empty_ir();
        let policy = DrcPolicy::build(&ir).unwrap();
        // No MatchedLengths rule → policy.matched_length is None → no check.
        let solution = two_net_solution(100.0, 200.0);

        let violations = check_lengths(&solution, &policy);
        assert!(violations.is_empty(), "no matched-length rule should produce no violations");
    }

    #[test]
    fn empty_solution_no_violation() {
        let ir = empty_ir();
        let policy = DrcPolicy::build(&ir).unwrap();
        let solution = RouteSolution::new();

        let violations = check_lengths(&solution, &policy);
        assert!(violations.is_empty());
    }

    #[test]
    fn length_below_minimum_emits_violation() {
        let mut ir = empty_ir();
        // Net length must be at least 50 mm.
        add_rule(
            &mut ir,
            AltRuleKind::Length,
            1,
            IrRuleParams::Length { min_mm: 50.0, max_mm: 200.0 },
        );
        let policy = DrcPolicy::build(&ir).unwrap();
        // Net length = 10 mm → below 50 mm minimum.
        let solution = single_net_solution(10.0);

        let violations = check_lengths(&solution, &policy);
        let below: Vec<_> = violations
            .iter()
            .filter(|v| v.kind == DrcViolationKind::NetLengthBelowMinimum)
            .collect();
        assert_eq!(below.len(), 1, "expected 1 LengthBelowMinimum, got {:?}", violations);
        assert_eq!(below[0].rule_kind, AltRuleKind::Length);
        assert!((below[0].actual_mm - 10.0).abs() < 1e-6,
            "actual should be 10mm, got {}", below[0].actual_mm);
        assert!((below[0].required_mm - 50.0).abs() < 1e-6,
            "required should be 50mm, got {}", below[0].required_mm);
    }

    #[test]
    fn length_above_maximum_emits_violation() {
        let mut ir = empty_ir();
        // Net length must be at most 100 mm.
        add_rule(
            &mut ir,
            AltRuleKind::Length,
            1,
            IrRuleParams::Length { min_mm: 0.0, max_mm: 100.0 },
        );
        let policy = DrcPolicy::build(&ir).unwrap();
        // Net length = 150 mm → above 100 mm maximum.
        let solution = single_net_solution(150.0);

        let violations = check_lengths(&solution, &policy);
        let above: Vec<_> = violations
            .iter()
            .filter(|v| v.kind == DrcViolationKind::NetLengthAboveMaximum)
            .collect();
        assert_eq!(above.len(), 1, "expected 1 LengthAboveMaximum, got {:?}", violations);
        assert_eq!(above[0].rule_kind, AltRuleKind::Length);
        assert!((above[0].actual_mm - 150.0).abs() < 1e-6,
            "actual should be 150mm, got {}", above[0].actual_mm);
        assert!((above[0].required_mm - 100.0).abs() < 1e-6,
            "required should be 100mm, got {}", above[0].required_mm);
    }

    #[test]
    fn length_within_bounds_no_violation() {
        let mut ir = empty_ir();
        add_rule(
            &mut ir,
            AltRuleKind::Length,
            1,
            IrRuleParams::Length { min_mm: 10.0, max_mm: 200.0 },
        );
        let policy = DrcPolicy::build(&ir).unwrap();
        // Net length = 50 mm → within [10, 200].
        let solution = single_net_solution(50.0);

        let violations = check_lengths(&solution, &policy);
        let bounds_viols: Vec<_> = violations
            .iter()
            .filter(|v| {
                v.kind == DrcViolationKind::NetLengthBelowMinimum
                    || v.kind == DrcViolationKind::NetLengthAboveMaximum
            })
            .collect();
        assert!(bounds_viols.is_empty(), "length within bounds should not violate: {:?}", violations);
    }

    #[test]
    fn no_length_rule_no_bounds_violations() {
        let ir = empty_ir();
        let policy = DrcPolicy::build(&ir).unwrap();
        // No Length rule → length_constraints is empty → no per-net check.
        let solution = single_net_solution(5.0);

        let violations = check_lengths(&solution, &policy);
        let bounds_viols: Vec<_> = violations
            .iter()
            .filter(|v| {
                v.kind == DrcViolationKind::NetLengthBelowMinimum
                    || v.kind == DrcViolationKind::NetLengthAboveMaximum
            })
            .collect();
        assert!(bounds_viols.is_empty(), "no length rule should produce no bounds violations");
    }

}
