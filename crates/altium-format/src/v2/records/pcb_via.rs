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

/// Parse via data from the raw binary block.
///
/// Via data is a single block with core fields at fixed offsets.
/// Optional extended fields are present when the data is long enough.
pub(crate) fn parse_via(data: &[u8]) -> crate::Result<crate::v2::backing_store::RecordOrigin> {
    use crate::v2::backing_store::{BinaryOrigin, FieldSpan};
    use crate::error::AltiumError;

    if data.len() < 31 {
        return Err(AltiumError::Parse(format!(
            "via data too short: {} bytes (need >= 31)", data.len()
        )));
    }

    // Core fields at fixed offsets (from v1 PcbVia::from_bytes)
    // Byte 0-12: PcbCommonHeader (13 bytes)
    let mut spans = vec![
        FieldSpan::new(13, 4),  // 0: position_x
        FieldSpan::new(17, 4),  // 1: position_y
        FieldSpan::new(21, 4),  // 2: diameter
        FieldSpan::new(25, 4),  // 3: hole_size
        FieldSpan::new(29, 1),  // 4: layer_start
        FieldSpan::new(30, 1),  // 5: layer_end
    ];

    // via_mode at offset 74 (if data is long enough)
    if data.len() > 74 {
        spans.push(FieldSpan::new(74, 1)); // 6: via_mode
    } else {
        // Point to a safe zero byte at end of core
        spans.push(FieldSpan::new(30, 1)); // 6: via_mode (fallback, reads layer_end)
    }

    // soldermask_expansion_manual at offset 66 bit 1
    if data.len() > 66 {
        spans.push(FieldSpan::new(66, 1)); // 7: soldermask_expansion_manual
    } else {
        spans.push(FieldSpan::new(30, 1)); // 7: fallback
    }

    Ok(crate::v2::backing_store::RecordOrigin::Binary(
        BinaryOrigin::with_spans(data.to_vec(), spans),
    ))
}

/// Serialize via data back to binary.
#[allow(dead_code)]
fn serialize_via(origin: &crate::v2::backing_store::BinaryOrigin) -> crate::Result<Vec<u8>> {
    Ok(origin.raw_block.clone())
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
