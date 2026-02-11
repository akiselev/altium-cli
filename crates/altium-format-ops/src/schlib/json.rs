// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! SchLib JSON export command.

use std::path::Path;

use super::open_schlib;

/// Serializes the library to JSON for LLM processing or external analysis.
pub fn cmd_json(path: &Path, full: bool) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let lib = open_schlib(path)?;

    if full {
        Ok(serde_json::to_value(&lib)?)
    } else {
        let mut components: Vec<serde_json::Value> = Vec::new();

        lib.for_each_component_ref(|entry, view| {
            let pin_count = view.pin_count();
            let primitive_count = view.child_count();
            components.push(serde_json::json!({
                "name": entry.lib_ref(),
                "description": entry.description(),
                "pin_count": pin_count,
                "part_count": entry.part_count(),
                "primitive_count": primitive_count,
            }));
        });

        Ok(serde_json::json!({
            "file": path.display().to_string(),
            "unique_id": lib.header().unique_id(),
            "component_count": lib.component_count(),
            "components": components,
        }))
    }
}
