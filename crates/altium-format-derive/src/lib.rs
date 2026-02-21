// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Procedural macros for Altium record types.
//!
//! This crate provides attribute macros for automatically generating
//! serialization/deserialization code for Altium file format records and enums.
//!
//! # Macros
//!
//! - `#[altium_record]` — attribute macro for record types (generates getters,
//!   setters, builders, and `RecordType` trait impl)
//! - `#[altium_enum]` — attribute macro for integer-backed enums (generates
//!   `AltiumEnum` and `ParamCodec` trait impls)

use proc_macro::TokenStream;

mod altium_enum_attr;
mod altium_record;
mod attrs;

/// Attribute macro for v2 Altium enums with integer mapping.
///
/// Generates `crate::traits::AltiumEnum` and `crate::traits::ParamCodec`
/// implementations.
///
/// # Macro-level Attributes
///
/// - `#[altium_enum]` — default: i32, first variant as fallback
/// - `#[altium_enum(fallback = "Unknown")]` — specific fallback variant
///
/// # Variant Attributes
///
/// - `#[altium(value = N)]` — explicit integer value (overrides discriminant)
///
/// # Example
///
/// ```ignore
/// #[altium_enum]
/// #[derive(Copy, Clone, Debug, PartialEq, Eq)]
/// pub enum PinElectricalType {
///     Input = 0,
///     IO = 1,
///     Output = 2,
/// }
/// ```
#[proc_macro_attribute]
pub fn altium_enum(attr: TokenStream, item: TokenStream) -> TokenStream {
    altium_enum_attr::expand(attr.into(), item.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Attribute macro for v2 Altium record types.
///
/// Replaces the annotated struct with a thin wrapper around `RecordOrigin`
/// and generates getters, setters, update closures, a builder, and a
/// `RecordType` trait implementation.
///
/// # Macro-level Attributes
///
/// - `kind = "sch"` or `kind = "pcb"` (required)
/// - `record_id = N` (required for sch)
/// - `object_id = Variant` (required for pcb)
/// - `codec = "params"` or `codec = "binary"` (required)
/// - `parse_fn = "name"` (optional, for complex binary records)
/// - `serialize_fn = "name"` (optional, for complex binary records)
///
/// # Field-level Attributes
///
/// - `#[altium(key = "KEY")]` — parameter key for param codec
/// - `#[altium(key = "KEY", emit = "sparse"|"with_default"|"never")]` — param emission policy
/// - `#[altium(codec_fn = "name")]` — custom codec escape hatch
/// - `#[altium(header)]` — marks PcbCommonHeader in binary records
/// - `#[altium(trailing)]` — marks adaptive trailing fields
/// - `#[altium(skip)]` — skip field entirely
///
/// # Example
///
/// ```ignore
/// #[altium_record(kind = "sch", record_id = 2, codec = "params")]
/// struct SchPinRecord {
///     #[altium(key = "DESIGNATOR")]
///     designator: Designator,
///
///     #[altium(key = "PINLENGTH")]
///     pin_length: SchCoord,
/// }
/// ```
#[proc_macro_attribute]
pub fn altium_record(attr: TokenStream, item: TokenStream) -> TokenStream {
    altium_record::expand(attr.into(), item.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
