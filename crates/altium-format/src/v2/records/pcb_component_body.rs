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

/// Parse component body data from the raw binary block.
///
/// Structurally identical to Region (same binary header + parametric +
/// vertices), just with a different object type ID (12 vs 11).
fn parse_component_body(data: &[u8]) -> crate::Result<crate::v2::backing_store::RecordOrigin> {
    use crate::v2::backing_store::{BinaryOrigin, FieldSpan};
    use crate::error::AltiumError;

    if data.len() < 22 {
        return Err(AltiumError::Parse(format!(
            "component body data too short: {} bytes (need >= 22)", data.len()
        )));
    }

    // Same layout as Region: header(13) + extra(5) + hole_count(2) + padding(2)
    let spans = vec![
        FieldSpan::new(0, 1),   // 0: layer
        FieldSpan::new(1, 2),   // 1: flags
        FieldSpan::new(3, 2),   // 2: net
        FieldSpan::new(5, 2),   // 3: polygon_ref
        FieldSpan::new(7, 2),   // 4: component_ref
        FieldSpan::new(18, 2),  // 5: hole_count
        find_body_outline_vertex_count_span(data), // 6: num_outline_vertices
    ];

    Ok(crate::v2::backing_store::RecordOrigin::Binary(
        BinaryOrigin::with_spans(data.to_vec(), spans),
    ))
}

/// Find the FieldSpan for num_outline_vertices within component body data.
fn find_body_outline_vertex_count_span(data: &[u8]) -> crate::v2::backing_store::FieldSpan {
    use crate::v2::backing_store::FieldSpan;

    // Skip: 13 (header) + 5 (extra) + 2 (hole_count) + 2 (padding) = 22
    let mut offset = 22usize;

    // Read prop_len and skip properties
    if offset + 4 <= data.len() {
        let prop_len = u32::from_le_bytes(
            data[offset..offset + 4].try_into().unwrap_or([0; 4]),
        ) as usize;
        offset += 4 + prop_len;
    }

    // Now at num_outline_vertices (u32)
    if offset + 4 <= data.len() {
        FieldSpan::new(offset, 4)
    } else {
        FieldSpan::new(0, 1)
    }
}

/// Serialize component body data back to binary.
fn serialize_component_body(
    origin: &crate::v2::backing_store::BinaryOrigin,
) -> crate::Result<Vec<u8>> {
    Ok(origin.raw_block.clone())
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
