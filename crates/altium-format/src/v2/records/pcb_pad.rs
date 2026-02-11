//! PCB Pad record type for the v2 API.
//!
//! The pad record is a complex multi-section binary format with 6 subrecords.
//! It uses custom parse/serialize functions which are stubbed for now and
//! will be fully implemented in Phase 4 (document I/O).
//!
//! Subrecords (from Ghidra):
//! 1. Pad name (WxString)
//! 2. Unknown string (often empty)
//! 3. Unknown string (often `|&|0`)
//! 4. Unknown string (often empty)
//! 5. Main pad data (172 bytes in AD26)
//! 6. Per-layer stack data (596/628/651 bytes)

use altium_format_derive::altium_record;
use crate::v2::coord::PcbCoord;

#[altium_record(kind = "pcb", object_id = Pad, codec = "binary",
    parse_fn = "parse_pad", serialize_fn = "serialize_pad")]
pub struct PcbPadRecord {
    /// X position in PCB coordinates.
    position_x: PcbCoord,
    /// Y position in PCB coordinates.
    position_y: PcbCoord,
    /// Top layer X size.
    top_size_x: PcbCoord,
    /// Top layer Y size.
    top_size_y: PcbCoord,
    /// Mid layer X size.
    mid_size_x: PcbCoord,
    /// Mid layer Y size.
    mid_size_y: PcbCoord,
    /// Bottom layer X size.
    bot_size_x: PcbCoord,
    /// Bottom layer Y size.
    bot_size_y: PcbCoord,
    /// Hole size.
    hole_size: PcbCoord,
    /// Top layer shape (TShape enum value).
    top_shape: u8,
    /// Mid layer shape.
    mid_shape: u8,
    /// Bottom layer shape.
    bot_shape: u8,
    /// Rotation in degrees.
    rotation: f64,
    /// Whether the pad hole is plated.
    is_plated: bool,
    /// Pad stack mode (TPadMode enum value).
    pad_mode: u8,
    /// Paste mask expansion.
    paste_mask_expansion: PcbCoord,
    /// Solder mask expansion.
    solder_mask_expansion: PcbCoord,
}

#[allow(dead_code)]
fn parse_pad(_data: &[u8]) -> crate::Result<crate::v2::backing_store::RecordOrigin> {
    todo!("Complex pad parsing -- will be implemented in Phase 4")
}

#[allow(dead_code)]
fn serialize_pad(_origin: &crate::v2::backing_store::BinaryOrigin) -> crate::Result<Vec<u8>> {
    todo!("Complex pad serialization -- will be implemented in Phase 4")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::backing_store::{BinaryOrigin, FieldSpan, RecordOrigin};
    use crate::v2::coord::AltiumCoord;

    /// Build a test BinaryOrigin with field spans for the pad record.
    fn make_test_pad_origin() -> RecordOrigin {
        let mut data = vec![0u8; 256];

        // position_x at offset 0
        data[0..4].copy_from_slice(&100_000i32.to_le_bytes());
        // position_y at offset 4
        data[4..8].copy_from_slice(&200_000i32.to_le_bytes());
        // top_size_x at offset 8
        data[8..12].copy_from_slice(&50_000i32.to_le_bytes());
        // top_size_y at offset 12
        data[12..16].copy_from_slice(&50_000i32.to_le_bytes());
        // mid_size_x at offset 16
        data[16..20].copy_from_slice(&40_000i32.to_le_bytes());
        // mid_size_y at offset 20
        data[20..24].copy_from_slice(&40_000i32.to_le_bytes());
        // bot_size_x at offset 24
        data[24..28].copy_from_slice(&50_000i32.to_le_bytes());
        // bot_size_y at offset 28
        data[28..32].copy_from_slice(&50_000i32.to_le_bytes());
        // hole_size at offset 32
        data[32..36].copy_from_slice(&10_000i32.to_le_bytes());
        // top_shape at offset 36
        data[36] = 1; // Rounded
        // mid_shape at offset 37
        data[37] = 1;
        // bot_shape at offset 38
        data[38] = 1;
        // rotation at offset 39
        data[39..47].copy_from_slice(&45.0f64.to_le_bytes());
        // is_plated at offset 47
        data[47] = 1;
        // pad_mode at offset 48
        data[48] = 0; // Simple
        // paste_mask_expansion at offset 49
        data[49..53].copy_from_slice(&1000i32.to_le_bytes());
        // solder_mask_expansion at offset 53
        data[53..57].copy_from_slice(&2000i32.to_le_bytes());

        let spans = vec![
            FieldSpan::new(0, 4),   // position_x
            FieldSpan::new(4, 4),   // position_y
            FieldSpan::new(8, 4),   // top_size_x
            FieldSpan::new(12, 4),  // top_size_y
            FieldSpan::new(16, 4),  // mid_size_x
            FieldSpan::new(20, 4),  // mid_size_y
            FieldSpan::new(24, 4),  // bot_size_x
            FieldSpan::new(28, 4),  // bot_size_y
            FieldSpan::new(32, 4),  // hole_size
            FieldSpan::new(36, 1),  // top_shape
            FieldSpan::new(37, 1),  // mid_shape
            FieldSpan::new(38, 1),  // bot_shape
            FieldSpan::new(39, 8),  // rotation
            FieldSpan::new(47, 1),  // is_plated
            FieldSpan::new(48, 1),  // pad_mode
            FieldSpan::new(49, 4),  // paste_mask_expansion
            FieldSpan::new(53, 4),  // solder_mask_expansion
        ];

        RecordOrigin::Binary(BinaryOrigin::with_spans(data, spans))
    }

    #[test]
    fn pad_read_from_field_spans() {
        let origin = make_test_pad_origin();
        let rec = PcbPadRecord::from_origin(origin);

        assert_eq!(rec.position_x().to_raw(), 100_000);
        assert_eq!(rec.position_y().to_raw(), 200_000);
        assert_eq!(rec.top_size_x().to_raw(), 50_000);
        assert_eq!(rec.hole_size().to_raw(), 10_000);
        assert_eq!(rec.top_shape(), 1);
        assert!((rec.rotation() - 45.0).abs() < 1e-10);
        assert!(rec.is_plated());
        assert_eq!(rec.pad_mode(), 0);
        assert_eq!(rec.paste_mask_expansion().to_raw(), 1000);
        assert_eq!(rec.solder_mask_expansion().to_raw(), 2000);
    }

    #[test]
    fn pad_write_via_field_spans() {
        let origin = make_test_pad_origin();
        let mut rec = PcbPadRecord::from_origin(origin);

        rec.set_position_x(PcbCoord::from_raw(999_999));
        assert_eq!(rec.position_x().to_raw(), 999_999);

        rec.set_rotation(90.0);
        assert!((rec.rotation() - 90.0).abs() < 1e-10);

        rec.set_is_plated(false);
        assert!(!rec.is_plated());
    }
}
