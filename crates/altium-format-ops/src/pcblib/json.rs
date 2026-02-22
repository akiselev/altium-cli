// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! PcbLib JSON export command.

use std::path::Path;

use altium_format::coord::AltiumCoord;
use altium_format::handles::{PcbFootprint, PcbPad};

use crate::output::*;

use super::{extract_pads, open_pcblib};

/// Serializes the library to JSON for LLM processing or external analysis.
pub fn cmd_json(path: &Path, full: bool) -> crate::Result<PcbLibJson> {
    let lib = open_pcblib(path)?;
    let unique_id = lib.unique_id();

    let mut footprints: Vec<FootprintJsonData> = Vec::new();

    let fp_handles = lib.query_all::<PcbFootprint>("#0")?;
    for fp in &fp_handles {
        let fp_name = fp.name()?;
        let fp_rec = fp.read();
        let description = fp_rec.description()?;
        let pad_count = fp.child_count::<PcbPad>();
        let primitive_count = fp.children_len();

        let pads = if full {
            let pad_list = extract_pads(fp)?;
            Some(
                pad_list
                    .iter()
                    .map(|pad| PadJsonData {
                        designator: pad.designator.clone(),
                        shape: pad.shape_name().to_string(),
                        size_x: format!("{:.3}mm", pad.record.top_size_x().to_mm()),
                        size_y: format!("{:.3}mm", pad.record.top_size_y().to_mm()),
                        hole_size: pad.hole_string(),
                        layer: pad.layer_name().to_string(),
                    })
                    .collect(),
            )
        } else {
            None
        };

        footprints.push(FootprintJsonData {
            name: fp_name,
            description,
            pad_count,
            primitive_count,
            pads,
        });
    }

    Ok(PcbLibJson {
        file: path.display().to_string(),
        footprint_count: lib.footprint_count(),
        unique_id,
        footprints,
    })
}
