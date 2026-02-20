// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Round-trip test for PcbLib files.
//!
//! Opens a real PcbLib file, saves it to a byte buffer, re-opens from that
//! buffer, then compares structural invariants (footprint count, names) and
//! raw CFB streams between the original and saved copies.
//!
//! This test is diagnostic: stream differences are reported, not asserted
//! (except for structural invariants like footprint count).
//!
//! Requires `Synthiam.PcbLib` at the repo root. Run with:
//!   cargo test --test pcblib_roundtrip -- --ignored --nocapture

mod common;

use std::io::Cursor;

use altium_format::documents::PcbLib;

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

    // Print footprint summary from public names API.
    for (i, name) in orig_lib.names().iter().enumerate() {
        println!("  [{}] '{}'", i, name);
    }

    // -----------------------------------------------------------------------
    // 2. Save to a byte buffer (identity write-back)
    // -----------------------------------------------------------------------
    let mut rebuilt_bytes_cursor = Cursor::new(Vec::new());
    orig_lib
        .save(&mut rebuilt_bytes_cursor)
        .expect("Failed to save PcbLib");
    let rebuilt_bytes = rebuilt_bytes_cursor.into_inner();

    // -----------------------------------------------------------------------
    // 3. Re-open from the saved buffer
    // -----------------------------------------------------------------------
    let reloaded_lib =
        PcbLib::open(Cursor::new(rebuilt_bytes.clone())).expect("Failed to re-open saved PcbLib");
    let reloaded_count = reloaded_lib.footprint_count();
    println!("Re-opened PcbLib: {} footprints", reloaded_count);

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
        reloaded_count, orig_footprint_count,
        "Footprint count mismatch after round-trip: original={}, reloaded={}",
        orig_footprint_count, reloaded_count
    );

    // Verify footprint names are preserved
    let orig_names = orig_lib.names();
    let reloaded_names = reloaded_lib.names();
    assert_eq!(
        orig_names, reloaded_names,
        "Footprint names changed after round-trip"
    );

    // Regression guard: system storages (e.g. FileVersionInfo) must not be
    // misclassified as footprints that synthesize Parameters streams.
    assert!(
        !report
            .only_in_rebuilt
            .iter()
            .any(|p| p == "/FileVersionInfo/Parameters"),
        "Unexpected synthesized /FileVersionInfo/Parameters stream"
    );

    // Log summary
    println!("=== Summary ===");
    println!(
        "Footprints: {} (original) / {} (reloaded)",
        orig_footprint_count, reloaded_count
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
