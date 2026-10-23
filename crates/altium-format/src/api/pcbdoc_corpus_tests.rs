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

/// Identity fields every R4 primitive carries, for comparing a board with
/// itself after a write.
fn r4_identity(board: &PcbDocBoard) -> Vec<(String, Option<String>, Option<usize>, u16)> {
    let mut out = Vec::new();
    macro_rules! collect {
        ($list:expr) => {
            for p in &$list {
                out.push((
                    p.id.clone(),
                    p.unique_id.clone(),
                    p.component_index,
                    p.flags.raw(),
                ));
            }
        };
    }
    collect!(board.tracks);
    collect!(board.arcs);
    collect!(board.vias);
    collect!(board.pads);
    collect!(board.fills);
    collect!(board.texts);
    collect!(board.regions);
    for b in &board.component_bodies {
        out.push((b.id.clone(), None, b.component_index, b.flags.raw()));
    }
    out
}

/// R4: unique ids, component indices, flags and the Components6 identity
/// fields read on every corpus board, agree with the designators, and survive
/// `update_board` → `board()` unchanged.
#[test]
fn corpus_r4_identity() {
    let (mut components, mut locked, mut unlocked_primitives, mut pad_ids) = (0, 0, 0, 0);
    let (mut linked, mut boards_round_tripped) = (0, 0);
    for (name, mut doc) in corpus_docs() {
        let ctx = build_context(&doc).unwrap_or_else(|e| panic!("{name}: {e}"));
        pad_ids += ctx.pad_unique_ids.len();
        for c in convert_components(&doc).unwrap_or_else(|e| panic!("{name}: {e}")) {
            components += 1;
            assert_eq!(c.unique_id.len(), 8, "{name}: {}", c.designator);
            locked += usize::from(c.locked);
            unlocked_primitives += usize::from(!c.primitive_lock);
        }
        let Ok(board) = doc.board() else {
            assert!(BOARDS_FAILING_AT_BASE.contains(&name.as_str()), "{name}");
            continue;
        };
        let pads_with_ids = board.pads.iter().filter(|p| p.unique_id.is_some()).count();
        assert_eq!(pads_with_ids, ctx.pad_unique_ids.len(), "{name}");
        let before = r4_identity(&board);
        for (id, _, index, _) in &before {
            if let Some(i) = index {
                assert!(*i < board.components.len(), "{name}: {id}");
                linked += 1;
            }
        }
        for p in &board.pads {
            if let Some(i) = p.component_index {
                assert_eq!(
                    p.component.as_deref(),
                    Some(board.components[i].designator.as_str()),
                    "{name}: {}",
                    p.id
                );
            }
        }
        doc.update_board(&board)
            .unwrap_or_else(|e| panic!("{name}: update_board: {e}"));
        let after = doc
            .board()
            .unwrap_or_else(|e| panic!("{name}: board() after update: {e}"));
        // update_board → board() added one region on 65 of the 79 boards up
        // to this branch (a board region duplicated into ShapeBasedRegions6).
        assert_eq!(
            after.regions.len(),
            board.regions.len(),
            "{name}: region count"
        );
        let after_ids = r4_identity(&after);
        assert_eq!(
            after_ids.len(),
            before.len(),
            "{name}: primitive count changed"
        );
        if let Some((a, b)) = after_ids.iter().zip(&before).find(|(a, b)| a != b) {
            panic!("{name}: identity changed on write: {b:?} became {a:?}");
        }
        let comp_ids = |b: &PcbDocBoard| -> Vec<(String, bool, bool)> {
            b.components
                .iter()
                .map(|c| (c.unique_id.clone(), c.locked, c.primitive_lock))
                .collect()
        };
        assert_eq!(
            comp_ids(&after),
            comp_ids(&board),
            "{name}: components changed"
        );
        boards_round_tripped += 1;
    }
    eprintln!(
        "corpus_r4_identity: {components} components ({locked} locked, {unlocked_primitives} \
         without primitive lock), {pad_ids} pad unique ids, {linked} primitives linked to a \
         component, {boards_round_tripped} boards written and re-read"
    );
    assert_eq!(components, 14_409);
    assert_eq!(locked, 395);
    assert_eq!(unlocked_primitives, 49);
    assert_eq!(pad_ids, 58_719);
    assert_eq!(boards_round_tripped, 79);
}

/// R3: every stored polygon index is in range or the split-plane line marker
/// (all 94 boards). On the readable boards every polygon member is a region on
/// its polygon's layer with no net of its own (one test board, test-via2, has
/// a top-layer region in a bottom-layer pour), and membership survives
/// `update_board` → `board()`.
#[test]
fn corpus_r3_polygon_membership() {
    let (mut stored, mut split_lines) = (0, 0);
    let (mut members_total, mut layer_mismatches, mut boards_round_tripped) = (0, 0, 0);
    for (name, mut doc) in corpus_docs() {
        let polygon_count =
            find_param_section(&doc, ParamSectionKind::Polygons6).map_or(0, <[_]>::len);
        for section in &doc.sections {
            if let PcbDocSection::Primitive(prim) = section {
                for record in &prim.records {
                    let common = match &record.primitive {
                        PcbPrimitive::Track(p) => &p.common,
                        PcbPrimitive::Arc(p) => &p.common,
                        PcbPrimitive::Via(p) => &p.common,
                        PcbPrimitive::Pad(p) => &p.common,
                        PcbPrimitive::Fill(p) => &p.common,
                        PcbPrimitive::Text(p) => &p.common,
                        PcbPrimitive::Region(p) => &p.common,
                        PcbPrimitive::ComponentBody(p) => &p.common,
                    };
                    let index = common.polygon_index;
                    if index == POLYGON_INDEX_PLANE_SPLIT_LINE {
                        split_lines += 1;
                        assert!(
                            matches!(
                                prim.kind,
                                PrimitiveSectionKind::Tracks6 | PrimitiveSectionKind::Arcs6
                            ) && common.layer.is_internal_plane(),
                            "{name}: split-line marker on {:?} {:?}",
                            prim.kind,
                            common.layer
                        );
                    } else if index != PRIMITIVE_INDEX_NONE {
                        stored += 1;
                        assert!(
                            usize::from(index) < polygon_count,
                            "{name}: {:?} polygon index {index} of {polygon_count}",
                            prim.kind
                        );
                    }
                }
            }
        }
        let Ok(board) = doc.board() else {
            assert!(BOARDS_FAILING_AT_BASE.contains(&name.as_str()), "{name}");
            continue;
        };
        let mut members: Vec<(String, Option<usize>, bool)> = Vec::new();
        macro_rules! no_members {
            ($list:expr, $split:expr) => {
                for p in &$list {
                    assert_eq!(
                        p.polygon, None,
                        "{name}: {} unexpectedly in a polygon",
                        p.id
                    );
                    members.push((p.id.clone(), p.polygon, $split(p)));
                }
            };
        }
        no_members!(board.tracks, |p: &Track| p.plane_split_line);
        no_members!(board.arcs, |p: &Arc| p.plane_split_line);
        no_members!(board.vias, |_: &Via| false);
        no_members!(board.pads, |_: &Pad| false);
        no_members!(board.fills, |_: &Fill| false);
        for r in &board.regions {
            members.push((r.id.clone(), r.polygon, false));
            let Some(i) = r.polygon else { continue };
            members_total += 1;
            assert_eq!(r.net, None, "{name}: pour region {} has a net", r.id);
            if r.layer.to_v6() != board.polygons[i].layer.to_v6() {
                assert_eq!(name, "test-via2.PcbDoc", "{}: layer vs polygon {i}", r.id);
                layer_mismatches += 1;
            }
        }
        let region_count = board.regions.len();
        doc.update_board(&board)
            .unwrap_or_else(|e| panic!("{name}: update_board: {e}"));
        let after = doc
            .board()
            .unwrap_or_else(|e| panic!("{name}: board() after update: {e}"));
        let mut after_members: Vec<(String, Option<usize>, bool)> = Vec::new();
        after_members.extend(
            after
                .tracks
                .iter()
                .map(|p| (p.id.clone(), p.polygon, p.plane_split_line)),
        );
        after_members.extend(
            after
                .arcs
                .iter()
                .map(|p| (p.id.clone(), p.polygon, p.plane_split_line)),
        );
        after_members.extend(after.vias.iter().map(|p| (p.id.clone(), p.polygon, false)));
        after_members.extend(after.pads.iter().map(|p| (p.id.clone(), p.polygon, false)));
        after_members.extend(after.fills.iter().map(|p| (p.id.clone(), p.polygon, false)));
        assert_eq!(
            after.regions.len(),
            region_count,
            "{name}: region count changed"
        );
        after_members.extend(
            after
                .regions
                .iter()
                .map(|p| (p.id.clone(), p.polygon, false)),
        );
        assert!(
            after_members == members,
            "{name}: polygon membership changed on write"
        );
        boards_round_tripped += 1;
    }
    eprintln!(
        "corpus_r3_polygon_membership: {stored} polygon indices and {split_lines} split-plane \
         line markers stored; {members_total} pour regions on readable boards \
         ({layer_mismatches} off their polygon's layer); {boards_round_tripped} boards written \
         and re-read"
    );
    assert_eq!(split_lines, 2_448);
    assert_eq!(layer_mismatches, 1);
    assert_eq!(boards_round_tripped, 79);
}

/// R1: every corpus board's Board6 outline reads with its arcs (counts from
/// the olefile survey: 94 outlines, 1,037 vertices, one open), survives
/// `update_board` → `board()`, and a replaced, shorter outline leaves none of
/// the old vertices behind.
#[test]
fn corpus_r1_board_outline() {
    let (mut outlines, mut vertices, mut arcs, mut open) = (0, 0, 0, 0);
    let mut boards_round_tripped = 0;
    for (name, mut doc) in corpus_docs() {
        let settings = convert_board_settings(&doc).unwrap_or_else(|e| panic!("{name}: {e}"));
        let outline = settings
            .outline
            .unwrap_or_else(|| panic!("{name}: no Board6 outline"));
        outlines += 1;
        vertices += outline.segments.len();
        arcs += outline
            .segments
            .iter()
            .filter(|s| matches!(s, ContourSegment::Arc { .. }))
            .count();
        let points = outline.to_points();
        if points.first() != points.last() {
            open += 1;
        }
        let Ok(mut board) = doc.board() else {
            assert!(BOARDS_FAILING_AT_BASE.contains(&name.as_str()), "{name}");
            continue;
        };
        doc.update_board(&board)
            .unwrap_or_else(|e| panic!("{name}: update_board: {e}"));
        let after = doc.board().unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(
            format!("{:?}", after.settings.outline),
            format!("{:?}", board.settings.outline),
            "{name}: outline changed on write"
        );
        let triangle = PcbContour {
            segments: points[..3]
                .iter()
                .map(|&endpoint| ContourSegment::Line { endpoint })
                .collect(),
        };
        board.settings.outline = Some(triangle.clone());
        doc.update_board(&board)
            .unwrap_or_else(|e| panic!("{name}: update_board (triangle): {e}"));
        let after = doc.board().unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(
            format!("{:?}", after.settings.outline),
            format!("{:?}", Some(triangle)),
            "{name}: replaced outline"
        );
        boards_round_tripped += 1;
    }
    eprintln!(
        "corpus_r1_board_outline: {outlines} outlines, {vertices} vertices, {arcs} arc edges, \
         {open} open; {boards_round_tripped} boards written and re-read"
    );
    assert_eq!(outlines, 94);
    assert_eq!(vertices, 1_037);
    assert_eq!(open, 1);
    assert_eq!(boards_round_tripped, 79);
}

/// R6: on every corpus board, each Track/Arc/Fill that stores both a V6 and a
/// V7 layer satisfies `legacy_v6(v7) == v6`; every stack copper layer has a
/// distinct `copper_v6`; on readable boards every copper via/pad layer is one
/// of them; and primitive layers survive `update_board` → `board()`.
#[test]
fn corpus_r6_layer_identity() {
    let (mut pairs, mut full_form, mut stack_layers, mut boards_round_tripped) = (0, 0, 0, 0);
    for (name, mut doc) in corpus_docs() {
        for section in &doc.sections {
            if let PcbDocSection::Primitive(prim) = section {
                for record in &prim.records {
                    let (v6, v7) = match &record.primitive {
                        PcbPrimitive::Track(p) => (p.common.layer, Some(p.layer_enum_index)),
                        PcbPrimitive::Arc(p) => (p.common.layer, Some(p.layer_enum_index)),
                        PcbPrimitive::Fill(p) => (p.common.layer, p.layer_enum_index),
                        _ => continue,
                    };
                    let Some(v7) = v7.filter(|v| v.raw() != 0) else {
                        continue;
                    };
                    pairs += 1;
                    full_form += usize::from(v7.to_v6().is_err());
                    assert_eq!(v7.legacy_v6(), Some(v6), "{name}: {:#010x}", v7.raw());
                }
            }
        }
        let settings = convert_board_settings(&doc).unwrap_or_else(|e| panic!("{name}: {e}"));
        let copper: Vec<V6Layer> = settings
            .layer_stack
            .layers
            .iter()
            .map(|l| {
                l.layer
                    .copper_v6()
                    .unwrap_or_else(|| panic!("{name}: stack layer {} not copper", l.layer))
            })
            .collect();
        stack_layers += copper.len();
        let mut distinct = copper.clone();
        distinct.sort_by_key(|v| *v as u8);
        distinct.dedup();
        assert_eq!(distinct.len(), copper.len(), "{name}: repeated stack layer");
        let Ok(board) = doc.board() else {
            assert!(BOARDS_FAILING_AT_BASE.contains(&name.as_str()), "{name}");
            continue;
        };
        let in_stack = |layer: &LayerRef, what: &str| {
            if let Some(v6) = layer.copper_v6() {
                assert!(
                    copper.contains(&v6),
                    "{name}: {what} on {layer} not in the stack"
                );
            }
        };
        for v in &board.vias {
            in_stack(&v.from_layer, &v.id);
            in_stack(&v.to_layer, &v.id);
        }
        for p in &board.pads {
            in_stack(&p.layer, &p.id);
        }
        let layers = |b: &PcbDocBoard| -> Vec<String> {
            let mut out: Vec<String> = Vec::new();
            out.extend(b.tracks.iter().map(|p| format!("{:?}", p.layer)));
            out.extend(b.arcs.iter().map(|p| format!("{:?}", p.layer)));
            out.extend(b.fills.iter().map(|p| format!("{:?}", p.layer)));
            out.extend(b.pads.iter().map(|p| format!("{:?}", p.layer)));
            out.extend(b.regions.iter().map(|p| format!("{:?}", p.layer)));
            out
        };
        let before = layers(&board);
        doc.update_board(&board)
            .unwrap_or_else(|e| panic!("{name}: update_board: {e}"));
        let after = doc.board().unwrap_or_else(|e| panic!("{name}: {e}"));
        let after = layers(&after);
        assert_eq!(after.len(), before.len(), "{name}");
        if let Some((a, b)) = after.iter().zip(&before).find(|(a, b)| a != b) {
            panic!("{name}: layer changed on write: {b} became {a}");
        }
        boards_round_tripped += 1;
    }
    eprintln!(
        "corpus_r6_layer_identity: {pairs} V6/V7 layer pairs ({full_form} in full V7 form), \
         {stack_layers} stack copper layers; {boards_round_tripped} boards written and re-read"
    );
    assert_eq!(boards_round_tripped, 79);
}
