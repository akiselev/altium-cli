//! Shared conversion functions for schematic record types.
//!
//! These converters are used by both SchLib and SchDoc API modules to convert
//! between internal `SchRecord` types and public API types (`Pin`, `Parameter`,
//! `Graphic`, `FootprintMap`).

use crate::api::schlib_types::*;
use crate::sch_records::{
    SchRecord,
    SchPin as InternalPin,
    SchParameter as InternalParameter,
    SchPrimitiveBase,
    SchLine, SchRectangle, SchRoundRectangle, SchArc, SchEllipticalArc,
    SchEllipse, SchPie, SchPolyline, SchPolygon, SchBezier,
    SchImage, SchLabel, SchTextFrame,
    PinTextPositioning as InternalPinTextPositioning,
};
use crate::{AltiumFormatError, Result};

use altium_format_types::sch::{TextHorzAnchor, TextVertAnchor};

// ── Read converters ──────────────────────────────────────────────────────────

/// Convert an internal `SchPin` to a public `Pin`.
pub(crate) fn pin_from_internal(p: &InternalPin) -> Pin {
    Pin {
        designator: p.designator.clone(),
        name: p.name.clone(),
        electrical: p.electrical,
        location: p.location,
        length: p.pin_length,
        orientation: p.orientation,
        is_hidden: p.is_hidden,
        hidden_net_name: p.hidden_net_name.clone(),
        owner_part_id: p.owner_part_id,
        show_name: p.show_name,
        show_designator: p.show_designator,
        symbol_inner_edge: p.symbol_inner_edge,
        symbol_outer_edge: p.symbol_outer_edge,
        symbol_inside: p.symbol_inside,
        symbol_outside: p.symbol_outside,
        swap_id_pin: p.swap_id_pin.clone(),
        swap_id_part: p.swap_id_part.clone(),
        swap_id_pair: p.swap_id_pair.clone(),
        default_value: p.default_value.clone(),
        pin_package_length: p.pin_package_length.clone(),
        propagation_delay: p.propagation_delay.clone(),
        pin_symbol_line_width: p.pin_symbol_line_width,
        name_text_data: p.name_text_data.as_ref().map(pin_text_from_internal),
        designator_text_data: p.designator_text_data.as_ref().map(pin_text_from_internal),
        description: p.description.clone(),
        formal_type: p.formal_type,
        spice_pin_name: p.spice_pin_name.clone(),
        unique_id: p.unique_id.clone(),
        color: p.color,
        is_not_accessible: p.is_not_accessible,
        graphically_locked: p.graphically_locked,
        owner_part_display_mode: p.owner_part_display_mode,
    }
}

/// Convert internal pin text positioning to public type.
pub(crate) fn pin_text_from_internal(ptd: &InternalPinTextPositioning) -> PinTextPositioning {
    PinTextPositioning {
        position_mode_custom: ptd.position_mode_custom,
        rotation_anchor_component: ptd.rotation_anchor_component,
        rotation_relative: ptd.rotation_relative,
        font_mode_custom: ptd.font_mode_custom,
        custom_position_margin: ptd.custom_position_margin,
        custom_font_id: ptd.custom_font_id,
        custom_color: ptd.custom_color,
    }
}

/// Convert an internal `SchParameter` to a public `Parameter`.
pub(crate) fn parameter_from_internal(p: &InternalParameter) -> Parameter {
    Parameter {
        name: p.name.clone(),
        text: p.text.clone(),
        is_hidden: p.is_hidden,
        read_only: p.read_only_state,
        location: p.location,
        orientation: p.orientation,
        color: p.color,
        font_id: p.font_id,
        justification: p.justification,
        is_mirrored: p.is_mirrored,
        show_name: p.show_name,
        unique_id: p.unique_id.clone(),
        not_auto_position: p.not_auto_position,
        param_type: p.param_type,
        description: p.description.clone(),
    }
}

/// Process a slice of records, extracting pins, parameters, designator, and graphics.
///
/// Shared between SchLib and SchDoc read paths. Records that are container-level
/// (Component, ImplementationList, etc.) are skipped — they are handled by the
/// caller's ownership logic.
pub(crate) fn process_records(
    records: &[SchRecord],
    designator: &mut Option<String>,
    pins: &mut Vec<Pin>,
    parameters: &mut Vec<Parameter>,
    graphics: &mut Vec<Graphic>,
) -> Result<()> {
    for rec in records {
        match rec {
            SchRecord::Pin(p) => {
                pins.push(pin_from_internal(p));
            }
            SchRecord::Designator(d) => {
                *designator = Some(d.text.clone());
            }
            SchRecord::Parameter(p) => {
                parameters.push(parameter_from_internal(p));
            }
            // Graphics
            SchRecord::Line(g) => {
                graphics.push(graphic_from_line(g));
            }
            SchRecord::Rectangle(g) => {
                graphics.push(graphic_from_rectangle(g));
            }
            SchRecord::RoundRectangle(g) => {
                graphics.push(graphic_from_round_rectangle(g));
            }
            SchRecord::Arc(g) => {
                graphics.push(graphic_from_arc(g));
            }
            SchRecord::EllipticalArc(g) => {
                graphics.push(graphic_from_elliptical_arc(g));
            }
            SchRecord::Ellipse(g) => {
                graphics.push(graphic_from_ellipse(g));
            }
            SchRecord::Pie(g) => {
                graphics.push(graphic_from_pie(g));
            }
            SchRecord::Polyline(g) => {
                graphics.push(graphic_from_polyline(g));
            }
            SchRecord::Polygon(g) => {
                graphics.push(graphic_from_polygon(g));
            }
            SchRecord::Bezier(g) => {
                graphics.push(graphic_from_bezier(g));
            }
            SchRecord::Image(g) => {
                graphics.push(graphic_from_image(g));
            }
            SchRecord::Label(g) => {
                graphics.push(graphic_from_label(g));
            }
            SchRecord::TextFrame(g) => {
                graphics.push(graphic_from_text_frame(g));
            }
            // Container records — invisible to API
            SchRecord::Component(_)
            | SchRecord::ImplementationList(_)
            | SchRecord::Implementation(_)
            | SchRecord::ImplementationMap(_)
            | SchRecord::MapDefiner(_)
            | SchRecord::ParameterList(_) => {}
            // Symbol (RECORD=3) — treated as a graphic
            SchRecord::Symbol(_) => {
                // SchSymbol has no unique_id and is not commonly used in SchLib.
                // Skip for now; add support when fixtures surface it.
            }
            // Hyperlink reuses SchLabel
            SchRecord::Hyperlink(g) => {
                graphics.push(graphic_from_label(g));
            }
            other => {
                return Err(AltiumFormatError::InvalidParamValue {
                    key: "RECORD".to_owned(),
                    detail: format!(
                        "unexpected record type {:?} in component",
                        std::mem::discriminant(other)
                    ),
                });
            }
        }
    }
    Ok(())
}

/// Convert a single SchRecord to a Graphic, if it is a graphic type.
///
/// Returns `None` for non-graphic records. Used by SchDoc read path where
/// records are dispatched individually rather than in bulk via `process_records`.
pub(crate) fn graphic_from_record(rec: &SchRecord) -> Option<Graphic> {
    match rec {
        SchRecord::Line(g) => Some(graphic_from_line(g)),
        SchRecord::Rectangle(g) => Some(graphic_from_rectangle(g)),
        SchRecord::RoundRectangle(g) => Some(graphic_from_round_rectangle(g)),
        SchRecord::Arc(g) => Some(graphic_from_arc(g)),
        SchRecord::EllipticalArc(g) => Some(graphic_from_elliptical_arc(g)),
        SchRecord::Ellipse(g) => Some(graphic_from_ellipse(g)),
        SchRecord::Pie(g) => Some(graphic_from_pie(g)),
        SchRecord::Polyline(g) => Some(graphic_from_polyline(g)),
        SchRecord::Polygon(g) => Some(graphic_from_polygon(g)),
        SchRecord::Bezier(g) => Some(graphic_from_bezier(g)),
        SchRecord::Image(g) => Some(graphic_from_image(g)),
        SchRecord::Label(g) | SchRecord::Hyperlink(g) => Some(graphic_from_label(g)),
        SchRecord::TextFrame(g) => Some(graphic_from_text_frame(g)),
        _ => None,
    }
}

// ── Individual graphic converters ────────────────────────────────────────────

fn graphic_from_line(g: &SchLine) -> Graphic {
    Graphic::Line(LineGraphic {
        unique_id: g.unique_id.clone(),
        owner_part_id: g.base.owner_part_id,
        location: g.location,
        corner: g.corner,
        line_width: g.line_width,
        line_style: g.line_style,
        color: g.color,
    })
}

fn graphic_from_rectangle(g: &SchRectangle) -> Graphic {
    Graphic::Rectangle(RectangleGraphic {
        unique_id: g.unique_id.clone(),
        owner_part_id: g.base.owner_part_id,
        location: g.location,
        corner: g.corner,
        line_width: g.line_width,
        line_style: g.line_style,
        color: g.color,
        area_color: g.area_color,
        is_solid: g.is_solid,
        transparent: g.transparent,
    })
}

fn graphic_from_round_rectangle(g: &SchRoundRectangle) -> Graphic {
    Graphic::RoundRectangle(RoundRectangleGraphic {
        unique_id: g.unique_id.clone(),
        owner_part_id: g.base.owner_part_id,
        location: g.location,
        corner: g.corner,
        corner_x_radius: g.corner_x_radius,
        corner_y_radius: g.corner_y_radius,
        line_width: g.line_width,
        color: g.color,
        area_color: g.area_color,
        is_solid: g.is_solid,
    })
}

fn graphic_from_arc(g: &SchArc) -> Graphic {
    Graphic::Arc(ArcGraphic {
        unique_id: g.unique_id.clone(),
        owner_part_id: g.base.owner_part_id,
        location: g.location,
        radius: g.radius,
        start_angle: g.start_angle,
        end_angle: g.end_angle,
        line_width: g.line_width,
        color: g.color,
    })
}

fn graphic_from_elliptical_arc(g: &SchEllipticalArc) -> Graphic {
    Graphic::EllipticalArc(EllipticalArcGraphic {
        unique_id: g.unique_id.clone(),
        owner_part_id: g.base.owner_part_id,
        location: g.location,
        radius: g.radius,
        secondary_radius: g.secondary_radius,
        start_angle: g.start_angle,
        end_angle: g.end_angle,
        line_width: g.line_width,
        color: g.color,
    })
}

fn graphic_from_ellipse(g: &SchEllipse) -> Graphic {
    Graphic::Ellipse(EllipseGraphic {
        unique_id: g.unique_id.clone(),
        owner_part_id: g.base.owner_part_id,
        location: g.location,
        radius: g.radius,
        secondary_radius: g.secondary_radius,
        line_width: g.line_width,
        color: g.color,
        area_color: g.area_color,
        is_solid: g.is_solid,
        transparent: g.transparent,
    })
}

fn graphic_from_pie(g: &SchPie) -> Graphic {
    Graphic::Pie(PieGraphic {
        owner_part_id: g.base.owner_part_id,
        location: g.location,
        radius: g.radius,
        start_angle: g.start_angle,
        end_angle: g.end_angle,
        line_width: g.line_width,
        color: g.color,
        area_color: g.area_color,
        is_solid: g.is_solid,
    })
}

fn graphic_from_polyline(g: &SchPolyline) -> Graphic {
    Graphic::Polyline(PolylineGraphic {
        unique_id: g.unique_id.clone(),
        owner_part_id: g.base.owner_part_id,
        vertices: g.vertices.clone(),
        line_width: g.line_width,
        line_style: g.line_style,
        start_line_shape: g.start_line_shape,
        end_line_shape: g.end_line_shape,
        line_shape_size: g.line_shape_size,
        color: g.color,
    })
}

fn graphic_from_polygon(g: &SchPolygon) -> Graphic {
    Graphic::Polygon(PolygonGraphic {
        unique_id: g.unique_id.clone(),
        owner_part_id: g.base.owner_part_id,
        vertices: g.vertices.clone(),
        line_width: g.line_width,
        color: g.color,
        area_color: g.area_color,
        is_solid: g.is_solid,
        transparent: g.transparent,
    })
}

fn graphic_from_bezier(g: &SchBezier) -> Graphic {
    Graphic::Bezier(BezierGraphic {
        unique_id: g.unique_id.clone(),
        owner_part_id: g.base.owner_part_id,
        vertices: g.vertices.clone(),
        line_width: g.line_width,
        color: g.color,
    })
}

fn graphic_from_image(g: &SchImage) -> Graphic {
    Graphic::Image(ImageGraphic {
        unique_id: g.unique_id.clone(),
        owner_part_id: g.base.owner_part_id,
        location: g.location,
        corner: g.corner,
        orientation: g.orientation,
        line_width: g.line_width,
        color: g.color,
        is_solid: g.is_solid,
        keep_aspect: g.keep_aspect,
        embed_image: g.embed_image,
        file_name: g.file_name.clone(),
    })
}

fn graphic_from_label(g: &SchLabel) -> Graphic {
    Graphic::Label(LabelGraphic {
        unique_id: g.unique_id.clone(),
        owner_part_id: g.base.owner_part_id,
        location: g.location,
        orientation: g.orientation,
        justification: g.justification,
        color: g.color,
        font_id: g.font_id,
        text: g.text.clone(),
        is_mirrored: g.is_mirrored,
        url: g.url.clone(),
    })
}

fn graphic_from_text_frame(g: &SchTextFrame) -> Graphic {
    Graphic::TextFrame(TextFrameGraphic {
        unique_id: g.unique_id.clone(),
        owner_part_id: g.base.owner_part_id,
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
    })
}

/// Build footprint maps by tracing the implementation ownership chain.
///
/// The chain is: ImplementationList → Implementation → ImplementationMap → MapDefiner
/// Each record uses `owner_index` (1-based) to point to its parent record.
///
/// This variant works for SchLib where indices are component-relative (1-based).
pub(crate) fn build_footprint_maps(records: &[SchRecord]) -> Result<Vec<FootprintMap>> {
    let mut footprints = Vec::new();

    for (rec_idx, rec) in records.iter().enumerate() {
        if let SchRecord::Implementation(imp) = rec {
            if !imp.model_type.eq_ignore_ascii_case("PCBLIB") {
                continue;
            }

            // Find all MapDefiner records that belong to this Implementation.
            // Chain: Implementation (rec_idx) → ImplementationMap → MapDefiner
            // ImplementationMap's owner_index points to this Implementation (rec_idx + 1, 1-based)
            // MapDefiner's owner_index points to the ImplementationMap
            let impl_1based = rec_idx + 1;

            let pin_pad_maps: Vec<PinPadMap> = records.iter().enumerate().filter_map(|(map_idx, map_rec)| {
                if let SchRecord::ImplementationMap(im) = map_rec {
                    if im.base.owner_index as usize == impl_1based {
                        // Found the ImplementationMap for this Implementation.
                        // Now find MapDefiners owned by this ImplementationMap.
                        let map_1based = map_idx + 1;
                        let definers: Vec<PinPadMap> = records.iter().filter_map(|def_rec| {
                            if let SchRecord::MapDefiner(md) = def_rec {
                                if md.base.owner_index as usize == map_1based {
                                    Some(PinPadMap {
                                        pin: md.des_intf.clone(),
                                        pad: md.des_imps.first().cloned().unwrap_or_default(),
                                    })
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        }).collect();
                        Some(definers)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }).flatten().collect();

            footprints.push(FootprintMap {
                model_name: imp.model_name.clone(),
                description: imp.description.clone(),
                is_current: imp.is_current,
                pin_pad_maps,
            });
        }
    }

    Ok(footprints)
}

/// Build footprint maps for SchDoc components using global ownership indices.
///
/// Unlike SchLib's 1-based component-relative indices, SchDoc uses global
/// record indices with a virtual index scheme: indices `< records.len()` refer
/// to main records, indices `>= records.len()` refer to additional records.
/// This function resolves the Implementation chain using the global ownership map.
pub(crate) fn build_footprint_maps_schdoc(
    records: &[SchRecord],
    additional_records: &[SchRecord],
    component_child_indices: &[usize],
    ownership_map: &std::collections::HashMap<usize, Vec<usize>>,
) -> Result<Vec<FootprintMap>> {
    let base_len = records.len();

    // Resolve a virtual index to the actual record reference
    let resolve = |idx: usize| -> &SchRecord {
        if idx < base_len {
            &records[idx]
        } else {
            &additional_records[idx - base_len]
        }
    };

    let mut footprints = Vec::new();

    // Find Implementation records among the component's children
    for &child_idx in component_child_indices {
        if let SchRecord::Implementation(imp) = resolve(child_idx) {
            if !imp.model_type.eq_ignore_ascii_case("PCBLIB") {
                continue;
            }

            // Find ImplementationMap children of this Implementation
            let mut pin_pad_maps = Vec::new();
            if let Some(impl_children) = ownership_map.get(&child_idx) {
                for &map_idx in impl_children {
                    if let SchRecord::ImplementationMap(_) = resolve(map_idx) {
                        // Find MapDefiner children of this ImplementationMap
                        if let Some(map_children) = ownership_map.get(&map_idx) {
                            for &def_idx in map_children {
                                if let SchRecord::MapDefiner(md) = resolve(def_idx) {
                                    pin_pad_maps.push(PinPadMap {
                                        pin: md.des_intf.clone(),
                                        pad: md.des_imps.first().cloned().unwrap_or_default(),
                                    });
                                }
                            }
                        }
                    }
                }
            }

            footprints.push(FootprintMap {
                model_name: imp.model_name.clone(),
                description: imp.description.clone(),
                is_current: imp.is_current,
                pin_pad_maps,
            });
        }
    }

    Ok(footprints)
}

// ── Write converters ─────────────────────────────────────────────────────────

/// Create a default `SchPrimitiveBase` with sensible defaults.
pub(crate) fn default_base() -> SchPrimitiveBase {
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

/// Convert a public `Pin` to internal `SchPin`.
pub(crate) fn pin_to_internal(pin: &Pin) -> InternalPin {
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

/// Convert public pin text positioning to internal type.
pub(crate) fn pin_text_to_internal(ptd: &PinTextPositioning) -> InternalPinTextPositioning {
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

/// Convert a public `Parameter` to internal `SchParameter`.
pub(crate) fn parameter_to_internal(param: &Parameter) -> InternalParameter {
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

/// Convert a public `Graphic` to an internal `SchRecord`.
pub(crate) fn graphic_to_record(graphic: &Graphic) -> SchRecord {
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
