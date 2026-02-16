// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Destructive round-trip test for PcbLib files.
//!
//! Opens a real PcbLib file, extracts all footprint data, rebuilds from
//! scratch using the v2 API, saves to a new CFB, then compares the
//! original and rebuilt files at the stream level.
//!
//! This test is diagnostic: differences are reported, not asserted
//! (except for structural invariants like footprint count).
//!
//! Requires `Synthiam.PcbLib` at the repo root. Run with:
//!   cargo test --test pcblib_roundtrip -- --ignored --nocapture

mod common;

use std::io::Cursor;

use altium_format::v2::backing_store::{FootprintGroup, PcbPrimitiveRef, RecordNode};
use altium_format::v2::documents::PcbLib;

use common::cfb_compare::compare_cfb_files;

/// Path to the fixture file (at repo root, not committed to git).
const FIXTURE_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Synthiam.PcbLib");

#[test]
#[ignore]
fn destructive_roundtrip_synthiam_pcblib() {
    // -----------------------------------------------------------------------
    // 1. Open the original PcbLib
    // -----------------------------------------------------------------------
    let orig_lib = PcbLib::open_file(FIXTURE_PATH).expect("Failed to open Synthiam.PcbLib");
    let orig_footprint_count = orig_lib.footprint_count();
    println!(
        "Opened Synthiam.PcbLib: {} footprints",
        orig_footprint_count
    );

    // Print footprint summary
    for (i, name) in orig_lib.footprint_names.iter().enumerate() {
        let group = &orig_lib.footprints[i];
        let prim_count = group.primitives.len();
        let pattern_name_len = group.raw_pattern_name_block.len();
        let header_len = group.raw_header.len();

        // Count primitives by type
        let mut type_counts: std::collections::HashMap<u8, usize> =
            std::collections::HashMap::new();
        for prim in &group.primitives {
            *type_counts.entry(prim.key).or_insert(0) += 1;
        }

        println!(
            "  [{}] '{}': {} primitives, pattern_name={} bytes, header={} bytes",
            i, name, prim_count, pattern_name_len, header_len
        );

        let mut type_summary: Vec<_> = type_counts.iter().collect();
        type_summary.sort_by_key(|(k, _)| **k);
        for (type_id, count) in &type_summary {
            let name = pcb_type_name(**type_id);
            println!("    type={} ({}): {}", type_id, name, count);
        }
    }

    // -----------------------------------------------------------------------
    // 2. Build a new PcbLib from scratch using extracted data
    // -----------------------------------------------------------------------
    let mut new_lib = PcbLib::default();

    for (i, orig_group) in orig_lib.footprints.iter().enumerate() {
        let name = &orig_lib.footprint_names[i];

        // Clone metadata record (footprint parameters), marking dirty
        let mut metadata_node = RecordNode::new(
            orig_group.metadata.key,
            orig_group.metadata.origin.clone(),
        );
        metadata_node.mark_dirty();

        // Clone all primitive records, marking each dirty
        let mut primitives = Vec::with_capacity(orig_group.primitives.len());
        let mut primitive_order = Vec::with_capacity(orig_group.original_primitive_order.len());

        for (idx, prim) in orig_group.primitives.iter().enumerate() {
            // Debug: check raw_block sizes for first 3 footprints
            if i < 3 {
                if let Some(b) = prim.origin.as_binary() {
                    println!(
                        "    prim[{}] type={} raw_block={} bytes, snapshot={} bytes",
                        idx, prim.key, b.raw_block.len(), prim.original_snapshot.len()
                    );
                }
            }
            let mut prim_node = RecordNode::new(prim.key, prim.origin.clone());
            prim_node.mark_dirty();
            primitives.push(prim_node);
            primitive_order.push(PcbPrimitiveRef::new(prim.key, idx));
        }

        println!(
            "  Rebuilding [{}] '{}': {} primitives",
            i,
            name,
            primitives.len()
        );

        // Build new footprint group from cloned origins
        // raw_pattern_name_block: clone from original (this is the binary pattern name)
        // raw_header: empty vec to force re-generation from primitive count
        let new_group = FootprintGroup::new(
            metadata_node,
            primitives,
            orig_group.raw_pattern_name_block.clone(),
            primitive_order,
            Vec::new(), // Empty raw_header forces re-generation
        );

        new_lib.footprint_names.push(name.clone());
        new_lib.footprints.push(new_group);
    }

    // -----------------------------------------------------------------------
    // 3. Save both to byte buffers
    // -----------------------------------------------------------------------
    let original_bytes = std::fs::read(FIXTURE_PATH).expect("Failed to read original file");

    let mut rebuilt_bytes_cursor = Cursor::new(Vec::new());
    new_lib
        .save(&mut rebuilt_bytes_cursor)
        .expect("Failed to save rebuilt PcbLib");
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
        new_lib.footprint_count(),
        orig_footprint_count,
        "Footprint count mismatch: original={}, rebuilt={}",
        orig_footprint_count,
        new_lib.footprint_count()
    );

    // Log summary
    println!("=== Summary ===");
    println!(
        "Footprints: {} (original) / {} (rebuilt)",
        orig_footprint_count,
        new_lib.footprint_count()
    );
    println!("Matched streams: {}", report.matched.len());
    println!("Text diffs: {}", report.text_diffs.len());
    println!("Binary diffs: {}", report.binary_diffs.len());
    println!("Only in original: {}", report.only_in_original.len());
    println!("Only in rebuilt: {}", report.only_in_rebuilt.len());
}

/// Map PCB primitive type IDs to human-readable names.
fn pcb_type_name(id: u8) -> &'static str {
    match id {
        0 => "NoObject",
        1 => "Arc",
        2 => "Pad",
        3 => "Via",
        4 => "Track",
        5 => "Text",
        6 => "Fill",
        7 => "Connection",
        8 => "Net",
        9 => "Component",
        10 => "Polygon",
        11 => "Region",
        12 => "ComponentBody",
        13 => "Dimension",
        _ => "Unknown",
    }
}
