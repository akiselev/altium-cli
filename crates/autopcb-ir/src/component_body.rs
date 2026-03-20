//! Component body (3D envelope / courtyard) in the IR.

use crate::handles::{ComponentBodyId, ComponentId};
use crate::types::PointMm;

/// A component body — the 3D envelope or courtyard outline for a component.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct IrComponentBody {
    pub id: ComponentBodyId,
    /// 2D outline in mm (tessellated).
    pub outline: Vec<PointMm>,
    /// Owning component, if any.
    pub component: Option<ComponentId>,
    /// Body color as RGBA.
    pub body_color: [u8; 4],
    /// Opacity (0.0–1.0).
    pub body_opacity: f64,
    /// Standoff height above the board surface in mm.
    pub standoff_height_mm: f64,
    /// Overall height of the component body in mm.
    pub overall_height_mm: f64,
    /// Layer display name.
    pub layer_name: String,
}
