//! Shared test helpers for DRC module tests.

use autopcb_ir::{
    handles::{IdMap, LayerId as IrLayerId},
    layer_stack::{IrCopperLayer, IrLayerStack, PreferredDirection},
    types::{BoundingBoxMm, PointMm},
    IrBoardGeometry, PcbIr,
};

/// Construct a minimal [`PcbIr`] with a two-layer copper stack for use in
/// unit tests.
pub(super) fn empty_ir() -> PcbIr {
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
                IrCopperLayer {
                    id: IrLayerId::from(1u32),
                    name: "Bottom Layer".into(),
                    is_top: false,
                    is_bottom: true,
                    preferred_direction: Some(PreferredDirection::Any),
                },
            ],
            copper_layer_count: 2,
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
