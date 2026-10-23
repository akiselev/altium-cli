//! Corpus checks for the PcbDoc board API (`data/pcbdoc`, a clone of
//! akiselev/altium-cli-test-pcbdoc at `6bc5aa7`).
//!
//! Run one at a time, filtered by name, e.g.
//! `cargo test -p altium-format --features test-fixtures corpus_board_reads`.

use std::path::{Path, PathBuf};

use super::*;

/// CFB PcbDocs in `data/pcbdoc` that `PcbDoc::open` reads (ASCII PcbDocs and
/// the CFB files `open` rejects are not part of these checks).
fn corpus_docs() -> Vec<(String, PcbDoc)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/pcbdoc");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .map(|e| e.expect("dir entry").path())
        .filter(|p| {
            p.extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("pcbdoc"))
        })
        .collect();
    paths.sort();
    let docs: Vec<(String, PcbDoc)> = paths
        .iter()
        .filter_map(|p| {
            let name = p.file_name()?.to_string_lossy().into_owned();
            PcbDoc::open(p).ok().map(|doc| (name, doc))
        })
        .collect();
    assert_eq!(docs.len(), 94, "PcbDocs that open at corpus 6bc5aa7");
    docs
}

/// Boards whose `board()` failed at altium-cli `18062d6` (the rev ecadabra
/// pins), all on a Dimensions6 `TEXTX` value outside this lane's requests.
const BOARDS_FAILING_AT_BASE: [&str; 15] = [
    "artiq-kasli.PcbDoc",
    "oshw-ac-rc-unit.PcbDoc",
    "rfsoc-acmc-mezzanine.PcbDoc",
    "stlink-v3-mb1367c.PcbDoc",
    "stm32h7-mb1137.PcbDoc",
    "thesis-lora-carrier.PcbDoc",
    "thesis-lora-som.PcbDoc",
    "thesis-tree-inspection-v06.PcbDoc",
    "thesis-tree-inspection.PcbDoc",
    "tracker-keyboard.PcbDoc",
    "tvws-agc-calib.PcbDoc",
    "tvws-rf-proto.PcbDoc",
    "tvws-wab-1x1.PcbDoc",
    "tvws-wab-1x4.PcbDoc",
    "tvws-whitespace-dc.PcbDoc",
];

/// No board that read at the base rev may fail now, and the known failures
/// still fail for their known reason (so a new failure cannot hide there).
#[test]
fn corpus_board_reads() {
    let mut ok = 0;
    for (name, doc) in corpus_docs() {
        match doc.board() {
            Ok(_) => {
                assert!(
                    !BOARDS_FAILING_AT_BASE.contains(&name.as_str()),
                    "{name} now reads; update BOARDS_FAILING_AT_BASE"
                );
                ok += 1;
            }
            Err(e) => {
                let msg = format!("{e}");
                assert!(
                    BOARDS_FAILING_AT_BASE.contains(&name.as_str()),
                    "{name} read at the base rev and now fails: {msg}"
                );
                assert!(
                    msg.contains("Dimensions6") && msg.contains("TEXTX"),
                    "{name}: {msg}"
                );
            }
        }
    }
    eprintln!("corpus_board_reads: {ok} of 94 boards read");
    assert_eq!(ok, 79);
}

/// R2: every Polygons6 record of every corpus board parses with its full
/// contour (counts from an independent olefile survey of the corpus), and
/// writing it back and reading that gives the same typed polygon.
#[test]
fn corpus_r2_polygons() {
    let (mut polygons, mut split_planes, mut vertices, mut arcs) = (0, 0, 0, 0);
    for (name, doc) in corpus_docs() {
        let ctx = build_context(&doc).unwrap();
        let parsed =
            convert_polygons(&doc, &ctx).unwrap_or_else(|e| panic!("{name}: Polygons6: {e}"));
        let net_names: Vec<&str> = ctx.net_names.iter().map(String::as_str).collect();
        for p in &parsed {
            let written = crate::api::pcbdoc_write::build_polygon_record_for_test(p, &net_names)
                .unwrap_or_else(|e| panic!("{name}: writing {}: {e}", p.id));
            let reread = convert_polygon_for_test(&written, &net_names)
                .unwrap_or_else(|e| panic!("{name}: re-reading {}: {e}", p.id));
            let mut reread = reread;
            reread.id.clone_from(&p.id);
            assert_eq!(
                format!("{reread:?}"),
                format!("{p:?}"),
                "{name}: {} write->read differs",
                p.id
            );
            polygons += 1;
            if p.polygon_type == PolygonType::SplitPlane {
                split_planes += 1;
                assert!(p.hatch_style.is_none(), "{name}: {}", p.id);
            } else {
                assert!(p.hatch_style.is_some(), "{name}: {}", p.id);
            }
            assert_eq!(p.vertices.len(), p.contour.segments.len());
            vertices += p.contour.segments.len();
            arcs += p
                .contour
                .segments
                .iter()
                .filter(|s| matches!(s, ContourSegment::Arc { .. }))
                .count();
            let first = p.vertices.first().expect("non-empty contour");
            let last = p.vertices.last().expect("non-empty contour");
            assert_eq!(first, last, "{name}: {} contour is not closed", p.id);
        }
    }
    eprintln!(
        "corpus_r2_polygons: {polygons} polygons ({split_planes} split planes), \
         {vertices} vertices, {arcs} arc edges"
    );
    assert_eq!(polygons, 1_618);
    assert_eq!(split_planes, 105);
    assert_eq!(vertices, 25_070);
}
