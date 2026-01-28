// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! DSL type definitions for the import format.

use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════════════
// TOP-LEVEL ENVELOPE
// ═══════════════════════════════════════════════════════════════════════════

/// Top-level import file. The `format` field selects the variant.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "format", rename_all = "lowercase")]
pub enum ImportFile {
    SchLib(SchLibImport),
    PcbLib(PcbLibImport),
    SchDoc(SchDocImport),
}

// ═══════════════════════════════════════════════════════════════════════════
// SCHLIB IMPORT
// ═══════════════════════════════════════════════════════════════════════════

/// A complete schematic library definition.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SchLibImport {
    pub components: Vec<SchLibComponentDef>,
}

/// A single schematic symbol component.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SchLibComponentDef {
    /// Component name / lib_reference (e.g., "LM358")
    pub name: String,

    /// Component description
    #[serde(default)]
    pub description: String,

    /// Component kind for BOM/sync behavior
    #[serde(default)]
    pub kind: ComponentKind,

    /// Symbol generation style
    #[serde(default)]
    pub style: SymbolStyle,

    /// Body width (e.g., "600mil", "15.24mm"). Auto-sized if omitted.
    #[serde(default)]
    pub width: Option<String>,

    /// Pin-to-pin spacing (default: "100mil")
    #[serde(default)]
    pub pin_spacing: Option<String>,

    /// Pin stub length (default: "200mil")
    #[serde(default)]
    pub pin_length: Option<String>,

    /// Number of parts (for multi-part symbols like dual op-amps)
    #[serde(default = "default_one")]
    pub part_count: i32,

    /// Number of display modes (normal, demorgan, ieee)
    #[serde(default = "default_one")]
    pub display_modes: i32,

    /// Pin definitions
    #[serde(default)]
    pub pins: Vec<SchLibPinDef>,

    /// Extra graphics primitives beyond the auto-generated body
    #[serde(default)]
    pub graphics: Vec<SchLibGraphic>,

    // ── Power symbol fields ──
    /// Power port symbol style (only for style: power)
    #[serde(default)]
    pub power_style: Option<String>,

    /// Hidden net name for power symbols
    #[serde(default)]
    pub net_name: Option<String>,

    // ── Connector fields ──
    /// Number of columns (for style: connector)
    #[serde(default)]
    pub columns: Option<usize>,

    /// Number of rows (for style: connector)
    #[serde(default)]
    pub rows: Option<usize>,
}

/// Component kind (Altium TComponentKind).
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ComponentKind {
    #[default]
    Standard,
    StandardNoBom,
    Mechanical,
    Graphical,
    NetTie,
    NetTieNoBom,
}

/// Symbol generation style.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SymbolStyle {
    /// Standard IC: rectangle body with pins on 4 sides, auto-sized
    #[default]
    Ic,
    /// Two-pin passive (resistor, capacitor, etc.)
    Discrete,
    /// Power port symbol (VCC, GND, etc.)
    Power,
    /// Multi-row pin grid connector
    Connector,
}

/// A pin definition for a schematic symbol.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SchLibPinDef {
    /// Pin designator / pad number (e.g., "1", "A1")
    pub designator: String,

    /// Pin name (e.g., "VCC", "GND", "DATA0")
    pub name: String,

    /// Which side of the body (for ic/connector styles)
    #[serde(default = "default_left")]
    pub side: PinSide,

    /// Electrical type
    #[serde(default)]
    pub r#type: PinElectrical,

    /// Pin description text
    #[serde(default)]
    pub description: String,

    /// Hide pin from display
    #[serde(default)]
    pub hidden: bool,
}

/// Which side of the component body a pin attaches to.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PinSide {
    #[default]
    Left,
    Right,
    Top,
    Bottom,
}

/// Pin electrical type.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PinElectrical {
    Input,
    Output,
    Io,
    #[default]
    Passive,
    Power,
    Oc,
    Oe,
    Hiz,
}

impl PinElectrical {
    pub fn to_str(&self) -> &'static str {
        match self {
            PinElectrical::Input => "input",
            PinElectrical::Output => "output",
            PinElectrical::Io => "io",
            PinElectrical::Passive => "passive",
            PinElectrical::Power => "power",
            PinElectrical::Oc => "oc",
            PinElectrical::Oe => "oe",
            PinElectrical::Hiz => "hiz",
        }
    }
}

impl PinSide {
    pub fn to_str(&self) -> &'static str {
        match self {
            PinSide::Left => "left",
            PinSide::Right => "right",
            PinSide::Top => "top",
            PinSide::Bottom => "bottom",
        }
    }
}

/// Extra graphics primitive for a schematic symbol.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SchLibGraphic {
    Line {
        x1: String,
        y1: String,
        x2: String,
        y2: String,
        #[serde(default = "default_border_color")]
        color: String,
    },
    Rectangle {
        x1: String,
        y1: String,
        x2: String,
        y2: String,
        #[serde(default)]
        filled: bool,
        #[serde(default = "default_fill_color")]
        fill_color: String,
        #[serde(default = "default_border_color")]
        border_color: String,
    },
    Arc {
        x: String,
        y: String,
        radius: String,
        #[serde(default)]
        start_angle: f64,
        #[serde(default = "default_360")]
        end_angle: f64,
        #[serde(default = "default_border_color")]
        color: String,
    },
    Ellipse {
        x: String,
        y: String,
        radius_x: String,
        radius_y: String,
        #[serde(default)]
        filled: bool,
        #[serde(default = "default_fill_color")]
        fill_color: String,
        #[serde(default = "default_border_color")]
        border_color: String,
    },
    Polyline {
        vertices: Vec<[String; 2]>,
        #[serde(default = "default_border_color")]
        color: String,
    },
    Polygon {
        vertices: Vec<[String; 2]>,
        #[serde(default)]
        filled: bool,
        #[serde(default = "default_fill_color")]
        fill_color: String,
        #[serde(default = "default_border_color")]
        border_color: String,
    },
    Text {
        x: String,
        y: String,
        text: String,
        #[serde(default = "default_horizontal")]
        orientation: String,
        #[serde(default = "default_center")]
        justification: String,
        #[serde(default = "default_border_color")]
        color: String,
    },
}

// ═══════════════════════════════════════════════════════════════════════════
// PCBLIB IMPORT
// ═══════════════════════════════════════════════════════════════════════════

/// A complete PCB footprint library definition.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PcbLibImport {
    pub footprints: Vec<PcbLibFootprintDef>,
}

/// A single PCB footprint definition.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PcbLibFootprintDef {
    /// Footprint name (e.g., "SOIC-8", "C_0402")
    pub name: String,

    /// Footprint description
    #[serde(default)]
    pub description: String,

    /// Package type — selects the auto-generation strategy
    pub package: PackageType,

    // ── chip fields ──
    /// Standard chip size code (for package: chip)
    #[serde(default)]
    pub chip_size: Option<String>,

    /// IPC-7351B density level
    #[serde(default)]
    pub density: Option<DensityLevel>,

    // ── dual-row / quad / no-lead shared fields ──
    /// SMD or through-hole
    #[serde(default)]
    pub technology: Option<Technology>,

    /// Number of pads per side
    #[serde(default)]
    pub pads_per_side: Option<usize>,

    /// Center-to-center pad pitch (e.g., "1.27mm", "100mil")
    #[serde(default)]
    pub pitch: Option<String>,

    /// Distance between opposite row centers
    #[serde(default)]
    pub row_spacing: Option<String>,

    /// Pad width perpendicular to row (mm string)
    #[serde(default)]
    pub pad_width: Option<String>,

    /// Pad height along row (mm string)
    #[serde(default)]
    pub pad_height: Option<String>,

    /// Pad shape
    #[serde(default)]
    pub pad_shape: Option<String>,

    // ── through-hole fields ──
    /// Hole diameter (for TH pads)
    #[serde(default)]
    pub hole_diameter: Option<String>,

    /// Through-hole pad diameter
    #[serde(default)]
    pub pad_diameter: Option<String>,

    // ── quad fields ──
    /// Span between opposite row centers (for quad packages)
    #[serde(default)]
    pub span: Option<String>,

    // ── BGA fields ──
    /// Number of rows (for BGA)
    #[serde(default)]
    pub rows: Option<usize>,

    /// Number of columns (for BGA)
    #[serde(default)]
    pub cols: Option<usize>,

    /// Thermal exclusion zone radius (for BGA center)
    #[serde(default)]
    pub skip_center: Option<String>,

    // ── no-lead fields ──
    /// Exposed/thermal pad definition
    #[serde(default)]
    pub exposed_pad: Option<ExposedPadDef>,

    // ── SOT fields ──
    /// SOT variant name
    #[serde(default)]
    pub variant: Option<String>,

    // ── single-row fields ──
    /// Total pad count (for single-row)
    #[serde(default)]
    pub pad_count: Option<usize>,

    /// Row direction
    #[serde(default)]
    pub direction: Option<String>,

    // ── custom fields ──
    /// Manual pad definitions (for package: custom)
    #[serde(default)]
    pub pads: Vec<CustomPadDef>,

    /// Silkscreen elements
    #[serde(default)]
    pub silkscreen: Vec<SilkscreenElement>,
}

/// Package generation strategy.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum PackageType {
    /// IPC chip passive (0201-2512)
    Chip,
    /// Two rows of pads (SOIC, SOP, SSOP, DIP, etc.)
    DualRow,
    /// Four rows of pads (QFP, LQFP, TQFP, PLCC)
    Quad,
    /// No-lead package with optional exposed pad (QFN, DFN, SON)
    NoLead,
    /// Ball/land grid array (BGA, LGA)
    Bga,
    /// Small outline transistor (SOT-23, SOT-223, etc.)
    Sot,
    /// Single row of pads (SIP, pin headers)
    SingleRow,
    /// Fully manual pad placement
    Custom,
}

/// IPC-7351B density level.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DensityLevel {
    /// Level A — most material, largest pads
    Most,
    /// Level B — standard
    Nominal,
    /// Level C — least material, smallest pads
    Least,
}

/// SMD vs through-hole technology.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Technology {
    #[default]
    Smd,
    ThroughHole,
}

/// Exposed/thermal pad for no-lead packages.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExposedPadDef {
    pub width: String,
    pub height: String,
    #[serde(default = "default_ep_designator")]
    pub designator: String,
    /// Thermal via array inside the exposed pad
    #[serde(default)]
    pub thermal_vias: Option<ThermalViaDef>,
}

/// Thermal via array definition for exposed pads.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThermalViaDef {
    /// Number of via rows
    pub rows: usize,
    /// Number of via columns
    pub cols: usize,
    /// Via pitch
    pub pitch: String,
    /// Via hole diameter
    pub hole_diameter: String,
    /// Via pad diameter
    pub pad_diameter: String,
}

/// Manual pad definition for custom footprints.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CustomPadDef {
    pub designator: String,
    pub x: String,
    pub y: String,
    pub width: String,
    pub height: String,
    #[serde(default = "default_rectangular")]
    pub shape: String,
    #[serde(default)]
    pub rotation: f64,
    /// Hole diameter (0 or absent = SMD)
    #[serde(default)]
    pub hole: Option<String>,
}

/// Silkscreen element for footprints.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SilkscreenElement {
    Rectangle {
        x: String,
        y: String,
        width: String,
        height: String,
    },
    Line {
        x1: String,
        y1: String,
        x2: String,
        y2: String,
        #[serde(default = "default_silk_width")]
        width: String,
    },
    Arc {
        x: String,
        y: String,
        radius: String,
        start_angle: f64,
        end_angle: f64,
        #[serde(default = "default_silk_width")]
        width: String,
    },
    Text {
        x: String,
        y: String,
        text: String,
        #[serde(default = "default_text_height")]
        height: String,
        #[serde(default)]
        rotation: f64,
        #[serde(default = "default_silk_width")]
        stroke_width: String,
    },
}

// ═══════════════════════════════════════════════════════════════════════════
// SCHDOC IMPORT
// ═══════════════════════════════════════════════════════════════════════════

/// A complete schematic document definition.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SchDocImport {
    /// Sheet properties
    #[serde(default)]
    pub sheet: SheetDef,

    /// Component instances (placed semantically, no coordinates)
    #[serde(default)]
    pub components: Vec<SchDocComponentDef>,

    /// Net connectivity definitions
    #[serde(default)]
    pub nets: Vec<NetDef>,

    /// Sheet ports for hierarchical designs
    #[serde(default)]
    pub ports: Vec<PortDef>,

    /// No-ERC markers to suppress false warnings
    #[serde(default)]
    pub no_erc: Vec<String>,

    /// Sheet-level parameters
    #[serde(default)]
    pub parameters: Vec<ParameterDef>,
}

/// Sheet properties.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SheetDef {
    /// Sheet title
    #[serde(default)]
    pub title: Option<String>,

    /// Sheet size
    #[serde(default = "default_a4")]
    pub size: String,

    /// Sheet-level parameters
    #[serde(default)]
    pub parameters: Vec<ParameterDef>,
}

/// A component instance on the schematic.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SchDocComponentDef {
    /// Designator (e.g., "U1", "R1", "C1")
    pub designator: String,

    /// Library reference name (must match a SchLib component)
    pub lib_reference: String,

    /// Component value / comment
    #[serde(default)]
    pub value: String,

    /// Placement region hint
    #[serde(default)]
    pub region: Option<PlacementRegion>,

    /// Group tag — components with the same group cluster together
    #[serde(default)]
    pub group: Option<String>,
}

/// Placement region hint for auto-layout.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum PlacementRegion {
    Center,
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// A net definition with connectivity.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NetDef {
    /// Net name
    pub name: String,

    /// If set, creates a power port symbol with this style
    #[serde(default)]
    pub power: Option<String>,

    /// Power port orientation (up/down/left/right)
    #[serde(default)]
    pub orientation: Option<String>,

    /// Pin connections (e.g., "U1:VCC" by name, "U1.8" by designator)
    #[serde(default)]
    pub connections: Vec<String>,
}

/// A sheet port for hierarchical designs.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PortDef {
    /// Port name
    pub name: String,

    /// I/O direction
    #[serde(default = "default_bidirectional")]
    pub r#type: String,

    /// Net this port connects to
    pub net: String,
}

/// A key-value parameter.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ParameterDef {
    pub name: String,
    pub value: String,
}

// ═══════════════════════════════════════════════════════════════════════════
// DEFAULT VALUE FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

fn default_one() -> i32 {
    1
}
fn default_left() -> PinSide {
    PinSide::Left
}
fn default_border_color() -> String {
    "000080".to_string()
}
fn default_fill_color() -> String {
    "FFFFB0".to_string()
}
fn default_360() -> f64 {
    360.0
}
fn default_horizontal() -> String {
    "horizontal".to_string()
}
fn default_center() -> String {
    "center".to_string()
}
fn default_ep_designator() -> String {
    "EP".to_string()
}
fn default_rectangular() -> String {
    "rectangular".to_string()
}
fn default_silk_width() -> String {
    "0.15mm".to_string()
}
fn default_text_height() -> String {
    "1.0mm".to_string()
}
fn default_a4() -> String {
    "A4".to_string()
}
fn default_bidirectional() -> String {
    "bidirectional".to_string()
}
