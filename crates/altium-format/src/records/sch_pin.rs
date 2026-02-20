//! Schematic pin record (RECORD=2).

use super::enums::*;
use crate::coord::SchCoord;
use crate::newtypes::{Designator, PinName, UniqueId};
use crate::traits::{AltiumEnum, RecordType};
use altium_format_derive::altium_record;
use encoding_rs::WINDOWS_1252;

/// Schematic pin record -- RECORD=2.
///
/// The most complex record type with 50+ fields. Pin does NOT use the
/// GraphicalObjectBase -- it has its own OwnerIndex/OwnerPartId fields
/// serialized directly.
#[altium_record(kind = "sch", record_id = 2, codec = "params")]
pub struct SchPinRecord {
    // --- Base fields (exported directly, not via ExportDataObject) ---
    #[altium(key = "OwnerIndex")]
    owner_index: i32,

    #[altium(key = "OwnerPartId")]
    owner_part_id: i16,

    #[altium(key = "OwnerPartDisplayMode")]
    owner_part_display_mode: u8,

    // --- IEEE symbols ---
    #[altium(key = "SymBol_InnerEdge")]
    symbol_inner_edge: IeeeSymbol,

    #[altium(key = "SymBol_OuterEdge")]
    symbol_outer_edge: IeeeSymbol,

    #[altium(key = "SymBol_Inner")]
    symbol_inner: IeeeSymbol,

    #[altium(key = "SymBol_Outer")]
    symbol_outer: IeeeSymbol,

    // --- Core pin properties ---
    #[altium(key = "Description")]
    description: String,

    #[altium(key = "FormalType")]
    formal_type: StdLogicState,

    #[altium(key = "Electrical")]
    electrical: PinElectricalType,

    // --- PinConglomerate (packed byte) ---
    #[altium(key = "PinConglomerate")]
    pin_conglomerate: u8,

    #[altium(key = "PinLength")]
    pin_length: SchCoord,

    #[altium(key = "Location.X")]
    location_x: SchCoord,

    #[altium(key = "Location.Y")]
    location_y: SchCoord,

    #[altium(key = "Color")]
    color: u32,

    #[altium(key = "Name")]
    name: PinName,

    #[altium(key = "Designator")]
    designator: Designator,

    #[altium(key = "SwapIdPin")]
    swap_id_pin: String,

    #[altium(key = "SwapIDPart")]
    swap_id_part: String,

    #[altium(key = "DefaultValue")]
    default_value: String,

    #[altium(key = "SwapIdPair")]
    swap_id_pair: String,

    // --- Name customization ---
    #[altium(key = "PinName_PositionConglomerate")]
    pin_name_position_conglomerate: u8,

    #[altium(key = "Name_CustomPosition_Margin")]
    name_custom_position_margin: i32,

    #[altium(key = "Name_CustomFontID")]
    name_custom_font_id: i32,

    #[altium(key = "Name_CustomColor")]
    name_custom_color: u32,

    // --- Designator customization ---
    #[altium(key = "PinDesignator_PositionConglomerate")]
    pin_designator_position_conglomerate: u8,

    #[altium(key = "Designator_CustomPosition_Margin")]
    designator_custom_position_margin: i32,

    #[altium(key = "Designator_CustomFontID")]
    designator_custom_font_id: i32,

    #[altium(key = "Designator_CustomColor")]
    designator_custom_color: u32,

    // --- Symbol line width ---
    #[altium(key = "SymBol_LineWidth")]
    symbol_line_width: Size,

    // --- Extended data ---
    #[altium(key = "PinPackageLength")]
    pin_package_length: SchCoord,

    #[altium(key = "PinPropagationDelay")]
    pin_propagation_delay: f64,

    // --- Unique ID ---
    #[altium(key = "UniqueID")]
    unique_id: UniqueId,

    // --- Alternate pin functions ---
    #[altium(key = "HidePinNameAsFunction")]
    hide_pin_name_as_function: bool,

    #[altium(key = "PinSymbolicName")]
    pin_symbolic_name: String,

    #[altium(key = "ShowPinSymbolicNameAsFunction")]
    show_symbolic_name_as_function: bool,
}

impl SchPinRecord {
    /// Parse a legacy binary SchLib pin payload into a typed pin record.
    ///
    /// Some SchLib files store `RECORD=2` pins in binary form (size flag with
    /// binary bit set) instead of parameter text. This helper decodes the
    /// mandatory prefix emitted by AD's binary serializer:
    /// `record_id, owner/index fields, symbol bytes, core coords/color, name,
    /// designator, swap/default strings`.
    pub fn from_legacy_binary_record_data(data: &[u8]) -> Option<Self> {
        fn read_u8(data: &[u8], pos: &mut usize) -> Option<u8> {
            let b = *data.get(*pos)?;
            *pos += 1;
            Some(b)
        }
        fn read_i16_le(data: &[u8], pos: &mut usize) -> Option<i16> {
            let s = data.get(*pos..*pos + 2)?;
            *pos += 2;
            Some(i16::from_le_bytes([s[0], s[1]]))
        }
        fn read_i32_le(data: &[u8], pos: &mut usize) -> Option<i32> {
            let s = data.get(*pos..*pos + 4)?;
            *pos += 4;
            Some(i32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        }
        fn read_u32_le(data: &[u8], pos: &mut usize) -> Option<u32> {
            let s = data.get(*pos..*pos + 4)?;
            *pos += 4;
            Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        }
        fn read_lp_string(data: &[u8], pos: &mut usize) -> Option<String> {
            let len = read_u8(data, pos)? as usize;
            let s = data.get(*pos..*pos + len)?;
            *pos += len;
            Some(String::from_utf8_lossy(s).to_string())
        }

        let mut pos = 0usize;
        let record_id = read_u8(data, &mut pos)?;
        if record_id != Self::RECORD_ID {
            return None;
        }

        let owner_index = read_i32_le(data, &mut pos)?;
        let owner_part_id = read_i16_le(data, &mut pos)?;
        let owner_part_display_mode = read_u8(data, &mut pos)?;

        let symbol_inner_edge = read_u8(data, &mut pos)?;
        let symbol_outer_edge = read_u8(data, &mut pos)?;
        let symbol_inner = read_u8(data, &mut pos)?;
        let symbol_outer = read_u8(data, &mut pos)?;

        let description = read_lp_string(data, &mut pos)?;
        let formal_type = read_u8(data, &mut pos)?;
        let electrical = read_u8(data, &mut pos)?;
        let pin_conglomerate = read_u8(data, &mut pos)?;

        let pin_length_whole = read_i16_le(data, &mut pos)?;
        let location_x_whole = read_i16_le(data, &mut pos)?;
        let location_y_whole = read_i16_le(data, &mut pos)?;

        let color = read_u32_le(data, &mut pos)?;
        let name = read_lp_string(data, &mut pos)?;
        let designator = read_lp_string(data, &mut pos)?;
        let swap_id_pin = read_lp_string(data, &mut pos)?;
        let swap_id_part = read_lp_string(data, &mut pos)?;
        let default_value = read_lp_string(data, &mut pos)?;

        let mut rec = Self::from_origin(crate::templates::sch_pin_default());
        rec.set_owner_index(owner_index);
        rec.set_owner_part_id(owner_part_id);
        rec.set_owner_part_display_mode(owner_part_display_mode);
        rec.set_symbol_inner_edge(IeeeSymbol::from_int(symbol_inner_edge as i32));
        rec.set_symbol_outer_edge(IeeeSymbol::from_int(symbol_outer_edge as i32));
        rec.set_symbol_inner(IeeeSymbol::from_int(symbol_inner as i32));
        rec.set_symbol_outer(IeeeSymbol::from_int(symbol_outer as i32));
        rec.set_description(description);
        rec.set_formal_type(StdLogicState::from_int(formal_type as i32));
        rec.set_electrical(PinElectricalType::from_int(electrical as i32));
        rec.set_pin_conglomerate(pin_conglomerate);
        rec.set_pin_length(SchCoord::from_binary_parts(pin_length_whole, 0));
        rec.set_location_x(SchCoord::from_binary_parts(location_x_whole, 0));
        rec.set_location_y(SchCoord::from_binary_parts(location_y_whole, 0));
        rec.set_color(color);
        rec.set_name(PinName::from(name));
        rec.set_designator(Designator::from(designator));
        rec.set_swap_id_pin(swap_id_pin);
        rec.set_swap_id_part(swap_id_part);
        rec.set_default_value(default_value);

        // Optional trailing dynamic string (typically UniqueID for document pins).
        if pos < data.len() {
            if let Some(unique_id) = read_lp_string(data, &mut pos) {
                if !unique_id.is_empty() {
                    rec.set_unique_id(UniqueId::from(unique_id));
                }
            }
        }

        Some(rec)
    }

    /// Encode this pin as a legacy SchLib binary `RECORD=2` payload.
    ///
    /// This mirrors AD's param serializer binary mode for SchLib pin records:
    /// fixed prefix fields plus short-string payloads up through `DefaultValue`.
    pub fn to_legacy_binary_record_data(&self) -> Vec<u8> {
        fn write_u8(out: &mut Vec<u8>, v: u8) {
            out.push(v);
        }
        fn write_i16_le(out: &mut Vec<u8>, v: i16) {
            out.extend_from_slice(&v.to_le_bytes());
        }
        fn write_i32_le(out: &mut Vec<u8>, v: i32) {
            out.extend_from_slice(&v.to_le_bytes());
        }
        fn write_u32_le(out: &mut Vec<u8>, v: u32) {
            out.extend_from_slice(&v.to_le_bytes());
        }
        fn write_lp_string(out: &mut Vec<u8>, s: &str) {
            // AD truncates dynamic strings to 254 chars in binary mode.
            let (bytes, _, _) = WINDOWS_1252.encode(s);
            let mut bytes = bytes.into_owned();
            if bytes.len() > 254 {
                bytes.truncate(254);
            }
            out.push(bytes.len() as u8);
            out.extend_from_slice(&bytes);
        }

        let mut out = Vec::with_capacity(64);
        write_u8(&mut out, Self::RECORD_ID);
        write_i32_le(&mut out, self.owner_index());
        write_i16_le(&mut out, self.owner_part_id());
        write_u8(&mut out, self.owner_part_display_mode());
        write_u8(&mut out, self.symbol_inner_edge().to_int() as u8);
        write_u8(&mut out, self.symbol_outer_edge().to_int() as u8);
        write_u8(&mut out, self.symbol_inner().to_int() as u8);
        write_u8(&mut out, self.symbol_outer().to_int() as u8);
        write_lp_string(&mut out, &self.description());
        write_u8(&mut out, self.formal_type().to_int() as u8);
        write_u8(&mut out, self.electrical().to_int() as u8);
        write_u8(&mut out, self.pin_conglomerate());

        let (pin_len_whole, _) = self.pin_length().to_binary_parts();
        let (loc_x_whole, _) = self.location_x().to_binary_parts();
        let (loc_y_whole, _) = self.location_y().to_binary_parts();
        write_i16_le(&mut out, pin_len_whole);
        write_i16_le(&mut out, loc_x_whole);
        write_i16_le(&mut out, loc_y_whole);

        write_u32_le(&mut out, self.color());
        write_lp_string(&mut out, &self.name().to_string());
        write_lp_string(&mut out, &self.designator().to_string());
        write_lp_string(&mut out, &self.swap_id_pin());
        write_lp_string(&mut out, &self.swap_id_part());
        write_lp_string(&mut out, &self.default_value());
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backing_store::{ParamOrigin, RecordOrigin};

    #[test]
    fn roundtrip_pin_getter() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=2|Designator=1|Name=VCC|PinLength=30|Location.X=100|Location.Y=200|Electrical=7|",
        ));
        let rec = SchPinRecord::from_origin(origin);
        assert_eq!(rec.designator(), Designator::from("1"));
        assert_eq!(rec.name(), PinName::from("VCC"));
        assert_eq!(rec.electrical(), PinElectricalType::Power);
    }

    #[test]
    fn roundtrip_pin_setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=2|Designator=1|Name=VCC|PinLength=30|",
        ));
        let mut rec = SchPinRecord::from_origin(origin);
        rec.set_designator(Designator::from("2"));
        assert_eq!(rec.designator(), Designator::from("2"));
        rec.set_name(PinName::from("GND"));
        assert_eq!(rec.name(), PinName::from("GND"));
    }

    #[test]
    fn copy_modeled_fields_from_copies_values() {
        let src = SchPinRecord::from_origin(RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=2|Designator=7|Name=SCL|PinLength=40|",
        )));
        let mut dst = SchPinRecord::from_origin(RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=2|Designator=1|Name=VCC|PinLength=30|",
        )));

        dst.copy_modeled_fields_from(&src);

        assert_eq!(dst.designator(), Designator::from("7"));
        assert_eq!(dst.name(), PinName::from("SCL"));
    }

    #[test]
    fn parse_legacy_binary_pin() {
        // Binary payload from Synthiam.SchLib pin record.
        let data: [u8; 33] = [
            0x02, 0x00, 0x00, 0x00, // record id + owner_index low bytes
            0x00, // owner_index high byte (=> 0)
            0x01, 0x00, // owner_part_id = 1
            0x00, // owner_part_display_mode
            0x00, // symbol_inner_edge
            0x00, // symbol_outer_edge
            0x00, // symbol_inner
            0x00, // symbol_outer
            0x00, // description length
            0x01, // formal_type
            0x04, // electrical
            0x3A, // pin_conglomerate
            0x1E, 0x00, // pin_length whole = 30
            0xE3, 0xFF, // location_x whole = -29
            0xEE, 0x00, // location_y whole = 238
            0x00, 0x00, 0x00, 0x00, // color
            0x01, b'1', // name
            0x01, b'1', // designator
            0x00, // swap_id_pin
            0x00, // swap_id_part
            0x00, // default_value
        ];

        let rec =
            SchPinRecord::from_legacy_binary_record_data(&data).expect("binary pin should decode");
        assert_eq!(rec.designator(), Designator::from("1"));
        assert_eq!(rec.name(), PinName::from("1"));
        assert_eq!(rec.owner_part_id(), 1);
    }
}
