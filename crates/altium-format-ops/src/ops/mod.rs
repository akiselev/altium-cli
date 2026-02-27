mod lower;
pub mod model;
pub mod schema;

use std::collections::HashSet;

use crate::parser::{
    compile_ops_to_high_pcbdoc, compile_ops_to_high_pcblib, compile_ops_to_high_schdoc,
    compile_ops_to_high_schlib,
};
use lower::composed_to_pcbdoc_low::lower_composed_to_pcbdoc_low;
use lower::composed_to_pcblib_low::lower_composed_to_pcblib_low;
use lower::composed_to_schdoc_low::lower_composed_to_schdoc_low;
use lower::composed_to_schlib_low::lower_composed_to_schlib_low;
use lower::high_to_composed::lower_high_ops;

pub use altium_format::sch_ops_core::{RefExpr, RefRoot, RefStep, Value};
pub use model::{
    AddAliasOp, AddArcHighOp, AddBezierHighOp, AddComponentOp, AddEllipseHighOp,
    AddEllipticalArcHighOp, AddFootprintHighOp, AddImageHighOp, AddLabelHighOp, AddLineHighOp,
    AddParameterOp, AddPieHighOp, AddPinOp, AddPolygonHighOp, AddPolylineHighOp,
    AddRectangleHighOp, AddRoundRectangleHighOp, AddTextFrameHighOp, AddTrackHighOp, AddViaHighOp,
    ApplyReport, ApplySpec, EditComponentHighOp, EditRecordHighOp, HighOp, QueryComponentsHighOp,
    QueryHighOp, QueryPinsHighOp, QueryRecordsHighOp, RemoveAliasOp, RemoveComponentOp,
    RemoveRecordsHighOp,
};
pub type Ref = RefExpr;

pub fn apply_schdoc(
    doc: &mut altium_format::SchDoc,
    high_ops: &[HighOp],
) -> crate::Result<ApplyReport> {
    ensure_sch_domain_ops(high_ops)?;
    let composed = lower_high_ops(high_ops);
    let low = lower_composed_to_schdoc_low(&composed);
    let results = altium_format::sch_ops_core::apply_schdoc_low_ops(doc, &low)?;

    let mut table = model::IndexMap::new();
    for r in results {
        table.insert(r.opid.clone(), r);
    }
    synthesize_aggregate_results(high_ops, &mut table);

    Ok(ApplyReport {
        high_op_count: high_ops.len(),
        composed_op_count: composed.len(),
        low_op_count: low.len(),
        results: table,
    })
}

pub fn apply_schlib(
    lib: &mut altium_format::SchLib,
    high_ops: &[HighOp],
) -> crate::Result<ApplyReport> {
    ensure_sch_domain_ops(high_ops)?;
    let composed = lower_high_ops(high_ops);
    let low = lower_composed_to_schlib_low(&composed);
    let results = altium_format::sch_ops_core::apply_schlib_low_ops(lib, &low)?;

    let mut table = model::IndexMap::new();
    for r in results {
        table.insert(r.opid.clone(), r);
    }
    synthesize_aggregate_results(high_ops, &mut table);

    Ok(ApplyReport {
        high_op_count: high_ops.len(),
        composed_op_count: composed.len(),
        low_op_count: low.len(),
        results: table,
    })
}

pub fn apply_pcbdoc(
    doc: &mut altium_format::PcbDoc,
    high_ops: &[HighOp],
) -> crate::Result<ApplyReport> {
    ensure_pcbdoc_domain_ops(high_ops)?;
    let composed = lower_high_ops(high_ops);
    let low = lower_composed_to_pcbdoc_low(&composed)?;
    let results = altium_format::pcb_ops_core::apply_pcbdoc_low_ops(doc, &low)?;

    let mut table = model::IndexMap::new();
    for r in results {
        table.insert(r.opid.clone(), r);
    }
    synthesize_aggregate_results(high_ops, &mut table);

    Ok(ApplyReport {
        high_op_count: high_ops.len(),
        composed_op_count: composed.len(),
        low_op_count: low.len(),
        results: table,
    })
}

pub fn apply_pcblib(
    lib: &mut altium_format::PcbLib,
    high_ops: &[HighOp],
) -> crate::Result<ApplyReport> {
    ensure_pcblib_domain_ops(high_ops)?;
    let composed = lower_high_ops(high_ops);
    let low = lower_composed_to_pcblib_low(&composed)?;
    let results = altium_format::pcb_ops_core::apply_pcblib_low_ops(lib, &low)?;

    let mut table = model::IndexMap::new();
    for r in results {
        table.insert(r.opid.clone(), r);
    }
    synthesize_aggregate_results(high_ops, &mut table);

    Ok(ApplyReport {
        high_op_count: high_ops.len(),
        composed_op_count: composed.len(),
        low_op_count: low.len(),
        results: table,
    })
}

fn ensure_sch_domain_ops(high_ops: &[HighOp]) -> crate::Result<()> {
    for op in high_ops {
        if matches!(op, HighOp::AddTrack(_) | HighOp::AddVia(_) | HighOp::AddFootprint(_)) {
            return Err(crate::AltiumOperationError::Unimplemented(
                "pcb-specific op is not supported in schematic domains".to_owned(),
            ));
        }
    }
    Ok(())
}

fn ensure_pcbdoc_domain_ops(high_ops: &[HighOp]) -> crate::Result<()> {
    for op in high_ops {
        if !matches!(op, HighOp::Query(_) | HighOp::AddTrack(_) | HighOp::AddVia(_)) {
            return Err(crate::AltiumOperationError::Unimplemented(
                "op is not supported for pcbdoc domain".to_owned(),
            ));
        }
    }
    Ok(())
}

fn ensure_pcblib_domain_ops(high_ops: &[HighOp]) -> crate::Result<()> {
    for op in high_ops {
        if !matches!(
            op,
            HighOp::Query(_)
                | HighOp::AddTrack(_)
                | HighOp::AddVia(_)
                | HighOp::AddFootprint(_)
                | HighOp::AddPad(_)
        ) {
            return Err(crate::AltiumOperationError::Unimplemented(
                "op is not supported for pcblib domain".to_owned(),
            ));
        }
    }
    Ok(())
}

fn synthesize_aggregate_results(
    high_ops: &[HighOp],
    table: &mut model::IndexMap<String, altium_format::sch_ops_core::OpResult>,
) {
    for (i, op) in high_ops.iter().enumerate() {
        let base = op
            .opid()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("op_{:04}", i + 1));
        if table.contains_key(&base) {
            continue;
        }
        let prefix = format!("{base}/");
        let children: Vec<&altium_format::sch_ops_core::OpResult> = table
            .iter()
            .filter_map(|(opid, result)| {
                if opid.starts_with(&prefix) {
                    Some(result)
                } else {
                    None
                }
            })
            .collect();
        if children.is_empty() {
            continue;
        }

        let mut seen = HashSet::new();
        let mut refs = Vec::new();
        for child in &children {
            if let Some(r) = &child.ref_ {
                if seen.insert(r.id.clone()) {
                    refs.push(r.clone());
                }
            }
            for r in &child.refs {
                if seen.insert(r.id.clone()) {
                    refs.push(r.clone());
                }
            }
        }
        let primary = children
            .iter()
            .find(|child| child.kind == "create_component_root")
            .and_then(|child| child.ref_.clone())
            .or_else(|| refs.first().cloned());

        let mut fields = model::IndexMap::new();
        if let Some(r) = &primary {
            fields.insert("ref".to_owned(), Value::Ref(r.clone()));
        }
        if !refs.is_empty() {
            fields.insert("refs".to_owned(), Value::Refs(refs.clone()));
        }

        table.insert(
            base.clone(),
            altium_format::sch_ops_core::OpResult {
                opid: base,
                kind: high_op_kind(op).to_owned(),
                ref_: primary,
                refs,
                fields,
                warnings: vec!["synthetic aggregate result".to_owned()],
            },
        );
    }
}

fn high_op_kind(op: &HighOp) -> &'static str {
    match op {
        HighOp::AddComponent(_) => "add_component",
        HighOp::AddPin(_) => "add_pin",
        HighOp::AddParameter(_) => "add_parameter",
        HighOp::AddAlias(_) => "add_alias",
        HighOp::RemoveAlias(_) => "remove_alias",
        HighOp::RemoveComponent(_) => "remove_component",
        HighOp::EditComponent(_) => "edit_component",
        HighOp::EditRecord(_) => "edit_record",
        HighOp::RemoveRecords(_) => "remove_records",
        HighOp::Query(_) => "query",
        HighOp::QueryComponents(_) => "query_components",
        HighOp::QueryPins(_) => "query_pins",
        HighOp::QueryRecords(_) => "query_records",
        HighOp::AddLine(_) => "add_line",
        HighOp::AddRectangle(_) => "add_rectangle",
        HighOp::AddArc(_) => "add_arc",
        HighOp::AddEllipticalArc(_) => "add_elliptical_arc",
        HighOp::AddEllipse(_) => "add_ellipse",
        HighOp::AddPolyline(_) => "add_polyline",
        HighOp::AddPolygon(_) => "add_polygon",
        HighOp::AddBezier(_) => "add_bezier",
        HighOp::AddPie(_) => "add_pie",
        HighOp::AddRoundRectangle(_) => "add_round_rectangle",
        HighOp::AddLabel(_) => "add_label",
        HighOp::AddTextFrame(_) => "add_text_frame",
        HighOp::AddImage(_) => "add_image",
        HighOp::AddTrack(_) => "add_track",
        HighOp::AddVia(_) => "add_via",
        HighOp::AddFootprint(_) => "add_footprint",
        HighOp::AddPad(_) => "add_pad",
    }
}

trait HighOpIdExt {
    fn opid(&self) -> Option<&str>;
}

impl HighOpIdExt for HighOp {
    fn opid(&self) -> Option<&str> {
        match self {
            HighOp::AddComponent(v) => v.opid.as_deref(),
            HighOp::AddPin(v) => v.opid.as_deref(),
            HighOp::AddParameter(v) => v.opid.as_deref(),
            HighOp::AddAlias(v) => v.opid.as_deref(),
            HighOp::RemoveAlias(v) => v.opid.as_deref(),
            HighOp::RemoveComponent(v) => v.opid.as_deref(),
            HighOp::EditComponent(v) => v.opid.as_deref(),
            HighOp::EditRecord(v) => v.opid.as_deref(),
            HighOp::RemoveRecords(v) => v.opid.as_deref(),
            HighOp::Query(v) => v.opid.as_deref(),
            HighOp::QueryComponents(v) => v.opid.as_deref(),
            HighOp::QueryPins(v) => v.opid.as_deref(),
            HighOp::QueryRecords(v) => v.opid.as_deref(),
            HighOp::AddLine(v) => v.opid.as_deref(),
            HighOp::AddRectangle(v) => v.opid.as_deref(),
            HighOp::AddArc(v) => v.opid.as_deref(),
            HighOp::AddEllipticalArc(v) => v.opid.as_deref(),
            HighOp::AddEllipse(v) => v.opid.as_deref(),
            HighOp::AddPolyline(v) => v.opid.as_deref(),
            HighOp::AddPolygon(v) => v.opid.as_deref(),
            HighOp::AddBezier(v) => v.opid.as_deref(),
            HighOp::AddPie(v) => v.opid.as_deref(),
            HighOp::AddRoundRectangle(v) => v.opid.as_deref(),
            HighOp::AddLabel(v) => v.opid.as_deref(),
            HighOp::AddTextFrame(v) => v.opid.as_deref(),
            HighOp::AddImage(v) => v.opid.as_deref(),
            HighOp::AddTrack(v) => v.opid.as_deref(),
            HighOp::AddVia(v) => v.opid.as_deref(),
            HighOp::AddFootprint(v) => v.opid.as_deref(),
            HighOp::AddPad(v) => v.opid.as_deref(),
        }
    }
}

pub fn apply_ops_source_schdoc(
    doc: &mut altium_format::SchDoc,
    source: &str,
) -> crate::Result<ApplyReport> {
    let high_ops = compile_ops_to_high_schdoc(source).map_err(|e| {
        crate::AltiumOperationError::Unimplemented(format!(
            "ops parse/typecheck failed:\n{}",
            e.render("input.ops", source)
        ))
    })?;
    apply_schdoc(doc, &high_ops)
}

pub fn apply_ops_source_schlib(
    lib: &mut altium_format::SchLib,
    source: &str,
) -> crate::Result<ApplyReport> {
    let high_ops = compile_ops_to_high_schlib(source).map_err(|e| {
        crate::AltiumOperationError::Unimplemented(format!(
            "ops parse/typecheck failed:\n{}",
            e.render("input.ops", source)
        ))
    })?;
    apply_schlib(lib, &high_ops)
}

pub fn apply_ops_source_pcbdoc(
    doc: &mut altium_format::PcbDoc,
    source: &str,
) -> crate::Result<ApplyReport> {
    let high_ops = compile_ops_to_high_pcbdoc(source).map_err(|e| {
        crate::AltiumOperationError::Unimplemented(format!(
            "ops parse/typecheck failed:\n{}",
            e.render("input.ops", source)
        ))
    })?;
    apply_pcbdoc(doc, &high_ops)
}

pub fn apply_ops_source_pcblib(
    lib: &mut altium_format::PcbLib,
    source: &str,
) -> crate::Result<ApplyReport> {
    let high_ops = compile_ops_to_high_pcblib(source).map_err(|e| {
        crate::AltiumOperationError::Unimplemented(format!(
            "ops parse/typecheck failed:\n{}",
            e.render("input.ops", source)
        ))
    })?;
    apply_pcblib(lib, &high_ops)
}

pub fn parse_apply_spec_json(data: &str) -> crate::Result<Vec<HighOp>> {
    let spec: ApplySpec = serde_json::from_str(data).map_err(|e| {
        crate::AltiumOperationError::Unimplemented(format!("invalid JSON spec: {e}"))
    })?;
    Ok(spec.into_ops())
}

pub fn parse_apply_spec_yaml(data: &str) -> crate::Result<Vec<HighOp>> {
    let spec: ApplySpec = serde_yaml::from_str(data).map_err(|e| {
        crate::AltiumOperationError::Unimplemented(format!("invalid YAML spec: {e}"))
    })?;
    Ok(spec.into_ops())
}
