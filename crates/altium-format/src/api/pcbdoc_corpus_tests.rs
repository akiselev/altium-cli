//! Corpus checks for the PcbDoc board API (`data/pcbdoc`, a clone of
//! akiselev/altium-cli-test-pcbdoc at `6bc5aa7`).
//!
//! Run one at a time, filtered by name, e.g.
//! `cargo test -p altium-format --features test-fixtures corpus_board_reads`.

use std::path::{Path, PathBuf};

use super::*;
use crate::api::pcbdoc_write::COMPONENT_DEFAULT_ONLY_KEYS;
use altium_format_types::pcb::{TCacheState, TextKind};

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

/// Every board that opens reads. At the base rev the 15 boards in
/// [`BOARDS_FAILING_AT_BASE`] failed on Dimensions6 `TEXTX` (stored as
/// `<decimal>mil`, read as an integer); batch 5 (R23) reads them. The other
/// corpus tests still skip those 15 so their counts stay comparable with R1–R17.
#[test]
fn corpus_board_reads() {
    let mut ok = 0;
    let mut dimensions = 0;
    for (name, doc) in corpus_docs() {
        let board = doc.board().unwrap_or_else(|e| panic!("{name}: {e:#}"));
        dimensions += board.dimensions.len();
        ok += 1;
    }
    eprintln!("corpus_board_reads: {ok} of 94 boards read, {dimensions} dimensions");
    assert_eq!(ok, 94);
    assert_eq!(dimensions, 92);
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
        // The R1–R17 checks stay on the boards that read at the base rev (see
        // corpus_board_reads); the 15 boards batch 5 made readable are listed open.
        if BOARDS_FAILING_AT_BASE.contains(&name.as_str()) {
            continue;
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
        // The R1–R17 checks stay on the boards that read at the base rev (see
        // corpus_board_reads); the 15 boards batch 5 made readable are listed open.
        if BOARDS_FAILING_AT_BASE.contains(&name.as_str()) {
            continue;
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
        // The R1–R17 checks stay on the boards that read at the base rev (see
        // corpus_board_reads); the 15 boards batch 5 made readable are listed open.
        if BOARDS_FAILING_AT_BASE.contains(&name.as_str()) {
            continue;
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
        // The R1–R17 checks stay on the boards that read at the base rev (see
        // corpus_board_reads); the 15 boards batch 5 made readable are listed open.
        if BOARDS_FAILING_AT_BASE.contains(&name.as_str()) {
            continue;
        }
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
        // The R1–R17 checks stay on the boards that read at the base rev (see
        // corpus_board_reads); the 15 boards batch 5 made readable are listed open.
        if BOARDS_FAILING_AT_BASE.contains(&name.as_str()) {
            continue;
        }
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
            // Stored violations index pads; with a pad inserted they must be
            // cleared (R16).
            inserted.violations.clear();
            inserted.waived_violations.clear();
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
        // The R1–R17 checks stay on the boards that read at the base rev (see
        // corpus_board_reads); the 15 boards batch 5 made readable are listed open.
        if BOARDS_FAILING_AT_BASE.contains(&name.as_str()) {
            continue;
        }
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
        // The R1–R17 checks stay on the boards that read at the base rev (see
        // corpus_board_reads); the 15 boards batch 5 made readable are listed open.
        if BOARDS_FAILING_AT_BASE.contains(&name.as_str()) {
            continue;
        }
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
        // The R1–R17 checks stay on the boards that read at the base rev (see
        // corpus_board_reads); the 15 boards batch 5 made readable are listed open.
        if BOARDS_FAILING_AT_BASE.contains(&name.as_str()) {
            continue;
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

/// R17: clearance and board-outline clearance rules carry their object matrix (counts from a Python scan
/// of the Rules6 streams of the boards checked here).
#[test]
fn corpus_r17_clearance_matrix() {
    // (rules, rules with a matrix, matrix entries)
    let mut clearance_counts = (0, 0, 0);
    let mut outline_counts = (0, 0, 0);
    for (name, doc) in corpus_docs() {
        if BOARDS_FAILING_AT_BASE.contains(&name.as_str()) {
            continue;
        }
        let board = doc.board().unwrap_or_else(|e| panic!("{name}: {e:#}"));
        for rule in &board.rules {
            let (RuleParams::Clearance {
                object_clearances, ..
            }
            | RuleParams::BoardOutlineClearance {
                object_clearances, ..
            }) = &rule.params
            else {
                continue;
            };
            {
                let outline = matches!(rule.params, RuleParams::BoardOutlineClearance { .. });
                let counts = if outline {
                    &mut outline_counts
                } else {
                    &mut clearance_counts
                };
                counts.0 += 1;
                counts.1 += usize::from(!object_clearances.is_empty());
                counts.2 += object_clearances.len();
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
    assert_eq!(clearance_counts, (145, 65, 944));
    assert_eq!(outline_counts, (46, 18, 183));
}

/// R13: plane nets from Board6 `PLANEnNETNAME`, and the writer stores edits.
#[test]
fn corpus_r13_plane_nets() {
    let docs = corpus_docs();
    let board = |name: &str| {
        let (_, doc) = docs.iter().find(|(n, _)| n == name).unwrap();
        doc.board().unwrap_or_else(|e| panic!("{name}: {e:#}"))
    };
    let planes = |b: &PcbDocBoard| -> Vec<(String, Option<String>, bool)> {
        b.settings
            .layer_stack
            .layers
            .iter()
            .filter(|l| l.is_plane)
            .map(|l| (l.name.clone(), l.plane_net.clone(), l.plane_multiple_nets))
            .collect()
    };
    let s = |v: &str| Some(v.to_owned());
    assert_eq!(
        planes(&board("mini-itx-motherboard.PcbDoc")),
        [
            ("Ground Plane 1 (GND)".to_owned(), None, true),
            ("Power Plane (12V)".to_owned(), None, true),
            ("Ground Plane 2 (GND)".to_owned(), s("GND"), false),
        ]
    );
    assert_eq!(
        planes(&board("textbook-smart-mcu.PcbDoc")),
        [
            ("GND_layer".to_owned(), s("GND"), false),
            ("3.3V_layer".to_owned(), s("+3.3V"), false),
        ]
    );
    // The only stack plane is plane 2; plane 1's key is stale.
    assert_eq!(
        planes(&board("rover-24v-buck.PcbDoc")),
        [("GND".to_owned(), s("GND"), false)]
    );
    // Plane nets with no planes in the stack (stale keys) do not fail.
    assert!(planes(&board("rover-bms.PcbDoc")).is_empty());
    for name in [
        "textbook-tiny-arduino.PcbDoc",
        "thesis-cubesat-4layer-template.PcbDoc",
    ] {
        assert!(
            planes(&board(name)).iter().all(|p| p.1.is_none() && !p.2),
            "{name}"
        );
    }

    let mut total = (0, 0, 0);
    for (name, doc) in &docs {
        if BOARDS_FAILING_AT_BASE.contains(&name.as_str()) {
            continue;
        }
        for p in planes(&doc.board().unwrap()) {
            total.0 += 1;
            total.1 += usize::from(p.1.is_some());
            total.2 += usize::from(p.2);
        }
    }
    eprintln!("planes, with a net, with multiple nets: {total:?}");

    // Write: unchanged keeps the stored key byte for byte; an edit is stored;
    // an unknown net is refused.
    let (_, doc) = docs
        .into_iter()
        .find(|(n, _)| n == "textbook-smart-mcu.PcbDoc")
        .unwrap();
    let mut doc = doc;
    let b = doc.board().unwrap();
    let key16 = |doc: &PcbDoc| {
        find_param_section(doc, ParamSectionKind::Board6).unwrap()[0]
            .params
            .get("PLANE16NETNAME")
            .map(str::to_owned)
    };
    let before = key16(&doc);
    doc.update_board(&b).unwrap();
    assert_eq!(key16(&doc), before);
    assert_eq!(planes(&doc.board().unwrap()), planes(&b));
    let mut edited = b.clone();
    let plane = edited
        .settings
        .layer_stack
        .layers
        .iter_mut()
        .find(|l| l.is_plane && l.plane_net.as_deref() == Some("+3.3V"))
        .unwrap();
    plane.plane_net = None;
    plane.plane_multiple_nets = true;
    let position = |doc: &PcbDoc, key: &str| {
        find_param_section(doc, ParamSectionKind::Board6).unwrap()[0]
            .params
            .iter()
            .position(|(k, _)| k == key)
    };
    let p2 = position(&doc, "PLANE2NETNAME");
    doc.update_board(&edited).unwrap();
    assert_eq!(
        planes(&doc.board().unwrap())[1],
        ("3.3V_layer".to_owned(), None, true)
    );
    assert_eq!(
        position(&doc, "PLANE2NETNAME"),
        p2,
        "edited key stays in its block"
    );
    let mut restacked = b.clone();
    restacked.settings.layer_stack.layers[0].name.push('x');
    let err = doc.update_board(&restacked).unwrap_err();
    assert!(format!("{err:#}").contains("layer stack"), "{err:#}");
    let mut bad = b.clone();
    bad.settings
        .layer_stack
        .layers
        .iter_mut()
        .find(|l| l.is_plane)
        .unwrap()
        .plane_net = s("NO_SUCH_NET");
    let err = doc.update_board(&bad).unwrap_err();
    assert!(format!("{err:#}").contains("NO_SUCH_NET"), "{err:#}");
}

/// R15: region PADINDEX, SUBPOLYINDEX and OBJECTKIND (BoardRegion) are typed,
/// read the same from both region sections, and survive `update_board`.
#[test]
fn corpus_r15_region_indices() {
    let (mut with_pad, mut with_subpoly, mut pour_subpoly, mut board_regions) = (0, 0, 0, 0);
    let mut edit_checked = false;
    for (name, mut doc) in corpus_docs() {
        if BOARDS_FAILING_AT_BASE.contains(&name.as_str()) {
            continue;
        }
        let board = doc.board().unwrap_or_else(|e| panic!("{name}: {e:#}"));
        for r in &board.regions {
            with_pad += usize::from(r.pad_index.is_some());
            with_subpoly += usize::from(r.subpoly_index.is_some());
            pour_subpoly += usize::from(r.subpoly_index.is_some() && r.polygon.is_some());
            board_regions += usize::from(r.board_region);
            if r.polygon.is_some() {
                assert!(
                    r.subpoly_index.is_some(),
                    "{name}: pour region without a subpoly index"
                );
            }
        }
        let key = |b: &PcbDocBoard| -> Vec<_> {
            b.regions
                .iter()
                .map(|r| (r.pad_index, r.subpoly_index, r.board_region))
                .collect()
        };
        let Some(i) = board.regions.iter().position(|r| r.pad_index.is_some()) else {
            continue;
        };
        doc.update_board(&board)
            .unwrap_or_else(|e| panic!("{name}: update_board: {e:#}"));
        assert_eq!(key(&doc.board().unwrap()), key(&board), "{name}");
        if !edit_checked {
            edit_checked = true;
            let mut edited = board.clone();
            edited.regions[i].pad_index = None;
            edited.regions[i].subpoly_index = Some(7);
            doc.update_board(&edited).unwrap();
            let again = doc.board().unwrap();
            assert_eq!(
                (again.regions[i].pad_index, again.regions[i].subpoly_index),
                (None, Some(7))
            );
            let mut bad = board.clone();
            bad.regions[i].pad_index = Some(u32::MAX);
            let err = doc.update_board(&bad).unwrap_err();
            assert!(format!("{err:#}").contains("PADINDEX"), "{name}: {err:#}");
        }
    }
    assert!(edit_checked);
    eprintln!(
        "pad {with_pad}, subpoly {with_subpoly} ({pour_subpoly} in pours), board {board_regions}"
    );
    assert_eq!((board_regions, with_pad, pour_subpoly), (79, 25, 43631));
}

/// R14: text style and designator/comment visibility are read, survive
/// `update_board` together with the Components6 keys the API does not carry,
/// and an edit is written.
#[test]
fn corpus_r14_text_style() {
    let style = |b: &PcbDocBoard| -> Vec<String> {
        b.texts
            .iter()
            .map(|t| {
                format!(
                    "{:?} {} {} {} {} {:?} {} {:?} {:?} {:?} {}",
                    t.font_kind,
                    t.stroke_font_id,
                    t.bold,
                    t.italic,
                    t.inverted,
                    t.inverted_border,
                    t.use_inverted_rectangle,
                    t.inverted_rect_width,
                    t.inverted_rect_height,
                    t.justification,
                    t.hidden
                )
            })
            .collect()
    };
    let stored = |doc: &PcbDoc| -> Vec<Vec<(String, String)>> {
        find_param_section(doc, ParamSectionKind::Components6)
            .map(|records| {
                records
                    .iter()
                    .map(|r| {
                        r.params
                            .iter()
                            .filter(|(k, _)| {
                                COMPONENT_DEFAULT_ONLY_KEYS.contains(k)
                                    || matches!(*k, "NAMEON" | "COMMENTON")
                            })
                            .map(|(k, v)| (k.to_owned(), v.to_owned()))
                            .collect()
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
    let (mut texts, mut hidden, mut truetype, mut inverted) = (0, 0, 0, 0);
    let mut edit_checked = false;
    for (name, mut doc) in corpus_docs() {
        if BOARDS_FAILING_AT_BASE.contains(&name.as_str()) {
            continue;
        }
        let board = doc.board().unwrap_or_else(|e| panic!("{name}: {e:#}"));
        for t in &board.texts {
            texts += 1;
            hidden += usize::from(t.hidden);
            truetype += usize::from(t.font_kind == TextKind::TrueTypeFont);
            inverted += usize::from(t.inverted);
        }
        let before = stored(&doc);
        doc.update_board(&board)
            .unwrap_or_else(|e| panic!("{name}: update_board: {e:#}"));
        let again = doc.board().unwrap();
        assert_eq!(style(&again), style(&board), "{name}");
        let after = stored(&doc);
        assert_eq!(after.len(), before.len(), "{name}");
        if let Some(i) = (0..after.len()).find(|&i| after[i] != before[i]) {
            panic!(
                "{name}: Components6 record #{i} keys changed:\n before {:?}\n after  {:?}",
                before[i], after[i]
            );
        }

        if !edit_checked {
            let Some(t) = board
                .texts
                .iter()
                .position(|t| t.is_designator && t.component_index.is_some())
            else {
                continue;
            };
            edit_checked = true;
            let c = board.texts[t].component_index.unwrap();
            let mut edited = board.clone();
            edited.components[c].designator_visible = !board.components[c].designator_visible;
            let err = doc.update_board(&edited).unwrap_err();
            assert!(format!("{err:#}").contains("hidden"), "{name}: {err:#}");
            for text in &mut edited.texts {
                text.hidden = text_hidden_by_component(text, &edited.components);
            }
            edited.texts[t].bold = !board.texts[t].bold;
            edited.texts[t].justification = TextAutoposition::BottomRight;
            doc.update_board(&edited).unwrap();
            let again = doc.board().unwrap();
            assert_eq!(again.texts[t].hidden, !board.texts[t].hidden, "{name}");
            assert_eq!(again.texts[t].bold, !board.texts[t].bold, "{name}");
            assert_eq!(
                again.texts[t].justification,
                TextAutoposition::BottomRight,
                "{name}"
            );
        }
    }
    eprintln!("{texts} texts, {hidden} hidden, {truetype} TrueType, {inverted} inverted");
    assert!(edit_checked && hidden > 0);
}

/// R22: every rule kind in the corpus has a typed `RuleParams` variant; room
/// definitions expose their outline, layer and lock flags; the per-object
/// plane/polygon connect settings and the newly exposed keys read as stored;
/// and rules read the same after a save and reopen.
#[test]
fn corpus_r22_rules() {
    use std::collections::BTreeMap;
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    // (rooms, enabled, vertices, arc edges, locked, bottom layer)
    let mut rooms = (0, 0, 0, 0, 0, 0);
    let mut plane_per_object = 0;
    let mut polygon_per_object = 0;
    let mut stacked_microvias = 0;
    let mut exposed_copper = 0;
    let mut without_3d_body = 0;
    let mut via_grids = 0;
    let mut all_files_rules = 0;
    for (name, doc) in corpus_docs() {
        // Rules6 is parsed by `open`, so every openable file's rules are checked
        // here, including the boards whose `board()` fails elsewhere.
        for (i, rule) in doc.rules.iter().enumerate() {
            all_files_rules += 1;
            let params = rule_params_from_internal(rule)
                .unwrap_or_else(|e| panic!("{name}: rule #{i}: {e:#}"));
            assert!(
                !matches!(params, RuleParams::Other { .. }),
                "{name}: rule #{i} untyped"
            );
        }
        if BOARDS_FAILING_AT_BASE.contains(&name.as_str()) {
            continue;
        }
        let board = doc.board().unwrap_or_else(|e| panic!("{name}: {e:#}"));
        for rule in &board.rules {
            let variant = format!("{:?}", rule.params);
            let variant = variant
                .split(|c: char| !c.is_alphanumeric())
                .next()
                .unwrap()
                .to_owned();
            *kinds.entry(variant).or_default() += 1;
            match &rule.params {
                RuleParams::Other { kind } => panic!("{name}: rule kind {kind:?} untyped"),
                RuleParams::ConfinementConstraint {
                    lock_components,
                    format_copy,
                    layer,
                    outline,
                    ..
                } => {
                    rooms.0 += 1;
                    rooms.1 += usize::from(rule.enabled);
                    rooms.2 += outline.segments.len();
                    rooms.3 += outline
                        .segments
                        .iter()
                        .filter(|s| matches!(s, ContourSegment::Arc { .. }))
                        .count();
                    rooms.4 += usize::from(*lock_components);
                    rooms.5 += usize::from(layer.legacy_v6() == Some(V6Layer::BottomLayer));
                    assert!(*format_copy, "{name}: {}", rule.name);
                    let pts = outline.to_points();
                    assert!(pts.len() >= 4, "{name}: {}", rule.name);
                    assert_eq!(pts.first(), pts.last(), "{name}: {} not closed", rule.name);
                }
                RuleParams::PowerPlaneConnectStyle {
                    connect_style,
                    per_object,
                    ..
                } => {
                    plane_per_object += usize::from(!per_object.is_empty());
                    assert_eq!(connect_style.is_none(), !per_object.is_empty(), "{name}");
                }
                RuleParams::PolygonConnectStyle {
                    connect_style,
                    per_object,
                    ..
                } => {
                    polygon_per_object += usize::from(!per_object.is_empty());
                    assert_eq!(connect_style.is_none(), !per_object.is_empty(), "{name}");
                }
                RuleParams::HoleToHoleClearance {
                    allow_stacked_microvias,
                    ..
                } => stacked_microvias += usize::from(*allow_stacked_microvias),
                RuleParams::SilkToSolderMaskClearance {
                    clearance_to_exposed_copper,
                    ..
                } => exposed_copper += usize::from(*clearance_to_exposed_copper),
                RuleParams::ComponentClearance {
                    do_not_check_without_3d_body,
                    ..
                } => without_3d_body += usize::from(*do_not_check_without_3d_body == Some(true)),
                RuleParams::FanoutControl { via_grid, .. } => {
                    via_grids += usize::from(*via_grid == Coord::from_mils_f64(1.0))
                }
                _ => {}
            }
        }
        // Rules are written from the typed model on save: every record must
        // serialize, re-parse and convert to the same public rule.
        for (i, rule) in doc.rules.iter().enumerate() {
            let rec = crate::pcbdoc::drc::serialize_rule(rule).unwrap();
            let mut params = rec.params.clone();
            let again = crate::pcbdoc::drc::parse_rule(rec.prefix, &mut params)
                .unwrap_or_else(|e| panic!("{name}: rule #{i} re-parse: {e:#}"));
            assert_eq!(
                format!("{:?}", rule_params_from_internal(rule).unwrap()),
                format!("{:?}", rule_params_from_internal(&again).unwrap()),
                "{name}: rule #{i} changed by write/read"
            );
        }
    }
    let expected: BTreeMap<String, usize> = [
        ("AcuteAngle", 10),
        ("AssemblyTestpointStyle", 79),
        ("AssemblyTestpointUsage", 80),
        ("BoardOutlineClearance", 46),
        ("BrokenNets", 79),
        ("Clearance", 145),
        ("ComponentClearance", 88),
        ("ConfinementConstraint", 94),
        ("CreepageDistance", 1),
        ("DiffPairsRouting", 99),
        ("FabricationTestpointStyle", 79),
        ("FabricationTestpointUsage", 79),
        ("FanoutControl", 396),
        ("HoleToHoleClearance", 79),
        ("LayerPair", 79),
        ("Length", 2),
        ("MatchedLengths", 78),
        ("MaxMinHeight", 80),
        ("MaxMinHoleSize", 80),
        ("MinimumAnnularRing", 40),
        ("MinimumSolderMaskSliver", 79),
        ("NetAntennae", 79),
        ("PasteMaskExpansion", 79),
        ("PolygonConnectStyle", 127),
        ("PowerPlaneClearance", 86),
        ("PowerPlaneConnectStyle", 96),
        ("ReturnPath", 2),
        ("RoutingCornerStyle", 79),
        ("RoutingLayers", 81),
        ("RoutingPriority", 79),
        ("RoutingTopology", 94),
        ("RoutingViaStyle", 85),
        ("ShortCircuit", 80),
        ("SignalMaximum", 2),
        ("SilkToBoardRegionClearance", 79),
        ("SilkToSilkClearance", 79),
        ("SilkToSolderMaskClearance", 79),
        ("SmdEntry", 2),
        ("SmdNeckDown", 1),
        ("SmdToCorner", 2),
        ("SmdToPlane", 1),
        ("SolderMaskExpansion", 91),
        ("SupplyNets", 39),
        ("UnconnectedPin", 4),
        ("UnpouredPolygon", 79),
        ("ViasUnderSmd", 4),
        ("Width", 116),
    ]
    .into_iter()
    .map(|(k, n)| (k.to_owned(), n))
    .collect();
    assert_eq!(kinds, expected);
    // 94 rooms (59 enabled), 823 vertices, all line edges, none locked.
    assert_eq!(rooms, (94, 59, 823, 0, 0, 0));
    assert_eq!((plane_per_object, polygon_per_object), (17, 37));
    assert_eq!(
        (stacked_microvias, exposed_copper, without_3d_body),
        (56, 79, 4)
    );
    assert_eq!(via_grids, 396);
    assert_eq!(all_files_rules, 4168, "rules in all 94 openable files");
}

/// R21 and Q15: classes read strictly (kind, superclass, Unicode members) and
/// differential pairs read their stored net keys; `update_board` keeps both,
/// matches classes by name and kind, and drops stale member keys.
#[test]
fn corpus_r21_classes_and_pairs() {
    let mut pad_classes = (0, 0); // (pad classes, with members)
    let mut unicode_members = 0;
    let mut pairs = 0;
    let mut docs = corpus_docs();
    for (name, doc) in &mut docs {
        if BOARDS_FAILING_AT_BASE.contains(&name.as_str()) {
            continue;
        }
        let board = doc.board().unwrap_or_else(|e| panic!("{name}: {e:#}"));
        let net_names: std::collections::HashSet<&str> =
            board.nets.iter().map(|n| n.name.as_str()).collect();
        for c in &board.classes {
            if c.kind == altium_format_types::pcb::ClassMemberKind::Pad {
                pad_classes.0 += 1;
                pad_classes.1 += usize::from(!c.members.is_empty());
                assert!(c.super_class, "{name}: user pad class {}", c.name);
            }
            unicode_members += c.members.iter().filter(|m| !m.is_ascii()).count();
        }
        for dp in &board.differential_pairs {
            pairs += 1;
            assert!(
                net_names.contains(dp.positive_net.as_str())
                    && net_names.contains(dp.negative_net.as_str()),
                "{name}: pair {} nets {} / {}",
                dp.name,
                dp.positive_net,
                dp.negative_net
            );
        }
        let classes_of = |b: &PcbDocBoard| format!("{:?}", b.classes);
        let pairs_of = |b: &PcbDocBoard| format!("{:?}", b.differential_pairs);
        let stored_pairs = |doc: &PcbDoc| {
            find_param_section(doc, ParamSectionKind::DifferentialPairs6).map(|r| {
                r.iter()
                    .map(|r| {
                        r.params
                            .iter()
                            .map(|(k, v)| format!("{k}={v}"))
                            .collect::<Vec<_>>()
                            .join("|")
                    })
                    .collect::<Vec<_>>()
            })
        };
        let before = stored_pairs(doc);
        doc.update_board(&board)
            .unwrap_or_else(|e| panic!("{name}: {e:#}"));
        let after = doc.board().unwrap();
        // ensure_standard_classes adds All Components / All Nets when absent.
        if board.classes.iter().any(|c| c.name == "All Components")
            && board.classes.iter().any(|c| c.name == "All Nets")
        {
            assert_eq!(classes_of(&board), classes_of(&after), "{name}: classes");
        }
        assert_eq!(pairs_of(&board), pairs_of(&after), "{name}: pairs");
        assert_eq!(before, stored_pairs(doc), "{name}: stored pair records");
    }
    // No populated user pad class: 79 `All Pads` superclasses, one storing a member.
    assert_eq!(pad_classes, (79, 1));
    assert_eq!((unicode_members, pairs), (36, 716));

    // A class with a Unicode member: unchanged, its record (placeholder bytes +
    // UNICODE__Mn sidecars) is kept byte for byte; changed while a member is
    // still outside Windows-1252, it is refused; changed to ASCII-only members,
    // the stale Mn and sidecar keys are dropped and the members read back.
    let (name, doc) = docs
        .iter_mut()
        .filter(|(n, _)| !BOARDS_FAILING_AT_BASE.contains(&n.as_str()))
        .find(|(_, d)| {
            d.board()
                .unwrap()
                .classes
                .iter()
                .any(|c| c.members.iter().any(|m| !m.is_ascii()))
        })
        .expect("a class with a Unicode member");
    let b = doc.board().unwrap();
    let c = b
        .classes
        .iter()
        .find(|c| c.members.iter().any(|m| !m.is_ascii()))
        .unwrap()
        .clone();
    let stored = |doc: &PcbDoc| -> Vec<String> {
        find_param_section(doc, ParamSectionKind::Classes6)
            .unwrap()
            .iter()
            .find(|r| r.params.get("NAME") == Some(c.name.as_str()))
            .unwrap()
            .params
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect()
    };
    let before = stored(doc);
    doc.update_board(&b).unwrap();
    assert_eq!(
        stored(doc),
        before,
        "{name}: unchanged Unicode class rewritten"
    );
    let mut grown = b.clone();
    grown
        .classes
        .iter_mut()
        .find(|k| k.name == c.name && k.kind == c.kind)
        .unwrap()
        .members
        .push("NEW1".to_owned());
    let err = doc.update_board(&grown).unwrap_err();
    assert!(format!("{err:#}").contains("Windows-1252"), "{err:#}");
    let mut ascii = b.clone();
    let k = ascii
        .classes
        .iter_mut()
        .find(|k| k.name == c.name && k.kind == c.kind)
        .unwrap();
    k.members = k
        .members
        .iter()
        .enumerate()
        .map(|(i, m)| {
            if m.is_ascii() {
                m.clone()
            } else {
                format!("U{i}")
            }
        })
        .collect();
    k.members.pop();
    let expected = k.members.clone();
    doc.update_board(&ascii).unwrap();
    let after = stored(doc);
    assert!(
        after
            .iter()
            .all(|kv| !kv.to_ascii_uppercase().starts_with("UNICODE")),
        "{name}: {after:?}"
    );
    let got = doc.board().unwrap();
    let got = got
        .classes
        .iter()
        .find(|k| k.name == c.name && k.kind == c.kind)
        .unwrap();
    assert_eq!(got.members, expected, "{name}");
}
