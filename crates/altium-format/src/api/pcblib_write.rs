//! Write path: convert public API types → internal PcbLib types.

use crate::api::pcb_common::pcb_contour_to_internal;
use crate::api::pcblib_types::*;
use crate::pcblib::{
    PcbArc, PcbComponentBody, PcbFill, PcbFootprint, PcbPad, PcbPadCache, PcbPrimitive,
    PcbPrimitiveCommon, PcbRegion, PcbText, PcbTrack, PcbVia,
};
use crate::util::generate_unique_id;
use altium_format_types::coord::{Coord, CoordPoint};
use altium_format_types::pcb::{
    BarcodeRenderMode, DaisyChainStyle, LayerRef, MaskExpansionState, PadStackMode, PcbFlags,
    PlaneConnectionStyle, TCacheState, TextKind, V6Layer,
};

/// Build a `PcbPrimitiveCommon` for a library context (no net/polygon/component links).
fn default_primitive_common(layer: &LayerRef, flags: PcbFlags) -> PcbPrimitiveCommon {
    PcbPrimitiveCommon {
        layer: layer.to_v6().unwrap_or(V6Layer::NoLayer),
        flags,
        net_index: 0xFFFF,
        polygon_index: 0xFFFF,
        component_index: 0xFFFF,
        coordinate_index: 0xFFFF,
        dimension_index: 0xFFFF,
    }
}

/// Convert a public `Footprint` to a fresh internal `PcbFootprint`.
pub(crate) fn footprint_to_internal(fp: &Footprint, cfb_key: &str) -> PcbFootprint {
    let mut primitives: Vec<PcbPrimitive> = Vec::new();

    for pad in &fp.pads {
        primitives.push(PcbPrimitive::Pad(pad_to_internal(pad)));
    }

    for graphic in &fp.graphics {
        match graphic {
            PcbGraphic::Track(g) => primitives.push(PcbPrimitive::Track(track_to_internal(g))),
            PcbGraphic::Arc(g) => primitives.push(PcbPrimitive::Arc(arc_to_internal(g))),
            PcbGraphic::Fill(g) => primitives.push(PcbPrimitive::Fill(fill_to_internal(g))),
            PcbGraphic::Region(g) => primitives.push(PcbPrimitive::Region(region_to_internal(g))),
            PcbGraphic::Text(g) => primitives.push(PcbPrimitive::Text(text_to_internal(g))),
            PcbGraphic::Via(g) => primitives.push(PcbPrimitive::Via(via_to_internal(g))),
            PcbGraphic::ComponentBody(g) => {
                primitives.push(PcbPrimitive::ComponentBody(component_body_to_internal(g)))
            }
        }
    }

    PcbFootprint {
        display_name: fp.display_name.clone(),
        cfb_key: cfb_key.to_owned(),
        pattern: fp.pattern.clone(),
        height: fp.height,
        description: fp.description.clone(),
        item_guid: generate_unique_id(),
        revision_guid: generate_unique_id(),
        component_kind: None,
        primitives,
        extended_primitive_info: Vec::new(),
        primitive_guids: Vec::new(),
        custom_shapes: Vec::new(),
        custom_mask_shapes: Vec::new(),
        corner_radius_chamfer: Vec::new(),
        shared_unions: Vec::new(),
    }
}

/// Rebuild internal representation from a `Footprint`, preserving format-internal fields
/// from an existing `PcbFootprint`.
///
/// - Pads matched by `pad_name`: format-internal fields copied from existing.
/// - Graphics matched by `unique_id`: format-internal fields copied from existing.
/// - Existing `ComponentBody` primitives are preserved unchanged.
/// - Footprint-level format fields are copied from existing.
pub(crate) fn update_footprint_internal(fp: &Footprint, existing: &PcbFootprint) -> PcbFootprint {
    let mut updated = footprint_to_internal(fp, &existing.cfb_key);

    // Preserve footprint-level format-internal fields
    updated.item_guid = existing.item_guid.clone();
    updated.revision_guid = existing.revision_guid.clone();
    updated.component_kind = existing.component_kind;
    updated.extended_primitive_info = existing.extended_primitive_info.clone();
    updated.primitive_guids = existing.primitive_guids.clone();
    updated.custom_shapes = existing.custom_shapes.clone();
    updated.custom_mask_shapes = existing.custom_mask_shapes.clone();
    updated.corner_radius_chamfer = existing.corner_radius_chamfer.clone();
    updated.shared_unions = existing.shared_unions.clone();

    // Patch pads: match by pad_name and copy format-internal fields
    for prim in &mut updated.primitives {
        if let PcbPrimitive::Pad(new_pad) = prim {
            let existing_pad = existing.primitives.iter().find_map(|p| {
                if let PcbPrimitive::Pad(ep) = p {
                    if ep.pad_name == new_pad.pad_name {
                        return Some(ep);
                    }
                }
                None
            });

            if let Some(ep) = existing_pad {
                new_pad.unknown_sub1 = ep.unknown_sub1.clone();
                new_pad.unknown_sub2 = ep.unknown_sub2.clone();
                new_pad.unknown_sub3 = ep.unknown_sub3.clone();
                new_pad.daisy_chain_style = ep.daisy_chain_style;
                new_pad.unknown_63 = ep.unknown_63;
                // Copy cache validity flags from existing
                new_pad.cache.plane_connection_style_valid = ep.cache.plane_connection_style_valid;
                new_pad.cache.relief_conductor_width_valid = ep.cache.relief_conductor_width_valid;
                new_pad.cache.relief_entries_valid = ep.cache.relief_entries_valid;
                new_pad.cache.relief_air_gap_valid = ep.cache.relief_air_gap_valid;
                new_pad.cache.power_plane_relief_expansion_valid =
                    ep.cache.power_plane_relief_expansion_valid;
                new_pad.cache.paste_mask_expansion_valid = ep.cache.paste_mask_expansion_valid;
                new_pad.cache.solder_mask_expansion_valid = ep.cache.solder_mask_expansion_valid;
                new_pad.cache.power_plane_clearance_valid = ep.cache.power_plane_clearance_valid;
                new_pad.cache.planes_valid = ep.cache.planes_valid;
                new_pad.selection_memory_flags = ep.selection_memory_flags;
                new_pad.union_index = ep.union_index;
                new_pad.jumper_id = ep.jumper_id;
                new_pad.v7_layer_override = ep.v7_layer_override;
                new_pad.is_assy_testpoint_top = ep.is_assy_testpoint_top;
                new_pad.is_assy_testpoint_bottom = ep.is_assy_testpoint_bottom;
                new_pad.use_separate_expansions = ep.use_separate_expansions;
                new_pad.solder_mask_bottom_expansion = ep.solder_mask_bottom_expansion;
                new_pad.solder_mask_expansion_from_hole_edge =
                    ep.solder_mask_expansion_from_hole_edge;
                new_pad.template_link_library_id = ep.template_link_library_id;
                new_pad.template_link_template_id = ep.template_link_template_id;
                new_pad.pin_package_length = ep.pin_package_length;
                new_pad.hole_positive_tolerance = ep.hole_positive_tolerance;
                new_pad.hole_negative_tolerance = ep.hole_negative_tolerance;
                new_pad.reserved_170 = ep.reserved_170;
                new_pad.has_sub4_extension = ep.has_sub4_extension;
                new_pad.sub4_extension = ep.sub4_extension.clone();
                new_pad.thermal_reliefs = ep.thermal_reliefs.clone();
                new_pad.stack_data = ep.stack_data.clone();
            }
        }
    }

    // Patch graphics: match by unique_id and copy format-internal fields.
    // Currently no extra format-internal fields beyond unique_id for non-Pad primitives,
    // but the match loop below is the place to add them if needed.
    for prim in &mut updated.primitives {
        let uid = match prim {
            PcbPrimitive::Track(t) => t.unique_id.clone(),
            PcbPrimitive::Arc(a) => a.unique_id.clone(),
            PcbPrimitive::Fill(f) => f.unique_id.clone(),
            PcbPrimitive::Region(r) => r.unique_id.clone(),
            PcbPrimitive::Text(t) => t.unique_id.clone(),
            PcbPrimitive::Via(v) => v.unique_id.clone(),
            PcbPrimitive::Pad(_) | PcbPrimitive::ComponentBody(_) => continue,
        };

        if uid.is_none() {
            continue;
        }

        // Find matching existing primitive by unique_id
        let _existing_match = existing.primitives.iter().find(|ep| {
            let ep_uid = match ep {
                PcbPrimitive::Track(t) => &t.unique_id,
                PcbPrimitive::Arc(a) => &a.unique_id,
                PcbPrimitive::Fill(f) => &f.unique_id,
                PcbPrimitive::Region(r) => &r.unique_id,
                PcbPrimitive::Text(t) => &t.unique_id,
                PcbPrimitive::Via(v) => &v.unique_id,
                PcbPrimitive::Pad(_) | PcbPrimitive::ComponentBody(_) => return false,
            };
            *ep_uid == uid
        });
        // No additional format-internal fields to copy for these primitive types at this time.
    }

    // Collect unique_ids of ComponentBody graphics converted from the API type.
    let converted_body_uids: Vec<Option<String>> = updated
        .primitives
        .iter()
        .filter_map(|p| {
            if let PcbPrimitive::ComponentBody(cb) = p {
                Some(cb.unique_id.clone())
            } else {
                None
            }
        })
        .collect();

    // Preserve existing ComponentBody primitives not already represented by a
    // converted API body (matched by unique_id).
    for existing_prim in &existing.primitives {
        if let PcbPrimitive::ComponentBody(cb) = existing_prim {
            let already_converted = converted_body_uids
                .iter()
                .any(|uid| uid.is_some() && cb.unique_id.is_some() && uid == &cb.unique_id);
            if !already_converted {
                updated
                    .primitives
                    .push(PcbPrimitive::ComponentBody(cb.clone()));
            }
        }
    }

    updated
}

fn pad_to_internal(pad: &Pad) -> PcbPad {
    PcbPad {
        common: default_primitive_common(&pad.layer, PcbFlags::default()),
        pad_name: pad.pad_name.clone(),
        unknown_sub1: String::new(),
        unknown_sub2: String::new(),
        unknown_sub3: String::new(),
        location: pad.location,
        size_top: CoordPoint::new(pad.stack.top.x_size, pad.stack.top.y_size),
        size_mid: CoordPoint::new(pad.stack.mid.x_size, pad.stack.mid.y_size),
        size_bot: CoordPoint::new(pad.stack.bot.x_size, pad.stack.bot.y_size),
        hole_size: pad.hole_size,
        shape_top: pad.stack.top.shape,
        shape_mid: pad.stack.mid.shape,
        shape_bot: pad.stack.bot.shape,
        rotation: pad.rotation,
        is_plated: pad.is_plated,
        daisy_chain_style: DaisyChainStyle::default(),
        pad_mode: pad.pad_mode,
        unknown_63: 0,
        cache: PcbPadCache {
            plane_connection_style: pad.plane_connection,
            relief_conductor_width: pad.relief_conductor_width,
            relief_entries: pad.relief_entries as i16,
            relief_air_gap: pad.relief_air_gap,
            power_plane_relief_expansion: Coord::ZERO,
            power_plane_clearance: Coord::ZERO,
            paste_mask_expansion: pad.paste_mask_expansion,
            solder_mask_expansion: pad.solder_mask_expansion,
            planes: 0,
            plane_connection_style_valid: TCacheState::Valid,
            relief_conductor_width_valid: TCacheState::Valid,
            relief_entries_valid: TCacheState::Valid,
            relief_air_gap_valid: TCacheState::Valid,
            power_plane_relief_expansion_valid: TCacheState::default(),
            paste_mask_expansion_valid: TCacheState::Valid,
            solder_mask_expansion_valid: TCacheState::Valid,
            power_plane_clearance_valid: TCacheState::default(),
            planes_valid: TCacheState::default(),
        },
        selection_memory_flags: 0,
        union_index: 0,
        jumper_id: 0,
        v7_layer_override: 0,
        is_assy_testpoint_top: false,
        is_assy_testpoint_bottom: false,
        use_separate_expansions: false,
        solder_mask_bottom_expansion: 0,
        solder_mask_expansion_from_hole_edge: false,
        template_link_library_id: [0u8; 16],
        template_link_template_id: [0u8; 16],
        pin_package_length: Coord::ZERO,
        hole_positive_tolerance: 0,
        hole_negative_tolerance: 0,
        reserved_170: 0,
        has_sub4_extension: false,
        sub4_extension: None,
        thermal_reliefs: Vec::new(),
        stack_data: None,
        unique_id: pad.unique_id.clone(),
    }
}

fn track_to_internal(g: &TrackGraphic) -> PcbTrack {
    PcbTrack {
        common: default_primitive_common(&g.layer, g.flags),
        start: g.start,
        end: g.end,
        width: g.width,
        subpoly_index: 0,
        user_routed: false,
        union_index: 0,
        track_kind: 0,
        layer_enum_index: g.layer.v7().raw() as i32,
        keepout_restrictions: 0,
        unique_id: g.unique_id.clone(),
    }
}

fn arc_to_internal(g: &PcbArcGraphic) -> PcbArc {
    PcbArc {
        common: default_primitive_common(&g.layer, g.flags),
        center: g.center,
        radius: g.radius,
        start_angle: g.start_angle,
        end_angle: g.end_angle,
        width: g.width,
        subpoly_index: 0,
        user_routed: false,
        union_index: 0,
        v7_layer: g.layer.v7(),
        keepout_restrictions: 0,
        unique_id: g.unique_id.clone(),
    }
}

fn fill_to_internal(g: &FillGraphic) -> PcbFill {
    PcbFill {
        common: default_primitive_common(&g.layer, g.flags),
        corner1: g.corner1,
        corner2: g.corner2,
        rotation: g.rotation,
        user_routed: false,
        union_index: 0,
        v7_layer: g.layer.v7(),
        keepout_restrictions: 0,
        unique_id: g.unique_id.clone(),
    }
}

fn region_to_internal(g: &RegionGraphic) -> PcbRegion {
    let outline = pcb_contour_to_internal(&g.outline);
    let is_shape_based = matches!(&outline, crate::pcblib::Contour::ShapeBased(_));
    PcbRegion {
        common: default_primitive_common(&g.layer, g.flags),
        kind: g.kind,
        v7_layer: g.layer.display_name().unwrap_or("").to_owned(),
        name: String::new(),
        param_kind: 0,
        subpoly_index: 0,
        union_index: 0,
        arc_resolution: Coord::ZERO,
        is_shape_based,
        cavity_height: Coord::ZERO,
        keepout_restrictions: 0,
        layer: String::new(),
        keepout: false,
        is_board_cutout: false,
        pad_index: -1,
        object_kind: String::new(),
        bending_line_count: 0,
        locked_3d: false,
        layer_stack_id: String::new(),
        outline,
        holes: g.holes.iter().map(|h| pcb_contour_to_internal(h)).collect(),
        shape_text_segments: None,
        hole_shape_text_segments: Vec::new(),
        unique_id: g.unique_id.clone(),
    }
}

fn text_to_internal(g: &TextGraphic) -> PcbText {
    PcbText {
        common: default_primitive_common(&g.layer, g.flags),
        location: g.location,
        height: g.height,
        text_kind: TextKind::default(),
        rotation: g.rotation,
        is_mirrored: g.is_mirrored,
        stroke_width: g.width,
        is_italic: false,
        is_bold: false,
        font_name: g.font_name.clone(),
        inverted: false,
        inverted_tt_text_border: Coord::ZERO,
        wide_string_index: -1,
        union_index: 0,
        is_inverted_rect: false,
        ttf_text_width: Coord::ZERO,
        ttf_text_height: Coord::ZERO,
        font_id: 0,
        barcode_inverted: false,
        barcode_full_width: Coord::ZERO,
        barcode_full_height: Coord::ZERO,
        barcode_x_margin: Coord::ZERO,
        barcode_y_margin: Coord::ZERO,
        barcode_min_width: Coord::ZERO,
        barcode_show_text: false,
        barcode_render_mode: BarcodeRenderMode::ByMinWidth,
        multiline: false,
        barcode_font_name: String::new(),
        ttf_inverted_justify: None,
        ttf_offset_from_inverted_rect: None,
        tail_reserved_227: None,
        multiline_auto_position: None,
        is_advance_justification_valid: None,
        advance_snapping: None,
        tail_reserved_231: None,
        advance_justification_x: None,
        advance_justification_y: None,
        use_text_alignment_by_snap: None,
        snap_point_x: None,
        snap_point_y: None,
        text: g.text.clone(),
        unique_id: g.unique_id.clone(),
    }
}

fn via_to_internal(g: &ViaGraphic) -> PcbVia {
    PcbVia {
        common: default_primitive_common(&g.layer, g.flags),
        location: g.location,
        diameter: g.diameter,
        hole_size: g.hole_size,
        from_layer: g.from_layer.to_v6().unwrap_or(V6Layer::TopLayer),
        to_layer: g.to_layer.to_v6().unwrap_or(V6Layer::BottomLayer),
        via_properties_version: 0,
        thermal_relief_air_gap: Coord::ZERO,
        thermal_relief_conductor_count: 0,
        thermal_relief_rotation_code: 0,
        thermal_relief_conductor_width: Coord::ZERO,
        power_plane_relief_expansion: Coord::ZERO,
        power_plane_clearance: Coord::ZERO,
        paste_mask_expansion: Coord::ZERO,
        solder_mask_expansion_front: Coord::ZERO,
        planes: 0,
        plane_connection_style_valid: TCacheState::default(),
        relief_conductor_width_valid: TCacheState::default(),
        relief_entries_valid: TCacheState::default(),
        relief_air_gap_valid: TCacheState::default(),
        power_plane_relief_expansion_valid: TCacheState::default(),
        paste_mask_expansion_valid: TCacheState::default(),
        solder_mask_expansion_valid: TCacheState::default(),
        power_plane_clearance_valid: TCacheState::default(),
        planes_valid: TCacheState::default(),
        plane_connection_style: PlaneConnectionStyle::default(),
        solder_mask_cache_flags: 0,
        solder_mask_expansion_state: MaskExpansionState::default(),
        paste_mask_cache_flags: 0,
        paste_mask_expansion_state: MaskExpansionState::default(),
        via_mode: PadStackMode::Simple,
        diameters_per_layer: [Coord::ZERO; 32],
        layer_enum_index: 0,
        stack_start_layer: 0,
        stack_end_layer: 0,
        is_testpoint_top: g.is_testpoint_top,
        is_testpoint_bottom: g.is_testpoint_bottom,
        is_assy_testpoint_top: g.is_assy_testpoint_top,
        is_assy_testpoint_bottom: g.is_assy_testpoint_bottom,
        solder_mask_override: g.solder_mask_override,
        use_separate_solder_mask_expansion: g.use_separate_solder_mask_expansion,
        solder_mask_expansion_from_hole_edge: g.solder_mask_expansion_from_hole_edge,
        paste_mask_override: g.paste_mask_override,
        solder_mask_expansion_linked: false,
        solder_mask_expansion_back: Coord::ZERO,
        template_link_version: None,
        template_link_library_id: None,
        template_link_template_id: None,
        hole_positive_tolerance: None,
        hole_negative_tolerance: None,
        template_link_flags: None,
        pad_layer_entries: Vec::new(),
        pad_layer_stride: 0,
        counter_hole_angle: None,
        via_structure_type: None,
        layer_diameter_overrides: Vec::new(),
        unique_id: g.unique_id.clone(),
    }
}

fn component_body_to_internal(g: &ComponentBodyGraphic) -> PcbComponentBody {
    let outline = pcb_contour_to_internal(&g.outline);
    let is_shape_based = matches!(&outline, crate::pcblib::Contour::ShapeBased(_));
    PcbComponentBody {
        common: default_primitive_common(&g.layer, g.flags),
        v7_layer: String::new(),
        name: String::new(),
        kind: 0,
        subpoly_index: -1,
        union_index: 0,
        arc_resolution: Coord::ZERO,
        is_shape_based,
        cavity_height: Coord::ZERO,
        standoff_height: g.standoff_height,
        overall_height: g.overall_height,
        body_projection: 0,
        body_color_3d: g.body_color_3d,
        body_opacity_3d: g.body_opacity_3d,
        identifier: String::new(),
        texture: String::new(),
        texture_center_x: Coord::ZERO,
        texture_center_y: Coord::ZERO,
        texture_size_x: Coord::ZERO,
        texture_size_y: Coord::ZERO,
        texture_rotation: 0.0,
        body_override_color: false,
        model_guid: String::new(),
        model_checksum: String::new(),
        model_embed: false,
        model_name: g.model_name.clone(),
        model_2d_x: Coord::ZERO,
        model_2d_y: Coord::ZERO,
        model_2d_rotation: 0.0,
        rotation_x: 0.0,
        rotation_y: 0.0,
        rotation_z: 0.0,
        model_3d_dz: Coord::ZERO,
        model_type: 0,
        model_source: String::new(),
        model_snap_points: Vec::new(),
        model_extruded_min_z: Coord::ZERO,
        model_extruded_max_z: Coord::ZERO,
        model_cylinder_radius: Coord::ZERO,
        model_cylinder_height: Coord::ZERO,
        model_sphere_radius: Coord::ZERO,
        outline,
        shape_text_segments: None,
        unique_id: g.unique_id.clone(),
    }
}
