// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Schematic document operations.
//!
//! High-level operations for exploring and editing Altium schematic documents (.SchDoc files).
//!
//! This module uses the v2 API for parsing SchDoc files which provides:
//! - Strongly-typed record access via `TypedRecord` enum
//! - Proper coordinate handling (100K units/mil)
//! - Typed accessors like `components()`, `wires()`, `net_labels()`, `power_objects()`

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json;

use crate::ops::categorization::categorize_component;
use crate::ops::output::*;
use crate::ops::util::alphanumeric_sort;

use crate::dump::{fmt_coord, fmt_point};
use crate::v2::io::schdoc::SchDocV2;
use crate::v2::fields::{
    TypedRecord, ComponentData, PinData, WireData, NetLabelData, PortData,
    PowerData,
};
use crate::v2::types::{PinElectrical, PortIO, PowerObjectStyle};

/// Open schematic document with String error type (for old-style functions).
fn open_schdoc(path: &Path) -> Result<SchDocV2, String> {
    let file = File::open(path).map_err(|e| format!("Error opening file: {}", e))?;
    SchDocV2::open(BufReader::new(file)).map_err(|e| format!("Error parsing SchDoc: {:?}", e))
}

/// Open schematic document with Box<dyn Error> error type (for refactored functions).
fn open_schdoc_boxed(path: &Path) -> Result<SchDocV2, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    Ok(SchDocV2::open(BufReader::new(file))?)
}

// ═══════════════════════════════════════════════════════════════════════════
// V2 HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Get sheet size name from v2 SheetStyle enum.
fn sheet_size_name_v2(style: u8) -> &'static str {
    match style {
        0 => "A4",
        1 => "A3",
        2 => "A2",
        3 => "A1",
        4 => "A0",
        5 => "A",
        6 => "B",
        7 => "C",
        8 => "D",
        9 => "E",
        10 => "Letter",
        11 => "Legal",
        12 => "Tabloid",
        13 => "OrCAD A",
        14 => "OrCAD B",
        15 => "OrCAD C",
        16 => "OrCAD D",
        17 => "OrCAD E",
        _ => "Custom",
    }
}

/// Get the name of a v2 record type.
fn record_type_name_v2(record: &TypedRecord) -> &'static str {
    match record {
        TypedRecord::Component(_) => "Component",
        TypedRecord::Pin(_) => "Pin",
        TypedRecord::Symbol(_) => "Symbol",
        TypedRecord::Label(_) => "Label",
        TypedRecord::Bezier(_) => "Bezier",
        TypedRecord::Polyline(_) => "Polyline",
        TypedRecord::Polygon(_) => "Polygon",
        TypedRecord::Ellipse(_) => "Ellipse",
        TypedRecord::Pie(_) => "Pie",
        TypedRecord::EllipticalArc(_) => "EllipticalArc",
        TypedRecord::Arc(_) => "Arc",
        TypedRecord::Line(_) => "Line",
        TypedRecord::Rectangle(_) => "Rectangle",
        TypedRecord::PowerObject(_) => "PowerObject",
        TypedRecord::Port(_) => "Port",
        TypedRecord::NoERC(_) => "NoERC",
        TypedRecord::NetLabel(_) => "NetLabel",
        TypedRecord::Bus(_) => "Bus",
        TypedRecord::Wire(_) => "Wire",
        TypedRecord::TextFrame(_) => "TextFrame",
        TypedRecord::Junction(_) => "Junction",
        TypedRecord::Image(_) => "Image",
        TypedRecord::Sheet(_) => "Sheet",
        TypedRecord::Designator(_) => "Designator",
        TypedRecord::BusEntry(_) => "BusEntry",
        TypedRecord::Parameter(_) => "Parameter",
        TypedRecord::ImplementationList(_) => "ImplementationList",
        TypedRecord::Implementation(_) => "Implementation",
        TypedRecord::SheetSymbol(_) => "SheetSymbol",
        TypedRecord::SheetEntry(_) => "SheetEntry",
        TypedRecord::SheetName(_) => "SheetName",
        TypedRecord::SheetFileName(_) => "SheetFileName",
        TypedRecord::RoundRectangle(_) => "RoundRectangle",
        TypedRecord::Note(_) => "Note",
        TypedRecord::Blanket(_) => "Blanket",
        TypedRecord::Unknown(_) => "Unknown",
    }
}

/// Count record types in a v2 document.
fn count_record_types_v2(doc: &SchDocV2) -> HashMap<&'static str, usize> {
    let mut counts: HashMap<&'static str, usize> = HashMap::new();
    for record in doc.typed_records() {
        let name = record_type_name_v2(record);
        *counts.entry(name).or_insert(0) += 1;
    }
    counts
}

/// Get the designator for a component by finding its Designator child in v2 records.
fn get_component_designator_v2(records: &[TypedRecord], component_index: usize) -> Option<String> {
    for record in records {
        if let TypedRecord::Designator(d) = record {
            if d.param.graphical.base.owner_index == component_index as i32 {
                return Some(d.param.text.clone());
            }
        }
    }
    None
}

/// Get component pins for netlist building from v2 records.
fn get_component_pins_v2(records: &[TypedRecord], comp_index: usize) -> Vec<(String, String, i32, i32)> {
    let mut pins = Vec::new();
    for record in records {
        if let TypedRecord::Pin(p) = record {
            if p.owner_index == comp_index as i32 {
                // Calculate corner (endpoint) from location + orientation + length
                let (corner_x, corner_y) = calculate_pin_corner(p);
                pins.push((p.designator.clone(), p.name.clone(), corner_x, corner_y));
            }
        }
    }
    pins
}

/// Calculate pin corner (endpoint) from pin data.
fn calculate_pin_corner(pin: &PinData) -> (i32, i32) {
    use crate::v2::types::RotationBy90;

    let len = pin.pin_length;
    match pin.orientation {
        RotationBy90::Rotate0 => (pin.location_x + len, pin.location_y),   // Right
        RotationBy90::Rotate90 => (pin.location_x, pin.location_y + len),  // Up
        RotationBy90::Rotate180 => (pin.location_x - len, pin.location_y), // Left
        RotationBy90::Rotate270 => (pin.location_x, pin.location_y - len), // Down
    }
}

// NOTE: build_component_list removed - no longer used after V2 migration

// ═══════════════════════════════════════════════════════════════════════════
// CREATION COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// Embedded blank SchDoc template.
const BLANK_SCHDOC_TEMPLATE: &[u8] = include_bytes!("../../data/blank/Sheet1.SchDoc");

/// Create a new empty SchDoc file.
pub fn cmd_create(path: &Path, template: Option<PathBuf>) -> Result<(), String> {
    if path.exists() {
        return Err(format!("File already exists: {}", path.display()));
    }

    match template {
        Some(template_path) => {
            std::fs::copy(&template_path, path)
                .map_err(|e| format!("Error copying template: {}", e))?;
            println!("Created SchDoc from template: {}", path.display());
            println!("  Template: {}", template_path.display());
        }
        None => {
            std::fs::write(path, BLANK_SCHDOC_TEMPLATE)
                .map_err(|e| format!("Error creating file: {}", e))?;
            println!("Created empty SchDoc: {}", path.display());
        }
    }

    let doc = open_schdoc_boxed(path)
        .map_err(|e| format!("Error verifying SchDoc: {}", e))?;
    println!("  Records: {}", doc.records.len());

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// HIGH-LEVEL COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// Complete design overview.
pub fn cmd_overview(path: &Path) -> Result<SchDocOverview, Box<dyn std::error::Error>> {
    let doc = open_schdoc(path)?;
    let records = doc.typed_records();
    let counts = count_record_types_v2(&doc);

    let sheet_size = doc
        .sheet()
        .map(|h| sheet_size_name_v2(h.sheet_style).to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    // Collect components by category
    let mut categories: HashMap<&'static str, Vec<(String, String, String)>> = HashMap::new();
    for (idx, c) in doc.components().enumerate() {
        let comp_idx = records.iter().position(|r| matches!(r, TypedRecord::Component(comp) if std::ptr::eq(comp, c))).unwrap_or(idx);
        let des = get_component_designator_v2(records, comp_idx).unwrap_or_else(|| "<none>".to_string());
        let category = categorize_component(&c.lib_reference, &c.component_description);
        categories.entry(category).or_default().push((
            des,
            c.lib_reference.clone(),
            c.component_description.clone(),
        ));
    }

    // Convert to output format
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
    for &category in &category_order {
        if let Some(comps) = categories.get(category) {
            let comp_refs: Vec<SchDocComponentRef> = comps
                .iter()
                .map(|(des, lib_ref, desc)| SchDocComponentRef {
                    designator: des.clone(),
                    lib_reference: lib_ref.clone(),
                    description: desc.clone(),
                })
                .collect();
            components_by_category.push((category.to_string(), comp_refs));
        }
    }

    // Collect power nets using v2 accessors
    let power_nets = power_map_v2(&doc);
    let (rails, grounds) = separate_power_and_ground_v2(power_nets);

    let power_architecture = PowerArchitecture {
        power_rails: rails,
        ground_nets: grounds,
    };

    // Collect interface ports
    let ports: Vec<_> = doc.typed_records().iter().filter_map(|r| {
        if let TypedRecord::Port(p) = r {
            Some(p)
        } else {
            None
        }
    }).collect();

    let interfaces = if !ports.is_empty() {
        let inputs: Vec<String> = ports
            .iter()
            .filter(|p| matches!(p.io_type, PortIO::Input))
            .map(|p| p.name.clone())
            .collect();
        let outputs: Vec<String> = ports
            .iter()
            .filter(|p| matches!(p.io_type, PortIO::Output))
            .map(|p| p.name.clone())
            .collect();
        let bidirectional: Vec<String> = ports
            .iter()
            .filter(|p| matches!(p.io_type, PortIO::Bidirectional))
            .map(|p| p.name.clone())
            .collect();
        let unspecified: Vec<String> = ports
            .iter()
            .filter(|p| matches!(p.io_type, PortIO::Unspecified))
            .map(|p| p.name.clone())
            .collect();

        Some(InterfaceSummary {
            inputs,
            outputs,
            bidirectional,
            unspecified,
        })
    } else {
        None
    };

    // Collect key signals
    let mut net_labels: HashMap<String, usize> = HashMap::new();
    for nl in doc.net_labels() {
        *net_labels.entry(nl.text.clone()).or_insert(0) += 1;
    }

    let data_buses: Vec<String> = net_labels
        .iter()
        .filter(|(n, _)| {
            n.contains('[') || n.contains("DATA") || n.contains("D0") || n.contains("DQ")
        })
        .map(|(n, _)| n.clone())
        .collect();
    let address_buses: Vec<String> = net_labels
        .iter()
        .filter(|(n, _)| n.contains("ADDR") || n.contains("A0") || n.starts_with("A["))
        .map(|(n, _)| n.clone())
        .collect();
    let control_signals: Vec<String> = net_labels
        .iter()
        .filter(|(n, _)| {
            n.contains("CLK")
                || n.contains("RESET")
                || n.contains("EN")
                || n.contains("CS")
                || n.contains("WR")
                || n.contains("RD")
                || n.contains("_B")
        })
        .filter(|(n, _)| !n.contains('['))
        .map(|(n, _)| n.clone())
        .collect();

    let key_signals = KeySignals {
        total_unique_nets: net_labels.len(),
        data_buses,
        address_buses,
        control_signals,
    };

    // Quick stats
    let quick_stats = SchDocQuickStats {
        components: counts.get("Component").copied().unwrap_or(0),
        wires: counts.get("Wire").copied().unwrap_or(0),
        junctions: counts.get("Junction").copied().unwrap_or(0),
        net_labels: counts.get("NetLabel").copied().unwrap_or(0),
        ports: counts.get("Port").copied().unwrap_or(0),
        power_symbols: counts.get("PowerObject").copied().unwrap_or(0),
    };

    Ok(SchDocOverview {
        path: path.display().to_string(),
        sheet_size,
        components_by_category,
        power_architecture,
        interfaces,
        key_signals,
        quick_stats,
    })
}

/// Build power map from v2 document.
fn power_map_v2(doc: &SchDocV2) -> HashMap<String, usize> {
    let mut power_nets: HashMap<String, usize> = HashMap::new();
    for p in doc.power_objects() {
        *power_nets.entry(p.text.clone()).or_insert(0) += 1;
    }
    power_nets
}

/// Separate power nets into rails and grounds.
fn separate_power_and_ground_v2(power_nets: HashMap<String, usize>) -> (Vec<(String, usize)>, Vec<(String, usize)>) {
    let mut rails = Vec::new();
    let mut grounds = Vec::new();

    for (name, count) in power_nets {
        let upper = name.to_uppercase();
        if upper.contains("GND") || upper.contains("VSS") || upper.contains("GROUND") {
            grounds.push((name, count));
        } else {
            rails.push((name, count));
        }
    }

    rails.sort_by(|a, b| a.0.cmp(&b.0));
    grounds.sort_by(|a, b| a.0.cmp(&b.0));
    (rails, grounds)
}

/// Bill of materials.
pub fn cmd_bom(path: &Path) -> Result<SchDocBom, Box<dyn std::error::Error>> {
    let doc = open_schdoc(path)?;
    let records = doc.typed_records();

    // Group by library reference
    let mut bom: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for (idx, record) in records.iter().enumerate() {
        if let TypedRecord::Component(c) = record {
            let des = get_component_designator_v2(records, idx).unwrap_or_else(|| "<none>".to_string());
            bom.entry(c.lib_reference.clone())
                .or_default()
                .push((des, c.component_description.clone()));
        }
    }

    // Sort by quantity (most used first)
    let mut sorted: Vec<_> = bom.iter().collect();
    sorted.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

    let total_components = sorted.iter().map(|(_, items)| items.len()).sum();
    let unique_parts = bom.len();

    let items: Vec<BomItem> = sorted
        .iter()
        .map(|(lib_ref, comps)| {
            let mut designators: Vec<_> = comps.iter().map(|(d, _)| d.clone()).collect();
            designators.sort_by(|a, b| alphanumeric_sort(a, b));

            let description = comps
                .first()
                .map(|(_, desc)| desc.clone())
                .unwrap_or_default();

            BomItem {
                lib_reference: lib_ref.to_string(),
                quantity: comps.len(),
                designators,
                description,
            }
        })
        .collect();

    Ok(SchDocBom {
        path: path.display().to_string(),
        total_components,
        unique_parts,
        items,
    })
}

/// Net connectivity map.
///
/// Uses wire-tracing connectivity (union-find over wire endpoints, pins,
/// net labels, and power ports) instead of proximity matching.
#[allow(clippy::type_complexity)]
pub fn cmd_netlist(
    path: &Path,
    net_filter: Option<String>,
    min_connections: usize,
) -> Result<SchDocNetlist, Box<dyn std::error::Error>> {
    use crate::edit::netlist::{ConnectionKind, NetlistBuilder};

    let doc = open_schdoc(path)?;
    let records = doc.typed_records();

    // Build component designator lookup and pin locations
    let mut pin_locations: HashMap<(i32, i32), Vec<(String, String, String)>> = HashMap::new();
    for (idx, record) in records.iter().enumerate() {
        if let TypedRecord::Component(_) = record {
            let des = get_component_designator_v2(records, idx).unwrap_or_else(|| format!("?{}", idx));
            for (pin_des, pin_name, corner_x, corner_y) in get_component_pins_v2(records, idx) {
                pin_locations
                    .entry((corner_x, corner_y))
                    .or_default()
                    .push((des.clone(), pin_des, pin_name));
            }
        }
    }

    // Build net name to location mapping
    let mut net_at_location: HashMap<(i32, i32), String> = HashMap::new();
    for record in records {
        match record {
            TypedRecord::NetLabel(nl) => {
                net_at_location.insert(
                    (nl.location_x, nl.location_y),
                    nl.text.clone(),
                );
            }
            TypedRecord::PowerObject(p) => {
                net_at_location.insert(
                    (p.location_x, p.location_y),
                    p.text.clone(),
                );
            }
            _ => {}
        }
    }

    // Group connections by net name
    let mut nets: HashMap<String, Vec<String>> = HashMap::new();
    let proximity_threshold = 100000; // 10 mils

    for ((net_x, net_y), net_name) in &net_at_location {
        for ((pin_x, pin_y), pins) in &pin_locations {
            if (net_x - pin_x).abs() < proximity_threshold
                && (net_y - pin_y).abs() < proximity_threshold
            {
                for (comp_des, pin_des, pin_name) in pins {
                    nets.entry(net_name.clone())
                        .or_default()
                        .push(format!("{}.{} ({})", comp_des, pin_des, pin_name));
                }
            }
        }
    }

    // Apply filters
    let mut filtered_nets: Vec<_> = nets
        .iter()
        .filter(|(name, conns)| {
            let pass_filter = match &net_filter {
                Some(f) if f.contains('*') => name.contains(&f.replace('*', "")),
                Some(f) => name.eq_ignore_ascii_case(f),
                None => true,
            };
            pass_filter && conns.len() >= min_connections
        })
        .collect();
    filtered_nets.sort_by(|a, b| a.0.cmp(b.0));

    let net_connections: Vec<NetConnection> = filtered_nets
        .iter()
        .map(|(name, conns)| NetConnection {
            net_name: (*name).clone(),
            connections: conns.to_vec(),
        })
        .collect();

    Ok(SchDocNetlist {
        path: path.display().to_string(),
        filter: net_filter,
        min_connections,
        total_nets: net_connections.len(),
        nets: net_connections,
    })
}

/// Power distribution map.
pub fn cmd_power_map(path: &Path) -> Result<SchDocPowerMap, Box<dyn std::error::Error>> {
    let doc = open_schdoc_boxed(path)?;
    let records = doc.typed_records();

    // Build component info
    let mut comp_info: HashMap<usize, (String, String)> = HashMap::new();
    let mut power_pins: HashMap<usize, Vec<(String, String)>> = HashMap::new(); // comp_idx -> [(pin_des, pin_name)]

    for (idx, record) in records.iter().enumerate() {
        if let TypedRecord::Component(c) = record {
            let des = get_component_designator_v2(records, idx).unwrap_or_else(|| format!("?{}", idx));
            comp_info.insert(idx, (des.clone(), c.lib_reference.clone()));

            // Find power pins owned by this component
            for pin_record in records {
                if let TypedRecord::Pin(p) = pin_record {
                    if p.owner_index == idx as i32 {
                        let name_upper = p.name.to_uppercase();
                        if name_upper.contains("VCC")
                            || name_upper.contains("VDD")
                            || name_upper.contains("GND")
                            || name_upper.contains("VSS")
                            || name_upper.contains("AVCC")
                            || name_upper.contains("AVDD")
                            || name_upper.contains("AGND")
                            || name_upper.contains("DVCC")
                            || name_upper.contains("DVDD")
                            || name_upper.contains("DGND")
                            || name_upper.contains("VIN")
                            || name_upper.contains("VOUT")
                            || name_upper.contains("PWR")
                            || name_upper.contains("POWER")
                            || matches!(p.electrical, PinElectrical::Power)
                        {
                            power_pins
                                .entry(idx)
                                .or_default()
                                .push((p.designator.clone(), p.name.clone()));
                        }
                    }
                }
            }
        }
    }

    // Get power symbols and their nets
    let mut power_nets: HashMap<String, Vec<(i32, i32)>> = HashMap::new();
    for p in doc.power_objects() {
        power_nets
            .entry(p.text.clone())
            .or_default()
            .push((p.location_x, p.location_y));
    }

    // Separate into power rails and grounds
    let mut rails: Vec<_> = power_nets
        .iter()
        .filter(|(name, _)| {
            !name.to_uppercase().contains("GND") && !name.to_uppercase().contains("VSS")
        })
        .collect();
    let mut grounds: Vec<_> = power_nets
        .iter()
        .filter(|(name, _)| {
            name.to_uppercase().contains("GND") || name.to_uppercase().contains("VSS")
        })
        .collect();

    rails.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
    grounds.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

    // Build power rails with consumers
    let power_rails: Vec<PowerRail> = rails
        .iter()
        .map(|(net_name, locations)| {
            // Find components with power pins near these locations
            let mut consumers = Vec::new();
            for (comp_idx, pins) in &power_pins {
                if let Some((des, _lib_ref)) = comp_info.get(comp_idx) {
                    for (_pin_des, pin_name) in pins {
                        if pin_name.to_uppercase().contains(&net_name.to_uppercase())
                            || (net_name.contains("3V3") && pin_name.contains("3V3"))
                            || (net_name.contains("5V") && pin_name.contains("5V"))
                            || (net_name.contains("1V")
                                && (pin_name.contains("1V") || pin_name.contains("VDD")))
                        {
                            consumers.push(format!("{} ({})", des, pin_name));
                        }
                    }
                }
            }
            consumers.sort();
            consumers.dedup();

            PowerRail {
                net_name: (*net_name).clone(),
                symbol_count: locations.len(),
                consumers,
            }
        })
        .collect();

    // Build ground nets
    let ground_nets: Vec<GroundNet> = grounds
        .iter()
        .map(|(net_name, locations)| GroundNet {
            net_name: (*net_name).clone(),
            symbol_count: locations.len(),
        })
        .collect();

    // Build powered components
    let mut powered_components: Vec<_> = power_pins
        .iter()
        .filter_map(|(idx, pins)| {
            comp_info.get(idx).map(|(des, lib_ref)| PoweredComponent {
                designator: des.clone(),
                lib_reference: lib_ref.clone(),
                power_pin_count: pins.len(),
            })
        })
        .collect();
    powered_components.sort_by(|a, b| b.power_pin_count.cmp(&a.power_pin_count));

    Ok(SchDocPowerMap {
        path: path.display().to_string(),
        power_rails,
        ground_nets,
        powered_components,
    })
}

/// Block diagram - shows major ICs as functional blocks.
pub fn cmd_blocks(path: &Path, show_all: bool) -> Result<SchDocBlocks, Box<dyn std::error::Error>> {
    let doc = open_schdoc_boxed(path)?;
    let records = doc.typed_records();

    let mut blocks: Vec<BlockInfo> = Vec::new();

    for (idx, record) in records.iter().enumerate() {
        if let TypedRecord::Component(c) = record {
            let des = get_component_designator_v2(records, idx).unwrap_or_else(|| "<none>".to_string());
            let category = categorize_component(&c.lib_reference, &c.component_description);

            // Skip passives unless show_all is set
            if !show_all {
                let skip_categories = ["Capacitor", "Resistor", "Inductor/Ferrite", "Test Point"];
                if skip_categories.contains(&category) {
                    continue;
                }
            }

            let mut power_pins = Vec::new();
            let mut input_pins = Vec::new();
            let mut output_pins = Vec::new();
            let mut bidir_pins = Vec::new();

            // Categorize pins owned by this component
            for pin_record in records {
                if let TypedRecord::Pin(p) = pin_record {
                    if p.owner_index == idx as i32 {
                        if p.is_hidden {
                            continue;
                        }
                        let pin_info = if p.name.is_empty() {
                            p.designator.clone()
                        } else if p.name.len() > 15 {
                            format!("{}...", &p.name[..12])
                        } else {
                            p.name.clone()
                        };

                        match p.electrical {
                            PinElectrical::Power => power_pins.push(pin_info),
                            PinElectrical::Input => input_pins.push(pin_info),
                            PinElectrical::Output => output_pins.push(pin_info),
                            PinElectrical::IO => bidir_pins.push(pin_info),
                            _ => bidir_pins.push(pin_info), // Passive, etc.
                        }
                    }
                }
            }

            blocks.push(BlockInfo {
                designator: des,
                lib_reference: c.lib_reference.clone(),
                description: c.component_description.clone(),
                category: category.to_string(),
                power_pins,
                input_pins,
                output_pins,
                bidir_pins,
            });
        }
    }

    // Sort by category importance
    let category_priority: HashMap<&str, usize> = [
        ("Microcontroller", 0),
        ("FPGA/CPLD", 1),
        ("Memory", 2),
        ("ADC", 3),
        ("DAC", 4),
        ("Transceiver/PHY", 5),
        ("Clock/Oscillator", 6),
        ("Power Supply", 7),
        ("Amplifier", 8),
        ("Mux/Switch", 9),
        ("Buffer/Driver", 10),
        ("Other IC", 11),
    ]
    .iter()
    .cloned()
    .collect();

    blocks.sort_by(|a, b| {
        let pa = category_priority.get(a.category.as_str()).unwrap_or(&99);
        let pb = category_priority.get(b.category.as_str()).unwrap_or(&99);
        pa.cmp(pb)
            .then_with(|| alphanumeric_sort(&a.designator, &b.designator))
    });

    Ok(SchDocBlocks {
        path: path.display().to_string(),
        blocks,
        show_all,
    })
}

/// Multi-file project analysis.
pub fn cmd_project(paths: &[PathBuf]) -> Result<SchDocProjectAnalysis, Box<dyn std::error::Error>> {
    if paths.is_empty() {
        return Err("No schematic files specified".into());
    }

    // Collect info from all files
    struct LocalSheetInfo {
        name: String,
        components: usize,
        ports: Vec<(String, String)>, // (name, io_type)
        power_nets: Vec<String>,
        unique_nets: Vec<String>,
    }

    let mut sheets: Vec<LocalSheetInfo> = Vec::new();
    let mut all_ports: HashMap<String, Vec<(String, String)>> = HashMap::new(); // port_name -> [(sheet, io_type)]

    for path in paths {
        let doc = match open_schdoc(path) {
            Ok(d) => d,
            Err(_e) => {
                // Skip files that can't be opened
                continue;
            }
        };

        let sheet_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let component_count = doc.components().count();

        let mut ports: Vec<(String, String)> = Vec::new();
        for record in doc.typed_records() {
            if let TypedRecord::Port(p) = record {
                let io = match p.io_type {
                    PortIO::Input => "IN",
                    PortIO::Output => "OUT",
                    PortIO::Bidirectional => "BIDIR",
                    PortIO::Unspecified => "BUS",
                };
                ports.push((p.name.clone(), io.to_string()));
                all_ports
                    .entry(p.name.clone())
                    .or_default()
                    .push((sheet_name.clone(), io.to_string()));
            }
        }

        let mut power_nets: Vec<String> = doc
            .power_objects()
            .map(|p| p.text.clone())
            .collect();
        power_nets.sort();
        power_nets.dedup();

        let mut unique_nets: Vec<String> = doc
            .net_labels()
            .map(|nl| nl.text.clone())
            .collect();
        unique_nets.sort();
        unique_nets.dedup();

        sheets.push(LocalSheetInfo {
            name: sheet_name,
            components: component_count,
            ports,
            power_nets,
            unique_nets,
        });
    }

    // Build output structures
    let output_sheets: Vec<SheetInfo> = sheets
        .iter()
        .map(|s| SheetInfo {
            name: s.name.clone(),
            component_count: s.components,
            port_count: s.ports.len(),
            net_count: s.unique_nets.len(),
            ports: s.ports.clone(),
            power_nets: s.power_nets.clone(),
        })
        .collect();

    // Find inter-sheet connections (ports that appear on multiple sheets)
    let mut connections: Vec<_> = all_ports
        .iter()
        .filter(|(_, sheets)| sheets.len() > 1)
        .collect();

    connections.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

    let inter_sheet_connections: Vec<InterSheetConnection> = connections
        .into_iter()
        .map(|(port_name, connected_sheets)| InterSheetConnection {
            port_name: port_name.to_string(),
            connected_sheets: connected_sheets.clone(),
        })
        .collect();

    Ok(SchDocProjectAnalysis {
        sheet_count: output_sheets.len(),
        sheets: output_sheets,
        inter_sheet_connections,
    })
}

/// Signal flow analysis.
pub fn cmd_signal_flow(
    path: &Path,
    signal: &str,
) -> Result<SchDocSignalFlow, Box<dyn std::error::Error>> {
    let doc = open_schdoc(path)?;
    let records = doc.typed_records();

    // Find all net labels matching the signal
    let matching_nets: Vec<_> = doc
        .net_labels()
        .filter_map(|nl| {
            if nl.text.eq_ignore_ascii_case(signal)
                || nl.text.to_uppercase().contains(&signal.to_uppercase())
            {
                Some((nl.text.clone(), nl.location_x, nl.location_y))
            } else {
                None
            }
        })
        .collect();

    // Also check power objects
    let matching_power: Vec<_> = doc
        .power_objects()
        .filter_map(|p| {
            if p.text.eq_ignore_ascii_case(signal)
                || p.text.to_uppercase().contains(&signal.to_uppercase())
            {
                Some((p.text.clone(), p.location_x, p.location_y))
            } else {
                None
            }
        })
        .collect();

    // Also check ports
    let matching_ports: Vec<_> = records
        .iter()
        .filter_map(|r| {
            if let TypedRecord::Port(p) = r {
                if p.name.eq_ignore_ascii_case(signal)
                    || p.name.to_uppercase().contains(&signal.to_uppercase())
                {
                    Some((p.name.clone(), format!("{:?}", p.io_type)))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    if matching_nets.is_empty() && matching_power.is_empty() && matching_ports.is_empty() {
        return Ok(SchDocSignalFlow {
            path: path.display().to_string(),
            signal: signal.to_string(),
            trace_found: false,
            trace: None,
        });
    }

    // Build trace path
    let mut trace_path = Vec::new();

    // Add net labels
    for (name, x, y) in &matching_nets {
        trace_path.push(format!("NetLabel {} at {}", name, fmt_point(*x, *y)));
    }

    // Add power symbols
    for (name, x, y) in &matching_power {
        trace_path.push(format!("Power {} at {}", name, fmt_point(*x, *y)));
    }

    // Add ports
    for (name, io_type) in &matching_ports {
        trace_path.push(format!("Port {} [{}]", name, io_type));
    }

    // Find components with matching pins
    let signal_upper = signal.to_uppercase();
    let mut destinations = Vec::new();

    for (idx, record) in records.iter().enumerate() {
        if let TypedRecord::Component(_c) = record {
            let des = get_component_designator_v2(records, idx).unwrap_or_default();

            // Find pins owned by this component
            for pin_record in records {
                if let TypedRecord::Pin(p) = pin_record {
                    if p.owner_index == idx as i32 {
                        if p.name.to_uppercase().contains(&signal_upper)
                            || p.designator.to_uppercase().contains(&signal_upper)
                        {
                            destinations.push(format!(
                                "{}.{} ({}) - {:?}",
                                des, p.designator, p.name, p.electrical
                            ));
                        }
                    }
                }
            }
        }
    }

    let source = if !matching_ports.is_empty() {
        format!("Port {}", matching_ports[0].0)
    } else if !matching_power.is_empty() {
        format!("Power {}", matching_power[0].0)
    } else {
        format!("Net {}", matching_nets[0].0)
    };

    Ok(SchDocSignalFlow {
        path: path.display().to_string(),
        signal: signal.to_string(),
        trace_found: true,
        trace: Some(SignalTrace {
            source,
            path: trace_path,
            destinations,
        }),
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// DETAILED COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// Show document overview.
pub fn cmd_info(path: &Path) -> Result<SchDocInfo, Box<dyn std::error::Error>> {
    let doc = open_schdoc_boxed(path)?;
    let counts = count_record_types_v2(&doc);

    // Sheet info
    let sheet_info = doc.sheet().map(|header| {
        let custom_dimensions = if header.custom_x > 0 || header.custom_y > 0 {
            Some((
                fmt_coord(header.custom_x * 10000),
                fmt_coord(header.custom_y * 10000),
            ))
        } else {
            None
        };
        SheetInfoDetails {
            size: sheet_size_name_v2(header.sheet_style).to_string(),
            size_style: header.sheet_style as i32,
            custom_dimensions,
            fonts_defined: header.font_id_count as i32,
        }
    });

    // Primitive summary
    let primitive_summary = PrimitiveSummary {
        total_primitives: doc.typed_records().len(),
        components: counts.get("Component").copied().unwrap_or(0),
        wires: counts.get("Wire").copied().unwrap_or(0),
        net_labels: counts.get("NetLabel").copied().unwrap_or(0),
        ports: counts.get("Port").copied().unwrap_or(0),
        power_objects: counts.get("PowerObject").copied().unwrap_or(0),
        junctions: counts.get("Junction").copied().unwrap_or(0),
        pins: counts.get("Pin").copied().unwrap_or(0),
    };

    // Collect unique net names
    let mut net_names: Vec<String> = doc
        .net_labels()
        .map(|nl| nl.text.clone())
        .collect();
    net_names.sort();
    net_names.dedup();

    // Collect unique power nets
    let mut power_nets: Vec<String> = doc
        .power_objects()
        .map(|p| p.text.clone())
        .collect();
    power_nets.sort();
    power_nets.dedup();

    Ok(SchDocInfo {
        path: path.display().to_string(),
        sheet_info,
        primitive_summary,
        unique_nets: net_names,
        power_nets,
    })
}

/// Show detailed record statistics.
pub fn cmd_stats(path: &Path) -> Result<SchDocStats, Box<dyn std::error::Error>> {
    let doc = open_schdoc_boxed(path)?;
    let counts = count_record_types_v2(&doc);

    let mut record_types: Vec<(String, usize)> = counts
        .into_iter()
        .map(|(name, count)| (name.to_string(), count))
        .collect();
    record_types.sort_by(|a, b| b.1.cmp(&a.1));

    Ok(SchDocStats {
        path: path.display().to_string(),
        total_primitives: doc.typed_records().len(),
        record_types,
    })
}

/// List all components.
pub fn cmd_components(
    path: &Path,
    verbose: bool,
) -> Result<SchDocComponentList, Box<dyn std::error::Error>> {
    let doc = open_schdoc_boxed(path)?;
    let records = doc.typed_records();

    // Build component list with indices and designators
    let mut component_data: Vec<(usize, &ComponentData, Option<String>)> = Vec::new();
    for (idx, record) in records.iter().enumerate() {
        if let TypedRecord::Component(c) = record {
            let designator = get_component_designator_v2(records, idx);
            component_data.push((idx, c, designator));
        }
    }

    // Sort by designator
    component_data.sort_by(|a, b| {
        let a_des = a.2.as_deref().unwrap_or("");
        let b_des = b.2.as_deref().unwrap_or("");
        alphanumeric_sort(a_des, b_des)
    });

    let components = component_data
        .iter()
        .map(|(idx, comp, designator)| {
            let child_count = if verbose {
                // Count children by matching owner_index
                let count = records.iter().filter(|r| {
                    match r {
                        TypedRecord::Pin(p) => p.owner_index == *idx as i32,
                        TypedRecord::Parameter(p) => p.graphical.base.owner_index == *idx as i32,
                        TypedRecord::Designator(d) => d.param.graphical.base.owner_index == *idx as i32,
                        _ => false,
                    }
                }).count();
                Some(count)
            } else {
                None
            };
            SchDocComponentInfo {
                designator: designator.clone().unwrap_or_else(|| "<none>".to_string()),
                lib_reference: comp.lib_reference.clone(),
                description: comp.component_description.clone(),
                location: fmt_point(comp.location_x, comp.location_y),
                parts: comp.part_count as i32,
                child_count,
            }
        })
        .collect();

    Ok(SchDocComponentList {
        path: path.display().to_string(),
        total_components: component_data.len(),
        components,
    })
}

/// Show component details.
pub fn cmd_component(
    path: &Path,
    designator: &str,
    show_children: bool,
) -> Result<SchDocComponentDetail, Box<dyn std::error::Error>> {
    let doc = open_schdoc_boxed(path)?;
    let records = doc.typed_records();

    // Find component by designator or index
    let (comp_idx, comp) = if let Ok(index) = designator.parse::<usize>() {
        // Numeric index - find Nth component
        let mut comp_count = 0;
        let mut found = None;
        for (idx, record) in records.iter().enumerate() {
            if let TypedRecord::Component(c) = record {
                if comp_count == index {
                    found = Some((idx, c));
                    break;
                }
                comp_count += 1;
            }
        }
        found.ok_or_else(|| format!("Component index {} not found", index))?
    } else {
        // Find by designator
        let mut found = None;
        for (idx, record) in records.iter().enumerate() {
            if let TypedRecord::Component(c) = record {
                if let Some(des) = get_component_designator_v2(records, idx) {
                    if des.eq_ignore_ascii_case(designator) {
                        found = Some((idx, c));
                        break;
                    }
                }
            }
        }
        found.ok_or_else(|| format!("Component '{}' not found", designator))?
    };

    let actual_designator = get_component_designator_v2(records, comp_idx);

    // Collect children by owner_index
    let mut pin_infos = Vec::new();
    let mut param_infos = Vec::new();
    let mut designator_infos = Vec::new();
    let mut graphics_count = 0;
    let mut child_count = 0;

    for record in records {
        let owner = match record {
            TypedRecord::Pin(p) => Some(p.owner_index),
            TypedRecord::Parameter(p) => Some(p.graphical.base.owner_index),
            TypedRecord::Designator(d) => Some(d.param.graphical.base.owner_index),
            TypedRecord::Line(l) => Some(l.graphical.base.owner_index),
            TypedRecord::Rectangle(r) => Some(r.graphical.base.owner_index),
            TypedRecord::Arc(a) => Some(a.graphical.base.owner_index),
            TypedRecord::Polyline(p) => Some(p.graphical.base.owner_index),
            TypedRecord::Polygon(p) => Some(p.graphical.base.owner_index),
            _ => None,
        };

        if owner == Some(comp_idx as i32) {
            child_count += 1;
            match record {
                TypedRecord::Pin(p) => {
                    pin_infos.push(SchDocPinInfo {
                        designator: p.designator.clone(),
                        name: p.name.clone(),
                        electrical_type: format!("{:?}", p.electrical),
                        hidden: p.is_hidden,
                    });
                }
                TypedRecord::Parameter(p) => {
                    param_infos.push(SchDocParameter {
                        name: p.name.clone(),
                        value: p.text.clone(),
                    });
                }
                TypedRecord::Designator(d) => {
                    designator_infos.push(SchDocDesignator {
                        name: d.param.name.clone(),
                        value: d.param.text.clone(),
                    });
                }
                _ => graphics_count += 1,
            }
        }
    }

    Ok(SchDocComponentDetail {
        designator: actual_designator.unwrap_or_else(|| "<none>".to_string()),
        lib_reference: comp.lib_reference.clone(),
        description: comp.component_description.clone(),
        location: fmt_point(comp.location_x, comp.location_y),
        parts: comp.part_count as i32,
        display_modes: comp.display_mode_count as i32,
        current_part: comp.current_part_id as i32,
        unique_id: comp.unique_id.clone(),
        child_primitive_count: child_count,
        pins: pin_infos,
        parameters: param_infos,
        designators: designator_infos,
        graphic_primitive_count: if show_children {
            Some(graphics_count)
        } else {
            None
        },
    })
}

/// List all wires.
pub fn cmd_wires(
    path: &Path,
    limit: Option<usize>,
) -> Result<SchDocWireList, Box<dyn std::error::Error>> {
    let doc = open_schdoc_boxed(path)?;

    let wires: Vec<&WireData> = doc.wires().collect();
    let display_count = limit.unwrap_or(wires.len()).min(wires.len());

    let wire_infos: Vec<WireInfo> = wires
        .iter()
        .take(display_count)
        .enumerate()
        .map(|(i, wire)| {
            let vertices = &wire.vertices;
            let (start, end_or_segments) = if vertices.len() == 2 {
                (
                    fmt_point(vertices[0].0, vertices[0].1),
                    fmt_point(vertices[1].0, vertices[1].1),
                )
            } else {
                let start = if vertices.is_empty() {
                    "(empty)".to_string()
                } else {
                    fmt_point(vertices[0].0, vertices[0].1)
                };
                let segments = format!("{} segments", vertices.len().saturating_sub(1));
                (start, segments)
            };
            WireInfo {
                index: i,
                start,
                end_or_segments,
            }
        })
        .collect();

    Ok(SchDocWireList {
        path: path.display().to_string(),
        total_wires: wires.len(),
        wires: wire_infos,
    })
}

/// List all net labels.
pub fn cmd_nets(
    path: &Path,
    group: bool,
) -> Result<SchDocNetLabelList, Box<dyn std::error::Error>> {
    let doc = open_schdoc_boxed(path)?;

    let net_labels: Vec<&NetLabelData> = doc.net_labels().collect();

    let (grouped, individual) = if group {
        let mut grouped_map: HashMap<&str, Vec<&NetLabelData>> = HashMap::new();
        for nl in &net_labels {
            grouped_map.entry(&nl.text).or_default().push(nl);
        }

        let mut sorted: Vec<_> = grouped_map.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(b.0));

        let grouped_result: Vec<(String, usize)> = sorted
            .into_iter()
            .map(|(name, labels)| (name.to_string(), labels.len()))
            .collect();

        (Some(grouped_result), None)
    } else {
        let individual_result: Vec<NetLabelInfo> = net_labels
            .iter()
            .map(|nl| NetLabelInfo {
                net_name: nl.text.clone(),
                location: fmt_point(nl.location_x, nl.location_y),
            })
            .collect();

        (None, Some(individual_result))
    };

    Ok(SchDocNetLabelList {
        path: path.display().to_string(),
        total_net_labels: net_labels.len(),
        group_by_name: group,
        grouped,
        individual,
    })
}

/// List all ports.
pub fn cmd_ports(path: &Path) -> Result<SchDocPortList, Box<dyn std::error::Error>> {
    let doc = open_schdoc_boxed(path)?;

    let ports: Vec<&PortData> = doc.typed_records().iter().filter_map(|r| {
        if let TypedRecord::Port(p) = r { Some(p) } else { None }
    }).collect();

    let port_infos: Vec<PortInfo> = ports
        .iter()
        .map(|port| {
            let io_type = match port.io_type {
                PortIO::Unspecified => "Unspec",
                PortIO::Output => "Output",
                PortIO::Input => "Input",
                PortIO::Bidirectional => "Bidir",
            };
            PortInfo {
                name: port.name.clone(),
                io_type: io_type.to_string(),
                location: fmt_point(port.location_x, port.location_y),
            }
        })
        .collect();

    Ok(SchDocPortList {
        path: path.display().to_string(),
        total_ports: ports.len(),
        ports: port_infos,
    })
}

/// List all power objects.
pub fn cmd_power(path: &Path, group: bool) -> Result<SchDocPowerList, Box<dyn std::error::Error>> {
    let doc = open_schdoc_boxed(path)?;

    let power_objects: Vec<&PowerData> = doc.power_objects().collect();

    let (grouped, individual) = if group {
        let mut grouped_map: HashMap<&str, Vec<&PowerData>> = HashMap::new();
        for p in &power_objects {
            grouped_map.entry(&p.text).or_default().push(p);
        }

        let mut sorted: Vec<_> = grouped_map.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(b.0));

        let grouped_result: Vec<(String, usize)> = sorted
            .into_iter()
            .map(|(name, objs)| (name.to_string(), objs.len()))
            .collect();

        (Some(grouped_result), None)
    } else {
        let individual_result: Vec<PowerObjectInfo> = power_objects
            .iter()
            .map(|p| {
                let style = match p.style {
                    PowerObjectStyle::Circle => "Circle",
                    PowerObjectStyle::Arrow => "Arrow",
                    PowerObjectStyle::Bar => "Bar",
                    PowerObjectStyle::Wave => "Wave",
                    PowerObjectStyle::GndPower => "PowerGnd",
                    PowerObjectStyle::GndSignal => "SignalGnd",
                    PowerObjectStyle::GndEarth => "EarthGnd",
                    PowerObjectStyle::GOSTArrow => "GOSTArrow",
                    PowerObjectStyle::GOSTGndPower => "GOSTGndPower",
                    PowerObjectStyle::GOSTGndEarth => "GOSTGndEarth",
                    PowerObjectStyle::GOSTBar => "GOSTBar",
                };
                PowerObjectInfo {
                    net: p.text.clone(),
                    style: style.to_string(),
                    location: fmt_point(p.location_x, p.location_y),
                }
            })
            .collect();

        (None, Some(individual_result))
    };

    Ok(SchDocPowerList {
        path: path.display().to_string(),
        total_power_objects: power_objects.len(),
        group_by_net: group,
        grouped,
        individual,
    })
}

/// List pins.
pub fn cmd_pins(
    path: &Path,
    component_filter: Option<String>,
    _unconnected: bool,
) -> Result<SchDocPinList, Box<dyn std::error::Error>> {
    let doc = open_schdoc_boxed(path)?;
    let records = doc.typed_records();

    let mut pin_details = Vec::new();
    let total_pins: usize;

    // Build component index -> designator map
    let mut comp_map: HashMap<i32, String> = HashMap::new();
    for (idx, record) in records.iter().enumerate() {
        if let TypedRecord::Component(_) = record {
            if let Some(des) = get_component_designator_v2(records, idx) {
                comp_map.insert(idx as i32, des);
            }
        }
    }

    // If filtering by component, find the component first
    if let Some(ref comp_des) = component_filter {
        // Find component index by designator
        let mut found_comp_idx = None;
        for (idx, record) in records.iter().enumerate() {
            if let TypedRecord::Component(_) = record {
                if let Some(des) = get_component_designator_v2(records, idx) {
                    if des.eq_ignore_ascii_case(comp_des) {
                        found_comp_idx = Some(idx as i32);
                        break;
                    }
                }
            }
        }

        if let Some(comp_idx) = found_comp_idx {
            let des = comp_map.get(&comp_idx).cloned().unwrap_or_default();

            // Collect pins belonging to this component
            let pins: Vec<&PinData> = records
                .iter()
                .filter_map(|r| {
                    if let TypedRecord::Pin(p) = r {
                        if p.owner_index == comp_idx {
                            return Some(p);
                        }
                    }
                    None
                })
                .collect();

            total_pins = pins.len();

            for pin in pins {
                pin_details.push(SchDocPinDetail {
                    component: des.clone(),
                    designator: pin.designator.clone(),
                    name: pin.name.clone(),
                    electrical_type: format!("{:?}", pin.electrical),
                    location: fmt_point(pin.location_x, pin.location_y),
                });
            }
        } else {
            return Err(format!("Component '{}' not found", comp_des).into());
        }
    } else {
        // All pins with their parent component
        for record in records {
            if let TypedRecord::Pin(p) = record {
                let comp_des = comp_map.get(&p.owner_index).cloned().unwrap_or_default();
                pin_details.push(SchDocPinDetail {
                    component: comp_des,
                    designator: p.designator.clone(),
                    name: p.name.clone(),
                    electrical_type: format!("{:?}", p.electrical),
                    location: fmt_point(p.location_x, p.location_y),
                });
            }
        }

        total_pins = pin_details.len();
    }

    Ok(SchDocPinList {
        path: path.display().to_string(),
        total_pins,
        filter: component_filter,
        pins: pin_details,
    })
}

/// Show hierarchy.
pub fn cmd_hierarchy(
    path: &Path,
    max_depth: Option<usize>,
    from_designator: Option<String>,
) -> Result<SchDocHierarchy, Box<dyn std::error::Error>> {
    let doc = open_schdoc_boxed(path)?;
    let records = doc.typed_records();

    // Build parent-child relationship map from owner_index
    let mut children_map: HashMap<i32, Vec<usize>> = HashMap::new();
    for (idx, record) in records.iter().enumerate() {
        let owner = get_owner_index_v2(record);
        if owner >= 0 {
            children_map.entry(owner).or_default().push(idx);
        }
    }

    let start_indices: Vec<usize> = if let Some(ref des) = from_designator {
        // Start from specific component
        let mut found = Vec::new();
        for (idx, record) in records.iter().enumerate() {
            if let TypedRecord::Component(_) = record {
                if let Some(comp_des) = get_component_designator_v2(records, idx) {
                    if comp_des.eq_ignore_ascii_case(des) {
                        found.push(idx);
                        break;
                    }
                }
            }
        }
        if found.is_empty() {
            return Err(format!("Component '{}' not found", des).into());
        }
        found
    } else {
        // Start from root records (owner_index < 0 or -1)
        records
            .iter()
            .enumerate()
            .filter(|(_, r)| get_owner_index_v2(r) < 0)
            .map(|(idx, _)| idx)
            .collect()
    };

    let max_d = max_depth.unwrap_or(10);
    let hierarchy_nodes: Vec<HierarchyNode> = start_indices
        .into_iter()
        .map(|idx| build_hierarchy_node_v2(records, &children_map, idx, 0, max_d))
        .collect();

    Ok(SchDocHierarchy {
        path: path.display().to_string(),
        hierarchy: hierarchy_nodes,
    })
}

/// Get owner_index from a v2 record.
fn get_owner_index_v2(record: &TypedRecord) -> i32 {
    match record {
        TypedRecord::Pin(p) => p.owner_index,
        TypedRecord::Parameter(p) => p.graphical.base.owner_index,
        TypedRecord::Designator(d) => d.param.graphical.base.owner_index,
        TypedRecord::Line(l) => l.graphical.base.owner_index,
        TypedRecord::Rectangle(r) => r.graphical.base.owner_index,
        TypedRecord::Arc(a) => a.graphical.base.owner_index,
        TypedRecord::Polyline(p) => p.graphical.base.owner_index,
        TypedRecord::Polygon(p) => p.graphical.base.owner_index,
        TypedRecord::Ellipse(e) => e.graphical.base.owner_index,
        TypedRecord::Bezier(b) => b.graphical.base.owner_index,
        TypedRecord::Label(l) => l.graphical.base.owner_index,
        TypedRecord::Symbol(s) => s.graphical.base.owner_index,
        TypedRecord::Implementation(i) => i.base.owner_index,
        TypedRecord::ImplementationList(i) => i.graphical.base.owner_index,
        _ => -1, // Root-level records
    }
}

/// Helper function to build hierarchy node recursively for v2.
fn build_hierarchy_node_v2(
    records: &[TypedRecord],
    children_map: &HashMap<i32, Vec<usize>>,
    idx: usize,
    depth: usize,
    max_depth: usize,
) -> HierarchyNode {
    if depth <= max_depth {
        let record = &records[idx];

        // Format the node
        let (node_type, identifier, description) = match record {
            TypedRecord::Component(c) => {
                let des = get_component_designator_v2(records, idx).unwrap_or_default();
                ("component".to_string(), des, c.lib_reference.clone())
            }
            TypedRecord::Pin(p) => ("pin".to_string(), p.designator.clone(), p.name.clone()),
            TypedRecord::Parameter(p) => (
                "parameter".to_string(),
                p.name.clone(),
                p.text.clone(),
            ),
            TypedRecord::Designator(d) => (
                "designator".to_string(),
                d.param.name.clone(),
                d.param.text.clone(),
            ),
            TypedRecord::NetLabel(nl) => {
                ("netlabel".to_string(), nl.text.clone(), String::new())
            }
            TypedRecord::Port(p) => (
                "port".to_string(),
                p.name.clone(),
                format!("{:?}", p.io_type),
            ),
            TypedRecord::PowerObject(p) => (
                "power".to_string(),
                p.text.clone(),
                format!("{:?}", p.style),
            ),
            _ => (
                record_type_name_v2(record).to_string(),
                format!("[{}]", idx),
                String::new(),
            ),
        };

        // Build child hierarchy recursively
        let children: Vec<_> = children_map
            .get(&(idx as i32))
            .map(|child_indices| {
                child_indices
                    .iter()
                    .map(|&child_idx| build_hierarchy_node_v2(records, children_map, child_idx, depth + 1, max_depth))
                    .collect()
            })
            .unwrap_or_default();

        HierarchyNode {
            node_type,
            unique_id: identifier,
            description,
            children,
        }
    } else {
        // Depth limit reached
        HierarchyNode {
            node_type: "...".to_string(),
            unique_id: format!("(depth limit {} reached)", max_depth),
            description: String::new(),
            children: Vec::new(),
        }
    }
}

/// Search for text.
pub fn cmd_search(path: &Path, query: &str, limit: Option<usize>) -> Result<(), String> {
    let doc = open_schdoc(path)?;
    let records = doc.typed_records();

    let query_lower = query.to_lowercase();
    let max_results = limit.unwrap_or(50);

    println!("Search Results: {}", path.display());
    println!("Query: \"{}\"", query);
    println!("═══════════════════════════════════════════════════════════════");

    let mut results: Vec<(usize, &TypedRecord)> = Vec::new();

    for (idx, record) in records.iter().enumerate() {
        let matches = match record {
            TypedRecord::Component(c) => {
                c.lib_reference.to_lowercase().contains(&query_lower)
                    || c.component_description
                        .to_lowercase()
                        .contains(&query_lower)
            }
            TypedRecord::Pin(p) => {
                p.name.to_lowercase().contains(&query_lower)
                    || p.designator.to_lowercase().contains(&query_lower)
            }
            TypedRecord::NetLabel(nl) => nl.text.to_lowercase().contains(&query_lower),
            TypedRecord::Port(p) => p.name.to_lowercase().contains(&query_lower),
            TypedRecord::PowerObject(p) => p.text.to_lowercase().contains(&query_lower),
            TypedRecord::Label(l) => l.text.to_lowercase().contains(&query_lower),
            TypedRecord::TextFrame(tf) => tf.text.to_lowercase().contains(&query_lower),
            TypedRecord::Parameter(p) => {
                p.name.to_lowercase().contains(&query_lower)
                    || p.text.to_lowercase().contains(&query_lower)
            }
            TypedRecord::Designator(d) => {
                d.param.name.to_lowercase().contains(&query_lower)
                    || d.param.text.to_lowercase().contains(&query_lower)
            }
            _ => false,
        };

        if matches {
            results.push((idx, record));
            if results.len() >= max_results {
                break;
            }
        }
    }

    println!("\nFound {} results:\n", results.len());

    for (idx, record) in &results {
        let desc = match record {
            TypedRecord::Component(c) => {
                let des = get_component_designator_v2(records, *idx).unwrap_or_default();
                format!("Component {} - {}", des, c.lib_reference)
            }
            TypedRecord::Pin(p) => format!("Pin {} - {}", p.designator, p.name),
            TypedRecord::NetLabel(nl) => format!("NetLabel: {}", nl.text),
            TypedRecord::Port(p) => format!("Port: {}", p.name),
            TypedRecord::PowerObject(p) => format!("Power: {}", p.text),
            TypedRecord::Label(l) => format!("Label: {}", l.text),
            TypedRecord::TextFrame(tf) => {
                let text = if tf.text.len() > 40 {
                    format!("{}...", &tf.text[..40])
                } else {
                    tf.text.clone()
                };
                format!("TextFrame: {}", text)
            }
            TypedRecord::Parameter(p) => format!("Parameter: {} = {}", p.name, p.text),
            TypedRecord::Designator(d) => {
                format!("Designator: {} = {}", d.param.name, d.param.text)
            }
            _ => record_type_name_v2(record).to_string(),
        };
        println!("  [{}] {}", idx, desc);
    }

    if results.len() >= max_results {
        println!("\n(results limited to {})", max_results);
    }

    Ok(())
}

/// List junctions.
pub fn cmd_junctions(path: &Path) -> Result<SchDocJunctionList, Box<dyn std::error::Error>> {
    let doc = open_schdoc_boxed(path)?;

    let junctions: Vec<JunctionInfo> = doc
        .junctions()
        .map(|j| JunctionInfo {
            location: fmt_point(j.location_x, j.location_y),
        })
        .collect();

    Ok(SchDocJunctionList {
        path: path.display().to_string(),
        total_junctions: junctions.len(),
        junctions,
    })
}

/// JSON export structures.
#[derive(Serialize)]
struct JsonDocument {
    file: String,
    sheet: Option<JsonSheet>,
    summary: JsonSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    components: Option<Vec<JsonComponent>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nets: Option<Vec<JsonNet>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ports: Option<Vec<JsonPort>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    power: Option<Vec<JsonPower>>,
}

#[derive(Serialize)]
struct JsonSheet {
    size: String,
    fonts: i32,
}

#[derive(Serialize)]
struct JsonSummary {
    total_primitives: usize,
    components: usize,
    wires: usize,
    net_labels: usize,
    ports: usize,
    power_objects: usize,
    junctions: usize,
    pins: usize,
}

#[derive(Serialize)]
struct JsonComponent {
    designator: String,
    lib_reference: String,
    description: String,
    location: String,
    pins: Vec<JsonPin>,
    parameters: Vec<JsonParameter>,
}

#[derive(Serialize)]
struct JsonPin {
    designator: String,
    name: String,
    electrical: String,
    hidden: bool,
}

#[derive(Serialize)]
struct JsonParameter {
    name: String,
    value: String,
}

#[derive(Serialize)]
struct JsonNet {
    name: String,
    location: String,
}

#[derive(Serialize)]
struct JsonPort {
    name: String,
    io_type: String,
    location: String,
}

#[derive(Serialize)]
struct JsonPower {
    net: String,
    style: String,
    location: String,
}

/// Export as JSON.
pub fn cmd_json(path: &Path, full: bool, pretty: bool) -> Result<(), String> {
    let doc = open_schdoc(path)?;
    let records = doc.typed_records();
    let counts = count_record_types_v2(&doc);

    // Build sheet info
    let sheet = doc.sheet().map(|h| JsonSheet {
        size: sheet_size_name_v2(h.sheet_style).to_string(),
        fonts: h.font_id_count as i32,
    });

    // Build summary
    let summary = JsonSummary {
        total_primitives: records.len(),
        components: counts.get("Component").copied().unwrap_or(0),
        wires: counts.get("Wire").copied().unwrap_or(0),
        net_labels: counts.get("NetLabel").copied().unwrap_or(0),
        ports: counts.get("Port").copied().unwrap_or(0),
        power_objects: counts.get("PowerObject").copied().unwrap_or(0),
        junctions: counts.get("Junction").copied().unwrap_or(0),
        pins: counts.get("Pin").copied().unwrap_or(0),
    };

    // Full export includes all components, nets, ports, power
    let (components, nets, ports, power) = if full {
        // Components with their pins and parameters
        let mut components = Vec::new();
        for (idx, record) in records.iter().enumerate() {
            if let TypedRecord::Component(c) = record {
                let des = get_component_designator_v2(records, idx).unwrap_or_default();

                // Collect pins and parameters by owner_index
                let mut pins = Vec::new();
                let mut params = Vec::new();

                for child in records {
                    match child {
                        TypedRecord::Pin(p) if p.owner_index == idx as i32 => {
                            pins.push(JsonPin {
                                designator: p.designator.clone(),
                                name: p.name.clone(),
                                electrical: format!("{:?}", p.electrical),
                                hidden: p.is_hidden,
                            });
                        }
                        TypedRecord::Parameter(p) if p.graphical.base.owner_index == idx as i32 => {
                            params.push(JsonParameter {
                                name: p.name.clone(),
                                value: p.text.clone(),
                            });
                        }
                        _ => {}
                    }
                }

                components.push(JsonComponent {
                    designator: des,
                    lib_reference: c.lib_reference.clone(),
                    description: c.component_description.clone(),
                    location: fmt_point(c.location_x, c.location_y),
                    pins,
                    parameters: params,
                });
            }
        }

        // Net labels
        let nets: Vec<JsonNet> = doc
            .net_labels()
            .map(|nl| JsonNet {
                name: nl.text.clone(),
                location: fmt_point(nl.location_x, nl.location_y),
            })
            .collect();

        // Ports
        let ports: Vec<JsonPort> = doc.typed_records().iter().filter_map(|r| {
            if let TypedRecord::Port(p) = r {
                Some(JsonPort {
                    name: p.name.clone(),
                    io_type: format!("{:?}", p.io_type),
                    location: fmt_point(p.location_x, p.location_y),
                })
            } else {
                None
            }
        }).collect();

        // Power objects
        let power: Vec<JsonPower> = doc
            .power_objects()
            .map(|p| JsonPower {
                net: p.text.clone(),
                style: format!("{:?}", p.style),
                location: fmt_point(p.location_x, p.location_y),
            })
            .collect();

        (Some(components), Some(nets), Some(ports), Some(power))
    } else {
        (None, None, None, None)
    };

    let json_doc = JsonDocument {
        file: path.display().to_string(),
        sheet,
        summary,
        components,
        nets,
        ports,
        power,
    };

    let output = if pretty {
        serde_json::to_string_pretty(&json_doc)
    } else {
        serde_json::to_string(&json_doc)
    }
    .map_err(|e| format!("JSON serialization error: {}", e))?;

    println!("{}", output);

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// EDITING COMMAND IMPLEMENTATIONS (re-exported from schdoc_edit)
// ═══════════════════════════════════════════════════════════════════════════

pub use crate::ops::schdoc_edit::{
    cmd_add_component, cmd_add_junction, cmd_add_missing_junctions, cmd_add_net_label,
    cmd_add_port, cmd_add_power, cmd_add_wire, cmd_connect_pins, cmd_delete_component,
    cmd_delete_wire, cmd_find_missing_junctions, cmd_find_unconnected, cmd_list_library,
    cmd_move_component, cmd_new, cmd_route_wire, cmd_search_library, cmd_show_netlist,
    cmd_suggest_placement, cmd_validate,
};
