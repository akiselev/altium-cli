use altium_format_types::constants::parsing::TEXT_SUBRECORD_COUNT;
use altium_format_types::TextKind;

use crate::binary_io::BinaryReader;
use crate::pcblib::primitives::common::parse_common_header;
use crate::pcblib::PcbText;
use crate::{AltiumFormatError, Result};

/// Parses a Text primitive from its 2 PcbLib subrecords.
///
/// Subrecord 0: text properties (common header + location + height + rotation + flags + font_kind)
/// Subrecord 1: text string content (raw Win1252 bytes)
pub(crate) fn parse_text(subrecords: &[&[u8]]) -> Result<PcbText> {
    if subrecords.len() != TEXT_SUBRECORD_COUNT {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "Text subrecords".to_owned(),
            detail: format!(
                "expected {} subrecords, got {}",
                TEXT_SUBRECORD_COUNT,
                subrecords.len()
            ),
        });
    }

    let mut reader = BinaryReader::new(subrecords[0]);
    let common = parse_common_header(&mut reader)?;
    let location = reader.read_coord_point()?;
    let height = reader.read_coord()?;
    let rotation = reader.read_f64_le()?;
    let is_mirrored = reader.read_u8()? != 0;
    let stroke_width = reader.read_coord()?;
    let is_comment = reader.read_u8()? != 0;
    let is_designator = reader.read_u8()? != 0;
    let font_kind = TextKind::try_from(reader.read_u8()?)?;
    reader.assert_exhausted()?;

    // Subrecord 1 contains the text string (Win1252 encoded).
    let text_bytes = subrecords[1];
    let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(text_bytes);
    let text = decoded.into_owned();

    Ok(PcbText {
        common,
        location,
        height,
        rotation,
        is_mirrored,
        stroke_width,
        is_comment,
        is_designator,
        font_kind,
        text,
        unique_id: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use altium_format_types::{Coord, CoordPoint, TextKind};
    use crate::binary_io::BinaryWriter;

    fn make_text_properties_subrecord(text_kind: u8) -> Vec<u8> {
        let mut w = BinaryWriter::new();
        // Common header (13 bytes)
        w.write_u8(1);      // layer
        w.write_u8(0);      // pad_byte
        w.write_u16_le(0);  // flags
        w.write_i32_le(-1); // net_index
        w.write_u16_le(0xFFFF); // polygon_index
        w.write_u16_le(0);  // component_index
        w.write_u8(0);      // unknown
        // Text fields
        w.write_coord_point(CoordPoint::new(
            Coord::from_internal(10_000),
            Coord::from_internal(20_000),
        ));
        w.write_coord(Coord::from_internal(6_000)); // height
        w.write_f64_le(0.0);  // rotation
        w.write_u8(0);        // is_mirrored = false
        w.write_coord(Coord::from_internal(1_000)); // stroke_width
        w.write_u8(0);        // is_comment = false
        w.write_u8(1);        // is_designator = true
        w.write_u8(text_kind); // font_kind
        w.finish()
    }

    fn make_text_subrecords(text_kind: u8, text: &str) -> [Vec<u8>; 2] {
        let sub0 = make_text_properties_subrecord(text_kind);
        let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(text);
        [sub0, encoded.to_vec()]
    }

    #[test]
    fn parse_text_known_bytes() {
        let subs = make_text_subrecords(0, "R1"); // StrokeFont
        let sub_refs: Vec<&[u8]> = subs.iter().map(|s| s.as_slice()).collect();
        let text = parse_text(&sub_refs).unwrap();
        assert_eq!(text.location.x.to_internal(), 10_000);
        assert_eq!(text.location.y.to_internal(), 20_000);
        assert_eq!(text.height.to_internal(), 6_000);
        assert_eq!(text.rotation, 0.0);
        assert!(!text.is_mirrored);
        assert_eq!(text.stroke_width.to_internal(), 1_000);
        assert!(!text.is_comment);
        assert!(text.is_designator);
        assert_eq!(text.font_kind, TextKind::StrokeFont);
        assert_eq!(text.text, "R1");
        assert!(text.unique_id.is_none());
    }

    #[test]
    fn parse_text_empty_string() {
        let subs = make_text_subrecords(0, "");
        let sub_refs: Vec<&[u8]> = subs.iter().map(|s| s.as_slice()).collect();
        let text = parse_text(&sub_refs).unwrap();
        assert_eq!(text.text, "");
    }

    #[test]
    fn parse_text_truetype_font_kind() {
        let subs = make_text_subrecords(1, "C1"); // TrueTypeFont
        let sub_refs: Vec<&[u8]> = subs.iter().map(|s| s.as_slice()).collect();
        let text = parse_text(&sub_refs).unwrap();
        assert_eq!(text.font_kind, TextKind::TrueTypeFont);
        assert_eq!(text.text, "C1");
    }

    #[test]
    fn wrong_subrecord_count_returns_error() {
        // Only 1 subrecord instead of 2
        let sub0 = make_text_properties_subrecord(0);
        let sub_refs: Vec<&[u8]> = vec![sub0.as_slice()];
        let result = parse_text(&sub_refs);
        assert!(result.is_err());
    }
}
