//! Via DRC: hole size, annular ring, via count per net, and hole-to-hole clearance.

use std::collections::BTreeMap;

use altium_format_types::pcb::RuleKind;
use autopcb_ir::{PcbIr, types::PointMm};
use autopcb_routes::{NetId, RouteSolution};

use super::{DrcObject, DrcViolation, DrcViolationKind};
use super::policy::DrcPolicy;

/// Check via constraints for the entire route solution.
///
/// Checks per-via: drill size bounds and annular ring minimum.
/// Checks per-net: maximum via count (if configured).
/// Checks all pairs: hole-to-hole clearance between via centers.
///
/// Looks up each net's net class from `ir` for scoped via rule cascade.
pub fn check_vias(
    solution: &RouteSolution,
    policy: &DrcPolicy,
    ir: &PcbIr,
) -> Vec<DrcViolation> {
    let mut violations = Vec::new();

    // Use global via_bounds for annular ring and hole-to-hole (not net-class scoped).
    let bounds = policy.global_via_bounds();

    // Collect all vias across all nets for pair-checking.
    let all_vias: Vec<_> = solution
        .nets
        .values()
        .flat_map(|n| n.vias.iter())
        .collect();

    // Per-via checks: drill size and annular ring.
    for via in &all_vias {
        let center = PointMm { x: via.position.x, y: via.position.y };

        let via_net_class: Option<&str> = super::net_class_for_net(ir, via.net_id);
        let via_bounds = policy.via_bounds_for(via_net_class);

        if via.drill_mm < via_bounds.hole_min_mm {
            violations.push(DrcViolation {
                kind: DrcViolationKind::HoleSizeBelowMinimum,
                rule_kind: RuleKind::MaxMinHoleSize,
                rule_name: "MaxMinHoleSize".to_string(),
                object_a: DrcObject::Via((*via).clone()),
                object_b: None,
                location: center,
                layer: None,
                actual_mm: via.drill_mm,
                required_mm: via_bounds.hole_min_mm,
            });
        } else if via.drill_mm > via_bounds.hole_max_mm {
            violations.push(DrcViolation {
                kind: DrcViolationKind::HoleSizeAboveMaximum,
                rule_kind: RuleKind::MaxMinHoleSize,
                rule_name: "MaxMinHoleSize".to_string(),
                object_a: DrcObject::Via((*via).clone()),
                object_b: None,
                location: center,
                layer: None,
                actual_mm: via.drill_mm,
                required_mm: via_bounds.hole_max_mm,
            });
        }

        if via.annular_ring_mm < via_bounds.annular_ring_min_mm {
            violations.push(DrcViolation {
                kind: DrcViolationKind::AnnularRingBelowMinimum,
                rule_kind: RuleKind::MinimumAnnularRing,
                rule_name: "MinimumAnnularRing".to_string(),
                object_a: DrcObject::Via((*via).clone()),
                object_b: None,
                location: center,
                layer: None,
                actual_mm: via.annular_ring_mm,
                required_mm: via_bounds.annular_ring_min_mm,
            });
        }
    }

    // Per-net via count check.
    if let Some(max_count) = bounds.max_via_count {
        let mut count_by_net: BTreeMap<NetId, u32> = BTreeMap::new();
        for net in solution.nets.values() {
            count_by_net.insert(net.net_id, net.vias.len() as u32);
        }
        for (net_id, count) in &count_by_net {
            if *count > max_count {
                // Use the first via of this net as the representative location.
                let representative_via = solution.nets[net_id].vias.first().unwrap();
                let center = PointMm {
                    x: representative_via.position.x,
                    y: representative_via.position.y,
                };
                violations.push(DrcViolation {
                    kind: DrcViolationKind::MaximumViaCountExceeded,
                    rule_kind: RuleKind::MaximumViaCount,
                    rule_name: "MaximumViaCount".to_string(),
                    object_a: DrcObject::Via(representative_via.clone()),
                    object_b: None,
                    location: center,
                    layer: None,
                    actual_mm: *count as f64,
                    required_mm: max_count as f64,
                });
            }
        }
    }

    // Hole-to-hole clearance: check all pairs of vias.
    //
    // Distance between via edges = distance between centers minus both radii
    // (each radius = drill_mm / 2).
    let min_clearance = bounds.hole_to_hole_clearance_mm;
    for i in 0..all_vias.len() {
        for j in (i + 1)..all_vias.len() {
            let a = all_vias[i];
            let b = all_vias[j];
            let dx = b.position.x - a.position.x;
            let dy = b.position.y - a.position.y;
            let center_dist = (dx * dx + dy * dy).sqrt();
            let edge_dist = center_dist - a.drill_mm / 2.0 - b.drill_mm / 2.0;
            if edge_dist < min_clearance {
                let mid = PointMm {
                    x: (a.position.x + b.position.x) / 2.0,
                    y: (a.position.y + b.position.y) / 2.0,
                };
                violations.push(DrcViolation {
                    kind: DrcViolationKind::HoleToHoleClearance,
                    rule_kind: RuleKind::HoleToHoleClearance,
                    rule_name: "HoleToHoleClearance".to_string(),
                    object_a: DrcObject::Via((*a).clone()),
                    object_b: Some(DrcObject::Via((*b).clone())),
                    location: mid,
                    layer: None,
                    actual_mm: edge_dist,
                    required_mm: min_clearance,
                });
            }
        }
    }

    violations
}

/// Check for vias placed directly under SMD pads.
///
/// STUB: Not yet implemented. Requires SMD pad identification in PcbIr
/// (pad type: SMD vs through-hole) which is not yet extracted.
pub fn check_vias_under_smd(
    _solution: &autopcb_routes::RouteSolution,
    _ir: &autopcb_ir::PcbIr,
) -> Vec<super::DrcViolation> {
    Vec::new()
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
    use autopcb_routes::{LayerId, NetId, Point, RoutedNet, RouteSolution, RoutedVia};

    use super::super::test_helpers::empty_ir;

    fn add_annular_ring_rule(ir: &mut PcbIr, min_mm: f64) {
        let id = ir.rules.push(IrDesignRule {
            id: RuleId::from(0u32),
            name: "MinimumAnnularRing".into(),
            kind: RuleKind::MinimumAnnularRing,
            priority: 1,
            enabled: true,
            scope: IrRuleScopePair::default(),
            params: IrRuleParams::MinimumAnnularRing { min_mm },
        });
        ir.rules[id].id = id;
    }

    fn make_via(net_id: NetId, x: f64, y: f64, drill_mm: f64, annular_ring_mm: f64) -> RoutedVia {
        RoutedVia {
            net_id,
            position: Point { x, y },
            from_layer: LayerId(0),
            to_layer: LayerId(1),
            drill_mm,
            annular_ring_mm,
        }
    }

    fn solution_with_vias(vias: Vec<(NetId, RoutedVia)>) -> RouteSolution {
        let mut solution = RouteSolution::new();
        for (net_id, via) in vias {
            solution
                .nets
                .entry(net_id)
                .or_insert_with(|| RoutedNet {
                    net_id,
                    segments: vec![],
                    vias: vec![],
                    routed_length_mm: 0.0,
                })
                .vias
                .push(via);
        }
        solution
    }

    #[test]
    fn via_within_bounds_no_violation() {
        let ir = empty_ir();
        let policy = DrcPolicy::build(&ir).unwrap();
        let net_id = NetId(1);
        // Default bounds: hole 0.1–6.35, annular ring ≥ 0.05.
        let via = make_via(net_id, 0.0, 0.0, 0.3, 0.1);
        let solution = solution_with_vias(vec![(net_id, via)]);
        let violations = check_vias(&solution, &policy, &ir);
        assert!(violations.is_empty(), "expected no violations, got {:?}", violations);
    }

    #[test]
    fn annular_ring_below_minimum_generates_violation() {
        let mut ir = empty_ir();
        add_annular_ring_rule(&mut ir, 0.1);
        let policy = DrcPolicy::build(&ir).unwrap();
        let net_id = NetId(1);
        let via = make_via(net_id, 0.0, 0.0, 0.3, 0.05);
        let solution = solution_with_vias(vec![(net_id, via)]);
        let violations = check_vias(&solution, &policy, &ir);
        let annular_violations: Vec<_> = violations
            .iter()
            .filter(|v| v.kind == DrcViolationKind::AnnularRingBelowMinimum)
            .collect();
        assert_eq!(annular_violations.len(), 1);
        let v = &annular_violations[0];
        assert!((v.actual_mm - 0.05).abs() < f64::EPSILON);
        assert!((v.required_mm - 0.1).abs() < f64::EPSILON);
    }

    #[test]
    fn net_with_too_many_vias_generates_violation() {
        let ir = empty_ir();
        // Build a policy with max_via_count = 2 by manipulating via_bounds directly.
        // Since DrcPolicy::build() doesn't set max_via_count from rules (no IR variant yet),
        // we rebuild the policy here using the internal build path and override.
        let mut policy = DrcPolicy::build(&ir).unwrap();
        policy.via_bounds.max_via_count = Some(2);

        let net_id = NetId(1);
        let vias = vec![
            (net_id, make_via(net_id, 0.0, 0.0, 0.3, 0.1)),
            (net_id, make_via(net_id, 1.0, 0.0, 0.3, 0.1)),
            (net_id, make_via(net_id, 2.0, 0.0, 0.3, 0.1)),
        ];
        let solution = solution_with_vias(vias);
        let violations = check_vias(&solution, &policy, &ir);
        let count_violations: Vec<_> = violations
            .iter()
            .filter(|v| v.kind == DrcViolationKind::MaximumViaCountExceeded)
            .collect();
        assert_eq!(count_violations.len(), 1);
        assert!((count_violations[0].actual_mm - 3.0).abs() < f64::EPSILON);
        assert!((count_violations[0].required_mm - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn hole_to_hole_clearance_violation() {
        let ir = empty_ir();
        let policy = DrcPolicy::build(&ir).unwrap();
        // Default hole-to-hole clearance is 0.25 mm.
        // Two vias 0.1 mm apart (edge distance with drill 0.2: 0.1 - 0.1 - 0.1 = -0.1, < 0.25).
        let net_id = NetId(1);
        let via_a = make_via(net_id, 0.0, 0.0, 0.2, 0.1);
        let via_b = make_via(net_id, 0.1, 0.0, 0.2, 0.1);
        let solution = solution_with_vias(vec![(net_id, via_a), (net_id, via_b)]);
        let violations = check_vias(&solution, &policy, &ir);
        let h2h: Vec<_> = violations
            .iter()
            .filter(|v| v.kind == DrcViolationKind::HoleToHoleClearance)
            .collect();
        assert_eq!(h2h.len(), 1);
        assert!(h2h[0].actual_mm < h2h[0].required_mm,
            "actual {} should be less than required {}", h2h[0].actual_mm, h2h[0].required_mm);
    }

    #[test]
    fn hole_to_hole_within_clearance_no_violation() {
        let ir = empty_ir();
        let policy = DrcPolicy::build(&ir).unwrap();
        // Default hole-to-hole clearance is 0.25 mm.
        // Two vias 1.0 mm apart (edge distance with drill 0.2: 1.0 - 0.1 - 0.1 = 0.8, > 0.25).
        let net_id = NetId(1);
        let via_a = make_via(net_id, 0.0, 0.0, 0.2, 0.1);
        let via_b = make_via(net_id, 1.0, 0.0, 0.2, 0.1);
        let solution = solution_with_vias(vec![(net_id, via_a), (net_id, via_b)]);
        let violations = check_vias(&solution, &policy, &ir);
        let h2h: Vec<_> = violations
            .iter()
            .filter(|v| v.kind == DrcViolationKind::HoleToHoleClearance)
            .collect();
        assert!(h2h.is_empty(), "expected no h2h violations, got {:?}", h2h);
    }
}
