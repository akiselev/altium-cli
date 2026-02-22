// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! SchLib JSON export command.

use std::path::Path;

use altium_format::handles::{SchComponent, SchPin};

use super::open_schlib;

/// Serializes the library to JSON for LLM processing or external analysis.
pub fn cmd_json(path: &Path, full: bool) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let lib = open_schlib(path)?;

    if full {
        // Build comprehensive JSON manually since SchLib is no longer Serialize.
        let components_all = lib.query_all::<SchComponent>("#1")?;
        let mut components: Vec<serde_json::Value> = Vec::new();
        for comp in &components_all {
            let pin_handles = comp.children::<SchPin>()?;
            let mut pins: Vec<serde_json::Value> = Vec::new();
            for pin_handle in &pin_handles {
                let pin = pin_handle.read();
                let designator = pin.designator()?.to_string();
                let name = pin.name()?.to_string();
                let electrical = pin.electrical()? as i32;
                pins.push(serde_json::json!({
                    "designator": designator,
                    "name": name,
                    "electrical": electrical,
                }));
            }
            let primitive_count = comp.children_len();
            let rec = comp.read();
            let display_mode_count = rec.display_mode_count()?;
            components.push(serde_json::json!({
                "name": comp.lib_ref()?,
                "description": comp.description()?,
                "part_count": comp.part_count()?,
                "display_mode_count": display_mode_count,
                "pin_count": pin_handles.len(),
                "primitive_count": primitive_count,
                "pins": pins,
            }));
        }

        Ok(serde_json::json!({
            "file": path.display().to_string(),
            "unique_id": lib.header().unique_id(),
            "component_count": lib.component_count(),
            "components": components,
        }))
    } else {
        let mut components: Vec<serde_json::Value> = Vec::new();

        let results = lib.query_all::<SchComponent>("#1")?;
        for comp in &results {
            let pin_count = comp.child_count::<SchPin>();
            let primitive_count = comp.children_len();
            components.push(serde_json::json!({
                "name": comp.lib_ref()?,
                "description": comp.description()?,
                "pin_count": pin_count,
                "part_count": comp.part_count()?,
                "primitive_count": primitive_count,
            }));
        }

        Ok(serde_json::json!({
            "file": path.display().to_string(),
            "unique_id": lib.header().unique_id(),
            "component_count": lib.component_count(),
            "components": components,
        }))
    }
}
