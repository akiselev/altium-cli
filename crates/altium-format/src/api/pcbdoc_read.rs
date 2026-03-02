//! Read path: convert internal PcbDoc types → public API types.
//!
//! The core function `board_from_internal` iterates all sections in a `PcbDoc`,
//! builds lookup tables for cross-reference resolution (net indices → names,
//! component indices → designators), and converts each section into public
//! API types.

use altium_format_types::color::Color;
use altium_format_types::common::Unit;
use altium_format_types::coord::{Coord, CoordPoint};
use altium_format_types::pcb::{
    ClassMemberKind, DimensionKind, LayerRef, PlaneConnectionStyle, RegionKind, V7Layer,
};

use crate::api::pcbdoc_types::*;
use crate::pcbdoc::primitives::PcbPrimitive;
use crate::pcbdoc::records::{
    ParamSectionKind, PrefixedParamSectionKind, PrimitiveSectionKind,
};
use crate::pcbdoc::{
    ModelsSectionData, ParamSectionData, PcbDoc, PcbDocSection, PrefixedParamSectionData,
    PrimitiveSectionData, WideStringsSectionData,
};
use crate::pcblib::{Contour, PcbComponentBody, PcbPad, PcbRegion, PcbVia};
use crate::{Result, ResultExt};

// ── Lookup context ──────────────────────────────────────────────────────────

/// Holds lookup tables built from parameter sections so that primitive
/// conversion functions can resolve indices to names.
struct ConvertContext {
    /// Net index → net name. Index 0xFFFF means "no net".
    net_names: Vec<String>,
    /// Component index → designator. Index 0xFFFF means "no component".
    component_designators: Vec<String>,
    /// WideStrings6 index → text (for PcbText wide_string_index).
    wide_strings: Vec<String>,
}

impl ConvertContext {
    fn resolve_net(&self, index: u16) -> Option<String> {
        if index == 0xFFFF {
            return None;
        }
        self.net_names.get(index as usize).cloned()
    }

    fn resolve_component(&self, index: u16) -> Option<String> {
        if index == 0xFFFF {
            return None;
        }
        self.component_designators.get(index as usize).cloned()
    }

    fn resolve_wide_string(&self, index: i32) -> Option<String> {
        if index < 0 {
            return None;
        }
        self.wide_strings.get(index as usize).cloned()
    }
}

// ── Core conversion ─────────────────────────────────────────────────────────

/// Convert a parsed `PcbDoc` into a public `PcbDocBoard`.
pub(crate) fn board_from_internal(doc: &PcbDoc) -> Result<PcbDocBoard> {
    // Step 1: Build lookup tables from parameter sections.
    let ctx = build_context(doc)?;

    // Step 2: Convert named collections from parameter sections.
    let nets = convert_nets(doc)?;
    let components = convert_components(doc)?;
    let polygons = convert_polygons(doc, &ctx)?;
    let classes = convert_classes(doc)?;
    let differential_pairs = convert_differential_pairs(doc)?;
    let dimensions = convert_dimensions(doc)?;
    let models = convert_models(doc);
    let rules = convert_rules(doc);
    let settings = convert_board_settings(doc)?;

    // Step 3: Convert primitives from binary sections.
    let mut tracks = Vec::new();
    let mut arcs = Vec::new();
    let mut vias = Vec::new();
    let mut pads = Vec::new();
    let mut fills = Vec::new();
    let mut texts = Vec::new();
    let mut regions = Vec::new();
    let mut component_bodies = Vec::new();

    // Track which legacy section kinds have modern counterparts present.
    let has_shape_based_regions = has_section(doc, PrimitiveSectionKind::ShapeBasedRegions6);
    let has_shape_based_bodies =
        has_section(doc, PrimitiveSectionKind::ShapeBasedComponentBodies6);
    let has_texts6 = has_section(doc, PrimitiveSectionKind::Texts6);

    for section in &doc.sections {
        if let PcbDocSection::Primitive(prim) = section {
            // Skip legacy sections when modern counterpart exists.
            match prim.kind {
                PrimitiveSectionKind::Regions6 if has_shape_based_regions => continue,
                PrimitiveSectionKind::ComponentBodies6 if has_shape_based_bodies => continue,
                PrimitiveSectionKind::Texts if has_texts6 => continue,
                _ => {}
            }

            convert_primitive_section(
                prim,
                &ctx,
                &mut tracks,
                &mut arcs,
                &mut vias,
                &mut pads,
                &mut fills,
                &mut texts,
                &mut regions,
                &mut component_bodies,
            );
        }
    }

    // Step 4: Extract board outline from regions.
    let board_outline = find_board_outline(&regions);
    let mut settings = settings;
    settings.board_outline = board_outline;

    Ok(PcbDocBoard {
        settings,
        nets,
        components,
        polygons,
        classes,
        rules,
        differential_pairs,
        tracks,
        arcs,
        vias,
        pads,
        fills,
        texts,
        regions,
        component_bodies,
        dimensions,
        models,
    })
}

// ── Context building ────────────────────────────────────────────────────────

fn build_context(doc: &PcbDoc) -> Result<ConvertContext> {
    let mut net_names = Vec::new();
    let mut component_designators = Vec::new();
    let mut wide_strings = Vec::new();

    for section in &doc.sections {
        match section {
            PcbDocSection::Parameter(ParamSectionData { kind, records })
                if *kind == ParamSectionKind::Nets6 =>
            {
                for rec in records {
                    let mut params = rec.params.clone();
                    let name: String = params
                        .remove_with_default("NAME", String::new())
                        .context("Nets6 record")?;
                    net_names.push(name);
                }
            }
            PcbDocSection::Parameter(ParamSectionData { kind, records })
                if *kind == ParamSectionKind::Components6 =>
            {
                for rec in records {
                    let mut params = rec.params.clone();
                    let designator: String = params
                        .remove_with_default("SOURCEDESIGNATOR", String::new())
                        .context("Components6 record")?;
                    component_designators.push(designator);
                }
            }
            PcbDocSection::WideStrings(WideStringsSectionData { entries }) => {
                for entry in entries {
                    // Entries are sequential by index — just push in order.
                    wide_strings.push(entry.text.clone());
                }
            }
            _ => {}
        }
    }

    Ok(ConvertContext {
        net_names,
        component_designators,
        wide_strings,
    })
}

// ── Named collection converters ─────────────────────────────────────────────

fn find_param_section<'a>(
    doc: &'a PcbDoc,
    kind: ParamSectionKind,
) -> Option<&'a [crate::pcbdoc::records::StandardParamRecord]> {
    for section in &doc.sections {
        if let PcbDocSection::Parameter(ParamSectionData {
            kind: k,
            records,
        }) = section
        {
            if *k == kind {
                return Some(records);
            }
        }
    }
    None
}

fn find_prefixed_param_section<'a>(
    doc: &'a PcbDoc,
    kind: PrefixedParamSectionKind,
) -> Option<&'a [crate::pcbdoc::records::PrefixedParamRecord]> {
    for section in &doc.sections {
        if let PcbDocSection::PrefixedParameter(PrefixedParamSectionData {
            kind: k,
            records,
        }) = section
        {
            if *k == kind {
                return Some(records);
            }
        }
    }
    None
}

fn has_section(doc: &PcbDoc, kind: PrimitiveSectionKind) -> bool {
    doc.sections.iter().any(|s| {
        matches!(s, PcbDocSection::Primitive(PrimitiveSectionData { kind: k, .. }) if *k == kind)
    })
}

fn convert_nets(doc: &PcbDoc) -> Result<Vec<Net>> {
    let records = match find_param_section(doc, ParamSectionKind::Nets6) {
        Some(r) => r,
        None => return Ok(Vec::new()),
    };

    let mut nets = Vec::with_capacity(records.len());
    for rec in records {
        let mut params = rec.params.clone();
        let name: String = params
            .remove_with_default("NAME", String::new())
            .context("Nets6 net name")?;
        let color_raw: i32 = params
            .remove_with_default("COLOR", 0)
            .context("Nets6 color")?;
        let visible: bool = params
            .remove_with_default("NETISVISIBLE", true)
            .context("Nets6 visible")?;

        let id = name.clone();
        nets.push(Net {
            id,
            name,
            color: Color::new(color_raw),
            visible,
        });
    }
    Ok(nets)
}

fn convert_components(doc: &PcbDoc) -> Result<Vec<PcbDocComponent>> {
    let records = match find_param_section(doc, ParamSectionKind::Components6) {
        Some(r) => r,
        None => return Ok(Vec::new()),
    };

    let mut components = Vec::with_capacity(records.len());
    for rec in records {
        let mut params = rec.params.clone();
        let designator: String = params
            .remove_with_default("SOURCEDESIGNATOR", String::new())
            .context("Components6 designator")?;
        let pattern: String = params
            .remove_with_default("PATTERN", String::new())
            .context("Components6 pattern")?;
        let comment: String = params
            .remove_with_default("COMMENT", String::new())
            .context("Components6 comment")?;
        let source_library: String = params
            .remove_with_default("SOURCECOMPONENTLIBRARY", String::new())
            .context("Components6 source library")?;
        let source_lib_reference: String = params
            .remove_with_default("SOURCELIBREFERENCE", String::new())
            .context("Components6 source lib reference")?;
        let x: i32 = params
            .remove_with_default("X1", 0)
            .context("Components6 X1")?;
        let y: i32 = params
            .remove_with_default("Y1", 0)
            .context("Components6 Y1")?;
        let rotation: f64 = params
            .remove_with_default("ROTATION", 0.0)
            .context("Components6 rotation")?;
        let layer_raw: u8 = params
            .remove_with_default("LAYER", 1u8)
            .context("Components6 layer")?;
        let layer = match altium_format_types::pcb::V6Layer::try_from(layer_raw) {
            Ok(v6) => LayerRef::from_v6(v6),
            Err(_) => LayerRef::from_v6(altium_format_types::pcb::V6Layer::TopLayer),
        };

        let id = designator.clone();
        components.push(PcbDocComponent {
            id,
            designator,
            pattern,
            comment,
            location: CoordPoint::new(Coord::from_internal(x), Coord::from_internal(y)),
            rotation,
            layer,
            source_library,
            source_lib_reference,
        });
    }
    Ok(components)
}

fn convert_polygons(doc: &PcbDoc, ctx: &ConvertContext) -> Result<Vec<Polygon>> {
    let records = match find_param_section(doc, ParamSectionKind::Polygons6) {
        Some(r) => r,
        None => return Ok(Vec::new()),
    };

    let mut polygons = Vec::with_capacity(records.len());
    for (idx, rec) in records.iter().enumerate() {
        let mut params = rec.params.clone();
        let name: String = params
            .remove_with_default("NAME", String::new())
            .context("Polygons6 name")?;
        let net_index: i32 = params
            .remove_with_default("NET", -1)
            .context("Polygons6 net")?;
        let net = if net_index >= 0 {
            ctx.net_names.get(net_index as usize).cloned()
        } else {
            None
        };
        let layer_raw: u8 = params
            .remove_with_default("LAYER", 1u8)
            .context("Polygons6 layer")?;
        let layer = match altium_format_types::pcb::V6Layer::try_from(layer_raw) {
            Ok(v6) => LayerRef::from_v6(v6),
            Err(_) => LayerRef::from_v6(altium_format_types::pcb::V6Layer::TopLayer),
        };
        let connect_style_raw: u8 = params
            .remove_with_default("CONNECTSTYLE", 1u8)
            .context("Polygons6 connect style")?;
        let connect_style = PlaneConnectionStyle::try_from(connect_style_raw)
            .unwrap_or(PlaneConnectionStyle::Relief);
        let pour_order: i32 = params
            .remove_with_default("POURORDER", 0)
            .context("Polygons6 pour order")?;
        let relief_conductor_width: i32 = params
            .remove_with_default("RELIEFCONDUCTORWIDTH", 254_000)
            .context("Polygons6 relief conductor width")?;
        let relief_entries: i32 = params
            .remove_with_default("RELIEFENTRIES", 4)
            .context("Polygons6 relief entries")?;
        let relief_air_gap: i32 = params
            .remove_with_default("RELIEFAIRGAP", 254_000)
            .context("Polygons6 relief air gap")?;

        // Extract vertices from indexed coordinate keys.
        let vertices = extract_polygon_vertices(&mut params)?;

        let id = if name.is_empty() {
            format!("polygon_{idx}")
        } else {
            name.clone()
        };

        polygons.push(Polygon {
            id,
            name,
            net,
            layer,
            connect_style,
            pour_order,
            vertices,
            relief_conductor_width: Coord::from_internal(relief_conductor_width),
            relief_entries,
            relief_air_gap: Coord::from_internal(relief_air_gap),
        });
    }
    Ok(polygons)
}

fn extract_polygon_vertices(
    params: &mut crate::param_collection::ParameterCollection,
) -> Result<Vec<CoordPoint>> {
    // Polygons use VX0/VY0, VX1/VY1, ... for vertices.
    let mut vertices = Vec::new();
    let mut i = 0;
    loop {
        let x_key = format!("VX{i}");
        let y_key = format!("VY{i}");
        let x: Option<i32> = params.remove_optional(&x_key).unwrap_or(None);
        let y: Option<i32> = params.remove_optional(&y_key).unwrap_or(None);
        match (x, y) {
            (Some(xv), Some(yv)) => {
                vertices.push(CoordPoint::new(
                    Coord::from_internal(xv),
                    Coord::from_internal(yv),
                ));
            }
            _ => break,
        }
        i += 1;
    }
    Ok(vertices)
}

fn convert_classes(doc: &PcbDoc) -> Result<Vec<NetClass>> {
    let records = match find_param_section(doc, ParamSectionKind::Classes6) {
        Some(r) => r,
        None => return Ok(Vec::new()),
    };

    let mut classes = Vec::with_capacity(records.len());
    for rec in records {
        let mut params = rec.params.clone();
        let name: String = params
            .remove_with_default("NAME", String::new())
            .context("Classes6 name")?;
        let kind_raw: u8 = params
            .remove_with_default("KIND", 0u8)
            .context("Classes6 kind")?;
        let kind = ClassMemberKind::try_from(kind_raw).unwrap_or(ClassMemberKind::Net);

        // Members are stored as M0, M1, M2, ... keys.
        let mut members = Vec::new();
        let mut i = 0;
        loop {
            let key = format!("M{i}");
            let member: Option<String> = params.remove_optional(&key).unwrap_or(None);
            match member {
                Some(m) => members.push(m),
                None => break,
            }
            i += 1;
        }

        let id = name.clone();
        classes.push(NetClass {
            id,
            name,
            kind,
            members,
        });
    }
    Ok(classes)
}

fn convert_differential_pairs(doc: &PcbDoc) -> Result<Vec<DifferentialPair>> {
    let records = match find_param_section(doc, ParamSectionKind::DifferentialPairs6) {
        Some(r) => r,
        None => return Ok(Vec::new()),
    };

    let mut pairs = Vec::with_capacity(records.len());
    for rec in records {
        let mut params = rec.params.clone();
        let name: String = params
            .remove_with_default("NAME", String::new())
            .context("DifferentialPairs6 name")?;
        let positive_net: String = params
            .remove_with_default("POSITIVENET", String::new())
            .context("DifferentialPairs6 positive net")?;
        let negative_net: String = params
            .remove_with_default("NEGATIVENET", String::new())
            .context("DifferentialPairs6 negative net")?;

        let id = name.clone();
        pairs.push(DifferentialPair {
            id,
            name,
            positive_net,
            negative_net,
        });
    }
    Ok(pairs)
}

fn convert_dimensions(doc: &PcbDoc) -> Result<Vec<Dimension>> {
    let records = match find_prefixed_param_section(doc, PrefixedParamSectionKind::Dimensions6) {
        Some(r) => r,
        None => return Ok(Vec::new()),
    };

    let mut dims = Vec::with_capacity(records.len());
    for (idx, rec) in records.iter().enumerate() {
        let mut params = rec.params.clone();
        let kind_raw: u8 = params
            .remove_with_default("DIMENSIONKIND", 0u8)
            .context("Dimensions6 kind")?;
        let kind = DimensionKind::try_from(kind_raw).unwrap_or(DimensionKind::NoDimension);
        let layer_raw: u8 = params
            .remove_with_default("LAYER", 0u8)
            .context("Dimensions6 layer")?;
        let layer = match altium_format_types::pcb::V6Layer::try_from(layer_raw) {
            Ok(v6) => LayerRef::from_v6(v6),
            Err(_) => LayerRef::from_v6(altium_format_types::pcb::V6Layer::NoLayer),
        };
        let text_x: i32 = params
            .remove_with_default("TEXTX", 0)
            .context("Dimensions6 text_x")?;
        let text_y: i32 = params
            .remove_with_default("TEXTY", 0)
            .context("Dimensions6 text_y")?;
        let text_height: i32 = params
            .remove_with_default("TEXTHEIGHT", 100_000)
            .context("Dimensions6 text_height")?;
        let text_width: i32 = params
            .remove_with_default("TEXTWIDTH", 0)
            .context("Dimensions6 text_width")?;

        dims.push(Dimension {
            id: format!("dimension_{idx}"),
            kind,
            layer,
            text_x: Coord::from_internal(text_x),
            text_y: Coord::from_internal(text_y),
            text_height: Coord::from_internal(text_height),
            text_width: Coord::from_internal(text_width),
        });
    }
    Ok(dims)
}

fn convert_models(doc: &PcbDoc) -> Vec<Model3D> {
    let mut models = Vec::new();
    for section in &doc.sections {
        if let PcbDocSection::Models(ModelsSectionData { metadata, .. }) = section {
            for (idx, entry) in metadata.iter().enumerate() {
                models.push(Model3D {
                    id: format!("model_{idx}"),
                    name: entry.name.clone(),
                    checksum: entry.checksum.clone(),
                });
            }
        }
    }
    models
}

fn convert_rules(doc: &PcbDoc) -> Vec<DesignRule> {
    doc.rules
        .iter()
        .enumerate()
        .map(|(idx, rule)| {
            let id = if rule.base.name.is_empty() {
                format!("rule_{idx}")
            } else {
                rule.base.name.clone()
            };
            DesignRule {
                id,
                name: rule.base.name.clone(),
                kind: rule.base.rule_kind,
                enabled: rule.base.enabled,
                priority: rule.base.priority as i32,
                scope: rule.base.scope1_expression.clone(),
                comment: rule.base.comment.clone(),
            }
        })
        .collect()
}

fn convert_board_settings(doc: &PcbDoc) -> Result<BoardSettings> {
    let records = find_param_section(doc, ParamSectionKind::Board6);
    let mut settings = BoardSettings {
        document_name: String::new(),
        signal_layer_count: 2,
        board_outline: None,
        snap_grid_size: Coord::from_internal(100_000), // 10 mil default
        visible_grid_size: Coord::from_internal(100_000),
        display_unit: Unit::Imperial,
    };

    if let Some(records) = records {
        if let Some(first) = records.first() {
            let mut params = first.params.clone();
            settings.document_name = params
                .remove_with_default("DOCUMENTNAME", String::new())
                .context("Board6 document name")?;
            settings.signal_layer_count = params
                .remove_with_default("SIGNALLAYERCOUNT", 2)
                .context("Board6 signal layer count")?;
            let snap: i32 = params
                .remove_with_default("SNAPGRIDSIZE", 100_000)
                .context("Board6 snap grid size")?;
            settings.snap_grid_size = Coord::from_internal(snap);
            let vis: i32 = params
                .remove_with_default("VISIBLEGRIDSIZE", 100_000)
                .context("Board6 visible grid size")?;
            settings.visible_grid_size = Coord::from_internal(vis);
            let unit_raw: u8 = params
                .remove_with_default("DISPLAYUNIT", 1u8)
                .context("Board6 display unit")?;
            settings.display_unit = Unit::try_from(unit_raw).unwrap_or(Unit::Imperial);
        }
    }

    Ok(settings)
}

// ── Primitive conversion ────────────────────────────────────────────────────

fn convert_primitive_section(
    section: &PrimitiveSectionData,
    ctx: &ConvertContext,
    tracks: &mut Vec<Track>,
    arcs: &mut Vec<Arc>,
    vias: &mut Vec<Via>,
    pads: &mut Vec<Pad>,
    fills: &mut Vec<Fill>,
    texts: &mut Vec<Text>,
    regions: &mut Vec<Region>,
    component_bodies: &mut Vec<ComponentBody>,
) {
    for record in &section.records {
        match &record.primitive {
            PcbPrimitive::Track(t) => {
                tracks.push(track_from_internal(tracks.len(), t, ctx));
            }
            PcbPrimitive::Arc(a) => {
                arcs.push(arc_from_internal(arcs.len(), a, ctx));
            }
            PcbPrimitive::Via(v) => {
                vias.push(via_from_internal(vias.len(), v, ctx));
            }
            PcbPrimitive::Pad(p) => {
                pads.push(pad_from_internal(pads.len(), p, ctx));
            }
            PcbPrimitive::Fill(f) => {
                fills.push(fill_from_internal(fills.len(), f, ctx));
            }
            PcbPrimitive::Text(t) => {
                texts.push(text_from_internal(texts.len(), t, ctx));
            }
            PcbPrimitive::Region(r) => {
                regions.push(region_from_internal(regions.len(), r, ctx));
            }
            PcbPrimitive::ComponentBody(b) => {
                component_bodies.push(body_from_internal(component_bodies.len(), b, ctx));
            }
        }
    }
}

// ── Per-type converters ─────────────────────────────────────────────────────

fn resolve_layer_v6_v7(
    v6: altium_format_types::pcb::V6Layer,
    v7: V7Layer,
) -> LayerRef {
    if v7.raw() != 0 {
        LayerRef::from_v6_and_v7(v6, v7)
    } else {
        LayerRef::from_v6(v6)
    }
}

fn track_from_internal(
    idx: usize,
    t: &crate::pcbdoc::primitives::PcbTrack,
    ctx: &ConvertContext,
) -> Track {
    Track {
        id: format!("track_{idx}"),
        layer: resolve_layer_v6_v7(t.common.layer, t.layer_enum_index),
        net: ctx.resolve_net(t.common.net_index),
        component: ctx.resolve_component(t.common.component_index),
        start: t.start,
        end: t.end,
        width: t.width,
    }
}

fn arc_from_internal(
    idx: usize,
    a: &crate::pcbdoc::primitives::PcbArc,
    ctx: &ConvertContext,
) -> Arc {
    Arc {
        id: format!("arc_{idx}"),
        layer: resolve_layer_v6_v7(a.common.layer, a.layer_enum_index),
        net: ctx.resolve_net(a.common.net_index),
        component: ctx.resolve_component(a.common.component_index),
        center: a.center,
        radius: a.radius,
        start_angle: a.start_angle,
        end_angle: a.end_angle,
        width: a.width,
    }
}

fn via_from_internal(idx: usize, v: &PcbVia, ctx: &ConvertContext) -> Via {
    // Solder mask expansion: use front expansion if the override flag is set.
    let solder_mask_expansion = if v.solder_mask_override {
        Some(v.solder_mask_expansion_front)
    } else {
        None
    };

    Via {
        id: format!("via_{idx}"),
        net: ctx.resolve_net(v.common.net_index),
        component: ctx.resolve_component(v.common.component_index),
        location: v.location,
        diameter: v.diameter,
        hole_size: v.hole_size,
        from_layer: LayerRef::from_v6(v.from_layer),
        to_layer: LayerRef::from_v6(v.to_layer),
        solder_mask_expansion,
    }
}

fn pad_from_internal(idx: usize, p: &PcbPad, ctx: &ConvertContext) -> Pad {
    Pad {
        id: format!("pad_{idx}"),
        pad_name: p.pad_name.clone(),
        layer: LayerRef::from_v6(p.common.layer),
        net: ctx.resolve_net(p.common.net_index),
        component: ctx.resolve_component(p.common.component_index),
        location: p.location,
        shape: p.shape_top,
        x_size: p.size_top.x,
        y_size: p.size_top.y,
        rotation: p.rotation,
        hole_size: p.hole_size,
        is_plated: p.is_plated,
        pad_mode: p.pad_mode,
        solder_mask_expansion: p.cache.solder_mask_expansion,
        paste_mask_expansion: p.cache.paste_mask_expansion,
        plane_connection: p.cache.plane_connection_style,
        relief_conductor_width: p.cache.relief_conductor_width,
        relief_entries: p.cache.relief_entries as i32,
        relief_air_gap: p.cache.relief_air_gap,
    }
}

fn fill_from_internal(
    idx: usize,
    f: &crate::pcbdoc::primitives::PcbFill,
    ctx: &ConvertContext,
) -> Fill {
    let layer = match f.layer_enum_index {
        Some(v7) => LayerRef::from_v6_and_v7(f.common.layer, v7),
        None => LayerRef::from_v6(f.common.layer),
    };
    Fill {
        id: format!("fill_{idx}"),
        layer,
        net: ctx.resolve_net(f.common.net_index),
        component: ctx.resolve_component(f.common.component_index),
        corner1: f.corner_1,
        corner2: f.corner_2,
        rotation: f.rotation,
    }
}

fn text_from_internal(
    idx: usize,
    t: &crate::pcbdoc::primitives::PcbText,
    ctx: &ConvertContext,
) -> Text {
    // Resolve wide string if the index is valid.
    let text = ctx
        .resolve_wide_string(t.wide_string_index)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| t.text.clone());

    Text {
        id: format!("text_{idx}"),
        layer: LayerRef::from_v6(t.common.layer),
        component: ctx.resolve_component(t.common.component_index),
        location: t.location,
        text,
        height: t.height,
        width: t.stroke_width,
        rotation: t.rotation,
        font_name: t.font_name.clone(),
        is_mirrored: t.is_mirrored,
        is_comment: t.is_comment,
        is_designator: t.is_designator,
    }
}

fn contour_to_coord_points(contour: &Contour) -> Vec<CoordPoint> {
    match contour {
        Contour::Legacy(pts) => pts.clone(),
        Contour::ShapeBased(segs) => segs.iter().map(|s| s.vertex).collect(),
    }
}

fn region_from_internal(idx: usize, r: &PcbRegion, ctx: &ConvertContext) -> Region {
    let layer = if !r.v7_layer.is_empty() {
        LayerRef::from_string_name(&r.v7_layer)
            .unwrap_or_else(|| LayerRef::from_v6(r.common.layer))
    } else {
        LayerRef::from_v6(r.common.layer)
    };
    Region {
        id: format!("region_{idx}"),
        layer,
        net: ctx.resolve_net(r.common.net_index),
        component: ctx.resolve_component(r.common.component_index),
        kind: r.kind,
        outline: contour_to_coord_points(&r.outline),
        holes: r.holes.iter().map(contour_to_coord_points).collect(),
        is_board_cutout: r.is_board_cutout,
        is_keepout: r.keepout,
    }
}

fn body_from_internal(
    idx: usize,
    b: &PcbComponentBody,
    ctx: &ConvertContext,
) -> ComponentBody {
    let layer = if !b.v7_layer.is_empty() {
        LayerRef::from_string_name(&b.v7_layer)
            .unwrap_or_else(|| LayerRef::from_v6(b.common.layer))
    } else {
        LayerRef::from_v6(b.common.layer)
    };
    ComponentBody {
        id: format!("body_{idx}"),
        layer,
        component: ctx.resolve_component(b.common.component_index),
        standoff_height: b.standoff_height,
        overall_height: b.overall_height,
        body_color_3d: b.body_color_3d,
        body_opacity_3d: b.body_opacity_3d,
        model_name: b.model_name.clone(),
        outline: contour_to_coord_points(&b.outline),
    }
}

// ── Board outline extraction ────────────────────────────────────────────────

fn find_board_outline(regions: &[Region]) -> Option<Vec<CoordPoint>> {
    // Look for a board cutout region or a board outline region kind.
    for region in regions {
        if region.kind == RegionKind::BoardCutout || region.is_board_cutout {
            if !region.outline.is_empty() {
                return Some(region.outline.clone());
            }
        }
    }
    None
}
