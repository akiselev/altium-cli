//! Free (non-component) copper geometry: tracks, vias, fills.

use crate::handles::NetId;
use crate::types::PointMm;

/// Collection of free-standing copper primitives (not owned by a component).
#[derive(Debug, Clone, Default)]
pub struct FreeCopperGeometry {
    pub tracks: Vec<IrTrack>,
    pub vias: Vec<IrVia>,
    pub fills: Vec<IrFill>,
}

/// A PCB track segment.
#[derive(Debug, Clone)]
pub struct IrTrack {
    pub start: PointMm,
    pub end: PointMm,
    pub width_mm: f64,
    pub layer_name: String,
    pub net: Option<NetId>,
}

/// A via (vertical interconnect).
#[derive(Debug, Clone)]
pub struct IrVia {
    pub position: PointMm,
    pub diameter_mm: f64,
    pub hole_size_mm: f64,
    pub net: Option<NetId>,
}

/// A solid copper fill (rectangle).
#[derive(Debug, Clone)]
pub struct IrFill {
    pub corner1: PointMm,
    pub corner2: PointMm,
    pub layer_name: String,
    pub net: Option<NetId>,
}
