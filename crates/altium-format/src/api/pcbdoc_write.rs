//! Write path: convert public API types → internal PcbDoc sections.
//!
//! The core function `board_to_internal` takes a `PcbDocBoard` and updates
//! the internal `PcbDoc` sections in place. Parameter sections are rebuilt
//! from scratch; primitive sections preserve format-internal fields from
//! existing records matched by position.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use altium_format_types::coord::{Coord, CoordPoint};
use altium_format_types::pcb::{
    BarcodeRenderMode, DaisyChainStyle, LayerRef, MaskExpansionState, PadStackMode, PcbFlags,
    PlaneConnectionStyle, TCacheState, TextKind, V6Layer,
};
use altium_format_types::{BarcodeKind, PcbObjectId};

use crate::api::pcbdoc_types::*;
use crate::param_collection::ParameterCollection;
use crate::param_value::{MilCoord, ToParamValue};
use crate::pcbdoc::primitives::{
    ParsedPrimitiveRecord, PcbArc, PcbFill, PcbPrimitive, PcbText, PcbTrack,
};
use crate::pcbdoc::records::{
    BinaryLenRecord, BinaryLenSectionKind, ConnectionCommonHeader, ParamSectionKind,
    PrimitiveSectionKind, StandardParamRecord, WideString6Record,
};
use crate::pcbdoc::{
    BinarySectionData, ParamSectionData, PcbDoc, PcbDocSection, PrimitiveSectionData,
    PrimitiveParametersSectionData, WideStringsSectionData,
};
use crate::pcbdoc::records::PrimitiveParameterGroup;
use crate::pcblib::{
    Contour, PcbComponentBody, PcbPad, PcbPadCache, PcbPrimitiveCommon, PcbRegion, PcbVia,
};
use crate::Result;

// ── Write context ──────────────────────────────────────────────────────────

/// Inverse of `ConvertContext` in `pcbdoc_read.rs`: maps names back to indices.
struct WriteContext {
    /// Net name → section index (position in Nets6).
    net_indices: HashMap<String, u16>,
    /// Component designator → section index (position in Components6).
    component_indices: HashMap<String, u16>,
}

impl WriteContext {
    fn new(board: &PcbDocBoard) -> Self {
        let mut net_indices = HashMap::with_capacity(board.nets.len());
        for (i, net) in board.nets.iter().enumerate() {
            net_indices.insert(net.name.clone(), i as u16);
        }

        let mut component_indices = HashMap::with_capacity(board.components.len());
        for (i, comp) in board.components.iter().enumerate() {
            component_indices.insert(comp.designator.clone(), i as u16);
        }

        Self {
            net_indices,
            component_indices,
        }
    }

    fn resolve_net_index(&self, net: &Option<String>) -> u16 {
        match net {
            Some(name) => self.net_indices.get(name.as_str()).copied().unwrap_or(0xFFFF),
            None => 0xFFFF,
        }
    }

    fn resolve_component_index(&self, component: &Option<String>) -> u16 {
        match component {
            Some(name) => self
                .component_indices
                .get(name.as_str())
                .copied()
                .unwrap_or(0xFFFF),
            None => 0xFFFF,
        }
    }

    /// Resolve net name → i32 index for parameter sections (Polygons6).
    /// Returns -1 for no net.
    fn resolve_net_param_index(&self, net: &Option<String>) -> i32 {
        match net {
            Some(name) => self
                .net_indices
                .get(name.as_str())
                .map(|&i| i as i32)
                .unwrap_or(-1),
            None => -1,
        }
    }
}

/// Build a `PcbPrimitiveCommon` for a board context with net/component resolution.
fn primitive_common_for_board(
    layer: &LayerRef,
    net: &Option<String>,
    component: &Option<String>,
    ctx: &WriteContext,
) -> PcbPrimitiveCommon {
    PcbPrimitiveCommon {
        layer: layer.to_v6().unwrap_or(V6Layer::NoLayer),
        flags: PcbFlags::default(),
        net_index: ctx.resolve_net_index(net),
        polygon_index: 0xFFFF, // polygon membership not tracked in API
        component_index: ctx.resolve_component_index(component),
        coordinate_index: 0xFFFF,
        dimension_index: 0xFFFF,
    }
}

// ── Core entry point ───────────────────────────────────────────────────────

/// Convert a public `PcbDocBoard` back into internal `PcbDoc` sections.
///
/// Replaces parameter sections with fresh `ParameterCollection`s built from
/// scratch. Primitive sections are rebuilt with format-internal fields
/// preserved from existing records matched by position index.
pub(crate) fn board_to_internal(board: &PcbDocBoard, doc: &mut PcbDoc) -> Result<()> {
    let ctx = WriteContext::new(board);

    // ── Step 1: Rebuild WideStrings first (needed for text primitive indices) ──
    let (wide_strings_data, wide_indices) = rebuild_wide_strings(&board.texts);

    // ── Step 2: Merge parameter sections (preserve unknown fields from old records) ──
    merge_param_section(
        doc,
        ParamSectionKind::Nets6,
        build_net_records(&board.nets),
        "NAME",
    );
    merge_param_section(
        doc,
        ParamSectionKind::Components6,
        build_component_records(&board.components),
        "SOURCEDESIGNATOR",
    );
    replace_param_section(
        doc,
        ParamSectionKind::Polygons6,
        build_polygon_records(&board.polygons, &ctx),
    );
    let enriched_classes = ensure_standard_classes(board);
    merge_param_section(
        doc,
        ParamSectionKind::Classes6,
        build_class_records(&enriched_classes),
        "NAME",
    );
    replace_param_section(
        doc,
        ParamSectionKind::DifferentialPairs6,
        build_diff_pair_records(&board.differential_pairs),
    );
    merge_param_section(
        doc,
        ParamSectionKind::Board6,
        build_board_record(&board.settings),
        "DOCUMENTNAME",
    );

    // ── Step 3: Replace primitive sections ──
    replace_primitives_with_preservation(
        doc,
        PrimitiveSectionKind::Tracks6,
        build_track_records(&board.tracks, &ctx),
    );
    replace_primitives_with_preservation(
        doc,
        PrimitiveSectionKind::Arcs6,
        build_arc_records(&board.arcs, &ctx),
    );
    replace_primitives_with_preservation(
        doc,
        PrimitiveSectionKind::Vias6,
        build_via_records(&board.vias, &ctx),
    );
    replace_primitives_with_preservation(
        doc,
        PrimitiveSectionKind::Pads6,
        build_pad_records(&board.pads, &ctx),
    );
    replace_primitives_with_preservation(
        doc,
        PrimitiveSectionKind::Fills6,
        build_fill_records(&board.fills, &ctx),
    );

    // Text section: use whichever kind was originally present.
    let text_kind = detect_section_kind(
        doc,
        PrimitiveSectionKind::Texts6,
        PrimitiveSectionKind::Texts,
    );
    replace_primitives_with_preservation(
        doc,
        text_kind,
        build_text_records(&board.texts, &ctx, &wide_indices),
    );

    // Region section: prefer ShapeBasedRegions6 if present.
    let region_kind = detect_section_kind(
        doc,
        PrimitiveSectionKind::ShapeBasedRegions6,
        PrimitiveSectionKind::Regions6,
    );
    replace_primitives_with_preservation(
        doc,
        region_kind,
        build_region_records(&board.regions, &ctx),
    );

    // ComponentBody section: prefer ShapeBasedComponentBodies6 if present.
    let body_kind = detect_section_kind(
        doc,
        PrimitiveSectionKind::ShapeBasedComponentBodies6,
        PrimitiveSectionKind::ComponentBodies6,
    );
    replace_primitives_with_preservation(
        doc,
        body_kind,
        build_body_records(&board.component_bodies, &ctx),
    );

    // ── Step 4: Replace WideStrings6 ──
    replace_wide_strings(doc, wide_strings_data);

    // ── Step 5: Generate Connections6 (ratsnest) ──
    let connections = compute_ratsnest(board, &ctx);
    replace_binary_section(doc, BinaryLenSectionKind::Connections6, connections);

    // ── Step 6: Generate PrimitiveParameters (BOM data) ──
    let prim_params = build_primitive_parameters(&board.components);
    replace_primitive_parameters_section(doc, prim_params);

    // ── Step 7: Rebuild UniqueIDPrimitiveInformation for pads ──
    assign_and_rebuild_unique_id_section(doc);

    Ok(())
}

// ── Parameter section builders ─────────────────────────────────────────────

fn build_net_records(nets: &[Net]) -> Vec<StandardParamRecord> {
    nets.iter()
        .map(|net| {
            let mut params = ParameterCollection::new();
            // Standard Altium net fields with sensible defaults.
            params.insert("SELECTION", "FALSE".to_string());
            params.insert("LAYER", "MULTILAYER".to_string());
            params.insert("LOCKED", "FALSE".to_string());
            params.insert("POLYGONOUTLINE", "FALSE".to_string());
            params.insert("USERROUTED", "TRUE".to_string());
            params.insert("KEEPOUT", "FALSE".to_string());
            params.insert("UNIONINDEX", "0".to_string());
            params.insert("PRIMITIVELOCK", "FALSE".to_string());
            params.insert("NAME", net.name.clone());
            params.insert("VISIBLE", net.visible.to_param_value());
            params.insert("COLOR", net.color.to_param_value());
            params.insert("LOOPREMOVAL", "-1".to_string());
            params.insert("OVERRIDECOLORFORDRAW", "FALSE".to_string());
            params.insert("UNIQUEID", generate_unique_id());
            StandardParamRecord { params }
        })
        .collect()
}

fn build_component_records(components: &[PcbDocComponent]) -> Vec<StandardParamRecord> {
    components
        .iter()
        .map(|comp| {
            let mut params = ParameterCollection::new();
            // Standard Altium component fields with sensible defaults.
            // These are written in the order Altium Designer expects.
            params.insert("SELECTION", "FALSE".to_string());
            let layer_v6 = comp.layer.to_v6().unwrap_or(V6Layer::TopLayer);
            let layer_name = match layer_v6 {
                V6Layer::TopLayer => "TOP".to_string(),
                V6Layer::BottomLayer => "BOTTOM".to_string(),
                other => other.to_string_name().to_uppercase(),
            };
            params.insert("LAYER", layer_name);
            params.insert("LOCKED", "FALSE".to_string());
            params.insert("POLYGONOUTLINE", "FALSE".to_string());
            params.insert("USERROUTED", "TRUE".to_string());
            params.insert("KEEPOUT", "FALSE".to_string());
            params.insert("PRIMITIVELOCK", "TRUE".to_string());
            params.insert("X", MilCoord(comp.location.x).to_param_value());
            params.insert("Y", MilCoord(comp.location.y).to_param_value());
            params.insert("PATTERN", comp.pattern.clone());
            params.insert("NAMEON", "TRUE".to_string());
            params.insert("COMMENTON", "FALSE".to_string());
            params.insert("GROUPNUM", "0".to_string());
            params.insert("COUNT", "0".to_string());
            params.insert("ROTATION", format!(" {:.13}", comp.rotation));
            params.insert("HEIGHT", "0mil".to_string());
            params.insert("COMMENTAUTOPOSITION", "0".to_string());
            params.insert("UNIONINDEX", "0".to_string());
            params.insert("CHANNELOFFSET", "0".to_string());
            params.insert("SOURCEDESIGNATOR", comp.designator.clone());
            params.insert("SOURCEUNIQUEID", comp.source_unique_id.clone());
            params.insert("SOURCEHIERARCHICALPATH", comp.source_hierarchical_path.clone());
            params.insert("SOURCEFOOTPRINTLIBRARY", String::new());
            params.insert(
                "SOURCECOMPONENTLIBRARY",
                comp.source_library.clone(),
            );
            params.insert(
                "SOURCELIBREFERENCE",
                comp.source_lib_reference.clone(),
            );
            params.insert("SOURCEDESCRIPTION", String::new());
            params.insert("FOOTPRINTDESCRIPTION", String::new());
            params.insert("COMMENT", comp.comment.clone());
            // Generate a unique ID for new components.
            params.insert("UNIQUEID", generate_unique_id());
            StandardParamRecord { params }
        })
        .collect()
}

/// Generate an 8-character uppercase alphabetic unique ID in the same format
/// Altium Designer uses (e.g. "OYNEOHXI"). Uses a counter + time-based seed
/// to ensure uniqueness within a process lifetime.
fn generate_unique_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let seed = COUNTER.fetch_add(1, Ordering::Relaxed);
    // Mix the counter with a time-based component for cross-process uniqueness.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let mut val = seed.wrapping_mul(6364136223846793005).wrapping_add(now);
    let mut id = String::with_capacity(8);
    for _ in 0..8 {
        id.push((b'A' + (val % 26) as u8) as char);
        val /= 26;
    }
    id
}

fn build_polygon_records(polygons: &[Polygon], ctx: &WriteContext) -> Vec<StandardParamRecord> {
    polygons
        .iter()
        .map(|poly| {
            let mut params = ParameterCollection::new();
            params.insert("NAME", poly.name.clone());
            params.insert("NET", ctx.resolve_net_param_index(&poly.net).to_param_value());
            let layer_v6 = poly.layer.to_v6().unwrap_or(V6Layer::TopLayer);
            params.insert("LAYER", (layer_v6 as u8).to_param_value());
            params.insert("CONNECTSTYLE", (poly.connect_style as u8).to_param_value());
            params.insert("POURORDER", poly.pour_order.to_param_value());
            params.insert(
                "RELIEFCONDUCTORWIDTH",
                poly.relief_conductor_width.to_param_value(),
            );
            params.insert("RELIEFENTRIES", poly.relief_entries.to_param_value());
            params.insert("RELIEFAIRGAP", poly.relief_air_gap.to_param_value());

            // Vertices are 0-indexed: VX0, VY0, VX1, VY1, ...
            for (i, vertex) in poly.vertices.iter().enumerate() {
                params.insert(&format!("VX{i}"), vertex.x.to_param_value());
                params.insert(&format!("VY{i}"), vertex.y.to_param_value());
            }

            StandardParamRecord { params }
        })
        .collect()
}

fn ensure_standard_classes(board: &PcbDocBoard) -> Vec<NetClass> {
    use altium_format_types::pcb::ClassMemberKind;

    let mut classes = board.classes.clone();

    if !classes.iter().any(|c| c.name == "All Components") {
        let members: Vec<String> = board.components.iter()
            .map(|c| c.designator.clone())
            .collect();
        classes.push(NetClass {
            id: "all-components".to_string(),
            name: "All Components".to_string(),
            kind: ClassMemberKind::Component,
            members,
        });
    }

    if !classes.iter().any(|c| c.name == "All Nets") {
        let members: Vec<String> = board.nets.iter()
            .map(|n| n.name.clone())
            .collect();
        classes.push(NetClass {
            id: "all-nets".to_string(),
            name: "All Nets".to_string(),
            kind: ClassMemberKind::Net,
            members,
        });
    }

    classes
}

fn build_class_records(classes: &[NetClass]) -> Vec<StandardParamRecord> {
    classes
        .iter()
        .map(|cls| {
            let mut params = ParameterCollection::new();
            params.insert("NAME", cls.name.clone());
            params.insert("KIND", (cls.kind as u8).to_param_value());
            for (i, member) in cls.members.iter().enumerate() {
                params.insert(&format!("M{i}"), member.clone());
            }
            StandardParamRecord { params }
        })
        .collect()
}

fn build_diff_pair_records(pairs: &[DifferentialPair]) -> Vec<StandardParamRecord> {
    pairs
        .iter()
        .map(|dp| {
            let mut params = ParameterCollection::new();
            params.insert("NAME", dp.name.clone());
            params.insert("POSITIVENET", dp.positive_net.clone());
            params.insert("NEGATIVENET", dp.negative_net.clone());
            StandardParamRecord { params }
        })
        .collect()
}

fn build_board_record(settings: &BoardSettings) -> Vec<StandardParamRecord> {
    let mut params = ParameterCollection::new();
    params.insert("DOCUMENTNAME", settings.document_name.clone());
    params.insert(
        "SIGNALLAYERCOUNT",
        settings.signal_layer_count.to_param_value(),
    );
    params.insert("SNAPGRIDSIZE", settings.snap_grid_size.to_param_value());
    params.insert(
        "VISIBLEGRIDSIZE",
        settings.visible_grid_size.to_param_value(),
    );
    params.insert("DISPLAYUNIT", (settings.display_unit as u8).to_param_value());
    vec![StandardParamRecord { params }]
}

// ── Primitive record builders ──────────────────────────────────────────────

fn build_track_records(tracks: &[Track], ctx: &WriteContext) -> Vec<ParsedPrimitiveRecord> {
    tracks
        .iter()
        .map(|t| ParsedPrimitiveRecord {
            object_id: PcbObjectId::Track,
            primitive: PcbPrimitive::Track(PcbTrack {
                common: primitive_common_for_board(&t.layer, &t.net, &t.component, ctx),
                start: t.start,
                end: t.end,
                width: t.width,
                subpoly_index: 0,
                user_routed: false,
                union_index: 0,
                track_kind: 0,
                layer_enum_index: t.layer.v7(),
                keepout_restrictions: None,
            }),
        })
        .collect()
}

fn build_arc_records(arcs: &[Arc], ctx: &WriteContext) -> Vec<ParsedPrimitiveRecord> {
    arcs.iter()
        .map(|a| ParsedPrimitiveRecord {
            object_id: PcbObjectId::Arc,
            primitive: PcbPrimitive::Arc(PcbArc {
                common: primitive_common_for_board(&a.layer, &a.net, &a.component, ctx),
                center: a.center,
                radius: a.radius,
                start_angle: a.start_angle,
                end_angle: a.end_angle,
                width: a.width,
                subpoly_index: 0,
                user_routed: false,
                union_index: 0,
                layer_enum_index: a.layer.v7(),
                keepout_restrictions: None,
            }),
        })
        .collect()
}

fn build_via_records(vias: &[Via], ctx: &WriteContext) -> Vec<ParsedPrimitiveRecord> {
    vias.iter()
        .map(|v| {
            let (solder_mask_override, solder_mask_expansion_front) =
                match v.solder_mask_expansion {
                    Some(exp) => (true, exp),
                    None => (false, Coord::ZERO),
                };
            ParsedPrimitiveRecord {
                object_id: PcbObjectId::Via,
                primitive: PcbPrimitive::Via(PcbVia {
                    common: primitive_common_for_board(
                        &LayerRef::from_v6(V6Layer::MultiLayer),
                        &v.net,
                        &v.component,
                        ctx,
                    ),
                    location: v.location,
                    diameter: v.diameter,
                    hole_size: v.hole_size,
                    from_layer: v.from_layer.to_v6().unwrap_or(V6Layer::TopLayer),
                    to_layer: v.to_layer.to_v6().unwrap_or(V6Layer::BottomLayer),
                    via_properties_version: 0,
                    thermal_relief_air_gap: Coord::ZERO,
                    thermal_relief_conductor_count: 0,
                    thermal_relief_rotation_code: 0,
                    thermal_relief_conductor_width: Coord::ZERO,
                    power_plane_relief_expansion: Coord::ZERO,
                    power_plane_clearance: Coord::ZERO,
                    paste_mask_expansion: Coord::ZERO,
                    solder_mask_expansion_front,
                    planes: 0,
                    plane_connection_style_valid: TCacheState::default(),
                    relief_conductor_width_valid: TCacheState::default(),
                    relief_entries_valid: TCacheState::default(),
                    relief_air_gap_valid: TCacheState::default(),
                    power_plane_relief_expansion_valid: TCacheState::default(),
                    paste_mask_expansion_valid: TCacheState::default(),
                    solder_mask_expansion_valid: TCacheState::default(),
                    power_plane_clearance_valid: TCacheState::default(),
                    planes_valid: TCacheState::default(),
                    plane_connection_style: PlaneConnectionStyle::default(),
                    solder_mask_cache_flags: 0,
                    solder_mask_expansion_state: MaskExpansionState::default(),
                    paste_mask_cache_flags: 0,
                    paste_mask_expansion_state: MaskExpansionState::default(),
                    via_mode: PadStackMode::Simple,
                    diameters_per_layer: [Coord::ZERO; 32],
                    layer_enum_index: 0,
                    stack_start_layer: 0,
                    stack_end_layer: 0,
                    is_testpoint_top: false,
                    is_testpoint_bottom: false,
                    is_assy_testpoint_top: false,
                    is_assy_testpoint_bottom: false,
                    solder_mask_override,
                    use_separate_solder_mask_expansion: false,
                    solder_mask_expansion_from_hole_edge: false,
                    paste_mask_override: false,
                    solder_mask_expansion_linked: false,
                    solder_mask_expansion_back: Coord::ZERO,
                    template_link_version: None,
                    template_link_library_id: None,
                    template_link_template_id: None,
                    hole_positive_tolerance: None,
                    hole_negative_tolerance: None,
                    template_link_flags: None,
                    pad_layer_entries: Vec::new(),
                    pad_layer_stride: 0,
                    counter_hole_angle: None,
                    via_structure_type: None,
                    layer_diameter_overrides: Vec::new(),
                    unique_id: None,
                }),
            }
        })
        .collect()
}

fn build_pad_records(pads: &[Pad], ctx: &WriteContext) -> Vec<ParsedPrimitiveRecord> {
    pads.iter()
        .map(|p| {
            ParsedPrimitiveRecord {
                object_id: PcbObjectId::Pad,
                primitive: PcbPrimitive::Pad(PcbPad {
                    common: primitive_common_for_board(&p.layer, &p.net, &p.component, ctx),
                    pad_name: p.pad_name.clone(),
                    unknown_sub1: String::new(),
                    unknown_sub2: String::new(),
                    unknown_sub3: String::new(),
                    location: p.location,
                    size_top: CoordPoint::new(p.stack.top.x_size, p.stack.top.y_size),
                    size_mid: CoordPoint::new(p.stack.mid.x_size, p.stack.mid.y_size),
                    size_bot: CoordPoint::new(p.stack.bot.x_size, p.stack.bot.y_size),
                    hole_size: p.hole_size,
                    shape_top: p.stack.top.shape,
                    shape_mid: p.stack.mid.shape,
                    shape_bot: p.stack.bot.shape,
                    rotation: p.rotation,
                    is_plated: p.is_plated,
                    daisy_chain_style: DaisyChainStyle::default(),
                    pad_mode: p.pad_mode,
                    unknown_63: 0,
                    cache: PcbPadCache {
                        plane_connection_style: p.plane_connection,
                        relief_conductor_width: p.relief_conductor_width,
                        relief_entries: p.relief_entries as i16,
                        relief_air_gap: p.relief_air_gap,
                        power_plane_relief_expansion: Coord::ZERO,
                        power_plane_clearance: Coord::ZERO,
                        paste_mask_expansion: p.paste_mask_expansion,
                        solder_mask_expansion: p.solder_mask_expansion,
                        planes: 0,
                        plane_connection_style_valid: TCacheState::Valid,
                        relief_conductor_width_valid: TCacheState::Valid,
                        relief_entries_valid: TCacheState::Valid,
                        relief_air_gap_valid: TCacheState::Valid,
                        power_plane_relief_expansion_valid: TCacheState::default(),
                        paste_mask_expansion_valid: TCacheState::Valid,
                        solder_mask_expansion_valid: TCacheState::Valid,
                        power_plane_clearance_valid: TCacheState::default(),
                        planes_valid: TCacheState::default(),
                    },
                    selection_memory_flags: 0,
                    union_index: 0,
                    jumper_id: 0,
                    v7_layer_override: 0,
                    is_assy_testpoint_top: false,
                    is_assy_testpoint_bottom: false,
                    use_separate_expansions: false,
                    solder_mask_bottom_expansion: 0,
                    solder_mask_expansion_from_hole_edge: false,
                    template_link_library_id: [0u8; 16],
                    template_link_template_id: [0u8; 16],
                    pin_package_length: Coord::ZERO,
                    hole_positive_tolerance: 0,
                    hole_negative_tolerance: 0,
                    reserved_170: 0,
                    has_sub4_extension: false,
                    sub4_extension: None,
                    thermal_reliefs: Vec::new(),
                    stack_data: None,
                    unique_id: None,
                }),
            }
        })
        .collect()
}

fn build_fill_records(fills: &[Fill], ctx: &WriteContext) -> Vec<ParsedPrimitiveRecord> {
    fills
        .iter()
        .map(|f| ParsedPrimitiveRecord {
            object_id: PcbObjectId::Fill,
            primitive: PcbPrimitive::Fill(PcbFill {
                common: primitive_common_for_board(&f.layer, &f.net, &f.component, ctx),
                corner_1: f.corner1,
                corner_2: f.corner2,
                rotation: f.rotation,
                user_routed: None,
                union_index: None,
                layer_enum_index: Some(f.layer.v7()),
                keepout_restrictions: None,
            }),
        })
        .collect()
}

fn build_text_records(
    texts: &[Text],
    ctx: &WriteContext,
    wide_indices: &[i32],
) -> Vec<ParsedPrimitiveRecord> {
    texts
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let wide_string_index = wide_indices.get(i).copied().unwrap_or(-1);
            ParsedPrimitiveRecord {
                object_id: PcbObjectId::Text,
                primitive: PcbPrimitive::Text(PcbText {
                    common: primitive_common_for_board(&t.layer, &None, &t.component, ctx),
                    location: t.location,
                    height: t.height,
                    stroke_font_type: 0,
                    text_kind: TextKind::default(),
                    rotation: t.rotation,
                    is_mirrored: t.is_mirrored,
                    stroke_width: t.width,
                    is_comment: t.is_comment,
                    is_designator: t.is_designator,
                    user_routed: false,
                    is_bold: false,
                    is_italic: false,
                    font_name: t.font_name.clone(),
                    is_inverted: false,
                    margin_border_width: 0,
                    wide_string_index,
                    union_index: 0,
                    is_inverted_rect: false,
                    textbox_rect_width: 0,
                    textbox_rect_height: 0,
                    textbox_rect_justification: 0,
                    text_offset_width: 0,
                    unk_vec_x: 0,
                    unk_vec_y: 0,
                    barcode_margin_x: 0,
                    barcode_margin_y: 0,
                    barcode_min_width: 0,
                    barcode_kind: BarcodeKind::default(),
                    barcode_render_mode: BarcodeRenderMode::default(),
                    barcode_inverted: false,
                    barcode_font_name: String::new(),
                    barcode_min_pixel_size: 0,
                    barcode_show_text: false,
                    has_v7_layer_data: None,
                    layer_enum_index: 0,
                    sentinel_1: 0,
                    sentinel_2: 0,
                    trailing_flag_1: 0,
                    trailing_flag_2: 0,
                    trailing_is_justification_valid: None,
                    advance_snapping: None,
                    advance_mode: None,
                    advance_justification_x: None,
                    advance_justification_y: None,
                    use_text_alignment_by_snap: None,
                    snap_point_x: None,
                    snap_point_y: None,
                    text: t.text.clone(),
                }),
            }
        })
        .collect()
}

fn build_region_records(regions: &[Region], ctx: &WriteContext) -> Vec<ParsedPrimitiveRecord> {
    regions
        .iter()
        .map(|r| ParsedPrimitiveRecord {
            object_id: PcbObjectId::Region,
            primitive: PcbPrimitive::Region(PcbRegion {
                common: primitive_common_for_board(&r.layer, &r.net, &r.component, ctx),
                kind: r.kind,
                v7_layer: r
                    .layer
                    .display_name()
                    .unwrap_or("")
                    .to_owned(),
                name: String::new(),
                param_kind: 0,
                subpoly_index: 0,
                union_index: 0,
                arc_resolution: Coord::ZERO,
                is_shape_based: false,
                cavity_height: Coord::ZERO,
                keepout_restrictions: 0,
                layer: String::new(),
                keepout: r.is_keepout,
                is_board_cutout: r.is_board_cutout,
                pad_index: -1,
                object_kind: String::new(),
                bending_line_count: 0,
                locked_3d: false,
                layer_stack_id: String::new(),
                outline: Contour::Legacy(r.outline.clone()),
                holes: r.holes.iter().map(|h| Contour::Legacy(h.clone())).collect(),
                shape_text_segments: None,
                hole_shape_text_segments: Vec::new(),
                unique_id: None,
            }),
        })
        .collect()
}

fn build_body_records(
    bodies: &[ComponentBody],
    ctx: &WriteContext,
) -> Vec<ParsedPrimitiveRecord> {
    bodies
        .iter()
        .map(|b| ParsedPrimitiveRecord {
            object_id: PcbObjectId::ComponentBody,
            primitive: PcbPrimitive::ComponentBody(PcbComponentBody {
                common: primitive_common_for_board(&b.layer, &None, &b.component, ctx),
                v7_layer: b
                    .layer
                    .display_name()
                    .unwrap_or("")
                    .to_owned(),
                name: String::new(),
                kind: 0,
                subpoly_index: -1,
                union_index: 0,
                arc_resolution: Coord::ZERO,
                is_shape_based: false,
                cavity_height: Coord::ZERO,
                standoff_height: b.standoff_height,
                overall_height: b.overall_height,
                body_projection: 0,
                body_color_3d: b.body_color_3d,
                body_opacity_3d: b.body_opacity_3d,
                identifier: String::new(),
                texture: String::new(),
                texture_center_x: Coord::ZERO,
                texture_center_y: Coord::ZERO,
                texture_size_x: Coord::ZERO,
                texture_size_y: Coord::ZERO,
                texture_rotation: 0.0,
                body_override_color: false,
                model_guid: String::new(),
                model_checksum: String::new(),
                model_embed: false,
                model_name: b.model_name.clone(),
                model_2d_x: Coord::ZERO,
                model_2d_y: Coord::ZERO,
                model_2d_rotation: 0.0,
                rotation_x: 0.0,
                rotation_y: 0.0,
                rotation_z: 0.0,
                model_3d_dz: Coord::ZERO,
                model_type: 0,
                model_source: String::new(),
                model_snap_points: Vec::new(),
                model_extruded_min_z: Coord::ZERO,
                model_extruded_max_z: Coord::ZERO,
                model_cylinder_radius: Coord::ZERO,
                model_cylinder_height: Coord::ZERO,
                model_sphere_radius: Coord::ZERO,
                outline: Contour::Legacy(b.outline.clone()),
                shape_text_segments: None,
                unique_id: None,
            }),
        })
        .collect()
}

// ── Field preservation ─────────────────────────────────────────────────────
//
// For each primitive type, copy format-internal fields (not exposed in the
// public API) from the old record at the same index. This preserves data
// like unique_ids, union indices, cache flags, etc. across roundtrips.

fn preserve_primitive_fields(
    old_records: &[ParsedPrimitiveRecord],
    new_records: &mut [ParsedPrimitiveRecord],
) {
    for (i, new_rec) in new_records.iter_mut().enumerate() {
        let Some(old_rec) = old_records.get(i) else {
            continue;
        };
        match (&mut new_rec.primitive, &old_rec.primitive) {
            (PcbPrimitive::Track(new), PcbPrimitive::Track(old)) => {
                preserve_track_fields(new, old);
            }
            (PcbPrimitive::Arc(new), PcbPrimitive::Arc(old)) => {
                preserve_arc_fields(new, old);
            }
            (PcbPrimitive::Via(new), PcbPrimitive::Via(old)) => {
                preserve_via_fields(new, old);
            }
            (PcbPrimitive::Pad(new), PcbPrimitive::Pad(old)) => {
                preserve_pad_fields(new, old);
            }
            (PcbPrimitive::Fill(new), PcbPrimitive::Fill(old)) => {
                preserve_fill_fields(new, old);
            }
            (PcbPrimitive::Text(new), PcbPrimitive::Text(old)) => {
                preserve_text_fields(new, old);
            }
            (PcbPrimitive::Region(new), PcbPrimitive::Region(old)) => {
                preserve_region_fields(new, old);
            }
            (PcbPrimitive::ComponentBody(new), PcbPrimitive::ComponentBody(old)) => {
                preserve_body_fields(new, old);
            }
            _ => {} // Type mismatch at same index — skip.
        }
    }
}

fn preserve_track_fields(new: &mut PcbTrack, old: &PcbTrack) {
    new.subpoly_index = old.subpoly_index;
    new.user_routed = old.user_routed;
    new.union_index = old.union_index;
    new.track_kind = old.track_kind;
    new.keepout_restrictions = old.keepout_restrictions;
}

fn preserve_arc_fields(new: &mut PcbArc, old: &PcbArc) {
    new.subpoly_index = old.subpoly_index;
    new.user_routed = old.user_routed;
    new.union_index = old.union_index;
    new.keepout_restrictions = old.keepout_restrictions;
}

fn preserve_via_fields(new: &mut PcbVia, old: &PcbVia) {
    // Thermal/mask cache fields
    new.via_properties_version = old.via_properties_version;
    new.thermal_relief_air_gap = old.thermal_relief_air_gap;
    new.thermal_relief_conductor_count = old.thermal_relief_conductor_count;
    new.thermal_relief_rotation_code = old.thermal_relief_rotation_code;
    new.thermal_relief_conductor_width = old.thermal_relief_conductor_width;
    new.power_plane_relief_expansion = old.power_plane_relief_expansion;
    new.power_plane_clearance = old.power_plane_clearance;
    new.paste_mask_expansion = old.paste_mask_expansion;
    new.planes = old.planes;
    // Cache validity flags
    new.plane_connection_style_valid = old.plane_connection_style_valid;
    new.relief_conductor_width_valid = old.relief_conductor_width_valid;
    new.relief_entries_valid = old.relief_entries_valid;
    new.relief_air_gap_valid = old.relief_air_gap_valid;
    new.power_plane_relief_expansion_valid = old.power_plane_relief_expansion_valid;
    new.paste_mask_expansion_valid = old.paste_mask_expansion_valid;
    new.solder_mask_expansion_valid = old.solder_mask_expansion_valid;
    new.power_plane_clearance_valid = old.power_plane_clearance_valid;
    new.planes_valid = old.planes_valid;
    new.plane_connection_style = old.plane_connection_style;
    new.solder_mask_cache_flags = old.solder_mask_cache_flags;
    new.solder_mask_expansion_state = old.solder_mask_expansion_state;
    new.paste_mask_cache_flags = old.paste_mask_cache_flags;
    new.paste_mask_expansion_state = old.paste_mask_expansion_state;
    // Via mode and per-layer data
    new.via_mode = old.via_mode;
    new.diameters_per_layer = old.diameters_per_layer;
    new.layer_enum_index = old.layer_enum_index;
    new.stack_start_layer = old.stack_start_layer;
    new.stack_end_layer = old.stack_end_layer;
    // Testpoint flags (not exposed in API)
    new.is_testpoint_top = old.is_testpoint_top;
    new.is_testpoint_bottom = old.is_testpoint_bottom;
    new.is_assy_testpoint_top = old.is_assy_testpoint_top;
    new.is_assy_testpoint_bottom = old.is_assy_testpoint_bottom;
    // Mask expansion details (solder_mask_override and solder_mask_expansion_front
    // are set from the API, but preserve the remaining mask fields)
    new.use_separate_solder_mask_expansion = old.use_separate_solder_mask_expansion;
    new.solder_mask_expansion_from_hole_edge = old.solder_mask_expansion_from_hole_edge;
    new.paste_mask_override = old.paste_mask_override;
    new.solder_mask_expansion_linked = old.solder_mask_expansion_linked;
    new.solder_mask_expansion_back = old.solder_mask_expansion_back;
    // Template link
    new.template_link_version = old.template_link_version;
    new.template_link_library_id = old.template_link_library_id;
    new.template_link_template_id = old.template_link_template_id;
    new.hole_positive_tolerance = old.hole_positive_tolerance;
    new.hole_negative_tolerance = old.hole_negative_tolerance;
    new.template_link_flags = old.template_link_flags;
    // Per-layer entries
    new.pad_layer_entries = old.pad_layer_entries.clone();
    new.pad_layer_stride = old.pad_layer_stride;
    new.layer_diameter_overrides = old.layer_diameter_overrides.clone();
    // Structure
    new.counter_hole_angle = old.counter_hole_angle;
    new.via_structure_type = old.via_structure_type;
    new.unique_id = old.unique_id.clone();
}

fn preserve_pad_fields(new: &mut PcbPad, old: &PcbPad) {
    new.unknown_sub1 = old.unknown_sub1.clone();
    new.unknown_sub2 = old.unknown_sub2.clone();
    new.unknown_sub3 = old.unknown_sub3.clone();
    new.daisy_chain_style = old.daisy_chain_style;
    new.unknown_63 = old.unknown_63;
    // Cache validity flags (values set from API, validity flags preserved)
    new.cache.plane_connection_style_valid = old.cache.plane_connection_style_valid;
    new.cache.relief_conductor_width_valid = old.cache.relief_conductor_width_valid;
    new.cache.relief_entries_valid = old.cache.relief_entries_valid;
    new.cache.relief_air_gap_valid = old.cache.relief_air_gap_valid;
    new.cache.power_plane_relief_expansion_valid = old.cache.power_plane_relief_expansion_valid;
    new.cache.paste_mask_expansion_valid = old.cache.paste_mask_expansion_valid;
    new.cache.solder_mask_expansion_valid = old.cache.solder_mask_expansion_valid;
    new.cache.power_plane_clearance_valid = old.cache.power_plane_clearance_valid;
    new.cache.planes_valid = old.cache.planes_valid;
    new.selection_memory_flags = old.selection_memory_flags;
    new.union_index = old.union_index;
    new.jumper_id = old.jumper_id;
    new.v7_layer_override = old.v7_layer_override;
    new.is_assy_testpoint_top = old.is_assy_testpoint_top;
    new.is_assy_testpoint_bottom = old.is_assy_testpoint_bottom;
    new.use_separate_expansions = old.use_separate_expansions;
    new.solder_mask_bottom_expansion = old.solder_mask_bottom_expansion;
    new.solder_mask_expansion_from_hole_edge = old.solder_mask_expansion_from_hole_edge;
    new.template_link_library_id = old.template_link_library_id;
    new.template_link_template_id = old.template_link_template_id;
    new.pin_package_length = old.pin_package_length;
    new.hole_positive_tolerance = old.hole_positive_tolerance;
    new.hole_negative_tolerance = old.hole_negative_tolerance;
    new.reserved_170 = old.reserved_170;
    new.has_sub4_extension = old.has_sub4_extension;
    new.sub4_extension = old.sub4_extension.clone();
    new.thermal_reliefs = old.thermal_reliefs.clone();
    new.stack_data = old.stack_data.clone();
    new.unique_id = old.unique_id.clone();
}

fn preserve_fill_fields(new: &mut PcbFill, old: &PcbFill) {
    new.user_routed = old.user_routed;
    new.union_index = old.union_index;
    new.keepout_restrictions = old.keepout_restrictions;
}

fn preserve_text_fields(new: &mut PcbText, old: &PcbText) {
    // Formatting fields not exposed in API
    new.stroke_font_type = old.stroke_font_type;
    new.text_kind = old.text_kind;
    new.user_routed = old.user_routed;
    new.is_bold = old.is_bold;
    new.is_italic = old.is_italic;
    new.is_inverted = old.is_inverted;
    new.margin_border_width = old.margin_border_width;
    new.union_index = old.union_index;
    new.is_inverted_rect = old.is_inverted_rect;
    new.textbox_rect_width = old.textbox_rect_width;
    new.textbox_rect_height = old.textbox_rect_height;
    new.textbox_rect_justification = old.textbox_rect_justification;
    new.text_offset_width = old.text_offset_width;
    new.unk_vec_x = old.unk_vec_x;
    new.unk_vec_y = old.unk_vec_y;
    // Barcode fields
    new.barcode_margin_x = old.barcode_margin_x;
    new.barcode_margin_y = old.barcode_margin_y;
    new.barcode_min_width = old.barcode_min_width;
    new.barcode_kind = old.barcode_kind;
    new.barcode_render_mode = old.barcode_render_mode;
    new.barcode_inverted = old.barcode_inverted;
    new.barcode_font_name = old.barcode_font_name.clone();
    new.barcode_min_pixel_size = old.barcode_min_pixel_size;
    new.barcode_show_text = old.barcode_show_text;
    // V7 layer data
    new.has_v7_layer_data = old.has_v7_layer_data;
    new.layer_enum_index = old.layer_enum_index;
    new.sentinel_1 = old.sentinel_1;
    new.sentinel_2 = old.sentinel_2;
    new.trailing_flag_1 = old.trailing_flag_1;
    new.trailing_flag_2 = old.trailing_flag_2;
    // Trailing optional fields
    new.trailing_is_justification_valid = old.trailing_is_justification_valid;
    new.advance_snapping = old.advance_snapping;
    new.advance_mode = old.advance_mode;
    new.advance_justification_x = old.advance_justification_x;
    new.advance_justification_y = old.advance_justification_y;
    new.use_text_alignment_by_snap = old.use_text_alignment_by_snap;
    new.snap_point_x = old.snap_point_x;
    new.snap_point_y = old.snap_point_y;
}

fn preserve_region_fields(new: &mut PcbRegion, old: &PcbRegion) {
    new.name = old.name.clone();
    new.param_kind = old.param_kind;
    new.subpoly_index = old.subpoly_index;
    new.union_index = old.union_index;
    new.arc_resolution = old.arc_resolution;
    new.is_shape_based = old.is_shape_based;
    new.cavity_height = old.cavity_height;
    new.keepout_restrictions = old.keepout_restrictions;
    new.layer = old.layer.clone();
    new.pad_index = old.pad_index;
    new.object_kind = old.object_kind.clone();
    new.bending_line_count = old.bending_line_count;
    new.locked_3d = old.locked_3d;
    new.layer_stack_id = old.layer_stack_id.clone();
    new.unique_id = old.unique_id.clone();
    // Preserve contour data to maintain the correct binary format (Legacy vs ShapeBased).
    // build_region_records always produces Contour::Legacy from the public API's Vec<CoordPoint>,
    // but ShapeBasedRegions6 records require Contour::ShapeBased (TPolySegment format).
    // Copying the original contours preserves the correct serialization format.
    new.outline = old.outline.clone();
    new.holes = old.holes.clone();
    new.shape_text_segments = old.shape_text_segments.clone();
    new.hole_shape_text_segments = old.hole_shape_text_segments.clone();
}

fn preserve_body_fields(new: &mut PcbComponentBody, old: &PcbComponentBody) {
    new.name = old.name.clone();
    new.kind = old.kind;
    new.subpoly_index = old.subpoly_index;
    new.union_index = old.union_index;
    new.arc_resolution = old.arc_resolution;
    new.is_shape_based = old.is_shape_based;
    new.cavity_height = old.cavity_height;
    new.body_projection = old.body_projection;
    new.identifier = old.identifier.clone();
    new.texture = old.texture.clone();
    new.texture_center_x = old.texture_center_x;
    new.texture_center_y = old.texture_center_y;
    new.texture_size_x = old.texture_size_x;
    new.texture_size_y = old.texture_size_y;
    new.texture_rotation = old.texture_rotation;
    new.body_override_color = old.body_override_color;
    new.model_guid = old.model_guid.clone();
    new.model_checksum = old.model_checksum.clone();
    new.model_embed = old.model_embed;
    new.model_2d_x = old.model_2d_x;
    new.model_2d_y = old.model_2d_y;
    new.model_2d_rotation = old.model_2d_rotation;
    new.rotation_x = old.rotation_x;
    new.rotation_y = old.rotation_y;
    new.rotation_z = old.rotation_z;
    new.model_3d_dz = old.model_3d_dz;
    new.model_type = old.model_type;
    new.model_source = old.model_source.clone();
    new.model_snap_points = old.model_snap_points.clone();
    new.model_extruded_min_z = old.model_extruded_min_z;
    new.model_extruded_max_z = old.model_extruded_max_z;
    new.model_cylinder_radius = old.model_cylinder_radius;
    new.model_cylinder_height = old.model_cylinder_height;
    new.model_sphere_radius = old.model_sphere_radius;
    new.unique_id = old.unique_id.clone();
}

// ── WideStrings6 rebuilding ────────────────────────────────────────────────

/// Build a new WideStrings6 section from all text primitives.
///
/// Returns the section data and a parallel Vec of wide_string_index values
/// (one per text). Deduplicates identical strings.
fn rebuild_wide_strings(texts: &[Text]) -> (WideStringsSectionData, Vec<i32>) {
    let mut string_to_index: HashMap<String, i32> = HashMap::new();
    let mut entries: Vec<WideString6Record> = Vec::new();
    let mut indices: Vec<i32> = Vec::with_capacity(texts.len());

    for text in texts {
        if text.text.is_empty() {
            indices.push(-1);
            continue;
        }
        let index = if let Some(&idx) = string_to_index.get(&text.text) {
            idx
        } else {
            let idx = entries.len() as i32;
            entries.push(WideString6Record {
                index: idx as u32,
                text: text.text.clone(),
            });
            string_to_index.insert(text.text.clone(), idx);
            idx
        };
        indices.push(index);
    }

    (WideStringsSectionData { entries }, indices)
}

// ── Ratsnest computation ───────────────────────────────────────────────────

/// Compute Connections6 ratsnest records from pad/net data.
///
/// Groups pads by net, skipping pads with no net. For each net with ≥2 pads,
/// produces a star topology: the pad closest to the geometric centroid is the
/// hub, with one `BinaryLenRecord` per remaining pad connecting to it.
/// Single-pad nets produce no records.
fn compute_ratsnest(board: &PcbDocBoard, ctx: &WriteContext) -> Vec<BinaryLenRecord> {
    // Group pad locations by net name.
    let mut net_pads: HashMap<&str, Vec<&Pad>> = HashMap::new();
    for pad in &board.pads {
        if let Some(net_name) = &pad.net {
            net_pads.entry(net_name.as_str()).or_default().push(pad);
        }
    }

    let mut connections = Vec::new();

    for (net_name, pads) in &net_pads {
        if pads.len() < 2 {
            continue;
        }

        let net_index = ctx.net_indices.get(*net_name).copied().unwrap_or(0xFFFF) as i16;

        // Compute geometric centroid of all pad locations in the net.
        let centroid_x: i64 = pads.iter().map(|p| p.location.x.raw() as i64).sum::<i64>()
            / pads.len() as i64;
        let centroid_y: i64 = pads.iter().map(|p| p.location.y.raw() as i64).sum::<i64>()
            / pads.len() as i64;

        // Pick the pad closest to the centroid as the hub.
        let hub_idx = pads
            .iter()
            .enumerate()
            .min_by_key(|(_, p)| {
                let dx = p.location.x.raw() as i64 - centroid_x;
                let dy = p.location.y.raw() as i64 - centroid_y;
                dx * dx + dy * dy
            })
            .map(|(i, _)| i)
            .unwrap_or(0);

        let hub = pads[hub_idx];
        let hub_layer = hub.layer.to_v6().unwrap_or(V6Layer::MultiLayer);

        for (i, pad) in pads.iter().enumerate() {
            if i == hub_idx {
                continue;
            }
            let pad_layer = pad.layer.to_v6().unwrap_or(V6Layer::MultiLayer);
            connections.push(BinaryLenRecord {
                common: ConnectionCommonHeader {
                    layer: V6Layer::MultiLayer,
                    flags: 0,
                    net_index,
                    unknown_1: 0,
                    component_index: -1,
                    polygon_index: -1,
                    unknown_2: 0,
                },
                from: hub.location,
                to: pad.location,
                from_layer: hub_layer,
                to_layer: pad_layer,
                connection_layer_enum: 0,
                from_layer_enum: 0,
                to_layer_enum: 0,
            });
        }
    }

    connections
}

// ── PrimitiveParameters (BOM data) generation ─────────────────────────────

/// Build PrimitiveParameterGroup records from component parameters.
///
/// Produces one group per component that has at least one parameter.
/// The group header contains `SOURCEDESIGNATOR` and `COUNT`.
/// Each parameter block contains `NAME`, `VALUE`, and `ISIMPORTED`.
fn build_primitive_parameters(components: &[PcbDocComponent]) -> Vec<PrimitiveParameterGroup> {
    components
        .iter()
        .filter(|c| !c.parameters.is_empty())
        .map(|comp| {
            let mut component_header = ParameterCollection::new();
            component_header.insert("SOURCEDESIGNATOR", comp.designator.clone());

            let parameters: Vec<ParameterCollection> = comp
                .parameters
                .iter()
                .map(|(name, value)| {
                    let mut p = ParameterCollection::new();
                    p.insert("NAME", name.clone());
                    p.insert("VALUE", value.clone());
                    p.insert("ISIMPORTED", "FALSE".to_string());
                    p
                })
                .collect();

            PrimitiveParameterGroup {
                component_header,
                parameters,
            }
        })
        .collect()
}

/// Replace the PrimitiveParameters section in `doc`, or append if not found.
fn replace_primitive_parameters_section(
    doc: &mut PcbDoc,
    groups: Vec<PrimitiveParameterGroup>,
) {
    for section in &mut doc.sections {
        if let PcbDocSection::PrimitiveParameters(pp) = section {
            pp.groups = groups;
            return;
        }
    }
    doc.sections.push(PcbDocSection::PrimitiveParameters(PrimitiveParametersSectionData {
        groups,
    }));
}

// ── UniqueIDPrimitiveInformation rebuild ───────────────────────────────────

/// Assigns unique IDs to any pad that lacks one, then rebuilds the
/// UniqueIDPrimitiveInformation parameter section from all pads.
///
/// Pre-existing pads preserve their unique IDs via `preserve_pad_fields`.
/// New pads (added by the apply pipeline, with index beyond the old Pads6 count)
/// arrive with `unique_id: None` and receive freshly generated IDs here.
///
/// The section is written as a standard param section:
/// Header=[u32 count], Data=[u32 len][|PRIMITIVEINDEX=N|PRIMITIVEOBJECTID=Pad|UNIQUEID=XXXXXXXX|]*N
fn assign_and_rebuild_unique_id_section(doc: &mut PcbDoc) {
    // Find the Pads6 section and assign unique IDs to any pad that lacks one.
    for section in doc.sections.iter_mut() {
        if let PcbDocSection::Primitive(prim) = section {
            if prim.kind == PrimitiveSectionKind::Pads6 {
                for record in prim.records.iter_mut() {
                    if let PcbPrimitive::Pad(pad) = &mut record.primitive {
                        if pad.unique_id.is_none() {
                            pad.unique_id = Some(generate_unique_id());
                        }
                    }
                }
                break;
            }
        }
    }

    // Build UniqueIDPrimitiveInformation records for all pads with a unique ID.
    let mut uid_records: Vec<StandardParamRecord> = Vec::new();
    for section in doc.sections.iter() {
        if let PcbDocSection::Primitive(prim) = section {
            if prim.kind == PrimitiveSectionKind::Pads6 {
                for (index, record) in prim.records.iter().enumerate() {
                    if let PcbPrimitive::Pad(pad) = &record.primitive {
                        if let Some(uid) = &pad.unique_id {
                            let mut params = ParameterCollection::new();
                            params.insert("PRIMITIVEINDEX", index.to_string());
                            params.insert("PRIMITIVEOBJECTID", PcbObjectId::Pad.to_string());
                            params.insert("UNIQUEID", uid.clone());
                            uid_records.push(StandardParamRecord { params });
                        }
                    }
                }
                break;
            }
        }
    }

    replace_param_section(doc, ParamSectionKind::UniqueIdPrimitiveInformation, uid_records);
}

// ── Section replacement helpers ────────────────────────────────────────────

/// Replace a parameter section by kind, or append if not found.
fn replace_param_section(
    doc: &mut PcbDoc,
    kind: ParamSectionKind,
    records: Vec<StandardParamRecord>,
) {
    for section in &mut doc.sections {
        if let PcbDocSection::Parameter(param) = section {
            if param.kind == kind {
                param.records = records;
                return;
            }
        }
    }
    // Not found — append.
    doc.sections
        .push(PcbDocSection::Parameter(ParamSectionData {
            kind,
            records,
        }));
}

/// Merge new records into a parameter section, preserving all fields from old records
/// that are not explicitly overwritten by the new records. Records are matched by a
/// key field (e.g. "SOURCEDESIGNATOR" for components, "NAME" for nets).
///
/// For matched records: old params are the base, new params overwrite on top.
/// For new records (no old match): used as-is.
/// Old records with no new match are removed.
fn merge_param_section(
    doc: &mut PcbDoc,
    kind: ParamSectionKind,
    new_records: Vec<StandardParamRecord>,
    match_key: &str,
) {
    // Extract old records from the existing section.
    let old_records = take_param_records(doc, kind);

    // Build a lookup from match_key value → old record params.
    let mut old_by_key: HashMap<String, &ParameterCollection> = HashMap::new();
    for old_rec in &old_records {
        if let Some(key_val) = old_rec.params.get(match_key) {
            old_by_key.insert(key_val.to_ascii_uppercase(), &old_rec.params);
        }
    }

    // Merge: for each new record, start with old params (if matched), then overlay new.
    let merged: Vec<StandardParamRecord> = new_records
        .into_iter()
        .map(|new_rec| {
            let key_val = new_rec.params.get(match_key).unwrap_or("");
            let key_upper = key_val.to_ascii_uppercase();

            if let Some(old_params) = old_by_key.get(&key_upper) {
                // Start with all old params, then overwrite with new values.
                let mut merged_params = (*old_params).clone();
                for (k, v) in new_rec.params.iter() {
                    merged_params.set(k, v.to_owned());
                }
                StandardParamRecord { params: merged_params }
            } else {
                // No old record — use new as-is.
                new_rec
            }
        })
        .collect();

    replace_param_section(doc, kind, merged);
}

/// Extract records from a parameter section, leaving an empty Vec in place.
fn take_param_records(doc: &mut PcbDoc, kind: ParamSectionKind) -> Vec<StandardParamRecord> {
    for section in &mut doc.sections {
        if let PcbDocSection::Parameter(param) = section {
            if param.kind == kind {
                return std::mem::take(&mut param.records);
            }
        }
    }
    Vec::new()
}

/// Take primitive records out of a section (replacing with empty Vec).
fn take_primitive_records(
    doc: &mut PcbDoc,
    kind: PrimitiveSectionKind,
) -> Vec<ParsedPrimitiveRecord> {
    for section in &mut doc.sections {
        if let PcbDocSection::Primitive(prim) = section {
            if prim.kind == kind {
                return std::mem::take(&mut prim.records);
            }
        }
    }
    Vec::new()
}

/// Replace a primitive section by kind, or append if not found.
fn put_primitive_records(
    doc: &mut PcbDoc,
    kind: PrimitiveSectionKind,
    records: Vec<ParsedPrimitiveRecord>,
) {
    for section in &mut doc.sections {
        if let PcbDocSection::Primitive(prim) = section {
            if prim.kind == kind {
                prim.records = records;
                return;
            }
        }
    }
    // Not found — append.
    doc.sections
        .push(PcbDocSection::Primitive(PrimitiveSectionData {
            kind,
            records,
        }));
}

/// Take old records, build new records with field preservation, then replace.
fn replace_primitives_with_preservation(
    doc: &mut PcbDoc,
    kind: PrimitiveSectionKind,
    mut new_records: Vec<ParsedPrimitiveRecord>,
) {
    let old_records = take_primitive_records(doc, kind);
    preserve_primitive_fields(&old_records, &mut new_records);
    put_primitive_records(doc, kind, new_records);
}

/// Replace the WideStrings6 section, or append if not found.
fn replace_wide_strings(doc: &mut PcbDoc, data: WideStringsSectionData) {
    for section in &mut doc.sections {
        if let PcbDocSection::WideStrings(ws) = section {
            *ws = data;
            return;
        }
    }
    doc.sections.push(PcbDocSection::WideStrings(data));
}

/// Replace a binary-len section by kind, or append if not found.
fn replace_binary_section(
    doc: &mut PcbDoc,
    kind: BinaryLenSectionKind,
    records: Vec<BinaryLenRecord>,
) {
    for section in &mut doc.sections {
        if let PcbDocSection::Binary(bin) = section {
            if bin.kind == kind {
                bin.records = records;
                return;
            }
        }
    }
    doc.sections.push(PcbDocSection::Binary(BinarySectionData { kind, records }));
}

/// Detect which section kind to write: prefer `modern` if it contains records, else `legacy`.
fn detect_section_kind(
    doc: &PcbDoc,
    modern: PrimitiveSectionKind,
    legacy: PrimitiveSectionKind,
) -> PrimitiveSectionKind {
    let has_modern_with_records = doc.sections.iter().any(|s| {
        matches!(s, PcbDocSection::Primitive(p) if p.kind == modern && !p.records.is_empty())
    });
    if has_modern_with_records {
        modern
    } else {
        legacy
    }
}
