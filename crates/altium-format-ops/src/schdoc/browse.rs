// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! SchDoc browse commands: overview, info, components.

use std::collections::HashMap;
use std::path::Path;

use crate::categorization::categorize_component;
use crate::helpers::*;
use crate::output::*;

use altium_format::coord::AltiumCoord;
use altium_format::handles::SchComponent;
use altium_format::records::{SchNetLabelRecord, SchPortRecord, SchPowerRecord};
use altium_format::traits::DocumentQuery;

use super::{
    collect_net_names, format_location, get_sheet_size, is_address_bus, is_control_signal,
    is_data_bus, is_ground_net, is_power_rail, open_schdoc, sheet_size_name,
};

/// Returns a schematic overview with component categories, power architecture,
/// interfaces, key signals, and quick statistics.
pub fn cmd_overview(path: &Path) -> Result<SchDocOverview, Box<dyn std::error::Error>> {
    let doc = open_schdoc(path)?;

    // 1. COMPONENTS BY CATEGORY
    let mut categories: HashMap<&'static str, Vec<SchDocComponentRef>> = HashMap::new();

    let components = DocumentQuery::<SchComponent>::query_all(&doc, "#1")?;
    for comp in &components {
        let rec = comp.read();
        let designator = rec.designator().to_string();
        let lib_reference = rec.lib_reference().to_string();
        let description = rec.component_description();
        let category = categorize_component(&lib_reference, &description);
        let comp_ref = SchDocComponentRef {
            designator,
            lib_reference,
            description,
        };
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

    // 2. POWER ARCHITECTURE
    let mut power_net_counts: HashMap<String, usize> = HashMap::new();
    let mut ground_net_counts: HashMap<String, usize> = HashMap::new();

    doc.for_each_record_of_type(17, |node| {
        let rec = SchPowerRecord::from_origin(node.origin.clone());
        let text = rec.text();
        if !text.is_empty() {
            if is_ground_net(&text) {
                *ground_net_counts.entry(text).or_insert(0) += 1;
            } else {
                *power_net_counts.entry(text).or_insert(0) += 1;
            }
        }
    });

    doc.for_each_record_of_type(25, |node| {
        let rec = SchNetLabelRecord::from_origin(node.origin.clone());
        let text = rec.text();
        if !text.is_empty() {
            if is_ground_net(&text) {
                *ground_net_counts.entry(text).or_insert(0) += 1;
            } else if is_power_rail(&text) {
                *power_net_counts.entry(text).or_insert(0) += 1;
            }
        }
    });

    let mut power_rails: Vec<(String, usize)> = power_net_counts.into_iter().collect();
    power_rails.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| alphanumeric_sort(&a.0, &b.0)));

    let mut ground_nets: Vec<(String, usize)> = ground_net_counts.into_iter().collect();
    ground_nets.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| alphanumeric_sort(&a.0, &b.0)));

    let power_architecture = PowerArchitecture {
        power_rails,
        ground_nets,
    };

    // 3. INTERFACES (from ports, RECORD=18)
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    let mut bidirectional = Vec::new();
    let mut unspecified = Vec::new();

    doc.for_each_record_of_type(18, |node| {
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
    });

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

    // 4. KEY SIGNALS
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

    // 5. QUICK STATS
    let quick_stats = SchDocQuickStats {
        components: doc.component_count(),
        wires: doc.count_record_type(27),
        junctions: doc.count_record_type(29),
        net_labels: doc.count_record_type(25),
        ports: doc.count_record_type(18),
        power_symbols: doc.count_record_type(17),
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

    // 1. SHEET INFO
    let sheet_info = if let Some(rec) = doc.sheet_record() {
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

    // 2. PRIMITIVE SUMMARY
    let wire_count = doc.count_record_type(27);
    let net_label_count = doc.count_record_type(25);
    let port_count = doc.count_record_type(18);
    let power_count = doc.count_record_type(17);
    let junction_count = doc.count_record_type(29);
    let pin_count = doc.count_record_type(2);

    let mut total_primitives = doc.component_count();
    let components = DocumentQuery::<SchComponent>::query_all(&doc, "#1")?;
    for comp in &components {
        total_primitives += comp.children_len();
    }
    total_primitives += doc.orphan_count();

    let primitive_summary = PrimitiveSummary {
        total_primitives,
        components: doc.component_count(),
        wires: wire_count,
        net_labels: net_label_count,
        ports: port_count,
        power_objects: power_count,
        junctions: junction_count,
        pins: pin_count,
    };

    // 3. NETS
    let unique_nets = super::collect_net_names(&doc);
    let power_nets = super::collect_power_nets(&doc);

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

    let mut components: Vec<SchDocComponentInfo> = Vec::new();

    let comps = DocumentQuery::<SchComponent>::query_all(&doc, "#1")?;
    for comp in &comps {
        let rec = comp.read();
        let designator = rec.designator().to_string();

        components.push(SchDocComponentInfo {
            designator,
            lib_reference: rec.lib_reference().to_string(),
            description: rec.component_description(),
            location: format_location(rec.location_x(), rec.location_y()),
            parts: rec.part_count() as i32,
            child_count: Some(comp.children_len()),
        });
    }

    components.sort_by(|a, b| alphanumeric_sort(&a.designator, &b.designator));

    Ok(SchDocComponentList {
        path: path.display().to_string(),
        total_components: components.len(),
        components,
    })
}
