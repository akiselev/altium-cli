use crate::binary_io::BinaryReader;
use crate::pcblib::primitives::common::parse_common_header;
use crate::pcblib::PcbFill;
use crate::Result;

pub(crate) fn parse_fill(data: &[u8]) -> Result<PcbFill> {
    let mut reader = BinaryReader::new(data);
    let common = parse_common_header(&mut reader)?;
    let corner1 = reader.read_coord_point()?;
    let corner2 = reader.read_coord_point()?;
    let rotation = reader.read_f64_le()?;
    let trailing_bytes = reader.read_remaining().to_vec();
    Ok(PcbFill {
        common,
        corner1,
        corner2,
        rotation,
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
        // corner1 (8), corner2 (8), rotation (8) = 24
        // 13 + 24 = 37 bytes total
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
        assert!(fill.trailing_bytes.is_empty());
    }

    #[test]
    fn parse_fill_ad26_50_bytes_stores_trailing() {
        let mut w = BinaryWriter::new();
        write_common_header(&mut w);
        w.write_coord_point(CoordPoint::new(Coord::from_internal(0), Coord::from_internal(0)));
        w.write_coord_point(CoordPoint::new(Coord::from_internal(0), Coord::from_internal(0)));
        w.write_f64_le(90.0);
        // 13 trailing bytes to reach 50
        w.write_bytes(&[0u8; 13]);
        let data = w.finish();
        assert_eq!(data.len(), 50);
        let fill = parse_fill(&data).unwrap();
        assert_eq!(fill.rotation, 90.0);
        assert_eq!(fill.trailing_bytes.len(), 13);
    }

    #[test]
    fn truncated_fill_returns_error() {
        let data = [0u8; 4];
        let result = parse_fill(&data);
        assert!(matches!(result, Err(AltiumFormatError::BinaryReadPastEnd { .. })));
    }
}
