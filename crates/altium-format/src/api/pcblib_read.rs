//! Read path: convert internal PcbLib types → public API types.

use crate::api::pcb_common::{contour_to_pcb_contour, extract_pad_stack, mask_mode_from_cache};
use crate::api::pcblib_types::*;
use crate::pcblib::{
    PcbArc, PcbComponentBody, PcbFill, PcbFootprint, PcbPad, PcbPrimitive, PcbRegion, PcbText,
    PcbTrack, PcbVia,
};
use altium_format_types::color::Color;
use altium_format_types::coord::Coord;
use altium_format_types::pcb::{LayerRef, V7Layer};

/// Convert an internal `PcbFootprint` into a public `Footprint`.
pub(crate) fn footprint_from_internal(fp: &PcbFootprint) -> Footprint {
    let mut pads = Vec::new();
    let mut graphics = Vec::new();

    for primitive in &fp.primitives {
        match primitive {
            PcbPrimitive::Pad(p) => {
                pads.push(pad_from_internal(p));
            }
            PcbPrimitive::Track(t) => {
                graphics.push(PcbGraphic::Track(track_from_internal(t)));
            }
            PcbPrimitive::Arc(a) => {
                graphics.push(PcbGraphic::Arc(arc_from_internal(a)));
            }
            PcbPrimitive::Fill(f) => {
                graphics.push(PcbGraphic::Fill(fill_from_internal(f)));
            }
            PcbPrimitive::Region(r) => {
                graphics.push(PcbGraphic::Region(region_from_internal(r)));
            }
            PcbPrimitive::Text(t) => {
                graphics.push(PcbGraphic::Text(text_from_internal(t)));
            }
            PcbPrimitive::Via(v) => {
                graphics.push(PcbGraphic::Via(via_from_internal(v)));
            }
            PcbPrimitive::ComponentBody(b) => {
                graphics.push(PcbGraphic::ComponentBody(body_from_internal(b)));
            }
        }
    }

    Footprint {
        display_name: fp.display_name.clone(),
        description: fp.description.clone(),
        pattern: fp.pattern.clone(),
        height: fp.height,
        pads,
        graphics,
    }
}

fn pad_from_internal(p: &PcbPad) -> Pad {
    let stack = extract_pad_stack(p);
    Pad {
        pad_name: p.pad_name.clone(),
        unique_id: p.unique_id.clone(),
        location: p.location,
        shape: p.shape_top,
        x_size: p.size_top.x,
        y_size: p.size_top.y,
        rotation: p.rotation,
        hole_size: p.hole_size,
        is_plated: p.is_plated,
        layer: LayerRef::from_v6(p.common.layer),
        pad_mode: p.pad_mode,
        solder_mask_expansion: p.cache.solder_mask_expansion,
        paste_mask_expansion: p.cache.paste_mask_expansion,
        solder_mask_mode: mask_mode_from_cache(p.cache.solder_mask_expansion_valid),
        paste_mask_mode: mask_mode_from_cache(p.cache.paste_mask_expansion_valid),
        solder_mask_bottom_expansion: p
            .use_separate_expansions
            .then(|| Coord::from_internal(p.solder_mask_bottom_expansion)),
        solder_mask_from_hole_edge: p.solder_mask_expansion_from_hole_edge,
        plane_connection: p.cache.plane_connection_style,
        relief_conductor_width: p.cache.relief_conductor_width,
        relief_entries: p.cache.relief_entries as i32,
        relief_air_gap: p.cache.relief_air_gap,
        stack,
    }
}

fn track_from_internal(t: &PcbTrack) -> TrackGraphic {
    let layer = if t.layer_enum_index != 0 {
        LayerRef::from_v6_and_v7(t.common.layer, V7Layer::new(t.layer_enum_index as u32))
    } else {
        LayerRef::from_v6(t.common.layer)
    };
    TrackGraphic {
        unique_id: t.unique_id.clone(),
        layer,
        flags: t.common.flags,
        start: t.start,
        end: t.end,
        width: t.width,
    }
}

fn arc_from_internal(a: &PcbArc) -> PcbArcGraphic {
    PcbArcGraphic {
        unique_id: a.unique_id.clone(),
        layer: LayerRef::from_v6_and_v7(a.common.layer, a.v7_layer),
        flags: a.common.flags,
        center: a.center,
        radius: a.radius,
        start_angle: a.start_angle,
        end_angle: a.end_angle,
        width: a.width,
    }
}

fn fill_from_internal(f: &PcbFill) -> FillGraphic {
    FillGraphic {
        unique_id: f.unique_id.clone(),
        layer: LayerRef::from_v6_and_v7(f.common.layer, f.v7_layer),
        flags: f.common.flags,
        corner1: f.corner1,
        corner2: f.corner2,
        rotation: f.rotation,
    }
}

fn region_from_internal(r: &PcbRegion) -> RegionGraphic {
    let layer = if !r.v7_layer.is_empty() {
        LayerRef::from_string_name(&r.v7_layer).unwrap_or_else(|| LayerRef::from_v6(r.common.layer))
    } else {
        LayerRef::from_v6(r.common.layer)
    };
    RegionGraphic {
        unique_id: r.unique_id.clone(),
        layer,
        flags: r.common.flags,
        kind: r.kind,
        outline: contour_to_pcb_contour(&r.outline),
        holes: r.holes.iter().map(contour_to_pcb_contour).collect(),
    }
}

fn text_from_internal(t: &PcbText) -> TextGraphic {
    TextGraphic {
        unique_id: t.unique_id.clone(),
        layer: LayerRef::from_v6(t.common.layer),
        flags: t.common.flags,
        location: t.location,
        text: t.text.clone(),
        rotation: t.rotation,
        height: t.height,
        width: t.stroke_width,
        color: Color::default(),
        font_name: t.font_name.clone(),
        is_mirrored: t.is_mirrored,
    }
}

fn via_from_internal(v: &PcbVia) -> ViaGraphic {
    ViaGraphic {
        unique_id: v.unique_id.clone(),
        layer: LayerRef::from_v6(v.common.layer),
        flags: v.common.flags,
        location: v.location,
        diameter: v.diameter,
        hole_size: v.hole_size,
        from_layer: LayerRef::from_v6(v.from_layer),
        to_layer: LayerRef::from_v6(v.to_layer),
        is_testpoint_top: v.is_testpoint_top,
        is_testpoint_bottom: v.is_testpoint_bottom,
        is_assy_testpoint_top: v.is_assy_testpoint_top,
        is_assy_testpoint_bottom: v.is_assy_testpoint_bottom,
        solder_mask_override: v.solder_mask_override,
        use_separate_solder_mask_expansion: v.use_separate_solder_mask_expansion,
        solder_mask_expansion_from_hole_edge: v.solder_mask_expansion_from_hole_edge,
        paste_mask_override: v.paste_mask_override,
    }
}

fn body_from_internal(b: &PcbComponentBody) -> ComponentBodyGraphic {
    let layer = if !b.v7_layer.is_empty() {
        LayerRef::from_string_name(&b.v7_layer).unwrap_or_else(|| LayerRef::from_v6(b.common.layer))
    } else {
        LayerRef::from_v6(b.common.layer)
    };
    ComponentBodyGraphic {
        unique_id: b.unique_id.clone(),
        layer,
        flags: b.common.flags,
        standoff_height: b.standoff_height,
        overall_height: b.overall_height,
        body_color_3d: b.body_color_3d,
        body_opacity_3d: b.body_opacity_3d,
        model_name: b.model_name.clone(),
        outline: contour_to_pcb_contour(&b.outline),
    }
}

/// R5 corpus check on akiselev/altium-cli-test-pcblib `c5dfdc7` (`data/pcblib`).
#[cfg(all(test, feature = "test-fixtures"))]
mod corpus_tests {
    use crate::pcblib::{PcbLib, PcbPrimitive};
    use altium_format_types::PcbObjectId;
    use altium_format_types::pcb::{MaskExpansionMode, TCacheState};
    use std::path::Path;

    /// Pad modes come from the cache states (no corpus pad has an
    /// ExtendedPrimitiveInformation entry), and `update_footprint` with the
    /// footprint read back keeps every pad's cache states.
    #[test]
    fn corpus_r5_pcblib_pad_modes() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/pcblib");
        let mut paths: Vec<_> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
            .map(|e| e.expect("dir entry").path())
            .filter(|p| {
                p.extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("pcblib"))
            })
            .collect();
        paths.sort();
        let (mut libs, mut pads, mut solder_manual, mut paste_manual) = (0, 0, 0, 0);
        for path in &paths {
            let mut lib = PcbLib::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            libs += 1;
            let states = |lib: &PcbLib| -> Vec<(TCacheState, TCacheState)> {
                lib.footprints
                    .iter()
                    .flat_map(|fp| fp.primitives.iter())
                    .filter_map(|p| match p {
                        PcbPrimitive::Pad(p) => Some((
                            p.cache.solder_mask_expansion_valid,
                            p.cache.paste_mask_expansion_valid,
                        )),
                        _ => None,
                    })
                    .collect()
            };
            for fp in &lib.footprints {
                assert!(
                    fp.extended_primitive_info
                        .iter()
                        .all(|e| e.primitive_object_id != PcbObjectId::Pad),
                    "{}: pad ExtendedPrimitiveInformation entry",
                    fp.display_name
                );
            }
            let before = states(&lib);
            let footprints = lib.footprints();
            let api_pads: Vec<_> = footprints.iter().flat_map(|f| f.pads.iter()).collect();
            assert_eq!(api_pads.len(), before.len());
            for (p, (solder, paste)) in api_pads.iter().zip(&before) {
                pads += 1;
                assert_eq!(
                    p.solder_mask_mode == MaskExpansionMode::Manual,
                    *solder == TCacheState::Manual
                );
                assert_eq!(
                    p.paste_mask_mode == MaskExpansionMode::Manual,
                    *paste == TCacheState::Manual
                );
                solder_manual += usize::from(p.solder_mask_mode == MaskExpansionMode::Manual);
                paste_manual += usize::from(p.paste_mask_mode == MaskExpansionMode::Manual);
            }
            for fp in &footprints {
                lib.update_footprint(fp)
                    .unwrap_or_else(|e| panic!("{}: {}: {e}", path.display(), fp.display_name));
            }
            assert_eq!(
                states(&lib),
                before,
                "{}: pad cache states changed",
                path.display()
            );
        }
        eprintln!(
            "corpus_r5_pcblib_pad_modes: {libs} libraries, {pads} pads, {solder_manual} manual \
             solder mask, {paste_manual} manual paste mask"
        );
        assert_eq!(libs, 40);
        assert_eq!(pads, 80_866);
        assert_eq!(solder_manual, 19_844);
        assert_eq!(paste_manual, 5_954);
    }
}
