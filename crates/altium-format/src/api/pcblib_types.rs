//! Public API types for PcbLib documents.
//!
//! These types are defined for future use. The read/write paths for PcbLib
//! are deferred to a follow-up plan — SchLib establishes the pattern first.

use altium_format_types::coord::{Coord, CoordPoint};
use altium_format_types::pcb::{
    PadShape, PadStackMode, PcbFlags, PlaneConnectionStyle, V6Layer,
};
use altium_format_types::color::Color;

// ── Footprint ────────────────────────────────────────────────────────────────

/// A PCB library footprint.
///
/// Identified by `display_name` (natural key, unique within a PcbLib).
#[derive(Debug, Clone)]
pub struct Footprint {
    pub display_name: String,
    pub description: String,
    pub pattern: String,
    pub height: Coord,

    pub pads: Vec<Pad>,
    pub graphics: Vec<PcbGraphic>,
}

// ── Pad ──────────────────────────────────────────────────────────────────────

/// A PCB pad.
///
/// Identified by `pad_name` (natural key, unique within a Footprint).
#[derive(Debug, Clone)]
pub struct Pad {
    pub pad_name: String,
    pub location: CoordPoint,
    pub shape: PadShape,
    pub x_size: Coord,
    pub y_size: Coord,
    pub rotation: f64,
    pub hole_size: Coord,
    pub is_plated: bool,
    pub layer: V6Layer,
    pub pad_mode: PadStackMode,
    pub solder_mask_expansion: Coord,
    pub paste_mask_expansion: Coord,
    pub plane_connection: PlaneConnectionStyle,
    pub relief_conductor_width: Coord,
    pub relief_entries: i32,
    pub relief_air_gap: Coord,
}

// ── PcbGraphic ───────────────────────────────────────────────────────────────

/// A PCB graphical primitive within a footprint.
///
/// Each variant corresponds to a PCB primitive type.
#[derive(Debug, Clone)]
pub enum PcbGraphic {
    Track(TrackGraphic),
    Arc(PcbArcGraphic),
    Fill(FillGraphic),
    Region(RegionGraphic),
    Text(TextGraphic),
    Via(ViaGraphic),
    ComponentBody(ComponentBodyGraphic),
}

#[derive(Debug, Clone)]
pub struct TrackGraphic {
    pub layer: V6Layer,
    pub flags: PcbFlags,
    pub start: CoordPoint,
    pub end: CoordPoint,
    pub width: Coord,
}

#[derive(Debug, Clone)]
pub struct PcbArcGraphic {
    pub layer: V6Layer,
    pub flags: PcbFlags,
    pub center: CoordPoint,
    pub radius: Coord,
    pub start_angle: f64,
    pub end_angle: f64,
    pub width: Coord,
}

#[derive(Debug, Clone)]
pub struct FillGraphic {
    pub layer: V6Layer,
    pub flags: PcbFlags,
    pub corner1: CoordPoint,
    pub corner2: CoordPoint,
    pub rotation: f64,
}

#[derive(Debug, Clone)]
pub struct RegionGraphic {
    pub layer: V6Layer,
    pub flags: PcbFlags,
    pub outline: Vec<CoordPoint>,
}

#[derive(Debug, Clone)]
pub struct TextGraphic {
    pub layer: V6Layer,
    pub flags: PcbFlags,
    pub location: CoordPoint,
    pub text: String,
    pub rotation: f64,
    pub height: Coord,
    pub width: Coord,
    pub color: Color,
}

#[derive(Debug, Clone)]
pub struct ViaGraphic {
    pub layer: V6Layer,
    pub flags: PcbFlags,
    pub location: CoordPoint,
    pub diameter: Coord,
    pub hole_size: Coord,
    pub from_layer: V6Layer,
    pub to_layer: V6Layer,
}

#[derive(Debug, Clone)]
pub struct ComponentBodyGraphic {
    pub layer: V6Layer,
    pub flags: PcbFlags,
}
