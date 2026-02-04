//! Integration test: SchLib v2 → JSON → v2 roundtrip.
//!
//! Reads Synthiam.SchLib via the v2 reader, serializes to JSON,
//! deserializes back, and verifies field-level equality.

use std::fs::File;
use std::path::PathBuf;

use altium_format::v2::io::schlib::SchLibV2;

fn synthiam_schlib_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // crates/
    path.pop(); // workspace root
    path.push("Synthiam.SchLib");
    path
}

/// Test reading Synthiam.SchLib - requires fixture in workspace root
#[ignore = "Requires Synthiam.SchLib fixture in workspace root"]
#[test]
fn read_synthiam_schlib() {
    let path = synthiam_schlib_path();
    if !path.exists() {
        panic!("Fixture not found: {}", path.display());
    }

    let file = File::open(&path).expect("failed to open Synthiam.SchLib");
    let lib = SchLibV2::open(file).expect("failed to parse Synthiam.SchLib");

    assert!(!lib.components.is_empty(), "expected at least one component");

    eprintln!("Parsed {} components:", lib.components.len());
    for comp in &lib.components {
        eprintln!(
            "  {} — {} records, {} parts, desc: {}",
            comp.entry.lib_ref,
            comp.records.len(),
            comp.entry.part_count,
            &comp.entry.description[..comp.entry.description.len().min(60)],
        );
    }
}

/// JSON roundtrip test - requires fixture in workspace root
#[ignore = "Requires Synthiam.SchLib fixture in workspace root"]
#[test]
fn json_roundtrip_synthiam_schlib() {
    let path = synthiam_schlib_path();
    if !path.exists() {
        panic!("Fixture not found: {}", path.display());
    }

    let file = File::open(&path).expect("failed to open Synthiam.SchLib");
    let original = SchLibV2::open(file).expect("failed to parse Synthiam.SchLib");

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&original).expect("failed to serialize to JSON");
    eprintln!("JSON size: {} bytes", json.len());

    // Deserialize back
    let restored: SchLibV2 =
        serde_json::from_str(&json).expect("failed to deserialize from JSON");

    // Structural equality
    assert_eq!(
        original.components.len(),
        restored.components.len(),
        "component count mismatch"
    );

    for (i, (orig, rest)) in original
        .components
        .iter()
        .zip(restored.components.iter())
        .enumerate()
    {
        assert_eq!(
            orig.entry.lib_ref, rest.entry.lib_ref,
            "component {} lib_ref mismatch",
            i
        );
        assert_eq!(
            orig.entry.description, rest.entry.description,
            "component {} description mismatch",
            i
        );
        assert_eq!(
            orig.entry.part_count, rest.entry.part_count,
            "component {} part_count mismatch",
            i
        );
        assert_eq!(
            orig.entry.aliases, rest.entry.aliases,
            "component {} aliases mismatch",
            i
        );
        assert_eq!(
            orig.records.len(),
            rest.records.len(),
            "component {} record count mismatch",
            i
        );

        for (j, (or, rr)) in orig.records.iter().zip(rest.records.iter()).enumerate() {
            assert_eq!(
                or.record_id, rr.record_id,
                "component {} record {} record_id mismatch",
                i, j
            );
            assert_eq!(
                or.record_id_ex, rr.record_id_ex,
                "component {} record {} record_id_ex mismatch",
                i, j
            );
            assert_eq!(
                or.params, rr.params,
                "component {} record {} params mismatch",
                i, j
            );
        }
    }

    // Header equality
    assert_eq!(original.header.header_text, restored.header.header_text);
    assert_eq!(original.header.weight, restored.header.weight);
    assert_eq!(original.header.minor_version, restored.header.minor_version);
    assert_eq!(original.header.unique_id, restored.header.unique_id);

    eprintln!(
        "JSON roundtrip: all {} components verified",
        original.components.len()
    );
}
