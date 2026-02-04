// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Schematic library operations.
//!
//! Provides high-level operations for exploring and manipulating Altium schematic
//! library (.SchLib) files. Mirrors the pcblib module pattern to maintain consistency
//! across the codebase.
//!
//! **V2 Migration**: This module uses the v2 SchLib types which provide typed record access.

// cmd_* functions mix presentation and business logic; separation punted until usage patterns clarify abstraction boundaries (premature abstraction risk)

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Cursor, Write};
use std::path::Path;

use crate::v2::fields::{PinData, TypedRecord};
use crate::v2::io::schlib::{SchLibComponent, SchLibComponentEntry, SchLibV2};
use crate::v2::types::PinElectrical;

use crate::ops::categorization::categorize_component;
use crate::ops::output::*;

// ═══════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Sorts strings with embedded numbers naturally (e.g., "A2" < "A10").
///
/// TODO: Consolidate with pcblib::alphanumeric_sort after all 4 ops modules exist
/// to validate the pattern before abstracting.
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
                    // Extract and compare numbers
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
/// Returns the parsed SchLibV2 structure or an error if the file cannot be read.
fn open_schlib(path: &Path) -> Result<SchLibV2, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    Ok(SchLibV2::open(BufReader::new(file)).map_err(|e| e.to_string())?)
}

/// Get the electrical type name for display.
fn electrical_type_name(electrical: PinElectrical) -> &'static str {
    match electrical {
        PinElectrical::Input => "Input",
        PinElectrical::Output => "Output",
        PinElectrical::IO => "Bidirectional",
        PinElectrical::Passive => "Passive",
        PinElectrical::Power => "Power",
        PinElectrical::HiZ => "Hi-Z",
        PinElectrical::OpenCollector => "Open Collector",
        PinElectrical::OpenEmitter => "Open Emitter",
    }
}

/// Parse electrical type from string.
fn parse_electrical_type(s: &str) -> PinElectrical {
    match s.to_lowercase().as_str() {
        "input" | "in" => PinElectrical::Input,
        "output" | "out" => PinElectrical::Output,
        "io" | "bidirectional" | "bidir" => PinElectrical::IO,
        "passive" | "pass" => PinElectrical::Passive,
        "power" | "pwr" => PinElectrical::Power,
        "hiz" | "hi-z" | "tristate" => PinElectrical::HiZ,
        "opencollector" | "open_collector" | "oc" => PinElectrical::OpenCollector,
        "openemitter" | "open_emitter" | "oe" => PinElectrical::OpenEmitter,
        _ => PinElectrical::Passive, // default
    }
}

/// Count primitives in a component's typed records.
fn count_primitives(comp: &SchLibComponent) -> HashMap<&'static str, usize> {
    let mut counts: HashMap<&'static str, usize> = HashMap::new();

    for record in &comp.typed_records {
        let name = match record {
            TypedRecord::Pin(_) => "Pin",
            TypedRecord::Rectangle(_) => "Rectangle",
            TypedRecord::Line(_) => "Line",
            TypedRecord::Arc(_) => "Arc",
            TypedRecord::Ellipse(_) => "Ellipse",
            TypedRecord::Polygon(_) => "Polygon",
            TypedRecord::Polyline(_) => "Polyline",
            TypedRecord::Bezier(_) => "Bezier",
            TypedRecord::EllipticalArc(_) => "EllipticalArc",
            TypedRecord::Pie(_) => "Pie",
            TypedRecord::RoundRectangle(_) => "RoundRectangle",
            TypedRecord::Label(_) => "Label",
            TypedRecord::Image(_) => "Image",
            TypedRecord::Designator(_) => "Designator",
            TypedRecord::Symbol(_) => "Symbol",
            TypedRecord::Parameter(_) => "Parameter",
            TypedRecord::Component(_) => "Component",
            TypedRecord::Implementation(_) => "Implementation",
            TypedRecord::ImplementationList(_) => "ImplementationList",
            TypedRecord::PowerObject(_) => "PowerObject",
            TypedRecord::Port(_) => "Port",
            TypedRecord::NoERC(_) => "NoERC",
            TypedRecord::NetLabel(_) => "NetLabel",
            TypedRecord::Bus(_) => "Bus",
            TypedRecord::Wire(_) => "Wire",
            TypedRecord::TextFrame(_) => "TextFrame",
            TypedRecord::Junction(_) => "Junction",
            TypedRecord::Sheet(_) => "Sheet",
            TypedRecord::BusEntry(_) => "BusEntry",
            TypedRecord::SheetSymbol(_) => "SheetSymbol",
            TypedRecord::SheetEntry(_) => "SheetEntry",
            TypedRecord::Note(_) => "Note",
            TypedRecord::Blanket(_) => "Blanket",
            TypedRecord::SheetName(_) => "SheetName",
            TypedRecord::SheetFileName(_) => "SheetFileName",
            TypedRecord::Unknown(_) => "Unknown",
        };
        *counts.entry(name).or_insert(0) += 1;
    }

    counts
}

/// Get total primitive count for a component.
fn total_primitive_count(comp: &SchLibComponent) -> usize {
    comp.typed_records.len()
}

// ═══════════════════════════════════════════════════════════════════════════
// BROWSE COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// Returns library overview with statistics and component category breakdown.
/// Categorizes each component and aggregates counts for quick library assessment.
pub fn cmd_overview(path: &Path) -> Result<SchLibOverview, Box<dyn std::error::Error>> {
    let lib = open_schlib(path)?;

    // ─────────────────────────────────────────────────────────────────────────
    // 1. COMPONENTS BY CATEGORY
    // ─────────────────────────────────────────────────────────────────────────
    let mut categories: HashMap<&'static str, Vec<ComponentSummary>> = HashMap::new();

    for comp in &lib.components {
        let category = categorize_component(&comp.entry.lib_ref, &comp.entry.description);
        let pin_count = comp.pins().count();
        categories
            .entry(category)
            .or_default()
            .push(ComponentSummary {
                name: comp.entry.lib_ref.clone(),
                description: comp.entry.description.clone(),
                pin_count,
                part_count: comp.entry.part_count as i32,
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

    for comp in &lib.components {
        for pin in comp.pins() {
            total_pins += 1;
            let type_name = electrical_type_name(pin.electrical);
            *pin_types.entry(type_name).or_insert(0) += 1;
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
        .components
        .iter()
        .filter(|c| c.entry.part_count > 1)
        .map(|c| ComponentSummary {
            name: c.entry.lib_ref.clone(),
            description: c.entry.description.clone(),
            pin_count: c.pins().count(),
            part_count: c.entry.part_count as i32,
        })
        .collect();
    multi_part_components.sort_by(|a, b| b.part_count.cmp(&a.part_count));

    // ─────────────────────────────────────────────────────────────────────────
    // 4. LARGEST COMPONENTS (by pin count)
    // ─────────────────────────────────────────────────────────────────────────
    let mut by_pins: Vec<_> = lib.components.iter().collect();
    by_pins.sort_by_key(|c| std::cmp::Reverse(c.pins().count()));

    let largest_components = by_pins
        .iter()
        .take(10)
        .map(|c| ComponentSummary {
            name: c.entry.lib_ref.clone(),
            description: c.entry.description.clone(),
            pin_count: c.pins().count(),
            part_count: c.entry.part_count as i32,
        })
        .collect();

    Ok(SchLibOverview {
        path: path.display().to_string(),
        total_components: lib.components.len(),
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
/// Uses alphanumeric_sort to handle embedded numbers naturally.
pub fn cmd_list(path: &Path) -> Result<SchLibComponentList, Box<dyn std::error::Error>> {
    let lib = open_schlib(path)?;

    let mut components: Vec<ComponentSummary> = lib
        .components
        .iter()
        .map(|c| ComponentSummary {
            name: c.entry.lib_ref.clone(),
            description: c.entry.description.clone(),
            pin_count: c.pins().count(),
            part_count: c.entry.part_count as i32,
        })
        .collect();

    components.sort_by(|a, b| alphanumeric_sort(&a.name, &b.name));

    Ok(SchLibComponentList {
        path: path.display().to_string(),
        total_components: lib.components.len(),
        components,
    })
}

/// Searches for components matching the query in name or description.
/// Returns results up to the optional limit, sorted by relevance.
pub fn cmd_search(
    path: &Path,
    query: &str,
    limit: Option<usize>,
) -> Result<SchLibSearchResults, Box<dyn std::error::Error>> {
    let lib = open_schlib(path)?;

    let query_lower = query.to_lowercase();
    let has_wildcard = query.contains('*');

    let mut matches: Vec<ComponentSummary> = lib
        .components
        .iter()
        .filter(|c| {
            let name = c.entry.lib_ref.to_lowercase();
            let desc = c.entry.description.to_lowercase();

            if has_wildcard {
                let pattern = query_lower.replace('*', "");
                name.contains(&pattern) || desc.contains(&pattern)
            } else {
                name.contains(&query_lower) || desc.contains(&query_lower)
            }
        })
        .map(|c| ComponentSummary {
            name: c.entry.lib_ref.clone(),
            description: c.entry.description.clone(),
            pin_count: c.pins().count(),
            part_count: c.entry.part_count as i32,
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

    // Apply limit
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

    // Count primitive types across all components
    let mut primitive_counts: HashMap<&'static str, usize> = HashMap::new();
    let mut total_primitives = 0;
    let mut multi_part_count = 0;

    for comp in &lib.components {
        let counts = count_primitives(comp);
        for (name, count) in counts {
            *primitive_counts.entry(name).or_insert(0) += count;
            total_primitives += count;
        }
        if comp.entry.part_count > 1 {
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
        component_count: lib.components.len(),
        total_primitives,
        primitive_types,
        multi_part_count,
    })
}

/// Returns detailed information about a single component.
/// When show_primitives is true, includes all graphical elements (lines, rectangles, arcs).
pub fn cmd_component(
    path: &Path,
    name: &str,
    show_primitives: bool,
) -> Result<SchLibComponentDetail, Box<dyn std::error::Error>> {
    let lib = open_schlib(path)?;

    let name_lower = name.to_lowercase();
    let comp = lib
        .components
        .iter()
        .find(|c| c.entry.lib_ref.to_lowercase() == name_lower)
        .ok_or_else(|| format!("Component '{}' not found", name))?;

    // Get component data if available
    let display_mode_count = comp
        .component_data()
        .map(|cd| cd.display_mode_count as i32)
        .unwrap_or(1);

    // Collect pin details
    let mut pins: Vec<PinDetail> = comp
        .pins()
        .map(|p| PinDetail {
            designator: p.designator.clone(),
            name: p.name.clone(),
            electrical_type: electrical_type_name(p.electrical).to_string(),
            description: p.description.clone(),
        })
        .collect();

    pins.sort_by(|a, b| alphanumeric_sort(&a.designator, &b.designator));

    // Primitive counts (optional)
    let primitive_counts = if show_primitives {
        let counts = count_primitives(comp);
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
        name: comp.entry.lib_ref.clone(),
        description: comp.entry.description.clone(),
        part_count: comp.entry.part_count as i32,
        display_mode_count,
        pin_count: pins.len(),
        total_primitives: total_primitive_count(comp),
        pins,
        primitive_counts,
    })
}

/// Lists pins for a specific component or all components if component is None.
/// Returns pin designators, names, and electrical types.
pub fn cmd_pins(
    path: &Path,
    component: Option<String>,
) -> Result<SchLibPinList, Box<dyn std::error::Error>> {
    let lib = open_schlib(path)?;

    let filter_lower = component.as_ref().map(|s| s.to_lowercase());

    let mut all_pins: Vec<PinWithComponent> = Vec::new();

    for comp in &lib.components {
        if let Some(ref filter) = filter_lower {
            if comp.entry.lib_ref.to_lowercase() != *filter {
                continue;
            }
        }

        for pin in comp.pins() {
            all_pins.push(PinWithComponent {
                component_name: comp.entry.lib_ref.clone(),
                designator: pin.designator.clone(),
                name: pin.name.clone(),
                electrical_type: electrical_type_name(pin.electrical).to_string(),
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
    // Add any remaining types
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

/// Lists graphical primitives (lines, rectangles, arcs, polygons) for a component.
/// Useful for analyzing component symbol complexity.
pub fn cmd_primitives(
    path: &Path,
    component: &str,
) -> Result<SchLibPrimitiveList, Box<dyn std::error::Error>> {
    let lib = open_schlib(path)?;

    let name_lower = component.to_lowercase();
    let comp = lib
        .components
        .iter()
        .find(|c| c.entry.lib_ref.to_lowercase() == name_lower)
        .ok_or_else(|| format!("Component '{}' not found", component))?;

    let mut primitives: Vec<PrimitiveInfo> = Vec::new();

    // Convert schematic coordinates to mils for display
    fn coord_to_mils(value: i32) -> String {
        // SchLib coordinates are stored as value * 100000
        let mils = value as f64 / 100000.0;
        format!("{:.1}", mils)
    }

    for record in &comp.typed_records {
        let info = match record {
            TypedRecord::Pin(p) => PrimitiveInfo::Pin {
                designator: p.designator.clone(),
                name: p.name.clone(),
                electrical_type: electrical_type_name(p.electrical).to_string(),
                x: coord_to_mils(p.location_x),
                y: coord_to_mils(p.location_y),
            },
            TypedRecord::Rectangle(r) => PrimitiveInfo::Rectangle {
                x1: coord_to_mils(r.location_x),
                y1: coord_to_mils(r.location_y),
                x2: coord_to_mils(r.corner_x),
                y2: coord_to_mils(r.corner_y),
            },
            TypedRecord::Line(l) => PrimitiveInfo::Line {
                x1: coord_to_mils(l.location_x),
                y1: coord_to_mils(l.location_y),
                x2: coord_to_mils(l.corner_x),
                y2: coord_to_mils(l.corner_y),
            },
            TypedRecord::Arc(a) => PrimitiveInfo::Arc {
                center_x: coord_to_mils(a.location_x),
                center_y: coord_to_mils(a.location_y),
                radius: coord_to_mils(a.radius),
                start_angle: a.start_angle,
                end_angle: a.end_angle,
            },
            TypedRecord::Polygon(p) => PrimitiveInfo::Polygon {
                vertex_count: p.vertices.len(),
            },
            TypedRecord::Polyline(p) => PrimitiveInfo::Polyline {
                vertex_count: p.vertices.len(),
            },
            TypedRecord::Label(l) => PrimitiveInfo::Label {
                text: l.text.clone(),
                x: coord_to_mils(l.location_x),
                y: coord_to_mils(l.location_y),
            },
            TypedRecord::Unknown(id) => PrimitiveInfo::Other {
                primitive_type: format!("Unknown({})", id),
            },
            // Skip Component, Parameter, Implementation records for primitive listing
            TypedRecord::Component(_)
            | TypedRecord::Parameter(_)
            | TypedRecord::Implementation(_)
            | TypedRecord::ImplementationList(_) => continue,
            // Other graphical primitives
            _ => PrimitiveInfo::Other {
                primitive_type: format!("{:?}", std::mem::discriminant(record)),
            },
        };
        primitives.push(info);
    }

    Ok(SchLibPrimitiveList {
        component_name: comp.entry.lib_ref.clone(),
        total_primitives: primitives.len(),
        primitives,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// MANIPULATION COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// Embedded blank SchLib template.
const BLANK_SCHLIB_TEMPLATE: &[u8] = include_bytes!("../../data/blank/Schlib1.SchLib");

/// Creates an empty SchLib file at the given path.
/// The new library contains default header values and no components.
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
/// The component starts with no pins or primitives.
pub fn cmd_add_component(
    path: &Path,
    name: &str,
    description: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Read existing library
    let mut lib = open_schlib(path)?;

    // Check if component already exists
    let name_lower = name.to_lowercase();
    if lib
        .components
        .iter()
        .any(|c| c.entry.lib_ref.to_lowercase() == name_lower)
    {
        return Err(format!("Component '{}' already exists in library", name).into());
    }

    // Add new component
    lib.components.push(SchLibComponent {
        entry: SchLibComponentEntry {
            lib_ref: name.to_string(),
            description: description.unwrap_or_default(),
            part_count: 1,
            aliases: Vec::new(),
        },
        records: Vec::new(),
        typed_records: Vec::new(),
    });

    // Clear raw header to force rebuild
    lib.header.raw = None;

    // Write to a buffer first, then to the actual file
    let mut buffer = Cursor::new(Vec::new());
    lib.write(&mut buffer).map_err(|e| e.to_string())?;
    let mut output = File::create(path)?;
    output.write_all(buffer.get_ref())?;

    println!("Added component '{}' to {}", name, path.display());
    Ok(())
}

/// Adds a pin to an existing component in the library.
/// electrical_type accepts: input, output, bidirectional, passive, power, etc.
pub fn cmd_add_pin(
    path: &Path,
    component: &str,
    designator: &str,
    name: &str,
    electrical_type: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Read existing library
    let mut lib = open_schlib(path)?;

    // Find component
    let comp_lower = component.to_lowercase();
    let comp = lib
        .components
        .iter_mut()
        .find(|c| c.entry.lib_ref.to_lowercase() == comp_lower)
        .ok_or_else(|| format!("Component '{}' not found", component))?;

    // Parse electrical type
    let electrical = parse_electrical_type(electrical_type);

    // Create pin record (ASCII format)
    let pin_params = format!(
        "|RECORD=2|OwnerIndex=0|OwnerPartId=1|Name={}|Designator={}|PinConglomerate=25|",
        name, designator
    );

    // Add to raw records
    comp.records.push(crate::v2::io::schlib::SchLibRecord {
        record_id: 2,
        record_id_ex: None,
        params: pin_params.clone(),
        raw: pin_params.into_bytes(),
    });

    // Add to typed records
    let mut pin = PinData::default();
    pin.name = name.to_string();
    pin.designator = designator.to_string();
    pin.electrical = electrical;
    comp.typed_records.push(TypedRecord::Pin(pin));

    // Clear raw header to force rebuild
    lib.header.raw = None;

    // Write back
    let mut buffer = Cursor::new(Vec::new());
    lib.write(&mut buffer).map_err(|e| e.to_string())?;
    let mut output = File::create(path)?;
    output.write_all(buffer.get_ref())?;

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
/// When full is true, includes all primitive details; otherwise uses compact format.
pub fn cmd_json(path: &Path, full: bool) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let lib = open_schlib(path)?;

    if full {
        // Full serialization of the library structure
        Ok(serde_json::to_value(&lib)?)
    } else {
        // Compact format with just component summaries
        let components: Vec<serde_json::Value> = lib
            .components
            .iter()
            .map(|c| {
                serde_json::json!({
                    "name": c.entry.lib_ref,
                    "description": c.entry.description,
                    "pin_count": c.pins().count(),
                    "part_count": c.entry.part_count,
                    "primitive_count": c.typed_records.len(),
                })
            })
            .collect();

        Ok(serde_json::json!({
            "file": path.display().to_string(),
            "unique_id": lib.header.unique_id,
            "component_count": lib.components.len(),
            "components": components,
        }))
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
        // Uses categorize_component from ops/categorization.rs
        // - ARM/LPC/MCU patterns -> Microcontroller
        assert_eq!(
            categorize_component("STM32F4_MCU", "ARM Microcontroller"),
            "Microcontroller"
        );
        assert_eq!(
            categorize_component("LPC1768", "Cortex-M3 MCU"),
            "Microcontroller"
        );
        // - "resistor" keyword -> Resistor
        assert_eq!(
            categorize_component("Resistor_0603", ""),
            "Resistor"
        );
        // - "capacitor" keyword -> Capacitor
        assert_eq!(
            categorize_component("Capacitor_100nF", ""),
            "Capacitor"
        );
        // - "header" keyword -> Connector
        assert_eq!(
            categorize_component("HEADER_2x5", "2x5 Pin Header"),
            "Connector"
        );
        // - "led" keyword -> LED
        assert_eq!(
            categorize_component("LED_0603", "SMD LED"),
            "LED"
        );
    }

    #[test]
    fn test_electrical_type_name() {
        assert_eq!(electrical_type_name(PinElectrical::Input), "Input");
        assert_eq!(electrical_type_name(PinElectrical::Output), "Output");
        assert_eq!(electrical_type_name(PinElectrical::IO), "Bidirectional");
        assert_eq!(electrical_type_name(PinElectrical::Passive), "Passive");
        assert_eq!(electrical_type_name(PinElectrical::Power), "Power");
    }

    #[test]
    fn test_parse_electrical_type() {
        assert!(matches!(parse_electrical_type("input"), PinElectrical::Input));
        assert!(matches!(parse_electrical_type("Output"), PinElectrical::Output));
        assert!(matches!(parse_electrical_type("IO"), PinElectrical::IO));
        assert!(matches!(parse_electrical_type("bidirectional"), PinElectrical::IO));
        assert!(matches!(parse_electrical_type("passive"), PinElectrical::Passive));
        assert!(matches!(parse_electrical_type("power"), PinElectrical::Power));
        assert!(matches!(parse_electrical_type("unknown"), PinElectrical::Passive)); // default
    }
}
