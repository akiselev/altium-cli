//! Public API types for PcbDoc documents.
//!
//! These types provide a clean, domain-typed interface for reading PCB board
//! designs. The read path in `pcbdoc_read.rs` handles conversion from internal
//! `PcbDoc` sections to these public types.
//!
//! Unlike PcbLib (footprint-level, no net/component context), PcbDoc types carry
//! resolved net names and component designators. Indices are resolved to human-
//! readable strings during the `board()` conversion.

use super::pcb_common::PadStack;
use altium_format_types::color::Color;
use altium_format_types::common::Unit;
use altium_format_types::coord::{Coord, CoordPoint};
use altium_format_types::pcb::{
    BgaFanoutDirection, BgaFanoutViaMode, ComponentCollisionCheckMode, CornerStyle,
    FanoutDirection, FanoutStyle, NetTopology, PolygonReliefAngle, RouteVia,
};
use altium_format_types::pcb::{
    ClassMemberKind, DimensionKind, LayerRef, PadShape, PadStackMode, PlaneConnectionStyle,
    RegionKind, RuleKind,
};
use altium_format_types::{
    ComponentPlacementType, ConfinementStyle, DielectricType, LayerStackStyle, NetScope,
    RuleLayerKind,
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
    /// Full layer stack metadata from Board6 configuration.
    pub layer_stack: LayerStack,
    /// Board geometry with arc-preserving contours.
    pub geometry: BoardGeometry,
}

// ── Layer stack ─────────────────────────────────────────────────────────────

/// The PCB layer stack describing copper and dielectric layer ordering.
///
/// Extracted from whichever layer stack version is present in the file
/// (V9 > V8 > V7 > legacy, first non-empty wins). Contains only copper
/// and dielectric layers in physical order from top to bottom.
#[derive(Debug, Clone)]
pub struct LayerStack {
    /// Stack construction style (LayerPairs, InternalLayerPairs, BuildUp).
    pub style: LayerStackStyle,
    /// Whether this is a flex PCB stack.
    pub is_flex: bool,
    /// Layers in physical order from top to bottom.
    pub layers: Vec<StackLayer>,
    /// Number of copper layers (signal + internal plane).
    pub copper_layer_count: usize,
}

/// A single layer in the physical stack.
#[derive(Debug, Clone)]
pub struct StackLayer {
    /// Layer reference for cross-referencing with primitives.
    pub layer: LayerRef,
    /// Human-readable layer name (e.g. "Top Layer", "Mid-Layer 1").
    pub name: String,
    /// 1-based physical position from top (1 = topmost copper).
    pub physical_order: usize,
    /// Whether this is an internal plane layer.
    pub is_plane: bool,
    /// Copper thickness for this layer.
    pub copper_thickness: Coord,
    /// Type of dielectric below this layer.
    pub dielectric_type: DielectricType,
    /// Dielectric constant (Er) for the dielectric below this layer.
    pub dielectric_constant: f64,
    /// Dielectric height (thickness) below this layer.
    pub dielectric_height: Coord,
    /// Dielectric material name (e.g. "FR-4").
    pub dielectric_material: String,
    /// Component placement allowed on this layer (top/bottom/not allowed).
    pub component_placement: Option<ComponentPlacementType>,
}

impl LayerStack {
    /// Get the topmost layer.
    pub fn top(&self) -> Option<&StackLayer> {
        self.layers.first()
    }

    /// Get the bottommost layer.
    pub fn bottom(&self) -> Option<&StackLayer> {
        self.layers.last()
    }

    /// Find a layer by its `LayerRef`.
    pub fn layer(&self, layer: &LayerRef) -> Option<&StackLayer> {
        self.layers.iter().find(|l| l.layer == *layer)
    }

    /// Get the physical order (1-based) of a layer.
    pub fn physical_order(&self, layer: &LayerRef) -> Option<usize> {
        self.layer(layer).map(|l| l.physical_order)
    }

    /// Get all inner layers (excluding top and bottom).
    pub fn inner_layers(&self) -> &[StackLayer] {
        if self.layers.len() <= 2 {
            &[]
        } else {
            &self.layers[1..self.layers.len() - 1]
        }
    }
}

// ── Board geometry ─────────────────────────────────────────────────────────

/// Board geometry with arc-preserving contours.
///
/// Extracted from regions with `is_board_cutout` and `keepout` flags.
/// Unlike `BoardSettings.board_outline` (which flattens arcs to points), these
/// contours preserve arc segments for accurate Gerber output and DRC.
#[derive(Debug, Clone)]
pub struct BoardGeometry {
    /// Primary board outline (first board cutout region found).
    pub outline: Option<BoardContour>,
    /// Additional cutout regions (holes in the board).
    pub cutouts: Vec<BoardContour>,
    /// Keepout zones.
    pub keepouts: Vec<KeepoutZone>,
}

/// A board contour — type alias for the shared `PcbContour`.
pub type BoardContour = super::pcb_common::PcbContour;

/// A keepout zone on a specific layer.
#[derive(Debug, Clone)]
pub struct KeepoutZone {
    pub outline: BoardContour,
    pub layer: LayerRef,
}

// `BoardContour::to_points()` is inherited from `PcbContour`.

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
    pub source_unique_id: String,
    pub source_hierarchical_path: String,
    /// BOM parameters (name, value) pairs, e.g. ("Value", "100R").
    pub parameters: Vec<(String, String)>,
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
    pub scope2: String,
    pub net_scope: NetScope,
    pub layer_scope: RuleLayerKind,
    pub comment: String,
    /// Typed rule parameters. `RuleParams::Other` for unrecognized kinds.
    pub params: RuleParams,
}

/// Typed rule parameters extracted from the internal `PcbRuleKindData`.
///
/// Common rules (clearance, width, mask expansion, etc.) have fully typed
/// variants. Less common rules use `Other { kind }`.
#[derive(Debug, Clone)]
pub enum RuleParams {
    Clearance {
        gap: Coord,
        ignore_pad_to_pad: bool,
    },
    Width {
        min: Coord,
        max: Coord,
        preferred: Coord,
    },
    Length {
        min: Coord,
        max: Coord,
    },
    MatchedLengths {
        tolerance: Coord,
    },
    ParallelSegment {
        gap: Coord,
        limit: Coord,
        parallel_length: Coord,
    },
    DaisyChainStubLength {
        max_limit: Coord,
    },
    ShortCircuit {
        allowed: bool,
    },
    BrokenNets {
        check_bad_connections: bool,
    },
    ViasUnderSmd {
        allowed: bool,
    },
    MaximumViaCount {
        max_via_count: u32,
    },
    MinimumAnnularRing {
        min: Coord,
    },
    HoleToHoleClearance {
        gap: Coord,
    },
    BoardOutlineClearance {
        gap: Coord,
    },
    MaxMinHoleSize {
        min: Coord,
        max: Coord,
    },
    SolderMaskExpansion {
        expansion: Coord,
        is_tenting_top: bool,
        is_tenting_bottom: bool,
    },
    PasteMaskExpansion {
        expansion: Coord,
        percent: f64,
    },
    PowerPlaneClearance {
        clearance: Coord,
    },
    PowerPlaneConnectStyle {
        connect_style: PlaneConnectionStyle,
        relief_conductor_width: Coord,
        relief_entries: i32,
        relief_air_gap: Coord,
    },
    PolygonConnectStyle {
        connect_style: PlaneConnectionStyle,
        relief_conductor_width: Coord,
        relief_entries: i32,
        relief_angle: PolygonReliefAngle,
        air_gap_width: Coord,
    },
    RoutingTopology {
        topology: NetTopology,
    },
    RoutingPriority {
        priority: i32,
    },
    RoutingLayers {
        /// (layer_name, enabled) pairs.
        layer_flags: Vec<(String, bool)>,
    },
    RoutingCornerStyle {
        corner_style: CornerStyle,
        min_setback: Coord,
        max_setback: Coord,
    },
    RoutingViaStyle {
        min_hole_width: Coord,
        max_hole_width: Coord,
        preferred_hole_width: Coord,
        min_width: Coord,
        max_width: Coord,
        preferred_width: Coord,
        via_style: RouteVia,
    },
    ComponentClearance {
        gap: Coord,
        collision_check_mode: ComponentCollisionCheckMode,
        vertical_gap: Coord,
    },
    ConfinementConstraint {
        confinement_style: ConfinementStyle,
    },
    DiffPairsRouting {
        min_gap: Coord,
        max_gap: Coord,
        preferred_gap: Coord,
        max_uncoupled_length: Coord,
    },
    FanoutControl {
        bga_dir: BgaFanoutDirection,
        bga_via_mode: BgaFanoutViaMode,
        fanout_style: FanoutStyle,
        fanout_direction: FanoutDirection,
    },
    MaxMinHeight {
        min_height: Coord,
        max_height: Coord,
        pref_height: Coord,
    },
    MinimumSolderMaskSliver {
        min_width: Coord,
    },
    SilkToSolderMaskClearance {
        gap: Coord,
    },
    SilkToSilkClearance {
        gap: Coord,
    },
    NetAntennae {
        tolerance: Coord,
    },
    SmdToCorner {
        distance: Coord,
    },
    SmdToPlane {
        distance: Coord,
    },
    SmdNeckDown {
        percent: f64,
    },
    SmdEntry {
        side: bool,
        corner: bool,
        any_angle: bool,
    },
    UnpouredPolygon {
        allow_unpoured: bool,
    },
    BackDrilling {
        depth: Coord,
    },
    CreepageDistance {
        gap: Coord,
    },
    AcuteAngle {
        minimum: f64,
    },
    LayerPair {
        enforce: bool,
    },
    /// Fallback for rule kinds not yet given a typed variant.
    Other {
        kind: RuleKind,
    },
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
    /// Per-layer pad shapes. Only populated for non-Simple pad modes.
    pub stack: PadStack,
    /// Pin-level swap group ID from the originating SchLib pin record.
    /// Not stored in PcbDoc format — populated during footprint instantiation.
    pub swap_id_pin: Option<String>,
    /// Part-level swap group ID from the originating SchLib pin record.
    /// Not stored in PcbDoc format — populated during footprint instantiation.
    pub swap_id_part: Option<String>,
}

// PadStack, PadLayerShape, PadInnerLayerOverride are re-exported from pcb_common.

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

// ── Connectivity ────────────────────────────────────────────────────────────

/// Board connectivity: pads grouped by net.
#[derive(Debug, Clone)]
pub struct BoardConnectivity {
    pub net_pins: Vec<NetPinList>,
}

/// All pins (pads) belonging to a single net.
#[derive(Debug, Clone)]
pub struct NetPinList {
    pub net_name: String,
    pub pins: Vec<NetPin>,
    /// Number of distinct components connected to this net.
    pub component_count: usize,
}

/// A single pin (pad) in a net's connectivity list.
#[derive(Debug, Clone)]
pub struct NetPin {
    pub component: Option<String>,
    pub pad_name: String,
    pub location: CoordPoint,
}

/// Primitives on a single layer.
#[derive(Debug, Clone)]
pub struct LayerPrimitives<'a> {
    pub tracks: Vec<&'a Track>,
    pub arcs: Vec<&'a Arc>,
    pub pads: Vec<&'a Pad>,
    pub fills: Vec<&'a Fill>,
    pub texts: Vec<&'a Text>,
    pub regions: Vec<&'a Region>,
}

/// A group of vias sharing the same drill pair (from/to layers).
#[derive(Debug, Clone)]
pub struct DrillPairGroup<'a> {
    pub from_layer: LayerRef,
    pub to_layer: LayerRef,
    pub vias: Vec<&'a Via>,
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

    // ── Layer queries ────────────────────────────────────────────────────

    /// All tracks on a given layer.
    pub fn tracks_on_layer(&self, layer: &LayerRef) -> Vec<&Track> {
        self.tracks.iter().filter(|t| t.layer == *layer).collect()
    }

    /// All pads on a given layer.
    pub fn pads_on_layer(&self, layer: &LayerRef) -> Vec<&Pad> {
        self.pads.iter().filter(|p| p.layer == *layer).collect()
    }

    /// All primitives on a given layer.
    pub fn primitives_on_layer(&self, layer: &LayerRef) -> LayerPrimitives<'_> {
        LayerPrimitives {
            tracks: self.tracks.iter().filter(|t| t.layer == *layer).collect(),
            arcs: self.arcs.iter().filter(|a| a.layer == *layer).collect(),
            pads: self.pads.iter().filter(|p| p.layer == *layer).collect(),
            fills: self.fills.iter().filter(|f| f.layer == *layer).collect(),
            texts: self.texts.iter().filter(|t| t.layer == *layer).collect(),
            regions: self.regions.iter().filter(|r| r.layer == *layer).collect(),
        }
    }

    // ── Polygon queries ──────────────────────────────────────────────────

    /// All regions that belong to a named polygon (matched by net).
    pub fn regions_for_polygon(&self, polygon_name: &str) -> Vec<&Region> {
        // Find the polygon's net.
        let poly_net = self
            .polygons
            .iter()
            .find(|p| p.name == polygon_name)
            .and_then(|p| p.net.as_deref());
        match poly_net {
            Some(net) => self
                .regions
                .iter()
                .filter(|r| r.net.as_deref() == Some(net))
                .collect(),
            None => Vec::new(),
        }
    }

    // ── Via queries ──────────────────────────────────────────────────────

    /// Group vias by their drill pair (from_layer, to_layer).
    pub fn vias_by_drill_pair(&self) -> Vec<DrillPairGroup<'_>> {
        let mut groups: std::collections::HashMap<(LayerRef, LayerRef), Vec<&Via>> =
            std::collections::HashMap::new();
        for via in &self.vias {
            groups
                .entry((via.from_layer.clone(), via.to_layer.clone()))
                .or_default()
                .push(via);
        }
        groups
            .into_iter()
            .map(|((from, to), vias)| DrillPairGroup {
                from_layer: from,
                to_layer: to,
                vias,
            })
            .collect()
    }

    // ── Pad queries ──────────────────────────────────────────────────────

    /// All plated through-hole pads.
    pub fn plated_through_hole_pads(&self) -> Vec<&Pad> {
        self.pads
            .iter()
            .filter(|p| p.is_plated && p.hole_size != Coord::ZERO)
            .collect()
    }

    /// All non-plated through-hole pads.
    pub fn non_plated_through_hole_pads(&self) -> Vec<&Pad> {
        self.pads
            .iter()
            .filter(|p| !p.is_plated && p.hole_size != Coord::ZERO)
            .collect()
    }

    // ── Connectivity ─────────────────────────────────────────────────────

    /// Build net connectivity from pads.
    pub fn connectivity(&self) -> BoardConnectivity {
        let mut net_map: std::collections::HashMap<&str, Vec<NetPin>> =
            std::collections::HashMap::new();
        for pad in &self.pads {
            if let Some(net_name) = pad.net.as_deref() {
                net_map.entry(net_name).or_default().push(NetPin {
                    component: pad.component.clone(),
                    pad_name: pad.pad_name.clone(),
                    location: pad.location,
                });
            }
        }

        let mut net_pins: Vec<NetPinList> = net_map
            .into_iter()
            .map(|(net_name, pins)| {
                let component_count = {
                    let mut comps: std::collections::HashSet<&str> =
                        std::collections::HashSet::new();
                    for pin in &pins {
                        if let Some(comp) = pin.component.as_deref() {
                            comps.insert(comp);
                        }
                    }
                    comps.len()
                };
                NetPinList {
                    net_name: net_name.to_string(),
                    pins,
                    component_count,
                }
            })
            .collect();

        net_pins.sort_by(|a, b| a.net_name.cmp(&b.net_name));
        BoardConnectivity { net_pins }
    }
}
