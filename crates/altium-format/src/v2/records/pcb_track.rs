//! PCB Track record type for the v2 API.
//!
//! Binary layout:
//! ```text
//! Offset  Size  Field
//!   0     13    PcbCommonHeader
//!  13      4    start_x (PcbCoord / i32)
//!  17      4    start_y (PcbCoord / i32)
//!  21      4    end_x (PcbCoord / i32)
//!  25      4    end_y (PcbCoord / i32)
//!  29      4    width (PcbCoord / i32)
//!  33      2    subpoly_index (u16)
//!  35      1    user_routed (bool, AD26+)
//!  36      4    union_index (i32, AD26+)
//!  40      1    track_kind (u8, AD26+)
//!  41      4    layer_enum_index (i32, AD26+)
//!  45      4    keepout_restrictions (i32, AD26+)
//! ```
//!
//! Legacy records can be shorter (35 bytes). AD26 tracks are 49 bytes.

use crate::v2::binary_helpers::PcbCommonHeader;
use crate::v2::coord::PcbCoord;
use altium_format_derive::altium_record;

#[altium_record(kind = "pcb", object_id = Track, codec = "binary",
    parse_fn = "parse_track", serialize_fn = "serialize_track")]
pub struct PcbTrackRecord {
    #[altium(header)]
    header: PcbCommonHeader,
    start_x: PcbCoord,
    start_y: PcbCoord,
    end_x: PcbCoord,
    end_y: PcbCoord,
    width: PcbCoord,
    subpoly_index: u16,
    user_routed: bool,
    union_index: i32,
    track_kind: u8,
    layer_enum_index: i32,
    keepout_restrictions: i32,
}

/// Parse track data from the raw binary block.
///
/// Supports both legacy (35-byte) and AD26 (49-byte) layouts.
pub(crate) fn parse_track(data: &[u8]) -> crate::Result<crate::v2::backing_store::RecordOrigin> {
    use crate::error::AltiumError;
    use crate::v2::backing_store::{BinaryOrigin, FieldSpan};

    if data.len() < 35 {
        return Err(AltiumError::Parse(format!(
            "track data too short: {} bytes (need >= 35)",
            data.len()
        )));
    }

    let mut spans = vec![
        FieldSpan::new(0, 13),  // 0: header
        FieldSpan::new(13, 4),  // 1: start_x
        FieldSpan::new(17, 4),  // 2: start_y
        FieldSpan::new(21, 4),  // 3: end_x
        FieldSpan::new(25, 4),  // 4: end_y
        FieldSpan::new(29, 4),  // 5: width
        FieldSpan::new(33, 2),  // 6: subpoly_index
    ];

    if data.len() >= 49 {
        spans.push(FieldSpan::new(35, 1)); // 7: user_routed
        spans.push(FieldSpan::new(36, 4)); // 8: union_index
        spans.push(FieldSpan::new(40, 1)); // 9: track_kind
        spans.push(FieldSpan::new(41, 4)); // 10: layer_enum_index
        spans.push(FieldSpan::new(45, 4)); // 11: keepout_restrictions
    } else {
        // Legacy fallback spans: keep typed accessors available without
        // requiring AD26-only trailing bytes.
        spans.push(FieldSpan::new(34, 1)); // 7: user_routed (fallback)
        spans.push(FieldSpan::new(29, 4)); // 8: union_index (fallback)
        spans.push(FieldSpan::new(34, 1)); // 9: track_kind (fallback)
        spans.push(FieldSpan::new(29, 4)); // 10: layer_enum_index (fallback)
        spans.push(FieldSpan::new(29, 4)); // 11: keepout_restrictions (fallback)
    }

    Ok(crate::v2::backing_store::RecordOrigin::Binary(
        BinaryOrigin::with_spans(data.to_vec(), spans),
    ))
}

/// Serialize track data back to binary.
#[allow(dead_code)]
fn serialize_track(origin: &crate::v2::backing_store::BinaryOrigin) -> crate::Result<Vec<u8>> {
    Ok(origin.raw_block.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::backing_store::{BinaryOrigin, FieldSpan, RecordOrigin};
    use crate::v2::coord::AltiumCoord;

    #[test]
    fn track_read_from_ad26_binary() {
        let mut data = vec![0u8; 49];
        // Write start_x at offset 13
        data[13..17].copy_from_slice(&100_000i32.to_le_bytes());
        // Write start_y at offset 17
        data[17..21].copy_from_slice(&200_000i32.to_le_bytes());
        // Write end_x at offset 21
        data[21..25].copy_from_slice(&300_000i32.to_le_bytes());
        // Write end_y at offset 25
        data[25..29].copy_from_slice(&400_000i32.to_le_bytes());
        // Write width at offset 29
        data[29..33].copy_from_slice(&10_000i32.to_le_bytes());
        // Write subpoly_index at offset 33
        data[33..35].copy_from_slice(&0xFFFFu16.to_le_bytes());
        // AD26 trailing fields
        data[35] = 1; // user_routed
        data[36..40].copy_from_slice(&42i32.to_le_bytes()); // union_index
        data[40] = 3; // track_kind
        data[41..45].copy_from_slice(&74i32.to_le_bytes()); // layer_enum_index
        data[45..49].copy_from_slice(&7i32.to_le_bytes()); // keepout_restrictions

        let origin = parse_track(&data).unwrap();
        let rec = PcbTrackRecord::from_origin(origin);

        assert_eq!(rec.start_x().to_raw(), 100_000);
        assert_eq!(rec.start_y().to_raw(), 200_000);
        assert_eq!(rec.end_x().to_raw(), 300_000);
        assert_eq!(rec.end_y().to_raw(), 400_000);
        assert_eq!(rec.width().to_raw(), 10_000);
        assert_eq!(rec.subpoly_index(), 0xFFFF);
        assert!(rec.user_routed());
        assert_eq!(rec.union_index(), 42);
        assert_eq!(rec.track_kind(), 3);
        assert_eq!(rec.layer_enum_index(), 74);
        assert_eq!(rec.keepout_restrictions(), 7);
    }

    #[test]
    fn track_write_roundtrip() {
        let mut data = vec![0u8; 49];
        data[13..17].copy_from_slice(&100_000i32.to_le_bytes());

        let origin = parse_track(&data).unwrap();
        let mut rec = PcbTrackRecord::from_origin(origin);

        // Verify initial value
        assert_eq!(rec.start_x().to_raw(), 100_000);

        // Modify and verify
        rec.set_start_x(PcbCoord::from_raw(999_999));
        assert_eq!(rec.start_x().to_raw(), 999_999);

        // Verify header getter works
        let header = rec.header();
        assert_eq!(header.layer, 0);

        rec.set_union_index(99);
        assert_eq!(rec.union_index(), 99);
    }

    #[test]
    fn track_header_access() {
        let mut data = vec![0u8; 49];
        // Set layer byte at offset 0
        data[0] = 1; // TopLayer
        // Set net at offset 3-4
        data[3] = 7;
        data[4] = 0;

        let origin = parse_track(&data).unwrap();
        let rec = PcbTrackRecord::from_origin(origin);

        let header = rec.header();
        assert_eq!(header.layer, 1);
        assert_eq!(header.net, 7);
    }

    #[test]
    fn copy_modeled_fields_from_copies_binary_fields() {
        let mut src_data = vec![0u8; 49];
        src_data[13..17].copy_from_slice(&111_000i32.to_le_bytes());
        src_data[29..33].copy_from_slice(&12_000i32.to_le_bytes());
        src_data[35] = 1;
        let src = PcbTrackRecord::from_origin(parse_track(&src_data).unwrap());

        let dst_data = vec![0u8; 49];
        let mut dst = PcbTrackRecord::from_origin(parse_track(&dst_data).unwrap());

        dst.copy_modeled_fields_from(&src);

        assert_eq!(dst.start_x().to_raw(), 111_000);
        assert_eq!(dst.width().to_raw(), 12_000);
        assert!(dst.user_routed());
    }

    #[test]
    fn track_legacy_layout_still_parses() {
        let mut data = vec![0u8; 35];
        data[13..17].copy_from_slice(&100_000i32.to_le_bytes());

        let rec = PcbTrackRecord::from_origin(parse_track(&data).unwrap());
        assert_eq!(rec.start_x().to_raw(), 100_000);
    }

    #[test]
    fn track_manual_spans_read_write() {
        let mut data = vec![0u8; 64];
        data[0] = 74;
        data[13..17].copy_from_slice(&123_456i32.to_le_bytes());
        data[35] = 0;

        let spans = vec![
            FieldSpan::new(0, 13),
            FieldSpan::new(13, 4),
            FieldSpan::new(17, 4),
            FieldSpan::new(21, 4),
            FieldSpan::new(25, 4),
            FieldSpan::new(29, 4),
            FieldSpan::new(33, 2),
            FieldSpan::new(35, 1),
            FieldSpan::new(36, 4),
            FieldSpan::new(40, 1),
            FieldSpan::new(41, 4),
            FieldSpan::new(45, 4),
        ];
        let mut rec =
            PcbTrackRecord::from_origin(RecordOrigin::Binary(BinaryOrigin::with_spans(data, spans)));
        assert_eq!(rec.start_x().to_raw(), 123_456);
        rec.set_user_routed(true);
        assert!(rec.user_routed());
    }
}
