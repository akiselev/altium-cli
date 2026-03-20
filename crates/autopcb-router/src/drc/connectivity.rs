//! Connectivity DRC: broken nets and net antennae.
//!
//! WHY this check exists: a routed solution may leave some nets entirely
//! unrouted (no segments at all) even though the IR declares pins for them.
//! PathFinder records such nets in `solution.unrouted`, but any net with
//! ≥ 2 pins that has neither routed segments nor an `unrouted` entry is a
//! silent failure that would produce an open circuit on the real board.
//!
//! Antenna detection: a segment endpoint that connects to nothing else (no
//! adjacent segment and no pad) is a dead-end trace stub that wastes copper
//! and may act as an RF antenna.  Each such endpoint is reported as
//! `DrcViolationKind::NetAntenna`.

use altium_format_types::pcb::RuleKind;
use autopcb_ir::PcbIr;
use autopcb_routes::{NetId, RouteSolution};

use super::{DrcObject, DrcViolation, DrcViolationKind};
use crate::drc::policy::DrcPolicy;

/// Tolerance for point equality when detecting shared segment endpoints (mm).
const POINT_TOLERANCE: f64 = 1e-6;

/// Returns true if two (x,y) pairs are within `POINT_TOLERANCE` of each other.
#[inline]
fn points_eq(ax: f64, ay: f64, bx: f64, by: f64) -> bool {
    (ax - bx).abs() < POINT_TOLERANCE && (ay - by).abs() < POINT_TOLERANCE
}

/// Check connectivity: detect broken nets and net antennae.
///
/// **Broken nets**: any net in the IR with ≥ 2 pins that is absent from both
/// `solution.nets` and `solution.unrouted` is flagged as a `BrokenNet`.
/// Nets in `solution.unrouted` are intentionally unrouted and not flagged here.
///
/// **Net antennae**: for each routed net, collect all segment endpoints.  An
/// endpoint that appears exactly once (not shared with another segment endpoint
/// and not coincident with any pad position) is a dead-end stub and is flagged
/// as `NetAntenna`.
pub fn check_connectivity(
    solution: &RouteSolution,
    ir: &PcbIr,
    _policy: &DrcPolicy,
) -> Vec<DrcViolation> {
    let mut violations = Vec::new();

    // Build a flat map from IR net id → pad positions for antenna detection.
    let mut pad_positions: std::collections::BTreeMap<NetId, Vec<(f64, f64)>> =
        std::collections::BTreeMap::new();
    for (_ir_net_id, ir_net) in ir.nets.iter() {
        let routes_net_id = NetId(ir_net.id.raw());
        let positions: Vec<(f64, f64)> = ir_net
            .pins
            .iter()
            .map(|p| (p.position.x, p.position.y))
            .collect();
        if !positions.is_empty() {
            pad_positions.insert(routes_net_id, positions);
        }
    }

    for (_ir_net_id, ir_net) in ir.nets.iter() {
        // Only check nets that need routing (≥ 2 pins).
        if ir_net.pins.len() < 2 {
            continue;
        }

        // Map IR net id to routes NetId by raw index.
        let routes_net_id = NetId(ir_net.id.raw());

        let has_segments = solution
            .nets
            .get(&routes_net_id)
            .map(|rn| !rn.segments.is_empty())
            .unwrap_or(false);

        let is_explicitly_unrouted = solution.unrouted.contains(&routes_net_id);

        if !has_segments && !is_explicitly_unrouted {
            // Use the first pin position as the violation location, falling
            // back to the origin if the net somehow has no pins (guarded
            // above, but defensive).
            let location = ir_net
                .pins
                .first()
                .map(|p| p.position)
                .unwrap_or(autopcb_ir::types::PointMm { x: 0.0, y: 0.0 });

            violations.push(DrcViolation {
                kind: DrcViolationKind::BrokenNet,
                rule_kind: RuleKind::BrokenNets,
                rule_name: "BrokenNets".to_string(),
                object_a: DrcObject::Pad {
                    component: ir_net
                        .pins
                        .first()
                        .map(|p| p.component.to_string())
                        .unwrap_or_default(),
                    pad: ir_net
                        .pins
                        .first()
                        .map(|p| p.pad.to_string())
                        .unwrap_or_default(),
                    position: location,
                },
                object_b: None,
                location,
                layer: None,
                actual_mm: 0.0,
                required_mm: 0.0,
            });
        }
    }

    // Antenna detection: for each routed net, count endpoint occurrences.
    for (&routes_net_id, routed_net) in &solution.nets {
        if routed_net.segments.is_empty() {
            continue;
        }

        // Collect all segment endpoints as (x, y) pairs.
        let mut endpoints: Vec<(f64, f64)> = Vec::with_capacity(routed_net.segments.len() * 2);
        for seg in &routed_net.segments {
            endpoints.push((seg.start.x, seg.start.y));
            endpoints.push((seg.end.x, seg.end.y));
        }

        let empty_pads: Vec<(f64, f64)> = Vec::new();
        let pads = pad_positions.get(&routes_net_id).unwrap_or(&empty_pads);

        // Count how many times each endpoint is shared (with another endpoint or pad).
        for (i, &(ex, ey)) in endpoints.iter().enumerate() {
            // Count occurrences among all other endpoints.
            let shared_with_endpoint = endpoints
                .iter()
                .enumerate()
                .filter(|&(j, &(ox, oy))| j != i && points_eq(ex, ey, ox, oy))
                .count();

            if shared_with_endpoint > 0 {
                // Junction or continuation — not an antenna.
                continue;
            }

            // Check against pad positions.
            let at_pad = pads.iter().any(|&(px, py)| points_eq(ex, ey, px, py));
            if at_pad {
                continue;
            }

            // This endpoint is not shared with any segment endpoint or pad → antenna.
            // Find the originating segment so we can report it as object_a.
            let seg_idx = i / 2;
            let seg = &routed_net.segments[seg_idx];
            violations.push(DrcViolation {
                kind: DrcViolationKind::NetAntenna,
                rule_kind: RuleKind::NetAntennae,
                rule_name: "NetAntennae".to_string(),
                object_a: DrcObject::Segment(seg.clone()),
                object_b: None,
                location: autopcb_ir::types::PointMm { x: ex, y: ey },
                layer: Some(seg.layer),
                actual_mm: 0.0,
                required_mm: 0.0,
            });
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
        net::{IrNet, IrNetPin},
        rule::{IrDesignRule, IrRuleParams},
        types::{BoundingBoxMm, PointMm},
        IrBoardGeometry, PcbIr,
    };
    use autopcb_routes::{LayerId, NetId, RoutedNet, RouteSolution, TraceSegment};
    use autopcb_routes::Point;
    use altium_format_types::pcb::RuleKind;
    use autopcb_ir::handles::{ComponentId, PadId};

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
            texts: IdMap::new(),
            regions: IdMap::new(),
            component_bodies: IdMap::new(),
        }
    }

    fn default_policy() -> DrcPolicy {
        DrcPolicy::build(&empty_ir()).unwrap()
    }

    fn make_net_pin(x: f64, y: f64) -> IrNetPin {
        IrNetPin {
            pad: PadId::from(0u32),
            component: ComponentId::from(0u32),
            position: PointMm { x, y },
        }
    }

    fn add_net_with_pins(ir: &mut PcbIr, pin_count: usize) -> IrNetId {
        let mut pins = Vec::new();
        for i in 0..pin_count {
            pins.push(make_net_pin(i as f64, 0.0));
        }
        let raw_id = ir.nets.len() as u32;
        let id = ir.nets.push(IrNet {
            id: IrNetId::from(0u32),
            name: format!("net_{}", raw_id),
            pins,
            component_count: 0,
            net_class: None,
            diff_pair_partner: None,
        });
        ir.nets[id].id = id;
        id
    }

    fn routed_solution_for(ir_net_id: IrNetId) -> RouteSolution {
        let routes_net_id = NetId(ir_net_id.raw());
        let mut solution = RouteSolution::new();
        let seg = TraceSegment {
            net_id: routes_net_id,
            layer: LayerId(0),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 1.0, y: 0.0 },
            width_mm: 0.2,
        };
        solution.nets.insert(
            routes_net_id,
            RoutedNet {
                net_id: routes_net_id,
                segments: vec![seg],
                vias: vec![],
                routed_length_mm: 1.0,
            },
        );
        solution
    }

    #[test]
    fn fully_routed_net_no_violation() {
        let mut ir = empty_ir();
        let net_id = add_net_with_pins(&mut ir, 2);
        let solution = routed_solution_for(net_id);
        let policy = default_policy();

        let violations = check_connectivity(&solution, &ir, &policy);
        assert!(violations.is_empty(), "expected no violations, got {:?}", violations);
    }

    #[test]
    fn net_missing_from_solution_broken_net_violation() {
        let mut ir = empty_ir();
        let _net_id = add_net_with_pins(&mut ir, 2);
        let solution = RouteSolution::new(); // empty — net not routed, not in unrouted
        let policy = default_policy();

        let violations = check_connectivity(&solution, &ir, &policy);
        assert_eq!(violations.len(), 1, "expected 1 BrokenNet violation");
        assert_eq!(violations[0].kind, DrcViolationKind::BrokenNet);
        assert_eq!(violations[0].rule_kind, RuleKind::BrokenNets);
    }

    #[test]
    fn net_in_unrouted_no_violation() {
        let mut ir = empty_ir();
        let net_id = add_net_with_pins(&mut ir, 2);
        let routes_net_id = NetId(net_id.raw());
        let mut solution = RouteSolution::new();
        solution.unrouted.push(routes_net_id);
        let policy = default_policy();

        let violations = check_connectivity(&solution, &ir, &policy);
        assert!(violations.is_empty(), "unrouted net should not be flagged as broken");
    }

    #[test]
    fn single_pin_net_not_checked() {
        let mut ir = empty_ir();
        let _net_id = add_net_with_pins(&mut ir, 1);
        let solution = RouteSolution::new();
        let policy = default_policy();

        // Single-pin nets can't form a connection; skip them.
        let violations = check_connectivity(&solution, &ir, &policy);
        assert!(violations.is_empty(), "single-pin net should not produce a violation");
    }

    /// Build a solution with a connected segment (start at pad A, end at pad B) plus
    /// a dangling segment that starts at pad B but ends at a free point.
    fn dangling_solution(ir_net_id: IrNetId, pad_a: (f64, f64), pad_b: (f64, f64)) -> RouteSolution {
        let routes_net_id = NetId(ir_net_id.raw());
        let mut solution = RouteSolution::new();
        // Segment A→B: both endpoints are pad positions → no antenna.
        let seg_ab = TraceSegment {
            net_id: routes_net_id,
            layer: LayerId(0),
            start: Point { x: pad_a.0, y: pad_a.1 },
            end: Point { x: pad_b.0, y: pad_b.1 },
            width_mm: 0.2,
        };
        // Segment B→C: starts at pad B (shared) but end C is free → antenna at C.
        let seg_bc = TraceSegment {
            net_id: routes_net_id,
            layer: LayerId(0),
            start: Point { x: pad_b.0, y: pad_b.1 },
            end: Point { x: 50.0, y: 50.0 },
            width_mm: 0.2,
        };
        solution.nets.insert(
            routes_net_id,
            RoutedNet {
                net_id: routes_net_id,
                segments: vec![seg_ab, seg_bc],
                vias: vec![],
                routed_length_mm: 10.0,
            },
        );
        solution
    }

    #[test]
    fn dangling_segment_emits_antenna_violation() {
        let mut ir = empty_ir();
        let net_id = add_net_with_pins(&mut ir, 2);

        // Override pin positions to match the segment endpoints.
        let pad_a = (0.0, 0.0);
        let pad_b = (10.0, 0.0);
        ir.nets[net_id].pins[0] = make_net_pin(pad_a.0, pad_a.1);
        ir.nets[net_id].pins[1] = make_net_pin(pad_b.0, pad_b.1);

        let solution = dangling_solution(net_id, pad_a, pad_b);
        let policy = default_policy();

        let violations = check_connectivity(&solution, &ir, &policy);
        let antenna_viols: Vec<_> = violations
            .iter()
            .filter(|v| v.kind == DrcViolationKind::NetAntenna)
            .collect();
        assert_eq!(antenna_viols.len(), 1,
            "expected 1 NetAntenna violation, got {:?}", violations);
        assert_eq!(antenna_viols[0].rule_kind, RuleKind::NetAntennae);
        // Antenna location should be at the dangling end (50, 50).
        assert!((antenna_viols[0].location.x - 50.0).abs() < 1e-6);
        assert!((antenna_viols[0].location.y - 50.0).abs() < 1e-6);
    }

    #[test]
    fn fully_connected_segments_no_antenna() {
        let mut ir = empty_ir();
        let net_id = add_net_with_pins(&mut ir, 2);

        let pad_a = (0.0, 0.0);
        let pad_b = (10.0, 0.0);
        ir.nets[net_id].pins[0] = make_net_pin(pad_a.0, pad_a.1);
        ir.nets[net_id].pins[1] = make_net_pin(pad_b.0, pad_b.1);

        // One segment from pad A to pad B: both ends are pads → no antenna.
        let solution = routed_solution_for(net_id);
        // Rebuild with exact pad positions.
        let routes_net_id = NetId(net_id.raw());
        let mut sol2 = RouteSolution::new();
        let seg = TraceSegment {
            net_id: routes_net_id,
            layer: LayerId(0),
            start: Point { x: pad_a.0, y: pad_a.1 },
            end: Point { x: pad_b.0, y: pad_b.1 },
            width_mm: 0.2,
        };
        sol2.nets.insert(
            routes_net_id,
            RoutedNet {
                net_id: routes_net_id,
                segments: vec![seg],
                vias: vec![],
                routed_length_mm: 10.0,
            },
        );
        let _ = solution; // suppress unused warning
        let policy = default_policy();

        let violations = check_connectivity(&sol2, &ir, &policy);
        let antenna_viols: Vec<_> = violations
            .iter()
            .filter(|v| v.kind == DrcViolationKind::NetAntenna)
            .collect();
        assert!(antenna_viols.is_empty(),
            "segment from pad to pad should not produce an antenna: {:?}", violations);
    }
}
