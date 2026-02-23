#![allow(dead_code)]

use indexmap::IndexMap;

use crate::param_collection::ParameterCollection;
use crate::{AltiumFormatError, Result};

pub(crate) struct PcbBoardConfig {
    pub(crate) record: String,
    // V9
    pub(crate) v9_master_stack: Option<PcbMasterStack>,
    pub(crate) v9_substacks: Vec<PcbSubStack>,
    pub(crate) v9_stack_layers: Vec<PcbStackLayerEntry>,
    pub(crate) v9_cache_layers: Vec<PcbCacheLayerEntry>,
    // V8
    pub(crate) v8_master_stack: Option<PcbMasterStack>,
    pub(crate) v8_layers: Vec<PcbStackLayerEntry>,
    // V7
    pub(crate) v7_layers: Vec<PcbV7LayerEntry>,
    // Legacy
    pub(crate) legacy_layers: Vec<PcbLegacyLayerEntry>,
    // Surface
    pub(crate) surface_properties: PcbSurfaceProperties,
    // Misc
    pub(crate) layer_sets: Vec<PcbLayerSet>,
    pub(crate) grid_settings: PcbGridSettings,
    pub(crate) viewport: PcbViewportState,
    pub(crate) view_configs: PcbViewConfigs,
    pub(crate) snapping: PcbSnappingConfig,
    pub(crate) near_far_objects: PcbNearFarObjects,
    pub(crate) cfg2d: PcbCfg2D,
    pub(crate) cfg3d: IndexMap<String, String>,
    pub(crate) cfgall: PcbCfgAll,
    // Scalars
    pub(crate) display_unit: i32,
    pub(crate) current_2d_3d_view_state: String,
    pub(crate) toggle_layers: String,
    pub(crate) show_default_sets: bool,
    pub(crate) board_version: String,
    pub(crate) vault_guid: String,
    pub(crate) folder_guid: String,
    pub(crate) lifecycle_definition_guid: String,
    pub(crate) revision_naming_scheme_guid: String,
    pub(crate) lib_grid_sn_guide: String,
    pub(crate) unicode: String,
    pub(crate) unicode_filename: String,
}

pub(crate) struct PcbMasterStack {
    pub(crate) style: i32,
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) show_top_dielectric: bool,
    pub(crate) show_bottom_dielectric: bool,
    pub(crate) is_flex: bool,
}

pub(crate) struct PcbSubStack {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) show_top_dielectric: bool,
    pub(crate) show_bottom_dielectric: bool,
    pub(crate) is_flex: bool,
}

pub(crate) struct PcbStackLayerEntry {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) layer_id: i32,
    pub(crate) used_by_prims: bool,
    pub(crate) mech_enabled: Option<bool>,
    pub(crate) cop_thick: Option<String>,
    pub(crate) component_placement: Option<i32>,
    pub(crate) diel_type: Option<i32>,
    pub(crate) diel_const: Option<String>,
    pub(crate) diel_height: Option<String>,
    pub(crate) diel_material: Option<String>,
    pub(crate) coverlay_expansion: Option<String>,
    pub(crate) mech_kind: Option<String>,
}

pub(crate) struct PcbCacheLayerEntry {
    pub(crate) layer: PcbStackLayerEntry,
    pub(crate) pullback_distance: Option<String>,
}

pub(crate) struct PcbV7LayerEntry {
    pub(crate) layer_id: i32,
    pub(crate) name: String,
    pub(crate) prev: i32,
    pub(crate) next: i32,
    pub(crate) mech_enabled: bool,
    pub(crate) mech_kind: Option<String>,
    pub(crate) cop_thick: String,
    pub(crate) diel_type: i32,
    pub(crate) diel_const: String,
    pub(crate) diel_height: String,
    pub(crate) diel_material: String,
}

pub(crate) struct PcbLegacyLayerEntry {
    pub(crate) name: String,
    pub(crate) prev: i32,
    pub(crate) next: i32,
    pub(crate) mech_enabled: bool,
    pub(crate) mech_kind: Option<String>,
    pub(crate) cop_thick: String,
    pub(crate) diel_type: i32,
    pub(crate) diel_const: String,
    pub(crate) diel_height: String,
    pub(crate) diel_material: String,
}

pub(crate) struct PcbSurfaceProperties {
    pub(crate) top_type: String,
    pub(crate) top_const: String,
    pub(crate) top_height: String,
    pub(crate) top_material: String,
    pub(crate) bottom_type: String,
    pub(crate) bottom_const: String,
    pub(crate) bottom_height: String,
    pub(crate) bottom_material: String,
    pub(crate) layer_stack_style: String,
    pub(crate) show_top_dielectric: bool,
    pub(crate) show_bottom_dielectric: bool,
}

pub(crate) struct PcbLayerSet {
    pub(crate) name: String,
    pub(crate) layers: String,
    pub(crate) active_layer: String,
    pub(crate) is_current: bool,
    pub(crate) is_locked: bool,
    pub(crate) flip_board: bool,
}

pub(crate) struct PcbGridSettings {
    pub(crate) big_visible_grid_size: String,
    pub(crate) visible_grid_size: String,
    pub(crate) snap_grid_size: String,
    pub(crate) snap_grid_size_x: String,
    pub(crate) snap_grid_size_y: String,
    pub(crate) visible_grid_mult_factor: String,
    pub(crate) big_visible_grid_mult_factor: String,
    pub(crate) electrical_grid_range: String,
    pub(crate) electrical_grid_enabled: bool,
    pub(crate) dot_grid: bool,
    pub(crate) dot_grid_large: bool,
}

pub(crate) struct PcbViewportState {
    pub(crate) lx: String,
    pub(crate) hx: String,
    pub(crate) ly: String,
    pub(crate) hy: String,
    pub(crate) lookat_x: String,
    pub(crate) lookat_y: String,
    pub(crate) lookat_z: String,
    pub(crate) eye_rotation_x: String,
    pub(crate) eye_rotation_y: String,
    pub(crate) eye_rotation_z: String,
    pub(crate) zoom_mult: String,
    pub(crate) view_size_x: String,
    pub(crate) view_size_y: String,
}

pub(crate) struct PcbViewConfigs {
    pub(crate) config_2d_type: String,
    pub(crate) configuration_2d: String,
    pub(crate) config_2d_full_filename: String,
    pub(crate) config_3d_type: String,
    pub(crate) configuration_3d: String,
    pub(crate) config_3d_full_filename: String,
    pub(crate) board_insight_view_configuration_name: String,
}

pub(crate) struct PcbSnappingConfig {
    pub(crate) eg_range: String,
    pub(crate) eg_mult: String,
    pub(crate) eg_enabled: bool,
    pub(crate) eg_snap_to_board_outline: bool,
    pub(crate) eg_snap_to_arc_centers: bool,
    pub(crate) eg_use_all_layers: bool,
    pub(crate) og_snap_enabled: bool,
    pub(crate) mg_snap_enabled: bool,
    pub(crate) point_guide_enabled: bool,
    pub(crate) grid_snap_enabled: bool,
    pub(crate) snapping_entity_set: String,
}

pub(crate) struct PcbNearFarObjects {
    pub(crate) near_objects_enabled: bool,
    pub(crate) far_objects_enabled: bool,
    pub(crate) near_object_set: String,
    pub(crate) far_object_set: String,
    pub(crate) near_distance: String,
}

pub(crate) struct PcbCfg2D {
    pub(crate) prim_draw_mode: String,
    pub(crate) current_layer: String,
    pub(crate) display_special_strings: bool,
    pub(crate) show_test_points: bool,
    pub(crate) show_origin_marker: bool,
    pub(crate) eye_dist: String,
    pub(crate) show_status_info: bool,
    pub(crate) show_pad_nets: bool,
    pub(crate) show_pad_numbers: bool,
    pub(crate) show_via_nets: bool,
    pub(crate) show_via_span: bool,
    pub(crate) use_transparent_layers: bool,
    pub(crate) plane_draw_mode: String,
    pub(crate) display_net_names_on_tracks: String,
    pub(crate) from_tos_display_mode: String,
    pub(crate) pad_types_display_mode: String,
    pub(crate) single_layer_mode_state: String,
    pub(crate) origin_marker_color: String,
    pub(crate) show_component_ref_point: bool,
    pub(crate) component_ref_point_color: String,
    pub(crate) positive_top_solder_mask: bool,
    pub(crate) positive_bottom_solder_mask: bool,
    pub(crate) top_positive_solder_mask_alpha: String,
    pub(crate) bottom_positive_solder_mask_alpha: String,
    pub(crate) all_connections_in_single_layer_mode: bool,
    pub(crate) multi_colored_connections: bool,
    pub(crate) show_special_strings_handles: bool,
    pub(crate) toggle_layers: String,
    pub(crate) toggle_layers_set: String,
    pub(crate) mech_layer_in_single_layer_mode: String,
    pub(crate) mech_layer_in_single_layer_mode_set: String,
    pub(crate) layers_in_single_layer_mode_set: String,
    pub(crate) mech_layer_linked_to_sheet: String,
    pub(crate) mech_layer_linked_to_sheet_set: String,
    pub(crate) mech_coverlay_updated: bool,
    pub(crate) layer_opacity: IndexMap<String, String>,
    pub(crate) workspace_col_alpha: IndexMap<String, String>,
}

pub(crate) struct PcbCfgAll {
    pub(crate) configuration_kind: String,
    pub(crate) configuration_desc: String,
    pub(crate) component_body_ref_point_color: String,
    pub(crate) component_body_snap_point_color: String,
    pub(crate) show_component_snap_markers: bool,
    pub(crate) show_component_snap_reference: bool,
    pub(crate) show_component_snap_custom: bool,
}

pub(crate) fn parse_board_config(params: &mut ParameterCollection) -> Result<PcbBoardConfig> {
    let record = params.remove_optional::<String>("RECORD")?.unwrap_or_default();

    // 1. V9 master stack (probe STYLE as existence check)
    let v9_master_stack = if params.remove_optional::<String>("V9_MASTERSTACK_STYLE")?.is_some() {
        // The style was already consumed by the probe above; we can't re-read it.
        // Instead we rebuild by consuming remaining master stack fields.
        let id = params.remove_optional::<String>("V9_MASTERSTACK_ID")?.unwrap_or_default();
        let name = params.remove_optional::<String>("V9_MASTERSTACK_NAME")?.unwrap_or_default();
        let show_top_dielectric =
            params.remove_optional::<bool>("V9_MASTERSTACK_SHOWTOPDIELECTRIC")?.unwrap_or_default();
        let show_bottom_dielectric =
            params.remove_optional::<bool>("V9_MASTERSTACK_SHOWBOTTOMDIELECTRIC")?.unwrap_or_default();
        let is_flex =
            params.remove_optional::<bool>("V9_MASTERSTACK_ISFLEX")?.unwrap_or_default();
        Some(PcbMasterStack { style: 0, id, name, show_top_dielectric, show_bottom_dielectric, is_flex })
    } else {
        // style not present; consume the other optional master stack fields anyway
        let _ = params.remove_optional::<String>("V9_MASTERSTACK_ID")?;
        let _ = params.remove_optional::<String>("V9_MASTERSTACK_NAME")?;
        let _ = params.remove_optional::<bool>("V9_MASTERSTACK_SHOWTOPDIELECTRIC")?;
        let _ = params.remove_optional::<bool>("V9_MASTERSTACK_SHOWBOTTOMDIELECTRIC")?;
        let _ = params.remove_optional::<bool>("V9_MASTERSTACK_ISFLEX")?;
        None
    };

    // 2. V9 substacks (probe loop, 0-based)
    let mut v9_substacks = Vec::new();
    let mut idx: u32 = 0;
    loop {
        let id_key = format!("V9_SUBSTACK{idx}_ID");
        match params.remove_optional::<String>(&id_key)? {
            None => break,
            Some(id) => {
                let name = params
                    .remove_optional::<String>(&format!("V9_SUBSTACK{idx}_NAME"))?
                    .unwrap_or_default();
                let show_top_dielectric = params
                    .remove_optional::<bool>(&format!("V9_SUBSTACK{idx}_SHOWTOPDIELECTRIC"))?
                    .unwrap_or_default();
                let show_bottom_dielectric = params
                    .remove_optional::<bool>(&format!("V9_SUBSTACK{idx}_SHOWBOTTOMDIELECTRIC"))?
                    .unwrap_or_default();
                let is_flex = params
                    .remove_optional::<bool>(&format!("V9_SUBSTACK{idx}_ISFLEX"))?
                    .unwrap_or_default();
                v9_substacks.push(PcbSubStack { id, name, show_top_dielectric, show_bottom_dielectric, is_flex });
                idx += 1;
            }
        }
    }

    // 3. V9 stack layers (0-based)
    let mut v9_stack_layers = Vec::new();
    let mut idx: u32 = 0;
    loop {
        let id_key = format!("V9_STACK_LAYER{idx}_ID");
        if params.remove_optional::<String>(&id_key)?.is_none() {
            break;
        }
        let prefix = format!("V9_STACK_LAYER{idx}_");
        let layer = parse_stack_layer_fields_after_id(params, &prefix)?;
        v9_stack_layers.push(layer);
        idx += 1;
    }

    // 4. V9 cache layers (0-based)
    let mut v9_cache_layers = Vec::new();
    let mut idx: u32 = 0;
    loop {
        let id_key = format!("V9_CACHE_LAYER{idx}_ID");
        if params.remove_optional::<String>(&id_key)?.is_none() {
            break;
        }
        let prefix = format!("V9_CACHE_LAYER{idx}_");
        let layer = parse_stack_layer_fields_after_id(params, &prefix)?;
        let pullback_distance =
            params.remove_optional::<String>(&format!("{prefix}PULLBACKDISTANCE"))?;
        v9_cache_layers.push(PcbCacheLayerEntry { layer, pullback_distance });
        idx += 1;
    }

    // 5. V8 master stack (probe LAYERMASTERSTACK_V8STYLE)
    let v8_master_stack = if let Some(style_str) =
        params.remove_optional::<String>("LAYERMASTERSTACK_V8STYLE")?
    {
        let style: i32 = style_str
            .parse()
            .map_err(|_| AltiumFormatError::InvalidParamValue {
                key: "LAYERMASTERSTACK_V8STYLE".to_owned(),
                detail: format!("cannot parse '{style_str}' as i32"),
            })?;
        let id = params
            .remove_optional::<String>("LAYERMASTERSTACK_V8ID")?
            .unwrap_or_default();
        let name = params
            .remove_optional::<String>("LAYERMASTERSTACK_V8NAME")?
            .unwrap_or_default();
        let show_top_dielectric = params
            .remove_optional::<bool>("LAYERMASTERSTACK_V8SHOWTOPDIELECTRIC")?
            .unwrap_or_default();
        let show_bottom_dielectric = params
            .remove_optional::<bool>("LAYERMASTERSTACK_V8SHOWBOTTOMDIELECTRIC")?
            .unwrap_or_default();
        let is_flex = params
            .remove_optional::<bool>("LAYERMASTERSTACK_V8ISFLEX")?
            .unwrap_or_default();
        Some(PcbMasterStack { style, id, name, show_top_dielectric, show_bottom_dielectric, is_flex })
    } else {
        let _ = params.remove_optional::<String>("LAYERMASTERSTACK_V8ID")?;
        let _ = params.remove_optional::<String>("LAYERMASTERSTACK_V8NAME")?;
        let _ = params.remove_optional::<bool>("LAYERMASTERSTACK_V8SHOWTOPDIELECTRIC")?;
        let _ = params.remove_optional::<bool>("LAYERMASTERSTACK_V8SHOWBOTTOMDIELECTRIC")?;
        let _ = params.remove_optional::<bool>("LAYERMASTERSTACK_V8ISFLEX")?;
        None
    };

    // 6. V8 layers (0-based, no separator between index and field name)
    let mut v8_layers = Vec::new();
    let mut idx: u32 = 0;
    loop {
        let id_key = format!("LAYER_V8_{idx}ID");
        if params.remove_optional::<String>(&id_key)?.is_none() {
            break;
        }
        let prefix = format!("LAYER_V8_{idx}");
        let layer = parse_v8_layer_fields_after_id(params, &prefix)?;
        v8_layers.push(layer);
        idx += 1;
    }

    // 7. Consume any remaining V8 substack keys — error if non-empty
    let v8_substack_remaining = params.remove_prefixed("LAYERSUBSTACK_V8_");
    if !v8_substack_remaining.is_empty() {
        let keys: Vec<String> = v8_substack_remaining.into_keys().collect();
        return Err(AltiumFormatError::UnknownParams { keys });
    }

    // 8. V7 layers (0-based)
    let mut v7_layers = Vec::new();
    let mut idx: u32 = 0;
    loop {
        let id_key = format!("LAYERV7_{idx}LAYERID");
        match params.remove_optional::<i32>(&id_key)? {
            None => break,
            Some(layer_id) => {
                let name = params
                    .remove_optional::<String>(&format!("LAYERV7_{idx}NAME"))?
                    .unwrap_or_default();
                let prev = params
                    .remove_optional::<i32>(&format!("LAYERV7_{idx}PREV"))?
                    .unwrap_or_default();
                let next = params
                    .remove_optional::<i32>(&format!("LAYERV7_{idx}NEXT"))?
                    .unwrap_or_default();
                let mech_enabled = params
                    .remove_optional::<bool>(&format!("LAYERV7_{idx}MECHENABLED"))?
                    .unwrap_or_default();
                let mech_kind =
                    params.remove_optional::<String>(&format!("LAYERV7_{idx}MECHKIND"))?;
                let cop_thick = params
                    .remove_optional::<String>(&format!("LAYERV7_{idx}COPTHICK"))?
                    .unwrap_or_default();
                let diel_type = params
                    .remove_optional::<i32>(&format!("LAYERV7_{idx}DIELTYPE"))?
                    .unwrap_or_default();
                let diel_const = params
                    .remove_optional::<String>(&format!("LAYERV7_{idx}DIELCONST"))?
                    .unwrap_or_default();
                let diel_height = params
                    .remove_optional::<String>(&format!("LAYERV7_{idx}DIELHEIGHT"))?
                    .unwrap_or_default();
                let diel_material = params
                    .remove_optional::<String>(&format!("LAYERV7_{idx}DIELMATERIAL"))?
                    .unwrap_or_default();
                v7_layers.push(PcbV7LayerEntry {
                    layer_id,
                    name,
                    prev,
                    next,
                    mech_enabled,
                    mech_kind,
                    cop_thick,
                    diel_type,
                    diel_const,
                    diel_height,
                    diel_material,
                });
                idx += 1;
            }
        }
    }

    // 9. Legacy layers (1-based, 1-82)
    let mut legacy_layers = Vec::new();
    for n in 1u32..=82 {
        let name_key = format!("LAYER{n}NAME");
        match params.remove_optional::<String>(&name_key)? {
            None => continue,
            Some(name) => {
                let prev = params
                    .remove_optional::<i32>(&format!("LAYER{n}PREV"))?
                    .unwrap_or_default();
                let next = params
                    .remove_optional::<i32>(&format!("LAYER{n}NEXT"))?
                    .unwrap_or_default();
                let mech_enabled = params
                    .remove_optional::<bool>(&format!("LAYER{n}MECHENABLED"))?
                    .unwrap_or_default();
                let mech_kind = params.remove_optional::<String>(&format!("LAYER{n}MECHKIND"))?;
                let cop_thick = params
                    .remove_optional::<String>(&format!("LAYER{n}COPTHICK"))?
                    .unwrap_or_default();
                let diel_type = params
                    .remove_optional::<i32>(&format!("LAYER{n}DIELTYPE"))?
                    .unwrap_or_default();
                let diel_const = params
                    .remove_optional::<String>(&format!("LAYER{n}DIELCONST"))?
                    .unwrap_or_default();
                let diel_height = params
                    .remove_optional::<String>(&format!("LAYER{n}DIELHEIGHT"))?
                    .unwrap_or_default();
                let diel_material = params
                    .remove_optional::<String>(&format!("LAYER{n}DIELMATERIAL"))?
                    .unwrap_or_default();
                legacy_layers.push(PcbLegacyLayerEntry {
                    name,
                    prev,
                    next,
                    mech_enabled,
                    mech_kind,
                    cop_thick,
                    diel_type,
                    diel_const,
                    diel_height,
                    diel_material,
                });
            }
        }
    }

    // 10. Surface properties
    let surface_properties = PcbSurfaceProperties {
        top_type: params.remove_optional::<String>("TOPTYPE")?.unwrap_or_default(),
        top_const: params.remove_optional::<String>("TOPCONST")?.unwrap_or_default(),
        top_height: params.remove_optional::<String>("TOPHEIGHT")?.unwrap_or_default(),
        top_material: params.remove_optional::<String>("TOPMATERIAL")?.unwrap_or_default(),
        bottom_type: params.remove_optional::<String>("BOTTOMTYPE")?.unwrap_or_default(),
        bottom_const: params.remove_optional::<String>("BOTTOMCONST")?.unwrap_or_default(),
        bottom_height: params.remove_optional::<String>("BOTTOMHEIGHT")?.unwrap_or_default(),
        bottom_material: params.remove_optional::<String>("BOTTOMMATERIAL")?.unwrap_or_default(),
        layer_stack_style: params
            .remove_optional::<String>("LAYERSTACKSTYLE")?
            .unwrap_or_default(),
        show_top_dielectric: params
            .remove_optional::<bool>("SHOWTOPDIELECTRIC")?
            .unwrap_or_default(),
        show_bottom_dielectric: params
            .remove_optional::<bool>("SHOWBOTTOMDIELECTRIC")?
            .unwrap_or_default(),
    };

    // 11. Mech pairs (probe loop, 0-based)
    let mut mech_pair_idx: u32 = 0;
    loop {
        let key = format!("MECHPAIR{mech_pair_idx}L1");
        match params.remove_optional::<String>(&key)? {
            None => break,
            Some(_) => {
                let _ = params
                    .remove_optional::<String>(&format!("MECHPAIR{mech_pair_idx}L2"))?;
                mech_pair_idx += 1;
            }
        }
    }

    // 12. Layer sets (count-based via LAYERSETSCOUNT, 1-based)
    let layer_sets_count: usize =
        params.remove_optional::<i32>("LAYERSETSCOUNT")?.unwrap_or(0) as usize;
    let mut layer_sets = Vec::with_capacity(layer_sets_count);
    for n in 1..=layer_sets_count {
        let name = params
            .remove_optional::<String>(&format!("LAYERSET{n}NAME"))?
            .unwrap_or_default();
        let layers = params
            .remove_optional::<String>(&format!("LAYERSET{n}LAYERS"))?
            .unwrap_or_default();
        let active_layer = params
            .remove_optional::<String>(&format!("LAYERSET{n}ACTIVELAYER.7"))?
            .unwrap_or_default();
        let is_current = params
            .remove_optional::<bool>(&format!("LAYERSET{n}ISCURRENT"))?
            .unwrap_or_default();
        let is_locked = params
            .remove_optional::<bool>(&format!("LAYERSET{n}ISLOCKED"))?
            .unwrap_or_default();
        let flip_board = params
            .remove_optional::<bool>(&format!("LAYERSET{n}FLIPBOARD"))?
            .unwrap_or_default();
        layer_sets.push(PcbLayerSet { name, layers, active_layer, is_current, is_locked, flip_board });
    }

    // 13. Grid settings
    let grid_settings = PcbGridSettings {
        big_visible_grid_size: params
            .remove_optional::<String>("BIGVISIBLEGRIDSIZE")?
            .unwrap_or_default(),
        visible_grid_size: params
            .remove_optional::<String>("VISIBLEGRIDSIZE")?
            .unwrap_or_default(),
        snap_grid_size: params
            .remove_optional::<String>("SNAPGRIDSIZE")?
            .unwrap_or_default(),
        snap_grid_size_x: params
            .remove_optional::<String>("SNAPGRIDSIZEX")?
            .unwrap_or_default(),
        snap_grid_size_y: params
            .remove_optional::<String>("SNAPGRIDSIZEY")?
            .unwrap_or_default(),
        visible_grid_mult_factor: params
            .remove_optional::<String>("VISIBLEGRIDMULTFACTOR")?
            .unwrap_or_default(),
        big_visible_grid_mult_factor: params
            .remove_optional::<String>("BIGVISIBLEGRIDMULTFACTOR")?
            .unwrap_or_default(),
        electrical_grid_range: params
            .remove_optional::<String>("ELECTRICALGRIDRANGE")?
            .unwrap_or_default(),
        electrical_grid_enabled: params
            .remove_optional::<bool>("ELECTRICALGRIDENABLED")?
            .unwrap_or_default(),
        dot_grid: params.remove_optional::<bool>("DOTGRID")?.unwrap_or_default(),
        dot_grid_large: params.remove_optional::<bool>("DOTGRIDLARGE")?.unwrap_or_default(),
    };

    // 14. Viewport
    let viewport = PcbViewportState {
        lx: params.remove_optional::<String>("VP.LX")?.unwrap_or_default(),
        hx: params.remove_optional::<String>("VP.HX")?.unwrap_or_default(),
        ly: params.remove_optional::<String>("VP.LY")?.unwrap_or_default(),
        hy: params.remove_optional::<String>("VP.HY")?.unwrap_or_default(),
        lookat_x: params.remove_optional::<String>("LOOKAT.X")?.unwrap_or_default(),
        lookat_y: params.remove_optional::<String>("LOOKAT.Y")?.unwrap_or_default(),
        lookat_z: params.remove_optional::<String>("LOOKAT.Z")?.unwrap_or_default(),
        eye_rotation_x: params.remove_optional::<String>("EYEROTATION.X")?.unwrap_or_default(),
        eye_rotation_y: params.remove_optional::<String>("EYEROTATION.Y")?.unwrap_or_default(),
        eye_rotation_z: params.remove_optional::<String>("EYEROTATION.Z")?.unwrap_or_default(),
        zoom_mult: params.remove_optional::<String>("ZOOMMULT")?.unwrap_or_default(),
        view_size_x: params.remove_optional::<String>("VIEWSIZE.X")?.unwrap_or_default(),
        view_size_y: params.remove_optional::<String>("VIEWSIZE.Y")?.unwrap_or_default(),
    };

    // 15. View configs
    let view_configs = PcbViewConfigs {
        config_2d_type: params.remove_optional::<String>("2DCONFIGTYPE")?.unwrap_or_default(),
        configuration_2d: params.remove_optional::<String>("2DCONFIGURATION")?.unwrap_or_default(),
        config_2d_full_filename: params
            .remove_optional::<String>("2DCONFIGFULLFILENAME")?
            .unwrap_or_default(),
        config_3d_type: params.remove_optional::<String>("3DCONFIGTYPE")?.unwrap_or_default(),
        configuration_3d: params.remove_optional::<String>("3DCONFIGURATION")?.unwrap_or_default(),
        config_3d_full_filename: params
            .remove_optional::<String>("3DCONFIGFULLFILENAME")?
            .unwrap_or_default(),
        board_insight_view_configuration_name: params
            .remove_optional::<String>("BOARDINSIGHTVIEWCONFIGURATIONNAME")?
            .unwrap_or_default(),
    };

    // 16. Snapping
    let snapping = PcbSnappingConfig {
        eg_range: params.remove_optional::<String>("EGRANGE")?.unwrap_or_default(),
        eg_mult: params.remove_optional::<String>("EGMULT")?.unwrap_or_default(),
        eg_enabled: params.remove_optional::<bool>("EGENABLED")?.unwrap_or_default(),
        eg_snap_to_board_outline: params
            .remove_optional::<bool>("EGSNAPTOBOARDOUTLINE")?
            .unwrap_or_default(),
        eg_snap_to_arc_centers: params
            .remove_optional::<bool>("EGSNAPTOARCCENTERS")?
            .unwrap_or_default(),
        eg_use_all_layers: params
            .remove_optional::<bool>("EGUSEALLLAYERS")?
            .unwrap_or_default(),
        og_snap_enabled: params.remove_optional::<bool>("OGSNAPENABLED")?.unwrap_or_default(),
        mg_snap_enabled: params.remove_optional::<bool>("MGSNAPENABLED")?.unwrap_or_default(),
        point_guide_enabled: params
            .remove_optional::<bool>("POINTGUIDEENABLED")?
            .unwrap_or_default(),
        grid_snap_enabled: params.remove_optional::<bool>("GRIDSNAPENABLED")?.unwrap_or_default(),
        snapping_entity_set: params
            .remove_optional::<String>("SNAPPINGENTITYSET")?
            .unwrap_or_default(),
    };

    // 17. Near/far objects
    let near_far_objects = PcbNearFarObjects {
        near_objects_enabled: params
            .remove_optional::<bool>("NEAROBJECTSENABLED")?
            .unwrap_or_default(),
        far_objects_enabled: params
            .remove_optional::<bool>("FAROBJECTSENABLED")?
            .unwrap_or_default(),
        near_object_set: params.remove_optional::<String>("NEAROBJECTSET")?.unwrap_or_default(),
        far_object_set: params.remove_optional::<String>("FAROBJECTSET")?.unwrap_or_default(),
        near_distance: params.remove_optional::<String>("NEARDISTANCE")?.unwrap_or_default(),
    };

    // 18. CFG2D typed scalars
    let cfg2d_prim_draw_mode =
        params.remove_optional::<String>("CFG2D.PRIMDRAWMODE")?.unwrap_or_default();
    let cfg2d_current_layer =
        params.remove_optional::<String>("CFG2D.CURRENTLAYER")?.unwrap_or_default();
    let cfg2d_display_special_strings =
        params.remove_optional::<bool>("CFG2D.DISPLAYSPECIALSTRINGS")?.unwrap_or_default();
    let cfg2d_show_test_points =
        params.remove_optional::<bool>("CFG2D.SHOWTESTPOINTS")?.unwrap_or_default();
    let cfg2d_show_origin_marker =
        params.remove_optional::<bool>("CFG2D.SHOWORIGINMARKER")?.unwrap_or_default();
    let cfg2d_eye_dist =
        params.remove_optional::<String>("CFG2D.EYEDIST")?.unwrap_or_default();
    let cfg2d_show_status_info =
        params.remove_optional::<bool>("CFG2D.SHOWSTATUSINFO")?.unwrap_or_default();
    let cfg2d_show_pad_nets =
        params.remove_optional::<bool>("CFG2D.SHOWPADNETS")?.unwrap_or_default();
    let cfg2d_show_pad_numbers =
        params.remove_optional::<bool>("CFG2D.SHOWPADNUMBERS")?.unwrap_or_default();
    let cfg2d_show_via_nets =
        params.remove_optional::<bool>("CFG2D.SHOWVIANETS")?.unwrap_or_default();
    let cfg2d_show_via_span =
        params.remove_optional::<bool>("CFG2D.SHOWVIASPAN")?.unwrap_or_default();
    let cfg2d_use_transparent_layers =
        params.remove_optional::<bool>("CFG2D.USETRANSPARENTLAYERS")?.unwrap_or_default();
    let cfg2d_plane_draw_mode =
        params.remove_optional::<String>("CFG2D.PLANEDRAWMODE")?.unwrap_or_default();
    let cfg2d_display_net_names_on_tracks =
        params.remove_optional::<String>("CFG2D.DISPLAYNETNAMESONTRACKS")?.unwrap_or_default();
    let cfg2d_from_tos_display_mode =
        params.remove_optional::<String>("CFG2D.FROMTOSDISPLAYMODE")?.unwrap_or_default();
    let cfg2d_pad_types_display_mode =
        params.remove_optional::<String>("CFG2D.PADTYPESDISPLAYMODE")?.unwrap_or_default();
    let cfg2d_single_layer_mode_state =
        params.remove_optional::<String>("CFG2D.SINGLELAYERMODESTATE")?.unwrap_or_default();
    let cfg2d_origin_marker_color =
        params.remove_optional::<String>("CFG2D.ORIGINMARKERCOLOR")?.unwrap_or_default();
    let cfg2d_show_component_ref_point =
        params.remove_optional::<bool>("CFG2D.SHOWCOMPONENTREFPOINT")?.unwrap_or_default();
    let cfg2d_component_ref_point_color =
        params.remove_optional::<String>("CFG2D.COMPONENTREFPOINTCOLOR")?.unwrap_or_default();
    let cfg2d_positive_top_solder_mask =
        params.remove_optional::<bool>("CFG2D.POSITIVETOPSOLDERMASK")?.unwrap_or_default();
    let cfg2d_positive_bottom_solder_mask =
        params.remove_optional::<bool>("CFG2D.POSITIVEBOTTOMSOLDERMASK")?.unwrap_or_default();
    let cfg2d_top_positive_solder_mask_alpha =
        params.remove_optional::<String>("CFG2D.TOPPOSITIVESOLDERMASKALPHA")?.unwrap_or_default();
    let cfg2d_bottom_positive_solder_mask_alpha = params
        .remove_optional::<String>("CFG2D.BOTTOMPOSITIVESOLDERMASKALPHA")?
        .unwrap_or_default();
    let cfg2d_all_connections_in_single_layer_mode = params
        .remove_optional::<bool>("CFG2D.ALLCONNECTIONSINSINGLELAYERMODE")?
        .unwrap_or_default();
    let cfg2d_multi_colored_connections =
        params.remove_optional::<bool>("CFG2D.MULTICOLOREDCONNECTIONS")?.unwrap_or_default();
    let cfg2d_show_special_strings_handles =
        params.remove_optional::<bool>("CFG2D.SHOWSPECIALSTRINGSHANDLES")?.unwrap_or_default();
    let cfg2d_toggle_layers =
        params.remove_optional::<String>("CFG2D.TOGGLELAYERS")?.unwrap_or_default();
    let cfg2d_toggle_layers_set =
        params.remove_optional::<String>("CFG2D.TOGGLELAYERS.SET")?.unwrap_or_default();
    let cfg2d_mech_layer_in_single_layer_mode =
        params.remove_optional::<String>("CFG2D.MECHLAYERINSINGLELAYERMODE")?.unwrap_or_default();
    let cfg2d_mech_layer_in_single_layer_mode_set = params
        .remove_optional::<String>("CFG2D.MECHLAYERINSINGLELAYERMODE.SET")?
        .unwrap_or_default();
    let cfg2d_layers_in_single_layer_mode_set = params
        .remove_optional::<String>("CFG2D.LAYERSINSINGLELAYERMODE.SET")?
        .unwrap_or_default();
    let cfg2d_mech_layer_linked_to_sheet =
        params.remove_optional::<String>("CFG2D.MECHLAYERLINKEDTOSHEET")?.unwrap_or_default();
    let cfg2d_mech_layer_linked_to_sheet_set = params
        .remove_optional::<String>("CFG2D.MECHLAYERLINKEDTOSHEET.SET")?
        .unwrap_or_default();
    let cfg2d_mech_coverlay_updated = params
        .remove_optional::<bool>("CFG2D.MECHCOVERLAYERUPDATED")?
        .unwrap_or_default();

    // CFG2D indexed families
    let opacity_raw = params.remove_prefixed("CFG2D.LAYEROPACITY.");
    let prefix_len = "CFG2D.LAYEROPACITY.".len();
    let layer_opacity: IndexMap<String, String> = opacity_raw
        .into_iter()
        .map(|(k, v)| (k[prefix_len..].to_owned(), v))
        .collect();

    let workspace_raw = params.remove_prefixed("CFG2D.WORKSPACECOLALPHA");
    let prefix_len2 = "CFG2D.WORKSPACECOLALPHA".len();
    let workspace_col_alpha: IndexMap<String, String> = workspace_raw
        .into_iter()
        .map(|(k, v)| (k[prefix_len2..].to_owned(), v))
        .collect();

    let cfg2d = PcbCfg2D {
        prim_draw_mode: cfg2d_prim_draw_mode,
        current_layer: cfg2d_current_layer,
        display_special_strings: cfg2d_display_special_strings,
        show_test_points: cfg2d_show_test_points,
        show_origin_marker: cfg2d_show_origin_marker,
        eye_dist: cfg2d_eye_dist,
        show_status_info: cfg2d_show_status_info,
        show_pad_nets: cfg2d_show_pad_nets,
        show_pad_numbers: cfg2d_show_pad_numbers,
        show_via_nets: cfg2d_show_via_nets,
        show_via_span: cfg2d_show_via_span,
        use_transparent_layers: cfg2d_use_transparent_layers,
        plane_draw_mode: cfg2d_plane_draw_mode,
        display_net_names_on_tracks: cfg2d_display_net_names_on_tracks,
        from_tos_display_mode: cfg2d_from_tos_display_mode,
        pad_types_display_mode: cfg2d_pad_types_display_mode,
        single_layer_mode_state: cfg2d_single_layer_mode_state,
        origin_marker_color: cfg2d_origin_marker_color,
        show_component_ref_point: cfg2d_show_component_ref_point,
        component_ref_point_color: cfg2d_component_ref_point_color,
        positive_top_solder_mask: cfg2d_positive_top_solder_mask,
        positive_bottom_solder_mask: cfg2d_positive_bottom_solder_mask,
        top_positive_solder_mask_alpha: cfg2d_top_positive_solder_mask_alpha,
        bottom_positive_solder_mask_alpha: cfg2d_bottom_positive_solder_mask_alpha,
        all_connections_in_single_layer_mode: cfg2d_all_connections_in_single_layer_mode,
        multi_colored_connections: cfg2d_multi_colored_connections,
        show_special_strings_handles: cfg2d_show_special_strings_handles,
        toggle_layers: cfg2d_toggle_layers,
        toggle_layers_set: cfg2d_toggle_layers_set,
        mech_layer_in_single_layer_mode: cfg2d_mech_layer_in_single_layer_mode,
        mech_layer_in_single_layer_mode_set: cfg2d_mech_layer_in_single_layer_mode_set,
        layers_in_single_layer_mode_set: cfg2d_layers_in_single_layer_mode_set,
        mech_layer_linked_to_sheet: cfg2d_mech_layer_linked_to_sheet,
        mech_layer_linked_to_sheet_set: cfg2d_mech_layer_linked_to_sheet_set,
        mech_coverlay_updated: cfg2d_mech_coverlay_updated,
        layer_opacity,
        workspace_col_alpha,
    };

    // 19. CFG3D (all keys with prefix "CFG3D.")
    let cfg3d_raw = params.remove_prefixed("CFG3D.");
    let cfg3d_prefix_len = "CFG3D.".len();
    let cfg3d: IndexMap<String, String> = cfg3d_raw
        .into_iter()
        .map(|(k, v)| (k[cfg3d_prefix_len..].to_owned(), v))
        .collect();

    // 20. CFGALL
    let cfgall = PcbCfgAll {
        configuration_kind: params
            .remove_optional::<String>("CFGALL.CONFIGURATIONKIND")?
            .unwrap_or_default(),
        configuration_desc: params
            .remove_optional::<String>("CFGALL.CONFIGURATIONDESC")?
            .unwrap_or_default(),
        component_body_ref_point_color: params
            .remove_optional::<String>("CFGALL.COMPONENTBODYREFPOINTCOLOR")?
            .unwrap_or_default(),
        component_body_snap_point_color: params
            .remove_optional::<String>("CFGALL.COMPONENTBODYSNAPPOINTCOLOR")?
            .unwrap_or_default(),
        show_component_snap_markers: params
            .remove_optional::<bool>("CFGALL.SHOWCOMPONENTSNAPMARKERS")?
            .unwrap_or_default(),
        show_component_snap_reference: params
            .remove_optional::<bool>("CFGALL.SHOWCOMPONENTSNAPREFERENCE")?
            .unwrap_or_default(),
        show_component_snap_custom: params
            .remove_optional::<bool>("CFGALL.SHOWCOMPONENTSNAPCUSTOM")?
            .unwrap_or_default(),
    };

    // 21. Remaining scalars
    let display_unit =
        params.remove_optional::<i32>("DISPLAYUNIT")?.unwrap_or_default();
    let current_2d_3d_view_state =
        params.remove_optional::<String>("CURRENT2D3DVIEWSTATE")?.unwrap_or_default();
    let toggle_layers =
        params.remove_optional::<String>("TOGGLELAYERS")?.unwrap_or_default();
    let show_default_sets =
        params.remove_optional::<bool>("SHOWDEFAULTSETS")?.unwrap_or_default();
    let board_version =
        params.remove_optional::<String>("BOARDVERSION")?.unwrap_or_default();
    let vault_guid =
        params.remove_optional::<String>("VAULTGUID")?.unwrap_or_default();
    let folder_guid =
        params.remove_optional::<String>("FOLDERGUID")?.unwrap_or_default();
    let lifecycle_definition_guid =
        params.remove_optional::<String>("LIFECYCLEDEFINITIONGUID")?.unwrap_or_default();
    let revision_naming_scheme_guid =
        params.remove_optional::<String>("REVISIONNAMINGSCHEMEGUID")?.unwrap_or_default();
    let lib_grid_sn_guide =
        params.remove_optional::<String>("LIBGRIDSNGUIDE")?.unwrap_or_default();
    let unicode =
        params.remove_optional::<String>("UNICODE")?.unwrap_or_default();
    let unicode_filename =
        params.remove_optional::<String>("UNICODE__FILENAME")?.unwrap_or_default();

    Ok(PcbBoardConfig {
        record,
        v9_master_stack,
        v9_substacks,
        v9_stack_layers,
        v9_cache_layers,
        v8_master_stack,
        v8_layers,
        v7_layers,
        legacy_layers,
        surface_properties,
        layer_sets,
        grid_settings,
        viewport,
        view_configs,
        snapping,
        near_far_objects,
        cfg2d,
        cfg3d,
        cfgall,
        display_unit,
        current_2d_3d_view_state,
        toggle_layers,
        show_default_sets,
        board_version,
        vault_guid,
        folder_guid,
        lifecycle_definition_guid,
        revision_naming_scheme_guid,
        lib_grid_sn_guide,
        unicode,
        unicode_filename,
    })
}

/// Parse stack layer fields that come AFTER the ID has already been consumed.
/// Prefix includes the trailing separator, e.g. "V9_STACK_LAYER0_".
fn parse_stack_layer_fields_after_id(
    params: &mut ParameterCollection,
    prefix: &str,
) -> Result<PcbStackLayerEntry> {
    let name = params
        .remove_optional::<String>(&format!("{prefix}NAME"))?
        .unwrap_or_default();
    let layer_id = params
        .remove_optional::<i32>(&format!("{prefix}LAYERID"))?
        .unwrap_or_default();
    let used_by_prims = params
        .remove_optional::<bool>(&format!("{prefix}USEDBYPRIMS"))?
        .unwrap_or_default();
    let mech_enabled = params.remove_optional::<bool>(&format!("{prefix}MECHENABLED"))?;
    let cop_thick = params.remove_optional::<String>(&format!("{prefix}COPTHICK"))?;
    let component_placement =
        params.remove_optional::<i32>(&format!("{prefix}COMPONENTPLACEMENT"))?;
    let diel_type = params.remove_optional::<i32>(&format!("{prefix}DIELTYPE"))?;
    let diel_const = params.remove_optional::<String>(&format!("{prefix}DIELCONST"))?;
    let diel_height = params.remove_optional::<String>(&format!("{prefix}DIELHEIGHT"))?;
    let diel_material = params.remove_optional::<String>(&format!("{prefix}DIELMATERIAL"))?;
    let coverlay_expansion =
        params.remove_optional::<String>(&format!("{prefix}COVERLAY_EXPANSION"))?;
    let mech_kind = params.remove_optional::<String>(&format!("{prefix}MECHKIND"))?;
    Ok(PcbStackLayerEntry {
        id: String::new(),
        name,
        layer_id,
        used_by_prims,
        mech_enabled,
        cop_thick,
        component_placement,
        diel_type,
        diel_const,
        diel_height,
        diel_material,
        coverlay_expansion,
        mech_kind,
    })
}

/// Parse V8 layer fields after the ID has already been consumed.
/// Prefix has NO trailing separator: e.g. "LAYER_V8_0".
/// Field names are concatenated directly: LAYER_V8_0NAME, LAYER_V8_0LAYERID, etc.
fn parse_v8_layer_fields_after_id(
    params: &mut ParameterCollection,
    prefix: &str,
) -> Result<PcbStackLayerEntry> {
    let name = params
        .remove_optional::<String>(&format!("{prefix}NAME"))?
        .unwrap_or_default();
    let layer_id = params
        .remove_optional::<i32>(&format!("{prefix}LAYERID"))?
        .unwrap_or_default();
    let used_by_prims = params
        .remove_optional::<bool>(&format!("{prefix}USEDBYPRIMS"))?
        .unwrap_or_default();
    let mech_enabled = params.remove_optional::<bool>(&format!("{prefix}MECHENABLED"))?;
    let cop_thick = params.remove_optional::<String>(&format!("{prefix}COPTHICK"))?;
    let component_placement =
        params.remove_optional::<i32>(&format!("{prefix}COMPONENTPLACEMENT"))?;
    let diel_type = params.remove_optional::<i32>(&format!("{prefix}DIELTYPE"))?;
    let diel_const = params.remove_optional::<String>(&format!("{prefix}DIELCONST"))?;
    let diel_height = params.remove_optional::<String>(&format!("{prefix}DIELHEIGHT"))?;
    let diel_material = params.remove_optional::<String>(&format!("{prefix}DIELMATERIAL"))?;
    let coverlay_expansion =
        params.remove_optional::<String>(&format!("{prefix}COVERLAY_EXPANSION"))?;
    let mech_kind = params.remove_optional::<String>(&format!("{prefix}MECHKIND"))?;
    Ok(PcbStackLayerEntry {
        id: String::new(),
        name,
        layer_id,
        used_by_prims,
        mech_enabled,
        cop_thick,
        component_placement,
        diel_type,
        diel_const,
        diel_height,
        diel_material,
        coverlay_expansion,
        mech_kind,
    })
}
