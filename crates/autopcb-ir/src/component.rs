//! Component and pad representations in the IR.

use crate::handles::{ComponentId, NetId, PadId};
use crate::types::{BoardSide, BoundingBoxMm, PointMm};

/// A placed component on the board.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct IrComponent {
    pub id: ComponentId,
    pub designator: String,
    pub pattern: String,
    pub value: String,
    /// World position in mm.
    pub position: PointMm,
    /// Rotation in degrees (0–360).
    pub rotation: f64,
    pub side: BoardSide,
    /// Bounding box in component-local coordinates.
    pub local_bounds: BoundingBoxMm,
    /// Bounding box in world coordinates.
    pub world_bounds: BoundingBoxMm,
    /// Pads belonging to this component.
    pub pads: Vec<IrComponentPad>,
}

/// A single pad on a component.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct IrComponentPad {
    /// Global pad ID (unique across the entire board).
    pub id: PadId,
    /// Pad name/number (e.g. "1", "A1", "GND").
    pub name: String,
    /// Position relative to the component origin.
    pub local_position: PointMm,
    /// Position in world (board) coordinates.
    pub world_position: PointMm,
    /// Net this pad belongs to, if any.
    pub net: Option<NetId>,
    /// Pad shape information.
    pub shape: PadShapeInfo,
    /// Whether this is a through-hole pad.
    pub is_through_hole: bool,
    /// Drill hole diameter in mm (0.0 for SMD pads).
    pub hole_size_mm: f64,
    /// Pin swap group identifier within the component (from back-annotated SchLib data).
    /// Pads with the same `swap_id_pin` within a component are electrically interchangeable.
    /// `None` if the pad has no pin swap group or if PcbDoc does not carry this data.
    pub swap_id_pin: Option<String>,
    /// Part swap group identifier across components (from back-annotated SchLib data).
    /// Components with the same `swap_id_part` have identical pinouts and can be swapped.
    /// `None` if the component has no part swap group or if PcbDoc does not carry this data.
    pub swap_id_part: Option<String>,
}

/// Describes the shape of a pad for rendering and clearance purposes.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PadShapeInfo {
    pub kind: PadShapeKind,
    /// X extent in mm.
    pub size_x: f64,
    /// Y extent in mm.
    pub size_y: f64,
    /// Pad rotation in degrees.
    pub rotation: f64,
}

/// Simplified pad shape classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum PadShapeKind {
    Round,
    Rectangular,
    RoundRect,
    Octagonal,
    Other,
}
