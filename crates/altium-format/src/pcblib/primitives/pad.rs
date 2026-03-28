use altium_format_types::constants::parsing::PAD_SUBRECORD_COUNT;
use altium_format_types::pcb::PolygonReliefAngle;
use altium_format_types::{
    Coord, CoordPoint, DaisyChainStyle, PadShape, PadStackMode, PlaneConnectionStyle, TCacheState,
    V7Layer,
};

use crate::binary_io::BinaryReader;
use crate::pcblib::primitives::common::parse_common_header;
use crate::pcblib::{PcbPad, PcbPadCache, PcbPadExtendedCrEntry, PcbPadStackData};
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
    let daisy_chain_style = DaisyChainStyle::try_from(reader.read_u8()?)?;
    let pad_mode = PadStackMode::try_from(reader.read_u8()?)?;

    // Field at offset 63 (FUN_01811110, always present)
    let unknown_63 = reader.read_i32_le()?;

    // TV6_PadCache (offsets 67-104, 38 bytes, always present)
    let cache = parse_pad_cache(&mut reader)?;

    // Post-cache fields (offsets 105-113, always present)
    let selection_memory_flags = reader.read_u8()?;
    let union_index = reader.read_i32_le()?;
    let jumper_id = reader.read_i32_le()?;

    // --- Extended fields (offsets 114+, version-dependent) ---
    // FUN_0187b7c0 reads these conditionally based on remaining data.

    // Layer override + hole flags (offsets 114-119, 6 bytes)
    let (v7_layer_override, is_assy_testpoint_top, is_assy_testpoint_bottom) =
        if reader.remaining() >= 6 {
            let v7 = reader.read_i32_le()?;
            let top = reader.read_u8()? != 0;
            let bottom = reader.read_u8()? != 0;
            (v7, top, bottom)
        } else {
            (0, false, false)
        };

    // Mask expansion linkage + bottom solder mask (offsets 120-125, 6 bytes)
    let (
        use_separate_expansions,
        solder_mask_bottom_expansion,
        solder_mask_expansion_from_hole_edge,
    ) = if reader.remaining() >= 6 {
        let separate = reader.read_u8()? != 0;
        let bottom = reader.read_i32_le()?;
        let from_hole = reader.read_u8()? != 0;
        (separate, bottom, from_hole)
    } else {
        (false, 0, false)
    };

    // Template link IDs (offsets 126-157, 32 bytes)
    let (template_link_library_id, template_link_template_id) = if reader.remaining() >= 32 {
        let mut library_id = [0u8; 16];
        library_id.copy_from_slice(reader.read_bytes(16)?);
        let mut template_id = [0u8; 16];
        template_id.copy_from_slice(reader.read_bytes(16)?);
        (library_id, template_id)
    } else {
        ([0u8; 16], [0u8; 16])
    };

    // Tolerances (offsets 158-169, 12 bytes)
    // In older format variants (170-byte sub4), these are the last fields —
    // reserved_170 and has_sub4_extension don't exist.
    let (pin_package_length, hole_positive_tolerance, hole_negative_tolerance) =
        if reader.remaining() >= 12 {
            let ppl = reader.read_coord()?;
            let hpt = reader.read_i32_le()?;
            let hnt = reader.read_i32_le()?;
            (ppl, hpt, hnt)
        } else {
            (Coord::from_internal(0), 0x7FFFFFFF, 0x7FFFFFFF)
        };

    // reserved_170 (offset 170, 1 byte)
    // Present in 171+ byte sub4 variants. Absent in 170-byte variant.
    let reserved_170 = if reader.remaining() >= 2 {
        // At least 2 bytes remain: reserved_170 + has_sub4_extension (or more).
        let reserved = reader.read_u8()?;
        if reserved != 0 {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "reserved byte 170".to_owned(),
                detail: format!("expected 0, got {reserved:#04X}"),
            });
        }
        reserved
    } else if reader.remaining() == 1 {
        // Exactly 1 byte: reserved_170 only (171-byte variant, no sub4 extension).
        let reserved = reader.read_u8()?;
        if reserved != 0 {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "reserved byte 170".to_owned(),
                detail: format!("expected 0, got {reserved:#04X}"),
            });
        }
        reserved
    } else {
        0u8
    };

    // has_sub4_extension (offset 171, 1 byte)
    // Present in 172+ byte sub4 variants.
    let has_sub4_extension = if reader.remaining() >= 1 {
        reader.read_u8()? != 0
    } else {
        false
    };

    let (sub4_extension, thermal_reliefs) = parse_sub4_extension(&mut reader, has_sub4_extension)?;

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
        daisy_chain_style,
        pad_mode,
        unknown_63,
        cache,
        selection_memory_flags,
        union_index,
        jumper_id,
        v7_layer_override,
        is_assy_testpoint_top,
        is_assy_testpoint_bottom,
        use_separate_expansions,
        solder_mask_bottom_expansion,
        solder_mask_expansion_from_hole_edge,
        template_link_library_id,
        template_link_template_id,
        pin_package_length,
        hole_positive_tolerance,
        hole_negative_tolerance,
        reserved_170,
        has_sub4_extension,
        sub4_extension,
        thermal_reliefs,
        stack_data,
        unique_id: None,
    })
}

fn parse_sub4_extension(
    reader: &mut BinaryReader<'_>,
    has_sub4_extension: bool,
) -> Result<(
    Option<crate::pcblib::PcbPadSub4Extension>,
    Vec<crate::pcblib::PcbPadThermalReliefEntry>,
)> {
    if !has_sub4_extension {
        if reader.remaining() != 0 {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "Pad subrecord 4".to_owned(),
                detail: format!(
                    "unexpected trailing bytes without sub4 extension flag: {}",
                    reader.remaining()
                ),
            });
        }
        return Ok((None, Vec::new()));
    }

    if reader.remaining() == 0 {
        return Ok((None, Vec::new()));
    }

    if reader.remaining() < 4 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "Pad subrecord 4 extension".to_owned(),
            detail: format!(
                "expected u32 extension header length, only {} bytes remain",
                reader.remaining()
            ),
        });
    }

    let header_len = reader.read_u32_le()? as usize;
    let available = reader.remaining();
    let effective_header_len = header_len.min(available);
    let header = reader.read_bytes(effective_header_len)?;

    let mut header_reader = BinaryReader::new(header);
    let thermal_relief_count = if header_reader.remaining() >= 4 {
        header_reader.read_u32_le()?
    } else {
        0
    };
    let propagation_delay_f32 = if header_reader.remaining() >= 4 {
        header_reader.read_f32_le()?
    } else {
        0.0
    };
    let flags8 = if header_reader.remaining() >= 1 {
        header_reader.read_u8()?
    } else {
        0
    };
    let flags9 = if header_reader.remaining() >= 1 {
        header_reader.read_u8()?
    } else {
        0
    };
    let propagation_delay_f64 = if header_reader.remaining() >= 8 {
        header_reader.read_f64_le()?
    } else {
        0.0
    };

    let x_pad_offset_all_layers = if header_reader.remaining() >= 4 {
        header_reader.read_coord()?
    } else {
        Coord::from_internal(0)
    };
    let y_pad_offset_all_layers = if header_reader.remaining() >= 4 {
        header_reader.read_coord()?
    } else {
        Coord::from_internal(0)
    };

    // Assert these are zero - all 37,669 pads with 26-byte headers have zeros here.
    // If non-zero values appear, investigate to confirm the XPadOffsetAllLayers hypothesis.
    if x_pad_offset_all_layers != Coord::from_internal(0)
        || y_pad_offset_all_layers != Coord::from_internal(0)
    {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "Pad subrecord 4 extension".to_owned(),
            detail: format!(
                "extension header bytes 18..25 (hypothesized XPadOffset/YPadOffset) are non-zero: x={}, y={}",
                x_pad_offset_all_layers.to_internal(),
                y_pad_offset_all_layers.to_internal(),
            ),
        });
    }

    if header_reader.remaining() != 0 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "Pad subrecord 4 extension".to_owned(),
            detail: format!(
                "unsupported extension header bytes after known fields: {}",
                header_reader.remaining()
            ),
        });
    }

    let mut thermal_reliefs = Vec::new();
    if thermal_relief_count > 0 {
        if reader.remaining() < 4 {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "Pad subrecord 4 extension thermal entries".to_owned(),
                detail: "missing thermal entry size".to_owned(),
            });
        }
        let entry_size = reader.read_u32_le()? as usize;
        // Thermal entry sizes vary by format version:
        //   23 bytes: oldest (through expansion, no conductor_by_pad_edge or later fields)
        //   29 bytes: intermediate (missing use_custom_relief)
        //   30 bytes: current AD26 format (all fields present)
        if entry_size < 23 {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "Pad subrecord 4 extension thermal entries".to_owned(),
                detail: format!("thermal entry size {entry_size} too small (minimum 23)"),
            });
        }
        if entry_size > 30 {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "Pad subrecord 4 extension thermal entries".to_owned(),
                detail: format!("thermal entry size {entry_size} too large (maximum 30)"),
            });
        }

        let needed = (thermal_relief_count as usize)
            .checked_mul(entry_size)
            .ok_or_else(|| AltiumFormatError::InvalidParamValue {
                key: "Pad subrecord 4 extension thermal entries".to_owned(),
                detail: "thermal entry payload size overflow".to_owned(),
            })?;
        if reader.remaining() < needed {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "Pad subrecord 4 extension thermal entries".to_owned(),
                detail: format!(
                    "insufficient thermal entry bytes: need {needed}, have {}",
                    reader.remaining()
                ),
            });
        }

        for _ in 0..thermal_relief_count {
            let entry_data = reader.read_bytes(entry_size)?;
            let entry = parse_thermal_relief_entry(entry_data, entry_size)?;
            thermal_reliefs.push(entry);
        }
    }

    if reader.remaining() != 0 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "Pad subrecord 4 extension".to_owned(),
            detail: format!(
                "unsupported trailing extension bytes: {}",
                reader.remaining()
            ),
        });
    }

    Ok((
        Some(crate::pcblib::PcbPadSub4Extension {
            header_len: header_len as u32,
            thermal_relief_count,
            propagation_delay_f32,
            flags8,
            flags9,
            propagation_delay_f64,
            x_pad_offset_all_layers,
            y_pad_offset_all_layers,
        }),
        thermal_reliefs,
    ))
}

/// Parses a single thermal relief entry from its fixed-size byte slice.
///
/// The 30-byte (current) layout is:
///   TV7_Layer (4) + TPadViaThermalReliefData (26):
///     DefinedType(1) + ConnectStyle(1) + AirGapWidth(4) + ConductorWidth(4) +
///     Rotation(1) + Entries(4) + Expansion(4) = 23 bytes (always present)
///     ConductorByPadEdge(1)                   = byte 24 (added later)
///     MinDistance(4) + EnableMinDistance(1)     = bytes 24-29 (added later)
///     UseCustomRelief(1)                       = byte 30 (added last)
fn parse_thermal_relief_entry(
    data: &[u8],
    entry_size: usize,
) -> Result<crate::pcblib::PcbPadThermalReliefEntry> {
    let mut r = BinaryReader::new(data);

    // Always present (23 bytes minimum)
    let layer = V7Layer::new(r.read_u32_le()?);
    let defined_type = r.read_u8()?;
    let connect_style = PlaneConnectionStyle::try_from(r.read_u8()?)?;
    let air_gap_width = r.read_coord()?;
    let conductor_width = r.read_coord()?;
    let rotation = PolygonReliefAngle::try_from(r.read_u8()?)?;
    let entries = r.read_u32_le()?;
    let expansion = r.read_coord()?;

    // Fields added in later versions, conditional on entry_size
    let conductor_by_pad_edge = if entry_size >= 24 {
        r.read_u8()? != 0
    } else {
        false
    };
    let min_distance = if entry_size >= 28 {
        r.read_coord()?
    } else {
        Coord::from_internal(0)
    };
    let enable_min_distance = if entry_size >= 29 {
        r.read_u8()? != 0
    } else {
        false
    };
    let use_custom_relief = if entry_size >= 30 {
        r.read_u8()? != 0
    } else {
        false
    };

    r.assert_exhausted()?;

    Ok(crate::pcblib::PcbPadThermalReliefEntry {
        layer,
        defined_type,
        connect_style,
        air_gap_width,
        conductor_width,
        rotation,
        entries,
        expansion,
        conductor_by_pad_edge,
        min_distance,
        enable_min_distance,
        use_custom_relief,
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
    let hole_shape = PadShape::try_from(reader.read_u8()?)?;
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

    let alt_shape_bytes = reader.read_bytes(32)?;
    let mut alt_shape = [PadShape::Round; 32];
    for (i, &b) in alt_shape_bytes.iter().enumerate() {
        alt_shape[i] = PadShape::try_from(b)?;
    }

    let mut corner_radius_pct = [0u8; 32];
    corner_radius_pct.copy_from_slice(reader.read_bytes(32)?);

    let mut per_layer_overrides = [0u8; 32];
    per_layer_overrides.copy_from_slice(reader.read_bytes(32)?);

    // Extended per-layer CR entries (offset 628+).
    // Format: u32 count + u32 entry_size (must be 15) + count * 15 bytes.
    let extended_cr = if reader.remaining() >= 8 {
        let count = reader.read_u32_le()? as usize;
        let entry_size = reader.read_u32_le()? as usize;
        if entry_size != 15 {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "Pad stack subrecord extended CR".to_owned(),
                detail: format!("expected extended CR entry size 15, got {}", entry_size),
            });
        }
        if reader.remaining() < count * entry_size {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "Pad stack subrecord extended CR".to_owned(),
                detail: format!(
                    "need {} bytes for {} extended CR entries (15 bytes each), only {} remain",
                    count * entry_size,
                    count,
                    reader.remaining()
                ),
            });
        }
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            let layer_id = reader.read_u32_le()?;
            let alt_shape_val = PadShape::try_from(reader.read_u8()?)?;
            let cr_pct_ex = reader.read_coord()?;
            let cr_size = reader.read_coord()?;
            let cr_pct = reader.read_u8()?;
            let use_percent = reader.read_u8()? != 0;
            entries.push(PcbPadExtendedCrEntry {
                layer_id,
                alt_shape: alt_shape_val,
                cr_pct_ex,
                cr_size,
                cr_pct,
                use_percent,
            });
        }
        entries
    } else {
        Vec::new()
    };

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
        extended_cr,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binary_io::BinaryWriter;
    use altium_format_types::{Coord, CoordPoint, DaisyChainStyle, PadShape, PadStackMode};

    fn write_common_header(w: &mut BinaryWriter) {
        w.write_u8(74); // layer = MultiLayer
        w.write_u16_le(0x000C); // flags
        w.write_u16_le(0xFFFF); // net_index = none
        w.write_u16_le(0xFFFF); // polygon_index = none
        w.write_u16_le(0xFFFF); // component_index = none
        w.write_u16_le(0xFFFF); // coordinate_index = none
        w.write_u16_le(0xFFFF); // dimension_index = none
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
        w.write_u8(0); // daisy_chain_style = Load
        w.write_u8(0); // pad_mode = Simple
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
        w.write_u8(0); // selection_memory_flags
        w.write_i32_le(0); // union_index
        w.write_i32_le(0); // jumper_id
    }

    /// Write extended fields (offsets 114-171, 58 bytes).
    fn write_pad_extended(w: &mut BinaryWriter) {
        w.write_i32_le(0); // v7_layer_override
        w.write_u8(0); // is_assy_testpoint_top
        w.write_u8(0); // is_assy_testpoint_bottom
        w.write_u8(0); // use_separate_expansions
        w.write_i32_le(0); // solder_mask_bottom_expansion
        w.write_u8(0); // solder_mask_expansion_from_hole_edge
        w.write_bytes(&[0u8; 16]); // template_link_library_id
        w.write_bytes(&[0u8; 16]); // template_link_template_id
        w.write_i32_le(0); // pin_package_length
        w.write_i32_le(0x7FFFFFFF); // hole_positive_tolerance
        w.write_i32_le(0x7FFFFFFF); // hole_negative_tolerance
        w.write_u8(0); // reserved_170
        w.write_u8(0); // has_sub4_extension
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
        assert_eq!(pad.daisy_chain_style, DaisyChainStyle::Load);
        assert_eq!(pad.pad_mode, PadStackMode::Simple);
        assert_eq!(
            pad.cache.plane_connection_style,
            PlaneConnectionStyle::NoConnect
        );
        assert_eq!(pad.cache.relief_entries, 4);
        assert!(!pad.has_sub4_extension);
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
        assert_eq!(pad.v7_layer_override, 0);
        assert!(!pad.is_assy_testpoint_top);
        assert!(!pad.use_separate_expansions);
        assert!(!pad.has_sub4_extension);
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
