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
//! # Architecture
//!
//! The library is organized into several modules:
//!
//! - [`types`] - Core data types (coordinates, units, layers, parameters)
//! - [`traits`] - Serialization traits for derive macros
//! - [`records`] - Record types for schematic and PCB primitives
//! - [`io`] - File I/O utilities for CFB format
//! - [`error`] - Error types
//!
//! # Derive Macros
//!
//! Record types can be defined using derive macros for automatic serialization:
//!
//! ```ignore
//! use altium_derive::AltiumRecord;
//!
//! #[derive(AltiumRecord)]
//! #[altium(record_id = 2, format = "params")]
//! pub struct SchPin {
//!     #[altium(flatten)]
//!     pub base: SchGraphicalBase,
//!
//!     #[altium(param = "ELECTRICAL", default)]
//!     pub electrical: PinElectricalType,
//!
//!     #[altium(unknown)]
//!     pub unknown_params: UnknownFields,
//! }
//! ```
//!
//! # Quick Start
//!
//! ## Reading a Schematic Library
//!
//! ```no_run
//! use altium_format::io::SchLib;
//! use std::fs::File;
//! use std::io::BufReader;
//!
//! let file = File::open("components.SchLib")?;
//! let lib = SchLib::open(BufReader::new(file))?;
//!
//! for component in &lib.components {
//!     println!("Component: {}", component.name());
//!     println!("  Pins: {}", component.pin_count());
//! }
//! # Ok::<(), altium_format::error::AltiumError>(())
//! ```
//!
//! ## Creating a Footprint
//!
//! ```no_run
//! use altium_format::footprint::FootprintBuilder;
//! use altium_format::records::pcb::PcbPadShape;
//!
//! let mut builder = FootprintBuilder::new("SOIC-8");
//! builder.add_dual_row_smd(
//!     4,      // pads per side
//!     1.27,   // pitch (mm)
//!     5.3,    // row spacing (mm)
//!     1.5,    // pad width (mm)
//!     0.6,    // pad height (mm)
//!     PcbPadShape::Rectangular,
//! );
//! let component = builder.build_deterministic(&mut ());
//! ```
//!
//! # Example
//!
//! ```ignore
//! use altium_format::types::{Coord, CoordPoint, ParameterCollection};
//!
//! // Parse parameters from Altium format
//! let params = ParameterCollection::from_string("|RECORD=1|NAME=Test|X=100mil|");
//! let name = params.get("NAME").unwrap().as_str();
//! let x = params.get("X").unwrap().as_coord_or(Coord::ZERO);
//! ```

pub mod api;
pub mod cli;
pub mod dump;
pub mod edit;
pub mod error;
pub mod footprint;
pub mod format;
pub mod io;
pub mod ops;
pub mod query;
pub mod records;
pub mod traits;
pub mod tree;
pub mod types;

pub use error::{AltiumError, Result};
pub use query::{
    Pattern, QueryMatch, Selector, SelectorEngine, SelectorParser, query_records,
    query_records_with_doc_name,
};
pub use traits::{
    AltiumRecord, FromBinary, FromParamValue, FromParams, PcbPrimitive, SchPrimitive, ToBinary,
    ToParamValue, ToParams,
};
pub use tree::{BreadthFirstWalker, ParentRef, RecordId, RecordTree, TreeRecord, TreeWalker};
pub use types::{
    Color, Coord, CoordPoint, CoordPoint3D, CoordRect, Layer, ParameterCollection, ParameterValue,
    Unit, UnknownFields,
};

// Re-export derive macros
pub use altium_derive::{AltiumBase, AltiumEnum, AltiumRecord};
