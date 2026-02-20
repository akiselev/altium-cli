// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! PcbLib browse commands: overview, list, search, info.

use std::collections::HashMap;
use std::path::Path;

use altium_format::v2::handles::{PcbFootprint, PcbPad};
use altium_format::v2::traits::DocumentQuery;

use crate::helpers::*;
use crate::output::*;

use super::{categorize_footprint, count_primitives, extract_pads, open_pcblib};

/// Returns library overview with statistics and footprint category breakdown.
pub fn cmd_overview(path: &Path) -> Result<PcbLibOverview, Box<dyn std::error::Error>> {
    let lib = open_pcblib(path)?;
    let unique_id = lib.unique_id();

    // 1. FOOTPRINTS BY CATEGORY
    let mut categories: HashMap<&'static str, Vec<FootprintSummaryExt>> = HashMap::new();

    let footprints = DocumentQuery::<PcbFootprint>::query_all(&lib, "#0")?;
    for fp in &footprints {
        let fp_name = fp.name();
        let fp_rec = fp.read();
        let description = fp_rec.description();
        let pad_count = fp.child_count::<PcbPad>();
        let category = categorize_footprint(&fp_name, &description);

        categories
            .entry(category)
            .or_default()
            .push(FootprintSummaryExt {
                name: fp_name,
                description,
                pad_count,
            });
    }

    let category_order = [
        "BGA",
        "QFP",
        "QFN/DFN",
        "SOIC/SOP",
        "SOT",
        "DIP",
        "Chip/SMD",
        "Connector",
        "Through-Hole",
        "Electrolytic",
        "Inductor",
        "Crystal/Oscillator",
        "LED",
        "Test Point",
        "Mounting Hole",
        "Other",
    ];

    let mut footprints_by_category = Vec::new();
    for category in category_order.iter() {
        if let Some(mut fps) = categories.remove(*category) {
            fps.sort_by(|a, b| alphanumeric_sort(&a.name, &b.name));
            footprints_by_category.push((category.to_string(), fps));
        }
    }
    for (category, mut fps) in categories {
        if !fps.is_empty() {
            fps.sort_by(|a, b| alphanumeric_sort(&a.name, &b.name));
            footprints_by_category.push((category.to_string(), fps));
        }
    }

    // 2. PAD STATISTICS
    let mut total_pads = 0;
    let mut smd_pads = 0;
    let mut th_pads = 0;
    let mut shape_counts: HashMap<&'static str, usize> = HashMap::new();
    let mut hole_counts: HashMap<String, usize> = HashMap::new();

    for fp in &footprints {
        let pads = extract_pads(fp);
        for pad in &pads {
            total_pads += 1;
            if pad.is_smd() {
                smd_pads += 1;
            } else {
                th_pads += 1;
                let hole_str = pad.hole_string().unwrap_or_default();
                if !hole_str.is_empty() {
                    *hole_counts.entry(hole_str).or_insert(0) += 1;
                }
            }
            *shape_counts.entry(pad.shape_name()).or_insert(0) += 1;
        }
    }

    let mut pad_shapes: Vec<_> = shape_counts
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    pad_shapes.sort_by(|a, b| b.1.cmp(&a.1));

    let mut hole_sizes: Vec<_> = hole_counts.into_iter().collect();
    hole_sizes.sort_by(|a, b| b.1.cmp(&a.1));

    // 3. LARGEST FOOTPRINTS (by pad count)
    let mut by_pads: Vec<(String, String, usize)> = Vec::new();
    for fp in &footprints {
        let fp_name = fp.name();
        let fp_rec = fp.read();
        let description = fp_rec.description();
        let pad_count = fp.child_count::<PcbPad>();
        by_pads.push((fp_name, description, pad_count));
    }
    by_pads.sort_by_key(|(_, _, pads)| std::cmp::Reverse(*pads));

    let largest_footprints = by_pads
        .iter()
        .take(10)
        .map(|(name, description, pads)| FootprintSummaryExt {
            name: name.clone(),
            description: description.clone(),
            pad_count: *pads,
        })
        .collect();

    Ok(PcbLibOverview {
        path: path.display().to_string(),
        total_footprints: lib.footprint_count(),
        unique_id,
        footprints_by_category,
        pad_statistics: PadStatistics {
            total_pads,
            smd_pads,
            th_pads,
            pad_shapes,
        },
        hole_sizes,
        largest_footprints,
    })
}

/// Lists all footprints in the library sorted alphanumerically.
pub fn cmd_list(path: &Path) -> Result<PcbLibFootprintList, Box<dyn std::error::Error>> {
    let lib = open_pcblib(path)?;

    let mut fps: Vec<FootprintSummaryExt> = Vec::new();
    let footprints = DocumentQuery::<PcbFootprint>::query_all(&lib, "#0")?;
    for fp in &footprints {
        let fp_name = fp.name();
        let fp_rec = fp.read();
        fps.push(FootprintSummaryExt {
            name: fp_name,
            description: fp_rec.description(),
            pad_count: fp.child_count::<PcbPad>(),
        });
    }

    fps.sort_by(|a, b| alphanumeric_sort(&a.name, &b.name));

    Ok(PcbLibFootprintList {
        path: path.display().to_string(),
        total_footprints: lib.footprint_count(),
        footprints: fps,
    })
}

/// Searches for footprints matching the query in name or description.
pub fn cmd_search(
    path: &Path,
    query: &str,
) -> Result<PcbLibSearchResults, Box<dyn std::error::Error>> {
    let lib = open_pcblib(path)?;

    let query_lower = query.to_lowercase();
    let has_wildcard = query.contains('*');

    let mut matches: Vec<FootprintSummaryExt> = Vec::new();

    let footprints = DocumentQuery::<PcbFootprint>::query_all(&lib, "#0")?;
    for fp in &footprints {
        let fp_name = fp.name();
        let fp_rec = fp.read();
        let desc = fp_rec.description();
        let name_lower = fp_name.to_lowercase();
        let desc_lower = desc.to_lowercase();

        let is_match = if has_wildcard {
            let pattern = query_lower.replace('*', "");
            name_lower.contains(&pattern) || desc_lower.contains(&pattern)
        } else {
            name_lower.contains(&query_lower) || desc_lower.contains(&query_lower)
        };

        if is_match {
            matches.push(FootprintSummaryExt {
                name: fp_name,
                description: desc,
                pad_count: fp.child_count::<PcbPad>(),
            });
        }
    }

    matches.sort_by(|a, b| {
        let a_exact = a.name.to_lowercase() == query_lower;
        let b_exact = b.name.to_lowercase() == query_lower;
        match (a_exact, b_exact) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => alphanumeric_sort(&a.name, &b.name),
        }
    });

    let total_matches = matches.len();

    Ok(PcbLibSearchResults {
        query: query.to_string(),
        total_matches,
        results: matches,
    })
}

/// Returns detailed library metadata including file info and primitive statistics.
pub fn cmd_info(path: &Path) -> Result<PcbLibInfo, Box<dyn std::error::Error>> {
    let lib = open_pcblib(path)?;
    let unique_id = lib.unique_id();

    let mut primitive_counts: HashMap<&'static str, usize> = HashMap::new();
    let mut total_primitives = 0;

    let footprints = DocumentQuery::<PcbFootprint>::query_all(&lib, "#0")?;
    for fp in &footprints {
        let counts = count_primitives(fp);
        for (name, count) in counts {
            *primitive_counts.entry(name).or_insert(0) += count;
            total_primitives += count;
        }
    }

    let mut primitive_types: Vec<_> = primitive_counts
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    primitive_types.sort_by(|a, b| b.1.cmp(&a.1));

    Ok(PcbLibInfo {
        path: path.display().to_string(),
        footprint_count: lib.footprint_count(),
        unique_id,
        total_primitives,
        primitive_types,
    })
}
