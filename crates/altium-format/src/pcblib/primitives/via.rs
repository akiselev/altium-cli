use altium_format_types::pcb::TentingMode;
use altium_format_types::{Coord, MaskExpansionMode, PlaneConnectionStyle, TCacheState, V6Layer};

use crate::Result;
use crate::binary_io::BinaryReader;
use crate::pcblib::primitives::common::parse_common_header;
use crate::pcblib::{PcbVia, PcbViaSection2Entry};

/// Parses a Via primitive from its single PcbLib subrecord.
///
/// Parsed coverage:
/// - Core section (31-byte legacy and 246-byte extended formats)
/// - Section 2 layer-diameter overrides (stride=9 records)
///
/// Fail-fast behavior:
/// - Sections 3-5 are known to exist in AD26 but are not semantically mapped yet.
///   If present, parsing returns an error instead of preserving opaque bytes.
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

    let mut solder_mask_expansion_manual = false;
    let mut solder_mask_expansion_valid = TCacheState::Invalid;
    let mut power_plane_clearance_valid = TCacheState::Invalid;
    let mut planes_valid = TCacheState::Invalid;
    let mut plane_connection_style = PlaneConnectionStyle::NoConnect;
    let mut solder_mask_expansion_mode = MaskExpansionMode::NoMask;
    let mut paste_mask_expansion_mode = MaskExpansionMode::NoMask;
    let mut tenting_mode = TentingMode::None;

    let mut via_mode: u8 = 0;
    let mut diameters_per_layer = [Coord::ZERO; 32];

    let mut layer_enum_index = 0i32;
    let mut stack_start_layer = 0u8;
    let mut stack_end_layer = 0u8;
    let mut extension_coord_209 = Coord::ZERO;
    let mut extension_coord_213 = Coord::ZERO;
    let mut extension_coord_217 = Coord::ZERO;
    let mut extension_coord_221 = Coord::ZERO;
    let mut extension_coord_225 = Coord::ZERO;
    let mut extension_coord_229 = Coord::ZERO;
    let mut extension_coord_233 = Coord::ZERO;
    let mut extension_coord_237 = Coord::ZERO;

    let mut solder_mask_expansion_linked = false;
    let mut solder_mask_expansion_back = Coord::ZERO;

    let mut layer_diameter_overrides = Vec::new();

    let pos_tolerance = Coord::ZERO;
    let neg_tolerance = Coord::ZERO;

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

        let manual_byte = reader.read_u8()?;
        solder_mask_expansion_manual = (manual_byte & 0x02) != 0;

        solder_mask_expansion_valid = TCacheState::try_from(reader.read_u8()?)?;
        power_plane_clearance_valid = TCacheState::try_from(reader.read_u8()?)?;
        planes_valid = TCacheState::try_from(reader.read_u8()?)?;
        plane_connection_style = PlaneConnectionStyle::try_from(reader.read_u8()?)?;
        solder_mask_expansion_mode = MaskExpansionMode::try_from(reader.read_u8()?)?;
        paste_mask_expansion_mode = MaskExpansionMode::try_from(reader.read_u8()?)?;
        tenting_mode = TentingMode::try_from(reader.read_u8()?)?;

        via_mode = reader.read_u8()?;
        for d in &mut diameters_per_layer {
            *d = reader.read_coord()?;
        }

        if reader.remaining() >= 43 {
            layer_enum_index = reader.read_i32_le()?;
            stack_start_layer = reader.read_u8()?;
            stack_end_layer = reader.read_u8()?;
            extension_coord_209 = reader.read_coord()?;
            extension_coord_213 = reader.read_coord()?;
            extension_coord_217 = reader.read_coord()?;
            extension_coord_221 = reader.read_coord()?;
            extension_coord_225 = reader.read_coord()?;
            extension_coord_229 = reader.read_coord()?;
            extension_coord_233 = reader.read_coord()?;
            extension_coord_237 = reader.read_coord()?;
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

        if reader.remaining() > 0 {
            return Err(crate::AltiumFormatError::InvalidParamValue {
                key: "via.section3_5".to_owned(),
                detail: format!(
                    "unmapped AD26 Via sections 3-5 present: {} bytes remain",
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
        solder_mask_expansion_manual,
        solder_mask_expansion_valid,
        power_plane_clearance_valid,
        planes_valid,
        plane_connection_style,
        solder_mask_expansion_mode,
        paste_mask_expansion_mode,
        tenting_mode,
        via_mode,
        diameters_per_layer,
        layer_enum_index,
        stack_start_layer,
        stack_end_layer,
        extension_coord_209,
        extension_coord_213,
        extension_coord_217,
        extension_coord_221,
        extension_coord_225,
        extension_coord_229,
        extension_coord_233,
        extension_coord_237,
        solder_mask_expansion_linked,
        solder_mask_expansion_back,
        pos_tolerance,
        neg_tolerance,
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
        for _ in 0..8 {
            w.write_i32_le(0);
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
        assert!(via.solder_mask_expansion_manual);
        assert!(via.solder_mask_expansion_linked);
        assert_eq!(via.solder_mask_expansion_back.to_internal(), 2_000);
        assert_eq!(via.layer_enum_index, 123);
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
        for _ in 0..8 {
            w.write_i32_le(0);
        }
        w.write_u8(0);
        w.write_i32_le(0);

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
    fn parse_via_with_unmapped_sections_returns_error() {
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
        match parse_via(&data) {
            Err(AltiumFormatError::InvalidParamValue { key, .. }) => {
                assert_eq!(key, "via.section3_5");
            }
            Ok(_) => panic!("expected via.section3_5 error, got Ok"),
            Err(other) => panic!("expected via.section3_5 error, got {other}"),
        }
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
