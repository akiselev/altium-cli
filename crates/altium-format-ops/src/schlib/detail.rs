// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! SchLib detail commands: component, pins, primitives.

use std::collections::HashMap;
use std::path::Path;

use crate::helpers::*;
use crate::output::*;

use super::{coord_to_mils, count_primitives_via_view, open_schlib};

/// Returns detailed information about a single component.
pub fn cmd_component(
    path: &Path,
    name: &str,
    show_primitives: bool,
) -> Result<SchLibComponentDetail, Box<dyn std::error::Error>> {
    let lib = open_schlib(path)?;

    let entry_idx = lib
        .find_component(name)
        .ok_or_else(|| format!("Component '{}' not found", name))?;

    let result = lib
        .with_component_ref(entry_idx, |entry, view| {
            let comp_rec = view.component_record();
            let display_mode_count = comp_rec.display_mode_count() as i32;

            // Collect pin details
            let mut pins: Vec<PinDetail> = Vec::new();
            view.for_each_pin(|pin| {
                pins.push(PinDetail {
                    designator: pin.designator().to_string(),
                    name: pin.name().to_string(),
                    electrical_type: electrical_type_name(pin.electrical()).to_string(),
                    description: pin.description().to_string(),
                });
            });

            pins.sort_by(|a, b| alphanumeric_sort(&a.designator, &b.designator));

            let primitive_counts = if show_primitives {
                let counts = count_primitives_via_view(&view);
                let mut counts_vec: Vec<_> = counts
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v))
                    .collect();
                counts_vec.sort_by(|a, b| b.1.cmp(&a.1));
                Some(counts_vec)
            } else {
                None
            };

            SchLibComponentDetail {
                name: entry.lib_ref().to_string(),
                description: entry.description().to_string(),
                part_count: entry.part_count(),
                display_mode_count,
                pin_count: pins.len(),
                total_primitives: view.child_count(),
                pins,
                primitive_counts,
            }
        })
        .ok_or_else(|| format!("Component group not found for '{}'", name))?;

    Ok(result)
}

/// Lists pins for a specific component or all components if component is None.
pub fn cmd_pins(
    path: &Path,
    component: Option<String>,
) -> Result<SchLibPinList, Box<dyn std::error::Error>> {
    let lib = open_schlib(path)?;

    let filter_lower = component.as_ref().map(|s| s.to_lowercase());

    let mut all_pins: Vec<PinWithComponent> = Vec::new();

    lib.for_each_component_ref(|entry, view| {
        if let Some(ref filter) = filter_lower {
            if entry.lib_ref().to_lowercase() != *filter {
                return;
            }
        }

        view.for_each_pin(|pin| {
            all_pins.push(PinWithComponent {
                component_name: entry.lib_ref().to_string(),
                designator: pin.designator().to_string(),
                name: pin.name().to_string(),
                electrical_type: electrical_type_name(pin.electrical()).to_string(),
            });
        });
    });

    // Sort by component name, then by pin designator
    all_pins.sort_by(|a, b| {
        let cmp = alphanumeric_sort(&a.component_name, &b.component_name);
        if cmp == std::cmp::Ordering::Equal {
            alphanumeric_sort(&a.designator, &b.designator)
        } else {
            cmp
        }
    });

    // Group by electrical type
    let mut by_type: HashMap<String, Vec<PinWithComponent>> = HashMap::new();
    for pin in &all_pins {
        by_type
            .entry(pin.electrical_type.clone())
            .or_default()
            .push(pin.clone());
    }

    let type_order = [
        "Input",
        "Output",
        "Bidirectional",
        "Passive",
        "Power",
        "Hi-Z",
        "Open Collector",
        "Open Emitter",
    ];
    let mut pins_by_type = Vec::new();
    for type_name in type_order {
        if let Some(pins) = by_type.remove(type_name) {
            pins_by_type.push((type_name.to_string(), pins));
        }
    }
    for (type_name, pins) in by_type {
        pins_by_type.push((type_name, pins));
    }

    Ok(SchLibPinList {
        path: path.display().to_string(),
        total_pins: all_pins.len(),
        pins: all_pins,
        pins_by_type: Some(pins_by_type),
    })
}

/// Lists graphical primitives for a component.
pub fn cmd_primitives(
    path: &Path,
    component: &str,
) -> Result<SchLibPrimitiveList, Box<dyn std::error::Error>> {
    let lib = open_schlib(path)?;

    let entry_idx = lib
        .find_component(component)
        .ok_or_else(|| format!("Component '{}' not found", component))?;

    let result = lib
        .with_component_ref(entry_idx, |entry, view| {
            let mut primitives: Vec<PrimitiveInfo> = Vec::new();

            view.for_each_child(|child| {
                let record_id = child.record_id();
                let info = match record_id {
                    2 => {
                        // Pin
                        if let Some(pin) = child.as_pin() {
                            PrimitiveInfo::Pin {
                                designator: pin.designator().to_string(),
                                name: pin.name().to_string(),
                                electrical_type: electrical_type_name(pin.electrical()).to_string(),
                                x: coord_to_mils(pin.location_x()),
                                y: coord_to_mils(pin.location_y()),
                            }
                        } else {
                            return;
                        }
                    }
                    14 => {
                        // Rectangle
                        if let Some(rect) = child.as_rectangle() {
                            PrimitiveInfo::Rectangle {
                                x1: coord_to_mils(rect.location_x()),
                                y1: coord_to_mils(rect.location_y()),
                                x2: coord_to_mils(rect.corner_x()),
                                y2: coord_to_mils(rect.corner_y()),
                            }
                        } else {
                            return;
                        }
                    }
                    13 => {
                        // Line
                        if let Some(line) = child.as_line() {
                            PrimitiveInfo::Line {
                                x1: coord_to_mils(line.location_x()),
                                y1: coord_to_mils(line.location_y()),
                                x2: coord_to_mils(line.corner_x()),
                                y2: coord_to_mils(line.corner_y()),
                            }
                        } else {
                            return;
                        }
                    }
                    12 => {
                        // Arc
                        if let Some(arc) = child.as_arc() {
                            PrimitiveInfo::Arc {
                                center_x: coord_to_mils(arc.location_x()),
                                center_y: coord_to_mils(arc.location_y()),
                                radius: coord_to_mils(arc.radius()),
                                start_angle: arc.start_angle(),
                                end_angle: arc.end_angle(),
                            }
                        } else {
                            return;
                        }
                    }
                    7 => {
                        // Polygon
                        // STUB: vertex data not in typed API (vertices are #[altium(skip)])
                        PrimitiveInfo::Polygon { vertex_count: 0 }
                    }
                    6 => {
                        // Polyline
                        // STUB: vertex data not in typed API (vertices are #[altium(skip)])
                        PrimitiveInfo::Polyline { vertex_count: 0 }
                    }
                    4 => {
                        // Label
                        if let Some(label) = child.as_label() {
                            PrimitiveInfo::Label {
                                text: label.text().to_string(),
                                x: coord_to_mils(label.location_x()),
                                y: coord_to_mils(label.location_y()),
                            }
                        } else {
                            return;
                        }
                    }
                    // Skip Component, Parameter, Implementation records for primitive listing
                    1 | 41 | 44 | 45 => return,
                    _ => PrimitiveInfo::Other {
                        primitive_type: sch_record_type_name(record_id).to_string(),
                    },
                };
                primitives.push(info);
            });

            SchLibPrimitiveList {
                component_name: entry.lib_ref().to_string(),
                total_primitives: primitives.len(),
                primitives,
            }
        })
        .ok_or_else(|| format!("Component group not found for '{}'", component))?;

    Ok(result)
}
