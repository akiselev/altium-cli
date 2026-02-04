//! Integration test: Read SchDoc files → Write new copies → verify roundtrip.

use std::fs::File;
use std::path::PathBuf;

use altium_format::v2::io::schdoc::SchDocV2;

fn test_schdoc_roundtrip(src_dir: &str, filename: &str) {
    let src_path = PathBuf::from(src_dir).join(filename);
    if !src_path.exists() {
        panic!("Fixture not found: {}", src_path.display());
    }

    // Read original
    let file = File::open(&src_path).expect("failed to open SchDoc");
    let doc = SchDocV2::open(file).expect("failed to parse SchDoc");

    eprintln!(
        "{}: {} records, {} raw streams, weight={}",
        filename,
        doc.records.len(),
        doc.raw_streams.len(),
        doc.weight,
    );

    // Write to new file
    let stem = filename.trim_end_matches(".SchDoc");
    let dst_name = format!("{}-new.SchDoc", stem);
    let dst_path = PathBuf::from(src_dir).join(&dst_name);
    let out_file = File::create(&dst_path).expect("failed to create output file");
    doc.write(out_file).expect("failed to write SchDoc");

    eprintln!("Wrote {}", dst_path.display());

    // Verify: re-read the written file
    let verify_file = File::open(&dst_path).expect("failed to open written file");
    let doc2 = SchDocV2::open(verify_file).expect("failed to re-parse written file");

    assert_eq!(
        doc.records.len(),
        doc2.records.len(),
        "{}: record count mismatch after roundtrip",
        filename,
    );

    assert_eq!(
        doc.weight, doc2.weight,
        "{}: weight mismatch after roundtrip",
        filename,
    );

    assert_eq!(
        doc.raw_streams.len(),
        doc2.raw_streams.len(),
        "{}: raw stream count mismatch after roundtrip",
        filename,
    );

    eprintln!(
        "{}: roundtrip verified ({} records, {} streams)",
        filename,
        doc2.records.len(),
        doc2.raw_streams.len(),
    );
}

/// CFB roundtrip test for M2 Mosaic SchDoc files - requires local fixtures
#[ignore = "Requires M2_Mosaic-G5_Smart fixtures at C:/Users/dev/git/"]
#[test]
fn cfb_roundtrip_m2_mosaic_schdocs() {
    let dir = "C:/Users/dev/git/M2_Mosaic-G5_Smart";
    test_schdoc_roundtrip(dir, "Cover_Page.SchDoc");
    test_schdoc_roundtrip(dir, "M.2_Key_A+E_Card.SchDoc");
    test_schdoc_roundtrip(dir, "Mosaic-G5.SchDoc");
}
