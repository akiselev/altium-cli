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
    ClassMemberKind, DimensionKind, LayerRef, PlaneConnectionStyle, RegionKind, RuleKind,
    V6Layer, V7Layer,
};
use altium_format_types::{DielectricType, LayerStackStyle};

use crate::api::pcb_common::{extract_pad_stack, contour_to_pcb_contour};
use crate::api::pcbdoc_types::*;
use crate::board_config::PcbBoardConfig;
use crate::pcbdoc::primitives::PcbPrimitive;
use crate::pcbdoc::records::{
    ParamSectionKind, PrefixedParamSectionKind, PrimitiveSectionKind,
};
use crate::pcbdoc::{
    ModelsSectionData, ParamSectionData, PcbDoc, PcbDocSection, PrefixedParamSectionData,
    PrimitiveSectionData, WideStringsSectionData,
};
use crate::pcbdoc::drc::PcbRuleKindData;
use crate::pcblib::{Contour, PcbComponentBody, PcbPad, PcbRegion, PcbVia};
use crate::param_value::MilCoord;
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
    let mut internal_regions: Vec<&PcbRegion> = Vec::new();

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

            // Collect internal regions for geometry extraction (before flattening).
            for record in &prim.records {
                if let PcbPrimitive::Region(r) = &record.primitive {
                    internal_regions.push(r);
                }
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

    // Step 4: Extract board geometry from internal regions (arc-preserving).
    let geometry = extract_board_geometry(&internal_regions);
    let board_outline = find_board_outline(&regions);
    let mut settings = settings;
    settings.board_outline = board_outline;
    settings.geometry = geometry;

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
        let x: MilCoord = params
            .remove_with_default("X", MilCoord::default())
            .context("Components6 X")?;
        let y: MilCoord = params
            .remove_with_default("Y", MilCoord::default())
            .context("Components6 Y")?;
        let rotation: f64 = params
            .remove_with_default("ROTATION", 0.0)
            .context("Components6 rotation")?;
        let layer = parse_layer_param(&mut params, V6Layer::TopLayer)
            .context("Components6 layer")?;

        let id = designator.clone();
        components.push(PcbDocComponent {
            id,
            designator,
            pattern,
            comment,
            location: CoordPoint::new(x.0, y.0),
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
        let layer = parse_layer_param(&mut params, V6Layer::TopLayer)
            .context("Polygons6 layer")?;
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
        let layer = parse_layer_param(&mut params, V6Layer::NoLayer)
            .context("Dimensions6 layer")?;
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
                scope2: rule.base.scope2_expression.clone(),
                net_scope: rule.base.net_scope,
                layer_scope: rule.base.layer_kind,
                comment: rule.base.comment.clone(),
                params: rule_params_from_internal(&rule.kind_data, rule.base.rule_kind),
            }
        })
        .collect()
}

fn rule_params_from_internal(kind_data: &PcbRuleKindData, rule_kind: RuleKind) -> RuleParams {
    match kind_data {
        PcbRuleKindData::Clearance(d) => RuleParams::Clearance {
            gap: d.gap.0,
            ignore_pad_to_pad: d.ignore_pad_to_pad,
        },
        PcbRuleKindData::Width(d) => RuleParams::Width {
            min: d.min_limit.0,
            max: d.max_limit.0,
            preferred: d.preferred_width.0,
        },
        PcbRuleKindData::Length(d) => RuleParams::Length {
            min: d.min_limit.0,
            max: d.max_limit.0,
        },
        PcbRuleKindData::MatchedLengths(d) => RuleParams::MatchedLengths {
            tolerance: d.tolerance.0,
        },
        PcbRuleKindData::ParallelSegment(d) => RuleParams::ParallelSegment {
            gap: d.gap.0,
            limit: d.limit.0,
            parallel_length: d.parallel_length.0,
        },
        PcbRuleKindData::DaisyChainStubLength(d) => RuleParams::DaisyChainStubLength {
            max_limit: d.max_limit.0,
        },
        PcbRuleKindData::ShortCircuit(d) => RuleParams::ShortCircuit {
            allowed: d.allowed,
        },
        PcbRuleKindData::BrokenNets(d) => RuleParams::BrokenNets {
            check_bad_connections: d.check_bad_connections,
        },
        PcbRuleKindData::ViasUnderSmd(d) => RuleParams::ViasUnderSmd {
            allowed: d.allowed,
        },
        PcbRuleKindData::MaximumViaCount(d) => RuleParams::MaximumViaCount {
            max_via_count: d.max_via_count,
        },
        PcbRuleKindData::MinimumAnnularRing(d) => RuleParams::MinimumAnnularRing {
            min: d.min_limit.0,
        },
        PcbRuleKindData::HoleToHoleClearance(d) => RuleParams::HoleToHoleClearance {
            gap: d.gap.0,
        },
        PcbRuleKindData::BoardOutlineClearance(d) => RuleParams::BoardOutlineClearance {
            gap: d.gap.0,
        },
        PcbRuleKindData::MaxMinHoleSize(d) => RuleParams::MaxMinHoleSize {
            min: d.min_limit.0,
            max: d.max_limit.0,
        },
        PcbRuleKindData::SolderMaskExpansion(d) => RuleParams::SolderMaskExpansion {
            expansion: d.expansion.0,
            is_tenting_top: d.is_tenting_top,
            is_tenting_bottom: d.is_tenting_bottom,
        },
        PcbRuleKindData::PasteMaskExpansion(d) => RuleParams::PasteMaskExpansion {
            expansion: d.expansion.0,
            percent: d.percent,
        },
        PcbRuleKindData::PowerPlaneClearance(d) => RuleParams::PowerPlaneClearance {
            clearance: d.clearance.0,
        },
        PcbRuleKindData::PowerPlaneConnectStyle(d) => RuleParams::PowerPlaneConnectStyle {
            connect_style: d.connect_style.unwrap_or_default(),
            relief_conductor_width: d.relief_conductor_width.map(|m| m.0).unwrap_or(Coord::ZERO),
            relief_entries: d.relief_entries.unwrap_or(4),
            relief_air_gap: d.relief_air_gap.map(|m| m.0).unwrap_or(Coord::ZERO),
        },
        PcbRuleKindData::PolygonConnectStyle(d) => RuleParams::PolygonConnectStyle {
            connect_style: d.connect_style.unwrap_or_default(),
            relief_conductor_width: d.relief_conductor_width.map(|m| m.0).unwrap_or(Coord::ZERO),
            relief_entries: d.relief_entries.unwrap_or(4),
            relief_angle: d.polygon_relief_angle.unwrap_or_default(),
            air_gap_width: d.air_gap_width.map(|m| m.0).unwrap_or(Coord::ZERO),
        },
        PcbRuleKindData::RoutingTopology(d) => RuleParams::RoutingTopology {
            topology: d.topology,
        },
        PcbRuleKindData::RoutingPriority(d) => RuleParams::RoutingPriority {
            priority: d.routing_priority,
        },
        PcbRuleKindData::RoutingLayers(d) => RuleParams::RoutingLayers {
            layer_flags: d.layer_flags.clone(),
        },
        PcbRuleKindData::RoutingCornerStyle(d) => RuleParams::RoutingCornerStyle {
            corner_style: d.corner_style,
            min_setback: d.min_setback.0,
            max_setback: d.max_setback.0,
        },
        PcbRuleKindData::RoutingViaStyle(d) => RuleParams::RoutingViaStyle {
            min_hole_width: d.min_hole_width.0,
            max_hole_width: d.max_hole_width.0,
            preferred_hole_width: d.preferred_hole_width.0,
            min_width: d.min_width.0,
            max_width: d.max_width.0,
            preferred_width: d.preferred_width.0,
            via_style: d.via_style,
        },
        PcbRuleKindData::ComponentClearance(d) => RuleParams::ComponentClearance {
            gap: d.gap.0,
            collision_check_mode: d.collision_check_mode,
            vertical_gap: d.vertical_gap.0,
        },
        PcbRuleKindData::ConfinementConstraint(d) => RuleParams::ConfinementConstraint {
            confinement_style: d.confinement_style,
        },
        PcbRuleKindData::DifferentialPairsRouting(d) => RuleParams::DiffPairsRouting {
            min_gap: d.min_limit.0,
            max_gap: d.max_limit.0,
            preferred_gap: d.most_freq_gap.0,
            max_uncoupled_length: d.max_uncoupled_length.0,
        },
        PcbRuleKindData::FanoutControl(d) => RuleParams::FanoutControl {
            bga_dir: d.bga_dir,
            bga_via_mode: d.bga_via_mode,
            fanout_style: d.fanout_style,
            fanout_direction: d.fanout_direction,
        },
        PcbRuleKindData::MaxMinHeight(d) => RuleParams::MaxMinHeight {
            min_height: d.min_height.0,
            max_height: d.max_height.0,
            pref_height: d.pref_height.0,
        },
        PcbRuleKindData::MinimumSolderMaskSliver(d) => RuleParams::MinimumSolderMaskSliver {
            min_width: d.min_solder_mask_width.0,
        },
        PcbRuleKindData::SilkToSolderMaskClearance(d) => RuleParams::SilkToSolderMaskClearance {
            gap: d.min_silkscreen_to_mask_gap.0,
        },
        PcbRuleKindData::SilkToSilkClearance(d) => RuleParams::SilkToSilkClearance {
            gap: d.silk_to_silk_clearance.0,
        },
        PcbRuleKindData::NetAntennae(d) => RuleParams::NetAntennae {
            tolerance: d.net_antennae_tolerance.0,
        },
        PcbRuleKindData::SmdToCorner(d) => RuleParams::SmdToCorner {
            distance: d.distance.0,
        },
        PcbRuleKindData::SmdToPlane(d) => RuleParams::SmdToPlane {
            distance: d.distance.0,
        },
        PcbRuleKindData::SmdNeckDown(d) => RuleParams::SmdNeckDown {
            percent: d.percent,
        },
        PcbRuleKindData::SmdEntry(d) => RuleParams::SmdEntry {
            side: d.side,
            corner: d.corner,
            any_angle: d.any_angle,
        },
        PcbRuleKindData::UnpouredPolygon(d) => RuleParams::UnpouredPolygon {
            allow_unpoured: d.allow_unpoured,
        },
        PcbRuleKindData::BackDrilling(d) => RuleParams::BackDrilling {
            depth: d.backdrill_depth.0,
        },
        PcbRuleKindData::Creepage(d) => RuleParams::CreepageDistance {
            gap: d.gap.0,
        },
        PcbRuleKindData::AcuteAngle(d) => RuleParams::AcuteAngle {
            minimum: d.minimum,
        },
        PcbRuleKindData::LayerPair(d) => RuleParams::LayerPair {
            enforce: d.enforce,
        },
        // All other variants fall through to Other
        _ => RuleParams::Other { kind: rule_kind },
    }
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
        layer_stack: LayerStack {
            style: LayerStackStyle::Pairs,
            is_flex: false,
            layers: Vec::new(),
            copper_layer_count: 0,
        },
        geometry: BoardGeometry {
            outline: None,
            cutouts: Vec::new(),
            keepouts: Vec::new(),
        },
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
            let snap: f64 = params
                .remove_with_default("SNAPGRIDSIZE", 100_000.0)
                .context("Board6 snap grid size")?;
            settings.snap_grid_size = Coord::from_internal(snap.round() as i32);
            let vis: f64 = params
                .remove_with_default("VISIBLEGRIDSIZE", 100_000.0)
                .context("Board6 visible grid size")?;
            settings.visible_grid_size = Coord::from_internal(vis.round() as i32);
            let unit_raw: u8 = params
                .remove_with_default("DISPLAYUNIT", 1u8)
                .context("Board6 display unit")?;
            settings.display_unit = Unit::try_from(unit_raw).unwrap_or(Unit::Imperial);

            // Parse full board config for layer stack extraction.
            // Re-parse from the original params (the above consumed only a few keys).
            let mut config_params = first.params.clone();
            let config = crate::board_config::parse_board_config(&mut config_params)
                .context("Board6 layer stack config")?;
            settings.layer_stack = extract_layer_stack(&config);
        }
    }

    Ok(settings)
}

// ── Layer stack extraction ──────────────────────────────────────────────────

/// Extract a unified `LayerStack` from whichever layer stack version is present.
/// Priority: V9 > V8 > V7 > legacy (first non-empty wins).
fn extract_layer_stack(config: &PcbBoardConfig) -> LayerStack {
    // Try V9 first
    if !config.v9_stack_layers.is_empty() {
        return extract_layer_stack_v9(config);
    }
    // Try V8
    if !config.v8_layers.is_empty() {
        return extract_layer_stack_v8(config);
    }
    // Try V7
    if !config.v7_layers.is_empty() {
        return extract_layer_stack_v7(config);
    }
    // Try legacy
    if !config.legacy_layers.is_empty() {
        return extract_layer_stack_legacy(config);
    }
    // Empty stack
    LayerStack {
        style: LayerStackStyle::Pairs,
        is_flex: false,
        layers: Vec::new(),
        copper_layer_count: 0,
    }
}

fn extract_layer_stack_v9(config: &PcbBoardConfig) -> LayerStack {
    let (style, is_flex) = config.v9_master_stack.as_ref().map_or(
        (LayerStackStyle::Pairs, false),
        |ms| (ms.style, ms.is_flex),
    );

    let mut layers: Vec<StackLayer> = config
        .v9_stack_layers
        .iter()
        .filter(|l| is_copper_layer_id(l.layer_id))
        .enumerate()
        .map(|(i, l)| stack_layer_from_v9_v8(l, i + 1))
        .collect();

    let copper_count = layers.len();
    // Layers are already in order (top → bottom) as stored in the V9 array.
    // Re-number physical_order to be safe.
    for (i, layer) in layers.iter_mut().enumerate() {
        layer.physical_order = i + 1;
    }

    LayerStack {
        style,
        is_flex,
        layers,
        copper_layer_count: copper_count,
    }
}

fn extract_layer_stack_v8(config: &PcbBoardConfig) -> LayerStack {
    let (style, is_flex) = config.v8_master_stack.as_ref().map_or(
        (LayerStackStyle::Pairs, false),
        |ms| (ms.style, ms.is_flex),
    );

    let mut layers: Vec<StackLayer> = config
        .v8_layers
        .iter()
        .filter(|l| is_copper_layer_id(l.layer_id))
        .enumerate()
        .map(|(i, l)| stack_layer_from_v9_v8(l, i + 1))
        .collect();

    let copper_count = layers.len();
    for (i, layer) in layers.iter_mut().enumerate() {
        layer.physical_order = i + 1;
    }

    LayerStack {
        style,
        is_flex,
        layers,
        copper_layer_count: copper_count,
    }
}

fn extract_layer_stack_v7(config: &PcbBoardConfig) -> LayerStack {
    // V7 layers use prev/next linked-list. Walk from top (prev == -1).
    let layers_map: std::collections::HashMap<i32, &crate::board_config::PcbV7LayerEntry> =
        config.v7_layers.iter().map(|l| (l.layer_id, l)).collect();

    // Find the head: layer with prev == -1
    let head = config.v7_layers.iter().find(|l| l.prev == -1);

    let mut ordered = Vec::new();
    if let Some(start) = head {
        let mut current = Some(start);
        while let Some(layer) = current {
            if is_copper_layer_id(layer.layer_id) {
                ordered.push(StackLayer {
                    layer: layer_ref_from_id(layer.layer_id),
                    name: layer.name.clone(),
                    physical_order: ordered.len() + 1,
                    is_plane: is_internal_plane_id(layer.layer_id),
                    copper_thickness: layer.cop_thick,
                    dielectric_type: layer.diel_type,
                    dielectric_constant: layer.diel_const,
                    dielectric_height: layer.diel_height,
                    dielectric_material: layer.diel_material.clone(),
                    component_placement: None,
                });
            }
            current = if layer.next >= 0 {
                layers_map.get(&layer.next).copied()
            } else {
                None
            };
        }
    }

    let copper_count = ordered.len();
    LayerStack {
        style: LayerStackStyle::Pairs,
        is_flex: false,
        layers: ordered,
        copper_layer_count: copper_count,
    }
}

fn extract_layer_stack_legacy(config: &PcbBoardConfig) -> LayerStack {
    // Legacy layers also use prev/next linked-list, but don't have layer_id.
    // They're indexed 1-82 in the file. We walk by prev/next on index position.
    let mut layers: Vec<StackLayer> = Vec::new();

    // Find head: layer with prev == 0 (legacy uses 0 for "none")
    // Legacy layers are stored in order already — index 1 = Top Layer, etc.
    // Just iterate in order and filter copper layers.
    for (idx, leg) in config.legacy_layers.iter().enumerate() {
        let layer_num = idx + 1; // 1-based
        if is_copper_layer_num(layer_num) {
            layers.push(StackLayer {
                layer: layer_ref_from_num(layer_num),
                name: leg.name.clone(),
                physical_order: layers.len() + 1,
                is_plane: is_internal_plane_num(layer_num),
                copper_thickness: leg.cop_thick,
                dielectric_type: leg.diel_type,
                dielectric_constant: leg.diel_const,
                dielectric_height: leg.diel_height,
                dielectric_material: leg.diel_material.clone(),
                component_placement: None,
            });
        }
    }

    let copper_count = layers.len();
    LayerStack {
        style: LayerStackStyle::Pairs,
        is_flex: false,
        layers,
        copper_layer_count: copper_count,
    }
}

fn stack_layer_from_v9_v8(
    l: &crate::board_config::PcbStackLayerEntry,
    physical_order: usize,
) -> StackLayer {
    StackLayer {
        layer: layer_ref_from_id(l.layer_id),
        name: l.name.clone(),
        physical_order,
        is_plane: is_internal_plane_id(l.layer_id),
        copper_thickness: l.cop_thick.unwrap_or(Coord::ZERO),
        dielectric_type: l.diel_type.unwrap_or(DielectricType::NoDielectric),
        dielectric_constant: l.diel_const.unwrap_or(0.0),
        dielectric_height: l.diel_height.unwrap_or(Coord::ZERO),
        dielectric_material: l.diel_material.clone().unwrap_or_default(),
        component_placement: l.component_placement,
    }
}

/// Check if a layer_id corresponds to a copper layer.
///
/// Handles both V6 (family=0, genus=0) and V7/V9 (family=1) layer IDs.
/// V9 genus encoding: 0=signal copper, 1=internal plane, 3=utility, 4=dielectric.
/// Copper = signal (genus=0) OR internal plane (genus=1).
fn is_copper_layer_id(id: i32) -> bool {
    let v7 = V7Layer::new(id as u32);
    // V6-compatible IDs (genus=0, family=0): check via V6Layer
    if let Ok(v6) = v7.to_v6() {
        return v6.is_copper();
    }
    // V9: genus=0 (signal copper) or genus=1 (internal plane) — both are copper
    matches!(v7.genus(), 0 | 1)
}

/// Check if a layer_id is an internal plane layer.
///
/// Handles both V6 (39..=54) and V7/V9 (family=1, genus=1) layer IDs.
fn is_internal_plane_id(id: i32) -> bool {
    let v7 = V7Layer::new(id as u32);
    // V6-compatible IDs: check via V6Layer
    if let Ok(v6) = v7.to_v6() {
        return v6.is_internal_plane();
    }
    // V9: genus=1 means internal plane layer
    v7.genus() == 1
}

fn is_copper_layer_num(num: usize) -> bool {
    // In legacy layers, positions 1..=32 are copper
    (1..=32).contains(&num)
}

fn is_internal_plane_num(_num: usize) -> bool {
    // In legacy encoding, internal planes are not distinguishable by index alone.
    // They use the same positions as mid layers. We'd need to check layer name.
    false
}

fn layer_ref_from_id(id: i32) -> LayerRef {
    let v7 = V7Layer::new(id as u32);
    if let Ok(v6) = v7.to_v6() {
        LayerRef::from_v6(v6)
    } else {
        LayerRef::from_v7(v7)
    }
}

fn layer_ref_from_num(num: usize) -> LayerRef {
    // Legacy layer numbering: 1=Top, 2=MidLayer1, ..., 32=Bottom
    layer_ref_from_id(num as i32)
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
    let stack = extract_pad_stack(p);
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
        stack,
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

fn extract_board_geometry(internal_regions: &[&PcbRegion]) -> BoardGeometry {
    let mut outline: Option<BoardContour> = None;
    let mut cutouts = Vec::new();
    let mut keepouts = Vec::new();

    for r in internal_regions {
        if r.is_board_cutout || r.kind == RegionKind::BoardCutout {
            let contour = contour_to_pcb_contour(&r.outline);
            if outline.is_none() {
                outline = Some(contour);
            } else {
                cutouts.push(contour);
            }
        } else if r.keepout {
            let layer = if !r.v7_layer.is_empty() {
                LayerRef::from_string_name(&r.v7_layer)
                    .unwrap_or_else(|| LayerRef::from_v6(r.common.layer))
            } else {
                LayerRef::from_v6(r.common.layer)
            };
            keepouts.push(KeepoutZone {
                outline: contour_to_pcb_contour(&r.outline),
                layer,
            });
        }
    }

    BoardGeometry {
        outline,
        cutouts,
        keepouts,
    }
}

// contour_to_pcb_contour is imported from pcb_common

/// Parse a LAYER parameter that may be either a numeric V6 layer ID (e.g. "1")
/// or a string layer name (e.g. "TopLayer", "MULTILAYER", "TOP").
/// Falls back to `default` if the key is absent or the name is unrecognized.
fn parse_layer_param(
    params: &mut crate::param_collection::ParameterCollection,
    default: V6Layer,
) -> Result<LayerRef> {
    let raw: String = params
        .remove_with_default("LAYER", String::new())?;
    if raw.is_empty() {
        return Ok(LayerRef::from_v6(default));
    }
    // Try numeric V6 layer ID first.
    if let Ok(id) = raw.parse::<u8>() {
        return Ok(match V6Layer::try_from(id) {
            Ok(v6) => LayerRef::from_v6(v6),
            Err(_) => LayerRef::from_v6(default),
        });
    }
    // Try canonical layer name lookup (case-insensitive, e.g. "TopLayer", "MULTILAYER").
    if let Some(layer) = LayerRef::from_string_name(&raw) {
        return Ok(layer);
    }
    // Try abbreviated names used in some newer Altium files (e.g. "TOP", "BOTTOM").
    let layer = match raw.to_ascii_uppercase().as_str() {
        "TOP" => LayerRef::from_v6(V6Layer::TopLayer),
        "BOTTOM" => LayerRef::from_v6(V6Layer::BottomLayer),
        _ => LayerRef::from_v6(default),
    };
    Ok(layer)
}
