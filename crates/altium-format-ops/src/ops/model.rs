use altium_format::sch_ops_core;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum HighOp {
    AddComponent(AddComponentOp),
    AddPin(AddPinOp),
    AddParameter(AddParameterOp),
    AddAlias(AddAliasOp),
    RemoveAlias(RemoveAliasOp),
    RemoveComponent(RemoveComponentOp),
    EditComponent(EditComponentHighOp),
    EditRecord(EditRecordHighOp),
    RemoveRecords(RemoveRecordsHighOp),
    Query(QueryHighOp),
    QueryComponents(QueryComponentsHighOp),
    QueryPins(QueryPinsHighOp),
    QueryRecords(QueryRecordsHighOp),
    AddLine(AddLineHighOp),
    AddRectangle(AddRectangleHighOp),
    AddArc(AddArcHighOp),
    AddEllipticalArc(AddEllipticalArcHighOp),
    AddEllipse(AddEllipseHighOp),
    AddPolyline(AddPolylineHighOp),
    AddPolygon(AddPolygonHighOp),
    AddBezier(AddBezierHighOp),
    AddPie(AddPieHighOp),
    AddRoundRectangle(AddRoundRectangleHighOp),
    AddLabel(AddLabelHighOp),
    AddTextFrame(AddTextFrameHighOp),
    AddImage(AddImageHighOp),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QueryHighOp {
    #[serde(default)]
    pub opid: Option<String>,
    pub selector: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QueryComponentsHighOp {
    #[serde(default)]
    pub opid: Option<String>,
    #[serde(default)]
    pub pattern: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QueryPinsHighOp {
    #[serde(default)]
    pub opid: Option<String>,
    pub component_ref: sch_ops_core::RefExpr,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QueryRecordsHighOp {
    #[serde(default)]
    pub opid: Option<String>,
    pub component_ref: sch_ops_core::RefExpr,
    #[serde(default)]
    pub record_type: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AddComponentOp {
    #[serde(default)]
    pub opid: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub component_ref: Option<sch_ops_core::RefExpr>,
    pub lib_reference: String,
    #[serde(default)]
    pub designator: Option<String>,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub pins: Vec<AddPinOp>,
    #[serde(default)]
    pub footprint: Option<FootprintOp>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AddPinOp {
    #[serde(default)]
    pub opid: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub component_ref: Option<sch_ops_core::RefExpr>,
    pub designator: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub electrical: Option<String>,
    #[serde(default)]
    pub length_mils: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AddParameterOp {
    #[serde(default)]
    pub opid: Option<String>,
    #[serde(default)]
    pub component_ref: Option<sch_ops_core::RefExpr>,
    pub name: String,
    pub text: String,
    #[serde(default)]
    pub is_hidden: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AddAliasOp {
    #[serde(default)]
    pub opid: Option<String>,
    pub component_ref: sch_ops_core::RefExpr,
    pub alias_name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RemoveAliasOp {
    #[serde(default)]
    pub opid: Option<String>,
    pub component_ref: sch_ops_core::RefExpr,
    pub alias_name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RemoveComponentOp {
    #[serde(default)]
    pub opid: Option<String>,
    pub component_ref: sch_ops_core::RefExpr,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EditComponentHighOp {
    #[serde(default)]
    pub opid: Option<String>,
    pub component_ref: sch_ops_core::RefExpr,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub part_count: Option<i32>,
    #[serde(default)]
    pub display_mode_count: Option<i32>,
    #[serde(default)]
    pub component_kind: Option<i32>,
    #[serde(default)]
    pub show_hidden_pins: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EditRecordHighOp {
    #[serde(default)]
    pub opid: Option<String>,
    #[serde(default)]
    pub component_ref: Option<sch_ops_core::RefExpr>,
    pub selector: sch_ops_core::RecordSelector,
    #[serde(default)]
    pub patch: sch_ops_core::RecordPatch,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RemoveRecordsHighOp {
    #[serde(default)]
    pub opid: Option<String>,
    #[serde(default)]
    pub component_ref: Option<sch_ops_core::RefExpr>,
    pub selector: sch_ops_core::RecordSelector,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AddLineHighOp {
    #[serde(default)]
    pub opid: Option<String>,
    #[serde(default)]
    pub component_ref: Option<sch_ops_core::RefExpr>,
    pub x1_mils: i32,
    pub y1_mils: i32,
    pub x2_mils: i32,
    pub y2_mils: i32,
    #[serde(default)]
    pub color: Option<i32>,
    #[serde(default)]
    pub line_width: Option<i32>,
    #[serde(default)]
    pub line_style: Option<i32>,
    #[serde(default)]
    pub owner_part_id: Option<i32>,
    #[serde(default)]
    pub owner_part_display_mode: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AddRectangleHighOp {
    #[serde(default)]
    pub opid: Option<String>,
    #[serde(default)]
    pub component_ref: Option<sch_ops_core::RefExpr>,
    pub x1_mils: i32,
    pub y1_mils: i32,
    pub x2_mils: i32,
    pub y2_mils: i32,
    #[serde(default)]
    pub color: Option<i32>,
    #[serde(default)]
    pub area_color: Option<i32>,
    #[serde(default)]
    pub is_solid: Option<bool>,
    #[serde(default)]
    pub transparent: Option<bool>,
    #[serde(default)]
    pub line_width: Option<i32>,
    #[serde(default)]
    pub owner_part_id: Option<i32>,
    #[serde(default)]
    pub owner_part_display_mode: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AddArcHighOp {
    #[serde(default)]
    pub opid: Option<String>,
    #[serde(default)]
    pub component_ref: Option<sch_ops_core::RefExpr>,
    pub cx_mils: i32,
    pub cy_mils: i32,
    pub radius_mils: i32,
    #[serde(default)]
    pub start_angle: Option<f64>,
    #[serde(default)]
    pub end_angle: Option<f64>,
    #[serde(default)]
    pub color: Option<i32>,
    #[serde(default)]
    pub line_width: Option<i32>,
    #[serde(default)]
    pub owner_part_id: Option<i32>,
    #[serde(default)]
    pub owner_part_display_mode: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AddEllipticalArcHighOp {
    #[serde(default)]
    pub opid: Option<String>,
    #[serde(default)]
    pub component_ref: Option<sch_ops_core::RefExpr>,
    pub cx_mils: i32,
    pub cy_mils: i32,
    pub radius_mils: i32,
    pub secondary_radius_mils: i32,
    #[serde(default)]
    pub start_angle: Option<f64>,
    #[serde(default)]
    pub end_angle: Option<f64>,
    #[serde(default)]
    pub color: Option<i32>,
    #[serde(default)]
    pub line_width: Option<i32>,
    #[serde(default)]
    pub owner_part_id: Option<i32>,
    #[serde(default)]
    pub owner_part_display_mode: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AddEllipseHighOp {
    #[serde(default)]
    pub opid: Option<String>,
    #[serde(default)]
    pub component_ref: Option<sch_ops_core::RefExpr>,
    pub cx_mils: i32,
    pub cy_mils: i32,
    pub radius_mils: i32,
    pub secondary_radius_mils: i32,
    #[serde(default)]
    pub color: Option<i32>,
    #[serde(default)]
    pub area_color: Option<i32>,
    #[serde(default)]
    pub is_solid: Option<bool>,
    #[serde(default)]
    pub line_width: Option<i32>,
    #[serde(default)]
    pub owner_part_id: Option<i32>,
    #[serde(default)]
    pub owner_part_display_mode: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AddPolylineHighOp {
    #[serde(default)]
    pub opid: Option<String>,
    #[serde(default)]
    pub component_ref: Option<sch_ops_core::RefExpr>,
    pub points_mils: Vec<(i32, i32)>,
    #[serde(default)]
    pub color: Option<i32>,
    #[serde(default)]
    pub line_width: Option<i32>,
    #[serde(default)]
    pub line_style: Option<i32>,
    #[serde(default)]
    pub owner_part_id: Option<i32>,
    #[serde(default)]
    pub owner_part_display_mode: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AddPolygonHighOp {
    #[serde(default)]
    pub opid: Option<String>,
    #[serde(default)]
    pub component_ref: Option<sch_ops_core::RefExpr>,
    pub points_mils: Vec<(i32, i32)>,
    #[serde(default)]
    pub color: Option<i32>,
    #[serde(default)]
    pub area_color: Option<i32>,
    #[serde(default)]
    pub is_solid: Option<bool>,
    #[serde(default)]
    pub line_width: Option<i32>,
    #[serde(default)]
    pub owner_part_id: Option<i32>,
    #[serde(default)]
    pub owner_part_display_mode: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AddBezierHighOp {
    #[serde(default)]
    pub opid: Option<String>,
    #[serde(default)]
    pub component_ref: Option<sch_ops_core::RefExpr>,
    pub points_mils: Vec<(i32, i32)>,
    #[serde(default)]
    pub color: Option<i32>,
    #[serde(default)]
    pub line_width: Option<i32>,
    #[serde(default)]
    pub owner_part_id: Option<i32>,
    #[serde(default)]
    pub owner_part_display_mode: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AddPieHighOp {
    #[serde(default)]
    pub opid: Option<String>,
    #[serde(default)]
    pub component_ref: Option<sch_ops_core::RefExpr>,
    pub cx_mils: i32,
    pub cy_mils: i32,
    pub radius_mils: i32,
    #[serde(default)]
    pub start_angle: Option<f64>,
    #[serde(default)]
    pub end_angle: Option<f64>,
    #[serde(default)]
    pub color: Option<i32>,
    #[serde(default)]
    pub area_color: Option<i32>,
    #[serde(default)]
    pub is_solid: Option<bool>,
    #[serde(default)]
    pub line_width: Option<i32>,
    #[serde(default)]
    pub owner_part_id: Option<i32>,
    #[serde(default)]
    pub owner_part_display_mode: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AddRoundRectangleHighOp {
    #[serde(default)]
    pub opid: Option<String>,
    #[serde(default)]
    pub component_ref: Option<sch_ops_core::RefExpr>,
    pub x1_mils: i32,
    pub y1_mils: i32,
    pub x2_mils: i32,
    pub y2_mils: i32,
    pub corner_x_radius_mils: i32,
    pub corner_y_radius_mils: i32,
    #[serde(default)]
    pub color: Option<i32>,
    #[serde(default)]
    pub area_color: Option<i32>,
    #[serde(default)]
    pub is_solid: Option<bool>,
    #[serde(default)]
    pub line_width: Option<i32>,
    #[serde(default)]
    pub owner_part_id: Option<i32>,
    #[serde(default)]
    pub owner_part_display_mode: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AddLabelHighOp {
    #[serde(default)]
    pub opid: Option<String>,
    #[serde(default)]
    pub component_ref: Option<sch_ops_core::RefExpr>,
    pub x_mils: i32,
    pub y_mils: i32,
    pub text: String,
    #[serde(default)]
    pub color: Option<i32>,
    #[serde(default)]
    pub font_id: Option<i32>,
    #[serde(default)]
    pub orientation: Option<i32>,
    #[serde(default)]
    pub justification: Option<i32>,
    #[serde(default)]
    pub is_mirrored: Option<bool>,
    #[serde(default)]
    pub owner_part_id: Option<i32>,
    #[serde(default)]
    pub owner_part_display_mode: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AddTextFrameHighOp {
    #[serde(default)]
    pub opid: Option<String>,
    #[serde(default)]
    pub component_ref: Option<sch_ops_core::RefExpr>,
    pub x1_mils: i32,
    pub y1_mils: i32,
    pub x2_mils: i32,
    pub y2_mils: i32,
    pub text: String,
    #[serde(default)]
    pub color: Option<i32>,
    #[serde(default)]
    pub area_color: Option<i32>,
    #[serde(default)]
    pub font_id: Option<i32>,
    #[serde(default)]
    pub alignment: Option<i32>,
    #[serde(default)]
    pub word_wrap: Option<bool>,
    #[serde(default)]
    pub show_border: Option<bool>,
    #[serde(default)]
    pub is_solid: Option<bool>,
    #[serde(default)]
    pub clip_to_rect: Option<bool>,
    #[serde(default)]
    pub owner_part_id: Option<i32>,
    #[serde(default)]
    pub owner_part_display_mode: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AddImageHighOp {
    #[serde(default)]
    pub opid: Option<String>,
    #[serde(default)]
    pub component_ref: Option<sch_ops_core::RefExpr>,
    pub x1_mils: i32,
    pub y1_mils: i32,
    pub x2_mils: i32,
    pub y2_mils: i32,
    pub file_name: String,
    #[serde(default)]
    pub image_data: Option<Vec<u8>>,
    #[serde(default)]
    pub keep_aspect: Option<bool>,
    #[serde(default)]
    pub owner_part_id: Option<i32>,
    #[serde(default)]
    pub owner_part_display_mode: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FootprintOp {
    pub model_name: String,
    #[serde(default)]
    pub map: Vec<FootprintMapEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FootprintMapEntry {
    pub pin: String,
    pub pad: String,
}

#[derive(Debug, Clone)]
pub enum ComposedOp {
    CreateComponentRoot(ComponentRoot),
    CreateComponentDesignator(ComponentText),
    CreateComponentComment(ComponentText),
    AddPin(PinNode),
    CreateImplementationList(ComponentRefNode),
    CreateImplementation(ImplementationNode),
    CreateImplementationMap(ComponentRefNode),
    CreateMapDefiner(MapDefinerNode),
    CreateParameterList(ComponentRefNode),
    AddParameter(ParameterNode),
    AddAlias(AliasNode),
    RemoveAlias(AliasNode),
    RemoveComponent(RemoveComponentNode),
    EditComponent(EditComponentNode),
    EditRecord(EditRecordNode),
    RemoveRecords(RemoveRecordsNode),
    Query(QueryNode),
    QueryComponents(QueryComponentsNode),
    QueryPins(QueryPinsNode),
    QueryRecords(QueryRecordsNode),
    AddLine(AddLineNode),
    AddRectangle(AddRectangleNode),
    AddArc(AddArcNode),
    AddEllipticalArc(AddEllipticalArcNode),
    AddEllipse(AddEllipseNode),
    AddPolyline(AddPolylineNode),
    AddPolygon(AddPolygonNode),
    AddBezier(AddBezierNode),
    AddPie(AddPieNode),
    AddRoundRectangle(AddRoundRectangleNode),
    AddLabel(AddLabelNode),
    AddTextFrame(AddTextFrameNode),
    AddImage(AddImageNode),
}

#[derive(Debug, Clone)]
pub struct QueryNode {
    pub opid: String,
    pub selector: String,
}

#[derive(Debug, Clone)]
pub struct QueryComponentsNode {
    pub opid: String,
    pub pattern: Option<String>,
}

#[derive(Debug, Clone)]
pub struct QueryPinsNode {
    pub opid: String,
    pub component_ref: sch_ops_core::RefExpr,
}

#[derive(Debug, Clone)]
pub struct QueryRecordsNode {
    pub opid: String,
    pub component_ref: sch_ops_core::RefExpr,
    pub record_type: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct ComponentRoot {
    pub opid: String,
    pub id: Option<String>,
    pub lib_reference: String,
    pub designator: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ComponentText {
    pub opid: String,
    pub component_ref: Option<sch_ops_core::RefExpr>,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct PinNode {
    pub opid: String,
    pub component_ref: Option<sch_ops_core::RefExpr>,
    pub designator: String,
    pub name: Option<String>,
    pub electrical: Option<String>,
    pub length_mils: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct ComponentRefNode {
    pub opid: String,
    pub component_ref: Option<sch_ops_core::RefExpr>,
}

#[derive(Debug, Clone)]
pub struct ImplementationNode {
    pub opid: String,
    pub component_ref: Option<sch_ops_core::RefExpr>,
    pub model_name: String,
}

#[derive(Debug, Clone)]
pub struct MapDefinerNode {
    pub opid: String,
    pub component_ref: Option<sch_ops_core::RefExpr>,
    pub pin_designator: String,
    pub pad_designator: String,
}

#[derive(Debug, Clone)]
pub struct ParameterNode {
    pub opid: String,
    pub component_ref: Option<sch_ops_core::RefExpr>,
    pub name: String,
    pub text: String,
    pub is_hidden: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct AliasNode {
    pub opid: String,
    pub component_ref: sch_ops_core::RefExpr,
    pub alias_name: String,
}

#[derive(Debug, Clone)]
pub struct RemoveComponentNode {
    pub opid: String,
    pub component_ref: sch_ops_core::RefExpr,
}

#[derive(Debug, Clone)]
pub struct EditComponentNode {
    pub opid: String,
    pub component_ref: sch_ops_core::RefExpr,
    pub description: Option<String>,
    pub part_count: Option<i32>,
    pub display_mode_count: Option<i32>,
    pub component_kind: Option<i32>,
    pub show_hidden_pins: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct EditRecordNode {
    pub opid: String,
    pub component_ref: Option<sch_ops_core::RefExpr>,
    pub selector: sch_ops_core::RecordSelector,
    pub patch: sch_ops_core::RecordPatch,
}

#[derive(Debug, Clone)]
pub struct RemoveRecordsNode {
    pub opid: String,
    pub component_ref: Option<sch_ops_core::RefExpr>,
    pub selector: sch_ops_core::RecordSelector,
}

#[derive(Debug, Clone)]
pub struct AddLineNode(pub sch_ops_core::AddLineOp);
#[derive(Debug, Clone)]
pub struct AddRectangleNode(pub sch_ops_core::AddRectangleOp);
#[derive(Debug, Clone)]
pub struct AddArcNode(pub sch_ops_core::AddArcOp);
#[derive(Debug, Clone)]
pub struct AddEllipticalArcNode(pub sch_ops_core::AddEllipticalArcOp);
#[derive(Debug, Clone)]
pub struct AddEllipseNode(pub sch_ops_core::AddEllipseOp);
#[derive(Debug, Clone)]
pub struct AddPolylineNode(pub sch_ops_core::AddPolylineOp);
#[derive(Debug, Clone)]
pub struct AddPolygonNode(pub sch_ops_core::AddPolygonOp);
#[derive(Debug, Clone)]
pub struct AddBezierNode(pub sch_ops_core::AddBezierOp);
#[derive(Debug, Clone)]
pub struct AddPieNode(pub sch_ops_core::AddPieOp);
#[derive(Debug, Clone)]
pub struct AddRoundRectangleNode(pub sch_ops_core::AddRoundRectangleOp);
#[derive(Debug, Clone)]
pub struct AddLabelNode(pub sch_ops_core::AddLabelOp);
#[derive(Debug, Clone)]
pub struct AddTextFrameNode(pub sch_ops_core::AddTextFrameOp);
#[derive(Debug, Clone)]
pub struct AddImageNode(pub sch_ops_core::AddImageOp);

#[derive(Debug, Clone, Serialize)]
pub struct ApplyReport {
    pub high_op_count: usize,
    pub composed_op_count: usize,
    pub low_op_count: usize,
    pub results: IndexMap<String, sch_ops_core::OpResult>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ApplySpec {
    Ops(Vec<HighOp>),
    Wrapped { ops: Vec<HighOp> },
}

impl ApplySpec {
    pub fn into_ops(self) -> Vec<HighOp> {
        match self {
            Self::Ops(v) => v,
            Self::Wrapped { ops } => ops,
        }
    }
}

pub use indexmap::IndexMap;
