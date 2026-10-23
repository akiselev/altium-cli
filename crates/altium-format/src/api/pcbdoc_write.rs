//! Write path: convert public API types → internal PcbDoc sections.
//!
//! The core function `board_to_internal` takes a `PcbDocBoard` and updates
//! the internal `PcbDoc` sections in place. Parameter sections are rebuilt
//! from scratch; primitive sections preserve format-internal fields from
//! existing records matched by position.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use altium_format_types::coord::{Coord, CoordPoint};
use altium_format_types::pcb::constants::{POLYGON_INDEX_PLANE_SPLIT_LINE, PRIMITIVE_INDEX_NONE};
use altium_format_types::pcb::{
    BarcodeRenderMode, DaisyChainStyle, LayerRef, MaskExpansionState, PadStackMode, PcbFlags,
    PlaneConnectionStyle, PolySegmentKind, PolygonType, TCacheState, TextKind, V6Layer,
};
use altium_format_types::{BarcodeKind, PcbObjectId};

use crate::api::pcb_common::{
    ContourSegment, PcbContour, mask_cache_state_for_write, merge_mask_cache_state,
};
use crate::api::pcbdoc_read::{
    hatch_style_to_param_value, layer_to_param_value, polygon_type_to_param_value,
    strip_param_contour, validate_unique_id,
};
use crate::api::pcbdoc_types::*;
use crate::param_collection::ParameterCollection;
use crate::param_value::{ExactMilCoord, MilCoord, ToParamValue};
use crate::pcbdoc::primitives::{
    ParsedPrimitiveRecord, PcbArc, PcbFill, PcbPrimitive, PcbText, PcbTrack,
};
use crate::pcbdoc::records::PrimitiveParameterGroup;
use crate::pcbdoc::records::{
    BinaryLenRecord, BinaryLenSectionKind, ConnectionCommonHeader, ParamSectionKind,
    PrimitiveSectionKind, StandardParamRecord, WideString6Record,
};
use crate::pcbdoc::{
    BinarySectionData, ParamSectionData, PcbDoc, PcbDocSection, PrimitiveParametersSectionData,
    PrimitiveSectionData, WideStringsSectionData,
};
use crate::pcblib::wide_strings::encode_text_for_wide_strings;
use crate::pcblib::{
    Contour, PcbComponentBody, PcbPad, PcbPadCache, PcbPrimitiveCommon, PcbRegion, PcbVia,
    PolySegment,
};
use crate::{Result, ResultExt};

// ── Write context ──────────────────────────────────────────────────────────

/// Inverse of `ConvertContext` in `pcbdoc_read.rs`: maps names back to indices.
struct WriteContext {
    /// Net name → section index (position in Nets6).
    net_indices: HashMap<String, u16>,
    /// Components6 position → designator (designators need not be unique).
    component_designators: Vec<String>,
    /// Number of polygons written to Polygons6 (range of `polygon` indices).
    polygon_count: usize,
}

impl WriteContext {
    fn new(board: &PcbDocBoard) -> Self {
        let mut net_indices = HashMap::with_capacity(board.nets.len());
        for (i, net) in board.nets.iter().enumerate() {
            net_indices.insert(net.name.clone(), i as u16);
        }

        Self {
            net_indices,
            component_designators: board
                .components
                .iter()
                .map(|c| c.designator.clone())
                .collect(),
            polygon_count: board.polygons.len(),
        }
    }

    fn resolve_net_index(&self, net: &Option<String>) -> u16 {
        match net {
            Some(name) => self
                .net_indices
                .get(name.as_str())
                .copied()
                .unwrap_or(PRIMITIVE_INDEX_NONE),
            None => PRIMITIVE_INDEX_NONE,
        }
    }

    /// Resolve a primitive's polygon membership to its Polygons6 index.
    fn resolve_polygon_index(
        &self,
        polygon: Option<usize>,
        plane_split_line: bool,
        layer: &LayerRef,
    ) -> Result<u16> {
        if plane_split_line {
            let on_plane = layer.legacy_v6().is_some_and(V6Layer::is_internal_plane);
            if polygon.is_some() || !on_plane {
                return Err(crate::AltiumFormatError::InvalidParamValue {
                    key: "polygon".to_owned(),
                    detail: format!(
                        "plane_split_line needs an internal plane layer and no polygon \
                         (layer {layer}, polygon {polygon:?})"
                    ),
                });
            }
            return Ok(POLYGON_INDEX_PLANE_SPLIT_LINE);
        }
        let Some(i) = polygon else {
            return Ok(PRIMITIVE_INDEX_NONE);
        };
        u16::try_from(i)
            .ok()
            .filter(|_| i < self.polygon_count)
            .ok_or_else(|| crate::AltiumFormatError::InvalidParamValue {
                key: "polygon".to_owned(),
                detail: format!(
                    "polygon {i} outside the board's {} polygons",
                    self.polygon_count
                ),
            })
    }

    /// Resolve a primitive's owning component to its Components6 index.
    ///
    /// `component_index` wins when set; it must be in range and, if
    /// `component` is also set, name the same designator. Without an index the
    /// designator must match exactly one component. Anything else is an error.
    fn resolve_component_index(
        &self,
        component: &Option<String>,
        component_index: Option<usize>,
    ) -> Result<u16> {
        let error = |detail: String| crate::AltiumFormatError::InvalidParamValue {
            key: "component".to_owned(),
            detail,
        };
        let index = match (component_index, component) {
            (None, None) => return Ok(PRIMITIVE_INDEX_NONE),
            (Some(i), designator) => {
                let stored = self.component_designators.get(i).ok_or_else(|| {
                    error(format!(
                        "component_index {i} outside the board's {} components",
                        self.component_designators.len()
                    ))
                })?;
                if let Some(name) = designator {
                    if name != stored {
                        return Err(error(format!(
                            "component {name:?} disagrees with component_index {i} ({stored:?})"
                        )));
                    }
                }
                i
            }
            (None, Some(name)) => {
                let mut matches = self
                    .component_designators
                    .iter()
                    .enumerate()
                    .filter(|(_, d)| *d == name)
                    .map(|(i, _)| i);
                match (matches.next(), matches.next()) {
                    (Some(i), None) => i,
                    (None, _) => return Err(error(format!("no component {name:?}"))),
                    (Some(_), Some(_)) => {
                        return Err(error(format!(
                            "designator {name:?} names several components; set component_index"
                        )));
                    }
                }
            }
        };
        u16::try_from(index)
            .ok()
            .filter(|&i| i != PRIMITIVE_INDEX_NONE)
            .ok_or_else(|| {
                error(format!(
                    "component index {index} does not fit the u16 field"
                ))
            })
    }
}

/// The V6 layer to store in a primitive's common header: the V6 form of a
/// V6-compatible layer, or the V6 layer Altium stores beside a full V7 id
/// ([`V7Layer::legacy_v6`](altium_format_types::pcb::V7Layer::legacy_v6), e.g.
/// `0x0100_FFFF` → BottomLayer). Before, primitives with a full V7 id were
/// written with `NoLayer`.
fn layer_v6_for_write(layer: &LayerRef) -> Result<V6Layer> {
    layer
        .legacy_v6()
        .ok_or_else(|| crate::AltiumFormatError::InvalidParamValue {
            key: "layer".to_owned(),
            detail: format!(
                "layer {layer} (V7 id {:#010x}) has no V6 layer to store",
                layer.v7().raw()
            ),
        })
}

/// Give the document's Pads6 records the unique ids its
/// UniqueIDPrimitiveInformation section stores (the parser leaves them in the
/// section), so pad field preservation can match old pads by id.
fn attach_pad_unique_ids(doc: &mut PcbDoc) -> Result<()> {
    let ids = crate::api::pcbdoc_read::convert_unique_id_primitive_information(doc)?;
    for section in &mut doc.sections {
        if let PcbDocSection::Primitive(prim) = section {
            if prim.kind == PrimitiveSectionKind::Pads6 {
                for (i, record) in prim.records.iter_mut().enumerate() {
                    if let PcbPrimitive::Pad(pad) = &mut record.primitive {
                        if pad.unique_id.is_none() {
                            pad.unique_id = ids.get(&i).cloned();
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// Build a `PcbPrimitiveCommon` for a board context with net/component resolution.
fn primitive_common_for_board(
    layer: &LayerRef,
    net: &Option<String>,
    component: &Option<String>,
    component_index: Option<usize>,
    flags: PcbFlags,
    polygon: Option<usize>,
    plane_split_line: bool,
    ctx: &WriteContext,
) -> Result<PcbPrimitiveCommon> {
    Ok(PcbPrimitiveCommon {
        layer: layer_v6_for_write(layer)?,
        flags,
        net_index: ctx.resolve_net_index(net),
        polygon_index: ctx.resolve_polygon_index(polygon, plane_split_line, layer)?,
        component_index: ctx.resolve_component_index(component, component_index)?,
        coordinate_index: PRIMITIVE_INDEX_NONE,
        dimension_index: PRIMITIVE_INDEX_NONE,
    })
}

/// The writer stores unique ids for pads only (UniqueIDPrimitiveInformation is
/// rebuilt from Pads6); an id on another primitive kind is an error, not dropped.
fn reject_unique_id(kind: &str, unique_id: &Option<String>) -> Result<()> {
    match unique_id {
        None => Ok(()),
        Some(id) => Err(crate::AltiumFormatError::InvalidParamValue {
            key: "UNIQUEID".to_owned(),
            detail: format!("writing unique id {id:?} for a {kind} is not supported (pads only)"),
        }),
    }
}

// ── Stored DRC violations ──────────────────────────────────────────────────

/// Stored violations are read-only. Per list (violations, waived violations):
/// an empty list clears the stored records (`false`); a list equal to what the
/// document holds keeps them (`true`) once every rule and object index is in
/// range of the board being written; anything else is an error, because the
/// records cannot be rebuilt from the public form.
fn check_stored_violations(board: &PcbDocBoard, doc: &PcbDoc) -> Result<(bool, bool)> {
    use super::pcbdoc_read::{convert_violations, convert_waived_violations};
    let keep_violations = if board.violations.is_empty() {
        false
    } else if board.violations == convert_violations(doc).context("stored violations")? {
        for (i, v) in board.violations.iter().enumerate() {
            check_violation_indices(board, v.rule_index, &v.objects)
                .with_context(|| format!("stored violation #{i}"))?;
        }
        true
    } else {
        return Err(crate::AltiumFormatError::InvalidParamValue {
            key: "violations".to_owned(),
            detail: "stored violations are read-only: keep them as read, or empty the list \
                     to clear them"
                .to_owned(),
        });
    };
    let keep_waived = if board.waived_violations.is_empty() {
        false
    } else if board.waived_violations == convert_waived_violations(doc)? {
        for (i, w) in board.waived_violations.iter().enumerate() {
            check_violation_indices(board, w.rule_index, &w.objects)
                .with_context(|| format!("waived violation #{i}"))?;
        }
        true
    } else {
        return Err(crate::AltiumFormatError::InvalidParamValue {
            key: "waived_violations".to_owned(),
            detail: "waived violations are read-only: keep them as read, or empty the list \
                     to clear them"
                .to_owned(),
        });
    };
    Ok((keep_violations, keep_waived))
}

fn check_violation_indices(
    board: &PcbDocBoard,
    rule_index: usize,
    objects: &[ViolationObject],
) -> Result<()> {
    if rule_index >= board.rules.len() {
        return Err(crate::AltiumFormatError::InvalidParamValue {
            key: "RULEINDEX".to_owned(),
            detail: format!(
                "rule index {rule_index} outside the board's {} rules",
                board.rules.len()
            ),
        });
    }
    for o in objects {
        let len = match o.kind {
            PcbObjectId::Track => board.tracks.len(),
            PcbObjectId::Arc => board.arcs.len(),
            PcbObjectId::Via => board.vias.len(),
            PcbObjectId::Pad => board.pads.len(),
            PcbObjectId::Fill => board.fills.len(),
            PcbObjectId::Text => board.texts.len(),
            PcbObjectId::Region => board.regions.len(),
            PcbObjectId::ComponentBody => board.component_bodies.len(),
            PcbObjectId::Polygon => board.polygons.len(),
            PcbObjectId::Component => board.components.len(),
            PcbObjectId::Net => board.nets.len(),
            PcbObjectId::DifferentialPair => board.differential_pairs.len(),
            PcbObjectId::Board => 1,
            kind => {
                return Err(crate::AltiumFormatError::InvalidParamValue {
                    key: "PRIM1ID".to_owned(),
                    detail: format!("violation object kind {kind:?} has no board list"),
                });
            }
        };
        if o.index >= len {
            return Err(crate::AltiumFormatError::InvalidParamValue {
                key: "PRIM1INDEX".to_owned(),
                detail: format!(
                    "{:?} index {} outside the board's {len} objects of that kind",
                    o.kind, o.index
                ),
            });
        }
    }
    Ok(())
}

// ── Core entry point ───────────────────────────────────────────────────────

/// Convert a public `PcbDocBoard` back into internal `PcbDoc` sections.
///
/// Replaces parameter sections with fresh `ParameterCollection`s built from
/// scratch. Primitive sections are rebuilt with format-internal fields
/// preserved from existing records matched by position index.
pub(crate) fn board_to_internal(board: &PcbDocBoard, doc: &mut PcbDoc) -> Result<()> {
    // Checked before anything is written (update_board is not transactional).
    if let Some(i) = board
        .components
        .iter()
        .position(|c| c.kind == altium_format_types::ComponentKind::Jumper)
    {
        return Err(crate::AltiumFormatError::InvalidParamValue {
            key: "COMPONENTKINDVERSION3".to_owned(),
            detail: format!(
                "component #{i}: writing kind Jumper is not supported (storage form unknown)"
            ),
        });
    }
    let (keep_violations, keep_waived) = check_stored_violations(board, doc)?;
    let ctx = WriteContext::new(board);
    attach_pad_unique_ids(doc).context("UniqueIDPrimitiveInformation")?;
    if !keep_violations {
        doc.violations.values_mut().for_each(Vec::clear);
    }
    if !keep_waived {
        doc.waived_violations.clear();
    }

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
        build_component_records(&board.components)?,
        // Designators repeat (test-padshapes has ten R14); the unique id does not.
        "UNIQUEID",
    );
    check_component_designators(board)?;
    drop_absent_source_designators(doc, &board.components)?;
    write_component_kinds(doc, &board.components)?;
    replace_param_section(
        doc,
        ParamSectionKind::Polygons6,
        build_polygon_records(&board.polygons, &ctx)?,
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
    replace_board_outline(doc, board.settings.outline.as_ref())?;

    // ── Step 3: Replace primitive sections ──
    replace_primitives_with_preservation(
        doc,
        PrimitiveSectionKind::Tracks6,
        build_track_records(&board.tracks, &ctx).context("building track records")?,
    );
    replace_primitives_with_preservation(
        doc,
        PrimitiveSectionKind::Arcs6,
        build_arc_records(&board.arcs, &ctx).context("building arc records")?,
    );
    replace_primitives_with_preservation(
        doc,
        PrimitiveSectionKind::Vias6,
        build_via_records(&board.vias, &ctx).context("building via records")?,
    );
    replace_primitives_with_preservation(
        doc,
        PrimitiveSectionKind::Pads6,
        build_pad_records(&board.pads, &ctx).context("building pad records")?,
    );
    replace_primitives_with_preservation(
        doc,
        PrimitiveSectionKind::Fills6,
        build_fill_records(&board.fills, &ctx).context("building fill records")?,
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
        build_text_records(&board.texts, &ctx, &wide_indices).context("building text records")?,
    );

    // Regions go back to the section they were read from: BoardRegions for
    // board regions, otherwise the section board() reads (ShapeBasedRegions6
    // whenever that section exists, even empty, else Regions6). Before, every
    // region went to one section, so a board region was duplicated into
    // ShapeBasedRegions6 and positional preservation shifted the attributes of
    // all following regions.
    let (board_regions, copper_regions): (Vec<Region>, Vec<Region>) =
        board.regions.iter().cloned().partition(|r| r.board_region);
    let region_kind = if has_primitive_section(doc, PrimitiveSectionKind::ShapeBasedRegions6) {
        PrimitiveSectionKind::ShapeBasedRegions6
    } else {
        PrimitiveSectionKind::Regions6
    };
    replace_primitives_with_preservation(
        doc,
        region_kind,
        build_region_records(
            &copper_regions,
            &ctx,
            region_kind == PrimitiveSectionKind::ShapeBasedRegions6,
        )
        .context("building region records")?,
    );
    if !board_regions.is_empty() || has_primitive_section(doc, PrimitiveSectionKind::BoardRegions) {
        replace_primitives_with_preservation(
            doc,
            PrimitiveSectionKind::BoardRegions,
            build_region_records(&board_regions, &ctx, false)
                .context("building board region records")?,
        );
    }

    // ComponentBody section: prefer ShapeBasedComponentBodies6 if present.
    let body_kind = detect_section_kind(
        doc,
        PrimitiveSectionKind::ShapeBasedComponentBodies6,
        PrimitiveSectionKind::ComponentBodies6,
    );
    replace_primitives_with_preservation(
        doc,
        body_kind,
        build_body_records(&board.component_bodies, &ctx).context("building body records")?,
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

/// Build Components6 records. A component's `unique_id` must be 8 uppercase
/// letters (or empty, to generate one) and may not repeat another component's.
/// A component's `designator` must be what the reader would rebuild from the
/// written record: its `source_designator`, or, without one, its designator
/// text. Otherwise a renamed `designator` would be silently dropped.
fn check_component_designators(board: &PcbDocBoard) -> Result<()> {
    for (i, comp) in board.components.iter().enumerate() {
        let stored = match &comp.source_designator {
            Some(source) => Some(source.as_str()),
            None => board
                .texts
                .iter()
                .find(|t| t.is_designator && t.component_index == Some(i))
                .map(|t| t.text.as_str()),
        };
        if stored != Some(comp.designator.as_str()) {
            return Err(crate::AltiumFormatError::InvalidParamValue {
                key: "SOURCEDESIGNATOR".to_owned(),
                detail: format!(
                    "component #{i}: designator {:?} would be stored as {stored:?}; set \
                     source_designator (or the designator text) to the new designator",
                    comp.designator
                ),
            });
        }
    }
    Ok(())
}

/// The Components6 merge starts from the old record's keys; a component whose
/// `source_designator` is `None` must not keep an old SOURCEDESIGNATOR.
fn drop_absent_source_designators(doc: &mut PcbDoc, components: &[PcbDocComponent]) -> Result<()> {
    for section in &mut doc.sections {
        if let PcbDocSection::Parameter(param) = section {
            if param.kind == ParamSectionKind::Components6 {
                for (record, comp) in param.records.iter_mut().zip(components) {
                    if comp.source_designator.is_none() {
                        record
                            .params
                            .remove_optional::<String>("SOURCEDESIGNATOR")
                            .context("Components6 SOURCEDESIGNATOR")?;
                    }
                }
            }
        }
    }
    Ok(())
}

/// Store each component's kind (the merge keeps the old record's keys, so all
/// three kind keys are replaced): Standard writes none, kinds 0–4 write
/// `COMPONENTKIND`, StandardNoBom writes `COMPONENTKIND=0` and
/// `COMPONENTKINDVERSION2=5` (as all 409 corpus records). How Altium stores
/// Jumper is not known (no corpus record), so writing one is an error.
fn write_component_kinds(doc: &mut PcbDoc, components: &[PcbDocComponent]) -> Result<()> {
    use altium_format_types::ComponentKind;
    for section in &mut doc.sections {
        let PcbDocSection::Parameter(param) = section else {
            continue;
        };
        if param.kind != ParamSectionKind::Components6 {
            continue;
        }
        for (i, (record, comp)) in param.records.iter_mut().zip(components).enumerate() {
            for key in [
                "COMPONENTKIND",
                "COMPONENTKINDVERSION2",
                "COMPONENTKINDVERSION3",
            ] {
                record
                    .params
                    .remove_optional::<String>(key)
                    .with_context(|| format!("component #{i} {key}"))?;
            }
            match comp.kind {
                ComponentKind::Standard => {}
                ComponentKind::StandardNoBom => {
                    record.params.insert("COMPONENTKIND", "0".to_owned());
                    record
                        .params
                        .insert("COMPONENTKINDVERSION2", (comp.kind as u8).to_string());
                }
                ComponentKind::Jumper => {
                    // Also rejected before anything is written.
                    return Err(crate::AltiumFormatError::InvalidParamValue {
                        key: "COMPONENTKINDVERSION3".to_owned(),
                        detail: format!("component #{i}: writing kind Jumper is not supported"),
                    });
                }
                other => record
                    .params
                    .insert("COMPONENTKIND", (other as u8).to_string()),
            }
        }
    }
    Ok(())
}

fn build_component_records(components: &[PcbDocComponent]) -> Result<Vec<StandardParamRecord>> {
    let mut seen: HashMap<&str, usize> = HashMap::with_capacity(components.len());
    for (i, comp) in components.iter().enumerate() {
        if comp.unique_id.is_empty() {
            continue;
        }
        validate_unique_id("UNIQUEID", &comp.unique_id)
            .with_context(|| format!("component #{i} ({})", comp.designator))?;
        if let Some(first) = seen.insert(comp.unique_id.as_str(), i) {
            return Err(crate::AltiumFormatError::InvalidParamValue {
                key: "UNIQUEID".to_owned(),
                detail: format!(
                    "components #{first} and #{i} share unique id {:?}",
                    comp.unique_id
                ),
            });
        }
    }
    components
        .iter()
        .map(|comp| {
            let mut params = ParameterCollection::new();
            // Standard Altium component fields with sensible defaults.
            // These are written in the order Altium Designer expects.
            params.insert("SELECTION", "FALSE".to_string());
            params.insert(
                "LAYER",
                layer_to_param_value("LAYER", &comp.layer)
                    .with_context(|| format!("component {}", comp.designator))?,
            );
            params.insert("LOCKED", comp.locked.to_param_value());
            params.insert("POLYGONOUTLINE", "FALSE".to_string());
            params.insert("USERROUTED", "TRUE".to_string());
            params.insert("KEEPOUT", "FALSE".to_string());
            params.insert("PRIMITIVELOCK", comp.primitive_lock.to_param_value());
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
            if let Some(source) = &comp.source_designator {
                params.insert("SOURCEDESIGNATOR", source.clone());
            }
            params.insert("SOURCEUNIQUEID", comp.source_unique_id.clone());
            params.insert(
                "SOURCEHIERARCHICALPATH",
                comp.source_hierarchical_path.clone(),
            );
            params.insert("SOURCEFOOTPRINTLIBRARY", String::new());
            params.insert("SOURCECOMPONENTLIBRARY", comp.source_library.clone());
            params.insert("SOURCELIBREFERENCE", comp.source_lib_reference.clone());
            params.insert("SOURCEDESCRIPTION", String::new());
            params.insert("FOOTPRINTDESCRIPTION", String::new());
            params.insert("COMMENT", comp.comment.clone());
            // Keep the component's id; generate one for a component built by hand.
            let unique_id = if comp.unique_id.is_empty() {
                generate_unique_id()
            } else {
                comp.unique_id.clone()
            };
            params.insert("UNIQUEID", unique_id);
            Ok(StandardParamRecord { params })
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

/// Build Polygons6 records in Altium's stored form, the inverse of
/// `pcbdoc_read::convert_polygon`: every typed field is written back, `None`
/// fields are omitted.
///
/// The outline comes from `contour`; a polygon built by hand with only
/// `vertices` (empty `contour`) is written with line edges through them. With a
/// non-empty `contour`, `vertices` must equal `contour.to_points()`.
fn build_polygon_records(
    polygons: &[Polygon],
    ctx: &WriteContext,
) -> Result<Vec<StandardParamRecord>> {
    polygons
        .iter()
        .enumerate()
        .map(|(idx, poly)| {
            build_polygon_record(poly, ctx).with_context(|| format!("Polygons6 record #{idx}"))
        })
        .collect()
}

fn build_polygon_record(poly: &Polygon, ctx: &WriteContext) -> Result<StandardParamRecord> {
    let mut params = ParameterCollection::new();
    let flag = |params: &mut ParameterCollection, key: &str, v: Option<bool>| {
        if let Some(v) = v {
            params.insert(key, v.to_param_value());
        }
    };
    let mil = |params: &mut ParameterCollection, key: &str, v: Option<Coord>| {
        if let Some(v) = v {
            params.insert(key, ExactMilCoord(v).to_param_value());
        }
    };
    params.insert("SELECTION", poly.selected.to_param_value());
    params.insert("LAYER", layer_to_param_value("LAYER", &poly.layer)?);
    params.insert("LOCKED", poly.locked.to_param_value());
    params.insert("POLYGONOUTLINE", poly.polygon_outline.to_param_value());
    params.insert("USERROUTED", poly.user_routed.to_param_value());
    params.insert("KEEPOUT", poly.keepout.to_param_value());
    params.insert("UNIONINDEX", poly.union_index.to_param_value());
    params.insert("PRIMITIVELOCK", poly.primitive_lock.to_param_value());
    params.insert(
        "POLYGONTYPE",
        polygon_type_to_param_value(poly.polygon_type)?.to_owned(),
    );
    flag(&mut params, "POUROVER", poly.pour_over);
    flag(&mut params, "REMOVEDEAD", poly.remove_dead);
    mil(&mut params, "GRIDSIZE", poly.grid_size);
    mil(&mut params, "TRACKWIDTH", poly.track_width);
    if let Some(h) = poly.hatch_style {
        params.insert("HATCHSTYLE", hatch_style_to_param_value(h)?.to_owned());
    }
    flag(&mut params, "USEOCTAGONS", poly.use_octagons);
    mil(&mut params, "MINPRIMLENGTH", poly.min_prim_length);

    let contour = if poly.contour.segments.is_empty() {
        PcbContour {
            segments: poly
                .vertices
                .iter()
                .map(|&endpoint| ContourSegment::Line { endpoint })
                .collect(),
        }
    } else {
        if poly.vertices != poly.contour.to_points() {
            return Err(crate::AltiumFormatError::InvalidParamValue {
                key: "VX0".to_owned(),
                detail: "polygon `vertices` differ from `contour`; edit `contour` (the \
                         written outline) or clear it to write `vertices` as line edges"
                    .to_owned(),
            });
        }
        poly.contour.clone()
    };
    insert_param_contour(&mut params, &contour);

    flag(&mut params, "SHELVED", poly.shelved);
    if let Some(layer) = &poly.restore_layer {
        params.insert("RESTORELAYER", layer_to_param_value("RESTORELAYER", layer)?);
    } else if poly.shelved.is_some() {
        params.insert("RESTORELAYER", "UNKNOWN".to_owned());
    }
    if let Some(net) = &poly.restore_net {
        params.insert("RESTORENET", net.clone());
    }
    flag(
        &mut params,
        "REMOVEISLANDSBYAREA",
        poly.remove_islands_by_area,
    );
    flag(&mut params, "REMOVENECKS", poly.remove_necks);
    if let Some(a) = poly.area_threshold {
        params.insert("AREATHRESHOLD", a.to_param_value());
    }
    mil(&mut params, "ARCRESOLUTION", poly.arc_approximation);
    mil(&mut params, "NECKWIDTHTHRESHOLD", poly.neck_width_threshold);
    flag(&mut params, "NECKWIDTHFROMRULE", poly.neck_width_from_rule);
    if let Some(s) = poly.pour_over_style {
        params.insert("POUROVERSTYLE", s.to_param_value());
    }
    if !poly.name.is_empty() {
        params.insert("NAME", encode_text_for_wide_strings(&poly.name));
    }
    if poly.pour_order != 0 || poly.polygon_type == PolygonType::SignalLayer {
        params.insert("POURINDEX", poly.pour_order.to_param_value());
    }
    flag(&mut params, "IGNOREVIOLATIONS", poly.ignore_violations);
    flag(&mut params, "AUTONAME", poly.auto_name);
    flag(
        &mut params,
        "OPTIMALVOIDROTATION",
        poly.optimal_void_rotation,
    );
    flag(&mut params, "OBEYPOLYGONCUTOUT", poly.obey_polygon_cutout);
    flag(&mut params, "COPPERINVALIDATE", poly.copper_invalidate);
    if poly.connect_style != PlaneConnectionStyle::Relief {
        params.insert("CONNECTSTYLE", (poly.connect_style as u8).to_param_value());
    }
    if poly.relief_conductor_width != Coord::from_internal(254_000) {
        params.insert(
            "RELIEFCONDUCTORWIDTH",
            poly.relief_conductor_width.to_param_value(),
        );
    }
    if poly.relief_entries != 4 {
        params.insert("RELIEFENTRIES", poly.relief_entries.to_param_value());
    }
    if poly.relief_air_gap != Coord::from_internal(254_000) {
        params.insert("RELIEFAIRGAP", poly.relief_air_gap.to_param_value());
    }
    if let Some(net) = &poly.net {
        let index = ctx.net_indices.get(net.as_str()).ok_or_else(|| {
            crate::AltiumFormatError::InvalidParamValue {
                key: "NET".to_owned(),
                detail: format!("polygon net {net:?} is not in the board's nets"),
            }
        })?;
        params.insert("NET", index.to_param_value());
    }
    Ok(StandardParamRecord { params })
}

/// Write a contour as indexed parameters (`KINDn`, `VXn`, `VYn`, `CXn`, `CYn`,
/// `SAn`, `EAn`, `Rn`), the inverse of `pcbdoc_read::remove_param_contour`.
/// Line edges get zero arc fields.
pub(crate) fn insert_param_contour(params: &mut ParameterCollection, contour: &PcbContour) {
    const ZERO_ANGLE: &str = " 0.00000000000000E+0000";
    for (i, seg) in contour.segments.iter().enumerate() {
        let (kind, endpoint, arc) = match seg {
            ContourSegment::Line { endpoint } => (PolySegmentKind::Line, endpoint, None),
            ContourSegment::Arc {
                endpoint,
                center,
                radius,
                start_angle,
                end_angle,
            } => (
                PolySegmentKind::Arc,
                endpoint,
                Some((center, radius, start_angle, end_angle)),
            ),
        };
        params.insert(&format!("KIND{i}"), (kind as u8).to_param_value());
        params.insert(
            &format!("VX{i}"),
            ExactMilCoord(endpoint.x).to_param_value(),
        );
        params.insert(
            &format!("VY{i}"),
            ExactMilCoord(endpoint.y).to_param_value(),
        );
        match arc {
            Some((center, radius, sa, ea)) => {
                params.insert(&format!("CX{i}"), ExactMilCoord(center.x).to_param_value());
                params.insert(&format!("CY{i}"), ExactMilCoord(center.y).to_param_value());
                params.insert(&format!("SA{i}"), format_altium_angle(*sa));
                params.insert(&format!("EA{i}"), format_altium_angle(*ea));
                params.insert(&format!("R{i}"), ExactMilCoord(*radius).to_param_value());
            }
            None => {
                params.insert(&format!("CX{i}"), "0mil".to_owned());
                params.insert(&format!("CY{i}"), "0mil".to_owned());
                params.insert(&format!("SA{i}"), ZERO_ANGLE.to_owned());
                params.insert(&format!("EA{i}"), ZERO_ANGLE.to_owned());
                params.insert(&format!("R{i}"), "0mil".to_owned());
            }
        }
    }
}

/// Altium's angle form in contour records: ` d.ddddddddddddddE+dddd`
/// (leading space for non-negative values, 14 fractional digits, 4-digit exponent).
fn format_altium_angle(value: f64) -> String {
    let s = format!("{value:.14E}");
    let (mantissa, exp) = s.split_once('E').expect("{:E} output has an exponent");
    let exp: i32 = exp.parse().expect("{:E} exponent is an integer");
    let sign = if exp < 0 { '-' } else { '+' };
    let lead = if value.is_sign_negative() { "" } else { " " };
    format!("{lead}{mantissa}E{sign}{:04}", exp.abs())
}

fn ensure_standard_classes(board: &PcbDocBoard) -> Vec<NetClass> {
    use altium_format_types::pcb::ClassMemberKind;

    let mut classes = board.classes.clone();

    if !classes.iter().any(|c| c.name == "All Components") {
        let members: Vec<String> = board
            .components
            .iter()
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
        let members: Vec<String> = board.nets.iter().map(|n| n.name.clone()).collect();
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
    params.insert(
        "DISPLAYUNIT",
        (settings.display_unit as u8).to_param_value(),
    );
    vec![StandardParamRecord { params }]
}

/// Replace the Board6 outline keys with `outline` (all of them removed when
/// `None`). The merge above keeps the old record's keys, so a shorter new
/// outline would otherwise leave the old record's trailing vertices behind.
/// Only the first Board6 record is touched, the one the reader takes.
fn replace_board_outline(doc: &mut PcbDoc, outline: Option<&PcbContour>) -> Result<()> {
    let record = doc.sections.iter_mut().find_map(|section| match section {
        PcbDocSection::Parameter(param) if param.kind == ParamSectionKind::Board6 => {
            param.records.first_mut()
        }
        _ => None,
    });
    match record {
        Some(record) => {
            replace_record_outline(&mut record.params, outline).context("Board6 outline")
        }
        None => Ok(()),
    }
}

fn replace_record_outline(
    params: &mut ParameterCollection,
    outline: Option<&PcbContour>,
) -> Result<()> {
    strip_param_contour(params)?;
    if let Some(outline) = outline {
        insert_param_contour(params, outline);
    }
    Ok(())
}

// ── Primitive record builders ──────────────────────────────────────────────

fn build_track_records(tracks: &[Track], ctx: &WriteContext) -> Result<Vec<ParsedPrimitiveRecord>> {
    tracks
        .iter()
        .map(|t| {
            reject_unique_id("track", &t.unique_id)?;
            Ok(ParsedPrimitiveRecord {
                object_id: PcbObjectId::Track,
                primitive: PcbPrimitive::Track(PcbTrack {
                    common: primitive_common_for_board(
                        &t.layer,
                        &t.net,
                        &t.component,
                        t.component_index,
                        t.flags,
                        t.polygon,
                        t.plane_split_line,
                        ctx,
                    )?,
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
        })
        .collect()
}

fn build_arc_records(arcs: &[Arc], ctx: &WriteContext) -> Result<Vec<ParsedPrimitiveRecord>> {
    arcs.iter()
        .map(|a| {
            reject_unique_id("arc", &a.unique_id)?;
            Ok(ParsedPrimitiveRecord {
                object_id: PcbObjectId::Arc,
                primitive: PcbPrimitive::Arc(PcbArc {
                    common: primitive_common_for_board(
                        &a.layer,
                        &a.net,
                        &a.component,
                        a.component_index,
                        a.flags,
                        a.polygon,
                        a.plane_split_line,
                        ctx,
                    )?,
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
        })
        .collect()
}

fn build_via_records(vias: &[Via], ctx: &WriteContext) -> Result<Vec<ParsedPrimitiveRecord>> {
    vias.iter()
        .map(|v| {
            reject_unique_id("via", &v.unique_id)?;
            let (solder_mask_override, solder_mask_expansion_front) = match v.solder_mask_expansion
            {
                Some(exp) => (true, exp),
                None => (false, Coord::ZERO),
            };
            Ok(ParsedPrimitiveRecord {
                object_id: PcbObjectId::Via,
                primitive: PcbPrimitive::Via(PcbVia {
                    common: primitive_common_for_board(
                        &LayerRef::from_v6(V6Layer::MultiLayer),
                        &v.net,
                        &v.component,
                        v.component_index,
                        v.flags,
                        v.polygon,
                        false,
                        ctx,
                    )?,
                    location: v.location,
                    diameter: v.diameter,
                    hole_size: v.hole_size,
                    from_layer: layer_v6_for_write(&v.from_layer)
                        .with_context(|| format!("via {} from layer", v.id))?,
                    to_layer: layer_v6_for_write(&v.to_layer)
                        .with_context(|| format!("via {} to layer", v.id))?,
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
                    legacy_tail: Vec::new(),
                    layer_diameter_overrides: Vec::new(),
                    unique_id: None,
                }),
            })
        })
        .collect()
}

fn build_pad_records(pads: &[Pad], ctx: &WriteContext) -> Result<Vec<ParsedPrimitiveRecord>> {
    pads.iter()
        .map(|p| {
            if let Some(id) = &p.unique_id {
                validate_unique_id("UNIQUEID", id).with_context(|| format!("pad {}", p.id))?;
            }
            Ok(ParsedPrimitiveRecord {
                object_id: PcbObjectId::Pad,
                primitive: PcbPrimitive::Pad(PcbPad {
                    common: primitive_common_for_board(
                        &p.layer,
                        &p.net,
                        &p.component,
                        p.component_index,
                        p.flags,
                        p.polygon,
                        false,
                        ctx,
                    )?,
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
                        paste_mask_expansion_valid: mask_cache_state_for_write(
                            "PASTEMASKEXPANSIONMODE",
                            p.paste_mask_mode,
                        )
                        .with_context(|| format!("pad {}", p.id))?,
                        solder_mask_expansion_valid: mask_cache_state_for_write(
                            "SOLDERMASKEXPANSIONMODE",
                            p.solder_mask_mode,
                        )
                        .with_context(|| format!("pad {}", p.id))?,
                        power_plane_clearance_valid: TCacheState::default(),
                        planes_valid: TCacheState::default(),
                    },
                    selection_memory_flags: 0,
                    union_index: 0,
                    jumper_id: 0,
                    v7_layer_override: 0,
                    is_assy_testpoint_top: false,
                    is_assy_testpoint_bottom: false,
                    use_separate_expansions: p.solder_mask_bottom_expansion.is_some(),
                    solder_mask_bottom_expansion: p
                        .solder_mask_bottom_expansion
                        .map_or(0, |c| c.to_internal()),
                    solder_mask_expansion_from_hole_edge: p.solder_mask_from_hole_edge,
                    template_link_library_id: [0u8; 16],
                    template_link_template_id: [0u8; 16],
                    pin_package_length: Coord::ZERO,
                    hole_positive_tolerance: 0,
                    hole_negative_tolerance: 0,
                    unknown_170: 0,
                    has_sub4_extension: false,
                    sub4_extension: None,
                    thermal_reliefs: Vec::new(),
                    stack_data: None,
                    unique_id: p.unique_id.clone(),
                }),
            })
        })
        .collect()
}

fn build_fill_records(fills: &[Fill], ctx: &WriteContext) -> Result<Vec<ParsedPrimitiveRecord>> {
    fills
        .iter()
        .map(|f| {
            reject_unique_id("fill", &f.unique_id)?;
            Ok(ParsedPrimitiveRecord {
                object_id: PcbObjectId::Fill,
                primitive: PcbPrimitive::Fill(PcbFill {
                    common: primitive_common_for_board(
                        &f.layer,
                        &f.net,
                        &f.component,
                        f.component_index,
                        f.flags,
                        f.polygon,
                        false,
                        ctx,
                    )?,
                    corner_1: f.corner1,
                    corner_2: f.corner2,
                    rotation: f.rotation,
                    user_routed: None,
                    union_index: None,
                    layer_enum_index: Some(f.layer.v7()),
                    keepout_restrictions: None,
                }),
            })
        })
        .collect()
}

fn build_text_records(
    texts: &[Text],
    ctx: &WriteContext,
    wide_indices: &[i32],
) -> Result<Vec<ParsedPrimitiveRecord>> {
    texts
        .iter()
        .enumerate()
        .map(|(i, t)| {
            reject_unique_id("text", &t.unique_id)?;
            let wide_string_index = wide_indices.get(i).copied().unwrap_or(-1);
            Ok(ParsedPrimitiveRecord {
                object_id: PcbObjectId::Text,
                primitive: PcbPrimitive::Text(PcbText {
                    common: primitive_common_for_board(
                        &t.layer,
                        &None,
                        &t.component,
                        t.component_index,
                        t.flags,
                        None,
                        false,
                        ctx,
                    )?,
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
            })
        })
        .collect()
}

/// The contour to write: `contour`, or line edges through `points` when the
/// contour is empty (a region built from vertices only). A non-empty contour
/// must agree with `points`.
fn region_contour_for_write(
    what: &str,
    points: &[CoordPoint],
    contour: &PcbContour,
) -> Result<PcbContour> {
    if contour.segments.is_empty() {
        return Ok(PcbContour {
            segments: points
                .iter()
                .map(|&endpoint| ContourSegment::Line { endpoint })
                .collect(),
        });
    }
    if contour.to_points() != points {
        return Err(crate::AltiumFormatError::InvalidParamValue {
            key: what.to_owned(),
            detail: format!(
                "region {what} vertices differ from its contour; edit the contour or clear it"
            ),
        });
    }
    Ok(contour.clone())
}

/// A contour in a vertex-only form (Regions6/BoardRegions outlines, all holes):
/// an arc edge cannot be stored there.
fn vertex_contour(what: &str, contour: &PcbContour) -> Result<Contour> {
    if contour
        .segments
        .iter()
        .any(|s| matches!(s, ContourSegment::Arc { .. }))
    {
        return Err(crate::AltiumFormatError::InvalidParamValue {
            key: what.to_owned(),
            detail: format!("the {what} has arc edges but its section stores vertices only"),
        });
    }
    Ok(Contour::Legacy(contour.to_points()))
}

/// A ShapeBasedRegions6 outline: TPolySegments, closed (first vertex repeated).
fn shape_based_contour(contour: &PcbContour) -> Result<Contour> {
    let points = contour.to_points();
    if points.len() < 2 || points.first() != points.last() {
        return Err(crate::AltiumFormatError::InvalidParamValue {
            key: "outline".to_owned(),
            detail: "a ShapeBasedRegions6 outline must repeat its first vertex as the last"
                .to_owned(),
        });
    }
    Ok(Contour::ShapeBased(
        contour
            .segments
            .iter()
            .map(|s| match *s {
                ContourSegment::Line { endpoint } => PolySegment {
                    kind: PolySegmentKind::Line,
                    vertex: endpoint,
                    center: CoordPoint::new(Coord::ZERO, Coord::ZERO),
                    radius: Coord::ZERO,
                    angle1: 0.0,
                    angle2: 0.0,
                },
                ContourSegment::Arc {
                    endpoint,
                    center,
                    radius,
                    start_angle,
                    end_angle,
                } => PolySegment {
                    kind: PolySegmentKind::Arc,
                    vertex: endpoint,
                    center,
                    radius,
                    angle1: start_angle,
                    angle2: end_angle,
                },
            })
            .collect(),
    ))
}

fn build_region_records(
    regions: &[Region],
    ctx: &WriteContext,
    shape_based: bool,
) -> Result<Vec<ParsedPrimitiveRecord>> {
    regions
        .iter()
        .map(|r| {
            reject_unique_id("region", &r.unique_id)?;
            let mut outline_contour =
                region_contour_for_write("outline", &r.outline, &r.outline_contour)
                    .with_context(|| format!("region {}", r.id))?;
            // A line-only outline has an implied closing edge (the Regions6 and
            // BoardRegions convention); ShapeBasedRegions6 stores it explicitly.
            let line_only = outline_contour
                .segments
                .iter()
                .all(|s| matches!(s, ContourSegment::Line { .. }));
            if shape_based && line_only {
                let first = outline_contour.segments.first().map(|s| match *s {
                    ContourSegment::Line { endpoint } | ContourSegment::Arc { endpoint, .. } => {
                        endpoint
                    }
                });
                if let Some(first) = first {
                    if outline_contour.to_points().last() != Some(&first) {
                        outline_contour
                            .segments
                            .push(ContourSegment::Line { endpoint: first });
                    }
                }
            }
            let outline = if shape_based {
                shape_based_contour(&outline_contour)
            } else {
                vertex_contour("outline", &outline_contour)
            }
            .with_context(|| format!("region {}", r.id))?;
            if r.hole_contours.len() != r.holes.len() && !r.hole_contours.is_empty() {
                return Err(crate::AltiumFormatError::InvalidParamValue {
                    key: "holes".to_owned(),
                    detail: format!(
                        "region {}: {} holes but {} hole contours",
                        r.id,
                        r.holes.len(),
                        r.hole_contours.len()
                    ),
                });
            }
            let empty = PcbContour {
                segments: Vec::new(),
            };
            let holes = r
                .holes
                .iter()
                .enumerate()
                .map(|(h, points)| {
                    let contour = r.hole_contours.get(h).unwrap_or(&empty);
                    vertex_contour("hole", &region_contour_for_write("hole", points, contour)?)
                })
                .collect::<Result<Vec<_>>>()
                .with_context(|| format!("region {}", r.id))?;
            Ok(ParsedPrimitiveRecord {
                object_id: PcbObjectId::Region,
                primitive: PcbPrimitive::Region(PcbRegion {
                    common: primitive_common_for_board(
                        &r.layer,
                        &r.net,
                        &r.component,
                        r.component_index,
                        r.flags,
                        r.polygon,
                        false,
                        ctx,
                    )?,
                    kind: r.kind,
                    v7_layer: r.layer.display_name().unwrap_or("").to_owned(),
                    name: String::new(),
                    param_kind: 0,
                    subpoly_index: 0,
                    union_index: 0,
                    arc_resolution: Coord::ZERO,
                    // ISSHAPEBASED: corpus ShapeBasedRegions6 records set it exactly when
                    // they have arcs (a preserved old record's value wins).
                    is_shape_based: !line_only,
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
                    outline,
                    holes,
                    shape_text_segments: None,
                    hole_shape_text_segments: Vec::new(),
                    unique_id: None,
                }),
            })
        })
        .collect()
}

fn build_body_records(
    bodies: &[ComponentBody],
    ctx: &WriteContext,
) -> Result<Vec<ParsedPrimitiveRecord>> {
    bodies
        .iter()
        .map(|b| {
            Ok(ParsedPrimitiveRecord {
                object_id: PcbObjectId::ComponentBody,
                primitive: PcbPrimitive::ComponentBody(PcbComponentBody {
                    common: primitive_common_for_board(
                        &b.layer,
                        &None,
                        &b.component,
                        b.component_index,
                        b.flags,
                        None,
                        false,
                        ctx,
                    )?,
                    v7_layer: b.layer.display_name().unwrap_or("").to_owned(),
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
    // Pads carry unique ids (R4): a pad keeps the internal fields of the old
    // pad with its id, so inserting or removing pads does not shift them onto
    // neighbours. Positional matching is the fallback (no id, an id the old
    // section repeats, or an id not found).
    let mut old_pad_by_uid: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, rec) in old_records.iter().enumerate() {
        if let PcbPrimitive::Pad(p) = &rec.primitive {
            if let Some(uid) = &p.unique_id {
                old_pad_by_uid.entry(uid.as_str()).or_default().push(i);
            }
        }
    }
    for (i, new_rec) in new_records.iter_mut().enumerate() {
        let old_index = match &new_rec.primitive {
            PcbPrimitive::Pad(p) => match p
                .unique_id
                .as_deref()
                .and_then(|uid| old_pad_by_uid.get(uid))
            {
                Some(found) if found.len() == 1 => found[0],
                _ => i,
            },
            _ => i,
        };
        let Some(old_rec) = old_records.get(old_index) else {
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
    new.legacy_tail = old.legacy_tail.clone();
    if new.unique_id.is_none() {
        new.unique_id = old.unique_id.clone();
    }
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
    new.cache.paste_mask_expansion_valid = merge_mask_cache_state(
        (
            new.cache.paste_mask_expansion_valid,
            new.cache.paste_mask_expansion,
        ),
        (
            old.cache.paste_mask_expansion_valid,
            old.cache.paste_mask_expansion,
        ),
    );
    new.cache.solder_mask_expansion_valid = merge_mask_cache_state(
        (
            new.cache.solder_mask_expansion_valid,
            new.cache.solder_mask_expansion,
        ),
        (
            old.cache.solder_mask_expansion_valid,
            old.cache.solder_mask_expansion,
        ),
    );
    new.cache.power_plane_clearance_valid = old.cache.power_plane_clearance_valid;
    new.cache.planes_valid = old.cache.planes_valid;
    new.selection_memory_flags = old.selection_memory_flags;
    new.union_index = old.union_index;
    new.jumper_id = old.jumper_id;
    new.v7_layer_override = old.v7_layer_override;
    new.is_assy_testpoint_top = old.is_assy_testpoint_top;
    new.is_assy_testpoint_bottom = old.is_assy_testpoint_bottom;
    // use_separate_expansions, the bottom expansion and from_hole_edge come
    // from the API pad; without separate expansions the stored (unused) bottom
    // value is kept.
    if !new.use_separate_expansions {
        new.solder_mask_bottom_expansion = old.solder_mask_bottom_expansion;
    }
    new.template_link_library_id = old.template_link_library_id;
    new.template_link_template_id = old.template_link_template_id;
    new.pin_package_length = old.pin_package_length;
    new.hole_positive_tolerance = old.hole_positive_tolerance;
    new.hole_negative_tolerance = old.hole_negative_tolerance;
    new.unknown_170 = old.unknown_170;
    new.has_sub4_extension = old.has_sub4_extension;
    new.sub4_extension = old.sub4_extension.clone();
    new.thermal_reliefs = old.thermal_reliefs.clone();
    new.stack_data = old.stack_data.clone();
    // The API pad's `unique_id` is authoritative: `None` means "generate one"
    // (assign_and_rebuild_unique_id_section). Taking the id of the old pad at
    // the same position would duplicate it when pads are inserted.
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
    if new.unique_id.is_none() {
        new.unique_id = old.unique_id.clone();
    }
    // The contours come from the API region, written in the section's format
    // by build_region_records. (Before, the old record's contours were copied
    // here, so an edited outline was silently discarded.) PcbDoc regions carry
    // no text-form geometry; the reader rejects it.
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
    if new.unique_id.is_none() {
        new.unique_id = old.unique_id.clone();
    }
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
        let centroid_x: i64 =
            pads.iter().map(|p| p.location.x.raw() as i64).sum::<i64>() / pads.len() as i64;
        let centroid_y: i64 =
            pads.iter().map(|p| p.location.y.raw() as i64).sum::<i64>() / pads.len() as i64;

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
        // Connections6 is derived (ratsnest); a pad layer without a V6 form is
        // drawn on MultiLayer here, the pad record itself rejects such a layer.
        let hub_layer = hub.layer.legacy_v6().unwrap_or(V6Layer::MultiLayer);

        for (i, pad) in pads.iter().enumerate() {
            if i == hub_idx {
                continue;
            }
            let pad_layer = pad.layer.legacy_v6().unwrap_or(V6Layer::MultiLayer);
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
            component_header.insert(
                "SOURCEDESIGNATOR",
                comp.source_designator
                    .clone()
                    .unwrap_or_else(|| comp.designator.clone()),
            );

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
fn replace_primitive_parameters_section(doc: &mut PcbDoc, groups: Vec<PrimitiveParameterGroup>) {
    for section in &mut doc.sections {
        if let PcbDocSection::PrimitiveParameters(pp) = section {
            pp.groups = groups;
            return;
        }
    }
    doc.sections.push(PcbDocSection::PrimitiveParameters(
        PrimitiveParametersSectionData { groups },
    ));
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

    replace_param_section(
        doc,
        ParamSectionKind::UniqueIdPrimitiveInformation,
        uid_records,
    );
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
        .push(PcbDocSection::Parameter(ParamSectionData { kind, records }));
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
                StandardParamRecord {
                    params: merged_params,
                }
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
    doc.sections
        .push(PcbDocSection::Binary(BinarySectionData { kind, records }));
}

/// True when `doc` has a primitive section of `kind` (empty or not).
fn has_primitive_section(doc: &PcbDoc, kind: PrimitiveSectionKind) -> bool {
    doc.sections
        .iter()
        .any(|s| matches!(s, PcbDocSection::Primitive(p) if p.kind == kind))
}

/// Detect which section kind to write: prefer `modern` if it contains records, else `legacy`.
fn detect_section_kind(
    doc: &PcbDoc,
    modern: PrimitiveSectionKind,
    legacy: PrimitiveSectionKind,
) -> PrimitiveSectionKind {
    let has_modern_with_records = doc.sections.iter().any(
        |s| matches!(s, PcbDocSection::Primitive(p) if p.kind == modern && !p.records.is_empty()),
    );
    if has_modern_with_records {
        modern
    } else {
        legacy
    }
}

/// Test hook for the corpus round trip (test-fixtures): write one polygon against the given
/// Nets6 names.
#[cfg(all(test, feature = "test-fixtures"))]
pub(crate) fn build_polygon_record_for_test(
    poly: &Polygon,
    net_names: &[&str],
) -> Result<ParameterCollection> {
    let ctx = WriteContext {
        net_indices: net_names
            .iter()
            .enumerate()
            .map(|(i, n)| ((*n).to_owned(), i as u16))
            .collect(),
        component_designators: Vec::new(),
        polygon_count: 0,
    };
    build_polygon_record(poly, &ctx).map(|r| r.params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::pcbdoc_read::convert_polygon_for_test;

    /// A stored Polygons6 record (corpus-shaped, two edges incl. an arc).
    const RECORD: &str = "|SELECTION=FALSE|LAYER=MID2|LOCKED=FALSE|POLYGONOUTLINE=FALSE\
        |USERROUTED=TRUE|KEEPOUT=FALSE|UNIONINDEX=0|PRIMITIVELOCK=TRUE|POLYGONTYPE=Polygon\
        |POUROVER=TRUE|REMOVEDEAD=TRUE|GRIDSIZE=19.685mil|TRACKWIDTH=8mil|HATCHSTYLE=90Degree\
        |USEOCTAGONS=FALSE|MINPRIMLENGTH=3mil\
        |KIND0=0|VX0=7250.904mil|VY0=6378.6mil|CX0=0mil|CY0=0mil\
        |SA0= 0.00000000000000E+0000|EA0= 0.00000000000000E+0000|R0=0mil\
        |KIND1=1|VX1=7406.376mil|VY1=6534.072mil|CX1=7250.904mil|CY1=6534.072mil\
        |SA1= 0.00000000000000E+0000|EA1= 9.00000000000000E+0001|R1=155.472mil\
        |KIND2=0|VX2=7250.904mil|VY2=6378.6mil|CX2=0mil|CY2=0mil\
        |SA2= 0.00000000000000E+0000|EA2= 0.00000000000000E+0000|R2=0mil\
        |SHELVED=FALSE|RESTORELAYER=UNKNOWN|RESTORENET=P2V5|REMOVEISLANDSBYAREA=TRUE\
        |REMOVENECKS=TRUE|AREATHRESHOLD=250000000000.000000|ARCRESOLUTION=0.5mil\
        |NECKWIDTHTHRESHOLD=5mil|POUROVERSTYLE=1|NAME=80,50,86,53|POURINDEX=365\
        |IGNOREVIOLATIONS=FALSE|AUTONAME=TRUE|NET=1|";

    fn write_ctx() -> WriteContext {
        WriteContext {
            net_indices: HashMap::from([("GND".to_owned(), 0), ("P2V5".to_owned(), 1)]),
            component_designators: Vec::new(),
            polygon_count: 0,
        }
    }

    fn sorted_pairs(params: &ParameterCollection) -> Vec<(String, String)> {
        let mut pairs: Vec<(String, String)> = params
            .iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect();
        pairs.sort();
        pairs
    }

    #[test]
    fn polygons6_record_write_read_round_trip_is_identical() {
        let stored = ParameterCollection::from_str(RECORD).unwrap();
        let poly = convert_polygon_for_test(&stored, &["GND", "P2V5"]).unwrap();
        let written = build_polygon_record(&poly, &write_ctx()).unwrap();
        assert_eq!(sorted_pairs(&written.params), sorted_pairs(&stored));
        let reread = convert_polygon_for_test(&written.params, &["GND", "P2V5"]).unwrap();
        assert_eq!(format!("{reread:?}"), format!("{poly:?}"));
    }

    /// AREATHRESHOLD is stored from an 80-bit Extended with six fractional
    /// digits; some corpus values (e.g. `250015000224.999968`) have more
    /// significant digits than an f64 holds, so the written value can differ in
    /// the last stored digit (here by 1e-6 internal units², relative 4e-18).
    #[test]
    fn polygons6_area_threshold_is_written_with_f64_precision() {
        let stored = ParameterCollection::from_str(
            &RECORD.replace("250000000000.000000", "250015000224.999968"),
        )
        .unwrap();
        let poly = convert_polygon_for_test(&stored, &["GND", "P2V5"]).unwrap();
        let written = build_polygon_record(&poly, &write_ctx()).unwrap();
        assert_eq!(
            written.params.get("AREATHRESHOLD"),
            Some("250015000224.999969")
        );
    }

    #[test]
    fn polygons6_writer_rejects_vertices_edited_apart_from_contour() {
        let stored = ParameterCollection::from_str(RECORD).unwrap();
        let mut poly = convert_polygon_for_test(&stored, &["GND", "P2V5"]).unwrap();
        poly.vertices[0].x = Coord::from_internal(0);
        let err = build_polygon_record(&poly, &write_ctx())
            .err()
            .expect("diverged vertices must fail");
        assert!(err.to_string().contains("contour"), "{err}");

        poly.contour.segments.clear();
        let written = build_polygon_record(&poly, &write_ctx()).unwrap();
        assert_eq!(written.params.get("VX0"), Some("0mil"));
        assert_eq!(written.params.get("KIND1"), Some("0"));
    }

    /// Line edges with stale arc keys (as 128 corpus edges have) read as plain
    /// lines and are written back with zero arc keys.
    #[test]
    fn polygons6_line_edge_stale_arc_keys_are_normalised() {
        let stale = RECORD.replace(
            "|KIND0=0|VX0=7250.904mil|VY0=6378.6mil|CX0=0mil|CY0=0mil\
        |SA0= 0.00000000000000E+0000|EA0= 0.00000000000000E+0000|R0=0mil",
            "|KIND0=0|VX0=7250.904mil|VY0=6378.6mil|CX0=100mil|CY0=200mil\
        |SA0= 1.80000000000000E+0002|EA0= 2.70000000000000E+0002|R0=42.312mil",
        );
        assert_ne!(stale, RECORD);
        let stored = ParameterCollection::from_str(&stale).unwrap();
        let poly = convert_polygon_for_test(&stored, &["GND", "P2V5"]).unwrap();
        assert!(matches!(
            poly.contour.segments[0],
            ContourSegment::Line { .. }
        ));
        let written = build_polygon_record(&poly, &write_ctx()).unwrap();
        assert_eq!(written.params.get("R0"), Some("0mil"));
        assert_eq!(written.params.get("SA0"), Some(" 0.00000000000000E+0000"));
    }

    #[test]
    fn component_resolution_prefers_index_and_rejects_ambiguity() {
        let ctx = WriteContext {
            net_indices: HashMap::new(),
            component_designators: vec!["R14".to_owned(), "R14".to_owned(), "C1".to_owned()],
            polygon_count: 0,
        };
        let r14 = Some("R14".to_owned());
        assert_eq!(ctx.resolve_component_index(&None, None).unwrap(), 0xFFFF);
        assert_eq!(ctx.resolve_component_index(&r14, Some(1)).unwrap(), 1);
        assert_eq!(ctx.resolve_component_index(&None, Some(1)).unwrap(), 1);
        assert_eq!(
            ctx.resolve_component_index(&Some("C1".to_owned()), None)
                .unwrap(),
            2
        );
        for (designator, index, needle) in [
            (r14.clone(), None, "several components"),
            (Some("U9".to_owned()), None, "no component"),
            (Some("C1".to_owned()), Some(0), "disagrees"),
            (None, Some(3), "outside"),
        ] {
            let err = ctx.resolve_component_index(&designator, index).unwrap_err();
            assert!(err.to_string().contains(needle), "{needle}: {err}");
        }
    }

    #[test]
    fn polygon_membership_resolution() {
        let ctx = WriteContext {
            net_indices: HashMap::new(),
            component_designators: Vec::new(),
            polygon_count: 2,
        };
        let top = LayerRef::from_v6(V6Layer::TopLayer);
        let plane = LayerRef::from_v6(V6Layer::InternalPlane2);
        assert_eq!(
            ctx.resolve_polygon_index(None, false, &top).unwrap(),
            PRIMITIVE_INDEX_NONE
        );
        assert_eq!(ctx.resolve_polygon_index(Some(1), false, &top).unwrap(), 1);
        assert!(ctx.resolve_polygon_index(Some(2), false, &top).is_err());
        assert_eq!(
            ctx.resolve_polygon_index(None, true, &plane).unwrap(),
            POLYGON_INDEX_PLANE_SPLIT_LINE
        );
        assert!(ctx.resolve_polygon_index(None, true, &top).is_err());
        assert!(ctx.resolve_polygon_index(Some(0), true, &plane).is_err());
        // A V7-form plane layer read together with its V6 layer.
        let v7_plane = LayerRef::from_v6_and_v7(
            V6Layer::InternalPlane1,
            altium_format_types::pcb::V7Layer::new(0x0101_0001),
        );
        assert_eq!(
            layer_v6_for_write(&v7_plane).unwrap(),
            V6Layer::InternalPlane1
        );
        // The V6 layer comes from the id, never from a display name.
        let named = LayerRef::from_v7(altium_format_types::pcb::V7Layer::new(0x0104_0001))
            .with_name("TopLayer".to_owned());
        assert!(layer_v6_for_write(&named).is_err());
        assert!(ctx.resolve_polygon_index(None, true, &named).is_err());
        let err = primitive_common_for_board(
            &named,
            &None,
            &None,
            None,
            PcbFlags::default(),
            None,
            false,
            &ctx,
        )
        .unwrap_err();
        assert!(err.to_string().contains("0x01040001"), "{err}");
        assert_eq!(
            ctx.resolve_polygon_index(None, true, &v7_plane).unwrap(),
            POLYGON_INDEX_PLANE_SPLIT_LINE
        );
    }

    fn component(designator: &str, unique_id: &str) -> PcbDocComponent {
        PcbDocComponent {
            id: designator.to_owned(),
            designator: designator.to_owned(),
            pattern: String::new(),
            comment: String::new(),
            location: CoordPoint::new(Coord::ZERO, Coord::ZERO),
            rotation: 0.0,
            layer: LayerRef::from_v6(V6Layer::TopLayer),
            source_library: String::new(),
            source_lib_reference: String::new(),
            source_unique_id: String::new(),
            source_hierarchical_path: String::new(),
            unique_id: unique_id.to_owned(),
            locked: true,
            primitive_lock: false,
            source_designator: Some(designator.to_owned()),
            kind: altium_format_types::ComponentKind::Standard,
            parameters: Vec::new(),
        }
    }

    #[test]
    fn component_records_keep_identity_and_reject_bad_ids() {
        let records =
            build_component_records(&[component("R14", "OYNEOHXI"), component("R14", "")]).unwrap();
        assert_eq!(records[0].params.get("UNIQUEID"), Some("OYNEOHXI"));
        assert_eq!(records[0].params.get("LOCKED"), Some("TRUE"));
        assert_eq!(records[0].params.get("PRIMITIVELOCK"), Some("FALSE"));
        let generated = records[1].params.get("UNIQUEID").unwrap();
        assert!(
            validate_unique_id("UNIQUEID", generated).is_ok(),
            "{generated}"
        );

        let dup =
            build_component_records(&[component("R1", "OYNEOHXI"), component("R2", "OYNEOHXI")])
                .err()
                .expect("shared id must fail");
        assert!(dup.to_string().contains("share"), "{dup}");
        let bad = build_component_records(&[component("R1", "short")])
            .err()
            .expect("malformed id must fail");
        assert!(bad.to_string().contains("8 uppercase"), "{bad}");
    }

    #[test]
    fn non_pad_unique_ids_are_rejected_not_dropped() {
        assert!(reject_unique_id("track", &None).is_ok());
        let err = reject_unique_id("track", &Some("ABCDEFGH".to_owned())).unwrap_err();
        assert!(err.to_string().contains("ABCDEFGH"), "{err}");
    }

    #[test]
    fn board_outline_replacement_leaves_no_old_vertices() {
        let mut params = ParameterCollection::from_str(
            "|DOCUMENTNAME=x\
             |KIND0=0|VX0=0mil|VY0=0mil|CX0=0mil|CY0=0mil|SA0=0|EA0=0|R0=0mil\
             |KIND1=1|VX1=10mil|VY1=0mil|CX1=5mil|CY1=0mil|SA1=0|EA1=180|R1=5mil\
             |KIND2=0|VX2=10mil|VY2=10mil|CX2=0mil|CY2=0mil|SA2=0|EA2=0|R2=0mil\
             |KIND3=0|VX3=0mil|VY3=0mil|CX3=0mil|CY3=0mil|SA3=0|EA3=0|R3=0mil|SHELVED=FALSE|",
        )
        .unwrap();
        let mil = |v: i32| Coord::from_internal(v * 10_000);
        let two = PcbContour {
            segments: vec![
                ContourSegment::Line {
                    endpoint: CoordPoint::new(mil(1), mil(2)),
                },
                ContourSegment::Line {
                    endpoint: CoordPoint::new(mil(3), mil(4)),
                },
            ],
        };
        replace_record_outline(&mut params, Some(&two)).unwrap();
        assert_eq!(params.get("VX2"), None);
        assert_eq!(params.get("R3"), None);
        assert_eq!(params.get("SHELVED"), Some("FALSE"));
        let mut copy = params.clone();
        let reread = crate::api::pcbdoc_read::remove_param_contour(&mut copy).unwrap();
        assert_eq!(reread.to_points(), two.to_points());
        assert_eq!(copy.remaining_keys(), vec!["DOCUMENTNAME", "SHELVED"]);

        replace_record_outline(&mut params, None).unwrap();
        assert_eq!(params.remaining_keys(), vec!["DOCUMENTNAME", "SHELVED"]);
    }

    #[test]
    fn region_contours_are_written_in_the_section_format() {
        let mil = |v: i32| Coord::from_internal(v * 10_000);
        let pt = |x, y| CoordPoint::new(mil(x), mil(y));
        let arc = PcbContour {
            segments: vec![
                ContourSegment::Line { endpoint: pt(0, 0) },
                ContourSegment::Arc {
                    endpoint: pt(10, 0),
                    center: pt(10, 5),
                    radius: mil(5),
                    start_angle: 270.0,
                    end_angle: 90.0,
                },
                ContourSegment::Line {
                    endpoint: pt(10, 10),
                },
                ContourSegment::Line { endpoint: pt(0, 0) },
            ],
        };
        match shape_based_contour(&arc).unwrap() {
            Contour::ShapeBased(segs) => {
                assert_eq!(segs.len(), 4);
                assert_eq!(segs[1].kind, PolySegmentKind::Arc);
                assert_eq!(segs[1].angle2, 90.0);
            }
            Contour::Legacy(_) => panic!("expected TPolySegments"),
        }
        assert!(vertex_contour("outline", &arc).is_err());
        let open = PcbContour {
            segments: arc.segments[..3].to_vec(),
        };
        assert!(shape_based_contour(&open).is_err());
        let lines = region_contour_for_write(
            "outline",
            &arc.to_points(),
            &PcbContour {
                segments: Vec::new(),
            },
        )
        .unwrap();
        assert!(
            matches!(vertex_contour("outline", &lines).unwrap(), Contour::Legacy(p) if p.len() == 4)
        );
        let mut moved = arc.to_points();
        moved[0].x = mil(1);
        assert!(region_contour_for_write("outline", &moved, &arc).is_err());
    }

    fn region(outline: Vec<CoordPoint>, holes: Vec<Vec<CoordPoint>>) -> Region {
        Region {
            id: "region_0".to_owned(),
            layer: LayerRef::from_v6(V6Layer::TopLayer),
            net: None,
            component: None,
            unique_id: None,
            component_index: None,
            flags: PcbFlags::default(),
            polygon: None,
            kind: altium_format_types::pcb::RegionKind::Copper,
            outline,
            holes,
            outline_contour: PcbContour {
                segments: Vec::new(),
            },
            hole_contours: Vec::new(),
            is_board_cutout: false,
            is_keepout: false,
            board_region: false,
        }
    }

    fn written_outline(record: &ParsedPrimitiveRecord) -> &Contour {
        match &record.primitive {
            PcbPrimitive::Region(r) => &r.outline,
            _ => panic!("not a region"),
        }
    }

    #[test]
    fn region_records_close_vertex_outlines_and_check_holes() {
        let ctx = WriteContext {
            net_indices: HashMap::new(),
            component_designators: Vec::new(),
            polygon_count: 0,
        };
        let mil = |v: i32| Coord::from_internal(v * 10_000);
        let square = vec![
            CoordPoint::new(mil(0), mil(0)),
            CoordPoint::new(mil(10), mil(0)),
            CoordPoint::new(mil(10), mil(10)),
        ];
        // Vertex-only region into ShapeBasedRegions6: the implied closing edge
        // is stored (N + 1 segments); into Regions6 it stays N vertices.
        let shape =
            build_region_records(&[region(square.clone(), Vec::new())], &ctx, true).unwrap();
        assert!(matches!(written_outline(&shape[0]), Contour::ShapeBased(s) if s.len() == 4));
        let legacy =
            build_region_records(&[region(square.clone(), Vec::new())], &ctx, false).unwrap();
        assert!(matches!(written_outline(&legacy[0]), Contour::Legacy(p) if p.len() == 3));
        // An open line-only contour (as read from Regions6) is closed too.
        let mut read_from_legacy = region(square.clone(), Vec::new());
        read_from_legacy.outline_contour = PcbContour {
            segments: square
                .iter()
                .map(|&endpoint| ContourSegment::Line { endpoint })
                .collect(),
        };
        assert!(build_region_records(&[read_from_legacy], &ctx, true).is_ok());
        // Holes: more contours than vertex lists is an error; an empty outline
        // cannot go to ShapeBasedRegions6.
        let mut bad_holes = region(square.clone(), vec![square.clone()]);
        bad_holes.hole_contours = vec![
            PcbContour {
                segments: Vec::new(),
            },
            PcbContour {
                segments: Vec::new(),
            },
        ];
        assert!(build_region_records(&[bad_holes], &ctx, false).is_err());
        assert!(build_region_records(&[region(Vec::new(), Vec::new())], &ctx, true).is_err());
    }

    #[test]
    fn polygons6_writer_rejects_unknown_net() {
        let stored = ParameterCollection::from_str(RECORD).unwrap();
        let mut poly = convert_polygon_for_test(&stored, &["GND", "P2V5"]).unwrap();
        poly.net = Some("NOT_A_NET".to_owned());
        let err = build_polygon_record(&poly, &write_ctx())
            .err()
            .expect("unknown net must fail");
        assert!(err.to_string().contains("NOT_A_NET"), "{err}");
    }
}
