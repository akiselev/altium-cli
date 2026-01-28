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
    LineWidth, PinConglomerateFlags, PinElectricalType, PinSymbol, SchArc, SchComponent,
    SchEllipse, SchGraphicalBase, SchLabel, SchLine, SchPin, SchPolygon, SchPolyline, SchRecord,
    SchRectangle, TextJustification, TextOrientations,
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

/// Export as JSON - returns component list (let presentation layer handle JSON serialization).
pub fn cmd_json(path: &Path) -> Result<SchLibComponentList, Box<dyn std::error::Error>> {
    cmd_list(path)
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
pub fn open_or_create_schlib(path: &Path) -> Result<SchLib, Box<dyn std::error::Error>> {
    if path.exists() {
        open_schlib(path)
    } else {
        load_blank_schlib()
    }
}

/// Save a SchLib file.
pub fn save_schlib(path: &Path, lib: &SchLib) -> Result<(), Box<dyn std::error::Error>> {
    Ok(lib.save_to_file(path)?)
}

/// Parse a hex color string to Win32 COLORREF (BGR format).
pub fn parse_color(hex: &str) -> Result<i32, Box<dyn std::error::Error>> {
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
pub fn parse_electrical_type(s: &str) -> Result<PinElectricalType, Box<dyn std::error::Error>> {
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
pub fn parse_pin_orientation(s: &str) -> Result<PinConglomerateFlags, Box<dyn std::error::Error>> {
    match s.to_lowercase().as_str() {
        "right" => Ok(PinConglomerateFlags::empty()), // Pin pointing right (default)
        "left" => Ok(PinConglomerateFlags::FLIPPED),  // Pin pointing left
        "up" => Ok(PinConglomerateFlags::ROTATED),    // Pin pointing up
        "down" => Ok(PinConglomerateFlags::ROTATED | PinConglomerateFlags::FLIPPED), // Pin pointing down
        _ => Err(format!("Unknown orientation: {}. Use: left, right, up, down", s).into()),
    }
}

/// Convert mils to raw coordinate value.
pub fn mils_to_raw(mils: i32) -> i32 {
    mils * 10000
}

/// Convert mils (f64) to raw coordinate value.
pub fn mils_f64_to_raw(mils: f64) -> i32 {
    (mils * 10000.0).round() as i32
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
pub fn parse_unit_value_or_mil(s: &str) -> Result<f64, Box<dyn std::error::Error>> {
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
        component_description: description.unwrap_or_default(),
        part_count: 1,
        display_mode_count: 1,
        current_part_id: 1,
        ..Default::default()
    };

    // Create SchLibComponent with the component record as first primitive
    let lib_component = SchLibComponent {
        component: component.clone(),
        primitives: vec![SchRecord::Component(component)],
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
        component_description: description.unwrap_or_default(),
        part_count: 1,
        display_mode_count: 1,
        current_part_id: 1,
        ..Default::default()
    };

    let mut primitives = vec![SchRecord::Component(component.clone())];

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
}

fn default_part_count() -> i32 {
    1
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

    // Parse JSON
    let component_def: SchComponentJson = serde_json::from_str(&json_content)?;

    // Open or create library
    let mut lib = open_or_create_schlib(path)?;

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
        display_mode_count: 1,
        current_part_id: 1,
        ..Default::default()
    };

    // Start with the component record as first primitive
    let mut primitives = vec![SchRecord::Component(component.clone())];

    // Add rectangles
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

    // Add pins
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
        graphical.color = 0x000080;

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

    let pin_count = component_def.pins.len();
    let rect_count = component_def.rectangles.len();
    let line_count = component_def.lines.len();
    let polygon_count = component_def.polygons.len();
    let label_count = component_def.labels.len();
    let arc_count = component_def.arcs.len();
    let polyline_count = component_def.polylines.len();
    let ellipse_count = component_def.ellipses.len();

    let lib_component = SchLibComponent {
        component,
        primitives,
    };

    lib.components.push(lib_component);
    save_schlib(path, &lib)?;

    // Build summary of added primitives
    let mut parts = vec![format!("{} pins", pin_count)];
    if rect_count > 0 {
        parts.push(format!("{} rectangles", rect_count));
    }
    if line_count > 0 {
        parts.push(format!("{} lines", line_count));
    }
    if polygon_count > 0 {
        parts.push(format!("{} polygons", polygon_count));
    }
    if label_count > 0 {
        parts.push(format!("{} labels", label_count));
    }
    if arc_count > 0 {
        parts.push(format!("{} arcs", arc_count));
    }
    if polyline_count > 0 {
        parts.push(format!("{} polylines", polyline_count));
    }
    if ellipse_count > 0 {
        parts.push(format!("{} ellipses", ellipse_count));
    }

    Ok(format!(
        "Added component '{}' with {} to {}",
        component_def.name,
        parts.join(", "),
        path.display()
    ))
}

/// Parse text orientation string.
pub fn parse_text_orientation(s: &str) -> Result<TextOrientations, Box<dyn std::error::Error>> {
    match s.to_lowercase().as_str() {
        "horizontal" | "0" => Ok(TextOrientations::NONE),
        "vertical_up" | "90" | "up" => Ok(TextOrientations::ROTATED),
        "vertical_down" | "270" | "down" => Ok(TextOrientations::ROTATED | TextOrientations::FLIPPED),
        "180" | "flipped" => Ok(TextOrientations::FLIPPED),
        _ => Err(format!("Unknown text orientation: {}. Use: horizontal, vertical_up, vertical_down, 90, 180, 270", s).into()),
    }
}

/// Parse text justification string.
pub fn parse_text_justification(s: &str) -> Result<TextJustification, Box<dyn std::error::Error>> {
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

        assert!(result.contains("4 pins"), "Expected 4 pins, got: {}", result);
        assert!(result.contains("1 left"), "Expected 1 left pin: {}", result);
        assert!(result.contains("1 right"), "Expected 1 right pin: {}", result);
        assert!(result.contains("1 top"), "Expected 1 top pin: {}", result);
        assert!(result.contains("1 bottom"), "Expected 1 bottom pin: {}", result);

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

        assert!(result.contains("2 pins"), "Expected 2 pins, got: {}", result);
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
            "1:A:io:left", "2:B:io:left", "3:C:io:left",
            "4:D:io:right", "5:E:io:right", "6:F:io:right", "7:G:io:right",
            "8:VCC:power:top", "9:VCC2:power:top",
            "10:GND:power:bottom", "11:GND2:power:bottom",
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

        assert!(result.contains("11 pins"), "Expected 11 pins, got: {}", result);
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
            &path,
            "WIDE_TOP",
            pins,
            None,
            "400mil",  // narrower than needed
            "200mil",
            "100mil",
        )
        .unwrap();

        let lib = open_or_create_schlib(&path).unwrap();
        let comp = lib
            .components
            .iter()
            .find(|c| c.component.lib_reference == "WIDE_TOP")
            .unwrap();

        // Find the rectangle (body)
        let rect = comp.primitives.iter().find_map(|r| {
            if let SchRecord::Rectangle(rect) = r {
                Some(rect)
            } else {
                None
            }
        }).expect("Must have body rectangle");

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
            "1:VCC:W:left",  // "W" is not valid, must be "power"
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
}
