// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! PcbLib detail commands: footprint, pads, primitives, holes.

use std::collections::HashMap;
use std::path::Path;

use altium_format::coord::AltiumCoord;
use altium_format::handles::PcbFootprint;

use crate::helpers::*;
use crate::output::*;

use super::{
    TYPE_PAD, compute_bounding_box, count_primitives, extract_pads, find_footprint_by_name,
    open_pcblib,
};

/// Returns detailed information about a single footprint.
pub fn cmd_footprint(
    path: &Path,
    name: &str,
    show_primitives: bool,
) -> Result<PcbLibFootprintDetail, Box<dyn std::error::Error>> {
    let lib = open_pcblib(path)?;
    let fp = find_footprint_by_name(&lib, name)?;

    let fp_rec = fp.read();
    let pattern = fp_rec.pattern()?;
    let description = fp_rec.description()?;
    let height = format!("{:.3}mm", fp_rec.height()?.to_mm());
    let total_prims = fp.children_len();

    let pads = extract_pads(&fp);
    let pad_count = pads.len();
    let bounding_box = compute_bounding_box(&pads);

    let mut pad_details: Vec<PadDetail> = pads
        .iter()
        .map(|pad| PadDetail {
            designator: pad.designator.clone(),
            shape: pad.shape_name().to_string(),
            size: pad.size_string(),
            hole: pad.hole_string(),
            layer: pad.layer_name().to_string(),
        })
        .collect();
    pad_details.sort_by(|a, b| alphanumeric_sort(&a.designator, &b.designator));

    let primitive_counts = if show_primitives {
        let counts = count_primitives(&fp);
        let mut counts_vec: Vec<_> = counts
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        counts_vec.sort_by(|a, b| b.1.cmp(&a.1));
        Some(counts_vec)
    } else {
        None
    };

    Ok(PcbLibFootprintDetail {
        pattern: if pattern.is_empty() {
            fp.name()
        } else {
            pattern
        },
        description,
        height,
        pad_count,
        total_primitives: total_prims,
        bounding_box,
        pads: pad_details,
        primitive_counts,
    })
}

/// Lists pads for a specific footprint or all footprints.
pub fn cmd_pads(
    path: &Path,
    footprint: Option<String>,
    _group_by_shape: bool,
) -> Result<PcbLibPadList, Box<dyn std::error::Error>> {
    let lib = open_pcblib(path)?;

    let filter_lower = footprint.as_ref().map(|s| s.to_lowercase());

    let mut all_pads: Vec<PadWithFootprint> = Vec::new();

    let footprints = lib.query_all::<PcbFootprint>("#0")?;
    for fp in &footprints {
        let fp_name = fp.name();
        if let Some(ref filter) = filter_lower {
            if fp_name.to_lowercase() != *filter {
                continue;
            }
        }

        let pads = extract_pads(fp);
        for pad in &pads {
            all_pads.push(PadWithFootprint {
                footprint_name: fp_name.clone(),
                designator: pad.designator.clone(),
                size: pad.size_string(),
                hole: pad.hole_string(),
                shape: pad.shape_name().to_string(),
            });
        }
    }

    all_pads.sort_by(|a, b| {
        let cmp = alphanumeric_sort(&a.footprint_name, &b.footprint_name);
        if cmp == std::cmp::Ordering::Equal {
            alphanumeric_sort(&a.designator, &b.designator)
        } else {
            cmp
        }
    });

    let mut by_shape: HashMap<String, Vec<PadWithFootprint>> = HashMap::new();
    for pad in &all_pads {
        by_shape
            .entry(pad.shape.clone())
            .or_default()
            .push(pad.clone());
    }

    let shape_order = [
        "Round",
        "Rectangular",
        "Octagonal",
        "RoundedRect",
        "NoShape",
    ];
    let mut pads_by_shape = Vec::new();
    for shape in shape_order {
        if let Some(pads) = by_shape.remove(shape) {
            pads_by_shape.push((shape.to_string(), pads));
        }
    }
    for (shape, pads) in by_shape {
        pads_by_shape.push((shape, pads));
    }

    let total_pads = all_pads.len();

    Ok(PcbLibPadList {
        path: path.display().to_string(),
        total_pads,
        pads: all_pads,
        pads_by_shape: Some(pads_by_shape),
    })
}

/// Lists primitives for a footprint.
pub fn cmd_primitives(
    path: &Path,
    footprint: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let lib = open_pcblib(path)?;
    let fp = find_footprint_by_name(&lib, footprint)?;

    let fp_name = fp.name();
    let total_prims = fp.children_len();

    let pads = extract_pads(&fp);
    let mut pad_iter = pads.iter();
    let mut primitives: Vec<serde_json::Value> = Vec::new();
    let mut prim_idx = 0;

    for (type_id, _) in fp.all_children() {
        let type_name = pcb_primitive_type_name(type_id);

        let mut entry = serde_json::json!({
            "index": prim_idx,
            "type": type_name,
            "type_id": type_id,
        });

        if type_id == TYPE_PAD {
            if let Some(pad) = pad_iter.next() {
                entry["designator"] = serde_json::json!(pad.designator.clone());
                entry["position"] = serde_json::json!(format!(
                    "({:.3}mm, {:.3}mm)",
                    pad.record.position_x().to_mm(),
                    pad.record.position_y().to_mm()
                ));
                entry["size"] = serde_json::json!(pad.size_string());
                entry["shape"] = serde_json::json!(pad.shape_name());
            }
        }

        primitives.push(entry);
        prim_idx += 1;
    }

    Ok(serde_json::json!({
        "footprint": fp_name,
        "total_primitives": total_prims,
        "primitives": primitives,
    }))
}

/// Analyze hole sizes across the library.
pub fn cmd_holes(path: &Path) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let lib = open_pcblib(path)?;

    let mut hole_info: HashMap<String, Vec<String>> = HashMap::new();

    let footprints = lib.query_all::<PcbFootprint>("#0")?;
    for fp in &footprints {
        let fp_name = fp.name();
        let pads = extract_pads(fp);
        for pad in &pads {
            if pad.record.hole_size().to_raw() > 0 {
                let hole_str = format!("{:.3}mm", pad.record.hole_size().to_mm());
                hole_info
                    .entry(hole_str)
                    .or_default()
                    .push(format!("{} ({})", fp_name, pad.designator));
            }
        }
    }

    let mut holes: Vec<serde_json::Value> = hole_info
        .into_iter()
        .map(|(size, footprints)| {
            serde_json::json!({
                "hole_size": size,
                "count": footprints.len(),
                "footprints": footprints,
            })
        })
        .collect();

    holes.sort_by(|a, b| {
        let ac = a["count"].as_u64().unwrap_or(0);
        let bc = b["count"].as_u64().unwrap_or(0);
        bc.cmp(&ac)
    });

    let total_holes: usize = holes
        .iter()
        .map(|h| h["count"].as_u64().unwrap_or(0) as usize)
        .sum();

    Ok(serde_json::json!({
        "path": path.display().to_string(),
        "total_through_hole_pads": total_holes,
        "unique_hole_sizes": holes.len(),
        "holes": holes,
    }))
}
