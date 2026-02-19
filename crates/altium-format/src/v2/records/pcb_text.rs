//! PCB Text record type for the v2 API.
//!
//! The text record consists of 2 subrecords:
//! 1. Main text data (252 bytes in AD26, minimum 40)
//! 2. Text string (variable length, null-terminated ASCII)
//!
//! Uses custom parse/serialize functions stubbed for Phase 4.

use altium_format_derive::altium_record;
use crate::v2::coord::PcbCoord;

#[altium_record(kind = "pcb", object_id = Text, codec = "binary",
    parse_fn = "parse_text", serialize_fn = "serialize_text")]
pub struct PcbTextRecord {
    /// X position in PCB coordinates.
    position_x: PcbCoord,
    /// Y position in PCB coordinates.
    position_y: PcbCoord,
    /// Text height.
    height: PcbCoord,
    /// Stroke font type.
    stroke_font_type: u16,
    /// Rotation in degrees.
    rotation: f64,
    /// Whether text is mirrored.
    is_mirrored: bool,
    /// Stroke width.
    stroke_width: PcbCoord,
    /// Font type (0=Stroke, 1=TrueType, 2=Barcode).
    font_type: u8,
    /// Whether text is bold.
    is_bold: bool,
    /// Whether text is italic.
    is_italic: bool,
    /// Whether text is inverted.
    is_inverted: bool,
    /// Whether this is a comment text.
    is_comment: bool,
    /// Whether this is a designator text.
    is_designator: bool,
}

/// Parse text data from the raw binary block (2 subrecords).
///
/// Subrecord 1: Main text data (u32 len + data, 252 bytes in AD26, min 40)
/// Subrecord 2: Text string (u32 len + null-terminated ASCII)
///
/// Typed fields are extracted from subrecord 1 at fixed offsets.
pub(crate) fn parse_text(data: &[u8]) -> crate::Result<crate::v2::backing_store::RecordOrigin> {
    use crate::v2::backing_store::{BinaryOrigin, FieldSpan};
    use crate::error::AltiumError;

    if data.len() < 4 {
        return Err(AltiumError::Parse(
            "text data too short for subrecord 1 length".into(),
        ));
    }

    // Subrecord 1: main text data
    let sub1_len = u32::from_le_bytes(
        data[0..4].try_into().unwrap(),
    ) as usize;
    let sub1_start = 4; // after length prefix
    if sub1_start + sub1_len > data.len() {
        return Err(AltiumError::Parse(
            "text subrecord 1 extends beyond data".into(),
        ));
    }
    if sub1_len < 40 {
        return Err(AltiumError::Parse(format!(
            "text subrecord 1 too short: {} bytes (need >= 40)", sub1_len
        )));
    }

    // Field offsets within subrecord 1 (from v1 PcbText::from_subrecords)
    // Byte 0-12: PcbCommonHeader (13 bytes), then typed fields
    let s = sub1_start;
    let mut spans = vec![
        FieldSpan::new(s + 13, 4),  // 0: position_x
        FieldSpan::new(s + 17, 4),  // 1: position_y
        FieldSpan::new(s + 21, 4),  // 2: height
        FieldSpan::new(s + 25, 2),  // 3: stroke_font_type
        FieldSpan::new(s + 27, 8),  // 4: rotation
        FieldSpan::new(s + 35, 1),  // 5: is_mirrored
        FieldSpan::new(s + 36, 4),  // 6: stroke_width
    ];

    // Extended fields (if subrecord 1 >= 46 bytes)
    if sub1_len >= 46 {
        spans.push(FieldSpan::new(s + 43, 1));  // 7: font_type
        spans.push(FieldSpan::new(s + 44, 1));  // 8: is_bold
        spans.push(FieldSpan::new(s + 45, 1));  // 9: is_italic
    } else {
        // Fallback spans for missing extended fields
        spans.push(FieldSpan::new(s + 35, 1));  // 7: font_type (fallback)
        spans.push(FieldSpan::new(s + 35, 1));  // 8: is_bold (fallback)
        spans.push(FieldSpan::new(s + 35, 1));  // 9: is_italic (fallback)
    }

    if sub1_len > 110 {
        spans.push(FieldSpan::new(s + 110, 1)); // 10: is_inverted
    } else {
        spans.push(FieldSpan::new(s + 35, 1));  // 10: fallback
    }

    if sub1_len >= 46 {
        spans.push(FieldSpan::new(s + 40, 1));  // 11: is_comment
        spans.push(FieldSpan::new(s + 41, 1));  // 12: is_designator
    } else {
        spans.push(FieldSpan::new(s + 35, 1));  // 11: fallback
        spans.push(FieldSpan::new(s + 35, 1));  // 12: fallback
    }

    Ok(crate::v2::backing_store::RecordOrigin::Binary(
        BinaryOrigin::with_spans(data.to_vec(), spans),
    ))
}

/// Serialize text data back to binary.
#[allow(dead_code)]
fn serialize_text(origin: &crate::v2::backing_store::BinaryOrigin) -> crate::Result<Vec<u8>> {
    Ok(origin.raw_block.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::backing_store::{BinaryOrigin, FieldSpan, RecordOrigin};
    use crate::v2::coord::AltiumCoord;

    fn make_test_text_origin() -> RecordOrigin {
        let mut data = vec![0u8; 128];

        // position_x at offset 0
        data[0..4].copy_from_slice(&100_000i32.to_le_bytes());
        // position_y at offset 4
        data[4..8].copy_from_slice(&200_000i32.to_le_bytes());
        // height at offset 8
        data[8..12].copy_from_slice(&50_000i32.to_le_bytes());
        // stroke_font_type at offset 12
        data[12..14].copy_from_slice(&1u16.to_le_bytes());
        // rotation at offset 14
        data[14..22].copy_from_slice(&90.0f64.to_le_bytes());
        // is_mirrored at offset 22
        data[22] = 0;
        // stroke_width at offset 23
        data[23..27].copy_from_slice(&5_000i32.to_le_bytes());
        // font_type at offset 27
        data[27] = 0;
        // is_bold at offset 28
        data[28] = 1;
        // is_italic at offset 29
        data[29] = 0;
        // is_inverted at offset 30
        data[30] = 0;
        // is_comment at offset 31
        data[31] = 0;
        // is_designator at offset 32
        data[32] = 1;

        let spans = vec![
            FieldSpan::new(0, 4),   // position_x
            FieldSpan::new(4, 4),   // position_y
            FieldSpan::new(8, 4),   // height
            FieldSpan::new(12, 2),  // stroke_font_type
            FieldSpan::new(14, 8),  // rotation
            FieldSpan::new(22, 1),  // is_mirrored
            FieldSpan::new(23, 4),  // stroke_width
            FieldSpan::new(27, 1),  // font_type
            FieldSpan::new(28, 1),  // is_bold
            FieldSpan::new(29, 1),  // is_italic
            FieldSpan::new(30, 1),  // is_inverted
            FieldSpan::new(31, 1),  // is_comment
            FieldSpan::new(32, 1),  // is_designator
        ];

        RecordOrigin::Binary(BinaryOrigin::with_spans(data, spans))
    }

    #[test]
    fn text_read_from_field_spans() {
        let origin = make_test_text_origin();
        let rec = PcbTextRecord::from_origin(origin);

        assert_eq!(rec.position_x().to_raw(), 100_000);
        assert_eq!(rec.position_y().to_raw(), 200_000);
        assert_eq!(rec.height().to_raw(), 50_000);
        assert_eq!(rec.stroke_font_type(), 1);
        assert!((rec.rotation() - 90.0).abs() < 1e-10);
        assert!(!rec.is_mirrored());
        assert_eq!(rec.stroke_width().to_raw(), 5_000);
        assert_eq!(rec.font_type(), 0);
        assert!(rec.is_bold());
        assert!(!rec.is_italic());
        assert!(!rec.is_inverted());
        assert!(!rec.is_comment());
        assert!(rec.is_designator());
    }

    #[test]
    fn text_write_via_field_spans() {
        let origin = make_test_text_origin();
        let mut rec = PcbTextRecord::from_origin(origin);

        rec.set_position_x(PcbCoord::from_raw(500_000));
        assert_eq!(rec.position_x().to_raw(), 500_000);

        rec.set_rotation(180.0);
        assert!((rec.rotation() - 180.0).abs() < 1e-10);

        rec.set_is_bold(false);
        assert!(!rec.is_bold());

        rec.set_is_designator(false);
        assert!(!rec.is_designator());
    }
}
