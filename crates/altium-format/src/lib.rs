// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Altium file format library for Rust.
//!
//! This library provides read/write support for Altium Designer files including:
//! - SchLib (Schematic symbol library)
//! - PcbLib (PCB footprint library)
//! - SchDoc (Schematic document)
//! - PcbDoc (PCB document)
//!
//! # V2 API (Recommended)
//!
//! The V2 API in the [`v2`] module is the recommended way to work with Altium files.
//! It provides:
//! - Correct coordinate scales (100K units/mil for schematics, 10K for PCB)
//! - Properly typed field structs matching Altium's internal format
//! - Full roundtrip support for reading and writing files
//!
//! ## Quick Start with V2
//!
//! ```ignore
//! use altium_format::v2::io::schlib::SchLibV2;
//! use altium_format::v2::{PinData, ComponentData};
//!
//! // Open a schematic library
//! let lib = SchLibV2::open_file("components.SchLib")?;
//!
//! // Iterate components with typed access
//! for comp in &lib.components {
//!     println!("Component: {}", comp.entry.lib_ref);
//!     for pin in comp.pins() {
//!         println!("  Pin: {} ({})", pin.name, pin.designator);
//!     }
//! }
//! ```
//!
//! # Architecture
//!
//! The library is organized into several modules:
//!
//! - [`v2`] - **Recommended**: V2 implementation with correct format handling
//!   - [`v2::io`] - File I/O (SchLibV2, SchDocV2)
//!   - [`v2::fields`] - Typed record structs (PinData, ComponentData, etc.)
//!   - [`v2::pcb`] - PCB types and I/O (PcbLibV2, PcbDocV2)
//! - [`types`] - Core data types (coordinates, units, layers, parameters)
//! - [`traits`] - Serialization traits for derive macros
//! - [`records`] - **Deprecated**: V1 record types (use v2::fields instead)
//! - [`io`] - **Deprecated**: V1 file I/O (use v2::io instead)
//! - [`error`] - Error types
//!
//! # Migration from V1 to V2
//!
//! | V1 Type | V2 Replacement |
//! |---------|----------------|
//! | `io::SchLib` | `v2::io::schlib::SchLibV2` |
//! | `io::SchDoc` | `v2::io::schdoc::SchDocV2` |
//! | `io::PcbLib` | `v2::pcb::io::pcblib::PcbLibV2` |
//! | `io::PcbDoc` | `v2::pcb::io::pcbdoc::PcbDocV2` |
//! | `records::sch::SchPin` | `v2::PinData` |
//! | `records::sch::SchComponent` | `v2::ComponentData` |
//! | `records::sch::SchRecord` | `v2::TypedRecord` |
//! | `records::pcb::PcbPad` | `v2::pcb::PcbPadV2` |
//!
//! # Example (V2)
//!
//! ```ignore
//! use altium_format::v2::io::schlib::SchLibV2;
//! use altium_format::v2::PinElectrical;
//!
//! let lib = SchLibV2::open_file("library.SchLib")?;
//! for comp in &lib.components {
//!     for pin in comp.pins() {
//!         if pin.electrical == PinElectrical::Power {
//!             println!("Power pin: {}", pin.name);
//!         }
//!     }
//! }
//! ```
//!
//! # Legacy V1 API (Deprecated)
//!
//! The V1 API in [`records`] and [`io`] modules is deprecated due to:
//! - Incorrect coordinate scales
//! - Field type mismatches with Altium's format
//! - Incomplete roundtrip support
//!
//! V1 types remain available for backwards compatibility but will emit deprecation warnings.

pub mod api;
pub mod dump;
pub mod edit;
pub mod error;
pub mod footprint;
pub mod format;
pub mod io;
pub mod ops;
pub mod query;
pub mod records;
pub mod templates;
pub mod traits;
pub mod tree;
pub mod types;
pub mod v2;

// =============================================================================
// V2 Type Re-exports (Recommended)
// =============================================================================

// V2 coordinate types
pub use v2::{V2Coord, V2Point};

// V2 common enums
pub use v2::{
    ComponentKind, HorizontalAlign, IeeeSymbol, LeftRightSide, LineShape, LineStyle, NoERCSymbol,
    ObjectId, ParameterReadOnlyState, ParameterSetStyle, ParameterType, PinElectrical, PinItemMode,
    PinTextRotationAnchor, PortArrowStyle, PortIO, PowerObjectStyle, RotationBy90, SheetStyle,
    Size, StdLogicState, TextHorzAnchor, TextJustification, TextVertAnchor,
};

// V2 field structs (typed record data)
pub use v2::{
    // TypedRecord enum for runtime dispatch
    TypedRecord,
    // Base structs
    BasicEntryObjectBase, DataObjectBase, GraphicalObjectBase, RectangularEntryContainerBase,
    // Primary record types
    ComponentData, ParameterData, PinData,
    // Primitives
    ArcData, BezierData, EllipseData, EllipticalArcData, ImageData, LineData, PieData, PolygonData,
    PolylineData, RectangleData, RoundRectangleData,
    // Schematic-specific
    BusData, BusEntryData, DesignatorData, JunctionData, LabelData, NetLabelData, NoERCData,
    NoteData, PortData, PowerData, SymbolData, TextFrameData, WireData,
    // Sheet/hierarchy
    SheetData, SheetEntryData, SheetFileNameData, SheetNameData, SheetSymbolData,
    // Implementation
    ImplementationData, ImplementationListData, MapDefinerData,
    // Misc
    BlanketData, HyperlinkData,
};

// =============================================================================
// Core Types (shared between V1 and V2)
// =============================================================================

pub use error::{AltiumError, Result};
pub use query::{
    Pattern, QueryMatch, Selector, SelectorEngine, SelectorParser, query_records,
    query_records_with_doc_name,
};
pub use tree::{BreadthFirstWalker, ParentRef, RecordId, RecordTree, TreeRecord, TreeWalker};
pub use types::{
    Color, Coord, CoordPoint, CoordPoint3D, CoordRect, Layer, ParameterCollection, ParameterValue,
    Unit, UnknownFields,
};

// Re-export derive macros
pub use altium_format_derive::{AltiumBase, AltiumEnum, AltiumRecord};

// =============================================================================
// V1 Traits (still used by V1 types, deprecated with V1 API)
// =============================================================================

pub use traits::{
    AltiumRecord, FromBinary, FromParamValue, FromParams, PcbPrimitive, SchPrimitive, ToBinary,
    ToParamValue, ToParams,
};
