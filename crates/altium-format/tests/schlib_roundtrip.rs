// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Destructive round-trip test for SchLib files.
//!
//! Opens a real SchLib file, extracts all component data, rebuilds from
//! scratch using the v2 API, saves to a new CFB, then compares the
//! original and rebuilt files at the stream level.
//!
//! This test is diagnostic: differences are reported, not asserted
//! (except for structural invariants like component count).
//!
//! Requires `Synthiam.SchLib` at the repo root. Run with:
//!   cargo test --test schlib_roundtrip -- --ignored --nocapture

mod common;

use std::io::Cursor;

use altium_format::v2::backing_store::{ComponentGroup, RecordNode};
use altium_format::v2::documents::{SchLib, SchLibComponentEntry, SchLibHeader};
use altium_format::v2::records::SchComponentRecord;

use common::cfb_compare::compare_cfb_files;

/// Path to the fixture file (at repo root, not committed to git).
const FIXTURE_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Synthiam.SchLib");

#[test]
#[ignore]
fn destructive_roundtrip_synthiam_schlib() {
    // -----------------------------------------------------------------------
    // 1. Open the original SchLib
    // -----------------------------------------------------------------------
    let orig_lib = SchLib::open_file(FIXTURE_PATH).expect("Failed to open Synthiam.SchLib");
    let orig_component_count = orig_lib.component_count();
    println!(
        "Opened Synthiam.SchLib: {} components",
        orig_component_count
    );

    // Print component summary
    for (i, entry) in orig_lib.component_entries.iter().enumerate() {
        let child_count = orig_lib.groups[i].children.len();
        println!(
            "  [{}] {} ({}) - {} children",
            i, entry.lib_ref, entry.description, child_count
        );
    }

    // -----------------------------------------------------------------------
    // 2. Build a new SchLib from scratch using extracted data
    // -----------------------------------------------------------------------
    let mut new_lib = SchLib::default();

    // Copy header but clear raw bytes to force re-serialization
    new_lib.header = SchLibHeader {
        header_text: orig_lib.header.header_text.clone(),
        weight: orig_lib.header.weight,
        minor_version: orig_lib.header.minor_version,
        unique_id: orig_lib.header.unique_id.clone(),
        raw: None, // Force re-serialization of FileHeader
    };

    // For each component: clone the records from the original and rebuild
    for (i, orig_group) in orig_lib.groups.iter().enumerate() {
        let entry = &orig_lib.component_entries[i];

        // Clone the component record's origin and create a new dirty node
        let mut comp_node = RecordNode::new(
            orig_group.component.key,
            orig_group.component.origin.clone(),
        );
        comp_node.mark_dirty();

        // Clone all child records, marking each dirty
        let mut children = Vec::with_capacity(orig_group.children.len());
        let mut record_type_counts: std::collections::HashMap<u8, usize> =
            std::collections::HashMap::new();

        for child in &orig_group.children {
            let mut child_node = RecordNode::new(child.key, child.origin.clone());
            child_node.mark_dirty();
            *record_type_counts.entry(child.key).or_insert(0) += 1;
            children.push(child_node);
        }

        // Report record type distribution for this component
        let comp_record =
            SchComponentRecord::from_origin(orig_group.component.origin.clone());
        println!(
            "  Rebuilding [{}] '{}': {} children",
            i,
            comp_record.lib_reference(),
            children.len()
        );
        let mut type_summary: Vec<_> = record_type_counts.iter().collect();
        type_summary.sort_by_key(|(k, _)| **k);
        for (record_id, count) in &type_summary {
            let name = record_type_name(**record_id);
            println!("    RECORD={} ({}): {}", record_id, name, count);
        }

        // Build new group from cloned origins
        let original_indices: Vec<usize> = (1..=children.len()).collect();
        let new_group = ComponentGroup::new(comp_node, children, original_indices);

        new_lib.component_entries.push(SchLibComponentEntry {
            lib_ref: entry.lib_ref.clone(),
            description: entry.description.clone(),
            part_count: entry.part_count,
        });
        new_lib.groups.push(new_group);
    }

    // -----------------------------------------------------------------------
    // 3. Save both to byte buffers
    // -----------------------------------------------------------------------
    let original_bytes = std::fs::read(FIXTURE_PATH).expect("Failed to read original file");

    let rebuilt_buf = Cursor::new(Vec::new());
    new_lib
        .save(rebuilt_buf)
        .expect("Failed to save rebuilt SchLib");

    // Re-read the rebuilt bytes: save into a fresh buffer to get the bytes
    let mut rebuilt_bytes_cursor = Cursor::new(Vec::new());
    new_lib
        .save(&mut rebuilt_bytes_cursor)
        .expect("Failed to save rebuilt SchLib (2nd pass)");
    let rebuilt_bytes = rebuilt_bytes_cursor.into_inner();

    // -----------------------------------------------------------------------
    // 4. Compare using CFB stream comparison
    // -----------------------------------------------------------------------
    let report = compare_cfb_files(&original_bytes, &rebuilt_bytes);
    println!("\n{}", report);

    // -----------------------------------------------------------------------
    // 5. Structural assertions (these SHOULD pass)
    // -----------------------------------------------------------------------
    assert_eq!(
        new_lib.component_count(),
        orig_component_count,
        "Component count mismatch: original={}, rebuilt={}",
        orig_component_count,
        new_lib.component_count()
    );

    // Log summary
    println!("=== Summary ===");
    println!("Components: {} (original) / {} (rebuilt)", orig_component_count, new_lib.component_count());
    println!("Matched streams: {}", report.matched.len());
    println!("Text diffs: {}", report.text_diffs.len());
    println!("Binary diffs: {}", report.binary_diffs.len());
    println!("Only in original: {}", report.only_in_original.len());
    println!("Only in rebuilt: {}", report.only_in_rebuilt.len());
}

/// Map record type IDs to human-readable names.
fn record_type_name(id: u8) -> &'static str {
    match id {
        1 => "Component",
        2 => "Pin",
        3 => "Symbol",
        4 => "Label",
        5 => "Bezier",
        6 => "Polyline",
        7 => "Polygon",
        8 => "Ellipse",
        9 => "Pie",
        10 => "RoundRectangle",
        11 => "EllipticalArc",
        12 => "Arc",
        13 => "Line",
        14 => "Rectangle",
        17 => "Power",
        18 => "Port",
        22 => "NoERC",
        25 => "NetLabel",
        26 => "Bus",
        27 => "Wire",
        28 => "TextFrame",
        29 => "Junction",
        30 => "Image",
        31 => "Sheet",
        32 => "SheetName",
        33 => "SheetFileName",
        34 => "Designator",
        37 => "BusEntry",
        39 => "SheetSymbol",
        40 => "SheetEntry",
        41 => "Parameter",
        44 => "ImplementationList",
        45 => "Implementation",
        209 => "Note",
        215 => "Blanket",
        _ => "Unknown",
    }
}
