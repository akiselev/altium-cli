mod lower;
pub mod model;
pub mod schema;

use lower::composed_to_schdoc_low::lower_composed_to_schdoc_low;
use lower::composed_to_schlib_low::lower_composed_to_schlib_low;
use lower::high_to_composed::lower_high_ops;

pub use altium_format::sch_ops_core::{RefExpr, RefRoot, RefStep, Value};
pub use model::{
    AddAliasOp, AddArcHighOp, AddBezierHighOp, AddComponentOp, AddEllipseHighOp,
    AddEllipticalArcHighOp, AddImageHighOp, AddLabelHighOp, AddLineHighOp, AddParameterOp,
    AddPieHighOp, AddPinOp, AddPolygonHighOp, AddPolylineHighOp, AddRectangleHighOp,
    AddRoundRectangleHighOp, AddTextFrameHighOp, ApplyReport, ApplySpec, EditComponentHighOp,
    EditRecordHighOp, HighOp, QueryComponentsHighOp, QueryHighOp, QueryPinsHighOp,
    QueryRecordsHighOp, RemoveAliasOp, RemoveComponentOp, RemoveRecordsHighOp,
};
pub type Ref = RefExpr;

pub fn apply_schdoc(
    doc: &mut altium_format::SchDoc,
    high_ops: &[HighOp],
) -> crate::Result<ApplyReport> {
    let composed = lower_high_ops(high_ops);
    let low = lower_composed_to_schdoc_low(&composed);
    let results = altium_format::sch_ops_core::apply_schdoc_low_ops(doc, &low)?;

    let mut table = model::IndexMap::new();
    for r in results {
        table.insert(r.opid.clone(), r);
    }

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
    let composed = lower_high_ops(high_ops);
    let low = lower_composed_to_schlib_low(&composed);
    let results = altium_format::sch_ops_core::apply_schlib_low_ops(lib, &low)?;

    let mut table = model::IndexMap::new();
    for r in results {
        table.insert(r.opid.clone(), r);
    }

    Ok(ApplyReport {
        high_op_count: high_ops.len(),
        composed_op_count: composed.len(),
        low_op_count: low.len(),
        results: table,
    })
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
