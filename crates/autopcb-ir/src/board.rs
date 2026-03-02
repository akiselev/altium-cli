//! Board geometry: outline, cutouts, and keepout zones.

use crate::types::{BoundingBoxMm, PointMm};

/// The physical board shape, in millimeters.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct IrBoardGeometry {
    /// Tessellated board outline (closed polygon, arcs sampled as line segments).
    pub outline: Vec<PointMm>,
    /// Interior cutouts (e.g. mounting holes routed out).
    pub cutouts: Vec<Vec<PointMm>>,
    /// Axis-aligned bounding box of the outline.
    pub bounds: BoundingBoxMm,
    /// Keepout zones restricting copper or components.
    pub keepouts: Vec<IrKeepoutZone>,
}

/// A keepout zone on the board.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct IrKeepoutZone {
    /// Tessellated outline of the keepout region.
    pub outline: Vec<PointMm>,
    /// Layer restriction (None = all layers).
    pub layer_name: Option<String>,
}
