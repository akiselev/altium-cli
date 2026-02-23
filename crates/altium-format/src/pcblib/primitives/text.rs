use altium_format_types::TextKind;

use crate::binary_io::BinaryReader;
use crate::pcblib::primitives::common::parse_common_header;
use crate::pcblib::PcbText;
use crate::Result;

/// Parses a Text primitive from a single PcbLib record payload.
///
/// PcbLib uses single-record format: the binary header fields are followed
/// by a pascal string (u8 length + Win1252 bytes) containing the text.
pub(crate) fn parse_text(data: &[u8]) -> Result<PcbText> {
    let mut reader = BinaryReader::new(data);
    let common = parse_common_header(&mut reader)?;
    let location = reader.read_coord_point()?;
    let height = reader.read_coord()?;
    let rotation = reader.read_f64_le()?;
    let is_mirrored = reader.read_u8()? != 0;
    let stroke_width = reader.read_coord()?;
    let is_comment = reader.read_u8()? != 0;
    let is_designator = reader.read_u8()? != 0;
    let font_kind = TextKind::try_from(reader.read_u8()?)?;
    let text = reader.read_pascal_string()?;
    let trailing_bytes = reader.read_remaining().to_vec();

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
        trailing_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use altium_format_types::{Coord, CoordPoint, TextKind};
    use crate::binary_io::BinaryWriter;
    use crate::AltiumFormatError;

    fn make_text_payload(text_kind: u8, text: &str) -> Vec<u8> {
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
        w.write_pascal_string(text); // embedded text
        w.finish()
    }

    #[test]
    fn parse_text_known_bytes() {
        let data = make_text_payload(0, "R1"); // StrokeFont
        let text = parse_text(&data).unwrap();
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
        assert!(text.trailing_bytes.is_empty());
        assert!(text.unique_id.is_none());
    }

    #[test]
    fn parse_text_empty_string() {
        let data = make_text_payload(0, "");
        let text = parse_text(&data).unwrap();
        assert_eq!(text.text, "");
    }

    #[test]
    fn parse_text_truetype_font_kind() {
        let data = make_text_payload(1, "C1"); // TrueTypeFont
        let text = parse_text(&data).unwrap();
        assert_eq!(text.font_kind, TextKind::TrueTypeFont);
        assert_eq!(text.text, "C1");
    }

    #[test]
    fn truncated_text_returns_error() {
        // Too short for common header (needs 13 bytes)
        let data = vec![0u8; 5];
        let result = parse_text(&data);
        assert!(matches!(result, Err(AltiumFormatError::BinaryReadPastEnd { .. })));
    }
}
