//! Width DRC: verify each segment's width is within min/max bounds.

use altium_format_types::pcb::RuleKind;
use autopcb_ir::types::PointMm;
use autopcb_routes::RouteSolution;

use super::{DrcObject, DrcViolation, DrcViolationKind};
use super::policy::DrcPolicy;

/// Check that each segment's width satisfies the policy width bounds.
///
/// Uses the default net-class bounds (`None`) for every segment — per-net-class
/// width lookup can be added once IR carries net-class scope.
pub fn check_widths(
    solution: &RouteSolution,
    policy: &DrcPolicy,
) -> Vec<DrcViolation> {
    let mut violations = Vec::new();

    for routed_net in solution.nets.values() {
        for segment in &routed_net.segments {
            let bounds = policy.width_bounds(None, None);

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
        handles::{IdMap, LayerId as IrLayerId, RuleId},
        layer_stack::{IrCopperLayer, IrLayerStack, PreferredDirection},
        rule::{IrDesignRule, IrRuleParams},
        types::{BoundingBoxMm, PointMm as IrPointMm},
        IrBoardGeometry, PcbIr,
    };
    use autopcb_routes::{LayerId, NetId, Point, RoutedNet, RouteSolution, TraceSegment};

    fn empty_ir() -> PcbIr {
        PcbIr {
            board: IrBoardGeometry {
                outline: vec![],
                cutouts: vec![],
                bounds: BoundingBoxMm {
                    min: IrPointMm { x: 0.0, y: 0.0 },
                    max: IrPointMm { x: 100.0, y: 100.0 },
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
            texts: IdMap::new(),
            regions: IdMap::new(),
            component_bodies: IdMap::new(),
        }
    }

    fn add_width_rule(ir: &mut PcbIr, priority: i32, min_mm: f64, max_mm: f64, preferred_mm: f64) {
        let id = ir.rules.push(IrDesignRule {
            id: RuleId::from(0u32),
            name: "Width".into(),
            kind: RuleKind::Width,
            priority,
            enabled: true,
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
        let violations = check_widths(&solution, &policy);
        assert!(violations.is_empty(), "expected no violations, got {:?}", violations);
    }

    #[test]
    fn width_below_minimum_generates_violation() {
        let mut ir = empty_ir();
        add_width_rule(&mut ir, 1, 0.15, 1.0, 0.2);
        let policy = DrcPolicy::build(&ir).unwrap();
        let solution = solution_with_segment(0.1);
        let violations = check_widths(&solution, &policy);
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
        let violations = check_widths(&solution, &policy);
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
        let violations = check_widths(&solution, &policy);
        assert!(violations.is_empty());
    }

    #[test]
    fn width_exactly_at_maximum_no_violation() {
        let mut ir = empty_ir();
        add_width_rule(&mut ir, 1, 0.1, 1.0, 0.2);
        let policy = DrcPolicy::build(&ir).unwrap();
        let solution = solution_with_segment(1.0);
        let violations = check_widths(&solution, &policy);
        assert!(violations.is_empty());
    }
}
