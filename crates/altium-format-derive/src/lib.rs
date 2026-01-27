// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Derive macros for Altium record types.
//!
//! This crate provides procedural macros for automatically generating
//! serialization/deserialization code for Altium file format records.
//!
//! # Macros
//!
//! - `AltiumRecord` - Derive for record types (generates FromParams/ToParams or FromBinary/ToBinary)
//! - `AltiumBase` - Derive for base types (generates HasXxxBase traits)
//! - `AltiumEnum` - Derive for enum value mapping

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod attrs;
mod base;
mod enum_derive;
mod record;

/// Derive macro for Altium record types.
///
/// # Container Attributes
///
/// - `#[altium(record_id = N)]` - Schematic record type ID
/// - `#[altium(object_id = Variant)]` - PCB object ID enum variant
/// - `#[altium(format = "params"|"binary"|"both")]` - Serialization format
///
/// # Field Attributes
///
/// - `#[altium(flatten)]` - Flatten a base type's fields
/// - `#[altium(param = "KEY")]` - Map field to parameter key
/// - `#[altium(param = "KEY", frac = "KEY_FRAC")]` - Integer with fractional part
/// - `#[altium(param = "KEY", default)]` - Use Default::default() if missing
/// - `#[altium(param = "KEY", default = value)]` - Use specific default value
/// - `#[altium(param = "KEY", optional)]` - Wrap in Option<T>
/// - `#[altium(binary, ty = "i32le")]` - Binary field type
/// - `#[altium(binary, coord_point)]` - Binary coordinate point
/// - `#[altium(unknown)]` - Store unknown parameters (non-destructive editing)
/// - `#[altium(unknown_binary)]` - Store unknown binary bytes
/// - `#[altium(skip)]` - Skip field entirely
///
/// # Example
///
/// ```ignore
/// #[derive(AltiumRecord)]
/// #[altium(record_id = 2, format = "params")]
/// pub struct SchPin {
///     #[altium(flatten)]
///     pub base: SchGraphicalBase,
///
///     #[altium(param = "ELECTRICAL", default)]
///     pub electrical: PinElectricalType,
///
///     #[altium(param = "PINLENGTH", frac = "PINLENGTH_FRAC")]
///     pub pin_length: Coord,
///
///     #[altium(unknown)]
///     pub unknown_params: UnknownFields,
/// }
/// ```
#[proc_macro_derive(AltiumRecord, attributes(altium))]
pub fn derive_altium_record(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    record::derive_record(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Derive macro for Altium base types.
///
/// Generates `HasXxxBase` traits for composition-based inheritance.
///
/// # Attributes
///
/// - `#[altium(base_name = "Name")]` - Name for generated trait (default: struct name)
/// - `#[altium(extends = "ParentBase")]` - Parent base type for trait inheritance
///
/// # Example
///
/// ```ignore
/// #[derive(AltiumBase)]
/// #[altium(base_name = "SchPrimitiveBase")]
/// pub struct SchPrimitiveBase {
///     #[altium(param = "OWNERINDEX", default)]
///     pub owner_index: i32,
/// }
///
/// #[derive(AltiumBase)]
/// #[altium(base_name = "SchGraphicalBase", extends = "SchPrimitiveBase")]
/// pub struct SchGraphicalBase {
///     #[altium(flatten)]
///     pub base: SchPrimitiveBase,
///
///     #[altium(param = "LOCATION.X", frac = "LOCATION.X_FRAC")]
///     pub location_x: i32,
/// }
/// ```
#[proc_macro_derive(AltiumBase, attributes(altium))]
pub fn derive_altium_base(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    base::derive_base(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Derive macro for Altium enums with integer mapping.
///
/// # Attributes
///
/// - `#[altium(repr = "i32"|"u8"|...)]` - Integer representation type
/// - `#[altium(value = N)]` - Map variant to specific integer value
/// - `#[altium(default)]` - Mark variant as default for unknown values
///
/// # Example
///
/// ```ignore
/// #[derive(AltiumEnum)]
/// #[altium(repr = "i32")]
/// pub enum PinElectricalType {
///     #[altium(value = 0)]
///     Input,
///     #[altium(value = 1)]
///     InputOutput,
///     #[altium(value = 2)]
///     Output,
///     #[altium(default)]
///     Passive = 4,
/// }
/// ```
#[proc_macro_derive(AltiumEnum, attributes(altium))]
pub fn derive_altium_enum(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    enum_derive::derive_enum(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
