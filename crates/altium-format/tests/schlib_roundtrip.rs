// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Round-trip test for SchLib files.
//!
//! Opens a real SchLib file, saves it to a byte buffer, re-opens from that
//! buffer, then compares structural invariants (component count, names) and
//! raw CFB streams between the original and saved copies.
//!
//! This test is diagnostic: stream differences are reported, not asserted
//! (except for structural invariants like component count).
//!
//! Requires `Synthiam.SchLib` at the repo root. Run with:
//!   cargo test --test schlib_roundtrip -- --ignored --nocapture

mod common;

use std::io::Cursor;

use altium_format::v2::documents::SchLib;

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

    // Print component summary using the new entries() API
    let entries = orig_lib.entries();
    let store = orig_lib.store().borrow();
    for (i, entry) in entries.iter().enumerate() {
        let group_id = store.group_ids()[i];
        let group = store.group(group_id);
        let child_count = group.child_ids().len();
        println!(
            "  [{}] {} ({}) - {} children",
            i, entry.lib_ref, entry.description, child_count
        );
    }
    drop(store);

    // Print header info
    let header = orig_lib.header();
    println!("Header: {}", header.header_text);
    println!("UniqueID: {}", header.unique_id);

    // -----------------------------------------------------------------------
    // 2. Save to a byte buffer (identity write-back)
    // -----------------------------------------------------------------------
    let mut rebuilt_bytes_cursor = Cursor::new(Vec::new());
    orig_lib
        .save(&mut rebuilt_bytes_cursor)
        .expect("Failed to save SchLib");
    let rebuilt_bytes = rebuilt_bytes_cursor.into_inner();

    // -----------------------------------------------------------------------
    // 3. Re-open from the saved buffer
    // -----------------------------------------------------------------------
    let reloaded_lib =
        SchLib::open(Cursor::new(rebuilt_bytes.clone())).expect("Failed to re-open saved SchLib");
    let reloaded_count = reloaded_lib.component_count();
    println!("Re-opened SchLib: {} components", reloaded_count);

    // -----------------------------------------------------------------------
    // 4. Compare using CFB stream comparison
    // -----------------------------------------------------------------------
    let original_bytes = std::fs::read(FIXTURE_PATH).expect("Failed to read original file");
    let report = compare_cfb_files(&original_bytes, &rebuilt_bytes);
    println!("\n{}", report);

    // -----------------------------------------------------------------------
    // 5. Structural assertions (these SHOULD pass)
    // -----------------------------------------------------------------------
    assert_eq!(
        reloaded_count, orig_component_count,
        "Component count mismatch after round-trip: original={}, reloaded={}",
        orig_component_count, reloaded_count
    );

    // Verify component names are preserved
    let orig_names = orig_lib.component_names();
    let reloaded_names = reloaded_lib.component_names();
    assert_eq!(
        orig_names, reloaded_names,
        "Component names changed after round-trip"
    );

    // Regression guard: FileHeader patching should preserve header semantics.
    assert!(
        !report
            .text_diffs
            .iter()
            .any(|d| d.stream_name == "/FileHeader"),
        "Unexpected /FileHeader text diff after round-trip"
    );

    // Log summary
    println!("=== Summary ===");
    println!(
        "Components: {} (original) / {} (reloaded)",
        orig_component_count, reloaded_count
    );
    println!("Matched streams: {}", report.matched.len());
    println!("Text diffs: {}", report.text_diffs.len());
    println!("Binary diffs: {}", report.binary_diffs.len());
    println!("Only in original: {}", report.only_in_original.len());
    println!("Only in rebuilt: {}", report.only_in_rebuilt.len());
}

/// Map record type IDs to human-readable names.
#[allow(dead_code)]
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
        43 => "TaskHolder",
        44 => "ImplementationList",
        45 => "Implementation",
        209 => "Note",
        225 => "Blanket",
        _ => "Unknown",
    }
}

/// Print a summary of record type distribution for a component's children.
#[allow(dead_code)]
fn print_record_type_summary(
    store: &altium_format::v2::store::DocumentStore,
    child_ids: &[altium_format::v2::ids::RecordId],
) {
    let mut type_counts: std::collections::HashMap<u8, usize> = std::collections::HashMap::new();
    for &id in child_ids {
        let key = store.record(id).key;
        *type_counts.entry(key).or_insert(0) += 1;
    }
    let mut type_summary: Vec<_> = type_counts.iter().collect();
    type_summary.sort_by_key(|(k, _)| **k);
    for (record_id, count) in &type_summary {
        let name = record_type_name(**record_id);
        println!("    RECORD={} ({}): {}", record_id, name, count);
    }
}
