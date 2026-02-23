use altium_format_types::V7Layer;

use crate::Result;
use crate::binary_io::BinaryReader;
use crate::pcblib::PcbFill;
use crate::pcblib::primitives::common::parse_common_header;

/// Parses a PcbFill primitive from its single subrecord (37 or 50 bytes).
///
/// Core layout (always present, 37 bytes):
///   0-12:  common header (13 bytes)
///   13-20: corner1 (CoordPoint, 8 bytes)
///   21-28: corner2 (CoordPoint, 8 bytes)
///   29-36: rotation (f64, 8 bytes)
///
/// Extended (AD26+, 50 bytes):
///   37:    user_routed (u8→bool, 1 byte)
///   38-41: union_index (i32, 4 bytes)
///   42-45: v7_layer (u32→V7Layer, 4 bytes)
///   46-49: keepout_restrictions (i32, 4 bytes)
pub(crate) fn parse_fill(data: &[u8]) -> Result<PcbFill> {
    let mut reader = BinaryReader::new(data);
    let common = parse_common_header(&mut reader)?;
    let corner1 = reader.read_coord_point()?;
    let corner2 = reader.read_coord_point()?;
    let rotation = reader.read_f64_le()?;

    // Extended fields present in AD26+ format (13 extra bytes).
    let (user_routed, union_index, v7_layer, keepout_restrictions) = if reader.remaining() > 0 {
        let user_routed = reader.read_u8()? != 0;
        let union_index = reader.read_i32_le()?;
        let v7_layer = V7Layer::new(reader.read_u32_le()?);
        let keepout_restrictions = reader.read_i32_le()?;
        (user_routed, union_index, v7_layer, keepout_restrictions)
    } else {
        (false, 0, V7Layer::default(), 0)
    };

    reader.assert_exhausted()?;
    Ok(PcbFill {
        common,
        corner1,
        corner2,
        rotation,
        user_routed,
        union_index,
        v7_layer,
        keepout_restrictions,
        unique_id: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AltiumFormatError;
    use crate::binary_io::BinaryWriter;
    use altium_format_types::{Coord, CoordPoint};

    fn write_common_header(w: &mut BinaryWriter) {
        w.write_u8(1);
        w.write_u8(0);
        w.write_u16_le(0);
        w.write_i32_le(-1);
        w.write_u16_le(0xFFFF);
        w.write_u16_le(0);
        w.write_u8(0);
    }

    #[test]
    fn parse_fill_legacy_37_bytes() {
        let mut w = BinaryWriter::new();
        write_common_header(&mut w);
        w.write_coord_point(CoordPoint::new(
            Coord::from_internal(10_000),
            Coord::from_internal(10_000),
        ));
        w.write_coord_point(CoordPoint::new(
            Coord::from_internal(100_000),
            Coord::from_internal(100_000),
        ));
        w.write_f64_le(0.0); // rotation = 0
        let data = w.finish();
        assert_eq!(data.len(), 37);
        let fill = parse_fill(&data).unwrap();
        assert_eq!(fill.corner1.x.to_internal(), 10_000);
        assert_eq!(fill.corner2.x.to_internal(), 100_000);
        assert_eq!(fill.rotation, 0.0);
        // Extended fields default when not present
        assert!(!fill.user_routed);
        assert_eq!(fill.union_index, 0);
        assert_eq!(fill.v7_layer.raw(), 0);
        assert_eq!(fill.keepout_restrictions, 0);
    }

    #[test]
    fn parse_fill_ad26_50_bytes() {
        let mut w = BinaryWriter::new();
        write_common_header(&mut w);
        w.write_coord_point(CoordPoint::new(
            Coord::from_internal(0),
            Coord::from_internal(0),
        ));
        w.write_coord_point(CoordPoint::new(
            Coord::from_internal(0),
            Coord::from_internal(0),
        ));
        w.write_f64_le(90.0);
        // AD26+ extension (13 bytes)
        w.write_u8(0); // user_routed = false
        w.write_i32_le(0); // union_index = 0
        w.write_u32_le(0x01030006); // v7_layer
        w.write_i32_le(0); // keepout_restrictions = 0
        let data = w.finish();
        assert_eq!(data.len(), 50);
        let fill = parse_fill(&data).unwrap();
        assert_eq!(fill.rotation, 90.0);
        assert!(!fill.user_routed);
        assert_eq!(fill.union_index, 0);
        assert_eq!(fill.v7_layer.raw(), 0x01030006);
        assert_eq!(fill.keepout_restrictions, 0);
    }

    #[test]
    fn parse_fill_ad26_keepout_restrictions() {
        let mut w = BinaryWriter::new();
        write_common_header(&mut w);
        w.write_coord_point(CoordPoint::new(
            Coord::from_internal(0),
            Coord::from_internal(0),
        ));
        w.write_coord_point(CoordPoint::new(
            Coord::from_internal(0),
            Coord::from_internal(0),
        ));
        w.write_f64_le(0.0);
        w.write_u8(0); // user_routed
        w.write_i32_le(0); // union_index
        w.write_u32_le(0x0103000d); // v7_layer
        w.write_i32_le(0x1F); // keepout_restrictions = 31
        let data = w.finish();
        assert_eq!(data.len(), 50);
        let fill = parse_fill(&data).unwrap();
        assert_eq!(fill.v7_layer.raw(), 0x0103000d);
        assert_eq!(fill.keepout_restrictions, 0x1F);
    }

    #[test]
    fn parse_fill_with_extra_trailing_bytes_errors() {
        let mut w = BinaryWriter::new();
        write_common_header(&mut w);
        w.write_coord_point(CoordPoint::new(
            Coord::from_internal(0),
            Coord::from_internal(0),
        ));
        w.write_coord_point(CoordPoint::new(
            Coord::from_internal(0),
            Coord::from_internal(0),
        ));
        w.write_f64_le(0.0);
        w.write_u8(0);
        w.write_i32_le(0);
        w.write_u32_le(0);
        w.write_i32_le(0);
        w.write_u8(0xAA); // extra trailing byte
        let data = w.finish();
        assert_eq!(data.len(), 51);
        let result = parse_fill(&data);
        assert!(matches!(
            result,
            Err(AltiumFormatError::UnexpectedTrailingData { .. })
        ));
    }

    #[test]
    fn truncated_fill_returns_error() {
        let data = [0u8; 4];
        let result = parse_fill(&data);
        assert!(matches!(
            result,
            Err(AltiumFormatError::BinaryReadPastEnd { .. })
        ));
    }
}
