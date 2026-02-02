//! Serializer infrastructure matching C# `ISchDataSerializer`.
//!
//! Two implementations:
//! - [`AsciiSerializer`] — Mode 0: `|KEY=VALUE|` parameter strings
//! - [`BinarySerializer`] — Mode 1: sequential typed binary fields

pub mod ascii;
pub mod binary;
pub mod format_v5;

use crate::v2::types::*;
use crate::error::Result;

/// Serializer trait matching C# `ISchDataSerializer` methods.
///
/// Each method has an export (write) and import (read) variant.
/// The serializer implementations handle the encoding differences
/// between ASCII (pipe-delimited params) and binary (sequential fields).
pub trait SchSerializer {
    // --- Stream management ---
    fn start_stream(&mut self, section: &str, name: &str) -> Result<()>;
    fn end_stream(&mut self) -> Result<()>;
    fn stream_exists(&self, section: &str, name: &str) -> bool;
    fn flush(&mut self) -> Result<()>;

    // --- Position/size ---
    fn position(&self) -> i32;
    fn size(&self) -> i32;
    fn seek(&mut self, position: i32) -> Result<()>;
    fn has_value(&self, name: &str) -> bool;

    // --- Instruction (record type marker) ---
    fn export_instruction(&mut self, value: u8, name: &str) -> Result<()>;
    fn import_instruction(&mut self, name: &str) -> Result<u8>;
    fn export_instruction_ex(&mut self, value: i32, name: &str) -> Result<()>;
    fn import_instruction_ex(&mut self, name: &str) -> Result<i32>;

    // --- Integer types ---
    fn export_byte(&mut self, value: u8, name: &str) -> Result<()>;
    fn import_byte(&mut self, name: &str) -> Result<u8>;
    fn export_short_int(&mut self, value: i32, name: &str) -> Result<()>;
    fn import_short_int(&mut self, name: &str) -> Result<i32>;
    fn export_long_int(&mut self, value: i32, name: &str) -> Result<()>;
    fn import_long_int(&mut self, name: &str) -> Result<i32>;
    fn export_long(&mut self, value: i64, name: &str) -> Result<()>;
    fn import_long(&mut self, name: &str) -> Result<i64>;

    // --- Coord ---
    fn export_coord(&mut self, value: i32, name: &str) -> Result<()>;
    fn import_coord(&mut self, name: &str) -> Result<i32>;

    // --- Boolean ---
    fn export_boolean(&mut self, value: bool, name: &str) -> Result<()>;
    fn import_boolean(&mut self, name: &str) -> Result<bool>;

    // --- Color ---
    fn export_color(&mut self, value: u32, name: &str) -> Result<()>;
    fn import_color(&mut self, name: &str) -> Result<u32>;

    // --- String types ---
    fn export_string(&mut self, value: &str, name: &str) -> Result<()>;
    fn import_string(&mut self, name: &str) -> Result<String>;
    fn export_dynamic_string(&mut self, value: &str, name: &str) -> Result<()>;
    fn import_dynamic_string(&mut self, name: &str) -> Result<String>;
    fn export_text(&mut self, value: &str, name: &str) -> Result<()>;
    fn import_text(&mut self, name: &str) -> Result<String>;

    // --- Double/float ---
    fn export_double(&mut self, value: f64, name: &str) -> Result<()>;
    fn import_double(&mut self, name: &str) -> Result<f64>;
    fn export_angle(&mut self, value: f64, name: &str) -> Result<()>;
    fn import_angle(&mut self, name: &str) -> Result<f64>;

    // --- Font ---
    fn export_font_id(&mut self, value: i32, name: &str) -> Result<()>;
    fn import_font_id(&mut self, name: &str) -> Result<i32>;

    // --- Display mode ---
    fn export_display_mode(&mut self, value: u8, name: &str) -> Result<()>;
    fn import_display_mode(&mut self, name: &str) -> Result<u8>;

    // --- Enum types (serialized as integers) ---
    fn export_rotation_by90(&mut self, value: RotationBy90, name: &str) -> Result<()>;
    fn import_rotation_by90(&mut self, name: &str) -> Result<RotationBy90>;
    fn export_pin_electrical(&mut self, value: PinElectrical, name: &str) -> Result<()>;
    fn import_pin_electrical(&mut self, name: &str) -> Result<PinElectrical>;
    fn export_ieee_symbol(&mut self, value: IeeeSymbol, name: &str) -> Result<()>;
    fn import_ieee_symbol(&mut self, name: &str) -> Result<IeeeSymbol>;
    fn export_line_style(&mut self, value: LineStyle, name: &str) -> Result<()>;
    fn import_line_style(&mut self, name: &str) -> Result<LineStyle>;
    fn export_port_arrow_style(&mut self, value: PortArrowStyle, name: &str) -> Result<()>;
    fn import_port_arrow_style(&mut self, name: &str) -> Result<PortArrowStyle>;
    fn export_port_io(&mut self, value: PortIO, name: &str) -> Result<()>;
    fn import_port_io(&mut self, name: &str) -> Result<PortIO>;
    fn export_power_object_style(&mut self, value: PowerObjectStyle, name: &str) -> Result<()>;
    fn import_power_object_style(&mut self, name: &str) -> Result<PowerObjectStyle>;
    fn export_text_justification(&mut self, value: TextJustification, name: &str) -> Result<()>;
    fn import_text_justification(&mut self, name: &str) -> Result<TextJustification>;
    fn export_size(&mut self, value: Size, name: &str) -> Result<()>;
    fn import_size(&mut self, name: &str) -> Result<Size>;
    fn export_no_erc_symbol(&mut self, value: NoERCSymbol, name: &str) -> Result<()>;
    fn import_no_erc_symbol(&mut self, name: &str) -> Result<NoERCSymbol>;
    fn export_parameter_type(&mut self, value: ParameterType, name: &str) -> Result<()>;
    fn import_parameter_type(&mut self, name: &str) -> Result<ParameterType>;
    fn export_left_right_side(&mut self, value: LeftRightSide, name: &str) -> Result<()>;
    fn import_left_right_side(&mut self, name: &str) -> Result<LeftRightSide>;
    fn export_line_shape(&mut self, value: LineShape, name: &str) -> Result<()>;
    fn import_line_shape(&mut self, name: &str) -> Result<LineShape>;
    fn export_horizontal_align(&mut self, value: HorizontalAlign, name: &str) -> Result<()>;
    fn import_horizontal_align(&mut self, name: &str) -> Result<HorizontalAlign>;
    fn export_text_horizontal_anchor(&mut self, value: TextHorzAnchor, name: &str) -> Result<()>;
    fn import_text_horizontal_anchor(&mut self, name: &str) -> Result<TextHorzAnchor>;
    fn export_text_vertical_anchor(&mut self, value: TextVertAnchor, name: &str) -> Result<()>;
    fn import_text_vertical_anchor(&mut self, name: &str) -> Result<TextVertAnchor>;
    fn export_parameter_kind(&mut self, value: ParameterType, name: &str) -> Result<()>;
    fn import_parameter_kind(&mut self, name: &str) -> Result<ParameterType>;
    fn export_parameter_read_only_state(&mut self, value: ParameterReadOnlyState, name: &str) -> Result<()>;
    fn import_parameter_read_only_state(&mut self, name: &str) -> Result<ParameterReadOnlyState>;
    fn export_parameter_set_style(&mut self, value: ParameterSetStyle, name: &str) -> Result<()>;
    fn import_parameter_set_style(&mut self, name: &str) -> Result<ParameterSetStyle>;

    // --- Boolean with default ---
    fn export_boolean_with_default(&mut self, value: bool, name: &str) -> Result<()>;
    fn import_boolean_with_default(&mut self, name: &str, default: bool) -> Result<bool>;

    // --- Binary data ---
    fn export_binary(&mut self, value: &[u8], name: &str) -> Result<()>;
    fn import_binary(&mut self, name: &str) -> Result<Vec<u8>>;

    // --- ASCII-only variants (no-op in binary mode) ---
    fn export_ascii_only_string(&mut self, value: &str, name: &str) -> Result<()>;
    fn import_ascii_only_string(&mut self, name: &str) -> Result<String>;
    fn export_ascii_only_boolean(&mut self, value: bool, name: &str) -> Result<()>;
    fn import_ascii_only_boolean(&mut self, name: &str) -> Result<bool>;
    fn export_ascii_only_coord(&mut self, value: i32, name: &str) -> Result<()>;
    fn import_ascii_only_coord(&mut self, name: &str) -> Result<i32>;
    fn export_ascii_only_color(&mut self, value: u32, name: &str) -> Result<()>;
    fn import_ascii_only_color(&mut self, name: &str) -> Result<u32>;
    fn export_ascii_only_byte(&mut self, value: u8, name: &str) -> Result<()>;
    fn import_ascii_only_byte(&mut self, name: &str) -> Result<u8>;
    fn export_ascii_only_long_int(&mut self, value: i32, name: &str) -> Result<()>;
    fn import_ascii_only_long_int(&mut self, name: &str) -> Result<i32>;
    fn export_ascii_only_font_id(&mut self, value: i32, name: &str) -> Result<()>;
    fn import_ascii_only_font_id(&mut self, name: &str) -> Result<i32>;
    fn export_ascii_only_double(&mut self, value: f64, name: &str) -> Result<()>;
    fn import_ascii_only_double(&mut self, name: &str) -> Result<f64>;
}
