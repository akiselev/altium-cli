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
//! - [`v2`] - V2 implementation with correct format handling
//!   - [`v2::io`] - File I/O (SchLibV2, SchDocV2)
//!   - [`v2::fields`] - Typed record structs (PinData, ComponentData, etc.)
//!   - [`v2::pcb`] - PCB types and I/O (PcbLib, PcbDoc)
//! - [`types`] - Core data types (coordinates, units, layers, parameters)
//! - [`ops`] - High-level operations for CLI and programmatic use
//! - [`io`] - Low-level I/O utilities (prjpcb, reader, writer)
//! - [`error`] - Error types
//!
//! # Example
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

pub mod dump;
pub mod error;
pub mod format;
pub mod io;
pub mod ops;
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
pub use tree::{BreadthFirstWalker, ParentRef, RecordId, RecordTree, TreeRecord, TreeWalker};
pub use types::{
    Color, Coord, CoordPoint, CoordPoint3D, CoordRect, Layer, ParameterCollection, ParameterValue,
    Unit, UnknownFields,
};

// Re-export derive macros
pub use altium_format_derive::{AltiumBase, AltiumEnum, AltiumRecord};

// Re-export value conversion traits
pub use traits::{FromParamList, FromParamValue, ToParamList, ToParamValue};
