//! Corpus checks for the PcbDoc board API (`data/pcbdoc`, a clone of
//! akiselev/altium-cli-test-pcbdoc at `6bc5aa7`).
//!
//! Run one at a time, filtered by name, e.g.
//! `cargo test -p altium-format --features test-fixtures corpus_board_reads`.

use std::path::{Path, PathBuf};

use super::*;
use altium_format_types::pcb::TCacheState;

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
        for c in convert_components(&doc, &ctx).unwrap_or_else(|e| panic!("{name}: {e}")) {
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

/// R5: pad mask expansion modes on every corpus board come from the pad cache
/// states, and survive `update_board` → `board()` (with the cache states
/// themselves unchanged).
#[test]
fn corpus_r5_pad_modes() {
    use altium_format_types::pcb::MaskExpansionMode;
    let (mut pads, mut solder_manual, mut paste_manual, mut boards_round_tripped) = (0, 0, 0, 0);
    let (mut separate_bottom, mut from_hole_edge) = (0, 0);
    for (name, mut doc) in corpus_docs() {
        let states = |doc: &PcbDoc| -> Vec<(TCacheState, TCacheState)> {
            doc.sections
                .iter()
                .filter_map(|s| match s {
                    PcbDocSection::Primitive(p) if p.kind == PrimitiveSectionKind::Pads6 => Some(p),
                    _ => None,
                })
                .flat_map(|p| p.records.iter())
                .filter_map(|r| match &r.primitive {
                    PcbPrimitive::Pad(p) => Some((
                        p.cache.solder_mask_expansion_valid,
                        p.cache.paste_mask_expansion_valid,
                    )),
                    _ => None,
                })
                .collect()
        };
        let before_states = states(&doc);
        let Ok(board) = doc.board() else {
            assert!(BOARDS_FAILING_AT_BASE.contains(&name.as_str()), "{name}");
            continue;
        };
        assert_eq!(board.pads.len(), before_states.len(), "{name}");
        for (p, (solder, paste)) in board.pads.iter().zip(&before_states) {
            pads += 1;
            assert_eq!(
                p.solder_mask_mode == MaskExpansionMode::Manual,
                *solder == TCacheState::Manual
            );
            assert_eq!(
                p.paste_mask_mode == MaskExpansionMode::Manual,
                *paste == TCacheState::Manual
            );
            assert_ne!(p.solder_mask_mode, MaskExpansionMode::NoMask);
            assert_ne!(p.paste_mask_mode, MaskExpansionMode::NoMask);
            separate_bottom += usize::from(p.solder_mask_bottom_expansion.is_some());
            from_hole_edge += usize::from(p.solder_mask_from_hole_edge);
            solder_manual += usize::from(p.solder_mask_mode == MaskExpansionMode::Manual);
            paste_manual += usize::from(p.paste_mask_mode == MaskExpansionMode::Manual);
        }
        doc.update_board(&board)
            .unwrap_or_else(|e| panic!("{name}: update_board: {e}"));
        assert_eq!(
            states(&doc),
            before_states,
            "{name}: pad cache states changed"
        );
        let pad_masks = |b: &PcbDocBoard| -> Vec<String> {
            b.pads
                .iter()
                .map(|p| {
                    format!(
                        "{:?} {:?} {:?} {:?} {:?} {}",
                        p.solder_mask_mode,
                        p.paste_mask_mode,
                        p.solder_mask_expansion,
                        p.paste_mask_expansion,
                        p.solder_mask_bottom_expansion,
                        p.solder_mask_from_hole_edge
                    )
                })
                .collect()
        };
        let after = doc.board().unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(
            pad_masks(&after),
            pad_masks(&board),
            "{name}: pad masks changed"
        );
        // Insert a new pad in front: the old pads keep their own internal
        // fields (matched by unique id), so their cache states do not shift.
        if let Some(first) = board.pads.first().cloned() {
            let mut inserted = board.clone();
            let mut new_pad = first;
            new_pad.unique_id = None;
            inserted.pads.insert(0, new_pad);
            doc.update_board(&inserted)
                .unwrap_or_else(|e| panic!("{name}: update_board (inserted): {e}"));
            assert_eq!(
                states(&doc)[1..],
                before_states[..],
                "{name}: states shifted"
            );
        }
        boards_round_tripped += 1;
    }
    eprintln!(
        "corpus_r5_pad_modes: {pads} pads on readable boards, {solder_manual} manual solder \
         mask, {paste_manual} manual paste mask, {separate_bottom} separate bottom solder \
         expansions, {from_hole_edge} from the hole edge; {boards_round_tripped} boards written"
    );
    assert_eq!((separate_bottom, from_hole_edge), (5, 0));
    assert_eq!(boards_round_tripped, 79);
}

/// R7: region contours keep their arcs (corpus: 223 ShapeBasedRegions6
/// regions with arc edges), survive `update_board` → `board()` exactly, and an
/// edited outline is written (the writer used to copy the old contours back).
#[test]
fn corpus_r7_region_contours() {
    let (mut regions_with_arcs, mut arc_edges, mut boards_round_tripped) = (0, 0, 0);
    for (name, mut doc) in corpus_docs() {
        let Ok(mut board) = doc.board() else {
            assert!(BOARDS_FAILING_AT_BASE.contains(&name.as_str()), "{name}");
            continue;
        };
        for r in &board.regions {
            assert_eq!(r.outline, r.outline_contour.to_points(), "{name}: {}", r.id);
            let arcs = r
                .outline_contour
                .segments
                .iter()
                .filter(|s| matches!(s, ContourSegment::Arc { .. }))
                .count();
            regions_with_arcs += usize::from(arcs > 0);
            arc_edges += arcs;
        }
        let contours = |b: &PcbDocBoard| -> Vec<String> {
            b.regions
                .iter()
                .map(|r| format!("{:?} {:?}", r.outline_contour, r.hole_contours))
                .collect()
        };
        let before = contours(&board);
        doc.update_board(&board)
            .unwrap_or_else(|e| panic!("{name}: update_board: {e}"));
        let after = doc.board().unwrap_or_else(|e| panic!("{name}: {e}"));
        let after_contours = contours(&after);
        assert_eq!(after_contours.len(), before.len(), "{name}");
        if let Some((i, _)) = after_contours
            .iter()
            .zip(&before)
            .enumerate()
            .find(|(_, (a, b))| a != b)
        {
            panic!("{name}: region_{i} contour changed on write");
        }
        // Move the last region by 1 mil and check the edit is written.
        if let Some(r) = board.regions.last_mut() {
            let shift = Coord::from_internal(10_000);
            for s in &mut r.outline_contour.segments {
                match s {
                    ContourSegment::Line { endpoint } => endpoint.x += shift,
                    ContourSegment::Arc {
                        endpoint, center, ..
                    } => {
                        endpoint.x += shift;
                        center.x += shift;
                    }
                }
            }
            r.outline = r.outline_contour.to_points();
            let expected = format!("{:?}", r.outline_contour);
            doc.update_board(&board)
                .unwrap_or_else(|e| panic!("{name}: update_board (moved): {e}"));
            let moved = doc.board().unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(
                format!(
                    "{:?}",
                    moved.regions.last().expect("region").outline_contour
                ),
                expected,
                "{name}: edited outline not written"
            );
        }
        boards_round_tripped += 1;
    }
    eprintln!(
        "corpus_r7_region_contours: {regions_with_arcs} regions with {arc_edges} arc edges on \
         readable boards; {boards_round_tripped} boards written and re-read"
    );
    assert_eq!(boards_round_tripped, 79);
    assert_eq!(regions_with_arcs, 192);
    assert_eq!(arc_edges, 1_017);
}

/// R8: components without SOURCEDESIGNATOR (603 in the corpus, 565 on
/// ebaz4205-fpga) take the designator from their one designator text; pads of
/// ebaz4205-fpga map to distinct components by index; and a write adds no
/// SOURCEDESIGNATOR key to them.
#[test]
fn corpus_r8_designators() {
    let (mut without_source, mut boards_round_tripped) = (0, 0);
    for (name, mut doc) in corpus_docs() {
        let ctx = build_context(&doc).unwrap_or_else(|e| panic!("{name}: {e}"));
        let missing: Vec<usize> = ctx
            .source_designators
            .iter()
            .enumerate()
            .filter(|(_, s)| s.is_none())
            .map(|(i, _)| i)
            .collect();
        without_source += missing.len();
        let Ok(board) = doc.board() else {
            assert!(BOARDS_FAILING_AT_BASE.contains(&name.as_str()), "{name}");
            continue;
        };
        for &i in &missing {
            let text = board
                .texts
                .iter()
                .find(|t| t.is_designator && t.component_index == Some(i))
                .unwrap_or_else(|| panic!("{name}: component #{i} has no designator text"));
            assert_eq!(board.components[i].designator, text.text, "{name}: #{i}");
        }
        if name == "ebaz4205-fpga.PcbDoc" {
            let owners: std::collections::BTreeSet<usize> = board
                .pads
                .iter()
                .filter_map(|p| p.component_index)
                .collect();
            let designators: std::collections::BTreeSet<&str> = board
                .components
                .iter()
                .map(|c| c.designator.as_str())
                .collect();
            eprintln!(
                "ebaz4205: {} pads, {} distinct owner indices, {} components, {} distinct designators",
                board.pads.len(),
                owners.len(),
                board.components.len(),
                designators.len()
            );
            assert_eq!(board.components.len(), 565);
            assert!(
                designators.len() > 500,
                "designators should not all be empty"
            );
        }
        doc.update_board(&board)
            .unwrap_or_else(|e| panic!("{name}: update_board: {e}"));
        let after = doc.board().unwrap_or_else(|e| panic!("{name}: {e}"));
        let ids = |b: &PcbDocBoard| -> Vec<(String, Option<String>)> {
            b.components
                .iter()
                .map(|c| (c.designator.clone(), c.source_designator.clone()))
                .collect()
        };
        assert_eq!(
            ids(&after),
            ids(&board),
            "{name}: designators changed on write"
        );
        // A rename must reach the stored key: alone it is an error, with the
        // source designator it reads back.
        if let Some(i) = board
            .components
            .iter()
            .position(|c| c.source_designator.is_some())
        {
            let mut renamed = board.clone();
            renamed.components[i].designator = "RENAMED1".to_owned();
            macro_rules! rename_owner {
                ($($list:ident),*) => {$(
                    for p in renamed.$list.iter_mut().filter(|p| p.component_index == Some(i)) {
                        p.component = Some("RENAMED1".to_owned());
                    }
                )*};
            }
            rename_owner!(
                tracks,
                arcs,
                vias,
                pads,
                fills,
                texts,
                regions,
                component_bodies
            );
            assert!(doc.update_board(&renamed).is_err(), "{name}: silent rename");
            renamed.components[i].source_designator = Some("RENAMED1".to_owned());
            doc.update_board(&renamed)
                .unwrap_or_else(|e| panic!("{name}: rename: {e}"));
            let reread = doc.board().unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(reread.components[i].designator, "RENAMED1", "{name}");
        }
        boards_round_tripped += 1;
    }
    eprintln!(
        "corpus_r8_designators: {without_source} components without SOURCEDESIGNATOR; \
         {boards_round_tripped} boards written and re-read"
    );
    assert_eq!(without_source, 603);
    assert_eq!(boards_round_tripped, 79);
}

/// R12: component kinds (corpus: 409 StandardNoBom, 24 NetTieNoBom, the rest
/// Standard) read on every board and survive `update_board` with the kind keys
/// of every Components6 record unchanged.
#[test]
fn corpus_r12_component_kinds() {
    use altium_format_types::ComponentKind;
    let mut counts: std::collections::BTreeMap<String, usize> = Default::default();
    let mut boards_round_tripped = 0;
    let kind_keys = |doc: &PcbDoc| -> Vec<Vec<(String, String)>> {
        find_param_section(doc, ParamSectionKind::Components6)
            .unwrap_or(&[])
            .iter()
            .map(|r| {
                r.params
                    .iter()
                    .filter(|(k, _)| k.starts_with("COMPONENTKIND"))
                    .map(|(k, v)| (k.to_owned(), v.to_owned()))
                    .collect()
            })
            .collect()
    };
    for (name, mut doc) in corpus_docs() {
        let ctx = build_context(&doc).unwrap_or_else(|e| panic!("{name}: {e}"));
        for c in convert_components(&doc, &ctx).unwrap_or_else(|e| panic!("{name}: {e}")) {
            *counts.entry(format!("{:?}", c.kind)).or_default() += 1;
        }
        let Ok(board) = doc.board() else {
            assert!(BOARDS_FAILING_AT_BASE.contains(&name.as_str()), "{name}");
            continue;
        };
        let before = kind_keys(&doc);
        doc.update_board(&board)
            .unwrap_or_else(|e| panic!("{name}: update_board: {e}"));
        assert_eq!(kind_keys(&doc), before, "{name}: kind keys changed");
        let after = doc.board().unwrap_or_else(|e| panic!("{name}: {e}"));
        let kinds = |b: &PcbDocBoard| -> Vec<ComponentKind> {
            b.components.iter().map(|c| c.kind).collect()
        };
        assert_eq!(kinds(&after), kinds(&board), "{name}");
        // Changing a kind writes the corpus forms; Jumper fails before writing.
        if !board.components.is_empty() {
            for (kind, keys) in [
                (ComponentKind::NetTieNoBom, vec![("COMPONENTKIND", "4")]),
                (
                    ComponentKind::StandardNoBom,
                    vec![("COMPONENTKIND", "0"), ("COMPONENTKINDVERSION2", "5")],
                ),
                (ComponentKind::Standard, vec![]),
            ] {
                let mut changed = board.clone();
                changed.components[0].kind = kind;
                doc.update_board(&changed)
                    .unwrap_or_else(|e| panic!("{name}: {kind:?}: {e}"));
                let expected: Vec<(String, String)> = keys
                    .iter()
                    .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                    .collect();
                assert_eq!(kind_keys(&doc)[0], expected, "{name}: {kind:?}");
                assert_eq!(doc.board().unwrap().components[0].kind, kind, "{name}");
            }
            let before = kind_keys(&doc);
            let mut jumper = board.clone();
            jumper.components[0].kind = ComponentKind::Jumper;
            assert!(doc.update_board(&jumper).is_err(), "{name}: Jumper");
            assert_eq!(
                kind_keys(&doc),
                before,
                "{name}: Jumper changed the document"
            );
        }
        boards_round_tripped += 1;
    }
    eprintln!("corpus_r12_component_kinds: {counts:?}; {boards_round_tripped} boards written");
    assert_eq!(counts.get("StandardNoBom"), Some(&409));
    assert_eq!(counts.get("NetTieNoBom"), Some(&24));
    assert_eq!(counts.get("Standard"), Some(&(14_409 - 433)));
    assert_eq!(boards_round_tripped, 79);
}

/// R16: stored violations. Every rule and object index is in range, and the
/// objects sit where the violation says: a layered object named by a violation
/// is on the violation's layer, except for kinds whose layer can belong to the
/// other object.
#[test]
fn corpus_r16_violations() {
    use std::collections::BTreeMap;
    let mut stats: BTreeMap<String, usize> = BTreeMap::new();
    let mut bump = |k: String| *stats.entry(k).or_default() += 1;
    let mut write_checked = false;
    let mut saved = 0;
    for (name, mut doc) in corpus_docs() {
        if BOARDS_FAILING_AT_BASE.contains(&name.as_str()) {
            continue;
        }
        let board = doc.board().unwrap_or_else(|e| panic!("{name}: {e:#}"));
        let write_now = !board.violations.is_empty();
        let stored: usize = doc.violations.values().map(Vec::len).sum();
        assert_eq!(board.violations.len(), stored, "{name}");
        assert_eq!(
            board.waived_violations.len(),
            doc.waived_violations.len(),
            "{name}"
        );
        for (i, v) in board.violations.iter().enumerate() {
            assert!(v.rule_index < board.rules.len(), "{name} violation #{i}");
            for o in &v.objects {
                let len = match o.kind {
                    PcbObjectId::Track => board.tracks.len(),
                    PcbObjectId::Arc => board.arcs.len(),
                    PcbObjectId::Via => board.vias.len(),
                    PcbObjectId::Pad => board.pads.len(),
                    PcbObjectId::Fill => board.fills.len(),
                    PcbObjectId::Text => board.texts.len(),
                    PcbObjectId::Region => board.regions.len(),
                    PcbObjectId::Polygon => board.polygons.len(),
                    PcbObjectId::Component => board.components.len(),
                    PcbObjectId::Net => board.nets.len(),
                    PcbObjectId::DifferentialPair => board.differential_pairs.len(),
                    PcbObjectId::ComponentBody => board.component_bodies.len(),
                    PcbObjectId::Board => 1,
                    k => panic!("{name} violation #{i}: object kind {k:?}"),
                };
                let in_range = o.index < len;
                bump(format!("{:?} in_range={in_range}", o.kind));
                if in_range && v.layer.is_some() {
                    let layer = match o.kind {
                        PcbObjectId::Track => Some(board.tracks[o.index].layer.clone()),
                        PcbObjectId::Arc => Some(board.arcs[o.index].layer.clone()),
                        PcbObjectId::Fill => Some(board.fills[o.index].layer.clone()),
                        PcbObjectId::Region => Some(board.regions[o.index].layer.clone()),
                        PcbObjectId::Text => Some(board.texts[o.index].layer.clone()),
                        _ => None,
                    };
                    if let Some(l) = layer {
                        let same = Some(l.legacy_v6()) == v.layer.as_ref().map(LayerRef::legacy_v6);
                        bump(format!("{:?} layer_match={same}", o.kind));
                    }
                    if o.kind == PcbObjectId::Region {
                        bump(format!(
                            "Region polygon={} kind={:?}",
                            board.regions[o.index].polygon.is_some(),
                            board.regions[o.index].kind
                        ));
                    }
                }
            }
        }
        for v in &board.violations {
            let Some(vl) = v.layer.as_ref().map(LayerRef::legacy_v6) else {
                continue;
            };
            let layers: Vec<Option<V6Layer>> = v
                .objects
                .iter()
                .filter_map(|o| match o.kind {
                    PcbObjectId::Track => Some(&board.tracks[o.index].layer),
                    PcbObjectId::Arc => Some(&board.arcs[o.index].layer),
                    PcbObjectId::Fill => Some(&board.fills[o.index].layer),
                    PcbObjectId::Region => Some(&board.regions[o.index].layer),
                    PcbObjectId::Text => Some(&board.texts[o.index].layer),
                    _ => None,
                })
                .map(LayerRef::legacy_v6)
                .collect();
            if !layers.is_empty() && !layers.contains(&vl) {
                // Two-object kinds whose layer can be the other object's
                // (a pad, via or the board edge) in the corpus.
                assert!(
                    matches!(
                        v.kind,
                        ViolationKind::BoardOutlineClearance
                            | ViolationKind::Clearance
                            | ViolationKind::ShortCircuit
                    ),
                    "{name}: {:?} names no object on its layer {vl:?}",
                    v.kind
                );
                bump(format!(
                    "no layered object on violation layer: {:?}",
                    v.kind
                ));
            }
        }
        for (i, w) in board.waived_violations.iter().enumerate() {
            assert!(w.rule_index < board.rules.len(), "{name} waived #{i}");
            for o in &w.objects {
                bump(format!("waived {:?}", o.kind));
            }
        }
        if write_now {
            r16_write_checks(&name, &mut doc, &board, &mut write_checked, &mut saved);
        }
    }
    for (k, n) in &stats {
        eprintln!("{k}: {n}");
    }
    assert!(
        stats.keys().all(|k| !k.contains("in_range=false")),
        "{stats:?}"
    );
    assert_eq!(stats.get("Pad in_range=true"), Some(&1418), "{stats:?}");
    assert_eq!(stats.get("waived Pad"), Some(&745), "{stats:?}");
    assert!(write_checked);
    assert!(saved > 0, "no cleared board saved and reopened");
}

/// `update_board` keeps unchanged stored violations, rejects edited ones and
/// kept ones whose indexed lists changed length (leaving the document as it
/// was), and clears emptied lists. The edit checks run once (on a board with
/// waived violations too); every board is cleared, saved and reopened, and
/// the reopen must succeed on some (it fails on boards that hit the known
/// component body writer gaps: ShapeBasedComponentBodies6 framing, and
/// ComponentBodies6 `%UTF8%MODEL.NAME`).
fn r16_write_checks(
    name: &str,
    doc: &mut PcbDoc,
    board: &PcbDocBoard,
    edits_checked: &mut bool,
    saved: &mut usize,
) {
    doc.update_board(board)
        .unwrap_or_else(|e| panic!("{name}: update_board: {e:#}"));
    let again = doc.board().unwrap();
    assert_eq!(again.violations, board.violations, "{name}");
    assert_eq!(again.waived_violations, board.waived_violations, "{name}");

    if !*edits_checked && !board.waived_violations.is_empty() {
        *edits_checked = true;
        let mut edited = board.clone();
        edited.violations[0].description.push('x');
        let err = doc.update_board(&edited).unwrap_err();
        assert!(err.to_string().contains("read-only"), "{name}: {err}");
        let mut edited = board.clone();
        edited.waived_violations[0].comment.push('x');
        assert!(doc.update_board(&edited).is_err(), "{name}");
        let mut shrunk = board.clone();
        let removed = board
            .violations
            .iter()
            .flat_map(|v| &v.objects)
            .find_map(|o| match o.kind {
                PcbObjectId::Track => shrunk.tracks.pop().map(|_| ()),
                PcbObjectId::Pad => shrunk.pads.pop().map(|_| ()),
                PcbObjectId::Via => shrunk.vias.pop().map(|_| ()),
                _ => None,
            });
        assert!(removed.is_some(), "{name}: no track, pad or via violation");
        let err = doc.update_board(&shrunk).unwrap_err();
        assert!(
            err.to_string().contains("empty the violation"),
            "{name}: {err}"
        );
        assert_eq!(doc.board().unwrap().violations, board.violations, "{name}");
    }

    let mut cleared = board.clone();
    cleared.violations.clear();
    cleared.waived_violations.clear();
    doc.update_board(&cleared)
        .unwrap_or_else(|e| panic!("{name}: update_board (cleared): {e:#}"));
    let after = doc.board().unwrap();
    assert!(after.violations.is_empty(), "{name}");
    assert!(after.waived_violations.is_empty(), "{name}");
    let tmp = tempfile::NamedTempFile::new().unwrap();
    doc.save(tmp.path())
        .unwrap_or_else(|e| panic!("{name}: save: {e:#}"));
    match PcbDoc::open(tmp.path()) {
        Ok(reopened) => {
            let reopened = reopened.board().unwrap_or_else(|e| panic!("{name}: {e:#}"));
            assert!(reopened.violations.is_empty(), "{name}");
            assert!(reopened.waived_violations.is_empty(), "{name}");
            *saved += 1;
        }
        Err(e) => assert!(
            format!("{e:?}").contains("ComponentBodies6"),
            "{name}: reopen: {e:#}"
        ),
    }
}

/// R17: clearance rules carry their object matrix (counts from a Python scan
/// of the Rules6 streams of the boards checked here).
#[test]
fn corpus_r17_clearance_matrix() {
    let (mut rules, mut with_matrix, mut entries) = (0, 0, 0);
    for (name, doc) in corpus_docs() {
        if BOARDS_FAILING_AT_BASE.contains(&name.as_str()) {
            continue;
        }
        let board = doc.board().unwrap_or_else(|e| panic!("{name}: {e:#}"));
        for rule in &board.rules {
            if let RuleParams::Clearance {
                object_clearances, ..
            } = &rule.params
            {
                rules += 1;
                with_matrix += usize::from(!object_clearances.is_empty());
                entries += object_clearances.len();
                for (i, a) in object_clearances.iter().enumerate() {
                    assert!((a.first as u8) <= (a.second as u8), "{name}: {a:?}");
                    assert!(
                        object_clearances[..i]
                            .iter()
                            .all(|b| (b.first, b.second) != (a.first, a.second)),
                        "{name}: {a:?} twice"
                    );
                }
            }
        }
    }
    assert_eq!((rules, with_matrix, entries), (145, 65, 944));
}
