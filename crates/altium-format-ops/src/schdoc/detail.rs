// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! SchDoc detail commands: netlist, wires, ports, power_map.

use std::collections::HashMap;
use std::path::Path;

use crate::helpers::*;
use crate::output::*;

use altium_format::records::{SchNetLabelRecord, SchPortRecord, SchPowerRecord};

use super::{format_location, open_schdoc, port_io_type_name, power_style_name};

/// Extracts net label information, optionally filtered by name pattern.
/// Labels are grouped by net name to show connectivity.
pub fn cmd_netlist(
    path: &Path,
    filter: Option<String>,
) -> Result<SchDocNetLabelList, Box<dyn std::error::Error>> {
    let doc = open_schdoc(path)?;

    let filter_lower = filter.as_ref().map(|f| f.to_lowercase());

    let mut net_labels: Vec<NetLabelInfo> = Vec::new();
    let mut net_counts: HashMap<String, usize> = HashMap::new();

    doc.for_each_record_of_type(25, |node| {
        let rec = SchNetLabelRecord::from_origin(node.origin.clone());
        if let Ok(text) = rec.text() {
            if text.is_empty() {
                return;
            }

            if let Some(ref pattern) = filter_lower {
                if !text.to_lowercase().contains(pattern) {
                    return;
                }
            }

            *net_counts.entry(text.clone()).or_insert(0) += 1;
            if let (Ok(x), Ok(y)) = (rec.location_x(), rec.location_y()) {
                net_labels.push(NetLabelInfo {
                    net_name: text,
                    location: format_location(x, y),
                });
            }
        }
    });

    net_labels.sort_by(|a, b| {
        alphanumeric_sort(&a.net_name, &b.net_name).then_with(|| a.location.cmp(&b.location))
    });

    let total = net_labels.len();

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
///
/// **Status:** Wire coordinate extraction is not yet available in the typed API.
/// The `SchWireRecord` vertex coordinates (`LOCATION.X/Y`, `CORNER.X/Y`) are
/// `#[altium(skip)]` and not covered by the typed record accessors.
pub fn cmd_wires(path: &Path) -> Result<SchDocWireList, Box<dyn std::error::Error>> {
    let _ = path;
    Err("Wire coordinate extraction is not yet available in the typed API".into())
}

/// Lists all port definitions (RECORD=18) for hierarchical design analysis.
pub fn cmd_ports(path: &Path) -> Result<SchDocPortList, Box<dyn std::error::Error>> {
    let doc = open_schdoc(path)?;

    let mut ports: Vec<PortInfo> = Vec::new();

    doc.for_each_record_of_type(18, |node| {
        let rec = SchPortRecord::from_origin(node.origin.clone());
        if let (Ok(name), Ok(io_type), Ok(x), Ok(y)) =
            (rec.name(), rec.io_type(), rec.location_x(), rec.location_y())
        {
            ports.push(PortInfo {
                name,
                io_type: port_io_type_name(io_type).to_string(),
                location: format_location(x, y),
            });
        }
    });

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

    let mut power_objects: Vec<PowerObjectInfo> = Vec::new();
    let mut net_counts: HashMap<String, usize> = HashMap::new();

    doc.for_each_record_of_type(17, |node| {
        let rec = SchPowerRecord::from_origin(node.origin.clone());
        if let (Ok(text), Ok(style), Ok(x), Ok(y)) =
            (rec.text(), rec.style(), rec.location_x(), rec.location_y())
        {
            if !text.is_empty() {
                *net_counts.entry(text.clone()).or_insert(0) += 1;
            }

            power_objects.push(PowerObjectInfo {
                net: text,
                style: power_style_name(style).to_string(),
                location: format_location(x, y),
            });
        }
    });

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
