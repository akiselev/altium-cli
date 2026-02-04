//! V2 Altium format implementation ported from decompiled C#.
//!
//! This module runs in parallel with the existing (v1) code and fixes
//! several critical bugs found by comparing decompiled C# against v1:
//!
//! - Coordinate system: uses correct 100,000 units/mil (v1 uses 10,000)
//! - Binary pin format: writes OwnerIndex, not record_type=2
//! - No phantom byte in binary pin serialization
//! - Correct FormalType export (actual value, not always 0)
//! - SwapIdPin field name (v1 incorrectly uses SwapIdGroup)
//! - Section key truncation: 30 chars with collision avoidance (v1 uses 31)
//! - Pin extended data streams (PinFrac, PinWideText, PinTextData, etc.)
//!
//! # Architecture
//!
//! - [`coord`] — V2Coord with 100K units/mil
//! - [`types`] — Shared enums (ObjectId, PinElectrical, etc.)
//! - [`consts`] — Parameter name constants from FileFormatConsts.cs
//! - [`serializer`] — SchSerializer trait + ASCII/Binary implementations
//! - [`fields`] — Per-record data structs and format functions
//! - [`io`] — SchLib/SchDoc file I/O
//!
//! # Usage
//!
//! ```rust,ignore
//! use altium_format::v2::io::schlib::SchLibV2;
//! use altium_format::v2::{PinData, ComponentData};
//!
//! let lib = SchLibV2::open(file)?;
//! for comp in &lib.components {
//!     println!("Component: {}", comp.entry.lib_ref);
//!     for pin in comp.pins() {
//!         println!("  Pin: {} ({})", pin.name, pin.designator);
//!     }
//! }
//! ```

pub mod consts;
pub mod coord;
pub mod fields;
pub mod io;
pub mod pcb;
pub mod serializer;
pub mod types;

// Re-export coordinate types
pub use coord::{V2Coord, V2Point};

// Re-export common enums from types
pub use types::{
    ObjectId, PinElectrical, RotationBy90, LineStyle, IeeeSymbol,
    PortArrowStyle, PortIO, PowerObjectStyle, TextJustification,
    SheetStyle, PinItemMode, PinTextRotationAnchor, ComponentKind,
    Size, NoERCSymbol, ParameterType, ParameterReadOnlyState,
    StdLogicState, HorizontalAlign, LineShape, LeftRightSide,
    ParameterSetStyle, TextHorzAnchor, TextVertAnchor,
};

// Re-export field structs (typed record data)
pub use fields::{
    // TypedRecord enum for runtime dispatch
    TypedRecord,
    // Base structs
    DataObjectBase, GraphicalObjectBase, RectangularEntryContainerBase, BasicEntryObjectBase,
    // Primary record types
    PinData, ComponentData, ParameterData,
    // Primitives
    ArcData, LineData, RectangleData, EllipseData, PolygonData, PolylineData,
    BezierData, RoundRectangleData, EllipticalArcData, PieData, ImageData,
    // Schematic-specific
    WireData, BusData, BusEntryData, JunctionData, NetLabelData, PowerData,
    PortData, NoERCData, LabelData, TextFrameData, DesignatorData, SymbolData, NoteData,
    // Sheet/hierarchy
    SheetData, SheetSymbolData, SheetEntryData, SheetNameData, SheetFileNameData,
    // Implementation
    ImplementationData, ImplementationListData, MapDefinerData,
    // Misc
    BlanketData, HyperlinkData,
};
