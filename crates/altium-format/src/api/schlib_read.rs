//! Read path: convert internal SchLib types → public API types.

use crate::api::schlib_types::*;
use crate::sch_records::{
    SchLibComponent, SchRecord,
    SchPin as InternalPin,
    SchParameter as InternalParameter,
    PinTextPositioning as InternalPinTextPositioning,
};
use crate::schlib::SchLibComponentIndex;
use crate::{AltiumFormatError, Result};

/// Convert an internal `SchLibComponent` + index entry into a public `Component`.
pub(crate) fn component_from_internal(
    comp: &SchLibComponent,
    index: &SchLibComponentIndex,
) -> Result<Component> {
    let mut designator = None;
    let mut pins = Vec::new();
    let mut parameters = Vec::new();
    let mut graphics = Vec::new();

    // Process main records
    process_records(&comp.records, &mut designator, &mut pins, &mut parameters, &mut graphics)?;
    // Process additional records (same logic)
    process_records(&comp.additional_records, &mut designator, &mut pins, &mut parameters, &mut graphics)?;

    // Build footprint maps from the implementation chain in main records
    let footprints = build_footprint_maps(&comp.records)?;

    Ok(Component {
        lib_reference: index.lib_ref.clone(),
        designator,
        description: if index.description.is_empty() {
            None
        } else {
            Some(index.description.clone())
        },
        component_kind: if comp.component.component_kind == altium_format_types::common::ComponentKind::Standard {
            None
        } else {
            Some(comp.component.component_kind)
        },
        part_count: comp.component.part_count,
        show_hidden_pins: comp.component.show_hidden_pins,
        pins,
        parameters,
        footprints,
        graphics,
        aliases: index.aliases.clone(),
    })
}

/// Process a slice of records, extracting pins, parameters, designator, and graphics.
fn process_records(
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
                graphics.push(Graphic::Line(LineGraphic {
                    unique_id: g.unique_id.clone(),
                    owner_part_id: g.base.owner_part_id,
                    location: g.location,
                    corner: g.corner,
                    line_width: g.line_width,
                    line_style: g.line_style,
                    color: g.color,
                }));
            }
            SchRecord::Rectangle(g) => {
                graphics.push(Graphic::Rectangle(RectangleGraphic {
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
                }));
            }
            SchRecord::RoundRectangle(g) => {
                graphics.push(Graphic::RoundRectangle(RoundRectangleGraphic {
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
                }));
            }
            SchRecord::Arc(g) => {
                graphics.push(Graphic::Arc(ArcGraphic {
                    unique_id: g.unique_id.clone(),
                    owner_part_id: g.base.owner_part_id,
                    location: g.location,
                    radius: g.radius,
                    start_angle: g.start_angle,
                    end_angle: g.end_angle,
                    line_width: g.line_width,
                    color: g.color,
                }));
            }
            SchRecord::EllipticalArc(g) => {
                graphics.push(Graphic::EllipticalArc(EllipticalArcGraphic {
                    unique_id: g.unique_id.clone(),
                    owner_part_id: g.base.owner_part_id,
                    location: g.location,
                    radius: g.radius,
                    secondary_radius: g.secondary_radius,
                    start_angle: g.start_angle,
                    end_angle: g.end_angle,
                    line_width: g.line_width,
                    color: g.color,
                }));
            }
            SchRecord::Ellipse(g) => {
                graphics.push(Graphic::Ellipse(EllipseGraphic {
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
                }));
            }
            SchRecord::Pie(g) => {
                graphics.push(Graphic::Pie(PieGraphic {
                    owner_part_id: g.base.owner_part_id,
                    location: g.location,
                    radius: g.radius,
                    start_angle: g.start_angle,
                    end_angle: g.end_angle,
                    line_width: g.line_width,
                    color: g.color,
                    area_color: g.area_color,
                    is_solid: g.is_solid,
                }));
            }
            SchRecord::Polyline(g) => {
                graphics.push(Graphic::Polyline(PolylineGraphic {
                    unique_id: g.unique_id.clone(),
                    owner_part_id: g.base.owner_part_id,
                    vertices: g.vertices.clone(),
                    line_width: g.line_width,
                    line_style: g.line_style,
                    start_line_shape: g.start_line_shape,
                    end_line_shape: g.end_line_shape,
                    line_shape_size: g.line_shape_size,
                    color: g.color,
                }));
            }
            SchRecord::Polygon(g) => {
                graphics.push(Graphic::Polygon(PolygonGraphic {
                    unique_id: g.unique_id.clone(),
                    owner_part_id: g.base.owner_part_id,
                    vertices: g.vertices.clone(),
                    line_width: g.line_width,
                    color: g.color,
                    area_color: g.area_color,
                    is_solid: g.is_solid,
                    transparent: g.transparent,
                }));
            }
            SchRecord::Bezier(g) => {
                graphics.push(Graphic::Bezier(BezierGraphic {
                    unique_id: g.unique_id.clone(),
                    owner_part_id: g.base.owner_part_id,
                    vertices: g.vertices.clone(),
                    line_width: g.line_width,
                    color: g.color,
                }));
            }
            SchRecord::Image(g) => {
                graphics.push(Graphic::Image(ImageGraphic {
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
                }));
            }
            SchRecord::Label(g) => {
                graphics.push(Graphic::Label(LabelGraphic {
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
                }));
            }
            SchRecord::TextFrame(g) => {
                graphics.push(Graphic::TextFrame(TextFrameGraphic {
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
                }));
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
                graphics.push(Graphic::Label(LabelGraphic {
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
                }));
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

/// Build footprint maps by tracing the implementation ownership chain.
///
/// The chain is: ImplementationList → Implementation → ImplementationMap → MapDefiner
/// Each record uses `owner_index` (1-based) to point to its parent record.
fn build_footprint_maps(records: &[SchRecord]) -> Result<Vec<FootprintMap>> {
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

fn pin_from_internal(p: &InternalPin) -> Pin {
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

fn pin_text_from_internal(ptd: &InternalPinTextPositioning) -> PinTextPositioning {
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

fn parameter_from_internal(p: &InternalParameter) -> Parameter {
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
