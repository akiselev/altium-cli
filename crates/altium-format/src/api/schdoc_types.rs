//! Public API types for SchDoc documents.
//!
//! These types provide a clean, domain-typed interface for querying and mutating
//! schematic documents. They abstract away internal format details like
//! `SchRecord`, `OWNERINDEX` linking, and CFB structure.
//!
//! The tree structure is:
//! ```text
//! SchDocSheet
//! ├── fonts, grid settings, template
//! └── objects: Vec<SheetObject>
//!     ├── Component "U1" { children: Vec<ComponentChild> }
//!     ├── Wire { vertices }
//!     ├── NetLabel "VCC"
//!     ├── SheetSymbol "Power" { children: Vec<SheetSymbolChild> }
//!     └── ...
//! ```

use altium_format_types::color::Color;
use altium_format_types::common::{ComponentKind, RotationBy90};
use altium_format_types::coord::{Coord, CoordPoint};
use altium_format_types::sch::{
    HorizontalAlign, LeftRightSide, LineStyle, PenWidth, PortArrowStyle, PortIoType,
    PowerObjectStyle, SheetStyle, SheetSymbolType, TextJustification,
};

use super::schlib_types::{FootprintMap, Graphic, Parameter, Pin};

// ── Top-level document ───────────────────────────────────────────────────────

/// A schematic document with sheet properties and an ordered list of objects.
///
/// Obtained via `SchDoc::sheet()`. The `objects` vec preserves the document
/// ordering which determines serialized record positions.
#[derive(Debug, Clone)]
pub struct SchDocSheet {
    // ── Sheet properties ─────────────────────────────────────
    pub fonts: Vec<Font>,
    pub snap_grid_size: Coord,
    pub visible_grid_size: Coord,
    pub hot_spot_grid_size: Coord,
    pub snap_grid_on: bool,
    pub visible_grid_on: bool,
    pub hot_spot_grid_on: bool,
    pub sheet_style: SheetStyle,
    pub use_custom_sheet: bool,
    pub custom_width: Coord,
    pub custom_height: Coord,
    pub area_color: Color,
    pub border_on: bool,
    pub title_block_on: bool,
    pub show_template_graphics: bool,
    pub template_file_name: String,
    pub display_unit: i32,
    pub workspace_orientation: i32,
    pub show_hidden_pins: bool,

    // ── Template ─────────────────────────────────────────────
    pub template: Template,

    // ── Ordered content ──────────────────────────────────────
    /// All sheet-level objects in document order. This ordering is preserved
    /// across save/load cycles and determines the serialized record positions.
    pub objects: Vec<SheetObject>,
}

/// A font entry in the schematic sheet's font table.
#[derive(Debug, Clone)]
pub struct Font {
    pub id: i32,
    pub name: String,
    pub size: i32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikeout: bool,
    pub rotation: i32,
}

/// The sheet template with its owned graphics.
#[derive(Debug, Clone)]
pub struct Template {
    pub file_name: String,
    /// Template-owned graphics (labels, images from the .SchDot template).
    pub children: Vec<Graphic>,
}

// ── Query methods on SchDocSheet ─────────────────────────────────────────────

impl SchDocSheet {
    /// All components on the sheet.
    pub fn components(&self) -> Vec<&SchDocComponent> {
        self.objects
            .iter()
            .filter_map(|o| match o {
                SheetObject::Component(c) => Some(c),
                _ => None,
            })
            .collect()
    }

    /// Find a component by designator (case-sensitive).
    pub fn component(&self, designator: &str) -> Option<&SchDocComponent> {
        self.components()
            .into_iter()
            .find(|c| c.designator == designator)
    }

    /// All wires on the sheet.
    pub fn wires(&self) -> Vec<&Wire> {
        self.objects
            .iter()
            .filter_map(|o| match o {
                SheetObject::Wire(w) => Some(w),
                _ => None,
            })
            .collect()
    }

    /// All buses on the sheet.
    pub fn buses(&self) -> Vec<&Bus> {
        self.objects
            .iter()
            .filter_map(|o| match o {
                SheetObject::Bus(b) => Some(b),
                _ => None,
            })
            .collect()
    }

    /// All net labels on the sheet.
    pub fn net_labels(&self) -> Vec<&NetLabel> {
        self.objects
            .iter()
            .filter_map(|o| match o {
                SheetObject::NetLabel(n) => Some(n),
                _ => None,
            })
            .collect()
    }

    /// All power objects on the sheet.
    pub fn power_objects(&self) -> Vec<&PowerObject> {
        self.objects
            .iter()
            .filter_map(|o| match o {
                SheetObject::PowerObject(p) => Some(p),
                _ => None,
            })
            .collect()
    }

    /// All ports on the sheet.
    pub fn ports(&self) -> Vec<&Port> {
        self.objects
            .iter()
            .filter_map(|o| match o {
                SheetObject::Port(p) => Some(p),
                _ => None,
            })
            .collect()
    }

    /// All sheet symbols on the sheet.
    pub fn sheet_symbols(&self) -> Vec<&SheetSymbol> {
        self.objects
            .iter()
            .filter_map(|o| match o {
                SheetObject::SheetSymbol(s) => Some(s),
                _ => None,
            })
            .collect()
    }

    /// All junctions on the sheet.
    pub fn junctions(&self) -> Vec<&Junction> {
        self.objects
            .iter()
            .filter_map(|o| match o {
                SheetObject::Junction(j) => Some(j),
                _ => None,
            })
            .collect()
    }

    /// All no-connect markers on the sheet.
    pub fn no_connects(&self) -> Vec<&NoConnect> {
        self.objects
            .iter()
            .filter_map(|o| match o {
                SheetObject::NoConnect(n) => Some(n),
                _ => None,
            })
            .collect()
    }

    /// All bus entries on the sheet.
    pub fn bus_entries(&self) -> Vec<&BusEntry> {
        self.objects
            .iter()
            .filter_map(|o| match o {
                SheetObject::BusEntry(b) => Some(b),
                _ => None,
            })
            .collect()
    }

    /// All parameter sets on the sheet.
    pub fn parameter_sets(&self) -> Vec<&ParameterSet> {
        self.objects
            .iter()
            .filter_map(|o| match o {
                SheetObject::ParameterSet(p) => Some(p),
                _ => None,
            })
            .collect()
    }

    /// All notes on the sheet.
    pub fn notes(&self) -> Vec<&Note> {
        self.objects
            .iter()
            .filter_map(|o| match o {
                SheetObject::Note(n) => Some(n),
                _ => None,
            })
            .collect()
    }

    /// All sheet-level graphics (not owned by a component).
    pub fn graphics(&self) -> Vec<&Graphic> {
        self.objects
            .iter()
            .filter_map(|o| match o {
                SheetObject::Graphic(g) => Some(g),
                _ => None,
            })
            .collect()
    }

    /// All sheet-level parameters.
    pub fn parameters(&self) -> Vec<&Parameter> {
        self.objects
            .iter()
            .filter_map(|o| match o {
                SheetObject::Parameter(p) => Some(p),
                _ => None,
            })
            .collect()
    }

    // ── Mutation methods ──────────────────────────────────────

    /// Add a sheet-level object.
    pub fn add_object(&mut self, obj: SheetObject) {
        self.objects.push(obj);
    }

    /// Remove all objects matching a predicate.
    pub fn remove_objects(&mut self, mut f: impl FnMut(&SheetObject) -> bool) {
        self.objects.retain(|o| !f(o));
    }

    /// Find a placed component by designator (mutable).
    pub fn component_mut(&mut self, designator: &str) -> Option<&mut SchDocComponent> {
        self.objects.iter_mut().find_map(|o| match o {
            SheetObject::Component(c) if c.designator == designator => Some(c),
            _ => None,
        })
    }

    /// Add a child to a placed component identified by designator.
    ///
    /// Returns `true` if the component was found and the child was added,
    /// `false` if no component with that designator exists.
    pub fn add_component_child(&mut self, designator: &str, child: ComponentChild) -> bool {
        if let Some(comp) = self.component_mut(designator) {
            comp.children.push(child);
            true
        } else {
            false
        }
    }
}

// ── SheetObject ──────────────────────────────────────────────────────────────

/// A top-level object on the schematic sheet.
///
/// Variants are ordered to match the SchDoc ownership tree. Each container
/// variant bundles its children inline rather than via separate collections.
/// The `Vec<SheetObject>` preserves document ordering.
#[derive(Debug, Clone)]
pub enum SheetObject {
    // ── Placed components ────────────────────────────────────
    Component(SchDocComponent),

    // ── Connectivity ─────────────────────────────────────────
    Wire(Wire),
    Bus(Bus),
    NetLabel(NetLabel),
    PowerObject(PowerObject),
    Port(Port),
    Junction(Junction),
    NoConnect(NoConnect),
    BusEntry(BusEntry),

    // ── Hierarchical ─────────────────────────────────────────
    SheetSymbol(SheetSymbol),

    // ── Annotations ──────────────────────────────────────────
    ParameterSet(ParameterSet),
    Note(Note),
    Probe(Probe),
    CompileMask(CompileMask),
    Blanket(Blanket),

    // ── Graphics (sheet-level, not owned by a component) ─────
    Graphic(Graphic),

    // ── Sheet-level parameters (CurrentTime, etc.) ───────────
    Parameter(Parameter),

    // ── Harness ──────────────────────────────────────────────
    HarnessConnector(HarnessConnector),
    SignalHarness(SignalHarness),
}

// ── SchDocComponent ──────────────────────────────────────────────────────────

/// A placed component instance on a schematic sheet.
///
/// Identity: `designator` (e.g. "R1", "U3"). Unique within the document.
///
/// Unlike SchLib's `Component` (identified by `lib_reference`), a SchDocComponent
/// represents a *placed instance* with position, orientation, and library back-references.
/// Its children are bundled in an ordered `Vec<ComponentChild>`.
#[derive(Debug, Clone)]
pub struct SchDocComponent {
    // ── Identity ─────────────────────────────────────────────
    pub designator: String,
    pub unique_id: String,

    // ── Library reference ────────────────────────────────────
    pub lib_reference: String,
    pub source_library_name: String,
    pub design_item_id: String,
    pub library_path: String,

    // ── Placement ────────────────────────────────────────────
    pub location: CoordPoint,
    pub orientation: RotationBy90,
    pub is_mirrored: bool,

    // ── Properties ───────────────────────────────────────────
    pub description: Option<String>,
    pub component_kind: ComponentKind,
    pub part_count: i32,
    pub current_part_id: i32,
    pub display_mode_count: i32,
    pub show_hidden_pins: bool,

    // ── Children (ordered) ───────────────────────────────────
    /// All component children in document order: pins, parameters,
    /// graphics, and footprint maps interleaved as they appear in the file.
    pub children: Vec<ComponentChild>,
}

/// A child object of a placed component.
///
/// This enum preserves the ordering of children within a component, which
/// mirrors the depth-first record order in the SchDoc file. The Designator
/// record (RECORD=34) is NOT included here — it is extracted to
/// `SchDocComponent.designator`.
#[derive(Debug, Clone)]
pub enum ComponentChild {
    Pin(Pin),
    Parameter(Parameter),
    Graphic(Graphic),
    FootprintMap(FootprintMap),
}

// ── Query methods on SchDocComponent ─────────────────────────────────────────

impl SchDocComponent {
    /// All pins in this component.
    pub fn pins(&self) -> Vec<&Pin> {
        self.children
            .iter()
            .filter_map(|c| match c {
                ComponentChild::Pin(p) => Some(p),
                _ => None,
            })
            .collect()
    }

    /// Find a pin by designator.
    pub fn pin(&self, designator: &str) -> Option<&Pin> {
        self.pins().into_iter().find(|p| p.designator == designator)
    }

    /// All parameters in this component.
    pub fn parameters(&self) -> Vec<&Parameter> {
        self.children
            .iter()
            .filter_map(|c| match c {
                ComponentChild::Parameter(p) => Some(p),
                _ => None,
            })
            .collect()
    }

    /// Find a parameter by name.
    pub fn parameter(&self, name: &str) -> Option<&Parameter> {
        self.parameters().into_iter().find(|p| p.name == name)
    }

    /// All graphics in this component.
    pub fn graphics(&self) -> Vec<&Graphic> {
        self.children
            .iter()
            .filter_map(|c| match c {
                ComponentChild::Graphic(g) => Some(g),
                _ => None,
            })
            .collect()
    }

    /// All footprint maps in this component.
    pub fn footprints(&self) -> Vec<&FootprintMap> {
        self.children
            .iter()
            .filter_map(|c| match c {
                ComponentChild::FootprintMap(f) => Some(f),
                _ => None,
            })
            .collect()
    }
}

// ── Connectivity types ───────────────────────────────────────────────────────

/// Electrical wire (RECORD=27).
#[derive(Debug, Clone)]
pub struct Wire {
    pub unique_id: String,
    pub vertices: Vec<CoordPoint>,
    pub color: Color,
    pub line_width: PenWidth,
    pub line_style: LineStyle,
}

/// Bus (RECORD=26).
#[derive(Debug, Clone)]
pub struct Bus {
    pub unique_id: String,
    pub vertices: Vec<CoordPoint>,
    pub color: Color,
    pub line_width: PenWidth,
}

/// Net label (RECORD=25).
#[derive(Debug, Clone)]
pub struct NetLabel {
    pub unique_id: String,
    pub text: String,
    pub location: CoordPoint,
    pub orientation: RotationBy90,
    pub justification: TextJustification,
    pub font_id: i32,
    pub color: Color,
    pub is_mirrored: bool,
}

/// Power object (RECORD=17).
#[derive(Debug, Clone)]
pub struct PowerObject {
    pub unique_id: String,
    pub text: String,
    pub location: CoordPoint,
    pub orientation: RotationBy90,
    pub style: PowerObjectStyle,
    pub show_net_name: bool,
    pub font_id: i32,
    pub color: Color,
    pub is_cross_sheet_connector: bool,
}

/// Port (RECORD=18).
#[derive(Debug, Clone)]
pub struct Port {
    pub unique_id: String,
    pub name: String,
    pub location: CoordPoint,
    pub io_type: PortIoType,
    pub style: PortArrowStyle,
    pub width: Coord,
    pub height: Coord,
    pub color: Color,
    pub area_color: Color,
    pub text_color: Color,
    pub font_id: i32,
    pub alignment: HorizontalAlign,
    pub harness_type: String,
    pub border_width: PenWidth,
    pub auto_size: bool,
    pub port_name_is_hidden: bool,
}

/// Junction (RECORD=29).
#[derive(Debug, Clone)]
pub struct Junction {
    pub unique_id: String,
    pub location: CoordPoint,
    pub color: Color,
}

/// No-connect marker (RECORD=22).
#[derive(Debug, Clone)]
pub struct NoConnect {
    pub unique_id: String,
    pub location: CoordPoint,
    pub color: Color,
    pub orientation: RotationBy90,
    pub symbol: String,
    pub is_active: bool,
    pub suppress_all: bool,
}

/// Bus entry (RECORD=37).
#[derive(Debug, Clone)]
pub struct BusEntry {
    pub unique_id: String,
    pub location: CoordPoint,
    pub corner: CoordPoint,
    pub color: Color,
    pub line_width: PenWidth,
}

// ── Hierarchical sheet types ─────────────────────────────────────────────────

/// Hierarchical sheet symbol with ordered children (RECORD=15).
///
/// Children include SheetEntry ports and Parameters. The field objects
/// SheetName (RECORD=32) and SheetFileName (RECORD=33) are extracted to
/// `sheet_name` and `file_name`.
#[derive(Debug, Clone)]
pub struct SheetSymbol {
    pub unique_id: String,
    pub location: CoordPoint,
    pub x_size: Coord,
    pub y_size: Coord,
    pub color: Color,
    pub area_color: Color,
    pub line_width: PenWidth,
    pub is_solid: bool,
    pub symbol_type: SheetSymbolType,

    /// Extracted from the SheetName child record (RECORD=32).
    pub sheet_name: String,
    /// Extracted from the SheetFileName child record (RECORD=33).
    pub file_name: String,

    /// Children (ordered): entries and parameters.
    pub children: Vec<SheetSymbolChild>,
}

/// A child of a SheetSymbol.
#[derive(Debug, Clone)]
pub enum SheetSymbolChild {
    Entry(SheetEntry),
    Parameter(Parameter),
}

/// Sheet entry port within a SheetSymbol (RECORD=16).
#[derive(Debug, Clone)]
pub struct SheetEntry {
    pub unique_id: String,
    pub name: String,
    pub io_type: PortIoType,
    pub side: LeftRightSide,
    pub distance_from_top: Coord,
    pub style: PortArrowStyle,
    pub color: Color,
    pub area_color: Color,
    pub text_color: Color,
    pub text_font_id: i32,
}

impl SheetSymbol {
    /// All entries in this sheet symbol.
    pub fn entries(&self) -> Vec<&SheetEntry> {
        self.children
            .iter()
            .filter_map(|c| match c {
                SheetSymbolChild::Entry(e) => Some(e),
                _ => None,
            })
            .collect()
    }

    /// Find an entry by name.
    pub fn entry(&self, name: &str) -> Option<&SheetEntry> {
        self.entries().into_iter().find(|e| e.name == name)
    }

    /// All parameters in this sheet symbol.
    pub fn parameters(&self) -> Vec<&Parameter> {
        self.children
            .iter()
            .filter_map(|c| match c {
                SheetSymbolChild::Parameter(p) => Some(p),
                _ => None,
            })
            .collect()
    }
}

// ── Annotation types ─────────────────────────────────────────────────────────

/// Parameter set attached to a net (RECORD=43), with ordered child parameters.
#[derive(Debug, Clone)]
pub struct ParameterSet {
    pub unique_id: String,
    pub location: CoordPoint,
    pub color: Color,
    pub orientation: RotationBy90,
    pub name: String,
    pub style: i32,
    pub parameters: Vec<Parameter>,
}

/// Note annotation (RECORD=209).
#[derive(Debug, Clone)]
pub struct Note {
    pub unique_id: String,
    pub location: CoordPoint,
    pub corner: CoordPoint,
    pub text: String,
    pub author: String,
    pub font_id: i32,
    pub color: Color,
    pub area_color: Color,
    pub text_color: Color,
    pub is_solid: bool,
    pub show_border: bool,
    pub alignment: HorizontalAlign,
    pub word_wrap: bool,
    pub clip_to_rect: bool,
    pub text_margin: Coord,
    pub collapsed: bool,
}

/// Probe (RECORD=210).
#[derive(Debug, Clone)]
pub struct Probe {
    pub unique_id: String,
    pub location: CoordPoint,
    pub color: Color,
    pub orientation: RotationBy90,
    pub name: String,
}

/// Compile mask (RECORD=211).
#[derive(Debug, Clone)]
pub struct CompileMask {
    pub unique_id: String,
    pub location: CoordPoint,
    pub corner: CoordPoint,
    pub color: Color,
    pub area_color: Color,
    pub line_width: PenWidth,
    pub collapsed: bool,
}

/// Blanket/dashed rectangle (RECORD=225).
#[derive(Debug, Clone)]
pub struct Blanket {
    pub unique_id: String,
    pub location: CoordPoint,
    pub corner: CoordPoint,
    pub color: Color,
    pub area_color: Color,
    pub line_style: LineStyle,
    pub line_width: PenWidth,
    pub vertices: Vec<CoordPoint>,
    pub collapsed: bool,
}

// ── Harness types ────────────────────────────────────────────────────────────

/// Harness connector (RECORD=215).
#[derive(Debug, Clone)]
pub struct HarnessConnector {
    pub unique_id: String,
    pub location: CoordPoint,
    pub x_size: Coord,
    pub y_size: Coord,
    pub color: Color,
    pub area_color: Color,
    pub line_width: PenWidth,
    pub children: Vec<HarnessChild>,
}

/// A child of a HarnessConnector.
#[derive(Debug, Clone)]
pub enum HarnessChild {
    Entry(SheetEntry),
    ConnectorType(String),
    Parameter(Parameter),
}

/// Signal harness (RECORD=226).
#[derive(Debug, Clone)]
pub struct SignalHarness {
    pub unique_id: String,
    pub vertices: Vec<CoordPoint>,
    pub color: Color,
    pub line_width: PenWidth,
}
