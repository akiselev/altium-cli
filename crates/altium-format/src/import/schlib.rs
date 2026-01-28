// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! SchLib generation from import DSL types.

use std::path::Path;

use crate::io::SchLibComponent;
use crate::ops::schlib::{
    mils_f64_to_raw, open_or_create_schlib, parse_color, parse_electrical_type,
    parse_text_justification, parse_text_orientation, parse_unit_value_or_mil, save_schlib,
};
use crate::records::sch::{
    LineWidth, PinConglomerateFlags, PinSymbol, SchArc, SchComponent, SchEllipse, SchGraphicalBase,
    SchLabel, SchLine, SchPin, SchPolygon, SchPolyline, SchRecord, SchRectangle,
};

use super::types::*;

/// Generate a complete SchLib from an import definition.
pub fn generate_schlib(
    output_path: &Path,
    import: &SchLibImport,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut lib = if output_path.exists() {
        open_or_create_schlib(output_path)?
    } else {
        // Create blank
        open_or_create_schlib(output_path)?
    };

    let mut generated_count = 0;

    for comp_def in &import.components {
        // Check for duplicates
        if lib
            .components
            .iter()
            .any(|c| c.component.lib_reference == comp_def.name)
        {
            return Err(format!("Component '{}' already exists in library", comp_def.name).into());
        }

        let lib_component = match &comp_def.style {
            SymbolStyle::Ic => generate_ic_component(comp_def)?,
            SymbolStyle::Discrete => generate_discrete_component(comp_def)?,
            SymbolStyle::Power => generate_power_component(comp_def)?,
            SymbolStyle::Connector => generate_connector_component(comp_def)?,
        };

        lib.components.push(lib_component);
        generated_count += 1;
    }

    save_schlib(output_path, &lib)?;

    Ok(format!(
        "Generated SchLib with {} component(s) -> {}",
        generated_count,
        output_path.display()
    ))
}

/// Generate an IC-style component (rectangle body with pins on 4 sides).
fn generate_ic_component(
    def: &SchLibComponentDef,
) -> Result<SchLibComponent, Box<dyn std::error::Error>> {
    let pin_spacing_mils = parse_unit_value_or_mil(
        def.pin_spacing.as_deref().unwrap_or("100mil"),
    )?;
    let pin_length_mils = parse_unit_value_or_mil(
        def.pin_length.as_deref().unwrap_or("200mil"),
    )?;
    let requested_width_mils = match &def.width {
        Some(w) => parse_unit_value_or_mil(w)?,
        None => 800.0, // Default 800 mils
    };

    // Separate pins by side
    let left_pins: Vec<_> = def.pins.iter().filter(|p| p.side == PinSide::Left).collect();
    let right_pins: Vec<_> = def.pins.iter().filter(|p| p.side == PinSide::Right).collect();
    let top_pins: Vec<_> = def.pins.iter().filter(|p| p.side == PinSide::Top).collect();
    let bottom_pins: Vec<_> = def.pins.iter().filter(|p| p.side == PinSide::Bottom).collect();

    let max_vertical_pins = left_pins.len().max(right_pins.len());
    let max_horizontal_pins = top_pins.len().max(bottom_pins.len());
    let body_height_mils = (max_vertical_pins + 1) as f64 * pin_spacing_mils;

    let min_width_for_tb = if max_horizontal_pins > 0 {
        (max_horizontal_pins + 1) as f64 * pin_spacing_mils
    } else {
        0.0
    };
    let width_mils = requested_width_mils.max(min_width_for_tb);

    // Create component record
    let component = SchComponent {
        lib_reference: def.name.clone(),
        component_description: def.description.clone(),
        part_count: def.part_count,
        display_mode_count: def.display_modes,
        current_part_id: 1,
        ..Default::default()
    };

    let mut primitives = vec![SchRecord::Component(component.clone())];

    // Add body rectangle
    let mut rect_graphical = SchGraphicalBase::default();
    rect_graphical.base.owner_part_id = Some(1);
    rect_graphical.location_x = mils_f64_to_raw(0.0);
    rect_graphical.location_y = mils_f64_to_raw(0.0);
    rect_graphical.color = parse_color("800000")?;
    rect_graphical.area_color = parse_color("FFFFB0")?;

    let rect = SchRectangle {
        graphical: rect_graphical,
        corner_x: mils_f64_to_raw(width_mils),
        corner_y: mils_f64_to_raw(body_height_mils),
        line_width: LineWidth::Small,
        is_solid: true,
        transparent: false,
        ..Default::default()
    };
    primitives.push(SchRecord::Rectangle(rect));

    // Add left pins (pointing right into body)
    for (i, pin_def) in left_pins.iter().enumerate() {
        let y_mils = body_height_mils - (i + 1) as f64 * pin_spacing_mils;
        let pin = make_pin(
            pin_def,
            -pin_length_mils,
            y_mils,
            pin_length_mils,
            PinConglomerateFlags::empty(), // points right
        )?;
        primitives.push(SchRecord::Pin(pin));
    }

    // Add right pins (pointing left into body)
    for (i, pin_def) in right_pins.iter().enumerate() {
        let y_mils = body_height_mils - (i + 1) as f64 * pin_spacing_mils;
        let pin = make_pin(
            pin_def,
            width_mils + pin_length_mils,
            y_mils,
            pin_length_mils,
            PinConglomerateFlags::FLIPPED,
        )?;
        primitives.push(SchRecord::Pin(pin));
    }

    // Add top pins (pointing down into body)
    for (i, pin_def) in top_pins.iter().enumerate() {
        let x_mils = (i + 1) as f64 * pin_spacing_mils;
        let pin = make_pin(
            pin_def,
            x_mils,
            body_height_mils + pin_length_mils,
            pin_length_mils,
            PinConglomerateFlags::ROTATED,
        )?;
        primitives.push(SchRecord::Pin(pin));
    }

    // Add bottom pins (pointing up into body)
    for (i, pin_def) in bottom_pins.iter().enumerate() {
        let x_mils = (i + 1) as f64 * pin_spacing_mils;
        let pin = make_pin(
            pin_def,
            x_mils,
            -pin_length_mils,
            pin_length_mils,
            PinConglomerateFlags::ROTATED | PinConglomerateFlags::FLIPPED,
        )?;
        primitives.push(SchRecord::Pin(pin));
    }

    // Add extra graphics
    add_graphics(&mut primitives, &def.graphics)?;

    Ok(SchLibComponent {
        component,
        primitives,
    })
}

/// Generate a two-pin discrete component (resistor, capacitor, etc.).
fn generate_discrete_component(
    def: &SchLibComponentDef,
) -> Result<SchLibComponent, Box<dyn std::error::Error>> {
    let pin_spacing_mils = parse_unit_value_or_mil(
        def.pin_spacing.as_deref().unwrap_or("100mil"),
    )?;
    let pin_length_mils = parse_unit_value_or_mil(
        def.pin_length.as_deref().unwrap_or("200mil"),
    )?;

    let component = SchComponent {
        lib_reference: def.name.clone(),
        component_description: def.description.clone(),
        part_count: def.part_count,
        display_mode_count: def.display_modes,
        current_part_id: 1,
        ..Default::default()
    };

    let mut primitives = vec![SchRecord::Component(component.clone())];

    // Simple rectangle body for discrete
    let body_width_mils = 100.0;
    let body_height_mils = pin_spacing_mils;

    let mut rect_graphical = SchGraphicalBase::default();
    rect_graphical.base.owner_part_id = Some(1);
    rect_graphical.location_x = mils_f64_to_raw(0.0);
    rect_graphical.location_y = mils_f64_to_raw(0.0);
    rect_graphical.color = parse_color("800000")?;
    rect_graphical.area_color = parse_color("FFFFB0")?;

    let rect = SchRectangle {
        graphical: rect_graphical,
        corner_x: mils_f64_to_raw(body_width_mils),
        corner_y: mils_f64_to_raw(body_height_mils),
        line_width: LineWidth::Small,
        is_solid: true,
        transparent: false,
        ..Default::default()
    };
    primitives.push(SchRecord::Rectangle(rect));

    // Pin 1 on left, Pin 2 on right (or use defined pins)
    if def.pins.len() >= 2 {
        let mid_y = body_height_mils / 2.0;
        let pin1 = make_pin(
            &def.pins[0],
            -pin_length_mils,
            mid_y,
            pin_length_mils,
            PinConglomerateFlags::empty(),
        )?;
        primitives.push(SchRecord::Pin(pin1));

        let pin2 = make_pin(
            &def.pins[1],
            body_width_mils + pin_length_mils,
            mid_y,
            pin_length_mils,
            PinConglomerateFlags::FLIPPED,
        )?;
        primitives.push(SchRecord::Pin(pin2));
    }

    // Any additional pins beyond 2
    for (i, pin_def) in def.pins.iter().skip(2).enumerate() {
        let y_mils = (i + 2) as f64 * pin_spacing_mils;
        let pin = make_pin(
            pin_def,
            -pin_length_mils,
            y_mils,
            pin_length_mils,
            PinConglomerateFlags::empty(),
        )?;
        primitives.push(SchRecord::Pin(pin));
    }

    add_graphics(&mut primitives, &def.graphics)?;

    Ok(SchLibComponent {
        component,
        primitives,
    })
}

/// Generate a power port symbol.
fn generate_power_component(
    def: &SchLibComponentDef,
) -> Result<SchLibComponent, Box<dyn std::error::Error>> {
    // Power symbols are basically a single pin with a power object style
    let component = SchComponent {
        lib_reference: def.name.clone(),
        component_description: def.description.clone(),
        part_count: 1,
        display_mode_count: 1,
        current_part_id: 1,
        ..Default::default()
    };

    let mut primitives = vec![SchRecord::Component(component.clone())];

    // Power symbols have a single hidden pin
    if let Some(first_pin) = def.pins.first() {
        let pin = make_pin(
            first_pin,
            0.0,
            0.0,
            0.0, // zero length for power pins
            PinConglomerateFlags::HIDE,
        )?;
        primitives.push(SchRecord::Pin(pin));
    } else {
        // Auto-create a hidden power pin
        let mut graphical = SchGraphicalBase::default();
        graphical.base.owner_part_id = Some(1);
        graphical.location_x = mils_f64_to_raw(0.0);
        graphical.location_y = mils_f64_to_raw(0.0);
        graphical.color = 0x000080;

        let net = def
            .net_name
            .as_deref()
            .unwrap_or(&def.name);

        let pin = SchPin {
            graphical,
            designator: "1".to_string(),
            name: net.to_string(),
            electrical: crate::records::sch::PinElectricalType::Power,
            pin_conglomerate: PinConglomerateFlags::HIDE,
            pin_length: 0,
            ..Default::default()
        };
        primitives.push(SchRecord::Pin(pin));
    }

    add_graphics(&mut primitives, &def.graphics)?;

    Ok(SchLibComponent {
        component,
        primitives,
    })
}

/// Generate a connector component (multi-row grid).
fn generate_connector_component(
    def: &SchLibComponentDef,
) -> Result<SchLibComponent, Box<dyn std::error::Error>> {
    let columns = def.columns.unwrap_or(1);
    let rows = def.rows.unwrap_or_else(|| {
        // Auto-compute rows from pin count and columns
        let pin_count = def.pins.len().max(2);
        (pin_count + columns - 1) / columns
    });

    let pin_spacing_mils = parse_unit_value_or_mil(
        def.pin_spacing.as_deref().unwrap_or("100mil"),
    )?;
    let pin_length_mils = parse_unit_value_or_mil(
        def.pin_length.as_deref().unwrap_or("200mil"),
    )?;

    let body_width_mils = if columns == 1 {
        200.0
    } else {
        (columns as f64) * 200.0
    };
    let body_height_mils = (rows + 1) as f64 * pin_spacing_mils;

    let component = SchComponent {
        lib_reference: def.name.clone(),
        component_description: def.description.clone(),
        part_count: def.part_count,
        display_mode_count: def.display_modes,
        current_part_id: 1,
        ..Default::default()
    };

    let mut primitives = vec![SchRecord::Component(component.clone())];

    // Body rectangle
    let mut rect_graphical = SchGraphicalBase::default();
    rect_graphical.base.owner_part_id = Some(1);
    rect_graphical.location_x = mils_f64_to_raw(0.0);
    rect_graphical.location_y = mils_f64_to_raw(0.0);
    rect_graphical.color = parse_color("800000")?;
    rect_graphical.area_color = parse_color("FFFFB0")?;

    let rect = SchRectangle {
        graphical: rect_graphical,
        corner_x: mils_f64_to_raw(body_width_mils),
        corner_y: mils_f64_to_raw(body_height_mils),
        line_width: LineWidth::Small,
        is_solid: true,
        transparent: false,
        ..Default::default()
    };
    primitives.push(SchRecord::Rectangle(rect));

    // Build a lookup of explicitly defined pins
    let explicit_pins: std::collections::HashMap<String, &SchLibPinDef> = def
        .pins
        .iter()
        .map(|p| (p.designator.clone(), p))
        .collect();

    // Generate pin grid
    let total_pins = columns * rows;
    for pin_idx in 0..total_pins {
        let designator = format!("{}", pin_idx + 1);
        let row = pin_idx / columns;
        let col = pin_idx % columns;

        let y_mils = body_height_mils - (row + 1) as f64 * pin_spacing_mils;

        if let Some(explicit) = explicit_pins.get(&designator) {
            // Use explicitly defined pin
            let (x, orientation) = if col == 0 {
                (-pin_length_mils, PinConglomerateFlags::empty())
            } else {
                (
                    body_width_mils + pin_length_mils,
                    PinConglomerateFlags::FLIPPED,
                )
            };
            let pin = make_pin(explicit, x, y_mils, pin_length_mils, orientation)?;
            primitives.push(SchRecord::Pin(pin));
        } else {
            // Auto-generate pin
            let (x, orientation) = if col == 0 {
                (-pin_length_mils, PinConglomerateFlags::empty())
            } else {
                (
                    body_width_mils + pin_length_mils,
                    PinConglomerateFlags::FLIPPED,
                )
            };

            let mut graphical = SchGraphicalBase::default();
            graphical.base.owner_part_id = Some(1);
            graphical.location_x = mils_f64_to_raw(x);
            graphical.location_y = mils_f64_to_raw(y_mils);
            graphical.color = 0x000080;

            let mut conglomerate = orientation;
            conglomerate |= PinConglomerateFlags::DISPLAY_NAME_VISIBLE;
            conglomerate |= PinConglomerateFlags::DESIGNATOR_VISIBLE;

            let pin = SchPin {
                graphical,
                designator: designator.clone(),
                name: designator,
                electrical: crate::records::sch::PinElectricalType::Passive,
                pin_conglomerate: conglomerate,
                pin_length: mils_f64_to_raw(pin_length_mils),
                ..Default::default()
            };
            primitives.push(SchRecord::Pin(pin));
        }
    }

    add_graphics(&mut primitives, &def.graphics)?;

    Ok(SchLibComponent {
        component,
        primitives,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════════════════════

/// Create a SchPin from a DSL pin definition.
fn make_pin(
    pin_def: &SchLibPinDef,
    x_mils: f64,
    y_mils: f64,
    pin_length_mils: f64,
    base_orientation: PinConglomerateFlags,
) -> Result<SchPin, Box<dyn std::error::Error>> {
    let electrical_type = parse_electrical_type(pin_def.r#type.to_str())?;

    let mut conglomerate = base_orientation;
    conglomerate |= PinConglomerateFlags::DISPLAY_NAME_VISIBLE;
    conglomerate |= PinConglomerateFlags::DESIGNATOR_VISIBLE;

    if pin_def.hidden {
        conglomerate |= PinConglomerateFlags::HIDE;
    }

    let mut graphical = SchGraphicalBase::default();
    graphical.base.owner_part_id = Some(1);
    graphical.location_x = mils_f64_to_raw(x_mils);
    graphical.location_y = mils_f64_to_raw(y_mils);
    graphical.color = 0x000080;

    Ok(SchPin {
        graphical,
        designator: pin_def.designator.clone(),
        name: pin_def.name.clone(),
        electrical: electrical_type,
        pin_conglomerate: conglomerate,
        pin_length: mils_f64_to_raw(pin_length_mils),
        description: pin_def.description.clone(),
        symbol_inner_edge: PinSymbol::None,
        symbol_outer_edge: PinSymbol::None,
        symbol_inside: PinSymbol::None,
        symbol_outside: PinSymbol::None,
        ..Default::default()
    })
}

/// Add extra graphics primitives from DSL definitions.
fn add_graphics(
    primitives: &mut Vec<SchRecord>,
    graphics: &[SchLibGraphic],
) -> Result<(), Box<dyn std::error::Error>> {
    for graphic in graphics {
        match graphic {
            SchLibGraphic::Line {
                x1,
                y1,
                x2,
                y2,
                color,
            } => {
                let color_val = parse_color(color)?;
                let mut graphical = SchGraphicalBase::default();
                graphical.base.owner_part_id = Some(1);
                graphical.location_x = mils_f64_to_raw(parse_unit_value_or_mil(x1)?);
                graphical.location_y = mils_f64_to_raw(parse_unit_value_or_mil(y1)?);
                graphical.color = color_val;

                let line = SchLine {
                    graphical,
                    corner_x: mils_f64_to_raw(parse_unit_value_or_mil(x2)?),
                    corner_y: mils_f64_to_raw(parse_unit_value_or_mil(y2)?),
                    line_width: LineWidth::Small,
                    ..Default::default()
                };
                primitives.push(SchRecord::Line(line));
            }
            SchLibGraphic::Rectangle {
                x1,
                y1,
                x2,
                y2,
                filled,
                fill_color,
                border_color,
            } => {
                let fill_color_val = parse_color(fill_color)?;
                let border_color_val = parse_color(border_color)?;
                let mut graphical = SchGraphicalBase::default();
                graphical.base.owner_part_id = Some(1);
                graphical.location_x = mils_f64_to_raw(parse_unit_value_or_mil(x1)?);
                graphical.location_y = mils_f64_to_raw(parse_unit_value_or_mil(y1)?);
                graphical.color = border_color_val;
                graphical.area_color = fill_color_val;

                let rect = SchRectangle {
                    graphical,
                    corner_x: mils_f64_to_raw(parse_unit_value_or_mil(x2)?),
                    corner_y: mils_f64_to_raw(parse_unit_value_or_mil(y2)?),
                    line_width: LineWidth::Small,
                    is_solid: *filled,
                    transparent: !filled,
                    ..Default::default()
                };
                primitives.push(SchRecord::Rectangle(rect));
            }
            SchLibGraphic::Arc {
                x,
                y,
                radius,
                start_angle,
                end_angle,
                color,
            } => {
                let color_val = parse_color(color)?;
                let mut graphical = SchGraphicalBase::default();
                graphical.base.owner_part_id = Some(1);
                graphical.location_x = mils_f64_to_raw(parse_unit_value_or_mil(x)?);
                graphical.location_y = mils_f64_to_raw(parse_unit_value_or_mil(y)?);
                graphical.color = color_val;

                let radius_raw = mils_f64_to_raw(parse_unit_value_or_mil(radius)?);
                let arc = SchArc {
                    graphical,
                    radius: radius_raw,
                    secondary_radius: radius_raw,
                    start_angle: *start_angle,
                    end_angle: *end_angle,
                    line_width: LineWidth::Small,
                    ..Default::default()
                };
                primitives.push(SchRecord::Arc(arc));
            }
            SchLibGraphic::Ellipse {
                x,
                y,
                radius_x,
                radius_y,
                filled,
                fill_color,
                border_color,
            } => {
                let fill_color_val = parse_color(fill_color)?;
                let border_color_val = parse_color(border_color)?;
                let mut graphical = SchGraphicalBase::default();
                graphical.base.owner_part_id = Some(1);
                graphical.location_x = mils_f64_to_raw(parse_unit_value_or_mil(x)?);
                graphical.location_y = mils_f64_to_raw(parse_unit_value_or_mil(y)?);
                graphical.color = border_color_val;
                graphical.area_color = fill_color_val;

                let ellipse = SchEllipse {
                    graphical,
                    radius_x: mils_f64_to_raw(parse_unit_value_or_mil(radius_x)?),
                    radius_y: mils_f64_to_raw(parse_unit_value_or_mil(radius_y)?),
                    is_solid: *filled,
                    transparent: !filled,
                    line_width: LineWidth::Small,
                    ..Default::default()
                };
                primitives.push(SchRecord::Ellipse(ellipse));
            }
            SchLibGraphic::Polyline { vertices, color } => {
                if vertices.len() < 2 {
                    return Err("Polyline must have at least 2 vertices".into());
                }
                let color_val = parse_color(color)?;
                let verts: Vec<(i32, i32)> = vertices
                    .iter()
                    .map(|v| {
                        Ok((
                            mils_f64_to_raw(parse_unit_value_or_mil(&v[0])?),
                            mils_f64_to_raw(parse_unit_value_or_mil(&v[1])?),
                        ))
                    })
                    .collect::<Result<_, Box<dyn std::error::Error>>>()?;

                let mut graphical = SchGraphicalBase::default();
                graphical.base.owner_part_id = Some(1);
                graphical.location_x = verts[0].0;
                graphical.location_y = verts[0].1;
                graphical.color = color_val;

                let polyline = SchPolyline {
                    graphical,
                    vertices: verts,
                    line_width: LineWidth::Small,
                    ..Default::default()
                };
                primitives.push(SchRecord::Polyline(polyline));
            }
            SchLibGraphic::Polygon {
                vertices,
                filled,
                fill_color,
                border_color,
            } => {
                if vertices.len() < 3 {
                    return Err("Polygon must have at least 3 vertices".into());
                }
                let fill_color_val = parse_color(fill_color)?;
                let border_color_val = parse_color(border_color)?;
                let verts: Vec<(i32, i32)> = vertices
                    .iter()
                    .map(|v| {
                        Ok((
                            mils_f64_to_raw(parse_unit_value_or_mil(&v[0])?),
                            mils_f64_to_raw(parse_unit_value_or_mil(&v[1])?),
                        ))
                    })
                    .collect::<Result<_, Box<dyn std::error::Error>>>()?;

                let mut graphical = SchGraphicalBase::default();
                graphical.base.owner_part_id = Some(1);
                graphical.location_x = verts[0].0;
                graphical.location_y = verts[0].1;
                graphical.color = border_color_val;
                graphical.area_color = fill_color_val;

                let polygon = SchPolygon {
                    graphical,
                    vertices: verts,
                    line_width: LineWidth::Small,
                    is_solid: *filled,
                    transparent: !filled,
                    ..Default::default()
                };
                primitives.push(SchRecord::Polygon(polygon));
            }
            SchLibGraphic::Text {
                x,
                y,
                text,
                orientation,
                justification,
                color,
            } => {
                let color_val = parse_color(color)?;
                let orient = parse_text_orientation(orientation)?;
                let justify = parse_text_justification(justification)?;

                let mut graphical = SchGraphicalBase::default();
                graphical.base.owner_part_id = Some(1);
                graphical.location_x = mils_f64_to_raw(parse_unit_value_or_mil(x)?);
                graphical.location_y = mils_f64_to_raw(parse_unit_value_or_mil(y)?);
                graphical.color = color_val;

                let label = SchLabel {
                    graphical,
                    text: text.clone(),
                    orientation: orient,
                    justification: justify,
                    font_id: 1,
                    is_hidden: false,
                    ..Default::default()
                };
                primitives.push(SchRecord::Label(label));
            }
        }
    }
    Ok(())
}
