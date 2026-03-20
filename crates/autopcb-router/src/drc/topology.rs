//! Topology DRC: daisy-chain stub length and routing topology checks.
//!
//! WHY this is a stub: full topology analysis requires knowing the component
//! placement graph and the net's spanning tree, which depends on data that
//! is not yet fully accessible from `RouteSolution` + `PcbIr` alone (we
//! need the connection order and the T-stub endpoints).  The check will be
//! filled in once that data is available from the IR placement layer.

use autopcb_routes::RouteSolution;

use super::DrcViolation;
use crate::drc::policy::DrcPolicy;

/// Check routing topology constraints (daisy-chain stub length).
///
/// Returns an empty violation list — full implementation requires component
/// placement data that is not yet exposed in the current IR + solution model.
pub fn check_topology(_solution: &RouteSolution, _policy: &DrcPolicy) -> Vec<DrcViolation> {
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
    use autopcb_routes::RouteSolution;

    fn empty_policy() -> DrcPolicy {
        let ir = PcbIr {
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
        };
        DrcPolicy::build(&ir).unwrap()
    }

    #[test]
    fn empty_solution_returns_no_violations() {
        let solution = RouteSolution::new();
        let policy = empty_policy();
        let violations = check_topology(&solution, &policy);
        assert!(violations.is_empty());
    }
}
