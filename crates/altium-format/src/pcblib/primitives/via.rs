use altium_format_types::{
    Coord, MaskExpansionMode, PadShape, PadStackMode, PlaneConnectionStyle, TCacheState, V6Layer,
    ViaStructureType,
};

use crate::Result;
use crate::binary_io::BinaryReader;
use crate::pcblib::primitives::common::parse_common_header;
use crate::pcblib::{PcbVia, PcbViaPadLayerEntry, PcbViaSection2Entry};

/// Parses a single per-layer pad stack entry from Section 4.
///
/// Stride 30 layout (confirmed from binary analysis across 67K+ entries):
///   layer(4) + shape(1) + mode(1) + solder_mask_exp(4) + paste_mask_exp(4) +
///   plane_conn_style(1) + relief_entries(2) + reserved(2) + conductor_width(4) +
///   reserved(1) + air_gap(4) + reserved(2) = 30 bytes
///
/// Strides 23/24/29 are older format versions with fewer fields.
fn parse_pad_layer_entry(reader: &mut BinaryReader, stride: u32) -> Result<PcbViaPadLayerEntry> {
    let layer_id = reader.read_u32_le()?;
    let shape = PadShape::try_from(reader.read_u8()?)?;
    let mode = PadStackMode::try_from(reader.read_u8()?)?;
    let solder_mask_expansion = reader.read_coord()?;

    match stride {
        30 => {
            let paste_mask_expansion = reader.read_coord()?;
            let plane_connection_style = PlaneConnectionStyle::try_from(reader.read_u8()?)?;
            let relief_entries = reader.read_i16_le()?;
            let _reserved_17 = reader.read_u16_le()?;
            let relief_conductor_width = reader.read_coord()?;
            let _reserved_23 = reader.read_u8()?;
            let relief_air_gap = reader.read_coord()?;
            let _reserved_28 = reader.read_u16_le()?;
            Ok(PcbViaPadLayerEntry {
                layer_id,
                shape,
                mode,
                solder_mask_expansion,
                paste_mask_expansion: Some(paste_mask_expansion),
                plane_connection_style,
                relief_entries: i32::from(relief_entries),
                relief_conductor_width: Some(relief_conductor_width),
                relief_air_gap: Some(relief_air_gap),
                trailing_flags: 0,
            })
        }
        29 => {
            let relief_conductor_width = reader.read_coord()?;
            let plane_connection_style = PlaneConnectionStyle::try_from(reader.read_u8()?)?;
            let relief_entries = reader.read_i16_le()?;
            let _reserved_17 = reader.read_u16_le()?;
            let reserved_i32 = reader.read_i32_le()?;
            let _reserved_23 = reader.read_u8()?;
            let relief_air_gap = reader.read_coord()?;
            let _reserved_28 = reader.read_u8()?;
            if reserved_i32 != 0 {
                return Err(crate::AltiumFormatError::InvalidParamValue {
                    key: "Via Section 4 entry (stride 29)".to_owned(),
                    detail: format!("expected reserved i32 = 0, got {reserved_i32}"),
                });
            }
            Ok(PcbViaPadLayerEntry {
                layer_id,
                shape,
                mode,
                solder_mask_expansion,
                paste_mask_expansion: None,
                plane_connection_style,
                relief_entries: i32::from(relief_entries),
                relief_conductor_width: Some(relief_conductor_width),
                relief_air_gap: Some(relief_air_gap),
                trailing_flags: 0,
            })
        }
        23 | 24 => {
            let relief_conductor_width = reader.read_coord()?;
            let plane_connection_style = PlaneConnectionStyle::try_from(reader.read_u8()?)?;
            let relief_entries = reader.read_i32_le()?;
            let trailing_bytes = reader.read_bytes((stride - 19) as usize)?;
            let mut trailing_flags = 0u32;
            for (i, &b) in trailing_bytes.iter().enumerate() {
                trailing_flags |= u32::from(b) << (i * 8);
            }
            Ok(PcbViaPadLayerEntry {
                layer_id,
                shape,
                mode,
                solder_mask_expansion,
                paste_mask_expansion: None,
                plane_connection_style,
                relief_entries,
                relief_conductor_width: Some(relief_conductor_width),
                relief_air_gap: None,
                trailing_flags,
            })
        }
        _ => Err(crate::AltiumFormatError::InvalidParamValue {
            key: "Via Section 4 entry".to_owned(),
            detail: format!("unsupported stride {stride}"),
        }),
    }
}

/// Parses a Via primitive from its single PcbLib subrecord.
///
/// Parsed coverage:
/// - Core section (31-byte legacy and 246-byte extended formats)
/// - Section 2 layer-diameter overrides (stride=9 records)
pub(crate) fn parse_via(data: &[u8]) -> Result<PcbVia> {
    let mut reader = BinaryReader::new(data);
    let common = parse_common_header(&mut reader)?;
    let location = reader.read_coord_point()?;
    let diameter = reader.read_coord()?;
    let hole_size = reader.read_coord()?;
    let from_layer = V6Layer::try_from(reader.read_u8()?)?;
    let to_layer = V6Layer::try_from(reader.read_u8()?)?;

    let mut via_properties_version: u8 = 0;
    let mut thermal_relief_air_gap = Coord::ZERO;
    let mut thermal_relief_conductor_count: u8 = 0;
    let mut thermal_relief_rotation_code: u8 = 0;
    let mut thermal_relief_conductor_width = Coord::ZERO;
    let mut power_plane_relief_expansion = Coord::ZERO;
    let mut power_plane_clearance = Coord::ZERO;
    let mut paste_mask_expansion = Coord::ZERO;
    let mut solder_mask_expansion_front = Coord::ZERO;

    let mut planes: u16 = 0;
    let mut plane_connection_style_valid = TCacheState::Invalid;
    let mut relief_conductor_width_valid = TCacheState::Invalid;
    let mut relief_entries_valid = TCacheState::Invalid;
    let mut relief_air_gap_valid = TCacheState::Invalid;
    let mut power_plane_relief_expansion_valid = TCacheState::Invalid;
    let mut paste_mask_expansion_valid = TCacheState::Invalid;

    let mut solder_mask_expansion_valid = TCacheState::Invalid;
    let mut power_plane_clearance_valid = TCacheState::Invalid;
    let mut planes_valid = TCacheState::Invalid;
    let mut plane_connection_style = PlaneConnectionStyle::NoConnect;
    let mut solder_mask_cache_flags: u8 = 0;
    let mut solder_mask_expansion_mode = MaskExpansionMode::NoMask;
    let mut paste_mask_cache_flags: u8 = 0;
    let mut paste_mask_expansion_mode = MaskExpansionMode::NoMask;

    let mut via_mode = PadStackMode::Simple;
    let mut diameters_per_layer = [Coord::ZERO; 32];

    let mut layer_enum_index = 0i32;
    let mut stack_start_layer = 0u8;
    let mut stack_end_layer = 0u8;
    let mut is_testpoint_top = false;
    let mut is_testpoint_bottom = false;
    let mut is_assy_testpoint_top = false;
    let mut is_assy_testpoint_bottom = false;
    let mut solder_mask_override = false;
    let mut use_separate_solder_mask_expansion = false;
    let mut solder_mask_expansion_from_hole_edge = false;
    let mut paste_mask_override = false;

    let mut solder_mask_expansion_linked = false;
    let mut solder_mask_expansion_back = Coord::ZERO;

    let mut layer_diameter_overrides = Vec::new();
    let mut template_link_version: Option<u8> = None;
    let mut template_link_library_id: Option<[u8; 16]> = None;
    let mut template_link_template_id: Option<[u8; 16]> = None;
    let mut hole_positive_tolerance: Option<Coord> = None;
    let mut hole_negative_tolerance: Option<Coord> = None;
    let mut template_link_flags: Option<u8> = None;
    let mut pad_layer_entries: Vec<PcbViaPadLayerEntry> = Vec::new();
    let mut pad_layer_stride: u32 = 0;
    let mut counter_hole_angle: Option<f64> = None;
    let mut via_structure_type: Option<ViaStructureType> = None;

    if reader.remaining() > 0 {
        via_properties_version = reader.read_u8()?;
        thermal_relief_air_gap = reader.read_coord()?;
        thermal_relief_conductor_count = reader.read_u8()?;
        thermal_relief_rotation_code = reader.read_u8()?;
        thermal_relief_conductor_width = reader.read_coord()?;

        power_plane_relief_expansion = reader.read_coord()?;
        power_plane_clearance = reader.read_coord()?;
        paste_mask_expansion = reader.read_coord()?;
        solder_mask_expansion_front = reader.read_coord()?;

        planes = reader.read_u16_le()?;
        plane_connection_style_valid = TCacheState::try_from(reader.read_u8()?)?;
        relief_conductor_width_valid = TCacheState::try_from(reader.read_u8()?)?;
        relief_entries_valid = TCacheState::try_from(reader.read_u8()?)?;
        relief_air_gap_valid = TCacheState::try_from(reader.read_u8()?)?;
        power_plane_relief_expansion_valid = TCacheState::try_from(reader.read_u8()?)?;
        paste_mask_expansion_valid = TCacheState::try_from(reader.read_u8()?)?;

        solder_mask_expansion_valid = TCacheState::try_from(reader.read_u8()?)?;
        power_plane_clearance_valid = TCacheState::try_from(reader.read_u8()?)?;
        planes_valid = TCacheState::try_from(reader.read_u8()?)?;
        plane_connection_style = PlaneConnectionStyle::try_from(reader.read_u8()?)?;

        // Offsets 70-73: packed cache/mode flags.
        // Byte 70: solder mask cache flags (packed 4×2-bit fields).
        // Byte 71: solder mask expansion mode/count (observed: 0-7).
        // Byte 72: paste mask cache flags (packed 4×2-bit fields).
        // Byte 73: paste mask expansion mode/count (observed: 0 in all test files).
        solder_mask_cache_flags = reader.read_u8()?;
        solder_mask_expansion_mode = MaskExpansionMode::try_from(reader.read_u8()?)?;
        paste_mask_cache_flags = reader.read_u8()?;
        paste_mask_expansion_mode = MaskExpansionMode::try_from(reader.read_u8()?)?;

        via_mode = PadStackMode::try_from(reader.read_u8()?)?;
        for d in &mut diameters_per_layer {
            *d = reader.read_coord()?;
        }

        if reader.remaining() >= 43 {
            layer_enum_index = reader.read_i32_le()?;
            stack_start_layer = reader.read_u8()?;
            stack_end_layer = reader.read_u8()?;
            // 32-byte extension region (offsets 209-240): individual boolean flags.
            // Confirmed via empirical analysis of 41K+ Via records and C#/Delphi source.
            let reserved_209 = reader.read_u8()?;
            if reserved_209 != 0 {
                return Err(crate::AltiumFormatError::InvalidParamValue {
                    key: "Via extension byte 209".to_owned(),
                    detail: format!("expected reserved byte = 0, got {reserved_209}"),
                });
            }
            is_testpoint_top = reader.read_u8()? != 0;           // offset 210
            is_testpoint_bottom = reader.read_u8()? != 0;        // offset 211
            is_assy_testpoint_top = reader.read_u8()? != 0;      // offset 212
            is_assy_testpoint_bottom = reader.read_u8()? != 0;   // offset 213
            solder_mask_override = reader.read_u8()? != 0;       // offset 214
            use_separate_solder_mask_expansion = reader.read_u8()? != 0; // offset 215
            let reserved_216 = reader.read_u8()?;
            if reserved_216 != 0 {
                return Err(crate::AltiumFormatError::InvalidParamValue {
                    key: "Via extension byte 216".to_owned(),
                    detail: format!("expected reserved byte = 0, got {reserved_216}"),
                });
            }
            solder_mask_expansion_from_hole_edge = reader.read_u8()? != 0; // offset 217
            let reserved_218_239 = reader.read_bytes(22)?;       // offsets 218-239
            if reserved_218_239.iter().any(|&b| b != 0) {
                return Err(crate::AltiumFormatError::InvalidParamValue {
                    key: "Via extension bytes 218-239".to_owned(),
                    detail: format!(
                        "expected 22 reserved zero bytes, got non-zero at offset(s): {}",
                        reserved_218_239.iter().enumerate()
                            .filter(|(_, b)| **b != 0)
                            .map(|(i, b)| format!("{}=0x{:02x}", 218 + i, b))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                });
            }
            paste_mask_override = reader.read_u8()? != 0;        // offset 240
            let linked_byte = reader.read_u8()?;
            solder_mask_expansion_linked = (linked_byte & 0x01) != 0;
            solder_mask_expansion_back = reader.read_coord()?;
        }

        if reader.remaining() >= 8 {
            let section2_count = reader.read_u32_le()? as usize;
            let section2_stride = reader.read_u32_le()?;
            if !(section2_stride == 9 || (section2_count == 0 && section2_stride == 0)) {
                return Err(crate::AltiumFormatError::InvalidParamValue {
                    key: "via.section2.stride".to_owned(),
                    detail: format!("expected stride 9, got {section2_stride}"),
                });
            }
            for _ in 0..section2_count {
                layer_diameter_overrides.push(PcbViaSection2Entry {
                    layer: reader.read_u8()?,
                    diameter: reader.read_coord()?,
                    rule_index: reader.read_u16_le()?,
                    flags: reader.read_u8()?,
                    mode: reader.read_u8()?,
                });
            }
        }

        // Via template link block: size-prefixed (4-byte size + payload).
        // Core payload (41 bytes): version(1) + LibraryID GUID(16) + TemplateID GUID(16)
        //   + HolePositiveTolerance(4, i32) + HoleNegativeTolerance(4, i32).
        // ext_size=42: adds flags(1). ext_size=45: adds flags(1) + 3 unknown bytes.
        if reader.remaining() >= 45 {
            let ext_size = reader.read_u32_le()? as usize;
            if ext_size < 41 || reader.remaining() < ext_size {
                return Err(crate::AltiumFormatError::InvalidParamValue {
                    key: "Via template link block".to_owned(),
                    detail: format!("declared size {ext_size} but only {} bytes remain", reader.remaining()),
                });
            }
            let mut ext = crate::binary_io::BinaryReader::new(reader.read_bytes(ext_size)?);
            template_link_version = Some(ext.read_u8()?);
            let mut guid1 = [0u8; 16];
            guid1.copy_from_slice(ext.read_bytes(16)?);
            template_link_library_id = Some(guid1);
            let mut guid2 = [0u8; 16];
            guid2.copy_from_slice(ext.read_bytes(16)?);
            template_link_template_id = Some(guid2);
            // Tolerances: i32::MAX (0x7FFFFFFF) is the Delphi "not set" sentinel
            // (Delphi initializes to MaxInt). Treat as None.
            let pos_tol_raw = ext.read_i32_le()?;
            if pos_tol_raw != i32::MAX {
                hole_positive_tolerance = Some(Coord::from_internal(pos_tol_raw));
            }
            let neg_tol_raw = ext.read_i32_le()?;
            if neg_tol_raw != i32::MAX {
                hole_negative_tolerance = Some(Coord::from_internal(neg_tol_raw));
            }
            if ext.remaining() >= 1 {
                template_link_flags = Some(ext.read_u8()?);
            }
            // ext_size=45: flags(1) + 3 extra bytes (empty RevisionID field,
            // from IPCB_PadViaTemplateLink.RevisionID). Always observed as zeros.
            if ext.remaining() > 0 {
                let trailing = ext.read_bytes(ext.remaining())?;
                if trailing.iter().any(|&b| b != 0) {
                    return Err(crate::AltiumFormatError::InvalidParamValue {
                        key: "Via template link trailing bytes".to_owned(),
                        detail: format!("expected zeros, got {:02x?}", trailing),
                    });
                }
            }
        }

        // Section 4: Per-layer pad stack entries (stride varies: 23, 29, 30).
        // Framed as u32 count + u32 stride, followed by count × stride entry bytes.
        if reader.remaining() >= 8 {
            let section4_count = reader.read_u32_le()? as usize;
            pad_layer_stride = reader.read_u32_le()?;

            // Validate stride: known values are 23, 24, 29, 30 (or 0 when count=0).
            if section4_count > 0
                && !matches!(pad_layer_stride, 23 | 24 | 29 | 30)
            {
                return Err(crate::AltiumFormatError::InvalidParamValue {
                    key: "via.section4.stride".to_owned(),
                    detail: format!(
                        "unexpected Section 4 stride {pad_layer_stride} with count {section4_count}"
                    ),
                });
            }

            for _ in 0..section4_count {
                pad_layer_entries.push(
                    parse_pad_layer_entry(&mut reader, pad_layer_stride)?
                );
            }
        }

        // Section 5: IPC-4761 / via structure.
        // Framed as u32 size + payload. Payload is 9 bytes (f64 angle + u8 via_structure_type)
        // or 4 bytes in older files (all zeros, no structure type defined).
        if reader.remaining() >= 4 {
            let section5_size = reader.read_u32_le()? as usize;
            if reader.remaining() < section5_size {
                return Err(crate::AltiumFormatError::InvalidParamValue {
                    key: "Via Section 5".to_owned(),
                    detail: format!(
                        "declared size {section5_size} but only {} bytes remain",
                        reader.remaining()
                    ),
                });
            }
            match section5_size {
                9 => {
                    counter_hole_angle = Some(reader.read_f64_le()?);
                    via_structure_type =
                        Some(ViaStructureType::try_from(reader.read_u8()?)?);
                }
                4 => {
                    // Older format: 4 bytes, observed as all zeros.
                    let placeholder = reader.read_u32_le()?;
                    if placeholder != 0 {
                        return Err(crate::AltiumFormatError::InvalidParamValue {
                            key: "Via Section 5 (4-byte)".to_owned(),
                            detail: format!(
                                "expected 4 zero bytes, got 0x{placeholder:08x}"
                            ),
                        });
                    }
                }
                other => {
                    return Err(crate::AltiumFormatError::InvalidParamValue {
                        key: "Via Section 5".to_owned(),
                        detail: format!(
                            "unexpected Section 5 payload size {other}, expected 4 or 9"
                        ),
                    });
                }
            }
        }

        if reader.remaining() > 0 {
            return Err(crate::AltiumFormatError::InvalidParamValue {
                key: "Via".to_owned(),
                detail: format!(
                    "unsupported trailing bytes after known via layout: {} bytes remain",
                    reader.remaining()
                ),
            });
        }
    }

    Ok(PcbVia {
        common,
        location,
        diameter,
        hole_size,
        from_layer,
        to_layer,
        via_properties_version,
        thermal_relief_air_gap,
        thermal_relief_conductor_count,
        thermal_relief_rotation_code,
        thermal_relief_conductor_width,
        power_plane_relief_expansion,
        power_plane_clearance,
        paste_mask_expansion,
        solder_mask_expansion_front,
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
        plane_connection_style,
        solder_mask_cache_flags,
        solder_mask_expansion_mode,
        paste_mask_cache_flags,
        paste_mask_expansion_mode,
        via_mode,
        diameters_per_layer,
        layer_enum_index,
        stack_start_layer,
        stack_end_layer,
        is_testpoint_top,
        is_testpoint_bottom,
        is_assy_testpoint_top,
        is_assy_testpoint_bottom,
        solder_mask_override,
        use_separate_solder_mask_expansion,
        solder_mask_expansion_from_hole_edge,
        paste_mask_override,
        solder_mask_expansion_linked,
        solder_mask_expansion_back,
        template_link_version,
        template_link_library_id,
        template_link_template_id,
        hole_positive_tolerance,
        hole_negative_tolerance,
        template_link_flags,
        pad_layer_entries,
        pad_layer_stride,
        counter_hole_angle,
        via_structure_type,
        layer_diameter_overrides,
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
        w.write_u8(1); // layer = TopLayer
        w.write_u16_le(0x000C); // flags
        w.write_u16_le(0xFFFF); // net_index = none
        w.write_u16_le(0xFFFF); // polygon_index = none
        w.write_u16_le(0xFFFF); // component_index = none
        w.write_u16_le(0xFFFF); // coordinate_index = none
        w.write_u16_le(0xFFFF); // dimension_index = none
    }

    #[test]
    fn parse_via_legacy_31_bytes() {
        let mut w = BinaryWriter::new();
        write_common_header(&mut w);
        w.write_coord_point(CoordPoint::new(
            Coord::from_internal(50_000),
            Coord::from_internal(75_000),
        ));
        w.write_coord(Coord::from_internal(20_000));
        w.write_coord(Coord::from_internal(8_000));
        w.write_u8(1);
        w.write_u8(32);
        let data = w.finish();
        assert_eq!(data.len(), 31);
        let via = parse_via(&data).unwrap();
        assert_eq!(via.location.x.to_internal(), 50_000);
        assert_eq!(via.location.y.to_internal(), 75_000);
        assert_eq!(via.diameter.to_internal(), 20_000);
        assert_eq!(via.hole_size.to_internal(), 8_000);
        assert!(via.layer_diameter_overrides.is_empty());
    }

    #[test]
    fn parse_via_extended_246_bytes() {
        let mut w = BinaryWriter::new();
        write_common_header(&mut w);
        w.write_coord_point(CoordPoint::new(
            Coord::from_internal(10_000),
            Coord::from_internal(20_000),
        ));
        w.write_coord(Coord::from_internal(40_000));
        w.write_coord(Coord::from_internal(12_000));
        w.write_u8(1);
        w.write_u8(32);
        w.write_u8(0);
        w.write_coord(Coord::from_internal(5_000));
        w.write_u8(1);
        w.write_u8(0);
        w.write_coord(Coord::from_internal(2_500));
        w.write_coord(Coord::from_internal(300));
        w.write_coord(Coord::from_internal(400));
        w.write_coord(Coord::from_internal(500));
        w.write_coord(Coord::from_internal(1_000));
        w.write_u16_le(0);
        for _ in 0..6 {
            w.write_u8(0);
        }
        w.write_u8(0x02);
        w.write_u8(0);
        w.write_u8(0);
        w.write_u8(0);
        w.write_u8(0);
        w.write_u8(0);
        w.write_u8(0);
        w.write_u8(0);
        w.write_u8(1);
        for i in 0..32u32 {
            w.write_coord(Coord::from_internal((i * 1000) as i32));
        }
        w.write_i32_le(123);
        w.write_u8(1);
        w.write_u8(32);
        // 32-byte extension region: individual boolean flags at specific offsets.
        // Byte 0 (offset 209): reserved (must be 0)
        w.write_u8(0);
        // Byte 1 (offset 210): is_testpoint_top = true
        w.write_u8(1);
        // Byte 2 (offset 211): is_testpoint_bottom = false
        w.write_u8(0);
        // Byte 3 (offset 212): is_assy_testpoint_top = false
        w.write_u8(0);
        // Byte 4 (offset 213): is_assy_testpoint_bottom = true
        w.write_u8(1);
        // Byte 5 (offset 214): solder_mask_override = false
        w.write_u8(0);
        // Byte 6 (offset 215): use_separate_solder_mask_expansion = false
        w.write_u8(0);
        // Byte 7 (offset 216): reserved (must be 0)
        w.write_u8(0);
        // Byte 8 (offset 217): solder_mask_expansion_from_hole_edge = false
        w.write_u8(0);
        // Bytes 9-30 (offsets 218-239): reserved (must be 0)
        w.write_bytes(&[0u8; 22]);
        // Byte 31 (offset 240): paste_mask_override = false
        w.write_u8(0);
        w.write_u8(0x01);
        w.write_coord(Coord::from_internal(2_000));

        let data = w.finish();
        assert_eq!(data.len(), 246);
        let via = parse_via(&data).unwrap();
        assert_eq!(via.thermal_relief_air_gap.to_internal(), 5_000);
        assert_eq!(via.thermal_relief_conductor_width.to_internal(), 2_500);
        assert_eq!(via.power_plane_relief_expansion.to_internal(), 300);
        assert_eq!(via.power_plane_clearance.to_internal(), 400);
        assert_eq!(via.paste_mask_expansion.to_internal(), 500);
        assert_eq!(via.solder_mask_expansion_front.to_internal(), 1_000);
        assert_eq!(via.solder_mask_expansion_valid, TCacheState::Manual);
        assert!(via.solder_mask_expansion_linked);
        assert_eq!(via.solder_mask_expansion_back.to_internal(), 2_000);
        assert_eq!(via.layer_enum_index, 123);
        // Verify extension boolean flags
        assert!(via.is_testpoint_top);
        assert!(!via.is_testpoint_bottom);
        assert!(!via.is_assy_testpoint_top);
        assert!(via.is_assy_testpoint_bottom);
        assert!(!via.solder_mask_override);
        assert!(!via.use_separate_solder_mask_expansion);
        assert!(!via.solder_mask_expansion_from_hole_edge);
        assert!(!via.paste_mask_override);
    }

    #[test]
    fn parse_via_with_layer_diameter_overrides_only() {
        let mut w = BinaryWriter::new();
        write_common_header(&mut w);
        w.write_coord_point(CoordPoint::new(Coord::ZERO, Coord::ZERO));
        w.write_coord(Coord::from_internal(20_000));
        w.write_coord(Coord::from_internal(10_000));
        w.write_u8(1);
        w.write_u8(32);
        w.write_u8(0);
        w.write_coord(Coord::ZERO);
        w.write_u8(0);
        w.write_u8(0);
        w.write_coord(Coord::ZERO);
        w.write_coord(Coord::ZERO);
        w.write_coord(Coord::ZERO);
        w.write_coord(Coord::ZERO);
        w.write_coord(Coord::ZERO);
        w.write_u16_le(0);
        for _ in 0..6 {
            w.write_u8(0);
        }
        w.write_u8(0);
        for _ in 0..7 {
            w.write_u8(0);
        }
        w.write_u8(0);
        for _ in 0..32 {
            w.write_i32_le(0);
        }
        w.write_i32_le(0);
        w.write_u8(1);
        w.write_u8(32);
        w.write_bytes(&[0u8; 32]); // 32-byte extension region (all flags false, reserved bytes zero)
        w.write_u8(0); // solder_mask_expansion_linked
        w.write_i32_le(0); // solder_mask_expansion_back

        w.write_u32_le(1);
        w.write_u32_le(9);
        for i in 0..9u8 {
            w.write_u8(i);
        }

        let data = w.finish();
        let via = parse_via(&data).unwrap();
        assert_eq!(via.layer_diameter_overrides.len(), 1);
        assert_eq!(via.layer_diameter_overrides[0].layer, 0);
        assert_eq!(
            via.layer_diameter_overrides[0].diameter.to_internal(),
            0x04030201
        );
        assert_eq!(via.layer_diameter_overrides[0].rule_index, 0x0605);
        assert_eq!(via.layer_diameter_overrides[0].flags, 7);
        assert_eq!(via.layer_diameter_overrides[0].mode, 8);
    }

    #[test]
    fn parse_via_with_unmapped_sections_errors() {
        let mut w = BinaryWriter::new();
        write_common_header(&mut w);
        w.write_coord_point(CoordPoint::new(Coord::ZERO, Coord::ZERO));
        w.write_coord(Coord::from_internal(20_000));
        w.write_coord(Coord::from_internal(10_000));
        w.write_u8(1);
        w.write_u8(32);
        w.write_u8(0);
        w.write_coord(Coord::ZERO);
        w.write_u8(0);
        w.write_u8(0);
        w.write_coord(Coord::ZERO);
        w.write_coord(Coord::ZERO);
        w.write_coord(Coord::ZERO);
        w.write_coord(Coord::ZERO);
        w.write_coord(Coord::ZERO);
        w.write_u16_le(0);
        for _ in 0..6 {
            w.write_u8(0);
        }
        w.write_u8(0);
        for _ in 0..7 {
            w.write_u8(0);
        }
        w.write_u8(0);
        for _ in 0..32 {
            w.write_i32_le(0);
        }
        w.write_i32_le(0);
        w.write_u8(1);
        w.write_u8(32);
        for _ in 0..8 {
            w.write_i32_le(0);
        }
        w.write_u8(0);
        w.write_i32_le(0);
        w.write_u32_le(0);
        w.write_u32_le(9);
        for _ in 0..42 {
            w.write_u8(0xAB);
        }

        let data = w.finish();
        let err = parse_via(&data);
        assert!(
            err.is_err(),
            "expected parse error for via with unmapped sections, but got Ok"
        );
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

    /// Helper: writes the full 246-byte extended via with all extension flags set to zero.
    /// Returns the BinaryWriter positioned just before the 32-byte extension region
    /// (i.e., after stack_start_layer and stack_end_layer).
    fn write_extended_via_up_to_extension_region() -> BinaryWriter {
        let mut w = BinaryWriter::new();
        write_common_header(&mut w);
        w.write_coord_point(CoordPoint::new(Coord::ZERO, Coord::ZERO));
        w.write_coord(Coord::from_internal(20_000));
        w.write_coord(Coord::from_internal(10_000));
        w.write_u8(1);  // from_layer
        w.write_u8(32); // to_layer
        // Extended fields up to offset 203
        w.write_u8(0);  // via_properties_version
        w.write_coord(Coord::ZERO); // thermal_relief_air_gap
        w.write_u8(0);  // thermal_relief_conductor_count
        w.write_u8(0);  // thermal_relief_rotation_code
        w.write_coord(Coord::ZERO); // thermal_relief_conductor_width
        w.write_coord(Coord::ZERO); // power_plane_relief_expansion
        w.write_coord(Coord::ZERO); // power_plane_clearance
        w.write_coord(Coord::ZERO); // paste_mask_expansion
        w.write_coord(Coord::ZERO); // solder_mask_expansion_front
        w.write_u16_le(0); // planes
        for _ in 0..6 { w.write_u8(0); } // cache validity fields
        w.write_u8(0);  // solder_mask_expansion_valid
        w.write_u8(0);  // power_plane_clearance_valid
        w.write_u8(0);  // planes_valid
        w.write_u8(0);  // plane_connection_style
        w.write_u8(0);  // solder_mask_cache_flags
        w.write_u8(0);  // solder_mask_expansion_mode
        w.write_u8(0);  // paste_mask_cache_flags
        w.write_u8(0);  // paste_mask_expansion_mode
        w.write_u8(0);  // via_mode
        for _ in 0..32 { w.write_coord(Coord::ZERO); } // diameters_per_layer
        w.write_i32_le(0); // layer_enum_index
        w.write_u8(1);    // stack_start_layer
        w.write_u8(32);   // stack_end_layer
        w
    }

    #[test]
    fn parse_via_reserved_byte_209_must_be_zero() {
        let mut w = write_extended_via_up_to_extension_region();
        // Write 32 extension bytes: byte 0 (offset 209) = nonzero
        w.write_u8(0xFF); // reserved_209 = nonzero (should fail)
        for _ in 0..31 { w.write_u8(0); }
        w.write_u8(0); // solder_mask_expansion_linked
        w.write_coord(Coord::ZERO); // solder_mask_expansion_back

        let data = w.finish();
        let err = parse_via(&data).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("209"), "error should mention byte 209: {msg}");
    }

    #[test]
    fn parse_via_reserved_byte_216_must_be_zero() {
        let mut w = write_extended_via_up_to_extension_region();
        // Bytes 0-6 of extension region
        w.write_u8(0); // reserved_209
        w.write_u8(0); // is_testpoint_top
        w.write_u8(0); // is_testpoint_bottom
        w.write_u8(0); // is_assy_testpoint_top
        w.write_u8(0); // is_assy_testpoint_bottom
        w.write_u8(0); // solder_mask_override
        w.write_u8(0); // use_separate_solder_mask_expansion
        // Byte 7 (offset 216) = nonzero
        w.write_u8(0xFF); // reserved_216 = nonzero (should fail)
        for _ in 0..24 { w.write_u8(0); } // remaining bytes
        w.write_u8(0); // solder_mask_expansion_linked
        w.write_coord(Coord::ZERO); // solder_mask_expansion_back

        let data = w.finish();
        let err = parse_via(&data).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("216"), "error should mention byte 216: {msg}");
    }

    #[test]
    fn parse_via_reserved_bytes_218_239_must_be_zero() {
        let mut w = write_extended_via_up_to_extension_region();
        // Bytes 0-8 (offsets 209-217): valid
        w.write_u8(0); // reserved_209
        w.write_u8(0); // is_testpoint_top
        w.write_u8(0); // is_testpoint_bottom
        w.write_u8(0); // is_assy_testpoint_top
        w.write_u8(0); // is_assy_testpoint_bottom
        w.write_u8(0); // solder_mask_override
        w.write_u8(0); // use_separate_solder_mask_expansion
        w.write_u8(0); // reserved_216
        w.write_u8(0); // solder_mask_expansion_from_hole_edge
        // Bytes 9-30 (offsets 218-239): put nonzero at offset 220
        w.write_u8(0);    // 218
        w.write_u8(0);    // 219
        w.write_u8(0xAB); // 220 = nonzero (should fail)
        for _ in 0..19 { w.write_u8(0); } // 221-239
        w.write_u8(0); // paste_mask_override (offset 240)
        w.write_u8(0); // solder_mask_expansion_linked
        w.write_coord(Coord::ZERO); // solder_mask_expansion_back

        let data = w.finish();
        let err = parse_via(&data).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("218-239"), "error should mention bytes 218-239: {msg}");
        assert!(msg.contains("220"), "error should mention offset 220: {msg}");
    }

    #[test]
    fn parse_via_extension_flags_roundtrip() {
        let mut w = write_extended_via_up_to_extension_region();
        // Set various flags to test roundtrip
        w.write_u8(0); // reserved_209
        w.write_u8(1); // is_testpoint_top = true
        w.write_u8(0); // is_testpoint_bottom = false
        w.write_u8(1); // is_assy_testpoint_top = true
        w.write_u8(0); // is_assy_testpoint_bottom = false
        w.write_u8(1); // solder_mask_override = true
        w.write_u8(1); // use_separate_solder_mask_expansion = true
        w.write_u8(0); // reserved_216
        w.write_u8(1); // solder_mask_expansion_from_hole_edge = true
        w.write_bytes(&[0u8; 22]); // reserved 218-239
        w.write_u8(1); // paste_mask_override = true
        w.write_u8(0); // solder_mask_expansion_linked
        w.write_coord(Coord::ZERO); // solder_mask_expansion_back

        let data = w.finish();
        let via = parse_via(&data).unwrap();
        assert!(via.is_testpoint_top);
        assert!(!via.is_testpoint_bottom);
        assert!(via.is_assy_testpoint_top);
        assert!(!via.is_assy_testpoint_bottom);
        assert!(via.solder_mask_override);
        assert!(via.use_separate_solder_mask_expansion);
        assert!(via.solder_mask_expansion_from_hole_edge);
        assert!(via.paste_mask_override);
    }
}
