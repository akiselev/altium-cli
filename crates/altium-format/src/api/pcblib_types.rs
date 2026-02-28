//! Public API types for PcbLib documents.
//!
//! These types provide a clean, domain-typed interface for querying and mutating
//! PCB library footprints. The read/write paths in `pcblib_read.rs` and
//! `pcblib_write.rs` handle conversion to/from internal `PcbFootprint` types.

use altium_format_types::coord::{Coord, CoordPoint};
use altium_format_types::pcb::{
    PadShape, PadStackMode, PcbFlags, PlaneConnectionStyle, RegionKind, V6Layer,
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
    pub unique_id: Option<String>,
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

impl PcbGraphic {
    /// Returns the unique ID for this graphic, if set.
    ///
    /// This follows the same pattern as `Graphic::unique_id()` in the SchLib API.
    pub fn unique_id(&self) -> Option<&str> {
        match self {
            PcbGraphic::Track(g) => g.unique_id.as_deref(),
            PcbGraphic::Arc(g) => g.unique_id.as_deref(),
            PcbGraphic::Fill(g) => g.unique_id.as_deref(),
            PcbGraphic::Region(g) => g.unique_id.as_deref(),
            PcbGraphic::Text(g) => g.unique_id.as_deref(),
            PcbGraphic::Via(g) => g.unique_id.as_deref(),
            PcbGraphic::ComponentBody(g) => g.unique_id.as_deref(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TrackGraphic {
    pub unique_id: Option<String>,
    pub layer: V6Layer,
    pub flags: PcbFlags,
    pub start: CoordPoint,
    pub end: CoordPoint,
    pub width: Coord,
}

#[derive(Debug, Clone)]
pub struct PcbArcGraphic {
    pub unique_id: Option<String>,
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
    pub unique_id: Option<String>,
    pub layer: V6Layer,
    pub flags: PcbFlags,
    pub corner1: CoordPoint,
    pub corner2: CoordPoint,
    pub rotation: f64,
}

#[derive(Debug, Clone)]
pub struct RegionGraphic {
    pub unique_id: Option<String>,
    pub layer: V6Layer,
    pub flags: PcbFlags,
    pub kind: RegionKind,
    pub outline: Vec<CoordPoint>,
    pub holes: Vec<Vec<CoordPoint>>,
}

#[derive(Debug, Clone)]
pub struct TextGraphic {
    pub unique_id: Option<String>,
    pub layer: V6Layer,
    pub flags: PcbFlags,
    pub location: CoordPoint,
    pub text: String,
    pub rotation: f64,
    pub height: Coord,
    pub width: Coord,
    pub color: Color,
    pub font_name: String,
    pub is_mirrored: bool,
}

#[derive(Debug, Clone)]
pub struct ViaGraphic {
    pub unique_id: Option<String>,
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
    pub unique_id: Option<String>,
    pub layer: V6Layer,
    pub flags: PcbFlags,
    pub standoff_height: Coord,
    pub overall_height: Coord,
    pub body_color_3d: Color,
    pub body_opacity_3d: f64,
    pub model_name: String,
    pub outline: Vec<CoordPoint>,
}
