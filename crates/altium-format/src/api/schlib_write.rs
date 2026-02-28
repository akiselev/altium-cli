//! Write path: convert public API types → internal SchLib records.

use crate::api::schlib_types::*;
use crate::sch_records::{
    SchComponent, SchRecord,
    SchPrimitiveBase, SchDesignator, SchParameter as InternalParameter,
    SchPin as InternalPin,
    SchLine, SchRectangle, SchRoundRectangle, SchArc, SchEllipticalArc,
    SchEllipse, SchPie, SchPolyline, SchPolygon, SchBezier,
    SchImage, SchLabel, SchTextFrame,
    SchImplementationList, SchImplementation, SchImplementationMap, SchMapDefiner,
    PinTextPositioning as InternalPinTextPositioning,
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

fn default_base() -> SchPrimitiveBase {
    SchPrimitiveBase {
        owner_index: 0,
        is_not_accessible: false,
        index_in_sheet: 0,
        owner_part_id: 0,
        owner_part_display_mode: 0,
        selection_memory: 0,
        graphically_locked: false,
        union_index: 0,
        style_id: 0,
    }
}

fn pin_to_internal(pin: &Pin) -> InternalPin {
    InternalPin {
        owner_index: 0,
        owner_part_id: pin.owner_part_id,
        owner_part_display_mode: pin.owner_part_display_mode,
        symbol_inner_edge: pin.symbol_inner_edge,
        symbol_outer_edge: pin.symbol_outer_edge,
        symbol_inside: pin.symbol_inside,
        symbol_outside: pin.symbol_outside,
        symbol_inner_edge_present: pin.symbol_inner_edge != altium_format_types::sch::IeeeSymbol::NoSymbol,
        symbol_outer_edge_present: pin.symbol_outer_edge != altium_format_types::sch::IeeeSymbol::NoSymbol,
        symbol_inside_present: pin.symbol_inside != altium_format_types::sch::IeeeSymbol::NoSymbol,
        symbol_outside_present: pin.symbol_outside != altium_format_types::sch::IeeeSymbol::NoSymbol,
        symbol: None,
        description: pin.description.clone(),
        formal_type: pin.formal_type,
        electrical: pin.electrical,
        pin_length: pin.length,
        location: pin.location,
        color: pin.color,
        name: pin.name.clone(),
        designator: pin.designator.clone(),
        swap_id_pin: pin.swap_id_pin.clone(),
        swap_id_part: pin.swap_id_part.clone(),
        swap_id_pair: pin.swap_id_pair.clone(),
        default_value: pin.default_value.clone(),
        spice_pin_name: pin.spice_pin_name.clone(),
        hidden_net_name: pin.hidden_net_name.clone(),
        unique_id: pin.unique_id.clone(),
        orientation: pin.orientation,
        is_hidden: pin.is_hidden,
        show_name: pin.show_name,
        show_designator: pin.show_designator,
        is_not_accessible: pin.is_not_accessible,
        graphically_locked: pin.graphically_locked,
        owner_index_additional_list: false,
        pin_symbol_line_width: pin.pin_symbol_line_width,
        pin_package_length: pin.pin_package_length.clone(),
        propagation_delay: pin.propagation_delay.clone(),
        selected_functions: Vec::new(),
        defined_functions: Vec::new(),
        name_text_data: pin.name_text_data.as_ref().map(pin_text_to_internal),
        designator_text_data: pin.designator_text_data.as_ref().map(pin_text_to_internal),
    }
}

fn pin_text_to_internal(ptd: &PinTextPositioning) -> InternalPinTextPositioning {
    InternalPinTextPositioning {
        position_mode_custom: ptd.position_mode_custom,
        rotation_anchor_component: ptd.rotation_anchor_component,
        rotation_relative: ptd.rotation_relative,
        font_mode_custom: ptd.font_mode_custom,
        custom_position_margin: ptd.custom_position_margin,
        custom_font_id: ptd.custom_font_id,
        custom_color: ptd.custom_color,
    }
}

fn parameter_to_internal(param: &Parameter) -> InternalParameter {
    InternalParameter {
        base: default_base(),
        location: param.location,
        orientation: param.orientation,
        justification: param.justification,
        color: param.color,
        font_id: param.font_id,
        is_hidden: param.is_hidden,
        text: param.text.clone(),
        param_type: param.param_type,
        name: param.name.clone(),
        show_name: param.show_name,
        read_only_state: param.read_only,
        unique_id: param.unique_id.clone(),
        description: param.description.clone(),
        not_allow_library_synchronize: false,
        not_allow_database_synchronize: false,
        not_auto_position: param.not_auto_position,
        override_not_auto_position: false,
        is_mirrored: param.is_mirrored,
        text_horz_anchor: TextHorzAnchor::None,
        text_vert_anchor: TextVertAnchor::None,
        is_image_parameter: false,
    }
}

fn graphic_to_record(graphic: &Graphic) -> SchRecord {
    match graphic {
        Graphic::Line(g) => SchRecord::Line(SchLine {
            base: SchPrimitiveBase {
                owner_part_id: g.owner_part_id,
                ..default_base()
            },
            location: g.location,
            corner: g.corner,
            line_width: g.line_width,
            line_style: g.line_style,
            color: g.color,
            line_style_ext: g.line_style,
            unique_id: g.unique_id.clone(),
        }),
        Graphic::Rectangle(g) => SchRecord::Rectangle(SchRectangle {
            base: SchPrimitiveBase {
                owner_part_id: g.owner_part_id,
                ..default_base()
            },
            location: g.location,
            corner: g.corner,
            line_style: g.line_style,
            line_width: g.line_width,
            color: g.color,
            area_color: g.area_color,
            is_solid: g.is_solid,
            transparent: g.transparent,
            unique_id: g.unique_id.clone(),
        }),
        Graphic::RoundRectangle(g) => SchRecord::RoundRectangle(SchRoundRectangle {
            base: SchPrimitiveBase {
                owner_part_id: g.owner_part_id,
                ..default_base()
            },
            location: g.location,
            corner: g.corner,
            corner_x_radius: g.corner_x_radius,
            corner_y_radius: g.corner_y_radius,
            line_width: g.line_width,
            color: g.color,
            area_color: g.area_color,
            is_solid: g.is_solid,
            unique_id: g.unique_id.clone(),
        }),
        Graphic::Arc(g) => SchRecord::Arc(SchArc {
            base: SchPrimitiveBase {
                owner_part_id: g.owner_part_id,
                ..default_base()
            },
            location: g.location,
            radius: g.radius,
            line_width: g.line_width,
            start_angle: g.start_angle,
            end_angle: g.end_angle,
            color: g.color,
            unique_id: g.unique_id.clone(),
        }),
        Graphic::EllipticalArc(g) => SchRecord::EllipticalArc(SchEllipticalArc {
            base: SchPrimitiveBase {
                owner_part_id: g.owner_part_id,
                ..default_base()
            },
            location: g.location,
            radius: g.radius,
            secondary_radius: g.secondary_radius,
            line_width: g.line_width,
            start_angle: g.start_angle,
            end_angle: g.end_angle,
            color: g.color,
            unique_id: g.unique_id.clone(),
        }),
        Graphic::Ellipse(g) => SchRecord::Ellipse(SchEllipse {
            base: SchPrimitiveBase {
                owner_part_id: g.owner_part_id,
                ..default_base()
            },
            location: g.location,
            radius: g.radius,
            secondary_radius: g.secondary_radius,
            line_width: g.line_width,
            color: g.color,
            area_color: g.area_color,
            is_solid: g.is_solid,
            transparent: g.transparent,
            unique_id: g.unique_id.clone(),
        }),
        Graphic::Pie(g) => SchRecord::Pie(SchPie {
            base: SchPrimitiveBase {
                owner_part_id: g.owner_part_id,
                ..default_base()
            },
            location: g.location,
            radius: g.radius,
            line_width: g.line_width,
            start_angle: g.start_angle,
            end_angle: g.end_angle,
            color: g.color,
            area_color: g.area_color,
            is_solid: g.is_solid,
        }),
        Graphic::Polyline(g) => SchRecord::Polyline(SchPolyline {
            base: SchPrimitiveBase {
                owner_part_id: g.owner_part_id,
                ..default_base()
            },
            line_width: g.line_width,
            line_style: g.line_style,
            start_line_shape: g.start_line_shape,
            end_line_shape: g.end_line_shape,
            line_shape_size: g.line_shape_size,
            color: g.color,
            vertices: g.vertices.clone(),
            line_style_ext: g.line_style,
            unique_id: g.unique_id.clone(),
        }),
        Graphic::Polygon(g) => SchRecord::Polygon(SchPolygon {
            base: SchPrimitiveBase {
                owner_part_id: g.owner_part_id,
                ..default_base()
            },
            line_width: g.line_width,
            color: g.color,
            area_color: g.area_color,
            is_solid: g.is_solid,
            transparent: g.transparent,
            vertices: g.vertices.clone(),
            unique_id: g.unique_id.clone(),
        }),
        Graphic::Bezier(g) => SchRecord::Bezier(SchBezier {
            base: SchPrimitiveBase {
                owner_part_id: g.owner_part_id,
                ..default_base()
            },
            line_width: g.line_width,
            color: g.color,
            vertices: g.vertices.clone(),
            unique_id: g.unique_id.clone(),
        }),
        Graphic::Image(g) => SchRecord::Image(SchImage {
            base: SchPrimitiveBase {
                owner_part_id: g.owner_part_id,
                ..default_base()
            },
            location: g.location,
            corner: g.corner,
            orientation: g.orientation,
            line_width: g.line_width,
            color: g.color,
            is_solid: g.is_solid,
            keep_aspect: g.keep_aspect,
            embed_image: g.embed_image,
            file_name: g.file_name.clone(),
            unique_id: g.unique_id.clone(),
        }),
        Graphic::Label(g) => SchRecord::Label(SchLabel {
            base: SchPrimitiveBase {
                owner_part_id: g.owner_part_id,
                ..default_base()
            },
            location: g.location,
            orientation: g.orientation,
            justification: g.justification,
            color: g.color,
            font_id: g.font_id,
            text: g.text.clone(),
            is_mirrored: g.is_mirrored,
            url: g.url.clone(),
            unique_id: g.unique_id.clone(),
        }),
        Graphic::TextFrame(g) => SchRecord::TextFrame(SchTextFrame {
            base: SchPrimitiveBase {
                owner_part_id: g.owner_part_id,
                ..default_base()
            },
            location: g.location,
            corner: g.corner,
            line_width: g.line_width,
            color: g.color,
            area_color: g.area_color,
            text_color: g.text_color,
            font_id: g.font_id,
            is_solid: g.is_solid,
            show_border: g.show_border,
            alignment: g.alignment,
            word_wrap: g.word_wrap,
            clip_to_rect: g.clip_to_rect,
            text: g.text.clone(),
            text_margin: g.text_margin,
            transparent: g.transparent,
            unique_id: g.unique_id.clone(),
        }),
    }
}
