use altium_format_types::{Coord, PlaneConnectionStyle, TCacheState, V6Layer, ViaStructureType};

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
    let shape = reader.read_u8()?;
    let mode = reader.read_u8()?;
    let solder_mask_expansion = reader.read_coord()?;

    match stride {
        30 => {
            let paste_mask_expansion = reader.read_coord()?;
            let plane_connection_style = reader.read_u8()?;
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
            let plane_connection_style = reader.read_u8()?;
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
            let plane_connection_style = reader.read_u8()?;
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
    let mut solder_mask_expansion_mode: u8 = 0;
    let mut paste_mask_cache_flags: u8 = 0;
    let mut paste_mask_expansion_mode: u8 = 0;

    let mut via_mode: u8 = 0;
    let mut diameters_per_layer = [Coord::ZERO; 32];

    let mut layer_enum_index = 0i32;
    let mut stack_start_layer = 0u8;
    let mut stack_end_layer = 0u8;
    let mut removed_pads_per_layer = [false; 32];

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
        solder_mask_expansion_mode = reader.read_u8()?;
        paste_mask_cache_flags = reader.read_u8()?;
        paste_mask_expansion_mode = reader.read_u8()?;

        via_mode = reader.read_u8()?;
        for d in &mut diameters_per_layer {
            *d = reader.read_coord()?;
        }

        if reader.remaining() >= 43 {
            layer_enum_index = reader.read_i32_le()?;
            stack_start_layer = reader.read_u8()?;
            stack_end_layer = reader.read_u8()?;
            for flag in &mut removed_pads_per_layer {
                *flag = reader.read_u8()? != 0;
            }
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
            hole_positive_tolerance = Some(Coord::from_internal(ext.read_i32_le()?));
            hole_negative_tolerance = Some(Coord::from_internal(ext.read_i32_le()?));
            if ext.remaining() >= 1 {
                template_link_flags = Some(ext.read_u8()?);
            }
            // ext_size=45 has 3 additional bytes (purpose unknown, skip them).
            if ext.remaining() > 0 {
                let _extra = ext.read_bytes(ext.remaining())?;
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
        removed_pads_per_layer,
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
        // 32 removed_pads_per_layer booleans (one per V6 layer 1-32).
        // Set layers 2, 5 as removed to test non-zero values.
        for i in 0..32u8 {
            w.write_u8(if i == 1 || i == 4 { 1 } else { 0 });
        }
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
        // Verify removed_pads_per_layer
        assert!(!via.removed_pads_per_layer[0]);
        assert!(via.removed_pads_per_layer[1]);
        assert!(!via.removed_pads_per_layer[2]);
        assert!(!via.removed_pads_per_layer[3]);
        assert!(via.removed_pads_per_layer[4]);
        assert!(!via.removed_pads_per_layer[31]);
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
        w.write_bytes(&[0u8; 32]); // removed_pads_per_layer (all false)
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
}
