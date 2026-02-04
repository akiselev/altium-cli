//! Integration test: Read PcbDoc → Write from types → Re-read → Compare typed fields.

use std::fs::File;
use std::path::PathBuf;

use altium_format::v2::pcb::io::pcbdoc::PcbDoc;

fn test_pcbdoc_roundtrip(src_dir: &str, filename: &str) {
    let src_path = PathBuf::from(src_dir).join(filename);
    if !src_path.exists() {
        panic!("Fixture not found: {}", src_path.display());
    }

    // Read original
    let file = File::open(&src_path).expect("failed to open PcbDoc");
    let doc = PcbDoc::open(file).expect("failed to parse PcbDoc");

    eprintln!(
        "{}: {} tracks, {} arcs, {} fills, {} pads, {} vias, {} texts, {} connections, \
         {} nets, {} components, {} polygons, {} regions, {} bodies, {} rules, {} classes, \
         {} dimensions, {} wide_strings, {} ext_info",
        filename,
        doc.tracks.len(), doc.arcs.len(), doc.fills.len(),
        doc.pads.len(), doc.vias.len(), doc.texts.len(),
        doc.connections.len(), doc.nets.len(), doc.components.len(),
        doc.polygons.len(), doc.regions.len(), doc.component_bodies.len(),
        doc.rules.len(), doc.classes.len(), doc.dimensions.len(),
        doc.wide_strings.len(), doc.extended_primitive_info.len(),
    );

    // Write to new file (serializes from typed fields, NOT raw bytes)
    let stem = filename.trim_end_matches(".PcbDoc");
    let dst_name = format!("{}-new.PcbDoc", stem);
    let dst_path = PathBuf::from(src_dir).join(&dst_name);
    let out_file = File::create(&dst_path).expect("failed to create output file");
    doc.write(out_file).expect("failed to write PcbDoc");
    eprintln!("Wrote {}", dst_path.display());

    // Re-read the written file
    let verify_file = File::open(&dst_path).expect("failed to open written file");
    let doc2 = PcbDoc::open(verify_file).expect("failed to re-parse written file");

    // Compare all typed field counts
    assert_eq!(doc.tracks.len(), doc2.tracks.len(), "track count mismatch");
    assert_eq!(doc.arcs.len(), doc2.arcs.len(), "arc count mismatch");
    assert_eq!(doc.fills.len(), doc2.fills.len(), "fill count mismatch");
    assert_eq!(doc.pads.len(), doc2.pads.len(), "pad count mismatch");
    assert_eq!(doc.vias.len(), doc2.vias.len(), "via count mismatch");
    assert_eq!(doc.texts.len(), doc2.texts.len(), "text count mismatch");
    assert_eq!(doc.connections.len(), doc2.connections.len(), "connection count mismatch");
    assert_eq!(doc.nets.len(), doc2.nets.len(), "net count mismatch");
    assert_eq!(doc.components.len(), doc2.components.len(), "component count mismatch");
    assert_eq!(doc.polygons.len(), doc2.polygons.len(), "polygon count mismatch");
    assert_eq!(doc.regions.len(), doc2.regions.len(), "region count mismatch");
    assert_eq!(doc.component_bodies.len(), doc2.component_bodies.len(), "component_body count mismatch");
    assert_eq!(doc.rules.len(), doc2.rules.len(), "rule count mismatch");
    assert_eq!(doc.classes.len(), doc2.classes.len(), "class count mismatch");
    assert_eq!(doc.dimensions.len(), doc2.dimensions.len(), "dimension count mismatch");
    assert_eq!(doc.wide_strings.len(), doc2.wide_strings.len(), "wide_string count mismatch");
    assert_eq!(doc.extended_primitive_info.len(), doc2.extended_primitive_info.len(), "ext_info count mismatch");

    // Compare actual typed field values
    for (i, (a, b)) in doc.tracks.iter().zip(doc2.tracks.iter()).enumerate() {
        assert_eq!(a, b, "track {} mismatch", i);
    }
    for (i, (a, b)) in doc.arcs.iter().zip(doc2.arcs.iter()).enumerate() {
        assert_eq!(a, b, "arc {} mismatch", i);
    }
    for (i, (a, b)) in doc.fills.iter().zip(doc2.fills.iter()).enumerate() {
        assert_eq!(a, b, "fill {} mismatch", i);
    }
    for (i, (a, b)) in doc.pads.iter().zip(doc2.pads.iter()).enumerate() {
        assert_eq!(a, b, "pad {} mismatch", i);
    }
    for (i, (a, b)) in doc.vias.iter().zip(doc2.vias.iter()).enumerate() {
        assert_eq!(a, b, "via {} mismatch", i);
    }
    for (i, (a, b)) in doc.texts.iter().zip(doc2.texts.iter()).enumerate() {
        assert_eq!(a, b, "text {} mismatch", i);
    }
    for (i, (a, b)) in doc.connections.iter().zip(doc2.connections.iter()).enumerate() {
        assert_eq!(a, b, "connection {} mismatch", i);
    }
    for (i, (a, b)) in doc.regions.iter().zip(doc2.regions.iter()).enumerate() {
        assert_eq!(a, b, "region {} mismatch", i);
    }
    for (i, (a, b)) in doc.component_bodies.iter().zip(doc2.component_bodies.iter()).enumerate() {
        assert_eq!(a, b, "component_body {} mismatch", i);
    }
    for (i, (a, b)) in doc.wide_strings.iter().zip(doc2.wide_strings.iter()).enumerate() {
        assert_eq!(a, b, "wide_string {} mismatch", i);
    }

    // Parametric types: compare properties maps
    for (i, (a, b)) in doc.nets.iter().zip(doc2.nets.iter()).enumerate() {
        assert_eq!(a.properties, b.properties, "net {} properties mismatch", i);
    }
    for (i, (a, b)) in doc.components.iter().zip(doc2.components.iter()).enumerate() {
        assert_eq!(a.properties, b.properties, "component {} properties mismatch", i);
    }
    for (i, (a, b)) in doc.polygons.iter().zip(doc2.polygons.iter()).enumerate() {
        assert_eq!(a.properties, b.properties, "polygon {} properties mismatch", i);
    }
    for (i, (a, b)) in doc.rules.iter().zip(doc2.rules.iter()).enumerate() {
        assert_eq!(a.properties, b.properties, "rule {} properties mismatch", i);
    }
    for (i, (a, b)) in doc.classes.iter().zip(doc2.classes.iter()).enumerate() {
        assert_eq!(a.properties, b.properties, "class {} properties mismatch", i);
    }
    for (i, (a, b)) in doc.dimensions.iter().zip(doc2.dimensions.iter()).enumerate() {
        assert_eq!(a.properties, b.properties, "dimension {} properties mismatch", i);
    }
    for (i, (a, b)) in doc.extended_primitive_info.iter().zip(doc2.extended_primitive_info.iter()).enumerate() {
        assert_eq!(a, b, "ext_info {} mismatch", i);
    }

    if let (Some(a), Some(b)) = (&doc.board, &doc2.board) {
        assert_eq!(a.properties, b.properties, "board properties mismatch");
    }

    eprintln!(
        "{}: full roundtrip verified — all typed fields match",
        filename,
    );
}

/// CFB roundtrip test for M2 Mosaic PcbDoc - requires local fixture
#[ignore = "Requires M2_Mosaic-G5_Smart fixtures at C:/Users/dev/git/"]
#[test]
fn cfb_roundtrip_m2_mosaic_pcbdoc() {
    test_pcbdoc_roundtrip(
        "C:/Users/dev/git/M2_Mosaic-G5_Smart",
        "M2_Mosaic-G5_Smart.PcbDoc",
    );
}
