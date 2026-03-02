//! Public API types for PcbLib documents.
//!
//! These types provide a clean, domain-typed interface for querying and mutating
//! PCB library footprints. The read/write paths in `pcblib_read.rs` and
//! `pcblib_write.rs` handle conversion to/from internal `PcbFootprint` types.

use altium_format_types::coord::{Coord, CoordPoint};
use altium_format_types::pcb::{
    LayerRef, PadShape, PadStackMode, PcbFlags, PlaneConnectionStyle, RegionKind,
};
use altium_format_types::color::Color;
use super::pcb_common::{PadStack, PcbContour};

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
/// The `shape`/`x_size`/`y_size` fields are top-layer convenience accessors.
/// For multi-layer pad details, use the `stack` field.
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
    pub layer: LayerRef,
    pub pad_mode: PadStackMode,
    pub solder_mask_expansion: Coord,
    pub paste_mask_expansion: Coord,
    pub plane_connection: PlaneConnectionStyle,
    pub relief_conductor_width: Coord,
    pub relief_entries: i32,
    pub relief_air_gap: Coord,
    /// Per-layer pad shapes. Only populated for non-Simple pad modes.
    pub stack: PadStack,
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

    /// Returns the layer reference for this graphic.
    pub fn layer(&self) -> &LayerRef {
        match self {
            PcbGraphic::Track(g) => &g.layer,
            PcbGraphic::Arc(g) => &g.layer,
            PcbGraphic::Fill(g) => &g.layer,
            PcbGraphic::Region(g) => &g.layer,
            PcbGraphic::Text(g) => &g.layer,
            PcbGraphic::Via(g) => &g.layer,
            PcbGraphic::ComponentBody(g) => &g.layer,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TrackGraphic {
    pub unique_id: Option<String>,
    pub layer: LayerRef,
    pub flags: PcbFlags,
    pub start: CoordPoint,
    pub end: CoordPoint,
    pub width: Coord,
}

#[derive(Debug, Clone)]
pub struct PcbArcGraphic {
    pub unique_id: Option<String>,
    pub layer: LayerRef,
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
    pub layer: LayerRef,
    pub flags: PcbFlags,
    pub corner1: CoordPoint,
    pub corner2: CoordPoint,
    pub rotation: f64,
}

#[derive(Debug, Clone)]
pub struct RegionGraphic {
    pub unique_id: Option<String>,
    pub layer: LayerRef,
    pub flags: PcbFlags,
    pub kind: RegionKind,
    pub outline: PcbContour,
    pub holes: Vec<PcbContour>,
}

#[derive(Debug, Clone)]
pub struct TextGraphic {
    pub unique_id: Option<String>,
    pub layer: LayerRef,
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
    pub layer: LayerRef,
    pub flags: PcbFlags,
    pub location: CoordPoint,
    pub diameter: Coord,
    pub hole_size: Coord,
    pub from_layer: LayerRef,
    pub to_layer: LayerRef,
    pub is_testpoint_top: bool,
    pub is_testpoint_bottom: bool,
    pub is_assy_testpoint_top: bool,
    pub is_assy_testpoint_bottom: bool,
    pub solder_mask_override: bool,
    pub use_separate_solder_mask_expansion: bool,
    pub solder_mask_expansion_from_hole_edge: bool,
    pub paste_mask_override: bool,
}

#[derive(Debug, Clone)]
pub struct ComponentBodyGraphic {
    pub unique_id: Option<String>,
    pub layer: LayerRef,
    pub flags: PcbFlags,
    pub standoff_height: Coord,
    pub overall_height: Coord,
    pub body_color_3d: Color,
    pub body_opacity_3d: f64,
    pub model_name: String,
    pub outline: PcbContour,
}

// ── Footprint query helpers ──────────────────────────────────────────────────

impl Footprint {
    /// Find a pad by name.
    pub fn pad(&self, name: &str) -> Option<&Pad> {
        self.pads.iter().find(|p| p.pad_name == name)
    }

    /// All pads on a given layer.
    pub fn pads_on_layer(&self, layer: &LayerRef) -> Vec<&Pad> {
        self.pads.iter().filter(|p| p.layer == *layer).collect()
    }

    /// All plated through-hole pads (has hole and is plated).
    pub fn plated_through_hole_pads(&self) -> Vec<&Pad> {
        self.pads
            .iter()
            .filter(|p| p.is_plated && p.hole_size != Coord::ZERO)
            .collect()
    }

    /// All non-plated through-hole pads (has hole but not plated).
    pub fn non_plated_through_hole_pads(&self) -> Vec<&Pad> {
        self.pads
            .iter()
            .filter(|p| !p.is_plated && p.hole_size != Coord::ZERO)
            .collect()
    }

    /// All surface-mount pads (no hole).
    pub fn smd_pads(&self) -> Vec<&Pad> {
        self.pads
            .iter()
            .filter(|p| p.hole_size == Coord::ZERO)
            .collect()
    }

    /// All graphics on a given layer.
    pub fn graphics_on_layer(&self, layer: &LayerRef) -> Vec<&PcbGraphic> {
        self.graphics
            .iter()
            .filter(|g| *g.layer() == *layer)
            .collect()
    }

    /// All region graphics.
    pub fn regions(&self) -> Vec<&RegionGraphic> {
        self.graphics
            .iter()
            .filter_map(|g| match g {
                PcbGraphic::Region(r) => Some(r),
                _ => None,
            })
            .collect()
    }

    /// All component body graphics.
    pub fn component_bodies(&self) -> Vec<&ComponentBodyGraphic> {
        self.graphics
            .iter()
            .filter_map(|g| match g {
                PcbGraphic::ComponentBody(b) => Some(b),
                _ => None,
            })
            .collect()
    }
}
