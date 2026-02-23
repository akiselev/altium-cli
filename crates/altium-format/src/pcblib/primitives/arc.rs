use crate::binary_io::BinaryReader;
use crate::pcblib::primitives::common::parse_common_header;
use crate::pcblib::PcbArc;
use crate::Result;

pub(crate) fn parse_arc(data: &[u8]) -> Result<PcbArc> {
    let mut reader = BinaryReader::new(data);
    let common = parse_common_header(&mut reader)?;
    let center = reader.read_coord_point()?;
    let radius = reader.read_coord()?;
    let start_angle = reader.read_f64_le()?;
    let end_angle = reader.read_f64_le()?;
    let width = reader.read_coord()?;
    reader.assert_exhausted()?;
    Ok(PcbArc {
        common,
        center,
        radius,
        start_angle,
        end_angle,
        width,
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

    #[test]
    fn parse_arc_legacy_45_bytes() {
        let mut w = BinaryWriter::new();
        write_common_header(&mut w);
        // center (8 bytes), radius (4), start_angle (8), end_angle (8), width (4) = 32
        // 13 (common) + 32 = 45 bytes total
        w.write_coord_point(CoordPoint::new(
            Coord::from_internal(100_000),
            Coord::from_internal(200_000),
        ));
        w.write_coord(Coord::from_internal(50_000)); // radius
        w.write_f64_le(0.0);   // start_angle
        w.write_f64_le(360.0); // end_angle
        w.write_coord(Coord::from_internal(10_000)); // width
        let data = w.finish();
        assert_eq!(data.len(), 45);
        let arc = parse_arc(&data).unwrap();
        assert_eq!(arc.center.x.to_internal(), 100_000);
        assert_eq!(arc.center.y.to_internal(), 200_000);
        assert_eq!(arc.radius.to_internal(), 50_000);
        assert_eq!(arc.start_angle, 0.0);
        assert_eq!(arc.end_angle, 360.0);
        assert_eq!(arc.width.to_internal(), 10_000);
        assert!(arc.unique_id.is_none());
    }

    #[test]
    fn parse_arc_ad26_58_bytes_errors_on_trailing() {
        let mut w = BinaryWriter::new();
        write_common_header(&mut w);
        w.write_coord_point(CoordPoint::new(
            Coord::from_internal(0),
            Coord::from_internal(0),
        ));
        w.write_coord(Coord::from_internal(25_000));
        w.write_f64_le(45.0);
        w.write_f64_le(135.0);
        w.write_coord(Coord::from_internal(5_000));
        // 13 trailing bytes to reach 58 total
        w.write_bytes(&[0u8; 13]);
        let data = w.finish();
        assert_eq!(data.len(), 58);
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
