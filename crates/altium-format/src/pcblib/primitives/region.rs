use altium_format_types::RegionKind;

use crate::binary_io::BinaryReader;
use crate::pcblib::primitives::common::parse_common_header;
use crate::pcblib::PcbRegion;
use crate::{AltiumFormatError, Result};

pub(crate) fn parse_region(data: &[u8]) -> Result<PcbRegion> {
    let mut reader = BinaryReader::new(data);
    let common = parse_common_header(&mut reader)?;
    let kind = RegionKind::try_from(reader.read_u8()?)?;
    let vertex_count_raw = reader.read_i32_le()?;
    if vertex_count_raw < 0 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "Region.vertex_count".to_owned(),
            detail: format!("vertex_count must be >= 0, got {}", vertex_count_raw),
        });
    }
    let vertex_count = vertex_count_raw as usize;
    let bytes_needed = vertex_count * 8; // each CoordPoint is 4+4 bytes
    if reader.remaining() < bytes_needed {
        return Err(AltiumFormatError::BinaryReadPastEnd {
            offset: reader.position(),
            needed: bytes_needed,
            available: reader.remaining(),
        });
    }
    let mut vertices = Vec::with_capacity(vertex_count);
    for _ in 0..vertex_count {
        vertices.push(reader.read_coord_point()?);
    }
    let trailing_bytes = reader.read_remaining().to_vec();
    Ok(PcbRegion {
        common,
        kind,
        vertices,
        unique_id: None,
        trailing_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use altium_format_types::{Coord, CoordPoint};
    use crate::binary_io::BinaryWriter;
    use crate::AltiumFormatError;

    fn write_common_header(w: &mut BinaryWriter) {
        w.write_u8(1);      // layer
        w.write_u8(0);      // pad_byte
        w.write_u16_le(0);  // flags
        w.write_i32_le(-1); // net_index
        w.write_u16_le(0xFFFF); // polygon_index
        w.write_u16_le(0);  // component_index
        w.write_u8(0);      // unknown
    }

    #[test]
    fn parse_region_no_vertices() {
        let mut w = BinaryWriter::new();
        write_common_header(&mut w);
        w.write_u8(0);       // kind = Copper
        w.write_i32_le(0);   // vertex_count = 0
        let data = w.finish();
        let region = parse_region(&data).unwrap();
        assert_eq!(region.kind, RegionKind::Copper);
        assert!(region.vertices.is_empty());
        assert!(region.trailing_bytes.is_empty());
    }

    #[test]
    fn parse_region_three_vertices() {
        let mut w = BinaryWriter::new();
        write_common_header(&mut w);
        w.write_u8(1);        // kind = Cutout
        w.write_i32_le(3);    // vertex_count = 3
        w.write_coord_point(CoordPoint::new(
            Coord::from_internal(0),
            Coord::from_internal(0),
        ));
        w.write_coord_point(CoordPoint::new(
            Coord::from_internal(10_000),
            Coord::from_internal(0),
        ));
        w.write_coord_point(CoordPoint::new(
            Coord::from_internal(10_000),
            Coord::from_internal(10_000),
        ));
        let data = w.finish();
        let region = parse_region(&data).unwrap();
        assert_eq!(region.kind, RegionKind::Cutout);
        assert_eq!(region.vertices.len(), 3);
        assert_eq!(region.vertices[0].x.to_internal(), 0);
        assert_eq!(region.vertices[1].x.to_internal(), 10_000);
        assert_eq!(region.vertices[2].y.to_internal(), 10_000);
        assert!(region.trailing_bytes.is_empty());
    }

    #[test]
    fn parse_region_stores_trailing_bytes() {
        let mut w = BinaryWriter::new();
        write_common_header(&mut w);
        w.write_u8(0); // kind = Copper
        w.write_i32_le(0); // vertex_count = 0
        w.write_bytes(&[0xAA, 0xBB, 0xCC]); // trailing bytes
        let data = w.finish();
        let region = parse_region(&data).unwrap();
        assert_eq!(region.trailing_bytes, &[0xAA, 0xBB, 0xCC]);
    }

    #[test]
    fn parse_region_negative_vertex_count_returns_error() {
        let mut w = BinaryWriter::new();
        write_common_header(&mut w);
        w.write_u8(0);
        w.write_i32_le(-1); // negative vertex_count
        let data = w.finish();
        let result = parse_region(&data);
        assert!(matches!(result, Err(AltiumFormatError::InvalidParamValue { .. })));
    }

    #[test]
    fn truncated_region_returns_error() {
        let data = [0u8; 5];
        let result = parse_region(&data);
        assert!(matches!(result, Err(AltiumFormatError::BinaryReadPastEnd { .. })));
    }

    #[test]
    fn parse_region_truncated_vertex_data_returns_error() {
        let mut w = BinaryWriter::new();
        write_common_header(&mut w);
        w.write_u8(0);      // kind
        w.write_i32_le(2);  // claims 2 vertices but only provides 4 bytes (too short for 16 bytes needed)
        w.write_bytes(&[0u8; 4]);
        let data = w.finish();
        let result = parse_region(&data);
        assert!(matches!(result, Err(AltiumFormatError::BinaryReadPastEnd { .. })));
    }
}
