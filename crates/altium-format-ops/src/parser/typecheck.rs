use std::collections::{HashMap, HashSet};

use indexmap::IndexMap;

use super::ast::{
    AssertCondition, BinOp, BindingValue, Expr, Object, ObjectItem, Op, SelectorAttrOp,
    SelectorCombinator, SelectorExpr, SelectorFilter, SelectorSimple, SelectorValue, Spanned,
    Statement, TemplatePart, Unit,
};
use super::diagnostic::{ParseError, ParseErrorCode};
use super::parse_ops;
use crate::ops::model::{
    AddAliasOp, AddArcHighOp, AddBezierHighOp, AddComponentOp, AddEllipseHighOp,
    AddEllipticalArcHighOp, AddFootprintHighOp, AddImageHighOp, AddLabelHighOp, AddLineHighOp,
    AddParameterOp, AddPieHighOp, AddPinOp, AddPolygonHighOp, AddPolylineHighOp,
    AddRectangleHighOp, AddRoundRectangleHighOp, AddTextFrameHighOp, AddTrackHighOp,
    AddViaHighOp,
    EditComponentHighOp, EditRecordHighOp, FootprintMapEntry, FootprintOp, HighOp,
    PinElectricalName, QueryComponentsHighOp, QueryHighOp, QueryPinsHighOp, QueryRecordsHighOp,
    RemoveAliasOp, RemoveComponentOp, RemoveRecordsHighOp,
};
use crate::ops::schema::{FieldType, HasOpsEnum, HasOpsSchema, normalize_enum_ident};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpsDomain {
    SchDoc,
    SchLib,
    PcbDoc,
    PcbLib,
}

pub fn compile_ops_to_high(source: &str, domain: OpsDomain) -> Result<Vec<HighOp>, ParseError> {
    let ast = parse_ops(source)?;
    Compiler::new(source, domain).compile_file(&ast)
}

pub fn compile_ops_to_high_schdoc(source: &str) -> Result<Vec<HighOp>, ParseError> {
    compile_ops_to_high(source, OpsDomain::SchDoc)
}

pub fn compile_ops_to_high_schlib(source: &str) -> Result<Vec<HighOp>, ParseError> {
    compile_ops_to_high(source, OpsDomain::SchLib)
}

pub fn compile_ops_to_high_pcbdoc(source: &str) -> Result<Vec<HighOp>, ParseError> {
    compile_ops_to_high(source, OpsDomain::PcbDoc)
}

pub fn compile_ops_to_high_pcblib(source: &str) -> Result<Vec<HighOp>, ParseError> {
    compile_ops_to_high(source, OpsDomain::PcbLib)
}

#[derive(Debug, Clone)]
struct TypedValue {
    value: Value,
    span: super::ast::Span,
}

#[derive(Debug, Clone)]
enum Value {
    Null,
    Bool(bool),
    Integer(i32),
    Float(f64),
    Dim(f64), // normalized to mils
    Coord(f64, f64),
    String(String),
    Color(u8, u8, u8),
    Object(IndexMap<String, TypedValue>),
    Array(Vec<TypedValue>),
    RefExpr(altium_format::sch_ops_core::RefExpr),
}

struct Compiler<'a> {
    source: &'a str,
    domain: OpsDomain,
    scopes: Vec<HashMap<String, TypedValue>>,
    op_bindings: HashSet<String>,
    next_auto_opid: usize,
}

impl<'a> Compiler<'a> {
    fn new(source: &'a str, domain: OpsDomain) -> Self {
        Self {
            source,
            domain,
            scopes: vec![HashMap::new()],
            op_bindings: HashSet::new(),
            next_auto_opid: 1,
        }
    }

    fn compile_file(&mut self, ast: &super::ast::OpsFile) -> Result<Vec<HighOp>, ParseError> {
        self.ensure_supported_domain()?;

        let mut out = Vec::new();
        for stmt in &ast.statements {
            match &stmt.node {
                Statement::Binding(binding) => match &binding.value.node {
                    BindingValue::Expr(expr) => {
                        let value =
                            self.eval_expr(&Spanned::new(expr.clone(), binding.value.span))?;
                        self.insert_binding(binding.name.node.clone(), value, binding.name.span)?;
                    }
                    BindingValue::Op(op) => {
                        let opid = binding.name.node.clone();
                        let compiled = self.compile_op(op, Some(opid.clone()))?;
                        self.op_bindings.insert(opid);
                        out.push(compiled);
                    }
                },
                Statement::Assert(assert_stmt) => {
                    self.eval_assert(assert_stmt)?;
                }
                Statement::Op(op) => {
                    let opid = format!("op_{:04}", self.next_auto_opid);
                    self.next_auto_opid += 1;
                    let compiled = self.compile_op(op, Some(opid.clone()))?;
                    self.op_bindings.insert(opid);
                    out.push(compiled);
                }
            }
        }

        Ok(out)
    }

    fn ensure_supported_domain(&self) -> Result<(), ParseError> {
        match self.domain {
            OpsDomain::SchDoc | OpsDomain::SchLib | OpsDomain::PcbDoc | OpsDomain::PcbLib => {
                Ok(())
            }
        }
    }

    fn compile_op(&mut self, op: &Op, opid: Option<String>) -> Result<HighOp, ParseError> {
        let op_name = op.name.node.as_str();
        if let Some(sel) = &op.selector
            && matches!(self.domain, OpsDomain::SchDoc | OpsDomain::SchLib)
        {
            self.validate_selector_semantics(sel)?;
        }

        match self.domain {
            OpsDomain::SchDoc | OpsDomain::SchLib => match op_name {
                "edit" => self.compile_edit(op, opid),
                "remove" => self.compile_remove(op, opid),
                "add_component" => self.compile_add_component(op, opid),
                "add_pin" => self.compile_add_pin(op, opid),
                "add_parameter" => self.compile_add_parameter(op, opid),
                "add_alias" => self.compile_add_alias(op, opid),
                "remove_alias" => self.compile_remove_alias(op, opid),
                "remove_component" => self.compile_remove_component(op, opid),
                "edit_component" => self.compile_edit_component(op, opid),
                "query" => self.compile_query(op, opid),
                "query_components" => self.compile_query_components(op, opid),
                "query_pins" => self.compile_query_pins(op, opid),
                "query_records" => self.compile_query_records(op, opid),
                "edit_record" => self.compile_edit_record(op, opid),
                "remove_records" => self.compile_remove_records(op, opid),
                "add_line" => self.compile_add_line(op, opid),
                "add_rectangle" => self.compile_add_rectangle(op, opid),
                "add_arc" => self.compile_add_arc(op, opid),
                "add_elliptical_arc" => self.compile_add_elliptical_arc(op, opid),
                "add_ellipse" => self.compile_add_ellipse(op, opid),
                "add_polyline" => self.compile_add_polyline(op, opid),
                "add_polygon" => self.compile_add_polygon(op, opid),
                "add_bezier" => self.compile_add_bezier(op, opid),
                "add_pie" => self.compile_add_pie(op, opid),
                "add_round_rectangle" => self.compile_add_round_rectangle(op, opid),
                "add_label" => self.compile_add_label(op, opid),
                "add_text_frame" => self.compile_add_text_frame(op, opid),
                "add_image" => self.compile_add_image(op, opid),
                _ => Err(ParseError::new(
                    ParseErrorCode::E2001,
                    format!("unsupported op '{op_name}' in schematic domains"),
                    op.name.span,
                )
                .with_help("supported ops in schematic domains: edit, remove, add_component, add_pin, add_parameter, add_alias, remove_alias, remove_component, edit_component, query, query_components, query_pins, query_records, edit_record, remove_records, add_line, add_rectangle, add_arc, add_elliptical_arc, add_ellipse, add_polyline, add_polygon, add_bezier, add_pie, add_round_rectangle, add_label, add_text_frame, add_image")),
            },
            OpsDomain::PcbDoc => match op_name {
                "query" => self.compile_query(op, opid),
                "add_track" => self.compile_add_track(op, opid),
                "add_via" => self.compile_add_via(op, opid),
                _ => Err(ParseError::new(
                    ParseErrorCode::E2001,
                    format!("unsupported op '{op_name}' in pcbdoc domain"),
                    op.name.span,
                )
                .with_help("supported ops in pcbdoc domain: query, add_track, add_via")),
            },
            OpsDomain::PcbLib => match op_name {
                "query" => self.compile_query(op, opid),
                "add_track" => self.compile_add_track(op, opid),
                "add_via" => self.compile_add_via(op, opid),
                "add_footprint" => self.compile_add_footprint(op, opid),
                _ => Err(ParseError::new(
                    ParseErrorCode::E2001,
                    format!("unsupported op '{op_name}' in pcblib domain"),
                    op.name.span,
                )
                .with_help("supported ops in pcblib domain: query, add_track, add_via, add_footprint")),
            },
        }
    }

    fn compile_edit(&mut self, op: &Op, opid: Option<String>) -> Result<HighOp, ParseError> {
        if op.target.is_some() {
            return Err(ParseError::new(
                ParseErrorCode::E2008,
                "edit does not accept a target expression",
                op.name.span,
            )
            .with_help("use `edit SELECTOR { ... }`"));
        }

        let selector = op.selector.as_ref().ok_or_else(|| {
            ParseError::new(
                ParseErrorCode::E2008,
                "edit requires a selector",
                op.name.span,
            )
        })?;
        let component_ref = self.selector_to_single_component_ref(selector)?;
        let body = self.expect_body(op)?;
        let map = self.eval_object(body)?;

        self.reject_unknown_keys(
            &map,
            &[
                "description",
                "part_count",
                "display_mode_count",
                "component_kind",
                "show_hidden_pins",
            ],
            body.span,
            "edit",
        )?;

        Ok(HighOp::EditComponent(EditComponentHighOp {
            opid,
            component_ref,
            description: self.opt_string(&map, "description")?,
            part_count: self.opt_i32(&map, "part_count")?,
            display_mode_count: self.opt_i32(&map, "display_mode_count")?,
            component_kind: self.opt_i32(&map, "component_kind")?,
            show_hidden_pins: self.opt_bool(&map, "show_hidden_pins")?,
        }))
    }

    fn compile_remove(&mut self, op: &Op, opid: Option<String>) -> Result<HighOp, ParseError> {
        if op.target.is_some() {
            return Err(ParseError::new(
                ParseErrorCode::E2008,
                "remove does not accept a target expression",
                op.name.span,
            )
            .with_help("use `remove SELECTOR`"));
        }
        if op.body.is_some() {
            return Err(ParseError::new(
                ParseErrorCode::E2008,
                "remove does not accept an object body",
                op.name.span,
            )
            .with_help("use `remove SELECTOR` without `{ ... }`"));
        }

        let selector = op.selector.as_ref().ok_or_else(|| {
            ParseError::new(
                ParseErrorCode::E2008,
                "remove requires a selector",
                op.name.span,
            )
        })?;
        let component_ref = self.selector_to_single_component_ref(selector)?;
        Ok(HighOp::RemoveComponent(RemoveComponentOp {
            opid,
            component_ref,
        }))
    }

    fn compile_add_component(
        &mut self,
        op: &Op,
        opid: Option<String>,
    ) -> Result<HighOp, ParseError> {
        let body = self.expect_body(op)?;
        self.validate_body_against_schema::<AddComponentOp>(body)?;
        let map = self.eval_object(body)?;

        let component_ref = if let Some(target) = &op.target {
            Some(self.eval_as_refexpr(target, "target must be a reference expression")?)
        } else {
            self.opt_refexpr(&map, "component_ref")?
        };

        let pins = match map.get("pins") {
            Some(v) => self.extract_pins_array(v)?,
            None => Vec::new(),
        };

        let footprint = match map.get("footprint") {
            Some(v) => Some(self.extract_footprint(v)?),
            None => None,
        };

        Ok(HighOp::AddComponent(AddComponentOp {
            opid,
            id: self.opt_string(&map, "id")?,
            component_ref,
            lib_reference: self.req_string(&map, "lib_reference")?,
            designator: self.opt_string(&map, "designator")?,
            value: self.opt_string(&map, "value")?,
            pins,
            footprint,
        }))
    }

    fn compile_add_pin(&mut self, op: &Op, opid: Option<String>) -> Result<HighOp, ParseError> {
        let body = self.expect_body(op)?;
        self.validate_body_against_schema::<AddPinOp>(body)?;
        let map = self.eval_object(body)?;

        let target_ref = if let Some(target) = &op.target {
            Some(self.eval_as_refexpr(target, "target must be a reference expression")?)
        } else {
            self.opt_refexpr(&map, "component_ref")?
        };

        Ok(HighOp::AddPin(AddPinOp {
            opid,
            id: self.opt_string(&map, "id")?,
            component_ref: target_ref,
            designator: self.req_string(&map, "designator")?,
            name: self.opt_string(&map, "name")?,
            electrical: self.opt_string(&map, "electrical")?,
            length_mils: self.opt_i32_or_dim_mils(&map, "length_mils")?,
            at: self.opt_coord_mils(&map, "at")?,
            rotation: self.opt_rotation_90(&map, "rotation")?,
        }))
    }

    fn compile_add_parameter(
        &mut self,
        op: &Op,
        opid: Option<String>,
    ) -> Result<HighOp, ParseError> {
        let body = self.expect_body(op)?;
        self.validate_body_against_schema::<AddParameterOp>(body)?;
        let map = self.eval_object(body)?;

        let target_ref = if let Some(target) = &op.target {
            Some(self.eval_as_refexpr(target, "target must be a reference expression")?)
        } else {
            self.opt_refexpr(&map, "component_ref")?
        };

        Ok(HighOp::AddParameter(AddParameterOp {
            opid,
            component_ref: target_ref,
            name: self.req_string(&map, "name")?,
            text: self.req_string(&map, "text")?,
            is_hidden: self.opt_bool(&map, "is_hidden")?,
        }))
    }

    fn compile_add_alias(&mut self, op: &Op, opid: Option<String>) -> Result<HighOp, ParseError> {
        if matches!(self.domain, OpsDomain::SchDoc) {
            return Err(ParseError::new(
                ParseErrorCode::E2008,
                "add_alias is a SchLib-only operation",
                op.name.span,
            )
            .with_help("use add_alias only when compiling for SchLib"));
        }
        let body = self.expect_body(op)?;
        self.validate_body_against_schema::<AddAliasOp>(body)?;
        let map = self.eval_object(body)?;

        let component_ref = if let Some(target) = &op.target {
            self.eval_as_refexpr(target, "target must be a reference expression")?
        } else {
            self.req_refexpr(&map, "component_ref")?
        };

        Ok(HighOp::AddAlias(AddAliasOp {
            opid,
            component_ref,
            alias_name: self.req_string(&map, "alias_name")?,
        }))
    }

    fn compile_remove_alias(
        &mut self,
        op: &Op,
        opid: Option<String>,
    ) -> Result<HighOp, ParseError> {
        if matches!(self.domain, OpsDomain::SchDoc) {
            return Err(ParseError::new(
                ParseErrorCode::E2008,
                "remove_alias is a SchLib-only operation",
                op.name.span,
            )
            .with_help("use remove_alias only when compiling for SchLib"));
        }
        let body = self.expect_body(op)?;
        self.validate_body_against_schema::<RemoveAliasOp>(body)?;
        let map = self.eval_object(body)?;

        let component_ref = if let Some(target) = &op.target {
            self.eval_as_refexpr(target, "target must be a reference expression")?
        } else {
            self.req_refexpr(&map, "component_ref")?
        };

        Ok(HighOp::RemoveAlias(RemoveAliasOp {
            opid,
            component_ref,
            alias_name: self.req_string(&map, "alias_name")?,
        }))
    }

    fn compile_remove_component(
        &mut self,
        op: &Op,
        opid: Option<String>,
    ) -> Result<HighOp, ParseError> {
        let body = self.expect_body(op)?;
        self.validate_body_against_schema::<RemoveComponentOp>(body)?;
        let map = self.eval_object(body)?;

        let component_ref = if let Some(target) = &op.target {
            self.eval_as_refexpr(target, "target must be a reference expression")?
        } else {
            self.req_refexpr(&map, "component_ref")?
        };

        Ok(HighOp::RemoveComponent(RemoveComponentOp {
            opid,
            component_ref,
        }))
    }

    fn compile_edit_component(
        &mut self,
        op: &Op,
        opid: Option<String>,
    ) -> Result<HighOp, ParseError> {
        let body = self.expect_body(op)?;
        self.validate_body_against_schema::<EditComponentHighOp>(body)?;
        let map = self.eval_object(body)?;

        let component_ref = if let Some(target) = &op.target {
            self.eval_as_refexpr(target, "target must be a reference expression")?
        } else {
            self.req_refexpr(&map, "component_ref")?
        };

        Ok(HighOp::EditComponent(EditComponentHighOp {
            opid,
            component_ref,
            description: self.opt_string(&map, "description")?,
            part_count: self.opt_i32(&map, "part_count")?,
            display_mode_count: self.opt_i32(&map, "display_mode_count")?,
            component_kind: self.opt_i32(&map, "component_kind")?,
            show_hidden_pins: self.opt_bool(&map, "show_hidden_pins")?,
        }))
    }

    fn compile_query(&mut self, op: &Op, opid: Option<String>) -> Result<HighOp, ParseError> {
        let selector = op
            .selector
            .as_ref()
            .ok_or_else(|| {
                ParseError::new(
                    ParseErrorCode::E2008,
                    "query requires a selector",
                    op.name.span,
                )
            })?
            .node
            .raw
            .clone();
        Ok(HighOp::Query(QueryHighOp { opid, selector }))
    }

    fn compile_query_components(
        &mut self,
        op: &Op,
        opid: Option<String>,
    ) -> Result<HighOp, ParseError> {
        let body = self.expect_body(op)?;
        self.validate_body_against_schema::<QueryComponentsHighOp>(body)?;
        let map = self.eval_object(body)?;
        Ok(HighOp::QueryComponents(QueryComponentsHighOp {
            opid,
            pattern: self.opt_string(&map, "pattern")?,
        }))
    }

    fn compile_query_pins(&mut self, op: &Op, opid: Option<String>) -> Result<HighOp, ParseError> {
        let body = self.expect_body(op)?;
        self.validate_body_against_schema::<QueryPinsHighOp>(body)?;
        let map = self.eval_object(body)?;

        let component_ref = if let Some(target) = &op.target {
            self.eval_as_refexpr(target, "target must be a reference expression")?
        } else {
            self.req_refexpr(&map, "component_ref")?
        };

        Ok(HighOp::QueryPins(QueryPinsHighOp {
            opid,
            component_ref,
        }))
    }

    fn compile_query_records(
        &mut self,
        op: &Op,
        opid: Option<String>,
    ) -> Result<HighOp, ParseError> {
        let body = self.expect_body(op)?;
        self.validate_body_against_schema::<QueryRecordsHighOp>(body)?;
        let map = self.eval_object(body)?;

        let component_ref = if let Some(target) = &op.target {
            self.eval_as_refexpr(target, "target must be a reference expression")?
        } else {
            self.req_refexpr(&map, "component_ref")?
        };

        Ok(HighOp::QueryRecords(QueryRecordsHighOp {
            opid,
            component_ref,
            record_type: self.opt_i32(&map, "record_type")?,
        }))
    }

    fn compile_edit_record(&mut self, op: &Op, opid: Option<String>) -> Result<HighOp, ParseError> {
        let body = self.expect_body(op)?;
        let map = self.eval_object(body)?;

        let component_ref = if let Some(target) = &op.target {
            Some(self.eval_as_refexpr(target, "target must be a reference expression")?)
        } else {
            self.opt_refexpr(&map, "component_ref")?
        };

        let selector = if let Some(sel) = map.get("selector") {
            self.value_to_record_selector(sel)?
        } else {
            return Err(ParseError::new(
                ParseErrorCode::E2002,
                "missing required field 'selector'",
                body.span,
            ));
        };

        let patch = if let Some(Value::Object(m)) = map.get("patch").map(|v| &v.value) {
            self.value_to_record_patch(m)?
        } else {
            altium_format::sch_ops_core::RecordPatch::default()
        };

        Ok(HighOp::EditRecord(EditRecordHighOp {
            opid,
            component_ref,
            selector,
            patch,
        }))
    }

    fn compile_remove_records(
        &mut self,
        op: &Op,
        opid: Option<String>,
    ) -> Result<HighOp, ParseError> {
        let body = self.expect_body(op)?;
        let map = self.eval_object(body)?;

        let component_ref = if let Some(target) = &op.target {
            Some(self.eval_as_refexpr(target, "target must be a reference expression")?)
        } else {
            self.opt_refexpr(&map, "component_ref")?
        };

        let selector = if let Some(sel) = map.get("selector") {
            self.value_to_record_selector(sel)?
        } else {
            return Err(ParseError::new(
                ParseErrorCode::E2002,
                "missing required field 'selector'",
                body.span,
            ));
        };

        Ok(HighOp::RemoveRecords(RemoveRecordsHighOp {
            opid,
            component_ref,
            selector,
        }))
    }

    fn compile_add_line(&mut self, op: &Op, opid: Option<String>) -> Result<HighOp, ParseError> {
        let body = self.expect_body(op)?;
        let map = self.eval_object(body)?;
        self.reject_unknown_keys(
            &map,
            &[
                "component_ref",
                "from",
                "to",
                "x1_mils",
                "y1_mils",
                "x2_mils",
                "y2_mils",
                "color",
                "line_width",
                "line_style",
                "owner_part_id",
                "owner_part_display_mode",
            ],
            body.span,
            "add_line",
        )?;

        let component_ref = if let Some(target) = &op.target {
            Some(self.eval_as_refexpr(target, "target must be a reference expression")?)
        } else {
            self.opt_refexpr(&map, "component_ref")?
        };
        let (from, to) = self.req_from_to_mils(&map, body.span)?;

        Ok(HighOp::AddLine(AddLineHighOp {
            opid,
            component_ref,
            from,
            to,
            color: self.opt_i32(&map, "color")?,
            line_width: self.opt_i32(&map, "line_width")?,
            line_style: self.opt_i32(&map, "line_style")?,
            owner_part_id: self.opt_i32(&map, "owner_part_id")?,
            owner_part_display_mode: self.opt_i32(&map, "owner_part_display_mode")?,
        }))
    }

    fn compile_add_rectangle(
        &mut self,
        op: &Op,
        opid: Option<String>,
    ) -> Result<HighOp, ParseError> {
        let body = self.expect_body(op)?;
        let map = self.eval_object(body)?;
        self.reject_unknown_keys(
            &map,
            &[
                "component_ref",
                "from",
                "to",
                "x1_mils",
                "y1_mils",
                "x2_mils",
                "y2_mils",
                "color",
                "area_color",
                "is_solid",
                "transparent",
                "line_width",
                "owner_part_id",
                "owner_part_display_mode",
            ],
            body.span,
            "add_rectangle",
        )?;

        let component_ref = if let Some(target) = &op.target {
            Some(self.eval_as_refexpr(target, "target must be a reference expression")?)
        } else {
            self.opt_refexpr(&map, "component_ref")?
        };
        let (from, to) = self.req_from_to_mils(&map, body.span)?;

        Ok(HighOp::AddRectangle(AddRectangleHighOp {
            opid,
            component_ref,
            from,
            to,
            color: self.opt_i32(&map, "color")?,
            area_color: self.opt_i32(&map, "area_color")?,
            is_solid: self.opt_bool(&map, "is_solid")?,
            transparent: self.opt_bool(&map, "transparent")?,
            line_width: self.opt_i32(&map, "line_width")?,
            owner_part_id: self.opt_i32(&map, "owner_part_id")?,
            owner_part_display_mode: self.opt_i32(&map, "owner_part_display_mode")?,
        }))
    }

    fn compile_add_label(&mut self, op: &Op, opid: Option<String>) -> Result<HighOp, ParseError> {
        let body = self.expect_body(op)?;
        let map = self.eval_object(body)?;
        self.reject_unknown_keys(
            &map,
            &[
                "component_ref",
                "x_mils",
                "y_mils",
                "text",
                "color",
                "font_id",
                "orientation",
                "justification",
                "is_mirrored",
                "owner_part_id",
                "owner_part_display_mode",
            ],
            body.span,
            "add_label",
        )?;

        let component_ref = if let Some(target) = &op.target {
            Some(self.eval_as_refexpr(target, "target must be a reference expression")?)
        } else {
            self.opt_refexpr(&map, "component_ref")?
        };

        Ok(HighOp::AddLabel(AddLabelHighOp {
            opid,
            component_ref,
            x_mils: self.req_i32_or_dim_mils(&map, "x_mils")?,
            y_mils: self.req_i32_or_dim_mils(&map, "y_mils")?,
            text: self.req_string(&map, "text")?,
            color: self.opt_i32(&map, "color")?,
            font_id: self.opt_i32(&map, "font_id")?,
            orientation: self.opt_i32(&map, "orientation")?,
            justification: self.opt_i32(&map, "justification")?,
            is_mirrored: self.opt_bool(&map, "is_mirrored")?,
            owner_part_id: self.opt_i32(&map, "owner_part_id")?,
            owner_part_display_mode: self.opt_i32(&map, "owner_part_display_mode")?,
        }))
    }

    fn compile_add_arc(&mut self, op: &Op, opid: Option<String>) -> Result<HighOp, ParseError> {
        let body = self.expect_body(op)?;
        let map = self.eval_object(body)?;
        self.reject_unknown_keys(
            &map,
            &[
                "component_ref",
                "cx_mils",
                "cy_mils",
                "radius_mils",
                "start_angle",
                "end_angle",
                "color",
                "line_width",
                "owner_part_id",
                "owner_part_display_mode",
            ],
            body.span,
            "add_arc",
        )?;

        let component_ref = if let Some(target) = &op.target {
            Some(self.eval_as_refexpr(target, "target must be a reference expression")?)
        } else {
            self.opt_refexpr(&map, "component_ref")?
        };
        Ok(HighOp::AddArc(AddArcHighOp {
            opid,
            component_ref,
            cx_mils: self.req_i32_or_dim_mils(&map, "cx_mils")?,
            cy_mils: self.req_i32_or_dim_mils(&map, "cy_mils")?,
            radius_mils: self.req_i32_or_dim_mils(&map, "radius_mils")?,
            start_angle: self.opt_f64(&map, "start_angle")?,
            end_angle: self.opt_f64(&map, "end_angle")?,
            color: self.opt_i32(&map, "color")?,
            line_width: self.opt_i32(&map, "line_width")?,
            owner_part_id: self.opt_i32(&map, "owner_part_id")?,
            owner_part_display_mode: self.opt_i32(&map, "owner_part_display_mode")?,
        }))
    }

    fn compile_add_elliptical_arc(
        &mut self,
        op: &Op,
        opid: Option<String>,
    ) -> Result<HighOp, ParseError> {
        let body = self.expect_body(op)?;
        let map = self.eval_object(body)?;
        self.reject_unknown_keys(
            &map,
            &[
                "component_ref",
                "cx_mils",
                "cy_mils",
                "radius_mils",
                "secondary_radius_mils",
                "start_angle",
                "end_angle",
                "color",
                "line_width",
                "owner_part_id",
                "owner_part_display_mode",
            ],
            body.span,
            "add_elliptical_arc",
        )?;

        let component_ref = if let Some(target) = &op.target {
            Some(self.eval_as_refexpr(target, "target must be a reference expression")?)
        } else {
            self.opt_refexpr(&map, "component_ref")?
        };
        Ok(HighOp::AddEllipticalArc(AddEllipticalArcHighOp {
            opid,
            component_ref,
            cx_mils: self.req_i32_or_dim_mils(&map, "cx_mils")?,
            cy_mils: self.req_i32_or_dim_mils(&map, "cy_mils")?,
            radius_mils: self.req_i32_or_dim_mils(&map, "radius_mils")?,
            secondary_radius_mils: self.req_i32_or_dim_mils(&map, "secondary_radius_mils")?,
            start_angle: self.opt_f64(&map, "start_angle")?,
            end_angle: self.opt_f64(&map, "end_angle")?,
            color: self.opt_i32(&map, "color")?,
            line_width: self.opt_i32(&map, "line_width")?,
            owner_part_id: self.opt_i32(&map, "owner_part_id")?,
            owner_part_display_mode: self.opt_i32(&map, "owner_part_display_mode")?,
        }))
    }

    fn compile_add_ellipse(&mut self, op: &Op, opid: Option<String>) -> Result<HighOp, ParseError> {
        let body = self.expect_body(op)?;
        let map = self.eval_object(body)?;
        self.reject_unknown_keys(
            &map,
            &[
                "component_ref",
                "cx_mils",
                "cy_mils",
                "radius_mils",
                "secondary_radius_mils",
                "color",
                "area_color",
                "is_solid",
                "line_width",
                "owner_part_id",
                "owner_part_display_mode",
            ],
            body.span,
            "add_ellipse",
        )?;
        let component_ref = if let Some(target) = &op.target {
            Some(self.eval_as_refexpr(target, "target must be a reference expression")?)
        } else {
            self.opt_refexpr(&map, "component_ref")?
        };
        Ok(HighOp::AddEllipse(AddEllipseHighOp {
            opid,
            component_ref,
            cx_mils: self.req_i32_or_dim_mils(&map, "cx_mils")?,
            cy_mils: self.req_i32_or_dim_mils(&map, "cy_mils")?,
            radius_mils: self.req_i32_or_dim_mils(&map, "radius_mils")?,
            secondary_radius_mils: self.req_i32_or_dim_mils(&map, "secondary_radius_mils")?,
            color: self.opt_i32(&map, "color")?,
            area_color: self.opt_i32(&map, "area_color")?,
            is_solid: self.opt_bool(&map, "is_solid")?,
            line_width: self.opt_i32(&map, "line_width")?,
            owner_part_id: self.opt_i32(&map, "owner_part_id")?,
            owner_part_display_mode: self.opt_i32(&map, "owner_part_display_mode")?,
        }))
    }

    fn compile_add_polyline(
        &mut self,
        op: &Op,
        opid: Option<String>,
    ) -> Result<HighOp, ParseError> {
        let body = self.expect_body(op)?;
        let map = self.eval_object(body)?;
        self.reject_unknown_keys(
            &map,
            &[
                "component_ref",
                "points_mils",
                "color",
                "line_width",
                "line_style",
                "owner_part_id",
                "owner_part_display_mode",
            ],
            body.span,
            "add_polyline",
        )?;
        let component_ref = if let Some(target) = &op.target {
            Some(self.eval_as_refexpr(target, "target must be a reference expression")?)
        } else {
            self.opt_refexpr(&map, "component_ref")?
        };
        Ok(HighOp::AddPolyline(AddPolylineHighOp {
            opid,
            component_ref,
            points_mils: self.req_points_mils(&map, "points_mils")?,
            color: self.opt_i32(&map, "color")?,
            line_width: self.opt_i32(&map, "line_width")?,
            line_style: self.opt_i32(&map, "line_style")?,
            owner_part_id: self.opt_i32(&map, "owner_part_id")?,
            owner_part_display_mode: self.opt_i32(&map, "owner_part_display_mode")?,
        }))
    }

    fn compile_add_polygon(&mut self, op: &Op, opid: Option<String>) -> Result<HighOp, ParseError> {
        let body = self.expect_body(op)?;
        let map = self.eval_object(body)?;
        self.reject_unknown_keys(
            &map,
            &[
                "component_ref",
                "points_mils",
                "color",
                "area_color",
                "is_solid",
                "line_width",
                "owner_part_id",
                "owner_part_display_mode",
            ],
            body.span,
            "add_polygon",
        )?;
        let component_ref = if let Some(target) = &op.target {
            Some(self.eval_as_refexpr(target, "target must be a reference expression")?)
        } else {
            self.opt_refexpr(&map, "component_ref")?
        };
        Ok(HighOp::AddPolygon(AddPolygonHighOp {
            opid,
            component_ref,
            points_mils: self.req_points_mils(&map, "points_mils")?,
            color: self.opt_i32(&map, "color")?,
            area_color: self.opt_i32(&map, "area_color")?,
            is_solid: self.opt_bool(&map, "is_solid")?,
            line_width: self.opt_i32(&map, "line_width")?,
            owner_part_id: self.opt_i32(&map, "owner_part_id")?,
            owner_part_display_mode: self.opt_i32(&map, "owner_part_display_mode")?,
        }))
    }

    fn compile_add_bezier(&mut self, op: &Op, opid: Option<String>) -> Result<HighOp, ParseError> {
        let body = self.expect_body(op)?;
        let map = self.eval_object(body)?;
        self.reject_unknown_keys(
            &map,
            &[
                "component_ref",
                "points_mils",
                "color",
                "line_width",
                "owner_part_id",
                "owner_part_display_mode",
            ],
            body.span,
            "add_bezier",
        )?;
        let component_ref = if let Some(target) = &op.target {
            Some(self.eval_as_refexpr(target, "target must be a reference expression")?)
        } else {
            self.opt_refexpr(&map, "component_ref")?
        };
        Ok(HighOp::AddBezier(AddBezierHighOp {
            opid,
            component_ref,
            points_mils: self.req_points_mils(&map, "points_mils")?,
            color: self.opt_i32(&map, "color")?,
            line_width: self.opt_i32(&map, "line_width")?,
            owner_part_id: self.opt_i32(&map, "owner_part_id")?,
            owner_part_display_mode: self.opt_i32(&map, "owner_part_display_mode")?,
        }))
    }

    fn compile_add_pie(&mut self, op: &Op, opid: Option<String>) -> Result<HighOp, ParseError> {
        let body = self.expect_body(op)?;
        let map = self.eval_object(body)?;
        self.reject_unknown_keys(
            &map,
            &[
                "component_ref",
                "cx_mils",
                "cy_mils",
                "radius_mils",
                "start_angle",
                "end_angle",
                "color",
                "area_color",
                "is_solid",
                "line_width",
                "owner_part_id",
                "owner_part_display_mode",
            ],
            body.span,
            "add_pie",
        )?;
        let component_ref = if let Some(target) = &op.target {
            Some(self.eval_as_refexpr(target, "target must be a reference expression")?)
        } else {
            self.opt_refexpr(&map, "component_ref")?
        };
        Ok(HighOp::AddPie(AddPieHighOp {
            opid,
            component_ref,
            cx_mils: self.req_i32_or_dim_mils(&map, "cx_mils")?,
            cy_mils: self.req_i32_or_dim_mils(&map, "cy_mils")?,
            radius_mils: self.req_i32_or_dim_mils(&map, "radius_mils")?,
            start_angle: self.opt_f64(&map, "start_angle")?,
            end_angle: self.opt_f64(&map, "end_angle")?,
            color: self.opt_i32(&map, "color")?,
            area_color: self.opt_i32(&map, "area_color")?,
            is_solid: self.opt_bool(&map, "is_solid")?,
            line_width: self.opt_i32(&map, "line_width")?,
            owner_part_id: self.opt_i32(&map, "owner_part_id")?,
            owner_part_display_mode: self.opt_i32(&map, "owner_part_display_mode")?,
        }))
    }

    fn compile_add_round_rectangle(
        &mut self,
        op: &Op,
        opid: Option<String>,
    ) -> Result<HighOp, ParseError> {
        let body = self.expect_body(op)?;
        let map = self.eval_object(body)?;
        self.reject_unknown_keys(
            &map,
            &[
                "component_ref",
                "from",
                "to",
                "x1_mils",
                "y1_mils",
                "x2_mils",
                "y2_mils",
                "corner_x_radius_mils",
                "corner_y_radius_mils",
                "color",
                "area_color",
                "is_solid",
                "line_width",
                "owner_part_id",
                "owner_part_display_mode",
            ],
            body.span,
            "add_round_rectangle",
        )?;
        let component_ref = if let Some(target) = &op.target {
            Some(self.eval_as_refexpr(target, "target must be a reference expression")?)
        } else {
            self.opt_refexpr(&map, "component_ref")?
        };
        let (from, to) = self.req_from_to_mils(&map, body.span)?;
        Ok(HighOp::AddRoundRectangle(AddRoundRectangleHighOp {
            opid,
            component_ref,
            from,
            to,
            corner_x_radius_mils: self.req_i32_or_dim_mils(&map, "corner_x_radius_mils")?,
            corner_y_radius_mils: self.req_i32_or_dim_mils(&map, "corner_y_radius_mils")?,
            color: self.opt_i32(&map, "color")?,
            area_color: self.opt_i32(&map, "area_color")?,
            is_solid: self.opt_bool(&map, "is_solid")?,
            line_width: self.opt_i32(&map, "line_width")?,
            owner_part_id: self.opt_i32(&map, "owner_part_id")?,
            owner_part_display_mode: self.opt_i32(&map, "owner_part_display_mode")?,
        }))
    }

    fn compile_add_text_frame(
        &mut self,
        op: &Op,
        opid: Option<String>,
    ) -> Result<HighOp, ParseError> {
        let body = self.expect_body(op)?;
        let map = self.eval_object(body)?;
        self.reject_unknown_keys(
            &map,
            &[
                "component_ref",
                "from",
                "to",
                "x1_mils",
                "y1_mils",
                "x2_mils",
                "y2_mils",
                "text",
                "color",
                "area_color",
                "font_id",
                "alignment",
                "word_wrap",
                "show_border",
                "is_solid",
                "clip_to_rect",
                "owner_part_id",
                "owner_part_display_mode",
            ],
            body.span,
            "add_text_frame",
        )?;
        let component_ref = if let Some(target) = &op.target {
            Some(self.eval_as_refexpr(target, "target must be a reference expression")?)
        } else {
            self.opt_refexpr(&map, "component_ref")?
        };
        let (from, to) = self.req_from_to_mils(&map, body.span)?;
        Ok(HighOp::AddTextFrame(AddTextFrameHighOp {
            opid,
            component_ref,
            from,
            to,
            text: self.req_string(&map, "text")?,
            color: self.opt_i32(&map, "color")?,
            area_color: self.opt_i32(&map, "area_color")?,
            font_id: self.opt_i32(&map, "font_id")?,
            alignment: self.opt_i32(&map, "alignment")?,
            word_wrap: self.opt_bool(&map, "word_wrap")?,
            show_border: self.opt_bool(&map, "show_border")?,
            is_solid: self.opt_bool(&map, "is_solid")?,
            clip_to_rect: self.opt_bool(&map, "clip_to_rect")?,
            owner_part_id: self.opt_i32(&map, "owner_part_id")?,
            owner_part_display_mode: self.opt_i32(&map, "owner_part_display_mode")?,
        }))
    }

    fn compile_add_image(&mut self, op: &Op, opid: Option<String>) -> Result<HighOp, ParseError> {
        let body = self.expect_body(op)?;
        let map = self.eval_object(body)?;
        self.reject_unknown_keys(
            &map,
            &[
                "component_ref",
                "from",
                "to",
                "x1_mils",
                "y1_mils",
                "x2_mils",
                "y2_mils",
                "file_name",
                "image_data",
                "keep_aspect",
                "owner_part_id",
                "owner_part_display_mode",
            ],
            body.span,
            "add_image",
        )?;
        let component_ref = if let Some(target) = &op.target {
            Some(self.eval_as_refexpr(target, "target must be a reference expression")?)
        } else {
            self.opt_refexpr(&map, "component_ref")?
        };
        let (from, to) = self.req_from_to_mils(&map, body.span)?;
        Ok(HighOp::AddImage(AddImageHighOp {
            opid,
            component_ref,
            from,
            to,
            file_name: self.req_string(&map, "file_name")?,
            image_data: self.opt_u8_array(&map, "image_data")?,
            keep_aspect: self.opt_bool(&map, "keep_aspect")?,
            owner_part_id: self.opt_i32(&map, "owner_part_id")?,
            owner_part_display_mode: self.opt_i32(&map, "owner_part_display_mode")?,
        }))
    }

    fn compile_add_track(&mut self, op: &Op, opid: Option<String>) -> Result<HighOp, ParseError> {
        let body = self.expect_body(op)?;
        let map = self.eval_object(body)?;
        self.reject_unknown_keys(
            &map,
            &[
                "footprint_ref",
                "start",
                "end",
                "x1_mils",
                "y1_mils",
                "x2_mils",
                "y2_mils",
                "width_mils",
                "layer",
            ],
            body.span,
            "add_track",
        )?;

        let footprint_ref = if let Some(target) = &op.target {
            Some(self.eval_as_refexpr(target, "target must be a reference expression")?)
        } else {
            self.opt_refexpr(&map, "footprint_ref")?
        };
        let start = self.opt_coord_mils(&map, "start")?;
        let end = self.opt_coord_mils(&map, "end")?;
        let (start, end) = if let (Some(start), Some(end)) = (start, end) {
            (start, end)
        } else {
            self.req_from_to_mils(&map, body.span)?
        };

        Ok(HighOp::AddTrack(AddTrackHighOp {
            opid,
            footprint_ref,
            start,
            end,
            width_mils: self.opt_i32_or_dim_mils(&map, "width_mils")?,
            layer: self.opt_string(&map, "layer")?,
        }))
    }

    fn compile_add_footprint(
        &mut self,
        op: &Op,
        opid: Option<String>,
    ) -> Result<HighOp, ParseError> {
        let body = self.expect_body(op)?;
        let map = self.eval_object(body)?;
        self.reject_unknown_keys(
            &map,
            &["id", "name", "pattern", "description"],
            body.span,
            "add_footprint",
        )?;

        Ok(HighOp::AddFootprint(AddFootprintHighOp {
            opid,
            id: self.opt_string(&map, "id")?,
            name: self.req_string(&map, "name")?,
            pattern: self.opt_string(&map, "pattern")?,
            description: self.opt_string(&map, "description")?,
        }))
    }

    fn compile_add_via(&mut self, op: &Op, opid: Option<String>) -> Result<HighOp, ParseError> {
        let body = self.expect_body(op)?;
        let map = self.eval_object(body)?;
        self.reject_unknown_keys(
            &map,
            &[
                "footprint_ref",
                "at",
                "x_mils",
                "y_mils",
                "diameter_mils",
                "hole_size_mils",
                "from_layer",
                "to_layer",
            ],
            body.span,
            "add_via",
        )?;

        let footprint_ref = if let Some(target) = &op.target {
            Some(self.eval_as_refexpr(target, "target must be a reference expression")?)
        } else {
            self.opt_refexpr(&map, "footprint_ref")?
        };

        let at = if let Some(v) = self.opt_coord_mils(&map, "at")? {
            v
        } else {
            (
                self.req_i32_or_dim_mils(&map, "x_mils")?,
                self.req_i32_or_dim_mils(&map, "y_mils")?,
            )
        };

        Ok(HighOp::AddVia(AddViaHighOp {
            opid,
            footprint_ref,
            at,
            diameter_mils: self.opt_i32_or_dim_mils(&map, "diameter_mils")?,
            hole_size_mils: self.opt_i32_or_dim_mils(&map, "hole_size_mils")?,
            from_layer: self.opt_string(&map, "from_layer")?,
            to_layer: self.opt_string(&map, "to_layer")?,
        }))
    }

    fn eval_assert(&mut self, stmt: &super::ast::AssertStmt) -> Result<(), ParseError> {
        let ok = match &stmt.condition.node {
            AssertCondition::Existence(expr) => !matches!(self.eval_expr(expr)?.value, Value::Null),
            AssertCondition::Comparison { left, op, right } => {
                let l = self.eval_expr(left)?;
                let r = self.eval_expr(right)?;
                self.compare_values(&l.value, &r.value, op.node, op.span)?
            }
        };

        if ok {
            return Ok(());
        }

        let msg = if let Some(m) = &stmt.message {
            match self.eval_expr(m)?.value {
                Value::String(s) => s,
                _ => "assertion failed".to_owned(),
            }
        } else {
            "assertion failed".to_owned()
        };

        Err(ParseError::new(
            ParseErrorCode::E2008,
            msg,
            stmt.condition.span,
        ))
    }

    fn compare_values(
        &self,
        left: &Value,
        right: &Value,
        op: super::ast::CompareOp,
        span: super::ast::Span,
    ) -> Result<bool, ParseError> {
        use super::ast::CompareOp;
        let out = match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => cmp_ord(*a as f64, *b as f64, op),
            (Value::Float(a), Value::Float(b)) => cmp_ord(*a, *b, op),
            (Value::Integer(a), Value::Float(b)) => cmp_ord(*a as f64, *b, op),
            (Value::Float(a), Value::Integer(b)) => cmp_ord(*a, *b as f64, op),
            (Value::Dim(a), Value::Dim(b)) => cmp_ord(*a, *b, op),
            (Value::String(a), Value::String(b)) => match op {
                CompareOp::Eq => a == b,
                CompareOp::Ne => a != b,
                _ => {
                    return Err(ParseError::new(
                        ParseErrorCode::E2003,
                        "invalid string comparison operator",
                        span,
                    ));
                }
            },
            (Value::Bool(a), Value::Bool(b)) => match op {
                CompareOp::Eq => a == b,
                CompareOp::Ne => a != b,
                _ => {
                    return Err(ParseError::new(
                        ParseErrorCode::E2003,
                        "invalid bool comparison operator",
                        span,
                    ));
                }
            },
            _ => {
                return Err(ParseError::new(
                    ParseErrorCode::E2003,
                    "assert comparison type mismatch",
                    span,
                ));
            }
        };
        Ok(out)
    }

    fn eval_object(
        &mut self,
        object: &Spanned<Object>,
    ) -> Result<IndexMap<String, TypedValue>, ParseError> {
        self.push_scope();
        let mut out: IndexMap<String, TypedValue> = IndexMap::new();

        for item in &object.node.items {
            match &item.node {
                ObjectItem::Binding(binding) => match &binding.value.node {
                    BindingValue::Expr(expr) => {
                        let v = self.eval_expr(&Spanned::new(expr.clone(), binding.value.span))?;
                        self.insert_binding(binding.name.node.clone(), v, binding.name.span)?;
                    }
                    BindingValue::Op(_) => {
                        self.pop_scope();
                        return Err(ParseError::new(
                            ParseErrorCode::E2008,
                            "op bindings are not allowed inside object bodies",
                            item.span,
                        ));
                    }
                },
                ObjectItem::Spread(expr) => {
                    let v = self.eval_expr(expr)?;
                    let Value::Object(m) = v.value else {
                        self.pop_scope();
                        return Err(ParseError::new(
                            ParseErrorCode::E2007,
                            "spread source must evaluate to an object",
                            expr.span,
                        ));
                    };
                    for (k, v) in m {
                        out.insert(k, v);
                    }
                }
                ObjectItem::Field(field) => {
                    let mut key = String::new();
                    for (idx, seg) in field.key.node.segments.iter().enumerate() {
                        if idx > 0 {
                            key.push('.');
                        }
                        key.push_str(&seg.node);
                    }
                    let value = self.eval_field_value(&key, &field.value)?;
                    out.insert(key, value);
                }
            }
        }

        self.pop_scope();
        Ok(out)
    }

    fn eval_expr(&mut self, expr: &Spanned<Expr>) -> Result<TypedValue, ParseError> {
        let value =
            match &expr.node {
                Expr::Null => Value::Null,
                Expr::Bool(v) => Value::Bool(*v),
                Expr::Integer(v) => Value::Integer(*v),
                Expr::Float(v) => Value::Float(*v),
                Expr::String(v) => Value::String(v.clone()),
                Expr::Color(r, g, b) => Value::Color(*r, *g, *b),
                Expr::Dim(v, unit) => Value::Dim(to_mils(*v, *unit)),
                Expr::TemplateString(template) => {
                    let mut out = String::new();
                    for part in &template.parts {
                        match &part.node {
                            TemplatePart::Literal(v) => out.push_str(v),
                            TemplatePart::Interpolation(v) => {
                                let iv = self.eval_expr(v)?;
                                out.push_str(&self.value_to_string(&iv.value)?);
                            }
                        }
                    }
                    Value::String(out)
                }
                Expr::Ident(name) => {
                    self.lookup_binding(name)
                        .ok_or_else(|| {
                            ParseError::new(
                        ParseErrorCode::E2005,
                        format!("unresolved identifier '{name}'"),
                        expr.span,
                    )
                    .with_help("define a binding before use, or use $name for op-result references")
                        })?
                        .value
                }
                Expr::DollarIdent(name) => {
                    if name == "last" {
                        Value::RefExpr(altium_format::sch_ops_core::RefExpr::last())
                    } else if name == "self" {
                        Value::RefExpr(altium_format::sch_ops_core::RefExpr::self_())
                    } else if name == "sheet" {
                        Value::RefExpr(altium_format::sch_ops_core::RefExpr::sheet())
                    } else if self.op_bindings.contains(name) {
                        Value::RefExpr(altium_format::sch_ops_core::RefExpr::op(name.clone()))
                    } else {
                        return Err(ParseError::new(
                            ParseErrorCode::E2005,
                            format!("unknown op-result reference '${name}'"),
                            expr.span,
                        )
                        .with_help("bind an op with `name = op ...` before referencing `$name`"));
                    }
                }
                Expr::Path(base, seg) => {
                    let base_val = self.eval_expr(base)?;
                    match base_val.value {
                        Value::Object(map) => map
                            .get(&seg.node)
                            .ok_or_else(|| {
                                ParseError::new(
                                    ParseErrorCode::E2005,
                                    format!("object has no field '{}'", seg.node),
                                    seg.span,
                                )
                            })?
                            .value
                            .clone(),
                        Value::RefExpr(mut r) => {
                            r.steps.push(altium_format::sch_ops_core::RefStep::Member(
                                seg.node.clone(),
                            ));
                            Value::RefExpr(r)
                        }
                        _ => {
                            return Err(ParseError::new(
                                ParseErrorCode::E2003,
                                "field access is only valid on objects and refs",
                                expr.span,
                            ));
                        }
                    }
                }
                Expr::Index(base, idx_expr) => {
                    let base_val = self.eval_expr(base)?;
                    let idx_val = self.eval_expr(idx_expr)?;
                    match base_val.value {
                        Value::Array(v) => {
                            let idx = as_usize(&idx_val.value, idx_expr.span)?;
                            v.get(idx)
                                .ok_or_else(|| {
                                    ParseError::new(
                                        ParseErrorCode::E2005,
                                        format!("array index {idx} out of bounds"),
                                        idx_expr.span,
                                    )
                                })?
                                .value
                                .clone()
                        }
                        Value::RefExpr(mut r) => {
                            match &idx_val.value {
                                Value::Integer(i) if *i >= 0 => r
                                    .steps
                                    .push(altium_format::sch_ops_core::RefStep::Index(*i as usize)),
                                Value::String(s) => r
                                    .steps
                                    .push(altium_format::sch_ops_core::RefStep::Member(s.clone())),
                                _ => {
                                    return Err(ParseError::new(
                                        ParseErrorCode::E2003,
                                        "reference index key must be integer or string",
                                        idx_expr.span,
                                    ));
                                }
                            }
                            Value::RefExpr(r)
                        }
                        _ => {
                            return Err(ParseError::new(
                                ParseErrorCode::E2003,
                                "indexing is only valid on arrays and refs",
                                expr.span,
                            ));
                        }
                    }
                }
                Expr::BinOp(left, op, right) => {
                    let l = self.eval_expr(left)?;
                    let r = self.eval_expr(right)?;
                    self.eval_binop(&l.value, op.node, &r.value, op.span)?
                }
                Expr::UnaryNeg(v) => {
                    let val = self.eval_expr(v)?;
                    match val.value {
                        Value::Integer(i) => Value::Integer(-i),
                        Value::Float(f) => Value::Float(-f),
                        Value::Dim(d) => Value::Dim(-d),
                        _ => {
                            return Err(ParseError::new(
                                ParseErrorCode::E2003,
                                "unary '-' requires numeric or dim value",
                                expr.span,
                            ));
                        }
                    }
                }
                Expr::Tuple(a, b) => {
                    let ax = self.eval_expr(a)?;
                    let by = self.eval_expr(b)?;
                    Value::Coord(
                        as_scalar_mils(&ax.value, a.span)?,
                        as_scalar_mils(&by.value, b.span)?,
                    )
                }
                Expr::Array(items) => {
                    let mut out = Vec::with_capacity(items.len());
                    for item in items {
                        out.push(self.eval_expr(item)?);
                    }
                    Value::Array(out)
                }
                Expr::Object(obj) => {
                    let m = self.eval_object(&Spanned::new(obj.clone(), expr.span))?;
                    Value::Object(m)
                }
            };

        Ok(TypedValue {
            value,
            span: expr.span,
        })
    }

    fn eval_field_value(
        &mut self,
        key: &str,
        expr: &Spanned<Expr>,
    ) -> Result<TypedValue, ParseError> {
        match self.eval_expr(expr) {
            Ok(v) => Ok(v),
            Err(err) => {
                let Expr::Ident(ident) = &expr.node else {
                    return Err(err);
                };
                if self.lookup_binding(ident).is_some() {
                    return Err(err);
                }
                if key == "electrical" {
                    let normalized = normalize_enum_ident(ident);
                    let schema = PinElectricalName::ops_enum_schema();
                    if let Some(variant) = schema
                        .variants
                        .iter()
                        .find(|variant| variant.normalized == normalized)
                    {
                        return Ok(TypedValue {
                            value: Value::String(variant.canonical.to_ascii_lowercase()),
                            span: expr.span,
                        });
                    }
                    let choices = schema
                        .variants
                        .iter()
                        .map(|v| v.canonical.to_ascii_lowercase())
                        .collect::<Vec<_>>()
                        .join(", ");
                    return Err(ParseError::new(
                        ParseErrorCode::E2003,
                        format!("unknown enum variant '{ident}' for field '{key}'"),
                        expr.span,
                    )
                    .with_help(format!("expected one of: {choices}")));
                }
                Err(err)
            }
        }
    }

    fn eval_binop(
        &self,
        left: &Value,
        op: BinOp,
        right: &Value,
        span: super::ast::Span,
    ) -> Result<Value, ParseError> {
        use BinOp::*;
        match (left, op, right) {
            (Value::Integer(a), Add, Value::Integer(b)) => Ok(Value::Integer(*a + *b)),
            (Value::Integer(a), Sub, Value::Integer(b)) => Ok(Value::Integer(*a - *b)),
            (Value::Integer(a), Mul, Value::Integer(b)) => Ok(Value::Integer(*a * *b)),
            (Value::Integer(a), Div, Value::Integer(b)) => Ok(Value::Float(*a as f64 / *b as f64)),

            (Value::Float(a), Add, Value::Float(b)) => Ok(Value::Float(*a + *b)),
            (Value::Float(a), Sub, Value::Float(b)) => Ok(Value::Float(*a - *b)),
            (Value::Float(a), Mul, Value::Float(b)) => Ok(Value::Float(*a * *b)),
            (Value::Float(a), Div, Value::Float(b)) => Ok(Value::Float(*a / *b)),

            (Value::Integer(a), Add, Value::Float(b)) => Ok(Value::Float(*a as f64 + *b)),
            (Value::Integer(a), Sub, Value::Float(b)) => Ok(Value::Float(*a as f64 - *b)),
            (Value::Integer(a), Mul, Value::Float(b)) => Ok(Value::Float(*a as f64 * *b)),
            (Value::Integer(a), Div, Value::Float(b)) => Ok(Value::Float(*a as f64 / *b)),

            (Value::Float(a), Add, Value::Integer(b)) => Ok(Value::Float(*a + *b as f64)),
            (Value::Float(a), Sub, Value::Integer(b)) => Ok(Value::Float(*a - *b as f64)),
            (Value::Float(a), Mul, Value::Integer(b)) => Ok(Value::Float(*a * *b as f64)),
            (Value::Float(a), Div, Value::Integer(b)) => Ok(Value::Float(*a / *b as f64)),

            (Value::Dim(a), Add, Value::Dim(b)) => Ok(Value::Dim(*a + *b)),
            (Value::Dim(a), Sub, Value::Dim(b)) => Ok(Value::Dim(*a - *b)),
            (Value::Dim(a), Mul, Value::Integer(b)) => Ok(Value::Dim(*a * *b as f64)),
            (Value::Dim(a), Mul, Value::Float(b)) => Ok(Value::Dim(*a * *b)),
            (Value::Dim(a), Div, Value::Integer(b)) => Ok(Value::Dim(*a / *b as f64)),
            (Value::Dim(a), Div, Value::Float(b)) => Ok(Value::Dim(*a / *b)),
            (Value::Integer(a), Mul, Value::Dim(b)) => Ok(Value::Dim(*a as f64 * *b)),
            (Value::Float(a), Mul, Value::Dim(b)) => Ok(Value::Dim(*a * *b)),

            _ => Err(ParseError::new(
                ParseErrorCode::E2003,
                "invalid operand types for arithmetic expression",
                span,
            )
            .with_help("supported: number +/-/*// number, dim +/- dim, dim * number, number * dim, dim / number")),
        }
    }

    fn validate_selector_semantics(
        &self,
        selector: &Spanned<super::ast::Selector>,
    ) -> Result<(), ParseError> {
        fn head_type(head: &SelectorSimple) -> String {
            match head {
                SelectorSimple::Type(v) => v.clone(),
                SelectorSimple::DesignatorPattern { .. }
                | SelectorSimple::DollarRef(_)
                | SelectorSimple::ComponentPin { .. } => "component".to_owned(),
                SelectorSimple::NetPattern(_) => "net".to_owned(),
                SelectorSimple::ValuePattern(_) => "component".to_owned(),
                SelectorSimple::PartPattern(_) => "component".to_owned(),
                SelectorSimple::IdPattern(_) => "component".to_owned(),
                SelectorSimple::Any => "component".to_owned(),
            }
        }

        fn validate_value_for_field(
            field_kind: &str,
            value: &SelectorValue,
            op: SelectorAttrOp,
            span: super::ast::Span,
        ) -> Result<(), ParseError> {
            let is_order_op = matches!(
                op,
                SelectorAttrOp::Gt | SelectorAttrOp::Lt | SelectorAttrOp::Ge | SelectorAttrOp::Le
            );
            match field_kind {
                "string" => {
                    if is_order_op {
                        return Err(ParseError::new(
                            ParseErrorCode::E2006,
                            "ordering operators are not valid for string selector fields",
                            span,
                        ));
                    }
                    match value {
                        SelectorValue::String(_)
                        | SelectorValue::Ident(_)
                        | SelectorValue::Regex(_) => Ok(()),
                        _ => Err(ParseError::new(
                            ParseErrorCode::E2006,
                            "string selector field expects string/ident/regex value",
                            span,
                        )),
                    }
                }
                "int" => match value {
                    SelectorValue::Integer(_)
                    | SelectorValue::Float(_)
                    | SelectorValue::Dim(_, _) => Ok(()),
                    _ => Err(ParseError::new(
                        ParseErrorCode::E2006,
                        "numeric selector field expects numeric value",
                        span,
                    )),
                },
                "bool" => match value {
                    SelectorValue::Bool(_) => Ok(()),
                    _ => Err(ParseError::new(
                        ParseErrorCode::E2006,
                        "bool selector field expects true/false",
                        span,
                    )),
                },
                _ => Ok(()),
            }
        }

        fn field_kind(entity_type: &str, field: &str) -> Option<&'static str> {
            match entity_type {
                "component" => match field {
                    "designator" | "lib_reference" | "value" | "description" => Some("string"),
                    "part_count" | "display_mode_count" | "component_kind" => Some("int"),
                    "show_hidden_pins" => Some("bool"),
                    "x" | "y" => Some("int"),
                    _ => None,
                },
                "pin" => match field {
                    "designator" | "name" | "electrical" => Some("string"),
                    "length" | "x" | "y" | "orientation" => Some("int"),
                    "is_hidden" => Some("bool"),
                    _ => None,
                },
                "net" => match field {
                    "name" => Some("string"),
                    _ => None,
                },
                _ => match field {
                    "name" | "text" => Some("string"),
                    "x" | "y" | "index" | "record_type" | "line_width" | "color" => Some("int"),
                    _ => None,
                },
            }
        }

        fn visit_expr(expr: &SelectorExpr, current_type: &mut String) -> Result<(), ParseError> {
            match expr {
                SelectorExpr::Or(v) | SelectorExpr::And(v) => {
                    for e in v {
                        visit_expr(&e.node, current_type)?;
                    }
                }
                SelectorExpr::Not(v) => visit_expr(&v.node, current_type)?,
                SelectorExpr::Chain(chain) => {
                    let mut local_type = head_type(&chain.first.node.head.node);
                    validate_compound(&chain.first, &mut local_type)?;
                    for link in &chain.rest {
                        match link.node.combinator.node {
                            SelectorCombinator::Descendant
                            | SelectorCombinator::Child
                            | SelectorCombinator::Adjacent
                            | SelectorCombinator::Sibling => {}
                        }
                        let mut t = head_type(&link.node.right.node.head.node);
                        validate_compound(&link.node.right, &mut t)?;
                    }
                }
            }
            let _ = current_type;
            Ok(())
        }

        fn validate_compound(
            compound: &Spanned<super::ast::SelectorCompound>,
            current_type: &mut String,
        ) -> Result<(), ParseError> {
            *current_type = head_type(&compound.node.head.node);
            for filt in &compound.node.filters {
                match &filt.node {
                    SelectorFilter::Attribute(attr) => {
                        let field_name = attr
                            .field
                            .iter()
                            .map(|s| s.node.as_str())
                            .collect::<Vec<_>>()
                            .join(".");
                        let Some(kind) = field_kind(current_type.as_str(), &field_name) else {
                            return Err(ParseError::new(
                                ParseErrorCode::E2006,
                                format!(
                                    "unknown selector field '{field_name}' for type '{}'",
                                    current_type
                                ),
                                filt.span,
                            ));
                        };
                        validate_value_for_field(
                            kind,
                            &attr.value.node,
                            attr.op.node,
                            attr.value.span,
                        )?;
                    }
                    SelectorFilter::Pseudo(pseudo) => {
                        let pseudo_name = pseudo.node.as_str();
                        let ok = match (current_type.as_str(), pseudo_name) {
                            (
                                "pin",
                                "input" | "output" | "io" | "power" | "passive" | "open-collector"
                                | "open-emitter" | "hiz",
                            ) => true,
                            ("component", "placed" | "locked" | "virtual") => true,
                            ("net", "power" | "ground" | "signal" | "differential") => true,
                            (_, "selected" | "visible" | "on-grid") => true,
                            _ => false,
                        };
                        if !ok {
                            return Err(ParseError::new(
                                ParseErrorCode::E2006,
                                format!(
                                    "invalid pseudo-class ':{pseudo_name}' for type '{}'",
                                    current_type
                                ),
                                pseudo.span,
                            ));
                        }
                    }
                }
            }
            Ok(())
        }

        let mut ty = "component".to_string();
        visit_expr(&selector.node.expr.node, &mut ty)
    }

    fn validate_body_against_schema<T: HasOpsSchema>(
        &mut self,
        body: &Spanned<Object>,
    ) -> Result<(), ParseError> {
        let schema = T::ops_schema();
        let map = self.eval_object(body)?;
        let allowed: HashMap<&str, _> = schema.fields.iter().map(|f| (f.name, f)).collect();

        for (field, value) in &map {
            let Some(field_schema) = allowed.get(field.as_str()) else {
                return Err(ParseError::new(
                    ParseErrorCode::E2002,
                    format!("unknown field '{field}' for op '{}'", schema.op_name),
                    value.span,
                ));
            };
            if !matches_field_type(field_schema.ty, &value.value) {
                return Err(ParseError::new(
                    ParseErrorCode::E2003,
                    format!("field '{field}' has wrong type for op '{}'", schema.op_name),
                    value.span,
                )
                .with_help(format!("expected type {:?}", field_schema.ty)));
            }
        }

        for field in schema.fields {
            if field.required && !map.contains_key(field.name) {
                return Err(ParseError::new(
                    ParseErrorCode::E2002,
                    format!(
                        "missing required field '{}' for op '{}'",
                        field.name, schema.op_name
                    ),
                    body.span,
                ));
            }
        }

        Ok(())
    }

    fn expect_body<'b>(&self, op: &'b Op) -> Result<&'b Spanned<Object>, ParseError> {
        op.body.as_ref().ok_or_else(|| {
            ParseError::new(
                ParseErrorCode::E2008,
                format!("op '{}' requires object body", op.name.node),
                op.name.span,
            )
        })
    }

    fn selector_to_single_component_ref(
        &self,
        selector: &Spanned<super::ast::Selector>,
    ) -> Result<altium_format::sch_ops_core::RefExpr, ParseError> {
        let expr = &selector.node.expr.node;
        let SelectorExpr::Chain(chain) = expr else {
            return Err(ParseError::new(
                ParseErrorCode::E2008,
                "selector shape is not yet supported for direct edit/remove lowering",
                selector.span,
            )
            .with_help(
                "for now, use a single op-result selector like `edit $r1 { ... }` or `remove $r1`",
            ));
        };
        if !chain.rest.is_empty() {
            return Err(ParseError::new(
                ParseErrorCode::E2008,
                "selector combinators are not yet supported for direct edit/remove lowering",
                selector.span,
            )
            .with_help("for now, use a single op-result selector like `$r1`"));
        }
        let compound = &chain.first.node;
        if !compound.filters.is_empty() {
            return Err(ParseError::new(
                ParseErrorCode::E2008,
                "selector filters are not yet supported for direct edit/remove lowering",
                selector.span,
            )
            .with_help("for now, use a plain op-result selector like `$r1`"));
        }

        match &compound.head.node {
            SelectorSimple::DollarRef(name) if self.op_bindings.contains(name) => {
                Ok(altium_format::sch_ops_core::RefExpr::op(name.clone()))
            }
            SelectorSimple::DollarRef(name) => Err(ParseError::new(
                ParseErrorCode::E2005,
                format!("unknown op-result reference '${name}'"),
                compound.head.span,
            )
            .with_help("bind an op with `name = op ...` before referencing `$name`")),
            _ => Err(ParseError::new(
                ParseErrorCode::E2008,
                "selector head is not yet supported for direct edit/remove lowering",
                compound.head.span,
            )
            .with_help("for now, use an op-result selector `$name` from `name = op ...`")),
        }
    }

    fn reject_unknown_keys(
        &self,
        map: &IndexMap<String, TypedValue>,
        allowed: &[&str],
        span: super::ast::Span,
        op_name: &str,
    ) -> Result<(), ParseError> {
        let allowed_set: HashSet<&str> = allowed.iter().copied().collect();
        for key in map.keys() {
            if !allowed_set.contains(key.as_str()) {
                return Err(ParseError::new(
                    ParseErrorCode::E2002,
                    format!("unknown field '{key}' for op '{op_name}'"),
                    span,
                ));
            }
        }
        Ok(())
    }

    fn value_to_string(&self, value: &Value) -> Result<String, ParseError> {
        Ok(match value {
            Value::String(v) => v.clone(),
            Value::Integer(v) => v.to_string(),
            Value::Float(v) => v.to_string(),
            Value::Bool(v) => v.to_string(),
            Value::Dim(v) => format!("{}mil", v),
            Value::Null => "null".to_string(),
            Value::RefExpr(_) => {
                return Err(ParseError::new(
                    ParseErrorCode::E2003,
                    "cannot interpolate reference expression into string at compile-time",
                    super::ast::Span::new(0, 0),
                ));
            }
            Value::Coord(_, _) | Value::Color(_, _, _) | Value::Object(_) | Value::Array(_) => {
                return Err(ParseError::new(
                    ParseErrorCode::E2003,
                    "cannot interpolate complex value into string at compile-time",
                    super::ast::Span::new(0, 0),
                ));
            }
        })
    }

    fn insert_binding(
        &mut self,
        name: String,
        value: TypedValue,
        span: super::ast::Span,
    ) -> Result<(), ParseError> {
        let Some(scope) = self.scopes.last_mut() else {
            return Err(ParseError::new(
                ParseErrorCode::E2008,
                "internal scope stack underflow",
                span,
            ));
        };
        if scope.contains_key(&name) {
            return Err(ParseError::new(
                ParseErrorCode::E2008,
                format!("binding '{name}' is already defined in this scope"),
                span,
            ));
        }
        scope.insert(name, value);
        Ok(())
    }

    fn lookup_binding(&self, name: &str) -> Option<TypedValue> {
        for scope in self.scopes.iter().rev() {
            if let Some(v) = scope.get(name) {
                return Some(v.clone());
            }
        }
        None
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        let _ = self.scopes.pop();
    }

    fn req_string(
        &self,
        map: &IndexMap<String, TypedValue>,
        key: &str,
    ) -> Result<String, ParseError> {
        match map.get(key) {
            Some(TypedValue {
                value: Value::String(v),
                ..
            }) => Ok(v.clone()),
            Some(v) => Err(ParseError::new(
                ParseErrorCode::E2003,
                format!("field '{key}' must be string"),
                v.span,
            )),
            None => Err(ParseError::new(
                ParseErrorCode::E2002,
                format!("missing required field '{key}'"),
                super::ast::Span::new(0, self.source.len() as u32),
            )),
        }
    }

    fn opt_string(
        &self,
        map: &IndexMap<String, TypedValue>,
        key: &str,
    ) -> Result<Option<String>, ParseError> {
        match map.get(key) {
            Some(TypedValue {
                value: Value::String(v),
                ..
            }) => Ok(Some(v.clone())),
            Some(TypedValue {
                value: Value::Null, ..
            }) => Ok(None),
            Some(v) => Err(ParseError::new(
                ParseErrorCode::E2003,
                format!("field '{key}' must be string"),
                v.span,
            )),
            None => Ok(None),
        }
    }

    fn req_refexpr(
        &self,
        map: &IndexMap<String, TypedValue>,
        key: &str,
    ) -> Result<altium_format::sch_ops_core::RefExpr, ParseError> {
        match map.get(key) {
            Some(v) => self.expect_refexpr(v, v.span, &format!("field '{key}' must be reference")),
            None => Err(ParseError::new(
                ParseErrorCode::E2002,
                format!("missing required field '{key}'"),
                super::ast::Span::new(0, self.source.len() as u32),
            )),
        }
    }

    fn opt_refexpr(
        &self,
        map: &IndexMap<String, TypedValue>,
        key: &str,
    ) -> Result<Option<altium_format::sch_ops_core::RefExpr>, ParseError> {
        match map.get(key) {
            Some(v) => Ok(Some(self.expect_refexpr(
                v,
                v.span,
                &format!("field '{key}' must be reference"),
            )?)),
            None => Ok(None),
        }
    }

    fn expect_refexpr(
        &self,
        value: &TypedValue,
        span: super::ast::Span,
        message: &str,
    ) -> Result<altium_format::sch_ops_core::RefExpr, ParseError> {
        match &value.value {
            Value::RefExpr(v) => Ok(v.clone()),
            _ => Err(ParseError::new(ParseErrorCode::E2003, message, span)),
        }
    }

    fn eval_as_refexpr(
        &mut self,
        expr: &Spanned<Expr>,
        message: &str,
    ) -> Result<altium_format::sch_ops_core::RefExpr, ParseError> {
        let v = self.eval_expr(expr)?;
        self.expect_refexpr(&v, expr.span, message)
    }

    fn opt_i32(
        &self,
        map: &IndexMap<String, TypedValue>,
        key: &str,
    ) -> Result<Option<i32>, ParseError> {
        match map.get(key) {
            Some(TypedValue {
                value: Value::Integer(v),
                ..
            }) => Ok(Some(*v)),
            Some(v) => Err(ParseError::new(
                ParseErrorCode::E2003,
                format!("field '{key}' must be integer"),
                v.span,
            )),
            None => Ok(None),
        }
    }

    fn req_i32_or_dim_mils(
        &self,
        map: &IndexMap<String, TypedValue>,
        key: &str,
    ) -> Result<i32, ParseError> {
        match map.get(key) {
            Some(TypedValue {
                value: Value::Integer(v),
                ..
            }) => Ok(*v),
            Some(TypedValue {
                value: Value::Dim(v),
                ..
            }) => Ok(*v as i32),
            Some(v) => Err(ParseError::new(
                ParseErrorCode::E2003,
                format!("field '{key}' must be integer or dim"),
                v.span,
            )),
            None => Err(ParseError::new(
                ParseErrorCode::E2002,
                format!("missing required field '{key}'"),
                super::ast::Span::new(0, self.source.len() as u32),
            )),
        }
    }

    fn opt_f64(
        &self,
        map: &IndexMap<String, TypedValue>,
        key: &str,
    ) -> Result<Option<f64>, ParseError> {
        match map.get(key) {
            Some(TypedValue {
                value: Value::Float(v),
                ..
            }) => Ok(Some(*v)),
            Some(TypedValue {
                value: Value::Integer(v),
                ..
            }) => Ok(Some(*v as f64)),
            Some(v) => Err(ParseError::new(
                ParseErrorCode::E2003,
                format!("field '{key}' must be float or integer"),
                v.span,
            )),
            None => Ok(None),
        }
    }

    fn opt_i32_or_dim_mils(
        &self,
        map: &IndexMap<String, TypedValue>,
        key: &str,
    ) -> Result<Option<i32>, ParseError> {
        match map.get(key) {
            Some(TypedValue {
                value: Value::Integer(v),
                ..
            }) => Ok(Some(*v)),
            Some(TypedValue {
                value: Value::Dim(v),
                ..
            }) => Ok(Some(*v as i32)),
            Some(v) => Err(ParseError::new(
                ParseErrorCode::E2003,
                format!("field '{key}' must be integer or dim"),
                v.span,
            )),
            None => Ok(None),
        }
    }

    fn opt_rotation_90(
        &self,
        map: &IndexMap<String, TypedValue>,
        key: &str,
    ) -> Result<Option<i32>, ParseError> {
        let Some(v) = self.opt_i32(map, key)? else {
            return Ok(None);
        };
        match v.rem_euclid(360) {
            0 | 90 | 180 | 270 => Ok(Some(v)),
            _ => Err(ParseError::new(
                ParseErrorCode::E2003,
                format!("field '{key}' must be one of 0, 90, 180, 270"),
                map.get(key).expect("rotation key exists").span,
            )),
        }
    }

    fn req_points_mils(
        &self,
        map: &IndexMap<String, TypedValue>,
        key: &str,
    ) -> Result<Vec<(i32, i32)>, ParseError> {
        let value = map.get(key).ok_or_else(|| {
            ParseError::new(
                ParseErrorCode::E2002,
                format!("missing required field '{key}'"),
                super::ast::Span::new(0, self.source.len() as u32),
            )
        })?;
        let Value::Array(items) = &value.value else {
            return Err(ParseError::new(
                ParseErrorCode::E2003,
                format!("field '{key}' must be array of points"),
                value.span,
            ));
        };
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            out.push(self.value_to_point_mils(item)?);
        }
        Ok(out)
    }

    fn opt_coord_mils(
        &self,
        map: &IndexMap<String, TypedValue>,
        key: &str,
    ) -> Result<Option<(i32, i32)>, ParseError> {
        let Some(value) = map.get(key) else {
            return Ok(None);
        };
        Ok(Some(self.value_to_point_mils(value)?))
    }

    fn req_from_to_mils(
        &self,
        map: &IndexMap<String, TypedValue>,
        span: super::ast::Span,
    ) -> Result<((i32, i32), (i32, i32)), ParseError> {
        let from = self.opt_coord_mils(map, "from")?;
        let to = self.opt_coord_mils(map, "to")?;
        if let (Some(from), Some(to)) = (from, to) {
            return Ok((from, to));
        }

        if map.contains_key("x1_mils")
            || map.contains_key("y1_mils")
            || map.contains_key("x2_mils")
            || map.contains_key("y2_mils")
        {
            let from = (
                self.req_i32_or_dim_mils(map, "x1_mils")?,
                self.req_i32_or_dim_mils(map, "y1_mils")?,
            );
            let to = (
                self.req_i32_or_dim_mils(map, "x2_mils")?,
                self.req_i32_or_dim_mils(map, "y2_mils")?,
            );
            return Ok((from, to));
        }

        Err(ParseError::new(
            ParseErrorCode::E2002,
            "missing geometry coordinates: use `from` and `to` coordinate tuples",
            span,
        )
        .with_help("example: from: (0mil, 0mil), to: (100mil, 50mil)"))
    }

    fn value_to_point_mils(&self, value: &TypedValue) -> Result<(i32, i32), ParseError> {
        match &value.value {
            Value::Coord(x, y) => Ok((*x as i32, *y as i32)),
            Value::Array(v) if v.len() == 2 => {
                let x = as_scalar_mils(&v[0].value, v[0].span)? as i32;
                let y = as_scalar_mils(&v[1].value, v[1].span)? as i32;
                Ok((x, y))
            }
            _ => Err(ParseError::new(
                ParseErrorCode::E2003,
                "point must be coord tuple `(x, y)` or `[x, y]`",
                value.span,
            )),
        }
    }

    fn opt_u8_array(
        &self,
        map: &IndexMap<String, TypedValue>,
        key: &str,
    ) -> Result<Option<Vec<u8>>, ParseError> {
        let Some(value) = map.get(key) else {
            return Ok(None);
        };
        let Value::Array(items) = &value.value else {
            return Err(ParseError::new(
                ParseErrorCode::E2003,
                format!("field '{key}' must be array of byte values"),
                value.span,
            ));
        };
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            let Value::Integer(v) = item.value else {
                return Err(ParseError::new(
                    ParseErrorCode::E2003,
                    format!("field '{key}' must contain only integer byte values"),
                    item.span,
                ));
            };
            if !(0..=255).contains(&v) {
                return Err(ParseError::new(
                    ParseErrorCode::E2003,
                    format!("byte value out of range in field '{key}': {v}"),
                    item.span,
                ));
            }
            out.push(v as u8);
        }
        Ok(Some(out))
    }

    fn opt_bool(
        &self,
        map: &IndexMap<String, TypedValue>,
        key: &str,
    ) -> Result<Option<bool>, ParseError> {
        match map.get(key) {
            Some(TypedValue {
                value: Value::Bool(v),
                ..
            }) => Ok(Some(*v)),
            Some(v) => Err(ParseError::new(
                ParseErrorCode::E2003,
                format!("field '{key}' must be bool"),
                v.span,
            )),
            None => Ok(None),
        }
    }

    fn extract_pins_array(&self, value: &TypedValue) -> Result<Vec<AddPinOp>, ParseError> {
        let Value::Array(items) = &value.value else {
            return Err(ParseError::new(
                ParseErrorCode::E2003,
                "pins must be array",
                value.span,
            ));
        };

        let mut out = Vec::with_capacity(items.len());
        for item in items {
            let Value::Object(obj) = &item.value else {
                return Err(ParseError::new(
                    ParseErrorCode::E2003,
                    "pins elements must be objects",
                    item.span,
                ));
            };
            out.push(AddPinOp {
                opid: None,
                id: self.opt_string(obj, "id")?,
                component_ref: self.opt_refexpr(obj, "component_ref")?,
                designator: self.req_string(obj, "designator")?,
                name: self.opt_string(obj, "name")?,
                electrical: self.opt_string(obj, "electrical")?,
                length_mils: self
                    .opt_i32_or_dim_mils(obj, "length")?
                    .or_else(|| self.opt_i32_or_dim_mils(obj, "length_mils").ok().flatten()),
                at: self.opt_coord_mils(obj, "at")?,
                rotation: self.opt_rotation_90(obj, "rotation")?,
            });
        }
        Ok(out)
    }

    fn extract_footprint(&self, value: &TypedValue) -> Result<FootprintOp, ParseError> {
        let Value::Object(obj) = &value.value else {
            return Err(ParseError::new(
                ParseErrorCode::E2003,
                "footprint must be object",
                value.span,
            ));
        };

        let map_entries = match obj.get("map") {
            Some(TypedValue {
                value: Value::Array(items),
                ..
            }) => {
                let mut out = Vec::new();
                for item in items {
                    let Value::Object(m) = &item.value else {
                        return Err(ParseError::new(
                            ParseErrorCode::E2003,
                            "footprint.map entries must be objects",
                            item.span,
                        ));
                    };
                    out.push(FootprintMapEntry {
                        pin: self.req_string(m, "pin")?,
                        pad: self.req_string(m, "pad")?,
                    });
                }
                out
            }
            Some(v) => {
                return Err(ParseError::new(
                    ParseErrorCode::E2003,
                    "footprint.map must be array",
                    v.span,
                ));
            }
            None => Vec::new(),
        };

        Ok(FootprintOp {
            model_name: self.req_string(obj, "model_name")?,
            map: map_entries,
        })
    }

    fn value_to_record_selector(
        &self,
        value: &TypedValue,
    ) -> Result<altium_format::sch_ops_core::RecordSelector, ParseError> {
        let Value::Object(m) = &value.value else {
            return Err(ParseError::new(
                ParseErrorCode::E2003,
                "record selector must be object",
                value.span,
            ));
        };

        if let Some(TypedValue {
            value: Value::String(v),
            ..
        }) = m.get("designator")
        {
            return Ok(altium_format::sch_ops_core::RecordSelector::ByDesignator(
                v.clone(),
            ));
        }
        if let Some(TypedValue {
            value: Value::Integer(v),
            ..
        }) = m.get("record_type")
        {
            return Ok(altium_format::sch_ops_core::RecordSelector::ByRecordType(
                *v,
            ));
        }
        if let Some(TypedValue {
            value: Value::Integer(v),
            ..
        }) = m.get("index")
        {
            if *v < 0 {
                return Err(ParseError::new(
                    ParseErrorCode::E2003,
                    "selector.index must be >= 0",
                    value.span,
                ));
            }
            return Ok(altium_format::sch_ops_core::RecordSelector::ByIndex(
                *v as usize,
            ));
        }
        if let Some(TypedValue {
            value: Value::String(v),
            ..
        }) = m.get("name")
        {
            return Ok(altium_format::sch_ops_core::RecordSelector::ByName(
                v.clone(),
            ));
        }

        Err(ParseError::new(
            ParseErrorCode::E2003,
            "selector object must contain one of: designator, record_type, index, name",
            value.span,
        ))
    }

    fn value_to_record_patch(
        &self,
        m: &IndexMap<String, TypedValue>,
    ) -> Result<altium_format::sch_ops_core::RecordPatch, ParseError> {
        Ok(altium_format::sch_ops_core::RecordPatch {
            text: self.opt_string(m, "text")?,
            name: self.opt_string(m, "name")?,
            designator: self.opt_string(m, "designator")?,
            is_hidden: self.opt_bool(m, "is_hidden")?,
            color: self.opt_i32(m, "color")?,
            line_width: self.opt_i32(m, "line_width")?,
        })
    }
}

fn as_usize(value: &Value, span: super::ast::Span) -> Result<usize, ParseError> {
    match value {
        Value::Integer(v) if *v >= 0 => Ok(*v as usize),
        _ => Err(ParseError::new(
            ParseErrorCode::E2003,
            "index must evaluate to non-negative integer",
            span,
        )),
    }
}

fn as_scalar_mils(value: &Value, span: super::ast::Span) -> Result<f64, ParseError> {
    match value {
        Value::Integer(v) => Ok(*v as f64),
        Value::Float(v) => Ok(*v),
        Value::Dim(v) => Ok(*v),
        _ => Err(ParseError::new(
            ParseErrorCode::E2003,
            "coordinate tuple component must be number or dim",
            span,
        )),
    }
}

fn to_mils(v: f64, unit: Unit) -> f64 {
    match unit {
        Unit::Mil => v,
        Unit::Mm => v * 39.370_078_74,
        Unit::Inch => v * 1000.0,
        Unit::Dxp => v * 10.0,
        Unit::Raw => v / 10_000.0,
    }
}

fn cmp_ord(left: f64, right: f64, op: super::ast::CompareOp) -> bool {
    use super::ast::CompareOp;
    match op {
        CompareOp::Eq => (left - right).abs() < f64::EPSILON,
        CompareOp::Ne => (left - right).abs() >= f64::EPSILON,
        CompareOp::Gt => left > right,
        CompareOp::Lt => left < right,
        CompareOp::Ge => left >= right,
        CompareOp::Le => left <= right,
    }
}

fn matches_field_type(expected: FieldType, value: &Value) -> bool {
    match expected {
        FieldType::Any => true,
        FieldType::String => matches!(value, Value::String(_) | Value::Null),
        FieldType::Integer => matches!(value, Value::Integer(_) | Value::Null),
        FieldType::Float => matches!(value, Value::Float(_) | Value::Integer(_) | Value::Null),
        FieldType::Bool => matches!(value, Value::Bool(_) | Value::Null),
        FieldType::Dim => matches!(
            value,
            Value::Dim(_) | Value::Integer(_) | Value::Float(_) | Value::Null
        ),
        FieldType::Coord => matches!(value, Value::Coord(_, _) | Value::Null),
        FieldType::Color => matches!(value, Value::Color(_, _, _) | Value::Null),
        FieldType::RefExpr => matches!(value, Value::RefExpr(_) | Value::Null),
        FieldType::Object => matches!(value, Value::Object(_) | Value::Null),
        FieldType::ObjectArray => {
            matches!(value, Value::Array(v) if v.iter().all(|item| matches!(item.value, Value::Object(_))))
                || matches!(value, Value::Null)
        }
        FieldType::Selector => matches!(value, Value::String(_) | Value::Null),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "proptest")]
    use proptest::prelude::*;
    #[cfg(feature = "proptest")]
    use proptest::string::string_regex;
    #[cfg(feature = "proptest")]
    use std::panic::{AssertUnwindSafe, catch_unwind};

    fn compile_ok(src: &str) -> Vec<HighOp> {
        compile_ops_to_high_schdoc(src).unwrap_or_else(|e| panic!("{}", e.render("tc.ops", src)))
    }

    fn op_kind(op: &HighOp) -> &'static str {
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

    #[test]
    fn compiles_add_component_and_query() {
        let src = r#"
pin_defaults = { electrical: "passive", length: 25 }
r1 = add_component {
  lib_reference: "R"
  designator: "R1"
  value: "10K"
  pins: [
    { designator: "1", ...pin_defaults }
    { designator: "2", ...pin_defaults }
  ]
}
query component[designator=R1]
"#;
        let ops = compile_ok(src);
        assert_eq!(ops.len(), 2);
        assert!(matches!(ops[0], HighOp::AddComponent(_)));
        assert!(matches!(ops[1], HighOp::Query(_)));
    }

    #[test]
    fn compiles_edit_and_remove_for_op_result_selector() {
        let src = r#"
r1 = add_component { lib_reference: "R", designator: "R1", value: "10K" }
edit $r1 { description: "updated" }
remove $r1
"#;
        let ops = compile_ok(src);
        assert_eq!(ops.len(), 3);
        assert!(matches!(ops[0], HighOp::AddComponent(_)));
        assert!(matches!(ops[1], HighOp::EditComponent(_)));
        assert!(matches!(ops[2], HighOp::RemoveComponent(_)));
    }

    #[test]
    fn rejects_unknown_field() {
        let src = r#"add_component { lib_reference: "R", bad_field: 1 }"#;
        let err = compile_ops_to_high_schdoc(src).expect_err("expected error");
        assert_eq!(err.code.as_str(), "E2002");
    }

    #[test]
    fn rejects_unsupported_edit_selector_shape() {
        let src = r#"edit component[designator=R1] { description: "x" }"#;
        let err = compile_ops_to_high_schdoc(src).expect_err("expected error");
        assert_eq!(err.code.as_str(), "E2008");
    }

    #[test]
    fn rejects_alias_ops_in_schdoc_domain() {
        let src = r#"add_alias $last { alias_name: "A1" }"#;
        let err = compile_ops_to_high_schdoc(src).expect_err("expected error");
        assert_eq!(err.code.as_str(), "E2008");
    }

    #[test]
    fn compiles_pcbdoc_track_ops() {
        let src = r#"
add_track { start: (0, 0), end: (100mil, 0), width_mils: 10, layer: "TopLayer" }
query track
"#;
        let ops = compile_ops_to_high_pcbdoc(src).expect("pcbdoc compile");
        assert_eq!(ops.len(), 2);
        assert!(matches!(ops[0], HighOp::AddTrack(_)));
        assert!(matches!(ops[1], HighOp::Query(_)));
    }

    #[test]
    fn compiles_pcblib_footprint_and_track_ops() {
        let src = r#"
fp = add_footprint { name: "OPS_FP" }
add_track $fp.ref { start: (0, 0), end: (100, 0), width_mils: 8 }
query footprint[name="OPS_FP"]
"#;
        let ops = compile_ops_to_high_pcblib(src).expect("pcblib compile");
        assert_eq!(ops.len(), 3);
        assert!(matches!(ops[0], HighOp::AddFootprint(_)));
        assert!(matches!(ops[1], HighOp::AddTrack(_)));
        assert!(matches!(ops[2], HighOp::Query(_)));
    }

    #[test]
    fn compiles_pcb_add_via_ops() {
        let src = r#"
add_via { at: (10, 20), diameter_mils: 24, hole_size_mils: 10, from_layer: "TopLayer", to_layer: "BottomLayer" }
query via
"#;
        let doc_ops = compile_ops_to_high_pcbdoc(src).expect("pcbdoc compile");
        let lib_ops = compile_ops_to_high_pcblib(src).expect("pcblib compile");
        assert_eq!(doc_ops.len(), 2);
        assert_eq!(lib_ops.len(), 2);
        assert!(matches!(doc_ops[0], HighOp::AddVia(_)));
        assert!(matches!(lib_ops[0], HighOp::AddVia(_)));
        assert!(matches!(doc_ops[1], HighOp::Query(_)));
        assert!(matches!(lib_ops[1], HighOp::Query(_)));
    }

    #[test]
    fn resolves_bare_enum_ident_for_electrical() {
        let src = r#"add_pin $last { designator: "1", electrical: passive }"#;
        let ops = compile_ok(src);
        let HighOp::AddPin(pin) = &ops[0] else {
            panic!("expected add_pin");
        };
        assert_eq!(pin.electrical.as_deref(), Some("passive"));
    }

    #[test]
    fn rejects_unknown_bare_enum_ident_for_electrical() {
        let src = r#"add_pin $last { designator: "1", electrical: passthru }"#;
        let err = compile_ops_to_high_schdoc(src).expect_err("expected enum error");
        assert_eq!(err.code.as_str(), "E2003");
    }

    #[test]
    fn selector_supports_negative_numeric_values() {
        let src = r#"query component[x>=-10][y<-2.5mm]"#;
        let ops = compile_ok(src);
        assert_eq!(ops.len(), 1);
        assert!(matches!(ops[0], HighOp::Query(_)));
    }

    #[test]
    fn compiles_graphics_ops_with_dim_inputs() {
        let src = r#"
add_line { x1_mils: 0mil, y1_mils: 0, x2_mils: 100, y2_mils: 100mil }
add_rectangle { x1_mils: 0, y1_mils: 0, x2_mils: 40mil, y2_mils: 20mil, is_solid: true }
add_arc { cx_mils: 0, cy_mils: 0, radius_mils: 10 }
add_elliptical_arc { cx_mils: 0, cy_mils: 0, radius_mils: 10, secondary_radius_mils: 5 }
add_ellipse { cx_mils: 0, cy_mils: 0, radius_mils: 10, secondary_radius_mils: 5 }
add_polyline { points_mils: [(0, 0), (10, 10)] }
add_polygon { points_mils: [(0, 0), (10, 0), (10, 10)] }
add_bezier { points_mils: [[0, 0], [10, 10], [20, 0], [30, 10]] }
add_pie { cx_mils: 0, cy_mils: 0, radius_mils: 10 }
add_round_rectangle { x1_mils: 0, y1_mils: 0, x2_mils: 20, y2_mils: 10, corner_x_radius_mils: 2, corner_y_radius_mils: 2 }
add_label { x_mils: 10mil, y_mils: 20, text: "U1" }
add_text_frame { x1_mils: 0, y1_mils: 0, x2_mils: 100, y2_mils: 50, text: "hello" }
add_image { x1_mils: 0, y1_mils: 0, x2_mils: 10, y2_mils: 10, file_name: "x.png", image_data: [1, 2, 3] }
"#;
        let ops = compile_ok(src);
        assert_eq!(ops.len(), 13);
        assert!(matches!(ops[0], HighOp::AddLine(_)));
        assert!(matches!(ops[1], HighOp::AddRectangle(_)));
        assert!(matches!(ops[2], HighOp::AddArc(_)));
        assert!(matches!(ops[3], HighOp::AddEllipticalArc(_)));
        assert!(matches!(ops[4], HighOp::AddEllipse(_)));
        assert!(matches!(ops[5], HighOp::AddPolyline(_)));
        assert!(matches!(ops[6], HighOp::AddPolygon(_)));
        assert!(matches!(ops[7], HighOp::AddBezier(_)));
        assert!(matches!(ops[8], HighOp::AddPie(_)));
        assert!(matches!(ops[9], HighOp::AddRoundRectangle(_)));
        assert!(matches!(ops[10], HighOp::AddLabel(_)));
        assert!(matches!(ops[11], HighOp::AddTextFrame(_)));
        assert!(matches!(ops[12], HighOp::AddImage(_)));
    }

    #[test]
    fn validates_selector_semantics() {
        let src = r#"query component[designator^=R] AND pin:power"#;
        let _ = compile_ok(src);

        let bad = r#"query component[unknown_field=1]"#;
        let err = compile_ops_to_high_schdoc(bad).expect_err("expected selector error");
        assert_eq!(err.code.as_str(), "E2006");
    }

    #[test]
    fn dim_and_integer_pin_length_are_equivalent() {
        let a = r#"add_pin $last { designator: "1", length_mils: 25 }"#;
        let b = r#"add_pin $last { designator: "1", length_mils: 25mil }"#;
        let ops_a = compile_ok(a);
        let ops_b = compile_ok(b);
        let HighOp::AddPin(pin_a) = &ops_a[0] else {
            panic!("expected add_pin");
        };
        let HighOp::AddPin(pin_b) = &ops_b[0] else {
            panic!("expected add_pin");
        };
        assert_eq!(pin_a.length_mils, pin_b.length_mils);
    }

    #[test]
    fn schdoc_and_schlib_compile_shape_match_for_shared_ops() {
        let src = r#"
r1 = add_component { lib_reference: "R", designator: "R1", value: "10K" }
add_pin $r1 { designator: "1", name: "A", electrical: "passive", length_mils: 25 }
query component[designator=R1]
"#;
        let doc_ops = compile_ops_to_high_schdoc(src).expect("schdoc compile");
        let lib_ops = compile_ops_to_high_schlib(src).expect("schlib compile");
        assert_eq!(doc_ops.len(), lib_ops.len());
        for (a, b) in doc_ops.iter().zip(lib_ops.iter()) {
            assert_eq!(op_kind(a), op_kind(b));
        }
    }

    #[cfg(feature = "proptest")]
    proptest! {
        #[test]
        fn prop_typecheck_never_panics(s in string_regex(r"(?s).{0,240}").expect("regex")) {
            let result = catch_unwind(AssertUnwindSafe(|| compile_ops_to_high_schdoc(&s)));
            prop_assert!(result.is_ok(), "typecheck panicked for input {:?}", s);
        }

        #[test]
        fn prop_noise_tokens_keep_compiled_op_kinds(
            d in "[A-Za-z_][A-Za-z0-9_]{0,4}",
            v in 1i32..5000
        ) {
            let canonical = format!(
                "r1 = add_component {{ lib_reference: \"R\", designator: \"{d}\", value: \"{v}\" }}\n\
                 add_pin $r1 {{ designator: \"1\", length_mils: 25 }}\n\
                 query component[designator={d}]"
            );
            let noisy = format!(
                "let r1 = add_component {{ lib_reference: \"R\", designator: \"{d}\", value: \"{v}\", }};\n\
                 // pin\n\
                 add_pin $r1 {{ designator: \"1\", length_mils: 25mil, }};\n\
                 query component[designator={d}];"
            );

            let a = compile_ops_to_high_schdoc(&canonical).expect("canonical compile");
            let b = compile_ops_to_high_schdoc(&noisy).expect("noisy compile");
            let ak: Vec<&str> = a.iter().map(op_kind).collect();
            let bk: Vec<&str> = b.iter().map(op_kind).collect();
            prop_assert_eq!(ak, bk);
        }

        #[test]
        fn prop_valid_generated_add_component_compiles(
            designator in "[A-Za-z_][A-Za-z0-9_]{0,5}",
            value in 1i32..100000,
            len in 1i32..500
        ) {
            let src = format!(
                "add_component {{\n\
                   lib_reference: \"R\"\n\
                   designator: \"{designator}\"\n\
                   value: \"{value}\"\n\
                   pins: [{{ designator: \"1\", length_mils: {len} }}]\n\
                 }}"
            );
            let ops = compile_ops_to_high_schdoc(&src).expect("compile");
            prop_assert_eq!(ops.len(), 1);
            prop_assert!(matches!(ops[0], HighOp::AddComponent(_)));
        }
    }
}
