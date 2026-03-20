//! Free (non-component) copper geometry: tracks, vias, fills.

use crate::handles::{LayerId, NetId};
use crate::types::PointMm;

/// Collection of free-standing copper primitives (not owned by a component).
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct FreeCopperGeometry {
    pub tracks: Vec<IrTrack>,
    pub arcs: Vec<IrArc>,
    pub vias: Vec<IrVia>,
    pub fills: Vec<IrFill>,
}

/// A PCB track segment.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct IrTrack {
    pub start: PointMm,
    pub end: PointMm,
    pub width_mm: f64,
    pub layer_name: String,
    /// Resolved copper layer identifier.
    pub layer: LayerId,
    pub net: Option<NetId>,
    /// Whether this track is locked (cannot be moved by the router).
    pub locked: bool,
    /// Whether this track was placed by a previous routing pass.
    pub pre_routed: bool,
}

/// A via (vertical interconnect).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct IrVia {
    pub position: PointMm,
    pub diameter_mm: f64,
    pub hole_size_mm: f64,
    pub net: Option<NetId>,
    /// Copper layer the via starts on (top of the drill span).
    pub from_layer: LayerId,
    /// Copper layer the via ends on (bottom of the drill span).
    pub to_layer: LayerId,
    /// Whether this via is locked.
    pub locked: bool,
    /// Whether this via was placed by a previous routing pass.
    pub pre_routed: bool,
}

/// A PCB arc segment.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct IrArc {
    pub center: PointMm,
    pub radius_mm: f64,
    pub start_angle_deg: f64,
    pub end_angle_deg: f64,
    pub width_mm: f64,
    pub layer_name: String,
    /// Resolved copper layer identifier. `None` for non-copper layers (silkscreen,
    /// mechanical, overlay) where there is no matching entry in the copper layer stack.
    pub layer: Option<LayerId>,
    pub net: Option<NetId>,
}

/// A solid copper fill (rectangle).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct IrFill {
    pub corner1: PointMm,
    pub corner2: PointMm,
    pub layer_name: String,
    pub net: Option<NetId>,
}
