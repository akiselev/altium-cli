//! Layer stack representation.

use crate::handles::LayerId;

/// The board's copper layer stack.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct IrLayerStack {
    /// Copper layers in physical order (top to bottom).
    pub copper_layers: Vec<IrCopperLayer>,
    /// Total number of copper layers.
    pub copper_layer_count: usize,
}

/// A single copper layer.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct IrCopperLayer {
    pub id: LayerId,
    pub name: String,
    pub is_top: bool,
    pub is_bottom: bool,
}
