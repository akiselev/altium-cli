//! Read path: convert internal PcbLib types → public API types.

use crate::api::pcblib_types::*;
use crate::pcblib::{
    PcbArc, PcbComponentBody, PcbFill, PcbFootprint, PcbPad, PcbPrimitive, PcbRegion, PcbText,
    PcbTrack, PcbVia,
};
use altium_format_types::color::Color;

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
        layer: p.common.layer,
        pad_mode: p.pad_mode,
        solder_mask_expansion: p.cache.solder_mask_expansion,
        paste_mask_expansion: p.cache.paste_mask_expansion,
        plane_connection: p.cache.plane_connection_style,
        relief_conductor_width: p.cache.relief_conductor_width,
        relief_entries: p.cache.relief_entries as i32,
        relief_air_gap: p.cache.relief_air_gap,
    }
}

fn track_from_internal(t: &PcbTrack) -> TrackGraphic {
    TrackGraphic {
        unique_id: t.unique_id.clone(),
        layer: t.common.layer,
        flags: t.common.flags,
        start: t.start,
        end: t.end,
        width: t.width,
    }
}

fn arc_from_internal(a: &PcbArc) -> PcbArcGraphic {
    PcbArcGraphic {
        unique_id: a.unique_id.clone(),
        layer: a.common.layer,
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
        layer: f.common.layer,
        flags: f.common.flags,
        corner1: f.corner1,
        corner2: f.corner2,
        rotation: f.rotation,
    }
}

fn region_from_internal(r: &PcbRegion) -> RegionGraphic {
    RegionGraphic {
        unique_id: r.unique_id.clone(),
        layer: r.common.layer,
        flags: r.common.flags,
        kind: r.kind,
        outline: r.outline.clone(),
        holes: r.holes.clone(),
    }
}

fn text_from_internal(t: &PcbText) -> TextGraphic {
    TextGraphic {
        unique_id: t.unique_id.clone(),
        layer: t.common.layer,
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
        layer: v.common.layer,
        flags: v.common.flags,
        location: v.location,
        diameter: v.diameter,
        hole_size: v.hole_size,
        from_layer: v.from_layer,
        to_layer: v.to_layer,
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
    ComponentBodyGraphic {
        unique_id: b.unique_id.clone(),
        layer: b.common.layer,
        flags: b.common.flags,
        standoff_height: b.standoff_height,
        overall_height: b.overall_height,
        body_color_3d: b.body_color_3d,
        body_opacity_3d: b.body_opacity_3d,
        model_name: b.model_name.clone(),
        outline: b.outline.clone(),
    }
}
