//! Text objects in the IR.

use crate::handles::{ComponentId, TextId};
use crate::types::PointMm;

/// A PCB text object.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct IrText {
    pub id: TextId,
    /// The text content to display.
    pub text: String,
    /// Position in mm.
    pub location: PointMm,
    /// Text height in mm.
    pub height_mm: f64,
    /// Stroke width in mm.
    pub width_mm: f64,
    /// Rotation in degrees (0–360).
    pub rotation_deg: f64,
    /// Whether the text is mirrored.
    pub is_mirrored: bool,
    /// Whether this text is a designator reference.
    pub is_designator: bool,
    /// Whether this text is a comment/value reference.
    pub is_comment: bool,
    /// Layer display name.
    pub layer_name: String,
    /// Owning component, if any.
    pub component: Option<ComponentId>,
}
