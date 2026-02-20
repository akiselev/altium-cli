//! PCB Connection record type for the v2 API.
//!
//! Connection records are stored in `Connections6/Data` (object ID 7) and are
//! framed as a single binary subrecord:
//! `[u8 type=7][u32 len][len bytes payload]`.
//!
//! Observed AD26 layout (minimum 47 bytes):
//! - 13-byte common header
//! - start/end coordinates and width
//! - layer range and layer-enum indices

use crate::binary_helpers::PcbCommonHeader;
use crate::coord::PcbCoord;
use altium_format_derive::altium_record;

#[altium_record(kind = "pcb", object_id = Connection, codec = "binary",
    parse_fn = "parse_connection", serialize_fn = "serialize_connection")]
pub struct PcbConnectionRecord {
    #[altium(header)]
    header: PcbCommonHeader,
    start_x: PcbCoord,
    start_y: PcbCoord,
    end_x: PcbCoord,
    end_y: PcbCoord,
    width: PcbCoord,
    from_layer: u8,
    to_layer: u8,
    from_layer_enum_index: i32,
    to_layer_enum_index: i32,
    layer_enum_index: i32,
}

/// Parse connection payload from raw binary bytes.
pub(crate) fn parse_connection(
    data: &[u8],
) -> crate::Result<crate::backing_store::RecordOrigin> {
    use crate::error::AltiumError;
    use crate::backing_store::{BinaryOrigin, FieldSpan};

    if data.len() < 47 {
        return Err(AltiumError::Parse(format!(
            "connection data too short: {} bytes (need >= 47 for AD26)",
            data.len()
        )));
    }

    let spans = vec![
        FieldSpan::new(0, 13), // 0: header
        FieldSpan::new(13, 4), // 1: start_x
        FieldSpan::new(17, 4), // 2: start_y
        FieldSpan::new(21, 4), // 3: end_x
        FieldSpan::new(25, 4), // 4: end_y
        FieldSpan::new(29, 4), // 5: width
        FieldSpan::new(33, 1), // 6: from_layer
        FieldSpan::new(34, 1), // 7: to_layer
        FieldSpan::new(35, 4), // 8: from_layer_enum_index
        FieldSpan::new(39, 4), // 9: to_layer_enum_index
        FieldSpan::new(43, 4), // 10: layer_enum_index
    ];

    Ok(crate::backing_store::RecordOrigin::Binary(
        BinaryOrigin::with_spans(data.to_vec(), spans),
    ))
}

#[allow(dead_code)]
fn serialize_connection(origin: &crate::backing_store::BinaryOrigin) -> crate::Result<Vec<u8>> {
    Ok(origin.raw_block.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coord::AltiumCoord;

    #[test]
    fn connection_read_write() {
        let mut data = vec![0u8; 47];
        data[13..17].copy_from_slice(&100_000i32.to_le_bytes());
        data[17..21].copy_from_slice(&200_000i32.to_le_bytes());
        data[21..25].copy_from_slice(&300_000i32.to_le_bytes());
        data[25..29].copy_from_slice(&400_000i32.to_le_bytes());
        data[29..33].copy_from_slice(&10_000i32.to_le_bytes());
        data[33] = 1;
        data[34] = 32;
        data[35..39].copy_from_slice(&11i32.to_le_bytes());
        data[39..43].copy_from_slice(&22i32.to_le_bytes());
        data[43..47].copy_from_slice(&33i32.to_le_bytes());

        let mut rec = PcbConnectionRecord::from_origin(parse_connection(&data).unwrap());
        assert_eq!(rec.start_x().to_raw(), 100_000);
        assert_eq!(rec.end_y().to_raw(), 400_000);
        assert_eq!(rec.from_layer(), 1);
        assert_eq!(rec.to_layer(), 32);
        assert_eq!(rec.layer_enum_index(), 33);

        rec.set_width(PcbCoord::from_raw(55_000));
        assert_eq!(rec.width().to_raw(), 55_000);
    }
}
