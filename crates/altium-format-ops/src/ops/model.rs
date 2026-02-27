use altium_format::sch_ops_core;
use altium_format_derive::{OpsEnum, OpsSchema};
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
    AddTrack(AddTrackHighOp),
    AddVia(AddViaHighOp),
    AddFootprint(AddFootprintHighOp),
}

#[derive(Debug, Clone, Deserialize, Serialize, OpsSchema)]
#[ops(op = "query", domain = "sch")]
pub struct QueryHighOp {
    #[serde(default)]
    #[ops(ty = "any")]
    pub opid: Option<String>,
    #[ops(ty = "selector", required)]
    pub selector: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, OpsSchema)]
#[ops(op = "query_components", domain = "sch")]
pub struct QueryComponentsHighOp {
    #[serde(default)]
    #[ops(ty = "any")]
    pub opid: Option<String>,
    #[serde(default)]
    #[ops(ty = "string")]
    pub pattern: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, OpsSchema)]
#[ops(op = "query_pins", domain = "sch")]
pub struct QueryPinsHighOp {
    #[serde(default)]
    #[ops(ty = "any")]
    pub opid: Option<String>,
    #[ops(ty = "refexpr", required)]
    pub component_ref: sch_ops_core::RefExpr,
}

#[derive(Debug, Clone, Deserialize, Serialize, OpsSchema)]
#[ops(op = "query_records", domain = "sch")]
pub struct QueryRecordsHighOp {
    #[serde(default)]
    #[ops(ty = "any")]
    pub opid: Option<String>,
    #[ops(ty = "refexpr", required)]
    pub component_ref: sch_ops_core::RefExpr,
    #[serde(default)]
    #[ops(ty = "integer")]
    pub record_type: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, OpsSchema)]
#[ops(op = "add_component", domain = "sch")]
pub struct AddComponentOp {
    #[serde(default)]
    #[ops(ty = "any")]
    pub opid: Option<String>,
    #[serde(default)]
    #[ops(ty = "any")]
    pub id: Option<String>,
    #[serde(default)]
    #[ops(ty = "refexpr")]
    pub component_ref: Option<sch_ops_core::RefExpr>,
    #[ops(ty = "string", required)]
    pub lib_reference: String,
    #[serde(default)]
    #[ops(ty = "string")]
    pub designator: Option<String>,
    #[serde(default)]
    #[ops(ty = "string")]
    pub value: Option<String>,
    #[serde(default)]
    #[ops(ty = "object_array")]
    pub pins: Vec<AddPinOp>,
    #[serde(default)]
    #[ops(ty = "object")]
    pub footprint: Option<FootprintOp>,
}

#[derive(Debug, Clone, Deserialize, Serialize, OpsSchema)]
#[ops(op = "add_pin", domain = "sch")]
pub struct AddPinOp {
    #[serde(default)]
    #[ops(ty = "any")]
    pub opid: Option<String>,
    #[serde(default)]
    #[ops(ty = "any")]
    pub id: Option<String>,
    #[serde(default)]
    #[ops(ty = "refexpr")]
    pub component_ref: Option<sch_ops_core::RefExpr>,
    #[ops(ty = "string", required)]
    pub designator: String,
    #[serde(default)]
    #[ops(ty = "string")]
    pub name: Option<String>,
    #[serde(default)]
    #[ops(ty = "string")]
    pub electrical: Option<String>,
    #[serde(default)]
    #[ops(ty = "dim")]
    pub length_mils: Option<i32>,
    #[serde(default)]
    #[ops(ty = "coord")]
    pub at: Option<(i32, i32)>,
    #[serde(default)]
    #[ops(ty = "integer")]
    pub rotation: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, OpsSchema)]
#[ops(op = "add_parameter", domain = "sch")]
pub struct AddParameterOp {
    #[serde(default)]
    #[ops(ty = "any")]
    pub opid: Option<String>,
    #[serde(default)]
    #[ops(ty = "refexpr")]
    pub component_ref: Option<sch_ops_core::RefExpr>,
    #[ops(ty = "string", required)]
    pub name: String,
    #[ops(ty = "string", required)]
    pub text: String,
    #[serde(default)]
    #[ops(ty = "bool")]
    pub is_hidden: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize, OpsSchema)]
#[ops(op = "add_alias", domain = "sch")]
pub struct AddAliasOp {
    #[serde(default)]
    #[ops(ty = "any")]
    pub opid: Option<String>,
    #[ops(ty = "refexpr", required)]
    pub component_ref: sch_ops_core::RefExpr,
    #[ops(ty = "string", required)]
    pub alias_name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, OpsSchema)]
#[ops(op = "remove_alias", domain = "sch")]
pub struct RemoveAliasOp {
    #[serde(default)]
    #[ops(ty = "any")]
    pub opid: Option<String>,
    #[ops(ty = "refexpr", required)]
    pub component_ref: sch_ops_core::RefExpr,
    #[ops(ty = "string", required)]
    pub alias_name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, OpsSchema)]
#[ops(op = "remove_component", domain = "sch")]
pub struct RemoveComponentOp {
    #[serde(default)]
    #[ops(ty = "any")]
    pub opid: Option<String>,
    #[ops(ty = "refexpr", required)]
    pub component_ref: sch_ops_core::RefExpr,
}

#[derive(Debug, Clone, Deserialize, Serialize, OpsSchema)]
#[ops(op = "edit_component", domain = "sch")]
pub struct EditComponentHighOp {
    #[serde(default)]
    #[ops(ty = "any")]
    pub opid: Option<String>,
    #[ops(ty = "refexpr", required)]
    pub component_ref: sch_ops_core::RefExpr,
    #[serde(default)]
    #[ops(ty = "string")]
    pub description: Option<String>,
    #[serde(default)]
    #[ops(ty = "integer")]
    pub part_count: Option<i32>,
    #[serde(default)]
    #[ops(ty = "integer")]
    pub display_mode_count: Option<i32>,
    #[serde(default)]
    #[ops(ty = "integer")]
    pub component_kind: Option<i32>,
    #[serde(default)]
    #[ops(ty = "bool")]
    pub show_hidden_pins: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize, OpsSchema)]
#[ops(op = "edit_record", domain = "sch")]
pub struct EditRecordHighOp {
    #[serde(default)]
    #[ops(ty = "any")]
    pub opid: Option<String>,
    #[serde(default)]
    #[ops(ty = "refexpr")]
    pub component_ref: Option<sch_ops_core::RefExpr>,
    #[ops(ty = "object", required)]
    pub selector: sch_ops_core::RecordSelector,
    #[serde(default)]
    #[ops(ty = "object")]
    pub patch: sch_ops_core::RecordPatch,
}

#[derive(Debug, Clone, Deserialize, Serialize, OpsSchema)]
#[ops(op = "remove_records", domain = "sch")]
pub struct RemoveRecordsHighOp {
    #[serde(default)]
    #[ops(ty = "any")]
    pub opid: Option<String>,
    #[serde(default)]
    #[ops(ty = "refexpr")]
    pub component_ref: Option<sch_ops_core::RefExpr>,
    #[ops(ty = "object", required)]
    pub selector: sch_ops_core::RecordSelector,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AddLineHighOp {
    #[serde(default)]
    pub opid: Option<String>,
    #[serde(default)]
    pub component_ref: Option<sch_ops_core::RefExpr>,
    pub from: (i32, i32),
    pub to: (i32, i32),
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
    pub from: (i32, i32),
    pub to: (i32, i32),
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
    pub from: (i32, i32),
    pub to: (i32, i32),
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
    pub from: (i32, i32),
    pub to: (i32, i32),
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
    pub from: (i32, i32),
    pub to: (i32, i32),
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
pub struct AddTrackHighOp {
    #[serde(default)]
    pub opid: Option<String>,
    #[serde(default)]
    pub footprint_ref: Option<sch_ops_core::RefExpr>,
    pub start: (i32, i32),
    pub end: (i32, i32),
    #[serde(default)]
    pub width_mils: Option<i32>,
    #[serde(default)]
    pub layer: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AddViaHighOp {
    #[serde(default)]
    pub opid: Option<String>,
    #[serde(default)]
    pub footprint_ref: Option<sch_ops_core::RefExpr>,
    pub at: (i32, i32),
    #[serde(default)]
    pub diameter_mils: Option<i32>,
    #[serde(default)]
    pub hole_size_mils: Option<i32>,
    #[serde(default)]
    pub from_layer: Option<String>,
    #[serde(default)]
    pub to_layer: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AddFootprintHighOp {
    #[serde(default)]
    pub opid: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, OpsSchema)]
#[ops(op = "footprint", domain = "sch")]
pub struct FootprintOp {
    #[ops(ty = "string", required)]
    pub model_name: String,
    #[serde(default)]
    #[ops(ty = "object_array")]
    pub map: Vec<FootprintMapEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize, OpsSchema)]
#[ops(op = "footprint_map_entry", domain = "sch")]
pub struct FootprintMapEntry {
    #[ops(ty = "string", required)]
    pub pin: String,
    #[ops(ty = "string", required)]
    pub pad: String,
}

#[derive(Debug, Clone, Copy, OpsEnum)]
pub enum PinElectricalName {
    Input,
    Output,
    Io,
    Passive,
    Power,
    OpenCollector,
    OpenEmitter,
    Hiz,
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
    AddTrack(AddTrackNode),
    AddVia(AddViaNode),
    AddFootprint(AddFootprintNode),
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
    pub at: Option<(i32, i32)>,
    pub rotation: Option<i32>,
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
#[derive(Debug, Clone)]
pub struct AddTrackNode(pub altium_format::pcb_ops_core::AddTrackOp);
#[derive(Debug, Clone)]
pub struct AddViaNode(pub altium_format::pcb_ops_core::AddViaOp);
#[derive(Debug, Clone)]
pub struct AddFootprintNode(pub altium_format::pcb_ops_core::AddFootprintOp);

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
