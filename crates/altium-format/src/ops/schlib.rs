// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Schematic library operations.
//!
//! High-level operations for exploring and manipulating Altium schematic library (.SchLib) files.

// cmd_* functions mix presentation and business logic; separation punted until usage patterns clarify abstraction boundaries (premature abstraction risk)

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json;

use super::util::alphanumeric_sort;
use crate::dump::fmt_coord;
use crate::io::{SchLib, SchLibComponent};
use crate::ops::categorization::categorize_component;
use crate::ops::output::*;
use crate::records::sch::{
    LineWidth, PinConglomerateFlags, PinElectricalType, PinSymbol, SchArc, SchBezier,
    SchComponent, SchDesignator, SchEllipse, SchEllipticalArc, SchGraphicalBase,
    SchImplementation, SchImplementationList, SchImplementationParameters, SchLabel, SchLine,
    SchMapDefiner, SchMapDefinerList, SchParameter, SchPin, SchPolygon, SchPolyline,
    SchPrimitiveBase, SchRecord, SchRectangle, TextJustification, TextOrientations,
};
use crate::types::Unit;

fn open_schlib(path: &Path) -> Result<SchLib, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    Ok(SchLib::open(BufReader::new(file))?)
}

/// Get electrical type name.
fn electrical_type_name(et: &PinElectricalType) -> &'static str {
    match et {
        PinElectricalType::Input => "Input",
        PinElectricalType::InputOutput => "I/O",
        PinElectricalType::Output => "Output",
        PinElectricalType::OpenCollector => "Open Collector",
        PinElectricalType::Passive => "Passive",
        PinElectricalType::HiZ => "Hi-Z",
        PinElectricalType::OpenEmitter => "Open Emitter",
        PinElectricalType::Power => "Power",
    }
}

/// Get record type name.
fn record_type_name(record: &SchRecord) -> &'static str {
    match record {
        SchRecord::Component(_) => "Component",
        SchRecord::Pin(_) => "Pin",
        SchRecord::Symbol(_) => "Symbol",
        SchRecord::Label(_) => "Label",
        SchRecord::Bezier(_) => "Bezier",
        SchRecord::Polyline(_) => "Polyline",
        SchRecord::Polygon(_) => "Polygon",
        SchRecord::Ellipse(_) => "Ellipse",
        SchRecord::Pie(_) => "Pie",
        SchRecord::EllipticalArc(_) => "EllipticalArc",
        SchRecord::Arc(_) => "Arc",
        SchRecord::Line(_) => "Line",
        SchRecord::Rectangle(_) => "Rectangle",
        SchRecord::PowerObject(_) => "PowerObject",
        SchRecord::Port(_) => "Port",
        SchRecord::NoErc(_) => "NoERC",
        SchRecord::NetLabel(_) => "NetLabel",
        SchRecord::Bus(_) => "Bus",
        SchRecord::Wire(_) => "Wire",
        SchRecord::TextFrame(_) => "TextFrame",
        SchRecord::TextFrameVariant(_) => "TextFrameVariant",
        SchRecord::Junction(_) => "Junction",
        SchRecord::Image(_) => "Image",
        SchRecord::SheetHeader(_) => "SheetHeader",
        SchRecord::Designator(_) => "Designator",
        SchRecord::BusEntry(_) => "BusEntry",
        SchRecord::Parameter(_) => "Parameter",
        SchRecord::WarningSign(_) => "WarningSign",
        SchRecord::ImplementationList(_) => "ImplementationList",
        SchRecord::Implementation(_) => "Implementation",
        SchRecord::MapDefinerList(_) => "MapDefinerList",
        SchRecord::MapDefiner(_) => "MapDefiner",
        SchRecord::ImplementationParameters(_) => "ImplementationParameters",
        SchRecord::Unknown { .. } => "Unknown",
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// HIGH-LEVEL COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// Complete library overview.
pub fn cmd_overview(path: &Path, full: bool) -> Result<SchLibOverview, Box<dyn std::error::Error>> {
    let lib = open_schlib(path)?;

    // ─────────────────────────────────────────────────────────────────────────
    // 1. COMPONENTS BY CATEGORY
    // ─────────────────────────────────────────────────────────────────────────
    let mut categories: HashMap<&'static str, Vec<ComponentSummary>> = HashMap::new();

    for comp in lib.iter() {
        let category = categorize_component(
            &comp.component.lib_reference,
            &comp.component.component_description,
        );
        categories
            .entry(category)
            .or_default()
            .push(ComponentSummary {
                name: comp.component.lib_reference.clone(),
                description: comp.component.component_description.clone(),
                pin_count: comp.pin_count(),
                part_count: comp.component.part_count,
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
        "Other IC",
        "Transistor",
        "Diode/Protection",
        "LED",
        "Capacitor",
        "Resistor",
        "Inductor/Ferrite",
        "Connector",
        "Test Point",
    ];

    let mut components_by_category = Vec::new();
    for category in category_order.iter() {
        if let Some(comps) = categories.remove(*category) {
            components_by_category.push((category.to_string(), comps));
        }
    }

    // Add any uncategorized
    for (category, comps) in categories {
        if !comps.is_empty() {
            components_by_category.push((category.to_string(), comps));
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 2. PIN STATISTICS
    // ─────────────────────────────────────────────────────────────────────────
    let mut total_pins = 0;
    let mut pin_types: HashMap<String, usize> = HashMap::new();

    for comp in lib.iter() {
        for prim in &comp.primitives {
            if let SchRecord::Pin(pin) = prim {
                total_pins += 1;
                *pin_types
                    .entry(electrical_type_name(&pin.electrical).to_string())
                    .or_insert(0) += 1;
            }
        }
    }

    let mut sorted_types: Vec<_> = pin_types.into_iter().collect();
    sorted_types.sort_by(|a, b| b.1.cmp(&a.1));

    let pin_statistics = PinStatistics {
        total_pins,
        pin_types: sorted_types,
    };

    // ─────────────────────────────────────────────────────────────────────────
    // 3. MULTI-PART COMPONENTS
    // ─────────────────────────────────────────────────────────────────────────
    let multi_part_components: Vec<ComponentSummary> = lib
        .iter()
        .filter(|c| c.component.part_count > 1)
        .map(|comp| ComponentSummary {
            name: comp.component.lib_reference.clone(),
            description: comp.component.component_description.clone(),
            pin_count: comp.pin_count(),
            part_count: comp.component.part_count,
        })
        .collect();

    // ─────────────────────────────────────────────────────────────────────────
    // 4. LARGEST COMPONENTS
    // ─────────────────────────────────────────────────────────────────────────
    let mut largest_components: Vec<ComponentSummary> = lib
        .iter()
        .map(|comp| ComponentSummary {
            name: comp.component.lib_reference.clone(),
            description: comp.component.component_description.clone(),
            pin_count: comp.pin_count(),
            part_count: comp.component.part_count,
        })
        .collect();
    largest_components.sort_by(|a, b| b.pin_count.cmp(&a.pin_count));
    largest_components.truncate(10);

    // ─────────────────────────────────────────────────────────────────────────
    // 5. FULL COMPONENT DETAILS (if requested)
    // ─────────────────────────────────────────────────────────────────────────
    let component_details = if full {
        Some(
            lib.iter()
                .map(|comp| {
                    let pins = comp
                        .primitives
                        .iter()
                        .filter_map(|prim| {
                            if let SchRecord::Pin(pin) = prim {
                                Some(PinDetail {
                                    designator: pin.designator.clone(),
                                    name: pin.name.clone(),
                                    electrical_type: electrical_type_name(&pin.electrical)
                                        .to_string(),
                                    description: pin.description.clone(),
                                })
                            } else {
                                None
                            }
                        })
                        .collect();

                    SchLibComponentDetail {
                        name: comp.component.lib_reference.clone(),
                        description: comp.component.component_description.clone(),
                        part_count: comp.component.part_count,
                        display_mode_count: comp.component.display_mode_count,
                        pin_count: comp.pin_count(),
                        total_primitives: comp.primitives.len(),
                        pins,
                        primitive_counts: None,
                    }
                })
                .collect(),
        )
    } else {
        None
    };

    Ok(SchLibOverview {
        path: path.display().to_string(),
        total_components: lib.components.len(),
        components_by_category,
        pin_statistics,
        multi_part_components,
        largest_components,
        component_details,
    })
}

/// List all components.
pub fn cmd_list(path: &Path) -> Result<SchLibComponentList, Box<dyn std::error::Error>> {
    let lib = open_schlib(path)?;

    let components: Vec<ComponentSummary> = lib
        .iter()
        .map(|comp| ComponentSummary {
            name: comp.component.lib_reference.clone(),
            description: comp.component.component_description.clone(),
            pin_count: comp.pin_count(),
            part_count: comp.component.part_count,
        })
        .collect();

    Ok(SchLibComponentList {
        path: path.display().to_string(),
        total_components: lib.components.len(),
        components,
    })
}

/// Search for components.
pub fn cmd_search(
    path: &Path,
    query: &str,
    limit: Option<usize>,
) -> Result<SchLibSearchResults, Box<dyn std::error::Error>> {
    let lib = open_schlib(path)?;

    let query_lower = query.to_lowercase();
    let has_wildcard = query.contains('*');

    let matches: Vec<ComponentSummary> = lib
        .iter()
        .filter(|comp| {
            let name = comp.component.lib_reference.to_lowercase();
            let desc = comp.component.component_description.to_lowercase();

            if has_wildcard {
                let pattern = query_lower.replace('*', "");
                name.contains(&pattern) || desc.contains(&pattern)
            } else {
                name.contains(&query_lower) || desc.contains(&query_lower)
            }
        })
        .map(|comp| ComponentSummary {
            name: comp.component.lib_reference.clone(),
            description: comp.component.component_description.clone(),
            pin_count: comp.pin_count(),
            part_count: comp.component.part_count,
        })
        .collect();

    let total_matches = matches.len();
    let results = if let Some(limit) = limit {
        matches.into_iter().take(limit).collect()
    } else {
        matches
    };

    Ok(SchLibSearchResults {
        query: query.to_string(),
        total_matches,
        results,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// DETAILED COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// Library info and statistics.
pub fn cmd_info(path: &Path) -> Result<SchLibInfo, Box<dyn std::error::Error>> {
    let lib = open_schlib(path)?;

    // Count primitive types across all components
    let mut primitive_counts: HashMap<String, usize> = HashMap::new();
    let mut total_primitives = 0;

    for comp in lib.iter() {
        for prim in &comp.primitives {
            let name = record_type_name(prim).to_string();
            *primitive_counts.entry(name).or_insert(0) += 1;
            total_primitives += 1;
        }
    }

    let mut sorted: Vec<_> = primitive_counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));

    // Multi-part component count
    let multi_part_count = lib.iter().filter(|c| c.component.part_count > 1).count();

    Ok(SchLibInfo {
        path: path.display().to_string(),
        component_count: lib.components.len(),
        total_primitives,
        primitive_types: sorted,
        multi_part_count,
    })
}

/// Component details.
pub fn cmd_component(
    path: &Path,
    name: &str,
    show_primitives: bool,
) -> Result<SchLibComponentDetail, Box<dyn std::error::Error>> {
    let lib = open_schlib(path)?;

    let name_lower = name.to_lowercase();
    let comp = lib
        .iter()
        .find(|c| c.component.lib_reference.to_lowercase() == name_lower)
        .ok_or_else(|| format!("Component '{}' not found", name))?;

    // List pins
    let mut pins: Vec<&SchPin> = comp
        .primitives
        .iter()
        .filter_map(|p| {
            if let SchRecord::Pin(pin) = p {
                Some(pin)
            } else {
                None
            }
        })
        .collect();

    pins.sort_by(|a, b| alphanumeric_sort(&a.designator, &b.designator));

    let pins_detail: Vec<PinDetail> = pins
        .iter()
        .map(|pin| PinDetail {
            designator: pin.designator.clone(),
            name: pin.name.clone(),
            electrical_type: electrical_type_name(&pin.electrical).to_string(),
            description: pin.description.clone(),
        })
        .collect();

    let primitive_counts = if show_primitives {
        let mut prim_counts: HashMap<String, usize> = HashMap::new();
        for prim in &comp.primitives {
            *prim_counts
                .entry(record_type_name(prim).to_string())
                .or_insert(0) += 1;
        }
        Some(prim_counts.into_iter().collect())
    } else {
        None
    };

    Ok(SchLibComponentDetail {
        name: comp.component.lib_reference.clone(),
        description: comp.component.component_description.clone(),
        part_count: comp.component.part_count,
        display_mode_count: comp.component.display_mode_count,
        pin_count: comp.pin_count(),
        total_primitives: comp.primitive_count(),
        pins: pins_detail,
        primitive_counts,
    })
}

/// List pins.
pub fn cmd_pins(
    path: &Path,
    component_filter: Option<String>,
    by_type: bool,
) -> Result<SchLibPinList, Box<dyn std::error::Error>> {
    let lib = open_schlib(path)?;

    let filter_lower = component_filter.as_ref().map(|s| s.to_lowercase());

    let mut all_pins: Vec<PinWithComponent> = Vec::new();

    for comp in lib.iter() {
        if let Some(ref filter) = filter_lower {
            if !comp.component.lib_reference.to_lowercase().contains(filter) {
                continue;
            }
        }

        for prim in &comp.primitives {
            if let SchRecord::Pin(pin) = prim {
                all_pins.push(PinWithComponent {
                    component_name: comp.component.lib_reference.clone(),
                    designator: pin.designator.clone(),
                    name: pin.name.clone(),
                    electrical_type: electrical_type_name(&pin.electrical).to_string(),
                });
            }
        }
    }

    let pins_by_type = if by_type {
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
            "I/O",
            "Passive",
            "Power",
            "Open Collector",
            "Open Emitter",
            "Hi-Z",
        ];
        let mut ordered: Vec<(String, Vec<PinWithComponent>)> = Vec::new();

        for etype in type_order {
            if let Some(pins) = by_type.remove(etype) {
                ordered.push((etype.to_string(), pins));
            }
        }

        // Add any remaining types not in the order
        for (etype, pins) in by_type {
            ordered.push((etype, pins));
        }

        Some(ordered)
    } else {
        None
    };

    Ok(SchLibPinList {
        path: path.display().to_string(),
        total_pins: all_pins.len(),
        pins: all_pins,
        pins_by_type,
    })
}

/// Show primitives for a component.
pub fn cmd_primitives(
    path: &Path,
    name: &str,
) -> Result<SchLibPrimitiveList, Box<dyn std::error::Error>> {
    let lib = open_schlib(path)?;

    let name_lower = name.to_lowercase();
    let comp = lib
        .iter()
        .find(|c| c.component.lib_reference.to_lowercase() == name_lower)
        .ok_or_else(|| format!("Component '{}' not found", name))?;

    // Skip the first primitive (component record itself)
    let primitives: Vec<PrimitiveInfo> = comp
        .primitives
        .iter()
        .skip(1)
        .map(|prim| match prim {
            SchRecord::Pin(p) => PrimitiveInfo::Pin {
                designator: p.designator.clone(),
                name: p.name.clone(),
                electrical_type: electrical_type_name(&p.electrical).to_string(),
                x: fmt_coord(p.graphical.location_x),
                y: fmt_coord(p.graphical.location_y),
            },
            SchRecord::Rectangle(r) => PrimitiveInfo::Rectangle {
                x1: fmt_coord(r.graphical.location_x),
                y1: fmt_coord(r.graphical.location_y),
                x2: fmt_coord(r.corner_x),
                y2: fmt_coord(r.corner_y),
            },
            SchRecord::Line(l) => PrimitiveInfo::Line {
                x1: fmt_coord(l.graphical.location_x),
                y1: fmt_coord(l.graphical.location_y),
                x2: fmt_coord(l.corner_x),
                y2: fmt_coord(l.corner_y),
            },
            SchRecord::Arc(a) => PrimitiveInfo::Arc {
                center_x: fmt_coord(a.graphical.location_x),
                center_y: fmt_coord(a.graphical.location_y),
                radius: fmt_coord(a.radius),
                start_angle: a.start_angle,
                end_angle: a.end_angle,
            },
            SchRecord::Polygon(p) => PrimitiveInfo::Polygon {
                vertex_count: p.vertices.len(),
            },
            SchRecord::Polyline(p) => PrimitiveInfo::Polyline {
                vertex_count: p.vertices.len(),
            },
            SchRecord::Label(l) => PrimitiveInfo::Label {
                text: l.text.clone(),
                x: fmt_coord(l.graphical.location_x),
                y: fmt_coord(l.graphical.location_y),
            },
            _ => PrimitiveInfo::Other {
                primitive_type: record_type_name(prim).to_string(),
            },
        })
        .collect();

    Ok(SchLibPrimitiveList {
        component_name: comp.component.lib_reference.clone(),
        total_primitives: comp.primitive_count(),
        primitives,
    })
}

/// Export as JSON - returns full library export with round-trip support.
pub fn cmd_json(path: &Path) -> Result<SchLibExport, Box<dyn std::error::Error>> {
    let lib = open_schlib(path)?;
    let components: Vec<SchComponentJson> = lib.iter().map(component_to_json).collect();
    let header: HashMap<String, String> = lib
        .header_params
        .iter()
        .map(|(k, v)| (k.to_string(), v.as_str().to_string()))
        .collect();
    Ok(SchLibExport {
        source: Some(path.display().to_string()),
        component_count: components.len(),
        header,
        components,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// CREATION COMMAND IMPLEMENTATIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Embedded blank SchLib template.
const BLANK_SCHLIB_TEMPLATE: &[u8] = include_bytes!("../../data/blank/Schlib1.SchLib");

/// Create a new empty SchLib file.
pub fn cmd_create(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    if path.exists() {
        return Err(format!("File already exists: {}", path.display()).into());
    }

    std::fs::write(path, BLANK_SCHLIB_TEMPLATE)?;

    Ok(format!("Created empty SchLib: {}", path.display()))
}

fn load_blank_schlib() -> Result<SchLib, Box<dyn std::error::Error>> {
    Ok(SchLib::open(Cursor::new(BLANK_SCHLIB_TEMPLATE))?)
}

/// Open or create a SchLib file.
fn open_or_create_schlib(path: &Path) -> Result<SchLib, Box<dyn std::error::Error>> {
    if path.exists() {
        open_schlib(path)
    } else {
        let mut lib = load_blank_schlib()?;
        // Strip the template's placeholder component so the blank template is truly empty
        lib.components
            .retain(|c| c.component.lib_reference != "Component_1");
        Ok(lib)
    }
}

/// Save a SchLib file.
fn save_schlib(path: &Path, lib: &SchLib) -> Result<(), Box<dyn std::error::Error>> {
    Ok(lib.save_to_file(path)?)
}

/// Parse a hex color string to Win32 COLORREF (BGR format).
fn parse_color(hex: &str) -> Result<i32, Box<dyn std::error::Error>> {
    // Remove leading # if present
    let hex = hex.trim_start_matches('#');

    if hex.len() != 6 {
        return Err(format!(
            "Invalid color format: {}. Expected 6 hex digits (RRGGBB)",
            hex
        )
        .into());
    }

    let r = u8::from_str_radix(&hex[0..2], 16)
        .map_err(|_| format!("Invalid red component in color: {}", hex))?;
    let g = u8::from_str_radix(&hex[2..4], 16)
        .map_err(|_| format!("Invalid green component in color: {}", hex))?;
    let b = u8::from_str_radix(&hex[4..6], 16)
        .map_err(|_| format!("Invalid blue component in color: {}", hex))?;

    // Win32 COLORREF is 0x00BBGGRR
    Ok((b as i32) << 16 | (g as i32) << 8 | (r as i32))
}

/// Parse electrical type string to PinElectricalType.
fn parse_electrical_type(s: &str) -> Result<PinElectricalType, Box<dyn std::error::Error>> {
    match s.to_lowercase().as_str() {
        "input" | "in" => Ok(PinElectricalType::Input),
        "output" | "out" => Ok(PinElectricalType::Output),
        "io" | "inputoutput" | "bidirectional" | "bidir" => Ok(PinElectricalType::InputOutput),
        "passive" | "pas" => Ok(PinElectricalType::Passive),
        "power" | "pwr" => Ok(PinElectricalType::Power),
        "oc" | "opencollector" => Ok(PinElectricalType::OpenCollector),
        "oe" | "openemitter" => Ok(PinElectricalType::OpenEmitter),
        "hiz" | "tristate" | "3state" => Ok(PinElectricalType::HiZ),
        _ => Err(format!(
            "Unknown electrical type: {}. Use: input, output, io, passive, power, oc, oe, hiz",
            s
        )
        .into()),
    }
}

/// Parse pin orientation to conglomerate flags.
fn parse_pin_orientation(s: &str) -> Result<PinConglomerateFlags, Box<dyn std::error::Error>> {
    match s.to_lowercase().as_str() {
        "right" => Ok(PinConglomerateFlags::empty()), // Pin pointing right (default)
        "left" => Ok(PinConglomerateFlags::FLIPPED),  // Pin pointing left
        "up" => Ok(PinConglomerateFlags::ROTATED),    // Pin pointing up
        "down" => Ok(PinConglomerateFlags::ROTATED | PinConglomerateFlags::FLIPPED), // Pin pointing down
        _ => Err(format!("Unknown orientation: {}. Use: left, right, up, down", s).into()),
    }
}

/// Convert mils to raw coordinate value.
fn mils_to_raw(mils: i32) -> i32 {
    mils * 10000
}

/// Convert mils (f64) to raw coordinate value.
fn mils_f64_to_raw(mils: f64) -> i32 {
    (mils * 10000.0).round() as i32
}

/// Infer component designator prefix from name and description.
fn infer_designator_prefix(name: &str, description: &str) -> String {
    let name_lower = name.to_lowercase();
    let desc_lower = description.to_lowercase();

    // Check name prefix first for single-letter components
    if name_lower.starts_with('c') && !name_lower.starts_with("conn") {
        "C?".to_string()
    } else if name_lower.starts_with('r') {
        "R?".to_string()
    } else if name_lower.starts_with('l') && !desc_lower.contains("led") {
        "L?".to_string()
    } else if name_lower.starts_with('d') {
        "D?".to_string()
    } else if name_lower.starts_with('j') || name_lower.starts_with("conn") {
        "J?".to_string()
    } else if name_lower.starts_with('k') {
        "K?".to_string()
    } else if name_lower.starts_with('q') {
        "Q?".to_string()
    } else if desc_lower.contains("capacitor") {
        "C?".to_string()
    } else if desc_lower.contains("resistor") {
        "R?".to_string()
    } else if desc_lower.contains("led") {
        "LED?".to_string()
    } else if desc_lower.contains("inductor") {
        "L?".to_string()
    } else if desc_lower.contains("diode") {
        "D?".to_string()
    } else if desc_lower.contains("connector")
        || desc_lower.contains("jst")
        || desc_lower.contains("usb")
        || desc_lower.contains("bnc")
        || desc_lower.contains("jack")
        || desc_lower.contains("header")
    {
        "J?".to_string()
    } else if desc_lower.contains("relay") {
        "K?".to_string()
    } else if desc_lower.contains("mosfet") || desc_lower.contains("transistor") {
        "Q?".to_string()
    } else {
        "U?".to_string()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// UNIT PARSING HELPERS
// ═══════════════════════════════════════════════════════════════════════════

/// Parse a value with unit suffix and return mils (e.g., "100mil", "2.54mm", "0.1in").
/// Returns mils as f64 for coordinate values.
#[allow(dead_code)] // Reserved for future unit parsing scenarios
fn parse_unit_value_to_mils(s: &str) -> Result<f64, Box<dyn std::error::Error>> {
    let (coord, _unit) =
        Unit::parse_with_unit(s).map_err(|e| format!("Invalid value '{}': {:?}", s, e))?;
    Ok(coord.to_mils())
}

/// Parse a value with optional unit suffix, defaulting to mils for plain numbers.
/// Handles: "100mil", "2.54mm", "0.1in", "100" (interpreted as mils)
fn parse_unit_value_or_mil(s: &str) -> Result<f64, Box<dyn std::error::Error>> {
    let s = s.trim();

    // Try parsing with unit suffix first
    if let Ok((coord, _unit)) = Unit::parse_with_unit(s) {
        return Ok(coord.to_mils());
    }

    // If no unit suffix, try as plain number (interpreted as mils)
    s.parse::<f64>().map_err(|_| {
        format!(
            "Invalid value '{}': expected number with optional unit (e.g., '100mil', '2.54mm')",
            s
        )
        .into()
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// JSON COORDINATE VALUE TYPE (supports both numbers and strings with units)
// ═══════════════════════════════════════════════════════════════════════════

/// A coordinate value that can be deserialized from either a number (mils) or a string with units.
/// Examples: 100, "100", "100mil", "2.54mm", "0.1in"
#[derive(Debug, Clone)]
pub struct CoordValue(pub f64);

impl CoordValue {
    /// Get the value in mils.
    pub fn to_mils(&self) -> f64 {
        self.0
    }

    /// Convert to raw internal coordinate value.
    pub fn to_raw(&self) -> i32 {
        mils_f64_to_raw(self.0)
    }
}

impl Default for CoordValue {
    fn default() -> Self {
        CoordValue(0.0)
    }
}

impl<'de> Deserialize<'de> for CoordValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, Visitor};

        struct CoordValueVisitor;

        impl<'de> Visitor<'de> for CoordValueVisitor {
            type Value = CoordValue;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str(
                    "a number (mils) or a string with unit (e.g., \"100mil\", \"2.54mm\")",
                )
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(CoordValue(value as f64))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(CoordValue(value as f64))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(CoordValue(value))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                parse_unit_value_or_mil(value)
                    .map(CoordValue)
                    .map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_any(CoordValueVisitor)
    }
}

impl Serialize for CoordValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize as a number (mils) for backward compatibility
        serializer.serialize_f64(self.0)
    }
}

/// Add a new component to the library.
pub fn cmd_add_component(
    path: &Path,
    name: &str,
    description: Option<String>,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut lib = open_or_create_schlib(path)?;

    // Check if component already exists
    if lib
        .components
        .iter()
        .any(|c| c.component.lib_reference == name)
    {
        return Err(format!("Component '{}' already exists", name).into());
    }

    // Create the component record
    let component = SchComponent {
        lib_reference: name.to_string(),
        component_description: description.clone().unwrap_or_default(),
        part_count: 1,
        display_mode_count: 1,
        current_part_id: 1,
        ..Default::default()
    };

    let mut primitives = vec![SchRecord::Component(component.clone())];

    // Create designator record (RECORD=34)
    let designator_text = infer_designator_prefix(name, &description.clone().unwrap_or_default());
    let designator = SchDesignator {
        param: SchParameter {
            label: SchLabel {
                text: designator_text,
                font_id: 1,
                ..Default::default()
            },
            name: "Designator".to_string(),
            read_only_state: 1,
            ..Default::default()
        },
        ..Default::default()
    };
    primitives.push(SchRecord::Designator(designator));

    // Create empty implementation list (RECORD=44)
    let impl_list = SchImplementationList {
        base: SchPrimitiveBase {
            owner_index: 0,
            ..Default::default()
        },
        ..Default::default()
    };
    primitives.push(SchRecord::ImplementationList(impl_list));

    // Create SchLibComponent
    let lib_component = SchLibComponent {
        component: component.clone(),
        primitives,
    };

    lib.components.push(lib_component);
    save_schlib(path, &lib)?;

    Ok(format!("Added component '{}' to {}", name, path.display()))
}

/// Add a pin to a component.
#[allow(clippy::too_many_arguments)]
pub fn cmd_add_pin(
    path: &Path,
    component_name: &str,
    designator: &str,
    name: &str,
    x: &str,
    y: &str,
    length: &str,
    electrical: &str,
    orientation: &str,
    hidden: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut lib = open_schlib(path)?;

    // Parse coordinate values with units
    let x_mils = parse_unit_value_or_mil(x)?;
    let y_mils = parse_unit_value_or_mil(y)?;
    let length_mils = parse_unit_value_or_mil(length)?;

    // Find the component
    let component = lib
        .components
        .iter_mut()
        .find(|c| c.component.lib_reference == component_name)
        .ok_or_else(|| format!("Component '{}' not found", component_name))?;

    // Parse electrical type and orientation
    let electrical_type = parse_electrical_type(electrical)?;
    let mut conglomerate = parse_pin_orientation(orientation)?;

    // Set visibility flags
    conglomerate |= PinConglomerateFlags::DISPLAY_NAME_VISIBLE;
    conglomerate |= PinConglomerateFlags::DESIGNATOR_VISIBLE;

    if hidden {
        conglomerate |= PinConglomerateFlags::HIDE;
    }

    // Create the pin
    let mut graphical = SchGraphicalBase::default();
    graphical.base.owner_part_id = Some(1);
    graphical.location_x = mils_f64_to_raw(x_mils);
    graphical.location_y = mils_f64_to_raw(y_mils);
    graphical.color = 0x000080; // Dark blue default

    let pin = SchPin {
        graphical,
        designator: designator.to_string(),
        name: name.to_string(),
        electrical: electrical_type,
        pin_conglomerate: conglomerate,
        pin_length: mils_f64_to_raw(length_mils),
        symbol_inner_edge: PinSymbol::None,
        symbol_outer_edge: PinSymbol::None,
        symbol_inside: PinSymbol::None,
        symbol_outside: PinSymbol::None,
        ..Default::default()
    };

    component.primitives.push(SchRecord::Pin(pin));
    save_schlib(path, &lib)?;

    Ok(format!(
        "Added pin '{}' ({}) to component '{}'",
        designator, name, component_name
    ))
}

/// Add a rectangle to a component.
#[allow(clippy::too_many_arguments)]
pub fn cmd_add_rectangle(
    path: &Path,
    component_name: &str,
    x1: &str,
    y1: &str,
    x2: &str,
    y2: &str,
    filled: bool,
    fill_color: &str,
    border_color: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut lib = open_schlib(path)?;

    // Parse coordinate values with units
    let x1_mils = parse_unit_value_or_mil(x1)?;
    let y1_mils = parse_unit_value_or_mil(y1)?;
    let x2_mils = parse_unit_value_or_mil(x2)?;
    let y2_mils = parse_unit_value_or_mil(y2)?;

    // Find the component
    let component = lib
        .components
        .iter_mut()
        .find(|c| c.component.lib_reference == component_name)
        .ok_or_else(|| format!("Component '{}' not found", component_name))?;

    // Parse colors
    let fill_color_val = parse_color(fill_color)?;
    let border_color_val = parse_color(border_color)?;

    // Create the rectangle
    let mut graphical = SchGraphicalBase::default();
    graphical.base.owner_part_id = Some(1);
    graphical.location_x = mils_f64_to_raw(x1_mils);
    graphical.location_y = mils_f64_to_raw(y1_mils);
    graphical.color = border_color_val;
    graphical.area_color = fill_color_val;

    let rect = SchRectangle {
        graphical,
        corner_x: mils_f64_to_raw(x2_mils),
        corner_y: mils_f64_to_raw(y2_mils),
        line_width: LineWidth::Small,
        is_solid: filled,
        transparent: !filled,
        ..Default::default()
    };

    component.primitives.push(SchRecord::Rectangle(rect));
    save_schlib(path, &lib)?;

    Ok(format!("Added rectangle to component '{}'", component_name))
}

/// Add a line to a component.
pub fn cmd_add_line(
    path: &Path,
    component_name: &str,
    x1: &str,
    y1: &str,
    x2: &str,
    y2: &str,
    color: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut lib = open_schlib(path)?;

    // Parse coordinate values with units
    let x1_mils = parse_unit_value_or_mil(x1)?;
    let y1_mils = parse_unit_value_or_mil(y1)?;
    let x2_mils = parse_unit_value_or_mil(x2)?;
    let y2_mils = parse_unit_value_or_mil(y2)?;

    // Find the component
    let component = lib
        .components
        .iter_mut()
        .find(|c| c.component.lib_reference == component_name)
        .ok_or_else(|| format!("Component '{}' not found", component_name))?;

    // Parse color
    let color_val = parse_color(color)?;

    // Create the line
    let mut graphical = SchGraphicalBase::default();
    graphical.base.owner_part_id = Some(1);
    graphical.location_x = mils_f64_to_raw(x1_mils);
    graphical.location_y = mils_f64_to_raw(y1_mils);
    graphical.color = color_val;

    let line = SchLine {
        graphical,
        corner_x: mils_f64_to_raw(x2_mils),
        corner_y: mils_f64_to_raw(y2_mils),
        line_width: LineWidth::Small,
        ..Default::default()
    };

    component.primitives.push(SchRecord::Line(line));
    save_schlib(path, &lib)?;

    Ok(format!("Added line to component '{}'", component_name))
}

/// Add a polygon to a component.
pub fn cmd_add_polygon(
    path: &Path,
    component_name: &str,
    vertices_str: &str,
    filled: bool,
    fill_color: &str,
    border_color: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut lib = open_schlib(path)?;

    // Find the component
    let component = lib
        .components
        .iter_mut()
        .find(|c| c.component.lib_reference == component_name)
        .ok_or_else(|| format!("Component '{}' not found", component_name))?;

    // Parse vertices with unit support
    let values: Vec<f64> = vertices_str
        .split(',')
        .map(|s| parse_unit_value_or_mil(s))
        .collect::<Result<Vec<_>, _>>()?;

    if values.len() < 6 || values.len() % 2 != 0 {
        return Err("Need at least 3 vertex pairs (6 values)".into());
    }

    let vertices: Vec<(i32, i32)> = values
        .chunks(2)
        .map(|chunk| (mils_f64_to_raw(chunk[0]), mils_f64_to_raw(chunk[1])))
        .collect();

    // Parse colors
    let fill_color_val = parse_color(fill_color)?;
    let border_color_val = parse_color(border_color)?;

    // Create the polygon
    let mut graphical = SchGraphicalBase::default();
    graphical.base.owner_part_id = Some(1);
    graphical.location_x = vertices[0].0;
    graphical.location_y = vertices[0].1;
    graphical.color = border_color_val;
    graphical.area_color = fill_color_val;

    let polygon = SchPolygon {
        graphical,
        vertices,
        line_width: LineWidth::Small,
        is_solid: filled,
        transparent: !filled,
        ..Default::default()
    };

    component.primitives.push(SchRecord::Polygon(polygon));
    save_schlib(path, &lib)?;

    Ok(format!(
        "Added polygon with {} vertices to component '{}'",
        values.len() / 2,
        component_name
    ))
}

/// Pin definition for gen-ic command.
struct PinDef {
    designator: String,
    name: String,
    electrical: PinElectricalType,
    side: String,
}

/// Parse pin definitions from string.
fn parse_pin_defs(pins_str: &str) -> Result<Vec<PinDef>, Box<dyn std::error::Error>> {
    let mut pins = Vec::new();

    for pin_spec in pins_str.split(',') {
        let parts: Vec<&str> = pin_spec.trim().split(':').collect();
        if parts.len() < 3 {
            return Err(format!(
                "Invalid pin spec '{}'. Format: designator:name:type[:side]",
                pin_spec
            )
            .into());
        }

        let electrical = parse_electrical_type(parts[2])?;
        let side = if parts.len() > 3 {
            parts[3].to_lowercase()
        } else {
            "left".to_string()
        };

        pins.push(PinDef {
            designator: parts[0].to_string(),
            name: parts[1].to_string(),
            electrical,
            side,
        });
    }

    Ok(pins)
}

/// Generate a standard IC symbol.
pub fn cmd_gen_ic(
    path: &Path,
    name: &str,
    pins_str: &str,
    description: Option<String>,
    width: &str,
    pin_length: &str,
    pin_spacing: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut lib = open_or_create_schlib(path)?;

    // Parse dimension values with units
    let width_mils = parse_unit_value_or_mil(width)?;
    let pin_length_mils = parse_unit_value_or_mil(pin_length)?;
    let pin_spacing_mils = parse_unit_value_or_mil(pin_spacing)?;

    // Check if component already exists
    if lib
        .components
        .iter()
        .any(|c| c.component.lib_reference == name)
    {
        return Err(format!("Component '{}' already exists", name).into());
    }

    // Parse pin definitions
    let pin_defs = parse_pin_defs(pins_str)?;

    // Separate pins by side
    let left_pins: Vec<_> = pin_defs.iter().filter(|p| p.side == "left").collect();
    let right_pins: Vec<_> = pin_defs.iter().filter(|p| p.side == "right").collect();
    let top_pins: Vec<_> = pin_defs.iter().filter(|p| p.side == "top").collect();
    let bottom_pins: Vec<_> = pin_defs.iter().filter(|p| p.side == "bottom").collect();

    // Calculate body dimensions based on pin count
    let left_count = left_pins.len();
    let right_count = right_pins.len();
    let top_count = top_pins.len();
    let bottom_count = bottom_pins.len();
    let max_vertical_pins = left_count.max(right_count);
    let max_horizontal_pins = top_count.max(bottom_count);
    let body_height_mils = (max_vertical_pins + 1) as f64 * pin_spacing_mils;
    // Widen body if top/bottom pins need more space
    let min_width_for_tb = if max_horizontal_pins > 0 {
        (max_horizontal_pins + 1) as f64 * pin_spacing_mils
    } else {
        0.0
    };
    let width_mils = width_mils.max(min_width_for_tb);

    // Create component
    let component = SchComponent {
        lib_reference: name.to_string(),
        component_description: description.clone().unwrap_or_default(),
        part_count: 1,
        display_mode_count: 1,
        current_part_id: 1,
        ..Default::default()
    };

    let mut primitives = vec![SchRecord::Component(component.clone())];

    // Create designator record (RECORD=34)
    let designator_text = infer_designator_prefix(name, &description.clone().unwrap_or_default());
    let designator = SchDesignator {
        param: SchParameter {
            label: SchLabel {
                text: designator_text,
                font_id: 1,
                ..Default::default()
            },
            name: "Designator".to_string(),
            read_only_state: 1,
            ..Default::default()
        },
        ..Default::default()
    };
    primitives.push(SchRecord::Designator(designator));

    // Create empty implementation list (RECORD=44)
    let impl_list = SchImplementationList {
        base: SchPrimitiveBase {
            owner_index: 0,
            ..Default::default()
        },
        ..Default::default()
    };
    primitives.push(SchRecord::ImplementationList(impl_list));

    // Add body rectangle
    let mut rect_graphical = SchGraphicalBase::default();
    rect_graphical.base.owner_part_id = Some(1);
    rect_graphical.location_x = mils_to_raw(0);
    rect_graphical.location_y = mils_to_raw(0);
    rect_graphical.color = parse_color("800000")?; // Dark red border
    rect_graphical.area_color = parse_color("FFFFB0")?; // Light yellow fill

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

        let mut graphical = SchGraphicalBase::default();
        graphical.base.owner_part_id = Some(1);
        graphical.location_x = mils_f64_to_raw(-pin_length_mils);
        graphical.location_y = mils_f64_to_raw(y_mils);
        graphical.color = 0x000080;

        let pin = SchPin {
            graphical,
            designator: pin_def.designator.clone(),
            name: pin_def.name.clone(),
            electrical: pin_def.electrical,
            pin_conglomerate: PinConglomerateFlags::DISPLAY_NAME_VISIBLE
                | PinConglomerateFlags::DESIGNATOR_VISIBLE,
            pin_length: mils_f64_to_raw(pin_length_mils),
            ..Default::default()
        };
        primitives.push(SchRecord::Pin(pin));
    }

    // Add right pins (pointing left into body)
    for (i, pin_def) in right_pins.iter().enumerate() {
        let y_mils = body_height_mils - (i + 1) as f64 * pin_spacing_mils;

        let mut graphical = SchGraphicalBase::default();
        graphical.base.owner_part_id = Some(1);
        graphical.location_x = mils_f64_to_raw(width_mils + pin_length_mils);
        graphical.location_y = mils_f64_to_raw(y_mils);
        graphical.color = 0x000080;

        let pin = SchPin {
            graphical,
            designator: pin_def.designator.clone(),
            name: pin_def.name.clone(),
            electrical: pin_def.electrical,
            pin_conglomerate: PinConglomerateFlags::DISPLAY_NAME_VISIBLE
                | PinConglomerateFlags::DESIGNATOR_VISIBLE
                | PinConglomerateFlags::FLIPPED,
            pin_length: mils_f64_to_raw(pin_length_mils),
            ..Default::default()
        };
        primitives.push(SchRecord::Pin(pin));
    }

    // Add top pins (pointing down into body)
    for (i, pin_def) in top_pins.iter().enumerate() {
        let x_mils = (i + 1) as f64 * pin_spacing_mils;

        let mut graphical = SchGraphicalBase::default();
        graphical.base.owner_part_id = Some(1);
        graphical.location_x = mils_f64_to_raw(x_mils);
        graphical.location_y = mils_f64_to_raw(body_height_mils + pin_length_mils);
        graphical.color = 0x000080;

        let pin = SchPin {
            graphical,
            designator: pin_def.designator.clone(),
            name: pin_def.name.clone(),
            electrical: pin_def.electrical,
            pin_conglomerate: PinConglomerateFlags::DISPLAY_NAME_VISIBLE
                | PinConglomerateFlags::DESIGNATOR_VISIBLE
                | PinConglomerateFlags::ROTATED,
            pin_length: mils_f64_to_raw(pin_length_mils),
            ..Default::default()
        };
        primitives.push(SchRecord::Pin(pin));
    }

    // Add bottom pins (pointing up into body)
    for (i, pin_def) in bottom_pins.iter().enumerate() {
        let x_mils = (i + 1) as f64 * pin_spacing_mils;

        let mut graphical = SchGraphicalBase::default();
        graphical.base.owner_part_id = Some(1);
        graphical.location_x = mils_f64_to_raw(x_mils);
        graphical.location_y = mils_f64_to_raw(-pin_length_mils);
        graphical.color = 0x000080;

        let pin = SchPin {
            graphical,
            designator: pin_def.designator.clone(),
            name: pin_def.name.clone(),
            electrical: pin_def.electrical,
            pin_conglomerate: PinConglomerateFlags::DISPLAY_NAME_VISIBLE
                | PinConglomerateFlags::DESIGNATOR_VISIBLE
                | PinConglomerateFlags::ROTATED
                | PinConglomerateFlags::FLIPPED,
            pin_length: mils_f64_to_raw(pin_length_mils),
            ..Default::default()
        };
        primitives.push(SchRecord::Pin(pin));
    }

    let lib_component = SchLibComponent {
        component,
        primitives,
    };

    lib.components.push(lib_component);
    save_schlib(path, &lib)?;

    Ok(format!(
        "Generated IC symbol '{}' with {} pins ({} left, {} right, {} top, {} bottom)",
        name,
        pin_defs.len(),
        left_count,
        right_count,
        top_count,
        bottom_count,
    ))
}

/// Render a component symbol as ASCII art.
pub fn cmd_render_ascii(
    path: &Path,
    component_name: &str,
    max_width: usize,
    max_height: usize,
) -> Result<String, Box<dyn std::error::Error>> {
    let lib = open_schlib(path)?;

    let name_lower = component_name.to_lowercase();
    let component = lib
        .components
        .iter()
        .find(|c| c.component.lib_reference.to_lowercase() == name_lower)
        .ok_or_else(|| format!("Component '{}' not found", component_name))?;

    // Find bounds
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;

    for prim in &component.primitives {
        match prim {
            SchRecord::Pin(p) => {
                let (cx, cy) = p.get_corner();
                min_x = min_x.min(p.graphical.location_x).min(cx);
                min_y = min_y.min(p.graphical.location_y).min(cy);
                max_x = max_x.max(p.graphical.location_x).max(cx);
                max_y = max_y.max(p.graphical.location_y).max(cy);
            }
            SchRecord::Rectangle(r) => {
                min_x = min_x.min(r.graphical.location_x).min(r.corner_x);
                min_y = min_y.min(r.graphical.location_y).min(r.corner_y);
                max_x = max_x.max(r.graphical.location_x).max(r.corner_x);
                max_y = max_y.max(r.graphical.location_y).max(r.corner_y);
            }
            SchRecord::Line(l) => {
                min_x = min_x.min(l.graphical.location_x).min(l.corner_x);
                min_y = min_y.min(l.graphical.location_y).min(l.corner_y);
                max_x = max_x.max(l.graphical.location_x).max(l.corner_x);
                max_y = max_y.max(l.graphical.location_y).max(l.corner_y);
            }
            _ => {}
        }
    }

    if min_x == i32::MAX {
        return Ok("No renderable primitives found.".to_string());
    }

    let width_raw = (max_x - min_x) as f64;
    let height_raw = (max_y - min_y) as f64;

    // Scale to fit
    let scale_x = (max_width as f64 - 2.0) / width_raw;
    let scale_y = (max_height as f64 - 2.0) / height_raw;
    let scale = scale_x.min(scale_y);

    let canvas_width = ((width_raw * scale) as usize + 2).min(max_width);
    let canvas_height = ((height_raw * scale) as usize + 2).min(max_height);

    // Create canvas
    let mut canvas: Vec<Vec<char>> = vec![vec![' '; canvas_width]; canvas_height];

    // Helper to convert coords
    let to_canvas = |x: i32, y: i32| -> (usize, usize) {
        let cx = ((x - min_x) as f64 * scale) as usize;
        let cy = canvas_height - 1 - (((y - min_y) as f64 * scale) as usize);
        (cx.min(canvas_width - 1), cy.min(canvas_height - 1))
    };

    // Draw rectangles
    for prim in &component.primitives {
        if let SchRecord::Rectangle(r) = prim {
            let (x1, y1) = to_canvas(r.graphical.location_x, r.graphical.location_y);
            let (x2, y2) = to_canvas(r.corner_x, r.corner_y);
            let (x1, x2) = (x1.min(x2), x1.max(x2));
            let (y1, y2) = (y1.min(y2), y1.max(y2));

            // Draw rectangle border
            for x in x1..=x2 {
                if y1 < canvas_height {
                    canvas[y1][x.min(canvas_width - 1)] = '-';
                }
                if y2 < canvas_height {
                    canvas[y2][x.min(canvas_width - 1)] = '-';
                }
            }
            for y in y1..=y2 {
                if x1 < canvas_width {
                    canvas[y.min(canvas_height - 1)][x1] = '|';
                }
                if x2 < canvas_width {
                    canvas[y.min(canvas_height - 1)][x2] = '|';
                }
            }
            // Corners
            if y1 < canvas_height && x1 < canvas_width {
                canvas[y1][x1] = '+';
            }
            if y1 < canvas_height && x2 < canvas_width {
                canvas[y1][x2] = '+';
            }
            if y2 < canvas_height && x1 < canvas_width {
                canvas[y2][x1] = '+';
            }
            if y2 < canvas_height && x2 < canvas_width {
                canvas[y2][x2] = '+';
            }
        }
    }

    // Draw pins
    for prim in &component.primitives {
        if let SchRecord::Pin(p) = prim {
            let (px, py) = to_canvas(p.graphical.location_x, p.graphical.location_y);
            let (cx, cy) = p.get_corner();
            let (ex, ey) = to_canvas(cx, cy);

            // Draw pin line
            if px == ex {
                let y_start = py.min(ey);
                let y_end = py.max(ey);
                for row in canvas.iter_mut().take(y_end + 1).skip(y_start) {
                    if let Some(cell) = row.get_mut(px) {
                        *cell = '|';
                    }
                }
            } else if let Some(row) = canvas.get_mut(py) {
                let x_start = px.min(ex);
                let x_end = px.max(ex);
                for cell in row.iter_mut().take(x_end + 1).skip(x_start) {
                    *cell = '-';
                }
            }

            // Draw pin endpoint marker
            if py < canvas_height && px < canvas_width {
                canvas[py][px] = 'o';
            }
        }
    }

    // Build output string
    let mut output = String::new();
    output.push_str(&format!("\n{}\n", component.component.lib_reference));
    output.push_str(&format!(
        "{}\n",
        "=".repeat(component.component.lib_reference.len())
    ));
    for row in &canvas {
        output.push_str(&format!("{}\n", row.iter().collect::<String>()));
    }

    // Add pin list
    output.push_str("\nPins:\n");
    let mut pins: Vec<&SchPin> = component
        .primitives
        .iter()
        .filter_map(|p| {
            if let SchRecord::Pin(pin) = p {
                Some(pin)
            } else {
                None
            }
        })
        .collect();
    pins.sort_by(|a, b| alphanumeric_sort(&a.designator, &b.designator));

    for pin in pins {
        output.push_str(&format!(
            "  {} - {} ({})\n",
            pin.designator,
            pin.name,
            electrical_type_name(&pin.electrical)
        ));
    }

    Ok(output)
}

// ═══════════════════════════════════════════════════════════════════════════
// REVERSE CONVERSION HELPERS (for JSON export)
// ═══════════════════════════════════════════════════════════════════════════

/// Convert raw coordinate value (10000 units/mil) back to mils.
fn raw_to_coord_value(raw: i32) -> CoordValue {
    CoordValue(raw as f64 / 10000.0)
}

/// Convert Win32 COLORREF (0x00BBGGRR) to "RRGGBB" hex string.
fn color_to_hex(color: i32) -> String {
    let r = (color & 0xFF) as u8;
    let g = ((color >> 8) & 0xFF) as u8;
    let b = ((color >> 16) & 0xFF) as u8;
    format!("{:02X}{:02X}{:02X}", r, g, b)
}

/// Convert PinConglomerateFlags to orientation string.
fn pin_orientation_to_string(flags: PinConglomerateFlags) -> String {
    let rotated = flags.contains(PinConglomerateFlags::ROTATED);
    let flipped = flags.contains(PinConglomerateFlags::FLIPPED);
    match (rotated, flipped) {
        (false, false) => "right".to_string(),
        (false, true) => "left".to_string(),
        (true, false) => "up".to_string(),
        (true, true) => "down".to_string(),
    }
}

/// Convert PinElectricalType to JSON-friendly string.
fn electrical_type_to_json(et: &PinElectricalType) -> String {
    match et {
        PinElectricalType::Input => "input".to_string(),
        PinElectricalType::InputOutput => "io".to_string(),
        PinElectricalType::Output => "output".to_string(),
        PinElectricalType::OpenCollector => "oc".to_string(),
        PinElectricalType::Passive => "passive".to_string(),
        PinElectricalType::HiZ => "hiz".to_string(),
        PinElectricalType::OpenEmitter => "oe".to_string(),
        PinElectricalType::Power => "power".to_string(),
    }
}

/// Convert TextOrientations flags to JSON string.
fn text_orientation_to_string(orient: TextOrientations) -> String {
    let rotated = orient.contains(TextOrientations::ROTATED);
    let flipped = orient.contains(TextOrientations::FLIPPED);
    match (rotated, flipped) {
        (false, false) => "horizontal".to_string(),
        (true, false) => "vertical_up".to_string(),
        (true, true) => "vertical_down".to_string(),
        (false, true) => "180".to_string(),
    }
}

/// Convert TextJustification to JSON string.
fn text_justification_to_string(just: TextJustification) -> String {
    if just == TextJustification::BOTTOM_LEFT {
        "bottom_left"
    } else if just == TextJustification::BOTTOM_CENTER {
        "bottom_center"
    } else if just == TextJustification::BOTTOM_RIGHT {
        "bottom_right"
    } else if just == TextJustification::MIDDLE_LEFT {
        "center_left"
    } else if just == TextJustification::MIDDLE_CENTER {
        "center"
    } else if just == TextJustification::MIDDLE_RIGHT {
        "center_right"
    } else if just == TextJustification::TOP_LEFT {
        "top_left"
    } else if just == TextJustification::TOP_CENTER {
        "top_center"
    } else if just == TextJustification::TOP_RIGHT {
        "top_right"
    } else {
        "bottom_left"
    }.to_string()
}

// ═══════════════════════════════════════════════════════════════════════════
// JSON INPUT STRUCTURES (for LLM tool calling and structured output)
// ═══════════════════════════════════════════════════════════════════════════

/// JSON schema for a pin in a schematic component.
/// Coordinates accept numbers (mils) or strings with units (e.g., "100mil", "2.54mm", "0.1in").
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SchPinJson {
    /// Pin designator (e.g., "1", "2", "A1")
    pub designator: String,
    /// Pin name (e.g., "VCC", "GND", "DATA0")
    pub name: String,
    /// X position: number (mils) or string with unit (e.g., "100mil", "2.54mm")
    pub x: CoordValue,
    /// Y position: number (mils) or string with unit (e.g., "100mil", "2.54mm")
    pub y: CoordValue,
    /// Pin length: number (mils) or string with unit (default: 200mil)
    #[serde(default = "default_pin_length")]
    pub length: CoordValue,
    /// Electrical type: "input", "output", "io", "passive", "power", "oc", "oe", "hiz"
    #[serde(default = "default_electrical")]
    pub electrical: String,
    /// Pin orientation: "right", "left", "up", "down"
    #[serde(default = "default_orientation")]
    pub orientation: String,
    /// Hide the pin (optional)
    #[serde(default)]
    pub hidden: bool,
    /// Pin description (optional)
    #[serde(default)]
    pub description: String,
}

fn default_pin_length() -> CoordValue {
    CoordValue(200.0)
}

fn default_electrical() -> String {
    "passive".to_string()
}

fn default_orientation() -> String {
    "right".to_string()
}

/// JSON schema for a rectangle in a schematic component.
/// Coordinates accept numbers (mils) or strings with units (e.g., "100mil", "2.54mm", "0.1in").
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SchRectangleJson {
    /// Corner 1 X: number (mils) or string with unit
    pub x1: CoordValue,
    /// Corner 1 Y: number (mils) or string with unit
    pub y1: CoordValue,
    /// Corner 2 X: number (mils) or string with unit
    pub x2: CoordValue,
    /// Corner 2 Y: number (mils) or string with unit
    pub y2: CoordValue,
    /// Fill the rectangle
    #[serde(default)]
    pub filled: bool,
    /// Fill color in hex (RRGGBB), default light yellow
    #[serde(default = "default_fill_color")]
    pub fill_color: String,
    /// Border color in hex (RRGGBB), default dark blue
    #[serde(default = "default_border_color")]
    pub border_color: String,
}

fn default_fill_color() -> String {
    "FFFFB0".to_string()
}

fn default_border_color() -> String {
    "000080".to_string()
}

/// JSON schema for a line in a schematic component.
/// Coordinates accept numbers (mils) or strings with units (e.g., "100mil", "2.54mm", "0.1in").
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SchLineJson {
    /// Start X: number (mils) or string with unit
    pub x1: CoordValue,
    /// Start Y: number (mils) or string with unit
    pub y1: CoordValue,
    /// End X: number (mils) or string with unit
    pub x2: CoordValue,
    /// End Y: number (mils) or string with unit
    pub y2: CoordValue,
    /// Line color in hex (RRGGBB)
    #[serde(default = "default_border_color")]
    pub color: String,
}

/// JSON schema for a polygon in a schematic component.
/// Vertices accept numbers (mils) or strings with units.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SchPolygonJson {
    /// Vertices as array of [x, y] pairs: numbers (mils) or strings with units
    pub vertices: Vec<[CoordValue; 2]>,
    /// Fill the polygon
    #[serde(default)]
    pub filled: bool,
    /// Fill color in hex (RRGGBB)
    #[serde(default = "default_fill_color")]
    pub fill_color: String,
    /// Border color in hex (RRGGBB)
    #[serde(default = "default_border_color")]
    pub border_color: String,
}

/// JSON schema for a text label in a schematic component.
/// Coordinates accept numbers (mils) or strings with units (e.g., "100mil", "2.54mm", "0.1in").
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SchLabelJson {
    /// X position: number (mils) or string with unit
    pub x: CoordValue,
    /// Y position: number (mils) or string with unit
    pub y: CoordValue,
    /// Label text
    pub text: String,
    /// Text orientation: "horizontal", "vertical_up", "vertical_down", "90", "180", "270"
    #[serde(default = "default_label_orientation")]
    pub orientation: String,
    /// Text justification: "bottom_left", "bottom_center", "bottom_right",
    /// "center_left", "center", "center_right", "top_left", "top_center", "top_right"
    #[serde(default = "default_justification")]
    pub justification: String,
    /// Text color in hex (RRGGBB)
    #[serde(default = "default_border_color")]
    pub color: String,
    /// Font ID (optional, default 1)
    #[serde(default = "default_font_id")]
    pub font_id: i32,
    /// Hide the label
    #[serde(default)]
    pub hidden: bool,
}

fn default_label_orientation() -> String {
    "horizontal".to_string()
}

fn default_justification() -> String {
    "bottom_left".to_string()
}

fn default_font_id() -> i32 {
    1
}

/// JSON schema for an arc in a schematic component.
/// Coordinates accept numbers (mils) or strings with units (e.g., "100mil", "2.54mm", "0.1in").
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SchArcJson {
    /// Center X: number (mils) or string with unit
    pub x: CoordValue,
    /// Center Y: number (mils) or string with unit
    pub y: CoordValue,
    /// Radius: number (mils) or string with unit
    pub radius: CoordValue,
    /// Start angle in degrees (0 = right, 90 = up)
    #[serde(default)]
    pub start_angle: f64,
    /// End angle in degrees
    #[serde(default = "default_end_angle")]
    pub end_angle: f64,
    /// Line color in hex (RRGGBB)
    #[serde(default = "default_border_color")]
    pub color: String,
}

fn default_end_angle() -> f64 {
    360.0
}

/// JSON schema for a polyline in a schematic component.
/// Vertices accept numbers (mils) or strings with units.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SchPolylineJson {
    /// Vertices as array of [x, y] pairs: numbers (mils) or strings with units
    pub vertices: Vec<[CoordValue; 2]>,
    /// Line color in hex (RRGGBB)
    #[serde(default = "default_border_color")]
    pub color: String,
}

/// JSON schema for an ellipse in a schematic component.
/// Coordinates accept numbers (mils) or strings with units (e.g., "100mil", "2.54mm", "0.1in").
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SchEllipseJson {
    /// Center X: number (mils) or string with unit
    pub x: CoordValue,
    /// Center Y: number (mils) or string with unit
    pub y: CoordValue,
    /// X radius: number (mils) or string with unit
    pub radius_x: CoordValue,
    /// Y radius: number (mils) or string with unit
    pub radius_y: CoordValue,
    /// Fill the ellipse
    #[serde(default)]
    pub filled: bool,
    /// Fill color in hex (RRGGBB)
    #[serde(default = "default_fill_color")]
    pub fill_color: String,
    /// Border color in hex (RRGGBB)
    #[serde(default = "default_border_color")]
    pub border_color: String,
}

/// JSON schema for a bezier curve in a schematic component.
/// Vertices accept numbers (mils) or strings with units.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SchBezierJson {
    /// Control points as array of [x, y] pairs: numbers (mils) or strings with units
    pub vertices: Vec<[CoordValue; 2]>,
    /// Line color in hex (RRGGBB)
    #[serde(default = "default_border_color")]
    pub color: String,
}

/// JSON schema for an elliptical arc in a schematic component.
/// Coordinates accept numbers (mils) or strings with units.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SchEllipticalArcJson {
    /// Center X: number (mils) or string with unit
    pub x: CoordValue,
    /// Center Y: number (mils) or string with unit
    pub y: CoordValue,
    /// Primary radius: number (mils) or string with unit
    pub radius: CoordValue,
    /// Secondary radius: number (mils) or string with unit
    pub secondary_radius: CoordValue,
    /// Start angle in degrees (0 = right, 90 = up)
    #[serde(default)]
    pub start_angle: f64,
    /// End angle in degrees
    #[serde(default = "default_end_angle")]
    pub end_angle: f64,
    /// Line color in hex (RRGGBB)
    #[serde(default = "default_border_color")]
    pub color: String,
}

/// JSON schema for a designator in a schematic component.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SchDesignatorJson {
    /// Designator text (e.g., "U?", "R?")
    pub text: String,
    /// X position (mils)
    #[serde(default)]
    pub x: CoordValue,
    /// Y position (mils)
    #[serde(default)]
    pub y: CoordValue,
    /// Font ID
    #[serde(default = "default_font_id")]
    pub font_id: i32,
    /// Whether hidden
    #[serde(default)]
    pub hidden: bool,
}

/// JSON schema for a parameter in a schematic component.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SchParameterJson {
    /// Parameter name (e.g., "Comment", "Value")
    pub name: String,
    /// Parameter value
    #[serde(default)]
    pub value: String,
    /// X position (mils)
    #[serde(default)]
    pub x: CoordValue,
    /// Y position (mils)
    #[serde(default)]
    pub y: CoordValue,
    /// Font ID
    #[serde(default = "default_font_id")]
    pub font_id: i32,
    /// Whether hidden
    #[serde(default)]
    pub hidden: bool,
    /// Read-only state
    #[serde(default)]
    pub read_only_state: i32,
    /// Text orientation
    #[serde(default = "default_label_orientation")]
    pub orientation: String,
}

/// JSON schema for an implementation (model reference) in a schematic component.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SchImplementationJson {
    /// Model name (e.g., footprint name)
    pub model_name: String,
    /// Model type (e.g., "PCBLIB", "SIM", "SI", "PCB3DLib")
    pub model_type: String,
    /// Description
    #[serde(default)]
    pub description: String,
    /// Whether this is the current/active implementation
    #[serde(default)]
    pub is_current: bool,
    /// Data file references
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub data_files: Vec<String>,
    /// Data file entity names (model names referenced by each data file)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub data_file_entities: Vec<String>,
    /// Pin mappings (schematic pin -> implementation pin)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pin_mappings: Vec<SchPinMappingJson>,
}

/// JSON schema for a pin mapping in an implementation.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SchPinMappingJson {
    /// Schematic pin designator
    pub schematic_pin: String,
    /// Implementation (footprint) pin designators
    pub implementation_pins: Vec<String>,
    /// Whether this is a trivial (identity) mapping
    #[serde(default)]
    pub is_trivial: bool,
}

/// JSON schema for a complete schematic component definition.
/// This is the top-level structure for the add-json command.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SchComponentJson {
    /// Component name (LIBREFERENCE)
    pub name: String,
    /// Component description (optional)
    #[serde(default)]
    pub description: String,
    /// Number of parts (for multi-part components, default 1)
    #[serde(default = "default_part_count")]
    pub part_count: i32,
    /// List of pins
    #[serde(default)]
    pub pins: Vec<SchPinJson>,
    /// List of rectangles (typically for the body)
    #[serde(default)]
    pub rectangles: Vec<SchRectangleJson>,
    /// List of lines
    #[serde(default)]
    pub lines: Vec<SchLineJson>,
    /// List of polygons
    #[serde(default)]
    pub polygons: Vec<SchPolygonJson>,
    /// List of text labels
    #[serde(default)]
    pub labels: Vec<SchLabelJson>,
    /// List of arcs
    #[serde(default)]
    pub arcs: Vec<SchArcJson>,
    /// List of polylines (multi-segment lines)
    #[serde(default)]
    pub polylines: Vec<SchPolylineJson>,
    /// List of ellipses
    #[serde(default)]
    pub ellipses: Vec<SchEllipseJson>,
    /// List of bezier curves
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub beziers: Vec<SchBezierJson>,
    /// List of elliptical arcs
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub elliptical_arcs: Vec<SchEllipticalArcJson>,
    /// Designator definition (optional - inferred from name if not provided)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub designator: Option<SchDesignatorJson>,
    /// Component parameters (Comment, Value, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<SchParameterJson>,
    /// Implementation references (footprints, simulation models)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub implementations: Vec<SchImplementationJson>,
    /// Number of display modes
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_mode_count: Option<i32>,
    /// Unique ID for cross-document tracking
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub unique_id: String,
}

fn default_part_count() -> i32 {
    1
}

/// File-level export of a schematic library for round-trip JSON support.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SchLibExport {
    /// Source file path (informational)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Number of components
    pub component_count: usize,
    /// Raw FileHeader parameters (fonts, grid, sheet settings).
    /// Keys are Altium parameter names (e.g. "FONTIDCOUNT", "SNAPGRIDSIZE").
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub header: HashMap<String, String>,
    /// Full component definitions
    pub components: Vec<SchComponentJson>,
}

/// Convert a SchLibComponent to SchComponentJson for export.
fn component_to_json(comp: &SchLibComponent) -> SchComponentJson {
    let mut pins = Vec::new();
    let mut rectangles = Vec::new();
    let mut lines = Vec::new();
    let mut polygons = Vec::new();
    let mut labels = Vec::new();
    let mut arcs = Vec::new();
    let mut polylines = Vec::new();
    let mut ellipses = Vec::new();
    let mut beziers = Vec::new();
    let mut elliptical_arcs = Vec::new();
    let mut designator: Option<SchDesignatorJson> = None;
    let mut parameters = Vec::new();
    let mut implementations = Vec::new();

    // Build ordered lists of implementation hierarchy (preserves insertion order)
    let mut impl_indices: Vec<usize> = Vec::new();
    let mut impl_records: Vec<SchImplementation> = Vec::new();
    let mut mdl_entries: Vec<(usize, usize)> = Vec::new(); // (mdl_idx, owner_impl_idx)
    let mut md_entries: Vec<(usize, usize)> = Vec::new(); // (md_idx, owner_mdl_idx)

    // First pass: identify implementation hierarchy
    for (idx, record) in comp.primitives.iter().enumerate() {
        match record {
            SchRecord::Implementation(impl_rec) => {
                impl_indices.push(idx);
                impl_records.push(impl_rec.clone());
            }
            SchRecord::MapDefinerList(mdl) => {
                mdl_entries.push((idx, mdl.base.owner_index as usize));
            }
            SchRecord::MapDefiner(md) => {
                md_entries.push((idx, md.base.owner_index as usize));
            }
            _ => {}
        }
    }

    // Second pass: convert primitives
    for (_idx, record) in comp.primitives.iter().enumerate() {
        match record {
            SchRecord::Component(_) => {
                // Metadata captured at top level
            }
            SchRecord::Pin(pin) => {
                pins.push(SchPinJson {
                    designator: pin.designator.clone(),
                    name: pin.name.clone(),
                    x: raw_to_coord_value(pin.graphical.location_x),
                    y: raw_to_coord_value(pin.graphical.location_y),
                    length: raw_to_coord_value(pin.pin_length),
                    electrical: electrical_type_to_json(&pin.electrical),
                    orientation: pin_orientation_to_string(pin.pin_conglomerate),
                    hidden: pin.pin_conglomerate.contains(PinConglomerateFlags::HIDE),
                    description: pin.description.clone(),
                });
            }
            SchRecord::Rectangle(rect) => {
                rectangles.push(SchRectangleJson {
                    x1: raw_to_coord_value(rect.graphical.location_x),
                    y1: raw_to_coord_value(rect.graphical.location_y),
                    x2: raw_to_coord_value(rect.corner_x),
                    y2: raw_to_coord_value(rect.corner_y),
                    filled: rect.is_solid,
                    fill_color: color_to_hex(rect.graphical.area_color),
                    border_color: color_to_hex(rect.graphical.color),
                });
            }
            SchRecord::Line(line) => {
                lines.push(SchLineJson {
                    x1: raw_to_coord_value(line.graphical.location_x),
                    y1: raw_to_coord_value(line.graphical.location_y),
                    x2: raw_to_coord_value(line.corner_x),
                    y2: raw_to_coord_value(line.corner_y),
                    color: color_to_hex(line.graphical.color),
                });
            }
            SchRecord::Polygon(poly) => {
                polygons.push(SchPolygonJson {
                    vertices: poly.vertices.iter().map(|(x, y)| {
                        [raw_to_coord_value(*x), raw_to_coord_value(*y)]
                    }).collect(),
                    filled: poly.is_solid,
                    fill_color: color_to_hex(poly.graphical.area_color),
                    border_color: color_to_hex(poly.graphical.color),
                });
            }
            SchRecord::Label(label) => {
                labels.push(SchLabelJson {
                    x: raw_to_coord_value(label.graphical.location_x),
                    y: raw_to_coord_value(label.graphical.location_y),
                    text: label.text.clone(),
                    orientation: text_orientation_to_string(label.orientation),
                    justification: text_justification_to_string(label.justification),
                    color: color_to_hex(label.graphical.color),
                    font_id: label.font_id,
                    hidden: label.is_hidden,
                });
            }
            SchRecord::Arc(arc) => {
                arcs.push(SchArcJson {
                    x: raw_to_coord_value(arc.graphical.location_x),
                    y: raw_to_coord_value(arc.graphical.location_y),
                    radius: raw_to_coord_value(arc.radius),
                    start_angle: arc.start_angle,
                    end_angle: arc.end_angle,
                    color: color_to_hex(arc.graphical.color),
                });
            }
            SchRecord::Polyline(polyline) => {
                polylines.push(SchPolylineJson {
                    vertices: polyline.vertices.iter().map(|(x, y)| {
                        [raw_to_coord_value(*x), raw_to_coord_value(*y)]
                    }).collect(),
                    color: color_to_hex(polyline.graphical.color),
                });
            }
            SchRecord::Ellipse(ellipse) => {
                ellipses.push(SchEllipseJson {
                    x: raw_to_coord_value(ellipse.graphical.location_x),
                    y: raw_to_coord_value(ellipse.graphical.location_y),
                    radius_x: raw_to_coord_value(ellipse.radius_x),
                    radius_y: raw_to_coord_value(ellipse.radius_y),
                    filled: ellipse.is_solid,
                    fill_color: color_to_hex(ellipse.graphical.area_color),
                    border_color: color_to_hex(ellipse.graphical.color),
                });
            }
            SchRecord::Designator(des) => {
                designator = Some(SchDesignatorJson {
                    text: des.param.label.text.clone(),
                    x: raw_to_coord_value(des.param.label.graphical.location_x),
                    y: raw_to_coord_value(des.param.label.graphical.location_y),
                    font_id: des.param.label.font_id,
                    hidden: des.param.label.is_hidden,
                });
            }
            SchRecord::Parameter(param) => {
                parameters.push(SchParameterJson {
                    name: param.name.clone(),
                    value: param.label.text.clone(),
                    x: raw_to_coord_value(param.label.graphical.location_x),
                    y: raw_to_coord_value(param.label.graphical.location_y),
                    font_id: param.label.font_id,
                    hidden: param.label.is_hidden,
                    read_only_state: param.read_only_state,
                    orientation: text_orientation_to_string(param.label.orientation),
                });
            }
            SchRecord::Bezier(bezier) => {
                beziers.push(SchBezierJson {
                    vertices: bezier.vertices.iter().map(|(x, y)| {
                        [raw_to_coord_value(*x), raw_to_coord_value(*y)]
                    }).collect(),
                    color: color_to_hex(bezier.graphical.color),
                });
            }
            SchRecord::EllipticalArc(earc) => {
                elliptical_arcs.push(SchEllipticalArcJson {
                    x: raw_to_coord_value(earc.graphical.location_x),
                    y: raw_to_coord_value(earc.graphical.location_y),
                    radius: raw_to_coord_value(earc.radius),
                    secondary_radius: raw_to_coord_value(earc.secondary_radius),
                    start_angle: earc.start_angle,
                    end_angle: earc.end_angle,
                    color: color_to_hex(earc.graphical.color),
                });
            }
            _ => {}
        }
    }

    // Third pass: build implementations with pin mappings (in order)
    for (i, impl_rec) in impl_records.iter().enumerate() {
        let impl_idx = impl_indices[i];
        let mut pin_mappings = Vec::new();

        // Find associated MapDefinerList(s) owned by this implementation
        for &(mdl_idx, mdl_owner) in &mdl_entries {
            if mdl_owner == impl_idx {
                // Find MapDefiner records owned by this MapDefinerList
                for &(md_idx, md_owner) in &md_entries {
                    if md_owner == mdl_idx {
                        if let SchRecord::MapDefiner(md) = &comp.primitives[md_idx] {
                            pin_mappings.push(SchPinMappingJson {
                                schematic_pin: md.designator_interface.clone(),
                                implementation_pins: md.designator_implementation.clone(),
                                is_trivial: md.is_trivial,
                            });
                        }
                    }
                }
            }
        }

        implementations.push(SchImplementationJson {
            model_name: impl_rec.model_name.clone(),
            model_type: impl_rec.model_type.clone(),
            description: impl_rec.description.clone(),
            is_current: impl_rec.is_current,
            data_files: impl_rec.data_files.clone(),
            data_file_entities: impl_rec.data_file_entities.clone(),
            pin_mappings,
        });
    }

    SchComponentJson {
        name: comp.component.lib_reference.clone(),
        description: comp.component.component_description.clone(),
        part_count: comp.component.part_count,
        pins,
        rectangles,
        lines,
        polygons,
        labels,
        arcs,
        polylines,
        ellipses,
        beziers,
        elliptical_arcs,
        designator,
        parameters,
        implementations,
        display_mode_count: if comp.component.display_mode_count != 1 {
            Some(comp.component.display_mode_count)
        } else {
            None
        },
        unique_id: comp.component.unique_id.clone(),
    }
}

/// Add a complete component from JSON input.
pub fn cmd_add_json(
    path: &Path,
    json_file: Option<String>,
    json_str: Option<String>,
) -> Result<String, Box<dyn std::error::Error>> {
    use std::io::{self, Read as IoRead};

    // Read JSON from file, stdin, or command line
    let json_content = match (json_file, json_str) {
        (_, Some(s)) => s,
        (Some(ref path), None) if path == "-" => {
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)?;
            buffer
        }
        (Some(ref file_path), None) => std::fs::read_to_string(file_path)?,
        (None, None) => {
            return Err("Must provide either --file <file> or --json <string>".into());
        }
    };

    // Parse JSON - try bulk export first, then single component
    let parsed_export = serde_json::from_str::<SchLibExport>(&json_content).ok();
    let (is_bulk, component_defs): (bool, Vec<SchComponentJson>) =
        if let Some(ref export) = parsed_export {
            (true, export.components.clone())
        } else {
            (false, vec![serde_json::from_str::<SchComponentJson>(&json_content)?])
        };

    // Open or create library
    let mut lib = open_or_create_schlib(path)?;

    // For bulk import, remove default components from blank template
    // (the blank template has a "Component_1" placeholder)
    if is_bulk {
        let import_names: std::collections::HashSet<&str> =
            component_defs.iter().map(|c| c.name.as_str()).collect();
        lib.components.retain(|c| import_names.contains(c.component.lib_reference.as_str()));

        // Restore header metadata from export (replace template defaults)
        if let Some(ref export) = parsed_export {
            if !export.header.is_empty() {
                lib.header_params = crate::types::ParameterCollection::new();
                for (k, v) in &export.header {
                    lib.header_params.add(k, v);
                }
            }
        }
    }

    let mut added_components = Vec::new();

    for component_def in component_defs {
        // Check if component already exists
        if lib
            .components
            .iter()
            .any(|c| c.component.lib_reference == component_def.name)
        {
            return Err(format!("Component '{}' already exists", component_def.name).into());
        }

        // Create component record
        let component = SchComponent {
            lib_reference: component_def.name.clone(),
            component_description: component_def.description.clone(),
            part_count: component_def.part_count,
            display_mode_count: component_def.display_mode_count.unwrap_or(1),
            current_part_id: 1,
            unique_id: component_def.unique_id.clone(),
            ..Default::default()
        };

        // Build primitives in groups, then assemble in Altium-native order:
        // Component(1) → hidden Params(41) → Pins → drawing Primitives →
        // Designator(34) → Comment(41) → ImplementationList(44) →
        // [Implementation(45) → MapDefinerList(46) → MapDefiner(47)... → ImplParams(48)]* →
        // remaining Parameters(41)

        // Start with the component record as first primitive
        let mut primitives = vec![SchRecord::Component(component.clone())];

        // Group 1: Hidden parameters (those that are hidden and not Comment/Designator)
        let mut hidden_params = Vec::new();
        let mut comment_param = None;
        let mut visible_params = Vec::new();
        for param_json in &component_def.parameters {
            let orientation = parse_text_orientation(&param_json.orientation)?;
            let mut graphical = SchGraphicalBase::default();
            graphical.base.owner_part_id = Some(-1);
            graphical.color = 8388608; // Dark red (standard for parameters)
            graphical.location_x = param_json.x.to_raw();
            graphical.location_y = param_json.y.to_raw();

            let param = SchParameter {
                label: SchLabel {
                    text: param_json.value.clone(),
                    font_id: param_json.font_id,
                    is_hidden: param_json.hidden,
                    orientation,
                    graphical,
                    ..Default::default()
                },
                name: param_json.name.clone(),
                read_only_state: param_json.read_only_state,
                ..Default::default()
            };
            if param_json.name == "Comment" {
                comment_param = Some(SchRecord::Parameter(param));
            } else if param_json.hidden {
                hidden_params.push(SchRecord::Parameter(param));
            } else {
                visible_params.push(SchRecord::Parameter(param));
            }
        }

        // Add hidden parameters first (after Component)
        primitives.extend(hidden_params);

        // Group 2: Pins
        for pin_def in &component_def.pins {
            let electrical_type = parse_electrical_type(&pin_def.electrical)?;
            let mut conglomerate = parse_pin_orientation(&pin_def.orientation)?;

            conglomerate |= PinConglomerateFlags::DISPLAY_NAME_VISIBLE;
            conglomerate |= PinConglomerateFlags::DESIGNATOR_VISIBLE;

            if pin_def.hidden {
                conglomerate |= PinConglomerateFlags::HIDE;
            }

            let mut graphical = SchGraphicalBase::default();
            graphical.base.owner_part_id = Some(1);
            graphical.location_x = pin_def.x.to_raw();
            graphical.location_y = pin_def.y.to_raw();
            let pin = SchPin {
                graphical,
                designator: pin_def.designator.clone(),
                name: pin_def.name.clone(),
                electrical: electrical_type,
                pin_conglomerate: conglomerate,
                pin_length: pin_def.length.to_raw(),
                description: pin_def.description.clone(),
                symbol_inner_edge: PinSymbol::None,
                symbol_outer_edge: PinSymbol::None,
                symbol_inside: PinSymbol::None,
                symbol_outside: PinSymbol::None,
                ..Default::default()
            };
            primitives.push(SchRecord::Pin(pin));
        }

        // Group 3: Drawing primitives (rectangles, lines, polygons, arcs, polylines, beziers, elliptical arcs, ellipses, labels)
        for rect in &component_def.rectangles {
            let fill_color_val = parse_color(&rect.fill_color)?;
            let border_color_val = parse_color(&rect.border_color)?;

            let mut graphical = SchGraphicalBase::default();
            graphical.base.owner_part_id = Some(1);
            graphical.location_x = rect.x1.to_raw();
            graphical.location_y = rect.y1.to_raw();
            graphical.color = border_color_val;
            graphical.area_color = fill_color_val;

            let sch_rect = SchRectangle {
                graphical,
                corner_x: rect.x2.to_raw(),
                corner_y: rect.y2.to_raw(),
                line_width: LineWidth::Small,
                is_solid: rect.filled,
                transparent: !rect.filled,
                ..Default::default()
            };
            primitives.push(SchRecord::Rectangle(sch_rect));
        }

        // Add lines
        for line in &component_def.lines {
            let color_val = parse_color(&line.color)?;

            let mut graphical = SchGraphicalBase::default();
            graphical.base.owner_part_id = Some(1);
            graphical.location_x = line.x1.to_raw();
            graphical.location_y = line.y1.to_raw();
            graphical.color = color_val;

            let sch_line = SchLine {
                graphical,
                corner_x: line.x2.to_raw(),
                corner_y: line.y2.to_raw(),
                line_width: LineWidth::Small,
                ..Default::default()
        };
            primitives.push(SchRecord::Line(sch_line));
    }

        // Add polygons
        for polygon in &component_def.polygons {
            if polygon.vertices.len() < 3 {
                return Err("Polygon must have at least 3 vertices".into());
        }

            let fill_color_val = parse_color(&polygon.fill_color)?;
            let border_color_val = parse_color(&polygon.border_color)?;

            let vertices: Vec<(i32, i32)> = polygon
                .vertices
                .iter()
                .map(|v| (v[0].to_raw(), v[1].to_raw()))
                .collect();

            let mut graphical = SchGraphicalBase::default();
            graphical.base.owner_part_id = Some(1);
            graphical.location_x = vertices[0].0;
            graphical.location_y = vertices[0].1;
            graphical.color = border_color_val;
            graphical.area_color = fill_color_val;

            let sch_polygon = SchPolygon {
                graphical,
                vertices,
                line_width: LineWidth::Small,
                is_solid: polygon.filled,
                transparent: !polygon.filled,
                ..Default::default()
        };
            primitives.push(SchRecord::Polygon(sch_polygon));
    }

        // Add labels (text)
        for label in &component_def.labels {
            let color_val = parse_color(&label.color)?;
            let orientation = parse_text_orientation(&label.orientation)?;
            let justification = parse_text_justification(&label.justification)?;

            let mut graphical = SchGraphicalBase::default();
            graphical.base.owner_part_id = Some(1);
            graphical.location_x = label.x.to_raw();
            graphical.location_y = label.y.to_raw();
            graphical.color = color_val;

            let sch_label = SchLabel {
                graphical,
                text: label.text.clone(),
                orientation,
                justification,
                font_id: label.font_id,
                is_hidden: label.hidden,
                ..Default::default()
        };
            primitives.push(SchRecord::Label(sch_label));
    }

        // Add arcs
        for arc in &component_def.arcs {
            let color_val = parse_color(&arc.color)?;

            let mut graphical = SchGraphicalBase::default();
            graphical.base.owner_part_id = Some(1);
            graphical.location_x = arc.x.to_raw();
            graphical.location_y = arc.y.to_raw();
            graphical.color = color_val;

            let sch_arc = SchArc {
                graphical,
                radius: arc.radius.to_raw(),
                secondary_radius: arc.radius.to_raw(), // Same as radius for circular arcs
                start_angle: arc.start_angle,
                end_angle: arc.end_angle,
                line_width: LineWidth::Small,
                ..Default::default()
        };
            primitives.push(SchRecord::Arc(sch_arc));
    }

        // Add polylines
        for polyline in &component_def.polylines {
            if polyline.vertices.len() < 2 {
                return Err("Polyline must have at least 2 vertices".into());
        }

            let color_val = parse_color(&polyline.color)?;

            let vertices: Vec<(i32, i32)> = polyline
                .vertices
                .iter()
                .map(|v| (v[0].to_raw(), v[1].to_raw()))
                .collect();

            let mut graphical = SchGraphicalBase::default();
            graphical.base.owner_part_id = Some(1);
            graphical.location_x = vertices[0].0;
            graphical.location_y = vertices[0].1;
            graphical.color = color_val;

            let sch_polyline = SchPolyline {
                graphical,
                vertices,
                line_width: LineWidth::Small,
                ..Default::default()
        };
            primitives.push(SchRecord::Polyline(sch_polyline));
    }

        // Add ellipses
        for ellipse in &component_def.ellipses {
            let fill_color_val = parse_color(&ellipse.fill_color)?;
            let border_color_val = parse_color(&ellipse.border_color)?;

            let mut graphical = SchGraphicalBase::default();
            graphical.base.owner_part_id = Some(1);
            graphical.location_x = ellipse.x.to_raw();
            graphical.location_y = ellipse.y.to_raw();
            graphical.color = border_color_val;
            graphical.area_color = fill_color_val;

            let sch_ellipse = SchEllipse {
                graphical,
                radius_x: ellipse.radius_x.to_raw(),
                radius_y: ellipse.radius_y.to_raw(),
                is_solid: ellipse.filled,
                transparent: !ellipse.filled,
                line_width: LineWidth::Small,
                ..Default::default()
        };
            primitives.push(SchRecord::Ellipse(sch_ellipse));
    }

        // Add bezier curves
        for bezier in &component_def.beziers {
            if bezier.vertices.len() < 4 {
                return Err("Bezier must have at least 4 control points".into());
            }

            let color_val = parse_color(&bezier.color)?;

            let vertices: Vec<(i32, i32)> = bezier
                .vertices
                .iter()
                .map(|v| (v[0].to_raw(), v[1].to_raw()))
                .collect();

            let mut graphical = SchGraphicalBase::default();
            graphical.base.owner_part_id = Some(1);
            graphical.location_x = vertices[0].0;
            graphical.location_y = vertices[0].1;
            graphical.color = color_val;

            let sch_bezier = SchBezier {
                graphical,
                vertices,
                ..Default::default()
            };
            primitives.push(SchRecord::Bezier(sch_bezier));
        }

        // Add elliptical arcs
        for earc in &component_def.elliptical_arcs {
            let color_val = parse_color(&earc.color)?;

            let mut graphical = SchGraphicalBase::default();
            graphical.base.owner_part_id = Some(1);
            graphical.location_x = earc.x.to_raw();
            graphical.location_y = earc.y.to_raw();
            graphical.color = color_val;

            let sch_earc = SchEllipticalArc {
                graphical,
                radius: earc.radius.to_raw(),
                secondary_radius: earc.secondary_radius.to_raw(),
                start_angle: earc.start_angle,
                end_angle: earc.end_angle,
                ..Default::default()
            };
            primitives.push(SchRecord::EllipticalArc(sch_earc));
        }

        // Group 4: Designator (RECORD=34)
        if let Some(des_json) = &component_def.designator {
            let mut graphical = SchGraphicalBase::default();
            graphical.base.owner_part_id = Some(-1);
            graphical.color = 8388608; // Dark red (standard for designators)
            graphical.location_x = des_json.x.to_raw();
            graphical.location_y = des_json.y.to_raw();

            let designator = SchDesignator {
                param: SchParameter {
                    label: SchLabel {
                        text: des_json.text.clone(),
                        font_id: des_json.font_id,
                        is_hidden: des_json.hidden,
                        graphical,
                        ..Default::default()
                    },
                    name: "Designator".to_string(),
                    read_only_state: 1,
                    ..Default::default()
                },
                ..Default::default()
            };
            primitives.push(SchRecord::Designator(designator));
        } else {
            let designator_text = infer_designator_prefix(&component_def.name, &component_def.description);
            let mut graphical = SchGraphicalBase::default();
            graphical.base.owner_part_id = Some(-1);
            graphical.color = 8388608;
            let designator = SchDesignator {
                param: SchParameter {
                    label: SchLabel {
                        text: designator_text,
                        font_id: 1,
                        graphical,
                        ..Default::default()
                    },
                    name: "Designator".to_string(),
                    read_only_state: 1,
                    ..Default::default()
                },
                ..Default::default()
            };
            primitives.push(SchRecord::Designator(designator));
        }

        // Group 5: Comment parameter (if present)
        if let Some(comment) = comment_param {
            primitives.push(comment);
        }

        // Group 6: Implementation hierarchy
        let impl_list_idx = primitives.len();
        let impl_list = SchImplementationList {
            base: SchPrimitiveBase {
                owner_index: 0,
                ..Default::default()
            },
            ..Default::default()
        };
        primitives.push(SchRecord::ImplementationList(impl_list));

        for impl_json in &component_def.implementations {
            let impl_idx = primitives.len();
            let implementation = SchImplementation {
                base: SchPrimitiveBase {
                    owner_index: impl_list_idx as i32,
                    ..Default::default()
                },
                description: impl_json.description.clone(),
                model_name: impl_json.model_name.clone(),
                model_type: impl_json.model_type.clone(),
                is_current: impl_json.is_current,
                data_files: impl_json.data_files.clone(),
                data_file_entities: impl_json.data_file_entities.clone(),
                ..Default::default()
            };
            primitives.push(SchRecord::Implementation(implementation));

            // MapDefinerList (always present)
            let mdl_idx = primitives.len();
            let map_definer_list = SchMapDefinerList {
                base: SchPrimitiveBase {
                    owner_index: impl_idx as i32,
                    ..Default::default()
                },
                ..Default::default()
            };
            primitives.push(SchRecord::MapDefinerList(map_definer_list));

            // MapDefiner records
            for mapping_json in &impl_json.pin_mappings {
                let map_definer = SchMapDefiner {
                    base: SchPrimitiveBase {
                        owner_index: mdl_idx as i32,
                        ..Default::default()
                    },
                    designator_interface: mapping_json.schematic_pin.clone(),
                    designator_implementation: mapping_json.implementation_pins.clone(),
                    is_trivial: mapping_json.is_trivial,
                    ..Default::default()
                };
                primitives.push(SchRecord::MapDefiner(map_definer));
            }

            // ImplementationParameters (always present, owned by Implementation)
            let impl_params = SchImplementationParameters {
                base: SchPrimitiveBase {
                    owner_index: impl_idx as i32,
                    ..Default::default()
                },
                ..Default::default()
            };
            primitives.push(SchRecord::ImplementationParameters(impl_params));
        }

        // Group 7: Remaining visible parameters
        primitives.extend(visible_params);

        let lib_component = SchLibComponent {
            component,
            primitives,
        };

        lib.components.push(lib_component);
        added_components.push(component_def.name.clone());
    }

    save_schlib(path, &lib)?;

    if added_components.len() == 1 {
        Ok(format!(
            "Added component '{}' to {}",
            added_components[0],
            path.display()
        ))
    } else {
        Ok(format!(
            "Added {} components to {}",
            added_components.len(),
            path.display()
        ))
    }
}

/// Parse text orientation string.
fn parse_text_orientation(s: &str) -> Result<TextOrientations, Box<dyn std::error::Error>> {
    match s.to_lowercase().as_str() {
        "horizontal" | "0" => Ok(TextOrientations::NONE),
        "vertical_up" | "90" | "up" => Ok(TextOrientations::ROTATED),
        "vertical_down" | "270" | "down" => Ok(TextOrientations::ROTATED | TextOrientations::FLIPPED),
        "180" | "flipped" => Ok(TextOrientations::FLIPPED),
        _ => Err(format!("Unknown text orientation: {}. Use: horizontal, vertical_up, vertical_down, 90, 180, 270", s).into()),
    }
}

/// Parse text justification string.
fn parse_text_justification(s: &str) -> Result<TextJustification, Box<dyn std::error::Error>> {
    match s.to_lowercase().replace('_', "").as_str() {
        "bottomleft" | "bl" => Ok(TextJustification::BOTTOM_LEFT),
        "bottomcenter" | "bc" => Ok(TextJustification::BOTTOM_CENTER),
        "bottomright" | "br" => Ok(TextJustification::BOTTOM_RIGHT),
        "centerleft" | "cl" | "middleleft" | "ml" => Ok(TextJustification::MIDDLE_LEFT),
        "center" | "c" | "middle" | "m" => Ok(TextJustification::MIDDLE_CENTER),
        "centerright" | "cr" | "middleright" | "mr" => Ok(TextJustification::MIDDLE_RIGHT),
        "topleft" | "tl" => Ok(TextJustification::TOP_LEFT),
        "topcenter" | "tc" => Ok(TextJustification::TOP_CENTER),
        "topright" | "tr" => Ok(TextJustification::TOP_RIGHT),
        _ => Err(format!("Unknown justification: {}. Use: bottom_left, bottom_center, bottom_right, center_left, center, center_right, top_left, top_center, top_right", s).into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_schlib() -> PathBuf {
        let id = uuid::Uuid::new_v4();
        std::env::temp_dir().join(format!("test_{}.SchLib", id))
    }

    /// Regression: gen-ic must place pins on all four sides (left, right, top, bottom).
    /// Previously, top and bottom pins were silently dropped.
    #[test]
    fn test_gen_ic_all_four_sides() {
        let path = temp_schlib();
        cmd_create(&path).unwrap();

        let result = cmd_gen_ic(
            &path,
            "TEST_4SIDE",
            "1:VCC:power:top,2:GND:power:bottom,3:IN:input:left,4:OUT:output:right",
            Some("4-side test".to_string()),
            "600mil",
            "200mil",
            "100mil",
        )
        .unwrap();

        assert!(
            result.contains("4 pins"),
            "Expected 4 pins, got: {}",
            result
        );
        assert!(result.contains("1 left"), "Expected 1 left pin: {}", result);
        assert!(
            result.contains("1 right"),
            "Expected 1 right pin: {}",
            result
        );
        assert!(result.contains("1 top"), "Expected 1 top pin: {}", result);
        assert!(
            result.contains("1 bottom"),
            "Expected 1 bottom pin: {}",
            result
        );

        // Verify all 4 pins are in the library
        let lib = open_or_create_schlib(&path).unwrap();
        let comp = lib
            .components
            .iter()
            .find(|c| c.component.lib_reference == "TEST_4SIDE")
            .expect("Component must exist");

        let pin_count = comp
            .primitives
            .iter()
            .filter(|r| matches!(r, SchRecord::Pin(_)))
            .count();
        assert_eq!(pin_count, 4, "All 4 pins must be saved, got {}", pin_count);

        std::fs::remove_file(&path).ok();
    }

    /// Regression: gen-ic with only top/bottom pins must not produce 0-pin component.
    #[test]
    fn test_gen_ic_only_top_bottom() {
        let path = temp_schlib();
        cmd_create(&path).unwrap();

        let result = cmd_gen_ic(
            &path,
            "PWR_2PIN",
            "1:VCC:power:top,2:GND:power:bottom",
            None,
            "400mil",
            "200mil",
            "100mil",
        )
        .unwrap();

        assert!(
            result.contains("2 pins"),
            "Expected 2 pins, got: {}",
            result
        );
        assert!(result.contains("1 top"), "Expected 1 top: {}", result);
        assert!(result.contains("1 bottom"), "Expected 1 bottom: {}", result);

        std::fs::remove_file(&path).ok();
    }

    /// gen-ic with many pins per side places all of them.
    #[test]
    fn test_gen_ic_multi_pin_sides() {
        let path = temp_schlib();
        cmd_create(&path).unwrap();

        // 3 left, 4 right, 2 top, 2 bottom = 11 pins
        let pins = [
            "1:A:io:left",
            "2:B:io:left",
            "3:C:io:left",
            "4:D:io:right",
            "5:E:io:right",
            "6:F:io:right",
            "7:G:io:right",
            "8:VCC:power:top",
            "9:VCC2:power:top",
            "10:GND:power:bottom",
            "11:GND2:power:bottom",
        ]
        .join(",");

        let result = cmd_gen_ic(
            &path,
            "MULTI_PIN",
            &pins,
            None,
            "800mil",
            "200mil",
            "100mil",
        )
        .unwrap();

        assert!(
            result.contains("11 pins"),
            "Expected 11 pins, got: {}",
            result
        );
        assert!(result.contains("3 left"), "Got: {}", result);
        assert!(result.contains("4 right"), "Got: {}", result);
        assert!(result.contains("2 top"), "Got: {}", result);
        assert!(result.contains("2 bottom"), "Got: {}", result);

        // Verify pin count in library
        let lib = open_or_create_schlib(&path).unwrap();
        let comp = lib
            .components
            .iter()
            .find(|c| c.component.lib_reference == "MULTI_PIN")
            .unwrap();
        let pin_count = comp
            .primitives
            .iter()
            .filter(|r| matches!(r, SchRecord::Pin(_)))
            .count();
        assert_eq!(pin_count, 11);

        std::fs::remove_file(&path).ok();
    }

    /// gen-ic body width auto-expands when top/bottom pins need more space.
    #[test]
    fn test_gen_ic_body_widens_for_top_bottom() {
        let path = temp_schlib();
        cmd_create(&path).unwrap();

        // 5 top pins at 100mil spacing need 600mil. Requested width is 400mil.
        // Body should expand to accommodate.
        let pins = "1:A:io:top,2:B:io:top,3:C:io:top,4:D:io:top,5:E:io:top,6:IN:input:left";

        cmd_gen_ic(
            &path, "WIDE_TOP", pins, None, "400mil", // narrower than needed
            "200mil", "100mil",
        )
        .unwrap();

        let lib = open_or_create_schlib(&path).unwrap();
        let comp = lib
            .components
            .iter()
            .find(|c| c.component.lib_reference == "WIDE_TOP")
            .unwrap();

        // Find the rectangle (body)
        let rect = comp
            .primitives
            .iter()
            .find_map(|r| {
                if let SchRecord::Rectangle(rect) = r {
                    Some(rect)
                } else {
                    None
                }
            })
            .expect("Must have body rectangle");

        // Body width = corner_x (origin is 0). Must be >= 600mil (5+1 * 100mil spacing)
        let body_width_mils = rect.corner_x as f64 / 10000.0;
        assert!(
            body_width_mils >= 600.0,
            "Body width should expand to at least 600mil for 5 top pins, got {}mil",
            body_width_mils
        );

        std::fs::remove_file(&path).ok();
    }

    /// Pin electrical type parsing rejects invalid values with clear error.
    #[test]
    fn test_gen_ic_invalid_pin_type() {
        let path = temp_schlib();
        cmd_create(&path).unwrap();

        let result = cmd_gen_ic(
            &path,
            "BAD",
            "1:VCC:W:left", // "W" is not valid, must be "power"
            None,
            "400mil",
            "200mil",
            "100mil",
        );

        assert!(result.is_err(), "Invalid pin type should fail");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Unknown electrical type"),
            "Error should mention electrical type, got: {}",
            err
        );

        std::fs::remove_file(&path).ok();
    }

    /// Bug 7 & 8: Components must have Designator and ImplementationList records.
    #[test]
    fn test_component_has_required_records() {
        let path = temp_schlib();
        cmd_create(&path).unwrap();

        // Test add-component
        cmd_add_component(&path, "TestCap", Some("100nF Capacitor".to_string())).unwrap();

        let lib = open_or_create_schlib(&path).unwrap();
        let comp = lib
            .components
            .iter()
            .find(|c| c.component.lib_reference == "TestCap")
            .expect("Component must exist");

        // Verify Designator record exists
        let has_designator = comp
            .primitives
            .iter()
            .any(|r| matches!(r, SchRecord::Designator(_)));
        assert!(
            has_designator,
            "Component must have Designator record (Bug 7)"
        );

        // Verify ImplementationList record exists
        let has_impl_list = comp
            .primitives
            .iter()
            .any(|r| matches!(r, SchRecord::ImplementationList(_)));
        assert!(
            has_impl_list,
            "Component must have ImplementationList record (Bug 8)"
        );

        // Verify designator text is correct (should be C? for capacitor)
        let designator = comp.primitives.iter().find_map(|r| {
            if let SchRecord::Designator(d) = r {
                Some(d)
            } else {
                None
            }
        });
        assert_eq!(
            designator.unwrap().param.label.text,
            "C?",
            "Capacitor should have C? designator"
        );

        std::fs::remove_file(&path).ok();
    }

    /// Bug 7 & 8: gen-ic components must have Designator and ImplementationList records.
    #[test]
    fn test_gen_ic_has_required_records() {
        let path = temp_schlib();
        cmd_create(&path).unwrap();

        cmd_gen_ic(
            &path,
            "TestIC",
            "1:VCC:power:left,2:GND:power:right",
            Some("Test integrated circuit".to_string()),
            "400mil",
            "200mil",
            "100mil",
        )
        .unwrap();

        let lib = open_or_create_schlib(&path).unwrap();
        let comp = lib
            .components
            .iter()
            .find(|c| c.component.lib_reference == "TestIC")
            .expect("Component must exist");

        // Verify Designator record exists
        let has_designator = comp
            .primitives
            .iter()
            .any(|r| matches!(r, SchRecord::Designator(_)));
        assert!(has_designator, "IC must have Designator record (Bug 7)");

        // Verify ImplementationList record exists
        let has_impl_list = comp
            .primitives
            .iter()
            .any(|r| matches!(r, SchRecord::ImplementationList(_)));
        assert!(
            has_impl_list,
            "IC must have ImplementationList record (Bug 8)"
        );

        // Verify designator text is U? (default)
        let designator = comp.primitives.iter().find_map(|r| {
            if let SchRecord::Designator(d) = r {
                Some(d)
            } else {
                None
            }
        });
        assert_eq!(
            designator.unwrap().param.label.text,
            "U?",
            "IC should have U? designator"
        );

        std::fs::remove_file(&path).ok();
    }

    /// Test designator prefix inference logic.
    #[test]
    fn test_designator_prefix_inference() {
        assert_eq!(infer_designator_prefix("CAP100", "100nF Capacitor"), "C?");
        assert_eq!(infer_designator_prefix("R1", "1k Resistor"), "R?");
        assert_eq!(infer_designator_prefix("LED1", "Red LED"), "LED?");
        assert_eq!(infer_designator_prefix("L1", "10uH Inductor"), "L?");
        assert_eq!(infer_designator_prefix("D1", "Schottky Diode"), "D?");
        assert_eq!(
            infer_designator_prefix("USB1", "USB Type-C Connector"),
            "J?"
        );
        assert_eq!(infer_designator_prefix("K1", "5V Relay"), "K?");
        assert_eq!(infer_designator_prefix("Q1", "N-Channel MOSFET"), "Q?");
        assert_eq!(infer_designator_prefix("U1", "Microcontroller"), "U?");
        assert_eq!(infer_designator_prefix("CONN1", "JST connector"), "J?");
    }

    /// Bug 7 & 8: add-json components must have Designator and ImplementationList records.
    #[test]
    fn test_add_json_has_required_records() {
        let path = temp_schlib();
        cmd_create(&path).unwrap();

        let json = r#"{
            "name": "RES1",
            "description": "1k Resistor",
            "pins": [
                {"designator": "1", "name": "1", "x": 0, "y": 0, "electrical": "passive"},
                {"designator": "2", "name": "2", "x": 400, "y": 0, "electrical": "passive"}
            ],
            "rectangles": [
                {"x1": 100, "y1": -50, "x2": 300, "y2": 50, "filled": true}
            ]
        }"#;

        cmd_add_json(&path, None, Some(json.to_string())).unwrap();

        let lib = open_or_create_schlib(&path).unwrap();
        let comp = lib
            .components
            .iter()
            .find(|c| c.component.lib_reference == "RES1")
            .expect("Component must exist");

        // Verify Designator record exists
        let has_designator = comp
            .primitives
            .iter()
            .any(|r| matches!(r, SchRecord::Designator(_)));
        assert!(
            has_designator,
            "JSON component must have Designator record (Bug 7)"
        );

        // Verify ImplementationList record exists
        let has_impl_list = comp
            .primitives
            .iter()
            .any(|r| matches!(r, SchRecord::ImplementationList(_)));
        assert!(
            has_impl_list,
            "JSON component must have ImplementationList record (Bug 8)"
        );

        // Verify designator text is R? (resistor)
        let designator = comp.primitives.iter().find_map(|r| {
            if let SchRecord::Designator(d) = r {
                Some(d)
            } else {
                None
            }
        });
        assert_eq!(
            designator.unwrap().param.label.text,
            "R?",
            "Resistor should have R? designator"
        );

        std::fs::remove_file(&path).ok();
    }

    /// Test that MapDefinerList and ImplementationParameters records survive save/reload.
    #[test]
    fn test_implementation_child_records_persist_to_file() {
        let path = std::env::temp_dir().join(format!("test_impl_persist_{}.SchLib", uuid::Uuid::new_v4()));

        // Create library and add component with implementations
        cmd_create(&path).unwrap();

        let json = r#"{
            "name": "TestImpl",
            "pins": [
                {"designator": "1", "name": "VCC", "x": 0, "y": 0, "length": 10, "electrical": "passive"}
            ],
            "rectangles": [{"x1": -10, "y1": -10, "x2": 10, "y2": 10}],
            "implementations": [
                {
                    "model_name": "SOIC-8",
                    "model_type": "PCBLIB",
                    "description": "Test footprint",
                    "is_current": true
                }
            ]
        }"#;

        cmd_add_json(&path, None, Some(json.to_string())).unwrap();

        // Re-read from file (this forces reading from the saved binary)
        let lib = open_schlib(&path).unwrap();
        let comp = lib.components.iter()
            .find(|c| c.component.lib_reference == "TestImpl")
            .expect("TestImpl not found");

        let record_types: Vec<&str> = comp.primitives.iter()
            .map(|r| r.record_type_name())
            .collect();

        eprintln!("Persisted record types: {:?}", record_types);

        assert!(record_types.contains(&"ImplementationList"),
            "Missing ImplementationList after save/reload, got: {:?}", record_types);
        assert!(record_types.contains(&"Implementation"),
            "Missing Implementation after save/reload, got: {:?}", record_types);
        assert!(record_types.contains(&"MapDefinerList"),
            "Missing MapDefinerList after save/reload, got: {:?}", record_types);
        assert!(record_types.contains(&"ImplementationParameters"),
            "Missing ImplementationParameters after save/reload, got: {:?}", record_types);

        std::fs::remove_file(&path).ok();
    }
}

