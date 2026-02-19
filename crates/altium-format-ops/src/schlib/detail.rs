// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! SchLib detail commands: component, pins, primitives.

use std::collections::HashMap;
use std::path::Path;

use altium_format::v2::handles::{
    SchArcHandle, SchComponent, SchComponentHandle, SchLabelHandle, SchLineHandle, SchPin,
    SchPinHandle, SchRectangleHandle,
};
use altium_format::v2::traits::DocumentQuery;

use crate::helpers::*;
use crate::output::*;

use super::{coord_to_mils, count_primitives, open_schlib};

/// Returns detailed information about a single component.
pub fn cmd_component(
    path: &Path,
    name: &str,
    show_primitives: bool,
) -> Result<SchLibComponentDetail, Box<dyn std::error::Error>> {
    let lib = open_schlib(path)?;

    let group_id = lib
        .find_component(name)
        .ok_or_else(|| format!("Component '{}' not found", name))?;

    let comp = SchComponentHandle::new(lib.store().clone(), group_id);

    let display_mode_count = comp.read().display_mode_count() as i32;

    // Collect pin details
    let mut pins: Vec<PinDetail> = Vec::new();
    let pin_handles = comp.children::<SchPin>();
    for pin_handle in &pin_handles {
        let pin = pin_handle.read();
        pins.push(PinDetail {
            designator: pin.designator().to_string(),
            name: pin.name().to_string(),
            electrical_type: electrical_type_name(pin.electrical()).to_string(),
            description: pin.description().to_string(),
        });
    }

    pins.sort_by(|a, b| alphanumeric_sort(&a.designator, &b.designator));

    let primitive_counts = if show_primitives {
        let counts = count_primitives(&comp);
        let mut counts_vec: Vec<_> = counts
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        counts_vec.sort_by(|a, b| b.1.cmp(&a.1));
        Some(counts_vec)
    } else {
        None
    };

    Ok(SchLibComponentDetail {
        name: comp.lib_ref(),
        description: comp.description(),
        part_count: comp.part_count(),
        display_mode_count,
        pin_count: pins.len(),
        total_primitives: comp.children_len(),
        pins,
        primitive_counts,
    })
}

/// Lists pins for a specific component or all components if component is None.
pub fn cmd_pins(
    path: &Path,
    component: Option<String>,
) -> Result<SchLibPinList, Box<dyn std::error::Error>> {
    let lib = open_schlib(path)?;

    let filter_lower = component.as_ref().map(|s| s.to_lowercase());

    let mut all_pins: Vec<PinWithComponent> = Vec::new();

    let components = DocumentQuery::<SchComponent>::query_all(&lib, "#1")?;
    for comp in &components {
        if let Some(ref filter) = filter_lower {
            if comp.lib_ref().to_lowercase() != *filter {
                continue;
            }
        }

        let pins = comp.children::<SchPin>();
        for pin_handle in &pins {
            let pin = pin_handle.read();
            all_pins.push(PinWithComponent {
                component_name: comp.lib_ref(),
                designator: pin.designator().to_string(),
                name: pin.name().to_string(),
                electrical_type: electrical_type_name(pin.electrical()).to_string(),
            });
        }
    }

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

    let group_id = lib
        .find_component(component)
        .ok_or_else(|| format!("Component '{}' not found", component))?;

    let comp = SchComponentHandle::new(lib.store().clone(), group_id);
    let component_name = comp.lib_ref();

    let mut primitives: Vec<PrimitiveInfo> = Vec::new();

    // Iterate all children preserving original order.
    let child_info = comp.all_children();

    for (type_id, record_id) in child_info {
        match type_id {
            2 => {
                // Pin
                let pin_handle = SchPinHandle::new(lib.store().clone(), record_id);
                let pin = pin_handle.read();
                primitives.push(PrimitiveInfo::Pin {
                    designator: pin.designator().to_string(),
                    name: pin.name().to_string(),
                    electrical_type: electrical_type_name(pin.electrical()).to_string(),
                    x: coord_to_mils(pin.location_x()),
                    y: coord_to_mils(pin.location_y()),
                });
            }
            14 => {
                // Rectangle
                let rect_handle = SchRectangleHandle::new(lib.store().clone(), record_id);
                let rect = rect_handle.read();
                primitives.push(PrimitiveInfo::Rectangle {
                    x1: coord_to_mils(rect.location_x()),
                    y1: coord_to_mils(rect.location_y()),
                    x2: coord_to_mils(rect.corner_x()),
                    y2: coord_to_mils(rect.corner_y()),
                });
            }
            13 => {
                // Line
                let line_handle = SchLineHandle::new(lib.store().clone(), record_id);
                let line = line_handle.read();
                primitives.push(PrimitiveInfo::Line {
                    x1: coord_to_mils(line.location_x()),
                    y1: coord_to_mils(line.location_y()),
                    x2: coord_to_mils(line.corner_x()),
                    y2: coord_to_mils(line.corner_y()),
                });
            }
            12 => {
                // Arc
                let arc_handle = SchArcHandle::new(lib.store().clone(), record_id);
                let arc = arc_handle.read();
                primitives.push(PrimitiveInfo::Arc {
                    center_x: coord_to_mils(arc.location_x()),
                    center_y: coord_to_mils(arc.location_y()),
                    radius: coord_to_mils(arc.radius()),
                    start_angle: arc.start_angle(),
                    end_angle: arc.end_angle(),
                });
            }
            7 => {
                // Polygon
                // STUB: vertex data not in typed API (vertices are #[altium(skip)])
                primitives.push(PrimitiveInfo::Polygon { vertex_count: 0 });
            }
            6 => {
                // Polyline
                // STUB: vertex data not in typed API (vertices are #[altium(skip)])
                primitives.push(PrimitiveInfo::Polyline { vertex_count: 0 });
            }
            4 => {
                // Label
                let label_handle = SchLabelHandle::new(lib.store().clone(), record_id);
                let label = label_handle.read();
                primitives.push(PrimitiveInfo::Label {
                    text: label.text().to_string(),
                    x: coord_to_mils(label.location_x()),
                    y: coord_to_mils(label.location_y()),
                });
            }
            // Skip Component, Parameter, Implementation records for primitive listing
            1 | 41 | 44 | 45 => {}
            _ => {
                primitives.push(PrimitiveInfo::Other {
                    primitive_type: sch_record_type_name(type_id).to_string(),
                });
            }
        }
    }

    Ok(SchLibPrimitiveList {
        component_name,
        total_primitives: primitives.len(),
        primitives,
    })
}
