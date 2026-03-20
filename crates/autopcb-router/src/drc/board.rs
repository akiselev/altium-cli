//! Board outline clearance checks.
//!
//! Verifies that all copper objects in the route solution maintain the required
//! clearance from the board edge. When `ir.board.outline` has at least 3 points,
//! uses polygon-based distance (point to nearest outline edge). Falls back to
//! rectangular bounding-box distance when no polygon outline is available.

use altium_format_types::pcb::RuleKind;
use autopcb_ir::{types::PointMm, PcbIr};
use autopcb_routes::{RouteSolution, TraceSegment};

use super::{policy::DrcPolicy, DrcObject, DrcViolation, DrcViolationKind};

/// Minimum distance from point `p` to the nearest edge of a closed polygon
/// outline. Returns `f64::MAX` when the outline has fewer than 2 points.
fn point_to_outline_distance(p: &autopcb_routes::Point, outline: &[PointMm]) -> f64 {
    if outline.len() < 2 {
        return f64::MAX;
    }
    let mut min_dist = f64::MAX;
    for i in 0..outline.len() {
        let j = (i + 1) % outline.len();
        let a = autopcb_routes::Point { x: outline[i].x, y: outline[i].y };
        let b = autopcb_routes::Point { x: outline[j].x, y: outline[j].y };
        let d = super::clearance::point_to_segment_distance(*p, a, b);
        if d < min_dist {
            min_dist = d;
        }
    }
    min_dist
}

/// Check board outline clearance for all routed segments.
///
/// When `ir.board.outline` has at least 3 vertices, each segment endpoint is
/// checked against the polygon outline using point-to-nearest-edge distance.
/// Otherwise falls back to rectangular bounding-box distance.
pub fn check_board(
    solution: &RouteSolution,
    ir: &PcbIr,
    policy: &DrcPolicy,
) -> Vec<DrcViolation> {
    let clearance = policy.board_outline_clearance_mm;
    let mut violations = Vec::new();

    for net in solution.nets.values() {
        for seg in &net.segments {
            if ir.board.outline.len() >= 3 {
                // Check the full segment body against the board outline polygon,
                // not just endpoints — a diagonal segment could cross the board
                // edge midway even if both endpoints are inside.
                let dist = super::clearance::segment_to_polyline_distance(
                    seg.start,
                    seg.end,
                    &ir.board.outline,
                );
                if dist < clearance {
                    let mid = PointMm {
                        x: (seg.start.x + seg.end.x) / 2.0,
                        y: (seg.start.y + seg.end.y) / 2.0,
                    };
                    violations.push(DrcViolation {
                        kind: DrcViolationKind::BoardOutlineClearance,
                        rule_kind: RuleKind::BoardOutlineClearance,
                        rule_name: "Board Outline Clearance".to_string(),
                        object_a: DrcObject::Segment(seg.clone()),
                        object_b: Some(DrcObject::BoardEdge),
                        location: mid,
                        layer: Some(seg.layer),
                        actual_mm: dist,
                        required_mm: clearance,
                    });
                }
            } else {
                let bounds = &ir.board.bounds;
                check_segment_endpoint(seg, &seg.start, bounds, clearance, &mut violations);
                check_segment_endpoint(seg, &seg.end, bounds, clearance, &mut violations);
            }
        }
    }

    violations
}

/// Minimum distance from point `p` to the rectangular board boundary.
///
/// Returns the smallest of the four edge distances. A positive value means
/// the point is inside the board; a negative value means it is outside.
fn min_edge_distance(
    p: &autopcb_routes::Point,
    bounds: &autopcb_ir::types::BoundingBoxMm,
) -> f64 {
    let d_left = p.x - bounds.min.x;
    let d_right = bounds.max.x - p.x;
    let d_bottom = p.y - bounds.min.y;
    let d_top = bounds.max.y - p.y;
    d_left.min(d_right).min(d_bottom).min(d_top)
}

fn check_segment_endpoint(
    seg: &TraceSegment,
    point: &autopcb_routes::Point,
    bounds: &autopcb_ir::types::BoundingBoxMm,
    clearance: f64,
    violations: &mut Vec<DrcViolation>,
) {
    let dist = min_edge_distance(point, bounds);
    if dist < clearance {
        violations.push(DrcViolation {
            kind: DrcViolationKind::BoardOutlineClearance,
            rule_kind: RuleKind::BoardOutlineClearance,
            rule_name: "Board Outline Clearance".to_string(),
            object_a: DrcObject::Segment(seg.clone()),
            object_b: Some(DrcObject::BoardEdge),
            location: PointMm { x: point.x, y: point.y },
            layer: Some(seg.layer),
            actual_mm: dist.max(0.0),
            required_mm: clearance,
        });
    }
}

/// Check component courtyard overlap.
///
/// STUB: Not yet implemented. Requires component courtyard/bounding-box
/// data from PcbIr. Will iterate all component pairs and check overlap
/// against `policy.component_clearance_mm`.
pub fn check_component_clearance(
    _solution: &autopcb_routes::RouteSolution,
    _ir: &autopcb_ir::PcbIr,
    _policy: &super::policy::DrcPolicy,
) -> Vec<super::DrcViolation> {
    Vec::new()
}

/// Check creepage distance (minimum surface-path distance for high voltage).
///
/// STUB: Requires surface-path distance computation. Will emit
/// `DrcViolationKind::CreepageViolation` when implemented.
pub fn check_creepage(
    _solution: &autopcb_routes::RouteSolution,
    _ir: &autopcb_ir::PcbIr,
    _policy: &super::policy::DrcPolicy,
) -> Vec<super::DrcViolation> {
    Vec::new()
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
    use autopcb_routes::{LayerId as RouteLayerId, NetId, Point, RoutedNet, TraceSegment};
    use altium_format_types::pcb::RuleKind;
    use autopcb_ir::rule::{IrDesignRule, IrRuleParams};
    use autopcb_ir::handles::RuleId;

    fn make_ir(
        min: (f64, f64),
        max: (f64, f64),
        board_clearance_mm: f64,
    ) -> PcbIr {
        let mut ir = PcbIr {
            board: IrBoardGeometry {
                outline: vec![],
                cutouts: vec![],
                bounds: BoundingBoxMm {
                    min: IrPointMm { x: min.0, y: min.1 },
                    max: IrPointMm { x: max.0, y: max.1 },
                },
                keepouts: vec![],
            },
            layer_stack: IrLayerStack {
                copper_layers: vec![
                    IrCopperLayer {
                        id: IrLayerId::from(0u32),
                        name: "Top Layer".into(),
                        is_top: true,
                        is_bottom: false,
                        preferred_direction: Some(PreferredDirection::Any),
                    },
                ],
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
        };
        let id = ir.rules.push(IrDesignRule {
            id: RuleId::from(0u32),
            name: "Board Outline Clearance".into(),
            kind: RuleKind::BoardOutlineClearance,
            priority: 1,
            enabled: true,
            params: IrRuleParams::BoardOutlineClearance { gap_mm: board_clearance_mm },
        });
        ir.rules[id].id = id;
        ir
    }

    fn make_solution_with_segment(
        start: (f64, f64),
        end: (f64, f64),
    ) -> RouteSolution {
        let net_id = NetId(1);
        let layer = RouteLayerId(0);
        let seg = TraceSegment {
            net_id,
            layer,
            start: Point { x: start.0, y: start.1 },
            end: Point { x: end.0, y: end.1 },
            width_mm: 0.2,
        };
        let mut solution = RouteSolution::new();
        solution.nets.insert(net_id, RoutedNet {
            net_id,
            segments: vec![seg],
            vias: vec![],
            routed_length_mm: 1.0,
        });
        solution
    }

    #[test]
    fn segment_well_within_board_passes() {
        // Board: (0,0) to (100,100), clearance: 0.3mm
        // Segment: (10,10) to (90,90) — all endpoints 10mm from any edge
        let ir = make_ir((0.0, 0.0), (100.0, 100.0), 0.3);
        let policy = DrcPolicy::build(&ir).unwrap();
        let solution = make_solution_with_segment((10.0, 10.0), (90.0, 90.0));
        let violations = check_board(&solution, &ir, &policy);
        assert!(
            violations.is_empty(),
            "expected no violations, got {}: {:?}",
            violations.len(),
            violations.iter().map(|v| v.actual_mm).collect::<Vec<_>>()
        );
    }

    #[test]
    fn segment_near_board_edge_produces_violation() {
        // Board: (0,0) to (100,100), clearance: 0.3mm
        // Start point at x=0.1 → distance to left edge = 0.1mm < 0.3mm clearance
        let ir = make_ir((0.0, 0.0), (100.0, 100.0), 0.3);
        let policy = DrcPolicy::build(&ir).unwrap();
        let solution = make_solution_with_segment((0.1, 50.0), (50.0, 50.0));
        let violations = check_board(&solution, &ir, &policy);
        assert_eq!(violations.len(), 1, "expected 1 violation, got {}", violations.len());
        let v = &violations[0];
        assert_eq!(v.kind, DrcViolationKind::BoardOutlineClearance);
        assert!(
            (v.actual_mm - 0.1).abs() < 1e-10,
            "expected actual_mm=0.1, got {}",
            v.actual_mm
        );
        assert!(
            (v.required_mm - 0.3).abs() < 1e-10,
            "expected required_mm=0.3, got {}",
            v.required_mm
        );
    }

    #[test]
    fn both_endpoints_near_edge_produce_two_violations() {
        // Both endpoints within 0.3mm of an edge
        let ir = make_ir((0.0, 0.0), (100.0, 100.0), 0.3);
        let policy = DrcPolicy::build(&ir).unwrap();
        let solution = make_solution_with_segment((0.1, 50.0), (99.9, 50.0));
        let violations = check_board(&solution, &ir, &policy);
        assert_eq!(violations.len(), 2, "expected 2 violations, got {}", violations.len());
    }

    #[test]
    fn violation_actual_less_than_required() {
        let ir = make_ir((0.0, 0.0), (100.0, 100.0), 0.5);
        let policy = DrcPolicy::build(&ir).unwrap();
        let solution = make_solution_with_segment((0.2, 50.0), (50.0, 50.0));
        let violations = check_board(&solution, &ir, &policy);
        assert_eq!(violations.len(), 1);
        let v = &violations[0];
        assert!(
            v.actual_mm < v.required_mm,
            "actual_mm {} must be < required_mm {}",
            v.actual_mm, v.required_mm
        );
    }
}
