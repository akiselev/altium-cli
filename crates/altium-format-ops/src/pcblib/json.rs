// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! PcbLib JSON export command.

use std::path::Path;

use altium_format::v2::coord::AltiumCoord;

use crate::output::*;

use super::{extract_pads_from_view, open_pcblib};

/// Serializes the library to JSON for LLM processing or external analysis.
pub fn cmd_json(
    path: &Path,
    full: bool,
) -> Result<PcbLibJson, Box<dyn std::error::Error>> {
    let lib = open_pcblib(path)?;
    let unique_id = lib.unique_id();

    let mut footprints: Vec<FootprintJsonData> = Vec::new();

    lib.for_each_footprint_ref(|name, view| {
        let description = view.description();
        let pad_count = view.pad_count();
        let primitive_count = view.primitive_count();

        let pads = if full {
            let pad_list = extract_pads_from_view(&view);
            Some(
                pad_list
                    .iter()
                    .map(|pad| PadJsonData {
                        designator: pad.designator.clone(),
                        shape: pad.shape_name().to_string(),
                        size_x: format!(
                            "{:.3}mm",
                            pad.record.top_size_x().to_mm()
                        ),
                        size_y: format!(
                            "{:.3}mm",
                            pad.record.top_size_y().to_mm()
                        ),
                        hole_size: pad.hole_string(),
                        layer: pad.layer_name().to_string(),
                    })
                    .collect(),
            )
        } else {
            None
        };

        footprints.push(FootprintJsonData {
            name: name.to_string(),
            description,
            pad_count,
            primitive_count,
            pads,
        });
    });

    Ok(PcbLibJson {
        file: path.display().to_string(),
        footprint_count: lib.footprint_count(),
        unique_id,
        footprints,
    })
}
