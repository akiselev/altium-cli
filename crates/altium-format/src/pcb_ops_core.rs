use std::collections::HashMap;

use altium_format_types::{Coord, CoordPoint, PcbFlags, PcbObjectId, V6Layer, V7Layer};
use indexmap::IndexMap;

use crate::pcbdoc::primitives::{
    ParsedPrimitiveRecord, PcbPrimitive as PcbDocPrimitive, PcbPrimitiveCommon as PcbDocCommon,
    PcbTrack as PcbDocTrack,
};
use crate::pcbdoc::records::PrimitiveSectionKind;
use crate::pcbdoc::{PcbDoc, PcbDocSection, PrimitiveSectionData};
use crate::pcblib::library::PcbLibComponentTocEntry;
use crate::pcblib::section_keys::resolve_footprint_key;
use crate::pcblib::{PcbFootprint, PcbPrimitive, PcbPrimitiveCommon, PcbTrack, PcbLib};
use crate::sch_ops_core::{EntityRef, EntityType, OpResult, RefExpr, RefRoot, RefStep, Value};
use crate::{AltiumFormatError, Result};

#[derive(Debug, Clone)]
pub struct QueryOp {
    pub opid: String,
    pub selector: String,
}

#[derive(Debug, Clone)]
pub struct AddTrackOp {
    pub opid: String,
    pub footprint_ref: Option<RefExpr>,
    pub start: CoordPoint,
    pub end: CoordPoint,
    pub width: Option<Coord>,
    pub layer: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AddFootprintOp {
    pub opid: String,
    pub id: Option<String>,
    pub name: String,
    pub pattern: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub enum PcbDocLowOp {
    Query(QueryOp),
    AddTrack(AddTrackOp),
}

#[derive(Debug, Clone)]
pub enum PcbLibLowOp {
    Query(QueryOp),
    AddFootprint(AddFootprintOp),
    AddTrack(AddTrackOp),
}

pub fn apply_pcbdoc_low_ops(doc: &mut PcbDoc, ops: &[PcbDocLowOp]) -> Result<Vec<OpResult>> {
    let mut out = Vec::with_capacity(ops.len());
    for op in ops {
        let result = match op {
            PcbDocLowOp::Query(v) => pcbdoc_query(doc, v)?,
            PcbDocLowOp::AddTrack(v) => pcbdoc_add_track(doc, v)?,
        };
        out.push(result);
    }
    Ok(out)
}

pub fn apply_pcblib_low_ops(lib: &mut PcbLib, ops: &[PcbLibLowOp]) -> Result<Vec<OpResult>> {
    let mut ctx = PcbLibExecCtx::default();
    let mut out = Vec::with_capacity(ops.len());
    for op in ops {
        let result = match op {
            PcbLibLowOp::Query(v) => pcblib_query(lib, v)?,
            PcbLibLowOp::AddFootprint(v) => pcblib_add_footprint(lib, v)?,
            PcbLibLowOp::AddTrack(v) => pcblib_add_track(lib, v, &ctx)?,
        };
        ctx.last_opid = Some(result.opid.clone());
        ctx.results.insert(result.opid.clone(), result.clone());
        out.push(result);
    }
    Ok(out)
}

#[derive(Default)]
struct PcbLibExecCtx {
    last_opid: Option<String>,
    results: HashMap<String, OpResult>,
}

fn parse_layer_name(layer: Option<&str>) -> Result<V6Layer> {
    let Some(name) = layer else {
        return Ok(V6Layer::TopLayer);
    };
    V6Layer::from_string_name(name).ok_or_else(|| AltiumFormatError::InvalidParamValue {
        key: "layer".to_owned(),
        detail: format!("unknown PCB layer '{name}'"),
    })
}

fn op_result(kind: &str, opid: &str, ref_: Option<EntityRef>, refs: Vec<EntityRef>) -> OpResult {
    let mut fields = IndexMap::new();
    if let Some(r) = &ref_ {
        fields.insert("ref".to_owned(), Value::Ref(r.clone()));
    }
    if !refs.is_empty() {
        fields.insert("refs".to_owned(), Value::Refs(refs.clone()));
    }
    fields.insert("count".to_owned(), Value::I64(refs.len() as i64));
    OpResult {
        opid: opid.to_owned(),
        kind: kind.to_owned(),
        ref_,
        refs,
        fields,
        warnings: Vec::new(),
    }
}

fn parse_selector_value(selector: &str, entity: &str, field: &str) -> Option<String> {
    let prefix = format!("{entity}[{field}=");
    if selector.starts_with(&prefix) && selector.ends_with(']') {
        let raw = &selector[prefix.len()..selector.len() - 1];
        return Some(raw.trim_matches('"').to_owned());
    }
    None
}

fn pcbdoc_track_refs(doc: &PcbDoc, selector: &str) -> Result<Vec<EntityRef>> {
    if selector != "track" {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "selector".to_owned(),
            detail: format!("unsupported pcbdoc query selector: {selector}"),
        });
    }

    let mut refs = Vec::new();
    for (sidx, section) in doc.sections.iter().enumerate() {
        if let PcbDocSection::Primitive(p) = section {
            if p.kind != PrimitiveSectionKind::Tracks6 {
                continue;
            }
            for ridx in 0..p.records.len() {
                refs.push(EntityRef {
                    domain: "PcbDoc".to_owned(),
                    entity_type: EntityType::Track,
                    id: format!("pcbdoc:track:{sidx}:{ridx}"),
                    display_path: format!("Tracks6[{ridx}]"),
                });
            }
        }
    }
    Ok(refs)
}

fn pcblib_refs(lib: &PcbLib, selector: &str) -> Result<Vec<EntityRef>> {
    if selector == "footprint" {
        return Ok(lib
            .footprints
            .iter()
            .enumerate()
            .map(|(idx, fp)| EntityRef {
                domain: "PcbLib".to_owned(),
                entity_type: EntityType::Footprint,
                id: format!("pcblib:footprint:{idx}"),
                display_path: fp.display_name.clone(),
            })
            .collect());
    }

    if let Some(name) = parse_selector_value(selector, "footprint", "name") {
        return Ok(lib
            .footprints
            .iter()
            .enumerate()
            .filter(|(_, fp)| fp.display_name == name)
            .map(|(idx, fp)| EntityRef {
                domain: "PcbLib".to_owned(),
                entity_type: EntityType::Footprint,
                id: format!("pcblib:footprint:{idx}"),
                display_path: fp.display_name.clone(),
            })
            .collect());
    }

    let selector_fp = parse_selector_value(selector, "track", "footprint");
    if selector == "track" || selector_fp.is_some() {
        let mut refs = Vec::new();
        for (fidx, fp) in lib.footprints.iter().enumerate() {
            if let Some(want) = &selector_fp {
                if fp.display_name != *want {
                    continue;
                }
            }
            for (pidx, prim) in fp.primitives.iter().enumerate() {
                if matches!(prim, PcbPrimitive::Track(_)) {
                    refs.push(EntityRef {
                        domain: "PcbLib".to_owned(),
                        entity_type: EntityType::Track,
                        id: format!("pcblib:track:{fidx}:{pidx}"),
                        display_path: format!("{}.track[{pidx}]", fp.display_name),
                    });
                }
            }
        }
        return Ok(refs);
    }

    Err(AltiumFormatError::InvalidParamValue {
        key: "selector".to_owned(),
        detail: format!("unsupported pcblib query selector: {selector}"),
    })
}

fn pcbdoc_query(doc: &PcbDoc, op: &QueryOp) -> Result<OpResult> {
    let refs = pcbdoc_track_refs(doc, &op.selector)?;
    let primary = if refs.len() == 1 {
        refs.first().cloned()
    } else {
        None
    };
    Ok(op_result("query", &op.opid, primary, refs))
}

fn pcblib_query(lib: &PcbLib, op: &QueryOp) -> Result<OpResult> {
    let refs = pcblib_refs(lib, &op.selector)?;
    let primary = if refs.len() == 1 {
        refs.first().cloned()
    } else {
        None
    };
    Ok(op_result("query", &op.opid, primary, refs))
}

fn ensure_track_section(doc: &mut PcbDoc) -> &mut PrimitiveSectionData {
    let existing = doc.sections.iter().position(|section| {
        if let PcbDocSection::Primitive(p) = section {
            p.kind == PrimitiveSectionKind::Tracks6
        } else {
            false
        }
    });
    let idx = if let Some(idx) = existing {
        idx
    } else {
        doc.sections
            .push(PcbDocSection::Primitive(PrimitiveSectionData {
                kind: PrimitiveSectionKind::Tracks6,
                records: Vec::new(),
            }));
        doc.sections.len() - 1
    };
    match &mut doc.sections[idx] {
        PcbDocSection::Primitive(p) => p,
        _ => unreachable!("created section must be primitive"),
    }
}

fn pcbdoc_add_track(doc: &mut PcbDoc, op: &AddTrackOp) -> Result<OpResult> {
    let layer = parse_layer_name(op.layer.as_deref())?;
    let track = PcbDocTrack {
        common: PcbDocCommon {
            layer,
            flags: PcbFlags::new(0),
            net_index: -1,
            unknown_1: 0,
            component_index: -1,
            polygon_index: -1,
            unknown_2: 0,
        },
        start: op.start,
        end: op.end,
        width: op.width.unwrap_or_else(|| Coord::from_mils(10)),
        subpoly_index: 0,
        user_routed: false,
        union_index: 0,
        track_kind: 0,
        layer_enum_index: V7Layer::from_v6(layer),
        keepout_restrictions: None,
    };

    let section = ensure_track_section(doc);
    let idx = section.records.len();
    section.records.push(ParsedPrimitiveRecord {
        object_id: PcbObjectId::Track,
        primitive: PcbDocPrimitive::Track(track),
    });

    let r = EntityRef {
        domain: "PcbDoc".to_owned(),
        entity_type: EntityType::Track,
        id: format!("pcbdoc:track:{idx}"),
        display_path: format!("Tracks6[{idx}]"),
    };
    Ok(op_result("add_track", &op.opid, Some(r), vec![]))
}

fn pcblib_add_footprint(lib: &mut PcbLib, op: &AddFootprintOp) -> Result<OpResult> {
    if lib.footprints.iter().any(|fp| fp.display_name == op.name) {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "name".to_owned(),
            detail: format!("footprint '{}' already exists", op.name),
        });
    }
    let cfb_key = resolve_footprint_key(&op.name, &lib.section_keys);
    if lib.footprints.iter().any(|fp| fp.cfb_key == cfb_key) {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "name".to_owned(),
            detail: format!("footprint storage key collision for '{}'", op.name),
        });
    }

    lib.footprints.push(PcbFootprint {
        display_name: op.name.clone(),
        cfb_key,
        pattern: op.pattern.clone().unwrap_or_else(|| op.name.clone()),
        height: Coord::from_mils(0),
        description: op.description.clone().unwrap_or_default(),
        item_guid: String::new(),
        revision_guid: String::new(),
        primitives: Vec::new(),
    });
    lib.component_toc.push(PcbLibComponentTocEntry {
        name: op.name.clone(),
        pad_count: 0,
        height: Coord::from_mils(0),
        description: op.description.clone().unwrap_or_default(),
    });
    let idx = lib.footprints.len() - 1;
    let r = EntityRef {
        domain: "PcbLib".to_owned(),
        entity_type: EntityType::Footprint,
        id: format!("pcblib:footprint:{idx}"),
        display_path: op.name.clone(),
    };
    Ok(op_result("add_footprint", &op.opid, Some(r), vec![]))
}

fn resolve_pcblib_footprint_index(
    lib: &PcbLib,
    r: &Option<RefExpr>,
    ctx: &PcbLibExecCtx,
) -> Result<usize> {
    if let Some(r) = r {
        let eref = resolve_ref_expr(r, ctx)?;
        if let Some(idx_str) = eref.id.strip_prefix("pcblib:footprint:") {
            let idx: usize = idx_str.parse().map_err(|_| AltiumFormatError::InvalidParamValue {
                key: "footprint_ref".to_owned(),
                detail: format!("invalid footprint ref id '{}'", eref.id),
            })?;
            if idx < lib.footprints.len() {
                return Ok(idx);
            }
        }
        return Err(AltiumFormatError::InvalidParamValue {
            key: "footprint_ref".to_owned(),
            detail: format!("reference does not resolve to a pcblib footprint: {}", eref.id),
        });
    }

    if lib.footprints.is_empty() {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "footprint_ref".to_owned(),
            detail: "footprint_ref is required when library has no footprints".to_owned(),
        });
    }
    Ok(lib.footprints.len() - 1)
}

fn resolve_ref_expr(r: &RefExpr, ctx: &PcbLibExecCtx) -> Result<EntityRef> {
    let mut cur = match &r.root {
        RefRoot::OpId(opid) => {
            let res = ctx.results.get(opid).ok_or_else(|| AltiumFormatError::InvalidParamValue {
                key: "ref".to_owned(),
                detail: format!("unknown opid '{opid}'"),
            })?;
            Value::Map(res.fields.clone())
        }
        RefRoot::Last => {
            let Some(opid) = &ctx.last_opid else {
                return Err(AltiumFormatError::InvalidParamValue {
                    key: "ref".to_owned(),
                    detail: "no previous op result for $last".to_owned(),
                });
            };
            let res = ctx.results.get(opid).ok_or_else(|| AltiumFormatError::InvalidParamValue {
                key: "ref".to_owned(),
                detail: format!("unknown opid '{opid}'"),
            })?;
            Value::Map(res.fields.clone())
        }
        RefRoot::Self_ | RefRoot::Sheet => {
            return Err(AltiumFormatError::InvalidParamValue {
                key: "ref".to_owned(),
                detail: "Self_/Sheet roots are not supported in pcb ops yet".to_owned(),
            })
        }
    };

    for step in &r.steps {
        cur = match (step, cur) {
            (RefStep::Member(name), Value::Map(map)) => map.get(name).cloned().ok_or_else(|| {
                AltiumFormatError::InvalidParamValue {
                    key: "ref".to_owned(),
                    detail: format!("missing member '{name}' in ref path"),
                }
            })?,
            (RefStep::Index(idx), Value::List(list)) => {
                list.get(*idx).cloned().ok_or_else(|| AltiumFormatError::InvalidParamValue {
                    key: "ref".to_owned(),
                    detail: format!("list index {idx} out of range in ref path"),
                })?
            }
            (RefStep::Index(idx), Value::Refs(list)) => list
                .get(*idx)
                .cloned()
                .map(Value::Ref)
                .ok_or_else(|| AltiumFormatError::InvalidParamValue {
                    key: "ref".to_owned(),
                    detail: format!("refs index {idx} out of range in ref path"),
                })?,
            _ => {
                return Err(AltiumFormatError::InvalidParamValue {
                    key: "ref".to_owned(),
                    detail: "invalid ref step for current value".to_owned(),
                })
            }
        }
    }

    match cur {
        Value::Ref(r) => Ok(r),
        _ => Err(AltiumFormatError::InvalidParamValue {
            key: "ref".to_owned(),
            detail: "reference did not resolve to an entity ref".to_owned(),
        }),
    }
}

fn pcblib_add_track(lib: &mut PcbLib, op: &AddTrackOp, ctx: &PcbLibExecCtx) -> Result<OpResult> {
    let idx = resolve_pcblib_footprint_index(lib, &op.footprint_ref, ctx)?;
    let layer = parse_layer_name(op.layer.as_deref())?;
    let fp = &mut lib.footprints[idx];

    let prim = PcbPrimitive::Track(PcbTrack {
        common: PcbPrimitiveCommon {
            layer,
            pad_byte: 0,
            flags: PcbFlags::new(0),
            net_index: -1,
            polygon_index: 0,
            component_index: 0,
            unknown: 0,
        },
        start: op.start,
        end: op.end,
        width: op.width.unwrap_or_else(|| Coord::from_mils(10)),
        subpoly_index: 0,
        user_routed: false,
        union_index: 0,
        track_kind: 0,
        layer_enum_index: layer as i32,
        keepout_restrictions: 0,
        unique_id: None,
    });
    fp.primitives.push(prim);
    let pidx = fp.primitives.len() - 1;
    let r = EntityRef {
        domain: "PcbLib".to_owned(),
        entity_type: EntityType::Track,
        id: format!("pcblib:track:{idx}:{pidx}"),
        display_path: format!("{}.track[{pidx}]", fp.display_name),
    };
    Ok(op_result("add_track", &op.opid, Some(r), vec![]))
}
