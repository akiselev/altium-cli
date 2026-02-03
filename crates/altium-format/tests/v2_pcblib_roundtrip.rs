//! Integration test: PcbLib v2 → JSON → v2 roundtrip.
//!
//! Reads Synthiam.PcbLib via the v2 reader, serializes to JSON,
//! deserializes back, and verifies field-level equality.

use std::fs::File;
use std::path::PathBuf;

use altium_format::v2::pcb::io::pcblib::PcbLib;

fn synthiam_pcblib_path() -> PathBuf {
    // Try workspace root
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // crates/
    path.pop(); // workspace root
    path.push("Synthiam.PcbLib");
    path
}

#[test]
fn read_synthiam_pcblib() {
    let path = synthiam_pcblib_path();
    if !path.exists() {
        eprintln!("Skipping: {} not found", path.display());
        return;
    }

    let file = File::open(&path).expect("failed to open Synthiam.PcbLib");
    let lib = PcbLib::open(file).expect("failed to parse Synthiam.PcbLib");

    assert!(!lib.footprints.is_empty(), "expected at least one footprint");

    // Print summary
    eprintln!("Parsed {} footprints:", lib.footprints.len());
    for fp in &lib.footprints {
        eprintln!(
            "  {} — {} tracks, {} arcs, {} pads, {} vias, {} texts, {} regions, {} bodies",
            fp.name,
            fp.tracks.len(),
            fp.arcs.len(),
            fp.pads.len(),
            fp.vias.len(),
            fp.texts.len(),
            fp.regions.len(),
            fp.component_bodies.len(),
        );
    }
}

#[test]
fn json_roundtrip_synthiam_pcblib() {
    let path = synthiam_pcblib_path();
    if !path.exists() {
        eprintln!("Skipping: {} not found", path.display());
        return;
    }

    let file = File::open(&path).expect("failed to open Synthiam.PcbLib");
    let original = PcbLib::open(file).expect("failed to parse Synthiam.PcbLib");

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&original).expect("failed to serialize to JSON");

    eprintln!("JSON size: {} bytes", json.len());

    // Deserialize back
    let restored: PcbLib = serde_json::from_str(&json).expect("failed to deserialize from JSON");

    // Structural equality checks
    assert_eq!(
        original.footprints.len(),
        restored.footprints.len(),
        "footprint count mismatch"
    );

    for (i, (orig_fp, rest_fp)) in original
        .footprints
        .iter()
        .zip(restored.footprints.iter())
        .enumerate()
    {
        assert_eq!(orig_fp.name, rest_fp.name, "footprint {} name mismatch", i);
        assert_eq!(
            orig_fp.primitive_count, rest_fp.primitive_count,
            "footprint {} primitive_count mismatch",
            i
        );
        assert_eq!(
            orig_fp.tracks.len(),
            rest_fp.tracks.len(),
            "footprint {} track count mismatch",
            i
        );
        assert_eq!(
            orig_fp.arcs.len(),
            rest_fp.arcs.len(),
            "footprint {} arc count mismatch",
            i
        );
        assert_eq!(
            orig_fp.pads.len(),
            rest_fp.pads.len(),
            "footprint {} pad count mismatch",
            i
        );
        assert_eq!(
            orig_fp.vias.len(),
            rest_fp.vias.len(),
            "footprint {} via count mismatch",
            i
        );
        assert_eq!(
            orig_fp.texts.len(),
            rest_fp.texts.len(),
            "footprint {} text count mismatch",
            i
        );
        assert_eq!(
            orig_fp.regions.len(),
            rest_fp.regions.len(),
            "footprint {} region count mismatch",
            i
        );
        assert_eq!(
            orig_fp.component_bodies.len(),
            rest_fp.component_bodies.len(),
            "footprint {} component_body count mismatch",
            i
        );
        assert_eq!(
            orig_fp.parameters, rest_fp.parameters,
            "footprint {} parameters mismatch",
            i
        );

        // Deep comparison: track fields
        for (j, (ot, rt)) in orig_fp.tracks.iter().zip(rest_fp.tracks.iter()).enumerate() {
            assert_eq!(ot, rt, "footprint {} track {} mismatch", i, j);
        }

        // Deep comparison: arc fields (approx for f64 angles)
        for (j, (oa, ra)) in orig_fp.arcs.iter().zip(rest_fp.arcs.iter()).enumerate() {
            assert_eq!(oa.header, ra.header, "footprint {} arc {} header mismatch", i, j);
            assert_eq!(oa.center_x, ra.center_x, "footprint {} arc {} center_x mismatch", i, j);
            assert_eq!(oa.center_y, ra.center_y, "footprint {} arc {} center_y mismatch", i, j);
            assert_eq!(oa.radius, ra.radius, "footprint {} arc {} radius mismatch", i, j);
            assert!(
                (oa.start_angle - ra.start_angle).abs() < 1e-10,
                "footprint {} arc {} start_angle mismatch: {} vs {}",
                i, j, oa.start_angle, ra.start_angle
            );
            assert!(
                (oa.end_angle - ra.end_angle).abs() < 1e-10,
                "footprint {} arc {} end_angle mismatch: {} vs {}",
                i, j, oa.end_angle, ra.end_angle
            );
            assert_eq!(oa.width, ra.width, "footprint {} arc {} width mismatch", i, j);
            assert_eq!(oa.subpoly_index, ra.subpoly_index, "footprint {} arc {} subpoly_index mismatch", i, j);
            assert_eq!(oa.trailing, ra.trailing, "footprint {} arc {} trailing mismatch", i, j);
        }

        // Deep comparison: pad fields (skipping raw_core which is #[serde(skip)])
        for (j, (op, rp)) in orig_fp.pads.iter().zip(rest_fp.pads.iter()).enumerate() {
            assert_eq!(op.name(), rp.name(), "footprint {} pad {} name mismatch", i, j);
            assert_eq!(
                op.core.position_x, rp.core.position_x,
                "footprint {} pad {} position_x mismatch",
                i, j
            );
            assert_eq!(
                op.core.position_y, rp.core.position_y,
                "footprint {} pad {} position_y mismatch",
                i, j
            );
            assert_eq!(
                op.core.hole_size, rp.core.hole_size,
                "footprint {} pad {} hole_size mismatch",
                i, j
            );
        }

        // Deep comparison: text fields (skipping raw_sub1 which is #[serde(skip)])
        for (j, (ot, rt)) in orig_fp.texts.iter().zip(rest_fp.texts.iter()).enumerate() {
            assert_eq!(
                ot.text, rt.text,
                "footprint {} text {} string mismatch",
                i, j
            );
            assert_eq!(
                ot.position_x, rt.position_x,
                "footprint {} text {} position_x mismatch",
                i, j
            );
        }
    }

    eprintln!("JSON roundtrip: all {} footprints verified", original.footprints.len());
}
