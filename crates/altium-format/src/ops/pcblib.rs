// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! PCB footprint library operations.
//!
//! High-level operations for exploring and manipulating Altium PCB footprint library (.PcbLib) files.
//!
//! **V2 Migration**: This module uses the v2 PCB types which have the correct coordinate scale
//! (10K units/mil) and properly-reversed-engineered binary formats.

// cmd_* functions mix presentation and business logic; separation punted until usage patterns clarify abstraction boundaries (premature abstraction risk)

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

// V2 PCB types - correct coordinate scale (10K units/mil)
use crate::v2::pcb::io::pcblib::{PcbLib, PcbLibFootprint};
use crate::v2::pcb::PcbCoord;
use crate::v2::pcb::pad::PcbPad;

use crate::ops::output::*;

/// Alphanumeric sort that handles numbers embedded in strings naturally.
/// "A1" < "A2" < "A10" instead of "A1" < "A10" < "A2".
fn alphanumeric_sort(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    let mut a_chars = a.chars().peekable();
    let mut b_chars = b.chars().peekable();

    loop {
        match (a_chars.peek(), b_chars.peek()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(&ac), Some(&bc)) => {
                if ac.is_ascii_digit() && bc.is_ascii_digit() {
                    // Extract and compare numbers
                    let a_num: String = a_chars.by_ref().take_while(|c| c.is_ascii_digit()).collect();
                    let b_num: String = b_chars.by_ref().take_while(|c| c.is_ascii_digit()).collect();
                    let a_val: u64 = a_num.parse().unwrap_or(0);
                    let b_val: u64 = b_num.parse().unwrap_or(0);
                    match a_val.cmp(&b_val) {
                        Ordering::Equal => continue,
                        other => return other,
                    }
                } else {
                    a_chars.next();
                    b_chars.next();
                    match ac.cmp(&bc) {
                        Ordering::Equal => continue,
                        other => return other,
                    }
                }
            }
        }
    }
}

fn open_pcblib(path: &Path) -> Result<PcbLib, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    Ok(PcbLib::open(BufReader::new(file)).map_err(|e| e.to_string())?)
}

/// Categorize a footprint by its pattern name.
fn categorize_footprint(pattern: &str, description: &str) -> &'static str {
    let pattern_lower = pattern.to_lowercase();
    let desc_lower = description.to_lowercase();

    // Package types
    if pattern_lower.contains("qfp")
        || pattern_lower.contains("tqfp")
        || pattern_lower.contains("lqfp")
    {
        return "QFP";
    }
    if pattern_lower.contains("qfn")
        || pattern_lower.contains("dfn")
        || pattern_lower.contains("mlf")
    {
        return "QFN/DFN";
    }
    if pattern_lower.contains("bga")
        || pattern_lower.contains("csbga")
        || pattern_lower.contains("wlcsp")
    {
        return "BGA";
    }
    if pattern_lower.contains("soic")
        || pattern_lower.contains("so-")
        || pattern_lower.contains("sop")
    {
        return "SOIC/SOP";
    }
    if pattern_lower.contains("ssop")
        || pattern_lower.contains("tssop")
        || pattern_lower.contains("msop")
    {
        return "SSOP/TSSOP";
    }
    if pattern_lower.contains("sot") {
        return "SOT";
    }
    if pattern_lower.contains("dip") || pattern_lower.contains("pdip") {
        return "DIP";
    }
    if pattern_lower.contains("to-")
        || pattern_lower.contains("to2")
        || pattern_lower.contains("to3")
        || pattern_lower.contains("dpak")
        || pattern_lower.contains("d2pak")
    {
        return "TO/DPAK";
    }

    // Passive components
    if pattern_lower.starts_with("0402")
        || pattern_lower.starts_with("0603")
        || pattern_lower.starts_with("0805")
        || pattern_lower.starts_with("1206")
        || pattern_lower.starts_with("1210")
        || pattern_lower.starts_with("0201")
        || pattern_lower.starts_with("1812")
        || pattern_lower.starts_with("2010")
        || pattern_lower.starts_with("2512")
    {
        return "Chip (SMD)";
    }
    if pattern_lower.contains("cap") || desc_lower.contains("capacitor") {
        return "Capacitor";
    }
    if pattern_lower.contains("res") || desc_lower.contains("resistor") {
        return "Resistor";
    }
    if pattern_lower.contains("ind")
        || pattern_lower.contains("ferrite")
        || desc_lower.contains("inductor")
    {
        return "Inductor";
    }

    // Connectors
    if pattern_lower.contains("header")
        || pattern_lower.contains("conn")
        || pattern_lower.contains("socket")
        || pattern_lower.contains("pin")
        || pattern_lower.contains("terminal")
    {
        return "Connector";
    }
    if pattern_lower.contains("usb") {
        return "USB";
    }
    if pattern_lower.contains("rj45") || pattern_lower.contains("ethernet") {
        return "RJ45/Ethernet";
    }

    // Diodes/LEDs
    if pattern_lower.contains("diode")
        || pattern_lower.contains("sod")
        || pattern_lower.contains("sma")
        || pattern_lower.contains("smb")
        || pattern_lower.contains("smc")
    {
        return "Diode";
    }
    if pattern_lower.contains("led") {
        return "LED";
    }

    // Crystal/Oscillator
    if pattern_lower.contains("crystal")
        || pattern_lower.contains("xtal")
        || pattern_lower.contains("osc")
    {
        return "Crystal/Oscillator";
    }

    // Test points
    if pattern_lower.contains("test") || pattern_lower.contains("tp") {
        return "Test Point";
    }

    // Through-hole
    if pattern_lower.contains("th")
        || pattern_lower.contains("axial")
        || pattern_lower.contains("radial")
    {
        return "Through-Hole";
    }

    "Other"
}

/// Get pad shape name from v2 shape byte.
fn pad_shape_name(shape: u8) -> &'static str {
    match shape {
        1 => "Round",
        2 => "Rectangular",
        3 => "Octagonal",
        4 => "Rounded Rect",
        _ => "Unknown",
    }
}

/// Format layer name from v2 layer byte.
fn layer_name(layer: u8) -> String {
    match layer {
        1 => "Top".to_string(),
        32 => "Bottom".to_string(),
        74 => "Multi".to_string(),
        _ => format!("L{}", layer),
    }
}

/// Format a PcbCoord value for display.
fn fmt_pcb_coord(coord: &PcbCoord) -> String {
    format!("{:.3}mm", coord.to_mms())
}

// ═══════════════════════════════════════════════════════════════════════════
// HIGH-LEVEL COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// Helper to get footprint description from parameters.
fn get_footprint_description(fp: &PcbLibFootprint) -> String {
    fp.parameters.get("DESCRIPTION")
        .cloned()
        .unwrap_or_default()
}

/// Complete library overview.
pub fn cmd_overview(path: &Path) -> Result<PcbLibOverview, Box<dyn std::error::Error>> {
    let lib = open_pcblib(path)?;

    // ─────────────────────────────────────────────────────────────────────────
    // 1. FOOTPRINTS BY CATEGORY
    // ─────────────────────────────────────────────────────────────────────────
    let mut categories: HashMap<&'static str, Vec<FootprintSummaryExt>> = HashMap::new();

    for fp in &lib.footprints {
        let description = get_footprint_description(fp);
        let category = categorize_footprint(&fp.name, &description);
        categories
            .entry(category)
            .or_default()
            .push(FootprintSummaryExt {
                name: fp.name.clone(),
                description,
                pad_count: fp.pads.len(),
            });
    }

    // Sort categories by importance
    let category_order = [
        "QFP",
        "QFN/DFN",
        "BGA",
        "SOIC/SOP",
        "SSOP/TSSOP",
        "SOT",
        "DIP",
        "TO/DPAK",
        "Chip (SMD)",
        "Capacitor",
        "Resistor",
        "Inductor",
        "Connector",
        "USB",
        "RJ45/Ethernet",
        "Diode",
        "LED",
        "Crystal/Oscillator",
        "Test Point",
        "Through-Hole",
        "Other",
    ];

    let mut footprints_by_category = Vec::new();
    for category in category_order.iter() {
        if let Some(footprints) = categories.remove(*category) {
            footprints_by_category.push((category.to_string(), footprints));
        }
    }

    // Add any uncategorized
    for (category, footprints) in categories {
        if !footprints.is_empty() {
            footprints_by_category.push((category.to_string(), footprints));
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 2. PAD STATISTICS
    // ─────────────────────────────────────────────────────────────────────────
    let mut total_pads = 0;
    let mut smd_pads = 0;
    let mut th_pads = 0;
    let mut pad_shapes: HashMap<&'static str, usize> = HashMap::new();

    for fp in &lib.footprints {
        for pad in &fp.pads {
            total_pads += 1;
            // V2 pad: has_hole if hole_size > 0
            if pad.core.hole_size.to_raw() > 0 {
                th_pads += 1;
            } else {
                smd_pads += 1;
            }
            *pad_shapes
                .entry(pad_shape_name(pad.core.top_shape))
                .or_insert(0) += 1;
        }
    }

    let mut pad_shapes_vec: Vec<_> = pad_shapes
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    pad_shapes_vec.sort_by(|a, b| b.1.cmp(&a.1));

    // ─────────────────────────────────────────────────────────────────────────
    // 3. COMMON HOLE SIZES
    // ─────────────────────────────────────────────────────────────────────────
    let mut hole_sizes: HashMap<String, usize> = HashMap::new();
    for fp in &lib.footprints {
        for pad in &fp.pads {
            if pad.core.hole_size.to_raw() > 0 {
                let size_str = fmt_pcb_coord(&pad.core.hole_size);
                *hole_sizes.entry(size_str).or_insert(0) += 1;
            }
        }
    }

    let mut hole_sizes_vec: Vec<_> = hole_sizes.into_iter().collect();
    hole_sizes_vec.sort_by(|a, b| b.1.cmp(&a.1));

    // ─────────────────────────────────────────────────────────────────────────
    // 4. LARGEST FOOTPRINTS
    // ─────────────────────────────────────────────────────────────────────────
    let mut by_pads: Vec<_> = lib.footprints.iter().collect();
    by_pads.sort_by_key(|fp| std::cmp::Reverse(fp.pads.len()));

    let largest_footprints = by_pads
        .iter()
        .take(10)
        .map(|fp| FootprintSummaryExt {
            name: fp.name.clone(),
            description: get_footprint_description(fp),
            pad_count: fp.pads.len(),
        })
        .collect();

    // V2 PcbLib doesn't have a unique_id field at library level - use empty string
    Ok(PcbLibOverview {
        path: path.display().to_string(),
        total_footprints: lib.footprints.len(),
        unique_id: String::new(),
        footprints_by_category,
        pad_statistics: PadStatistics {
            total_pads,
            smd_pads,
            th_pads,
            pad_shapes: pad_shapes_vec,
        },
        hole_sizes: hole_sizes_vec.into_iter().take(10).collect(),
        largest_footprints,
    })
}

/// List all footprints.
pub fn cmd_list(path: &Path) -> Result<PcbLibFootprintList, Box<dyn std::error::Error>> {
    let lib = open_pcblib(path)?;

    let footprints = lib
        .footprints
        .iter()
        .map(|fp| FootprintSummaryExt {
            name: fp.name.clone(),
            description: get_footprint_description(fp),
            pad_count: fp.pads.len(),
        })
        .collect();

    Ok(PcbLibFootprintList {
        path: path.display().to_string(),
        total_footprints: lib.footprints.len(),
        footprints,
    })
}

/// Search for footprints.
pub fn cmd_search(
    path: &Path,
    query: &str,
) -> Result<PcbLibSearchResults, Box<dyn std::error::Error>> {
    let lib = open_pcblib(path)?;

    let query_lower = query.to_lowercase();
    let has_wildcard = query.contains('*');

    let matches: Vec<_> = lib
        .footprints
        .iter()
        .filter(|fp| {
            let name = fp.name.to_lowercase();
            let desc = get_footprint_description(fp).to_lowercase();

            if has_wildcard {
                let pattern = query_lower.replace('*', "");
                name.contains(&pattern) || desc.contains(&pattern)
            } else {
                name.contains(&query_lower) || desc.contains(&query_lower)
            }
        })
        .map(|fp| FootprintSummaryExt {
            name: fp.name.clone(),
            description: get_footprint_description(fp),
            pad_count: fp.pads.len(),
        })
        .collect();

    Ok(PcbLibSearchResults {
        query: query.to_string(),
        total_matches: matches.len(),
        results: matches,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// DETAILED COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// Library info and statistics.
pub fn cmd_info(path: &Path) -> Result<PcbLibInfo, Box<dyn std::error::Error>> {
    let lib = open_pcblib(path)?;

    // Count primitive types across all footprints
    let mut primitive_counts: HashMap<&'static str, usize> = HashMap::new();
    let mut total_primitives = 0;

    for fp in &lib.footprints {
        // V2 stores primitives in separate typed vectors
        let track_count = fp.tracks.len();
        let arc_count = fp.arcs.len();
        let fill_count = fp.fills.len();
        let pad_count = fp.pads.len();
        let via_count = fp.vias.len();
        let text_count = fp.texts.len();
        let region_count = fp.regions.len();
        let body_count = fp.component_bodies.len();
        let raw_count = fp.raw_primitives.len();

        if track_count > 0 { *primitive_counts.entry("Track").or_insert(0) += track_count; }
        if arc_count > 0 { *primitive_counts.entry("Arc").or_insert(0) += arc_count; }
        if fill_count > 0 { *primitive_counts.entry("Fill").or_insert(0) += fill_count; }
        if pad_count > 0 { *primitive_counts.entry("Pad").or_insert(0) += pad_count; }
        if via_count > 0 { *primitive_counts.entry("Via").or_insert(0) += via_count; }
        if text_count > 0 { *primitive_counts.entry("Text").or_insert(0) += text_count; }
        if region_count > 0 { *primitive_counts.entry("Region").or_insert(0) += region_count; }
        if body_count > 0 { *primitive_counts.entry("ComponentBody").or_insert(0) += body_count; }
        if raw_count > 0 { *primitive_counts.entry("Unknown").or_insert(0) += raw_count; }

        total_primitives += track_count + arc_count + fill_count + pad_count
            + via_count + text_count + region_count + body_count + raw_count;
    }

    let mut primitive_types: Vec<_> = primitive_counts
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    primitive_types.sort_by(|a, b| b.1.cmp(&a.1));

    Ok(PcbLibInfo {
        path: path.display().to_string(),
        footprint_count: lib.footprints.len(),
        unique_id: String::new(),
        total_primitives,
        primitive_types,
    })
}

/// Helper to compute total primitive count for a footprint.
fn footprint_primitive_count(fp: &PcbLibFootprint) -> usize {
    fp.tracks.len() + fp.arcs.len() + fp.fills.len() + fp.pads.len()
        + fp.vias.len() + fp.texts.len() + fp.regions.len()
        + fp.component_bodies.len() + fp.raw_primitives.len()
}

/// Helper to calculate bounding box from footprint primitives.
fn calculate_footprint_bounds(fp: &PcbLibFootprint) -> (PcbCoord, PcbCoord, PcbCoord, PcbCoord) {
    let mut min_x = PcbCoord::MAX;
    let mut max_x = PcbCoord::from_raw(i32::MIN);
    let mut min_y = PcbCoord::MAX;
    let mut max_y = PcbCoord::from_raw(i32::MIN);

    for pad in &fp.pads {
        let x = pad.core.position_x;
        let y = pad.core.position_y;
        let half_w = PcbCoord::from_raw(pad.core.top_size_x.to_raw() / 2);
        let half_h = PcbCoord::from_raw(pad.core.top_size_y.to_raw() / 2);
        min_x = min_x.min(x - half_w);
        max_x = max_x.max(x + half_w);
        min_y = min_y.min(y - half_h);
        max_y = max_y.max(y + half_h);
    }

    for track in &fp.tracks {
        min_x = min_x.min(track.start_x.min(track.end_x));
        max_x = max_x.max(track.start_x.max(track.end_x));
        min_y = min_y.min(track.start_y.min(track.end_y));
        max_y = max_y.max(track.start_y.max(track.end_y));
    }

    for arc in &fp.arcs {
        let r = arc.radius;
        min_x = min_x.min(arc.center_x - r);
        max_x = max_x.max(arc.center_x + r);
        min_y = min_y.min(arc.center_y - r);
        max_y = max_y.max(arc.center_y + r);
    }

    (min_x, max_x, min_y, max_y)
}

/// Footprint details.
pub fn cmd_footprint(
    path: &Path,
    name: &str,
    show_primitives: bool,
) -> Result<PcbLibFootprintDetail, Box<dyn std::error::Error>> {
    let lib = open_pcblib(path)?;

    let name_lower = name.to_lowercase();
    let fp = lib
        .footprints
        .iter()
        .find(|f| f.name.to_lowercase() == name_lower)
        .ok_or_else(|| format!("Footprint '{}' not found", name))?;

    // Bounds
    let (min_x, max_x, min_y, max_y) = calculate_footprint_bounds(fp);

    // List pads - need to extract designators from subrecord data
    let mut pads: Vec<(&PcbPad, String)> = fp.pads.iter()
        .map(|pad| {
            // V2 pad has name in sub1 (designator)
            let designator = pad.name();
            (pad, designator)
        })
        .collect();
    pads.sort_by(|a, b| alphanumeric_sort(&a.1, &b.1));

    let pad_details = pads
        .iter()
        .map(|(pad, designator)| {
            let size_str = format!("{}x{}",
                fmt_pcb_coord(&pad.core.top_size_x),
                fmt_pcb_coord(&pad.core.top_size_y));
            let hole_str = if pad.core.hole_size.to_raw() > 0 {
                Some(fmt_pcb_coord(&pad.core.hole_size))
            } else {
                None
            };
            PadDetail {
                designator: designator.clone(),
                shape: pad_shape_name(pad.core.top_shape).to_string(),
                size: size_str,
                hole: hole_str,
                layer: layer_name(pad.core.header.layer),
            }
        })
        .collect();

    let primitive_counts = if show_primitives {
        let mut prim_counts: HashMap<&'static str, usize> = HashMap::new();
        if !fp.tracks.is_empty() { prim_counts.insert("Track", fp.tracks.len()); }
        if !fp.arcs.is_empty() { prim_counts.insert("Arc", fp.arcs.len()); }
        if !fp.fills.is_empty() { prim_counts.insert("Fill", fp.fills.len()); }
        if !fp.pads.is_empty() { prim_counts.insert("Pad", fp.pads.len()); }
        if !fp.vias.is_empty() { prim_counts.insert("Via", fp.vias.len()); }
        if !fp.texts.is_empty() { prim_counts.insert("Text", fp.texts.len()); }
        if !fp.regions.is_empty() { prim_counts.insert("Region", fp.regions.len()); }
        if !fp.component_bodies.is_empty() { prim_counts.insert("ComponentBody", fp.component_bodies.len()); }
        if !fp.raw_primitives.is_empty() { prim_counts.insert("Unknown", fp.raw_primitives.len()); }
        let mut counts: Vec<_> = prim_counts
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        counts.sort_by(|a, b| b.1.cmp(&a.1));
        Some(counts)
    } else {
        None
    };

    // Get height from parameters if available
    let height_str = fp.parameters.get("HEIGHT")
        .and_then(|h| h.parse::<i32>().ok())
        .filter(|&h| h > 0)
        .map(|h| fmt_pcb_coord(&PcbCoord::from_raw(h)))
        .unwrap_or_default();

    Ok(PcbLibFootprintDetail {
        pattern: fp.name.clone(),
        description: get_footprint_description(fp),
        height: height_str,
        pad_count: fp.pads.len(),
        total_primitives: footprint_primitive_count(fp),
        bounding_box: BoundingBox {
            width: fmt_pcb_coord(&(max_x - min_x)),
            height: fmt_pcb_coord(&(max_y - min_y)),
        },
        pads: pad_details,
        primitive_counts,
    })
}

/// List pads.
pub fn cmd_pads(
    path: &Path,
    footprint_filter: Option<String>,
    by_shape: bool,
) -> Result<PcbLibPadList, Box<dyn std::error::Error>> {
    let lib = open_pcblib(path)?;

    let filter_lower = footprint_filter.as_ref().map(|s| s.to_lowercase());

    let mut all_pads: Vec<PadWithFootprint> = Vec::new();

    for fp in &lib.footprints {
        if let Some(ref filter) = filter_lower {
            if !fp.name.to_lowercase().contains(filter) {
                continue;
            }
        }

        for pad in &fp.pads {
            let size_str = format!("{}x{}",
                fmt_pcb_coord(&pad.core.top_size_x),
                fmt_pcb_coord(&pad.core.top_size_y));
            let hole_str = if pad.core.hole_size.to_raw() > 0 {
                Some(fmt_pcb_coord(&pad.core.hole_size))
            } else {
                None
            };
            all_pads.push(PadWithFootprint {
                footprint_name: fp.name.clone(),
                designator: pad.name(),
                size: size_str,
                hole: hole_str,
                shape: pad_shape_name(pad.core.top_shape).to_string(),
            });
        }
    }

    let pads_by_shape = if by_shape {
        let mut by_shape: HashMap<String, Vec<PadWithFootprint>> = HashMap::new();
        for pad in &all_pads {
            by_shape
                .entry(pad.shape.clone())
                .or_default()
                .push(pad.clone());
        }

        let shape_order = ["Round", "Rectangular", "Rounded Rect", "Octagonal"];
        let mut result = Vec::new();
        for shape in shape_order {
            if let Some(pads) = by_shape.remove(shape) {
                result.push((shape.to_string(), pads));
            }
        }
        // Add remaining shapes
        for (shape, pads) in by_shape {
            result.push((shape, pads));
        }
        Some(result)
    } else {
        None
    };

    Ok(PcbLibPadList {
        path: path.display().to_string(),
        total_pads: all_pads.len(),
        pads: all_pads,
        pads_by_shape,
    })
}

/// Show primitives for a footprint.
pub fn cmd_primitives(
    path: &Path,
    name: &str,
) -> Result<PcbLibPrimitiveList, Box<dyn std::error::Error>> {
    let lib = open_pcblib(path)?;

    let name_lower = name.to_lowercase();
    let fp = lib
        .footprints
        .iter()
        .find(|f| f.name.to_lowercase() == name_lower)
        .ok_or_else(|| format!("Footprint '{}' not found", name))?;

    // V2 stores primitives in separate typed vectors
    // Build list following primitive_order if available, otherwise group by type
    let mut primitives: Vec<PrimitiveDetail> = Vec::new();

    // Add pads
    for pad in &fp.pads {
        let hole = if pad.core.hole_size.to_raw() > 0 {
            Some(fmt_pcb_coord(&pad.core.hole_size))
        } else {
            None
        };
        primitives.push(PrimitiveDetail::Pad {
            designator: pad.name(),
            shape: pad_shape_name(pad.core.top_shape).to_string(),
            size: format!("{}x{}",
                fmt_pcb_coord(&pad.core.top_size_x),
                fmt_pcb_coord(&pad.core.top_size_y)),
            hole,
        });
    }

    // Add tracks
    for track in &fp.tracks {
        primitives.push(PrimitiveDetail::Track {
            start_x: fmt_pcb_coord(&track.start_x),
            start_y: fmt_pcb_coord(&track.start_y),
            end_x: fmt_pcb_coord(&track.end_x),
            end_y: fmt_pcb_coord(&track.end_y),
            width: fmt_pcb_coord(&track.width),
        });
    }

    // Add arcs
    for arc in &fp.arcs {
        primitives.push(PrimitiveDetail::Arc {
            center_x: fmt_pcb_coord(&arc.center_x),
            center_y: fmt_pcb_coord(&arc.center_y),
            radius: fmt_pcb_coord(&arc.radius),
            start_angle: arc.start_angle,
            end_angle: arc.end_angle,
        });
    }

    // Add texts
    for text in &fp.texts {
        primitives.push(PrimitiveDetail::Text {
            text: text.text.clone(),
            x: fmt_pcb_coord(&text.position_x),
            y: fmt_pcb_coord(&text.position_y),
        });
    }

    // Add fills
    for fill in &fp.fills {
        primitives.push(PrimitiveDetail::Fill {
            x1: fmt_pcb_coord(&fill.corner1_x),
            y1: fmt_pcb_coord(&fill.corner1_y),
            x2: fmt_pcb_coord(&fill.corner2_x),
            y2: fmt_pcb_coord(&fill.corner2_y),
        });
    }

    // Add regions
    for region in &fp.regions {
        primitives.push(PrimitiveDetail::Region {
            vertex_count: region.outline.len(),
            layer: layer_name(region.header.layer),
        });
    }

    // Add component bodies
    for body in &fp.component_bodies {
        primitives.push(PrimitiveDetail::ComponentBody {
            vertex_count: body.outline.len(),
            height: String::new(), // V2 region doesn't have explicit height
        });
    }

    // Add unknown/raw primitives
    for _ in &fp.raw_primitives {
        primitives.push(PrimitiveDetail::Other {
            primitive_type: "Unknown".to_string(),
        });
    }

    Ok(PcbLibPrimitiveList {
        footprint_name: fp.name.clone(),
        total_primitives: footprint_primitive_count(fp),
        primitives,
    })
}

/// Analyze hole sizes.
pub fn cmd_holes(path: &Path) -> Result<PcbLibHoleAnalysis, Box<dyn std::error::Error>> {
    let lib = open_pcblib(path)?;

    let mut hole_sizes: HashMap<String, Vec<String>> = HashMap::new();

    for fp in &lib.footprints {
        for pad in &fp.pads {
            if pad.core.hole_size.to_raw() > 0 {
                let size_str = fmt_pcb_coord(&pad.core.hole_size);
                hole_sizes
                    .entry(size_str)
                    .or_default()
                    .push(fp.name.clone());
            }
        }
    }

    let mut hole_size_infos: Vec<_> = hole_sizes
        .into_iter()
        .map(|(size, footprints)| {
            // Deduplicate footprint names
            let unique_footprints: std::collections::HashSet<_> = footprints.into_iter().collect();
            let count = unique_footprints.len();
            let example_footprints: Vec<_> = unique_footprints.into_iter().take(3).collect();

            HoleSizeInfo {
                size,
                count,
                example_footprints,
            }
        })
        .collect();

    // Sort by count (descending)
    hole_size_infos.sort_by(|a, b| b.count.cmp(&a.count));

    Ok(PcbLibHoleAnalysis {
        path: path.display().to_string(),
        hole_sizes: hole_size_infos,
    })
}

/// Export as JSON.
pub fn cmd_json(path: &Path, full: bool) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    if full {
        // Full export uses v1 types - stubbed until M7
        return Err("cmd_json --full is stubbed - requires M7 (footprint module migration to V2)".into());
    }

    let lib = open_pcblib(path)?;

    let footprints: Vec<FootprintJsonData> = lib
        .footprints
        .iter()
        .map(|fp| {
            FootprintJsonData {
                name: fp.name.clone(),
                description: get_footprint_description(fp),
                pad_count: fp.pads.len(),
                primitive_count: footprint_primitive_count(fp),
                pads: None,
            }
        })
        .collect();

    let result = PcbLibJson {
        file: path.display().to_string(),
        footprint_count: lib.footprints.len(),
        unique_id: String::new(),
        footprints,
    };
    Ok(serde_json::to_value(&result)?)
}

// ═══════════════════════════════════════════════════════════════════════════
// MEASUREMENT COMMAND IMPLEMENTATION
// ═══════════════════════════════════════════════════════════════════════════

/// Measure distances and dimensions in a footprint.
///
/// **STUBBED**: Requires M7 (footprint module migration to V2).
pub fn cmd_measure(
    _path: &Path,
    _name: &str,
    _measure_type: &str,
    _pad1: Option<String>,
    _pad2: Option<String>,
    _pad: Option<String>,
    _output_json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("cmd_measure is stubbed - requires M7 (footprint module migration to V2)".into())
}

// NOTE: Measurement print functions removed - require M7 (footprint module migration)

// NOTE: JSON output helpers for measurement functions removed - require M7 (footprint module migration)

// ═══════════════════════════════════════════════════════════════════════════
// CREATION/EDITING COMMAND IMPLEMENTATIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Embedded blank PcbLib template.
const BLANK_PCBLIB_TEMPLATE: &[u8] = include_bytes!("../../data/blank/PcbLib1.PcbLib");

// NOTE: crate::footprint imports removed - they were used by stubbed commands (M7)

/// Create a new empty PcbLib file.
pub fn cmd_create(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if path.exists() {
        return Err(format!("File already exists: {}", path.display()).into());
    }

    std::fs::write(path, BLANK_PCBLIB_TEMPLATE)
        .map_err(|e| format!("Error creating file: {}", e))?;

    println!("Created empty PcbLib: {}", path.display());
    Ok(())
}

// NOTE: load_blank_pcblib removed - used by stubbed footprint creation commands (M7)

/// Add a new footprint to a library.
///
/// **STUBBED**: Requires M7 (footprint module migration to V2).
pub fn cmd_add_footprint(
    _path: &Path,
    _name: &str,
    _description: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("cmd_add_footprint is stubbed - requires M7 (footprint module migration to V2)".into())
}

/// Add a pad to a footprint.
///
/// **STUBBED**: Requires M7 (footprint module migration to V2).
#[allow(clippy::too_many_arguments)]
pub fn cmd_add_pad(
    _path: &Path,
    _footprint: &str,
    _designator: &str,
    _x: f64,
    _y: f64,
    _width: f64,
    _height: f64,
    _shape_str: &str,
    _hole: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("cmd_add_pad is stubbed - requires M7 (footprint module migration to V2)".into())
}

/// Add a silkscreen line to a footprint.
///
/// **STUBBED**: Requires M7 (footprint module migration to V2).
pub fn cmd_add_silkscreen(
    _path: &Path,
    _footprint: &str,
    _x1: f64,
    _y1: f64,
    _x2: f64,
    _y2: f64,
    _width: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("cmd_add_silkscreen is stubbed - requires M7 (footprint module migration to V2)".into())
}

/// Add a silkscreen arc to a footprint.
///
/// **STUBBED**: Requires M7 (footprint module migration to V2).
#[allow(clippy::too_many_arguments)]
pub fn cmd_add_arc(
    _path: &Path,
    _footprint: &str,
    _x: f64,
    _y: f64,
    _radius: f64,
    _start_angle: f64,
    _end_angle: f64,
    _width: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("cmd_add_arc is stubbed - requires M7 (footprint module migration to V2)".into())
}

/// Generate a standard chip/passive footprint.
///
/// **STUBBED**: Requires M7 (footprint module migration to V2).
pub fn cmd_gen_chip(
    _path: &Path,
    _size: &str,
    _density_str: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("cmd_gen_chip is stubbed - requires M7 (footprint module migration to V2)".into())
}

/// Render footprint to SVG.
///
/// **STUBBED**: Requires M7 (footprint module migration to V2).
pub fn cmd_render_svg(
    _path: &Path,
    _name: &str,
    _output: Option<PathBuf>,
    _scale: f64,
    _light: bool,
    _no_grid: bool,
    _no_designators: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("cmd_render_svg is stubbed - requires M7 (footprint module migration to V2)".into())
}

/// Render footprint to PNG.
///
/// **STUBBED**: Requires M7 (footprint module migration to V2).
pub fn cmd_render_png(
    _path: &Path,
    _name: &str,
    _output: Option<PathBuf>,
    _scale: f64,
    _target_width: Option<u32>,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("cmd_render_png is stubbed - requires M7 (footprint module migration to V2)".into())
}

/// Render footprint to ASCII art.
///
/// **STUBBED**: Requires M7 (footprint module migration to V2).
pub fn cmd_render_ascii(
    _path: &Path,
    _name: &str,
    _max_width: usize,
    _max_height: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("cmd_render_ascii is stubbed - requires M7 (footprint module migration to V2)".into())
}

// Helper functions

// NOTE: open_or_create_pcblib, save_pcblib, and matches_pattern removed - used by stubbed commands (M7)

// ═══════════════════════════════════════════════════════════════════════════
// JSON INPUT STRUCTURES (for LLM tool calling and structured output)
// ═══════════════════════════════════════════════════════════════════════════

/// JSON schema for a pad in a footprint.
/// All coordinates are in millimeters.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PadJson {
    /// Pad designator (e.g., "1", "2", "A1")
    pub designator: String,
    /// X position in mm (can be negative)
    pub x: f64,
    /// Y position in mm (can be negative)
    pub y: f64,
    /// Pad width in mm
    pub width: f64,
    /// Pad height in mm
    pub height: f64,
    /// Pad shape: "round", "rectangular", "rounded_rect", "octagonal"
    #[serde(default = "default_pad_shape")]
    pub shape: String,
    /// Hole diameter in mm (0 or omit for SMD pad)
    #[serde(default)]
    pub hole: f64,
    /// Rotation angle in degrees (optional)
    #[serde(default)]
    pub rotation: f64,
}

fn default_pad_shape() -> String {
    "rectangular".to_string()
}

/// JSON schema for a silkscreen line.
/// All coordinates are in millimeters.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LineJson {
    /// Start X in mm
    pub x1: f64,
    /// Start Y in mm
    pub y1: f64,
    /// End X in mm
    pub x2: f64,
    /// End Y in mm
    pub y2: f64,
    /// Line width in mm
    #[serde(default = "default_line_width")]
    pub width: f64,
}

fn default_line_width() -> f64 {
    0.15
}

/// JSON schema for a silkscreen arc.
/// All coordinates are in millimeters.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ArcJson {
    /// Center X in mm
    pub x: f64,
    /// Center Y in mm
    pub y: f64,
    /// Radius in mm
    pub radius: f64,
    /// Start angle in degrees (0 = right, 90 = up)
    pub start_angle: f64,
    /// End angle in degrees
    pub end_angle: f64,
    /// Line width in mm
    #[serde(default = "default_line_width")]
    pub width: f64,
}

/// JSON schema for text on a PCB footprint.
/// All coordinates are in millimeters.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TextJson {
    /// X position in mm
    pub x: f64,
    /// Y position in mm
    pub y: f64,
    /// Text content
    pub text: String,
    /// Text height in mm
    #[serde(default = "default_text_height")]
    pub height: f64,
    /// Rotation angle in degrees
    #[serde(default)]
    pub rotation: f64,
    /// Stroke width in mm (for stroke font)
    #[serde(default = "default_stroke_width")]
    pub stroke_width: f64,
    /// Layer: "top_overlay", "bottom_overlay", "top", "bottom"
    #[serde(default = "default_text_layer")]
    pub layer: String,
    /// Mirror the text
    #[serde(default)]
    pub mirrored: bool,
}

fn default_text_height() -> f64 {
    1.0
}

fn default_stroke_width() -> f64 {
    0.15
}

fn default_text_layer() -> String {
    "top_overlay".to_string()
}

// ═══════════════════════════════════════════════════════════════════════════
// HIGH-LEVEL JSON STRUCTURES (datasheet-style specifications)
// ═══════════════════════════════════════════════════════════════════════════

/// JSON schema for a row of pads.
/// Values can include unit suffixes (mm, mil, in) or be plain numbers (interpreted as mm).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PadRowJson {
    /// Number of pads
    pub count: usize,
    /// Center-to-center distance between pads (with optional unit: "0.5mm", "50mil")
    pub pitch: String,
    /// Pad width (with optional unit)
    pub pad_width: String,
    /// Pad height (with optional unit)
    pub pad_height: String,
    /// Row direction: "horizontal" or "vertical"
    #[serde(default = "default_direction")]
    pub direction: String,
    /// Starting pad designator number
    #[serde(default = "default_start")]
    pub start: u32,
    /// X position of first pad (with optional unit, default "0mm")
    #[serde(default)]
    pub x: String,
    /// Y position of first pad (with optional unit, default "0mm")
    #[serde(default)]
    pub y: String,
    /// Pad shape
    #[serde(default = "default_pad_shape_str")]
    pub shape: String,
    /// Hole diameter for through-hole pads (with optional unit, omit or "0" for SMD)
    #[serde(default)]
    pub hole: String,
    /// Use spacing (edge-to-edge) instead of pitch (center-to-center)
    #[serde(default)]
    pub use_spacing: bool,
}

/// JSON schema for dual rows of pads (SOIC, DIP style).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DualRowJson {
    /// Number of pads on each side
    pub pads_per_side: usize,
    /// Center-to-center distance between adjacent pads (with optional unit)
    pub pitch: String,
    /// Distance between row centers / lead span (with optional unit)
    pub row_spacing: String,
    /// Pad width for SMD (with optional unit)
    #[serde(default)]
    pub pad_width: Option<String>,
    /// Pad height for SMD (with optional unit)
    #[serde(default)]
    pub pad_height: Option<String>,
    /// Pad diameter for through-hole (with optional unit)
    #[serde(default)]
    pub pad_diameter: Option<String>,
    /// Hole diameter for through-hole (with optional unit, omit for SMD)
    #[serde(default)]
    pub hole: Option<String>,
    /// Pad shape
    #[serde(default = "default_pad_shape_str")]
    pub shape: String,
}

/// JSON schema for quad pads (QFP style).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QuadPadsJson {
    /// Number of pads on each side
    pub pads_per_side: usize,
    /// Center-to-center distance between adjacent pads (with optional unit)
    pub pitch: String,
    /// Distance between opposite row centers / lead span (with optional unit)
    pub span: String,
    /// Pad width - perpendicular to body edge (with optional unit)
    pub pad_width: String,
    /// Pad height - along body edge (with optional unit)
    pub pad_height: String,
    /// Pad shape
    #[serde(default = "default_pad_shape_str")]
    pub shape: String,
}

/// JSON schema for a grid of pads (BGA style).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PadGridJson {
    /// Number of rows (A, B, C, ...)
    pub rows: usize,
    /// Number of columns (1, 2, 3, ...)
    pub cols: usize,
    /// Center-to-center distance between pads (with optional unit)
    pub pitch: String,
    /// Pad diameter (with optional unit)
    pub pad_diameter: String,
    /// Pad shape (default: "round")
    #[serde(default = "default_round_shape")]
    pub shape: String,
    /// Skip pads within this radius from center (with optional unit, for thermal pad)
    #[serde(default)]
    pub skip_center: String,
}

fn default_direction() -> String {
    "horizontal".to_string()
}

fn default_start() -> u32 {
    1
}

fn default_pad_shape_str() -> String {
    "rectangular".to_string()
}

fn default_round_shape() -> String {
    "round".to_string()
}

/// JSON schema for a complete footprint definition.
/// This is the top-level structure for the add-json command.
///
/// Supports both low-level (individual pads) and high-level (datasheet-style) specifications.
/// High-level constructs are processed first, then individual pads are added.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FootprintJson {
    /// Footprint name (pattern)
    pub name: String,
    /// Footprint description (optional)
    #[serde(default)]
    pub description: String,

    // ─── Low-level primitives (individual elements) ───
    /// List of individual pads (coordinates in mm)
    #[serde(default)]
    pub pads: Vec<PadJson>,
    /// List of silkscreen lines
    #[serde(default)]
    pub lines: Vec<LineJson>,
    /// List of silkscreen arcs
    #[serde(default)]
    pub arcs: Vec<ArcJson>,
    /// List of text elements
    #[serde(default)]
    pub texts: Vec<TextJson>,

    // ─── High-level constructs (datasheet-style, with unit support) ───
    /// Rows of equally-spaced pads
    #[serde(default)]
    pub pad_rows: Vec<PadRowJson>,
    /// Dual rows of pads (SOIC, DIP style)
    #[serde(default)]
    pub dual_rows: Vec<DualRowJson>,
    /// Quad arrangements of pads (QFP style)
    #[serde(default)]
    pub quad_pads: Vec<QuadPadsJson>,
    /// Grids of pads (BGA style)
    #[serde(default)]
    pub pad_grids: Vec<PadGridJson>,
}

/// Add a complete footprint from JSON input.
///
/// **STUBBED**: Requires M7 (footprint module migration to V2).
pub fn cmd_add_json(
    _path: &Path,
    _json_file: Option<String>,
    _json_str: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("cmd_add_json is stubbed - requires M7 (footprint module migration to V2)".into())
}

// NOTE: parse_pcb_layer, parse_pad_shape, parse_unit_value, parse_unit_value_or_mm
// removed - they were used by stubbed footprint creation commands (requires M7)

/// Add a row of pads.
///
/// **STUBBED**: Requires M7 (footprint module migration to V2).
#[allow(clippy::too_many_arguments)]
pub fn cmd_add_pad_row(
    _path: &Path,
    _footprint: &str,
    _count: usize,
    _pitch: &str,
    _pad_width: &str,
    _pad_height: &str,
    _direction: &str,
    _start: u32,
    _x: &str,
    _y: &str,
    _shape_str: &str,
    _hole: &str,
    _use_spacing: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("cmd_add_pad_row is stubbed - requires M7 (footprint module migration to V2)".into())
}

/// Add dual rows of pads (SOIC, DIP style).
///
/// **STUBBED**: Requires M7 (footprint module migration to V2).
#[allow(clippy::too_many_arguments)]
pub fn cmd_add_dual_row(
    _path: &Path,
    _footprint: &str,
    _pads_per_side: usize,
    _pitch: &str,
    _row_spacing: &str,
    _pad_width: Option<&str>,
    _pad_height: Option<&str>,
    _pad_diameter: Option<&str>,
    _hole: Option<&str>,
    _shape_str: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("cmd_add_dual_row is stubbed - requires M7 (footprint module migration to V2)".into())
}

/// Add quad arrangement of pads (QFP style).
///
/// **STUBBED**: Requires M7 (footprint module migration to V2).
#[allow(clippy::too_many_arguments)]
pub fn cmd_add_quad_pads(
    _path: &Path,
    _footprint: &str,
    _pads_per_side: usize,
    _pitch: &str,
    _span: &str,
    _pad_width: &str,
    _pad_height: &str,
    _shape_str: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("cmd_add_quad_pads is stubbed - requires M7 (footprint module migration to V2)".into())
}

/// Add a grid of pads (BGA style).
///
/// **STUBBED**: Requires M7 (footprint module migration to V2).
#[allow(clippy::too_many_arguments)]
pub fn cmd_add_pad_grid(
    _path: &Path,
    _footprint: &str,
    _rows: usize,
    _cols: usize,
    _pitch: &str,
    _pad_diameter: &str,
    _shape_str: &str,
    _skip_center: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("cmd_add_pad_grid is stubbed - requires M7 (footprint module migration to V2)".into())
}

// ═══════════════════════════════════════════════════════════════════════════
// FULL JSON EXPORT/IMPORT (binary-compatible round-trip)
// ═══════════════════════════════════════════════════════════════════════════

/// Full PcbLib export (top-level, like SchLibExport).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbLibExport {
    pub source: Option<String>,
    pub unique_id: String,
    pub footprint_count: usize,
    pub library_parameters: HashMap<String, String>,
    pub file_header_version: String,
    pub file_header_field1: String,
    pub file_header_field2: String,
    pub footprints: Vec<PcbFootprintFullJson>,
}

/// Full footprint JSON with all primitives.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbFootprintFullJson {
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    pub height: i64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub item_guid: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub revision_guid: String,
    pub primitives: Vec<PcbPrimitiveJson>,
}

/// Common fields for all PCB primitives.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbCommonJson {
    pub layer: u8,
    pub flags: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unique_id: Option<String>,
}

/// Tagged enum for all PCB primitive types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PcbPrimitiveJson {
    Arc {
        common: PcbCommonJson,
        location: [i64; 2],
        radius: i64,
        start_angle: f64,
        end_angle: f64,
        width: i64,
    },
    Pad {
        common: PcbCommonJson,
        designator: String,
        location: [i64; 2],
        rotation: f64,
        is_plated: bool,
        jumper_id: i16,
        stack_mode: u8,
        hole_size: i64,
        hole_shape: u8,
        hole_rotation: f64,
        hole_slot_length: i64,
        paste_mask_expansion: PcbMaskExpansionJson,
        solder_mask_expansion: PcbMaskExpansionJson,
        size_layers: Vec<[i64; 2]>,
        shape_layers: Vec<u8>,
        corner_radius_percentage: Vec<u8>,
        offsets_from_hole_center: Vec<[i64; 2]>,
    },
    Via {
        common: PcbCommonJson,
        location: [i64; 2],
        hole_size: i64,
        from_layer: u8,
        to_layer: u8,
        thermal_relief_air_gap_width: i64,
        thermal_relief_conductors: u8,
        thermal_relief_conductors_width: i64,
        solder_mask_expansion: PcbMaskExpansionJson,
        diameter_stack_mode: u8,
        diameters: Vec<i64>,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        unknown_trailer: String,
    },
    Track {
        common: PcbCommonJson,
        start: [i64; 2],
        end: [i64; 2],
        width: i64,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        unknown_trailer: String,
    },
    Text {
        common: PcbCommonJson,
        corner1: [i64; 2],
        corner2: [i64; 2],
        rotation: f64,
        mirrored: bool,
        text_kind: u8,
        stroke_font: i16,
        stroke_width: i64,
        font_bold: bool,
        font_italic: bool,
        font_name: String,
        barcode_lr_margin: i64,
        barcode_tb_margin: i64,
        font_inverted: bool,
        font_inverted_border: i64,
        font_inverted_rect: bool,
        font_inverted_rect_width: i64,
        font_inverted_rect_height: i64,
        font_inverted_rect_justification: u8,
        font_inverted_rect_text_offset: i64,
        text: String,
        wide_strings_index: i32,
    },
    Fill {
        common: PcbCommonJson,
        corner1: [i64; 2],
        corner2: [i64; 2],
        rotation: f64,
    },
    Region {
        common: PcbCommonJson,
        parameters: HashMap<String, String>,
        outline: Vec<[i64; 2]>,
    },
    ComponentBody {
        common: PcbCommonJson,
        parameters: HashMap<String, String>,
        outline: Vec<[i64; 2]>,
    },
    Unknown {
        object_id: u8,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        raw_data: String,
    },
}

/// Mask expansion mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbMaskExpansionJson {
    pub mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<i64>,
}


// NOTE: JSON conversion functions removed - they depend on v1 types and are used only by stubbed commands (M7)
// Removed: common_to_json, common_from_json, mask_expansion_to_json, mask_expansion_from_json,
// vec_to_coord_array, vec_to_shape_array, vec_to_u8_array, vec_to_coord_diameter_array,
// primitive_to_json, primitive_from_json, footprint_to_full_json, footprint_from_full_json,
// cmd_json_full, cmd_add_json_full_export

