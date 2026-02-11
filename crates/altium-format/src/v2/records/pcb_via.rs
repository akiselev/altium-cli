//! PCB Via record type for the v2 API.
//!
//! The via record is a complex multi-section binary format.
//! Via writer produces multiple sections:
//! 1. Core via data (246 bytes)
//! 2. Extended entries (N x 9 bytes with count/stride header)
//! 3. Additional section (42 bytes)
//! 4. Pad layer entries (M x 30 bytes with count/stride header)
//! 5. Trailing data (9 bytes)
//!
//! Uses custom parse/serialize functions stubbed for Phase 4.

use altium_format_derive::altium_record;
use crate::v2::coord::PcbCoord;

#[altium_record(kind = "pcb", object_id = Via, codec = "binary",
    parse_fn = "parse_via", serialize_fn = "serialize_via")]
pub struct PcbViaRecord {
    /// X position in PCB coordinates.
    position_x: PcbCoord,
    /// Y position in PCB coordinates.
    position_y: PcbCoord,
    /// Via diameter.
    diameter: PcbCoord,
    /// Drill hole size.
    hole_size: PcbCoord,
    /// Start layer index.
    layer_start: u8,
    /// End layer index.
    layer_end: u8,
    /// Via mode (0=Simple, etc.).
    via_mode: u8,
    /// Whether solder mask expansion is manually specified.
    soldermask_expansion_manual: bool,
}

#[allow(dead_code)]
fn parse_via(_data: &[u8]) -> crate::Result<crate::v2::backing_store::RecordOrigin> {
    todo!("Complex via parsing -- will be implemented in Phase 4")
}

#[allow(dead_code)]
fn serialize_via(_origin: &crate::v2::backing_store::BinaryOrigin) -> crate::Result<Vec<u8>> {
    todo!("Complex via serialization -- will be implemented in Phase 4")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::backing_store::{BinaryOrigin, FieldSpan, RecordOrigin};
    use crate::v2::coord::AltiumCoord;

    fn make_test_via_origin() -> RecordOrigin {
        let mut data = vec![0u8; 128];

        // position_x at offset 0
        data[0..4].copy_from_slice(&100_000i32.to_le_bytes());
        // position_y at offset 4
        data[4..8].copy_from_slice(&200_000i32.to_le_bytes());
        // diameter at offset 8
        data[8..12].copy_from_slice(&30_000i32.to_le_bytes());
        // hole_size at offset 12
        data[12..16].copy_from_slice(&10_000i32.to_le_bytes());
        // layer_start at offset 16
        data[16] = 1;
        // layer_end at offset 17
        data[17] = 32;
        // via_mode at offset 18
        data[18] = 0;
        // soldermask_expansion_manual at offset 19
        data[19] = 0;

        let spans = vec![
            FieldSpan::new(0, 4),   // position_x
            FieldSpan::new(4, 4),   // position_y
            FieldSpan::new(8, 4),   // diameter
            FieldSpan::new(12, 4),  // hole_size
            FieldSpan::new(16, 1),  // layer_start
            FieldSpan::new(17, 1),  // layer_end
            FieldSpan::new(18, 1),  // via_mode
            FieldSpan::new(19, 1),  // soldermask_expansion_manual
        ];

        RecordOrigin::Binary(BinaryOrigin::with_spans(data, spans))
    }

    #[test]
    fn via_read_from_field_spans() {
        let origin = make_test_via_origin();
        let rec = PcbViaRecord::from_origin(origin);

        assert_eq!(rec.position_x().to_raw(), 100_000);
        assert_eq!(rec.position_y().to_raw(), 200_000);
        assert_eq!(rec.diameter().to_raw(), 30_000);
        assert_eq!(rec.hole_size().to_raw(), 10_000);
        assert_eq!(rec.layer_start(), 1);
        assert_eq!(rec.layer_end(), 32);
        assert_eq!(rec.via_mode(), 0);
        assert!(!rec.soldermask_expansion_manual());
    }

    #[test]
    fn via_write_via_field_spans() {
        let origin = make_test_via_origin();
        let mut rec = PcbViaRecord::from_origin(origin);

        rec.set_position_x(PcbCoord::from_raw(500_000));
        assert_eq!(rec.position_x().to_raw(), 500_000);

        rec.set_hole_size(PcbCoord::from_raw(20_000));
        assert_eq!(rec.hole_size().to_raw(), 20_000);

        rec.set_layer_start(5);
        assert_eq!(rec.layer_start(), 5);
    }
}
