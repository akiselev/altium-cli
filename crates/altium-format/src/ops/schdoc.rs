// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Schematic document operations.
//!
//! Provides high-level operations for exploring Altium schematic document (.SchDoc) files.
//! Mirrors the schlib/pcblib module patterns to maintain consistency across the codebase.
//!
//! **V2 Migration**: This module uses the v2 SchDoc types which provide typed record access.

// cmd_* functions mix presentation and business logic; separation punted until usage patterns clarify abstraction boundaries (premature abstraction risk)

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use crate::v2::fields::{
    ComponentData, NetLabelData, PinData, PortData, PowerData, TypedRecord, WireData,
};
use crate::v2::io::schdoc::SchDocV2;
use crate::v2::types::{PinElectrical, PortIO, PowerObjectStyle, SheetStyle};

use crate::ops::categorization::categorize_component;
use crate::ops::output::*;

// ===========================================================================
// HELPER FUNCTIONS
// ===========================================================================

/// Sorts strings with embedded numbers naturally (e.g., "A2" < "A10").
///
/// TODO: Consolidate with schlib/pcblib::alphanumeric_sort after all 4 ops modules exist
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

/// Opens and parses a SchDoc file from the given path.
/// Returns the parsed SchDocV2 structure or an error if the file cannot be read.
pub fn open_schdoc(path: &Path) -> Result<SchDocV2, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    Ok(SchDocV2::open(BufReader::new(file)).map_err(|e| e.to_string())?)
}

/// Convert schematic coordinates to mils for display.
/// SchDoc coordinates are stored as value * 100000.
fn coord_to_mils(value: i32) -> f64 {
    value as f64 / 100000.0
}

/// Format a coordinate pair as a string.
fn format_coord(x: i32, y: i32) -> String {
    format!("({:.1}, {:.1})", coord_to_mils(x), coord_to_mils(y))
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

/// Get port I/O type name for display.
fn port_io_name(io_type: PortIO) -> &'static str {
    match io_type {
        PortIO::Unspecified => "Unspecified",
        PortIO::Output => "Output",
        PortIO::Input => "Input",
        PortIO::Bidirectional => "Bidirectional",
    }
}

/// Get power object style name for display.
fn power_style_name(style: PowerObjectStyle) -> &'static str {
    match style {
        PowerObjectStyle::Circle => "Circle",
        PowerObjectStyle::Arrow => "Arrow",
        PowerObjectStyle::Bar => "Bar",
        PowerObjectStyle::Wave => "Wave",
        PowerObjectStyle::GndPower => "GND (Power)",
        PowerObjectStyle::GndSignal => "GND (Signal)",
        PowerObjectStyle::GndEarth => "GND (Earth)",
        PowerObjectStyle::GOSTArrow => "GOST Arrow",
        PowerObjectStyle::GOSTGndPower => "GOST GND Power",
        PowerObjectStyle::GOSTGndEarth => "GOST GND Earth",
        PowerObjectStyle::GOSTBar => "GOST Bar",
    }
}

/// Get sheet size name from sheet style.
fn sheet_size_name(style: u8) -> &'static str {
    match SheetStyle::from_u8(style) {
        Some(SheetStyle::A4) => "A4",
        Some(SheetStyle::A3) => "A3",
        Some(SheetStyle::A2) => "A2",
        Some(SheetStyle::A1) => "A1",
        Some(SheetStyle::A0) => "A0",
        Some(SheetStyle::A) => "A (ANSI)",
        Some(SheetStyle::B) => "B (ANSI)",
        Some(SheetStyle::C) => "C (ANSI)",
        Some(SheetStyle::D) => "D (ANSI)",
        Some(SheetStyle::E) => "E (ANSI)",
        Some(SheetStyle::Letter) => "Letter",
        Some(SheetStyle::Legal) => "Legal",
        Some(SheetStyle::Tabloid) => "Tabloid",
        Some(SheetStyle::OrcadA) => "OrCAD A",
        Some(SheetStyle::OrcadB) => "OrCAD B",
        Some(SheetStyle::OrcadC) => "OrCAD C",
        Some(SheetStyle::OrcadD) => "OrCAD D",
        Some(SheetStyle::OrcadE) => "OrCAD E",
        None => "Custom",
    }
}

/// Check if a net name is a power rail.
fn is_power_net(name: &str) -> bool {
    let name_upper = name.to_uppercase();
    name_upper.starts_with("VCC")
        || name_upper.starts_with("VDD")
        || name_upper.starts_with("+3V")
        || name_upper.starts_with("+5V")
        || name_upper.starts_with("+12V")
        || name_upper.starts_with("3.3V")
        || name_upper.starts_with("5V")
        || name_upper.starts_with("12V")
        || name_upper.starts_with("1.8V")
        || name_upper.starts_with("VBAT")
        || name_upper.starts_with("VCAP")
        || name_upper.starts_with("VREF")
        || name_upper.starts_with("AVCC")
        || name_upper.starts_with("AVDD")
        || name_upper.starts_with("DVCC")
        || name_upper.starts_with("DVDD")
        || name_upper.starts_with("VBUS")
        || name_upper.starts_with("VIN")
        || name_upper.starts_with("VOUT")
}

/// Check if a net name is a ground net.
fn is_ground_net(name: &str) -> bool {
    let name_upper = name.to_uppercase();
    name_upper == "GND"
        || name_upper == "AGND"
        || name_upper == "DGND"
        || name_upper == "VSS"
        || name_upper == "AVSS"
        || name_upper == "DVSS"
        || name_upper.starts_with("GND_")
        || name_upper.ends_with("_GND")
        || name_upper == "PGND"
        || name_upper == "SGND"
        || name_upper == "GROUND"
}

/// Extract designator from component data if present.
/// Note: ComponentData stores designator in a separate Designator record,
/// so we use unique_id or a placeholder if not available.
fn get_component_designator(comp: &ComponentData) -> String {
    // ComponentData doesn't store the designator directly - it's in a child Designator record.
    // We use the unique_id or generate a placeholder based on index.
    if !comp.unique_id.is_empty() {
        // Try to extract designator pattern from unique_id if present
        comp.unique_id.clone()
    } else {
        format!("COMP_{}", comp.graphical.base.index_in_sheet)
    }
}

/// Collect all component records from the document.
fn collect_components(doc: &SchDocV2) -> Vec<&ComponentData> {
    doc.typed_records
        .iter()
        .filter_map(|r| match r {
            TypedRecord::Component(c) => Some(c),
            _ => None,
        })
        .collect()
}

/// Collect all pin records from the document.
fn collect_pins(doc: &SchDocV2) -> Vec<&PinData> {
    doc.typed_records
        .iter()
        .filter_map(|r| match r {
            TypedRecord::Pin(p) => Some(p),
            _ => None,
        })
        .collect()
}

/// Collect all wire records from the document.
fn collect_wires(doc: &SchDocV2) -> Vec<&WireData> {
    doc.typed_records
        .iter()
        .filter_map(|r| match r {
            TypedRecord::Wire(w) => Some(w),
            _ => None,
        })
        .collect()
}

/// Collect all net label records from the document.
fn collect_net_labels(doc: &SchDocV2) -> Vec<&NetLabelData> {
    doc.typed_records
        .iter()
        .filter_map(|r| match r {
            TypedRecord::NetLabel(n) => Some(n),
            _ => None,
        })
        .collect()
}

/// Collect all port records from the document.
fn collect_ports(doc: &SchDocV2) -> Vec<&PortData> {
    doc.typed_records
        .iter()
        .filter_map(|r| match r {
            TypedRecord::Port(p) => Some(p),
            _ => None,
        })
        .collect()
}

/// Collect all power object records from the document.
fn collect_power_objects(doc: &SchDocV2) -> Vec<&PowerData> {
    doc.typed_records
        .iter()
        .filter_map(|r| match r {
            TypedRecord::PowerObject(p) => Some(p),
            _ => None,
        })
        .collect()
}

/// Count record types in the document.
fn count_record_types(doc: &SchDocV2) -> HashMap<&'static str, usize> {
    let mut counts: HashMap<&'static str, usize> = HashMap::new();

    for record in &doc.typed_records {
        let name = match record {
            TypedRecord::Component(_) => "Component",
            TypedRecord::Pin(_) => "Pin",
            TypedRecord::Wire(_) => "Wire",
            TypedRecord::NetLabel(_) => "NetLabel",
            TypedRecord::Port(_) => "Port",
            TypedRecord::PowerObject(_) => "PowerObject",
            TypedRecord::Junction(_) => "Junction",
            TypedRecord::Bus(_) => "Bus",
            TypedRecord::BusEntry(_) => "BusEntry",
            TypedRecord::Rectangle(_) => "Rectangle",
            TypedRecord::Line(_) => "Line",
            TypedRecord::Arc(_) => "Arc",
            TypedRecord::Polygon(_) => "Polygon",
            TypedRecord::Polyline(_) => "Polyline",
            TypedRecord::Label(_) => "Label",
            TypedRecord::TextFrame(_) => "TextFrame",
            TypedRecord::Sheet(_) => "Sheet",
            TypedRecord::SheetSymbol(_) => "SheetSymbol",
            TypedRecord::SheetEntry(_) => "SheetEntry",
            TypedRecord::SheetName(_) => "SheetName",
            TypedRecord::SheetFileName(_) => "SheetFileName",
            TypedRecord::Parameter(_) => "Parameter",
            TypedRecord::Designator(_) => "Designator",
            TypedRecord::Implementation(_) => "Implementation",
            TypedRecord::ImplementationList(_) => "ImplementationList",
            TypedRecord::NoERC(_) => "NoERC",
            TypedRecord::Image(_) => "Image",
            TypedRecord::Symbol(_) => "Symbol",
            TypedRecord::Ellipse(_) => "Ellipse",
            TypedRecord::Bezier(_) => "Bezier",
            TypedRecord::EllipticalArc(_) => "EllipticalArc",
            TypedRecord::Pie(_) => "Pie",
            TypedRecord::RoundRectangle(_) => "RoundRectangle",
            TypedRecord::Note(_) => "Note",
            TypedRecord::Blanket(_) => "Blanket",
            TypedRecord::Unknown(_) => "Unknown",
        };
        *counts.entry(name).or_insert(0) += 1;
    }

    counts
}

// ===========================================================================
// BROWSE COMMANDS
// ===========================================================================

/// Returns document overview with statistics and component categorization.
pub fn cmd_overview(path: &Path) -> Result<SchDocOverview, Box<dyn std::error::Error>> {
    let doc = open_schdoc(path)?;

    // Get sheet size
    let sheet_size = doc
        .sheet()
        .map(|s| sheet_size_name(s.sheet_style).to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    // Collect components and categorize
    let components = collect_components(&doc);
    let mut categories: HashMap<&'static str, Vec<SchDocComponentRef>> = HashMap::new();

    for comp in &components {
        let designator = get_component_designator(comp);
        let category = categorize_component(&comp.lib_reference, &comp.component_description);
        categories
            .entry(category)
            .or_default()
            .push(SchDocComponentRef {
                designator,
                lib_reference: comp.lib_reference.clone(),
                description: comp.component_description.clone(),
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
            comps.sort_by(|a, b| alphanumeric_sort(&a.designator, &b.designator));
            components_by_category.push((category.to_string(), comps));
        }
    }

    // Add any remaining categories
    for (category, mut comps) in categories {
        if !comps.is_empty() {
            comps.sort_by(|a, b| alphanumeric_sort(&a.designator, &b.designator));
            components_by_category.push((category.to_string(), comps));
        }
    }

    // Power architecture analysis
    let power_objects = collect_power_objects(&doc);
    let net_labels = collect_net_labels(&doc);

    let mut power_rail_counts: HashMap<String, usize> = HashMap::new();
    let mut ground_net_counts: HashMap<String, usize> = HashMap::new();

    // Count power objects by net name
    for power in &power_objects {
        let net = power.text.clone();
        if is_power_net(&net) {
            *power_rail_counts.entry(net).or_insert(0) += 1;
        } else if is_ground_net(&net) {
            *ground_net_counts.entry(net).or_insert(0) += 1;
        }
    }

    // Also check net labels for power/ground
    for label in &net_labels {
        if is_power_net(&label.text) {
            *power_rail_counts.entry(label.text.clone()).or_insert(0) += 1;
        } else if is_ground_net(&label.text) {
            *ground_net_counts.entry(label.text.clone()).or_insert(0) += 1;
        }
    }

    let mut power_rails: Vec<(String, usize)> = power_rail_counts.into_iter().collect();
    power_rails.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let mut ground_nets: Vec<(String, usize)> = ground_net_counts.into_iter().collect();
    ground_nets.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    // Interface summary from ports
    let ports = collect_ports(&doc);
    let interfaces = if !ports.is_empty() {
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        let mut bidirectional = Vec::new();
        let mut unspecified = Vec::new();

        for port in &ports {
            match port.io_type {
                PortIO::Input => inputs.push(port.name.clone()),
                PortIO::Output => outputs.push(port.name.clone()),
                PortIO::Bidirectional => bidirectional.push(port.name.clone()),
                PortIO::Unspecified => unspecified.push(port.name.clone()),
            }
        }

        Some(InterfaceSummary {
            inputs,
            outputs,
            bidirectional,
            unspecified,
        })
    } else {
        None
    };

    // Key signals analysis
    let mut unique_nets: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut data_buses = Vec::new();
    let mut address_buses = Vec::new();
    let mut control_signals = Vec::new();

    for label in &net_labels {
        unique_nets.insert(label.text.clone());
        let text_upper = label.text.to_uppercase();
        if text_upper.contains("DATA") || text_upper.starts_with('D') && text_upper.len() <= 3 {
            if !data_buses.contains(&label.text) {
                data_buses.push(label.text.clone());
            }
        }
        if text_upper.contains("ADDR") || text_upper.starts_with('A') && text_upper.len() <= 3 {
            if !address_buses.contains(&label.text) {
                address_buses.push(label.text.clone());
            }
        }
        if text_upper.contains("CLK")
            || text_upper.contains("CS")
            || text_upper.contains("RST")
            || text_upper.contains("EN")
            || text_upper.contains("IRQ")
            || text_upper.contains("INT")
        {
            if !control_signals.contains(&label.text) {
                control_signals.push(label.text.clone());
            }
        }
    }

    data_buses.sort();
    address_buses.sort();
    control_signals.sort();

    // Quick stats
    let wires = collect_wires(&doc);
    let junctions: Vec<_> = doc
        .typed_records
        .iter()
        .filter(|r| matches!(r, TypedRecord::Junction(_)))
        .collect();

    let quick_stats = SchDocQuickStats {
        components: components.len(),
        wires: wires.len(),
        junctions: junctions.len(),
        net_labels: net_labels.len(),
        ports: ports.len(),
        power_symbols: power_objects.len(),
    };

    Ok(SchDocOverview {
        path: path.display().to_string(),
        sheet_size,
        components_by_category,
        power_architecture: PowerArchitecture {
            power_rails,
            ground_nets,
        },
        interfaces,
        key_signals: KeySignals {
            total_unique_nets: unique_nets.len(),
            data_buses,
            address_buses,
            control_signals,
        },
        quick_stats,
    })
}

/// Returns detailed sheet metadata and properties.
pub fn cmd_info(path: &Path) -> Result<SchDocInfo, Box<dyn std::error::Error>> {
    let doc = open_schdoc(path)?;

    // Sheet info
    let sheet_info = doc.sheet().map(|s| {
        let size = sheet_size_name(s.sheet_style).to_string();
        let custom_dimensions = if s.use_custom_sheet {
            Some((
                format!("{:.1}", coord_to_mils(s.custom_x)),
                format!("{:.1}", coord_to_mils(s.custom_y)),
            ))
        } else {
            None
        };
        SheetInfoDetails {
            size,
            size_style: s.sheet_style as i32,
            custom_dimensions,
            fonts_defined: s.font_id_count,
        }
    });

    // Primitive summary
    let record_counts = count_record_types(&doc);
    let total_primitives = doc.typed_records.len();
    let components = *record_counts.get("Component").unwrap_or(&0);
    let wires = *record_counts.get("Wire").unwrap_or(&0);
    let net_labels_count = *record_counts.get("NetLabel").unwrap_or(&0);
    let ports = *record_counts.get("Port").unwrap_or(&0);
    let power_objects = *record_counts.get("PowerObject").unwrap_or(&0);
    let junctions = *record_counts.get("Junction").unwrap_or(&0);
    let pins = *record_counts.get("Pin").unwrap_or(&0);

    // Unique nets
    let net_labels = collect_net_labels(&doc);
    let mut unique_nets: Vec<String> = net_labels
        .iter()
        .map(|n| n.text.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    unique_nets.sort();

    // Power nets
    let power_objs = collect_power_objects(&doc);
    let mut power_nets: Vec<String> = power_objs
        .iter()
        .map(|p| p.text.clone())
        .filter(|t| is_power_net(t) || is_ground_net(t))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    power_nets.sort();

    Ok(SchDocInfo {
        path: path.display().to_string(),
        sheet_info,
        primitive_summary: PrimitiveSummary {
            total_primitives,
            components,
            wires,
            net_labels: net_labels_count,
            ports,
            power_objects,
            junctions,
            pins,
        },
        unique_nets,
        power_nets,
    })
}

/// Lists all placed components in the document.
pub fn cmd_components(path: &Path) -> Result<SchDocComponentList, Box<dyn std::error::Error>> {
    let doc = open_schdoc(path)?;

    let components = collect_components(&doc);

    let mut component_list: Vec<SchDocComponentInfo> = components
        .iter()
        .map(|c| {
            let designator = get_component_designator(c);
            SchDocComponentInfo {
                designator,
                lib_reference: c.lib_reference.clone(),
                description: c.component_description.clone(),
                location: format_coord(c.location_x, c.location_y),
                parts: c.part_count as i32,
                child_count: None,
            }
        })
        .collect();

    component_list.sort_by(|a, b| alphanumeric_sort(&a.designator, &b.designator));

    Ok(SchDocComponentList {
        path: path.display().to_string(),
        total_components: component_list.len(),
        components: component_list,
    })
}

/// Extracts net connectivity information.
pub fn cmd_netlist(
    path: &Path,
    filter: Option<String>,
) -> Result<SchDocNetlist, Box<dyn std::error::Error>> {
    let doc = open_schdoc(path)?;

    // Build net connectivity from net labels, ports, and power objects
    let net_labels = collect_net_labels(&doc);
    let ports = collect_ports(&doc);
    let power_objects = collect_power_objects(&doc);

    // Collect unique net names
    let mut net_names: std::collections::HashSet<String> = std::collections::HashSet::new();

    for label in &net_labels {
        net_names.insert(label.text.clone());
    }
    for port in &ports {
        net_names.insert(port.name.clone());
    }
    for power in &power_objects {
        net_names.insert(power.text.clone());
    }

    // Apply filter if provided
    let filter_lower = filter.as_ref().map(|f| f.to_lowercase());
    let filtered_nets: Vec<String> = if let Some(ref f) = filter_lower {
        net_names
            .into_iter()
            .filter(|n| n.to_lowercase().contains(f))
            .collect()
    } else {
        net_names.into_iter().collect()
    };

    // Build connection info for each net
    let mut nets: Vec<NetConnection> = Vec::new();

    for net_name in &filtered_nets {
        let mut connections = Vec::new();

        // Find net labels at this net
        for label in &net_labels {
            if &label.text == net_name {
                connections.push(format!(
                    "NetLabel at {}",
                    format_coord(label.location_x, label.location_y)
                ));
            }
        }

        // Find ports at this net
        for port in &ports {
            if &port.name == net_name {
                connections.push(format!(
                    "Port {} ({}) at {}",
                    port.name,
                    port_io_name(port.io_type),
                    format_coord(port.location_x, port.location_y)
                ));
            }
        }

        // Find power objects at this net
        for power in &power_objects {
            if &power.text == net_name {
                connections.push(format!(
                    "Power {} at {}",
                    power_style_name(power.style),
                    format_coord(power.location_x, power.location_y)
                ));
            }
        }

        if !connections.is_empty() {
            nets.push(NetConnection {
                net_name: net_name.clone(),
                connections,
            });
        }
    }

    nets.sort_by(|a, b| alphanumeric_sort(&a.net_name, &b.net_name));

    Ok(SchDocNetlist {
        path: path.display().to_string(),
        filter,
        min_connections: 1,
        total_nets: nets.len(),
        nets,
    })
}

/// Lists wire primitives for routing analysis.
pub fn cmd_wires(path: &Path) -> Result<SchDocWireList, Box<dyn std::error::Error>> {
    let doc = open_schdoc(path)?;

    let wires = collect_wires(&doc);

    let wire_list: Vec<WireInfo> = wires
        .iter()
        .enumerate()
        .map(|(i, w)| {
            let start = if !w.vertices.is_empty() {
                format_coord(w.vertices[0].0, w.vertices[0].1)
            } else {
                "(?, ?)".to_string()
            };

            let end_or_segments = if w.vertices.len() == 2 {
                format_coord(w.vertices[1].0, w.vertices[1].1)
            } else if w.vertices.len() > 2 {
                format!("{} vertices", w.vertices.len())
            } else {
                "single point".to_string()
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
        total_wires: wire_list.len(),
        wires: wire_list,
    })
}

/// Lists port definitions for hierarchical designs.
pub fn cmd_ports(path: &Path) -> Result<SchDocPortList, Box<dyn std::error::Error>> {
    let doc = open_schdoc(path)?;

    let ports = collect_ports(&doc);

    let mut port_list: Vec<PortInfo> = ports
        .iter()
        .map(|p| PortInfo {
            name: p.name.clone(),
            io_type: port_io_name(p.io_type).to_string(),
            location: format_coord(p.location_x, p.location_y),
        })
        .collect();

    port_list.sort_by(|a, b| alphanumeric_sort(&a.name, &b.name));

    Ok(SchDocPortList {
        path: path.display().to_string(),
        total_ports: port_list.len(),
        ports: port_list,
    })
}

/// Analyzes power distribution and connections.
pub fn cmd_power_map(path: &Path) -> Result<SchDocPowerMap, Box<dyn std::error::Error>> {
    let doc = open_schdoc(path)?;

    let power_objects = collect_power_objects(&doc);
    let components = collect_components(&doc);
    let pins = collect_pins(&doc);

    // Group power objects by net name
    let mut power_rail_map: HashMap<String, Vec<&PowerData>> = HashMap::new();
    let mut ground_map: HashMap<String, Vec<&PowerData>> = HashMap::new();

    for power in &power_objects {
        if is_ground_net(&power.text) {
            ground_map.entry(power.text.clone()).or_default().push(power);
        } else {
            power_rail_map
                .entry(power.text.clone())
                .or_default()
                .push(power);
        }
    }

    // Find power pins on components
    let mut power_pin_components: HashMap<String, usize> = HashMap::new();
    for pin in &pins {
        if pin.electrical == PinElectrical::Power {
            let comp_name = format!("Component {}", pin.owner_index);
            *power_pin_components.entry(comp_name).or_insert(0) += 1;
        }
    }

    // Build power rails output
    let mut power_rails: Vec<PowerRail> = power_rail_map
        .into_iter()
        .map(|(net_name, objs)| PowerRail {
            net_name,
            symbol_count: objs.len(),
            consumers: Vec::new(), // Would need netlist traversal
        })
        .collect();
    power_rails.sort_by(|a, b| alphanumeric_sort(&a.net_name, &b.net_name));

    // Build ground nets output
    let mut ground_nets: Vec<GroundNet> = ground_map
        .into_iter()
        .map(|(net_name, objs)| GroundNet {
            net_name,
            symbol_count: objs.len(),
        })
        .collect();
    ground_nets.sort_by(|a, b| alphanumeric_sort(&a.net_name, &b.net_name));

    // Build powered components list
    let mut powered_components: Vec<PoweredComponent> = components
        .iter()
        .filter_map(|c| {
            let designator = get_component_designator(c);
            // Count power pins for this component
            let power_pin_count = pins
                .iter()
                .filter(|p| {
                    p.owner_index == c.graphical.base.index_in_sheet
                        && p.electrical == PinElectrical::Power
                })
                .count();

            if power_pin_count > 0 {
                Some(PoweredComponent {
                    designator,
                    lib_reference: c.lib_reference.clone(),
                    power_pin_count,
                })
            } else {
                None
            }
        })
        .collect();

    powered_components.sort_by(|a, b| alphanumeric_sort(&a.designator, &b.designator));

    Ok(SchDocPowerMap {
        path: path.display().to_string(),
        power_rails,
        ground_nets,
        powered_components,
    })
}

// ===========================================================================
// EXPORT COMMANDS
// ===========================================================================

/// Serializes the document to JSON for LLM processing or external analysis.
pub fn cmd_json(path: &Path, full: bool) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let doc = open_schdoc(path)?;

    // Sheet info
    let sheet = doc.sheet().map(|s| JsonSheetInfo {
        size: sheet_size_name(s.sheet_style).to_string(),
        fonts: s.font_id_count,
    });

    // Record counts
    let record_counts = count_record_types(&doc);
    let total_primitives = doc.typed_records.len();

    let summary = JsonDocSummary {
        total_primitives,
        components: *record_counts.get("Component").unwrap_or(&0),
        wires: *record_counts.get("Wire").unwrap_or(&0),
        net_labels: *record_counts.get("NetLabel").unwrap_or(&0),
        ports: *record_counts.get("Port").unwrap_or(&0),
        power_objects: *record_counts.get("PowerObject").unwrap_or(&0),
        junctions: *record_counts.get("Junction").unwrap_or(&0),
        pins: *record_counts.get("Pin").unwrap_or(&0),
    };

    if full {
        // Full export with all details
        let components = collect_components(&doc);
        let pins = collect_pins(&doc);
        let net_labels = collect_net_labels(&doc);
        let ports = collect_ports(&doc);
        let power_objects = collect_power_objects(&doc);

        // Build component info with pins
        let component_info: Vec<JsonComponentInfo> = components
            .iter()
            .map(|c| {
                let designator = get_component_designator(c);
                let comp_pins: Vec<JsonPinInfo> = pins
                    .iter()
                    .filter(|p| p.owner_index == c.graphical.base.index_in_sheet)
                    .map(|p| JsonPinInfo {
                        designator: p.designator.clone(),
                        name: p.name.clone(),
                        electrical: electrical_type_name(p.electrical).to_string(),
                        hidden: p.is_hidden,
                    })
                    .collect();

                JsonComponentInfo {
                    designator,
                    lib_reference: c.lib_reference.clone(),
                    description: c.component_description.clone(),
                    location: format_coord(c.location_x, c.location_y),
                    pins: comp_pins,
                    parameters: Vec::new(), // Parameters are separate records
                }
            })
            .collect();

        let net_info: Vec<JsonNetInfo> = net_labels
            .iter()
            .map(|n| JsonNetInfo {
                name: n.text.clone(),
                location: format_coord(n.location_x, n.location_y),
            })
            .collect();

        let port_info: Vec<JsonPortInfo> = ports
            .iter()
            .map(|p| JsonPortInfo {
                name: p.name.clone(),
                io_type: port_io_name(p.io_type).to_string(),
                location: format_coord(p.location_x, p.location_y),
            })
            .collect();

        let power_info: Vec<JsonPowerInfo> = power_objects
            .iter()
            .map(|p| JsonPowerInfo {
                net: p.text.clone(),
                style: power_style_name(p.style).to_string(),
                location: format_coord(p.location_x, p.location_y),
            })
            .collect();

        Ok(serde_json::json!({
            "file": path.display().to_string(),
            "sheet": sheet,
            "summary": summary,
            "components": component_info,
            "nets": net_info,
            "ports": port_info,
            "power": power_info,
        }))
    } else {
        // Compact format
        Ok(serde_json::json!({
            "file": path.display().to_string(),
            "sheet": sheet,
            "summary": summary,
        }))
    }
}

// ===========================================================================
// TESTS
// ===========================================================================

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
    fn test_alphanumeric_sort_mixed() {
        let mut items = vec!["U10", "U2", "U1", "U20", "R1", "C1"];
        items.sort_by(|a, b| alphanumeric_sort(a, b));
        assert_eq!(items, vec!["C1", "R1", "U1", "U2", "U10", "U20"]);
    }

    #[test]
    fn test_is_power_net() {
        assert!(is_power_net("VCC"));
        assert!(is_power_net("VDD"));
        assert!(is_power_net("+3V3"));
        assert!(is_power_net("+5V"));
        assert!(is_power_net("3.3V"));
        assert!(is_power_net("VBAT"));
        assert!(!is_power_net("DATA0"));
        assert!(!is_power_net("CLK"));
    }

    #[test]
    fn test_is_ground_net() {
        assert!(is_ground_net("GND"));
        assert!(is_ground_net("AGND"));
        assert!(is_ground_net("DGND"));
        assert!(is_ground_net("VSS"));
        assert!(is_ground_net("PGND"));
        assert!(!is_ground_net("VCC"));
        assert!(!is_ground_net("DATA"));
    }

    #[test]
    fn test_coord_to_mils() {
        assert!((coord_to_mils(100000) - 1.0).abs() < 0.001);
        assert!((coord_to_mils(1000000) - 10.0).abs() < 0.001);
        assert!((coord_to_mils(-500000) - (-5.0)).abs() < 0.001);
    }

    #[test]
    fn test_format_coord() {
        let result = format_coord(100000, 200000);
        assert!(result.contains("1.0"));
        assert!(result.contains("2.0"));
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
    fn test_port_io_name() {
        assert_eq!(port_io_name(PortIO::Input), "Input");
        assert_eq!(port_io_name(PortIO::Output), "Output");
        assert_eq!(port_io_name(PortIO::Bidirectional), "Bidirectional");
        assert_eq!(port_io_name(PortIO::Unspecified), "Unspecified");
    }

    #[test]
    fn test_power_style_name() {
        assert_eq!(power_style_name(PowerObjectStyle::Circle), "Circle");
        assert_eq!(power_style_name(PowerObjectStyle::Arrow), "Arrow");
        assert_eq!(power_style_name(PowerObjectStyle::GndPower), "GND (Power)");
        assert_eq!(power_style_name(PowerObjectStyle::GndSignal), "GND (Signal)");
    }

    #[test]
    fn test_sheet_size_name() {
        assert_eq!(sheet_size_name(0), "A4");
        assert_eq!(sheet_size_name(1), "A3");
        assert_eq!(sheet_size_name(5), "A (ANSI)");
        assert_eq!(sheet_size_name(10), "Letter");
        assert_eq!(sheet_size_name(255), "Custom");
    }
}
