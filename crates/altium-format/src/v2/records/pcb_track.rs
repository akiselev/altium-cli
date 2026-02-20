//! PCB Track record type for the v2 API.
//!
//! Binary layout (sequential):
//! ```text
//! Offset  Size  Field
//!   0     13    PcbCommonHeader
//!  13      4    start_x (PcbCoord / i32)
//!  17      4    start_y (PcbCoord / i32)
//!  21      4    end_x (PcbCoord / i32)
//!  25      4    end_y (PcbCoord / i32)
//!  29      4    width (PcbCoord / i32)
//!  33      2    subpoly_index (u16)
//! ```
//! Total: 35 bytes for the sequential portion.

use crate::v2::binary_helpers::PcbCommonHeader;
use crate::v2::coord::PcbCoord;
use altium_format_derive::altium_record;

#[altium_record(kind = "pcb", object_id = Track, codec = "binary")]
pub struct PcbTrackRecord {
    #[altium(header)]
    header: PcbCommonHeader,
    start_x: PcbCoord,
    start_y: PcbCoord,
    end_x: PcbCoord,
    end_y: PcbCoord,
    width: PcbCoord,
    subpoly_index: u16,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::backing_store::{BinaryOrigin, RecordOrigin};
    use crate::v2::coord::AltiumCoord;

    #[test]
    fn track_read_from_binary() {
        let mut data = vec![0u8; 35];
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

        let origin = RecordOrigin::Binary(BinaryOrigin::new(data));
        let rec = PcbTrackRecord::from_origin(origin);

        assert_eq!(rec.start_x().to_raw(), 100_000);
        assert_eq!(rec.start_y().to_raw(), 200_000);
        assert_eq!(rec.end_x().to_raw(), 300_000);
        assert_eq!(rec.end_y().to_raw(), 400_000);
        assert_eq!(rec.width().to_raw(), 10_000);
        assert_eq!(rec.subpoly_index(), 0xFFFF);
    }

    #[test]
    fn track_write_roundtrip() {
        let mut data = vec![0u8; 35];
        data[13..17].copy_from_slice(&100_000i32.to_le_bytes());

        let origin = RecordOrigin::Binary(BinaryOrigin::new(data));
        let mut rec = PcbTrackRecord::from_origin(origin);

        // Verify initial value
        assert_eq!(rec.start_x().to_raw(), 100_000);

        // Modify and verify
        rec.set_start_x(PcbCoord::from_raw(999_999));
        assert_eq!(rec.start_x().to_raw(), 999_999);

        // Verify header getter works
        let header = rec.header();
        assert_eq!(header.layer, 0);
    }

    #[test]
    fn track_header_access() {
        let mut data = vec![0u8; 35];
        // Set layer byte at offset 0
        data[0] = 1; // TopLayer
        // Set net at offset 3-4
        data[3] = 7;
        data[4] = 0;

        let origin = RecordOrigin::Binary(BinaryOrigin::new(data));
        let rec = PcbTrackRecord::from_origin(origin);

        let header = rec.header();
        assert_eq!(header.layer, 1);
        assert_eq!(header.net, 7);
    }
}
