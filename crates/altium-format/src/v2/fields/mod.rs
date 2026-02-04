//! Per-record data structs and format functions.
//!
//! Each record type gets a struct and export/import function pair,
//! ported from `FileFormatV5.cs`.

pub mod block;
pub mod component;
pub mod harness;
pub mod implementation;
pub mod misc;
pub mod parameter;
pub mod pin;
pub mod primitives;
pub mod schematic;
pub mod sheet;

// Re-export everything for convenience
pub use block::*;
pub use component::ComponentData;
pub use harness::*;
pub use implementation::*;
pub use misc::*;
pub use parameter::*;
pub use pin::PinData;
pub use primitives::*;
pub use schematic::*;
pub use sheet::*;

use crate::v2::types::*;

// ============================================================================
// TypedRecord enum — parsed representations of schematic records
// ============================================================================

/// Typed record enum for schematic library/document records.
///
/// This provides strongly-typed access to record data that was parsed
/// from the raw ASCII/binary parameter strings. Each variant corresponds
/// to a specific record type ID in the Altium file format.
#[derive(Clone, Debug)]
pub enum TypedRecord {
    /// Component record (RECORD=1)
    Component(ComponentData),
    /// Pin record (RECORD=2)
    Pin(PinData),
    /// Symbol record (RECORD=3)
    Symbol(SymbolData),
    /// Label record (RECORD=4)
    Label(LabelData),
    /// Bezier curve record (RECORD=5)
    Bezier(BezierData),
    /// Polyline record (RECORD=6)
    Polyline(PolylineData),
    /// Polygon record (RECORD=7)
    Polygon(PolygonData),
    /// Ellipse record (RECORD=8)
    Ellipse(EllipseData),
    /// Pie record (RECORD=9)
    Pie(PieData),
    /// Elliptical arc record (RECORD=11)
    EllipticalArc(EllipticalArcData),
    /// Arc record (RECORD=12)
    Arc(ArcData),
    /// Line record (RECORD=13)
    Line(LineData),
    /// Rectangle record (RECORD=14)
    Rectangle(RectangleData),
    /// Power object record (RECORD=17)
    PowerObject(PowerData),
    /// Port record (RECORD=18)
    Port(PortData),
    /// NoERC marker record (RECORD=22)
    NoERC(NoERCData),
    /// Net label record (RECORD=25)
    NetLabel(NetLabelData),
    /// Bus record (RECORD=26)
    Bus(BusData),
    /// Wire record (RECORD=27)
    Wire(WireData),
    /// Text frame record (RECORD=28)
    TextFrame(TextFrameData),
    /// Junction record (RECORD=29)
    Junction(JunctionData),
    /// Image record (RECORD=30)
    Image(ImageData),
    /// Sheet header record (RECORD=31)
    Sheet(SheetData),
    /// Designator record (RECORD=34)
    Designator(DesignatorData),
    /// Bus entry record (RECORD=37)
    BusEntry(BusEntryData),
    /// Sheet symbol record (RECORD=39)
    SheetSymbol(SheetSymbolData),
    /// Sheet entry record (RECORD=40)
    SheetEntry(SheetEntryData),
    /// Parameter record (RECORD=41)
    Parameter(ParameterData),
    /// Implementation list record (RECORD=44)
    ImplementationList(ImplementationListData),
    /// Implementation record (RECORD=45)
    Implementation(ImplementationData),
    /// Round rectangle record (RECORD=10)
    RoundRectangle(RoundRectangleData),
    /// Note record (RECORD=209)
    Note(NoteData),
    /// Blanket record (RECORD=215)
    Blanket(BlanketData),
    /// Sheet name record (RECORD=32)
    SheetName(SheetNameData),
    /// Sheet file name record (RECORD=33)
    SheetFileName(SheetFileNameData),
    /// Unknown/unsupported record type — stores raw record ID
    Unknown(u8),
}

// ============================================================================
// Base object structs (shared across multiple record types)
// ============================================================================

/// Base data object fields — from `ExportDataObject`/`ImportDataObject`.
#[derive(Clone, Debug, Default)]
pub struct DataObjectBase {
    pub owner_index: i32,
    pub is_not_accessible: bool,
    pub owner_index_additional_list: bool,
    pub index_in_sheet: i32,
    pub ignore_on_load: bool,
    pub is_schematic_block_object: bool,
    pub unique_id_in_reuse_block: String,
}

/// Graphical object fields — from `ExportGraphicalObject`/`ImportGraphicalObject`.
///
/// Extends DataObjectBase.
#[derive(Clone, Debug, Default)]
pub struct GraphicalObjectBase {
    pub base: DataObjectBase,
    pub owner_part_id: i16,
    pub owner_part_display_mode: u8,
    pub selection_memory: u8,
    pub union_index: i32,
    pub graphically_locked: bool,
}

/// Rectangular entry container base — from `ExportRectangularEntryContainer`.
#[derive(Clone, Debug, Default)]
pub struct RectangularEntryContainerBase {
    pub graphical: GraphicalObjectBase,
    pub location_x: i32,
    pub location_y: i32,
    pub x_size: i32,
    pub y_size: i32,
    pub line_width: Size,
    pub color: u32,
    pub area_color: u32,
}

/// Basic entry object base — from `ExportBasicEntryObject`.
#[derive(Clone, Debug, Default)]
pub struct BasicEntryObjectBase {
    pub graphical: GraphicalObjectBase,
    pub side: LeftRightSide,
    pub distance_from_top: i32,
    pub color: u32,
    pub area_color: u32,
    pub text_color: u32,
    pub text_font_id: i32,
    pub text_style: String,
    pub name: String,
    pub harness_type: String,
    pub unique_id: String,
}
