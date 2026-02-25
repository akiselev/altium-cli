use altium_format_types::constants::parsing::PAD_SUBRECORD_COUNT;
use altium_format_types::{
    Coord, CoordPoint, HoleType, PadShape, PadStackMode, PlaneConnectionStyle, TCacheState,
};

use crate::binary_io::BinaryReader;
use crate::pcblib::primitives::common::parse_common_header;
use crate::pcblib::{PcbPad, PcbPadCache, PcbPadStackData};
use crate::{AltiumFormatError, Result};

/// The index of the main pad data subrecord (within the 6 pad subrecords).
///
/// Subrecord layout (from Ghidra FUN_018a2900):
///   0: pad name (length-prefixed string, via FUN_018113d0 SetName)
///   1: unknown string (length-prefixed, via FUN_018118b0)
///   2: unknown string (length-prefixed, via FUN_01811820)
///   3: unknown string (length-prefixed, via FUN_01811940)
///   4: main pad binary data (114-200+ bytes, version-dependent)
///   5: per-layer stack data (0 or 596+ bytes)
const MAIN_DATA_SUBRECORD: usize = 4;

/// Minimum size for per-layer stack data (subrecord 5).
/// From Ghidra FUN_018a2840: initialization copies 596 bytes from template.
const MIN_STACK_DATA_SIZE: usize = 596;

/// Parses a Pad primitive from its 6 PcbLib subrecords.
///
/// Full layout confirmed by Ghidra decompilation of Altium.PCB.BinaryLoader.dll
/// (FUN_0186d700, FUN_0187b7c0, FUN_018a2900) and C# TV6_PadCache struct.
///
/// The main subrecord (index 4) has version-dependent size:
///   - Always present: offsets 0-113 (114 bytes) — common + core + cache + post-cache
///   - Extended fields (114+) are read conditionally based on remaining data
///     (matching FUN_0187b7c0's remaining-bytes checks)
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

    // --- Subrecords 0-3: length-prefixed strings ---
    let pad_name = parse_string_subrecord(subrecords[0], 0)?;
    let unknown_sub1 = parse_string_subrecord(subrecords[1], 1)?;
    let unknown_sub2 = parse_string_subrecord(subrecords[2], 2)?;
    let unknown_sub3 = parse_string_subrecord(subrecords[3], 3)?;

    // --- Subrecord 4: main pad binary data ---
    let main_data = subrecords[MAIN_DATA_SUBRECORD];
    let mut reader = BinaryReader::new(main_data);

    // Common header (offsets 0-12, 13 bytes)
    let common = parse_common_header(&mut reader)?;

    // Core pad fields (offsets 13-62, always present)
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
    let hole_type = HoleType::try_from(reader.read_u8()?)?;
    let stack_mode = PadStackMode::try_from(reader.read_u8()?)?;

    // Field at offset 63 (FUN_01811110, always present)
    let unknown_63 = reader.read_i32_le()?;

    // TV6_PadCache (offsets 67-104, 38 bytes, always present)
    let cache = parse_pad_cache(&mut reader)?;

    // Post-cache fields (offsets 105-113, always present)
    let user_routed = reader.read_u8()? != 0;
    let union_index = reader.read_i32_le()?;
    let unknown_110 = reader.read_i32_le()?;

    // --- Extended fields (offsets 114+, version-dependent) ---
    // FUN_0187b7c0 reads these conditionally based on remaining data.

    // Layer override + hole flags (offsets 114-119, 6 bytes)
    let (layer_override, hole_flag_1, hole_flag_2) = if reader.remaining() >= 6 {
        let lo = reader.read_i32_le()?;
        let hf1 = reader.read_u8()? != 0;
        let hf2 = reader.read_u8()? != 0;
        (lo, hf1, hf2)
    } else {
        (0, false, false)
    };

    // Stack fields + swap IDs (offsets 120-157, 38 bytes)
    let (stack_flag, stack_conditional, unknown_125, swap_id_pad, swap_id_part) =
        if reader.remaining() >= 38 {
            let sf = reader.read_u8()? != 0;
            let sc = reader.read_i32_le()?;
            let u125 = reader.read_u8()? != 0;
            let mut sid_pad = [0u8; 16];
            sid_pad.copy_from_slice(reader.read_bytes(16)?);
            let mut sid_part = [0u8; 16];
            sid_part.copy_from_slice(reader.read_bytes(16)?);
            (sf, sc, u125, sid_pad, sid_part)
        } else {
            (false, 0, false, [0u8; 16], [0u8; 16])
        };

    // Tolerances + unknown_170 (offsets 158-170, 13 bytes)
    let (pin_package_length, hole_positive_tolerance, hole_negative_tolerance, unknown_170) =
        if reader.remaining() >= 13 {
            let ppl = reader.read_coord()?;
            let hpt = reader.read_i32_le()?;
            let hnt = reader.read_i32_le()?;
            let u170 = reader.read_u8()?;
            (ppl, hpt, hnt, u170)
        } else {
            (Coord::from_internal(0), 0x7FFFFFFF, 0x7FFFFFFF, 0u8)
        };

    // has_stack_data (offset 171, 1 byte)
    let has_stack_data = if reader.remaining() >= 1 {
        reader.read_u8()? != 0
    } else {
        false
    };

    if reader.remaining() != 0 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "Pad subrecord 4".to_owned(),
            detail: format!(
                "unsupported trailing bytes after known pad layout: {} bytes remain",
                reader.remaining()
            ),
        });
    }

    // --- Subrecord 5: per-layer stack data ---
    let stack_data = parse_stack_subrecord(subrecords[5])?;

    Ok(PcbPad {
        common,
        pad_name,
        unknown_sub1,
        unknown_sub2,
        unknown_sub3,
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
        hole_type,
        stack_mode,
        unknown_63,
        cache,
        user_routed,
        union_index,
        unknown_110,
        layer_override,
        hole_flag_1,
        hole_flag_2,
        stack_flag,
        stack_conditional,
        unknown_125,
        swap_id_pad,
        swap_id_part,
        pin_package_length,
        hole_positive_tolerance,
        hole_negative_tolerance,
        unknown_170,
        has_stack_data,
        stack_data,
        unique_id: None,
    })
}

/// Parses a string subrecord (subrecords 0-3): u8 length prefix + Windows-1252 bytes.
fn parse_string_subrecord(data: &[u8], index: usize) -> Result<String> {
    let mut reader = BinaryReader::new(data);
    let s = reader.read_pascal_string()?;
    reader
        .assert_exhausted()
        .map_err(|e| AltiumFormatError::InvalidParamValue {
            key: format!("Pad subrecord {index}"),
            detail: format!("string subrecord has unparsed bytes after string: {e}"),
        })?;
    Ok(s)
}

/// Parses the TV6_PadCache (38 bytes, offsets 67-104 in main subrecord).
fn parse_pad_cache(reader: &mut BinaryReader) -> Result<PcbPadCache> {
    let plane_connection_style = PlaneConnectionStyle::try_from(reader.read_u8()?)?;
    let relief_conductor_width = reader.read_coord()?;
    let relief_entries = reader.read_i16_le()?;
    let relief_air_gap = reader.read_coord()?;
    let power_plane_relief_expansion = reader.read_coord()?;
    let power_plane_clearance = reader.read_coord()?;
    let paste_mask_expansion = reader.read_coord()?;
    let solder_mask_expansion = reader.read_coord()?;
    let planes = reader.read_u16_le()?;
    let plane_connection_style_valid = TCacheState::try_from(reader.read_u8()?)?;
    let relief_conductor_width_valid = TCacheState::try_from(reader.read_u8()?)?;
    let relief_entries_valid = TCacheState::try_from(reader.read_u8()?)?;
    let relief_air_gap_valid = TCacheState::try_from(reader.read_u8()?)?;
    let power_plane_relief_expansion_valid = TCacheState::try_from(reader.read_u8()?)?;
    let paste_mask_expansion_valid = TCacheState::try_from(reader.read_u8()?)?;
    let solder_mask_expansion_valid = TCacheState::try_from(reader.read_u8()?)?;
    let power_plane_clearance_valid = TCacheState::try_from(reader.read_u8()?)?;
    let planes_valid = TCacheState::try_from(reader.read_u8()?)?;

    Ok(PcbPadCache {
        plane_connection_style,
        relief_conductor_width,
        relief_entries,
        relief_air_gap,
        power_plane_relief_expansion,
        power_plane_clearance,
        paste_mask_expansion,
        solder_mask_expansion,
        planes,
        plane_connection_style_valid,
        relief_conductor_width_valid,
        relief_entries_valid,
        relief_air_gap_valid,
        power_plane_relief_expansion_valid,
        paste_mask_expansion_valid,
        solder_mask_expansion_valid,
        power_plane_clearance_valid,
        planes_valid,
    })
}

/// Parses per-layer stack data from subrecord 5.
///
/// From Ghidra FUN_018a2840 (init) + FUN_0187c7d0 (per-layer loop):
/// - len == 0: no stack data
/// - len >= 596: parse full stack structure
/// - 0 < len < 596: error (invalid stack data size)
fn parse_stack_subrecord(data: &[u8]) -> Result<Option<PcbPadStackData>> {
    if data.is_empty() {
        return Ok(None);
    }

    if data.len() < MIN_STACK_DATA_SIZE {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "Pad subrecord 5".to_owned(),
            detail: format!(
                "per-layer stack data must be 0 or >= {} bytes, got {}",
                MIN_STACK_DATA_SIZE,
                data.len()
            ),
        });
    }

    let mut reader = BinaryReader::new(data);

    let mut inner_size_x = [Coord::from_internal(0); 29];
    for coord in &mut inner_size_x {
        *coord = reader.read_coord()?;
    }

    let mut inner_size_y = [Coord::from_internal(0); 29];
    for coord in &mut inner_size_y {
        *coord = reader.read_coord()?;
    }

    let mut inner_shape = [PadShape::Round; 29];
    for shape in &mut inner_shape {
        *shape = PadShape::try_from(reader.read_u8()?)?;
    }

    let padding_261 = reader.read_u8()?;
    let hole_shape = reader.read_u8()?;
    let slot_size = reader.read_coord()?;
    let slot_rotation = reader.read_f64_le()?;

    let mut hole_offset_x = [Coord::from_internal(0); 32];
    for coord in &mut hole_offset_x {
        *coord = reader.read_coord()?;
    }

    let mut hole_offset_y = [Coord::from_internal(0); 32];
    for coord in &mut hole_offset_y {
        *coord = reader.read_coord()?;
    }

    let padding_531 = reader.read_u8()?;

    let mut alt_shape = [0u8; 32];
    alt_shape.copy_from_slice(reader.read_bytes(32)?);

    let mut corner_radius_pct = [0u8; 32];
    corner_radius_pct.copy_from_slice(reader.read_bytes(32)?);

    let mut per_layer_overrides = [0u8; 32];
    per_layer_overrides.copy_from_slice(reader.read_bytes(32)?);

    if reader.remaining() != 0 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "Pad subrecord 5".to_owned(),
            detail: format!(
                "unsupported trailing bytes after known stack layout: {} bytes remain",
                reader.remaining()
            ),
        });
    }

    Ok(Some(PcbPadStackData {
        inner_size_x,
        inner_size_y,
        inner_shape,
        padding_261,
        hole_shape,
        slot_size,
        slot_rotation,
        hole_offset_x,
        hole_offset_y,
        padding_531,
        alt_shape,
        corner_radius_pct,
        per_layer_overrides,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binary_io::BinaryWriter;
    use altium_format_types::{Coord, CoordPoint, HoleType, PadShape, PadStackMode};

    fn write_common_header(w: &mut BinaryWriter) {
        w.write_u8(74); // layer = MultiLayer
        w.write_u8(0); // pad_byte
        w.write_u16_le(0); // flags
        w.write_i32_le(-1); // net_index
        w.write_u16_le(0xFFFF); // polygon_index
        w.write_u16_le(0); // component_index
        w.write_u8(0); // unknown
    }

    fn make_string_sub(s: &str) -> Vec<u8> {
        let mut v = Vec::new();
        v.push(s.len() as u8);
        v.extend_from_slice(s.as_bytes());
        v
    }

    /// Write the always-present portion of pad data (114 bytes: offsets 0-113).
    fn write_pad_core(w: &mut BinaryWriter) {
        write_common_header(w);
        // Core fields (offsets 13-62)
        w.write_coord_point(CoordPoint::new(
            Coord::from_internal(50_000),
            Coord::from_internal(75_000),
        ));
        w.write_coord(Coord::from_internal(30_000)); // size_top_x
        w.write_coord(Coord::from_internal(30_000)); // size_top_y
        w.write_coord(Coord::from_internal(30_000)); // size_mid_x
        w.write_coord(Coord::from_internal(30_000)); // size_mid_y
        w.write_coord(Coord::from_internal(30_000)); // size_bot_x
        w.write_coord(Coord::from_internal(30_000)); // size_bot_y
        w.write_coord(Coord::from_internal(15_000)); // hole_size
        w.write_u8(1); // shape_top = Round
        w.write_u8(1); // shape_mid = Round
        w.write_u8(1); // shape_bot = Round
        w.write_f64_le(0.0); // rotation
        w.write_u8(1); // is_plated = true
        w.write_u8(0); // hole_type = Round
        w.write_u8(0); // stack_mode = Simple
        // unknown_63 (offset 63)
        w.write_i32_le(0);
        // TV6_PadCache (offsets 67-104, 38 bytes)
        w.write_u8(0); // plane_connection_style = NoConnect
        w.write_i32_le(0); // relief_conductor_width
        w.write_i16_le(4); // relief_entries
        w.write_i32_le(0); // relief_air_gap
        w.write_i32_le(0); // power_plane_relief_expansion
        w.write_i32_le(0); // power_plane_clearance
        w.write_i32_le(0); // paste_mask_expansion
        w.write_i32_le(0); // solder_mask_expansion
        w.write_u16_le(0); // planes
        w.write_u8(0); // plane_connection_style_valid
        w.write_u8(0); // relief_conductor_width_valid
        w.write_u8(0); // relief_entries_valid
        w.write_u8(0); // relief_air_gap_valid
        w.write_u8(0); // power_plane_relief_expansion_valid
        w.write_u8(0); // paste_mask_expansion_valid
        w.write_u8(0); // solder_mask_expansion_valid
        w.write_u8(0); // power_plane_clearance_valid
        w.write_u8(0); // planes_valid
        // Post-cache fields (offsets 105-113)
        w.write_u8(0); // user_routed
        w.write_i32_le(0); // union_index
        w.write_i32_le(0); // unknown_110
    }

    /// Write extended fields (offsets 114-171, 58 bytes).
    fn write_pad_extended(w: &mut BinaryWriter) {
        w.write_i32_le(0); // layer_override
        w.write_u8(0); // hole_flag_1
        w.write_u8(0); // hole_flag_2
        w.write_u8(0); // stack_flag
        w.write_i32_le(0); // stack_conditional
        w.write_u8(0); // unknown_125
        w.write_bytes(&[0u8; 16]); // swap_id_pad
        w.write_bytes(&[0u8; 16]); // swap_id_part
        w.write_i32_le(0); // pin_package_length
        w.write_i32_le(0x7FFFFFFF); // hole_positive_tolerance
        w.write_i32_le(0x7FFFFFFF); // hole_negative_tolerance
        w.write_u8(0); // unknown_170
        w.write_u8(0); // has_stack_data
    }

    fn make_pad_subrecords(main_data: &[u8]) -> [Vec<u8>; 6] {
        [
            make_string_sub("1"), // sub 0: pad name
            make_string_sub(""),  // sub 1: empty string
            make_string_sub(""),  // sub 2: empty string
            make_string_sub(""),  // sub 3: empty string
            main_data.to_vec(),   // sub 4: main pad data
            vec![],               // sub 5: no stack data
        ]
    }

    #[test]
    fn parse_pad_full_172_bytes() {
        let mut w = BinaryWriter::new();
        write_pad_core(&mut w);
        write_pad_extended(&mut w);
        let main_data = w.finish();
        assert_eq!(main_data.len(), 172);
        let subs = make_pad_subrecords(&main_data);
        let sub_refs: Vec<&[u8]> = subs.iter().map(|s| s.as_slice()).collect();
        let pad = parse_pad(&sub_refs).unwrap();
        assert_eq!(pad.pad_name, "1");
        assert_eq!(pad.location.x.to_internal(), 50_000);
        assert_eq!(pad.location.y.to_internal(), 75_000);
        assert_eq!(pad.hole_size.to_internal(), 15_000);
        assert_eq!(pad.shape_top, PadShape::Round);
        assert!(pad.is_plated);
        assert_eq!(pad.hole_type, HoleType::Round);
        assert_eq!(pad.stack_mode, PadStackMode::Simple);
        assert_eq!(
            pad.cache.plane_connection_style,
            PlaneConnectionStyle::NoConnect
        );
        assert_eq!(pad.cache.relief_entries, 4);
        assert!(!pad.has_stack_data);
        assert!(pad.stack_data.is_none());
    }

    #[test]
    fn parse_pad_minimal_114_bytes() {
        let mut w = BinaryWriter::new();
        write_pad_core(&mut w);
        let main_data = w.finish();
        assert_eq!(main_data.len(), 114);
        let subs = make_pad_subrecords(&main_data);
        let sub_refs: Vec<&[u8]> = subs.iter().map(|s| s.as_slice()).collect();
        let pad = parse_pad(&sub_refs).unwrap();
        assert_eq!(pad.pad_name, "1");
        assert_eq!(pad.location.x.to_internal(), 50_000);
        // Extended fields default to zero/false
        assert_eq!(pad.layer_override, 0);
        assert!(!pad.hole_flag_1);
        assert!(!pad.stack_flag);
        assert!(!pad.has_stack_data);
    }

    #[test]
    fn parse_pad_with_post_172_data_errors() {
        let mut w = BinaryWriter::new();
        write_pad_core(&mut w);
        write_pad_extended(&mut w);
        let mut main_data = w.finish();
        main_data.extend_from_slice(&[0x01, 0x02, 0x03, 0x04, 0x05]);
        let subs = make_pad_subrecords(&main_data);
        let sub_refs: Vec<&[u8]> = subs.iter().map(|s| s.as_slice()).collect();
        let err = parse_pad(&sub_refs);
        assert!(matches!(
            err,
            Err(AltiumFormatError::InvalidParamValue { .. })
        ));
    }

    #[test]
    fn wrong_subrecord_count_returns_error() {
        let mut w = BinaryWriter::new();
        write_pad_core(&mut w);
        let main_data = w.finish();
        let sub_refs: Vec<&[u8]> = vec![main_data.as_slice()];
        let result = parse_pad(&sub_refs);
        assert!(result.is_err());
    }

    #[test]
    fn invalid_stack_data_size_returns_error() {
        let mut w = BinaryWriter::new();
        write_pad_core(&mut w);
        let main_data = w.finish();
        let mut subs = make_pad_subrecords(&main_data);
        subs[5] = vec![0u8; 100];
        let sub_refs: Vec<&[u8]> = subs.iter().map(|s| s.as_slice()).collect();
        let result = parse_pad(&sub_refs);
        assert!(result.is_err());
    }
}
