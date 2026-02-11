// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Schematic library operations (v2).
//!
//! Provides high-level operations for exploring and manipulating Altium schematic
//! library (.SchLib) files using the v2 backing-store architecture.

use std::collections::HashMap;
use std::io::Cursor;
use std::path::Path;

use crate::v2::backing_store::{ComponentGroup, ParamOrigin, RecordNode, RecordOrigin};
use crate::v2::coord::{AltiumCoord, SchCoord};
use crate::v2::documents::schlib::{SchLib, SchLibComponentEntry};
use crate::v2::ops::categorization::categorize_component;
use crate::v2::ops::output::*;
use crate::v2::records::enums::PinElectricalType;
use crate::v2::records::sch_arc::SchArcRecord;
use crate::v2::records::sch_component::SchComponentRecord;
use crate::v2::records::sch_label::SchLabelRecord;
use crate::v2::records::sch_line::SchLineRecord;
use crate::v2::records::sch_pin::SchPinRecord;
use crate::v2::records::sch_rectangle::SchRectangleRecord;
use crate::v2::traits::AltiumEnum;

// ═══════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Sorts strings with embedded numbers naturally (e.g., "A2" < "A10").
fn alphanumeric_sort(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    let mut a_chars = a.chars().peekable();
    let mut b_chars = b.chars().peekable();

    loop {
        match (a_chars.peek(), b_chars.peek()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(&ac), Some(&bc)) => {
                if ac.is_ascii_digit() && bc.is_ascii_digit() {
                    let a_num: String = a_chars
                        .by_ref()
                        .take_while(|c| c.is_ascii_digit())
                        .collect();
                    let b_num: String = b_chars
                        .by_ref()
                        .take_while(|c| c.is_ascii_digit())
                        .collect();
                    let a_val: u64 = a_num.parse().unwrap_or(0);
                    let b_val: u64 = b_num.parse().unwrap_or(0);
                    match a_val.cmp(&b_val) {
                        Ordering::Equal => continue,
                        other => return other,
                    }
                } else {
                    a_chars.next();
                    b_chars.next();
                    match ac.cmp(&bc) {
                        Ordering::Equal => continue,
                        other => return other,
                    }
                }
            }
        }
    }
}

/// Opens and parses a SchLib file from the given path.
fn open_schlib(path: &Path) -> Result<SchLib, Box<dyn std::error::Error>> {
    Ok(SchLib::open_file(path).map_err(|e| e.to_string())?)
}

/// Get the electrical type name for display.
fn electrical_type_name(electrical: PinElectricalType) -> &'static str {
    match electrical {
        PinElectricalType::Input => "Input",
        PinElectricalType::Output => "Output",
        PinElectricalType::IO => "Bidirectional",
        PinElectricalType::Passive => "Passive",
        PinElectricalType::Power => "Power",
        PinElectricalType::HiZ => "Hi-Z",
        PinElectricalType::OpenCollector => "Open Collector",
        PinElectricalType::OpenEmitter => "Open Emitter",
    }
}

/// Parse electrical type from string.
fn parse_electrical_type(s: &str) -> PinElectricalType {
    match s.to_lowercase().as_str() {
        "input" | "in" => PinElectricalType::Input,
        "output" | "out" => PinElectricalType::Output,
        "io" | "bidirectional" | "bidir" => PinElectricalType::IO,
        "passive" | "pass" => PinElectricalType::Passive,
        "power" | "pwr" => PinElectricalType::Power,
        "hiz" | "hi-z" | "tristate" => PinElectricalType::HiZ,
        "opencollector" | "open_collector" | "oc" => PinElectricalType::OpenCollector,
        "openemitter" | "open_emitter" | "oe" => PinElectricalType::OpenEmitter,
        _ => PinElectricalType::Passive, // default
    }
}

/// Map a record key (u8) to a human-readable type name.
fn record_type_name(key: u8) -> &'static str {
    match key {
        1 => "Component",
        2 => "Pin",
        3 => "Symbol",
        4 => "Label",
        5 => "Bezier",
        6 => "Polyline",
        7 => "Polygon",
        8 => "Ellipse",
        9 => "Pie",
        10 => "RoundRectangle",
        11 => "EllipticalArc",
        12 => "Arc",
        13 => "Line",
        14 => "Rectangle",
        17 => "Power",
        18 => "Port",
        22 => "NoERC",
        25 => "NetLabel",
        26 => "Bus",
        27 => "Wire",
        28 => "TextFrame",
        29 => "Junction",
        30 => "Image",
        31 => "Sheet",
        32 => "SheetName",
        33 => "SheetFileName",
        34 => "Designator",
        37 => "BusEntry",
        39 => "SheetSymbol",
        40 => "SheetEntry",
        41 => "Parameter",
        44 => "ImplementationList",
        45 => "Implementation",
        209 => "Note",
        255 => "Blanket",
        _ => "Unknown",
    }
}

/// Count the number of pin children in a component group.
fn count_pins(group: &ComponentGroup) -> usize {
    group.children.iter().filter(|c| c.key == 2).count()
}

/// Count primitives by type name in a component group's children.
fn count_primitives(group: &ComponentGroup) -> HashMap<&'static str, usize> {
    let mut counts: HashMap<&'static str, usize> = HashMap::new();
    for child in &group.children {
        let name = record_type_name(child.key);
        *counts.entry(name).or_insert(0) += 1;
    }
    counts
}

/// Convert schematic coordinates to mils for display.
fn coord_to_mils(value: SchCoord) -> String {
    format!("{:.1}", value.to_mils())
}

// ═══════════════════════════════════════════════════════════════════════════
// BROWSE COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// Returns library overview with statistics and component category breakdown.
pub fn cmd_overview(path: &Path) -> Result<SchLibOverview, Box<dyn std::error::Error>> {
    let lib = open_schlib(path)?;

    // ─────────────────────────────────────────────────────────────────────────
    // 1. COMPONENTS BY CATEGORY
    // ─────────────────────────────────────────────────────────────────────────
    let mut categories: HashMap<&'static str, Vec<ComponentSummary>> = HashMap::new();

    for (i, entry) in lib.component_entries.iter().enumerate() {
        let category = categorize_component(&entry.lib_ref, &entry.description);
        let pin_count = if i < lib.groups.len() {
            count_pins(&lib.groups[i])
        } else {
            0
        };
        categories
            .entry(category)
            .or_default()
            .push(ComponentSummary {
                name: entry.lib_ref.clone(),
                description: entry.description.clone(),
                pin_count,
                part_count: entry.part_count,
            });
    }

    // Sort categories by importance
    let category_order = [
        "Microcontroller",
        "FPGA/CPLD",
        "Memory",
        "ADC",
        "DAC",
        "Transceiver/PHY",
        "Clock/Oscillator",
        "Power Supply",
        "Amplifier",
        "Mux/Switch",
        "Buffer/Driver",
        "Capacitor",
        "Resistor",
        "Inductor/Ferrite",
        "Diode/Protection",
        "Transistor",
        "LED",
        "Connector",
        "Test Point",
        "Other IC",
    ];

    let mut components_by_category = Vec::new();
    for category in category_order.iter() {
        if let Some(mut comps) = categories.remove(*category) {
            comps.sort_by(|a, b| alphanumeric_sort(&a.name, &b.name));
            components_by_category.push((category.to_string(), comps));
        }
    }
    // Add any uncategorized
    for (category, mut comps) in categories {
        if !comps.is_empty() {
            comps.sort_by(|a, b| alphanumeric_sort(&a.name, &b.name));
            components_by_category.push((category.to_string(), comps));
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 2. PIN STATISTICS
    // ─────────────────────────────────────────────────────────────────────────
    let mut total_pins = 0;
    let mut pin_types: HashMap<&'static str, usize> = HashMap::new();

    for group in &lib.groups {
        for child in &group.children {
            if child.key == 2 {
                total_pins += 1;
                let pin = SchPinRecord::from_origin(child.origin.clone());
                let electrical = pin.electrical();
                let type_name = electrical_type_name(electrical);
                *pin_types.entry(type_name).or_insert(0) += 1;
            }
        }
    }

    let mut pin_types_vec: Vec<_> = pin_types
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    pin_types_vec.sort_by(|a, b| b.1.cmp(&a.1));

    // ─────────────────────────────────────────────────────────────────────────
    // 3. MULTI-PART COMPONENTS
    // ─────────────────────────────────────────────────────────────────────────
    let mut multi_part_components: Vec<ComponentSummary> = lib
        .component_entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.part_count > 1)
        .map(|(i, e)| ComponentSummary {
            name: e.lib_ref.clone(),
            description: e.description.clone(),
            pin_count: if i < lib.groups.len() {
                count_pins(&lib.groups[i])
            } else {
                0
            },
            part_count: e.part_count,
        })
        .collect();
    multi_part_components.sort_by(|a, b| b.part_count.cmp(&a.part_count));

    // ─────────────────────────────────────────────────────────────────────────
    // 4. LARGEST COMPONENTS (by pin count)
    // ─────────────────────────────────────────────────────────────────────────
    let mut by_pins: Vec<(usize, usize)> = lib
        .component_entries
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let pins = if i < lib.groups.len() {
                count_pins(&lib.groups[i])
            } else {
                0
            };
            (i, pins)
        })
        .collect();
    by_pins.sort_by_key(|(_, pins)| std::cmp::Reverse(*pins));

    let largest_components = by_pins
        .iter()
        .take(10)
        .map(|(i, pins)| {
            let entry = &lib.component_entries[*i];
            ComponentSummary {
                name: entry.lib_ref.clone(),
                description: entry.description.clone(),
                pin_count: *pins,
                part_count: entry.part_count,
            }
        })
        .collect();

    Ok(SchLibOverview {
        path: path.display().to_string(),
        total_components: lib.component_entries.len(),
        components_by_category,
        pin_statistics: PinStatistics {
            total_pins,
            pin_types: pin_types_vec,
        },
        multi_part_components,
        largest_components,
        component_details: None,
    })
}

/// Lists all components in the library sorted alphanumerically.
pub fn cmd_list(path: &Path) -> Result<SchLibComponentList, Box<dyn std::error::Error>> {
    let lib = open_schlib(path)?;

    let mut components: Vec<ComponentSummary> = lib
        .component_entries
        .iter()
        .enumerate()
        .map(|(i, e)| ComponentSummary {
            name: e.lib_ref.clone(),
            description: e.description.clone(),
            pin_count: if i < lib.groups.len() {
                count_pins(&lib.groups[i])
            } else {
                0
            },
            part_count: e.part_count,
        })
        .collect();

    components.sort_by(|a, b| alphanumeric_sort(&a.name, &b.name));

    Ok(SchLibComponentList {
        path: path.display().to_string(),
        total_components: lib.component_entries.len(),
        components,
    })
}

/// Searches for components matching the query in name or description.
pub fn cmd_search(
    path: &Path,
    query: &str,
    limit: Option<usize>,
) -> Result<SchLibSearchResults, Box<dyn std::error::Error>> {
    let lib = open_schlib(path)?;

    let query_lower = query.to_lowercase();
    let has_wildcard = query.contains('*');

    let mut matches: Vec<ComponentSummary> = lib
        .component_entries
        .iter()
        .enumerate()
        .filter(|(_, e)| {
            let name = e.lib_ref.to_lowercase();
            let desc = e.description.to_lowercase();

            if has_wildcard {
                let pattern = query_lower.replace('*', "");
                name.contains(&pattern) || desc.contains(&pattern)
            } else {
                name.contains(&query_lower) || desc.contains(&query_lower)
            }
        })
        .map(|(i, e)| ComponentSummary {
            name: e.lib_ref.clone(),
            description: e.description.clone(),
            pin_count: if i < lib.groups.len() {
                count_pins(&lib.groups[i])
            } else {
                0
            },
            part_count: e.part_count,
        })
        .collect();

    // Sort by relevance (exact name match first, then by name)
    matches.sort_by(|a, b| {
        let a_exact = a.name.to_lowercase() == query_lower;
        let b_exact = b.name.to_lowercase() == query_lower;
        match (a_exact, b_exact) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => alphanumeric_sort(&a.name, &b.name),
        }
    });

    if let Some(max) = limit {
        matches.truncate(max);
    }

    let total_matches = matches.len();

    Ok(SchLibSearchResults {
        query: query.to_string(),
        total_matches,
        results: matches,
    })
}

/// Returns detailed library metadata including file info and header data.
pub fn cmd_info(path: &Path) -> Result<SchLibInfo, Box<dyn std::error::Error>> {
    let lib = open_schlib(path)?;

    let mut primitive_counts: HashMap<&'static str, usize> = HashMap::new();
    let mut total_primitives = 0;
    let mut multi_part_count = 0;

    for (i, group) in lib.groups.iter().enumerate() {
        let counts = count_primitives(group);
        for (name, count) in counts {
            *primitive_counts.entry(name).or_insert(0) += count;
            total_primitives += count;
        }
        if i < lib.component_entries.len() && lib.component_entries[i].part_count > 1 {
            multi_part_count += 1;
        }
    }

    let mut primitive_types: Vec<_> = primitive_counts
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    primitive_types.sort_by(|a, b| b.1.cmp(&a.1));

    Ok(SchLibInfo {
        path: path.display().to_string(),
        component_count: lib.component_entries.len(),
        total_primitives,
        primitive_types,
        multi_part_count,
    })
}

/// Returns detailed information about a single component.
pub fn cmd_component(
    path: &Path,
    name: &str,
    show_primitives: bool,
) -> Result<SchLibComponentDetail, Box<dyn std::error::Error>> {
    let lib = open_schlib(path)?;

    let name_lower = name.to_lowercase();
    let (entry_idx, entry) = lib
        .component_entries
        .iter()
        .enumerate()
        .find(|(_, e)| e.lib_ref.to_lowercase() == name_lower)
        .ok_or_else(|| format!("Component '{}' not found", name))?;

    let group = lib
        .groups
        .get(entry_idx)
        .ok_or_else(|| format!("Component group not found for '{}'", name))?;

    let comp_rec = SchComponentRecord::from_origin(group.component.origin.clone());
    let display_mode_count = comp_rec.display_mode_count() as i32;

    // Collect pin details
    let mut pins: Vec<PinDetail> = group
        .children
        .iter()
        .filter(|c| c.key == 2)
        .map(|c| {
            let pin = SchPinRecord::from_origin(c.origin.clone());
            PinDetail {
                designator: pin.designator().to_string(),
                name: pin.name().to_string(),
                electrical_type: electrical_type_name(pin.electrical()).to_string(),
                description: pin.description(),
            }
        })
        .collect();

    pins.sort_by(|a, b| alphanumeric_sort(&a.designator, &b.designator));

    let primitive_counts = if show_primitives {
        let counts = count_primitives(group);
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
        name: entry.lib_ref.clone(),
        description: entry.description.clone(),
        part_count: entry.part_count,
        display_mode_count,
        pin_count: pins.len(),
        total_primitives: group.children.len(),
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

    for (i, group) in lib.groups.iter().enumerate() {
        let entry = match lib.component_entries.get(i) {
            Some(e) => e,
            None => continue,
        };

        if let Some(ref filter) = filter_lower {
            if entry.lib_ref.to_lowercase() != *filter {
                continue;
            }
        }

        for child in &group.children {
            if child.key == 2 {
                let pin = SchPinRecord::from_origin(child.origin.clone());
                all_pins.push(PinWithComponent {
                    component_name: entry.lib_ref.clone(),
                    designator: pin.designator().to_string(),
                    name: pin.name().to_string(),
                    electrical_type: electrical_type_name(pin.electrical()).to_string(),
                });
            }
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

    let name_lower = component.to_lowercase();
    let (entry_idx, _entry) = lib
        .component_entries
        .iter()
        .enumerate()
        .find(|(_, e)| e.lib_ref.to_lowercase() == name_lower)
        .ok_or_else(|| format!("Component '{}' not found", component))?;

    let group = lib
        .groups
        .get(entry_idx)
        .ok_or_else(|| format!("Component group not found for '{}'", component))?;

    let mut primitives: Vec<PrimitiveInfo> = Vec::new();

    for child in &group.children {
        let info = match child.key {
            2 => {
                // Pin
                let pin = SchPinRecord::from_origin(child.origin.clone());
                PrimitiveInfo::Pin {
                    designator: pin.designator().to_string(),
                    name: pin.name().to_string(),
                    electrical_type: electrical_type_name(pin.electrical()).to_string(),
                    x: coord_to_mils(pin.location_x()),
                    y: coord_to_mils(pin.location_y()),
                }
            }
            14 => {
                // Rectangle
                let rect = SchRectangleRecord::from_origin(child.origin.clone());
                PrimitiveInfo::Rectangle {
                    x1: coord_to_mils(rect.location_x()),
                    y1: coord_to_mils(rect.location_y()),
                    x2: coord_to_mils(rect.corner_x()),
                    y2: coord_to_mils(rect.corner_y()),
                }
            }
            13 => {
                // Line
                let line = SchLineRecord::from_origin(child.origin.clone());
                PrimitiveInfo::Line {
                    x1: coord_to_mils(line.location_x()),
                    y1: coord_to_mils(line.location_y()),
                    x2: coord_to_mils(line.corner_x()),
                    y2: coord_to_mils(line.corner_y()),
                }
            }
            12 => {
                // Arc
                let arc = SchArcRecord::from_origin(child.origin.clone());
                PrimitiveInfo::Arc {
                    center_x: coord_to_mils(arc.location_x()),
                    center_y: coord_to_mils(arc.location_y()),
                    radius: coord_to_mils(arc.radius()),
                    start_angle: arc.start_angle(),
                    end_angle: arc.end_angle(),
                }
            }
            7 => {
                // Polygon
                let vertex_count = count_vertices(child);
                PrimitiveInfo::Polygon { vertex_count }
            }
            6 => {
                // Polyline
                let vertex_count = count_vertices(child);
                PrimitiveInfo::Polyline { vertex_count }
            }
            4 => {
                // Label
                let label = SchLabelRecord::from_origin(child.origin.clone());
                PrimitiveInfo::Label {
                    text: label.text(),
                    x: coord_to_mils(label.location_x()),
                    y: coord_to_mils(label.location_y()),
                }
            }
            // Skip Component, Parameter, Implementation records for primitive listing
            1 | 41 | 44 | 45 => continue,
            _ => PrimitiveInfo::Other {
                primitive_type: record_type_name(child.key).to_string(),
            },
        };
        primitives.push(info);
    }

    Ok(SchLibPrimitiveList {
        component_name: lib.component_entries[entry_idx].lib_ref.clone(),
        total_primitives: primitives.len(),
        primitives,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// MANIPULATION COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// Embedded blank SchLib template.
const BLANK_SCHLIB_TEMPLATE: &[u8] = include_bytes!("../../../data/blank/Schlib1.SchLib");

/// Creates an empty SchLib file at the given path.
pub fn cmd_create(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if path.exists() {
        return Err(format!("File already exists: {}", path.display()).into());
    }

    std::fs::write(path, BLANK_SCHLIB_TEMPLATE)
        .map_err(|e| format!("Error creating file: {}", e))?;

    println!("Created empty SchLib: {}", path.display());
    Ok(())
}

/// Adds a new component to an existing library.
pub fn cmd_add_component(
    path: &Path,
    name: &str,
    description: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut lib = open_schlib(path)?;

    // Check if component already exists
    let name_lower = name.to_lowercase();
    if lib
        .component_entries
        .iter()
        .any(|e| e.lib_ref.to_lowercase() == name_lower)
    {
        return Err(format!("Component '{}' already exists in library", name).into());
    }

    // Add component entry
    lib.component_entries.push(SchLibComponentEntry {
        lib_ref: name.to_string(),
        description: description.unwrap_or_default(),
        part_count: 1,
    });

    // Add component group with a RECORD=1 node
    let param_str = format!(
        "|RECORD=1|LIBREFERENCE={}|PARTCOUNT=1|DISPLAYMODECOUNT=1|CURRENTPARTID=1|",
        name
    );
    let origin = RecordOrigin::Param(ParamOrigin::new(&param_str));
    let component_node = RecordNode::new(1, origin);
    lib.groups.push(ComponentGroup::new(
        component_node,
        Vec::new(),
        Vec::new(),
    ));

    // Clear raw header to force rebuild
    lib.header.raw = None;

    // Write back
    let buf = Cursor::new(Vec::new());
    lib.save(buf).map_err(|e| e.to_string())?;

    // Re-save to actual file
    lib.save_file(path).map_err(|e| e.to_string())?;

    println!("Added component '{}' to {}", name, path.display());
    Ok(())
}

/// Adds a pin to an existing component in the library.
pub fn cmd_add_pin(
    path: &Path,
    component: &str,
    designator: &str,
    name: &str,
    electrical_type: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut lib = open_schlib(path)?;

    // Find component
    let comp_lower = component.to_lowercase();
    let entry_idx = lib
        .component_entries
        .iter()
        .position(|e| e.lib_ref.to_lowercase() == comp_lower)
        .ok_or_else(|| format!("Component '{}' not found", component))?;

    let group = lib
        .groups
        .get_mut(entry_idx)
        .ok_or_else(|| format!("Component group not found for '{}'", component))?;

    // Parse electrical type
    let electrical = parse_electrical_type(electrical_type);
    let electrical_int = electrical.to_int();

    // Create pin record
    let pin_params = format!(
        "|RECORD=2|OWNERINDEX=0|OWNERPARTID=1|NAME={}|DESIGNATOR={}|ELECTRICAL={}|PINCONGLOMERATE=25|",
        name, designator, electrical_int
    );
    let origin = RecordOrigin::Param(ParamOrigin::new(&pin_params));
    let pin_node = RecordNode::new(2, origin);

    group.children.push(pin_node);

    // Clear raw header to force rebuild
    lib.header.raw = None;

    // Write back
    lib.save_file(path).map_err(|e| e.to_string())?;

    println!(
        "Added pin '{}' ({}) to component '{}' in {}",
        designator,
        name,
        component,
        path.display()
    );
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// EXPORT COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// Serializes the library to JSON for LLM processing or external analysis.
pub fn cmd_json(path: &Path, full: bool) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let lib = open_schlib(path)?;

    if full {
        Ok(serde_json::to_value(&lib)?)
    } else {
        let components: Vec<serde_json::Value> = lib
            .component_entries
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let pin_count = if i < lib.groups.len() {
                    count_pins(&lib.groups[i])
                } else {
                    0
                };
                let primitive_count = if i < lib.groups.len() {
                    lib.groups[i].children.len()
                } else {
                    0
                };
                serde_json::json!({
                    "name": e.lib_ref,
                    "description": e.description,
                    "pin_count": pin_count,
                    "part_count": e.part_count,
                    "primitive_count": primitive_count,
                })
            })
            .collect();

        Ok(serde_json::json!({
            "file": path.display().to_string(),
            "unique_id": lib.header.unique_id,
            "component_count": lib.component_entries.len(),
            "components": components,
        }))
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// INTERNAL HELPERS
// ═══════════════════════════════════════════════════════════════════════════

/// Count vertices (LOCATIONCOUNT parameter) in a polygon/polyline record.
///
/// NOTE: Raw param access is intentional here -- vertex data is stored as
/// dynamic X1/Y1/X2/Y2/... keys which aren't covered by the typed record
/// API (vertices are `#[altium(skip)]` on SchPolylineRecord/SchPolygonRecord).
fn count_vertices(node: &RecordNode) -> usize {
    let count = node
        .origin
        .as_param()
        .and_then(|p| p.params.get("LOCATIONCOUNT"))
        .map(|v| v.as_int_or(0))
        .unwrap_or(0);
    if count > 0 {
        count as usize
    } else {
        // Fall back to counting X1/X2/... style parameters
        let mut n = 0;
        if let Some(param) = node.origin.as_param() {
            while param.params.get(&format!("X{}", n + 1)).is_some() {
                n += 1;
            }
        }
        n
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alphanumeric_sort() {
        let mut items = vec!["A10", "A2", "A1", "B1"];
        items.sort_by(|a, b| alphanumeric_sort(a, b));
        assert_eq!(items, vec!["A1", "A2", "A10", "B1"]);
    }

    #[test]
    fn test_alphanumeric_sort_mixed() {
        let mut items = vec!["PIN10", "PIN2", "PIN1", "PIN20", "VCC", "GND"];
        items.sort_by(|a, b| alphanumeric_sort(a, b));
        assert_eq!(
            items,
            vec!["GND", "PIN1", "PIN2", "PIN10", "PIN20", "VCC"]
        );
    }

    #[test]
    fn test_alphanumeric_sort_pure_numbers() {
        let mut items = vec!["100", "2", "1", "20"];
        items.sort_by(|a, b| alphanumeric_sort(a, b));
        assert_eq!(items, vec!["1", "2", "20", "100"]);
    }

    #[test]
    fn test_categorize_component() {
        use crate::v2::ops::categorization::categorize_component;
        assert_eq!(
            categorize_component("STM32F4_MCU", "ARM Microcontroller"),
            "Microcontroller"
        );
        assert_eq!(
            categorize_component("LPC1768", "Cortex-M3 MCU"),
            "Microcontroller"
        );
        assert_eq!(categorize_component("Resistor_0603", ""), "Resistor");
        assert_eq!(categorize_component("Capacitor_100nF", ""), "Capacitor");
        assert_eq!(
            categorize_component("HEADER_2x5", "2x5 Pin Header"),
            "Connector"
        );
        assert_eq!(categorize_component("LED_0603", "SMD LED"), "LED");
    }

    #[test]
    fn test_electrical_type_name() {
        assert_eq!(electrical_type_name(PinElectricalType::Input), "Input");
        assert_eq!(electrical_type_name(PinElectricalType::Output), "Output");
        assert_eq!(electrical_type_name(PinElectricalType::IO), "Bidirectional");
        assert_eq!(electrical_type_name(PinElectricalType::Passive), "Passive");
        assert_eq!(electrical_type_name(PinElectricalType::Power), "Power");
    }

    #[test]
    fn test_parse_electrical_type() {
        assert!(matches!(
            parse_electrical_type("input"),
            PinElectricalType::Input
        ));
        assert!(matches!(
            parse_electrical_type("Output"),
            PinElectricalType::Output
        ));
        assert!(matches!(
            parse_electrical_type("IO"),
            PinElectricalType::IO
        ));
        assert!(matches!(
            parse_electrical_type("bidirectional"),
            PinElectricalType::IO
        ));
        assert!(matches!(
            parse_electrical_type("passive"),
            PinElectricalType::Passive
        ));
        assert!(matches!(
            parse_electrical_type("power"),
            PinElectricalType::Power
        ));
        assert!(matches!(
            parse_electrical_type("unknown"),
            PinElectricalType::Passive
        )); // default
    }

    #[test]
    fn test_record_type_name() {
        assert_eq!(record_type_name(1), "Component");
        assert_eq!(record_type_name(2), "Pin");
        assert_eq!(record_type_name(14), "Rectangle");
        assert_eq!(record_type_name(12), "Arc");
        assert_eq!(record_type_name(41), "Parameter");
        assert_eq!(record_type_name(200), "Unknown");
    }
}
