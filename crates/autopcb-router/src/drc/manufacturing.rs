//! Manufacturing constraint checks: solder mask, silk clearance.
//!
//! These checks require mask and silk layer data that is not yet present in
//! the PCB IR. This module is a placeholder — it returns an empty violation
//! list until IR extensions expose manufacturing layer geometry.

use autopcb_ir::PcbIr;
use autopcb_routes::RouteSolution;

use super::{policy::DrcPolicy, DrcViolation};

/// Check manufacturing constraints (solder mask, silk clearance).
///
/// Currently returns an empty list: manufacturing checks require mask/silk
/// layer data that is not yet modelled in `PcbIr`. The function signature
/// matches the other per-rule checkers so it can be wired into
/// `DrcEngine::check_full()` without further changes when IR support arrives.
pub fn check_manufacturing(
    _solution: &RouteSolution,
    _ir: &PcbIr,
    _policy: &DrcPolicy,
) -> Vec<DrcViolation> {
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
        types::{BoundingBoxMm, PointMm},
        IrBoardGeometry, PcbIr,
    };

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
        }
    }

    #[test]
    fn returns_empty_with_no_manufacturing_data() {
        let ir = empty_ir();
        let policy = DrcPolicy::build(&ir).unwrap();
        let solution = RouteSolution::new();
        let violations = check_manufacturing(&solution, &ir, &policy);
        assert!(
            violations.is_empty(),
            "expected no violations (manufacturing data not in IR), got {}",
            violations.len()
        );
    }

    #[test]
    fn returns_empty_with_populated_solution() {
        use autopcb_routes::{LayerId, NetId, Point, RoutedNet, TraceSegment};

        let ir = empty_ir();
        let policy = DrcPolicy::build(&ir).unwrap();

        let net_id = NetId(1);
        let seg = TraceSegment {
            net_id,
            layer: LayerId(0),
            start: Point { x: 10.0, y: 10.0 },
            end: Point { x: 90.0, y: 90.0 },
            width_mm: 0.2,
        };
        let mut solution = RouteSolution::new();
        solution.nets.insert(net_id, RoutedNet {
            net_id,
            segments: vec![seg],
            vias: vec![],
            routed_length_mm: 1.0,
        });

        let violations = check_manufacturing(&solution, &ir, &policy);
        assert!(violations.is_empty());
    }
}
