// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! SchLib JSON export command.

use std::path::Path;

use altium_format::v2::traits::DocumentQuery;
use altium_format::v2::views::{SchComponent, SchPin};

use super::open_schlib;

/// Serializes the library to JSON for LLM processing or external analysis.
pub fn cmd_json(path: &Path, full: bool) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let mut lib = open_schlib(path)?;

    if full {
        Ok(serde_json::to_value(&lib)?)
    } else {
        let mut components: Vec<serde_json::Value> = Vec::new();

        DocumentQuery::<SchComponent>::query_all(&mut lib, "#1")?.for_each_mut(|entry, view| {
            let pin_count = view.child_keys::<SchPin>().count();
            let primitive_count = view.children_len();
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
