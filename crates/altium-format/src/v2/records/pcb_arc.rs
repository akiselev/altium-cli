//! PCB Arc record type for the v2 API.
//!
//! Binary layout (sequential):
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
//! ```
//! Total: 47 bytes for the sequential portion.

use altium_format_derive::altium_record;
use crate::v2::binary_helpers::PcbCommonHeader;
use crate::v2::coord::PcbCoord;

#[altium_record(kind = "pcb", object_id = Arc, codec = "binary")]
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::backing_store::{BinaryOrigin, RecordOrigin};
    use crate::v2::coord::AltiumCoord;

    #[test]
    fn arc_read_from_binary() {
        let mut data = vec![0u8; 47];
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

        let origin = RecordOrigin::Binary(BinaryOrigin::new(data));
        let rec = PcbArcRecord::from_origin(origin);

        assert_eq!(rec.center_x().to_raw(), 500_000);
        assert_eq!(rec.center_y().to_raw(), 500_000);
        assert_eq!(rec.radius().to_raw(), 100_000);
        assert!((rec.start_angle() - 0.0).abs() < 1e-10);
        assert!((rec.end_angle() - 90.0).abs() < 1e-10);
        assert_eq!(rec.width().to_raw(), 10_000);
        assert_eq!(rec.subpoly_index(), 0xFFFF);
    }

    #[test]
    fn arc_write_roundtrip() {
        let mut data = vec![0u8; 47];
        data[25..33].copy_from_slice(&45.0f64.to_le_bytes());

        let origin = RecordOrigin::Binary(BinaryOrigin::new(data));
        let mut rec = PcbArcRecord::from_origin(origin);

        assert!((rec.start_angle() - 45.0).abs() < 1e-10);

        rec.set_start_angle(180.0);
        assert!((rec.start_angle() - 180.0).abs() < 1e-10);

        rec.set_radius(PcbCoord::from_raw(250_000));
        assert_eq!(rec.radius().to_raw(), 250_000);
    }

    #[test]
    fn arc_header_access() {
        let mut data = vec![0u8; 47];
        data[0] = 33; // TopOverlay layer
        data[3] = 5; // net = 5
        data[4] = 0;

        let origin = RecordOrigin::Binary(BinaryOrigin::new(data));
        let rec = PcbArcRecord::from_origin(origin);

        let header = rec.header();
        assert_eq!(header.layer, 33);
        assert_eq!(header.net, 5);
    }
}
