//! Dimension annotation objects in the IR.

use crate::handles::DimensionId;
use crate::types::PointMm;

/// A PCB dimension annotation.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct IrDimension {
    pub id: DimensionId,
    /// First reference point in mm.
    pub reference1: PointMm,
    /// Second reference point in mm.
    pub reference2: PointMm,
    /// Text anchor position in mm.
    pub text_position: PointMm,
    /// Text height in mm.
    pub text_height_mm: f64,
    /// Rendered dimension text (e.g. "25.4 mm").
    pub text: String,
    /// Layer display name.
    pub layer_name: String,
}
