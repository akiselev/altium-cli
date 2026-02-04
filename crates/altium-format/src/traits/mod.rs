//! Core traits for Altium record serialization and deserialization.
//!
//! This module defines the fundamental traits used by the derive macros to
//! generate serialization code for Altium record types.
//!
//! # Migration Status
//!
//! The V1 record types that implement these traits are deprecated. For new code:
//! - Use `v2::fields` typed structs (PinData, ComponentData, etc.)
//! - Use `v2::serializer::format_v5` functions for serialization
//! - Use `v2::io` types (SchLibV2, SchDocV2, PcbLibV2, PcbDocV2)
//!
//! These traits remain for:
//! - Backwards compatibility with existing code
//! - Support for modules still being migrated (edit/, query/, footprint/)
//! - Derive macro support during transition period

mod binary;
mod conversion;
mod params;

pub use binary::{FromBinary, ToBinary};
pub use conversion::{FromParamList, FromParamValue, ToParamList, ToParamValue};
pub use params::{FromParams, ToParams};

use crate::types::{CoordPoint, CoordRect, Layer};

/// Marker trait for all Altium record types.
///
/// This trait provides common metadata about record types and is automatically
/// implemented by the `AltiumRecord` derive macro.
pub trait AltiumRecord: Sized + Clone + Default {
    /// Human-readable name for this record type.
    fn record_type_name() -> &'static str;

    /// Whether this record type supports unknown field preservation.
    fn supports_unknown_preservation() -> bool {
        true
    }
}

/// Trait for schematic primitives.
///
/// Implemented by all schematic record types that have a record ID.
pub trait SchPrimitive: AltiumRecord + FromParams + ToParams {
    /// The record type ID (e.g., 1 for Component, 2 for Pin).
    const RECORD_ID: i32;

    /// Get the owner index (parent reference in the hierarchy).
    fn owner_index(&self) -> i32;

    /// Set the owner index.
    fn set_owner_index(&mut self, index: i32);

    /// Calculate the bounding rectangle of this primitive.
    fn calculate_bounds(&self) -> CoordRect;

    /// Get the location of this primitive (if applicable).
    fn location(&self) -> Option<CoordPoint> {
        None
    }

    /// Get the human-readable record type name.
    fn record_type_name(&self) -> &'static str;

    /// Get a property value by name (for dynamic access).
    fn get_property(&self, _name: &str) -> Option<String> {
        None
    }
}

/// Trait for PCB primitives.
///
/// Implemented by all PCB record types that have an object ID.
pub trait PcbPrimitive: AltiumRecord + FromBinary + ToBinary {
    /// The object ID for this primitive type.
    const OBJECT_ID: crate::records::pcb::PcbObjectId;

    /// Get the layer this primitive is on.
    fn layer(&self) -> Layer;

    /// Calculate the bounding rectangle of this primitive.
    fn calculate_bounds(&self) -> CoordRect;
}

/// Trait for primitives that have a location.
pub trait Locatable {
    /// Get the location as (x, y) raw coordinate values.
    fn location(&self) -> (i32, i32);

    /// Set the location.
    fn set_location(&mut self, x: i32, y: i32);
}

/// Trait for graphical primitives with color properties.
pub trait Graphical: Locatable {
    /// Get the primary color.
    fn color(&self) -> i32;

    /// Set the primary color.
    fn set_color(&mut self, color: i32);

    /// Get the area/fill color.
    fn area_color(&self) -> i32;

    /// Set the area/fill color.
    fn set_area_color(&mut self, color: i32);
}

/// Trait for container records that can have children.
pub trait Container: AltiumRecord {
    /// Type of children this container can hold.
    type Child: AltiumRecord;

    /// Whether this is a root container (like SchComponent).
    fn is_root_container() -> bool {
        false
    }
}
