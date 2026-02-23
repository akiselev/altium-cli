use altium_format_types::V7Layer;

use crate::binary_io::BinaryReader;
use crate::pcblib::primitives::common::parse_common_header;
use crate::pcblib::PcbArc;
use crate::Result;

/// Parses a PcbArc primitive from its single subrecord (56-60 bytes).
///
/// Core layout (always present, 56 bytes):
///   0-12:  common header (13 bytes)
///   13-20: center (CoordPoint, 8 bytes)
///   21-24: radius (Coord, 4 bytes)
///   25-32: start_angle (f64, 8 bytes)
///   33-40: end_angle (f64, 8 bytes)
///   41-44: width (Coord, 4 bytes)
///   45-46: subpoly_index (u16, 2 bytes)
///   47:    user_routed (u8→bool, 1 byte)
///   48-51: union_index (i32, 4 bytes)
///   52-55: v7_layer (u32→V7Layer, 4 bytes)
///
/// Extended (if 60 bytes):
///   56-59: keepout_restrictions (i32, 4 bytes)
pub(crate) fn parse_arc(data: &[u8]) -> Result<PcbArc> {
    let mut reader = BinaryReader::new(data);
    let common = parse_common_header(&mut reader)?;
    let center = reader.read_coord_point()?;
    let radius = reader.read_coord()?;
    let start_angle = reader.read_f64_le()?;
    let end_angle = reader.read_f64_le()?;
    let width = reader.read_coord()?;
    let subpoly_index = reader.read_u16_le()?;
    let user_routed = reader.read_u8()? != 0;
    let union_index = reader.read_i32_le()?;
    let v7_layer = V7Layer::new(reader.read_u32_le()?);

    // keepout_restrictions is present in newer format versions (60 bytes)
    let keepout_restrictions = if reader.remaining() >= 4 {
        reader.read_i32_le()?
    } else {
        0
    };

    reader.assert_exhausted()?;
    Ok(PcbArc {
        common,
        center,
        radius,
        start_angle,
        end_angle,
        width,
        subpoly_index,
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

    fn make_arc_core(w: &mut BinaryWriter) {
        write_common_header(w);
        w.write_coord_point(CoordPoint::new(
            Coord::from_internal(100_000),
            Coord::from_internal(200_000),
        ));
        w.write_coord(Coord::from_internal(50_000)); // radius
        w.write_f64_le(0.0);   // start_angle
        w.write_f64_le(360.0); // end_angle
        w.write_coord(Coord::from_internal(10_000)); // width
        w.write_u16_le(0xFFFF); // subpoly_index
        w.write_u8(1);          // user_routed = true
        w.write_i32_le(0);      // union_index
        w.write_u32_le(0x0102_001d); // v7_layer (Mechanical1)
    }

    #[test]
    fn parse_arc_60_bytes() {
        let mut w = BinaryWriter::new();
        make_arc_core(&mut w);
        w.write_i32_le(0); // keepout_restrictions
        let data = w.finish();
        assert_eq!(data.len(), 60);
        let arc = parse_arc(&data).unwrap();
        assert_eq!(arc.center.x.to_internal(), 100_000);
        assert_eq!(arc.center.y.to_internal(), 200_000);
        assert_eq!(arc.radius.to_internal(), 50_000);
        assert_eq!(arc.start_angle, 0.0);
        assert_eq!(arc.end_angle, 360.0);
        assert_eq!(arc.width.to_internal(), 10_000);
        assert_eq!(arc.subpoly_index, 0xFFFF);
        assert!(arc.user_routed);
        assert_eq!(arc.union_index, 0);
        assert_eq!(arc.v7_layer.raw(), 0x0102_001d);
        assert_eq!(arc.keepout_restrictions, 0);
        assert!(arc.unique_id.is_none());
    }

    #[test]
    fn parse_arc_56_bytes_no_keepout() {
        let mut w = BinaryWriter::new();
        make_arc_core(&mut w);
        // No keepout_restrictions
        let data = w.finish();
        assert_eq!(data.len(), 56);
        let arc = parse_arc(&data).unwrap();
        assert_eq!(arc.center.x.to_internal(), 100_000);
        assert_eq!(arc.keepout_restrictions, 0); // defaults to 0
    }

    #[test]
    fn parse_arc_with_trailing_bytes_errors() {
        let mut w = BinaryWriter::new();
        make_arc_core(&mut w);
        w.write_i32_le(0);
        let mut data = w.finish();
        data.extend_from_slice(&[0xAA, 0xBB]);
        let result = parse_arc(&data);
        assert!(matches!(result, Err(AltiumFormatError::UnexpectedTrailingData { .. })));
    }

    #[test]
    fn truncated_arc_returns_error() {
        let data = [0u8; 10]; // too short for common header (13 bytes)
        let result = parse_arc(&data);
        assert!(matches!(result, Err(AltiumFormatError::BinaryReadPastEnd { .. })));
    }
}
