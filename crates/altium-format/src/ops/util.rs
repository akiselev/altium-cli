// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Shared utilities for operations modules.

use crate::io::SchDoc;
use crate::records::sch::SchRecord;
use crate::tree::{RecordId, RecordTree};
use std::collections::HashMap;

/// Sort strings alphanumerically (e.g., "R1", "R2", "R10" instead of "R1", "R10", "R2").
///
/// This function extracts numeric suffixes from strings and sorts them numerically
/// while keeping the prefix in alphabetical order.
pub fn alphanumeric_sort(a: &str, b: &str) -> std::cmp::Ordering {
    let extract_num = |s: &str| -> (String, i32) {
        let mut prefix = String::new();
        let mut num_str = String::new();

        for c in s.chars() {
            if c.is_ascii_digit() {
                num_str.push(c);
            } else if num_str.is_empty() {
                prefix.push(c);
            }
        }

        let num = num_str.parse().unwrap_or(0);
        (prefix, num)
    };

    let (prefix_a, num_a) = extract_num(a);
    let (prefix_b, num_b) = extract_num(b);

    match prefix_a.cmp(&prefix_b) {
        std::cmp::Ordering::Equal => num_a.cmp(&num_b),
        other => other,
    }
}

/// Get the name of a record type.
pub fn record_type_name(record: &SchRecord) -> &'static str {
    match record {
        SchRecord::Component(_) => "Component",
        SchRecord::Pin(_) => "Pin",
        SchRecord::Symbol(_) => "Symbol",
        SchRecord::Label(_) => "Label",
        SchRecord::Bezier(_) => "Bezier",
        SchRecord::Polyline(_) => "Polyline",
        SchRecord::Polygon(_) => "Polygon",
        SchRecord::Ellipse(_) => "Ellipse",
        SchRecord::Pie(_) => "Pie",
        SchRecord::EllipticalArc(_) => "EllipticalArc",
        SchRecord::Arc(_) => "Arc",
        SchRecord::Line(_) => "Line",
        SchRecord::Rectangle(_) => "Rectangle",
        SchRecord::PowerObject(_) => "PowerObject",
        SchRecord::Port(_) => "Port",
        SchRecord::NoErc(_) => "NoERC",
        SchRecord::NetLabel(_) => "NetLabel",
        SchRecord::Bus(_) => "Bus",
        SchRecord::Wire(_) => "Wire",
        SchRecord::TextFrame(_) => "TextFrame",
        SchRecord::TextFrameVariant(_) => "TextFrameVariant",
        SchRecord::Junction(_) => "Junction",
        SchRecord::Image(_) => "Image",
        SchRecord::SheetHeader(_) => "SheetHeader",
        SchRecord::Designator(_) => "Designator",
        SchRecord::BusEntry(_) => "BusEntry",
        SchRecord::Parameter(_) => "Parameter",
        SchRecord::WarningSign(_) => "WarningSign",
        SchRecord::ImplementationList(_) => "ImplementationList",
        SchRecord::Implementation(_) => "Implementation",
        SchRecord::MapDefinerList(_) => "MapDefinerList",
        SchRecord::MapDefiner(_) => "MapDefiner",
        SchRecord::ImplementationParameters(_) => "ImplementationParameters",
        SchRecord::Unknown { .. } => "Unknown",
    }
}

/// Count record types in a document.
pub fn count_record_types(doc: &SchDoc) -> HashMap<&'static str, usize> {
    let mut counts: HashMap<&'static str, usize> = HashMap::new();
    for record in &doc.primitives {
        let name = record_type_name(record);
        *counts.entry(name).or_insert(0) += 1;
    }
    counts
}

/// Get sheet size name from style number.
pub fn sheet_size_name(style: i32) -> &'static str {
    match style {
        0 => "A4",
        1 => "A3",
        2 => "A2",
        3 => "A1",
        4 => "A0",
        5 => "A",
        6 => "B",
        7 => "C",
        8 => "D",
        9 => "E",
        10 => "Letter",
        11 => "Legal",
        12 => "Tabloid",
        13 => "OrCAD A",
        14 => "OrCAD B",
        15 => "OrCAD C",
        16 => "OrCAD D",
        17 => "OrCAD E",
        _ => "Custom",
    }
}

/// Get the designator for a component by finding its Designator child.
pub fn get_component_designator(
    tree: &RecordTree<SchRecord>,
    component_id: RecordId,
) -> Option<String> {
    for (_child_id, child) in tree.children(component_id) {
        if let SchRecord::Designator(d) = child {
            return Some(d.param.label.text.clone());
        }
    }
    None
}

/// Get component pins for netlist building.
pub fn get_component_pins(
    tree: &RecordTree<SchRecord>,
    comp_id: RecordId,
) -> Vec<(String, String, i32, i32)> {
    let mut pins = Vec::new();
    for (_, child) in tree.children(comp_id) {
        if let SchRecord::Pin(p) = child {
            let (corner_x, corner_y) = p.get_corner();
            pins.push((p.designator.clone(), p.name.clone(), corner_x, corner_y));
        }
    }
    pins
}
