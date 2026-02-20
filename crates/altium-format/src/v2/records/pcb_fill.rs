//! PCB Fill record type for the v2 API.
//!
//! Binary layout:
//! ```text
//! Offset  Size  Field
//!   0     13    PcbCommonHeader
//!  13      4    corner1_x (PcbCoord / i32)
//!  17      4    corner1_y (PcbCoord / i32)
//!  21      4    corner2_x (PcbCoord / i32)
//!  25      4    corner2_y (PcbCoord / i32)
//!  29      8    rotation (f64, degrees)
//!  37      1    user_routed (bool, AD26+)
//!  38      4    union_index (i32, AD26+)
//!  42      4    layer_enum_index (i32, AD26+)
//!  46      4    keepout_restrictions (i32, AD26+)
//! ```
//!
//! Legacy records can be shorter (37 bytes). AD26 fills are 50 bytes.

use crate::v2::binary_helpers::PcbCommonHeader;
use crate::v2::coord::PcbCoord;
use altium_format_derive::altium_record;

#[altium_record(kind = "pcb", object_id = Fill, codec = "binary",
    parse_fn = "parse_fill", serialize_fn = "serialize_fill")]
pub struct PcbFillRecord {
    #[altium(header)]
    header: PcbCommonHeader,
    corner1_x: PcbCoord,
    corner1_y: PcbCoord,
    corner2_x: PcbCoord,
    corner2_y: PcbCoord,
    rotation: f64,
    user_routed: bool,
    union_index: i32,
    layer_enum_index: i32,
    keepout_restrictions: i32,
}

/// Parse fill data from the raw binary block.
///
/// Supports both legacy (37-byte) and AD26 (50-byte) layouts.
pub(crate) fn parse_fill(data: &[u8]) -> crate::Result<crate::v2::backing_store::RecordOrigin> {
    use crate::error::AltiumError;
    use crate::v2::backing_store::{BinaryOrigin, FieldSpan};

    if data.len() < 37 {
        return Err(AltiumError::Parse(format!(
            "fill data too short: {} bytes (need >= 37)",
            data.len()
        )));
    }

    let mut spans = vec![
        FieldSpan::new(0, 13),  // 0: header
        FieldSpan::new(13, 4),  // 1: corner1_x
        FieldSpan::new(17, 4),  // 2: corner1_y
        FieldSpan::new(21, 4),  // 3: corner2_x
        FieldSpan::new(25, 4),  // 4: corner2_y
        FieldSpan::new(29, 8),  // 5: rotation
    ];

    if data.len() >= 50 {
        spans.push(FieldSpan::new(37, 1)); // 6: user_routed
        spans.push(FieldSpan::new(38, 4)); // 7: union_index
        spans.push(FieldSpan::new(42, 4)); // 8: layer_enum_index
        spans.push(FieldSpan::new(46, 4)); // 9: keepout_restrictions
    } else {
        // Legacy fallback spans for AD26-only fields.
        spans.push(FieldSpan::new(36, 1)); // 6: user_routed (fallback)
        spans.push(FieldSpan::new(29, 4)); // 7: union_index (fallback)
        spans.push(FieldSpan::new(29, 4)); // 8: layer_enum_index (fallback)
        spans.push(FieldSpan::new(29, 4)); // 9: keepout_restrictions (fallback)
    }

    Ok(crate::v2::backing_store::RecordOrigin::Binary(
        BinaryOrigin::with_spans(data.to_vec(), spans),
    ))
}

/// Serialize fill data back to binary.
#[allow(dead_code)]
fn serialize_fill(origin: &crate::v2::backing_store::BinaryOrigin) -> crate::Result<Vec<u8>> {
    Ok(origin.raw_block.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::coord::AltiumCoord;

    #[test]
    fn fill_read_from_binary() {
        let mut data = vec![0u8; 50];
        // corner1_x at offset 13
        data[13..17].copy_from_slice(&100_000i32.to_le_bytes());
        // corner1_y at offset 17
        data[17..21].copy_from_slice(&100_000i32.to_le_bytes());
        // corner2_x at offset 21
        data[21..25].copy_from_slice(&200_000i32.to_le_bytes());
        // corner2_y at offset 25
        data[25..29].copy_from_slice(&200_000i32.to_le_bytes());
        // rotation at offset 29
        data[29..37].copy_from_slice(&45.0f64.to_le_bytes());
        data[37] = 1; // user_routed
        data[38..42].copy_from_slice(&17i32.to_le_bytes()); // union_index
        data[42..46].copy_from_slice(&74i32.to_le_bytes()); // layer_enum_index
        data[46..50].copy_from_slice(&3i32.to_le_bytes()); // keepout_restrictions

        let origin = parse_fill(&data).unwrap();
        let rec = PcbFillRecord::from_origin(origin);

        assert_eq!(rec.corner1_x().to_raw(), 100_000);
        assert_eq!(rec.corner1_y().to_raw(), 100_000);
        assert_eq!(rec.corner2_x().to_raw(), 200_000);
        assert_eq!(rec.corner2_y().to_raw(), 200_000);
        assert!((rec.rotation() - 45.0).abs() < 1e-10);
        assert!(rec.user_routed());
        assert_eq!(rec.union_index(), 17);
        assert_eq!(rec.layer_enum_index(), 74);
        assert_eq!(rec.keepout_restrictions(), 3);
    }

    #[test]
    fn fill_write_roundtrip() {
        let mut data = vec![0u8; 50];
        data[13..17].copy_from_slice(&100_000i32.to_le_bytes());
        data[29..37].copy_from_slice(&45.0f64.to_le_bytes());

        let origin = parse_fill(&data).unwrap();
        let mut rec = PcbFillRecord::from_origin(origin);

        assert_eq!(rec.corner1_x().to_raw(), 100_000);
        assert!((rec.rotation() - 45.0).abs() < 1e-10);

        rec.set_rotation(90.0);
        assert!((rec.rotation() - 90.0).abs() < 1e-10);

        rec.set_corner1_x(PcbCoord::from_raw(999_999));
        assert_eq!(rec.corner1_x().to_raw(), 999_999);
    }

    #[test]
    fn fill_header_access() {
        let mut data = vec![0u8; 50];
        data[0] = 1; // TopLayer

        let origin = parse_fill(&data).unwrap();
        let rec = PcbFillRecord::from_origin(origin);

        let header = rec.header();
        assert_eq!(header.layer, 1);
    }
}
