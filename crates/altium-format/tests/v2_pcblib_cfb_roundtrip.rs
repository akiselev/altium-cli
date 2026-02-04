//! Integration test: Read Synthiam.PcbLib → Write from types → Re-read → Compare typed fields.

use std::fs::File;
use std::path::PathBuf;

use altium_format::v2::pcb::io::pcblib::PcbLib;

fn workspace_root() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // crates/
    path.pop(); // workspace root
    path
}

/// CFB roundtrip test - requires Synthiam.PcbLib fixture
#[ignore = "Requires Synthiam.PcbLib fixture in workspace root"]
#[test]
fn cfb_roundtrip_synthiam_pcblib() {
    let root = workspace_root();
    let src_path = root.join("Synthiam.PcbLib");
    if !src_path.exists() {
        panic!("Fixture not found: {}", src_path.display());
    }

    // Read original
    let file = File::open(&src_path).expect("failed to open Synthiam.PcbLib");
    let lib = PcbLib::open(file).expect("failed to parse Synthiam.PcbLib");

    eprintln!(
        "Read {} footprints, {} raw streams",
        lib.footprints.len(),
        lib.raw_streams.len(),
    );

    // Write to new file (serializes from typed fields, NOT raw bytes)
    let dst_path = root.join("Synthiam-roundtrip2.PcbLib");
    let out_file = File::create(&dst_path).expect("failed to create output file");
    lib.write(out_file).expect("failed to write PcbLib");
    eprintln!("Wrote {}", dst_path.display());

    // Re-read the written file
    let verify_file = File::open(&dst_path).expect("failed to open written file");
    let lib2 = PcbLib::open(verify_file).expect("failed to re-parse written file");

    assert_eq!(
        lib.footprints.len(),
        lib2.footprints.len(),
        "footprint count mismatch after roundtrip"
    );

    for (i, (orig, written)) in lib.footprints.iter().zip(lib2.footprints.iter()).enumerate() {
        assert_eq!(orig.name, written.name, "footprint {} name mismatch", i);

        // Compare all typed primitive counts
        assert_eq!(
            orig.tracks.len(), written.tracks.len(),
            "footprint {} ({}) track count mismatch", i, orig.name
        );
        assert_eq!(
            orig.arcs.len(), written.arcs.len(),
            "footprint {} ({}) arc count mismatch", i, orig.name
        );
        assert_eq!(
            orig.fills.len(), written.fills.len(),
            "footprint {} ({}) fill count mismatch", i, orig.name
        );
        assert_eq!(
            orig.pads.len(), written.pads.len(),
            "footprint {} ({}) pad count mismatch", i, orig.name
        );
        assert_eq!(
            orig.vias.len(), written.vias.len(),
            "footprint {} ({}) via count mismatch", i, orig.name
        );
        assert_eq!(
            orig.texts.len(), written.texts.len(),
            "footprint {} ({}) text count mismatch", i, orig.name
        );
        assert_eq!(
            orig.regions.len(), written.regions.len(),
            "footprint {} ({}) region count mismatch", i, orig.name
        );
        assert_eq!(
            orig.component_bodies.len(), written.component_bodies.len(),
            "footprint {} ({}) component_body count mismatch", i, orig.name
        );

        // Compare actual typed field values
        for (j, (a, b)) in orig.tracks.iter().zip(written.tracks.iter()).enumerate() {
            assert_eq!(a, b, "footprint {} ({}) track {} mismatch", i, orig.name, j);
        }
        for (j, (a, b)) in orig.arcs.iter().zip(written.arcs.iter()).enumerate() {
            assert_eq!(a, b, "footprint {} ({}) arc {} mismatch", i, orig.name, j);
        }
        for (j, (a, b)) in orig.fills.iter().zip(written.fills.iter()).enumerate() {
            assert_eq!(a, b, "footprint {} ({}) fill {} mismatch", i, orig.name, j);
        }
        for (j, (a, b)) in orig.pads.iter().zip(written.pads.iter()).enumerate() {
            assert_eq!(a, b, "footprint {} ({}) pad {} mismatch", i, orig.name, j);
        }
        for (j, (a, b)) in orig.vias.iter().zip(written.vias.iter()).enumerate() {
            assert_eq!(a, b, "footprint {} ({}) via {} mismatch", i, orig.name, j);
        }
        for (j, (a, b)) in orig.texts.iter().zip(written.texts.iter()).enumerate() {
            assert_eq!(a, b, "footprint {} ({}) text {} mismatch", i, orig.name, j);
        }
        for (j, (a, b)) in orig.regions.iter().zip(written.regions.iter()).enumerate() {
            assert_eq!(a, b, "footprint {} ({}) region {} mismatch", i, orig.name, j);
        }
        for (j, (a, b)) in orig.component_bodies.iter().zip(written.component_bodies.iter()).enumerate() {
            assert_eq!(a, b, "footprint {} ({}) component_body {} mismatch", i, orig.name, j);
        }

        // Compare parameters
        assert_eq!(
            orig.parameters, written.parameters,
            "footprint {} ({}) parameters mismatch", i, orig.name
        );
    }

    eprintln!(
        "Full roundtrip verified: {} footprints, all typed fields match",
        lib2.footprints.len()
    );
}
