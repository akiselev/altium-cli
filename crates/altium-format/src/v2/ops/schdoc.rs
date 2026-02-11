// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Schematic document operations (v2).
//!
//! Provides high-level operations for exploring and analyzing Altium schematic
//! document (.SchDoc) files using the v2 backing-store architecture.

use std::collections::HashMap;
use std::path::Path;

use crate::v2::backing_store::{ComponentGroup, RecordNode};
use crate::v2::coord::{AltiumCoord, SchCoord};
use crate::v2::documents::schdoc::SchDoc;
use crate::v2::ops::categorization::categorize_component;
use crate::v2::ops::output::*;
use crate::v2::records::sch_component::SchComponentRecord;
use crate::v2::records::sch_net_label::SchNetLabelRecord;
use crate::v2::records::sch_port::SchPortRecord;
use crate::v2::records::sch_power::SchPowerRecord;
use crate::v2::records::sch_sheet::SchSheetRecord;

// ═══════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Opens and parses a SchDoc file from the given path.
fn open_schdoc(path: &Path) -> Result<SchDoc, Box<dyn std::error::Error>> {
    Ok(SchDoc::open_file(path).map_err(|e| e.to_string())?)
}

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

/// Format a coordinate pair as a display string.
fn format_location(x: SchCoord, y: SchCoord) -> String {
    format!("({:.1}, {:.1})", x.to_mils(), y.to_mils())
}

/// Decode sheet size style integer to a human-readable name.
fn sheet_size_name(style: i32) -> &'static str {
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

/// Decode power port style integer to a human-readable name.
fn power_style_name(style: i32) -> &'static str {
    match style {
        0 => "Circle",
        1 => "Arrow",
        2 => "Bar",
        3 => "Wave",
        4 => "Power Ground",
        5 => "Signal Ground",
        6 => "Earth",
        7 => "GND Power",
        _ => "Unknown",
    }
}

/// Decode port I/O type integer to a human-readable name.
fn port_io_type_name(io_type: i32) -> &'static str {
    match io_type {
        0 => "Unspecified",
        1 => "Output",
        2 => "Input",
        3 => "Bidirectional",
        _ => "Unspecified",
    }
}

/// Collect all records from all sources: component children and orphans.
fn collect_all_records(doc: &SchDoc) -> Vec<&RecordNode> {
    let mut all = Vec::new();
    for group in &doc.groups {
        for child in &group.children {
            all.push(child);
        }
    }
    for orphan in &doc.orphan_records {
        all.push(orphan);
    }
    all
}

/// Count records of a given type across all groups and orphans.
fn count_record_type(doc: &SchDoc, key: u8) -> usize {
    let mut count = 0;
    for group in &doc.groups {
        for child in &group.children {
            if child.key == key {
                count += 1;
            }
        }
    }
    for orphan in &doc.orphan_records {
        if orphan.key == key {
            count += 1;
        }
    }
    count
}

/// Find the sheet record (RECORD=31) from orphan records.
fn find_sheet_record(doc: &SchDoc) -> Option<&RecordNode> {
    doc.orphan_records.iter().find(|r| r.key == 31)
}

/// Extract the sheet size string from the document.
fn get_sheet_size(doc: &SchDoc) -> String {
    if let Some(sheet) = find_sheet_record(doc) {
        let rec = SchSheetRecord::from_origin(sheet.origin.clone());
        sheet_size_name(rec.sheet_style() as i32).to_string()
    } else {
        "Unknown".to_string()
    }
}

/// Collect all unique net names from net labels (RECORD=25).
fn collect_net_names(doc: &SchDoc) -> Vec<String> {
    let mut nets: HashMap<String, bool> = HashMap::new();
    let all = collect_all_records(doc);
    for node in &all {
        if node.key == 25 {
            let rec = SchNetLabelRecord::from_origin(node.origin.clone());
            let text = rec.text();
            if !text.is_empty() {
                nets.insert(text, true);
            }
        }
    }
    let mut result: Vec<String> = nets.into_keys().collect();
    result.sort_by(|a, b| alphanumeric_sort(a, b));
    result
}

/// Collect all unique power net names from power port records (RECORD=17).
fn collect_power_nets(doc: &SchDoc) -> Vec<String> {
    let mut nets: HashMap<String, bool> = HashMap::new();
    let all = collect_all_records(doc);
    for node in &all {
        if node.key == 17 {
            let rec = SchPowerRecord::from_origin(node.origin.clone());
            let text = rec.text();
            if !text.is_empty() {
                nets.insert(text, true);
            }
        }
    }
    let mut result: Vec<String> = nets.into_keys().collect();
    result.sort_by(|a, b| alphanumeric_sort(a, b));
    result
}

/// Extract component reference info from a component group.
fn extract_component_ref(group: &ComponentGroup) -> SchDocComponentRef {
    let rec = SchComponentRecord::from_origin(group.component.origin.clone());
    // NOTE: DESIGNATOR is stored on the component node but isn't part of the
    // typed SchComponentRecord API (it's architecturally a child record).
    // Raw access is intentional here.
    let designator = group
        .component
        .origin
        .as_param()
        .and_then(|p| p.params.get("DESIGNATOR"))
        .map(|v| v.as_str().to_string())
        .unwrap_or_default();
    SchDocComponentRef {
        designator,
        lib_reference: rec.lib_reference().to_string(),
        description: rec.component_description(),
    }
}

/// Check if a net name looks like a power rail.
fn is_power_rail(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.starts_with("vcc")
        || lower.starts_with("vdd")
        || lower.starts_with("v3")
        || lower.starts_with("v5")
        || lower.starts_with("v1")
        || lower.starts_with('+')
        || lower.contains("pwr")
        || lower.contains("supply")
        || lower.contains("vin")
        || lower.contains("vout")
        || lower.contains("vbat")
        || lower.contains("vbus")
        || lower.contains("avcc")
        || lower.contains("dvcc")
        || lower.contains("avdd")
        || lower.contains("dvdd")
}

/// Check if a net name looks like a ground reference.
fn is_ground_net(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower == "gnd"
        || lower == "agnd"
        || lower == "dgnd"
        || lower == "pgnd"
        || lower == "vss"
        || lower == "avss"
        || lower == "dvss"
        || lower.starts_with("gnd")
        || lower.contains("ground")
}

/// Check if a net name looks like a data bus signal.
fn is_data_bus(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.starts_with("d0")
        || lower.starts_with("d[")
        || lower.starts_with("data")
        || lower.starts_with("sd")
        || lower.starts_with("sda")
        || lower.starts_with("mosi")
        || lower.starts_with("miso")
}

/// Check if a net name looks like an address bus signal.
fn is_address_bus(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.starts_with("a0")
        || lower.starts_with("a[")
        || lower.starts_with("addr")
}

/// Check if a net name looks like a control signal.
fn is_control_signal(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.contains("cs")
        || lower.contains("ce")
        || lower.contains("we")
        || lower.contains("oe")
        || lower.contains("rd")
        || lower.contains("wr")
        || lower.contains("sel")
        || lower.contains("strobe")
}

// ═══════════════════════════════════════════════════════════════════════════
// COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// Returns a schematic overview with component categories, power architecture,
/// interfaces, key signals, and quick statistics.
pub fn cmd_overview(path: &Path) -> Result<SchDocOverview, Box<dyn std::error::Error>> {
    let doc = open_schdoc(path)?;

    // ─────────────────────────────────────────────────────────────────────────
    // 1. COMPONENTS BY CATEGORY
    // ─────────────────────────────────────────────────────────────────────────
    let mut categories: HashMap<&'static str, Vec<SchDocComponentRef>> = HashMap::new();

    for group in &doc.groups {
        let comp_ref = extract_component_ref(group);
        let category = categorize_component(&comp_ref.lib_reference, &comp_ref.description);
        categories.entry(category).or_default().push(comp_ref);
    }

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
            comps.sort_by(|a, b| alphanumeric_sort(&a.designator, &b.designator));
            components_by_category.push((category.to_string(), comps));
        }
    }
    for (category, mut comps) in categories {
        if !comps.is_empty() {
            comps.sort_by(|a, b| alphanumeric_sort(&a.designator, &b.designator));
            components_by_category.push((category.to_string(), comps));
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 2. POWER ARCHITECTURE
    // ─────────────────────────────────────────────────────────────────────────
    let all = collect_all_records(&doc);
    let mut power_net_counts: HashMap<String, usize> = HashMap::new();
    let mut ground_net_counts: HashMap<String, usize> = HashMap::new();

    // Count power ports (RECORD=17)
    for node in &all {
        if node.key == 17 {
            let rec = SchPowerRecord::from_origin(node.origin.clone());
            let text = rec.text();
            if !text.is_empty() {
                if is_ground_net(&text) {
                    *ground_net_counts.entry(text).or_insert(0) += 1;
                } else {
                    *power_net_counts.entry(text).or_insert(0) += 1;
                }
            }
        }
    }

    // Also check net labels that look like power/ground
    for node in &all {
        if node.key == 25 {
            let rec = SchNetLabelRecord::from_origin(node.origin.clone());
            let text = rec.text();
            if !text.is_empty() {
                if is_ground_net(&text) {
                    *ground_net_counts.entry(text).or_insert(0) += 1;
                } else if is_power_rail(&text) {
                    *power_net_counts.entry(text).or_insert(0) += 1;
                }
            }
        }
    }

    let mut power_rails: Vec<(String, usize)> = power_net_counts.into_iter().collect();
    power_rails.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| alphanumeric_sort(&a.0, &b.0)));

    let mut ground_nets: Vec<(String, usize)> = ground_net_counts.into_iter().collect();
    ground_nets.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| alphanumeric_sort(&a.0, &b.0)));

    let power_architecture = PowerArchitecture {
        power_rails,
        ground_nets,
    };

    // ─────────────────────────────────────────────────────────────────────────
    // 3. INTERFACES (from ports, RECORD=18)
    // ─────────────────────────────────────────────────────────────────────────
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    let mut bidirectional = Vec::new();
    let mut unspecified = Vec::new();

    for node in &all {
        if node.key == 18 {
            let rec = SchPortRecord::from_origin(node.origin.clone());
            let name = rec.name();
            let io_type = rec.io_type();
            if !name.is_empty() {
                match io_type {
                    1 => outputs.push(name),
                    2 => inputs.push(name),
                    3 => bidirectional.push(name),
                    _ => unspecified.push(name),
                }
            }
        }
    }

    inputs.sort_by(|a, b| alphanumeric_sort(a, b));
    outputs.sort_by(|a, b| alphanumeric_sort(a, b));
    bidirectional.sort_by(|a, b| alphanumeric_sort(a, b));
    unspecified.sort_by(|a, b| alphanumeric_sort(a, b));

    let has_ports = !inputs.is_empty()
        || !outputs.is_empty()
        || !bidirectional.is_empty()
        || !unspecified.is_empty();

    let interfaces = if has_ports {
        Some(InterfaceSummary {
            inputs,
            outputs,
            bidirectional,
            unspecified,
        })
    } else {
        None
    };

    // ─────────────────────────────────────────────────────────────────────────
    // 4. KEY SIGNALS
    // ─────────────────────────────────────────────────────────────────────────
    let unique_nets = collect_net_names(&doc);
    let mut data_buses = Vec::new();
    let mut address_buses = Vec::new();
    let mut control_signals = Vec::new();

    for net in &unique_nets {
        if is_data_bus(net) {
            data_buses.push(net.clone());
        } else if is_address_bus(net) {
            address_buses.push(net.clone());
        } else if is_control_signal(net) {
            control_signals.push(net.clone());
        }
    }

    let key_signals = KeySignals {
        total_unique_nets: unique_nets.len(),
        data_buses,
        address_buses,
        control_signals,
    };

    // ─────────────────────────────────────────────────────────────────────────
    // 5. QUICK STATS
    // ─────────────────────────────────────────────────────────────────────────
    let quick_stats = SchDocQuickStats {
        components: doc.groups.len(),
        wires: count_record_type(&doc, 27),
        junctions: count_record_type(&doc, 29),
        net_labels: count_record_type(&doc, 25),
        ports: count_record_type(&doc, 18),
        power_symbols: count_record_type(&doc, 17),
    };

    Ok(SchDocOverview {
        path: path.display().to_string(),
        sheet_size: get_sheet_size(&doc),
        components_by_category,
        power_architecture,
        interfaces,
        key_signals,
        quick_stats,
    })
}

/// Returns detailed sheet metadata, primitive summary, and net information.
pub fn cmd_info(path: &Path) -> Result<SchDocInfo, Box<dyn std::error::Error>> {
    let doc = open_schdoc(path)?;

    // ─────────────────────────────────────────────────────────────────────────
    // 1. SHEET INFO
    // ─────────────────────────────────────────────────────────────────────────
    let sheet_info = if let Some(sheet) = find_sheet_record(&doc) {
        let rec = SchSheetRecord::from_origin(sheet.origin.clone());
        let size_style = rec.sheet_style() as i32;
        let size = sheet_size_name(size_style).to_string();
        let fonts_defined = rec.font_id_count();

        let custom_dimensions = if size_style >= 18 {
            Some((
                format!("{:.1}", rec.custom_x().to_mils()),
                format!("{:.1}", rec.custom_y().to_mils()),
            ))
        } else {
            None
        };

        Some(SheetInfoDetails {
            size,
            size_style,
            custom_dimensions,
            fonts_defined,
        })
    } else {
        None
    };

    // ─────────────────────────────────────────────────────────────────────────
    // 2. PRIMITIVE SUMMARY
    // ─────────────────────────────────────────────────────────────────────────
    let wire_count = count_record_type(&doc, 27);
    let net_label_count = count_record_type(&doc, 25);
    let port_count = count_record_type(&doc, 18);
    let power_count = count_record_type(&doc, 17);
    let junction_count = count_record_type(&doc, 29);
    let pin_count = count_record_type(&doc, 2);

    let mut total_primitives = doc.groups.len(); // component records themselves
    for group in &doc.groups {
        total_primitives += group.children.len();
    }
    total_primitives += doc.orphan_records.len();

    let primitive_summary = PrimitiveSummary {
        total_primitives,
        components: doc.groups.len(),
        wires: wire_count,
        net_labels: net_label_count,
        ports: port_count,
        power_objects: power_count,
        junctions: junction_count,
        pins: pin_count,
    };

    // ─────────────────────────────────────────────────────────────────────────
    // 3. NETS
    // ─────────────────────────────────────────────────────────────────────────
    let unique_nets = collect_net_names(&doc);
    let power_nets = collect_power_nets(&doc);

    Ok(SchDocInfo {
        path: path.display().to_string(),
        sheet_info,
        primitive_summary,
        unique_nets,
        power_nets,
    })
}

/// Lists all placed components with their designators, references, and locations.
pub fn cmd_components(path: &Path) -> Result<SchDocComponentList, Box<dyn std::error::Error>> {
    let doc = open_schdoc(path)?;

    let mut components: Vec<SchDocComponentInfo> = doc
        .groups
        .iter()
        .map(|group| {
            let rec = SchComponentRecord::from_origin(group.component.origin.clone());
            // NOTE: DESIGNATOR is stored on the component node but isn't part of the
            // typed SchComponentRecord API (it's architecturally a child record).
            // Raw access is intentional here.
            let designator = group
                .component
                .origin
                .as_param()
                .and_then(|p| p.params.get("DESIGNATOR"))
                .map(|v| v.as_str().to_string())
                .unwrap_or_default();

            SchDocComponentInfo {
                designator,
                lib_reference: rec.lib_reference().to_string(),
                description: rec.component_description(),
                location: format_location(rec.location_x(), rec.location_y()),
                parts: rec.part_count() as i32,
                child_count: Some(group.children.len()),
            }
        })
        .collect();

    components.sort_by(|a, b| alphanumeric_sort(&a.designator, &b.designator));

    Ok(SchDocComponentList {
        path: path.display().to_string(),
        total_components: components.len(),
        components,
    })
}

/// Extracts net label information, optionally filtered by name pattern.
/// Labels are grouped by net name to show connectivity.
pub fn cmd_netlist(
    path: &Path,
    filter: Option<String>,
) -> Result<SchDocNetLabelList, Box<dyn std::error::Error>> {
    let doc = open_schdoc(path)?;
    let all = collect_all_records(&doc);

    let filter_lower = filter.as_ref().map(|f| f.to_lowercase());

    // Collect all net labels (RECORD=25)
    let mut net_labels: Vec<NetLabelInfo> = Vec::new();
    let mut net_counts: HashMap<String, usize> = HashMap::new();

    for node in &all {
        if node.key == 25 {
            let rec = SchNetLabelRecord::from_origin(node.origin.clone());
            let text = rec.text();
            if text.is_empty() {
                continue;
            }

            // Apply filter
            if let Some(ref pattern) = filter_lower {
                if !text.to_lowercase().contains(pattern) {
                    continue;
                }
            }

            *net_counts.entry(text.clone()).or_insert(0) += 1;
            net_labels.push(NetLabelInfo {
                net_name: text,
                location: format_location(rec.location_x(), rec.location_y()),
            });
        }
    }

    net_labels.sort_by(|a, b| {
        alphanumeric_sort(&a.net_name, &b.net_name)
            .then_with(|| a.location.cmp(&b.location))
    });

    let total = net_labels.len();

    // Group by name for summary
    let mut grouped: Vec<(String, usize)> = net_counts.into_iter().collect();
    grouped.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| alphanumeric_sort(&a.0, &b.0)));

    Ok(SchDocNetLabelList {
        path: path.display().to_string(),
        total_net_labels: total,
        group_by_name: true,
        grouped: Some(grouped),
        individual: Some(net_labels),
    })
}

/// Lists all wire primitives (RECORD=27) with start/end coordinates.
pub fn cmd_wires(path: &Path) -> Result<SchDocWireList, Box<dyn std::error::Error>> {
    let doc = open_schdoc(path)?;
    let all = collect_all_records(&doc);

    let mut wires: Vec<WireInfo> = Vec::new();
    let mut index = 0;

    for node in &all {
        if node.key == 27 {
            // NOTE: Raw param access is intentional -- SchWireRecord's vertex
            // coordinates (LOCATION.X/Y, CORNER.X/Y) are `#[altium(skip)]`
            // and not yet covered by the typed API.
            let x1 = node
                .origin
                .as_param()
                .and_then(|p| p.params.get("LOCATION.X"))
                .map(|v| v.as_int_or(0))
                .unwrap_or(0);
            let y1 = node
                .origin
                .as_param()
                .and_then(|p| p.params.get("LOCATION.Y"))
                .map(|v| v.as_int_or(0))
                .unwrap_or(0);
            let x2 = node
                .origin
                .as_param()
                .and_then(|p| p.params.get("CORNER.X"))
                .map(|v| v.as_int_or(0))
                .unwrap_or(0);
            let y2 = node
                .origin
                .as_param()
                .and_then(|p| p.params.get("CORNER.Y"))
                .map(|v| v.as_int_or(0))
                .unwrap_or(0);

            wires.push(WireInfo {
                index,
                start: format!(
                    "({:.1}, {:.1})",
                    x1 as f64 / 100000.0,
                    y1 as f64 / 100000.0
                ),
                end_or_segments: format!(
                    "({:.1}, {:.1})",
                    x2 as f64 / 100000.0,
                    y2 as f64 / 100000.0
                ),
            });
            index += 1;
        }
    }

    Ok(SchDocWireList {
        path: path.display().to_string(),
        total_wires: wires.len(),
        wires,
    })
}

/// Lists all port definitions (RECORD=18) for hierarchical design analysis.
pub fn cmd_ports(path: &Path) -> Result<SchDocPortList, Box<dyn std::error::Error>> {
    let doc = open_schdoc(path)?;
    let all = collect_all_records(&doc);

    let mut ports: Vec<PortInfo> = Vec::new();

    for node in &all {
        if node.key == 18 {
            let rec = SchPortRecord::from_origin(node.origin.clone());
            let name = rec.name();
            let io_type = rec.io_type();

            ports.push(PortInfo {
                name,
                io_type: port_io_type_name(io_type).to_string(),
                location: format_location(rec.location_x(), rec.location_y()),
            });
        }
    }

    ports.sort_by(|a, b| alphanumeric_sort(&a.name, &b.name));

    Ok(SchDocPortList {
        path: path.display().to_string(),
        total_ports: ports.len(),
        ports,
    })
}

/// Analyzes power distribution by listing all power port objects (RECORD=17),
/// grouped by net name.
pub fn cmd_power_map(path: &Path) -> Result<SchDocPowerList, Box<dyn std::error::Error>> {
    let doc = open_schdoc(path)?;
    let all = collect_all_records(&doc);

    let mut power_objects: Vec<PowerObjectInfo> = Vec::new();
    let mut net_counts: HashMap<String, usize> = HashMap::new();

    for node in &all {
        if node.key == 17 {
            let rec = SchPowerRecord::from_origin(node.origin.clone());
            let text = rec.text();
            let style = rec.style();

            if !text.is_empty() {
                *net_counts.entry(text.clone()).or_insert(0) += 1;
            }

            power_objects.push(PowerObjectInfo {
                net: text,
                style: power_style_name(style).to_string(),
                location: format_location(rec.location_x(), rec.location_y()),
            });
        }
    }

    power_objects.sort_by(|a, b| {
        alphanumeric_sort(&a.net, &b.net).then_with(|| a.location.cmp(&b.location))
    });

    let mut grouped: Vec<(String, usize)> = net_counts.into_iter().collect();
    grouped.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| alphanumeric_sort(&a.0, &b.0)));

    Ok(SchDocPowerList {
        path: path.display().to_string(),
        total_power_objects: power_objects.len(),
        group_by_net: true,
        grouped: Some(grouped),
        individual: Some(power_objects),
    })
}

/// Serializes the schematic document to JSON for LLM processing or external analysis.
pub fn cmd_json(
    path: &Path,
    full: bool,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let doc = open_schdoc(path)?;

    if full {
        Ok(serde_json::to_value(&doc)?)
    } else {
        // Summarized output
        let mut components: Vec<serde_json::Value> = doc
            .groups
            .iter()
            .map(|group| {
                let rec = SchComponentRecord::from_origin(group.component.origin.clone());
                // NOTE: DESIGNATOR is stored on the component node but isn't part of the
                // typed SchComponentRecord API (it's architecturally a child record).
                // Raw access is intentional here.
                let designator = group
                    .component
                    .origin
                    .as_param()
                    .and_then(|p| p.params.get("DESIGNATOR"))
                    .map(|v| v.as_str().to_string())
                    .unwrap_or_default();
                let pin_count = group.children.iter().filter(|c| c.key == 2).count();
                let child_count = group.children.len();

                serde_json::json!({
                    "designator": designator,
                    "lib_reference": rec.lib_reference().to_string(),
                    "pin_count": pin_count,
                    "child_count": child_count,
                })
            })
            .collect();

        components.sort_by(|a, b| {
            let ad = a["designator"].as_str().unwrap_or("");
            let bd = b["designator"].as_str().unwrap_or("");
            alphanumeric_sort(ad, bd)
        });

        let net_names = collect_net_names(&doc);
        let power_nets = collect_power_nets(&doc);

        Ok(serde_json::json!({
            "file": path.display().to_string(),
            "sheet_size": get_sheet_size(&doc),
            "component_count": doc.groups.len(),
            "wire_count": count_record_type(&doc, 27),
            "net_label_count": count_record_type(&doc, 25),
            "port_count": count_record_type(&doc, 18),
            "power_port_count": count_record_type(&doc, 17),
            "junction_count": count_record_type(&doc, 29),
            "components": components,
            "unique_nets": net_names,
            "power_nets": power_nets,
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
        let mut items = vec!["R10", "R2", "R1", "C1"];
        items.sort_by(|a, b| alphanumeric_sort(a, b));
        assert_eq!(items, vec!["C1", "R1", "R2", "R10"]);
    }

    #[test]
    fn test_schcoord_to_mils() {
        assert!((SchCoord::from_raw(10000000).to_mils() - 100.0).abs() < 0.001);
        assert!((SchCoord::from_raw(0).to_mils() - 0.0).abs() < 0.001);
        assert!((SchCoord::from_raw(-5000000).to_mils() - (-50.0)).abs() < 0.001);
    }

    #[test]
    fn test_sheet_size_name() {
        assert_eq!(sheet_size_name(0), "A4");
        assert_eq!(sheet_size_name(5), "A");
        assert_eq!(sheet_size_name(10), "Letter");
        assert_eq!(sheet_size_name(99), "Custom");
    }

    #[test]
    fn test_power_style_name() {
        assert_eq!(power_style_name(0), "Circle");
        assert_eq!(power_style_name(4), "Power Ground");
        assert_eq!(power_style_name(99), "Unknown");
    }

    #[test]
    fn test_port_io_type_name() {
        assert_eq!(port_io_type_name(0), "Unspecified");
        assert_eq!(port_io_type_name(1), "Output");
        assert_eq!(port_io_type_name(2), "Input");
        assert_eq!(port_io_type_name(3), "Bidirectional");
    }

    #[test]
    fn test_is_power_rail() {
        assert!(is_power_rail("VCC"));
        assert!(is_power_rail("VDD"));
        assert!(is_power_rail("+3V3"));
        assert!(is_power_rail("VBAT"));
        assert!(!is_power_rail("GND"));
        assert!(!is_power_rail("SDA"));
    }

    #[test]
    fn test_is_ground_net() {
        assert!(is_ground_net("GND"));
        assert!(is_ground_net("AGND"));
        assert!(is_ground_net("DGND"));
        assert!(is_ground_net("VSS"));
        assert!(!is_ground_net("VCC"));
        assert!(!is_ground_net("CLK"));
    }

    #[test]
    fn test_is_data_bus() {
        assert!(is_data_bus("D0"));
        assert!(is_data_bus("D[7:0]"));
        assert!(is_data_bus("DATA_IN"));
        assert!(is_data_bus("SDA"));
        assert!(is_data_bus("MOSI"));
        assert!(!is_data_bus("CLK"));
    }

    #[test]
    fn test_is_address_bus() {
        assert!(is_address_bus("A0"));
        assert!(is_address_bus("A[15:0]"));
        assert!(is_address_bus("ADDR0"));
        assert!(!is_address_bus("DATA"));
    }

    #[test]
    fn test_is_control_signal() {
        assert!(is_control_signal("CS"));
        assert!(is_control_signal("nWE"));
        assert!(is_control_signal("STROBE"));
        assert!(!is_control_signal("VCC"));
    }

    #[test]
    fn test_format_location() {
        assert_eq!(
            format_location(SchCoord::from_raw(10000000), SchCoord::from_raw(20000000)),
            "(100.0, 200.0)"
        );
        assert_eq!(
            format_location(SchCoord::from_raw(0), SchCoord::from_raw(0)),
            "(0.0, 0.0)"
        );
    }
}
