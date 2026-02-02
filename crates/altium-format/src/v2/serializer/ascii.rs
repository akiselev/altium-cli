//! ASCII (parametric) serializer — Mode 0: `|KEY=VALUE|` parameter strings.
//!
//! In ASCII mode, each record is serialized as a pipe-delimited string like:
//! `|RECORD=2|OwnerIndex=0|OwnerPartId=1|...|`
//!
//! Booleans: "T"/"F" (export omits false by default, with_default always writes)
//! Integers: decimal string (omitted if zero for non-default exports)
//! Coords: exported as whole mils (i16 short), fractional part in KEY_Frac
//! Strings: literal value (omitted if empty for non-default exports)

use indexmap::IndexMap;

use crate::error::{AltiumError, Result};
use crate::v2::types::*;
use super::SchSerializer;

/// ASCII parametric serializer.
///
/// When writing, accumulates key-value pairs and serializes them
/// as `|KEY=VALUE|` on flush. When reading, parses a parameter string
/// into a lookup map.
#[derive(Clone, Debug)]
pub struct AsciiSerializer {
    params: IndexMap<String, String>,
}

impl AsciiSerializer {
    /// Creates a new writer.
    pub fn new_writer() -> Self {
        AsciiSerializer {
            params: IndexMap::new(),
        }
    }

    /// Creates a reader from a pipe-delimited parameter string.
    pub fn from_params(param_string: &str) -> Self {
        let mut params = IndexMap::new();
        // Parse |KEY=VALUE| format
        for segment in param_string.split('|') {
            let segment = segment.trim();
            if segment.is_empty() {
                continue;
            }
            if let Some((key, value)) = segment.split_once('=') {
                params.insert(key.to_string(), value.to_string());
            }
        }
        AsciiSerializer {
            params,
        }
    }

    /// Returns the serialized parameter string.
    pub fn to_param_string(&self) -> String {
        let mut s = String::new();
        for (key, value) in &self.params {
            s.push('|');
            s.push_str(key);
            s.push('=');
            s.push_str(value);
        }
        if !s.is_empty() {
            s.push('|');
        }
        s
    }

    /// Returns the serialized parameter string as UTF-8 bytes with null terminator.
    pub fn to_param_bytes(&self) -> Vec<u8> {
        let s = self.to_param_string();
        let mut bytes = s.into_bytes();
        bytes.push(0); // null terminated
        bytes
    }

    fn set_param(&mut self, name: &str, value: String) {
        self.params.insert(name.to_string(), value);
    }

    fn get_param(&self, name: &str) -> Option<&str> {
        self.params.get(name).map(|s| s.as_str())
    }

    fn get_param_or_default<'a>(&'a self, name: &str, default: &'a str) -> &'a str {
        self.params
            .get(name)
            .map(|s| s.as_str())
            .unwrap_or(default)
    }

    fn parse_int(&self, name: &str) -> Result<i32> {
        match self.get_param(name) {
            Some(s) => s
                .parse::<i32>()
                .map_err(|_| AltiumError::Parse(format!("Invalid int for {}: {:?}", name, s))),
            None => Ok(0),
        }
    }

    fn parse_bool(&self, name: &str) -> Result<bool> {
        match self.get_param(name) {
            Some("T") | Some("t") => Ok(true),
            _ => Ok(false),
        }
    }
}

impl SchSerializer for AsciiSerializer {
    fn start_stream(&mut self, _section: &str, _name: &str) -> Result<()> {
        Ok(())
    }

    fn end_stream(&mut self) -> Result<()> {
        Ok(())
    }

    fn stream_exists(&self, _section: &str, _name: &str) -> bool {
        false
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }

    fn position(&self) -> i32 {
        0
    }

    fn size(&self) -> i32 {
        0
    }

    fn seek(&mut self, _position: i32) -> Result<()> {
        Ok(())
    }

    fn has_value(&self, name: &str) -> bool {
        self.params.contains_key(name)
    }

    // --- Instruction ---
    fn export_instruction(&mut self, value: u8, name: &str) -> Result<()> {
        self.set_param(name, value.to_string());
        Ok(())
    }

    fn import_instruction(&mut self, name: &str) -> Result<u8> {
        match self.get_param(name) {
            Some(s) => s
                .parse::<u8>()
                .map_err(|_| AltiumError::Parse(format!("Invalid instruction for {}", name))),
            None => Ok(0),
        }
    }

    fn export_instruction_ex(&mut self, value: i32, name: &str) -> Result<()> {
        self.set_param(name, value.to_string());
        Ok(())
    }

    fn import_instruction_ex(&mut self, name: &str) -> Result<i32> {
        self.parse_int(name)
    }

    // --- Byte ---
    fn export_byte(&mut self, value: u8, name: &str) -> Result<()> {
        if value != 0 {
            self.set_param(name, (value as u32).to_string());
        }
        Ok(())
    }

    fn import_byte(&mut self, name: &str) -> Result<u8> {
        match self.get_param(name) {
            Some(s) => {
                let v: i32 = s
                    .parse()
                    .map_err(|_| AltiumError::Parse(format!("Invalid byte for {}", name)))?;
                Ok(v as u8)
            }
            None => Ok(0),
        }
    }

    // --- Short ---
    fn export_short_int(&mut self, value: i32, name: &str) -> Result<()> {
        if value != 0 {
            self.set_param(name, value.to_string());
        }
        Ok(())
    }

    fn import_short_int(&mut self, name: &str) -> Result<i32> {
        self.parse_int(name)
    }

    // --- Long Int ---
    fn export_long_int(&mut self, value: i32, name: &str) -> Result<()> {
        if value != 0 {
            self.set_param(name, value.to_string());
        }
        Ok(())
    }

    fn import_long_int(&mut self, name: &str) -> Result<i32> {
        self.parse_int(name)
    }

    // --- Coord ---
    fn export_coord(&mut self, value: i32, name: &str) -> Result<()> {
        // ASCII mode: coord as whole mils (short), frac in KEY_Frac
        let whole = value / 100_000;
        let frac = value - whole * 100_000;
        if whole != 0 {
            self.set_param(name, whole.to_string());
        }
        if frac != 0 {
            self.set_param(&format!("{}_Frac", name), frac.to_string());
        }
        Ok(())
    }

    fn import_coord(&mut self, name: &str) -> Result<i32> {
        let whole = self.parse_int(name)?;
        let frac_name = format!("{}_Frac", name);
        let frac = self.parse_int(&frac_name).unwrap_or(0);
        Ok(whole * 100_000 + frac)
    }

    // --- Boolean ---
    fn export_boolean(&mut self, value: bool, name: &str) -> Result<()> {
        // C#: WriteBool — only writes "T" if true, omits if false
        if value {
            self.set_param(name, "T".to_string());
        }
        Ok(())
    }

    fn import_boolean(&mut self, name: &str) -> Result<bool> {
        self.parse_bool(name)
    }

    // --- Color ---
    fn export_color(&mut self, value: u32, name: &str) -> Result<()> {
        if value != 0 {
            self.set_param(name, value.to_string());
        }
        Ok(())
    }

    fn import_color(&mut self, name: &str) -> Result<u32> {
        match self.get_param(name) {
            Some(s) => s
                .parse::<u32>()
                .map_err(|_| AltiumError::Parse(format!("Invalid color for {}", name))),
            None => Ok(0),
        }
    }

    // --- Strings ---
    fn export_string(&mut self, value: &str, name: &str) -> Result<()> {
        if !value.is_empty() {
            self.set_param(name, value.to_string());
        }
        Ok(())
    }

    fn import_string(&mut self, name: &str) -> Result<String> {
        Ok(self.get_param_or_default(name, "").to_string())
    }

    fn export_dynamic_string(&mut self, value: &str, name: &str) -> Result<()> {
        self.export_string(value, name)
    }

    fn import_dynamic_string(&mut self, name: &str) -> Result<String> {
        self.import_string(name)
    }

    fn export_text(&mut self, value: &str, name: &str) -> Result<()> {
        self.export_string(value, name)
    }

    fn import_text(&mut self, name: &str) -> Result<String> {
        self.import_string(name)
    }

    // --- Double ---
    fn export_double(&mut self, value: f64, name: &str) -> Result<()> {
        if value.abs() > f64::EPSILON {
            self.set_param(name, format!("{:.3}", value));
        }
        Ok(())
    }

    fn import_double(&mut self, name: &str) -> Result<f64> {
        match self.get_param(name) {
            Some(s) => s
                .parse::<f64>()
                .map_err(|_| AltiumError::Parse(format!("Invalid double for {}", name))),
            None => Ok(0.0),
        }
    }

    fn export_angle(&mut self, value: f64, name: &str) -> Result<()> {
        self.export_double(value, name)
    }

    fn import_angle(&mut self, name: &str) -> Result<f64> {
        self.import_double(name)
    }

    // --- Font ---
    fn export_font_id(&mut self, value: i32, name: &str) -> Result<()> {
        self.export_short_int(value, name)
    }

    fn import_font_id(&mut self, name: &str) -> Result<i32> {
        self.import_short_int(name)
    }

    // --- Display mode ---
    fn export_display_mode(&mut self, value: u8, name: &str) -> Result<()> {
        self.export_byte(value, name)
    }

    fn import_display_mode(&mut self, name: &str) -> Result<u8> {
        self.import_byte(name)
    }

    // --- Enum types ---
    fn export_rotation_by90(&mut self, value: RotationBy90, name: &str) -> Result<()> {
        self.export_byte(value as u8, name)
    }

    fn import_rotation_by90(&mut self, name: &str) -> Result<RotationBy90> {
        let v = self.import_byte(name)?;
        Ok(RotationBy90::from_u8(v).unwrap_or_default())
    }

    fn export_pin_electrical(&mut self, value: PinElectrical, name: &str) -> Result<()> {
        self.export_byte(value as u8, name)
    }

    fn import_pin_electrical(&mut self, name: &str) -> Result<PinElectrical> {
        let v = self.import_byte(name)?;
        Ok(PinElectrical::from_u8(v).unwrap_or_default())
    }

    fn export_ieee_symbol(&mut self, value: IeeeSymbol, name: &str) -> Result<()> {
        self.export_byte(value as u8, name)
    }

    fn import_ieee_symbol(&mut self, name: &str) -> Result<IeeeSymbol> {
        let v = self.import_byte(name)?;
        Ok(IeeeSymbol::from_u8(v))
    }

    fn export_line_style(&mut self, value: LineStyle, name: &str) -> Result<()> {
        self.export_byte(value as u8, name)
    }

    fn import_line_style(&mut self, name: &str) -> Result<LineStyle> {
        let v = self.import_byte(name)?;
        Ok(LineStyle::from_u8(v).unwrap_or_default())
    }

    fn export_port_arrow_style(&mut self, value: PortArrowStyle, name: &str) -> Result<()> {
        self.export_byte(value as u8, name)
    }

    fn import_port_arrow_style(&mut self, name: &str) -> Result<PortArrowStyle> {
        let v = self.import_byte(name)?;
        Ok(PortArrowStyle::from_u8(v).unwrap_or_default())
    }

    fn export_port_io(&mut self, value: PortIO, name: &str) -> Result<()> {
        self.export_byte(value as u8, name)
    }

    fn import_port_io(&mut self, name: &str) -> Result<PortIO> {
        let v = self.import_byte(name)?;
        Ok(PortIO::from_u8(v).unwrap_or_default())
    }

    fn export_power_object_style(&mut self, value: PowerObjectStyle, name: &str) -> Result<()> {
        self.export_byte(value as u8, name)
    }

    fn import_power_object_style(&mut self, name: &str) -> Result<PowerObjectStyle> {
        let v = self.import_byte(name)?;
        Ok(PowerObjectStyle::from_u8(v).unwrap_or_default())
    }

    fn export_text_justification(
        &mut self,
        value: TextJustification,
        name: &str,
    ) -> Result<()> {
        self.export_byte(value as u8, name)
    }

    fn import_text_justification(&mut self, name: &str) -> Result<TextJustification> {
        let v = self.import_byte(name)?;
        Ok(TextJustification::from_u8(v).unwrap_or_default())
    }

    fn export_size(&mut self, value: Size, name: &str) -> Result<()> {
        self.export_byte(value as u8, name)
    }

    fn import_size(&mut self, name: &str) -> Result<Size> {
        let v = self.import_byte(name)?;
        Ok(Size::from_u8(v).unwrap_or_default())
    }

    fn export_no_erc_symbol(&mut self, value: NoERCSymbol, name: &str) -> Result<()> {
        self.export_byte(value as u8, name)
    }

    fn import_no_erc_symbol(&mut self, name: &str) -> Result<NoERCSymbol> {
        let v = self.import_byte(name)?;
        Ok(NoERCSymbol::from_u8(v).unwrap_or_default())
    }

    fn export_parameter_type(&mut self, value: ParameterType, name: &str) -> Result<()> {
        self.export_byte(value as u8, name)
    }

    fn import_parameter_type(&mut self, name: &str) -> Result<ParameterType> {
        let v = self.import_byte(name)?;
        Ok(ParameterType::from_u8(v).unwrap_or_default())
    }

    fn export_left_right_side(&mut self, value: LeftRightSide, name: &str) -> Result<()> {
        self.export_byte(value as u8, name)
    }

    fn import_left_right_side(&mut self, name: &str) -> Result<LeftRightSide> {
        let v = self.import_byte(name)?;
        Ok(LeftRightSide::from_u8(v).unwrap_or_default())
    }

    fn export_line_shape(&mut self, value: LineShape, name: &str) -> Result<()> {
        self.export_byte(value as u8, name)
    }

    fn import_line_shape(&mut self, name: &str) -> Result<LineShape> {
        let v = self.import_byte(name)?;
        Ok(LineShape::from_u8(v).unwrap_or_default())
    }

    fn export_horizontal_align(&mut self, value: HorizontalAlign, name: &str) -> Result<()> {
        self.export_byte(value as u8, name)
    }

    fn import_horizontal_align(&mut self, name: &str) -> Result<HorizontalAlign> {
        let v = self.import_byte(name)?;
        Ok(HorizontalAlign::from_u8(v).unwrap_or_default())
    }

    fn export_text_horizontal_anchor(&mut self, value: TextHorzAnchor, name: &str) -> Result<()> {
        self.export_byte(value as u8, name)
    }

    fn import_text_horizontal_anchor(&mut self, name: &str) -> Result<TextHorzAnchor> {
        let v = self.import_byte(name)?;
        Ok(TextHorzAnchor::from_u8(v).unwrap_or_default())
    }

    fn export_text_vertical_anchor(&mut self, value: TextVertAnchor, name: &str) -> Result<()> {
        self.export_byte(value as u8, name)
    }

    fn import_text_vertical_anchor(&mut self, name: &str) -> Result<TextVertAnchor> {
        let v = self.import_byte(name)?;
        Ok(TextVertAnchor::from_u8(v).unwrap_or_default())
    }

    fn export_parameter_kind(&mut self, value: ParameterType, name: &str) -> Result<()> {
        self.export_byte(value as u8, name)
    }

    fn import_parameter_kind(&mut self, name: &str) -> Result<ParameterType> {
        let v = self.import_byte(name)?;
        Ok(ParameterType::from_u8(v).unwrap_or_default())
    }

    fn export_parameter_read_only_state(&mut self, value: ParameterReadOnlyState, name: &str) -> Result<()> {
        self.export_byte(value as u8, name)
    }

    fn import_parameter_read_only_state(&mut self, name: &str) -> Result<ParameterReadOnlyState> {
        let v = self.import_byte(name)?;
        Ok(ParameterReadOnlyState::from_u8(v).unwrap_or_default())
    }

    fn export_boolean_with_default(&mut self, value: bool, name: &str) -> Result<()> {
        // With-default always writes, even if false
        self.params.insert(name.to_string(), if value { "T".to_string() } else { "F".to_string() });
        Ok(())
    }

    fn import_boolean_with_default(&mut self, name: &str, default: bool) -> Result<bool> {
        match self.params.get(name) {
            Some(v) => Ok(v == "T"),
            None => Ok(default),
        }
    }

    // --- Binary data ---
    fn export_binary(&mut self, _value: &[u8], _name: &str) -> Result<()> {
        // Binary data in ASCII mode uses hex encoding + zlib compression
        // TODO: implement hex encoding + zlib
        Ok(())
    }

    fn import_binary(&mut self, _name: &str) -> Result<Vec<u8>> {
        Ok(Vec::new())
    }

    // --- ASCII-only variants (these are active in ASCII mode) ---
    fn export_ascii_only_string(&mut self, value: &str, name: &str) -> Result<()> {
        self.export_string(value, name)
    }

    fn import_ascii_only_string(&mut self, name: &str) -> Result<String> {
        self.import_string(name)
    }

    fn export_ascii_only_boolean(&mut self, value: bool, name: &str) -> Result<()> {
        self.export_boolean(value, name)
    }

    fn import_ascii_only_boolean(&mut self, name: &str) -> Result<bool> {
        self.import_boolean(name)
    }

    fn export_ascii_only_coord(&mut self, value: i32, name: &str) -> Result<()> {
        self.export_coord(value, name)
    }

    fn import_ascii_only_coord(&mut self, name: &str) -> Result<i32> {
        self.import_coord(name)
    }

    fn export_ascii_only_color(&mut self, value: u32, name: &str) -> Result<()> {
        self.export_color(value, name)
    }

    fn import_ascii_only_color(&mut self, name: &str) -> Result<u32> {
        self.import_color(name)
    }

    fn export_ascii_only_byte(&mut self, value: u8, name: &str) -> Result<()> {
        self.export_byte(value, name)
    }

    fn import_ascii_only_byte(&mut self, name: &str) -> Result<u8> {
        self.import_byte(name)
    }

    fn export_ascii_only_long_int(&mut self, value: i32, name: &str) -> Result<()> {
        self.export_long_int(value, name)
    }

    fn import_ascii_only_long_int(&mut self, name: &str) -> Result<i32> {
        self.import_long_int(name)
    }

    fn export_ascii_only_font_id(&mut self, value: i32, name: &str) -> Result<()> {
        self.export_font_id(value, name)
    }

    fn import_ascii_only_font_id(&mut self, name: &str) -> Result<i32> {
        self.import_font_id(name)
    }

    fn export_ascii_only_double(&mut self, value: f64, name: &str) -> Result<()> {
        self.export_double(value, name)
    }

    fn import_ascii_only_double(&mut self, name: &str) -> Result<f64> {
        self.import_double(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_params() {
        let mut w = AsciiSerializer::new_writer();
        w.export_instruction(2, "RECORD").unwrap();
        w.export_long_int(42, "OwnerIndex").unwrap();
        w.export_boolean(true, "IsHidden").unwrap();
        w.export_string("GND", "Name").unwrap();
        w.export_coord(500_000, "Location.X").unwrap(); // 5.0 mils

        let param_str = w.to_param_string();
        assert!(param_str.contains("|RECORD=2|"));
        assert!(param_str.contains("|OwnerIndex=42|"));
        assert!(param_str.contains("|IsHidden=T|"));
        assert!(param_str.contains("|Name=GND|"));

        let mut r = AsciiSerializer::from_params(&param_str);
        assert_eq!(r.import_instruction("RECORD").unwrap(), 2);
        assert_eq!(r.import_long_int("OwnerIndex").unwrap(), 42);
        assert!(r.import_boolean("IsHidden").unwrap());
        assert_eq!(r.import_string("Name").unwrap(), "GND");
        assert_eq!(r.import_coord("Location.X").unwrap(), 500_000);
    }

    #[test]
    fn coord_with_fraction() {
        let mut w = AsciiSerializer::new_writer();
        // 3.50123 mils = 350_123 internal units
        w.export_coord(350_123, "X").unwrap();

        let s = w.to_param_string();
        let mut r = AsciiSerializer::from_params(&s);
        assert_eq!(r.import_coord("X").unwrap(), 350_123);
    }

    #[test]
    fn boolean_false_omitted() {
        let mut w = AsciiSerializer::new_writer();
        w.export_boolean(false, "Flag").unwrap();
        let s = w.to_param_string();
        assert!(!s.contains("Flag"));
    }

    #[test]
    fn zero_int_omitted() {
        let mut w = AsciiSerializer::new_writer();
        w.export_long_int(0, "Value").unwrap();
        let s = w.to_param_string();
        assert!(!s.contains("Value"));
    }
}
