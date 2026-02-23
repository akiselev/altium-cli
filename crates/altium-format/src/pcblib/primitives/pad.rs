use altium_format_types::{CoordPoint, PadShape, PadStackMode};

use crate::binary_io::BinaryReader;
use crate::pcblib::primitives::common::parse_common_header;
use crate::pcblib::PcbPad;
use crate::Result;

/// Parses a Pad primitive from a single PcbLib record payload.
///
/// PcbLib uses single-record format (unlike PcbDoc which has 6 subrecords
/// for Pad). The entire pad data is in one contiguous payload.
pub(crate) fn parse_pad(data: &[u8]) -> Result<PcbPad> {
    let mut reader = BinaryReader::new(data);
    let common = parse_common_header(&mut reader)?;
    let location = reader.read_coord_point()?;
    let size_top_x = reader.read_coord()?;
    let size_top_y = reader.read_coord()?;
    let size_mid_x = reader.read_coord()?;
    let size_mid_y = reader.read_coord()?;
    let size_bot_x = reader.read_coord()?;
    let size_bot_y = reader.read_coord()?;
    let hole_size = reader.read_coord()?;
    let shape_top = PadShape::try_from(reader.read_u8()?)?;
    let shape_mid = PadShape::try_from(reader.read_u8()?)?;
    let shape_bot = PadShape::try_from(reader.read_u8()?)?;
    let rotation = reader.read_f64_le()?;
    let is_plated = reader.read_u8()? != 0;
    let stack_mode = PadStackMode::try_from(reader.read_u8()?)?;
    let trailing = reader.read_remaining().to_vec();

    Ok(PcbPad {
        common,
        location,
        size_top: CoordPoint::new(size_top_x, size_top_y),
        size_mid: CoordPoint::new(size_mid_x, size_mid_y),
        size_bot: CoordPoint::new(size_bot_x, size_bot_y),
        hole_size,
        shape_top,
        shape_mid,
        shape_bot,
        rotation,
        is_plated,
        stack_mode,
        unique_id: None,
        subrecord_trailing: [trailing, vec![], vec![], vec![], vec![], vec![]],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use altium_format_types::{Coord, CoordPoint};
    use crate::binary_io::BinaryWriter;
    use crate::AltiumFormatError;

    fn write_common_header(w: &mut BinaryWriter) {
        w.write_u8(74);     // layer = MultiLayer
        w.write_u8(0);      // pad_byte
        w.write_u16_le(0);  // flags
        w.write_i32_le(-1); // net_index
        w.write_u16_le(0xFFFF); // polygon_index
        w.write_u16_le(0);  // component_index
        w.write_u8(0);      // unknown
    }

    fn make_pad_payload() -> Vec<u8> {
        let mut w = BinaryWriter::new();
        write_common_header(&mut w);
        w.write_coord_point(CoordPoint::new(
            Coord::from_internal(50_000),
            Coord::from_internal(75_000),
        ));
        // size_top
        w.write_coord(Coord::from_internal(30_000));
        w.write_coord(Coord::from_internal(30_000));
        // size_mid
        w.write_coord(Coord::from_internal(30_000));
        w.write_coord(Coord::from_internal(30_000));
        // size_bot
        w.write_coord(Coord::from_internal(30_000));
        w.write_coord(Coord::from_internal(30_000));
        w.write_coord(Coord::from_internal(15_000)); // hole_size
        w.write_u8(1); // shape_top = Round
        w.write_u8(1); // shape_mid = Round
        w.write_u8(1); // shape_bot = Round
        w.write_f64_le(0.0); // rotation
        w.write_u8(1); // is_plated = true
        w.write_u8(0); // stack_mode = Simple
        w.finish()
    }

    #[test]
    fn parse_pad_known_bytes() {
        let data = make_pad_payload();
        let pad = parse_pad(&data).unwrap();
        assert_eq!(pad.location.x.to_internal(), 50_000);
        assert_eq!(pad.location.y.to_internal(), 75_000);
        assert_eq!(pad.size_top.x.to_internal(), 30_000);
        assert_eq!(pad.size_top.y.to_internal(), 30_000);
        assert_eq!(pad.hole_size.to_internal(), 15_000);
        assert_eq!(pad.shape_top, PadShape::Round);
        assert_eq!(pad.shape_mid, PadShape::Round);
        assert_eq!(pad.shape_bot, PadShape::Round);
        assert_eq!(pad.rotation, 0.0);
        assert!(pad.is_plated);
        assert_eq!(pad.stack_mode, PadStackMode::Simple);
        assert!(pad.subrecord_trailing[0].is_empty());
        assert!(pad.unique_id.is_none());
    }

    #[test]
    fn parse_pad_with_trailing_bytes() {
        let mut data = make_pad_payload();
        data.extend_from_slice(&[0xAA, 0xBB, 0xCC]);
        let pad = parse_pad(&data).unwrap();
        assert_eq!(pad.subrecord_trailing[0], &[0xAA, 0xBB, 0xCC]);
    }

    #[test]
    fn truncated_pad_returns_error() {
        // Too short for common header (needs 13 bytes minimum)
        let data = vec![0u8; 5];
        let result = parse_pad(&data);
        assert!(matches!(result, Err(AltiumFormatError::BinaryReadPastEnd { .. })));
    }
}
