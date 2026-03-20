//! Geometry DRC: acute angle detection between consecutive segments, and
//! SMD-to-corner clearance checking.

use altium_format_types::pcb::RuleKind;
use autopcb_ir::{types::PointMm, PcbIr};
use autopcb_routes::{Point, RouteSolution};

use super::{DrcObject, DrcViolation, DrcViolationKind};
use super::policy::DrcPolicy;

/// Check geometric constraints: acute angles between consecutive segments,
/// and SMD-to-corner clearance.
///
/// For each net, finds consecutive segment pairs where one segment's end
/// equals another segment's start on the same layer. Computes the interior
/// angle at the junction and flags it as `AcuteAngle` when the angle is
/// below `policy.acute_angle_min_deg`.
///
/// Angle is computed via the dot product of the two direction vectors
/// originating from the shared junction point:
///
/// ```text
/// cos θ = (v1 · v2) / (|v1| × |v2|)
/// ```
///
/// where v1 points *away* from the junction along the incoming segment and
/// v2 points *away* from the junction along the outgoing segment.
///
/// SMD-to-corner: for every bend (junction) in the route, if the junction
/// point is within `policy.smd_to_corner_clearance_mm` of an SMD pad
/// (`!is_through_hole`) on the same layer and on a different net, a
/// `SmdToCorner` violation is emitted.
pub fn check_geometry(
    solution: &RouteSolution,
    policy: &DrcPolicy,
    ir: &PcbIr,
) -> Vec<DrcViolation> {
    let threshold_deg = policy.acute_angle_min_deg;
    let mut violations = Vec::new();

    for routed_net in solution.nets.values() {
        let segments = &routed_net.segments;

        // For each segment i, look for a segment j whose start matches i's end
        // on the same layer.
        for i in 0..segments.len() {
            let seg_i = &segments[i];
            for j in 0..segments.len() {
                if i == j {
                    continue;
                }
                let seg_j = &segments[j];
                if seg_i.layer != seg_j.layer {
                    continue;
                }
                if !points_equal(&seg_i.end, &seg_j.start) {
                    continue;
                }
                // Shared junction is seg_i.end == seg_j.start.
                let junction = &seg_i.end;

                // v1: direction from junction back along seg_i (toward seg_i.start).
                let v1x = seg_i.start.x - junction.x;
                let v1y = seg_i.start.y - junction.y;
                // v2: direction from junction forward along seg_j (toward seg_j.end).
                let v2x = seg_j.end.x - junction.x;
                let v2y = seg_j.end.y - junction.y;

                let len1 = (v1x * v1x + v1y * v1y).sqrt();
                let len2 = (v2x * v2x + v2y * v2y).sqrt();

                // Skip degenerate (zero-length) segments.
                if len1 < f64::EPSILON || len2 < f64::EPSILON {
                    continue;
                }

                let cos_theta = (v1x * v2x + v1y * v2y) / (len1 * len2);
                // Clamp to [-1, 1] to guard against floating-point rounding.
                let cos_theta = cos_theta.clamp(-1.0, 1.0);
                let angle_deg = cos_theta.acos().to_degrees();

                if angle_deg < threshold_deg {
                    violations.push(DrcViolation {
                        kind: DrcViolationKind::AcuteAngle,
                        rule_kind: RuleKind::AcuteAngle,
                        rule_name: "AcuteAngle".to_string(),
                        object_a: DrcObject::Segment(seg_i.clone()),
                        object_b: Some(DrcObject::Segment(seg_j.clone())),
                        location: PointMm { x: junction.x, y: junction.y },
                        layer: Some(seg_i.layer),
                        actual_mm: angle_deg,
                        required_mm: threshold_deg,
                    });
                }

                // --- SMD-to-corner ---
                // When a trace bends at a junction, check whether the bend
                // point is too close to an SMD pad on a different net.
                if policy.smd_to_corner_clearance_mm > 0.0 {
                    let required = policy.smd_to_corner_clearance_mm;
                    let jx = junction.x;
                    let jy = junction.y;
                    for (_comp_id, comp) in ir.components.iter() {
                        for pad in &comp.pads {
                            // Only SMD pads.
                            if pad.is_through_hole {
                                continue;
                            }
                            // Only pads on the same layer as the segment junction.
                            let seg_ir_layer = autopcb_ir::handles::LayerId::from(seg_i.layer.0 as u32);
                            if !pad.layer_set.contains(&seg_ir_layer) {
                                continue;
                            }
                            // Skip pads on the same net as the routed segment.
                            if pad.net.map(|n| n.raw()) == Some(routed_net.net_id.0 as u32) {
                                continue;
                            }
                            let pad_radius =
                                (pad.shape.size_x.max(pad.shape.size_y)) / 2.0;
                            let dx = jx - pad.world_position.x;
                            let dy = jy - pad.world_position.y;
                            let dist = (dx * dx + dy * dy).sqrt();
                            let actual = dist - pad_radius;
                            if actual < required {
                                violations.push(DrcViolation {
                                    kind: DrcViolationKind::SmdToCorner,
                                    rule_kind: RuleKind::SmdToCorner,
                                    rule_name: "SmdToCorner".to_string(),
                                    object_a: DrcObject::Segment(seg_i.clone()),
                                    object_b: Some(DrcObject::Pad {
                                        component: comp.designator.clone(),
                                        pad: pad.name.clone(),
                                        position: pad.world_position,
                                    }),
                                    location: PointMm { x: jx, y: jy },
                                    layer: Some(seg_i.layer),
                                    actual_mm: actual,
                                    required_mm: required,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    violations
}

/// Compare two points with a small epsilon for floating-point equality.
fn points_equal(a: &Point, b: &Point) -> bool {
    const EPS: f64 = 1e-9;
    (a.x - b.x).abs() < EPS && (a.y - b.y).abs() < EPS
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use autopcb_ir::{
        handles::{IdMap, LayerId as IrLayerId},
        layer_stack::{IrCopperLayer, IrLayerStack, PreferredDirection},
        types::{BoundingBoxMm, PointMm as IrPointMm},
        IrBoardGeometry, PcbIr,
    };
    use autopcb_routes::{LayerId, NetId, RoutedNet, RouteSolution, TraceSegment};

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
        }
    }

    /// Build a solution with two consecutive segments meeting at a given angle.
    ///
    /// `angle_deg` is the angle (in degrees) measured between the two outgoing
    /// direction vectors at the junction. The first segment runs horizontally
    /// to the right and ends at the origin; the second segment departs at the
    /// given angle relative to the positive-x axis (pointing right), so:
    ///   - 90° → perpendicular (valid)
    ///   - 30° → acute relative to horizontal leg (invalid)
    fn solution_with_angle(angle_deg: f64) -> RouteSolution {
        let net_id = NetId(1);
        let layer = LayerId(0);

        // seg_a: from (-1, 0) to (0, 0) — runs right, ends at junction.
        let seg_a = TraceSegment {
            net_id,
            layer,
            start: Point { x: -1.0, y: 0.0 },
            end: Point { x: 0.0, y: 0.0 },
            width_mm: 0.2,
        };

        // seg_b: departs from junction (0,0) in the direction of `angle_deg`
        // measured from positive-x axis.
        let angle_rad = angle_deg.to_radians();
        let seg_b = TraceSegment {
            net_id,
            layer,
            start: Point { x: 0.0, y: 0.0 },
            end: Point {
                x: angle_rad.cos(),
                y: angle_rad.sin(),
            },
            width_mm: 0.2,
        };

        let routed_net = RoutedNet {
            net_id,
            segments: vec![seg_a, seg_b],
            vias: vec![],
            routed_length_mm: 2.0,
        };
        let mut solution = RouteSolution::new();
        solution.nets.insert(net_id, routed_net);
        solution
    }

    #[test]
    fn ninety_degree_angle_no_violation() {
        let ir = empty_ir();
        let policy = DrcPolicy::build(&ir).unwrap();
        // 90° departure → interior angle = 90° ≥ 45° threshold.
        let solution = solution_with_angle(90.0);
        let violations = check_geometry(&solution, &policy, &ir);
        assert!(violations.is_empty(), "expected no violations at 90°, got {:?}", violations);
    }

    #[test]
    fn thirty_degree_angle_generates_violation() {
        let ir = empty_ir();
        let policy = DrcPolicy::build(&ir).unwrap();
        // 30° departure → interior angle between incoming (-x direction) and
        // outgoing (30° from +x) = 180° - 30° = 150°. Wait — re-check:
        // v1 = incoming reversed = (1,0) (from junction toward seg_a.start = (-1,0) flipped)
        //   No: v1x = seg_a.start.x - junction.x = -1 - 0 = -1, v1y = 0.
        //   So v1 = (-1, 0) pointing left.
        // v2 = seg_b direction = (cos30°, sin30°) ≈ (0.866, 0.5) pointing upper-right.
        // angle = arccos(v1·v2 / |v1||v2|) = arccos(-0.866) ≈ 150°.
        //
        // So a 30° departure from horizontal gives 150° interior angle — that's NOT acute.
        // To get an acute interior angle, we need the departure angle to be > 135° from +x
        // (so the interior angle is < 45°). Use 160° departure.
        let solution = solution_with_angle(160.0);
        let violations = check_geometry(&solution, &policy, &ir);
        // v1 = (-1, 0), v2 = (cos160°, sin160°) ≈ (-0.940, 0.342)
        // dot = (-1)(-0.940) + (0)(0.342) = 0.940
        // angle = arccos(0.940) ≈ 20° < 45° → violation.
        let acute: Vec<_> = violations
            .iter()
            .filter(|v| v.kind == DrcViolationKind::AcuteAngle)
            .collect();
        assert!(!acute.is_empty(), "expected AcuteAngle violation at ~20° interior angle");
        assert!(acute[0].actual_mm < policy.acute_angle_min_deg,
            "actual angle {} should be < {}", acute[0].actual_mm, policy.acute_angle_min_deg);
    }

    #[test]
    fn exactly_forty_five_degree_interior_no_violation() {
        let ir = empty_ir();
        let policy = DrcPolicy::build(&ir).unwrap();
        // v1 = (-1, 0). We want interior angle = exactly 45°.
        // arccos(v1·v2) = 45° → v1·v2 = cos(45°) ≈ 0.7071.
        // v2 = (cos(135°), sin(135°)) ≈ (-0.7071, 0.7071).
        // dot = (-1)(-0.7071) + (0)(0.7071) = 0.7071. angle = 45°. No violation.
        let solution = solution_with_angle(135.0);
        let violations = check_geometry(&solution, &policy, &ir);
        let acute: Vec<_> = violations
            .iter()
            .filter(|v| v.kind == DrcViolationKind::AcuteAngle)
            .collect();
        assert!(acute.is_empty(),
            "expected no violation at exactly 45° interior angle, got {:?}", acute);
    }

    #[test]
    fn segments_on_different_layers_not_checked() {
        let ir = empty_ir();
        let policy = DrcPolicy::build(&ir).unwrap();
        let net_id = NetId(1);
        // Two segments meeting at (0,0) but on different layers.
        let seg_a = TraceSegment {
            net_id,
            layer: LayerId(0),
            start: Point { x: -1.0, y: 0.0 },
            end: Point { x: 0.0, y: 0.0 },
            width_mm: 0.2,
        };
        let seg_b = TraceSegment {
            net_id,
            layer: LayerId(1),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 1.0, y: 0.0 },
            width_mm: 0.2,
        };
        let routed_net = RoutedNet {
            net_id,
            segments: vec![seg_a, seg_b],
            vias: vec![],
            routed_length_mm: 2.0,
        };
        let mut solution = RouteSolution::new();
        solution.nets.insert(net_id, routed_net);
        let violations = check_geometry(&solution, &policy, &ir);
        assert!(violations.is_empty(),
            "different-layer segments should not be angle-checked, got {:?}", violations);
    }

    /// SMD pad too close to a trace bend produces a SmdToCorner violation.
    #[test]
    fn smd_to_corner_violation_detected() {
        use autopcb_ir::{
            component::{IrComponentPad, IrComponent, PadShapeInfo, PadShapeKind},
            handles::{ComponentId, PadId, NetId as IrNetId},
            types::BoardSide,
            rule::{IrDesignRule, IrRuleParams},
            handles::RuleId,
        };
        use altium_format_types::pcb::RuleKind as AltiumRuleKind;

        let mut ir = empty_ir();

        // Add a SmdToCorner rule with 0.5 mm clearance.
        let rule_id = ir.rules.push(IrDesignRule {
            id: RuleId::from(0u32),
            name: "SmdToCorner".into(),
            kind: AltiumRuleKind::SmdToCorner,
            priority: 1,
            enabled: true,
            params: IrRuleParams::SmdToCorner { clearance_mm: 0.5 },
        });
        ir.rules[rule_id].id = rule_id;

        // Place an SMD pad at (0.1, 0.0) with 0.1 mm radius (size_x = 0.2).
        // The bend junction will be at (0.0, 0.0), distance to pad center = 0.1.
        // actual = 0.1 - 0.05 = 0.05 < required 0.5 → violation.
        let pad = IrComponentPad {
            id: PadId::from(0u32),
            name: "1".into(),
            local_position: PointMm { x: 0.0, y: 0.0 },
            world_position: PointMm { x: 0.1, y: 0.0 },
            net: Some(IrNetId::from(99u32)),
            shape: PadShapeInfo {
                kind: PadShapeKind::Round,
                size_x: 0.1,
                size_y: 0.1,
                rotation: 0.0,
            },
            is_through_hole: false,
            hole_size_mm: 0.0,
            swap_id_pin: None,
            swap_id_part: None,
            layer_set: vec![IrLayerId::from(0u32)],
        };
        let zero_bb = autopcb_ir::types::BoundingBoxMm::new(
            PointMm { x: 0.0, y: 0.0 },
            PointMm { x: 0.0, y: 0.0 },
        );
        let comp_id = ir.components.push(IrComponent {
            id: ComponentId::from(0u32),
            designator: "U1".into(),
            pattern: "SMD".into(),
            value: "Test".into(),
            position: PointMm { x: 0.0, y: 0.0 },
            rotation: 0.0,
            side: BoardSide::Top,
            local_bounds: zero_bb,
            world_bounds: zero_bb,
            pads: vec![pad],
        });
        ir.components[comp_id].id = comp_id;

        let policy = DrcPolicy::build(&ir).unwrap();
        assert!(policy.smd_to_corner_clearance_mm > 0.0,
            "policy should have non-zero smd_to_corner_clearance_mm");

        // Build a solution with a bend at (0,0): seg_a ends at (0,0), seg_b starts at (0,0).
        let net_id = NetId(0); // different net from pad (net 99)
        let layer = LayerId(0);
        let seg_a = TraceSegment {
            net_id,
            layer,
            start: Point { x: -1.0, y: 0.0 },
            end: Point { x: 0.0, y: 0.0 },
            width_mm: 0.1,
        };
        let seg_b = TraceSegment {
            net_id,
            layer,
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 0.0, y: 1.0 },
            width_mm: 0.1,
        };
        let routed_net = RoutedNet {
            net_id,
            segments: vec![seg_a, seg_b],
            vias: vec![],
            routed_length_mm: 2.0,
        };
        let mut solution = RouteSolution::new();
        solution.nets.insert(net_id, routed_net);

        let violations = check_geometry(&solution, &policy, &ir);
        let smd_corners: Vec<_> = violations
            .iter()
            .filter(|v| v.kind == DrcViolationKind::SmdToCorner)
            .collect();
        assert!(!smd_corners.is_empty(), "expected SmdToCorner violation");
        assert!(smd_corners[0].actual_mm < smd_corners[0].required_mm,
            "actual {} must be < required {}", smd_corners[0].actual_mm, smd_corners[0].required_mm);
    }
}
