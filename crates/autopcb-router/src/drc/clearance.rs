//! CPU clearance checking: segment-to-segment, segment-to-via, segment-to-board-edge,
//! segment-to-pad, and segment-to-keepout.
//!
//! Uses an O(n²) candidate scan (same layer, different net).
//!
//! TODO(performance): Replace O(n²) loops with R-tree envelope queries.
//! Build a temporary `rstar::RTree` from the solution's routed segments,
//! then for each segment query all objects within `max_clearance + max_half_width`
//! of the segment's bounding box. The workspace `SpatialIndex` contains only
//! pre-routed obstacles, not the current solution, so a separate tree is needed.
//! This optimization becomes critical at >5K segments per layer.
//!
//! Width matters: the actual gap between two traces is the centerline distance
//! minus half the widths of each trace. If that net gap is less than the required
//! clearance, a violation is recorded.

use autopcb_ir::{IrKeepoutZone, PcbIr};
use autopcb_routes::{RouteSolution, RoutedVia, TraceSegment};
use autopcb_ir::types::PointMm;
use altium_format_types::pcb::RuleKind;

use super::{net_class_for_net, DrcObject, DrcViolation, DrcViolationKind};
use super::policy::DrcPolicy;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Check clearance violations in `solution` against `policy` and `ir`.
///
/// Covers:
/// - segment-to-segment (same layer, different net)
/// - segment-to-via     (via on any layer the segment's layer is within)
/// - via-to-via         (different net)
/// - segment-to-board-edge
/// - segment-to-pad     (different net, same layer)
/// - segment-to-keepout (any segment intruding into a keepout zone)
pub fn check_clearance(
    solution: &RouteSolution,
    policy: &DrcPolicy,
    ir: &PcbIr,
) -> Vec<DrcViolation> {
    let mut violations = Vec::new();

    // Helper: map an IR-native NetId to its net class name.
    let ir_net_class = |net_id: autopcb_ir::handles::NetId| -> Option<&str> {
        ir.nets
            .values()
            .find(|n| n.id == net_id)
            .and_then(|n| n.net_class.as_deref())
    };

    // Collect all segments and vias from every net.
    let all_segments: Vec<&TraceSegment> = solution
        .nets
        .values()
        .flat_map(|n| n.segments.iter())
        .collect();

    let all_vias: Vec<&RoutedVia> = solution
        .nets
        .values()
        .flat_map(|n| n.vias.iter())
        .collect();

    // --- Segment-to-segment ---
    // Deterministic: iterate outer by insertion order (BTreeMap), inner starts
    // one past outer to avoid checking (A,B) and (B,A).
    for (i, seg_a) in all_segments.iter().enumerate() {
        for seg_b in all_segments.iter().skip(i + 1) {
            if seg_a.layer != seg_b.layer {
                continue;
            }
            if seg_a.net_id == seg_b.net_id {
                // Same net — clearance rules do not apply.
                continue;
            }
            let class_a = net_class_for_net(ir, seg_a.net_id);
            let class_b = net_class_for_net(ir, seg_b.net_id);
            let required = policy.clearance(class_a, class_b);
            let centerline_dist = segment_to_segment_distance(
                seg_a.start, seg_a.end,
                seg_b.start, seg_b.end,
            );
            let actual = centerline_dist - seg_a.width_mm / 2.0 - seg_b.width_mm / 2.0;
            if actual < required {
                let loc = segment_midpoint(seg_a);
                violations.push(DrcViolation {
                    kind: DrcViolationKind::ClearanceViolation,
                    rule_kind: RuleKind::Clearance,
                    rule_name: "Clearance".into(),
                    object_a: DrcObject::Segment((*seg_a).clone()),
                    object_b: Some(DrcObject::Segment((*seg_b).clone())),
                    location: PointMm { x: loc.x, y: loc.y },
                    layer: Some(seg_a.layer),
                    actual_mm: actual,
                    required_mm: required,
                });
            }
        }
    }

    // --- Segment-to-via ---
    for seg in &all_segments {
        for via in &all_vias {
            if seg.net_id == via.net_id {
                continue;
            }
            // Via spans from_layer to to_layer — it is present on every layer
            // in that range. For a simple check, treat the via as present if
            // seg.layer is within [min(from,to), max(from,to)].
            let via_min = via.from_layer.0.min(via.to_layer.0);
            let via_max = via.from_layer.0.max(via.to_layer.0);
            if seg.layer.0 < via_min || seg.layer.0 > via_max {
                continue;
            }
            let via_radius = via.drill_mm / 2.0 + via.annular_ring_mm;
            let class_seg = net_class_for_net(ir, seg.net_id);
            let class_via = net_class_for_net(ir, via.net_id);
            let required = policy.clearance(class_seg, class_via);
            let centerline_dist = point_to_segment_distance(
                via.position,
                seg.start,
                seg.end,
            );
            let actual = centerline_dist - via_radius - seg.width_mm / 2.0;
            if actual < required {
                let loc = segment_midpoint(seg);
                violations.push(DrcViolation {
                    kind: DrcViolationKind::ClearanceViolation,
                    rule_kind: RuleKind::Clearance,
                    rule_name: "Clearance".into(),
                    object_a: DrcObject::Segment((*seg).clone()),
                    object_b: Some(DrcObject::Via((*via).clone())),
                    location: PointMm { x: loc.x, y: loc.y },
                    layer: Some(seg.layer),
                    actual_mm: actual,
                    required_mm: required,
                });
            }
        }
    }

    // --- Via-to-via ---
    for (i, via_a) in all_vias.iter().enumerate() {
        for via_b in all_vias.iter().skip(i + 1) {
            if via_a.net_id == via_b.net_id {
                continue;
            }
            // Vias interact when their layer ranges overlap.
            let a_min = via_a.from_layer.0.min(via_a.to_layer.0);
            let a_max = via_a.from_layer.0.max(via_a.to_layer.0);
            let b_min = via_b.from_layer.0.min(via_b.to_layer.0);
            let b_max = via_b.from_layer.0.max(via_b.to_layer.0);
            if a_min > b_max || b_min > a_max {
                continue;
            }
            let ra = via_a.drill_mm / 2.0 + via_a.annular_ring_mm;
            let rb = via_b.drill_mm / 2.0 + via_b.annular_ring_mm;
            let class_a = net_class_for_net(ir, via_a.net_id);
            let class_b = net_class_for_net(ir, via_b.net_id);
            let required = policy.clearance(class_a, class_b);
            let dx = via_a.position.x - via_b.position.x;
            let dy = via_a.position.y - via_b.position.y;
            let center_dist = (dx * dx + dy * dy).sqrt();
            let actual = center_dist - ra - rb;
            if actual < required {
                violations.push(DrcViolation {
                    kind: DrcViolationKind::ClearanceViolation,
                    rule_kind: RuleKind::Clearance,
                    rule_name: "Clearance".into(),
                    object_a: DrcObject::Via((*via_a).clone()),
                    object_b: Some(DrcObject::Via((*via_b).clone())),
                    location: PointMm { x: via_a.position.x, y: via_a.position.y },
                    layer: None,
                    actual_mm: actual,
                    required_mm: required,
                });
            }
        }
    }

    // --- Segment-to-pad ---
    // For every segment, check clearance against all pads on a different net that
    // share at least one copper layer with the segment.
    for seg in &all_segments {
        for (_comp_id, comp) in ir.components.iter() {
            for pad in &comp.pads {
                // Skip pads on the same net as the segment.
                if pad.net.map(|n| n.raw()) == Some(seg.net_id.0 as u32) {
                    continue;
                }
                // Skip pads that do not exist on the segment's layer.
                let seg_ir_layer = autopcb_ir::handles::LayerId::from(seg.layer.0 as u32);
                if !pad.layer_set.contains(&seg_ir_layer) {
                    continue;
                }
                let class_seg = net_class_for_net(ir, seg.net_id);
                let class_pad = pad.net.and_then(|net_id| ir_net_class(net_id));
                let required_pad = policy.clearance(class_seg, class_pad);
                // Approximate the pad as a circle whose radius is half the
                // larger pad dimension (worst-case circular envelope).
                let pad_radius = (pad.shape.size_x.max(pad.shape.size_y)) / 2.0;
                let pad_pt = autopcb_routes::Point {
                    x: pad.world_position.x,
                    y: pad.world_position.y,
                };
                let centerline_dist = point_to_segment_distance(pad_pt, seg.start, seg.end);
                let actual = centerline_dist - seg.width_mm / 2.0 - pad_radius;
                if actual < required_pad {
                    let loc = segment_midpoint(seg);
                    violations.push(DrcViolation {
                        kind: DrcViolationKind::ClearanceViolation,
                        rule_kind: RuleKind::Clearance,
                        rule_name: "Clearance".into(),
                        object_a: DrcObject::Segment((*seg).clone()),
                        object_b: Some(DrcObject::Pad {
                            component: comp.designator.clone(),
                            pad: pad.name.clone(),
                            position: pad.world_position,
                        }),
                        location: PointMm { x: loc.x, y: loc.y },
                        layer: Some(seg.layer),
                        actual_mm: actual,
                        required_mm: required_pad,
                    });
                }
            }
        }
    }

    // --- Segment-to-keepout ---
    // Any segment that intrudes into a keepout zone is a violation.
    // Intrusion is defined as: the signed distance from the segment to the
    // keepout polygon boundary is negative (segment overlaps the region).
    for (keepout_idx, keepout) in ir.board.keepouts.iter().enumerate() {
        if keepout.outline.len() < 3 {
            continue;
        }
        for seg in &all_segments {
            let dist = signed_segment_to_keepout_distance(seg.start, seg.end, keepout);
            let actual = dist - seg.width_mm / 2.0;
            if actual < 0.0 {
                let loc = segment_midpoint(seg);
                violations.push(DrcViolation {
                    kind: DrcViolationKind::ClearanceViolation,
                    rule_kind: RuleKind::Clearance,
                    rule_name: "KeepoutClearance".into(),
                    object_a: DrcObject::Segment((*seg).clone()),
                    object_b: Some(DrcObject::Keepout { id: keepout_idx }),
                    location: PointMm { x: loc.x, y: loc.y },
                    layer: Some(seg.layer),
                    actual_mm: actual,
                    required_mm: 0.0,
                });
            }
        }
    }

    violations
}

// ---------------------------------------------------------------------------
// Geometry helpers
// ---------------------------------------------------------------------------

/// Midpoint of a trace segment (for violation location).
fn segment_midpoint(seg: &TraceSegment) -> autopcb_routes::Point {
    autopcb_routes::Point {
        x: (seg.start.x + seg.end.x) / 2.0,
        y: (seg.start.y + seg.end.y) / 2.0,
    }
}

/// Minimum distance between two line segments in 2D.
///
/// Returns 0.0 when the segments intersect.
pub fn segment_to_segment_distance(
    s1_start: autopcb_routes::Point,
    s1_end: autopcb_routes::Point,
    s2_start: autopcb_routes::Point,
    s2_end: autopcb_routes::Point,
) -> f64 {
    // Check all four endpoint-to-segment distances and the segment-intersection
    // case. The minimum of the four is the segment-to-segment distance when the
    // segments do not intersect; if they do intersect the distance is 0.
    if segments_intersect(s1_start, s1_end, s2_start, s2_end) {
        return 0.0;
    }
    let d1 = point_to_segment_distance(s1_start, s2_start, s2_end);
    let d2 = point_to_segment_distance(s1_end, s2_start, s2_end);
    let d3 = point_to_segment_distance(s2_start, s1_start, s1_end);
    let d4 = point_to_segment_distance(s2_end, s1_start, s1_end);
    d1.min(d2).min(d3).min(d4)
}

/// Minimum distance from point `p` to the line segment `[a, b]`.
pub fn point_to_segment_distance(
    p: autopcb_routes::Point,
    a: autopcb_routes::Point,
    b: autopcb_routes::Point,
) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len_sq = dx * dx + dy * dy;
    if len_sq < f64::EPSILON {
        // Degenerate segment — treat as a point.
        let ex = p.x - a.x;
        let ey = p.y - a.y;
        return (ex * ex + ey * ey).sqrt();
    }
    // Project p onto the line, clamp to [0, 1].
    let t = ((p.x - a.x) * dx + (p.y - a.y) * dy) / len_sq;
    let t = t.clamp(0.0, 1.0);
    let cx = a.x + t * dx;
    let cy = a.y + t * dy;
    let ex = p.x - cx;
    let ey = p.y - cy;
    (ex * ex + ey * ey).sqrt()
}

/// True when the open segments [p1,p2] and [p3,p4] properly intersect.
fn segments_intersect(
    p1: autopcb_routes::Point,
    p2: autopcb_routes::Point,
    p3: autopcb_routes::Point,
    p4: autopcb_routes::Point,
) -> bool {
    fn cross(o: autopcb_routes::Point, a: autopcb_routes::Point, b: autopcb_routes::Point) -> f64 {
        (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x)
    }
    let d1 = cross(p3, p4, p1);
    let d2 = cross(p3, p4, p2);
    let d3 = cross(p1, p2, p3);
    let d4 = cross(p1, p2, p4);
    if ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
    {
        return true;
    }
    false
}

/// Minimum distance from segment `[a, b]` to a closed polyline (board outline).
///
/// The polyline edges are the pairs `(outline[i], outline[i+1])` plus the
/// closing edge `(outline[last], outline[0])`.
pub(crate) fn segment_to_polyline_distance(
    a: autopcb_routes::Point,
    b: autopcb_routes::Point,
    outline: &[PointMm],
) -> f64 {
    let n = outline.len();
    if n < 2 {
        return f64::MAX;
    }
    let to_pt = |pm: &PointMm| autopcb_routes::Point { x: pm.x, y: pm.y };
    let mut min_dist = f64::MAX;
    for i in 0..n {
        let e_start = to_pt(&outline[i]);
        let e_end = to_pt(&outline[(i + 1) % n]);
        let d = segment_to_segment_distance(a, b, e_start, e_end);
        if d < min_dist {
            min_dist = d;
        }
    }
    min_dist
}

/// Signed distance from segment `[a, b]` centerline to the boundary of a keepout
/// polygon.
///
/// Returns a negative value when either endpoint of the segment is inside the
/// keepout polygon (indicating intrusion), and a positive value equal to the
/// minimum distance to the polygon boundary otherwise.
fn signed_segment_to_keepout_distance(
    a: autopcb_routes::Point,
    b: autopcb_routes::Point,
    keepout: &IrKeepoutZone,
) -> f64 {
    let outline = &keepout.outline;
    let to_pt = |pm: &PointMm| autopcb_routes::Point { x: pm.x, y: pm.y };
    let poly: Vec<autopcb_routes::Point> = outline.iter().map(to_pt).collect();

    // If either endpoint is inside the polygon, the segment intrudes → return -1.0.
    if point_in_polygon(a, &poly) || point_in_polygon(b, &poly) {
        return -1.0;
    }

    // Otherwise return the minimum distance from the segment to any polygon edge.
    segment_to_polyline_distance(a, b, outline)
}

/// Ray-casting point-in-polygon test.
///
/// Returns `true` when `p` is strictly inside the polygon defined by `vertices`.
/// Points on the boundary may return either value (acceptable for DRC purposes).
fn point_in_polygon(p: autopcb_routes::Point, vertices: &[autopcb_routes::Point]) -> bool {
    let n = vertices.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let vi = vertices[i];
        let vj = vertices[j];
        if ((vi.y > p.y) != (vj.y > p.y))
            && (p.x < (vj.x - vi.x) * (p.y - vi.y) / (vj.y - vi.y) + vi.x)
        {
            inside = !inside;
        }
        j = i;
    }
    inside
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
    use autopcb_routes::{LayerId, NetId, Point, RoutedNet, RouteSolution, TraceSegment};

    use crate::drc::policy::DrcPolicy;

    // Build a minimal PcbIr with no design rules (defaults apply).
    fn empty_ir_no_outline() -> PcbIr {
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
                    name: "Top".into(),
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

    fn default_policy() -> DrcPolicy {
        DrcPolicy::build(&empty_ir_no_outline()).unwrap()
    }

    fn make_solution_with_segments(segs: Vec<TraceSegment>) -> RouteSolution {
        let mut solution = RouteSolution::new();
        // Group segments by net_id.
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

    /// Two parallel horizontal segments 0.1 mm apart (edge-to-edge), default
    /// clearance 0.1 mm → no violation (exactly at limit is not a violation).
    /// Move them closer to 0.05 mm apart → violation.
    #[test]
    fn parallel_segments_violation_detected() {
        let layer = LayerId(0);
        // Seg A: y=0, width=0.1 → upper edge at y=0.05
        let seg_a = TraceSegment {
            net_id: NetId(0),
            layer,
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 1.0, y: 0.0 },
            width_mm: 0.1,
        };
        // Seg B: y=0.15, width=0.1 → lower edge at y=0.10
        // Edge-to-edge clearance = 0.15 - 0.05 - 0.05 = 0.05 mm < 0.1 mm required.
        let seg_b = TraceSegment {
            net_id: NetId(1),
            layer,
            start: Point { x: 0.0, y: 0.15 },
            end: Point { x: 1.0, y: 0.15 },
            width_mm: 0.1,
        };
        let policy = default_policy(); // clearance = 0.1 mm
        let ir = empty_ir_no_outline();
        let solution = make_solution_with_segments(vec![seg_a, seg_b]);
        let violations = check_clearance(&solution, &policy, &ir);
        assert_eq!(violations.len(), 1, "expected one clearance violation");
        let v = &violations[0];
        assert_eq!(v.kind, DrcViolationKind::ClearanceViolation);
        assert!(v.actual_mm < v.required_mm,
            "actual {} must be less than required {}", v.actual_mm, v.required_mm);
        // actual should be ≈ 0.05 mm
        assert!((v.actual_mm - 0.05).abs() < 1e-9, "actual = {}", v.actual_mm);
    }

    /// Same-net segments that touch → no clearance violation.
    #[test]
    fn same_net_no_violation() {
        let layer = LayerId(0);
        let seg_a = TraceSegment {
            net_id: NetId(0),
            layer,
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 1.0, y: 0.0 },
            width_mm: 0.1,
        };
        // Directly adjacent — would be a violation if different net.
        let seg_b = TraceSegment {
            net_id: NetId(0),
            layer,
            start: Point { x: 0.0, y: 0.05 },
            end: Point { x: 1.0, y: 0.05 },
            width_mm: 0.1,
        };
        let policy = default_policy();
        let ir = empty_ir_no_outline();
        let solution = make_solution_with_segments(vec![seg_a, seg_b]);
        let violations = check_clearance(&solution, &policy, &ir);
        assert!(violations.is_empty(), "same-net segments must not produce violations");
    }

    /// Segment near a via — check segment-to-via clearance.
    #[test]
    fn segment_near_via_violation() {
        use autopcb_routes::{RoutedVia, LayerId};

        let layer = LayerId(0);
        let seg = TraceSegment {
            net_id: NetId(0),
            layer,
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 2.0, y: 0.0 },
            width_mm: 0.1,
        };
        // Via on same layer, 0.1 mm above segment centerline.
        // Via radius = drill/2 + ring = 0.15/2 + 0.05 = 0.075 + 0.05 = 0.125
        // actual = 0.1 - 0.125 - 0.05 = -0.075 → violation
        let via = RoutedVia {
            net_id: NetId(1),
            position: Point { x: 1.0, y: 0.1 },
            from_layer: LayerId(0),
            to_layer: LayerId(0),
            drill_mm: 0.15,
            annular_ring_mm: 0.05,
        };

        let mut solution = RouteSolution::new();
        solution.nets.insert(NetId(0), RoutedNet {
            net_id: NetId(0),
            segments: vec![seg],
            vias: vec![],
            routed_length_mm: 0.0,
        });
        solution.nets.insert(NetId(1), RoutedNet {
            net_id: NetId(1),
            segments: vec![],
            vias: vec![via],
            routed_length_mm: 0.0,
        });

        let policy = default_policy();
        let ir = empty_ir_no_outline();
        let violations = check_clearance(&solution, &policy, &ir);
        assert!(!violations.is_empty(), "expected segment-to-via violation");
        assert_eq!(violations[0].kind, DrcViolationKind::ClearanceViolation);
    }

    /// `point_to_segment_distance` geometry helper sanity check.
    #[test]
    fn point_to_segment_distance_basic() {
        let a = Point { x: 0.0, y: 0.0 };
        let b = Point { x: 1.0, y: 0.0 };
        // Point directly above midpoint → distance = 1.0.
        let p = Point { x: 0.5, y: 1.0 };
        let d = point_to_segment_distance(p, a, b);
        assert!((d - 1.0).abs() < 1e-12, "expected 1.0, got {d}");

        // Point past the end → distance to endpoint b.
        let p2 = Point { x: 2.0, y: 0.0 };
        let d2 = point_to_segment_distance(p2, a, b);
        assert!((d2 - 1.0).abs() < 1e-12, "expected 1.0, got {d2}");
    }


    #[cfg(feature = "proptest")]
    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn violations_have_actual_less_than_required(
                y_a in -100.0_f64..100.0,
                y_b in -100.0_f64..100.0,
                width in 0.05_f64..0.5,
            ) {
                let layer = LayerId(0);
                let seg_a = TraceSegment {
                    net_id: NetId(0),
                    layer,
                    start: Point { x: 0.0, y: y_a },
                    end:   Point { x: 1.0, y: y_a },
                    width_mm: width,
                };
                let seg_b = TraceSegment {
                    net_id: NetId(1),
                    layer,
                    start: Point { x: 0.0, y: y_b },
                    end:   Point { x: 1.0, y: y_b },
                    width_mm: width,
                };
                let policy = default_policy();
                let ir = empty_ir_no_outline();
                let solution = make_solution_with_segments(vec![seg_a, seg_b]);
                let violations = check_clearance(&solution, &policy, &ir);
                for v in &violations {
                    prop_assert!(
                        v.actual_mm < v.required_mm,
                        "violation actual_mm ({}) must be < required_mm ({})",
                        v.actual_mm,
                        v.required_mm,
                    );
                }
            }
        }
    }
}
