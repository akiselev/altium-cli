use altium_format_types::constants::parsing::PAD_SUBRECORD_COUNT;
use altium_format_types::{CoordPoint, PadShape, PadStackMode};

use crate::binary_io::BinaryReader;
use crate::pcblib::primitives::common::parse_common_header;
use crate::pcblib::PcbPad;
use crate::{AltiumFormatError, Result};

/// The index of the main pad data subrecord (within the 6 pad subrecords).
///
/// Subrecord layout:
///   0: small header (2 bytes)
///   1: flag (1 byte)
///   2: pad name (length-prefixed string)
///   3: flag (1 byte)
///   4: main pad binary data (110-200+ bytes)
///   5: extended data (optional, may be 0 bytes)
const MAIN_DATA_SUBRECORD: usize = 4;

/// Parses a Pad primitive from its 6 PcbLib subrecords.
///
/// The main pad binary data (common header + coordinates + shapes + etc.)
/// lives in subrecord 4. The other subrecords carry supplementary data
/// (small headers, pad name, flags, extended fields).
pub(crate) fn parse_pad(subrecords: &[&[u8]]) -> Result<PcbPad> {
    if subrecords.len() != PAD_SUBRECORD_COUNT {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "Pad subrecords".to_owned(),
            detail: format!(
                "expected {} subrecords, got {}",
                PAD_SUBRECORD_COUNT,
                subrecords.len()
            ),
        });
    }

    let main_data = subrecords[MAIN_DATA_SUBRECORD];
    let mut reader = BinaryReader::new(main_data);
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
    reader.assert_exhausted()?;

    // Assert all non-main subrecords are fully consumed (no opaque passthrough).
    for (i, &sub) in subrecords.iter().enumerate() {
        if i == MAIN_DATA_SUBRECORD {
            continue;
        }
        BinaryReader::new(sub).assert_exhausted()
            .map_err(|e| AltiumFormatError::InvalidParamValue {
                key: format!("Pad subrecord {i}"),
                detail: format!("non-main subrecord has {} unparsed bytes: {e}", sub.len()),
            })?;
    }

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
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use altium_format_types::{Coord, CoordPoint};
    use crate::binary_io::BinaryWriter;

    fn write_common_header(w: &mut BinaryWriter) {
        w.write_u8(74);     // layer = MultiLayer
        w.write_u8(0);      // pad_byte
        w.write_u16_le(0);  // flags
        w.write_i32_le(-1); // net_index
        w.write_u16_le(0xFFFF); // polygon_index
        w.write_u16_le(0);  // component_index
        w.write_u8(0);      // unknown
    }

    fn make_main_pad_subrecord() -> Vec<u8> {
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

    /// Build the 6-subrecord slice array for a pad.
    ///
    /// Non-main subrecords must be empty (the parser enforces assert_exhausted
    /// on all non-main subrecords).
    fn make_pad_subrecords(main_data: &[u8]) -> [Vec<u8>; 6] {
        [
            vec![],                 // sub 0: empty
            vec![],                 // sub 1: empty
            vec![],                 // sub 2: empty
            vec![],                 // sub 3: empty
            main_data.to_vec(),     // sub 4: main pad data
            vec![],                 // sub 5: empty
        ]
    }

    #[test]
    fn parse_pad_known_bytes() {
        let main_data = make_main_pad_subrecord();
        let subs = make_pad_subrecords(&main_data);
        let sub_refs: Vec<&[u8]> = subs.iter().map(|s| s.as_slice()).collect();
        let pad = parse_pad(&sub_refs).unwrap();
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
        assert!(pad.unique_id.is_none());
    }

    #[test]
    fn parse_pad_with_trailing_bytes_in_main_subrecord_errors() {
        let mut main_data = make_main_pad_subrecord();
        main_data.extend_from_slice(&[0xAA, 0xBB, 0xCC]);
        let subs = make_pad_subrecords(&main_data);
        let sub_refs: Vec<&[u8]> = subs.iter().map(|s| s.as_slice()).collect();
        let result = parse_pad(&sub_refs);
        assert!(matches!(result, Err(AltiumFormatError::UnexpectedTrailingData { .. })));
    }

    #[test]
    fn wrong_subrecord_count_returns_error() {
        let main_data = make_main_pad_subrecord();
        // Only provide 1 subrecord instead of 6
        let sub_refs: Vec<&[u8]> = vec![main_data.as_slice()];
        let result = parse_pad(&sub_refs);
        assert!(result.is_err());
    }
}
