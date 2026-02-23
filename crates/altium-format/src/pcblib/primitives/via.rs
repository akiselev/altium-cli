use altium_format_types::{Coord, V6Layer};

use crate::Result;
use crate::binary_io::BinaryReader;
use crate::pcblib::PcbVia;
use crate::pcblib::primitives::common::parse_common_header;

/// Parses a Via primitive from its single PcbLib subrecord.
///
/// The Via binary record is a multi-section format (300-321+ bytes in AD26).
///
/// Section 1 — Core via data (246 bytes, or 31 for legacy):
///   0-12:   common header (13 bytes)
///   13-20:  location (CoordPoint, 8 bytes)
///   21-24:  diameter (Coord, 4 bytes)
///   25-28:  hole_size (Coord, 4 bytes)
///   29:     from_layer (u8 → V6Layer)
///   30:     to_layer (u8 → V6Layer)
///   31:     unknown (u8)
///   32-35:  thermal_relief_air_gap (i32)
///   36:     thermal_relief_conductor_count (u8)
///   37:     skip (u8)
///   38-41:  thermal_relief_conductor_width (i32)
///   42-53:  unknown/skip (12 bytes)
///   54-57:  solder_mask_expansion_front (i32)
///   58-65:  skip (8 bytes)
///   66:     solder_mask_expansion_manual (u8, bit 0x02)
///   67-73:  skip (7 bytes)
///   74:     via_mode (u8, 0=simple, 1=pad-stack)
///   75-202: diameters_per_layer (32 × i32 = 128 bytes)
///   203-240: skip (38 bytes)
///   241:    solder_mask_expansion_linked (u8, bit 0x01)
///   242-245: solder_mask_expansion_back (i32)
///
/// Sections 2-5 (variable, consumed as raw bytes):
///   2: u32 count + u32 stride(=9) + count*9 extended entries
///   3: 42 bytes additional data
///   4: u32 count + u32 stride(=30) + count*30 pad layer entries
///   5: 9 bytes trailing data
pub(crate) fn parse_via(data: &[u8]) -> Result<PcbVia> {
    let mut reader = BinaryReader::new(data);
    let common = parse_common_header(&mut reader)?;
    let location = reader.read_coord_point()?;
    let diameter = reader.read_coord()?;
    let hole_size = reader.read_coord()?;
    let from_layer = V6Layer::try_from(reader.read_u8()?)?;
    let to_layer = V6Layer::try_from(reader.read_u8()?)?;
    // offset 31 — end of legacy core

    // Extended fields (offsets 31-202): thermal relief, solder mask, via mode, per-layer diameters
    let mut thermal_relief_air_gap = Coord::ZERO;
    let mut thermal_relief_conductor_count: u8 = 0;
    let mut thermal_relief_conductor_width = Coord::ZERO;
    let mut solder_mask_expansion_front = Coord::ZERO;
    let mut solder_mask_expansion_manual = false;
    let mut via_mode: u8 = 0;
    let mut diameters_per_layer = [Coord::ZERO; 32];
    let mut solder_mask_expansion_linked = false;
    let mut solder_mask_expansion_back = Coord::ZERO;
    let pos_tolerance = Coord::ZERO;
    let neg_tolerance = Coord::ZERO;
    let mut post_section_data = Vec::new();

    // Offsets 31-202 (172 bytes): extended via data
    if reader.remaining() > 0 {
        // 31: unknown byte
        let _unknown_31 = reader.read_u8()?;
        // 32-35: thermal relief air gap
        thermal_relief_air_gap = reader.read_coord()?;
        // 36: thermal relief conductor count
        thermal_relief_conductor_count = reader.read_u8()?;
        // 37: skip
        reader.skip(1)?;
        // 38-41: thermal relief conductor width
        thermal_relief_conductor_width = reader.read_coord()?;
        // 42-49: unknown fields (8 bytes)
        reader.skip(8)?;
        // 50-53: skip (4 bytes)
        reader.skip(4)?;
        // 54-57: solder mask expansion front
        solder_mask_expansion_front = reader.read_coord()?;
        // 58-65: skip (8 bytes)
        reader.skip(8)?;
        // 66: solder mask expansion manual (bit 0x02)
        let manual_byte = reader.read_u8()?;
        solder_mask_expansion_manual = (manual_byte & 0x02) != 0;
        // 67-73: skip (7 bytes)
        reader.skip(7)?;
        // 74: via mode (0=simple, 1=pad-stack)
        via_mode = reader.read_u8()?;
        // 75-202: diameters per layer (32 × i32)
        for d in &mut diameters_per_layer {
            *d = reader.read_coord()?;
        }

        // Offsets 203-245 (43 bytes): additional extended fields
        if reader.remaining() >= 43 {
            // 203-240: skip (38 bytes)
            reader.skip(38)?;
            // 241: solder mask expansion linked (bit 0x01)
            let linked_byte = reader.read_u8()?;
            solder_mask_expansion_linked = (linked_byte & 0x01) != 0;
            // 242-245: solder mask expansion back
            solder_mask_expansion_back = reader.read_coord()?;
        }

        // Remaining data is sections 2-5 (variable length).
        // Store as raw bytes to consume the reader.
        if reader.remaining() > 0 {
            post_section_data = reader.read_bytes(reader.remaining())?.to_vec();
        }
    }

    Ok(PcbVia {
        common,
        location,
        diameter,
        hole_size,
        from_layer,
        to_layer,
        thermal_relief_air_gap,
        thermal_relief_conductor_count,
        thermal_relief_conductor_width,
        solder_mask_expansion_front,
        solder_mask_expansion_manual,
        via_mode,
        diameters_per_layer,
        solder_mask_expansion_linked,
        solder_mask_expansion_back,
        pos_tolerance,
        neg_tolerance,
        post_section_data,
        unique_id: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AltiumFormatError;
    use crate::binary_io::BinaryWriter;
    use altium_format_types::CoordPoint;

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
    fn parse_via_legacy_31_bytes() {
        let mut w = BinaryWriter::new();
        write_common_header(&mut w);
        w.write_coord_point(CoordPoint::new(
            Coord::from_internal(50_000),
            Coord::from_internal(75_000),
        ));
        w.write_coord(Coord::from_internal(20_000)); // diameter
        w.write_coord(Coord::from_internal(8_000)); // hole_size
        w.write_u8(1); // from_layer (TopLayer)
        w.write_u8(32); // to_layer (BottomLayer)
        let data = w.finish();
        assert_eq!(data.len(), 31);
        let via = parse_via(&data).unwrap();
        assert_eq!(via.location.x.to_internal(), 50_000);
        assert_eq!(via.location.y.to_internal(), 75_000);
        assert_eq!(via.diameter.to_internal(), 20_000);
        assert_eq!(via.hole_size.to_internal(), 8_000);
        assert_eq!(via.thermal_relief_air_gap.to_internal(), 0);
        assert_eq!(via.via_mode, 0);
        assert!(via.post_section_data.is_empty());
        assert!(via.unique_id.is_none());
    }

    #[test]
    fn parse_via_extended_246_bytes() {
        let mut w = BinaryWriter::new();
        write_common_header(&mut w);
        // Core (offsets 13-30)
        w.write_coord_point(CoordPoint::new(
            Coord::from_internal(10_000),
            Coord::from_internal(20_000),
        ));
        w.write_coord(Coord::from_internal(40_000)); // diameter
        w.write_coord(Coord::from_internal(12_000)); // hole_size
        w.write_u8(1); // from_layer
        w.write_u8(32); // to_layer
        // Extended (offsets 31-202)
        w.write_u8(0); // 31: unknown
        w.write_coord(Coord::from_internal(5_000)); // 32-35: thermal_relief_air_gap
        w.write_u8(4); // 36: thermal_relief_conductor_count
        w.write_u8(0); // 37: skip
        w.write_coord(Coord::from_internal(2_500)); // 38-41: thermal_relief_conductor_width
        w.write_i32_le(0);
        w.write_i32_le(0); // 42-49: unknown
        w.write_i32_le(0); // 50-53: skip
        w.write_coord(Coord::from_internal(1_000)); // 54-57: solder_mask_expansion_front
        for _ in 0..8 {
            w.write_u8(0);
        } // 58-65: skip
        w.write_u8(0x02); // 66: solder_mask_expansion_manual = true
        for _ in 0..7 {
            w.write_u8(0);
        } // 67-73: skip
        w.write_u8(1); // 74: via_mode = pad-stack
        // 75-202: diameters per layer (32 x i32)
        for i in 0..32u32 {
            w.write_coord(Coord::from_internal((i * 1000) as i32));
        }
        // Additional extended (offsets 203-245)
        for _ in 0..38 {
            w.write_u8(0);
        } // 203-240: skip
        w.write_u8(0x01); // 241: solder_mask_expansion_linked = true
        w.write_coord(Coord::from_internal(2_000)); // 242-245: solder_mask_expansion_back
        let data = w.finish();
        assert_eq!(data.len(), 246);
        let via = parse_via(&data).unwrap();
        assert_eq!(via.diameter.to_internal(), 40_000);
        assert_eq!(via.hole_size.to_internal(), 12_000);
        assert_eq!(via.thermal_relief_air_gap.to_internal(), 5_000);
        assert_eq!(via.thermal_relief_conductor_count, 4);
        assert_eq!(via.thermal_relief_conductor_width.to_internal(), 2_500);
        assert_eq!(via.solder_mask_expansion_front.to_internal(), 1_000);
        assert!(via.solder_mask_expansion_manual);
        assert_eq!(via.via_mode, 1);
        assert_eq!(via.diameters_per_layer[0].to_internal(), 0);
        assert_eq!(via.diameters_per_layer[1].to_internal(), 1_000);
        assert_eq!(via.diameters_per_layer[31].to_internal(), 31_000);
        assert!(via.solder_mask_expansion_linked);
        assert_eq!(via.solder_mask_expansion_back.to_internal(), 2_000);
        assert!(via.post_section_data.is_empty());
    }

    #[test]
    fn parse_via_with_post_section_data() {
        let mut w = BinaryWriter::new();
        write_common_header(&mut w);
        // Core
        w.write_coord_point(CoordPoint::new(Coord::ZERO, Coord::ZERO));
        w.write_coord(Coord::from_internal(20_000));
        w.write_coord(Coord::from_internal(10_000));
        w.write_u8(1);
        w.write_u8(32);
        // Extended (offsets 31-202)
        w.write_u8(0); // 31
        w.write_coord(Coord::ZERO); // 32-35
        w.write_u8(4); // 36
        w.write_u8(0); // 37
        w.write_coord(Coord::ZERO); // 38-41
        w.write_i32_le(0);
        w.write_i32_le(0); // 42-49
        w.write_i32_le(0); // 50-53
        w.write_coord(Coord::ZERO); // 54-57
        for _ in 0..8 {
            w.write_u8(0);
        } // 58-65
        w.write_u8(0); // 66
        for _ in 0..7 {
            w.write_u8(0);
        } // 67-73
        w.write_u8(0); // 74
        for _ in 0..32 {
            w.write_i32_le(0);
        } // 75-202
        // Additional extended (offsets 203-245)
        for _ in 0..38 {
            w.write_u8(0);
        } // 203-240
        w.write_u8(0); // 241
        w.write_i32_le(0); // 242-245
        // Simulate sections 2-5 (just some bytes)
        w.write_u32_le(0); // section 2 count = 0
        w.write_u32_le(9); // section 2 stride = 9
        for _ in 0..42 {
            w.write_u8(0xAB);
        } // section 3 (42 bytes)
        let data = w.finish();
        assert_eq!(data.len(), 246 + 8 + 42);
        let via = parse_via(&data).unwrap();
        assert_eq!(via.post_section_data.len(), 50); // 8 + 42
    }

    #[test]
    fn truncated_via_returns_error() {
        let data = [0u8; 3];
        let result = parse_via(&data);
        assert!(matches!(
            result,
            Err(AltiumFormatError::BinaryReadPastEnd { .. })
        ));
    }
}
