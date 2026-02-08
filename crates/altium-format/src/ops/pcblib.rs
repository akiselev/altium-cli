// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! PCB footprint library operations.
//!
//! High-level operations for exploring and manipulating Altium PCB footprint library (.PcbLib) files.

// cmd_* functions mix presentation and business logic; separation punted until usage patterns clarify abstraction boundaries (premature abstraction risk)

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::{Path, PathBuf};

use base64::Engine;
use png;
use resvg;
use serde::{Deserialize, Serialize};
use serde_json;

use crate::dump::fmt_coord_val;
use crate::footprint::{
    AsciiOptions, FootprintBuilder, PadRowDirection, SvgOptions, render_ascii, render_svg,
};
use crate::io::PcbLib;
use crate::records::pcb::{
    PcbArc, PcbComponent, PcbComponentBody, PcbFill, PcbFlags, PcbObjectId, PcbPad,
    PcbPadHoleShape, PcbPadShape, PcbPrimitiveCommon, PcbRecord, PcbRectangularBase, PcbRegion,
    PcbStackMode, PcbText, PcbTextJustification, PcbTextKind, PcbTextStrokeFont, PcbTrack, PcbVia,
};
use crate::types::{Coord, CoordPoint, Layer, MaskExpansion, ParameterCollection, Unit};

use super::util::alphanumeric_sort;
use crate::ops::output::*;

fn open_pcblib(path: &Path) -> Result<PcbLib, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    Ok(PcbLib::open(BufReader::new(file))?)
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

/// Get pad shape name.
fn pad_shape_name(shape: PcbPadShape) -> &'static str {
    shape.name()
}

/// Get record type name.
fn record_type_name(record: &PcbRecord) -> &'static str {
    match record {
        PcbRecord::Arc(_) => "Arc",
        PcbRecord::Pad(_) => "Pad",
        PcbRecord::Via(_) => "Via",
        PcbRecord::Track(_) => "Track",
        PcbRecord::Text(_) => "Text",
        PcbRecord::Fill(_) => "Fill",
        PcbRecord::Region(_) => "Region",
        PcbRecord::ComponentBody(_) => "ComponentBody",
        PcbRecord::Polygon(_) => "Polygon",
        PcbRecord::Dimension(_) => "Dimension",
        PcbRecord::Coordinate(_) => "Coordinate",
        PcbRecord::Unknown { .. } => "Unknown",
    }
}

/// Format layer name.
fn layer_name(layer: &Layer) -> String {
    match layer.to_byte() {
        1 => "Top".to_string(),
        32 => "Bottom".to_string(),
        74 => "Multi".to_string(),
        _ => format!("L{}", layer.to_byte()),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// HIGH-LEVEL COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// Complete library overview.
pub fn cmd_overview(path: &Path) -> Result<PcbLibOverview, Box<dyn std::error::Error>> {
    let lib = open_pcblib(path)?;

    // ─────────────────────────────────────────────────────────────────────────
    // 1. FOOTPRINTS BY CATEGORY
    // ─────────────────────────────────────────────────────────────────────────
    let mut categories: HashMap<&'static str, Vec<FootprintSummaryExt>> = HashMap::new();

    for comp in lib.iter() {
        let category = categorize_footprint(&comp.pattern, &comp.description);
        categories
            .entry(category)
            .or_default()
            .push(FootprintSummaryExt {
                name: comp.pattern.clone(),
                description: comp.description.clone(),
                pad_count: comp.pad_count(),
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

    for comp in lib.iter() {
        for pad in comp.pads() {
            total_pads += 1;
            if pad.has_hole() {
                th_pads += 1;
            } else {
                smd_pads += 1;
            }
            *pad_shapes
                .entry(pad_shape_name(pad.shape_top()))
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
    for comp in lib.iter() {
        for pad in comp.pads() {
            if pad.has_hole() && pad.hole_size.to_raw() > 0 {
                let size_str = fmt_coord_val(&pad.hole_size);
                *hole_sizes.entry(size_str).or_insert(0) += 1;
            }
        }
    }

    let mut hole_sizes_vec: Vec<_> = hole_sizes.into_iter().collect();
    hole_sizes_vec.sort_by(|a, b| b.1.cmp(&a.1));

    // ─────────────────────────────────────────────────────────────────────────
    // 4. LARGEST FOOTPRINTS
    // ─────────────────────────────────────────────────────────────────────────
    let mut by_pads: Vec<_> = lib.iter().collect();
    by_pads.sort_by_key(|b| std::cmp::Reverse(b.pad_count()));

    let largest_footprints = by_pads
        .iter()
        .take(10)
        .map(|comp| FootprintSummaryExt {
            name: comp.pattern.clone(),
            description: comp.description.clone(),
            pad_count: comp.pad_count(),
        })
        .collect();

    Ok(PcbLibOverview {
        path: path.display().to_string(),
        total_footprints: lib.components.len(),
        unique_id: lib.unique_id.clone(),
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
        .iter()
        .map(|comp| FootprintSummaryExt {
            name: comp.pattern.clone(),
            description: comp.description.clone(),
            pad_count: comp.pad_count(),
        })
        .collect();

    Ok(PcbLibFootprintList {
        path: path.display().to_string(),
        total_footprints: lib.components.len(),
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
        .iter()
        .filter(|comp| {
            let name = comp.pattern.to_lowercase();
            let desc = comp.description.to_lowercase();

            if has_wildcard {
                let pattern = query_lower.replace('*', "");
                name.contains(&pattern) || desc.contains(&pattern)
            } else {
                name.contains(&query_lower) || desc.contains(&query_lower)
            }
        })
        .map(|comp| FootprintSummaryExt {
            name: comp.pattern.clone(),
            description: comp.description.clone(),
            pad_count: comp.pad_count(),
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

    for comp in lib.iter() {
        for prim in &comp.primitives {
            let name = record_type_name(prim);
            *primitive_counts.entry(name).or_insert(0) += 1;
            total_primitives += 1;
        }
    }

    let mut primitive_types: Vec<_> = primitive_counts
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    primitive_types.sort_by(|a, b| b.1.cmp(&a.1));

    Ok(PcbLibInfo {
        path: path.display().to_string(),
        footprint_count: lib.components.len(),
        unique_id: lib.unique_id.clone(),
        total_primitives,
        primitive_types,
    })
}

/// Footprint details.
pub fn cmd_footprint(
    path: &Path,
    name: &str,
    show_primitives: bool,
) -> Result<PcbLibFootprintDetail, Box<dyn std::error::Error>> {
    let lib = open_pcblib(path)?;

    let name_lower = name.to_lowercase();
    let comp = lib
        .iter()
        .find(|c| c.pattern.to_lowercase() == name_lower)
        .ok_or_else(|| format!("Footprint '{}' not found", name))?;

    // Bounds
    let bounds = comp.calculate_bounds();

    // List pads
    let mut pads: Vec<&PcbPad> = comp.pads().collect();
    pads.sort_by(|a, b| alphanumeric_sort(&a.designator, &b.designator));

    let pad_details = pads
        .iter()
        .map(|pad| {
            let size = pad.size_top();
            let size_str = format!("{}x{}", fmt_coord_val(&size.x), fmt_coord_val(&size.y));
            let hole_str = if pad.has_hole() {
                Some(fmt_coord_val(&pad.hole_size))
            } else {
                None
            };
            PadDetail {
                designator: pad.designator.clone(),
                shape: pad_shape_name(pad.shape_top()).to_string(),
                size: size_str,
                hole: hole_str,
                layer: layer_name(&pad.common.layer),
            }
        })
        .collect();

    let primitive_counts = if show_primitives {
        let mut prim_counts: HashMap<&'static str, usize> = HashMap::new();
        for prim in &comp.primitives {
            *prim_counts.entry(record_type_name(prim)).or_insert(0) += 1;
        }
        let mut counts: Vec<_> = prim_counts
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        counts.sort_by(|a, b| b.1.cmp(&a.1));
        Some(counts)
    } else {
        None
    };

    Ok(PcbLibFootprintDetail {
        pattern: comp.pattern.clone(),
        description: comp.description.clone(),
        height: if comp.height.to_raw() > 0 {
            fmt_coord_val(&comp.height)
        } else {
            String::new()
        },
        pad_count: comp.pad_count(),
        total_primitives: comp.primitive_count(),
        bounding_box: BoundingBox {
            width: fmt_coord_val(&bounds.width()),
            height: fmt_coord_val(&bounds.height()),
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

    for comp in lib.iter() {
        if let Some(ref filter) = filter_lower {
            if !comp.pattern.to_lowercase().contains(filter) {
                continue;
            }
        }

        for pad in comp.pads() {
            let size = pad.size_top();
            let size_str = format!("{}x{}", fmt_coord_val(&size.x), fmt_coord_val(&size.y));
            let hole_str = if pad.has_hole() {
                Some(fmt_coord_val(&pad.hole_size))
            } else {
                None
            };
            all_pads.push(PadWithFootprint {
                footprint_name: comp.pattern.clone(),
                designator: pad.designator.clone(),
                size: size_str,
                hole: hole_str,
                shape: pad_shape_name(pad.shape_top()).to_string(),
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
    let comp = lib
        .iter()
        .find(|c| c.pattern.to_lowercase() == name_lower)
        .ok_or_else(|| format!("Footprint '{}' not found", name))?;

    let primitives = comp
        .primitives
        .iter()
        .map(|prim| match prim {
            PcbRecord::Pad(p) => {
                let size = p.size_top();
                let hole = if p.has_hole() {
                    Some(fmt_coord_val(&p.hole_size))
                } else {
                    None
                };
                PrimitiveDetail::Pad {
                    designator: p.designator.clone(),
                    shape: pad_shape_name(p.shape_top()).to_string(),
                    size: format!("{}x{}", fmt_coord_val(&size.x), fmt_coord_val(&size.y)),
                    hole,
                }
            }
            PcbRecord::Track(t) => PrimitiveDetail::Track {
                start_x: fmt_coord_val(&t.start.x),
                start_y: fmt_coord_val(&t.start.y),
                end_x: fmt_coord_val(&t.end.x),
                end_y: fmt_coord_val(&t.end.y),
                width: fmt_coord_val(&t.width),
            },
            PcbRecord::Arc(a) => PrimitiveDetail::Arc {
                center_x: fmt_coord_val(&a.location.x),
                center_y: fmt_coord_val(&a.location.y),
                radius: fmt_coord_val(&a.radius),
                start_angle: a.start_angle,
                end_angle: a.end_angle,
            },
            PcbRecord::Text(t) => PrimitiveDetail::Text {
                text: t.text.clone(),
                x: fmt_coord_val(&t.base.corner1.x),
                y: fmt_coord_val(&t.base.corner1.y),
            },
            PcbRecord::Fill(f) => PrimitiveDetail::Fill {
                x1: fmt_coord_val(&f.base.corner1.x),
                y1: fmt_coord_val(&f.base.corner1.y),
                x2: fmt_coord_val(&f.base.corner2.x),
                y2: fmt_coord_val(&f.base.corner2.y),
            },
            PcbRecord::Region(r) => PrimitiveDetail::Region {
                vertex_count: r.outline.len(),
                layer: layer_name(&r.common.layer),
            },
            PcbRecord::ComponentBody(b) => PrimitiveDetail::ComponentBody {
                vertex_count: b.outline.len(),
                height: fmt_coord_val(&b.overall_height),
            },
            _ => PrimitiveDetail::Other {
                primitive_type: record_type_name(prim).to_string(),
            },
        })
        .collect();

    Ok(PcbLibPrimitiveList {
        footprint_name: comp.pattern.clone(),
        total_primitives: comp.primitive_count(),
        primitives,
    })
}

/// Analyze hole sizes.
pub fn cmd_holes(path: &Path) -> Result<PcbLibHoleAnalysis, Box<dyn std::error::Error>> {
    let lib = open_pcblib(path)?;

    let mut hole_sizes: HashMap<String, Vec<String>> = HashMap::new();

    for comp in lib.iter() {
        for pad in comp.pads() {
            if pad.has_hole() && pad.hole_size.to_raw() > 0 {
                let size_str = fmt_coord_val(&pad.hole_size);
                hole_sizes
                    .entry(size_str)
                    .or_default()
                    .push(comp.pattern.clone());
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
    let lib = open_pcblib(path)?;

    if full {
        let export = cmd_json_full(&lib, path);
        Ok(serde_json::to_value(&export)?)
    } else {
        let footprints: Vec<FootprintJsonData> = lib
            .iter()
            .map(|comp| FootprintJsonData {
                name: comp.pattern.clone(),
                description: comp.description.clone(),
                pad_count: comp.pad_count(),
                primitive_count: comp.primitive_count(),
                pads: None,
            })
            .collect();

        let result = PcbLibJson {
            file: path.display().to_string(),
            footprint_count: lib.components.len(),
            unique_id: lib.unique_id.clone(),
            footprints,
        };
        Ok(serde_json::to_value(&result)?)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// MEASUREMENT COMMAND IMPLEMENTATION
// ═══════════════════════════════════════════════════════════════════════════

use crate::footprint::{
    Measurement, analyze_pitch, generate_report, measure_dimensions, measure_pad,
    measure_pad_distance, minimum_pad_clearance, pad_to_silkscreen_clearance,
};

/// Measure distances and dimensions in a footprint.
pub fn cmd_measure(
    path: &Path,
    name: &str,
    measure_type: &str,
    pad1: Option<String>,
    pad2: Option<String>,
    pad: Option<String>,
    output_json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let lib = open_pcblib(path)?;

    let name_lower = name.to_lowercase();
    let component = lib
        .iter()
        .find(|c| c.pattern.to_lowercase() == name_lower || matches_pattern(&c.pattern, name))
        .ok_or_else(|| format!("Footprint '{}' not found", name))?;

    match measure_type.to_lowercase().as_str() {
        "all" | "report" => {
            let report = generate_report(component);

            if output_json {
                print_measurement_report_json(&report)?;
            } else {
                print_measurement_report(&report);
            }
        }
        "distance" | "dist" => {
            let p1 = pad1.ok_or("--pad1 required for distance measurement")?;
            let p2 = pad2.ok_or("--pad2 required for distance measurement")?;

            let dist = measure_pad_distance(component, &p1, &p2).ok_or_else(|| {
                format!("Could not measure distance between pads {} and {}", p1, p2)
            })?;

            if output_json {
                print_distance_json(&dist)?;
            } else {
                println!("Distance: {} to {}", dist.pad1, dist.pad2);
                println!("  Center-to-center: {}", dist.center_to_center.display());
                println!("  Edge-to-edge:     {}", dist.edge_to_edge.display());
            }
        }
        "pitch" => {
            let pitches = analyze_pitch(component);

            if output_json {
                print_pitch_json(&pitches)?;
            } else if pitches.is_empty() {
                println!("No regular pitch detected (footprint may have irregular pad spacing)");
            } else {
                println!("Pitch Analysis for: {}", component.pattern);
                println!("═══════════════════════════════════════════════════════════════");
                for pitch_info in &pitches {
                    println!(
                        "\n{} pitch: {}",
                        pitch_info.direction,
                        pitch_info.pitch.display()
                    );
                    println!("  {} pad pairs with this spacing", pitch_info.count);
                    for (p1, p2, dist) in pitch_info.pad_pairs.iter().take(5) {
                        println!("    {} ↔ {}: {}", p1, p2, dist.display());
                    }
                    if pitch_info.pad_pairs.len() > 5 {
                        println!("    ... and {} more pairs", pitch_info.pad_pairs.len() - 5);
                    }
                }
            }
        }
        "dimensions" | "dims" | "bounds" => {
            let dims = measure_dimensions(component);

            if output_json {
                print_dimensions_json(&dims)?;
            } else {
                println!("Dimensions for: {}", component.pattern);
                println!("═══════════════════════════════════════════════════════════════");
                println!("  Width:  {}", dims.width.display());
                println!("  Height: {}", dims.height.display());
                println!(
                    "  X range: {} to {}",
                    dims.min_x.display(),
                    dims.max_x.display()
                );
                println!(
                    "  Y range: {} to {}",
                    dims.min_y.display(),
                    dims.max_y.display()
                );
            }
        }
        "clearance" | "clear" => {
            let pad_clear = minimum_pad_clearance(component);
            let silk_clear = pad_to_silkscreen_clearance(component);

            if output_json {
                print_clearance_json(pad_clear.as_ref(), silk_clear.as_ref())?;
            } else {
                println!("Clearance Analysis for: {}", component.pattern);
                println!("═══════════════════════════════════════════════════════════════");

                if let Some(pc) = pad_clear {
                    println!("\nMinimum pad-to-pad clearance: {}", pc.clearance.display());
                    println!("  Location: {}", pc.location);
                } else {
                    println!("\nNo pad-to-pad clearance (single pad or overlapping pads)");
                }

                if let Some(sc) = silk_clear {
                    println!("\nPad-to-silkscreen clearance: {}", sc.clearance.display());
                    println!("  Location: {}", sc.location);
                } else {
                    println!("\nNo silkscreen elements found");
                }
            }
        }
        "pad" => {
            let des = pad.ok_or("--pad required for pad measurement")?;
            let info =
                measure_pad(component, &des).ok_or_else(|| format!("Pad '{}' not found", des))?;

            if output_json {
                print_pad_json(&info)?;
            } else {
                println!("Pad {} info:", info.designator);
                println!("═══════════════════════════════════════════════════════════════");
                println!("  Position: ({}, {})", info.x.display(), info.y.display());
                println!(
                    "  Size:     {} x {}",
                    info.width.display(),
                    info.height.display()
                );
                println!("  Shape:    {}", info.shape);
                if let Some(hole) = &info.hole {
                    println!("  Hole:     {}", hole.display());
                } else {
                    println!("  Type:     SMD");
                }
            }
        }
        "pads" => {
            let report = generate_report(component);

            if output_json {
                print_all_pads_json(&report.pads)?;
            } else {
                println!("All Pads for: {}", component.pattern);
                println!("═══════════════════════════════════════════════════════════════");
                println!(
                    "\n{:<6} {:>10} {:>10} {:>10} {:>10} {:>10} Shape",
                    "Pad", "X (mm)", "Y (mm)", "W (mm)", "H (mm)", "Hole"
                );
                println!(
                    "{:-<6} {:->10} {:->10} {:->10} {:->10} {:->10} {:-<12}",
                    "", "", "", "", "", "", ""
                );

                for pad_info in &report.pads {
                    let hole_str = pad_info
                        .hole
                        .as_ref()
                        .map(|h| format!("{:.3}", h.mm))
                        .unwrap_or_else(|| "-".to_string());

                    println!(
                        "{:<6} {:>10.3} {:>10.3} {:>10.3} {:>10.3} {:>10} {}",
                        pad_info.designator,
                        pad_info.x.mm,
                        pad_info.y.mm,
                        pad_info.width.mm,
                        pad_info.height.mm,
                        hole_str,
                        pad_info.shape
                    );
                }
            }
        }
        _ => {
            return Err(format!(
                "Unknown measurement type: '{}'. Use: all, distance, pitch, dimensions, clearance, pad, pads",
                measure_type
            ).into());
        }
    }

    Ok(())
}

/// Print full measurement report in human-readable format.
fn print_measurement_report(report: &crate::footprint::MeasurementReport) {
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║                  FOOTPRINT MEASUREMENT REPORT                  ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!("\nFootprint: {}", report.name);

    // Dimensions
    println!("\n┌─────────────────────────────────────────────────────────────────┐");
    println!("│ DIMENSIONS                                                       │");
    println!("└─────────────────────────────────────────────────────────────────┘");
    println!("  Width:  {}", report.dimensions.width.display());
    println!("  Height: {}", report.dimensions.height.display());

    if let Some(span) = &report.row_span {
        println!("  Row span: {}", span.display());
    }

    // Pads summary
    println!("\n┌─────────────────────────────────────────────────────────────────┐");
    println!(
        "│ PADS ({} total)                                                   │",
        report.pads.len()
    );
    println!("└─────────────────────────────────────────────────────────────────┘");

    println!(
        "\n{:<6} {:>10} {:>10} {:>10} {:>10} Shape",
        "Pad", "X (mm)", "Y (mm)", "W (mm)", "H (mm)"
    );
    println!(
        "{:-<6} {:->10} {:->10} {:->10} {:->10} {:-<12}",
        "", "", "", "", "", ""
    );

    for pad in &report.pads {
        println!(
            "{:<6} {:>10.3} {:>10.3} {:>10.3} {:>10.3} {}",
            pad.designator, pad.x.mm, pad.y.mm, pad.width.mm, pad.height.mm, pad.shape
        );
    }

    // Pitch analysis
    if !report.pitch.is_empty() {
        println!("\n┌─────────────────────────────────────────────────────────────────┐");
        println!("│ PITCH ANALYSIS                                                   │");
        println!("└─────────────────────────────────────────────────────────────────┘");

        for pitch_info in &report.pitch {
            println!(
                "\n  {} pitch: {}",
                pitch_info.direction,
                pitch_info.pitch.display()
            );
            println!("    {} adjacent pad pairs", pitch_info.count);
        }
    }

    // Clearances
    println!("\n┌─────────────────────────────────────────────────────────────────┐");
    println!("│ CLEARANCES                                                       │");
    println!("└─────────────────────────────────────────────────────────────────┘");

    if let Some(pc) = &report.min_pad_clearance {
        println!("\n  Minimum pad-to-pad gap: {}", pc.clearance.display());
        println!("    {}", pc.location);
    }

    if let Some(sc) = &report.silkscreen_clearance {
        println!("\n  Pad-to-silkscreen: {}", sc.clearance.display());
        println!("    {}", sc.location);
    }
}

// JSON output helpers

fn print_measurement_report_json(
    report: &crate::footprint::MeasurementReport,
) -> Result<(), Box<dyn std::error::Error>> {
    #[derive(Serialize)]
    struct MeasurementJson {
        mm: f64,
        mils: f64,
    }

    impl From<&Measurement> for MeasurementJson {
        fn from(m: &Measurement) -> Self {
            MeasurementJson {
                mm: m.mm,
                mils: m.mils,
            }
        }
    }

    #[derive(Serialize)]
    struct PadInfoJson {
        designator: String,
        x_mm: f64,
        y_mm: f64,
        width_mm: f64,
        height_mm: f64,
        hole_mm: Option<f64>,
        shape: String,
    }

    #[derive(Serialize)]
    struct PitchJson {
        pitch: MeasurementJson,
        direction: String,
        count: usize,
    }

    #[derive(Serialize)]
    struct ClearanceJson {
        feature1: String,
        feature2: String,
        clearance: MeasurementJson,
        location: String,
    }

    #[derive(Serialize)]
    struct ReportJson {
        name: String,
        dimensions: DimensionsJson,
        pads: Vec<PadInfoJson>,
        pitch: Vec<PitchJson>,
        min_pad_clearance: Option<ClearanceJson>,
        silkscreen_clearance: Option<ClearanceJson>,
        row_span: Option<MeasurementJson>,
    }

    #[derive(Serialize)]
    struct DimensionsJson {
        width: MeasurementJson,
        height: MeasurementJson,
        min_x: MeasurementJson,
        max_x: MeasurementJson,
        min_y: MeasurementJson,
        max_y: MeasurementJson,
    }

    let output = ReportJson {
        name: report.name.clone(),
        dimensions: DimensionsJson {
            width: (&report.dimensions.width).into(),
            height: (&report.dimensions.height).into(),
            min_x: (&report.dimensions.min_x).into(),
            max_x: (&report.dimensions.max_x).into(),
            min_y: (&report.dimensions.min_y).into(),
            max_y: (&report.dimensions.max_y).into(),
        },
        pads: report
            .pads
            .iter()
            .map(|p| PadInfoJson {
                designator: p.designator.clone(),
                x_mm: p.x.mm,
                y_mm: p.y.mm,
                width_mm: p.width.mm,
                height_mm: p.height.mm,
                hole_mm: p.hole.as_ref().map(|h| h.mm),
                shape: p.shape.clone(),
            })
            .collect(),
        pitch: report
            .pitch
            .iter()
            .map(|p| PitchJson {
                pitch: (&p.pitch).into(),
                direction: p.direction.clone(),
                count: p.count,
            })
            .collect(),
        min_pad_clearance: report.min_pad_clearance.as_ref().map(|c| ClearanceJson {
            feature1: c.feature1.clone(),
            feature2: c.feature2.clone(),
            clearance: (&c.clearance).into(),
            location: c.location.clone(),
        }),
        silkscreen_clearance: report.silkscreen_clearance.as_ref().map(|c| ClearanceJson {
            feature1: c.feature1.clone(),
            feature2: c.feature2.clone(),
            clearance: (&c.clearance).into(),
            location: c.location.clone(),
        }),
        row_span: report.row_span.as_ref().map(|s| s.into()),
    };

    let json = serde_json::to_string_pretty(&output).map_err(|e| e.to_string())?;
    println!("{}", json);
    Ok(())
}

fn print_distance_json(
    dist: &crate::footprint::PadDistance,
) -> Result<(), Box<dyn std::error::Error>> {
    #[derive(Serialize)]
    struct DistanceJson {
        pad1: String,
        pad2: String,
        center_to_center_mm: f64,
        center_to_center_mils: f64,
        edge_to_edge_mm: f64,
        edge_to_edge_mils: f64,
    }

    let output = DistanceJson {
        pad1: dist.pad1.clone(),
        pad2: dist.pad2.clone(),
        center_to_center_mm: dist.center_to_center.mm,
        center_to_center_mils: dist.center_to_center.mils,
        edge_to_edge_mm: dist.edge_to_edge.mm,
        edge_to_edge_mils: dist.edge_to_edge.mils,
    };

    let json = serde_json::to_string_pretty(&output).map_err(|e| e.to_string())?;
    println!("{}", json);
    Ok(())
}

fn print_pitch_json(
    pitches: &[crate::footprint::PitchAnalysis],
) -> Result<(), Box<dyn std::error::Error>> {
    #[derive(Serialize)]
    struct PitchJson {
        direction: String,
        pitch_mm: f64,
        pitch_mils: f64,
        pad_pair_count: usize,
    }

    let output: Vec<PitchJson> = pitches
        .iter()
        .map(|p| PitchJson {
            direction: p.direction.clone(),
            pitch_mm: p.pitch.mm,
            pitch_mils: p.pitch.mils,
            pad_pair_count: p.count,
        })
        .collect();

    let json = serde_json::to_string_pretty(&output).map_err(|e| e.to_string())?;
    println!("{}", json);
    Ok(())
}

fn print_dimensions_json(
    dims: &crate::footprint::FootprintDimensions,
) -> Result<(), Box<dyn std::error::Error>> {
    #[derive(Serialize)]
    struct DimsJson {
        width_mm: f64,
        width_mils: f64,
        height_mm: f64,
        height_mils: f64,
        min_x_mm: f64,
        max_x_mm: f64,
        min_y_mm: f64,
        max_y_mm: f64,
    }

    let output = DimsJson {
        width_mm: dims.width.mm,
        width_mils: dims.width.mils,
        height_mm: dims.height.mm,
        height_mils: dims.height.mils,
        min_x_mm: dims.min_x.mm,
        max_x_mm: dims.max_x.mm,
        min_y_mm: dims.min_y.mm,
        max_y_mm: dims.max_y.mm,
    };

    let json = serde_json::to_string_pretty(&output).map_err(|e| e.to_string())?;
    println!("{}", json);
    Ok(())
}

fn print_clearance_json(
    pad_clear: Option<&crate::footprint::ClearanceResult>,
    silk_clear: Option<&crate::footprint::ClearanceResult>,
) -> Result<(), Box<dyn std::error::Error>> {
    #[derive(Serialize)]
    struct ClearanceJson {
        feature1: String,
        feature2: String,
        clearance_mm: f64,
        clearance_mils: f64,
        location: String,
    }

    #[derive(Serialize)]
    struct Output {
        pad_to_pad: Option<ClearanceJson>,
        pad_to_silkscreen: Option<ClearanceJson>,
    }

    let output = Output {
        pad_to_pad: pad_clear.map(|c| ClearanceJson {
            feature1: c.feature1.clone(),
            feature2: c.feature2.clone(),
            clearance_mm: c.clearance.mm,
            clearance_mils: c.clearance.mils,
            location: c.location.clone(),
        }),
        pad_to_silkscreen: silk_clear.map(|c| ClearanceJson {
            feature1: c.feature1.clone(),
            feature2: c.feature2.clone(),
            clearance_mm: c.clearance.mm,
            clearance_mils: c.clearance.mils,
            location: c.location.clone(),
        }),
    };

    let json = serde_json::to_string_pretty(&output).map_err(|e| e.to_string())?;
    println!("{}", json);
    Ok(())
}

fn print_pad_json(info: &crate::footprint::PadInfo) -> Result<(), Box<dyn std::error::Error>> {
    #[derive(Serialize)]
    struct PadJson {
        designator: String,
        x_mm: f64,
        y_mm: f64,
        width_mm: f64,
        height_mm: f64,
        hole_mm: Option<f64>,
        shape: String,
        is_smd: bool,
    }

    let output = PadJson {
        designator: info.designator.clone(),
        x_mm: info.x.mm,
        y_mm: info.y.mm,
        width_mm: info.width.mm,
        height_mm: info.height.mm,
        hole_mm: info.hole.as_ref().map(|h| h.mm),
        shape: info.shape.clone(),
        is_smd: info.hole.is_none(),
    };

    let json = serde_json::to_string_pretty(&output).map_err(|e| e.to_string())?;
    println!("{}", json);
    Ok(())
}

fn print_all_pads_json(
    pads: &[crate::footprint::PadInfo],
) -> Result<(), Box<dyn std::error::Error>> {
    #[derive(Serialize)]
    struct PadJson {
        designator: String,
        x_mm: f64,
        y_mm: f64,
        width_mm: f64,
        height_mm: f64,
        hole_mm: Option<f64>,
        shape: String,
    }

    let output: Vec<PadJson> = pads
        .iter()
        .map(|p| PadJson {
            designator: p.designator.clone(),
            x_mm: p.x.mm,
            y_mm: p.y.mm,
            width_mm: p.width.mm,
            height_mm: p.height.mm,
            hole_mm: p.hole.as_ref().map(|h| h.mm),
            shape: p.shape.clone(),
        })
        .collect();

    let json = serde_json::to_string_pretty(&output).map_err(|e| e.to_string())?;
    println!("{}", json);
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// CREATION/EDITING COMMAND IMPLEMENTATIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Embedded blank PcbLib template.
const BLANK_PCBLIB_TEMPLATE: &[u8] = include_bytes!("../../data/blank/PcbLib1.PcbLib");

use crate::footprint::{ChipSpec, IpcDensity};

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

fn load_blank_pcblib() -> Result<PcbLib, Box<dyn std::error::Error>> {
    Ok(PcbLib::open(Cursor::new(BLANK_PCBLIB_TEMPLATE))?)
}

/// Add a new footprint to a library.
pub fn cmd_add_footprint(
    path: &Path,
    name: &str,
    description: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut lib = open_or_create_pcblib(path)?;

    // Check if footprint already exists
    if lib.components.iter().any(|c| c.pattern == name) {
        return Err(format!("Footprint '{}' already exists", name).into());
    }

    let mut det = ();
    let mut component = PcbComponent::new_deterministic(name, &mut det);
    if let Some(desc) = description {
        component.set_description(desc);
    }

    lib.components.push(component);
    save_pcblib(path, &lib)?;

    println!("Added footprint '{}' to {}", name, path.display());
    Ok(())
}

/// Add a pad to a footprint.
#[allow(clippy::too_many_arguments)]
pub fn cmd_add_pad(
    path: &Path,
    footprint: &str,
    designator: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    shape_str: &str,
    hole: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut lib = open_pcblib(path)?;

    let component = lib
        .components
        .iter_mut()
        .find(|c| c.pattern == footprint)
        .ok_or_else(|| format!("Footprint '{}' not found", footprint))?;

    // Parse shape
    let shape = match shape_str.to_lowercase().as_str() {
        "round" => PcbPadShape::Round,
        "rectangular" | "rect" => PcbPadShape::Rectangular,
        "rounded_rect" | "roundedrect" => PcbPadShape::RoundedRectangle,
        "octagonal" | "oct" => PcbPadShape::Octagonal,
        _ => return Err(format!("Unknown pad shape: {}", shape_str).into()),
    };

    // Create pad using FootprintBuilder helper
    let mut builder = FootprintBuilder::new(footprint);
    if hole > 0.0 {
        builder.add_th_pad(designator, x, y, width.max(height), hole, shape);
    } else {
        builder.add_smd_pad(designator, x, y, width, height, shape);
    }

    // Extract the pad from the built component
    let mut det = ();
    let temp = builder.build_deterministic(&mut det);
    if let Some(PcbRecord::Pad(pad)) = temp.primitives.into_iter().next() {
        component.add_primitive(PcbRecord::Pad(pad));
    }

    save_pcblib(path, &lib)?;
    println!(
        "Added pad '{}' to footprint '{}' at ({}, {}) mm",
        designator, footprint, x, y
    );
    Ok(())
}

/// Add a silkscreen line to a footprint.
pub fn cmd_add_silkscreen(
    path: &Path,
    footprint: &str,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    width: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut lib = open_pcblib(path)?;

    let component = lib
        .components
        .iter_mut()
        .find(|c| c.pattern == footprint)
        .ok_or_else(|| format!("Footprint '{}' not found", footprint))?;

    let track = PcbTrack {
        common: PcbPrimitiveCommon {
            layer: Layer::TOP_OVERLAY,
            flags: PcbFlags::UNLOCKED | PcbFlags::UNKNOWN8,
            unique_id: None,
        },
        start: CoordPoint::from_mms(x1, y1),
        end: CoordPoint::from_mms(x2, y2),
        width: Coord::from_mms(width),
        unknown: vec![0u8; 16],
    };

    component.add_primitive(PcbRecord::Track(track));
    save_pcblib(path, &lib)?;

    println!("Added silkscreen line to footprint '{}'", footprint);
    Ok(())
}

/// Add a silkscreen arc to a footprint.
#[allow(clippy::too_many_arguments)]
pub fn cmd_add_arc(
    path: &Path,
    footprint: &str,
    x: f64,
    y: f64,
    radius: f64,
    start_angle: f64,
    end_angle: f64,
    width: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut lib = open_pcblib(path)?;

    let component = lib
        .components
        .iter_mut()
        .find(|c| c.pattern == footprint)
        .ok_or_else(|| format!("Footprint '{}' not found", footprint))?;

    let arc = PcbArc {
        common: PcbPrimitiveCommon {
            layer: Layer::TOP_OVERLAY,
            flags: PcbFlags::UNLOCKED | PcbFlags::UNKNOWN8,
            unique_id: None,
        },
        location: CoordPoint::from_mms(x, y),
        radius: Coord::from_mms(radius),
        start_angle,
        end_angle,
        width: Coord::from_mms(width),
    };

    component.add_primitive(PcbRecord::Arc(arc));
    save_pcblib(path, &lib)?;

    println!(
        "Added silkscreen arc to footprint '{}' (center: ({}, {}) mm, radius: {} mm, {:.0}° to {:.0}°)",
        footprint, x, y, radius, start_angle, end_angle
    );
    Ok(())
}

/// Generate a standard chip/passive footprint.
pub fn cmd_gen_chip(
    path: &Path,
    size: &str,
    density_str: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut lib = open_or_create_pcblib(path)?;

    let spec = match size.to_uppercase().as_str() {
        "0201" => ChipSpec::chip_0201(),
        "0402" => ChipSpec::chip_0402(),
        "0603" => ChipSpec::chip_0603(),
        "0805" => ChipSpec::chip_0805(),
        "1206" => ChipSpec::chip_1206(),
        _ => {
            return Err(format!(
                "Unknown chip size: {}. Supported: 0201, 0402, 0603, 0805, 1206",
                size
            )
            .into());
        }
    };

    let density = parse_density(density_str)?;
    let mut det = ();
    let component = spec.to_footprint(density).build_deterministic(&mut det);
    let name = component.pattern.clone();

    // Check if already exists
    if lib.components.iter().any(|c| c.pattern == name) {
        return Err(format!("Footprint '{}' already exists", name).into());
    }

    lib.components.push(component);
    save_pcblib(path, &lib)?;

    println!(
        "Generated chip footprint '{}' with {} density",
        name, density_str
    );
    Ok(())
}

fn parse_density(s: &str) -> Result<IpcDensity, Box<dyn std::error::Error>> {
    match s.to_lowercase().as_str() {
        "most" | "a" | "dense" => Ok(IpcDensity::MostDense),
        "nominal" | "b" | "normal" => Ok(IpcDensity::Nominal),
        "least" | "c" | "loose" => Ok(IpcDensity::LeastDense),
        _ => Err(format!("Unknown density: {}. Use: most, nominal, least", s).into()),
    }
}

pub fn cmd_render_svg(
    path: &Path,
    name: &str,
    output: Option<PathBuf>,
    scale: f64,
    light: bool,
    no_grid: bool,
    no_designators: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;

    let lib = open_pcblib(path)?;

    // Find the footprint
    let name_lower = name.to_lowercase();
    let component = lib
        .iter()
        .find(|c| c.pattern.to_lowercase() == name_lower || matches_pattern(&c.pattern, name))
        .ok_or_else(|| format!("Footprint '{}' not found", name))?;

    // Build options
    let mut options = if light {
        SvgOptions::light()
    } else {
        SvgOptions::default()
    };
    options.scale = scale;
    options.show_grid = !no_grid;
    options.show_designators = !no_designators;

    // Render to SVG
    let svg = render_svg(component, &options);

    // Determine output path
    let output_path = output.unwrap_or_else(|| {
        PathBuf::from(format!(
            "{}.svg",
            component.pattern.replace(['/', '\\', ' '], "_")
        ))
    });

    // Write to file
    fs::write(&output_path, &svg).map_err(|e| format!("Error writing SVG: {}", e))?;

    println!(
        "Rendered footprint '{}' to {}",
        component.pattern,
        output_path.display()
    );
    println!("  Size: {} bytes", svg.len());
    println!("  Theme: {}", if light { "light" } else { "dark" });
    println!("  Scale: {} px/mil", scale);

    Ok(())
}

pub fn cmd_render_png(
    path: &Path,
    name: &str,
    output: Option<PathBuf>,
    scale: f64,
    target_width: Option<u32>,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::io::BufWriter;

    let lib = open_pcblib(path)?;

    // Find the footprint
    let name_lower = name.to_lowercase();
    let component = lib
        .iter()
        .find(|c| c.pattern.to_lowercase() == name_lower || matches_pattern(&c.pattern, name))
        .ok_or_else(|| format!("Footprint '{}' not found", name))?;

    // Use Altium-style colors (dark background, colored layers)
    let options = SvgOptions {
        scale,
        show_grid: false, // No grid for PNG
        show_designators: true,
        ..Default::default()
    };

    // Render to SVG first
    let svg_data = render_svg(component, &options);

    // Parse SVG and render to PNG using resvg
    let tree = resvg::usvg::Tree::from_str(&svg_data, &resvg::usvg::Options::default())
        .map_err(|e| format!("Error parsing SVG: {}", e))?;

    // Calculate dimensions
    let svg_size = tree.size();
    let (width, height) = if let Some(w) = target_width {
        let h = (w as f32 * svg_size.height() / svg_size.width()) as u32;
        (w, h)
    } else {
        (svg_size.width() as u32, svg_size.height() as u32)
    };

    // Create pixmap and render
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| "Failed to create pixmap".to_string())?;

    // Fill with dark background
    pixmap.fill(resvg::tiny_skia::Color::from_rgba8(30, 30, 30, 255));

    // Render SVG onto pixmap
    let scale_x = width as f32 / svg_size.width();
    let scale_y = height as f32 / svg_size.height();
    let transform = resvg::tiny_skia::Transform::from_scale(scale_x, scale_y);

    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Determine output path
    let output_path = output.unwrap_or_else(|| {
        PathBuf::from(format!(
            "{}.png",
            component.pattern.replace(['/', '\\', ' '], "_")
        ))
    });

    // Write PNG
    let file = File::create(&output_path).map_err(|e| format!("Error creating file: {}", e))?;
    let writer = BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);

    let mut png_writer = encoder
        .write_header()
        .map_err(|e| format!("Error writing PNG header: {}", e))?;
    png_writer
        .write_image_data(pixmap.data())
        .map_err(|e| format!("Error writing PNG data: {}", e))?;

    println!(
        "Rendered footprint '{}' to {}",
        component.pattern,
        output_path.display()
    );
    println!("  Size: {}x{} pixels", width, height);

    Ok(())
}

pub fn cmd_render_ascii(
    path: &Path,
    name: &str,
    max_width: usize,
    max_height: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let lib = open_pcblib(path)?;

    // Find the footprint
    let name_lower = name.to_lowercase();
    let component = lib
        .iter()
        .find(|c| c.pattern.to_lowercase() == name_lower || matches_pattern(&c.pattern, name))
        .ok_or_else(|| format!("Footprint '{}' not found", name))?;

    // Build options
    let options = AsciiOptions {
        max_width,
        max_height,
        ..Default::default()
    };

    // Render and print
    let ascii = render_ascii(component, &options);
    println!("{}", ascii);

    Ok(())
}

// Helper functions

/// Open an existing PcbLib or create a new blank one if the file doesn't exist.
pub fn open_or_create(path: &Path) -> Result<PcbLib, Box<dyn std::error::Error>> {
    open_or_create_pcblib(path)
}

fn open_or_create_pcblib(path: &Path) -> Result<PcbLib, Box<dyn std::error::Error>> {
    if path.exists() {
        open_pcblib(path)
    } else {
        load_blank_pcblib()
    }
}

fn save_pcblib(path: &Path, lib: &PcbLib) -> Result<(), Box<dyn std::error::Error>> {
    Ok(lib.save_to_file(path)?)
}

/// Simple wildcard pattern matching (supports * and ?).
fn matches_pattern(text: &str, pattern: &str) -> bool {
    let text = text.to_lowercase();
    let pattern = pattern.to_lowercase();

    fn matches(text: &[char], pattern: &[char]) -> bool {
        match (text.first(), pattern.first()) {
            (None, None) => true,
            (None, Some('*')) => matches(text, &pattern[1..]),
            (None, Some(_)) => false,
            (Some(_), None) => false,
            (Some(_), Some('*')) => {
                // * can match zero or more characters
                matches(text, &pattern[1..]) || matches(&text[1..], pattern)
            }
            (Some(_), Some('?')) => {
                // ? matches exactly one character
                matches(&text[1..], &pattern[1..])
            }
            (Some(t), Some(p)) => *t == *p && matches(&text[1..], &pattern[1..]),
        }
    }

    let text_chars: Vec<char> = text.chars().collect();
    let pattern_chars: Vec<char> = pattern.chars().collect();
    matches(&text_chars, &pattern_chars)
}

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
pub fn cmd_add_json(
    path: &Path,
    json_file: Option<String>,
    json_str: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::{self, Read as IoRead};

    // Read JSON from file, stdin, or command line
    let json_content = match (json_file, json_str) {
        (_, Some(s)) => s,
        (Some(ref path), None) if path == "-" => {
            let mut buffer = String::new();
            io::stdin()
                .read_to_string(&mut buffer)
                .map_err(|e| format!("Error reading from stdin: {}", e))?;
            buffer
        }
        (Some(ref file_path), None) => std::fs::read_to_string(file_path)
            .map_err(|e| format!("Error reading file '{}': {}", file_path, e))?,
        (None, None) => {
            return Err("Must provide either --file <path> or --json <string>"
                .to_string()
                .into());
        }
    };

    // Try full export format first
    if let Ok(export) = serde_json::from_str::<PcbLibExport>(&json_content) {
        return cmd_add_json_full_export(path, &export);
    }

    // Try single full footprint
    if let Ok(full_fp) = serde_json::from_str::<PcbFootprintFullJson>(&json_content) {
        let mut lib = open_or_create_pcblib(path)?;
        if lib.components.iter().any(|c| c.pattern == full_fp.name) {
            return Err(format!("Footprint '{}' already exists", full_fp.name).into());
        }
        let component = footprint_from_full_json(&full_fp)
            .map_err(|e| format!("Error converting footprint: {}", e))?;
        let name = component.pattern.clone();
        lib.components.push(component);
        save_pcblib(path, &lib)?;
        println!("Added full footprint '{}' to {}", name, path.display());
        return Ok(());
    }

    // Parse JSON as FootprintJson (fall back)
    let footprint_def: FootprintJson =
        serde_json::from_str(&json_content).map_err(|e| format!("Invalid JSON: {}", e))?;

    // Open or create library
    let mut lib = open_or_create_pcblib(path)?;

    // Check if footprint already exists
    if lib
        .components
        .iter()
        .any(|c| c.pattern == footprint_def.name)
    {
        return Err(format!("Footprint '{}' already exists", footprint_def.name).into());
    }

    // Create footprint using FootprintBuilder
    let mut builder = FootprintBuilder::new(&footprint_def.name);

    if !footprint_def.description.is_empty() {
        builder = builder.description(&footprint_def.description);
    }

    // ─── Process high-level constructs first (datasheet-style) ───

    // Add pad rows
    for row in &footprint_def.pad_rows {
        let pitch_mm = parse_unit_value_or_mm(&row.pitch)?;
        let pad_width_mm = parse_unit_value_or_mm(&row.pad_width)?;
        let pad_height_mm = parse_unit_value_or_mm(&row.pad_height)?;
        let x_mm = if row.x.is_empty() {
            0.0
        } else {
            parse_unit_value_or_mm(&row.x)?
        };
        let y_mm = if row.y.is_empty() {
            0.0
        } else {
            parse_unit_value_or_mm(&row.y)?
        };
        let hole_mm = if row.hole.is_empty() {
            0.0
        } else {
            parse_unit_value_or_mm(&row.hole)?
        };
        let dir = PadRowDirection::try_parse(&row.direction)
            .ok_or_else(|| format!("Invalid direction '{}' in pad_row", row.direction))?;
        let shape = parse_pad_shape(&row.shape)?;

        if hole_mm > 0.0 {
            let pad_diameter = pad_width_mm.max(pad_height_mm);
            if row.use_spacing {
                let pad_along_row = match dir {
                    PadRowDirection::Horizontal => pad_width_mm,
                    PadRowDirection::Vertical => pad_height_mm,
                };
                let effective_pitch = pitch_mm + pad_along_row;
                builder.add_th_pad_row(
                    row.count,
                    effective_pitch,
                    pad_diameter,
                    hole_mm,
                    x_mm,
                    y_mm,
                    dir,
                    row.start,
                    shape,
                );
            } else {
                builder.add_th_pad_row(
                    row.count,
                    pitch_mm,
                    pad_diameter,
                    hole_mm,
                    x_mm,
                    y_mm,
                    dir,
                    row.start,
                    shape,
                );
            }
        } else if row.use_spacing {
            builder.add_pad_row_with_spacing(
                row.count,
                pitch_mm,
                pad_width_mm,
                pad_height_mm,
                x_mm,
                y_mm,
                dir,
                row.start,
                shape,
            );
        } else {
            builder.add_pad_row(
                row.count,
                pitch_mm,
                pad_width_mm,
                pad_height_mm,
                x_mm,
                y_mm,
                dir,
                row.start,
                shape,
            );
        }
    }

    // Add dual rows
    for dual in &footprint_def.dual_rows {
        let pitch_mm = parse_unit_value_or_mm(&dual.pitch)?;
        let row_spacing_mm = parse_unit_value_or_mm(&dual.row_spacing)?;
        let shape = parse_pad_shape(&dual.shape)?;

        if let Some(ref hole_str) = dual.hole {
            // Through-hole
            let hole_mm = parse_unit_value_or_mm(hole_str)?;
            let pad_dia_mm = if let Some(ref d) = dual.pad_diameter {
                parse_unit_value_or_mm(d)?
            } else if let Some(ref w) = dual.pad_width {
                parse_unit_value_or_mm(w)?
            } else {
                return Err("Through-hole dual_row requires pad_diameter or pad_width"
                    .to_string()
                    .into());
            };
            builder.add_dual_row_th(
                dual.pads_per_side,
                pitch_mm,
                row_spacing_mm,
                pad_dia_mm,
                hole_mm,
                shape,
            );
        } else {
            // SMD
            let pad_width_mm = dual
                .pad_width
                .as_ref()
                .ok_or("SMD dual_row requires pad_width")?;
            let pad_height_mm = dual
                .pad_height
                .as_ref()
                .ok_or("SMD dual_row requires pad_height")?;
            let pad_width_mm = parse_unit_value_or_mm(pad_width_mm)?;
            let pad_height_mm = parse_unit_value_or_mm(pad_height_mm)?;
            builder.add_dual_row_smd(
                dual.pads_per_side,
                pitch_mm,
                row_spacing_mm,
                pad_width_mm,
                pad_height_mm,
                shape,
            );
        }
    }

    // Add quad pads
    for quad in &footprint_def.quad_pads {
        let pitch_mm = parse_unit_value_or_mm(&quad.pitch)?;
        let span_mm = parse_unit_value_or_mm(&quad.span)?;
        let pad_width_mm = parse_unit_value_or_mm(&quad.pad_width)?;
        let pad_height_mm = parse_unit_value_or_mm(&quad.pad_height)?;
        let shape = parse_pad_shape(&quad.shape)?;
        builder.add_quad_pads_smd(
            quad.pads_per_side,
            pitch_mm,
            span_mm,
            pad_width_mm,
            pad_height_mm,
            shape,
        );
    }

    // Add pad grids
    for grid in &footprint_def.pad_grids {
        let pitch_mm = parse_unit_value_or_mm(&grid.pitch)?;
        let pad_diameter_mm = parse_unit_value_or_mm(&grid.pad_diameter)?;
        let skip_center_mm = if grid.skip_center.is_empty() {
            0.0
        } else {
            parse_unit_value_or_mm(&grid.skip_center)?
        };
        let shape = parse_pad_shape(&grid.shape)?;
        builder.add_pad_grid(
            grid.rows,
            grid.cols,
            pitch_mm,
            pad_diameter_mm,
            shape,
            skip_center_mm,
        );
    }

    // ─── Process low-level primitives (individual pads, lines, etc.) ───

    // Add individual pads
    for pad in &footprint_def.pads {
        let shape = parse_pad_shape(&pad.shape)?;

        if pad.hole > 0.0 {
            builder.add_th_pad(
                &pad.designator,
                pad.x,
                pad.y,
                pad.width.max(pad.height),
                pad.hole,
                shape,
            );
        } else {
            builder.add_smd_pad(&pad.designator, pad.x, pad.y, pad.width, pad.height, shape);
        }
    }

    // Add silkscreen lines
    for line in &footprint_def.lines {
        builder.add_silkscreen_line(line.x1, line.y1, line.x2, line.y2, line.width);
    }

    // Add arcs
    for arc in &footprint_def.arcs {
        builder.add_silkscreen_arc(
            arc.x,
            arc.y,
            arc.radius,
            arc.start_angle,
            arc.end_angle,
            arc.width,
        );
    }

    let mut det = ();
    let mut component = builder.build_deterministic(&mut det);

    // Add text elements (not supported by FootprintBuilder, add directly)
    for text_def in &footprint_def.texts {
        let layer = parse_pcb_layer(&text_def.layer)?;

        let text = PcbText::new(
            text_def.x,
            text_def.y,
            &text_def.text,
            text_def.height,
            text_def.stroke_width,
            text_def.rotation,
            text_def.mirrored,
            layer,
        );
        component.add_primitive(PcbRecord::Text(text));
    }

    let pad_count = component.pad_count();
    let line_count = footprint_def.lines.len();
    let arc_count = footprint_def.arcs.len();
    let text_count = footprint_def.texts.len();

    lib.components.push(component);
    save_pcblib(path, &lib)?;

    // Build summary of added primitives
    let mut parts = vec![format!("{} pads", pad_count)];
    if line_count > 0 {
        parts.push(format!("{} lines", line_count));
    }
    if arc_count > 0 {
        parts.push(format!("{} arcs", arc_count));
    }
    if text_count > 0 {
        parts.push(format!("{} texts", text_count));
    }

    println!(
        "Added footprint '{}' with {} to {}",
        footprint_def.name,
        parts.join(", "),
        path.display()
    );

    Ok(())
}

fn parse_pcb_layer(s: &str) -> Result<Layer, String> {
    match s.to_lowercase().replace('_', "").as_str() {
        "topoverlay" | "silkscreen" | "top_overlay" => Ok(Layer::TOP_OVERLAY),
        "bottomoverlay" | "bottom_overlay" => Ok(Layer::BOTTOM_OVERLAY),
        "top" | "toplayer" => Ok(Layer::TOP_LAYER),
        "bottom" | "bottomlayer" => Ok(Layer::BOTTOM_LAYER),
        _ => Err(format!(
            "Unknown layer: {}. Use: top_overlay, bottom_overlay, top, bottom",
            s
        )),
    }
}

fn parse_pad_shape(s: &str) -> Result<PcbPadShape, String> {
    match s.to_lowercase().as_str() {
        "round" => Ok(PcbPadShape::Round),
        "rectangular" | "rect" => Ok(PcbPadShape::Rectangular),
        "rounded_rect" | "roundedrect" | "rounded_rectangle" => Ok(PcbPadShape::RoundedRectangle),
        "octagonal" | "oct" => Ok(PcbPadShape::Octagonal),
        _ => Err(format!(
            "Unknown pad shape: {}. Use: round, rectangular, rounded_rect, octagonal",
            s
        )),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// HIGH-LEVEL PAD COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// Parse a value with unit suffix (e.g., "0.5mm", "50mil", "0.05in").
/// Plain numbers without a unit suffix are treated as mm (not DxpDefault).
fn parse_unit_value(s: &str) -> Result<f64, String> {
    let s = s.trim();

    match Unit::parse_with_unit(s) {
        Ok((coord, unit)) => {
            if unit == Unit::DxpDefault {
                // Bare number without unit — treat as mm, not DxpDefault (10 mils).
                // Users of CLI commands expect mm as the default unit.
                let value: f64 = s.parse().map_err(|_| {
                    format!("Invalid value '{}': expected number with optional unit", s)
                })?;
                Ok(value)
            } else {
                Ok(coord.to_mms())
            }
        }
        Err(e) => Err(format!("Invalid value '{}': {:?}", s, e)),
    }
}

/// Parse a value with optional unit suffix, defaulting to mm for plain numbers.
/// Handles: "0.5mm", "50mil", "0.05in", "0.5" (interpreted as mm)
fn parse_unit_value_or_mm(s: &str) -> Result<f64, String> {
    let s = s.trim();

    // Try parsing with unit suffix first
    if let Ok((coord, _unit)) = Unit::parse_with_unit(s) {
        return Ok(coord.to_mms());
    }

    // If no unit suffix, try as plain number (interpreted as mm)
    s.parse::<f64>().map_err(|_| {
        format!(
            "Invalid value '{}': expected number with optional unit (e.g., '0.5mm', '50mil')",
            s
        )
    })
}

/// Add a row of pads.
#[allow(clippy::too_many_arguments)]
pub fn cmd_add_pad_row(
    path: &Path,
    footprint: &str,
    count: usize,
    pitch: &str,
    pad_width: &str,
    pad_height: &str,
    direction: &str,
    start: u32,
    x: &str,
    y: &str,
    shape_str: &str,
    hole: &str,
    use_spacing: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut lib = open_pcblib(path)?;

    let component = lib
        .components
        .iter_mut()
        .find(|c| c.pattern == footprint)
        .ok_or_else(|| format!("Footprint '{}' not found", footprint))?;

    // Parse all values with units
    let pitch_mm = parse_unit_value(pitch)?;
    let pad_width_mm = parse_unit_value(pad_width)?;
    let pad_height_mm = parse_unit_value(pad_height)?;
    let x_mm = parse_unit_value(x)?;
    let y_mm = parse_unit_value(y)?;
    let hole_mm = parse_unit_value(hole)?;

    let dir = PadRowDirection::try_parse(direction).ok_or_else(|| {
        format!(
            "Invalid direction '{}'. Use: horizontal (h/x) or vertical (v/y)",
            direction
        )
    })?;

    let shape = parse_pad_shape(shape_str)?;

    // Create pad row using FootprintBuilder
    let mut builder = FootprintBuilder::new(footprint);

    if hole_mm > 0.0 {
        // Through-hole pads
        let pad_diameter = pad_width_mm.max(pad_height_mm);
        if use_spacing {
            // Convert spacing to pitch
            let pad_along_row = match dir {
                PadRowDirection::Horizontal => pad_width_mm,
                PadRowDirection::Vertical => pad_height_mm,
            };
            let effective_pitch = pitch_mm + pad_along_row;
            builder.add_th_pad_row(
                count,
                effective_pitch,
                pad_diameter,
                hole_mm,
                x_mm,
                y_mm,
                dir,
                start,
                shape,
            );
        } else {
            builder.add_th_pad_row(
                count,
                pitch_mm,
                pad_diameter,
                hole_mm,
                x_mm,
                y_mm,
                dir,
                start,
                shape,
            );
        }
    } else {
        // SMD pads
        if use_spacing {
            builder.add_pad_row_with_spacing(
                count,
                pitch_mm,
                pad_width_mm,
                pad_height_mm,
                x_mm,
                y_mm,
                dir,
                start,
                shape,
            );
        } else {
            builder.add_pad_row(
                count,
                pitch_mm,
                pad_width_mm,
                pad_height_mm,
                x_mm,
                y_mm,
                dir,
                start,
                shape,
            );
        }
    }

    // Extract pads from the built component and add to existing footprint
    let mut det = ();
    let temp = builder.build_deterministic(&mut det);
    for prim in temp.primitives {
        component.add_primitive(prim);
    }

    save_pcblib(path, &lib)?;

    let term = if use_spacing { "spacing" } else { "pitch" };
    println!(
        "Added {} pads to '{}' ({} {} {}, direction: {})",
        count,
        footprint,
        pitch,
        term,
        if hole_mm > 0.0 { "through-hole" } else { "SMD" },
        direction
    );

    Ok(())
}

/// Add dual rows of pads (SOIC, DIP style).
#[allow(clippy::too_many_arguments)]
pub fn cmd_add_dual_row(
    path: &Path,
    footprint: &str,
    pads_per_side: usize,
    pitch: &str,
    row_spacing: &str,
    pad_width: Option<&str>,
    pad_height: Option<&str>,
    pad_diameter: Option<&str>,
    hole: Option<&str>,
    shape_str: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut lib = open_pcblib(path)?;

    let component = lib
        .components
        .iter_mut()
        .find(|c| c.pattern == footprint)
        .ok_or_else(|| format!("Footprint '{}' not found", footprint))?;

    let pitch_mm = parse_unit_value(pitch)?;
    let row_spacing_mm = parse_unit_value(row_spacing)?;
    let shape = parse_pad_shape(shape_str)?;

    let mut builder = FootprintBuilder::new(footprint);

    // Determine if through-hole or SMD based on hole parameter
    if let Some(hole_str) = hole {
        // Through-hole
        let hole_mm = parse_unit_value(hole_str)?;
        let pad_dia_mm = if let Some(d) = pad_diameter {
            parse_unit_value(d)?
        } else if let Some(w) = pad_width {
            parse_unit_value(w)?
        } else {
            return Err("Through-hole pads require --pad-diameter or --pad-width"
                .to_string()
                .into());
        };

        builder.add_dual_row_th(
            pads_per_side,
            pitch_mm,
            row_spacing_mm,
            pad_dia_mm,
            hole_mm,
            shape,
        );
    } else {
        // SMD
        let pad_w = pad_width.ok_or("SMD pads require --pad-width")?;
        let pad_h = pad_height.ok_or("SMD pads require --pad-height")?;
        let pad_width_mm = parse_unit_value(pad_w)?;
        let pad_height_mm = parse_unit_value(pad_h)?;

        builder.add_dual_row_smd(
            pads_per_side,
            pitch_mm,
            row_spacing_mm,
            pad_width_mm,
            pad_height_mm,
            shape,
        );
    }

    // Add pads to existing footprint
    let mut det = ();
    let temp = builder.build_deterministic(&mut det);
    for prim in temp.primitives {
        component.add_primitive(prim);
    }

    save_pcblib(path, &lib)?;

    let total_pads = pads_per_side * 2;
    let pad_type = if hole.is_some() {
        "through-hole"
    } else {
        "SMD"
    };
    println!(
        "Added dual row ({} {} pads, {} per side) to '{}' (pitch: {}, row spacing: {})",
        total_pads, pad_type, pads_per_side, footprint, pitch, row_spacing
    );

    Ok(())
}

/// Add quad arrangement of pads (QFP style).
#[allow(clippy::too_many_arguments)]
pub fn cmd_add_quad_pads(
    path: &Path,
    footprint: &str,
    pads_per_side: usize,
    pitch: &str,
    span: &str,
    pad_width: &str,
    pad_height: &str,
    shape_str: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut lib = open_pcblib(path)?;

    let component = lib
        .components
        .iter_mut()
        .find(|c| c.pattern == footprint)
        .ok_or_else(|| format!("Footprint '{}' not found", footprint))?;

    let pitch_mm = parse_unit_value(pitch)?;
    let span_mm = parse_unit_value(span)?;
    let pad_width_mm = parse_unit_value(pad_width)?;
    let pad_height_mm = parse_unit_value(pad_height)?;
    let shape = parse_pad_shape(shape_str)?;

    let mut builder = FootprintBuilder::new(footprint);
    builder.add_quad_pads_smd(
        pads_per_side,
        pitch_mm,
        span_mm,
        pad_width_mm,
        pad_height_mm,
        shape,
    );

    // Add pads to existing footprint
    let mut det = ();
    let temp = builder.build_deterministic(&mut det);
    for prim in temp.primitives {
        component.add_primitive(prim);
    }

    save_pcblib(path, &lib)?;

    let total_pads = pads_per_side * 4;
    println!(
        "Added quad arrangement ({} SMD pads, {} per side) to '{}' (pitch: {}, span: {})",
        total_pads, pads_per_side, footprint, pitch, span
    );

    Ok(())
}

/// Add a grid of pads (BGA style).
#[allow(clippy::too_many_arguments)]
pub fn cmd_add_pad_grid(
    path: &Path,
    footprint: &str,
    rows: usize,
    cols: usize,
    pitch: &str,
    pad_diameter: &str,
    shape_str: &str,
    skip_center: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut lib = open_pcblib(path)?;

    let component = lib
        .components
        .iter_mut()
        .find(|c| c.pattern == footprint)
        .ok_or_else(|| format!("Footprint '{}' not found", footprint))?;

    let pitch_mm = parse_unit_value(pitch)?;
    let pad_diameter_mm = parse_unit_value(pad_diameter)?;
    let skip_center_mm = parse_unit_value(skip_center)?;
    let shape = parse_pad_shape(shape_str)?;

    let mut builder = FootprintBuilder::new(footprint);
    builder.add_pad_grid(rows, cols, pitch_mm, pad_diameter_mm, shape, skip_center_mm);

    // Add pads to existing footprint
    let mut det = ();
    let temp = builder.build_deterministic(&mut det);
    let pad_count = temp.primitives.len();
    for prim in temp.primitives {
        component.add_primitive(prim);
    }

    save_pcblib(path, &lib)?;

    let max_pads = rows * cols;
    let skipped = max_pads - pad_count;
    if skipped > 0 {
        println!(
            "Added {}x{} grid ({} pads, {} skipped in center) to '{}' (pitch: {})",
            rows, cols, pad_count, skipped, footprint, pitch
        );
    } else {
        println!(
            "Added {}x{} grid ({} pads) to '{}' (pitch: {})",
            rows, cols, pad_count, footprint, pitch
        );
    }

    Ok(())
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

fn common_to_json(common: &PcbPrimitiveCommon) -> PcbCommonJson {
    PcbCommonJson {
        layer: common.layer.0,
        flags: common.flags.bits(),
        unique_id: common.unique_id.clone(),
    }
}

fn common_from_json(json: &PcbCommonJson) -> PcbPrimitiveCommon {
    PcbPrimitiveCommon {
        layer: Layer::new(json.layer),
        flags: PcbFlags::from_bits_truncate(json.flags),
        unique_id: json.unique_id.clone(),
    }
}

fn mask_expansion_to_json(me: &MaskExpansion) -> PcbMaskExpansionJson {
    match me {
        MaskExpansion::Auto => PcbMaskExpansionJson {
            mode: "auto".to_string(),
            value: None,
        },
        MaskExpansion::Manual(c) => PcbMaskExpansionJson {
            mode: "manual".to_string(),
            value: Some(c.to_raw() as i64),
        },
    }
}

fn mask_expansion_from_json(json: &PcbMaskExpansionJson) -> MaskExpansion {
    if json.mode == "auto" {
        MaskExpansion::Auto
    } else {
        MaskExpansion::Manual(Coord::from_raw(json.value.unwrap_or(0) as i32))
    }
}

fn vec_to_coord_array(v: &[[i64; 2]]) -> [CoordPoint; 32] {
    let mut arr = [CoordPoint::default(); 32];
    for (i, pt) in v.iter().enumerate().take(32) {
        arr[i] = CoordPoint::new(Coord::from_raw(pt[0] as i32), Coord::from_raw(pt[1] as i32));
    }
    arr
}

fn vec_to_shape_array(v: &[u8]) -> [PcbPadShape; 32] {
    let mut arr = [PcbPadShape::default(); 32];
    for (i, &s) in v.iter().enumerate().take(32) {
        arr[i] = PcbPadShape::from_byte(s);
    }
    arr
}

fn vec_to_u8_array(v: &[u8]) -> [u8; 32] {
    let mut arr = [0u8; 32];
    for (i, &val) in v.iter().enumerate().take(32) {
        arr[i] = val;
    }
    arr
}

fn vec_to_coord_diameter_array(v: &[i64]) -> [Coord; 32] {
    let mut arr = [Coord::default(); 32];
    for (i, &d) in v.iter().enumerate().take(32) {
        arr[i] = Coord::from_raw(d as i32);
    }
    arr
}

fn primitive_to_json(record: &PcbRecord) -> PcbPrimitiveJson {
    match record {
        PcbRecord::Arc(arc) => PcbPrimitiveJson::Arc {
            common: common_to_json(&arc.common),
            location: [
                arc.location.x.to_raw() as i64,
                arc.location.y.to_raw() as i64,
            ],
            radius: arc.radius.to_raw() as i64,
            start_angle: arc.start_angle,
            end_angle: arc.end_angle,
            width: arc.width.to_raw() as i64,
        },
        PcbRecord::Pad(pad) => PcbPrimitiveJson::Pad {
            common: common_to_json(&pad.common),
            designator: pad.designator.clone(),
            location: [
                pad.location.x.to_raw() as i64,
                pad.location.y.to_raw() as i64,
            ],
            rotation: pad.rotation,
            is_plated: pad.is_plated,
            jumper_id: pad.jumper_id,
            stack_mode: pad.stack_mode.to_byte(),
            hole_size: pad.hole_size.to_raw() as i64,
            hole_shape: pad.hole_shape.to_byte(),
            hole_rotation: pad.hole_rotation,
            hole_slot_length: pad.hole_slot_length.to_raw() as i64,
            paste_mask_expansion: mask_expansion_to_json(&pad.paste_mask_expansion),
            solder_mask_expansion: mask_expansion_to_json(&pad.solder_mask_expansion),
            size_layers: pad
                .size_layers
                .iter()
                .map(|cp| [cp.x.to_raw() as i64, cp.y.to_raw() as i64])
                .collect(),
            shape_layers: pad.shape_layers.iter().map(|s| s.to_byte()).collect(),
            corner_radius_percentage: pad.corner_radius_percentage.to_vec(),
            offsets_from_hole_center: pad
                .offsets_from_hole_center
                .iter()
                .map(|cp| [cp.x.to_raw() as i64, cp.y.to_raw() as i64])
                .collect(),
        },
        PcbRecord::Via(via) => PcbPrimitiveJson::Via {
            common: common_to_json(&via.common),
            location: [
                via.location.x.to_raw() as i64,
                via.location.y.to_raw() as i64,
            ],
            hole_size: via.hole_size.to_raw() as i64,
            from_layer: via.from_layer.0,
            to_layer: via.to_layer.0,
            thermal_relief_air_gap_width: via.thermal_relief_air_gap_width.to_raw() as i64,
            thermal_relief_conductors: via.thermal_relief_conductors,
            thermal_relief_conductors_width: via.thermal_relief_conductors_width.to_raw() as i64,
            solder_mask_expansion: mask_expansion_to_json(&via.solder_mask_expansion),
            diameter_stack_mode: via.diameter_stack_mode.to_byte(),
            diameters: via.diameters.iter().map(|d| d.to_raw() as i64).collect(),
            unknown_trailer: base64::engine::general_purpose::STANDARD.encode(&via.unknown),
        },
        PcbRecord::Track(t) => PcbPrimitiveJson::Track {
            common: common_to_json(&t.common),
            start: [t.start.x.to_raw() as i64, t.start.y.to_raw() as i64],
            end: [t.end.x.to_raw() as i64, t.end.y.to_raw() as i64],
            width: t.width.to_raw() as i64,
            unknown_trailer: base64::engine::general_purpose::STANDARD.encode(&t.unknown),
        },
        PcbRecord::Text(text) => PcbPrimitiveJson::Text {
            common: common_to_json(&text.base.common),
            corner1: [
                text.base.corner1.x.to_raw() as i64,
                text.base.corner1.y.to_raw() as i64,
            ],
            corner2: [
                text.base.corner2.x.to_raw() as i64,
                text.base.corner2.y.to_raw() as i64,
            ],
            rotation: text.base.rotation,
            mirrored: text.mirrored,
            text_kind: text.text_kind.to_byte(),
            stroke_font: text.stroke_font.to_i16(),
            stroke_width: text.stroke_width.to_raw() as i64,
            font_bold: text.font_bold,
            font_italic: text.font_italic,
            font_name: text.font_name.clone(),
            barcode_lr_margin: text.barcode_lr_margin.to_raw() as i64,
            barcode_tb_margin: text.barcode_tb_margin.to_raw() as i64,
            font_inverted: text.font_inverted,
            font_inverted_border: text.font_inverted_border.to_raw() as i64,
            font_inverted_rect: text.font_inverted_rect,
            font_inverted_rect_width: text.font_inverted_rect_width.to_raw() as i64,
            font_inverted_rect_height: text.font_inverted_rect_height.to_raw() as i64,
            font_inverted_rect_justification: text.font_inverted_rect_justification.to_byte(),
            font_inverted_rect_text_offset: text.font_inverted_rect_text_offset.to_raw() as i64,
            text: text.text.clone(),
            wide_strings_index: text.wide_strings_index,
        },
        PcbRecord::Fill(fill) => PcbPrimitiveJson::Fill {
            common: common_to_json(&fill.base.common),
            corner1: [
                fill.base.corner1.x.to_raw() as i64,
                fill.base.corner1.y.to_raw() as i64,
            ],
            corner2: [
                fill.base.corner2.x.to_raw() as i64,
                fill.base.corner2.y.to_raw() as i64,
            ],
            rotation: fill.base.rotation,
        },
        PcbRecord::Region(region) => {
            let param_map: HashMap<String, String> = region
                .parameters
                .iter()
                .map(|(k, v)| (k.to_string(), v.as_str().to_string()))
                .collect();
            PcbPrimitiveJson::Region {
                common: common_to_json(&region.common),
                parameters: param_map,
                outline: region
                    .outline
                    .iter()
                    .map(|pt| [pt.x.to_raw() as i64, pt.y.to_raw() as i64])
                    .collect(),
            }
        }
        PcbRecord::ComponentBody(body) => {
            let mut param_map = HashMap::new();
            if !body.model_id.is_empty() {
                param_map.insert("MODELID".to_string(), body.model_id.clone());
            }
            if !body.name.is_empty() {
                param_map.insert("NAME".to_string(), body.name.clone());
            }
            PcbPrimitiveJson::ComponentBody {
                common: common_to_json(&body.common),
                parameters: param_map,
                outline: body
                    .outline
                    .iter()
                    .map(|pt| [pt.x.to_raw() as i64, pt.y.to_raw() as i64])
                    .collect(),
            }
        }
        PcbRecord::Polygon(_) => PcbPrimitiveJson::Unknown {
            object_id: PcbObjectId::Polygon.to_byte(),
            raw_data: String::new(),
        },
        PcbRecord::Unknown {
            object_id,
            raw_data,
        } => PcbPrimitiveJson::Unknown {
            object_id: object_id.to_byte(),
            raw_data: base64::engine::general_purpose::STANDARD.encode(raw_data),
        },
    }
}

fn primitive_from_json(json: &PcbPrimitiveJson) -> Result<PcbRecord, String> {
    match json {
        PcbPrimitiveJson::Arc {
            common,
            location,
            radius,
            start_angle,
            end_angle,
            width,
        } => Ok(PcbRecord::Arc(PcbArc {
            common: common_from_json(common),
            location: CoordPoint::new(
                Coord::from_raw(location[0] as i32),
                Coord::from_raw(location[1] as i32),
            ),
            radius: Coord::from_raw(*radius as i32),
            start_angle: *start_angle,
            end_angle: *end_angle,
            width: Coord::from_raw(*width as i32),
        })),
        PcbPrimitiveJson::Pad {
            common,
            designator,
            location,
            rotation,
            is_plated,
            jumper_id,
            stack_mode,
            hole_size,
            hole_shape,
            hole_rotation,
            hole_slot_length,
            paste_mask_expansion,
            solder_mask_expansion,
            size_layers,
            shape_layers,
            corner_radius_percentage,
            offsets_from_hole_center,
        } => Ok(PcbRecord::Pad(Box::new(PcbPad {
            common: common_from_json(common),
            designator: designator.clone(),
            location: CoordPoint::new(
                Coord::from_raw(location[0] as i32),
                Coord::from_raw(location[1] as i32),
            ),
            rotation: *rotation,
            is_plated: *is_plated,
            jumper_id: *jumper_id,
            stack_mode: PcbStackMode::from_byte(*stack_mode),
            hole_size: Coord::from_raw(*hole_size as i32),
            hole_shape: PcbPadHoleShape::from_byte(*hole_shape),
            hole_rotation: *hole_rotation,
            hole_slot_length: Coord::from_raw(*hole_slot_length as i32),
            paste_mask_expansion: mask_expansion_from_json(paste_mask_expansion),
            solder_mask_expansion: mask_expansion_from_json(solder_mask_expansion),
            size_layers: vec_to_coord_array(size_layers),
            shape_layers: vec_to_shape_array(shape_layers),
            corner_radius_percentage: vec_to_u8_array(corner_radius_percentage),
            offsets_from_hole_center: vec_to_coord_array(offsets_from_hole_center),
        }))),
        PcbPrimitiveJson::Via {
            common,
            location,
            hole_size,
            from_layer,
            to_layer,
            thermal_relief_air_gap_width,
            thermal_relief_conductors,
            thermal_relief_conductors_width,
            solder_mask_expansion,
            diameter_stack_mode,
            diameters,
            unknown_trailer,
        } => Ok(PcbRecord::Via(PcbVia {
            common: common_from_json(common),
            location: CoordPoint::new(
                Coord::from_raw(location[0] as i32),
                Coord::from_raw(location[1] as i32),
            ),
            hole_size: Coord::from_raw(*hole_size as i32),
            from_layer: Layer::new(*from_layer),
            to_layer: Layer::new(*to_layer),
            thermal_relief_air_gap_width: Coord::from_raw(*thermal_relief_air_gap_width as i32),
            thermal_relief_conductors: *thermal_relief_conductors,
            thermal_relief_conductors_width: Coord::from_raw(
                *thermal_relief_conductors_width as i32,
            ),
            solder_mask_expansion: mask_expansion_from_json(solder_mask_expansion),
            diameter_stack_mode: PcbStackMode::from_byte(*diameter_stack_mode),
            diameters: vec_to_coord_diameter_array(diameters),
            unknown: base64::engine::general_purpose::STANDARD
                .decode(unknown_trailer)
                .unwrap_or_default(),
        })),
        PcbPrimitiveJson::Track {
            common,
            start,
            end,
            width,
            unknown_trailer,
        } => Ok(PcbRecord::Track(PcbTrack {
            common: common_from_json(common),
            start: CoordPoint::new(
                Coord::from_raw(start[0] as i32),
                Coord::from_raw(start[1] as i32),
            ),
            end: CoordPoint::new(
                Coord::from_raw(end[0] as i32),
                Coord::from_raw(end[1] as i32),
            ),
            width: Coord::from_raw(*width as i32),
            unknown: base64::engine::general_purpose::STANDARD
                .decode(unknown_trailer)
                .unwrap_or_default(),
        })),
        PcbPrimitiveJson::Text {
            common,
            corner1,
            corner2,
            rotation,
            mirrored,
            text_kind,
            stroke_font,
            stroke_width,
            font_bold,
            font_italic,
            font_name,
            barcode_lr_margin,
            barcode_tb_margin,
            font_inverted,
            font_inverted_border,
            font_inverted_rect,
            font_inverted_rect_width,
            font_inverted_rect_height,
            font_inverted_rect_justification,
            font_inverted_rect_text_offset,
            text,
            wide_strings_index,
        } => Ok(PcbRecord::Text(PcbText {
            base: PcbRectangularBase {
                common: common_from_json(common),
                corner1: CoordPoint::new(
                    Coord::from_raw(corner1[0] as i32),
                    Coord::from_raw(corner1[1] as i32),
                ),
                corner2: CoordPoint::new(
                    Coord::from_raw(corner2[0] as i32),
                    Coord::from_raw(corner2[1] as i32),
                ),
                rotation: *rotation,
            },
            mirrored: *mirrored,
            text_kind: PcbTextKind::from_byte(*text_kind),
            stroke_font: PcbTextStrokeFont::from_i16(*stroke_font),
            stroke_width: Coord::from_raw(*stroke_width as i32),
            font_bold: *font_bold,
            font_italic: *font_italic,
            font_name: font_name.clone(),
            barcode_lr_margin: Coord::from_raw(*barcode_lr_margin as i32),
            barcode_tb_margin: Coord::from_raw(*barcode_tb_margin as i32),
            font_inverted: *font_inverted,
            font_inverted_border: Coord::from_raw(*font_inverted_border as i32),
            font_inverted_rect: *font_inverted_rect,
            font_inverted_rect_width: Coord::from_raw(*font_inverted_rect_width as i32),
            font_inverted_rect_height: Coord::from_raw(*font_inverted_rect_height as i32),
            font_inverted_rect_justification: PcbTextJustification::from_byte(
                *font_inverted_rect_justification,
            ),
            font_inverted_rect_text_offset: Coord::from_raw(*font_inverted_rect_text_offset as i32),
            text: text.clone(),
            wide_strings_index: *wide_strings_index,
        })),
        PcbPrimitiveJson::Fill {
            common,
            corner1,
            corner2,
            rotation,
        } => Ok(PcbRecord::Fill(PcbFill {
            base: PcbRectangularBase {
                common: common_from_json(common),
                corner1: CoordPoint::new(
                    Coord::from_raw(corner1[0] as i32),
                    Coord::from_raw(corner1[1] as i32),
                ),
                corner2: CoordPoint::new(
                    Coord::from_raw(corner2[0] as i32),
                    Coord::from_raw(corner2[1] as i32),
                ),
                rotation: *rotation,
            },
        })),
        PcbPrimitiveJson::Region {
            common,
            parameters,
            outline,
        } => {
            let mut params = ParameterCollection::new();
            for (k, v) in parameters {
                params.add(k, v);
            }
            let region = PcbRegion {
                common: common_from_json(common),
                parameters: params,
                outline: outline
                    .iter()
                    .map(|pt| {
                        CoordPoint::new(
                            Coord::from_raw(pt[0] as i32),
                            Coord::from_raw(pt[1] as i32),
                        )
                    })
                    .collect(),
            };
            Ok(PcbRecord::Region(region))
        }
        PcbPrimitiveJson::ComponentBody {
            common,
            parameters,
            outline,
        } => {
            let body = PcbComponentBody {
                common: common_from_json(common),
                outline: outline
                    .iter()
                    .map(|pt| {
                        CoordPoint::new(
                            Coord::from_raw(pt[0] as i32),
                            Coord::from_raw(pt[1] as i32),
                        )
                    })
                    .collect(),
                model_id: parameters.get("MODELID").cloned().unwrap_or_default(),
                name: parameters.get("NAME").cloned().unwrap_or_default(),
                ..Default::default()
            };
            Ok(PcbRecord::ComponentBody(Box::new(body)))
        }
        PcbPrimitiveJson::Unknown {
            object_id,
            raw_data,
        } => Ok(PcbRecord::Unknown {
            object_id: PcbObjectId::from_byte(*object_id),
            raw_data: base64::engine::general_purpose::STANDARD
                .decode(raw_data)
                .unwrap_or_default(),
        }),
    }
}

fn footprint_to_full_json(comp: &PcbComponent) -> PcbFootprintFullJson {
    PcbFootprintFullJson {
        name: comp.pattern.clone(),
        description: comp.description.clone(),
        height: comp.height.to_raw() as i64,
        item_guid: comp.item_guid.clone(),
        revision_guid: comp.revision_guid.clone(),
        primitives: comp.primitives.iter().map(primitive_to_json).collect(),
    }
}

fn footprint_from_full_json(json: &PcbFootprintFullJson) -> Result<PcbComponent, String> {
    let mut primitives = Vec::new();
    for (i, prim_json) in json.primitives.iter().enumerate() {
        match primitive_from_json(prim_json) {
            Ok(prim) => primitives.push(prim),
            Err(e) => return Err(format!("Error converting primitive {}: {}", i, e)),
        }
    }

    Ok(PcbComponent {
        pattern: json.name.clone(),
        description: json.description.clone(),
        height: Coord::from_raw(json.height as i32),
        item_guid: json.item_guid.clone(),
        revision_guid: json.revision_guid.clone(),
        primitives,
    })
}

fn cmd_json_full(lib: &PcbLib, path: &Path) -> PcbLibExport {
    let footprints = lib.iter().map(footprint_to_full_json).collect();
    let library_parameters: HashMap<String, String> = lib
        .library_parameters
        .as_ref()
        .map(|p| {
            p.iter()
                .map(|(k, v)| (k.to_string(), v.as_str().to_string()))
                .collect()
        })
        .unwrap_or_default();
    PcbLibExport {
        source: Some(path.display().to_string()),
        unique_id: lib.unique_id.clone(),
        footprint_count: lib.components.len(),
        library_parameters,
        file_header_version: lib.file_header_version.clone(),
        file_header_field1: lib.file_header_field1.clone(),
        file_header_field2: lib.file_header_field2.clone(),
        footprints,
    }
}

fn cmd_add_json_full_export(
    path: &Path,
    export: &PcbLibExport,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut lib = open_or_create_pcblib(path)?;

    lib.unique_id = export.unique_id.clone();
    lib.file_header_version = export.file_header_version.clone();
    lib.file_header_field1 = export.file_header_field1.clone();
    lib.file_header_field2 = export.file_header_field2.clone();

    if !export.library_parameters.is_empty() {
        let mut params = ParameterCollection::new();
        for (k, v) in &export.library_parameters {
            params.add(k, v);
        }
        lib.library_parameters = Some(params);
    }

    lib.components.clear();

    let mut added = 0;
    for fp_json in &export.footprints {
        let component = footprint_from_full_json(fp_json)
            .map_err(|e| format!("Error converting footprint '{}': {}", fp_json.name, e))?;
        lib.components.push(component);
        added += 1;
    }

    save_pcblib(path, &lib)?;
    println!("Imported {} footprints to {}", added, path.display());
    Ok(())
}
