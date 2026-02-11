//! PCB ComponentBody record type for the v2 API.
//!
//! Structurally identical to Region (same binary header, parametric
//! properties, and vertex format) but with a different object type ID (12)
//! and 3D-specific properties in the parametric block (STANDOFFHEIGHT,
//! OVERALLHEIGHT, BODYPROJECTION, BODYCOLOR3D, BODYOPACITY3D, etc.).
//!
//! Uses custom parse/serialize functions stubbed for Phase 4.

use altium_format_derive::altium_record;

#[altium_record(kind = "pcb", object_id = ComponentBody, codec = "binary",
    parse_fn = "parse_component_body", serialize_fn = "serialize_component_body")]
pub struct PcbComponentBodyRecord {
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
fn parse_component_body(_data: &[u8]) -> crate::Result<crate::v2::backing_store::RecordOrigin> {
    todo!("Complex component body parsing -- will be implemented in Phase 4")
}

#[allow(dead_code)]
fn serialize_component_body(
    _origin: &crate::v2::backing_store::BinaryOrigin,
) -> crate::Result<Vec<u8>> {
    todo!("Complex component body serialization -- will be implemented in Phase 4")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::backing_store::{BinaryOrigin, FieldSpan, RecordOrigin};

    fn make_test_component_body_origin() -> RecordOrigin {
        let mut data = vec![0u8; 64];

        // layer at offset 0
        data[0] = 1;
        // flags at offset 1
        data[1..3].copy_from_slice(&0x0001u16.to_le_bytes());
        // net at offset 3
        data[3..5].copy_from_slice(&0u16.to_le_bytes());
        // polygon_ref at offset 5
        data[5..7].copy_from_slice(&0u16.to_le_bytes());
        // component_ref at offset 7
        data[7..9].copy_from_slice(&5u16.to_le_bytes());
        // hole_count at offset 9
        data[9..11].copy_from_slice(&0u16.to_le_bytes());
        // num_outline_vertices at offset 11
        data[11..15].copy_from_slice(&8u32.to_le_bytes());

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
    fn component_body_read_from_field_spans() {
        let origin = make_test_component_body_origin();
        let rec = PcbComponentBodyRecord::from_origin(origin);

        assert_eq!(rec.layer(), 1);
        assert_eq!(rec.flags(), 0x0001);
        assert_eq!(rec.net(), 0);
        assert_eq!(rec.component_ref(), 5);
        assert_eq!(rec.hole_count(), 0);
        assert_eq!(rec.num_outline_vertices(), 8);
    }

    #[test]
    fn component_body_write_via_field_spans() {
        let origin = make_test_component_body_origin();
        let mut rec = PcbComponentBodyRecord::from_origin(origin);

        rec.set_layer(32);
        assert_eq!(rec.layer(), 32);

        rec.set_component_ref(10);
        assert_eq!(rec.component_ref(), 10);
    }
}
