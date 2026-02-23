use altium_format_types::V6Layer;

use crate::binary_io::BinaryReader;
use crate::pcblib::primitives::common::parse_common_header;
use crate::pcblib::PcbVia;
use crate::Result;

/// Parses a Via primitive from its single PcbLib subrecord.
///
/// PcbLib via layout (after 13-byte common header):
///   location (8) + diameter (4) + hole_size (4) + from_layer (1) + to_layer (1)
pub(crate) fn parse_via(data: &[u8]) -> Result<PcbVia> {
    let mut reader = BinaryReader::new(data);
    let common = parse_common_header(&mut reader)?;
    let location = reader.read_coord_point()?;
    let diameter = reader.read_coord()?;
    let hole_size = reader.read_coord()?;
    let from_layer = V6Layer::try_from(reader.read_u8()?)?;
    let to_layer = V6Layer::try_from(reader.read_u8()?)?;
    reader.assert_exhausted()?;
    Ok(PcbVia {
        common,
        location,
        diameter,
        hole_size,
        from_layer,
        to_layer,
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

    #[test]
    fn parse_via_known_bytes() {
        let mut w = BinaryWriter::new();
        write_common_header(&mut w);
        // location (8) + diameter (4) + hole_size (4) + from_layer (1) + to_layer (1) = 18
        // 13 + 18 = 31 bytes minimum
        w.write_coord_point(CoordPoint::new(
            Coord::from_internal(50_000),
            Coord::from_internal(75_000),
        ));
        w.write_coord(Coord::from_internal(20_000)); // diameter
        w.write_coord(Coord::from_internal(8_000));  // hole_size
        w.write_u8(1);  // from_layer (TopLayer)
        w.write_u8(32); // to_layer (BottomLayer)
        let data = w.finish();
        assert_eq!(data.len(), 31);
        let via = parse_via(&data).unwrap();
        assert_eq!(via.location.x.to_internal(), 50_000);
        assert_eq!(via.location.y.to_internal(), 75_000);
        assert_eq!(via.diameter.to_internal(), 20_000);
        assert_eq!(via.hole_size.to_internal(), 8_000);
        assert!(via.unique_id.is_none());
    }

    #[test]
    fn truncated_via_returns_error() {
        let data = [0u8; 3];
        let result = parse_via(&data);
        assert!(matches!(result, Err(AltiumFormatError::BinaryReadPastEnd { .. })));
    }
}
