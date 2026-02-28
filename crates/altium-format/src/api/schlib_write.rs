//! Write path: convert public API types → internal SchLib records.

use crate::api::schlib_types::*;
use crate::api::sch_common::{
    default_base, pin_to_internal, parameter_to_internal, graphic_to_record,
};
use crate::sch_records::{
    SchComponent, SchRecord,
    SchPrimitiveBase, SchDesignator,
    SchImplementationList, SchImplementation, SchImplementationMap, SchMapDefiner,
};
use crate::schlib::SchLibComponentIndex;
use crate::util::generate_unique_id;
use crate::Result;

use altium_format_types::color::Color;
use altium_format_types::common::{ComponentKind, RotationBy90};
use altium_format_types::coord::CoordPoint;
use altium_format_types::sch::{
    ParameterReadOnlyState, ParameterType,
    TextJustification, TextHorzAnchor, TextVertAnchor,
};

/// Convert a public `Component` to internal representation.
///
/// Returns: (SchComponent header, records vec, additional_records vec, SchLibComponentIndex)
pub(crate) fn component_to_internal(
    comp: &Component,
) -> Result<(SchComponent, Vec<SchRecord>, Vec<SchRecord>, SchLibComponentIndex)> {
    let mut records = Vec::new();

    // 1. Designator record (RECORD=34) if present
    if let Some(ref des_text) = comp.designator {
        records.push(SchRecord::Designator(SchDesignator {
            base: default_base(),
            location: CoordPoint::zero(),
            color: Color::new(0x00000080),
            font_id: 1,
            text: des_text.clone(),
            name: "Designator".to_owned(),
            is_hidden: false,
            orientation: RotationBy90::Rotate0,
            justification: TextJustification::BottomLeft,
            is_mirrored: false,
            unique_id: generate_unique_id(),
            show_name: false,
            read_only_state: ParameterReadOnlyState::Name,
            not_auto_position: false,
            override_not_auto_position: false,
            not_allow_library_synchronize: false,
            not_allow_database_synchronize: false,
            description: String::new(),
            param_type: ParameterType::String,
            text_horz_anchor: TextHorzAnchor::None,
            text_vert_anchor: TextVertAnchor::None,
            is_image_parameter: false,
        }));
    }

    // 2. Parameter records (RECORD=41)
    for param in &comp.parameters {
        records.push(SchRecord::Parameter(parameter_to_internal(param)));
    }

    // 3. Pin records (RECORD=2)
    for pin in &comp.pins {
        records.push(SchRecord::Pin(pin_to_internal(pin)));
    }

    // 4. Graphic records
    for graphic in &comp.graphics {
        records.push(graphic_to_record(graphic));
    }

    // 5. Implementation chain for footprint maps
    // Structure: ImplementationList → Implementation → ImplementationMap → MapDefiner(s)
    if !comp.footprints.is_empty() {
        // ImplementationList is always at owner_index=0 (root-owned)
        let impl_list_idx = records.len();
        records.push(SchRecord::ImplementationList(SchImplementationList {
            base: default_base(),
        }));

        for fp in &comp.footprints {
            // Implementation owned by ImplementationList
            let impl_idx = records.len();
            records.push(SchRecord::Implementation(SchImplementation {
                base: SchPrimitiveBase {
                    owner_index: (impl_list_idx + 1) as i32, // 1-based
                    ..default_base()
                },
                description: fp.description.clone(),
                use_component_library: true,
                model_name: fp.model_name.clone(),
                model_type: "PCBLIB".to_owned(),
                model_vault_guid: String::new(),
                model_item_guid: String::new(),
                model_revision_guid: String::new(),
                datafile_links: Vec::new(),
                is_current: fp.is_current,
                datalinks_locked: false,
                database_datalinks_locked: false,
                integrated_model: false,
                database_model: false,
                unique_id: generate_unique_id(),
                model_location: String::new(),
            }));

            // ImplementationMap owned by Implementation
            let map_idx = records.len();
            records.push(SchRecord::ImplementationMap(SchImplementationMap {
                base: SchPrimitiveBase {
                    owner_index: (impl_idx + 1) as i32,
                    ..default_base()
                },
                unique_id: generate_unique_id(),
            }));

            // MapDefiner records owned by ImplementationMap
            for ppm in &fp.pin_pad_maps {
                records.push(SchRecord::MapDefiner(SchMapDefiner {
                    base: SchPrimitiveBase {
                        owner_index: (map_idx + 1) as i32,
                        ..default_base()
                    },
                    des_intf: ppm.pin.clone(),
                    des_imps: if ppm.pad.is_empty() {
                        Vec::new()
                    } else {
                        vec![ppm.pad.clone()]
                    },
                }));
            }
        }
    }

    // Build the SchComponent header
    let sch_component = SchComponent {
        lib_reference: comp.lib_reference.clone(),
        component_description: comp.description.clone().unwrap_or_default(),
        part_count: comp.part_count,
        display_mode_count: 0,
        owner_index: 0,
        is_not_accessible: false,
        index_in_sheet: 0,
        owner_part_id: 0,
        owner_part_display_mode: 0,
        graphically_locked: false,
        union_index: 0,
        location: CoordPoint::zero(),
        display_mode: 0,
        is_mirrored: false,
        orientation: RotationBy90::Rotate0,
        current_part_id: 1,
        show_hidden_fields: false,
        show_hidden_pins: comp.show_hidden_pins,
        library_path: String::new(),
        source_library_name: String::new(),
        database_table_name: String::new(),
        sheet_part_file_name: String::new(),
        target_file_name: String::new(),
        unique_id: generate_unique_id(),
        area_color: Color::new(11_599_871),
        color: Color::new(12_800_000),
        pin_color: Color::new(8_388_608),
        override_colors: false,
        display_field_names: false,
        designator_locked: false,
        part_id_locked: false,
        pins_moveable: false,
        alias_list: String::new(),
        not_use_library_name: false,
        not_use_db_table_name: false,
        design_item_id: String::new(),
        vault_guid: String::new(),
        item_guid: String::new(),
        revision_guid: String::new(),
        symbol_vault_guid: String::new(),
        symbol_item_guid: String::new(),
        symbol_revision_guid: String::new(),
        generic_component_template_guid: String::new(),
        has_only_current_part_info: false,
        all_pin_count: comp.pins.len() as i32,
        key_component_unique_id: String::new(),
        component_kind: comp.component_kind.unwrap_or(ComponentKind::Standard),
        component_kind_version2: comp.component_kind.unwrap_or(ComponentKind::Standard),
        component_kind_version3: comp.component_kind.unwrap_or(ComponentKind::Standard),
        custom_display_mode_names: Vec::new(),
    };

    let index_entry = SchLibComponentIndex {
        lib_ref: comp.lib_reference.clone(),
        description: comp.description.clone().unwrap_or_default(),
        part_count: comp.part_count,
        aliases: comp.aliases.clone(),
    };

    Ok((sch_component, records, Vec::new(), index_entry))
}

/// Rebuild internal representation from a Component, preserving existing SchComponent fields.
pub(crate) fn update_component_internal(
    comp: &Component,
    existing: &SchComponent,
) -> Result<(SchComponent, Vec<SchRecord>, Vec<SchRecord>, SchLibComponentIndex)> {
    let (mut sch_comp, records, additional, index) = component_to_internal(comp)?;

    // Preserve format-internal fields from the existing component that the API doesn't expose
    sch_comp.unique_id = existing.unique_id.clone();
    sch_comp.area_color = existing.area_color;
    sch_comp.color = existing.color;
    sch_comp.pin_color = existing.pin_color;
    sch_comp.override_colors = existing.override_colors;
    sch_comp.display_field_names = existing.display_field_names;
    sch_comp.designator_locked = existing.designator_locked;
    sch_comp.part_id_locked = existing.part_id_locked;
    sch_comp.pins_moveable = existing.pins_moveable;
    sch_comp.library_path = existing.library_path.clone();
    sch_comp.source_library_name = existing.source_library_name.clone();
    sch_comp.database_table_name = existing.database_table_name.clone();
    sch_comp.design_item_id = existing.design_item_id.clone();
    sch_comp.vault_guid = existing.vault_guid.clone();
    sch_comp.item_guid = existing.item_guid.clone();
    sch_comp.revision_guid = existing.revision_guid.clone();
    sch_comp.symbol_vault_guid = existing.symbol_vault_guid.clone();
    sch_comp.symbol_item_guid = existing.symbol_item_guid.clone();
    sch_comp.symbol_revision_guid = existing.symbol_revision_guid.clone();
    sch_comp.generic_component_template_guid = existing.generic_component_template_guid.clone();
    sch_comp.display_mode_count = existing.display_mode_count;
    sch_comp.has_only_current_part_info = existing.has_only_current_part_info;
    sch_comp.key_component_unique_id = existing.key_component_unique_id.clone();
    sch_comp.custom_display_mode_names = existing.custom_display_mode_names.clone();

    Ok((sch_comp, records, additional, index))
}
