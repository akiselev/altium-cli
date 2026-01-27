//! Multi-layer API for Altium file access.
//!
//! This module provides a layered architecture for working with Altium files:
//!
//! - **Layer 1 (CFB)**: Low-level CFB wrapper with reverse engineering helpers
//! - **Layer 2 (Generic)**: Dynamic record access without type knowledge
//! - **Layer 3 (Typed)**: Strongly-typed access with derive macros
//!
//! # Quick Start
//!
//! ```ignore
//! use altium_format::api::AltiumDocument;
//!
//! // Open any Altium file
//! let mut doc = AltiumDocument::open("library.SchLib")?;
//!
//! // Layer 1: CFB access
//! for stream in doc.cfb().streams()? {
//!     println!("{}: {} bytes", stream.path, stream.size);
//! }
//!
//! // Layer 2: Generic access
//! let container = doc.records("/Resistor/Data")?;
//! for record in container.iter() {
//!     println!("{:?}", record.get("LIBREFERENCE"));
//! }
//!
//! // Layer 3: Typed access
//! let component: TypedAccessor<SchComponent> = doc.record_as("/Resistor/Data", 0)?;
//! println!("Component: {}", component.lib_reference);
//! ```

mod cfb;
mod document;
pub mod generic;
pub mod typed;
pub mod views;

pub use cfb::{AltiumCfb, AltiumFileType, Block, StorageInfo, StreamInfo};
pub use document::AltiumDocument;
pub use generic::{BinaryContainer, BinaryRecord, GenericRecord, ParamsContainer, Value};
pub use typed::{EditTransaction, TypedAccessor};
