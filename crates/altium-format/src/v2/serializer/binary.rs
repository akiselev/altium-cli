//! Binary serializer — Mode 1: sequential typed binary fields.
//!
//! In binary mode, fields are written sequentially with no names or delimiters.
//! The field order must exactly match the C# export/import functions.

use std::io::{Cursor, Read, Write};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::error::{AltiumError, Result};
use crate::v2::types::*;
use super::SchSerializer;

/// Binary field serializer operating on a byte buffer.
pub struct BinarySerializer {
    cursor: Cursor<Vec<u8>>,
}

impl BinarySerializer {
    pub fn new_writer() -> Self {
        BinarySerializer { cursor: Cursor::new(Vec::new()) }
    }

    pub fn from_bytes(data: Vec<u8>) -> Self {
        BinarySerializer { cursor: Cursor::new(data) }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.cursor.into_inner()
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.cursor.get_ref()
    }

    fn read_pascal_string(&mut self) -> Result<String> {
        let len = self.cursor.read_u8().map_err(AltiumError::Io)?;
        let mut buf = vec![0u8; len as usize];
        self.cursor.read_exact(&mut buf).map_err(AltiumError::Io)?;
        Ok(String::from_utf8_lossy(&buf).into_owned())
    }

    fn write_pascal_string(&mut self, s: &str) -> Result<()> {
        let bytes = s.as_bytes();
        let len = bytes.len().min(255) as u8;
        self.cursor.write_u8(len).map_err(AltiumError::Io)?;
        self.cursor.write_all(&bytes[..len as usize]).map_err(AltiumError::Io)?;
        Ok(())
    }

    fn read_text_string(&mut self) -> Result<String> {
        let len = self.cursor.read_i16::<LittleEndian>().map_err(AltiumError::Io)?;
        let total = (len + 1) as usize;
        let mut buf = vec![0u8; total];
        self.cursor.read_exact(&mut buf).map_err(AltiumError::Io)?;
        Ok(String::from_utf8_lossy(&buf[..len as usize]).into_owned())
    }

    fn write_text_string(&mut self, s: &str) -> Result<()> {
        let bytes = s.as_bytes();
        let len = bytes.len() as i16;
        self.cursor.write_i16::<LittleEndian>(len).map_err(AltiumError::Io)?;
        self.cursor.write_all(bytes).map_err(AltiumError::Io)?;
        self.cursor.write_u8(0).map_err(AltiumError::Io)?;
        Ok(())
    }
}

macro_rules! impl_enum_binary {
    ($export_fn:ident, $import_fn:ident, $enum_ty:ty, $from_fn:expr) => {
        fn $export_fn(&mut self, value: $enum_ty, _name: &str) -> Result<()> {
            self.cursor.write_u8(value as u8).map_err(AltiumError::Io)
        }
        fn $import_fn(&mut self, _name: &str) -> Result<$enum_ty> {
            let v = self.cursor.read_u8().map_err(AltiumError::Io)?;
            Ok($from_fn(v))
        }
    };
}

impl SchSerializer for BinarySerializer {
    fn start_stream(&mut self, _section: &str, _name: &str) -> Result<()> { Ok(()) }
    fn end_stream(&mut self) -> Result<()> { Ok(()) }
    fn stream_exists(&self, _section: &str, _name: &str) -> bool { false }
    fn flush(&mut self) -> Result<()> { Ok(()) }
    fn position(&self) -> i32 { self.cursor.position() as i32 }
    fn size(&self) -> i32 { self.cursor.get_ref().len() as i32 }
    fn seek(&mut self, position: i32) -> Result<()> { self.cursor.set_position(position as u64); Ok(()) }
    fn has_value(&self, _name: &str) -> bool { true }

    fn export_instruction(&mut self, value: u8, _name: &str) -> Result<()> {
        self.cursor.write_u8(value).map_err(AltiumError::Io)
    }
    fn import_instruction(&mut self, _name: &str) -> Result<u8> {
        self.cursor.read_u8().map_err(AltiumError::Io)
    }
    fn export_instruction_ex(&mut self, value: i32, _name: &str) -> Result<()> {
        self.cursor.write_i32::<LittleEndian>(value).map_err(AltiumError::Io)
    }
    fn import_instruction_ex(&mut self, _name: &str) -> Result<i32> {
        self.cursor.read_i32::<LittleEndian>().map_err(AltiumError::Io)
    }

    fn export_byte(&mut self, value: u8, _name: &str) -> Result<()> {
        self.cursor.write_u8(value).map_err(AltiumError::Io)
    }
    fn import_byte(&mut self, _name: &str) -> Result<u8> {
        self.cursor.read_u8().map_err(AltiumError::Io)
    }

    fn export_short_int(&mut self, value: i32, _name: &str) -> Result<()> {
        self.cursor.write_i16::<LittleEndian>(value as i16).map_err(AltiumError::Io)
    }
    fn import_short_int(&mut self, _name: &str) -> Result<i32> {
        self.cursor.read_i16::<LittleEndian>().map(|v| v as i32).map_err(AltiumError::Io)
    }

    fn export_long_int(&mut self, value: i32, _name: &str) -> Result<()> {
        self.cursor.write_i32::<LittleEndian>(value).map_err(AltiumError::Io)
    }
    fn import_long_int(&mut self, _name: &str) -> Result<i32> {
        self.cursor.read_i32::<LittleEndian>().map_err(AltiumError::Io)
    }
    fn export_long(&mut self, value: i64, _name: &str) -> Result<()> {
        self.cursor.write_i64::<LittleEndian>(value).map_err(AltiumError::Io)
    }
    fn import_long(&mut self, _name: &str) -> Result<i64> {
        self.cursor.read_i64::<LittleEndian>().map_err(AltiumError::Io)
    }

    fn export_coord(&mut self, value: i32, _name: &str) -> Result<()> {
        let whole = (value / 100_000) as i16;
        self.cursor.write_i16::<LittleEndian>(whole).map_err(AltiumError::Io)
    }
    fn import_coord(&mut self, _name: &str) -> Result<i32> {
        let whole = self.cursor.read_i16::<LittleEndian>().map_err(AltiumError::Io)?;
        Ok(whole as i32 * 100_000)
    }

    fn export_boolean(&mut self, value: bool, _name: &str) -> Result<()> {
        self.cursor.write_u8(if value { 1 } else { 0 }).map_err(AltiumError::Io)
    }
    fn import_boolean(&mut self, _name: &str) -> Result<bool> {
        let v = self.cursor.read_u8().map_err(AltiumError::Io)?;
        Ok(v != 0)
    }

    fn export_color(&mut self, value: u32, _name: &str) -> Result<()> {
        self.cursor.write_u32::<LittleEndian>(value).map_err(AltiumError::Io)
    }
    fn import_color(&mut self, _name: &str) -> Result<u32> {
        self.cursor.read_u32::<LittleEndian>().map_err(AltiumError::Io)
    }

    fn export_string(&mut self, value: &str, _name: &str) -> Result<()> {
        self.write_pascal_string(value)
    }
    fn import_string(&mut self, _name: &str) -> Result<String> {
        self.read_pascal_string()
    }

    fn export_dynamic_string(&mut self, value: &str, _name: &str) -> Result<()> {
        let truncated = if value.len() > 254 { &value[..254] } else { value };
        self.write_pascal_string(truncated)
    }
    fn import_dynamic_string(&mut self, _name: &str) -> Result<String> {
        self.read_pascal_string()
    }

    fn export_text(&mut self, value: &str, _name: &str) -> Result<()> {
        self.write_text_string(value)
    }
    fn import_text(&mut self, _name: &str) -> Result<String> {
        self.read_text_string()
    }

    fn export_double(&mut self, value: f64, _name: &str) -> Result<()> {
        self.cursor.write_f64::<LittleEndian>(value).map_err(AltiumError::Io)
    }
    fn import_double(&mut self, _name: &str) -> Result<f64> {
        self.cursor.read_f64::<LittleEndian>().map_err(AltiumError::Io)
    }

    fn export_angle(&mut self, value: f64, _name: &str) -> Result<()> {
        // TODO: Real48 encoding for angles in binary mode
        self.cursor.write_f64::<LittleEndian>(value).map_err(AltiumError::Io)
    }
    fn import_angle(&mut self, _name: &str) -> Result<f64> {
        self.cursor.read_f64::<LittleEndian>().map_err(AltiumError::Io)
    }

    fn export_font_id(&mut self, value: i32, _name: &str) -> Result<()> {
        self.cursor.write_i16::<LittleEndian>(value as i16).map_err(AltiumError::Io)
    }
    fn import_font_id(&mut self, _name: &str) -> Result<i32> {
        self.cursor.read_i16::<LittleEndian>().map(|v| v as i32).map_err(AltiumError::Io)
    }

    fn export_display_mode(&mut self, value: u8, _name: &str) -> Result<()> {
        self.cursor.write_u8(value).map_err(AltiumError::Io)
    }
    fn import_display_mode(&mut self, _name: &str) -> Result<u8> {
        self.cursor.read_u8().map_err(AltiumError::Io)
    }

    impl_enum_binary!(export_rotation_by90, import_rotation_by90, RotationBy90, |v| RotationBy90::from_u8(v).unwrap_or_default());
    impl_enum_binary!(export_pin_electrical, import_pin_electrical, PinElectrical, |v| PinElectrical::from_u8(v).unwrap_or_default());
    impl_enum_binary!(export_ieee_symbol, import_ieee_symbol, IeeeSymbol, IeeeSymbol::from_u8);
    impl_enum_binary!(export_line_style, import_line_style, LineStyle, |v| LineStyle::from_u8(v).unwrap_or_default());
    impl_enum_binary!(export_port_arrow_style, import_port_arrow_style, PortArrowStyle, |v| PortArrowStyle::from_u8(v).unwrap_or_default());
    impl_enum_binary!(export_port_io, import_port_io, PortIO, |v| PortIO::from_u8(v).unwrap_or_default());
    impl_enum_binary!(export_power_object_style, import_power_object_style, PowerObjectStyle, |v| PowerObjectStyle::from_u8(v).unwrap_or_default());
    impl_enum_binary!(export_text_justification, import_text_justification, TextJustification, |v| TextJustification::from_u8(v).unwrap_or_default());
    impl_enum_binary!(export_size, import_size, Size, |v| Size::from_u8(v).unwrap_or_default());
    impl_enum_binary!(export_no_erc_symbol, import_no_erc_symbol, NoERCSymbol, |v| NoERCSymbol::from_u8(v).unwrap_or_default());
    impl_enum_binary!(export_parameter_type, import_parameter_type, ParameterType, |v| ParameterType::from_u8(v).unwrap_or_default());
    impl_enum_binary!(export_left_right_side, import_left_right_side, LeftRightSide, |v| LeftRightSide::from_u8(v).unwrap_or_default());
    impl_enum_binary!(export_line_shape, import_line_shape, LineShape, |v| LineShape::from_u8(v).unwrap_or_default());
    impl_enum_binary!(export_horizontal_align, import_horizontal_align, HorizontalAlign, |v| HorizontalAlign::from_u8(v).unwrap_or_default());
    impl_enum_binary!(export_text_horizontal_anchor, import_text_horizontal_anchor, TextHorzAnchor, |v| TextHorzAnchor::from_u8(v).unwrap_or_default());
    impl_enum_binary!(export_text_vertical_anchor, import_text_vertical_anchor, TextVertAnchor, |v| TextVertAnchor::from_u8(v).unwrap_or_default());
    impl_enum_binary!(export_parameter_kind, import_parameter_kind, ParameterType, |v| ParameterType::from_u8(v).unwrap_or_default());
    impl_enum_binary!(export_parameter_read_only_state, import_parameter_read_only_state, ParameterReadOnlyState, |v| ParameterReadOnlyState::from_u8(v).unwrap_or_default());
    impl_enum_binary!(export_parameter_set_style, import_parameter_set_style, ParameterSetStyle, |v| ParameterSetStyle::from_u8(v).unwrap_or_default());

    fn export_boolean_with_default(&mut self, value: bool, name: &str) -> Result<()> {
        self.export_boolean(value, name)
    }
    fn import_boolean_with_default(&mut self, name: &str, _default: bool) -> Result<bool> {
        self.import_boolean(name)
    }

    fn export_binary(&mut self, value: &[u8], _name: &str) -> Result<()> {
        let len = value.len() as i32;
        self.cursor.write_i32::<LittleEndian>(len).map_err(AltiumError::Io)?;
        self.cursor.write_all(value).map_err(AltiumError::Io)?;
        Ok(())
    }
    fn import_binary(&mut self, _name: &str) -> Result<Vec<u8>> {
        let len = self.cursor.read_i32::<LittleEndian>().map_err(AltiumError::Io)?;
        let mut buf = vec![0u8; len as usize];
        self.cursor.read_exact(&mut buf).map_err(AltiumError::Io)?;
        Ok(buf)
    }

    // ASCII-only variants are no-ops in binary mode
    fn export_ascii_only_string(&mut self, _v: &str, _n: &str) -> Result<()> { Ok(()) }
    fn import_ascii_only_string(&mut self, _n: &str) -> Result<String> { Ok(String::new()) }
    fn export_ascii_only_boolean(&mut self, _v: bool, _n: &str) -> Result<()> { Ok(()) }
    fn import_ascii_only_boolean(&mut self, _n: &str) -> Result<bool> { Ok(false) }
    fn export_ascii_only_coord(&mut self, _v: i32, _n: &str) -> Result<()> { Ok(()) }
    fn import_ascii_only_coord(&mut self, _n: &str) -> Result<i32> { Ok(0) }
    fn export_ascii_only_color(&mut self, _v: u32, _n: &str) -> Result<()> { Ok(()) }
    fn import_ascii_only_color(&mut self, _n: &str) -> Result<u32> { Ok(0) }
    fn export_ascii_only_byte(&mut self, _v: u8, _n: &str) -> Result<()> { Ok(()) }
    fn import_ascii_only_byte(&mut self, _n: &str) -> Result<u8> { Ok(0) }
    fn export_ascii_only_long_int(&mut self, _v: i32, _n: &str) -> Result<()> { Ok(()) }
    fn import_ascii_only_long_int(&mut self, _n: &str) -> Result<i32> { Ok(0) }
    fn export_ascii_only_font_id(&mut self, _v: i32, _n: &str) -> Result<()> { Ok(()) }
    fn import_ascii_only_font_id(&mut self, _n: &str) -> Result<i32> { Ok(0) }
    fn export_ascii_only_double(&mut self, _v: f64, _n: &str) -> Result<()> { Ok(()) }
    fn import_ascii_only_double(&mut self, _n: &str) -> Result<f64> { Ok(0.0) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_basic_types() {
        let mut w = BinarySerializer::new_writer();
        w.export_byte(42, "b").unwrap();
        w.export_boolean(true, "flag").unwrap();
        w.export_short_int(300, "s").unwrap();
        w.export_long_int(-12345, "i").unwrap();
        w.export_color(0xFF00FF, "c").unwrap();
        w.export_string("hello", "str").unwrap();

        let data = w.into_bytes();
        let mut r = BinarySerializer::from_bytes(data);

        assert_eq!(r.import_byte("b").unwrap(), 42);
        assert!(r.import_boolean("flag").unwrap());
        assert_eq!(r.import_short_int("s").unwrap(), 300);
        assert_eq!(r.import_long_int("i").unwrap(), -12345);
        assert_eq!(r.import_color("c").unwrap(), 0xFF00FF);
        assert_eq!(r.import_string("str").unwrap(), "hello");
    }

    #[test]
    fn round_trip_coord() {
        let mut w = BinarySerializer::new_writer();
        w.export_coord(4_200_000, "X").unwrap();

        let data = w.into_bytes();
        assert_eq!(data.len(), 2);

        let mut r = BinarySerializer::from_bytes(data);
        assert_eq!(r.import_coord("X").unwrap(), 4_200_000);
    }

    #[test]
    fn round_trip_enums() {
        let mut w = BinarySerializer::new_writer();
        w.export_rotation_by90(RotationBy90::Rotate270, "rot").unwrap();
        w.export_pin_electrical(PinElectrical::Output, "elec").unwrap();
        w.export_ieee_symbol(IeeeSymbol::Clock, "ieee").unwrap();

        let data = w.into_bytes();
        let mut r = BinarySerializer::from_bytes(data);

        assert_eq!(r.import_rotation_by90("rot").unwrap(), RotationBy90::Rotate270);
        assert_eq!(r.import_pin_electrical("elec").unwrap(), PinElectrical::Output);
        assert_eq!(r.import_ieee_symbol("ieee").unwrap(), IeeeSymbol::Clock);
    }

    #[test]
    fn ascii_only_noop_in_binary() {
        let mut w = BinarySerializer::new_writer();
        w.export_ascii_only_string("ignored", "s").unwrap();
        w.export_ascii_only_boolean(true, "b").unwrap();
        w.export_ascii_only_coord(100, "c").unwrap();

        let data = w.into_bytes();
        assert!(data.is_empty());
    }

    #[test]
    fn round_trip_text() {
        let mut w = BinarySerializer::new_writer();
        w.export_text("hello world", "t").unwrap();

        let data = w.into_bytes();
        let mut r = BinarySerializer::from_bytes(data);
        assert_eq!(r.import_text("t").unwrap(), "hello world");
    }
}
