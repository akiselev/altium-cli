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

fn parse_text(_data: &[u8]) -> crate::Result<crate::v2::backing_store::RecordOrigin> {
    todo!("Complex text parsing -- will be implemented in Phase 4")
}

fn serialize_text(_origin: &crate::v2::backing_store::BinaryOrigin) -> crate::Result<Vec<u8>> {
    todo!("Complex text serialization -- will be implemented in Phase 4")
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
