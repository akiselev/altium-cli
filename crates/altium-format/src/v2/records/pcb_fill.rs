//! PCB Fill record type for the v2 API.
//!
//! Binary layout (sequential):
//! ```text
//! Offset  Size  Field
//!   0     13    PcbCommonHeader
//!  13      4    corner1_x (PcbCoord / i32)
//!  17      4    corner1_y (PcbCoord / i32)
//!  21      4    corner2_x (PcbCoord / i32)
//!  25      4    corner2_y (PcbCoord / i32)
//!  29      8    rotation (f64, degrees)
//! ```
//! Total: 37 bytes for the sequential portion.

use altium_format_derive::altium_record;
use crate::v2::binary_helpers::PcbCommonHeader;
use crate::v2::coord::PcbCoord;

#[altium_record(kind = "pcb", object_id = Fill, codec = "binary")]
pub struct PcbFillRecord {
    #[altium(header)]
    header: PcbCommonHeader,
    corner1_x: PcbCoord,
    corner1_y: PcbCoord,
    corner2_x: PcbCoord,
    corner2_y: PcbCoord,
    rotation: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::backing_store::{BinaryOrigin, RecordOrigin};
    use crate::v2::coord::AltiumCoord;

    #[test]
    fn fill_read_from_binary() {
        let mut data = vec![0u8; 37];
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

        let origin = RecordOrigin::Binary(BinaryOrigin::new(data));
        let rec = PcbFillRecord::from_origin(origin);

        assert_eq!(rec.corner1_x().to_raw(), 100_000);
        assert_eq!(rec.corner1_y().to_raw(), 100_000);
        assert_eq!(rec.corner2_x().to_raw(), 200_000);
        assert_eq!(rec.corner2_y().to_raw(), 200_000);
        assert!((rec.rotation() - 45.0).abs() < 1e-10);
    }

    #[test]
    fn fill_write_roundtrip() {
        let mut data = vec![0u8; 37];
        data[13..17].copy_from_slice(&100_000i32.to_le_bytes());
        data[29..37].copy_from_slice(&45.0f64.to_le_bytes());

        let origin = RecordOrigin::Binary(BinaryOrigin::new(data));
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
        let mut data = vec![0u8; 37];
        data[0] = 1; // TopLayer

        let origin = RecordOrigin::Binary(BinaryOrigin::new(data));
        let rec = PcbFillRecord::from_origin(origin);

        let header = rec.header();
        assert_eq!(header.layer, 1);
    }
}
