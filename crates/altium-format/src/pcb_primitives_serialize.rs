//! Shared primitive serialization functions used by both PcbLib and PcbDoc.
//!
//! These serialize the primitive types that are common between the two formats:
//! Via, Pad, Region, ComponentBody, plus shared helpers (common header, contours, format_mil).

use altium_format_types::{Coord, PcbObjectId, ViaStructureType};

use crate::binary_io::BinaryWriter;
use crate::pcblib::{
    PcbComponentBody, PcbPad, PcbPrimitiveCommon, PcbRegion, PcbVia, PolySegment,
};
use crate::pcblib::primitives::component_body::{encode_identifier, format_scientific_float};
use crate::Result;

/// Formats a `Coord` as an Altium mil string, stripping unnecessary trailing zeros.
///
/// Altium writes mil values with minimal precision: `0mil`, `0.5mil`, `47.744mil`.
/// Never `0.0000mil` or `0.5000mil`.
pub(crate) fn format_mil(coord: Coord) -> String {
    let mils = coord.to_mils();
    if mils == 0.0 {
        return "0mil".to_owned();
    }
    // Format with 4 decimal places, then strip trailing zeros.
    let formatted = format!("{:.4}", mils);
    let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
    format!("{}mil", trimmed)
}

/// Writes the 13-byte common header shared by all PCB primitives.
pub(crate) fn write_primitive_common(w: &mut BinaryWriter, c: &PcbPrimitiveCommon) {
    w.write_u8(c.layer as u8);
    w.write_u16_le(c.flags.raw());
    w.write_u16_le(c.net_index);
    w.write_u16_le(c.polygon_index);
    w.write_u16_le(c.component_index);
    w.write_u16_le(c.coordinate_index);
    w.write_u16_le(c.dimension_index);
}

/// Serialize a Via primitive to binary bytes.
///
/// Always writes the full extended format (core + extended properties + layer/flag
/// extension + all optional sections). Per "upgrade to latest format" philosophy,
/// even legacy 31-byte vias get upgraded to the full layout on save.
pub(crate) fn serialize_via(p: &PcbVia) -> Vec<u8> {
    let mut w = BinaryWriter::new();

    // Core section (31 bytes)
    write_primitive_common(&mut w, &p.common);
    w.write_coord_point(p.location);
    w.write_coord(p.diameter);
    w.write_coord(p.hole_size);
    w.write_u8(p.from_layer as u8);
    w.write_u8(p.to_layer as u8);

    // Extended properties section (172 bytes after core)
    w.write_u8(p.via_properties_version);
    w.write_coord(p.thermal_relief_air_gap);
    w.write_u8(p.thermal_relief_conductor_count);
    w.write_u8(p.thermal_relief_rotation_code);
    w.write_coord(p.thermal_relief_conductor_width);
    w.write_coord(p.power_plane_relief_expansion);
    w.write_coord(p.power_plane_clearance);
    w.write_coord(p.paste_mask_expansion);
    w.write_coord(p.solder_mask_expansion_front);
    w.write_u16_le(p.planes);
    w.write_u8(p.plane_connection_style_valid as u8);
    w.write_u8(p.relief_conductor_width_valid as u8);
    w.write_u8(p.relief_entries_valid as u8);
    w.write_u8(p.relief_air_gap_valid as u8);
    w.write_u8(p.power_plane_relief_expansion_valid as u8);
    w.write_u8(p.paste_mask_expansion_valid as u8);
    w.write_u8(p.solder_mask_expansion_valid as u8);
    w.write_u8(p.power_plane_clearance_valid as u8);
    w.write_u8(p.planes_valid as u8);
    w.write_u8(p.plane_connection_style as u8);
    w.write_u8(p.solder_mask_cache_flags);
    w.write_u8(p.solder_mask_expansion_state.as_u8());
    w.write_u8(p.paste_mask_cache_flags);
    w.write_u8(p.paste_mask_expansion_state.as_u8());
    w.write_u8(p.via_mode as u8);
    for d in &p.diameters_per_layer {
        w.write_coord(*d);
    }

    // Layer/flag extension (43 bytes)
    w.write_i32_le(p.layer_enum_index);
    w.write_u8(p.stack_start_layer);
    w.write_u8(p.stack_end_layer);
    // 32-byte extension region: individual boolean flags at specific offsets
    w.write_u8(0); // reserved_209
    w.write_u8(p.is_testpoint_top as u8);
    w.write_u8(p.is_testpoint_bottom as u8);
    w.write_u8(p.is_assy_testpoint_top as u8);
    w.write_u8(p.is_assy_testpoint_bottom as u8);
    w.write_u8(p.solder_mask_override as u8);
    w.write_u8(p.use_separate_solder_mask_expansion as u8);
    w.write_u8(0); // reserved_216
    w.write_u8(p.solder_mask_expansion_from_hole_edge as u8);
    w.write_bytes(&[0u8; 22]); // reserved_218_239
    w.write_u8(p.paste_mask_override as u8);
    let linked_byte = if p.solder_mask_expansion_linked { 0x01u8 } else { 0x00u8 };
    w.write_u8(linked_byte);
    w.write_coord(p.solder_mask_expansion_back);

    // Section 2: Layer-diameter overrides
    w.write_u32_le(p.layer_diameter_overrides.len() as u32);
    if p.layer_diameter_overrides.is_empty() {
        w.write_u32_le(0); // stride = 0 when count = 0
    } else {
        w.write_u32_le(9); // stride = 9
        for entry in &p.layer_diameter_overrides {
            w.write_u8(entry.layer);
            w.write_coord(entry.diameter);
            w.write_u16_le(entry.rule_index);
            w.write_u8(entry.flags);
            w.write_u8(entry.mode);
        }
    }

    // Template link block (size-prefixed, always latest ext_size=45)
    if let Some(version) = p.template_link_version {
        w.write_u32_le(45); // latest format: core(41) + flags(1) + trailing(3)
        w.write_u8(version);
        w.write_bytes(&p.template_link_library_id.unwrap_or([0u8; 16]));
        w.write_bytes(&p.template_link_template_id.unwrap_or([0u8; 16]));
        // Tolerances: None → i32::MAX (Delphi "not set" sentinel)
        w.write_i32_le(
            p.hole_positive_tolerance
                .map(|c| c.to_internal())
                .unwrap_or(i32::MAX),
        );
        w.write_i32_le(
            p.hole_negative_tolerance
                .map(|c| c.to_internal())
                .unwrap_or(i32::MAX),
        );
        w.write_u8(p.template_link_flags.unwrap_or(0));
        w.write_bytes(&[0u8; 3]); // trailing RevisionID (always zeros)
    }

    // Section 4: Per-layer pad stack entries (always written in latest format).
    // Uses stride 30 (latest) for new entries, preserves original stride for roundtrip.
    w.write_u32_le(p.pad_layer_entries.len() as u32);
    let stride = if p.pad_layer_entries.is_empty() {
        0u32
    } else if p.pad_layer_stride > 0 {
        p.pad_layer_stride
    } else {
        30 // default to latest stride
    };
    w.write_u32_le(stride);
    for entry in &p.pad_layer_entries {
        w.write_u32_le(entry.layer_id);
        w.write_u8(entry.shape as u8);
        w.write_u8(entry.mode as u8);
        w.write_coord(entry.solder_mask_expansion);
        match stride {
            30 => {
                w.write_coord(entry.paste_mask_expansion.unwrap_or(Coord::ZERO));
                w.write_u8(entry.plane_connection_style as u8);
                w.write_i16_le(entry.relief_entries as i16);
                w.write_u16_le(0); // reserved_17
                w.write_coord(entry.relief_conductor_width.unwrap_or(Coord::ZERO));
                w.write_u8(0); // reserved_23
                w.write_coord(entry.relief_air_gap.unwrap_or(Coord::ZERO));
                w.write_u16_le(0); // reserved_28
            }
            29 => {
                w.write_coord(entry.relief_conductor_width.unwrap_or(Coord::ZERO));
                w.write_u8(entry.plane_connection_style as u8);
                w.write_i16_le(entry.relief_entries as i16);
                w.write_u16_le(0); // reserved_17
                w.write_i32_le(0); // reserved_i32
                w.write_u8(0); // reserved_23
                w.write_coord(entry.relief_air_gap.unwrap_or(Coord::ZERO));
                w.write_u8(0); // reserved_28
            }
            23 | 24 => {
                w.write_coord(entry.relief_conductor_width.unwrap_or(Coord::ZERO));
                w.write_u8(entry.plane_connection_style as u8);
                w.write_i32_le(entry.relief_entries);
                let trailing_len = (stride - 19) as usize;
                for i in 0..trailing_len {
                    w.write_u8(((entry.trailing_flags >> (i * 8)) & 0xFF) as u8);
                }
            }
            _ => {
                // stride=0 means no entries, already handled by empty check
            }
        }
    }

    // Section 5: IPC-4761 / via structure (always written in latest format).
    if let Some(angle) = p.counter_hole_angle {
        w.write_u32_le(9); // section5_size
        w.write_f64_le(angle);
        w.write_u8(p.via_structure_type.unwrap_or(ViaStructureType::None) as u8);
    } else {
        // Placeholder form: 4-byte size + 4 zero bytes
        w.write_u32_le(4);
        w.write_u32_le(0);
    }

    w.finish()
}

/// Serialize a Pad primitive to 6 subrecords.
pub(crate) fn serialize_pad(p: &PcbPad) -> Result<Vec<Vec<u8>>> {
    let mut sub0 = BinaryWriter::new();
    sub0.write_pascal_string(&p.pad_name)?;
    let mut sub1 = BinaryWriter::new();
    sub1.write_pascal_string(&p.unknown_sub1)?;
    let mut sub2 = BinaryWriter::new();
    sub2.write_pascal_string(&p.unknown_sub2)?;
    let mut sub3 = BinaryWriter::new();
    sub3.write_pascal_string(&p.unknown_sub3)?;

    let mut sub4 = BinaryWriter::new();
    write_primitive_common(&mut sub4, &p.common);
    sub4.write_coord_point(p.location);
    sub4.write_coord(p.size_top.x);
    sub4.write_coord(p.size_top.y);
    sub4.write_coord(p.size_mid.x);
    sub4.write_coord(p.size_mid.y);
    sub4.write_coord(p.size_bot.x);
    sub4.write_coord(p.size_bot.y);
    sub4.write_coord(p.hole_size);
    sub4.write_u8(p.shape_top as u8);
    sub4.write_u8(p.shape_mid as u8);
    sub4.write_u8(p.shape_bot as u8);
    sub4.write_f64_le(p.rotation);
    sub4.write_u8(p.is_plated as u8);
    sub4.write_u8(p.daisy_chain_style as u8);
    sub4.write_u8(p.pad_mode as u8);
    sub4.write_i32_le(p.unknown_63);
    sub4.write_u8(p.cache.plane_connection_style as u8);
    sub4.write_coord(p.cache.relief_conductor_width);
    sub4.write_i16_le(p.cache.relief_entries);
    sub4.write_coord(p.cache.relief_air_gap);
    sub4.write_coord(p.cache.power_plane_relief_expansion);
    sub4.write_coord(p.cache.power_plane_clearance);
    sub4.write_coord(p.cache.paste_mask_expansion);
    sub4.write_coord(p.cache.solder_mask_expansion);
    sub4.write_u16_le(p.cache.planes);
    sub4.write_u8(p.cache.plane_connection_style_valid as u8);
    sub4.write_u8(p.cache.relief_conductor_width_valid as u8);
    sub4.write_u8(p.cache.relief_entries_valid as u8);
    sub4.write_u8(p.cache.relief_air_gap_valid as u8);
    sub4.write_u8(p.cache.power_plane_relief_expansion_valid as u8);
    sub4.write_u8(p.cache.paste_mask_expansion_valid as u8);
    sub4.write_u8(p.cache.solder_mask_expansion_valid as u8);
    sub4.write_u8(p.cache.power_plane_clearance_valid as u8);
    sub4.write_u8(p.cache.planes_valid as u8);
    sub4.write_u8(p.selection_memory_flags);
    sub4.write_i32_le(p.union_index);
    sub4.write_i32_le(p.jumper_id);
    sub4.write_i32_le(p.v7_layer_override);
    sub4.write_u8(p.is_assy_testpoint_top as u8);
    sub4.write_u8(p.is_assy_testpoint_bottom as u8);
    sub4.write_u8(p.use_separate_expansions as u8);
    sub4.write_i32_le(p.solder_mask_bottom_expansion);
    sub4.write_u8(p.solder_mask_expansion_from_hole_edge as u8);
    sub4.write_bytes(&p.template_link_library_id);
    sub4.write_bytes(&p.template_link_template_id);
    sub4.write_coord(p.pin_package_length);
    sub4.write_i32_le(p.hole_positive_tolerance);
    sub4.write_i32_le(p.hole_negative_tolerance);
    sub4.write_u8(p.reserved_170);
    sub4.write_u8(p.has_sub4_extension as u8);
    if let Some(ext) = &p.sub4_extension {
        sub4.write_u32_le(ext.header_len);
        let mut hdr = BinaryWriter::new();
        hdr.write_u32_le(ext.thermal_relief_count);
        hdr.write_f32_le(ext.propagation_delay_f32);
        hdr.write_u8(ext.flags8);
        hdr.write_u8(ext.flags9);
        hdr.write_f64_le(ext.propagation_delay_f64);
        hdr.write_coord(ext.x_pad_offset_all_layers);
        hdr.write_coord(ext.y_pad_offset_all_layers);
        let mut hdr_bytes = hdr.finish();
        hdr_bytes.truncate(ext.header_len as usize);
        sub4.write_bytes(&hdr_bytes);
        if !p.thermal_reliefs.is_empty() {
            sub4.write_u32_le(30);
            for relief in &p.thermal_reliefs {
                sub4.write_u32_le(relief.layer.raw());
                sub4.write_u8(relief.defined_type);
                sub4.write_u8(relief.connect_style as u8);
                sub4.write_coord(relief.air_gap_width);
                sub4.write_coord(relief.conductor_width);
                sub4.write_u8(relief.rotation as u8);
                sub4.write_u32_le(relief.entries);
                sub4.write_coord(relief.expansion);
                sub4.write_u8(relief.conductor_by_pad_edge as u8);
                sub4.write_coord(relief.min_distance);
                sub4.write_u8(relief.enable_min_distance as u8);
                sub4.write_u8(relief.use_custom_relief as u8);
            }
        }
    }

    let mut sub5 = BinaryWriter::new();
    if let Some(stack) = &p.stack_data {
        for v in stack.inner_size_x {
            sub5.write_coord(v);
        }
        for v in stack.inner_size_y {
            sub5.write_coord(v);
        }
        for v in stack.inner_shape {
            sub5.write_u8(v as u8);
        }
        sub5.write_u8(stack.padding_261);
        sub5.write_u8(stack.hole_shape as u8);
        sub5.write_coord(stack.slot_size);
        sub5.write_f64_le(stack.slot_rotation);
        for v in stack.hole_offset_x {
            sub5.write_coord(v);
        }
        for v in stack.hole_offset_y {
            sub5.write_coord(v);
        }
        sub5.write_u8(stack.padding_531);
        let alt_shape_bytes: [u8; 32] = std::array::from_fn(|i| stack.alt_shape[i] as u8);
        sub5.write_bytes(&alt_shape_bytes);
        sub5.write_bytes(&stack.corner_radius_pct);
        sub5.write_bytes(&stack.per_layer_overrides);
        if !stack.extended_cr.is_empty() {
            sub5.write_u32_le(stack.extended_cr.len() as u32);
            sub5.write_u32_le(15); // entry_size is always 15
            for entry in &stack.extended_cr {
                sub5.write_u32_le(entry.layer_id);
                sub5.write_u8(entry.alt_shape as u8);
                sub5.write_coord(entry.cr_pct_ex);
                sub5.write_coord(entry.cr_size);
                sub5.write_u8(entry.cr_pct);
                sub5.write_u8(entry.use_percent as u8);
            }
        }
    }

    Ok(vec![sub0.finish(), sub1.finish(), sub2.finish(), sub3.finish(), sub4.finish(), sub5.finish()])
}

/// Write a legacy contour (f64 coordinate pairs).
pub(crate) fn write_legacy_contour(w: &mut BinaryWriter, points: &[altium_format_types::CoordPoint]) {
    w.write_i32_le(points.len() as i32);
    for v in points {
        w.write_f64_le(v.x.to_internal() as f64);
        w.write_f64_le(v.y.to_internal() as f64);
    }
}

/// Write a polysegment (shape-based) contour.
pub(crate) fn write_polysegment_contour(w: &mut BinaryWriter, segments: &[PolySegment]) {
    // edge_count = vertex_count - 1 (N+1 vertices for N edges)
    let edge_count = segments.len().saturating_sub(1) as i32;
    w.write_i32_le(edge_count);
    for seg in segments {
        w.write_u8(seg.kind as u8);
        w.write_i32_le(seg.vertex.x.to_internal());
        w.write_i32_le(seg.vertex.y.to_internal());
        w.write_i32_le(seg.center.x.to_internal());
        w.write_i32_le(seg.center.y.to_internal());
        w.write_i32_le(seg.radius.to_internal());
        w.write_f64_le(seg.angle1);
        w.write_f64_le(seg.angle2);
    }
}

/// Write a contour (dispatches to legacy or polysegment format).
pub(crate) fn write_contour(w: &mut BinaryWriter, contour: &crate::pcblib::Contour) {
    match contour {
        crate::pcblib::Contour::Legacy(points) => write_legacy_contour(w, points),
        crate::pcblib::Contour::ShapeBased(segments) => write_polysegment_contour(w, segments),
    }
}

/// Serialize a Region primitive to binary bytes.
pub(crate) fn serialize_region(p: &PcbRegion) -> Vec<u8> {
    let mut w = BinaryWriter::new();
    write_primitive_common(&mut w, &p.common);
    w.write_u8(p.kind as u8);
    w.write_i32_le(p.holes.len() as i32);
    let mut params = crate::param_collection::ParameterCollection::new();
    params.insert("V7_LAYER", p.v7_layer.clone());
    params.insert("NAME", p.name.clone());
    params.insert("KIND", p.param_kind.to_string());
    params.insert("SUBPOLYINDEX", p.subpoly_index.to_string());
    params.insert("UNIONINDEX", p.union_index.to_string());
    params.insert("ARCRESOLUTION", format_mil(p.arc_resolution));
    params.insert("ISSHAPEBASED", if p.is_shape_based { "TRUE".to_owned() } else { "FALSE".to_owned() });
    params.insert("CAVITYHEIGHT", format_mil(p.cavity_height));
    params.insert("KEEPOUTRESTRICTIONS", p.keepout_restrictions.to_string());
    params.insert("LAYER", p.layer.clone());
    params.insert("KEEPOUT", if p.keepout { "TRUE".to_owned() } else { "FALSE".to_owned() });
    params.insert("ISBOARDCUTOUT", if p.is_board_cutout { "TRUE".to_owned() } else { "FALSE".to_owned() });
    params.insert("PADINDEX", p.pad_index.to_string());
    if !p.object_kind.is_empty() {
        params.insert("OBJECTKIND", p.object_kind.clone());
    }
    if p.bending_line_count != 0 || !p.object_kind.is_empty() {
        params.insert("BENDINGLINECOUNT", p.bending_line_count.to_string());
    }
    if p.locked_3d || !p.object_kind.is_empty() {
        params.insert("LOCKED3D", if p.locked_3d { "TRUE".to_owned() } else { "FALSE".to_owned() });
    }
    if !p.layer_stack_id.is_empty() {
        params.insert("LAYERSTACKID", p.layer_stack_id.clone());
    }
    let pbytes = params.to_bytes();
    w.write_u32_le(pbytes.len() as u32);
    w.write_bytes(&pbytes);
    write_contour(&mut w, &p.outline);
    for hole in &p.holes {
        write_contour(&mut w, hole);
    }
    w.finish()
}

/// Serialize a ComponentBody primitive to binary bytes.
pub(crate) fn serialize_component_body(p: &PcbComponentBody) -> Vec<u8> {
    let mut w = BinaryWriter::new();
    write_primitive_common(&mut w, &p.common);
    w.write_u8(0);
    w.write_i32_le(0);
    let mut params = crate::param_collection::ParameterCollection::new();
    params.insert("V7_LAYER", p.v7_layer.clone());
    params.insert("NAME", p.name.clone());
    params.insert("KIND", p.kind.to_string());
    params.insert("SUBPOLYINDEX", p.subpoly_index.to_string());
    params.insert("UNIONINDEX", p.union_index.to_string());
    params.insert("ARCRESOLUTION", format_mil(p.arc_resolution));
    params.insert("ISSHAPEBASED", if p.is_shape_based { "TRUE".to_owned() } else { "FALSE".to_owned() });
    params.insert("CAVITYHEIGHT", format_mil(p.cavity_height));
    params.insert("STANDOFFHEIGHT", format_mil(p.standoff_height));
    params.insert("OVERALLHEIGHT", format_mil(p.overall_height));
    params.insert("BODYPROJECTION", p.body_projection.to_string());
    params.insert("BODYCOLOR3D", p.body_color_3d.raw().to_string());
    params.insert("BODYOPACITY3D", format!("{:.3}", p.body_opacity_3d));
    params.insert("IDENTIFIER", encode_identifier(&p.identifier));
    params.insert("TEXTURE", p.texture.clone());
    params.insert("TEXTURECENTERX", format_mil(p.texture_center_x));
    params.insert("TEXTURECENTERY", format_mil(p.texture_center_y));
    params.insert("TEXTURESIZEX", format_mil(p.texture_size_x));
    params.insert("TEXTURESIZEY", format_mil(p.texture_size_y));
    params.insert("TEXTUREROTATION", format_scientific_float(p.texture_rotation));
    params.insert("BODYOVERRIDECOLOR", if p.body_override_color { "TRUE".to_owned() } else { "FALSE".to_owned() });
    params.insert("MODELID", p.model_guid.clone());
    params.insert("MODEL.CHECKSUM", p.model_checksum.clone());
    params.insert("MODEL.EMBED", if p.model_embed { "TRUE".to_owned() } else { "FALSE".to_owned() });
    params.insert("MODEL.NAME", p.model_name.clone());
    params.insert("MODEL.2D.X", format_mil(p.model_2d_x));
    params.insert("MODEL.2D.Y", format_mil(p.model_2d_y));
    params.insert("MODEL.2D.ROTATION", format!("{:.3}", p.model_2d_rotation));
    params.insert("MODEL.3D.ROTX", format!("{:.3}", p.rotation_x));
    params.insert("MODEL.3D.ROTY", format!("{:.3}", p.rotation_y));
    params.insert("MODEL.3D.ROTZ", format!("{:.3}", p.rotation_z));
    params.insert("MODEL.3D.DZ", format_mil(p.model_3d_dz));
    params.insert("MODEL.MODELTYPE", p.model_type.to_string());
    params.insert("MODEL.MODELSOURCE", p.model_source.clone());
    params.insert("MODEL.SNAPCOUNT", p.model_snap_points.len().to_string());
    for (i, (sx, sy, sz)) in p.model_snap_points.iter().enumerate() {
        params.insert(&format!("MODEL.S{}X", i), sx.to_internal().to_string());
        params.insert(&format!("MODEL.S{}Y", i), sy.to_internal().to_string());
        params.insert(&format!("MODEL.S{}Z", i), sz.to_internal().to_string());
    }
    if p.model_extruded_min_z != Coord::ZERO || p.model_extruded_max_z != Coord::ZERO {
        params.insert("MODEL.EXTRUDED.MINZ", format_mil(p.model_extruded_min_z));
        params.insert("MODEL.EXTRUDED.MAXZ", format_mil(p.model_extruded_max_z));
    }
    if p.model_cylinder_radius != Coord::ZERO || p.model_cylinder_height != Coord::ZERO {
        params.insert("MODEL.CYLINDER.RADIUS", format_mil(p.model_cylinder_radius));
        params.insert("MODEL.CYLINDER.HEIGHT", format_mil(p.model_cylinder_height));
    }
    if p.model_sphere_radius != Coord::ZERO {
        params.insert("MODEL.SPHERE.RADIUS", format_mil(p.model_sphere_radius));
    }
    let pbytes = params.to_bytes();
    w.write_u32_le(pbytes.len() as u32);
    w.write_bytes(&pbytes);
    write_contour(&mut w, &p.outline);
    w.finish()
}

/// Dispatch shared primitive serialization by object ID.
/// Returns (object_id, list_of_subrecord_bytes).
/// Only handles shared types (Via, Pad, Region, ComponentBody).
pub(crate) fn serialize_shared_primitive(obj: PcbObjectId, prim: &crate::pcblib::PcbPrimitive) -> Result<Vec<Vec<u8>>> {
    match prim {
        crate::pcblib::PcbPrimitive::Via(p) => Ok(vec![serialize_via(p)]),
        crate::pcblib::PcbPrimitive::Pad(p) => serialize_pad(p),
        crate::pcblib::PcbPrimitive::Region(p) => Ok(vec![serialize_region(p)]),
        crate::pcblib::PcbPrimitive::ComponentBody(p) => Ok(vec![serialize_component_body(p)]),
        _ => Err(crate::AltiumFormatError::InvalidParamValue {
            key: "pcb_primitives_serialize".to_owned(),
            detail: format!("{obj:?} is not a shared primitive type"),
        }),
    }
}
