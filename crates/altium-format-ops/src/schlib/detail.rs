// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! SchLib detail commands: component, pins, primitives.

use std::collections::HashMap;
use std::path::Path;

use altium_format::v2::traits::DocumentQuery;
use altium_format::v2::views::{
    SchArc, SchComponent, SchComponentView, SchLabel, SchLine, SchPin, SchRectangle,
};

use crate::helpers::*;
use crate::output::*;

use super::{coord_to_mils, count_primitives_via_view, open_schlib};

/// Returns detailed information about a single component.
pub fn cmd_component(
    path: &Path,
    name: &str,
    show_primitives: bool,
) -> Result<SchLibComponentDetail, Box<dyn std::error::Error>> {
    let mut lib = open_schlib(path)?;

    let entry_idx = lib
        .find_component(name)
        .ok_or_else(|| format!("Component '{}' not found", name))?;

    let (comp, children) = lib.groups[entry_idx].split_borrow();
    let entry = &lib.component_entries[entry_idx];
    let mut view = SchComponentView::new(comp, children);

    let display_mode_count = view.display_mode_count() as i32;

    // Collect pin details
    let mut pins: Vec<PinDetail> = Vec::new();
    let pin_keys: Vec<_> = view.child_keys::<SchPin>().collect();
    for key in pin_keys {
        view.with_child_mut(key, |pin| {
            pins.push(PinDetail {
                designator: pin.designator().to_string(),
                name: pin.name().to_string(),
                electrical_type: electrical_type_name(pin.electrical()).to_string(),
                description: pin.description().to_string(),
            });
        });
    }

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

    Ok(SchLibComponentDetail {
        name: entry.lib_ref().to_string(),
        description: entry.description().to_string(),
        part_count: entry.part_count(),
        display_mode_count,
        pin_count: pins.len(),
        total_primitives: view.children_len(),
        pins,
        primitive_counts,
    })
}

/// Lists pins for a specific component or all components if component is None.
pub fn cmd_pins(
    path: &Path,
    component: Option<String>,
) -> Result<SchLibPinList, Box<dyn std::error::Error>> {
    let mut lib = open_schlib(path)?;

    let filter_lower = component.as_ref().map(|s| s.to_lowercase());

    let mut all_pins: Vec<PinWithComponent> = Vec::new();

    DocumentQuery::<SchComponent>::query_all(&mut lib, "#1")?.for_each_mut(|entry, mut view| {
        if let Some(ref filter) = filter_lower {
            if entry.lib_ref().to_lowercase() != *filter {
                return;
            }
        }

        let keys: Vec<_> = view.child_keys::<SchPin>().collect();
        for key in keys {
            view.with_child_mut(key, |pin| {
                all_pins.push(PinWithComponent {
                    component_name: entry.lib_ref().to_string(),
                    designator: pin.designator().to_string(),
                    name: pin.name().to_string(),
                    electrical_type: electrical_type_name(pin.electrical()).to_string(),
                });
            });
        }
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
    let mut lib = open_schlib(path)?;

    let entry_idx = lib
        .find_component(component)
        .ok_or_else(|| format!("Component '{}' not found", component))?;

    let entry_name = lib.component_entries[entry_idx].lib_ref().to_string();

    let (comp, children) = lib.groups[entry_idx].split_borrow();
    let mut view = SchComponentView::new(comp, children);

    let mut primitives: Vec<PrimitiveInfo> = Vec::new();

    // Iterate by record type using child_record_ids to preserve original order.
    // We collect (index, record_id) pairs, then process each by type.
    let child_info: Vec<(usize, u8)> = view
        .child_record_ids()
        .enumerate()
        .collect();

    for (idx, record_id) in child_info {
        match record_id {
            2 => {
                // Pin
                use altium_format::v2::views::child_handle::ChildKey;
                let key: ChildKey<SchPin> = ChildKey::new(idx);
                view.with_child_mut(key, |pin| {
                    primitives.push(PrimitiveInfo::Pin {
                        designator: pin.designator().to_string(),
                        name: pin.name().to_string(),
                        electrical_type: electrical_type_name(pin.electrical()).to_string(),
                        x: coord_to_mils(pin.location_x()),
                        y: coord_to_mils(pin.location_y()),
                    });
                });
            }
            14 => {
                // Rectangle
                use altium_format::v2::views::child_handle::ChildKey;
                let key: ChildKey<SchRectangle> = ChildKey::new(idx);
                view.with_child_mut(key, |rect| {
                    primitives.push(PrimitiveInfo::Rectangle {
                        x1: coord_to_mils(rect.location_x()),
                        y1: coord_to_mils(rect.location_y()),
                        x2: coord_to_mils(rect.corner_x()),
                        y2: coord_to_mils(rect.corner_y()),
                    });
                });
            }
            13 => {
                // Line
                use altium_format::v2::views::child_handle::ChildKey;
                let key: ChildKey<SchLine> = ChildKey::new(idx);
                view.with_child_mut(key, |line| {
                    primitives.push(PrimitiveInfo::Line {
                        x1: coord_to_mils(line.location_x()),
                        y1: coord_to_mils(line.location_y()),
                        x2: coord_to_mils(line.corner_x()),
                        y2: coord_to_mils(line.corner_y()),
                    });
                });
            }
            12 => {
                // Arc
                use altium_format::v2::views::child_handle::ChildKey;
                let key: ChildKey<SchArc> = ChildKey::new(idx);
                view.with_child_mut(key, |arc| {
                    primitives.push(PrimitiveInfo::Arc {
                        center_x: coord_to_mils(arc.location_x()),
                        center_y: coord_to_mils(arc.location_y()),
                        radius: coord_to_mils(arc.radius()),
                        start_angle: arc.start_angle(),
                        end_angle: arc.end_angle(),
                    });
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
                use altium_format::v2::views::child_handle::ChildKey;
                let key: ChildKey<SchLabel> = ChildKey::new(idx);
                view.with_child_mut(key, |label| {
                    primitives.push(PrimitiveInfo::Label {
                        text: label.text().to_string(),
                        x: coord_to_mils(label.location_x()),
                        y: coord_to_mils(label.location_y()),
                    });
                });
            }
            // Skip Component, Parameter, Implementation records for primitive listing
            1 | 41 | 44 | 45 => {}
            _ => {
                primitives.push(PrimitiveInfo::Other {
                    primitive_type: sch_record_type_name(record_id).to_string(),
                });
            }
        }
    }

    Ok(SchLibPrimitiveList {
        component_name: entry_name,
        total_primitives: primitives.len(),
        primitives,
    })
}
