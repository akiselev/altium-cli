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
pub(crate) fn parse_component_body(
    data: &[u8],
) -> crate::Result<crate::backing_store::RecordOrigin> {
    use crate::error::AltiumError;
    use crate::backing_store::{BinaryOrigin, FieldSpan};

    if data.len() < 22 {
        return Err(AltiumError::Parse(format!(
            "component body data too short: {} bytes (need >= 22)",
            data.len()
        )));
    }

    // Same layout family as Region: header(13) + extension(5) + prop_len + props + ...
    let spans = vec![
        FieldSpan::new(0, 1),                       // 0: layer
        FieldSpan::new(1, 2),                       // 1: flags
        FieldSpan::new(3, 2),                       // 2: net
        FieldSpan::new(5, 2),                       // 3: polygon_ref
        FieldSpan::new(7, 2),                       // 4: component_ref
        FieldSpan::new(14, 2),                      // 5: hole_count
        find_body_outline_vertex_count_span(data)?, // 6: num_outline_vertices
    ];

    Ok(crate::backing_store::RecordOrigin::Binary(
        BinaryOrigin::with_spans(data.to_vec(), spans),
    ))
}

/// Find the FieldSpan for num_outline_vertices within component body data.
fn find_body_outline_vertex_count_span(
    data: &[u8],
) -> crate::Result<crate::backing_store::FieldSpan> {
    use crate::error::AltiumError;
    use crate::backing_store::FieldSpan;

    // Skip common header + extension header to reach prop_len.
    let mut offset = 18usize;

    if offset + 4 > data.len() {
        return Err(AltiumError::Parse(
            "component body data missing prop_len field".to_string(),
        ));
    }

    // Read prop_len and skip properties
    let prop_len =
        u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap_or([0; 4])) as usize;
    offset += 4;
    if offset + prop_len > data.len() {
        return Err(AltiumError::Parse(format!(
            "component body data prop_len={} exceeds payload",
            prop_len
        )));
    }
    offset += prop_len;

    // Now at num_outline_vertices (u32)
    if offset + 4 > data.len() {
        return Err(AltiumError::Parse(
            "component body data missing num_outline_vertices field".to_string(),
        ));
    }

    Ok(FieldSpan::new(offset, 4))
}

/// Serialize component body data back to binary.
#[allow(dead_code)]
fn serialize_component_body(
    origin: &crate::backing_store::BinaryOrigin,
) -> crate::Result<Vec<u8>> {
    Ok(origin.raw_block.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backing_store::{BinaryOrigin, FieldSpan, RecordOrigin};

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
        // hole_count at extension offset 14
        data[14..16].copy_from_slice(&0u16.to_le_bytes());
        // prop_len at offset 18
        data[18..22].copy_from_slice(&4u32.to_le_bytes());
        // num_outline_vertices after props: 18 + 4 + 4 = 26
        data[26..30].copy_from_slice(&8u32.to_le_bytes());

        let spans = vec![
            FieldSpan::new(0, 1),  // layer
            FieldSpan::new(1, 2),  // flags
            FieldSpan::new(3, 2),  // net
            FieldSpan::new(5, 2),  // polygon_ref
            FieldSpan::new(7, 2),  // component_ref
            FieldSpan::new(14, 2), // hole_count
            FieldSpan::new(26, 4), // num_outline_vertices
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
