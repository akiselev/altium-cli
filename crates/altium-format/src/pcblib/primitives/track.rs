use crate::binary_io::BinaryReader;
use crate::pcblib::primitives::common::parse_common_header;
use crate::pcblib::PcbTrack;
use crate::Result;

pub(crate) fn parse_track(data: &[u8]) -> Result<PcbTrack> {
    let mut reader = BinaryReader::new(data);
    let common = parse_common_header(&mut reader)?;
    let start = reader.read_coord_point()?;
    let end = reader.read_coord_point()?;
    let width = reader.read_coord()?;
    let subpoly_index = reader.read_u16_le()?;
    let trailing_bytes = reader.read_remaining().to_vec();
    Ok(PcbTrack {
        common,
        start,
        end,
        width,
        subpoly_index,
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
    fn parse_track_legacy_35_bytes() {
        let mut w = BinaryWriter::new();
        write_common_header(&mut w);
        // start (8), end (8), width (4), subpoly_index (2) = 22
        // 13 + 22 = 35 bytes total
        w.write_coord_point(CoordPoint::new(
            Coord::from_internal(10_000),
            Coord::from_internal(20_000),
        ));
        w.write_coord_point(CoordPoint::new(
            Coord::from_internal(30_000),
            Coord::from_internal(40_000),
        ));
        w.write_coord(Coord::from_internal(5_000));
        w.write_u16_le(0xFFFF); // subpoly_index = no polygon
        let data = w.finish();
        assert_eq!(data.len(), 35);
        let track = parse_track(&data).unwrap();
        assert_eq!(track.start.x.to_internal(), 10_000);
        assert_eq!(track.start.y.to_internal(), 20_000);
        assert_eq!(track.end.x.to_internal(), 30_000);
        assert_eq!(track.end.y.to_internal(), 40_000);
        assert_eq!(track.width.to_internal(), 5_000);
        assert_eq!(track.subpoly_index, 0xFFFF);
        assert!(track.trailing_bytes.is_empty());
    }

    #[test]
    fn parse_track_ad26_49_bytes_stores_trailing() {
        let mut w = BinaryWriter::new();
        write_common_header(&mut w);
        w.write_coord_point(CoordPoint::new(Coord::from_internal(0), Coord::from_internal(0)));
        w.write_coord_point(CoordPoint::new(Coord::from_internal(0), Coord::from_internal(0)));
        w.write_coord(Coord::from_internal(0));
        w.write_u16_le(0);
        // 14 trailing bytes to reach 49
        w.write_bytes(&[0u8; 14]);
        let data = w.finish();
        assert_eq!(data.len(), 49);
        let track = parse_track(&data).unwrap();
        assert_eq!(track.trailing_bytes.len(), 14);
    }

    #[test]
    fn truncated_track_returns_error() {
        let data = [0u8; 5];
        let result = parse_track(&data);
        assert!(matches!(result, Err(AltiumFormatError::BinaryReadPastEnd { .. })));
    }
}
