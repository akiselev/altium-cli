use crate::param_collection::ParameterCollection;
use crate::param_value::ToParamValue;
use crate::sch_records::{
    SchDesignator, SchImplementation, SchImplementationList, SchImplementationMap, SchMapDefiner,
    SchParameter, SchParameterList, SchPin, SchPrimitiveBase, SchRecord, parse_component_record,
    parse_text_pin,
};
use crate::schdoc::SchDoc;
use crate::schlib::SchLib;
use crate::{AltiumFormatError, Result};
use altium_format_types::constants::component::LIB_REFERENCE;
use altium_format_types::constants::electrical::ELECTRICAL;
use altium_format_types::constants::pin::PIN_LENGTH;
use altium_format_types::constants::record_structure::OWNER_INDEX;
use altium_format_types::constants::text::NAME;
use altium_format_types::{
    Color, Coord, CoordPoint, ParameterReadOnlyState, ParameterType, PinElectricalType,
    RotationBy90, TextHorzAnchor, TextJustification, TextVertAnchor,
};
use indexmap::IndexMap;
use std::collections::HashMap;

pub type OpId = String;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum RefRoot {
    OpId(OpId),
    Last,
    Self_,
    Sheet,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum RefStep {
    Member(String),
    Index(usize),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RefExpr {
    pub root: RefRoot,
    #[serde(default)]
    pub steps: Vec<RefStep>,
}

impl RefExpr {
    pub fn op(id: impl Into<String>) -> Self {
        Self {
            root: RefRoot::OpId(id.into()),
            steps: Vec::new(),
        }
    }
    pub fn last() -> Self {
        Self {
            root: RefRoot::Last,
            steps: Vec::new(),
        }
    }
    pub fn self_() -> Self {
        Self {
            root: RefRoot::Self_,
            steps: Vec::new(),
        }
    }
    pub fn sheet() -> Self {
        Self {
            root: RefRoot::Sheet,
            steps: Vec::new(),
        }
    }
    pub fn member(mut self, name: impl Into<String>) -> Self {
        self.steps.push(RefStep::Member(name.into()));
        self
    }
    pub fn index(mut self, idx: usize) -> Self {
        self.steps.push(RefStep::Index(idx));
        self
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum EntityType {
    Component,
    Pin,
    Implementation,
    Other,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EntityRef {
    pub domain: String,
    pub entity_type: EntityType,
    pub id: String,
    pub display_path: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Value {
    Null,
    Bool(bool),
    I64(i64),
    F64(f64),
    String(String),
    Ref(EntityRef),
    Refs(Vec<EntityRef>),
    Map(IndexMap<String, Value>),
    List(Vec<Value>),
    Expr(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OpResult {
    pub opid: OpId,
    pub kind: String,
    pub ref_: Option<EntityRef>,
    pub refs: Vec<EntityRef>,
    pub fields: IndexMap<String, Value>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ComponentRootOp {
    pub opid: OpId,
    pub id: Option<String>,
    pub lib_reference: String,
    pub designator: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ComponentTextOp {
    pub opid: OpId,
    pub component_ref: Option<RefExpr>,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct ComponentRefOp {
    pub opid: OpId,
    pub component_ref: Option<RefExpr>,
}

#[derive(Debug, Clone)]
pub struct PinOp {
    pub opid: OpId,
    pub component_ref: Option<RefExpr>,
    pub designator: String,
    pub name: Option<String>,
    pub electrical: Option<String>,
    pub length_mils: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct ImplementationOp {
    pub opid: OpId,
    pub component_ref: Option<RefExpr>,
    pub model_name: String,
    pub model_type: Option<String>,
    pub is_current: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct MapDefinerOp {
    pub opid: OpId,
    pub component_ref: Option<RefExpr>,
    pub pin_designator: String,
    pub pad_designator: String,
}

#[derive(Debug, Clone)]
pub struct QueryOp {
    pub opid: OpId,
    pub selector: String,
}

#[derive(Debug, Clone)]
pub struct ParameterOp {
    pub opid: OpId,
    pub component_ref: Option<RefExpr>,
    pub name: String,
    pub text: String,
    pub is_hidden: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct AddAliasOp {
    pub opid: OpId,
    pub component_ref: RefExpr,
    pub alias_name: String,
}

#[derive(Debug, Clone)]
pub struct RemoveAliasOp {
    pub opid: OpId,
    pub component_ref: RefExpr,
    pub alias_name: String,
}

#[derive(Debug, Clone)]
pub struct RemoveComponentOp {
    pub opid: OpId,
    pub component_ref: RefExpr,
}

#[derive(Debug, Clone)]
pub struct EditComponentOp {
    pub opid: OpId,
    pub component_ref: RefExpr,
    pub description: Option<String>,
    pub part_count: Option<i32>,
    pub display_mode_count: Option<i32>,
    pub component_kind: Option<i32>,
    pub show_hidden_pins: Option<bool>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum RecordSelector {
    ByDesignator(String),
    ByRecordType(i32),
    ByIndex(usize),
    ByName(String),
}

#[derive(Debug, Clone)]
pub struct EditRecordOp {
    pub opid: OpId,
    pub component_ref: Option<RefExpr>,
    pub selector: RecordSelector,
    pub patch: RecordPatch,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct RecordPatch {
    pub text: Option<String>,
    pub name: Option<String>,
    pub designator: Option<String>,
    pub is_hidden: Option<bool>,
    pub color: Option<i32>,
    pub line_width: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct RemoveRecordsOp {
    pub opid: OpId,
    pub component_ref: Option<RefExpr>,
    pub selector: RecordSelector,
}

#[derive(Debug, Clone)]
pub struct QueryComponentsOp {
    pub opid: OpId,
    pub pattern: Option<String>,
}

#[derive(Debug, Clone)]
pub struct QueryPinsOp {
    pub opid: OpId,
    pub component_ref: RefExpr,
}

#[derive(Debug, Clone)]
pub struct QueryRecordsOp {
    pub opid: OpId,
    pub component_ref: RefExpr,
    pub record_type: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct AddLineOp {
    pub opid: OpId,
    pub component_ref: Option<RefExpr>,
    pub x1_mils: i32,
    pub y1_mils: i32,
    pub x2_mils: i32,
    pub y2_mils: i32,
    pub color: Option<i32>,
    pub line_width: Option<i32>,
    pub line_style: Option<i32>,
    pub owner_part_id: Option<i32>,
    pub owner_part_display_mode: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct AddRectangleOp {
    pub opid: OpId,
    pub component_ref: Option<RefExpr>,
    pub x1_mils: i32,
    pub y1_mils: i32,
    pub x2_mils: i32,
    pub y2_mils: i32,
    pub color: Option<i32>,
    pub area_color: Option<i32>,
    pub is_solid: Option<bool>,
    pub transparent: Option<bool>,
    pub line_width: Option<i32>,
    pub owner_part_id: Option<i32>,
    pub owner_part_display_mode: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct AddArcOp {
    pub opid: OpId,
    pub component_ref: Option<RefExpr>,
    pub cx_mils: i32,
    pub cy_mils: i32,
    pub radius_mils: i32,
    pub start_angle: Option<f64>,
    pub end_angle: Option<f64>,
    pub color: Option<i32>,
    pub line_width: Option<i32>,
    pub owner_part_id: Option<i32>,
    pub owner_part_display_mode: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct AddEllipticalArcOp {
    pub opid: OpId,
    pub component_ref: Option<RefExpr>,
    pub cx_mils: i32,
    pub cy_mils: i32,
    pub radius_mils: i32,
    pub secondary_radius_mils: i32,
    pub start_angle: Option<f64>,
    pub end_angle: Option<f64>,
    pub color: Option<i32>,
    pub line_width: Option<i32>,
    pub owner_part_id: Option<i32>,
    pub owner_part_display_mode: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct AddEllipseOp {
    pub opid: OpId,
    pub component_ref: Option<RefExpr>,
    pub cx_mils: i32,
    pub cy_mils: i32,
    pub radius_mils: i32,
    pub secondary_radius_mils: i32,
    pub color: Option<i32>,
    pub area_color: Option<i32>,
    pub is_solid: Option<bool>,
    pub line_width: Option<i32>,
    pub owner_part_id: Option<i32>,
    pub owner_part_display_mode: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct AddPolylineOp {
    pub opid: OpId,
    pub component_ref: Option<RefExpr>,
    pub points_mils: Vec<(i32, i32)>,
    pub color: Option<i32>,
    pub line_width: Option<i32>,
    pub line_style: Option<i32>,
    pub owner_part_id: Option<i32>,
    pub owner_part_display_mode: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct AddPolygonOp {
    pub opid: OpId,
    pub component_ref: Option<RefExpr>,
    pub points_mils: Vec<(i32, i32)>,
    pub color: Option<i32>,
    pub area_color: Option<i32>,
    pub is_solid: Option<bool>,
    pub line_width: Option<i32>,
    pub owner_part_id: Option<i32>,
    pub owner_part_display_mode: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct AddBezierOp {
    pub opid: OpId,
    pub component_ref: Option<RefExpr>,
    pub points_mils: Vec<(i32, i32)>,
    pub color: Option<i32>,
    pub line_width: Option<i32>,
    pub owner_part_id: Option<i32>,
    pub owner_part_display_mode: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct AddPieOp {
    pub opid: OpId,
    pub component_ref: Option<RefExpr>,
    pub cx_mils: i32,
    pub cy_mils: i32,
    pub radius_mils: i32,
    pub start_angle: Option<f64>,
    pub end_angle: Option<f64>,
    pub color: Option<i32>,
    pub area_color: Option<i32>,
    pub is_solid: Option<bool>,
    pub line_width: Option<i32>,
    pub owner_part_id: Option<i32>,
    pub owner_part_display_mode: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct AddRoundRectangleOp {
    pub opid: OpId,
    pub component_ref: Option<RefExpr>,
    pub x1_mils: i32,
    pub y1_mils: i32,
    pub x2_mils: i32,
    pub y2_mils: i32,
    pub corner_x_radius_mils: i32,
    pub corner_y_radius_mils: i32,
    pub color: Option<i32>,
    pub area_color: Option<i32>,
    pub is_solid: Option<bool>,
    pub line_width: Option<i32>,
    pub owner_part_id: Option<i32>,
    pub owner_part_display_mode: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct AddLabelOp {
    pub opid: OpId,
    pub component_ref: Option<RefExpr>,
    pub x_mils: i32,
    pub y_mils: i32,
    pub text: String,
    pub color: Option<i32>,
    pub font_id: Option<i32>,
    pub orientation: Option<i32>,
    pub justification: Option<i32>,
    pub is_mirrored: Option<bool>,
    pub owner_part_id: Option<i32>,
    pub owner_part_display_mode: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct AddTextFrameOp {
    pub opid: OpId,
    pub component_ref: Option<RefExpr>,
    pub x1_mils: i32,
    pub y1_mils: i32,
    pub x2_mils: i32,
    pub y2_mils: i32,
    pub text: String,
    pub color: Option<i32>,
    pub area_color: Option<i32>,
    pub font_id: Option<i32>,
    pub alignment: Option<i32>,
    pub word_wrap: Option<bool>,
    pub show_border: Option<bool>,
    pub is_solid: Option<bool>,
    pub clip_to_rect: Option<bool>,
    pub owner_part_id: Option<i32>,
    pub owner_part_display_mode: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct AddImageOp {
    pub opid: OpId,
    pub component_ref: Option<RefExpr>,
    pub x1_mils: i32,
    pub y1_mils: i32,
    pub x2_mils: i32,
    pub y2_mils: i32,
    pub file_name: String,
    pub image_data: Option<Vec<u8>>,
    pub keep_aspect: Option<bool>,
    pub owner_part_id: Option<i32>,
    pub owner_part_display_mode: Option<i32>,
}

#[derive(Debug, Clone)]
pub enum SchDocLowOp {
    CreateComponentRoot(ComponentRootOp),
    CreateComponentDesignator(ComponentTextOp),
    CreateComponentComment(ComponentTextOp),
    AddPin(PinOp),
    CreateImplementationList(ComponentRefOp),
    CreateImplementation(ImplementationOp),
    CreateImplementationMap(ComponentRefOp),
    CreateMapDefiner(MapDefinerOp),
    CreateParameterList(ComponentRefOp),
    AddParameter(ParameterOp),
    Query(QueryOp),
    AddAlias(AddAliasOp),
    RemoveAlias(RemoveAliasOp),
    RemoveComponent(RemoveComponentOp),
    EditComponent(EditComponentOp),
    EditRecord(EditRecordOp),
    RemoveRecords(RemoveRecordsOp),
    QueryComponents(QueryComponentsOp),
    QueryPins(QueryPinsOp),
    QueryRecords(QueryRecordsOp),
    AddLine(AddLineOp),
    AddRectangle(AddRectangleOp),
    AddArc(AddArcOp),
    AddEllipticalArc(AddEllipticalArcOp),
    AddEllipse(AddEllipseOp),
    AddPolyline(AddPolylineOp),
    AddPolygon(AddPolygonOp),
    AddBezier(AddBezierOp),
    AddPie(AddPieOp),
    AddRoundRectangle(AddRoundRectangleOp),
    AddLabel(AddLabelOp),
    AddTextFrame(AddTextFrameOp),
    AddImage(AddImageOp),
}

#[derive(Debug, Clone)]
pub enum SchLibLowOp {
    CreateComponentRoot(ComponentRootOp),
    CreateComponentDesignator(ComponentTextOp),
    CreateComponentComment(ComponentTextOp),
    AddPin(PinOp),
    CreateImplementationList(ComponentRefOp),
    CreateImplementation(ImplementationOp),
    CreateImplementationMap(ComponentRefOp),
    CreateMapDefiner(MapDefinerOp),
    CreateParameterList(ComponentRefOp),
    AddParameter(ParameterOp),
    Query(QueryOp),
    AddAlias(AddAliasOp),
    RemoveAlias(RemoveAliasOp),
    RemoveComponent(RemoveComponentOp),
    EditComponent(EditComponentOp),
    EditRecord(EditRecordOp),
    RemoveRecords(RemoveRecordsOp),
    QueryComponents(QueryComponentsOp),
    QueryPins(QueryPinsOp),
    QueryRecords(QueryRecordsOp),
    AddLine(AddLineOp),
    AddRectangle(AddRectangleOp),
    AddArc(AddArcOp),
    AddEllipticalArc(AddEllipticalArcOp),
    AddEllipse(AddEllipseOp),
    AddPolyline(AddPolylineOp),
    AddPolygon(AddPolygonOp),
    AddBezier(AddBezierOp),
    AddPie(AddPieOp),
    AddRoundRectangle(AddRoundRectangleOp),
    AddLabel(AddLabelOp),
    AddTextFrame(AddTextFrameOp),
    AddImage(AddImageOp),
}

pub fn apply_schdoc_low_ops(doc: &mut SchDoc, ops: &[SchDocLowOp]) -> Result<Vec<OpResult>> {
    let mut ctx = SchDocExecCtx::new(doc);
    let mut out = Vec::with_capacity(ops.len());
    for op in ops {
        let result = apply_schdoc_low_op(doc, op, &mut ctx)?;
        ctx.last_opid = Some(result.opid.clone());
        ctx.results.insert(result.opid.clone(), result.clone());
        out.push(result);
    }
    doc.header.weight = doc.records.len() as i32;
    Ok(out)
}

pub fn apply_schlib_low_ops(lib: &mut SchLib, ops: &[SchLibLowOp]) -> Result<Vec<OpResult>> {
    let mut ctx = SchLibExecCtx::new(lib);
    let mut out = Vec::with_capacity(ops.len());
    for op in ops {
        let result = apply_schlib_low_op(lib, op, &mut ctx)?;
        ctx.last_opid = Some(result.opid.clone());
        ctx.results.insert(result.opid.clone(), result.clone());
        out.push(result);
    }
    Ok(out)
}

fn apply_schdoc_low_op(
    doc: &mut SchDoc,
    op: &SchDocLowOp,
    ctx: &mut SchDocExecCtx,
) -> Result<OpResult> {
    match op {
        SchDocLowOp::CreateComponentRoot(v) => {
            let idx = schdoc_create_component_root(doc, v)?;
            let eref = component_ref_schdoc(idx, v.designator.as_deref());
            if let Some(id) = &v.id {
                ctx.alias_to_component.insert(id.clone(), idx);
            }
            if let Some(designator) = &v.designator {
                ctx.alias_to_component.insert(designator.clone(), idx);
            }
            ctx.entity_id_to_component.insert(eref.id.clone(), idx);
            ctx.last_component = Some(idx);
            Ok(op_result(
                &v.opid,
                "create_component_root",
                Some(eref),
                vec![],
            ))
        }
        SchDocLowOp::CreateComponentDesignator(v) => {
            let idx = resolve_component_index_schdoc(v.component_ref.as_ref(), ctx)?;
            schdoc_append_designator(doc, idx, &v.text)?;
            Ok(op_result(
                &v.opid,
                "create_component_designator",
                None,
                vec![],
            ))
        }
        SchDocLowOp::CreateComponentComment(v) => {
            let idx = resolve_component_index_schdoc(v.component_ref.as_ref(), ctx)?;
            schdoc_append_comment(doc, idx, &v.text)?;
            Ok(op_result(&v.opid, "create_component_comment", None, vec![]))
        }
        SchDocLowOp::AddPin(v) => {
            let idx = resolve_component_index_schdoc(v.component_ref.as_ref(), ctx)?;
            let pin_record_idx = schdoc_append_pin(doc, idx, v)?;
            let pin_ref = EntityRef {
                domain: "SchDoc".to_owned(),
                entity_type: EntityType::Pin,
                id: format!("schdoc:pin:{pin_record_idx}"),
                display_path: format!("component[{idx}].pin[{}]", v.designator),
            };
            Ok(op_result(&v.opid, "add_pin", Some(pin_ref), vec![]))
        }
        SchDocLowOp::CreateImplementationList(v) => {
            let idx = resolve_component_index_schdoc(v.component_ref.as_ref(), ctx)?;
            let impl_list_idx = schdoc_append_implementation_list(doc, idx)?;
            ctx.chain_state.entry(idx).or_default().impl_list = Some(impl_list_idx);
            Ok(op_result(
                &v.opid,
                "create_implementation_list",
                None,
                vec![],
            ))
        }
        SchDocLowOp::CreateImplementation(v) => {
            let idx = resolve_component_index_schdoc(v.component_ref.as_ref(), ctx)?;
            let impl_list_idx = ctx
                .chain_state
                .get(&idx)
                .and_then(|s| s.impl_list)
                .ok_or_else(|| AltiumFormatError::InvalidParamValue {
                    key: "implementation".to_owned(),
                    detail: "CreateImplementation requires CreateImplementationList first"
                        .to_owned(),
                })?;
            let impl_idx = schdoc_append_implementation(doc, impl_list_idx, &v.model_name)?;
            ctx.chain_state.entry(idx).or_default().implementation = Some(impl_idx);
            let eref = EntityRef {
                domain: "SchDoc".to_owned(),
                entity_type: EntityType::Implementation,
                id: format!("schdoc:implementation:{impl_idx}"),
                display_path: format!("component[{idx}].implementation"),
            };
            Ok(op_result(
                &v.opid,
                "create_implementation",
                Some(eref),
                vec![],
            ))
        }
        SchDocLowOp::CreateImplementationMap(v) => {
            let idx = resolve_component_index_schdoc(v.component_ref.as_ref(), ctx)?;
            let implementation_idx = ctx
                .chain_state
                .get(&idx)
                .and_then(|s| s.implementation)
                .ok_or_else(|| AltiumFormatError::InvalidParamValue {
                    key: "implementation_map".to_owned(),
                    detail: "CreateImplementationMap requires CreateImplementation first"
                        .to_owned(),
                })?;
            let map_idx = schdoc_append_implementation_map(doc, implementation_idx)?;
            ctx.chain_state.entry(idx).or_default().implementation_map = Some(map_idx);
            Ok(op_result(
                &v.opid,
                "create_implementation_map",
                None,
                vec![],
            ))
        }
        SchDocLowOp::CreateMapDefiner(v) => {
            let idx = resolve_component_index_schdoc(v.component_ref.as_ref(), ctx)?;
            let map_idx = ctx
                .chain_state
                .get(&idx)
                .and_then(|s| s.implementation_map)
                .ok_or_else(|| AltiumFormatError::InvalidParamValue {
                    key: "map_definer".to_owned(),
                    detail: "CreateMapDefiner requires CreateImplementationMap first".to_owned(),
                })?;
            schdoc_append_map_definer(doc, map_idx, &v.pin_designator, &v.pad_designator)?;
            Ok(op_result(&v.opid, "create_map_definer", None, vec![]))
        }
        SchDocLowOp::CreateParameterList(v) => {
            let idx = resolve_component_index_schdoc(v.component_ref.as_ref(), ctx)?;
            let implementation_idx = ctx
                .chain_state
                .get(&idx)
                .and_then(|s| s.implementation)
                .ok_or_else(|| AltiumFormatError::InvalidParamValue {
                    key: "parameter_list".to_owned(),
                    detail: "CreateParameterList requires CreateImplementation first".to_owned(),
                })?;
            schdoc_append_parameter_list(doc, implementation_idx)?;
            Ok(op_result(&v.opid, "create_parameter_list", None, vec![]))
        }
        SchDocLowOp::AddParameter(_)
        | SchDocLowOp::AddAlias(_)
        | SchDocLowOp::RemoveAlias(_)
        | SchDocLowOp::RemoveComponent(_)
        | SchDocLowOp::EditComponent(_)
        | SchDocLowOp::EditRecord(_)
        | SchDocLowOp::RemoveRecords(_)
        | SchDocLowOp::QueryComponents(_)
        | SchDocLowOp::QueryPins(_)
        | SchDocLowOp::QueryRecords(_)
        | SchDocLowOp::AddLine(_)
        | SchDocLowOp::AddRectangle(_)
        | SchDocLowOp::AddArc(_)
        | SchDocLowOp::AddEllipticalArc(_)
        | SchDocLowOp::AddEllipse(_)
        | SchDocLowOp::AddPolyline(_)
        | SchDocLowOp::AddPolygon(_)
        | SchDocLowOp::AddBezier(_)
        | SchDocLowOp::AddPie(_)
        | SchDocLowOp::AddRoundRectangle(_)
        | SchDocLowOp::AddLabel(_)
        | SchDocLowOp::AddTextFrame(_)
        | SchDocLowOp::AddImage(_) => Err(AltiumFormatError::InvalidParamValue {
            key: "op".to_owned(),
            detail: "operation is not supported for SchDoc".to_owned(),
        }),
        SchDocLowOp::Query(v) => schdoc_query(doc, v),
    }
}

fn apply_schlib_low_op(
    lib: &mut SchLib,
    op: &SchLibLowOp,
    ctx: &mut SchLibExecCtx,
) -> Result<OpResult> {
    match op {
        SchLibLowOp::CreateComponentRoot(v) => {
            let idx = lib.ops_append_component_root(v)?;
            let eref = component_ref_schlib(idx, &v.lib_reference);
            if let Some(id) = &v.id {
                ctx.alias_to_component.insert(id.clone(), idx);
            }
            if !v.lib_reference.is_empty() {
                ctx.alias_to_component.insert(v.lib_reference.clone(), idx);
            }
            ctx.entity_id_to_component.insert(eref.id.clone(), idx);
            ctx.last_component = Some(idx);
            Ok(op_result(
                &v.opid,
                "create_component_root",
                Some(eref),
                vec![],
            ))
        }
        SchLibLowOp::CreateComponentDesignator(v) => {
            let idx = resolve_component_index_schlib(lib, v.component_ref.as_ref(), ctx)?;
            lib.ops_append_designator(idx, &v.text)?;
            Ok(op_result(
                &v.opid,
                "create_component_designator",
                None,
                vec![],
            ))
        }
        SchLibLowOp::CreateComponentComment(v) => {
            let idx = resolve_component_index_schlib(lib, v.component_ref.as_ref(), ctx)?;
            lib.ops_append_comment(idx, &v.text)?;
            Ok(op_result(&v.opid, "create_component_comment", None, vec![]))
        }
        SchLibLowOp::AddPin(v) => {
            let idx = resolve_component_index_schlib(lib, v.component_ref.as_ref(), ctx)?;
            lib.ops_append_pin(idx, v)?;
            let pin_ref = EntityRef {
                domain: "SchLib".to_owned(),
                entity_type: EntityType::Pin,
                id: format!("schlib:component:{idx}:pin:{}", v.designator),
                display_path: format!("component[{idx}].pin[{}]", v.designator),
            };
            Ok(op_result(&v.opid, "add_pin", Some(pin_ref), vec![]))
        }
        SchLibLowOp::CreateImplementationList(v) => {
            let idx = resolve_component_index_schlib(lib, v.component_ref.as_ref(), ctx)?;
            let stream_idx = lib.ops_append_implementation_list(idx)?;
            ctx.chain_state.entry(idx).or_default().impl_list = Some(stream_idx);
            Ok(op_result(
                &v.opid,
                "create_implementation_list",
                None,
                vec![],
            ))
        }
        SchLibLowOp::CreateImplementation(v) => {
            let idx = resolve_component_index_schlib(lib, v.component_ref.as_ref(), ctx)?;
            let impl_list_stream_idx = ctx
                .chain_state
                .get(&idx)
                .and_then(|s| s.impl_list)
                .ok_or_else(|| AltiumFormatError::InvalidParamValue {
                    key: "implementation".to_owned(),
                    detail: "CreateImplementation requires CreateImplementationList first"
                        .to_owned(),
                })?;
            let impl_stream_idx = lib.ops_append_implementation(
                idx,
                impl_list_stream_idx,
                &v.model_name,
                v.model_type.as_deref(),
                v.is_current,
            )?;
            ctx.chain_state.entry(idx).or_default().implementation = Some(impl_stream_idx);
            let eref = EntityRef {
                domain: "SchLib".to_owned(),
                entity_type: EntityType::Implementation,
                id: format!("schlib:component:{idx}:implementation:{impl_stream_idx}"),
                display_path: format!("component[{idx}].implementation"),
            };
            Ok(op_result(
                &v.opid,
                "create_implementation",
                Some(eref),
                vec![],
            ))
        }
        SchLibLowOp::CreateImplementationMap(v) => {
            let idx = resolve_component_index_schlib(lib, v.component_ref.as_ref(), ctx)?;
            let impl_stream_idx = ctx
                .chain_state
                .get(&idx)
                .and_then(|s| s.implementation)
                .ok_or_else(|| AltiumFormatError::InvalidParamValue {
                    key: "implementation_map".to_owned(),
                    detail: "CreateImplementationMap requires CreateImplementation first"
                        .to_owned(),
                })?;
            let map_stream_idx = lib.ops_append_implementation_map(idx, impl_stream_idx)?;
            ctx.chain_state.entry(idx).or_default().implementation_map = Some(map_stream_idx);
            Ok(op_result(
                &v.opid,
                "create_implementation_map",
                None,
                vec![],
            ))
        }
        SchLibLowOp::CreateMapDefiner(v) => {
            let idx = resolve_component_index_schlib(lib, v.component_ref.as_ref(), ctx)?;
            let map_stream_idx = ctx
                .chain_state
                .get(&idx)
                .and_then(|s| s.implementation_map)
                .ok_or_else(|| AltiumFormatError::InvalidParamValue {
                    key: "map_definer".to_owned(),
                    detail: "CreateMapDefiner requires CreateImplementationMap first".to_owned(),
                })?;
            lib.ops_append_map_definer(idx, map_stream_idx, &v.pin_designator, &v.pad_designator)?;
            Ok(op_result(&v.opid, "create_map_definer", None, vec![]))
        }
        SchLibLowOp::CreateParameterList(v) => {
            let idx = resolve_component_index_schlib(lib, v.component_ref.as_ref(), ctx)?;
            let impl_stream_idx = ctx
                .chain_state
                .get(&idx)
                .and_then(|s| s.implementation)
                .ok_or_else(|| AltiumFormatError::InvalidParamValue {
                    key: "parameter_list".to_owned(),
                    detail: "CreateParameterList requires CreateImplementation first".to_owned(),
                })?;
            lib.ops_append_parameter_list(idx, impl_stream_idx)?;
            Ok(op_result(&v.opid, "create_parameter_list", None, vec![]))
        }
        SchLibLowOp::AddParameter(v) => {
            let idx = resolve_component_index_schlib(lib, v.component_ref.as_ref(), ctx)?;
            lib.ops_append_parameter(idx, v)?;
            Ok(op_result(&v.opid, "add_parameter", None, vec![]))
        }
        SchLibLowOp::AddAlias(v) => {
            let idx = resolve_component_index_schlib(lib, Some(&v.component_ref), ctx)?;
            lib.ops_add_alias(idx, &v.alias_name)?;
            Ok(op_result(&v.opid, "add_alias", None, vec![]))
        }
        SchLibLowOp::RemoveAlias(v) => {
            let idx = resolve_component_index_schlib(lib, Some(&v.component_ref), ctx)?;
            lib.ops_remove_alias(idx, &v.alias_name)?;
            Ok(op_result(&v.opid, "remove_alias", None, vec![]))
        }
        SchLibLowOp::RemoveComponent(v) => {
            let idx = resolve_component_index_schlib(lib, Some(&v.component_ref), ctx)?;
            lib.ops_remove_component(idx)?;
            ctx.last_component = None;
            ctx.chain_state.clear();
            ctx.entity_id_to_component.clear();
            for i in 0..lib.component_count() {
                ctx.entity_id_to_component
                    .insert(format!("schlib:component:{i}"), i);
            }
            Ok(op_result(&v.opid, "remove_component", None, vec![]))
        }
        SchLibLowOp::EditComponent(v) => {
            let idx = resolve_component_index_schlib(lib, Some(&v.component_ref), ctx)?;
            lib.ops_edit_component(idx, v)?;
            Ok(op_result(&v.opid, "edit_component", None, vec![]))
        }
        SchLibLowOp::EditRecord(v) => {
            let idx = resolve_component_index_schlib(lib, v.component_ref.as_ref(), ctx)?;
            let changed = lib.ops_edit_records(idx, &v.selector, &v.patch)?;
            let mut r = op_result(&v.opid, "edit_record", None, vec![]);
            r.fields
                .insert("changed".to_owned(), Value::I64(changed as i64));
            Ok(r)
        }
        SchLibLowOp::RemoveRecords(v) => {
            let idx = resolve_component_index_schlib(lib, v.component_ref.as_ref(), ctx)?;
            let removed = lib.ops_remove_records(idx, &v.selector)?;
            let mut r = op_result(&v.opid, "remove_records", None, vec![]);
            r.fields
                .insert("removed".to_owned(), Value::I64(removed as i64));
            Ok(r)
        }
        SchLibLowOp::QueryComponents(v) => schlib_query_components(lib, v),
        SchLibLowOp::QueryPins(v) => schlib_query_pins(lib, v, ctx),
        SchLibLowOp::QueryRecords(v) => schlib_query_records(lib, v, ctx),
        SchLibLowOp::AddLine(v) => {
            let idx = resolve_component_index_schlib(lib, v.component_ref.as_ref(), ctx)?;
            lib.ops_append_line(idx, v)?;
            Ok(op_result(&v.opid, "add_line", None, vec![]))
        }
        SchLibLowOp::AddRectangle(v) => {
            let idx = resolve_component_index_schlib(lib, v.component_ref.as_ref(), ctx)?;
            lib.ops_append_rectangle(idx, v)?;
            Ok(op_result(&v.opid, "add_rectangle", None, vec![]))
        }
        SchLibLowOp::AddArc(v) => {
            let idx = resolve_component_index_schlib(lib, v.component_ref.as_ref(), ctx)?;
            lib.ops_append_arc(idx, v)?;
            Ok(op_result(&v.opid, "add_arc", None, vec![]))
        }
        SchLibLowOp::AddEllipticalArc(v) => {
            let idx = resolve_component_index_schlib(lib, v.component_ref.as_ref(), ctx)?;
            lib.ops_append_elliptical_arc(idx, v)?;
            Ok(op_result(&v.opid, "add_elliptical_arc", None, vec![]))
        }
        SchLibLowOp::AddEllipse(v) => {
            let idx = resolve_component_index_schlib(lib, v.component_ref.as_ref(), ctx)?;
            lib.ops_append_ellipse(idx, v)?;
            Ok(op_result(&v.opid, "add_ellipse", None, vec![]))
        }
        SchLibLowOp::AddPolyline(v) => {
            let idx = resolve_component_index_schlib(lib, v.component_ref.as_ref(), ctx)?;
            lib.ops_append_polyline(idx, v)?;
            Ok(op_result(&v.opid, "add_polyline", None, vec![]))
        }
        SchLibLowOp::AddPolygon(v) => {
            let idx = resolve_component_index_schlib(lib, v.component_ref.as_ref(), ctx)?;
            lib.ops_append_polygon(idx, v)?;
            Ok(op_result(&v.opid, "add_polygon", None, vec![]))
        }
        SchLibLowOp::AddBezier(v) => {
            let idx = resolve_component_index_schlib(lib, v.component_ref.as_ref(), ctx)?;
            lib.ops_append_bezier(idx, v)?;
            Ok(op_result(&v.opid, "add_bezier", None, vec![]))
        }
        SchLibLowOp::AddPie(v) => {
            let idx = resolve_component_index_schlib(lib, v.component_ref.as_ref(), ctx)?;
            lib.ops_append_pie(idx, v)?;
            Ok(op_result(&v.opid, "add_pie", None, vec![]))
        }
        SchLibLowOp::AddRoundRectangle(v) => {
            let idx = resolve_component_index_schlib(lib, v.component_ref.as_ref(), ctx)?;
            lib.ops_append_round_rectangle(idx, v)?;
            Ok(op_result(&v.opid, "add_round_rectangle", None, vec![]))
        }
        SchLibLowOp::AddLabel(v) => {
            let idx = resolve_component_index_schlib(lib, v.component_ref.as_ref(), ctx)?;
            lib.ops_append_label(idx, v)?;
            Ok(op_result(&v.opid, "add_label", None, vec![]))
        }
        SchLibLowOp::AddTextFrame(v) => {
            let idx = resolve_component_index_schlib(lib, v.component_ref.as_ref(), ctx)?;
            lib.ops_append_text_frame(idx, v)?;
            Ok(op_result(&v.opid, "add_text_frame", None, vec![]))
        }
        SchLibLowOp::AddImage(v) => {
            let idx = resolve_component_index_schlib(lib, v.component_ref.as_ref(), ctx)?;
            lib.ops_append_image(idx, v)?;
            Ok(op_result(&v.opid, "add_image", None, vec![]))
        }
        SchLibLowOp::Query(v) => schlib_query(lib, v),
    }
}

fn op_result(opid: &str, kind: &str, ref_: Option<EntityRef>, refs: Vec<EntityRef>) -> OpResult {
    let mut fields = IndexMap::new();
    if let Some(r) = &ref_ {
        fields.insert("ref".to_owned(), Value::Ref(r.clone()));
    }
    if !refs.is_empty() {
        fields.insert("refs".to_owned(), Value::Refs(refs.clone()));
    }
    OpResult {
        opid: opid.to_owned(),
        kind: kind.to_owned(),
        ref_,
        refs,
        fields,
        warnings: Vec::new(),
    }
}

fn schdoc_query(doc: &SchDoc, op: &QueryOp) -> Result<OpResult> {
    let refs = schdoc_query_component_refs(doc, &op.selector)?;
    let primary = if refs.len() == 1 {
        refs.first().cloned()
    } else {
        None
    };
    Ok(op_result(&op.opid, "query", primary, refs))
}

fn schlib_query(lib: &SchLib, op: &QueryOp) -> Result<OpResult> {
    let refs = schlib_query_component_refs(lib, &op.selector)?;
    let primary = if refs.len() == 1 {
        refs.first().cloned()
    } else {
        None
    };
    Ok(op_result(&op.opid, "query", primary, refs))
}

fn schlib_query_components(lib: &SchLib, op: &QueryComponentsOp) -> Result<OpResult> {
    let rows = lib.ops_query_components(op.pattern.as_deref());
    let mut refs = Vec::new();
    let mut components = Vec::new();
    for row in rows {
        let eref = component_ref_schlib(row.index, &row.lib_reference);
        refs.push(eref.clone());
        let mut m = IndexMap::new();
        m.insert("index".to_owned(), Value::I64(row.index as i64));
        m.insert("lib_reference".to_owned(), Value::String(row.lib_reference));
        m.insert("description".to_owned(), Value::String(row.description));
        m.insert("part_count".to_owned(), Value::I64(row.part_count as i64));
        m.insert("pin_count".to_owned(), Value::I64(row.pin_count as i64));
        m.insert(
            "aliases".to_owned(),
            Value::List(row.aliases.into_iter().map(Value::String).collect()),
        );
        m.insert("has_footprint".to_owned(), Value::Bool(row.has_footprint));
        components.push(Value::Map(m));
    }
    let mut r = op_result(&op.opid, "query_components", None, refs);
    r.fields
        .insert("components".to_owned(), Value::List(components));
    Ok(r)
}

fn schlib_query_pins(lib: &SchLib, op: &QueryPinsOp, ctx: &SchLibExecCtx) -> Result<OpResult> {
    let idx = resolve_component_index_schlib(lib, Some(&op.component_ref), ctx)?;
    let rows = lib.ops_query_pins(idx)?;
    let mut pins = Vec::new();
    for row in rows {
        let mut m = IndexMap::new();
        m.insert("designator".to_owned(), Value::String(row.designator));
        m.insert("name".to_owned(), Value::String(row.name));
        m.insert("electrical".to_owned(), Value::String(row.electrical));
        m.insert("x".to_owned(), Value::I64(row.location.x.to_mils() as i64));
        m.insert("y".to_owned(), Value::I64(row.location.y.to_mils() as i64));
        m.insert("length".to_owned(), Value::I64(row.length.to_mils() as i64));
        m.insert("orientation".to_owned(), Value::I64(row.orientation as i64));
        m.insert("is_hidden".to_owned(), Value::Bool(row.is_hidden));
        m.insert(
            "owner_part_id".to_owned(),
            Value::I64(row.owner_part_id as i64),
        );
        pins.push(Value::Map(m));
    }
    let mut r = op_result(&op.opid, "query_pins", None, vec![]);
    r.fields.insert("pins".to_owned(), Value::List(pins));
    Ok(r)
}

fn schlib_query_records(
    lib: &SchLib,
    op: &QueryRecordsOp,
    ctx: &SchLibExecCtx,
) -> Result<OpResult> {
    let idx = resolve_component_index_schlib(lib, Some(&op.component_ref), ctx)?;
    let rows = lib.ops_query_records(idx, op.record_type)?;
    let mut records = Vec::new();
    for row in rows {
        let mut m = IndexMap::new();
        m.insert("index".to_owned(), Value::I64(row.index as i64));
        m.insert("record_type".to_owned(), Value::I64(row.record_type as i64));
        m.insert("owner_index".to_owned(), Value::I64(row.owner_index as i64));
        m.insert("summary".to_owned(), Value::String(row.summary));
        records.push(Value::Map(m));
    }
    let mut r = op_result(&op.opid, "query_records", None, vec![]);
    r.fields.insert("records".to_owned(), Value::List(records));
    Ok(r)
}

fn schdoc_query_component_refs(doc: &SchDoc, selector: &str) -> Result<Vec<EntityRef>> {
    let mut out = Vec::new();
    if selector == "component" {
        for (idx, rec) in doc.records.iter().enumerate() {
            if let SchRecord::Component(c) = rec {
                out.push(component_ref_schdoc(idx, Some(&c.lib_reference)));
            }
        }
        return Ok(out);
    }

    if let Some(designator) = parse_selector_value(selector, "component", "designator") {
        for rec in &doc.records {
            if let SchRecord::Designator(d) = rec {
                if d.text == designator && d.base.owner_index >= 0 {
                    out.push(component_ref_schdoc(
                        d.base.owner_index as usize,
                        Some(&d.text),
                    ));
                }
            }
        }
        return Ok(out);
    }

    Err(AltiumFormatError::InvalidParamValue {
        key: "selector".to_owned(),
        detail: format!("unsupported query selector: {selector}"),
    })
}

fn schlib_query_component_refs(lib: &SchLib, selector: &str) -> Result<Vec<EntityRef>> {
    let mut out = Vec::new();
    if selector == "component" {
        for idx in 0..lib.component_count() {
            if let Some(name) = lib.component_lib_ref(idx) {
                out.push(component_ref_schlib(idx, name));
            }
        }
        return Ok(out);
    }

    if let Some(lib_ref) = parse_selector_value(selector, "component", "lib_reference") {
        if let Some(idx) = lib.ops_find_component_index_by_ref(&lib_ref) {
            out.push(component_ref_schlib(idx, &lib_ref));
        }
        return Ok(out);
    }

    if let Some(designator) = parse_selector_value(selector, "component", "designator") {
        for idx in 0..lib.component_count() {
            if let Some(matches) = lib.component_has_designator(idx, &designator) {
                if matches {
                    let name = lib
                        .component_lib_ref(idx)
                        .map(ToOwned::to_owned)
                        .unwrap_or_else(|| format!("component[{idx}]"));
                    out.push(component_ref_schlib(idx, &name));
                }
            }
        }
        return Ok(out);
    }

    Err(AltiumFormatError::InvalidParamValue {
        key: "selector".to_owned(),
        detail: format!("unsupported query selector: {selector}"),
    })
}

fn parse_selector_value(selector: &str, entity: &str, field: &str) -> Option<String> {
    let prefix = format!("{entity}[{field}=");
    if selector.starts_with(&prefix) && selector.ends_with(']') {
        let raw = &selector[prefix.len()..selector.len() - 1];
        return Some(raw.trim_matches('"').to_owned());
    }
    None
}

pub(crate) fn generate_unique_id() -> String {
    let hex = uuid::Uuid::new_v4().simple().to_string();
    hex[..8].to_ascii_uppercase()
}

fn component_ref_schdoc(component_idx: usize, name_hint: Option<&str>) -> EntityRef {
    EntityRef {
        domain: "SchDoc".to_owned(),
        entity_type: EntityType::Component,
        id: format!("schdoc:component:{component_idx}"),
        display_path: name_hint
            .map(|v| v.to_owned())
            .unwrap_or_else(|| format!("component[{component_idx}]")),
    }
}

fn component_ref_schlib(component_idx: usize, name_hint: &str) -> EntityRef {
    EntityRef {
        domain: "SchLib".to_owned(),
        entity_type: EntityType::Component,
        id: format!("schlib:component:{component_idx}"),
        display_path: name_hint.to_owned(),
    }
}

fn schdoc_create_component_root(doc: &mut SchDoc, op: &ComponentRootOp) -> Result<usize> {
    let mut params = ParameterCollection::new();
    params.insert(LIB_REFERENCE, op.lib_reference.clone());
    let mut component = parse_component_record(&mut params)?;
    component.owner_index = 0;
    component.unique_id = generate_unique_id();
    component.lib_reference = op.lib_reference.clone();
    let idx = doc.records.len();
    doc.records.push(SchRecord::Component(component));
    Ok(idx)
}

fn schdoc_append_designator(doc: &mut SchDoc, component_index: usize, text: &str) -> Result<()> {
    let rec = SchDesignator {
        base: primitive_base(component_index as i32),
        location: CoordPoint::new(Coord::from_internal(0), Coord::from_internal(0)),
        orientation: RotationBy90::Rotate0,
        justification: TextJustification::BottomLeft,
        color: Color::BLACK,
        font_id: 1,
        is_hidden: false,
        text: text.to_owned(),
        name: "Designator".to_owned(),
        show_name: false,
        read_only_state: ParameterReadOnlyState::Name,
        unique_id: generate_unique_id(),
        not_auto_position: false,
        override_not_auto_position: false,
        is_mirrored: false,
    };
    doc.records.push(SchRecord::Designator(rec));
    Ok(())
}

fn schdoc_append_comment(doc: &mut SchDoc, component_index: usize, text: &str) -> Result<()> {
    let rec = SchParameter {
        base: primitive_base(component_index as i32),
        location: CoordPoint::new(Coord::from_internal(0), Coord::from_internal(0)),
        orientation: RotationBy90::Rotate0,
        justification: TextJustification::BottomLeft,
        color: Color::BLACK,
        font_id: 1,
        is_hidden: false,
        text: text.to_owned(),
        param_type: ParameterType::String,
        name: "Comment".to_owned(),
        show_name: false,
        read_only_state: ParameterReadOnlyState::None,
        unique_id: generate_unique_id(),
        description: String::new(),
        not_allow_library_synchronize: false,
        not_allow_database_synchronize: false,
        not_auto_position: false,
        override_not_auto_position: false,
        is_mirrored: false,
        text_horz_anchor: TextHorzAnchor::None,
        text_vert_anchor: TextVertAnchor::None,
        is_image_parameter: false,
    };
    doc.records.push(SchRecord::Parameter(rec));
    Ok(())
}

fn schdoc_append_pin(doc: &mut SchDoc, component_index: usize, pin: &PinOp) -> Result<usize> {
    let mut params = ParameterCollection::new();
    params.insert(OWNER_INDEX, (component_index as i32).to_param_value());
    params.insert("Designator", pin.designator.clone());
    if let Some(name) = &pin.name {
        params.insert(NAME, name.clone());
    }
    if let Some(electrical) = &pin.electrical {
        let code = parse_electrical_type(electrical)? as u8;
        params.insert(ELECTRICAL, (code as i32).to_param_value());
    }
    if let Some(length_mils) = pin.length_mils {
        params.insert_coord(PIN_LENGTH, "PinLength_Frac", Coord::from_mils(length_mils));
    }
    let mut record: SchPin = parse_text_pin(&mut params)?;
    record.unique_id = generate_unique_id();
    let idx = doc.records.len();
    doc.records.push(SchRecord::Pin(record));
    Ok(idx)
}

fn schdoc_append_implementation_list(doc: &mut SchDoc, component_index: usize) -> Result<usize> {
    let idx = doc.records.len();
    doc.records
        .push(SchRecord::ImplementationList(SchImplementationList {
            base: primitive_base(component_index as i32),
        }));
    Ok(idx)
}

fn schdoc_append_implementation(
    doc: &mut SchDoc,
    implementation_list_index: usize,
    model_name: &str,
) -> Result<usize> {
    let idx = doc.records.len();
    doc.records
        .push(SchRecord::Implementation(SchImplementation {
            base: primitive_base(implementation_list_index as i32),
            description: String::new(),
            use_component_library: false,
            model_name: model_name.to_owned(),
            model_type: "PCBLIB".to_owned(),
            datafile_count: 0,
            model_vault_guid: String::new(),
            model_item_guid: String::new(),
            model_revision_guid: String::new(),
            model_datafile0: String::new(),
            model_datafile_entity0: String::new(),
            model_datafile_kind0: String::new(),
            is_current: true,
            datalinks_locked: false,
            database_datalinks_locked: false,
            integrated_model: false,
            database_model: false,
            unique_id: generate_unique_id(),
            model_location: String::new(),
        }));
    Ok(idx)
}

fn schdoc_append_implementation_map(
    doc: &mut SchDoc,
    implementation_index: usize,
) -> Result<usize> {
    let idx = doc.records.len();
    doc.records
        .push(SchRecord::ImplementationMap(SchImplementationMap {
            base: primitive_base(implementation_index as i32),
            unique_id: generate_unique_id(),
        }));
    Ok(idx)
}

fn schdoc_append_map_definer(
    doc: &mut SchDoc,
    implementation_map_index: usize,
    pin_designator: &str,
    pad_designator: &str,
) -> Result<()> {
    doc.records.push(SchRecord::MapDefiner(SchMapDefiner {
        base: primitive_base(implementation_map_index as i32),
        des_intf: pin_designator.to_owned(),
        des_imps: vec![pad_designator.to_owned()],
    }));
    Ok(())
}

fn schdoc_append_parameter_list(doc: &mut SchDoc, implementation_index: usize) -> Result<()> {
    doc.records.push(SchRecord::ParameterList(SchParameterList {
        base: primitive_base(implementation_index as i32),
    }));
    Ok(())
}

pub(crate) fn parse_electrical_type(v: &str) -> Result<PinElectricalType> {
    let key = v.to_ascii_lowercase().replace('_', "");
    let parsed = match key.as_str() {
        "input" => PinElectricalType::Input,
        "io" | "inputoutput" => PinElectricalType::InputOutput,
        "output" => PinElectricalType::Output,
        "opencollector" => PinElectricalType::OpenCollector,
        "passive" => PinElectricalType::Passive,
        "hiz" | "highz" => PinElectricalType::HiZ,
        "openemitter" => PinElectricalType::OpenEmitter,
        "power" => PinElectricalType::Power,
        _ => {
            return Err(AltiumFormatError::InvalidParamValue {
                key: ELECTRICAL.to_owned(),
                detail: format!("unknown electrical type: {v}"),
            });
        }
    };
    Ok(parsed)
}

fn primitive_base(owner_index: i32) -> SchPrimitiveBase {
    SchPrimitiveBase {
        owner_index,
        is_not_accessible: false,
        index_in_sheet: 0,
        owner_part_id: 0,
        owner_part_display_mode: 0,
        graphically_locked: false,
        union_index: 0,
    }
}

struct SchDocExecCtx {
    alias_to_component: HashMap<String, usize>,
    entity_id_to_component: HashMap<String, usize>,
    last_component: Option<usize>,
    chain_state: HashMap<usize, ImplChainState>,
    results: IndexMap<OpId, OpResult>,
    last_opid: Option<OpId>,
}

impl SchDocExecCtx {
    fn new(doc: &SchDoc) -> Self {
        let mut alias_to_component = HashMap::new();
        let mut entity_id_to_component = HashMap::new();
        for (idx, record) in doc.records.iter().enumerate() {
            if matches!(record, SchRecord::Component(_)) {
                entity_id_to_component.insert(format!("schdoc:component:{idx}"), idx);
            }
            if let SchRecord::Designator(d) = record {
                if d.base.owner_index >= 0 {
                    alias_to_component.insert(d.text.clone(), d.base.owner_index as usize);
                }
            }
        }
        Self {
            alias_to_component,
            entity_id_to_component,
            last_component: None,
            chain_state: HashMap::new(),
            results: IndexMap::new(),
            last_opid: None,
        }
    }
}

struct SchLibExecCtx {
    alias_to_component: HashMap<String, usize>,
    entity_id_to_component: HashMap<String, usize>,
    last_component: Option<usize>,
    chain_state: HashMap<usize, ImplChainState>,
    results: IndexMap<OpId, OpResult>,
    last_opid: Option<OpId>,
}

impl SchLibExecCtx {
    fn new(lib: &SchLib) -> Self {
        let mut entity_id_to_component = HashMap::new();
        for idx in 0..lib.component_count() {
            entity_id_to_component.insert(format!("schlib:component:{idx}"), idx);
        }
        Self {
            alias_to_component: HashMap::new(),
            entity_id_to_component,
            last_component: None,
            chain_state: HashMap::new(),
            results: IndexMap::new(),
            last_opid: None,
        }
    }
}

#[derive(Default)]
struct ImplChainState {
    impl_list: Option<usize>,
    implementation: Option<usize>,
    implementation_map: Option<usize>,
}

fn resolve_component_index_schdoc(
    ref_expr: Option<&RefExpr>,
    ctx: &SchDocExecCtx,
) -> Result<usize> {
    if let Some(expr) = ref_expr {
        if let Some(idx) = resolve_component_index_from_expr(
            expr,
            &ctx.results,
            ctx.last_opid.as_ref(),
            &ctx.entity_id_to_component,
        )? {
            return Ok(idx);
        }
        return Err(AltiumFormatError::InvalidParamValue {
            key: "component_ref".to_owned(),
            detail: "resolved ref does not point to a component".to_owned(),
        });
    }

    ctx.last_component
        .ok_or_else(|| AltiumFormatError::InvalidParamValue {
            key: "component_ref".to_owned(),
            detail: "component_ref is required when there is no prior component in apply batch"
                .to_owned(),
        })
}

fn resolve_component_index_schlib(
    lib: &SchLib,
    ref_expr: Option<&RefExpr>,
    ctx: &SchLibExecCtx,
) -> Result<usize> {
    if let Some(expr) = ref_expr {
        if let Some(idx) = resolve_component_index_from_expr(
            expr,
            &ctx.results,
            ctx.last_opid.as_ref(),
            &ctx.entity_id_to_component,
        )? {
            return Ok(idx);
        }
        // fallback to existing lookup by display path semantics if opid ref not found
        if let Some(value) = try_eval_ref_expr(expr, &ctx.results, ctx.last_opid.as_ref()) {
            if let Value::String(s) = value {
                if let Some(idx) = lib.ops_find_component_index_by_ref(&s) {
                    return Ok(idx);
                }
            }
        }
        return Err(AltiumFormatError::InvalidParamValue {
            key: "component_ref".to_owned(),
            detail: "resolved ref does not point to a component".to_owned(),
        });
    }

    ctx.last_component
        .ok_or_else(|| AltiumFormatError::InvalidParamValue {
            key: "component_ref".to_owned(),
            detail: "component_ref is required when there is no prior component in apply batch"
                .to_owned(),
        })
}

fn resolve_component_index_from_expr(
    expr: &RefExpr,
    results: &IndexMap<OpId, OpResult>,
    last_opid: Option<&OpId>,
    entity_id_to_component: &HashMap<String, usize>,
) -> Result<Option<usize>> {
    if let Some(value) = try_eval_ref_expr(expr, results, last_opid) {
        match value {
            Value::Ref(r) => Ok(entity_id_to_component.get(&r.id).copied()),
            Value::Refs(mut rs) => {
                if rs.len() == 1 {
                    Ok(entity_id_to_component.get(&rs.remove(0).id).copied())
                } else {
                    Err(AltiumFormatError::InvalidParamValue {
                        key: "component_ref".to_owned(),
                        detail: "reference resolved to multiple entities".to_owned(),
                    })
                }
            }
            _ => Ok(None),
        }
    } else {
        Ok(None)
    }
}

fn try_eval_ref_expr(
    expr: &RefExpr,
    results: &IndexMap<OpId, OpResult>,
    last_opid: Option<&OpId>,
) -> Option<Value> {
    let mut current = match &expr.root {
        RefRoot::OpId(id) => results.get(id).map(op_result_to_value)?,
        RefRoot::Last => {
            let id = last_opid?;
            results.get(id).map(op_result_to_value)?
        }
        RefRoot::Self_ | RefRoot::Sheet => return None,
    };

    for step in &expr.steps {
        current = match (step, current) {
            (RefStep::Member(name), Value::Map(m)) => m.get(name)?.clone(),
            (RefStep::Index(i), Value::List(v)) => v.get(*i)?.clone(),
            (RefStep::Index(i), Value::Refs(v)) => Value::Ref(v.get(*i)?.clone()),
            _ => return None,
        };
    }
    Some(current)
}

fn op_result_to_value(r: &OpResult) -> Value {
    let mut m = IndexMap::new();
    m.insert("opid".to_owned(), Value::String(r.opid.clone()));
    m.insert("kind".to_owned(), Value::String(r.kind.clone()));
    m.insert(
        "ref".to_owned(),
        r.ref_
            .as_ref()
            .map(|v| Value::Ref(v.clone()))
            .unwrap_or(Value::Null),
    );
    m.insert("refs".to_owned(), Value::Refs(r.refs.clone()));
    m.insert("fields".to_owned(), Value::Map(r.fields.clone()));
    Value::Map(m)
}
