// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! PCB footprint library operations (v2).
//!
//! Provides high-level operations for exploring and manipulating Altium PCB
//! footprint library (.PcbLib) files using the v2 backing-store architecture.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::v2::backing_store::{
    BinaryOrigin, FootprintGroup, PcbPrimitiveRef, RecordNode, RecordOrigin,
};
use crate::v2::coord::{AltiumCoord, PcbCoord};
use crate::v2::documents::pcblib::PcbLib;
use crate::v2::ops::output::*;
use crate::v2::records::pcb_pad::PcbPadRecord;

// ═══════════════════════════════════════════════════════════════════════════
// CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════

/// PCB primitive type IDs (from the binary framing byte).
const TYPE_ARC: u8 = 1;
const TYPE_PAD: u8 = 2;
#[allow(dead_code)]
const TYPE_VIA: u8 = 3;
const TYPE_TRACK: u8 = 4;
#[allow(dead_code)]
const TYPE_TEXT: u8 = 5;
#[allow(dead_code)]
const TYPE_FILL: u8 = 6;
#[allow(dead_code)]
const TYPE_REGION: u8 = 11;
#[allow(dead_code)]
const TYPE_COMPONENT_BODY: u8 = 12;

// ═══════════════════════════════════════════════════════════════════════════
// PAD DATA EXTRACTION
// ═══════════════════════════════════════════════════════════════════════════

/// Pad data extracted using the typed `PcbPadRecord` API plus raw access
/// for fields not yet in the typed API (designator, layer).
struct PadData {
    designator: String,
    layer: u8,
    record: PcbPadRecord,
}

impl PadData {
    /// Extract pad data from a binary primitive node.
    ///
    /// Uses `PcbPadRecord::from_binary()` for geometric data (position, size,
    /// shape, hole, rotation, plating).
    ///
    /// Designator and layer are extracted from raw binary since they are
    /// not yet covered by the typed `PcbPadRecord` API:
    /// - Designator: subrecord 1 (variable-length string before core data)
    /// - Layer: byte 0 of subrecord 5 (PcbCommonHeader)
    fn from_node(node: &RecordNode) -> Option<Self> {
        let binary = node.origin.as_binary()?;
        let data = &binary.raw_block;

        // Extract designator from subrecord 1
        if data.len() < 4 {
            return None;
        }
        let name_len = u32::from_le_bytes(data[0..4].try_into().ok()?) as usize;
        if 4 + name_len > data.len() {
            return None;
        }
        let designator = String::from_utf8_lossy(&data[4..4 + name_len])
            .trim_end_matches('\0')
            .to_string();

        // Walk through 4 string subrecords to find subrecord 5
        let mut offset = 0usize;
        for _ in 0..4 {
            if offset + 4 > data.len() {
                return None;
            }
            let sub_len = u32::from_le_bytes(
                data[offset..offset + 4].try_into().ok()?,
            ) as usize;
            offset += 4 + sub_len;
        }

        // Extract layer from byte 0 of subrecord 5 core data
        if offset + 4 > data.len() {
            return None;
        }
        let core_start = offset + 4;
        let layer = data.get(core_start).copied().unwrap_or(0);

        // Use typed PcbPadRecord for all geometric data
        let record = PcbPadRecord::from_binary(data).ok()?;

        Some(PadData {
            designator,
            layer,
            record,
        })
    }

    /// Returns true if this is an SMD pad (no through-hole).
    fn is_smd(&self) -> bool {
        self.record.hole_size().to_raw() == 0
    }

    /// Returns a human-readable shape name.
    fn shape_name(&self) -> &'static str {
        shape_name(self.record.top_shape())
    }

    /// Returns the layer name for display.
    fn layer_name(&self) -> &'static str {
        layer_name(self.layer)
    }

    /// Returns the size formatted as a string in mm.
    fn size_string(&self) -> String {
        let x_mm = self.record.top_size_x().to_mm();
        let y_mm = self.record.top_size_y().to_mm();
        if (x_mm - y_mm).abs() < 0.001 {
            format!("{:.3}mm", x_mm)
        } else {
            format!("{:.3}mm x {:.3}mm", x_mm, y_mm)
        }
    }

    /// Returns the hole size formatted as a string in mm, or None for SMD.
    fn hole_string(&self) -> Option<String> {
        if self.record.hole_size().to_raw() == 0 {
            None
        } else {
            Some(format!("{:.3}mm", self.record.hole_size().to_mm()))
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Sorts strings with embedded numbers naturally (e.g., "A2" < "A10").
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
                    let a_num: String = a_chars
                        .by_ref()
                        .take_while(|c| c.is_ascii_digit())
                        .collect();
                    let b_num: String = b_chars
                        .by_ref()
                        .take_while(|c| c.is_ascii_digit())
                        .collect();
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

/// Opens and parses a PcbLib file from the given path.
fn open_pcblib(path: &Path) -> Result<PcbLib, Box<dyn std::error::Error>> {
    Ok(PcbLib::open_file(path).map_err(|e| e.to_string())?)
}

/// Map a PCB primitive type byte to a human-readable name.
fn primitive_type_name(key: u8) -> &'static str {
    match key {
        TYPE_ARC => "Arc",
        TYPE_PAD => "Pad",
        3 => "Via",
        TYPE_TRACK => "Track",
        5 => "Text",
        6 => "Fill",
        7 => "Connection",
        8 => "Net",
        9 => "Component",
        10 => "Polygon",
        11 => "Region",
        12 => "ComponentBody",
        13 => "Dimension",
        14 => "Coordinate",
        _ => "Unknown",
    }
}

/// Map a TShape value to a human-readable shape name.
fn shape_name(shape: u8) -> &'static str {
    match shape {
        0 => "NoShape",
        1 => "Round",
        2 => "Rectangular",
        3 => "Octagonal",
        4 => "Circle",
        5 => "Arc",
        6 => "Terminator",
        7 => "RoundedRect",
        8 => "RotatedRect",
        9 => "RoundedRectangular",
        _ => "Unknown",
    }
}

/// Map a layer byte to a display name.
fn layer_name(layer: u8) -> &'static str {
    match layer {
        0 => "NoLayer",
        1 => "TopLayer",
        32 => "BottomLayer",
        33 => "TopOverlay",
        34 => "BottomOverlay",
        35 => "TopPaste",
        36 => "BottomPaste",
        37 => "TopSolder",
        38 => "BottomSolder",
        74 => "MultiLayer",
        _ => {
            if layer >= 2 && layer <= 31 {
                "MidLayer"
            } else if layer >= 39 && layer <= 54 {
                "InternalPlane"
            } else if layer >= 57 && layer <= 72 {
                "Mechanical"
            } else {
                "Other"
            }
        }
    }
}

/// Count pads in a footprint group.
fn count_pads(group: &FootprintGroup) -> usize {
    group
        .primitives
        .iter()
        .filter(|p| p.key == TYPE_PAD)
        .count()
}

/// Extract all pad data from a footprint group using the typed `PcbPadRecord` API.
fn extract_all_pads(group: &FootprintGroup) -> Vec<PadData> {
    group
        .primitives
        .iter()
        .filter(|p| p.key == TYPE_PAD)
        .filter_map(|p| PadData::from_node(p))
        .collect()
}

/// Count primitives by type in a footprint group.
fn count_primitives(group: &FootprintGroup) -> HashMap<&'static str, usize> {
    let mut counts: HashMap<&'static str, usize> = HashMap::new();
    for prim in &group.primitives {
        let name = primitive_type_name(prim.key);
        *counts.entry(name).or_insert(0) += 1;
    }
    counts
}

// ═══════════════════════════════════════════════════════════════════════════
// FOOTPRINT METADATA HELPERS
//
// NOTE: Raw param access is intentional for footprint metadata (PATTERN,
// DESCRIPTION, HEIGHT, UNIQUEID). The metadata node is a generic RecordNode
// (key=0) with no corresponding typed record in the v2 API.
// ═══════════════════════════════════════════════════════════════════════════

/// Extract the PATTERN parameter from footprint metadata.
fn get_pattern_name(group: &FootprintGroup) -> String {
    if let Some(param) = group.metadata.origin.as_param() {
        param
            .params
            .get("PATTERN")
            .map(|v| v.as_str().to_string())
            .unwrap_or_default()
    } else {
        String::new()
    }
}

/// Extract the DESCRIPTION parameter from footprint metadata.
fn get_description(group: &FootprintGroup) -> String {
    if let Some(param) = group.metadata.origin.as_param() {
        param
            .params
            .get("DESCRIPTION")
            .map(|v| v.as_str().to_string())
            .unwrap_or_default()
    } else {
        String::new()
    }
}

/// Extract the HEIGHT parameter from footprint metadata.
fn get_height(group: &FootprintGroup) -> String {
    if let Some(param) = group.metadata.origin.as_param() {
        param
            .params
            .get("HEIGHT")
            .map(|v| {
                let raw = v.as_int_or(0);
                if raw != 0 {
                    format!("{:.3}mm", PcbCoord::from_raw(raw).to_mm())
                } else {
                    String::new()
                }
            })
            .unwrap_or_default()
    } else {
        String::new()
    }
}

/// Extract a unique ID from the library (from first footprint metadata or empty).
fn get_library_unique_id(lib: &PcbLib) -> String {
    // PcbLib files may store a UNIQUEID in individual footprint metadata.
    // We check footprints or return an empty string.
    for group in &lib.footprints {
        if let Some(param) = group.metadata.origin.as_param() {
            if let Some(v) = param.params.get("UNIQUEID") {
                let s = v.as_str().to_string();
                if !s.is_empty() {
                    return s;
                }
            }
        }
    }
    String::new()
}

/// Categorize a footprint by its name and description.
fn categorize_footprint(name: &str, description: &str) -> &'static str {
    let name_lower = name.to_lowercase();
    let desc_lower = description.to_lowercase();

    // Package types
    if name_lower.contains("bga") || desc_lower.contains("bga") {
        return "BGA";
    }
    if name_lower.contains("qfp")
        || name_lower.contains("tqfp")
        || name_lower.contains("lqfp")
    {
        return "QFP";
    }
    if name_lower.contains("qfn") || name_lower.contains("dfn") {
        return "QFN/DFN";
    }
    if name_lower.contains("soic")
        || name_lower.contains("sop")
        || name_lower.contains("ssop")
        || name_lower.contains("tssop")
        || name_lower.contains("msop")
    {
        return "SOIC/SOP";
    }
    if name_lower.contains("sot") {
        return "SOT";
    }
    if name_lower.contains("dip") || name_lower.contains("pdip") {
        return "DIP";
    }

    // Passive SMD sizes
    if name_lower.starts_with("0201")
        || name_lower.starts_with("0402")
        || name_lower.starts_with("0603")
        || name_lower.starts_with("0805")
        || name_lower.starts_with("1206")
        || name_lower.starts_with("1210")
        || name_lower.starts_with("2010")
        || name_lower.starts_with("2512")
    {
        return "Chip/SMD";
    }

    // Connectors
    if name_lower.contains("header")
        || name_lower.contains("connector")
        || name_lower.contains("socket")
        || name_lower.contains("terminal")
        || name_lower.contains("usb")
        || name_lower.contains("rj45")
    {
        return "Connector";
    }

    // Through-hole
    if name_lower.contains("axial")
        || name_lower.contains("radial")
        || name_lower.contains("through")
        || name_lower.contains("th_")
    {
        return "Through-Hole";
    }

    // Electrolytic / Power
    if name_lower.contains("cap_elec") || name_lower.contains("electrolytic") {
        return "Electrolytic";
    }

    // Inductor
    if name_lower.contains("inductor")
        || name_lower.contains("choke")
        || name_lower.contains("ferrite")
    {
        return "Inductor";
    }

    // Crystal / Oscillator
    if name_lower.contains("xtal")
        || name_lower.contains("crystal")
        || name_lower.contains("oscillator")
    {
        return "Crystal/Oscillator";
    }

    // LED
    if name_lower.contains("led") {
        return "LED";
    }

    // Test point
    if name_lower.contains("test") || name_lower.contains("tp_") {
        return "Test Point";
    }

    // Mounting hole
    if name_lower.contains("mount") || name_lower.contains("standoff") {
        return "Mounting Hole";
    }

    "Other"
}

/// Compute bounding box for a footprint's pads.
fn compute_bounding_box(pads: &[PadData]) -> BoundingBox {
    if pads.is_empty() {
        return BoundingBox {
            width: "0mm".to_string(),
            height: "0mm".to_string(),
        };
    }

    let mut min_x = i32::MAX;
    let mut max_x = i32::MIN;
    let mut min_y = i32::MAX;
    let mut max_y = i32::MIN;

    for pad in pads {
        let pos_x = pad.record.position_x().to_raw();
        let pos_y = pad.record.position_y().to_raw();
        let half_x = pad.record.top_size_x().to_raw() / 2;
        let half_y = pad.record.top_size_y().to_raw() / 2;
        min_x = min_x.min(pos_x - half_x);
        max_x = max_x.max(pos_x + half_x);
        min_y = min_y.min(pos_y - half_y);
        max_y = max_y.max(pos_y + half_y);
    }

    let width_mm = PcbCoord::from_raw(max_x - min_x).to_mm();
    let height_mm = PcbCoord::from_raw(max_y - min_y).to_mm();

    BoundingBox {
        width: format!("{:.3}mm", width_mm),
        height: format!("{:.3}mm", height_mm),
    }
}

/// Parse a dimension string with unit suffix (e.g., "2.54mm", "100mil") into
/// internal PCB coordinate units.
fn parse_dimension(s: &str) -> Result<i32, Box<dyn std::error::Error>> {
    let s = s.trim();
    if let Some(val) = s.strip_suffix("mm") {
        let mm: f64 = val
            .parse()
            .map_err(|_| format!("Invalid dimension: {}", s))?;
        Ok(PcbCoord::from_mm(mm).to_raw())
    } else if let Some(val) = s.strip_suffix("mil") {
        let mils: f64 = val
            .parse()
            .map_err(|_| format!("Invalid dimension: {}", s))?;
        Ok(PcbCoord::from_mils(mils).to_raw())
    } else if let Some(val) = s.strip_suffix("in") {
        let inches: f64 = val
            .parse()
            .map_err(|_| format!("Invalid dimension: {}", s))?;
        Ok(PcbCoord::from_mils(inches * 1000.0).to_raw())
    } else {
        // Default to mm
        let mm: f64 = s
            .parse()
            .map_err(|_| format!("Invalid dimension: {}", s))?;
        Ok(PcbCoord::from_mm(mm).to_raw())
    }
}

/// Convert mm to internal PCB coordinate units.
fn mm_to_raw(mm: f64) -> i32 {
    PcbCoord::from_mm(mm).to_raw()
}

/// Parse shape name to TShape byte value.
fn parse_shape(s: &str) -> u8 {
    match s.to_lowercase().as_str() {
        "round" | "circular" => 1,
        "rectangular" | "rect" | "rectangle" => 2,
        "octagonal" | "oct" => 3,
        "rounded_rect" | "roundedrect" | "rounded_rectangular" => 9,
        _ => 2, // default to rectangular
    }
}

/// Find a footprint group by name (case-insensitive). Returns (index, name).
fn find_footprint<'a>(
    lib: &'a PcbLib,
    name: &str,
) -> Result<(usize, &'a str), Box<dyn std::error::Error>> {
    let name_lower = name.to_lowercase();
    for (i, fp_name) in lib.footprint_names.iter().enumerate() {
        if fp_name.to_lowercase() == name_lower {
            return Ok((i, fp_name.as_str()));
        }
    }
    Err(format!("Footprint '{}' not found in library", name).into())
}

// ═══════════════════════════════════════════════════════════════════════════
// PAD BINARY CONSTRUCTION
// ═══════════════════════════════════════════════════════════════════════════

/// Build a binary pad primitive block.
///
/// The pad format is 6 subrecords:
/// 1. Designator string (u32 length + bytes)
/// 2-4. Empty strings (u32 length = 0)
/// 5. Core data (u32 length + 172 bytes min)
/// 6. Stack data (u32 length + 596 bytes min)
fn build_pad_binary(
    designator: &str,
    x: i32,
    y: i32,
    size_x: i32,
    size_y: i32,
    shape: u8,
    hole_size: i32,
    layer: u8,
) -> Vec<u8> {
    let mut data = Vec::new();

    // Subrecord 1: designator string
    let name_bytes = designator.as_bytes();
    data.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
    data.extend_from_slice(name_bytes);

    // Subrecords 2-4: empty strings
    for _ in 0..3 {
        data.extend_from_slice(&0u32.to_le_bytes());
    }

    // Subrecord 5: core data (172 bytes)
    let core_len: usize = 172;
    data.extend_from_slice(&(core_len as u32).to_le_bytes());
    let core_start = data.len();
    data.resize(core_start + core_len, 0);

    // Layer (byte 0 of core)
    data[core_start] = layer;

    // Position X (offset 13 from core start)
    data[core_start + 13..core_start + 17].copy_from_slice(&x.to_le_bytes());
    // Position Y (offset 17)
    data[core_start + 17..core_start + 21].copy_from_slice(&y.to_le_bytes());
    // Top size X (offset 21)
    data[core_start + 21..core_start + 25].copy_from_slice(&size_x.to_le_bytes());
    // Top size Y (offset 25)
    data[core_start + 25..core_start + 29].copy_from_slice(&size_y.to_le_bytes());
    // Mid size X (offset 29) -- same as top for simple mode
    data[core_start + 29..core_start + 33].copy_from_slice(&size_x.to_le_bytes());
    // Mid size Y (offset 33)
    data[core_start + 33..core_start + 37].copy_from_slice(&size_y.to_le_bytes());
    // Bot size X (offset 37) -- same as top for simple mode
    data[core_start + 37..core_start + 41].copy_from_slice(&size_x.to_le_bytes());
    // Bot size Y (offset 41)
    data[core_start + 41..core_start + 45].copy_from_slice(&size_y.to_le_bytes());
    // Hole size (offset 45)
    data[core_start + 45..core_start + 49].copy_from_slice(&hole_size.to_le_bytes());
    // Top shape (offset 49)
    data[core_start + 49] = shape;
    // Mid shape (offset 50)
    data[core_start + 50] = shape;
    // Bot shape (offset 51)
    data[core_start + 51] = shape;
    // Rotation (offset 52, f64 = 0.0)
    data[core_start + 52..core_start + 60].copy_from_slice(&0.0f64.to_le_bytes());
    // Is plated (offset 60)
    data[core_start + 60] = if hole_size > 0 { 1 } else { 0 };

    // Subrecord 6: stack data (596 bytes, zeros for simple mode)
    let stack_len: usize = 596;
    data.extend_from_slice(&(stack_len as u32).to_le_bytes());
    data.resize(data.len() + stack_len, 0);

    data
}

/// Build a binary track primitive block (for silkscreen lines).
fn build_track_binary(
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    width: i32,
    layer: u8,
) -> Vec<u8> {
    // Track record: PcbCommonHeader (13 bytes) + x1(4) + y1(4) + x2(4) + y2(4) + width(4)
    let size = 13 + 4 + 4 + 4 + 4 + 4;
    let mut data = vec![0u8; size];

    // Layer
    data[0] = layer;

    // Start X (offset 13)
    data[13..17].copy_from_slice(&x1.to_le_bytes());
    // Start Y (offset 17)
    data[17..21].copy_from_slice(&y1.to_le_bytes());
    // End X (offset 21)
    data[21..25].copy_from_slice(&x2.to_le_bytes());
    // End Y (offset 25)
    data[25..29].copy_from_slice(&y2.to_le_bytes());
    // Width (offset 29)
    data[29..33].copy_from_slice(&width.to_le_bytes());

    data
}

/// Build a binary arc primitive block (for silkscreen arcs).
fn build_arc_binary(
    cx: i32,
    cy: i32,
    radius: i32,
    start_angle: f64,
    end_angle: f64,
    width: i32,
    layer: u8,
) -> Vec<u8> {
    // Arc record: PcbCommonHeader (13) + cx(4) + cy(4) + radius(4) +
    //             start_angle(8) + end_angle(8) + width(4)
    let size = 13 + 4 + 4 + 4 + 8 + 8 + 4;
    let mut data = vec![0u8; size];

    // Layer
    data[0] = layer;

    // Center X (offset 13)
    data[13..17].copy_from_slice(&cx.to_le_bytes());
    // Center Y (offset 17)
    data[17..21].copy_from_slice(&cy.to_le_bytes());
    // Radius (offset 21)
    data[21..25].copy_from_slice(&radius.to_le_bytes());
    // Start angle (offset 25)
    data[25..33].copy_from_slice(&start_angle.to_le_bytes());
    // End angle (offset 33)
    data[33..41].copy_from_slice(&end_angle.to_le_bytes());
    // Width (offset 41)
    data[41..45].copy_from_slice(&width.to_le_bytes());

    data
}

// ═══════════════════════════════════════════════════════════════════════════
// BROWSE COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// Returns library overview with statistics and footprint category breakdown.
pub fn cmd_overview(path: &Path) -> Result<PcbLibOverview, Box<dyn std::error::Error>> {
    let lib = open_pcblib(path)?;
    let unique_id = get_library_unique_id(&lib);

    // ─────────────────────────────────────────────────────────────────────────
    // 1. FOOTPRINTS BY CATEGORY
    // ─────────────────────────────────────────────────────────────────────────
    let mut categories: HashMap<&'static str, Vec<FootprintSummaryExt>> = HashMap::new();

    for (i, group) in lib.footprints.iter().enumerate() {
        let name = lib.footprint_names.get(i).cloned().unwrap_or_default();
        let description = get_description(group);
        let pad_count = count_pads(group);
        let category = categorize_footprint(&name, &description);

        categories
            .entry(category)
            .or_default()
            .push(FootprintSummaryExt {
                name,
                description,
                pad_count,
            });
    }

    // Sort categories by typical importance
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
    // Add remaining uncategorized
    for (category, mut fps) in categories {
        if !fps.is_empty() {
            fps.sort_by(|a, b| alphanumeric_sort(&a.name, &b.name));
            footprints_by_category.push((category.to_string(), fps));
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 2. PAD STATISTICS
    // ─────────────────────────────────────────────────────────────────────────
    let mut total_pads = 0;
    let mut smd_pads = 0;
    let mut th_pads = 0;
    let mut shape_counts: HashMap<&'static str, usize> = HashMap::new();
    let mut hole_counts: HashMap<String, usize> = HashMap::new();

    for group in &lib.footprints {
        let pads = extract_all_pads(group);
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

    // ─────────────────────────────────────────────────────────────────────────
    // 3. LARGEST FOOTPRINTS (by pad count)
    // ─────────────────────────────────────────────────────────────────────────
    let mut by_pads: Vec<(usize, usize)> = lib
        .footprints
        .iter()
        .enumerate()
        .map(|(i, g)| (i, count_pads(g)))
        .collect();
    by_pads.sort_by_key(|(_, pads)| std::cmp::Reverse(*pads));

    let largest_footprints = by_pads
        .iter()
        .take(10)
        .map(|(i, pads)| {
            let name = lib.footprint_names.get(*i).cloned().unwrap_or_default();
            let description = get_description(&lib.footprints[*i]);
            FootprintSummaryExt {
                name,
                description,
                pad_count: *pads,
            }
        })
        .collect();

    Ok(PcbLibOverview {
        path: path.display().to_string(),
        total_footprints: lib.footprints.len(),
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

    let mut footprints: Vec<FootprintSummaryExt> = lib
        .footprints
        .iter()
        .enumerate()
        .map(|(i, group)| {
            let name = lib.footprint_names.get(i).cloned().unwrap_or_default();
            let description = get_description(group);
            let pad_count = count_pads(group);
            FootprintSummaryExt {
                name,
                description,
                pad_count,
            }
        })
        .collect();

    footprints.sort_by(|a, b| alphanumeric_sort(&a.name, &b.name));

    Ok(PcbLibFootprintList {
        path: path.display().to_string(),
        total_footprints: lib.footprints.len(),
        footprints,
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

    let mut matches: Vec<FootprintSummaryExt> = lib
        .footprints
        .iter()
        .enumerate()
        .filter(|(i, group)| {
            let name = lib
                .footprint_names
                .get(*i)
                .map(|s| s.to_lowercase())
                .unwrap_or_default();
            let desc = get_description(group).to_lowercase();

            if has_wildcard {
                let pattern = query_lower.replace('*', "");
                name.contains(&pattern) || desc.contains(&pattern)
            } else {
                name.contains(&query_lower) || desc.contains(&query_lower)
            }
        })
        .map(|(i, group)| {
            let name = lib.footprint_names.get(i).cloned().unwrap_or_default();
            let description = get_description(group);
            let pad_count = count_pads(group);
            FootprintSummaryExt {
                name,
                description,
                pad_count,
            }
        })
        .collect();

    // Sort by relevance (exact name match first, then by name)
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
    let unique_id = get_library_unique_id(&lib);

    let mut primitive_counts: HashMap<&'static str, usize> = HashMap::new();
    let mut total_primitives = 0;

    for group in &lib.footprints {
        let counts = count_primitives(group);
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
        footprint_count: lib.footprints.len(),
        unique_id,
        total_primitives,
        primitive_types,
    })
}

/// Returns detailed information about a single footprint.
pub fn cmd_footprint(
    path: &Path,
    name: &str,
    show_primitives: bool,
) -> Result<PcbLibFootprintDetail, Box<dyn std::error::Error>> {
    let lib = open_pcblib(path)?;
    let (idx, fp_name) = find_footprint(&lib, name)?;
    let group = &lib.footprints[idx];

    let pattern = get_pattern_name(group);
    let description = get_description(group);
    let height = get_height(group);

    let pads = extract_all_pads(group);
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
        let counts = count_primitives(group);
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
            fp_name.to_string()
        } else {
            pattern
        },
        description,
        height,
        pad_count,
        total_primitives: group.primitives.len(),
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

    for (i, group) in lib.footprints.iter().enumerate() {
        let fp_name = lib.footprint_names.get(i).cloned().unwrap_or_default();

        if let Some(ref filter) = filter_lower {
            if fp_name.to_lowercase() != *filter {
                continue;
            }
        }

        let pads = extract_all_pads(group);
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

    // Sort by footprint name, then by designator
    all_pads.sort_by(|a, b| {
        let cmp = alphanumeric_sort(&a.footprint_name, &b.footprint_name);
        if cmp == std::cmp::Ordering::Equal {
            alphanumeric_sort(&a.designator, &b.designator)
        } else {
            cmp
        }
    });

    // Group by shape
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
    let (idx, fp_name) = find_footprint(&lib, footprint)?;
    let group = &lib.footprints[idx];

    let mut primitives: Vec<serde_json::Value> = Vec::new();

    for (i, prim) in group.primitives.iter().enumerate() {
        let type_name = primitive_type_name(prim.key);
        let data_size = match &prim.origin {
            RecordOrigin::Binary(b) => b.raw_block.len(),
            RecordOrigin::Param(p) => p.raw_record_text.len(),
        };

        let mut entry = serde_json::json!({
            "index": i,
            "type": type_name,
            "type_id": prim.key,
            "data_size": data_size,
        });

        // Add extra info for pads
        if prim.key == TYPE_PAD {
            if let Some(pad) = PadData::from_node(prim) {
                entry["designator"] = serde_json::json!(pad.designator);
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
    }

    Ok(serde_json::json!({
        "footprint": fp_name,
        "total_primitives": group.primitives.len(),
        "primitives": primitives,
    }))
}

/// Analyze hole sizes across the library.
pub fn cmd_holes(
    path: &Path,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let lib = open_pcblib(path)?;

    let mut hole_info: HashMap<String, Vec<String>> = HashMap::new();

    for (i, group) in lib.footprints.iter().enumerate() {
        let fp_name = lib.footprint_names.get(i).cloned().unwrap_or_default();
        let pads = extract_all_pads(group);

        for pad in &pads {
            if pad.record.hole_size().to_raw() > 0 {
                let hole_str =
                    format!("{:.3}mm", pad.record.hole_size().to_mm());
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

    // Sort by count descending
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

/// Measure footprint dimensions and clearances.
pub fn cmd_measure(
    path: &Path,
    footprint: &str,
    _measure_type: &str,
    _pad1: Option<&str>,
    _pad2: Option<&str>,
    _axis: Option<&str>,
    as_json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let lib = open_pcblib(path)?;
    let (idx, fp_name) = find_footprint(&lib, footprint)?;
    let group = &lib.footprints[idx];

    let pads = extract_all_pads(group);
    let bb = compute_bounding_box(&pads);

    // Calculate pad pitch (min center-to-center distance)
    let mut min_pitch = f64::MAX;
    for i in 0..pads.len() {
        for j in (i + 1)..pads.len() {
            let dx = (pads[i].record.position_x().to_raw()
                - pads[j].record.position_x().to_raw()) as f64;
            let dy = (pads[i].record.position_y().to_raw()
                - pads[j].record.position_y().to_raw()) as f64;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist > 0.0 && dist < min_pitch {
                min_pitch = dist;
            }
        }
    }

    let pitch_mm = if min_pitch < f64::MAX {
        PcbCoord::from_raw(min_pitch as i32).to_mm()
    } else {
        0.0
    };

    if as_json {
        let result = serde_json::json!({
            "footprint": fp_name,
            "pad_count": pads.len(),
            "bounding_box": {
                "width": bb.width,
                "height": bb.height,
            },
            "min_pitch_mm": format!("{:.3}", pitch_mm),
            "smd_pads": pads.iter().filter(|p| p.is_smd()).count(),
            "th_pads": pads.iter().filter(|p| !p.is_smd()).count(),
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("Footprint: {}", fp_name);
        println!("Pad count: {}", pads.len());
        println!("Bounding box: {} x {}", bb.width, bb.height);
        if pitch_mm > 0.0 {
            println!("Min pad pitch: {:.3}mm", pitch_mm);
        }
        println!(
            "SMD: {}, Through-hole: {}",
            pads.iter().filter(|p| p.is_smd()).count(),
            pads.iter().filter(|p| !p.is_smd()).count()
        );
    }

    Ok(())
}

/// Serializes the library to JSON for LLM processing or external analysis.
pub fn cmd_json(
    path: &Path,
    full: bool,
) -> Result<PcbLibJson, Box<dyn std::error::Error>> {
    let lib = open_pcblib(path)?;
    let unique_id = get_library_unique_id(&lib);

    let footprints: Vec<FootprintJsonData> = lib
        .footprints
        .iter()
        .enumerate()
        .map(|(i, group)| {
            let name = lib.footprint_names.get(i).cloned().unwrap_or_default();
            let description = get_description(group);
            let pad_count = count_pads(group);
            let primitive_count = group.primitives.len();

            let pads = if full {
                let pad_list = extract_all_pads(group);
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

            FootprintJsonData {
                name,
                description,
                pad_count,
                primitive_count,
                pads,
            }
        })
        .collect();

    Ok(PcbLibJson {
        file: path.display().to_string(),
        footprint_count: lib.footprints.len(),
        unique_id,
        footprints,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// RENDERING COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// Render footprint as ASCII art.
pub fn cmd_render_ascii(
    path: &Path,
    footprint: &str,
    width: u32,
    height: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let lib = open_pcblib(path)?;
    let (idx, fp_name) = find_footprint(&lib, footprint)?;
    let group = &lib.footprints[idx];

    let pads = extract_all_pads(group);
    if pads.is_empty() {
        println!("Footprint '{}' has no pads to render.", fp_name);
        return Ok(());
    }

    // Compute bounds
    let mut min_x = i32::MAX;
    let mut max_x = i32::MIN;
    let mut min_y = i32::MAX;
    let mut max_y = i32::MIN;

    for pad in &pads {
        let pos_x = pad.record.position_x().to_raw();
        let pos_y = pad.record.position_y().to_raw();
        let half_x = pad.record.top_size_x().to_raw() / 2;
        let half_y = pad.record.top_size_y().to_raw() / 2;
        min_x = min_x.min(pos_x - half_x);
        max_x = max_x.max(pos_x + half_x);
        min_y = min_y.min(pos_y - half_y);
        max_y = max_y.max(pos_y + half_y);
    }

    // Add margin
    let margin_x = (max_x - min_x) / 10;
    let margin_y = (max_y - min_y) / 10;
    min_x -= margin_x;
    max_x += margin_x;
    min_y -= margin_y;
    max_y += margin_y;

    let range_x = (max_x - min_x) as f64;
    let range_y = (max_y - min_y) as f64;

    if range_x <= 0.0 || range_y <= 0.0 {
        println!("Footprint '{}' has zero extent.", fp_name);
        return Ok(());
    }

    let w = width as usize;
    let h = height as usize;

    // Build a character grid
    let mut grid = vec![vec![' '; w]; h];

    // Draw pads
    for pad in &pads {
        let pos_x = pad.record.position_x().to_raw();
        let pos_y = pad.record.position_y().to_raw();
        let size_x = pad.record.top_size_x().to_raw();
        let size_y = pad.record.top_size_y().to_raw();

        let cx =
            ((pos_x - min_x) as f64 / range_x * (w - 1) as f64) as usize;
        let cy = h
            - 1
            - ((pos_y - min_y) as f64 / range_y * (h - 1) as f64) as usize;

        let half_w = (size_x as f64 / range_x * (w - 1) as f64 / 2.0)
            .max(0.5) as usize;
        let half_h = (size_y as f64 / range_y * (h - 1) as f64 / 2.0)
            .max(0.5) as usize;

        let x_start = cx.saturating_sub(half_w);
        let x_end = (cx + half_w).min(w - 1);
        let y_start = cy.saturating_sub(half_h);
        let y_end = (cy + half_h).min(h - 1);

        let ch = if pad.is_smd() { '#' } else { 'O' };
        for gy in y_start..=y_end {
            for gx in x_start..=x_end {
                grid[gy][gx] = ch;
            }
        }

        // Try to place designator
        if !pad.designator.is_empty() && cx < w {
            let label_chars: Vec<char> = pad.designator.chars().collect();
            let label_start = cx.saturating_sub(label_chars.len() / 2);
            for (ci, &lc) in label_chars.iter().enumerate() {
                let lx = label_start + ci;
                if lx < w && cy < h {
                    grid[cy][lx] = lc;
                }
            }
        }
    }

    // Print
    println!("Footprint: {} ({}x{} ASCII)", fp_name, w, h);
    println!("  # = SMD pad, O = TH pad");
    let border: String = std::iter::repeat('+').take(w + 2).collect();
    println!("{}", border);
    for row in &grid {
        let line: String = row.iter().collect();
        println!("|{}|", line);
    }
    println!("{}", border);

    Ok(())
}

/// Render footprint as SVG.
pub fn cmd_render_svg(
    _path: &Path,
    _footprint: &str,
    _output: Option<PathBuf>,
    _scale: f64,
    _light: bool,
    _no_grid: bool,
    _no_designators: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("SVG rendering is not yet implemented in the v2 API. \
         Use cmd_render_ascii for a quick text-mode preview."
        .into())
}

/// Render footprint as PNG.
pub fn cmd_render_png(
    _path: &Path,
    _footprint: &str,
    _output: Option<PathBuf>,
    _scale: f64,
    _width: Option<u32>,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("PNG rendering is not yet implemented in the v2 API. \
         Use cmd_render_ascii for a quick text-mode preview."
        .into())
}

// ═══════════════════════════════════════════════════════════════════════════
// MANIPULATION COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// Embedded blank PcbLib template.
const BLANK_PCBLIB_TEMPLATE: &[u8] =
    include_bytes!("../../../data/blank/PcbLib1.PcbLib");

/// Creates an empty PcbLib file at the given path.
pub fn cmd_create(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if path.exists() {
        return Err(format!("File already exists: {}", path.display()).into());
    }

    std::fs::write(path, BLANK_PCBLIB_TEMPLATE)
        .map_err(|e| format!("Error creating file: {}", e))?;

    println!("Created empty PcbLib: {}", path.display());
    Ok(())
}

/// Adds a new footprint pattern to an existing library.
pub fn cmd_add_footprint(
    path: &Path,
    name: &str,
    description: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut lib = open_pcblib(path)?;

    // Check if footprint already exists
    let name_lower = name.to_lowercase();
    if lib
        .footprint_names
        .iter()
        .any(|n| n.to_lowercase() == name_lower)
    {
        return Err(
            format!("Footprint '{}' already exists in library", name).into(),
        );
    }

    // Build metadata parameter string
    let desc = description.as_deref().unwrap_or("");
    let param_str = format!("|PATTERN={}|DESCRIPTION={}|", name, desc);
    let metadata = RecordNode::new(
        0,
        RecordOrigin::Param(crate::v2::backing_store::ParamOrigin::new(
            &param_str,
        )),
    );

    lib.footprint_names.push(name.to_string());
    lib.footprints.push(FootprintGroup::new(
        metadata,
        Vec::new(),
        name.as_bytes().to_vec(),
        Vec::new(),
        Vec::new(),
    ));

    // Write back
    lib.save_file(path).map_err(|e| e.to_string())?;

    println!("Added footprint '{}' to {}", name, path.display());
    Ok(())
}

/// Adds a pad to an existing footprint in the library.
pub fn cmd_add_pad(
    path: &Path,
    footprint: &str,
    designator: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    shape: &str,
    hole: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut lib = open_pcblib(path)?;
    let (idx, _) = find_footprint(&lib, footprint)?;

    let x_raw = mm_to_raw(x);
    let y_raw = mm_to_raw(y);
    let w_raw = mm_to_raw(width);
    let h_raw = mm_to_raw(height);
    let hole_raw = mm_to_raw(hole);
    let shape_byte = parse_shape(shape);

    // If hole > 0, use MultiLayer (74); otherwise TopLayer (1)
    let layer = if hole_raw > 0 { 74 } else { 1 };

    let pad_data = build_pad_binary(
        designator, x_raw, y_raw, w_raw, h_raw, shape_byte, hole_raw, layer,
    );

    let prim = RecordNode::new(
        TYPE_PAD,
        RecordOrigin::Binary(BinaryOrigin::new(pad_data)),
    );
    let prim_idx = lib.footprints[idx].primitives.len();
    lib.footprints[idx].primitives.push(prim);
    lib.footprints[idx]
        .original_primitive_order
        .push(PcbPrimitiveRef::new(TYPE_PAD, prim_idx));

    // Write back
    lib.save_file(path).map_err(|e| e.to_string())?;

    println!(
        "Added pad '{}' ({:.3}mm x {:.3}mm) to footprint '{}' in {}",
        designator,
        width,
        height,
        footprint,
        path.display()
    );
    Ok(())
}

/// Adds a silkscreen track (line) to a footprint.
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
    let (idx, _) = find_footprint(&lib, footprint)?;

    let track_data = build_track_binary(
        mm_to_raw(x1),
        mm_to_raw(y1),
        mm_to_raw(x2),
        mm_to_raw(y2),
        mm_to_raw(width),
        33, // TopOverlay (silkscreen)
    );

    let prim = RecordNode::new(
        TYPE_TRACK,
        RecordOrigin::Binary(BinaryOrigin::new(track_data)),
    );
    let prim_idx = lib.footprints[idx].primitives.len();
    lib.footprints[idx].primitives.push(prim);
    lib.footprints[idx]
        .original_primitive_order
        .push(PcbPrimitiveRef::new(TYPE_TRACK, prim_idx));

    lib.save_file(path).map_err(|e| e.to_string())?;

    println!(
        "Added silkscreen line ({:.3},{:.3})->({:.3},{:.3}) to '{}' in {}",
        x1,
        y1,
        x2,
        y2,
        footprint,
        path.display()
    );
    Ok(())
}

/// Adds a silkscreen arc to a footprint.
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
    let (idx, _) = find_footprint(&lib, footprint)?;

    let arc_data = build_arc_binary(
        mm_to_raw(x),
        mm_to_raw(y),
        mm_to_raw(radius),
        start_angle,
        end_angle,
        mm_to_raw(width),
        33, // TopOverlay (silkscreen)
    );

    let prim = RecordNode::new(
        TYPE_ARC,
        RecordOrigin::Binary(BinaryOrigin::new(arc_data)),
    );
    let prim_idx = lib.footprints[idx].primitives.len();
    lib.footprints[idx].primitives.push(prim);
    lib.footprints[idx]
        .original_primitive_order
        .push(PcbPrimitiveRef::new(TYPE_ARC, prim_idx));

    lib.save_file(path).map_err(|e| e.to_string())?;

    println!(
        "Added arc (center={:.3},{:.3} r={:.3} {:.1}-{:.1}deg) to '{}' in {}",
        x,
        y,
        radius,
        start_angle,
        end_angle,
        footprint,
        path.display()
    );
    Ok(())
}

/// Generate a standard chip (0201/0402/0603/0805/1206) footprint.
pub fn cmd_gen_chip(
    path: &Path,
    size: &str,
    density: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Chip dimensions table: (body_length_mm, body_width_mm)
    // Pad sizes vary by density level per IPC-7351B
    let (body_l, body_w) = match size {
        "0201" => (0.6, 0.3),
        "0402" => (1.0, 0.5),
        "0603" => (1.6, 0.8),
        "0805" => (2.0, 1.25),
        "1206" => (3.2, 1.6),
        "1210" => (3.2, 2.5),
        _ => {
            return Err(format!(
                "Unknown chip size '{}'. Supported: 0201, 0402, 0603, 0805, 1206, 1210",
                size
            )
            .into())
        }
    };

    // Density factor for pad extension beyond body
    let (toe, _heel, side) = match density {
        "most" | "a" => (0.55, 0.0, 0.05),
        "nominal" | "b" => (0.35, 0.0, 0.0),
        "least" | "c" => (0.15, 0.0, -0.05),
        _ => {
            return Err(format!(
                "Unknown density '{}'. Supported: most, nominal, least",
                density
            )
            .into())
        }
    };

    let pad_width = body_w + 2.0 * side;
    let pad_length = body_l / 2.0 + toe;
    let pad_spacing = body_l - pad_length + toe;

    let fp_name = format!("CHIP_{}", size);

    // Create the footprint
    cmd_add_footprint(
        path,
        &fp_name,
        Some(format!(
            "{} chip footprint, {} density",
            size, density
        )),
    )?;

    // Add pads (pad 1 on left, pad 2 on right)
    let x_offset = pad_spacing / 2.0;
    cmd_add_pad(
        path,
        &fp_name,
        "1",
        -x_offset,
        0.0,
        pad_length,
        pad_width,
        "rectangular",
        0.0,
    )?;
    cmd_add_pad(
        path,
        &fp_name,
        "2",
        x_offset,
        0.0,
        pad_length,
        pad_width,
        "rectangular",
        0.0,
    )?;

    // Add silkscreen outline
    let silk_margin = 0.1;
    let silk_x = body_l / 2.0 + silk_margin;
    let silk_y = body_w / 2.0 + silk_margin;
    cmd_add_silkscreen(
        path, &fp_name, -silk_x, silk_y, silk_x, silk_y, 0.15,
    )?;
    cmd_add_silkscreen(
        path, &fp_name, -silk_x, -silk_y, silk_x, -silk_y, 0.15,
    )?;

    println!(
        "Generated {} chip footprint '{}' ({} density)",
        size, fp_name, density
    );
    Ok(())
}

/// Batch import from JSON.
pub fn cmd_add_json(
    path: &Path,
    file: Option<String>,
    input_json: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let json_str = if let Some(ref json) = input_json {
        json.clone()
    } else if let Some(ref file_path) = file {
        if file_path == "-" {
            use std::io::Read;
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            buf
        } else {
            std::fs::read_to_string(file_path)
                .map_err(|e| format!("Error reading {}: {}", file_path, e))?
        }
    } else {
        return Err("Either --file or --input must be provided".into());
    };

    let value: serde_json::Value =
        serde_json::from_str(&json_str).map_err(|e| format!("Invalid JSON: {}", e))?;

    // Support both single footprint and array of footprints
    let footprints = if value.is_array() {
        value.as_array().unwrap().clone()
    } else {
        vec![value]
    };

    let mut count = 0;
    for fp_json in &footprints {
        let name = fp_json["name"]
            .as_str()
            .ok_or("Footprint JSON must have a 'name' field")?;
        let description = fp_json["description"].as_str().map(|s| s.to_string());

        cmd_add_footprint(path, name, description)?;

        if let Some(pads) = fp_json["pads"].as_array() {
            for pad_json in pads {
                let designator = pad_json["designator"].as_str().unwrap_or("?");
                let x = pad_json["x"].as_f64().unwrap_or(0.0);
                let y = pad_json["y"].as_f64().unwrap_or(0.0);
                let width = pad_json["width"].as_f64().unwrap_or(1.0);
                let height = pad_json["height"].as_f64().unwrap_or(1.0);
                let shape = pad_json["shape"].as_str().unwrap_or("rectangular");
                let hole = pad_json["hole"].as_f64().unwrap_or(0.0);

                cmd_add_pad(
                    path,
                    name,
                    designator,
                    x,
                    y,
                    width,
                    height,
                    shape,
                    hole,
                )?;
            }
        }

        count += 1;
    }

    println!(
        "Imported {} footprint(s) from JSON into {}",
        count,
        path.display()
    );
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// PAD PATTERN GENERATION COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// Add a row of pads to a footprint.
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
    shape: &str,
    hole: &str,
    _use_spacing: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let pitch_raw = parse_dimension(pitch)?;
    let pw_raw = parse_dimension(pad_width)?;
    let ph_raw = parse_dimension(pad_height)?;
    let x_offset = parse_dimension(x)?;
    let y_offset = parse_dimension(y)?;
    let hole_raw = parse_dimension(hole)?;
    let shape_byte = parse_shape(shape);
    let layer = if hole_raw > 0 { 74 } else { 1 };

    let is_horizontal = matches!(
        direction.to_lowercase().as_str(),
        "horizontal" | "h" | "x"
    );

    let mut lib = open_pcblib(path)?;
    let (idx, _) = find_footprint(&lib, footprint)?;

    // Center the row so pad centers are symmetric about (x_offset, y_offset)
    let total_span = pitch_raw as i64 * (count as i64 - 1);

    for i in 0..count {
        let pad_num = start + i as u32;
        let designator = pad_num.to_string();
        let offset_along = -(total_span / 2) + pitch_raw as i64 * i as i64;

        let (px, py) = if is_horizontal {
            (x_offset as i64 + offset_along, y_offset as i64)
        } else {
            (x_offset as i64, y_offset as i64 + offset_along)
        };

        let pad_data = build_pad_binary(
            &designator,
            px as i32,
            py as i32,
            pw_raw,
            ph_raw,
            shape_byte,
            hole_raw,
            layer,
        );

        let prim = RecordNode::new(
            TYPE_PAD,
            RecordOrigin::Binary(BinaryOrigin::new(pad_data)),
        );
        let prim_idx = lib.footprints[idx].primitives.len();
        lib.footprints[idx].primitives.push(prim);
        lib.footprints[idx]
            .original_primitive_order
            .push(PcbPrimitiveRef::new(TYPE_PAD, prim_idx));
    }

    lib.save_file(path).map_err(|e| e.to_string())?;

    println!(
        "Added {} pads (row) to footprint '{}' in {}",
        count,
        footprint,
        path.display()
    );
    Ok(())
}

/// Add dual row of pads (SOIC, DIP style).
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
    shape: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let pitch_raw = parse_dimension(pitch)?;
    let spacing_raw = parse_dimension(row_spacing)?;
    let hole_raw = hole.map(|h| parse_dimension(h)).transpose()?.unwrap_or(0);

    // Determine pad dimensions
    let (pw_raw, ph_raw) = if let Some(diam) = pad_diameter {
        let d = parse_dimension(diam)?;
        (d, d)
    } else {
        let pw = pad_width
            .map(|w| parse_dimension(w))
            .transpose()?
            .unwrap_or_else(|| mm_to_raw(0.6));
        let ph = pad_height
            .map(|h| parse_dimension(h))
            .transpose()?
            .unwrap_or_else(|| mm_to_raw(1.5));
        (pw, ph)
    };

    let shape_byte = parse_shape(shape);
    let layer = if hole_raw > 0 { 74 } else { 1 };

    let mut lib = open_pcblib(path)?;
    let (idx, _) = find_footprint(&lib, footprint)?;

    let half_spacing = spacing_raw / 2;
    let total_span = pitch_raw as i64 * (pads_per_side as i64 - 1);
    let total_pads = pads_per_side * 2;

    // Left side: pads 1..N (bottom to top)
    for i in 0..pads_per_side {
        let pad_num = i + 1;
        let designator = pad_num.to_string();
        let y = -(total_span / 2) + pitch_raw as i64 * i as i64;

        let pad_data = build_pad_binary(
            &designator,
            -half_spacing,
            y as i32,
            pw_raw,
            ph_raw,
            shape_byte,
            hole_raw,
            layer,
        );

        let prim = RecordNode::new(
            TYPE_PAD,
            RecordOrigin::Binary(BinaryOrigin::new(pad_data)),
        );
        let prim_idx = lib.footprints[idx].primitives.len();
        lib.footprints[idx].primitives.push(prim);
        lib.footprints[idx]
            .original_primitive_order
            .push(PcbPrimitiveRef::new(TYPE_PAD, prim_idx));
    }

    // Right side: pads N+1..2N (top to bottom, standard IC numbering)
    for i in 0..pads_per_side {
        let pad_num = pads_per_side + 1 + i;
        let designator = pad_num.to_string();
        let y = (total_span / 2) - pitch_raw as i64 * i as i64;

        let pad_data = build_pad_binary(
            &designator,
            half_spacing,
            y as i32,
            pw_raw,
            ph_raw,
            shape_byte,
            hole_raw,
            layer,
        );

        let prim = RecordNode::new(
            TYPE_PAD,
            RecordOrigin::Binary(BinaryOrigin::new(pad_data)),
        );
        let prim_idx = lib.footprints[idx].primitives.len();
        lib.footprints[idx].primitives.push(prim);
        lib.footprints[idx]
            .original_primitive_order
            .push(PcbPrimitiveRef::new(TYPE_PAD, prim_idx));
    }

    lib.save_file(path).map_err(|e| e.to_string())?;

    println!(
        "Added {} pads (dual row, {} per side) to footprint '{}' in {}",
        total_pads,
        pads_per_side,
        footprint,
        path.display()
    );
    Ok(())
}

/// Add quad pattern pads (QFP style).
pub fn cmd_add_quad_pads(
    path: &Path,
    footprint: &str,
    pads_per_side: usize,
    pitch: &str,
    span: &str,
    pad_width: &str,
    pad_height: &str,
    shape: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let pitch_raw = parse_dimension(pitch)?;
    let span_raw = parse_dimension(span)?;
    let pw_raw = parse_dimension(pad_width)?;
    let ph_raw = parse_dimension(pad_height)?;
    let shape_byte = parse_shape(shape);
    let layer: u8 = 1; // SMD pads on TopLayer

    let mut lib = open_pcblib(path)?;
    let (idx, _) = find_footprint(&lib, footprint)?;

    let half_span = span_raw / 2;
    let total_span = pitch_raw as i64 * (pads_per_side as i64 - 1);
    let total_pads = pads_per_side * 4;
    let mut pad_num: u32 = 1;

    // Side 1: Bottom (left to right)
    for i in 0..pads_per_side {
        let x = -(total_span / 2) + pitch_raw as i64 * i as i64;
        let pad_data = build_pad_binary(
            &pad_num.to_string(),
            x as i32,
            -half_span,
            pw_raw,
            ph_raw,
            shape_byte,
            0,
            layer,
        );
        let prim = RecordNode::new(
            TYPE_PAD,
            RecordOrigin::Binary(BinaryOrigin::new(pad_data)),
        );
        let prim_idx = lib.footprints[idx].primitives.len();
        lib.footprints[idx].primitives.push(prim);
        lib.footprints[idx]
            .original_primitive_order
            .push(PcbPrimitiveRef::new(TYPE_PAD, prim_idx));
        pad_num += 1;
    }

    // Side 2: Right (bottom to top)
    for i in 0..pads_per_side {
        let y = -(total_span / 2) + pitch_raw as i64 * i as i64;
        let pad_data = build_pad_binary(
            &pad_num.to_string(),
            half_span,
            y as i32,
            ph_raw, // rotated: width/height swapped
            pw_raw,
            shape_byte,
            0,
            layer,
        );
        let prim = RecordNode::new(
            TYPE_PAD,
            RecordOrigin::Binary(BinaryOrigin::new(pad_data)),
        );
        let prim_idx = lib.footprints[idx].primitives.len();
        lib.footprints[idx].primitives.push(prim);
        lib.footprints[idx]
            .original_primitive_order
            .push(PcbPrimitiveRef::new(TYPE_PAD, prim_idx));
        pad_num += 1;
    }

    // Side 3: Top (right to left)
    for i in 0..pads_per_side {
        let x = (total_span / 2) - pitch_raw as i64 * i as i64;
        let pad_data = build_pad_binary(
            &pad_num.to_string(),
            x as i32,
            half_span,
            pw_raw,
            ph_raw,
            shape_byte,
            0,
            layer,
        );
        let prim = RecordNode::new(
            TYPE_PAD,
            RecordOrigin::Binary(BinaryOrigin::new(pad_data)),
        );
        let prim_idx = lib.footprints[idx].primitives.len();
        lib.footprints[idx].primitives.push(prim);
        lib.footprints[idx]
            .original_primitive_order
            .push(PcbPrimitiveRef::new(TYPE_PAD, prim_idx));
        pad_num += 1;
    }

    // Side 4: Left (top to bottom)
    for i in 0..pads_per_side {
        let y = (total_span / 2) - pitch_raw as i64 * i as i64;
        let pad_data = build_pad_binary(
            &pad_num.to_string(),
            -half_span,
            y as i32,
            ph_raw, // rotated: width/height swapped
            pw_raw,
            shape_byte,
            0,
            layer,
        );
        let prim = RecordNode::new(
            TYPE_PAD,
            RecordOrigin::Binary(BinaryOrigin::new(pad_data)),
        );
        let prim_idx = lib.footprints[idx].primitives.len();
        lib.footprints[idx].primitives.push(prim);
        lib.footprints[idx]
            .original_primitive_order
            .push(PcbPrimitiveRef::new(TYPE_PAD, prim_idx));
        pad_num += 1;
    }

    lib.save_file(path).map_err(|e| e.to_string())?;

    println!(
        "Added {} pads (quad, {} per side) to footprint '{}' in {}",
        total_pads,
        pads_per_side,
        footprint,
        path.display()
    );
    Ok(())
}

/// Add a grid of pads (BGA style).
pub fn cmd_add_pad_grid(
    path: &Path,
    footprint: &str,
    rows: usize,
    cols: usize,
    pitch: &str,
    pad_diameter: &str,
    shape: &str,
    skip_center: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let pitch_raw = parse_dimension(pitch)?;
    let diam_raw = parse_dimension(pad_diameter)?;
    let skip_raw = parse_dimension(skip_center)?;
    let shape_byte = parse_shape(shape);
    let layer: u8 = 1; // SMD pads on TopLayer

    let mut lib = open_pcblib(path)?;
    let (idx, _) = find_footprint(&lib, footprint)?;

    let skip_radius_sq = if skip_raw > 0 {
        let half = skip_raw as f64 / 2.0;
        half * half
    } else {
        0.0
    };

    let x_span = pitch_raw as i64 * (cols as i64 - 1);
    let y_span = pitch_raw as i64 * (rows as i64 - 1);
    let mut pad_count: usize = 0;

    for row in 0..rows {
        let row_letter = (b'A' + row as u8) as char;
        let y = (y_span / 2) - pitch_raw as i64 * row as i64;

        for col in 0..cols {
            let x = -(x_span / 2) + pitch_raw as i64 * col as i64;

            // Skip center region
            if skip_radius_sq > 0.0 {
                let dist_sq =
                    (x as f64) * (x as f64) + (y as f64) * (y as f64);
                if dist_sq < skip_radius_sq {
                    continue;
                }
            }

            let designator = format!("{}{}", row_letter, col + 1);
            let pad_data = build_pad_binary(
                &designator,
                x as i32,
                y as i32,
                diam_raw,
                diam_raw,
                shape_byte,
                0,
                layer,
            );

            let prim = RecordNode::new(
                TYPE_PAD,
                RecordOrigin::Binary(BinaryOrigin::new(pad_data)),
            );
            let prim_idx = lib.footprints[idx].primitives.len();
            lib.footprints[idx].primitives.push(prim);
            lib.footprints[idx]
                .original_primitive_order
                .push(PcbPrimitiveRef::new(TYPE_PAD, prim_idx));
            pad_count += 1;
        }
    }

    lib.save_file(path).map_err(|e| e.to_string())?;

    println!(
        "Added {} pads ({}x{} grid) to footprint '{}' in {}",
        pad_count,
        rows,
        cols,
        footprint,
        path.display()
    );
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alphanumeric_sort() {
        let mut items = vec!["A10", "A2", "A1", "B1"];
        items.sort_by(|a, b| alphanumeric_sort(a, b));
        assert_eq!(items, vec!["A1", "A2", "A10", "B1"]);
    }

    #[test]
    fn test_alphanumeric_sort_pad_designators() {
        let mut items = vec!["10", "2", "1", "20", "A1", "A2"];
        items.sort_by(|a, b| alphanumeric_sort(a, b));
        assert_eq!(items, vec!["1", "2", "10", "20", "A1", "A2"]);
    }

    #[test]
    fn test_primitive_type_name() {
        assert_eq!(primitive_type_name(1), "Arc");
        assert_eq!(primitive_type_name(2), "Pad");
        assert_eq!(primitive_type_name(3), "Via");
        assert_eq!(primitive_type_name(4), "Track");
        assert_eq!(primitive_type_name(5), "Text");
        assert_eq!(primitive_type_name(6), "Fill");
        assert_eq!(primitive_type_name(11), "Region");
        assert_eq!(primitive_type_name(12), "ComponentBody");
        assert_eq!(primitive_type_name(99), "Unknown");
    }

    #[test]
    fn test_shape_name() {
        assert_eq!(shape_name(0), "NoShape");
        assert_eq!(shape_name(1), "Round");
        assert_eq!(shape_name(2), "Rectangular");
        assert_eq!(shape_name(3), "Octagonal");
        assert_eq!(shape_name(9), "RoundedRectangular");
    }

    #[test]
    fn test_layer_name() {
        assert_eq!(layer_name(0), "NoLayer");
        assert_eq!(layer_name(1), "TopLayer");
        assert_eq!(layer_name(32), "BottomLayer");
        assert_eq!(layer_name(33), "TopOverlay");
        assert_eq!(layer_name(74), "MultiLayer");
        assert_eq!(layer_name(5), "MidLayer");
        assert_eq!(layer_name(57), "Mechanical");
    }

    #[test]
    fn test_parse_shape() {
        assert_eq!(parse_shape("round"), 1);
        assert_eq!(parse_shape("rectangular"), 2);
        assert_eq!(parse_shape("rect"), 2);
        assert_eq!(parse_shape("octagonal"), 3);
        assert_eq!(parse_shape("rounded_rect"), 9);
        assert_eq!(parse_shape("unknown"), 2); // defaults to rectangular
    }

    #[test]
    fn test_parse_dimension() {
        // mm
        let raw_mm = parse_dimension("1.0mm").unwrap();
        let coord = PcbCoord::from_raw(raw_mm);
        assert!((coord.to_mm() - 1.0).abs() < 0.01);

        // mil
        let raw_mil = parse_dimension("100mil").unwrap();
        let coord = PcbCoord::from_raw(raw_mil);
        assert!((coord.to_mils() - 100.0).abs() < 0.1);

        // default (mm)
        let raw_default = parse_dimension("2.54").unwrap();
        let coord = PcbCoord::from_raw(raw_default);
        assert!((coord.to_mm() - 2.54).abs() < 0.01);
    }

    #[test]
    fn test_categorize_footprint() {
        assert_eq!(categorize_footprint("BGA-256", ""), "BGA");
        assert_eq!(categorize_footprint("TQFP-100", ""), "QFP");
        assert_eq!(categorize_footprint("QFN-32", ""), "QFN/DFN");
        assert_eq!(categorize_footprint("SOIC-8", ""), "SOIC/SOP");
        assert_eq!(categorize_footprint("SOT-23", ""), "SOT");
        assert_eq!(categorize_footprint("DIP-8", ""), "DIP");
        assert_eq!(categorize_footprint("0603", ""), "Chip/SMD");
        assert_eq!(categorize_footprint("USB_TypeC", ""), "Connector");
        assert_eq!(categorize_footprint("LED_0805", ""), "LED");
        assert_eq!(categorize_footprint("TestPoint_1mm", ""), "Test Point");
        assert_eq!(categorize_footprint("CustomFP", ""), "Other");
    }

    #[test]
    fn test_pad_data_from_node() {
        // Build a minimal pad binary block
        let mut data = Vec::new();

        // Subrecord 1: designator "1"
        let name = b"1";
        data.extend_from_slice(&(name.len() as u32).to_le_bytes());
        data.extend_from_slice(name);

        // Subrecords 2-4: empty
        for _ in 0..3 {
            data.extend_from_slice(&0u32.to_le_bytes());
        }

        // Subrecord 5: core data (172 bytes)
        let core_len: usize = 172;
        data.extend_from_slice(&(core_len as u32).to_le_bytes());
        let core_start = data.len();
        data.resize(core_start + core_len, 0);

        // Layer = TopLayer
        data[core_start] = 1;
        // position_x at +13
        let x_raw = PcbCoord::from_mm(1.0).to_raw();
        data[core_start + 13..core_start + 17]
            .copy_from_slice(&x_raw.to_le_bytes());
        // position_y at +17
        let y_raw = PcbCoord::from_mm(2.0).to_raw();
        data[core_start + 17..core_start + 21]
            .copy_from_slice(&y_raw.to_le_bytes());
        // top_size_x at +21
        let sx = PcbCoord::from_mm(0.5).to_raw();
        data[core_start + 21..core_start + 25]
            .copy_from_slice(&sx.to_le_bytes());
        // top_size_y at +25
        let sy = PcbCoord::from_mm(0.8).to_raw();
        data[core_start + 25..core_start + 29]
            .copy_from_slice(&sy.to_le_bytes());
        // hole_size at +45 = 0 (SMD)
        // top_shape at +49 = 2 (Rectangular)
        data[core_start + 49] = 2;

        // Subrecord 6: stack data
        let stack_len: usize = 596;
        data.extend_from_slice(&(stack_len as u32).to_le_bytes());
        data.resize(data.len() + stack_len, 0);

        let node = RecordNode::new(
            TYPE_PAD,
            RecordOrigin::Binary(BinaryOrigin::new(data)),
        );

        let pad = PadData::from_node(&node).expect("Should parse pad");
        assert_eq!(pad.designator, "1");
        assert!((pad.record.position_x().to_mm() - 1.0).abs() < 0.01);
        assert!((pad.record.position_y().to_mm() - 2.0).abs() < 0.01);
        assert_eq!(pad.record.top_shape(), 2);
        assert!(pad.is_smd());
        assert_eq!(pad.shape_name(), "Rectangular");
        assert_eq!(pad.layer_name(), "TopLayer");
    }

    /// Helper: build a pad binary block for testing bounding box computation.
    fn make_test_pad_node(
        designator: &str,
        x_mm: f64,
        y_mm: f64,
        size_x_mm: f64,
        size_y_mm: f64,
        layer: u8,
    ) -> RecordNode {
        let data = build_pad_binary(
            designator,
            PcbCoord::from_mm(x_mm).to_raw(),
            PcbCoord::from_mm(y_mm).to_raw(),
            PcbCoord::from_mm(size_x_mm).to_raw(),
            PcbCoord::from_mm(size_y_mm).to_raw(),
            2, // Rectangular
            0, // SMD
            layer,
        );
        RecordNode::new(
            TYPE_PAD,
            RecordOrigin::Binary(BinaryOrigin::new(data)),
        )
    }

    #[test]
    fn test_compute_bounding_box() {
        let node1 = make_test_pad_node("1", -1.0, 0.0, 0.5, 0.5, 1);
        let node2 = make_test_pad_node("2", 1.0, 0.0, 0.5, 0.5, 1);

        let pads: Vec<PadData> = vec![
            PadData::from_node(&node1).unwrap(),
            PadData::from_node(&node2).unwrap(),
        ];

        let bb = compute_bounding_box(&pads);
        // Width should be about 2.5mm (pad centers at -1 and +1, each 0.5 wide)
        assert!(bb.width.contains("2.5"));
    }

    #[test]
    fn test_build_pad_binary_roundtrip() {
        let data = build_pad_binary(
            "A1",
            PcbCoord::from_mm(1.27).to_raw(),
            PcbCoord::from_mm(-0.635).to_raw(),
            PcbCoord::from_mm(0.3).to_raw(),
            PcbCoord::from_mm(0.3).to_raw(),
            1, // Round
            0, // SMD
            1, // TopLayer
        );

        let node = RecordNode::new(
            TYPE_PAD,
            RecordOrigin::Binary(BinaryOrigin::new(data)),
        );

        let pad = PadData::from_node(&node).expect("Should parse built pad");
        assert_eq!(pad.designator, "A1");
        assert!(
            (pad.record.position_x().to_mm() - 1.27).abs() < 0.01
        );
        assert!(
            (pad.record.position_y().to_mm() - (-0.635)).abs() < 0.01
        );
        assert_eq!(pad.record.top_shape(), 1); // Round
        assert!(pad.is_smd());
    }
}
