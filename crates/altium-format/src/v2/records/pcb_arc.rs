//! PCB Arc record type for the v2 API.
//!
//! Binary layout:
//! ```text
//! Offset  Size  Field
//!   0     13    PcbCommonHeader
//!  13      4    center_x (PcbCoord / i32)
//!  17      4    center_y (PcbCoord / i32)
//!  21      4    radius (PcbCoord / i32)
//!  25      8    start_angle (f64, degrees)
//!  33      8    end_angle (f64, degrees)
//!  41      4    width (PcbCoord / i32)
//!  45      2    subpoly_index (u16)
//!  47      1    user_routed (bool, AD26+)
//!  48      4    union_index (i32, AD26+)
//!  52      4    layer_enum_index (i32, AD26+)
//!  56      4    keepout_restrictions (i32, AD26+)
//! ```
//!
//! AD26 arcs are 60 bytes.

use crate::v2::binary_helpers::PcbCommonHeader;
use crate::v2::coord::PcbCoord;
use altium_format_derive::altium_record;

#[altium_record(kind = "pcb", object_id = Arc, codec = "binary",
    parse_fn = "parse_arc", serialize_fn = "serialize_arc")]
pub struct PcbArcRecord {
    #[altium(header)]
    header: PcbCommonHeader,
    center_x: PcbCoord,
    center_y: PcbCoord,
    radius: PcbCoord,
    start_angle: f64,
    end_angle: f64,
    width: PcbCoord,
    subpoly_index: u16,
    user_routed: bool,
    union_index: i32,
    layer_enum_index: i32,
    keepout_restrictions: i32,
}

/// Parse arc data from the raw binary block.
///
/// Strict AD26 parser: requires 60-byte layout.
pub(crate) fn parse_arc(data: &[u8]) -> crate::Result<crate::v2::backing_store::RecordOrigin> {
    use crate::error::AltiumError;
    use crate::v2::backing_store::{BinaryOrigin, FieldSpan};

    if data.len() < 60 {
        return Err(AltiumError::Parse(format!(
            "arc data too short: {} bytes (need >= 60 for AD26)",
            data.len()
        )));
    }

    let mut spans = vec![
        FieldSpan::new(0, 13),  // 0: header
        FieldSpan::new(13, 4),  // 1: center_x
        FieldSpan::new(17, 4),  // 2: center_y
        FieldSpan::new(21, 4),  // 3: radius
        FieldSpan::new(25, 8),  // 4: start_angle
        FieldSpan::new(33, 8),  // 5: end_angle
        FieldSpan::new(41, 4),  // 6: width
        FieldSpan::new(45, 2),  // 7: subpoly_index
    ];

    spans.push(FieldSpan::new(47, 1)); // 8: user_routed
    spans.push(FieldSpan::new(48, 4)); // 9: union_index
    spans.push(FieldSpan::new(52, 4)); // 10: layer_enum_index
    spans.push(FieldSpan::new(56, 4)); // 11: keepout_restrictions

    Ok(crate::v2::backing_store::RecordOrigin::Binary(
        BinaryOrigin::with_spans(data.to_vec(), spans),
    ))
}

/// Serialize arc data back to binary.
#[allow(dead_code)]
fn serialize_arc(origin: &crate::v2::backing_store::BinaryOrigin) -> crate::Result<Vec<u8>> {
    Ok(origin.raw_block.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::coord::AltiumCoord;

    #[test]
    fn arc_read_from_binary() {
        let mut data = vec![0u8; 60];
        // center_x at offset 13
        data[13..17].copy_from_slice(&500_000i32.to_le_bytes());
        // center_y at offset 17
        data[17..21].copy_from_slice(&500_000i32.to_le_bytes());
        // radius at offset 21
        data[21..25].copy_from_slice(&100_000i32.to_le_bytes());
        // start_angle at offset 25
        data[25..33].copy_from_slice(&0.0f64.to_le_bytes());
        // end_angle at offset 33
        data[33..41].copy_from_slice(&90.0f64.to_le_bytes());
        // width at offset 41
        data[41..45].copy_from_slice(&10_000i32.to_le_bytes());
        // subpoly_index at offset 45
        data[45..47].copy_from_slice(&0xFFFFu16.to_le_bytes());
        data[47] = 1; // user_routed
        data[48..52].copy_from_slice(&7i32.to_le_bytes()); // union_index
        data[52..56].copy_from_slice(&33i32.to_le_bytes()); // layer_enum_index
        data[56..60].copy_from_slice(&5i32.to_le_bytes()); // keepout_restrictions

        let origin = parse_arc(&data).unwrap();
        let rec = PcbArcRecord::from_origin(origin);

        assert_eq!(rec.center_x().to_raw(), 500_000);
        assert_eq!(rec.center_y().to_raw(), 500_000);
        assert_eq!(rec.radius().to_raw(), 100_000);
        assert!((rec.start_angle() - 0.0).abs() < 1e-10);
        assert!((rec.end_angle() - 90.0).abs() < 1e-10);
        assert_eq!(rec.width().to_raw(), 10_000);
        assert_eq!(rec.subpoly_index(), 0xFFFF);
        assert!(rec.user_routed());
        assert_eq!(rec.union_index(), 7);
        assert_eq!(rec.layer_enum_index(), 33);
        assert_eq!(rec.keepout_restrictions(), 5);
    }

    #[test]
    fn arc_write_roundtrip() {
        let mut data = vec![0u8; 60];
        data[25..33].copy_from_slice(&45.0f64.to_le_bytes());

        let origin = parse_arc(&data).unwrap();
        let mut rec = PcbArcRecord::from_origin(origin);

        assert!((rec.start_angle() - 45.0).abs() < 1e-10);

        rec.set_start_angle(180.0);
        assert!((rec.start_angle() - 180.0).abs() < 1e-10);

        rec.set_radius(PcbCoord::from_raw(250_000));
        assert_eq!(rec.radius().to_raw(), 250_000);
    }

    #[test]
    fn arc_header_access() {
        let mut data = vec![0u8; 60];
        data[0] = 33; // TopOverlay layer
        data[3] = 5; // net = 5
        data[4] = 0;

        let origin = parse_arc(&data).unwrap();
        let rec = PcbArcRecord::from_origin(origin);

        let header = rec.header();
        assert_eq!(header.layer, 33);
        assert_eq!(header.net, 5);
    }
}
