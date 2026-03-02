//! Public API types for PcbDoc documents.
//!
//! These types provide a clean, domain-typed interface for reading PCB board
//! designs. The read path in `pcbdoc_read.rs` handles conversion from internal
//! `PcbDoc` sections to these public types.
//!
//! Unlike PcbLib (footprint-level, no net/component context), PcbDoc types carry
//! resolved net names and component designators. Indices are resolved to human-
//! readable strings during the `board()` conversion.

use altium_format_types::color::Color;
use altium_format_types::common::Unit;
use altium_format_types::coord::{Coord, CoordPoint};
use altium_format_types::pcb::{
    ClassMemberKind, DimensionKind, LayerRef, PadShape, PadStackMode, PlaneConnectionStyle,
    RegionKind, RuleKind,
};

// ── Root type ───────────────────────────────────────────────────────────────

/// A parsed PcbDoc board with all cross-references resolved.
///
/// Obtained via [`PcbDoc::board()`]. Contains typed collections for every
/// section in the file, with net indices resolved to names and component
/// indices resolved to designators.
#[derive(Debug, Clone)]
pub struct PcbDocBoard {
    /// Board-level metadata (from Board6).
    pub settings: BoardSettings,

    // Named collections (from parameter sections)
    pub nets: Vec<Net>,
    pub components: Vec<PcbDocComponent>,
    pub polygons: Vec<Polygon>,
    pub classes: Vec<NetClass>,
    pub rules: Vec<DesignRule>,
    pub differential_pairs: Vec<DifferentialPair>,

    // Primitives (from binary sections, cross-referenced)
    pub tracks: Vec<Track>,
    pub arcs: Vec<Arc>,
    pub vias: Vec<Via>,
    pub pads: Vec<Pad>,
    pub fills: Vec<Fill>,
    pub texts: Vec<Text>,
    pub regions: Vec<Region>,
    pub component_bodies: Vec<ComponentBody>,

    // Dimensions and coordinates (prefixed param sections)
    pub dimensions: Vec<Dimension>,

    // Models (3D)
    pub models: Vec<Model3D>,
}

// ── Board settings ──────────────────────────────────────────────────────────

/// Board-level metadata extracted from the Board6 section.
///
/// Exposes a curated subset of the extensive Board6 configuration. Full layer
/// stack editing is deferred — internal data is preserved during roundtrip but
/// not exposed for mutation in the initial API.
#[derive(Debug, Clone)]
pub struct BoardSettings {
    pub document_name: String,
    pub signal_layer_count: i32,
    pub board_outline: Option<Vec<CoordPoint>>,
    pub snap_grid_size: Coord,
    pub visible_grid_size: Coord,
    pub display_unit: Unit,
}

// ── Named collections ───────────────────────────────────────────────────────

/// A PCB net. ID defaults to the net name (unique within a board).
#[derive(Debug, Clone)]
pub struct Net {
    pub id: String,
    pub name: String,
    pub color: Color,
    pub visible: bool,
}

/// A PCB component placement. ID defaults to the designator (unique within a board).
#[derive(Debug, Clone)]
pub struct PcbDocComponent {
    pub id: String,
    pub designator: String,
    pub pattern: String,
    pub comment: String,
    pub location: CoordPoint,
    pub rotation: f64,
    pub layer: LayerRef,
    pub source_library: String,
    pub source_lib_reference: String,
}

/// A copper polygon pour.
#[derive(Debug, Clone)]
pub struct Polygon {
    pub id: String,
    pub name: String,
    pub net: Option<String>,
    pub layer: LayerRef,
    pub connect_style: PlaneConnectionStyle,
    pub pour_order: i32,
    pub vertices: Vec<CoordPoint>,
    pub relief_conductor_width: Coord,
    pub relief_entries: i32,
    pub relief_air_gap: Coord,
}

/// A net/component/layer class.
#[derive(Debug, Clone)]
pub struct NetClass {
    pub id: String,
    pub name: String,
    pub kind: ClassMemberKind,
    pub members: Vec<String>,
}

/// A design rule.
#[derive(Debug, Clone)]
pub struct DesignRule {
    pub id: String,
    pub name: String,
    pub kind: RuleKind,
    pub enabled: bool,
    pub priority: i32,
    pub scope: String,
    pub comment: String,
}

/// A differential pair definition.
#[derive(Debug, Clone)]
pub struct DifferentialPair {
    pub id: String,
    pub name: String,
    pub positive_net: String,
    pub negative_net: String,
}

// ── Primitive types ─────────────────────────────────────────────────────────
//
// All primitives share: `id`, `layer`, `net: Option<String>`,
// `component: Option<String>`. Net and component are resolved from indices.

/// A PCB track segment.
#[derive(Debug, Clone)]
pub struct Track {
    pub id: String,
    pub layer: LayerRef,
    pub net: Option<String>,
    pub component: Option<String>,
    pub start: CoordPoint,
    pub end: CoordPoint,
    pub width: Coord,
}

/// A PCB arc segment.
#[derive(Debug, Clone)]
pub struct Arc {
    pub id: String,
    pub layer: LayerRef,
    pub net: Option<String>,
    pub component: Option<String>,
    pub center: CoordPoint,
    pub radius: Coord,
    pub start_angle: f64,
    pub end_angle: f64,
    pub width: Coord,
}

/// A PCB via.
#[derive(Debug, Clone)]
pub struct Via {
    pub id: String,
    pub net: Option<String>,
    pub component: Option<String>,
    pub location: CoordPoint,
    pub diameter: Coord,
    pub hole_size: Coord,
    pub from_layer: LayerRef,
    pub to_layer: LayerRef,
    pub solder_mask_expansion: Option<Coord>,
}

/// A PCB pad.
#[derive(Debug, Clone)]
pub struct Pad {
    pub id: String,
    pub pad_name: String,
    pub layer: LayerRef,
    pub net: Option<String>,
    pub component: Option<String>,
    pub location: CoordPoint,
    pub shape: PadShape,
    pub x_size: Coord,
    pub y_size: Coord,
    pub rotation: f64,
    pub hole_size: Coord,
    pub is_plated: bool,
    pub pad_mode: PadStackMode,
    pub solder_mask_expansion: Coord,
    pub paste_mask_expansion: Coord,
    pub plane_connection: PlaneConnectionStyle,
    pub relief_conductor_width: Coord,
    pub relief_entries: i32,
    pub relief_air_gap: Coord,
}

/// A PCB fill (solid rectangle).
#[derive(Debug, Clone)]
pub struct Fill {
    pub id: String,
    pub layer: LayerRef,
    pub net: Option<String>,
    pub component: Option<String>,
    pub corner1: CoordPoint,
    pub corner2: CoordPoint,
    pub rotation: f64,
}

/// A PCB text string.
#[derive(Debug, Clone)]
pub struct Text {
    pub id: String,
    pub layer: LayerRef,
    pub component: Option<String>,
    pub location: CoordPoint,
    pub text: String,
    pub height: Coord,
    pub width: Coord,
    pub rotation: f64,
    pub font_name: String,
    pub is_mirrored: bool,
    pub is_comment: bool,
    pub is_designator: bool,
}

/// A PCB region (copper pour, board cutout, keepout, etc.).
#[derive(Debug, Clone)]
pub struct Region {
    pub id: String,
    pub layer: LayerRef,
    pub net: Option<String>,
    pub component: Option<String>,
    pub kind: RegionKind,
    pub outline: Vec<CoordPoint>,
    pub holes: Vec<Vec<CoordPoint>>,
    pub is_board_cutout: bool,
    pub is_keepout: bool,
}

/// A 3D component body.
#[derive(Debug, Clone)]
pub struct ComponentBody {
    pub id: String,
    pub layer: LayerRef,
    pub component: Option<String>,
    pub standoff_height: Coord,
    pub overall_height: Coord,
    pub body_color_3d: Color,
    pub body_opacity_3d: f64,
    pub model_name: String,
    pub outline: Vec<CoordPoint>,
}

// ── Annotations ─────────────────────────────────────────────────────────────

/// A PCB dimension annotation.
#[derive(Debug, Clone)]
pub struct Dimension {
    pub id: String,
    pub kind: DimensionKind,
    pub layer: LayerRef,
    pub text_x: Coord,
    pub text_y: Coord,
    pub text_height: Coord,
    pub text_width: Coord,
}

/// A 3D model reference.
#[derive(Debug, Clone)]
pub struct Model3D {
    pub id: String,
    pub name: String,
    pub checksum: String,
}

// ── Query helpers ───────────────────────────────────────────────────────────

impl PcbDocBoard {
    // ── Net queries ─────────────────────────────────────────────────────

    /// Find a net by name.
    pub fn net(&self, name: &str) -> Option<&Net> {
        self.nets.iter().find(|n| n.name == name)
    }

    /// All tracks belonging to a given net.
    pub fn tracks_for_net(&self, net_name: &str) -> Vec<&Track> {
        self.tracks
            .iter()
            .filter(|t| t.net.as_deref() == Some(net_name))
            .collect()
    }

    /// All pads belonging to a given net.
    pub fn pads_for_net(&self, net_name: &str) -> Vec<&Pad> {
        self.pads
            .iter()
            .filter(|p| p.net.as_deref() == Some(net_name))
            .collect()
    }

    /// All vias belonging to a given net.
    pub fn vias_for_net(&self, net_name: &str) -> Vec<&Via> {
        self.vias
            .iter()
            .filter(|v| v.net.as_deref() == Some(net_name))
            .collect()
    }

    // ── Component queries ───────────────────────────────────────────────

    /// Find a component by designator.
    pub fn component(&self, designator: &str) -> Option<&PcbDocComponent> {
        self.components.iter().find(|c| c.designator == designator)
    }

    /// All pads belonging to a given component.
    pub fn pads_for_component(&self, designator: &str) -> Vec<&Pad> {
        self.pads
            .iter()
            .filter(|p| p.component.as_deref() == Some(designator))
            .collect()
    }

    /// All tracks belonging to a given component.
    pub fn tracks_for_component(&self, designator: &str) -> Vec<&Track> {
        self.tracks
            .iter()
            .filter(|t| t.component.as_deref() == Some(designator))
            .collect()
    }

    /// All component bodies belonging to a given component.
    pub fn bodies_for_component(&self, designator: &str) -> Vec<&ComponentBody> {
        self.component_bodies
            .iter()
            .filter(|b| b.component.as_deref() == Some(designator))
            .collect()
    }

    // ── Rule queries ────────────────────────────────────────────────────

    /// Find a design rule by name.
    pub fn rule(&self, name: &str) -> Option<&DesignRule> {
        self.rules.iter().find(|r| r.name == name)
    }

    /// All design rules of a given kind.
    pub fn rules_for_kind(&self, kind: RuleKind) -> Vec<&DesignRule> {
        self.rules.iter().filter(|r| r.kind == kind).collect()
    }
}
