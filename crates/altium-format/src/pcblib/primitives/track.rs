use altium_format_types::V7Layer;

use crate::binary_io::BinaryReader;
use crate::pcblib::primitives::common::parse_common_header;
use crate::pcblib::PcbTrack;
use crate::Result;

/// Parses a PcbTrack primitive from its single subrecord (35-49 bytes).
///
/// Core layout (always present, 35 bytes):
///   0-12:  common header (13 bytes)
///   13-20: start (CoordPoint, 8 bytes)
///   21-28: end (CoordPoint, 8 bytes)
///   29-32: width (Coord, 4 bytes)
///   33-34: subpoly_index (u16, 2 bytes)
///
/// Extended (AD26+, 49 bytes):
///   35:    user_routed (u8→bool, 1 byte)
///   36-39: union_index (i32, 4 bytes)
///   40-43: v7_layer (u32→V7Layer, 4 bytes)
///   44-47: keepout_restrictions (i32, 4 bytes)
///   48:    subnet_jumper (u8→bool, 1 byte)
pub(crate) fn parse_track(data: &[u8]) -> Result<PcbTrack> {
    let mut reader = BinaryReader::new(data);
    let common = parse_common_header(&mut reader)?;
    let start = reader.read_coord_point()?;
    let end = reader.read_coord_point()?;
    let width = reader.read_coord()?;
    let subpoly_index = reader.read_u16_le()?;

    // Extended fields present in AD26+ format (14 extra bytes).
    let (user_routed, union_index, v7_layer, keepout_restrictions, subnet_jumper) =
        if reader.remaining() > 0 {
            let user_routed = reader.read_u8()? != 0;
            let union_index = reader.read_i32_le()?;
            let v7_layer = V7Layer::new(reader.read_u32_le()?);
            let keepout_restrictions = reader.read_i32_le()?;
            let subnet_jumper = reader.read_u8()? != 0;
            (user_routed, union_index, v7_layer, keepout_restrictions, subnet_jumper)
        } else {
            (false, 0, V7Layer::default(), 0, false)
        };

    reader.assert_exhausted()?;
    Ok(PcbTrack {
        common,
        start,
        end,
        width,
        subpoly_index,
        user_routed,
        union_index,
        v7_layer,
        keepout_restrictions,
        subnet_jumper,
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
        w.write_u8(1);
        w.write_u8(0);
        w.write_u16_le(0);
        w.write_i32_le(-1);
        w.write_u16_le(0xFFFF);
        w.write_u16_le(0);
        w.write_u8(0);
    }

    fn make_track_core(w: &mut BinaryWriter) {
        write_common_header(w);
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
    }

    #[test]
    fn parse_track_legacy_35_bytes() {
        let mut w = BinaryWriter::new();
        make_track_core(&mut w);
        let data = w.finish();
        assert_eq!(data.len(), 35);
        let track = parse_track(&data).unwrap();
        assert_eq!(track.start.x.to_internal(), 10_000);
        assert_eq!(track.start.y.to_internal(), 20_000);
        assert_eq!(track.end.x.to_internal(), 30_000);
        assert_eq!(track.end.y.to_internal(), 40_000);
        assert_eq!(track.width.to_internal(), 5_000);
        assert_eq!(track.subpoly_index, 0xFFFF);
        // Extended fields default when not present
        assert!(!track.user_routed);
        assert_eq!(track.union_index, 0);
        assert_eq!(track.v7_layer.raw(), 0);
        assert_eq!(track.keepout_restrictions, 0);
        assert!(!track.subnet_jumper);
    }

    #[test]
    fn parse_track_ad26_49_bytes() {
        let mut w = BinaryWriter::new();
        make_track_core(&mut w);
        w.write_u8(1);             // user_routed = true
        w.write_i32_le(42);        // union_index
        w.write_u32_le(0x0102_0021); // v7_layer (TopOverlay)
        w.write_i32_le(7);         // keepout_restrictions
        w.write_u8(1);             // subnet_jumper = true
        let data = w.finish();
        assert_eq!(data.len(), 49);
        let track = parse_track(&data).unwrap();
        assert_eq!(track.start.x.to_internal(), 10_000);
        assert_eq!(track.start.y.to_internal(), 20_000);
        assert_eq!(track.end.x.to_internal(), 30_000);
        assert_eq!(track.end.y.to_internal(), 40_000);
        assert_eq!(track.width.to_internal(), 5_000);
        assert_eq!(track.subpoly_index, 0xFFFF);
        assert!(track.user_routed);
        assert_eq!(track.union_index, 42);
        assert_eq!(track.v7_layer.raw(), 0x0102_0021);
        assert_eq!(track.keepout_restrictions, 7);
        assert!(track.subnet_jumper);
        assert!(track.unique_id.is_none());
    }

    #[test]
    fn parse_track_with_trailing_bytes_errors() {
        let mut w = BinaryWriter::new();
        make_track_core(&mut w);
        w.write_u8(0);             // user_routed
        w.write_i32_le(0);         // union_index
        w.write_u32_le(0);         // v7_layer
        w.write_i32_le(0);         // keepout_restrictions
        w.write_u8(0);             // subnet_jumper
        w.write_u8(0xAA);          // extra trailing byte
        let data = w.finish();
        assert_eq!(data.len(), 50);
        let result = parse_track(&data);
        assert!(matches!(result, Err(AltiumFormatError::UnexpectedTrailingData { .. })));
    }

    #[test]
    fn truncated_track_returns_error() {
        let data = [0u8; 5];
        let result = parse_track(&data);
        assert!(matches!(result, Err(AltiumFormatError::BinaryReadPastEnd { .. })));
    }
}
