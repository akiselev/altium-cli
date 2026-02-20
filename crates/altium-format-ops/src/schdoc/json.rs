// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! SchDoc JSON export command.

use std::path::Path;

use crate::helpers::*;

use altium_format::v2::handles::{SchComponent, SchPin};
use altium_format::v2::traits::DocumentQuery;

use super::{collect_net_names, collect_power_nets, get_sheet_size, open_schdoc};

/// Serializes the schematic document to JSON for LLM processing or external analysis.
pub fn cmd_json(path: &Path, full: bool) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let doc = open_schdoc(path)?;

    if full {
        let mut components_json = Vec::new();
        let comps = DocumentQuery::<SchComponent>::query_all(&doc, "#1")?;
        for comp in &comps {
            let rec = comp.read();
            let pins: Vec<serde_json::Value> = comp
                .children::<SchPin>()
                .iter()
                .map(|p| {
                    let pr = p.read();
                    serde_json::json!({
                        "designator": pr.designator().to_string(),
                        "name": pr.name().to_string(),
                    })
                })
                .collect();
            components_json.push(serde_json::json!({
                "designator": rec.designator().to_string(),
                "lib_reference": rec.lib_reference().to_string(),
                "description": rec.component_description(),
                "pin_count": pins.len(),
                "child_count": comp.children_len(),
                "pins": pins,
            }));
        }

        components_json.sort_by(|a, b| {
            let ad = a["designator"].as_str().unwrap_or("");
            let bd = b["designator"].as_str().unwrap_or("");
            alphanumeric_sort(ad, bd)
        });

        let net_names = collect_net_names(&doc);
        let power_nets = collect_power_nets(&doc);

        Ok(serde_json::json!({
            "file": path.display().to_string(),
            "sheet_size": get_sheet_size(&doc),
            "component_count": doc.component_count(),
            "wire_count": doc.count_record_type(27),
            "net_label_count": doc.count_record_type(25),
            "port_count": doc.count_record_type(18),
            "power_port_count": doc.count_record_type(17),
            "junction_count": doc.count_record_type(29),
            "components": components_json,
            "unique_nets": net_names,
            "power_nets": power_nets,
        }))
    } else {
        let mut components: Vec<serde_json::Value> = Vec::new();

        let comps = DocumentQuery::<SchComponent>::query_all(&doc, "#1")?;
        for comp in &comps {
            let rec = comp.read();
            let designator = rec.designator().to_string();
            let pin_count = comp.child_count::<SchPin>();
            let child_count = comp.children_len();

            components.push(serde_json::json!({
                "designator": designator,
                "lib_reference": rec.lib_reference().to_string(),
                "pin_count": pin_count,
                "child_count": child_count,
            }));
        }

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
            "component_count": doc.component_count(),
            "wire_count": doc.count_record_type(27),
            "net_label_count": doc.count_record_type(25),
            "port_count": doc.count_record_type(18),
            "power_port_count": doc.count_record_type(17),
            "junction_count": doc.count_record_type(29),
            "components": components,
            "unique_nets": net_names,
            "power_nets": power_nets,
        }))
    }
}
