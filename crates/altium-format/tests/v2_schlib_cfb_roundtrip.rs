//! Integration test: Read Synthiam.SchLib → Write Synthiam-new.SchLib
//!
//! Creates a new SchLib file from the parsed v2 data. The output can be
//! opened in Altium Designer to verify correctness.

use std::fs::File;
use std::path::PathBuf;

use altium_format::v2::io::schlib::SchLibV2;

fn workspace_root() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // crates/
    path.pop(); // workspace root
    path
}

/// CFB roundtrip test - requires Synthiam.SchLib fixture
#[ignore = "Requires Synthiam.SchLib fixture in workspace root"]
#[test]
fn cfb_roundtrip_synthiam_schlib() {
    let root = workspace_root();
    let src_path = root.join("Synthiam.SchLib");
    if !src_path.exists() {
        panic!("Fixture not found: {}", src_path.display());
    }

    // Read original
    let file = File::open(&src_path).expect("failed to open Synthiam.SchLib");
    let lib = SchLibV2::open(file).expect("failed to parse Synthiam.SchLib");

    eprintln!(
        "Read {} components, header: {}",
        lib.components.len(),
        &lib.header.header_text[..lib.header.header_text.len().min(60)]
    );

    // Write to new file
    let dst_path = root.join("Synthiam-new.SchLib");
    let out_file = File::create(&dst_path).expect("failed to create output file");
    lib.write(out_file).expect("failed to write Synthiam-new.SchLib");

    eprintln!("Wrote {}", dst_path.display());

    // Verify: re-read the written file
    let verify_file = File::open(&dst_path).expect("failed to open written file");
    let lib2 = SchLibV2::open(verify_file).expect("failed to re-parse written file");

    assert_eq!(
        lib.components.len(),
        lib2.components.len(),
        "component count mismatch after roundtrip"
    );

    for (i, (orig, written)) in lib.components.iter().zip(lib2.components.iter()).enumerate() {
        assert_eq!(
            orig.entry.lib_ref, written.entry.lib_ref,
            "component {} lib_ref mismatch",
            i
        );
        assert_eq!(
            orig.entry.part_count, written.entry.part_count,
            "component {} part_count mismatch",
            i
        );
        assert_eq!(
            orig.records.len(),
            written.records.len(),
            "component {} ({}) record count mismatch",
            i,
            orig.entry.lib_ref
        );

        for (j, (or, wr)) in orig.records.iter().zip(written.records.iter()).enumerate() {
            assert_eq!(
                or.record_id, wr.record_id,
                "component {} record {} record_id mismatch",
                i, j
            );
            assert_eq!(
                or.params, wr.params,
                "component {} record {} params mismatch",
                i, j
            );
        }
    }

    eprintln!(
        "Roundtrip verified: {} components, all records match",
        lib2.components.len()
    );
}
