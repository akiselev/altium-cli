// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Schematic document operations.
//!
//! High-level operations for exploring and editing Altium schematic documents (.SchDoc files).

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json;

use crate::ops::categorization::categorize_component;
use crate::ops::output::*;
use crate::ops::queries::power::{power_map, separate_power_and_ground};
use crate::ops::util::{
    alphanumeric_sort, count_record_types, get_component_designator, get_component_pins,
    record_type_name, sheet_size_name,
};

use crate::dump::{fmt_coord, fmt_point};
use crate::io::SchDoc;
use crate::records::sch::{
    PinElectricalType, PortIoType, PowerObjectStyle, SchComponent, SchNetLabel, SchPort,
    SchPowerObject, SchRecord, SchWire,
};
use crate::tree::{RecordId, RecordTree};

/// Open schematic document with String error type (for old-style functions).
fn open_schdoc(path: &Path) -> Result<SchDoc, String> {
    let file = File::open(path).map_err(|e| format!("Error opening file: {}", e))?;
    SchDoc::open(BufReader::new(file)).map_err(|e| format!("Error parsing SchDoc: {:?}", e))
}

/// Open schematic document with Box<dyn Error> error type (for refactored functions).
fn open_schdoc_boxed(path: &Path) -> Result<SchDoc, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    Ok(SchDoc::open(BufReader::new(file))?)
}

// ═══════════════════════════════════════════════════════════════════════════
// HIGH-LEVEL COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// Complete design overview.
pub fn cmd_overview(path: &Path) -> Result<SchDocOverview, Box<dyn std::error::Error>> {
    let doc = open_schdoc(path)?;
    let tree = RecordTree::from_records(doc.primitives.clone());
    let counts = count_record_types(&doc);

    let sheet_size = doc
        .sheet_header()
        .map(|h| sheet_size_name(h.sheet_size).to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    // Collect components by category
    let mut categories: HashMap<&'static str, Vec<(String, String, String)>> = HashMap::new();
    for (id, record) in tree.iter() {
        if let SchRecord::Component(c) = record {
            let des = get_component_designator(&tree, id).unwrap_or_else(|| "<none>".to_string());
            let category = categorize_component(&c.lib_reference, &c.component_description);
            categories.entry(category).or_default().push((
                des,
                c.lib_reference.clone(),
                c.component_description.clone(),
            ));
        }
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

    // Collect power nets using extracted query functions
    let power_nets = power_map(&doc);
    let (rails, grounds) = separate_power_and_ground(power_nets);

    let power_architecture = PowerArchitecture {
        power_rails: rails,
        ground_nets: grounds,
    };

    // Collect interface ports
    let ports: Vec<_> = doc
        .primitives
        .iter()
        .filter_map(|r| {
            if let SchRecord::Port(p) = r {
                Some(p)
            } else {
                None
            }
        })
        .collect();

    let interfaces = if !ports.is_empty() {
        let inputs: Vec<String> = ports
            .iter()
            .filter(|p| matches!(p.io_type, PortIoType::Input))
            .map(|p| p.name.clone())
            .collect();
        let outputs: Vec<String> = ports
            .iter()
            .filter(|p| matches!(p.io_type, PortIoType::Output))
            .map(|p| p.name.clone())
            .collect();
        let bidirectional: Vec<String> = ports
            .iter()
            .filter(|p| matches!(p.io_type, PortIoType::Bidirectional))
            .map(|p| p.name.clone())
            .collect();
        let unspecified: Vec<String> = ports
            .iter()
            .filter(|p| matches!(p.io_type, PortIoType::Unspecified))
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
    for record in &doc.primitives {
        if let SchRecord::NetLabel(nl) = record {
            *net_labels.entry(nl.label.text.clone()).or_insert(0) += 1;
        }
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

/// Bill of materials.
pub fn cmd_bom(path: &Path) -> Result<SchDocBom, Box<dyn std::error::Error>> {
    let doc = open_schdoc(path)?;
    let tree = RecordTree::from_records(doc.primitives.clone());

    // Group by library reference
    let mut bom: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for (id, record) in tree.iter() {
        if let SchRecord::Component(c) = record {
            let des = get_component_designator(&tree, id).unwrap_or_else(|| "<none>".to_string());
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
#[allow(clippy::type_complexity)]
pub fn cmd_netlist(
    path: &Path,
    net_filter: Option<String>,
    min_connections: usize,
) -> Result<SchDocNetlist, Box<dyn std::error::Error>> {
    let doc = open_schdoc(path)?;
    let tree = RecordTree::from_records(doc.primitives.clone());

    // Build component designator lookup and pin locations
    let mut pin_locations: HashMap<(i32, i32), Vec<(String, String, String)>> = HashMap::new();
    for (id, record) in tree.iter() {
        if let SchRecord::Component(_) = record {
            let des =
                get_component_designator(&tree, id).unwrap_or_else(|| format!("?{}", id.index()));
            for (pin_des, pin_name, corner_x, corner_y) in get_component_pins(&tree, id) {
                pin_locations
                    .entry((corner_x, corner_y))
                    .or_default()
                    .push((des.clone(), pin_des, pin_name));
            }
        }
    }

    // Build net name to location mapping
    let mut net_at_location: HashMap<(i32, i32), String> = HashMap::new();
    for record in &doc.primitives {
        match record {
            SchRecord::NetLabel(nl) => {
                net_at_location.insert(
                    (nl.label.graphical.location_x, nl.label.graphical.location_y),
                    nl.label.text.clone(),
                );
            }
            SchRecord::PowerObject(p) => {
                net_at_location.insert(
                    (p.graphical.location_x, p.graphical.location_y),
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
    let tree = RecordTree::from_records(doc.primitives.clone());

    // Build component info
    let mut comp_info: HashMap<RecordId, (String, String)> = HashMap::new();
    let mut power_pins: HashMap<RecordId, Vec<(String, String)>> = HashMap::new(); // comp_id -> [(pin_des, pin_name)]

    for (id, record) in tree.iter() {
        if let SchRecord::Component(c) = record {
            let des =
                get_component_designator(&tree, id).unwrap_or_else(|| format!("?{}", id.index()));
            comp_info.insert(id, (des.clone(), c.lib_reference.clone()));

            // Find power pins
            for (_, child) in tree.children(id) {
                if let SchRecord::Pin(p) = child {
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
                        || format!("{:?}", p.electrical).contains("Power")
                    {
                        power_pins
                            .entry(id)
                            .or_default()
                            .push((p.designator.clone(), p.name.clone()));
                    }
                }
            }
        }
    }

    // Get power symbols and their nets
    let mut power_nets: HashMap<String, Vec<(i32, i32)>> = HashMap::new();
    for record in &doc.primitives {
        if let SchRecord::PowerObject(p) = record {
            power_nets
                .entry(p.text.clone())
                .or_default()
                .push((p.graphical.location_x, p.graphical.location_y));
        }
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
            for (comp_id, pins) in &power_pins {
                if let Some((des, _lib_ref)) = comp_info.get(comp_id) {
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
        .filter_map(|(id, pins)| {
            comp_info.get(id).map(|(des, lib_ref)| PoweredComponent {
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
    let tree = RecordTree::from_records(doc.primitives.clone());

    let mut blocks: Vec<BlockInfo> = Vec::new();

    for (id, record) in tree.iter() {
        if let SchRecord::Component(c) = record {
            let des = get_component_designator(&tree, id).unwrap_or_else(|| "<none>".to_string());
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

            // Categorize pins
            for (_, child) in tree.children(id) {
                if let SchRecord::Pin(p) = child {
                    if p.is_hidden() {
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
                        PinElectricalType::Power => power_pins.push(pin_info),
                        PinElectricalType::Input => input_pins.push(pin_info),
                        PinElectricalType::Output => output_pins.push(pin_info),
                        PinElectricalType::InputOutput => bidir_pins.push(pin_info),
                        _ => bidir_pins.push(pin_info), // Passive, etc.
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

        let component_count = doc
            .primitives
            .iter()
            .filter(|r| matches!(r, SchRecord::Component(_)))
            .count();

        let mut ports: Vec<(String, String)> = Vec::new();
        for record in &doc.primitives {
            if let SchRecord::Port(p) = record {
                let io = match p.io_type {
                    PortIoType::Input => "IN",
                    PortIoType::Output => "OUT",
                    PortIoType::Bidirectional => "BIDIR",
                    PortIoType::Unspecified => "BUS",
                };
                ports.push((p.name.clone(), io.to_string()));
                all_ports
                    .entry(p.name.clone())
                    .or_default()
                    .push((sheet_name.clone(), io.to_string()));
            }
        }

        let mut power_nets: Vec<String> = doc
            .primitives
            .iter()
            .filter_map(|r| {
                if let SchRecord::PowerObject(p) = r {
                    Some(p.text.clone())
                } else {
                    None
                }
            })
            .collect();
        power_nets.sort();
        power_nets.dedup();

        let mut unique_nets: Vec<String> = doc
            .primitives
            .iter()
            .filter_map(|r| {
                if let SchRecord::NetLabel(nl) = r {
                    Some(nl.label.text.clone())
                } else {
                    None
                }
            })
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
    let tree = RecordTree::from_records(doc.primitives.clone());

    // Find all net labels matching the signal
    let matching_nets: Vec<_> = doc
        .primitives
        .iter()
        .filter_map(|r| {
            if let SchRecord::NetLabel(nl) = r {
                if nl.label.text.eq_ignore_ascii_case(signal)
                    || nl
                        .label
                        .text
                        .to_uppercase()
                        .contains(&signal.to_uppercase())
                {
                    Some((
                        nl.label.text.clone(),
                        nl.label.graphical.location_x,
                        nl.label.graphical.location_y,
                    ))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    // Also check power objects
    let matching_power: Vec<_> = doc
        .primitives
        .iter()
        .filter_map(|r| {
            if let SchRecord::PowerObject(p) = r {
                if p.text.eq_ignore_ascii_case(signal)
                    || p.text.to_uppercase().contains(&signal.to_uppercase())
                {
                    Some((
                        p.text.clone(),
                        p.graphical.location_x,
                        p.graphical.location_y,
                    ))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    // Also check ports
    let matching_ports: Vec<_> = doc
        .primitives
        .iter()
        .filter_map(|r| {
            if let SchRecord::Port(p) = r {
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

    for (id, record) in tree.iter() {
        if let SchRecord::Component(_c) = record {
            let des = get_component_designator(&tree, id).unwrap_or_default();

            for (_, child) in tree.children(id) {
                if let SchRecord::Pin(p) = child {
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
    let doc = open_schdoc(path)?;
    let counts = count_record_types(&doc);

    // Sheet info
    let sheet_info = doc.sheet_header().map(|header| {
        let custom_dimensions = if header.custom_x > 0 || header.custom_y > 0 {
            Some((
                fmt_coord(header.custom_x * 10000),
                fmt_coord(header.custom_y * 10000),
            ))
        } else {
            None
        };
        SheetInfoDetails {
            size: sheet_size_name(header.sheet_size).to_string(),
            size_style: header.sheet_size,
            custom_dimensions,
            fonts_defined: header.font_id_count,
        }
    });

    // Primitive summary
    let primitive_summary = PrimitiveSummary {
        total_primitives: doc.primitives.len(),
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
        .primitives
        .iter()
        .filter_map(|r| {
            if let SchRecord::NetLabel(nl) = r {
                Some(nl.label.text.clone())
            } else {
                None
            }
        })
        .collect();
    net_names.sort();
    net_names.dedup();

    // Collect unique power nets
    let mut power_nets: Vec<String> = doc
        .primitives
        .iter()
        .filter_map(|r| {
            if let SchRecord::PowerObject(p) = r {
                Some(p.text.clone())
            } else {
                None
            }
        })
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
    let doc = open_schdoc(path)?;
    let counts = count_record_types(&doc);

    let mut record_types: Vec<(String, usize)> = counts
        .into_iter()
        .map(|(name, count)| (name.to_string(), count))
        .collect();
    record_types.sort_by(|a, b| b.1.cmp(&a.1));

    Ok(SchDocStats {
        path: path.display().to_string(),
        total_primitives: doc.primitives.len(),
        record_types,
    })
}

/// List all components.
pub fn cmd_components(
    path: &Path,
    verbose: bool,
) -> Result<SchDocComponentList, Box<dyn std::error::Error>> {
    let doc = open_schdoc(path)?;
    let tree = RecordTree::from_records(doc.primitives.clone());

    let mut component_data: Vec<(RecordId, &SchComponent, Option<String>)> = Vec::new();
    for (id, record) in tree.iter() {
        if let SchRecord::Component(c) = record {
            let designator = get_component_designator(&tree, id);
            component_data.push((id, c, designator));
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
        .map(|(id, comp, designator)| {
            let child_count = if verbose {
                Some(tree.child_count(*id))
            } else {
                None
            };
            SchDocComponentInfo {
                designator: designator.clone().unwrap_or_else(|| "<none>".to_string()),
                lib_reference: comp.lib_reference.clone(),
                description: comp.component_description.clone(),
                location: fmt_point(comp.graphical.location_x, comp.graphical.location_y),
                parts: comp.part_count,
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
    let doc = open_schdoc(path)?;
    let tree = RecordTree::from_records(doc.primitives.clone());

    // Find component by designator or index
    let component_id = if let Ok(index) = designator.parse::<usize>() {
        // Numeric index
        let mut comp_idx = 0;
        let mut found_id = None;
        for (id, record) in tree.iter() {
            if matches!(record, SchRecord::Component(_)) {
                if comp_idx == index {
                    found_id = Some(id);
                    break;
                }
                comp_idx += 1;
            }
        }
        found_id.ok_or_else(|| format!("Component index {} not found", index))?
    } else {
        // Find by designator
        let mut found_id = None;
        for (id, record) in tree.iter() {
            if matches!(record, SchRecord::Component(_)) {
                if let Some(des) = get_component_designator(&tree, id) {
                    if des.eq_ignore_ascii_case(designator) {
                        found_id = Some(id);
                        break;
                    }
                }
            }
        }
        found_id.ok_or_else(|| format!("Component '{}' not found", designator))?
    };

    let comp = match tree.get(component_id) {
        Some(SchRecord::Component(c)) => c,
        _ => return Err("Invalid component".into()),
    };

    let actual_designator = get_component_designator(&tree, component_id);

    // Count and categorize children
    let children: Vec<_> = tree.children(component_id).collect();
    let mut pin_infos = Vec::new();
    let mut param_infos = Vec::new();
    let mut designator_infos = Vec::new();
    let mut graphics_count = 0;

    for (_id, child) in &children {
        match child {
            SchRecord::Pin(p) => {
                pin_infos.push(SchDocPinInfo {
                    designator: p.designator.clone(),
                    name: p.name.clone(),
                    electrical_type: format!("{:?}", p.electrical),
                    hidden: p.is_hidden(),
                });
            }
            SchRecord::Parameter(p) => {
                param_infos.push(SchDocParameter {
                    name: p.name.clone(),
                    value: p.label.text.clone(),
                });
            }
            SchRecord::Designator(d) => {
                designator_infos.push(SchDocDesignator {
                    name: d.param.name.clone(),
                    value: d.param.label.text.clone(),
                });
            }
            _ => graphics_count += 1,
        }
    }

    Ok(SchDocComponentDetail {
        designator: actual_designator.unwrap_or_else(|| "<none>".to_string()),
        lib_reference: comp.lib_reference.clone(),
        description: comp.component_description.clone(),
        location: fmt_point(comp.graphical.location_x, comp.graphical.location_y),
        parts: comp.part_count,
        display_modes: comp.display_mode_count,
        current_part: comp.current_part_id,
        unique_id: comp.unique_id.clone(),
        child_primitive_count: children.len(),
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
    let doc = open_schdoc(path)?;

    let wires: Vec<&SchWire> = doc
        .primitives
        .iter()
        .filter_map(|r| {
            if let SchRecord::Wire(w) = r {
                Some(w)
            } else {
                None
            }
        })
        .collect();

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
    let doc = open_schdoc(path)?;

    let net_labels: Vec<&SchNetLabel> = doc
        .primitives
        .iter()
        .filter_map(|r| {
            if let SchRecord::NetLabel(nl) = r {
                Some(nl)
            } else {
                None
            }
        })
        .collect();

    let (grouped, individual) = if group {
        let mut grouped_map: HashMap<&str, Vec<&SchNetLabel>> = HashMap::new();
        for nl in &net_labels {
            grouped_map.entry(&nl.label.text).or_default().push(nl);
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
                net_name: nl.label.text.clone(),
                location: fmt_point(nl.label.graphical.location_x, nl.label.graphical.location_y),
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
    let doc = open_schdoc(path)?;

    let ports: Vec<&SchPort> = doc
        .primitives
        .iter()
        .filter_map(|r| {
            if let SchRecord::Port(p) = r {
                Some(p)
            } else {
                None
            }
        })
        .collect();

    let port_infos: Vec<PortInfo> = ports
        .iter()
        .map(|port| {
            let io_type = match port.io_type {
                PortIoType::Unspecified => "Unspec",
                PortIoType::Output => "Output",
                PortIoType::Input => "Input",
                PortIoType::Bidirectional => "Bidir",
            };
            PortInfo {
                name: port.name.clone(),
                io_type: io_type.to_string(),
                location: fmt_point(port.graphical.location_x, port.graphical.location_y),
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
    let doc = open_schdoc(path)?;

    let power_objects: Vec<&SchPowerObject> = doc
        .primitives
        .iter()
        .filter_map(|r| {
            if let SchRecord::PowerObject(p) = r {
                Some(p)
            } else {
                None
            }
        })
        .collect();

    let (grouped, individual) = if group {
        let mut grouped_map: HashMap<&str, Vec<&SchPowerObject>> = HashMap::new();
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
                    PowerObjectStyle::Arrow => "Arrow",
                    PowerObjectStyle::Bar => "Bar",
                    PowerObjectStyle::Wave => "Wave",
                    PowerObjectStyle::Ground => "Ground",
                    PowerObjectStyle::PowerGround => "PowerGnd",
                    PowerObjectStyle::SignalGround => "SignalGnd",
                    PowerObjectStyle::EarthGround => "EarthGnd",
                    PowerObjectStyle::Circle => "Circle",
                };
                PowerObjectInfo {
                    net: p.text.clone(),
                    style: style.to_string(),
                    location: fmt_point(p.graphical.location_x, p.graphical.location_y),
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
    let doc = open_schdoc(path)?;
    let tree = RecordTree::from_records(doc.primitives.clone());

    let mut pin_details = Vec::new();
    let total_pins: usize;

    // If filtering by component, find the component first
    if let Some(ref comp_des) = component_filter {
        // Find by iterating
        let mut found_component = None;
        for (id, record) in tree.iter() {
            if matches!(record, SchRecord::Component(_)) {
                if let Some(des) = get_component_designator(&tree, id) {
                    if des.eq_ignore_ascii_case(comp_des) {
                        found_component = Some(id);
                        break;
                    }
                }
            }
        }

        if let Some(comp_id) = found_component {
            let des = get_component_designator(&tree, comp_id).unwrap_or_default();

            let pins: Vec<_> = tree
                .children(comp_id)
                .filter_map(|(_, r)| {
                    if let SchRecord::Pin(p) = r {
                        Some(p)
                    } else {
                        None
                    }
                })
                .collect();

            total_pins = pins.len();

            for pin in pins {
                pin_details.push(SchDocPinDetail {
                    component: des.clone(),
                    designator: pin.designator.clone(),
                    name: pin.name.clone(),
                    electrical_type: format!("{:?}", pin.electrical),
                    location: fmt_point(0, 0), // Pins don't have absolute location
                });
            }
        } else {
            return Err(format!("Component '{}' not found", comp_des).into());
        }
    } else {
        // All pins - need to build component map
        let mut comp_map: HashMap<RecordId, String> = HashMap::new();
        for (id, record) in tree.iter() {
            if let SchRecord::Component(_) = record {
                if let Some(des) = get_component_designator(&tree, id) {
                    comp_map.insert(id, des);
                }
            }
        }

        // Collect all pins with their parent component
        for (id, record) in tree.iter() {
            if let SchRecord::Pin(p) = record {
                // Find parent component
                if let Some(parent_id) = tree.parent_id(id) {
                    let comp_des = comp_map.get(&parent_id).cloned().unwrap_or_default();
                    pin_details.push(SchDocPinDetail {
                        component: comp_des,
                        designator: p.designator.clone(),
                        name: p.name.clone(),
                        electrical_type: format!("{:?}", p.electrical),
                        location: fmt_point(0, 0),
                    });
                }
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
    let doc = open_schdoc(path)?;
    let tree = RecordTree::from_records(doc.primitives.clone());

    let start_ids: Vec<RecordId> = if let Some(ref des) = from_designator {
        // Start from specific component
        let mut found = Vec::new();
        for (id, record) in tree.iter() {
            if matches!(record, SchRecord::Component(_)) {
                if let Some(comp_des) = get_component_designator(&tree, id) {
                    if comp_des.eq_ignore_ascii_case(des) {
                        found.push(id);
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
        // Start from roots
        tree.roots().map(|(id, _)| id).collect()
    };

    let max_d = max_depth.unwrap_or(10);
    let hierarchy_nodes: Vec<HierarchyNode> = start_ids
        .into_iter()
        .map(|id| build_hierarchy_node(&tree, id, 0, max_d))
        .collect();

    Ok(SchDocHierarchy {
        path: path.display().to_string(),
        hierarchy: hierarchy_nodes,
    })
}

/// Helper function to build hierarchy node recursively.
fn build_hierarchy_node(
    tree: &RecordTree<SchRecord>,
    id: RecordId,
    depth: usize,
    max_depth: usize,
) -> HierarchyNode {
    if depth <= max_depth {
        let record = match tree.get(id) {
            Some(r) => r,
            None => {
                return HierarchyNode {
                    node_type: "error".to_string(),
                    unique_id: "Invalid ID".to_string(),
                    description: String::new(),
                    children: Vec::new(),
                };
            }
        };

        // Format the node
        let (node_type, identifier, description) = match record {
            SchRecord::Component(c) => {
                let des = get_component_designator(tree, id).unwrap_or_default();
                ("component".to_string(), des, c.lib_reference.clone())
            }
            SchRecord::Pin(p) => ("pin".to_string(), p.designator.clone(), p.name.clone()),
            SchRecord::Parameter(p) => (
                "parameter".to_string(),
                p.name.clone(),
                p.label.text.clone(),
            ),
            SchRecord::Designator(d) => (
                "designator".to_string(),
                d.param.name.clone(),
                d.param.label.text.clone(),
            ),
            SchRecord::NetLabel(nl) => {
                ("netlabel".to_string(), nl.label.text.clone(), String::new())
            }
            SchRecord::Port(p) => (
                "port".to_string(),
                p.name.clone(),
                format!("{:?}", p.io_type),
            ),
            SchRecord::PowerObject(p) => (
                "power".to_string(),
                p.text.clone(),
                format!("{:?}", p.style),
            ),
            _ => (
                record_type_name(record).to_string(),
                format!("[{}]", id.index()),
                String::new(),
            ),
        };

        // Build child hierarchy recursively
        let children: Vec<_> = tree
            .children(id)
            .map(|(child_id, _)| build_hierarchy_node(tree, child_id, depth + 1, max_depth))
            .collect();

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
    let tree = RecordTree::from_records(doc.primitives.clone());

    let query_lower = query.to_lowercase();
    let max_results = limit.unwrap_or(50);

    println!("Search Results: {}", path.display());
    println!("Query: \"{}\"", query);
    println!("═══════════════════════════════════════════════════════════════");

    let mut results = Vec::new();

    for (id, record) in tree.iter() {
        let matches = match record {
            SchRecord::Component(c) => {
                c.lib_reference.to_lowercase().contains(&query_lower)
                    || c.component_description
                        .to_lowercase()
                        .contains(&query_lower)
            }
            SchRecord::Pin(p) => {
                p.name.to_lowercase().contains(&query_lower)
                    || p.designator.to_lowercase().contains(&query_lower)
            }
            SchRecord::NetLabel(nl) => nl.label.text.to_lowercase().contains(&query_lower),
            SchRecord::Port(p) => p.name.to_lowercase().contains(&query_lower),
            SchRecord::PowerObject(p) => p.text.to_lowercase().contains(&query_lower),
            SchRecord::Label(l) => l.text.to_lowercase().contains(&query_lower),
            SchRecord::TextFrame(tf) => tf.text.to_lowercase().contains(&query_lower),
            SchRecord::Parameter(p) => {
                p.name.to_lowercase().contains(&query_lower)
                    || p.label.text.to_lowercase().contains(&query_lower)
            }
            SchRecord::Designator(d) => {
                d.param.name.to_lowercase().contains(&query_lower)
                    || d.param.label.text.to_lowercase().contains(&query_lower)
            }
            _ => false,
        };

        if matches {
            results.push((id, record));
            if results.len() >= max_results {
                break;
            }
        }
    }

    println!("\nFound {} results:\n", results.len());

    for (id, record) in &results {
        let desc = match record {
            SchRecord::Component(c) => {
                let des = get_component_designator(&tree, *id).unwrap_or_default();
                format!("Component {} - {}", des, c.lib_reference)
            }
            SchRecord::Pin(p) => format!("Pin {} - {}", p.designator, p.name),
            SchRecord::NetLabel(nl) => format!("NetLabel: {}", nl.label.text),
            SchRecord::Port(p) => format!("Port: {}", p.name),
            SchRecord::PowerObject(p) => format!("Power: {}", p.text),
            SchRecord::Label(l) => format!("Label: {}", l.text),
            SchRecord::TextFrame(tf) => {
                let text = if tf.text.len() > 40 {
                    format!("{}...", &tf.text[..40])
                } else {
                    tf.text.clone()
                };
                format!("TextFrame: {}", text)
            }
            SchRecord::Parameter(p) => format!("Parameter: {} = {}", p.name, p.label.text),
            SchRecord::Designator(d) => {
                format!("Designator: {} = {}", d.param.name, d.param.label.text)
            }
            _ => record_type_name(record).to_string(),
        };
        println!("  [{}] {}", id.index(), desc);
    }

    if results.len() >= max_results {
        println!("\n(results limited to {})", max_results);
    }

    Ok(())
}

/// List junctions.
pub fn cmd_junctions(path: &Path) -> Result<SchDocJunctionList, Box<dyn std::error::Error>> {
    let doc = open_schdoc(path)?;

    let junctions: Vec<JunctionInfo> = doc
        .primitives
        .iter()
        .filter_map(|r| {
            if let SchRecord::Junction(j) = r {
                Some(JunctionInfo {
                    location: fmt_point(j.graphical.location_x, j.graphical.location_y),
                })
            } else {
                None
            }
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
    let tree = RecordTree::from_records(doc.primitives.clone());
    let counts = count_record_types(&doc);

    // Build sheet info
    let sheet = doc.sheet_header().map(|h| JsonSheet {
        size: sheet_size_name(h.sheet_size).to_string(),
        fonts: h.font_id_count,
    });

    // Build summary
    let summary = JsonSummary {
        total_primitives: doc.primitives.len(),
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
        for (id, record) in tree.iter() {
            if let SchRecord::Component(c) = record {
                let des = get_component_designator(&tree, id).unwrap_or_default();

                let mut pins = Vec::new();
                let mut params = Vec::new();

                for (_, child) in tree.children(id) {
                    match child {
                        SchRecord::Pin(p) => {
                            pins.push(JsonPin {
                                designator: p.designator.clone(),
                                name: p.name.clone(),
                                electrical: format!("{:?}", p.electrical),
                                hidden: p.is_hidden(),
                            });
                        }
                        SchRecord::Parameter(p) => {
                            params.push(JsonParameter {
                                name: p.name.clone(),
                                value: p.label.text.clone(),
                            });
                        }
                        _ => {}
                    }
                }

                components.push(JsonComponent {
                    designator: des,
                    lib_reference: c.lib_reference.clone(),
                    description: c.component_description.clone(),
                    location: fmt_point(c.graphical.location_x, c.graphical.location_y),
                    pins,
                    parameters: params,
                });
            }
        }

        // Net labels
        let nets: Vec<JsonNet> = doc
            .primitives
            .iter()
            .filter_map(|r| {
                if let SchRecord::NetLabel(nl) = r {
                    Some(JsonNet {
                        name: nl.label.text.clone(),
                        location: fmt_point(
                            nl.label.graphical.location_x,
                            nl.label.graphical.location_y,
                        ),
                    })
                } else {
                    None
                }
            })
            .collect();

        // Ports
        let ports: Vec<JsonPort> = doc
            .primitives
            .iter()
            .filter_map(|r| {
                if let SchRecord::Port(p) = r {
                    Some(JsonPort {
                        name: p.name.clone(),
                        io_type: format!("{:?}", p.io_type),
                        location: fmt_point(p.graphical.location_x, p.graphical.location_y),
                    })
                } else {
                    None
                }
            })
            .collect();

        // Power objects
        let power: Vec<JsonPower> = doc
            .primitives
            .iter()
            .filter_map(|r| {
                if let SchRecord::PowerObject(p) = r {
                    Some(JsonPower {
                        net: p.text.clone(),
                        style: format!("{:?}", p.style),
                        location: fmt_point(p.graphical.location_x, p.graphical.location_y),
                    })
                } else {
                    None
                }
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
