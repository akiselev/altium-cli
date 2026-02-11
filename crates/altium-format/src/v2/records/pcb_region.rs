//! PCB Region record type for the v2 API.
//!
//! The region record is a hybrid binary+parametric format:
//! - 18-byte binary header (layer, flags, net, polygon, component, skip5, holecount, skip2)
//! - Parametric properties (`|KEY=VALUE|` text)
//! - Outline vertices (f64 x, f64 y pairs)
//! - Hole vertex lists
//!
//! Uses custom parse/serialize functions stubbed for Phase 4.

use altium_format_derive::altium_record;

#[altium_record(kind = "pcb", object_id = Region, codec = "binary",
    parse_fn = "parse_region", serialize_fn = "serialize_region")]
pub struct PcbRegionRecord {
    /// Layer byte from the binary header.
    layer: u8,
    /// Flags from the binary header.
    flags: u16,
    /// Net index from the binary header.
    net: u16,
    /// Polygon reference from the binary header.
    polygon_ref: u16,
    /// Component reference from the binary header.
    component_ref: u16,
    /// Number of holes (cutouts).
    hole_count: u16,
    /// Number of outline vertices.
    num_outline_vertices: u32,
}

#[allow(dead_code)]
fn parse_region(_data: &[u8]) -> crate::Result<crate::v2::backing_store::RecordOrigin> {
    todo!("Complex region parsing -- will be implemented in Phase 4")
}

#[allow(dead_code)]
fn serialize_region(_origin: &crate::v2::backing_store::BinaryOrigin) -> crate::Result<Vec<u8>> {
    todo!("Complex region serialization -- will be implemented in Phase 4")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::backing_store::{BinaryOrigin, FieldSpan, RecordOrigin};

    fn make_test_region_origin() -> RecordOrigin {
        let mut data = vec![0u8; 64];

        // layer at offset 0
        data[0] = 1;
        // flags at offset 1
        data[1..3].copy_from_slice(&0x0001u16.to_le_bytes());
        // net at offset 3
        data[3..5].copy_from_slice(&7u16.to_le_bytes());
        // polygon_ref at offset 5
        data[5..7].copy_from_slice(&0u16.to_le_bytes());
        // component_ref at offset 7
        data[7..9].copy_from_slice(&3u16.to_le_bytes());
        // hole_count at offset 9
        data[9..11].copy_from_slice(&2u16.to_le_bytes());
        // num_outline_vertices at offset 11
        data[11..15].copy_from_slice(&4u32.to_le_bytes());

        let spans = vec![
            FieldSpan::new(0, 1),   // layer
            FieldSpan::new(1, 2),   // flags
            FieldSpan::new(3, 2),   // net
            FieldSpan::new(5, 2),   // polygon_ref
            FieldSpan::new(7, 2),   // component_ref
            FieldSpan::new(9, 2),   // hole_count
            FieldSpan::new(11, 4),  // num_outline_vertices
        ];

        RecordOrigin::Binary(BinaryOrigin::with_spans(data, spans))
    }

    #[test]
    fn region_read_from_field_spans() {
        let origin = make_test_region_origin();
        let rec = PcbRegionRecord::from_origin(origin);

        assert_eq!(rec.layer(), 1);
        assert_eq!(rec.flags(), 0x0001);
        assert_eq!(rec.net(), 7);
        assert_eq!(rec.polygon_ref(), 0);
        assert_eq!(rec.component_ref(), 3);
        assert_eq!(rec.hole_count(), 2);
        assert_eq!(rec.num_outline_vertices(), 4);
    }

    #[test]
    fn region_write_via_field_spans() {
        let origin = make_test_region_origin();
        let mut rec = PcbRegionRecord::from_origin(origin);

        rec.set_layer(32); // BottomLayer
        assert_eq!(rec.layer(), 32);

        rec.set_net(10);
        assert_eq!(rec.net(), 10);

        rec.set_hole_count(5);
        assert_eq!(rec.hole_count(), 5);
    }
}
