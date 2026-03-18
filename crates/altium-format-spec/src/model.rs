use std::path::PathBuf;

use altium_format_types::{
    Color, ComponentKind, Coord, CoordPoint, LayerRef, PadShape, PadStackMode, PinElectricalType,
    PlaneConnectionStyle, RotationBy90,
};
use altium_format_types::sch::{
    HorizontalAlign, LeftRightSide, LineStyle, PenWidth, PortArrowStyle, PortIoType,
    PowerObjectStyle, TextJustification,
};
use altium_format_types::project::{
    ChannelRoomNamingStyle, ConnectionCode, CrossRefLocationStyle, CrossRefPorts,
    CrossRefSheetStyle, ErrorLevel, FlattenMode, SortLocation, SortOrder, VariationKind,
};

/// A layer specification in the spec language.
///
/// Layers can be specified as:
/// - A known V6/V7 layer name (resolved at compile time)
/// - A copper position like `copper(3)` (resolved at execution time against a board stack)
/// - A custom layer name (resolved at execution time against a board stack)
#[derive(Debug, Clone)]
pub enum LayerSpec {
    /// A fully resolved layer reference (e.g. "TopLayer", "Mechanical1").
    Resolved(LayerRef),
    /// The Nth copper layer (1-indexed), resolved against a board's layer stack.
    CopperPosition(usize),
    /// A custom layer name, resolved against a board's layer stack.
    NamedLayer(String),
}

// ── SchLib ──────────────────────────────────────────────────────────────────

pub struct SchLibSpec {
    pub components: Vec<ComponentSpec>,
}

pub struct ComponentSpec {
    pub lib_reference: String,
    pub designator: Option<String>,
    pub description: Option<String>,
    pub component_kind: Option<ComponentKind>,
    pub part_count: Option<i32>,
    pub show_hidden_pins: Option<bool>,

    pub pins: Vec<PinSpec>,
    pub parameters: Vec<ParameterSpec>,
    pub aliases: Vec<String>,
    pub footprints: Vec<FootprintMapSpec>,
    pub graphics: Vec<GraphicSpec>,

    pub parts: Vec<PartSpec>,
}

pub struct PartSpec {
    pub part_number: i32,
    pub pins: Vec<PinSpec>,
    pub graphics: Vec<GraphicSpec>,
}

pub struct PinSpec {
    pub designator: String,
    pub name: Option<String>,
    pub electrical: Option<PinElectricalType>,
    pub length: Option<Coord>,
    pub location: CoordPoint,
    pub orientation: RotationBy90,
    pub is_hidden: Option<bool>,
    pub hidden_net_name: Option<String>,
    pub owner_part_id: i32,
    pub swap_group: Option<String>,
    pub part_swap_group: Option<String>,
    pub pair_swap_group: Option<String>,
}

pub struct ParameterSpec {
    pub name: String,
    pub text: String,
    pub is_hidden: Option<bool>,
}

pub struct FootprintMapSpec {
    pub model_name: String,
    pub maps: Vec<PinPadMap>,
    pub source: Option<PathBuf>,
}

pub struct PinPadMap {
    pub pin: String,
    pub pad: String,
}

// ── SchDoc ──────────────────────────────────────────────────────────────────

pub struct SchDocSpec {
    pub sheets: Vec<SheetSpec>,
}

pub struct SheetSpec {
    // Sheet metadata
    pub fonts: Vec<FontSpec>,
    pub custom_width: Option<Coord>,
    pub custom_height: Option<Coord>,
    pub snap_grid_on: Option<bool>,
    pub visible_grid_on: Option<bool>,
    pub hot_spot_grid_on: Option<bool>,
    pub show_hidden_pins: Option<bool>,
    pub border_on: Option<bool>,
    pub title_block_on: Option<bool>,

    // Placed objects
    pub components: Vec<SchDocComponentSpec>,
    pub nets: Vec<NetSpec>,
    pub powers: Vec<PowerSpec>,
    pub objects: Vec<SchDocObjectSpec>,
}

pub struct FontSpec {
    pub id: i32,
    pub name: String,
    pub size: i32,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub strikeout: Option<bool>,
    pub rotation: Option<i32>,
}

pub struct SchDocComponentSpec {
    pub designator: String,
    pub symbol: SymbolRef,
    pub location: CoordPoint,
    pub orientation: Option<RotationBy90>,
    pub is_mirrored: Option<bool>,
    pub description: Option<String>,
    pub parameters: Vec<ParameterSpec>,
}

/// How a SchDoc component references its symbol.
#[derive(Debug, Clone)]
pub enum SymbolRef {
    /// `$alias.ComponentName` — references an imported SchLib.
    Import { alias: String, name: String },
    /// `lib_reference: "Name"` — direct library reference string.
    Literal(String),
}

pub struct NetSpec {
    pub name: String,
    pub pins: Vec<PinRef>,
}

pub struct PowerSpec {
    pub name: String,
    pub style: PowerObjectStyle,
    pub pins: Vec<PinRef>,
    pub show_net_name: Option<bool>,
    pub orientation: Option<RotationBy90>,
}

/// A reference to a specific pin on a placed component: "U1.14".
#[derive(Debug, Clone)]
pub struct PinRef {
    pub component: String,
    pub pin: String,
}

/// Low-level SchDoc objects for full dump roundtrip.
pub enum SchDocObjectSpec {
    Wire(WireSpec),
    Bus(BusSpec),
    NetLabel(NetLabelSpec),
    PowerObject(PowerObjectSpec),
    Port(PortSpec),
    Junction(JunctionSpec),
    NoConnect(NoConnectSpec),
    BusEntry(BusEntrySpec),
    SheetSymbol(SheetSymbolSpec),
    ParameterSet(ParameterSetSpec),
    Note(NoteSpec),
    Probe(ProbeSpec),
    CompileMask(CompileMaskSpec),
    Blanket(BlanketSpec),
    Graphic(GraphicSpec),
    Parameter(ParameterSpec),
    HarnessConnector(HarnessConnectorSpec),
    SignalHarness(SignalHarnessSpec),
}

pub struct WireSpec {
    pub vertices: Vec<CoordPoint>,
    pub color: Option<Color>,
    pub line_width: Option<PenWidth>,
    pub line_style: Option<LineStyle>,
}

pub struct BusSpec {
    pub vertices: Vec<CoordPoint>,
    pub color: Option<Color>,
    pub line_width: Option<PenWidth>,
}

pub struct NetLabelSpec {
    pub text: String,
    pub location: CoordPoint,
    pub orientation: Option<RotationBy90>,
    pub justification: Option<TextJustification>,
    pub font_id: Option<i32>,
    pub color: Option<Color>,
    pub is_mirrored: Option<bool>,
}

pub struct PowerObjectSpec {
    pub text: String,
    pub location: CoordPoint,
    pub orientation: Option<RotationBy90>,
    pub style: Option<PowerObjectStyle>,
    pub show_net_name: Option<bool>,
    pub font_id: Option<i32>,
    pub color: Option<Color>,
    pub is_cross_sheet_connector: Option<bool>,
}

pub struct PortSpec {
    pub name: String,
    pub location: CoordPoint,
    pub io_type: Option<PortIoType>,
    pub style: Option<PortArrowStyle>,
    pub width: Option<Coord>,
    pub height: Option<Coord>,
    pub color: Option<Color>,
    pub area_color: Option<Color>,
    pub text_color: Option<Color>,
    pub font_id: Option<i32>,
    pub alignment: Option<HorizontalAlign>,
}

pub struct JunctionSpec {
    pub location: CoordPoint,
    pub color: Option<Color>,
}

pub struct NoConnectSpec {
    pub location: CoordPoint,
    pub color: Option<Color>,
    pub orientation: Option<RotationBy90>,
}

pub struct BusEntrySpec {
    pub location: CoordPoint,
    pub corner: CoordPoint,
    pub color: Option<Color>,
    pub line_width: Option<PenWidth>,
}

pub struct SheetSymbolSpec {
    pub sheet_name: String,
    pub file_name: Option<String>,
    pub location: CoordPoint,
    pub x_size: Option<Coord>,
    pub y_size: Option<Coord>,
    pub color: Option<Color>,
    pub area_color: Option<Color>,
    pub entries: Vec<SheetEntrySpec>,
}

pub struct SheetEntrySpec {
    pub name: String,
    pub io_type: Option<PortIoType>,
    pub side: Option<LeftRightSide>,
    pub distance_from_top: Option<Coord>,
}

pub struct ParameterSetSpec {
    pub name: String,
    pub location: Option<CoordPoint>,
    pub parameters: Vec<ParameterSpec>,
}

pub struct NoteSpec {
    pub location: CoordPoint,
    pub text: String,
    pub color: Option<Color>,
    pub area_color: Option<Color>,
    pub font_id: Option<i32>,
}

pub struct ProbeSpec {
    pub name: String,
    pub location: CoordPoint,
    pub color: Option<Color>,
}

pub struct CompileMaskSpec {
    pub location: CoordPoint,
    pub corner: CoordPoint,
    pub color: Option<Color>,
}

pub struct BlanketSpec {
    pub location: CoordPoint,
    pub corner: CoordPoint,
    pub vertices: Option<Vec<CoordPoint>>,
    pub color: Option<Color>,
}

pub struct HarnessConnectorSpec {
    pub location: CoordPoint,
    pub x_size: Option<Coord>,
    pub y_size: Option<Coord>,
    pub color: Option<Color>,
    pub area_color: Option<Color>,
}

pub struct SignalHarnessSpec {
    pub vertices: Vec<CoordPoint>,
    pub color: Option<Color>,
    pub line_width: Option<PenWidth>,
}

// ── PcbLib ──────────────────────────────────────────────────────────────────

pub struct PcbLibSpec {
    pub footprints: Vec<FootprintSpec>,
}

pub struct FootprintSpec {
    pub display_name: String,
    pub description: Option<String>,
    pub height: Option<Coord>,
    pub pattern: Option<String>,

    pub pads: Vec<PadSpec>,
    pub graphics: Vec<PcbGraphicSpec>,
}

pub struct PadSpec {
    pub pad_name: String,
    pub at: CoordPoint,
    pub shape: Option<PadShape>,
    pub x_size: Option<Coord>,
    pub y_size: Option<Coord>,
    pub rotation: Option<f64>,
    pub hole_size: Option<Coord>,
    pub is_plated: Option<bool>,
    pub layer: Option<LayerSpec>,
    pub pad_mode: Option<PadStackMode>,
    pub solder_mask_expansion: Option<Coord>,
    pub paste_mask_expansion: Option<Coord>,
    pub plane_connection: Option<PlaneConnectionStyle>,
    pub relief_conductor_width: Option<Coord>,
    pub relief_entries: Option<i32>,
    pub relief_air_gap: Option<Coord>,
}

// ── Graphics (Schematic) ─────────────────────────────────────────────────────

pub struct GraphicSpec {
    pub unique_id: String,
    pub graphic_type: GraphicType,
    pub properties: GraphicProperties,
}

#[derive(Debug, Clone)]
pub enum GraphicType {
    Line,
    Rectangle,
    Arc,
    EllipticalArc,
    Ellipse,
    Polyline,
    Polygon,
    Bezier,
    Pie,
    RoundRectangle,
    Label,
    TextFrame,
    Image,
}

pub struct GraphicProperties {
    // Box types (rectangle, round_rectangle, text_frame, image)
    pub from: Option<CoordPoint>,
    pub to: Option<CoordPoint>,
    pub is_solid: Option<bool>,
    pub corner_x_radius: Option<Coord>,
    pub corner_y_radius: Option<Coord>,

    // Center+radius types (arc, ellipse, pie)
    pub center: Option<CoordPoint>,
    pub radius: Option<Coord>,
    pub secondary_radius: Option<Coord>,
    pub start_angle: Option<f64>,
    pub end_angle: Option<f64>,

    // Vertex-list types (polyline, polygon, bezier)
    pub points: Option<Vec<CoordPoint>>,

    // Common visual
    pub color: Option<Color>,
    pub area_color: Option<Color>,
    pub line_width: Option<Coord>,

    // Text (label, text_frame)
    pub text: Option<String>,
    pub font_id: Option<i32>,
    pub at: Option<CoordPoint>,

    // Image
    pub file_name: Option<String>,
    pub image_data: Option<Vec<u8>>,

    // PCB-specific
    pub layer: Option<LayerSpec>,
    pub width: Option<Coord>,
    pub closed: Option<bool>,
    pub show_border: Option<bool>,
}

// ── Graphics (PCB) ───────────────────────────────────────────────────────────

pub struct PcbGraphicSpec {
    pub unique_id: String,
    pub graphic_type: PcbGraphicType,
    pub properties: PcbGraphicProperties,
}

#[derive(Debug, Clone)]
pub enum PcbGraphicType {
    Track,
    Arc,
    Fill,
    Region,
    Text,
    Via,
    ComponentBody,
    Polyline,
}

pub struct PcbGraphicProperties {
    pub layer: Option<LayerSpec>,
    pub width: Option<Coord>,
    pub from: Option<CoordPoint>,
    pub to: Option<CoordPoint>,
    pub center: Option<CoordPoint>,
    pub radius: Option<Coord>,
    pub start_angle: Option<f64>,
    pub end_angle: Option<f64>,
    pub points: Option<Vec<CoordPoint>>,
    pub text: Option<String>,
    pub at: Option<CoordPoint>,
    pub rotation: Option<f64>,
    pub hole_size: Option<Coord>,
    pub diameter: Option<Coord>,
    pub is_solid: Option<bool>,
}

// ── Domain / SpecModel ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecDomain {
    SchLib,
    SchDoc,
    PcbLib,
    PcbDoc,
    PrjPcb,
}

pub enum SpecModel {
    SchLib(SchLibSpec),
    SchDoc(SchDocSpec),
    PcbLib(PcbLibSpec),
    PcbDoc(PcbDocSpec),
    PrjPcb(PrjPcbSpec),
}

// ── PcbDoc ──────────────────────────────────────────────────────────────────

pub struct PcbDocSpec {
    pub boards: Vec<BoardSpec>,
    pub placement: Option<PlacementSpec>,
    pub placement_rules: Vec<PlacementRuleSpec>,
}

pub struct BoardSpec {
    pub name: String,
    pub signal_layer_count: Option<i32>,
    pub snap_grid_size: Option<Coord>,
    pub visible_grid_size: Option<Coord>,
    pub display_unit: Option<String>,

    pub nets: Vec<PcbDocNetSpec>,
    pub components: Vec<PcbDocComponentSpec>,
    pub tracks: Vec<PcbDocPrimitiveSpec>,
    pub arcs: Vec<PcbDocPrimitiveSpec>,
    pub vias: Vec<PcbDocPrimitiveSpec>,
    pub pads: Vec<PcbDocPrimitiveSpec>,
    pub fills: Vec<PcbDocPrimitiveSpec>,
    pub texts: Vec<PcbDocPrimitiveSpec>,
    pub regions: Vec<PcbDocPrimitiveSpec>,
    pub component_bodies: Vec<PcbDocPrimitiveSpec>,
    pub dimensions: Vec<PcbDocPrimitiveSpec>,
    pub polygons: Vec<PcbDocPolygonSpec>,
    pub rules: Vec<PcbDocRuleSpec>,
    pub classes: Vec<PcbDocClassSpec>,
    pub differential_pairs: Vec<PcbDocDifferentialPairSpec>,
}

/// A generic PcbDoc primitive spec (track, arc, via, pad, fill, text, region, component_body, dimension).
/// Properties are stored as evaluated key-value pairs; the executor converts them to typed API objects.
pub struct PcbDocPrimitiveSpec {
    pub id: String,
    pub position_index: usize,
    pub primitive_type: String,
    pub properties: indexmap::IndexMap<String, crate::eval::Value>,
}

pub struct PcbDocNetSpec {
    pub name: String,
    pub color: Option<Color>,
    pub visible: Option<bool>,
}

pub struct PcbDocComponentSpec {
    pub designator: String,
    pub pattern: Option<String>,
    pub comment: Option<String>,
    pub location: Option<CoordPoint>,
    pub rotation: Option<f64>,
    pub layer: Option<LayerSpec>,
    pub source_library: Option<String>,
}

pub struct PcbDocPolygonSpec {
    pub name: String,
    pub net: Option<String>,
    pub layer: Option<LayerSpec>,
    pub connect_style: Option<String>,
    pub pour_order: Option<i32>,
}

pub struct PcbDocRuleSpec {
    pub name: String,
    pub kind: Option<String>,
    pub enabled: Option<bool>,
    pub priority: Option<i32>,
}

pub struct PcbDocClassSpec {
    pub name: String,
    pub kind: Option<String>,
}

pub struct PcbDocDifferentialPairSpec {
    pub name: String,
    pub positive_net: Option<String>,
    pub negative_net: Option<String>,
}

pub struct PlacementSpec {
    pub target: Option<String>,
    pub places: Vec<PlacementPlaceSpec>,
    pub constraints: Vec<PlacementConstraintSpec>,
    pub optimize: PlacementOptimizeSpec,
    pub clearance: PlacementClearanceSpec,
    pub autoplace_config: Option<AutoplaceConfig>,
    pub unplaced: UnplacedStrategy,
    pub allow_pin_swap: bool,
    pub allow_part_swap: bool,
    pub allow_gate_swap: bool,
    pub groups: Vec<PlacementGroupSpec>,
}

pub struct PlacementPlaceSpec {
    pub designators: Vec<String>,
    pub region_name: Option<String>,
    pub region_rect: Option<(CoordPoint, CoordPoint)>,
    pub edge: Option<String>,
    pub inset: Option<Coord>,
    pub near: Option<String>,
    pub max_distance: Option<Coord>,
    pub rotation_options: Vec<i32>,
    pub fixed: bool,
    pub at: Option<CoordPoint>,
    pub side: Option<String>,
    pub autoplace: bool,
    pub no_pin_swap: Vec<String>,
    pub no_part_swap: bool,
}

/// Strategy for components present in PcbDoc but not mentioned in the spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnplacedStrategy {
    #[default]
    Autoplace,
    Ignore,
    Error,
}

/// Configuration for the autoplace solver.
pub struct AutoplaceConfig {
    pub algorithm: Option<String>,
    pub sa_cooling: Option<f64>,
    pub sa_moves_per_temp: Option<usize>,
    pub sa_max_steps: Option<usize>,
    pub enable_net_crossings: Option<bool>,
    pub default_clearance: Option<Coord>,
    pub board_edge_clearance: Option<Coord>,
    pub grid_snap: Option<Coord>,
    pub auto_cluster: Option<bool>,
}

/// A named group of components for placement solver grouping.
pub struct PlacementGroupSpec {
    pub name: String,
    pub components: Vec<String>,
}

pub enum PlacementConstraintSpec {
    LeftOf { a: String, b: String, gap: Option<Coord> },
    RightOf { a: String, b: String, gap: Option<Coord> },
    Above { a: String, b: String, gap: Option<Coord> },
    Below { a: String, b: String, gap: Option<Coord> },
}

pub struct PlacementOptimizeSpec {
    pub ratsnest: bool,
    pub ratsnest_weight: f64,
}

pub struct PlacementClearanceSpec {
    pub all: Option<Coord>,
    pub edge: Option<Coord>,
}

pub struct PlacementRuleSpec {
    pub name: String,
    pub kind: Option<String>,
    pub gap: Option<Coord>,
}

// ── PrjPcb ──────────────────────────────────────────────────────────────────

pub struct PrjPcbSpec {
    pub projects: Vec<ProjectSpec>,
}

pub struct ProjectSpec {
    pub name: String,

    // [Design] scalar properties — all Option (None = don't override)
    pub hierarchy_mode: Option<FlattenMode>,
    pub channel_room_naming_style: Option<ChannelRoomNamingStyle>,
    pub channel_designator_format: Option<String>,
    pub channel_room_level_separator: Option<String>,
    pub allow_port_net_names: Option<bool>,
    pub allow_sheet_entry_net_names: Option<bool>,
    pub netlist_single_pin_nets: Option<bool>,
    pub append_sheet_number_to_local_nets: Option<bool>,
    pub name_nets_hierarchically: Option<bool>,
    pub power_port_names_take_priority: Option<bool>,
    pub pin_swap_by_netlabel: Option<bool>,
    pub pin_swap_by_pin: Option<bool>,
    pub cross_ref_sheet_style: Option<CrossRefSheetStyle>,
    pub cross_ref_location_style: Option<CrossRefLocationStyle>,
    pub cross_ref_ports: Option<CrossRefPorts>,
    pub cross_ref_cross_sheets: Option<bool>,
    pub cross_ref_sheet_entries: Option<bool>,
    pub output_path: Option<String>,

    // Children
    pub documents: Vec<DocumentSpec>,
    pub annotation: Option<AnnotationSpec>,
    pub erc_matrix_overrides: Vec<ErcMatrixOverride>,
    pub erc_level_overrides: Vec<ErcLevelOverride>,
    pub output_groups: Vec<OutputGroupSpec>,
    pub comparison_rules: Vec<ComparisonRuleSpec>,
    pub class_gen: Option<ClassGenSpec>,
    pub library_update: Option<LibraryUpdateSpec>,
    pub variants: Vec<VariantSpec>,
}

pub struct DocumentSpec {
    pub path: String,
    pub annotation_enabled: Option<bool>,
    pub annotate_start_value: Option<i32>,
    pub do_library_update: Option<bool>,
    pub do_database_update: Option<bool>,
}

pub struct AnnotationSpec {
    pub sort_order: Option<SortOrder>,
    pub sort_location: Option<SortLocation>,
    pub match_parameters: Vec<AnnotationMatchParamSpec>,
}

pub struct AnnotationMatchParamSpec {
    pub index: i32,
    pub properties: indexmap::IndexMap<String, String>,
}

pub struct ErcMatrixOverride {
    pub row: ConnectionCode,
    pub col: ConnectionCode,
    pub level: ErrorLevel,
}

pub struct ErcLevelOverride {
    pub name: String,
    pub level: ErrorLevel,
}

pub struct OutputGroupSpec {
    pub name: String,
    pub description: Option<String>,
    pub outputs: Vec<OutputSpec>,
}

pub struct OutputSpec {
    pub name: String,
    pub output_type: Option<String>,
    pub document_path: Option<String>,
    pub variant_name: Option<String>,
}

pub struct ComparisonRuleSpec {
    pub kind: String,
    pub properties: indexmap::IndexMap<String, String>,
}

pub struct ClassGenSpec {
    pub generate_component_classes: Option<bool>,
    pub generate_net_classes: Option<bool>,
}

pub struct LibraryUpdateSpec {
    pub update_components: Option<bool>,
    pub update_models: Option<bool>,
}

pub struct VariantSpec {
    pub name: String,
    pub description: Option<String>,
    pub variations: Vec<VariationSpec>,
    pub param_variations: Vec<ParamVariationSpec>,
}

pub struct VariationSpec {
    pub designator: String,
    pub kind: Option<VariationKind>,
    pub alternate_part: Option<String>,
}

pub struct ParamVariationSpec {
    pub designator: String,
    pub parameter: String,
    pub value: String,
}
