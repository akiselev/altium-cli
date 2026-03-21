//! Width DRC: verify each segment's width is within min/max bounds.

use altium_format_types::pcb::RuleKind;
use autopcb_ir::PcbIr;
use autopcb_ir::types::PointMm;
use autopcb_routes::RouteSolution;

use super::{DrcObject, DrcViolation, DrcViolationKind};
use super::policy::DrcPolicy;

/// Check that each segment's width satisfies the policy width bounds.
///
/// Looks up the net class for each segment's net from `ir` and passes both
/// net class and layer to `policy.width_bounds()` for scoped cascade resolution.
pub fn check_widths(
    solution: &RouteSolution,
    policy: &DrcPolicy,
    ir: &PcbIr,
) -> Vec<DrcViolation> {
    let mut violations = Vec::new();

    for routed_net in solution.nets.values() {
        let net_class: Option<&str> = super::net_class_for_net(ir, routed_net.net_id);

        for segment in &routed_net.segments {
            let bounds = policy.width_bounds(net_class, Some(segment.layer));

            if segment.width_mm < bounds.min_mm {
                let mid = PointMm {
                    x: (segment.start.x + segment.end.x) / 2.0,
                    y: (segment.start.y + segment.end.y) / 2.0,
                };
                violations.push(DrcViolation {
                    kind: DrcViolationKind::WidthBelowMinimum,
                    rule_kind: RuleKind::Width,
                    rule_name: "Width".to_string(),
                    object_a: DrcObject::Segment(segment.clone()),
                    object_b: None,
                    location: mid,
                    layer: Some(segment.layer),
                    actual_mm: segment.width_mm,
                    required_mm: bounds.min_mm,
                });
            } else if segment.width_mm > bounds.max_mm {
                let mid = PointMm {
                    x: (segment.start.x + segment.end.x) / 2.0,
                    y: (segment.start.y + segment.end.y) / 2.0,
                };
                violations.push(DrcViolation {
                    kind: DrcViolationKind::WidthAboveMaximum,
                    rule_kind: RuleKind::Width,
                    rule_name: "Width".to_string(),
                    object_a: DrcObject::Segment(segment.clone()),
                    object_b: None,
                    location: mid,
                    layer: Some(segment.layer),
                    actual_mm: segment.width_mm,
                    required_mm: bounds.max_mm,
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
    use altium_format_types::pcb::RuleKind;
    use autopcb_ir::{
        handles::RuleId,
        rule::{IrDesignRule, IrRuleParams, IrRuleScopePair},
        PcbIr,
    };
    use autopcb_routes::{LayerId, NetId, Point, RoutedNet, RouteSolution, TraceSegment};

    use super::super::test_helpers::empty_ir;

    fn add_width_rule(ir: &mut PcbIr, priority: i32, min_mm: f64, max_mm: f64, preferred_mm: f64) {
        let id = ir.rules.push(IrDesignRule {
            id: RuleId::from(0u32),
            name: "Width".into(),
            kind: RuleKind::Width,
            priority,
            enabled: true,
            scope: IrRuleScopePair::default(),
            params: IrRuleParams::Width { min_mm, max_mm, preferred_mm },
        });
        ir.rules[id].id = id;
    }

    fn solution_with_segment(width_mm: f64) -> RouteSolution {
        let net_id = NetId(1);
        let segment = TraceSegment {
            net_id,
            layer: LayerId(0),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 1.0, y: 0.0 },
            width_mm,
        };
        let routed_net = RoutedNet {
            net_id,
            segments: vec![segment],
            vias: vec![],
            routed_length_mm: 1.0,
        };
        let mut solution = RouteSolution::new();
        solution.nets.insert(net_id, routed_net);
        solution
    }

    #[test]
    fn width_within_bounds_no_violation() {
        let mut ir = empty_ir();
        add_width_rule(&mut ir, 1, 0.1, 1.0, 0.2);
        let policy = DrcPolicy::build(&ir).unwrap();
        let solution = solution_with_segment(0.2);
        let violations = check_widths(&solution, &policy, &ir);
        assert!(violations.is_empty(), "expected no violations, got {:?}", violations);
    }

    #[test]
    fn width_below_minimum_generates_violation() {
        let mut ir = empty_ir();
        add_width_rule(&mut ir, 1, 0.15, 1.0, 0.2);
        let policy = DrcPolicy::build(&ir).unwrap();
        let solution = solution_with_segment(0.1);
        let violations = check_widths(&solution, &policy, &ir);
        assert_eq!(violations.len(), 1);
        let v = &violations[0];
        assert_eq!(v.kind, DrcViolationKind::WidthBelowMinimum);
        assert!((v.actual_mm - 0.1).abs() < f64::EPSILON,
            "actual_mm should be 0.1, got {}", v.actual_mm);
        assert!((v.required_mm - 0.15).abs() < f64::EPSILON,
            "required_mm should be 0.15, got {}", v.required_mm);
    }

    #[test]
    fn width_above_maximum_generates_violation() {
        let mut ir = empty_ir();
        add_width_rule(&mut ir, 1, 0.1, 0.5, 0.2);
        let policy = DrcPolicy::build(&ir).unwrap();
        let solution = solution_with_segment(1.0);
        let violations = check_widths(&solution, &policy, &ir);
        assert_eq!(violations.len(), 1);
        let v = &violations[0];
        assert_eq!(v.kind, DrcViolationKind::WidthAboveMaximum);
        assert!((v.actual_mm - 1.0).abs() < f64::EPSILON);
        assert!((v.required_mm - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn width_exactly_at_minimum_no_violation() {
        let mut ir = empty_ir();
        add_width_rule(&mut ir, 1, 0.1, 1.0, 0.2);
        let policy = DrcPolicy::build(&ir).unwrap();
        let solution = solution_with_segment(0.1);
        let violations = check_widths(&solution, &policy, &ir);
        assert!(violations.is_empty());
    }

    #[test]
    fn width_exactly_at_maximum_no_violation() {
        let mut ir = empty_ir();
        add_width_rule(&mut ir, 1, 0.1, 1.0, 0.2);
        let policy = DrcPolicy::build(&ir).unwrap();
        let solution = solution_with_segment(1.0);
        let violations = check_widths(&solution, &policy, &ir);
        assert!(violations.is_empty());
    }
}
