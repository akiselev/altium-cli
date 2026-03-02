//! SpecModel compiler: transforms a parsed [`SpecFile`] AST into a typed [`SpecModel`].
//!
//! ## Scope rules
//!
//! - File-level `let` bindings are evaluated in a single root scope.
//! - Each `component { ... }` and `footprint { ... }` gets its own scope frame
//!   pushed on top of the root scope.
//! - Each `part N { ... }` block gets its own scope frame pushed on top of the
//!   component scope.
//! - Bindings within a scope are forward-visible (two-pass).
//! - `import` declarations are skipped (M8).
//! - `row`, `column`, `grid` blocks are skipped (M10).
//! - Anchor references (`on:`, `after:`, `before:`) are resolved in M7.

use std::collections::HashMap;

use indexmap::IndexMap;

use altium_format_types::{
    Color, ComponentKind, Coord, CoordPoint, LayerRef, PadShape, PadStackMode, PinElectricalType,
    PlaneConnectionStyle, RotationBy90,
};
use altium_format_types::sch::{
    HorizontalAlign, LeftRightSide, LineStyle, PenWidth, PortArrowStyle, PortIoType,
    PowerObjectStyle, TextJustification,
};

use crate::ast::{
    AliasDecl, ComponentDecl, ComponentItem, FootprintDecl, FootprintItem, FootprintMapDecl,
    FootprintRef, GraphicDecl, MapEntry, Object, ObjectItem, PadDecl, ParameterDecl, PartBlock,
    PartItem, PinDecl, ProjectDecl, ProjectItem, SchDocObjectDecl, SchDocObjectItem, SheetDecl,
    SheetItem, SpecFile, SpecItem,
};
use crate::eval::{EvalResult, ScopeStack, SpecError, SpecErrorCode, Value, eval_expr};
use crate::model::{
    AnnotationMatchParamSpec, AnnotationSpec, BlanketSpec, BusEntrySpec, BusSpec, ClassGenSpec,
    ComparisonRuleSpec, CompileMaskSpec, ComponentSpec, DocumentSpec, ErcLevelOverride,
    ErcMatrixOverride, FontSpec, FootprintMapSpec, FootprintSpec, GraphicProperties, GraphicSpec,
    GraphicType, HarnessConnectorSpec, JunctionSpec, LayerSpec, LibraryUpdateSpec, NetLabelSpec,
    NetSpec, NoConnectSpec, NoteSpec, OutputGroupSpec, OutputSpec, PadSpec, ParamVariationSpec,
    ParameterSetSpec, ParameterSpec, PartSpec, PcbGraphicProperties, PcbGraphicSpec,
    PcbGraphicType, PinPadMap, PinRef, PinSpec, PortSpec, PowerObjectSpec, PowerSpec, PrjPcbSpec,
    ProbeSpec, ProjectSpec, SchDocComponentSpec, SchDocObjectSpec, SchDocSpec, SchLibSpec,
    SheetEntrySpec, SheetSpec, SheetSymbolSpec, SignalHarnessSpec, SpecDomain, SpecModel,
    SymbolRef, VariantSpec, VariationSpec, WireSpec,
};

use altium_format_types::project::{
    ChannelRoomNamingStyle, ConnectionCode, CrossRefLocationStyle, CrossRefPorts,
    CrossRefSheetStyle, ErrorLevel, FlattenMode, SortLocation, SortOrder, VariationKind,
};
use crate::diagnostic::Spanned;

// ── Public API ────────────────────────────────────────────────────────────────

/// Compile a parsed spec file into a typed [`SpecModel`].
///
/// `domain` selects whether to compile SchLib or PcbLib entities.
/// Top-level entities that don't match the domain are silently skipped.
pub fn compile_spec(file: &SpecFile, domain: SpecDomain) -> Result<SpecModel, SpecError> {
    let mut compiler = SpecCompiler::new(domain);
    compiler.compile(file)
}

// ── Compiler state ────────────────────────────────────────────────────────────

struct SpecCompiler {
    domain: SpecDomain,
    scope: ScopeStack,
    /// Counter for unnamed graphic unique_ids within the current entity context.
    unnamed_counters: IndexMap<String, usize>,
    /// Current entity context name (component or footprint name) for unique_id generation.
    context_name: String,
    /// Current part context (e.g. "part1") for part-scoped unique_ids.
    part_context: Option<String>,
}

impl SpecCompiler {
    fn new(domain: SpecDomain) -> Self {
        Self {
            domain,
            scope: ScopeStack::new(),
            unnamed_counters: IndexMap::new(),
            context_name: String::new(),
            part_context: None,
        }
    }

    fn compile(&mut self, file: &SpecFile) -> Result<SpecModel, SpecError> {
        // Root scope for file-level let bindings.
        self.scope.push();

        // Collect and evaluate file-level let bindings (forward-visible).
        let file_lets: Vec<_> = file.items.iter().filter_map(|item| {
            match &item.node {
                SpecItem::LetBinding(lb) => Some((&*lb.name.node, &lb.value)),
                _ => None,
            }
        }).collect();
        eval_let_bindings_slice(&file_lets, &mut self.scope)?;

        match self.domain {
            SpecDomain::SchLib => {
                let mut components = Vec::new();
                for item in &file.items {
                    if let SpecItem::Component(decl) = &item.node {
                        components.push(self.compile_component(decl)?);
                    }
                }
                self.scope.pop();
                Ok(SpecModel::SchLib(SchLibSpec { components }))
            }
            SpecDomain::PcbLib => {
                let mut footprints = Vec::new();
                for item in &file.items {
                    if let SpecItem::Footprint(decl) = &item.node {
                        footprints.push(self.compile_footprint(decl)?);
                    }
                }
                self.scope.pop();
                Ok(SpecModel::PcbLib(crate::model::PcbLibSpec { footprints }))
            }
            SpecDomain::PrjPcb => {
                let mut projects = Vec::new();
                for item in &file.items {
                    if let SpecItem::Project(decl) = &item.node {
                        projects.push(self.compile_project(decl)?);
                    }
                }
                self.scope.pop();
                Ok(SpecModel::PrjPcb(PrjPcbSpec { projects }))
            }
            SpecDomain::SchDoc => {
                let spec = self.compile_schdoc(file)?;
                self.scope.pop();
                Ok(SpecModel::SchDoc(spec))
            }
        }
    }

    // ── Component compilation ──────────────────────────────────────────────

    fn compile_component(&mut self, decl: &ComponentDecl) -> Result<ComponentSpec, SpecError> {
        let lib_reference = decl.name.node.as_str();
        self.context_name = lib_reference.clone();
        self.unnamed_counters.clear();
        self.part_context = None;

        // Push component scope.
        self.scope.push();

        // Collect and evaluate component-level let bindings.
        let comp_lets: Vec<_> = decl.body.iter().filter_map(|item| {
            match &item.node {
                ComponentItem::LetBinding(lb) => Some((&*lb.name.node, &lb.value)),
                _ => None,
            }
        }).collect();
        eval_let_bindings_slice(&comp_lets, &mut self.scope)?;

        // Collect component-level properties from Property items.
        let props = collect_object_properties_from_items(
            decl.body.iter().filter_map(|item| {
                match &item.node {
                    ComponentItem::Property(p) => Some(p),
                    _ => None,
                }
            }),
            &self.scope,
        )?;

        let designator = get_string_opt(&props, "designator");
        let description = get_string_opt(&props, "description");
        let component_kind = get_enum_opt(&props, "component_kind", parse_component_kind)?;
        let part_count = get_integer_opt(&props, "part_count");
        let show_hidden_pins = get_bool_opt(&props, "show_hidden_pins");

        // Pass 1: build graphic binding map for anchor resolution.
        // Bound box-type graphics (rectangle, round_rectangle, text_frame, image)
        // expose named edges that pins can reference via `on: $body.left` etc.
        let binding_map = build_graphic_binding_map(
            decl.body.iter().filter_map(|item| {
                if let ComponentItem::Graphic(g) = &item.node { Some(g) } else { None }
            }),
            &self.scope,
        )?;

        // Pass 2: collect all anchor-pinned pin decls by edge for sequencing.
        // We need to resolve after:/before: chains before producing final PinSpecs.
        // For single-part components (part_count absent or 1), all items belong to part 1.
        // For multi-part components, component-level items are shared (part 0).
        let default_owner_part_id = if part_count.unwrap_or(1) <= 1 { 1i32 } else { 0i32 };

        let all_pin_decls_at_level: Vec<(&PinDecl, i32)> = decl.body.iter()
            .filter_map(|item| {
                if let ComponentItem::Pin(p) = &item.node { Some((p, default_owner_part_id)) } else { None }
            })
            .collect();

        // Compile children.
        let mut pins = resolve_anchor_pins(&all_pin_decls_at_level, &binding_map, &self.scope)?;
        let mut parameters = Vec::new();
        let mut aliases = Vec::new();
        let mut footprints = Vec::new();
        let mut graphics = Vec::new();
        let mut parts = Vec::new();

        for item in &decl.body {
            match &item.node {
                ComponentItem::Pin(_) => {
                    // Already compiled above via resolve_anchor_pins.
                }
                ComponentItem::Parameter(param_decl) => {
                    parameters.push(self.compile_parameter(param_decl)?);
                }
                ComponentItem::Alias(alias_decl) => {
                    aliases.push(self.compile_alias(alias_decl));
                }
                ComponentItem::FootprintMap(fp_decl) => {
                    footprints.push(self.compile_footprint_map(fp_decl)?);
                }
                ComponentItem::Graphic(graphic_decl) => {
                    graphics.push(self.compile_sch_graphic(graphic_decl)?);
                }
                ComponentItem::Part(part_block) => {
                    parts.push(self.compile_part_with_anchors(part_block, &binding_map)?);
                }
                ComponentItem::Property(_) | ComponentItem::LetBinding(_) => {
                    // Already handled above.
                }
            }
        }

        self.scope.pop();

        Ok(ComponentSpec {
            lib_reference,
            designator,
            description,
            component_kind,
            part_count,
            show_hidden_pins,
            pins,
            parameters,
            aliases,
            footprints,
            graphics,
            parts,
        })
    }

    // ── SchDoc compilation ──────────────────────────────────────────────────

    fn compile_schdoc(&mut self, file: &SpecFile) -> Result<SchDocSpec, SpecError> {
        let mut fonts = Vec::new();
        let mut custom_width = None;
        let mut custom_height = None;
        let mut snap_grid_on = None;
        let mut visible_grid_on = None;
        let mut hot_spot_grid_on = None;
        let mut show_hidden_pins = None;
        let mut border_on = None;
        let mut title_block_on = None;

        let mut components = Vec::new();
        let mut nets = Vec::new();
        let mut powers = Vec::new();
        let mut objects = Vec::new();

        for item in &file.items {
            match &item.node {
                SpecItem::Sheet(sheet_decl) => {
                    self.compile_sheet_metadata(
                        sheet_decl, &mut fonts,
                        &mut custom_width, &mut custom_height,
                        &mut snap_grid_on, &mut visible_grid_on, &mut hot_spot_grid_on,
                        &mut show_hidden_pins, &mut border_on, &mut title_block_on,
                    )?;
                }
                SpecItem::Component(comp_decl) => {
                    components.push(self.compile_schdoc_component(comp_decl)?);
                }
                SpecItem::Net(net_decl) => {
                    nets.push(self.compile_net(net_decl)?);
                }
                SpecItem::Power(power_decl) => {
                    powers.push(self.compile_power(power_decl)?);
                }
                SpecItem::SchDocObject(obj_decl) => {
                    objects.push(self.compile_schdoc_object(obj_decl)?);
                }
                SpecItem::Import(_) | SpecItem::LetBinding(_)
                | SpecItem::Footprint(_) | SpecItem::Project(_) => {
                    // Imports and let bindings handled above; other domains silently skipped.
                }
            }
        }

        let sheet = SheetSpec {
            fonts,
            custom_width,
            custom_height,
            snap_grid_on,
            visible_grid_on,
            hot_spot_grid_on,
            show_hidden_pins,
            border_on,
            title_block_on,
            components,
            nets,
            powers,
            objects,
        };

        Ok(SchDocSpec { sheets: vec![sheet] })
    }

    fn compile_sheet_metadata(
        &mut self,
        decl: &SheetDecl,
        fonts: &mut Vec<FontSpec>,
        custom_width: &mut Option<Coord>,
        custom_height: &mut Option<Coord>,
        snap_grid_on: &mut Option<bool>,
        visible_grid_on: &mut Option<bool>,
        hot_spot_grid_on: &mut Option<bool>,
        show_hidden_pins: &mut Option<bool>,
        border_on: &mut Option<bool>,
        title_block_on: &mut Option<bool>,
    ) -> Result<(), SpecError> {
        for item in &decl.body {
            match &item.node {
                SheetItem::FontBlock(fb) => {
                    for font_spanned in &fb.fonts {
                        fonts.push(self.compile_font(&font_spanned.node)?);
                    }
                }
                SheetItem::Property(prop) => {
                    let val = eval_expr(&prop.value, &self.scope)?;
                    match prop.key.node.as_str() {
                        "custom_width" => *custom_width = Some(value_to_coord(&val, Some(prop.value.span))?),
                        "custom_height" => *custom_height = Some(value_to_coord(&val, Some(prop.value.span))?),
                        "snap_grid_on" => *snap_grid_on = Some(value_to_bool(&val, Some(prop.value.span))?),
                        "visible_grid_on" => *visible_grid_on = Some(value_to_bool(&val, Some(prop.value.span))?),
                        "hot_spot_grid_on" => *hot_spot_grid_on = Some(value_to_bool(&val, Some(prop.value.span))?),
                        "show_hidden_pins" => *show_hidden_pins = Some(value_to_bool(&val, Some(prop.value.span))?),
                        "border_on" => *border_on = Some(value_to_bool(&val, Some(prop.value.span))?),
                        "title_block_on" => *title_block_on = Some(value_to_bool(&val, Some(prop.value.span))?),
                        other => {
                            return Err(SpecError::new(
                                SpecErrorCode::AltiumFormat,
                                format!("unknown sheet property '{}'", other),
                                Some(prop.key.span),
                            ));
                        }
                    }
                }
                SheetItem::LetBinding(_) => {
                    // Let bindings already evaluated at file level.
                }
            }
        }
        Ok(())
    }

    fn compile_font(&mut self, decl: &crate::ast::FontDecl) -> Result<FontSpec, SpecError> {
        let props = eval_object_to_map(&decl.body.node, &self.scope)?;
        let name = get_string_value_key(&props, "name", decl.body.span)?;
        let size = get_integer_opt(&props, "size").unwrap_or(10);
        let bold = get_bool_opt(&props, "bold");
        let italic = get_bool_opt(&props, "italic");
        let underline = get_bool_opt(&props, "underline");
        let strikeout = get_bool_opt(&props, "strikeout");
        let rotation = get_integer_opt(&props, "rotation");

        Ok(FontSpec {
            id: decl.id.node,
            name,
            size,
            bold,
            italic,
            underline,
            strikeout,
            rotation,
        })
    }

    fn compile_schdoc_component(
        &mut self,
        decl: &ComponentDecl,
    ) -> Result<SchDocComponentSpec, SpecError> {
        let designator = decl.name.node.as_str();

        self.scope.push();

        // Evaluate component-level let bindings.
        let comp_lets: Vec<_> = decl.body.iter().filter_map(|item| {
            match &item.node {
                ComponentItem::LetBinding(lb) => Some((&*lb.name.node, &lb.value)),
                _ => None,
            }
        }).collect();
        eval_let_bindings_slice(&comp_lets, &mut self.scope)?;

        let props = collect_object_properties_from_items(
            decl.body.iter().filter_map(|item| {
                match &item.node {
                    ComponentItem::Property(p) => Some(p),
                    _ => None,
                }
            }),
            &self.scope,
        )?;

        // Resolve symbol reference: either $alias.Name or lib_reference: "Name"
        let symbol = if let Some(v) = props.get("symbol") {
            match v {
                Value::String(s) => {
                    // Check if it's a $alias.Name from a DollarIdent expression
                    SymbolRef::Literal(s.clone())
                }
                _ => {
                    return Err(SpecError::no_span(
                        SpecErrorCode::TypeMismatch,
                        "symbol must be a string or $alias.Name reference".to_string(),
                    ));
                }
            }
        } else if let Some(v) = props.get("lib_reference") {
            match v {
                Value::String(s) => SymbolRef::Literal(s.clone()),
                _ => {
                    return Err(SpecError::no_span(
                        SpecErrorCode::TypeMismatch,
                        "lib_reference must be a string".to_string(),
                    ));
                }
            }
        } else {
            // Default: use designator as lib_reference
            SymbolRef::Literal(designator.clone())
        };

        let location = if let Some(v) = props.get("at") {
            value_to_coord_point(v, None)?
        } else {
            CoordPoint::zero()
        };
        let orientation = get_enum_opt(&props, "orientation", parse_rotation_by90)?;
        let is_mirrored = get_bool_opt(&props, "is_mirrored");
        let description = get_string_opt(&props, "description");

        // Compile parameters
        let mut parameters = Vec::new();
        for item in &decl.body {
            if let ComponentItem::Parameter(param_decl) = &item.node {
                parameters.push(self.compile_parameter(param_decl)?);
            }
        }

        self.scope.pop();

        Ok(SchDocComponentSpec {
            designator,
            symbol,
            location,
            orientation,
            is_mirrored,
            description,
            parameters,
        })
    }

    fn compile_net(&mut self, decl: &crate::ast::NetDecl) -> Result<NetSpec, SpecError> {
        let name = decl.name.node.as_str();
        let props = eval_object_to_map(&decl.body.node, &self.scope)?;

        let pins = self.compile_pin_refs(&props, decl.body.span)?;

        Ok(NetSpec { name, pins })
    }

    fn compile_power(&mut self, decl: &crate::ast::PowerDecl) -> Result<PowerSpec, SpecError> {
        let name = decl.name.node.as_str();
        let props = eval_object_to_map(&decl.body.node, &self.scope)?;

        let style = get_enum_opt(&props, "style", parse_power_object_style)?
            .unwrap_or(PowerObjectStyle::Bar);
        let show_net_name = get_bool_opt(&props, "show_net_name");
        let orientation = get_enum_opt(&props, "orientation", parse_rotation_by90)?;
        let pins = self.compile_pin_refs(&props, decl.body.span)?;

        Ok(PowerSpec { name, style, pins, show_net_name, orientation })
    }

    /// Parse a `pins: [U1.14, C1.1]` array into PinRef values.
    fn compile_pin_refs(
        &self,
        props: &IndexMap<String, Value>,
        span: crate::diagnostic::Span,
    ) -> Result<Vec<PinRef>, SpecError> {
        let pins_val = props.get("pins").ok_or_else(|| {
            SpecError::new(SpecErrorCode::TypeMismatch, "'pins' field is required".to_string(), Some(span))
        })?;

        let arr = match pins_val {
            Value::Array(a) => a,
            _ => return Err(SpecError::new(
                SpecErrorCode::TypeMismatch,
                "'pins' must be an array".to_string(),
                Some(span),
            )),
        };

        let mut refs = Vec::new();
        for item in arr {
            let s = match item {
                Value::String(s) => s.clone(),
                _ => return Err(SpecError::no_span(
                    SpecErrorCode::TypeMismatch,
                    "pin ref must be a string like \"U1.14\"".to_string(),
                )),
            };
            let (component, pin) = s.split_once('.').ok_or_else(|| {
                SpecError::no_span(
                    SpecErrorCode::TypeMismatch,
                    format!("invalid pin ref '{}': expected COMPONENT.PIN format", s),
                )
            })?;
            refs.push(PinRef {
                component: component.to_string(),
                pin: pin.to_string(),
            });
        }

        Ok(refs)
    }

    fn compile_schdoc_object(
        &mut self,
        decl: &SchDocObjectDecl,
    ) -> Result<SchDocObjectSpec, SpecError> {
        match decl.object_type.node.as_str() {
            "wire" => self.compile_wire_spec(decl),
            "bus" => self.compile_bus_spec(decl),
            "net_label" => self.compile_net_label_spec(decl),
            "power_object" => self.compile_power_object_spec(decl),
            "port" => self.compile_port_spec(decl),
            "junction" => self.compile_junction_spec(decl),
            "no_connect" => self.compile_no_connect_spec(decl),
            "bus_entry" => self.compile_bus_entry_spec(decl),
            "sheet_symbol" => self.compile_sheet_symbol_spec(decl),
            "parameter_set" => self.compile_parameter_set_spec(decl),
            "note" => self.compile_note_spec(decl),
            "probe" => self.compile_probe_spec(decl),
            "compile_mask" => self.compile_compile_mask_spec(decl),
            "blanket" => self.compile_blanket_spec(decl),
            "harness_connector" => self.compile_harness_connector_spec(decl),
            "signal_harness" => self.compile_signal_harness_spec(decl),
            "parameter" => self.compile_parameter_object_spec(decl),
            other => {
                // Try as a graphic type (label, line, rectangle, etc.)
                if let Some(graphic_type) = parse_sch_graphic_type(other) {
                    let props = self.collect_schdoc_object_props(decl)?;
                    let properties = compile_graphic_properties(&props, decl.object_type.span)?;
                    let unique_id = self.make_unique_id(None, other);
                    Ok(SchDocObjectSpec::Graphic(GraphicSpec { unique_id, graphic_type, properties }))
                } else {
                    Err(SpecError::new(
                        SpecErrorCode::AltiumFormat,
                        format!("unknown SchDoc object type '{}'", other),
                        Some(decl.object_type.span),
                    ))
                }
            }
        }
    }

    /// Collect properties from SchDocObjectDecl body items.
    fn collect_schdoc_object_props(
        &self,
        decl: &SchDocObjectDecl,
    ) -> Result<IndexMap<String, Value>, SpecError> {
        let mut props = IndexMap::new();
        for item in &decl.body {
            if let SchDocObjectItem::Property(p) = &item.node {
                let val = eval_expr(&p.value, &self.scope)?;
                props.insert(p.key.node.clone(), val);
            }
        }
        Ok(props)
    }

    fn compile_wire_spec(&mut self, decl: &SchDocObjectDecl) -> Result<SchDocObjectSpec, SpecError> {
        let props = self.collect_schdoc_object_props(decl)?;
        let vertices = get_coord_point_array(&props, "vertices")?;
        let color = get_color_opt(&props, "color");
        let line_width = get_enum_opt(&props, "line_width", parse_pen_width)?;
        let line_style = get_enum_opt(&props, "line_style", parse_line_style)?;
        Ok(SchDocObjectSpec::Wire(WireSpec { vertices, color, line_width, line_style }))
    }

    fn compile_bus_spec(&mut self, decl: &SchDocObjectDecl) -> Result<SchDocObjectSpec, SpecError> {
        let props = self.collect_schdoc_object_props(decl)?;
        let vertices = get_coord_point_array(&props, "vertices")?;
        let color = get_color_opt(&props, "color");
        let line_width = get_enum_opt(&props, "line_width", parse_pen_width)?;
        Ok(SchDocObjectSpec::Bus(BusSpec { vertices, color, line_width }))
    }

    fn compile_net_label_spec(&mut self, decl: &SchDocObjectDecl) -> Result<SchDocObjectSpec, SpecError> {
        let text = decl.name.as_ref().map(|n| n.node.as_str()).unwrap_or_default();
        let props = self.collect_schdoc_object_props(decl)?;
        let location = get_coord_point_required(&props, "at")?;
        let orientation = get_enum_opt(&props, "orientation", parse_rotation_by90)?;
        let justification = get_enum_opt(&props, "justification", parse_text_justification)?;
        let font_id = get_integer_opt(&props, "font_id");
        let color = get_color_opt(&props, "color");
        let is_mirrored = get_bool_opt(&props, "is_mirrored");
        Ok(SchDocObjectSpec::NetLabel(NetLabelSpec {
            text, location, orientation, justification, font_id, color, is_mirrored,
        }))
    }

    fn compile_power_object_spec(&mut self, decl: &SchDocObjectDecl) -> Result<SchDocObjectSpec, SpecError> {
        let text = decl.name.as_ref().map(|n| n.node.as_str()).unwrap_or_default();
        let props = self.collect_schdoc_object_props(decl)?;
        let location = get_coord_point_required(&props, "at")?;
        let orientation = get_enum_opt(&props, "orientation", parse_rotation_by90)?;
        let style = get_enum_opt(&props, "style", parse_power_object_style)?;
        let show_net_name = get_bool_opt(&props, "show_net_name");
        let font_id = get_integer_opt(&props, "font_id");
        let color = get_color_opt(&props, "color");
        let is_cross_sheet_connector = get_bool_opt(&props, "is_cross_sheet_connector");
        Ok(SchDocObjectSpec::PowerObject(PowerObjectSpec {
            text, location, orientation, style, show_net_name, font_id, color, is_cross_sheet_connector,
        }))
    }

    fn compile_port_spec(&mut self, decl: &SchDocObjectDecl) -> Result<SchDocObjectSpec, SpecError> {
        let name = decl.name.as_ref().map(|n| n.node.as_str()).unwrap_or_default();
        let props = self.collect_schdoc_object_props(decl)?;
        let location = get_coord_point_required(&props, "at")?;
        let io_type = get_enum_opt(&props, "io_type", parse_port_io_type)?;
        let style = get_enum_opt(&props, "style", parse_port_arrow_style)?;
        let width = get_coord_opt(&props, "width")?;
        let height = get_coord_opt(&props, "height")?;
        let color = get_color_opt(&props, "color");
        let area_color = get_color_opt(&props, "area_color");
        let text_color = get_color_opt(&props, "text_color");
        let font_id = get_integer_opt(&props, "font_id");
        let alignment = get_enum_opt(&props, "alignment", parse_horizontal_align)?;
        Ok(SchDocObjectSpec::Port(PortSpec {
            name, location, io_type, style, width, height, color, area_color, text_color, font_id, alignment,
        }))
    }

    fn compile_junction_spec(&mut self, decl: &SchDocObjectDecl) -> Result<SchDocObjectSpec, SpecError> {
        let props = self.collect_schdoc_object_props(decl)?;
        let location = get_coord_point_required(&props, "at")?;
        let color = get_color_opt(&props, "color");
        Ok(SchDocObjectSpec::Junction(JunctionSpec { location, color }))
    }

    fn compile_no_connect_spec(&mut self, decl: &SchDocObjectDecl) -> Result<SchDocObjectSpec, SpecError> {
        let props = self.collect_schdoc_object_props(decl)?;
        let location = get_coord_point_required(&props, "at")?;
        let color = get_color_opt(&props, "color");
        let orientation = get_enum_opt(&props, "orientation", parse_rotation_by90)?;
        Ok(SchDocObjectSpec::NoConnect(NoConnectSpec { location, color, orientation }))
    }

    fn compile_bus_entry_spec(&mut self, decl: &SchDocObjectDecl) -> Result<SchDocObjectSpec, SpecError> {
        let props = self.collect_schdoc_object_props(decl)?;
        let location = get_coord_point_required(&props, "at")?;
        let corner = get_coord_point_required(&props, "corner")?;
        let color = get_color_opt(&props, "color");
        let line_width = get_enum_opt(&props, "line_width", parse_pen_width)?;
        Ok(SchDocObjectSpec::BusEntry(BusEntrySpec { location, corner, color, line_width }))
    }

    fn compile_sheet_symbol_spec(&mut self, decl: &SchDocObjectDecl) -> Result<SchDocObjectSpec, SpecError> {
        let sheet_name = decl.name.as_ref().map(|n| n.node.as_str()).unwrap_or_default();
        let props = self.collect_schdoc_object_props(decl)?;
        let location = get_coord_point_required(&props, "at")?;
        let file_name = get_string_opt(&props, "file_name");
        let x_size = get_coord_opt(&props, "x_size")?;
        let y_size = get_coord_opt(&props, "y_size")?;
        let color = get_color_opt(&props, "color");
        let area_color = get_color_opt(&props, "area_color");

        // Compile entry blocks
        let mut entries = Vec::new();
        for item in &decl.body {
            if let SchDocObjectItem::Entry(entry_decl) = &item.node {
                entries.push(self.compile_sheet_entry(entry_decl)?);
            }
        }

        Ok(SchDocObjectSpec::SheetSymbol(SheetSymbolSpec {
            sheet_name, file_name, location, x_size, y_size, color, area_color, entries,
        }))
    }

    fn compile_sheet_entry(&mut self, decl: &crate::ast::EntryDecl) -> Result<SheetEntrySpec, SpecError> {
        let name = decl.name.node.as_str();
        let props = eval_object_to_map(&decl.body.node, &self.scope)?;
        let io_type = get_enum_opt(&props, "io_type", parse_port_io_type)?;
        let side = get_enum_opt(&props, "side", parse_left_right_side)?;
        let distance_from_top = get_coord_opt(&props, "distance")?;
        Ok(SheetEntrySpec { name, io_type, side, distance_from_top })
    }

    fn compile_parameter_set_spec(&mut self, decl: &SchDocObjectDecl) -> Result<SchDocObjectSpec, SpecError> {
        let name = decl.name.as_ref().map(|n| n.node.as_str()).unwrap_or_default();
        let props = self.collect_schdoc_object_props(decl)?;
        let location = if let Some(v) = props.get("at") {
            Some(value_to_coord_point(v, None)?)
        } else {
            None
        };

        let mut parameters = Vec::new();
        for item in &decl.body {
            if let SchDocObjectItem::Parameter(param_decl) = &item.node {
                parameters.push(self.compile_parameter(param_decl)?);
            }
        }

        Ok(SchDocObjectSpec::ParameterSet(ParameterSetSpec { name, location, parameters }))
    }

    fn compile_note_spec(&mut self, decl: &SchDocObjectDecl) -> Result<SchDocObjectSpec, SpecError> {
        let props = self.collect_schdoc_object_props(decl)?;
        let location = get_coord_point_required(&props, "at")?;
        let text = get_string_opt(&props, "text").unwrap_or_default();
        let color = get_color_opt(&props, "color");
        let area_color = get_color_opt(&props, "area_color");
        let font_id = get_integer_opt(&props, "font_id");
        Ok(SchDocObjectSpec::Note(NoteSpec { location, text, color, area_color, font_id }))
    }

    fn compile_probe_spec(&mut self, decl: &SchDocObjectDecl) -> Result<SchDocObjectSpec, SpecError> {
        let name = decl.name.as_ref().map(|n| n.node.as_str()).unwrap_or_default();
        let props = self.collect_schdoc_object_props(decl)?;
        let location = get_coord_point_required(&props, "at")?;
        let color = get_color_opt(&props, "color");
        Ok(SchDocObjectSpec::Probe(ProbeSpec { name, location, color }))
    }

    fn compile_compile_mask_spec(&mut self, decl: &SchDocObjectDecl) -> Result<SchDocObjectSpec, SpecError> {
        let props = self.collect_schdoc_object_props(decl)?;
        let location = get_coord_point_required(&props, "at")?;
        let corner = get_coord_point_required(&props, "corner")?;
        let color = get_color_opt(&props, "color");
        Ok(SchDocObjectSpec::CompileMask(CompileMaskSpec { location, corner, color }))
    }

    fn compile_blanket_spec(&mut self, decl: &SchDocObjectDecl) -> Result<SchDocObjectSpec, SpecError> {
        let props = self.collect_schdoc_object_props(decl)?;
        let location = get_coord_point_required(&props, "at")?;
        let corner = get_coord_point_required(&props, "corner")?;
        let vertices = if props.contains_key("vertices") {
            Some(get_coord_point_array(&props, "vertices")?)
        } else {
            None
        };
        let color = get_color_opt(&props, "color");
        Ok(SchDocObjectSpec::Blanket(BlanketSpec { location, corner, vertices, color }))
    }

    fn compile_harness_connector_spec(&mut self, decl: &SchDocObjectDecl) -> Result<SchDocObjectSpec, SpecError> {
        let props = self.collect_schdoc_object_props(decl)?;
        let location = get_coord_point_required(&props, "at")?;
        let x_size = get_coord_opt(&props, "x_size")?;
        let y_size = get_coord_opt(&props, "y_size")?;
        let color = get_color_opt(&props, "color");
        let area_color = get_color_opt(&props, "area_color");
        Ok(SchDocObjectSpec::HarnessConnector(HarnessConnectorSpec {
            location, x_size, y_size, color, area_color,
        }))
    }

    fn compile_signal_harness_spec(&mut self, decl: &SchDocObjectDecl) -> Result<SchDocObjectSpec, SpecError> {
        let props = self.collect_schdoc_object_props(decl)?;
        let vertices = get_coord_point_array(&props, "vertices")?;
        let color = get_color_opt(&props, "color");
        let line_width = get_enum_opt(&props, "line_width", parse_pen_width)?;
        Ok(SchDocObjectSpec::SignalHarness(SignalHarnessSpec { vertices, color, line_width }))
    }

    fn compile_parameter_object_spec(&mut self, decl: &SchDocObjectDecl) -> Result<SchDocObjectSpec, SpecError> {
        let name = decl.name.as_ref().map(|n| n.node.as_str()).unwrap_or_default();
        let props = self.collect_schdoc_object_props(decl)?;
        let text = get_string_opt(&props, "value").unwrap_or_default();
        let is_hidden = get_bool_opt(&props, "is_hidden");
        Ok(SchDocObjectSpec::Parameter(ParameterSpec {
            name: name.to_string(),
            text,
            is_hidden,
        }))
    }

    // ── Part compilation ───────────────────────────────────────────────────

    fn compile_part(&mut self, part_block: &PartBlock) -> Result<PartSpec, SpecError> {
        self.compile_part_with_anchors(part_block, &HashMap::new())
    }

    fn compile_part_with_anchors(
        &mut self,
        part_block: &PartBlock,
        binding_map: &GraphicBindingMap,
    ) -> Result<PartSpec, SpecError> {
        let part_number = part_block.number.node;
        self.part_context = Some(format!("part{}", part_number));

        self.scope.push();

        let part_lets: Vec<_> = part_block.body.iter().filter_map(|item| {
            match &item.node {
                PartItem::LetBinding(lb) => Some((&*lb.name.node, &lb.value)),
                _ => None,
            }
        }).collect();
        eval_let_bindings_slice(&part_lets, &mut self.scope)?;

        // Part-level graphic bindings (may shadow component-level ones).
        let part_binding_map = {
            let part_graphics = part_block.body.iter().filter_map(|item| {
                if let PartItem::Graphic(g) = &item.node { Some(g) } else { None }
            });
            let mut m = build_graphic_binding_map(part_graphics, &self.scope)?;
            // Merge component-level map (part-level takes precedence).
            for (k, v) in binding_map {
                m.entry(k.clone()).or_insert_with(|| v.clone());
            }
            m
        };

        let part_pin_decls: Vec<(&PinDecl, i32)> = part_block.body.iter()
            .filter_map(|item| {
                if let PartItem::Pin(p) = &item.node { Some((p, part_number)) } else { None }
            })
            .collect();

        let pins = resolve_anchor_pins(&part_pin_decls, &part_binding_map, &self.scope)?;

        let mut graphics = Vec::new();
        for item in &part_block.body {
            match &item.node {
                PartItem::Graphic(graphic_decl) => {
                    graphics.push(self.compile_sch_graphic(graphic_decl)?);
                }
                PartItem::Pin(_) | PartItem::LetBinding(_) => {}
            }
        }

        self.scope.pop();
        self.part_context = None;

        Ok(PartSpec { part_number, pins, graphics })
    }

    // ── Pin compilation ────────────────────────────────────────────────────

    fn compile_pin(
        &mut self,
        decl: &PinDecl,
        owner_part_id: i32,
    ) -> Result<PinSpec, SpecError> {
        let designator = decl.name.node.as_str();
        let props = eval_object_to_map(&decl.body.node, &self.scope)?;

        let name = get_string_opt(&props, "name");
        let electrical = get_enum_opt(&props, "electrical", parse_pin_electrical_type)?;
        let length = get_coord_opt(&props, "length")?;
        let is_hidden = get_bool_opt(&props, "is_hidden");
        let hidden_net_name = get_string_opt(&props, "hidden_net_name");
        let orientation = get_enum_opt(&props, "orientation", parse_rotation_by90)?
            .unwrap_or(RotationBy90::Rotate0);

        let location = if let Some(v) = props.get("at") {
            value_to_coord_point(v, Some(decl.body.span))?
        } else if let Some(x_val) = props.get("x") {
            let x = value_to_coord(x_val, Some(decl.body.span))?;
            let y = props.get("y")
                .map(|v| value_to_coord(v, Some(decl.body.span)))
                .transpose()?
                .unwrap_or(Coord::ZERO);
            CoordPoint::new(x, y)
        } else {
            CoordPoint::zero()
        };

        Ok(PinSpec {
            designator,
            name,
            electrical,
            length,
            location,
            orientation,
            is_hidden,
            hidden_net_name,
            owner_part_id,
        })
    }

    // ── Parameter compilation ──────────────────────────────────────────────

    fn compile_parameter(&mut self, decl: &ParameterDecl) -> Result<ParameterSpec, SpecError> {
        let name = decl.name.node.as_str();
        let props = eval_object_to_map(&decl.body.node, &self.scope)?;

        let text = get_string_opt(&props, "text")
            .or_else(|| get_string_opt(&props, "value"))
            .unwrap_or_default();
        let is_hidden = get_bool_opt(&props, "is_hidden");

        Ok(ParameterSpec { name, text, is_hidden })
    }

    // ── Alias compilation ──────────────────────────────────────────────────

    fn compile_alias(&mut self, decl: &AliasDecl) -> String {
        decl.name.node.as_str()
    }

    // ── FootprintMap compilation ───────────────────────────────────────────

    fn compile_footprint_map(
        &mut self,
        decl: &FootprintMapDecl,
    ) -> Result<FootprintMapSpec, SpecError> {
        let model_name = match &decl.name.node {
            FootprintRef::Name(n) => n.as_str(),
            FootprintRef::DollarPath(dp) => dp.root.node.clone(),
        };

        let mut maps = Vec::new();
        for map_entry_spanned in &decl.maps {
            let map_entry: &MapEntry = &map_entry_spanned.node;
            let map_props = eval_object_to_map(&map_entry.body.node, &self.scope)?;

            let pin = get_string_value_key(&map_props, "pin", map_entry.body.span)?;
            let pad = get_string_value_key(&map_props, "pad", map_entry.body.span)?;
            maps.push(PinPadMap { pin, pad });
        }

        Ok(FootprintMapSpec {
            model_name,
            maps,
            source: None,
        })
    }

    // ── Schematic graphic compilation ──────────────────────────────────────

    fn compile_sch_graphic(
        &mut self,
        decl: &GraphicDecl,
    ) -> Result<GraphicSpec, SpecError> {
        let graphic_type = parse_sch_graphic_type(&decl.graphic_type.node)
            .ok_or_else(|| SpecError::at(
                SpecErrorCode::TypeMismatch,
                format!("unknown schematic graphic type: '{}'", decl.graphic_type.node),
                decl.graphic_type.span,
            ))?;

        let unique_id = self.make_unique_id(decl.binding.as_ref(), &decl.graphic_type.node);

        let props = eval_object_to_map(&decl.body.node, &self.scope)?;
        let properties = compile_graphic_properties(&props, decl.body.span)?;

        Ok(GraphicSpec { unique_id, graphic_type, properties })
    }

    // ── Footprint compilation (PcbLib) ─────────────────────────────────────

    fn compile_footprint(
        &mut self,
        decl: &FootprintDecl,
    ) -> Result<FootprintSpec, SpecError> {
        let display_name = decl.name.node.as_str();
        self.context_name = display_name.clone();
        self.unnamed_counters.clear();
        self.part_context = None;

        self.scope.push();

        let fp_lets: Vec<_> = decl.body.iter().filter_map(|item| {
            match &item.node {
                FootprintItem::LetBinding(lb) => Some((&*lb.name.node, &lb.value)),
                _ => None,
            }
        }).collect();
        eval_let_bindings_slice(&fp_lets, &mut self.scope)?;

        let props = collect_object_properties_from_items(
            decl.body.iter().filter_map(|item| {
                match &item.node {
                    FootprintItem::Property(p) => Some(p),
                    _ => None,
                }
            }),
            &self.scope,
        )?;

        let description = get_string_opt(&props, "description");
        let height = get_coord_opt(&props, "height")?;
        let pattern = get_string_opt(&props, "pattern");

        let mut pads = Vec::new();
        let mut graphics = Vec::new();

        // First pass: collect explicit pads for override lookup.
        let explicit_pads: HashMap<String, &PadDecl> = decl.body.iter()
            .filter_map(|item| {
                if let FootprintItem::Pad(pd) = &item.node {
                    Some((pd.name.node.as_str(), pd))
                } else {
                    None
                }
            })
            .collect();

        // Track which explicit pad names were claimed by layout expansion.
        let mut claimed_by_layout: std::collections::HashSet<String> = std::collections::HashSet::new();

        for item in &decl.body {
            match &item.node {
                FootprintItem::Pad(_) => {
                    // Handled after layout expansion (explicit pads not claimed by layout).
                }
                FootprintItem::Graphic(graphic_decl) => {
                    graphics.push(self.compile_pcb_graphic(graphic_decl)?);
                }
                FootprintItem::Row(row_decl) | FootprintItem::Column(row_decl) => {
                    let generated = expand_row(row_decl, &self.scope)?;
                    for mut pad in generated {
                        let name = pad.pad_name.clone();
                        if let Some(explicit) = explicit_pads.get(&name) {
                            let explicit_props = eval_object_to_map(&explicit.body.node, &self.scope)?;
                            merge_pad_override_from_props(&mut pad, &explicit_props, explicit.body.span)?;
                            claimed_by_layout.insert(name);
                        }
                        pads.push(pad);
                    }
                }
                FootprintItem::Grid(grid_decl) => {
                    let generated = expand_grid(grid_decl, &self.scope)?;
                    for mut pad in generated {
                        let name = pad.pad_name.clone();
                        if let Some(explicit) = explicit_pads.get(&name) {
                            let explicit_props = eval_object_to_map(&explicit.body.node, &self.scope)?;
                            merge_pad_override_from_props(&mut pad, &explicit_props, explicit.body.span)?;
                            claimed_by_layout.insert(name);
                        }
                        pads.push(pad);
                    }
                }
                FootprintItem::Property(_) | FootprintItem::LetBinding(_) => {}
            }
        }

        // Second pass: add explicit pads that were NOT claimed by any layout.
        for item in &decl.body {
            if let FootprintItem::Pad(pad_decl) = &item.node {
                let name = pad_decl.name.node.as_str();
                if !claimed_by_layout.contains(&name) {
                    pads.push(self.compile_pad(pad_decl)?);
                }
            }
        }

        self.scope.pop();

        Ok(FootprintSpec {
            display_name,
            description,
            height,
            pattern,
            pads,
            graphics,
        })
    }

    // ── Pad compilation ────────────────────────────────────────────────────

    fn compile_pad(&mut self, decl: &PadDecl) -> Result<PadSpec, SpecError> {
        let pad_name = decl.name.node.as_str();
        let props = eval_object_to_map(&decl.body.node, &self.scope)?;

        let at = if let Some(v) = props.get("at") {
            value_to_coord_point(v, Some(decl.body.span))?
        } else if let Some(x_val) = props.get("x") {
            let x = value_to_coord(x_val, Some(decl.body.span))?;
            let y = props.get("y")
                .map(|v| value_to_coord(v, Some(decl.body.span)))
                .transpose()?
                .unwrap_or(Coord::ZERO);
            CoordPoint::new(x, y)
        } else {
            CoordPoint::zero()
        };

        let shape = get_enum_opt(&props, "shape", parse_pad_shape)?;
        let x_size = get_coord_opt(&props, "x_size")?;
        let y_size = get_coord_opt(&props, "y_size")?;
        let rotation = get_float_opt(&props, "rotation");
        let hole_size = get_coord_opt(&props, "hole_size")?;
        let is_plated = get_bool_opt(&props, "is_plated");
        let layer = get_enum_opt(&props, "layer", parse_layer_spec)?;
        let pad_mode = get_enum_opt(&props, "pad_mode", parse_pad_stack_mode)?;
        let solder_mask_expansion = get_coord_opt(&props, "solder_mask_expansion")?;
        let paste_mask_expansion = get_coord_opt(&props, "paste_mask_expansion")?;
        let plane_connection = get_enum_opt(&props, "plane_connection", parse_plane_connection)?;
        let relief_conductor_width = get_coord_opt(&props, "relief_conductor_width")?;
        let relief_entries = get_integer_opt(&props, "relief_entries");
        let relief_air_gap = get_coord_opt(&props, "relief_air_gap")?;

        Ok(PadSpec {
            pad_name,
            at,
            shape,
            x_size,
            y_size,
            rotation,
            hole_size,
            is_plated,
            layer,
            pad_mode,
            solder_mask_expansion,
            paste_mask_expansion,
            plane_connection,
            relief_conductor_width,
            relief_entries,
            relief_air_gap,
        })
    }

    // ── PCB graphic compilation ────────────────────────────────────────────

    fn compile_pcb_graphic(
        &mut self,
        decl: &GraphicDecl,
    ) -> Result<PcbGraphicSpec, SpecError> {
        let graphic_type = parse_pcb_graphic_type(&decl.graphic_type.node)
            .ok_or_else(|| SpecError::at(
                SpecErrorCode::TypeMismatch,
                format!("unknown PCB graphic type: '{}'", decl.graphic_type.node),
                decl.graphic_type.span,
            ))?;

        let unique_id = self.make_unique_id(decl.binding.as_ref(), &decl.graphic_type.node);

        let props = eval_object_to_map(&decl.body.node, &self.scope)?;
        let properties = compile_pcb_graphic_properties(&props, decl.body.span)?;

        Ok(PcbGraphicSpec { unique_id, graphic_type, properties })
    }

    // ── unique_id generation ───────────────────────────────────────────────

    fn make_unique_id(
        &mut self,
        binding: Option<&Spanned<String>>,
        type_name: &str,
    ) -> String {
        if let Some(b) = binding {
            // Named binding: spec:{context}[:part_context]:{name}
            if let Some(ref part_ctx) = self.part_context.clone() {
                format!("spec:{}:{}:{}", self.context_name, part_ctx, b.node)
            } else {
                format!("spec:{}:{}", self.context_name, b.node)
            }
        } else {
            // Unnamed: spec:{context}[:part_context]:{type}_{n}
            let counter_key = if let Some(ref part_ctx) = self.part_context.clone() {
                format!("{}:{}:{}", self.context_name, part_ctx, type_name)
            } else {
                format!("{}:{}", self.context_name, type_name)
            };
            let n = self.unnamed_counters.entry(counter_key).or_insert(0);
            let id = if let Some(ref part_ctx) = self.part_context.clone() {
                format!("spec:{}:{}:{}_{}", self.context_name, part_ctx, type_name, n)
            } else {
                format!("spec:{}:{}_{}", self.context_name, type_name, n)
            };
            *n += 1;
            id
        }
    }

    // ── Project compilation (PrjPcb) ──────────────────────────────────────

    fn compile_project(&mut self, decl: &ProjectDecl) -> Result<ProjectSpec, SpecError> {
        let name = decl.name.node.as_str();
        self.context_name = name.clone();
        self.unnamed_counters.clear();

        // Push project scope.
        self.scope.push();

        // Collect and evaluate project-level let bindings.
        let proj_lets: Vec<_> = decl.body.iter().filter_map(|item| {
            match &item.node {
                ProjectItem::LetBinding(lb) => Some((&*lb.name.node, &lb.value)),
                _ => None,
            }
        }).collect();
        eval_let_bindings_slice(&proj_lets, &mut self.scope)?;

        // Collect project-level properties from Property items.
        let props = collect_object_properties_from_items(
            decl.body.iter().filter_map(|item| {
                match &item.node {
                    ProjectItem::Property(p) => Some(p),
                    _ => None,
                }
            }),
            &self.scope,
        )?;

        // Extract scalar fields.
        let hierarchy_mode = get_enum_opt(&props, "hierarchy_mode", parse_flatten_mode)?;
        let channel_room_naming_style = get_enum_opt(&props, "channel_room_naming_style", parse_channel_room_naming_style)?;
        let channel_designator_format = get_string_opt(&props, "channel_designator_format");
        let channel_room_level_separator = get_string_opt(&props, "channel_room_level_separator");
        let allow_port_net_names = get_bool_opt(&props, "allow_port_net_names");
        let allow_sheet_entry_net_names = get_bool_opt(&props, "allow_sheet_entry_net_names");
        let netlist_single_pin_nets = get_bool_opt(&props, "netlist_single_pin_nets");
        let append_sheet_number_to_local_nets = get_bool_opt(&props, "append_sheet_number_to_local_nets");
        let name_nets_hierarchically = get_bool_opt(&props, "name_nets_hierarchically");
        let power_port_names_take_priority = get_bool_opt(&props, "power_port_names_take_priority");
        let pin_swap_by_netlabel = get_bool_opt(&props, "pin_swap_by_netlabel");
        let pin_swap_by_pin = get_bool_opt(&props, "pin_swap_by_pin");
        let cross_ref_sheet_style = get_enum_opt(&props, "cross_ref_sheet_style", parse_cross_ref_sheet_style)?;
        let cross_ref_location_style = get_enum_opt(&props, "cross_ref_location_style", parse_cross_ref_location_style)?;
        let cross_ref_ports = get_enum_opt(&props, "cross_ref_ports", parse_cross_ref_ports)?;
        let cross_ref_cross_sheets = get_bool_opt(&props, "cross_ref_cross_sheets");
        let cross_ref_sheet_entries = get_bool_opt(&props, "cross_ref_sheet_entries");
        let output_path = get_string_opt(&props, "output_path");

        // Compile child blocks.
        let mut documents = Vec::new();
        let mut annotation = None;
        let mut erc_matrix_overrides = Vec::new();
        let mut erc_level_overrides = Vec::new();
        let mut output_groups = Vec::new();
        let mut comparison_rules = Vec::new();
        let mut class_gen = None;
        let mut library_update = None;
        let mut variants = Vec::new();

        for item in &decl.body {
            match &item.node {
                ProjectItem::Property(_) | ProjectItem::LetBinding(_) => {
                    // Already handled above.
                }
                ProjectItem::Document(doc) => {
                    documents.push(self.compile_document(doc)?);
                }
                ProjectItem::Annotation(ann) => {
                    annotation = Some(self.compile_annotation(ann)?);
                }
                ProjectItem::ErcMatrix(entries) => {
                    for entry in entries {
                        erc_matrix_overrides.push(self.compile_erc_matrix_entry(&entry.node)?);
                    }
                }
                ProjectItem::ErcLevels(entries) => {
                    for entry in entries {
                        erc_level_overrides.push(self.compile_erc_level_entry(&entry.node)?);
                    }
                }
                ProjectItem::OutputGroup(group) => {
                    output_groups.push(self.compile_output_group(group)?);
                }
                ProjectItem::Comparison(rules) => {
                    for rule in rules {
                        comparison_rules.push(self.compile_comparison_rule(&rule.node)?);
                    }
                }
                ProjectItem::ClassGen(props_list) => {
                    let cg_props = collect_object_properties_from_items(
                        props_list.iter().map(|p| &p.node),
                        &self.scope,
                    )?;
                    class_gen = Some(ClassGenSpec {
                        generate_component_classes: get_bool_opt(&cg_props, "generate_component_classes"),
                        generate_net_classes: get_bool_opt(&cg_props, "generate_net_classes"),
                    });
                }
                ProjectItem::LibraryUpdate(props_list) => {
                    let lu_props = collect_object_properties_from_items(
                        props_list.iter().map(|p| &p.node),
                        &self.scope,
                    )?;
                    library_update = Some(LibraryUpdateSpec {
                        update_components: get_bool_opt(&lu_props, "update_components"),
                        update_models: get_bool_opt(&lu_props, "update_models"),
                    });
                }
                ProjectItem::Variant(var) => {
                    variants.push(self.compile_variant(var)?);
                }
            }
        }

        self.scope.pop();

        Ok(ProjectSpec {
            name,
            hierarchy_mode,
            channel_room_naming_style,
            channel_designator_format,
            channel_room_level_separator,
            allow_port_net_names,
            allow_sheet_entry_net_names,
            netlist_single_pin_nets,
            append_sheet_number_to_local_nets,
            name_nets_hierarchically,
            power_port_names_take_priority,
            pin_swap_by_netlabel,
            pin_swap_by_pin,
            cross_ref_sheet_style,
            cross_ref_location_style,
            cross_ref_ports,
            cross_ref_cross_sheets,
            cross_ref_sheet_entries,
            output_path,
            documents,
            annotation,
            erc_matrix_overrides,
            erc_level_overrides,
            output_groups,
            comparison_rules,
            class_gen,
            library_update,
            variants,
        })
    }

    fn compile_document(&mut self, doc: &crate::ast::DocumentBlockDecl) -> Result<DocumentSpec, SpecError> {
        let path = doc.path.node.as_str();
        let props = collect_object_properties_from_items(
            doc.body.iter().map(|p| &p.node),
            &self.scope,
        )?;
        Ok(DocumentSpec {
            path,
            annotation_enabled: get_bool_opt(&props, "annotation_enabled"),
            annotate_start_value: get_integer_opt(&props, "annotate_start_value"),
            do_library_update: get_bool_opt(&props, "do_library_update"),
            do_database_update: get_bool_opt(&props, "do_database_update"),
        })
    }

    fn compile_annotation(&mut self, ann: &crate::ast::AnnotationBlockDecl) -> Result<AnnotationSpec, SpecError> {
        let props = collect_object_properties_from_items(
            ann.properties.iter().map(|p| &p.node),
            &self.scope,
        )?;
        let sort_order = get_enum_opt(&props, "sort_order", parse_sort_order)?;
        let sort_location = get_enum_opt(&props, "sort_location", parse_sort_location)?;

        let mut match_parameters = Vec::new();
        for mp in &ann.match_parameters {
            let obj_map = eval_object_to_map(&mp.node.body.node, &self.scope)?;
            let mut str_props = IndexMap::new();
            for (k, v) in &obj_map {
                str_props.insert(k.clone(), v.display());
            }
            match_parameters.push(AnnotationMatchParamSpec {
                index: mp.node.index.node,
                properties: str_props,
            });
        }

        Ok(AnnotationSpec {
            sort_order,
            sort_location,
            match_parameters,
        })
    }

    fn compile_erc_matrix_entry(&self, entry: &crate::ast::ErcMatrixEntryDecl) -> Result<ErcMatrixOverride, SpecError> {
        let row = parse_connection_code(&entry.row.node).ok_or_else(|| {
            SpecError::new(
                SpecErrorCode::TypeMismatch,
                format!("unknown ERC connection code: '{}'", entry.row.node),
                Some(entry.row.span),
            )
        })?;
        let col = parse_connection_code(&entry.col.node).ok_or_else(|| {
            SpecError::new(
                SpecErrorCode::TypeMismatch,
                format!("unknown ERC connection code: '{}'", entry.col.node),
                Some(entry.col.span),
            )
        })?;
        let level = parse_error_level(&entry.level.node).ok_or_else(|| {
            SpecError::new(
                SpecErrorCode::TypeMismatch,
                format!("unknown error level: '{}' (expected no_report, warning, error, fatal)", entry.level.node),
                Some(entry.level.span),
            )
        })?;
        Ok(ErcMatrixOverride { row, col, level })
    }

    fn compile_erc_level_entry(&self, entry: &crate::ast::ErcLevelEntryDecl) -> Result<ErcLevelOverride, SpecError> {
        let level_val = eval_expr(&entry.level, &self.scope)?;
        let level_str = match &level_val {
            Value::String(s) => s.clone(),
            Value::Integer(n) => {
                return Ok(ErcLevelOverride {
                    name: entry.name.node.clone(),
                    level: ErrorLevel::try_from(*n).map_err(|_| {
                        SpecError::new(
                            SpecErrorCode::TypeMismatch,
                            format!("invalid error level integer: {n}"),
                            Some(entry.level.span),
                        )
                    })?,
                });
            }
            other => {
                return Err(SpecError::new(
                    SpecErrorCode::TypeMismatch,
                    format!("expected error level string or integer, got {}", other.kind_name()),
                    Some(entry.level.span),
                ));
            }
        };
        let level = parse_error_level(&level_str).ok_or_else(|| {
            SpecError::new(
                SpecErrorCode::TypeMismatch,
                format!("unknown error level: '{}' (expected no_report, warning, error, fatal)", level_str),
                Some(entry.level.span),
            )
        })?;
        Ok(ErcLevelOverride { name: entry.name.node.clone(), level })
    }

    fn compile_output_group(&mut self, group: &crate::ast::OutputGroupBlockDecl) -> Result<OutputGroupSpec, SpecError> {
        let name = group.name.node.as_str();
        let props = collect_object_properties_from_items(
            group.properties.iter().map(|p| &p.node),
            &self.scope,
        )?;
        let description = get_string_opt(&props, "description");

        let mut outputs = Vec::new();
        for out in &group.outputs {
            let out_props = collect_object_properties_from_items(
                out.node.body.iter().map(|p| &p.node),
                &self.scope,
            )?;
            outputs.push(OutputSpec {
                name: out.node.name.node.as_str(),
                output_type: get_string_opt(&out_props, "output_type"),
                document_path: get_string_opt(&out_props, "document_path"),
                variant_name: get_string_opt(&out_props, "variant_name"),
            });
        }

        Ok(OutputGroupSpec { name, description, outputs })
    }

    fn compile_comparison_rule(&self, rule: &crate::ast::ComparisonRuleDecl) -> Result<ComparisonRuleSpec, SpecError> {
        let kind = rule.kind.node.as_str();
        let obj_map = eval_object_to_map(&rule.body.node, &self.scope)?;
        let mut properties = IndexMap::new();
        for (k, v) in &obj_map {
            properties.insert(k.clone(), v.display());
        }
        Ok(ComparisonRuleSpec { kind, properties })
    }

    fn compile_variant(&mut self, var: &crate::ast::VariantBlockDecl) -> Result<VariantSpec, SpecError> {
        let name = var.name.node.as_str();
        let props = collect_object_properties_from_items(
            var.properties.iter().map(|p| &p.node),
            &self.scope,
        )?;
        let description = get_string_opt(&props, "description");

        let mut variations = Vec::new();
        for v in &var.variations {
            let v_map = eval_object_to_map(&v.node.body.node, &self.scope)?;
            let kind = get_enum_opt(&v_map, "kind", parse_variation_kind)?;
            let alternate_part = get_string_opt(&v_map, "alternate_part");
            variations.push(VariationSpec {
                designator: v.node.designator.node.as_str(),
                kind,
                alternate_part,
            });
        }

        let mut param_variations = Vec::new();
        for pv in &var.param_variations {
            let pv_map = eval_object_to_map(&pv.node.body.node, &self.scope)?;
            let parameter = get_string_opt(&pv_map, "parameter")
                .unwrap_or_default();
            let value = get_string_opt(&pv_map, "value")
                .unwrap_or_default();
            param_variations.push(ParamVariationSpec {
                designator: pv.node.designator.node.as_str(),
                parameter,
                value,
            });
        }

        Ok(VariantSpec { name, description, variations, param_variations })
    }
}

// ── Project enum parsers ─────────────────────────────────────────────────────

fn parse_flatten_mode(s: &str) -> Option<FlattenMode> {
    match s {
        "smart" => Some(FlattenMode::Smart),
        "flat" => Some(FlattenMode::Flat),
        "hierarchical_global_ports" => Some(FlattenMode::HierarchicalGlobalPorts),
        "global" => Some(FlattenMode::Global),
        "hierarchical_strict" => Some(FlattenMode::HierarchicalStrict),
        _ => None,
    }
}

fn parse_channel_room_naming_style(s: &str) -> Option<ChannelRoomNamingStyle> {
    match s {
        "flat_numeric_with_names" => Some(ChannelRoomNamingStyle::FlatNumericWithNames),
        "flat_numeric" => Some(ChannelRoomNamingStyle::FlatNumeric),
        "fully_qualified" => Some(ChannelRoomNamingStyle::FullyQualified),
        "fully_qualified_short" => Some(ChannelRoomNamingStyle::FullyQualifiedShort),
        "mixed_name_path" => Some(ChannelRoomNamingStyle::MixedNamePath),
        _ => None,
    }
}

fn parse_cross_ref_sheet_style(s: &str) -> Option<CrossRefSheetStyle> {
    match s {
        "none" => Some(CrossRefSheetStyle::None),
        "name" => Some(CrossRefSheetStyle::Name),
        "number" => Some(CrossRefSheetStyle::Number),
        _ => None,
    }
}

fn parse_cross_ref_location_style(s: &str) -> Option<CrossRefLocationStyle> {
    match s {
        "none" => Some(CrossRefLocationStyle::None),
        "zone" => Some(CrossRefLocationStyle::Zone),
        "xy" => Some(CrossRefLocationStyle::XY),
        _ => None,
    }
}

fn parse_cross_ref_ports(s: &str) -> Option<CrossRefPorts> {
    match s {
        "disabled" => Some(CrossRefPorts::Disabled),
        "sheet_entry" => Some(CrossRefPorts::SheetEntry),
        "ports" => Some(CrossRefPorts::Ports),
        "sheet_entry_and_ports" => Some(CrossRefPorts::SheetEntryAndPorts),
        _ => None,
    }
}

fn parse_sort_order(s: &str) -> Option<SortOrder> {
    match s {
        "up_then_across" => Some(SortOrder::UpThenAcross),
        "down_then_across" => Some(SortOrder::DownThenAcross),
        "across_then_up" => Some(SortOrder::AcrossThenUp),
        "across_then_down" => Some(SortOrder::AcrossThenDown),
        _ => None,
    }
}

fn parse_sort_location(s: &str) -> Option<SortLocation> {
    match s {
        "designator" => Some(SortLocation::Designator),
        "part" => Some(SortLocation::Part),
        _ => None,
    }
}

fn parse_error_level(s: &str) -> Option<ErrorLevel> {
    match s {
        "no_report" => Some(ErrorLevel::NoReport),
        "warning" => Some(ErrorLevel::Warning),
        "error" => Some(ErrorLevel::Error),
        "fatal" => Some(ErrorLevel::Fatal),
        _ => None,
    }
}

fn parse_connection_code(s: &str) -> Option<ConnectionCode> {
    match s {
        "pin_input" => Some(ConnectionCode::PinInput),
        "pin_bidirectional" => Some(ConnectionCode::PinBidirectional),
        "pin_output" => Some(ConnectionCode::PinOutput),
        "pin_open_collector" => Some(ConnectionCode::PinOpenCollector),
        "pin_passive" => Some(ConnectionCode::PinPassive),
        "pin_hi_z" => Some(ConnectionCode::PinHiZ),
        "pin_open_emitter" => Some(ConnectionCode::PinOpenEmitter),
        "pin_power" => Some(ConnectionCode::PinPower),
        "sheet_entry_input" => Some(ConnectionCode::SheetEntryInput),
        "sheet_entry_bidirectional" => Some(ConnectionCode::SheetEntryBidirectional),
        "sheet_entry_output" => Some(ConnectionCode::SheetEntryOutput),
        "port_unspecified" => Some(ConnectionCode::PortUnspecified),
        "pin_unspecified" => Some(ConnectionCode::PinUnspecified),
        "sheet_entry_unspecified" => Some(ConnectionCode::SheetEntryUnspecified),
        "port_input" => Some(ConnectionCode::PortInput),
        "port_output" => Some(ConnectionCode::PortOutput),
        "unconnected" => Some(ConnectionCode::Unconnected),
        _ => None,
    }
}

fn parse_variation_kind(s: &str) -> Option<VariationKind> {
    match s {
        "none" => Some(VariationKind::None),
        "not_fitted" => Some(VariationKind::NotFitted),
        "alternate" => Some(VariationKind::Alternate),
        _ => None,
    }
}

// ── Let binding evaluation ────────────────────────────────────────────────────

/// Evaluate a slice of `(name, expr)` let bindings in forward-visible order.
/// Pushes names as cycle sentinels before evaluating each.
fn eval_let_bindings_slice(
    bindings: &[(&str, &Spanned<crate::ast::Expr>)],
    scope: &mut ScopeStack,
) -> Result<(), SpecError> {
    for (name, expr) in bindings {
        scope.mark_evaluating(name);
        let value = eval_expr(expr, scope)?;
        scope.define(name.to_string(), value);
    }
    Ok(())
}

// ── Object evaluation helpers ─────────────────────────────────────────────────

/// Evaluate an [`Object`] AST node into an `IndexMap<String, Value>`.
fn eval_object_to_map(
    obj: &Object,
    scope: &ScopeStack,
) -> EvalResult<IndexMap<String, Value>> {
    let mut result: IndexMap<String, Value> = IndexMap::new();
    for item in &obj.items {
        match &item.node {
            ObjectItem::LetBinding(_) => {
                // Let bindings inside entity body objects contribute to scope,
                // not to the property map. Skip here — handled at entity level.
            }
            ObjectItem::Spread(spread_expr) => {
                let spread_val = eval_expr(spread_expr, scope)?;
                let spread_map = spread_val.into_object(Some(spread_expr.span))?;
                for (k, v) in spread_map {
                    result.insert(k, v);
                }
            }
            ObjectItem::Property(prop) => {
                let val = eval_expr(&prop.value, scope)?;
                result.insert(prop.key.node.clone(), val);
            }
        }
    }
    Ok(result)
}

/// Like `eval_object_to_map` but skips properties whose keys are consumed by
/// the raw-AST anchor extraction helpers (`on`, `after`, `before`). Those
/// properties reference bindings (`$body`, `$p1`) that are not present in the
/// expression scope, so evaluating them would produce spurious errors.
fn eval_object_to_map_skip_anchor_keys(
    obj: &Object,
    scope: &ScopeStack,
) -> EvalResult<IndexMap<String, Value>> {
    const SKIP: &[&str] = &["on", "after", "before"];
    let mut result: IndexMap<String, Value> = IndexMap::new();
    for item in &obj.items {
        match &item.node {
            ObjectItem::LetBinding(_) => {}
            ObjectItem::Spread(spread_expr) => {
                let spread_val = eval_expr(spread_expr, scope)?;
                let spread_map = spread_val.into_object(Some(spread_expr.span))?;
                for (k, v) in spread_map {
                    if !SKIP.contains(&k.as_str()) {
                        result.insert(k, v);
                    }
                }
            }
            ObjectItem::Property(prop) => {
                if !SKIP.contains(&prop.key.node.as_str()) {
                    let val = eval_expr(&prop.value, scope)?;
                    result.insert(prop.key.node.clone(), val);
                }
            }
        }
    }
    Ok(result)
}

/// Collect properties from an iterator of [`crate::ast::Property`] items
/// by evaluating each value in the given scope.
fn collect_object_properties_from_items<'a>(
    props: impl Iterator<Item = &'a crate::ast::Property>,
    scope: &ScopeStack,
) -> EvalResult<IndexMap<String, Value>> {
    let mut result = IndexMap::new();
    for prop in props {
        let val = eval_expr(&prop.value, scope)?;
        result.insert(prop.key.node.clone(), val);
    }
    Ok(result)
}

// ── Property extraction helpers ───────────────────────────────────────────────

fn get_string_opt(props: &IndexMap<String, Value>, key: &str) -> Option<String> {
    props.get(key).and_then(|v| match v {
        Value::String(s) => Some(s.clone()),
        Value::Integer(n) => Some(n.to_string()),
        _ => None,
    })
}

fn get_bool_opt(props: &IndexMap<String, Value>, key: &str) -> Option<bool> {
    props.get(key).and_then(|v| match v {
        Value::Bool(b) => Some(*b),
        _ => None,
    })
}

fn get_integer_opt(props: &IndexMap<String, Value>, key: &str) -> Option<i32> {
    props.get(key).and_then(|v| match v {
        Value::Integer(n) => Some(*n),
        _ => None,
    })
}

fn get_float_opt(props: &IndexMap<String, Value>, key: &str) -> Option<f64> {
    props.get(key).and_then(|v| match v {
        Value::Float(f) => Some(*f),
        Value::Integer(n) => Some(*n as f64),
        _ => None,
    })
}

fn get_coord_opt(
    props: &IndexMap<String, Value>,
    key: &str,
) -> Result<Option<Coord>, SpecError> {
    match props.get(key) {
        None => Ok(None),
        Some(v) => {
            let raw = v.to_dim(None)?;
            Ok(Some(Coord::new(raw)))
        }
    }
}

fn get_enum_opt<T, F>(
    props: &IndexMap<String, Value>,
    key: &str,
    parse: F,
) -> Result<Option<T>, SpecError>
where
    F: Fn(&str) -> Option<T>,
{
    match props.get(key) {
        None => Ok(None),
        Some(Value::String(s)) => {
            parse(s.as_str()).map(Some).ok_or_else(|| SpecError::no_span(
                SpecErrorCode::TypeMismatch,
                format!("invalid enum value '{}' for key '{key}'", s),
            ))
        }
        Some(Value::Integer(n)) => {
            parse(&n.to_string()).map(Some).ok_or_else(|| SpecError::no_span(
                SpecErrorCode::TypeMismatch,
                format!("invalid enum integer {n} for key '{key}'"),
            ))
        }
        Some(other) => Err(SpecError::no_span(
            SpecErrorCode::TypeMismatch,
            format!("expected string for enum key '{key}', got {}", other.kind_name()),
        )),
    }
}

fn get_string_value_key(
    props: &IndexMap<String, Value>,
    key: &str,
    span: crate::diagnostic::Span,
) -> Result<String, SpecError> {
    match props.get(key) {
        Some(Value::String(s)) => Ok(s.clone()),
        Some(Value::Integer(n)) => Ok(n.to_string()),
        Some(other) => Err(SpecError::at(
            SpecErrorCode::TypeMismatch,
            format!("expected string/integer for '{key}', got {}", other.kind_name()),
            span,
        )),
        None => Err(SpecError::at(
            SpecErrorCode::TypeMismatch,
            format!("missing required key '{key}'"),
            span,
        )),
    }
}

// ── Coordinate conversion helpers ─────────────────────────────────────────────

fn value_to_coord(v: &Value, span: Option<crate::diagnostic::Span>) -> Result<Coord, SpecError> {
    Ok(Coord::new(v.to_dim(span)?))
}

fn value_to_coord_point(
    v: &Value,
    span: Option<crate::diagnostic::Span>,
) -> Result<CoordPoint, SpecError> {
    match v {
        Value::CoordPoint(x, y) => Ok(CoordPoint::new(Coord::new(*x), Coord::new(*y))),
        other => Err(SpecError::new(
            SpecErrorCode::TypeMismatch,
            format!("expected coord point (x, y), got {}", other.kind_name()),
            span,
        )),
    }
}

fn value_to_color(v: &Value, span: Option<crate::diagnostic::Span>) -> Result<Color, SpecError> {
    match v {
        Value::Color(r, g, b) => Ok(Color::from_rgb(*r, *g, *b)),
        other => Err(SpecError::new(
            SpecErrorCode::TypeMismatch,
            format!("expected color, got {}", other.kind_name()),
            span,
        )),
    }
}

fn value_to_points(
    v: &Value,
    span: Option<crate::diagnostic::Span>,
) -> Result<Vec<CoordPoint>, SpecError> {
    match v {
        Value::Array(arr) => {
            let mut pts = Vec::with_capacity(arr.len());
            for item in arr {
                pts.push(value_to_coord_point(item, span)?);
            }
            Ok(pts)
        }
        other => Err(SpecError::new(
            SpecErrorCode::TypeMismatch,
            format!("expected array of points, got {}", other.kind_name()),
            span,
        )),
    }
}

// ── SchDoc-specific helpers ─────────────────────────────────────────────────────

fn value_to_bool(v: &Value, span: Option<crate::diagnostic::Span>) -> Result<bool, SpecError> {
    match v {
        Value::Bool(b) => Ok(*b),
        other => Err(SpecError::new(
            SpecErrorCode::TypeMismatch,
            format!("expected bool, got {}", other.kind_name()),
            span,
        )),
    }
}

fn get_color_opt(props: &IndexMap<String, Value>, key: &str) -> Option<Color> {
    match props.get(key) {
        Some(Value::Color(r, g, b)) => Some(Color::from_rgb(*r, *g, *b)),
        _ => None,
    }
}

fn get_coord_point_required(
    props: &IndexMap<String, Value>,
    key: &str,
) -> Result<CoordPoint, SpecError> {
    match props.get(key) {
        Some(v) => value_to_coord_point(v, None),
        None => Err(SpecError::no_span(
            SpecErrorCode::TypeMismatch,
            format!("missing required field '{}'", key),
        )),
    }
}

fn get_coord_point_array(
    props: &IndexMap<String, Value>,
    key: &str,
) -> Result<Vec<CoordPoint>, SpecError> {
    match props.get(key) {
        Some(v) => value_to_points(v, None),
        None => Err(SpecError::no_span(
            SpecErrorCode::TypeMismatch,
            format!("missing required field '{}'", key),
        )),
    }
}

// ── Enum parsers ───────────────────────────────────────────────────────────────

fn parse_component_kind(s: &str) -> Option<ComponentKind> {
    match s.to_ascii_lowercase().as_str() {
        "standard" => Some(ComponentKind::Standard),
        "mechanical" => Some(ComponentKind::Mechanical),
        "graphical" => Some(ComponentKind::Graphical),
        "net_tie_bom" | "nettie_bom" | "nettiebom" => Some(ComponentKind::NetTieBom),
        "net_tie_no_bom" | "nettie_no_bom" | "nettienobom" => Some(ComponentKind::NetTieNoBom),
        "standard_no_bom" | "standardnobom" => Some(ComponentKind::StandardNoBom),
        "jumper" => Some(ComponentKind::Jumper),
        _ => None,
    }
}

fn parse_pin_electrical_type(s: &str) -> Option<PinElectricalType> {
    match s.to_ascii_lowercase().as_str() {
        "input" => Some(PinElectricalType::Input),
        "input_output" | "inputoutput" | "io" | "bidir" => Some(PinElectricalType::InputOutput),
        "output" => Some(PinElectricalType::Output),
        "open_collector" | "opencollector" | "oc" => Some(PinElectricalType::OpenCollector),
        "passive" => Some(PinElectricalType::Passive),
        "hiz" | "hi_z" | "tristate" => Some(PinElectricalType::HiZ),
        "open_emitter" | "openemitter" | "oe" => Some(PinElectricalType::OpenEmitter),
        "power" => Some(PinElectricalType::Power),
        _ => None,
    }
}

fn parse_rotation_by90(s: &str) -> Option<RotationBy90> {
    match s.to_ascii_lowercase().as_str() {
        "0" | "rotate0" | "right" | "east" => Some(RotationBy90::Rotate0),
        "90" | "rotate90" | "up" | "north" => Some(RotationBy90::Rotate90),
        "180" | "rotate180" | "left" | "west" => Some(RotationBy90::Rotate180),
        "270" | "rotate270" | "down" | "south" => Some(RotationBy90::Rotate270),
        _ => None,
    }
}

fn parse_pad_shape(s: &str) -> Option<PadShape> {
    match s.to_ascii_lowercase().as_str() {
        "no_shape" | "none" => Some(PadShape::NoShape),
        "round" | "circle" => Some(PadShape::Round),
        "rectangular" | "rect" | "square" => Some(PadShape::Rectangular),
        "octagonal" | "octagon" => Some(PadShape::Octagonal),
        "arc" => Some(PadShape::Arc),
        "terminator" => Some(PadShape::Terminator),
        "round_rect" | "roundrect" => Some(PadShape::RoundRect),
        "rotated_rect" | "rotatedrect" => Some(PadShape::RotatedRect),
        "rounded_rectangular" | "roundedrectangular" => Some(PadShape::RoundedRectangular),
        "custom" => Some(PadShape::Custom),
        _ => None,
    }
}

fn parse_pad_stack_mode(s: &str) -> Option<PadStackMode> {
    match s.to_ascii_lowercase().as_str() {
        "simple" => Some(PadStackMode::Simple),
        "local_stack" | "localstack" => Some(PadStackMode::LocalStack),
        "external_stack" | "externalstack" => Some(PadStackMode::ExternalStack),
        _ => None,
    }
}

fn parse_plane_connection(s: &str) -> Option<PlaneConnectionStyle> {
    match s.to_ascii_lowercase().as_str() {
        "no_connect" | "noconnect" | "none" => Some(PlaneConnectionStyle::NoConnect),
        "relief" => Some(PlaneConnectionStyle::Relief),
        "direct" => Some(PlaneConnectionStyle::Direct),
        _ => None,
    }
}

fn parse_power_object_style(s: &str) -> Option<PowerObjectStyle> {
    match s.to_ascii_lowercase().as_str() {
        "circle" | "0" => Some(PowerObjectStyle::Circle),
        "arrow" | "1" => Some(PowerObjectStyle::Arrow),
        "bar" | "2" => Some(PowerObjectStyle::Bar),
        "wave" | "3" => Some(PowerObjectStyle::Wave),
        "gnd_power" | "gndpower" | "4" => Some(PowerObjectStyle::GndPower),
        "gnd_signal" | "gndsignal" | "5" => Some(PowerObjectStyle::GndSignal),
        "gnd_earth" | "gndearth" | "6" => Some(PowerObjectStyle::GndEarth),
        "gost_arrow" | "gostarrow" | "7" => Some(PowerObjectStyle::GostArrow),
        "gost_gnd_power" | "gostgndpower" | "8" => Some(PowerObjectStyle::GostGndPower),
        "gost_gnd_earth" | "gostgndearth" | "9" => Some(PowerObjectStyle::GostGndEarth),
        "gost_bar" | "gostbar" | "10" => Some(PowerObjectStyle::GostBar),
        _ => None,
    }
}

fn parse_pen_width(s: &str) -> Option<PenWidth> {
    match s.to_ascii_lowercase().as_str() {
        "zero" | "0" => Some(PenWidth::Zero),
        "small" | "1" => Some(PenWidth::Small),
        "medium" | "2" => Some(PenWidth::Medium),
        "large" | "3" => Some(PenWidth::Large),
        _ => None,
    }
}

fn parse_line_style(s: &str) -> Option<LineStyle> {
    match s.to_ascii_lowercase().as_str() {
        "solid" | "0" => Some(LineStyle::Solid),
        "dashed" | "1" => Some(LineStyle::Dashed),
        "dotted" | "2" => Some(LineStyle::Dotted),
        "dash_dotted" | "dashdotted" | "3" => Some(LineStyle::DashDotted),
        _ => None,
    }
}

fn parse_text_justification(s: &str) -> Option<TextJustification> {
    match s.to_ascii_lowercase().as_str() {
        "bottom_left" | "bottomleft" | "0" => Some(TextJustification::BottomLeft),
        "bottom_center" | "bottomcenter" | "1" => Some(TextJustification::BottomCenter),
        "bottom_right" | "bottomright" | "2" => Some(TextJustification::BottomRight),
        "center_left" | "centerleft" | "3" => Some(TextJustification::CenterLeft),
        "center" | "4" => Some(TextJustification::Center),
        "center_right" | "centerright" | "5" => Some(TextJustification::CenterRight),
        "top_left" | "topleft" | "6" => Some(TextJustification::TopLeft),
        "top_center" | "topcenter" | "7" => Some(TextJustification::TopCenter),
        "top_right" | "topright" | "8" => Some(TextJustification::TopRight),
        _ => None,
    }
}

fn parse_port_io_type(s: &str) -> Option<PortIoType> {
    match s.to_ascii_lowercase().as_str() {
        "unspecified" | "0" => Some(PortIoType::Unspecified),
        "output" | "1" => Some(PortIoType::Output),
        "input" | "2" => Some(PortIoType::Input),
        "bidirectional" | "bidi" | "3" => Some(PortIoType::Bidirectional),
        _ => None,
    }
}

fn parse_port_arrow_style(s: &str) -> Option<PortArrowStyle> {
    match s.to_ascii_lowercase().as_str() {
        "none" | "0" => Some(PortArrowStyle::None),
        "left" | "1" => Some(PortArrowStyle::Left),
        "right" | "2" => Some(PortArrowStyle::Right),
        "left_right" | "leftright" | "3" => Some(PortArrowStyle::LeftRight),
        "none_vertical" | "nonevertical" | "4" => Some(PortArrowStyle::NoneVertical),
        "top" | "5" => Some(PortArrowStyle::Top),
        "bottom" | "6" => Some(PortArrowStyle::Bottom),
        "top_bottom" | "topbottom" | "7" => Some(PortArrowStyle::TopBottom),
        _ => None,
    }
}

fn parse_horizontal_align(s: &str) -> Option<HorizontalAlign> {
    match s.to_ascii_lowercase().as_str() {
        "center" | "0" => Some(HorizontalAlign::Center),
        "left" | "1" => Some(HorizontalAlign::Left),
        "right" | "2" => Some(HorizontalAlign::Right),
        _ => None,
    }
}

fn parse_left_right_side(s: &str) -> Option<LeftRightSide> {
    match s.to_ascii_lowercase().as_str() {
        "left" | "0" => Some(LeftRightSide::Left),
        "right" | "1" => Some(LeftRightSide::Right),
        "top" | "2" => Some(LeftRightSide::Top),
        "bottom" | "3" => Some(LeftRightSide::Bottom),
        _ => None,
    }
}

fn parse_layer_spec(s: &str) -> Option<LayerSpec> {
    // Try copper(N) syntax
    if let Some(n) = parse_copper_position(s) {
        return Some(LayerSpec::CopperPosition(n));
    }
    // Try V6 canonical name
    if let Some(lr) = LayerRef::from_string_name(s) {
        return Some(LayerSpec::Resolved(lr));
    }
    // Treat as custom stack name (deferred resolution)
    Some(LayerSpec::NamedLayer(s.to_owned()))
}

fn parse_copper_position(s: &str) -> Option<usize> {
    let s = s.trim();
    if s.starts_with("copper(") && s.ends_with(')') {
        s[7..s.len() - 1].trim().parse().ok()
    } else {
        None
    }
}

fn parse_sch_graphic_type(s: &str) -> Option<GraphicType> {
    match s {
        "line" => Some(GraphicType::Line),
        "rectangle" => Some(GraphicType::Rectangle),
        "arc" => Some(GraphicType::Arc),
        "elliptical_arc" => Some(GraphicType::EllipticalArc),
        "ellipse" => Some(GraphicType::Ellipse),
        "polyline" => Some(GraphicType::Polyline),
        "polygon" => Some(GraphicType::Polygon),
        "bezier" => Some(GraphicType::Bezier),
        "pie" => Some(GraphicType::Pie),
        "round_rectangle" => Some(GraphicType::RoundRectangle),
        "label" => Some(GraphicType::Label),
        "text_frame" => Some(GraphicType::TextFrame),
        "image" => Some(GraphicType::Image),
        _ => None,
    }
}

fn parse_pcb_graphic_type(s: &str) -> Option<PcbGraphicType> {
    match s {
        "track" => Some(PcbGraphicType::Track),
        "arc" => Some(PcbGraphicType::Arc),
        "fill" => Some(PcbGraphicType::Fill),
        "region" => Some(PcbGraphicType::Region),
        "text" => Some(PcbGraphicType::Text),
        "via" => Some(PcbGraphicType::Via),
        "component_body" => Some(PcbGraphicType::ComponentBody),
        "polyline" | "line" => Some(PcbGraphicType::Polyline),
        _ => None,
    }
}

// ── Graphic property compilation ──────────────────────────────────────────────

fn compile_graphic_properties(
    props: &IndexMap<String, Value>,
    span: crate::diagnostic::Span,
) -> Result<GraphicProperties, SpecError> {
    let from = props.get("from")
        .map(|v| value_to_coord_point(v, Some(span)))
        .transpose()?;
    let to = props.get("to")
        .map(|v| value_to_coord_point(v, Some(span)))
        .transpose()?;
    let center = props.get("center")
        .map(|v| value_to_coord_point(v, Some(span)))
        .transpose()?;
    let at = props.get("at")
        .map(|v| value_to_coord_point(v, Some(span)))
        .transpose()?;

    let radius = props.get("radius").map(|v| value_to_coord(v, Some(span))).transpose()?;
    let secondary_radius = props.get("secondary_radius").map(|v| value_to_coord(v, Some(span))).transpose()?;
    let line_width = props.get("line_width").map(|v| value_to_coord(v, Some(span))).transpose()?;
    let width = props.get("width").map(|v| value_to_coord(v, Some(span))).transpose()?;
    let corner_x_radius = props.get("corner_x_radius").map(|v| value_to_coord(v, Some(span))).transpose()?;
    let corner_y_radius = props.get("corner_y_radius").map(|v| value_to_coord(v, Some(span))).transpose()?;

    let start_angle = get_float_opt(props, "start_angle");
    let end_angle = get_float_opt(props, "end_angle");
    let is_solid = get_bool_opt(props, "is_solid");
    let closed = get_bool_opt(props, "closed");
    let show_border = get_bool_opt(props, "show_border");
    let font_id = get_integer_opt(props, "font_id");
    let text = get_string_opt(props, "text");
    let file_name = get_string_opt(props, "file_name");

    let color = props.get("color")
        .map(|v| value_to_color(v, Some(span)))
        .transpose()?;
    let area_color = props.get("area_color")
        .map(|v| value_to_color(v, Some(span)))
        .transpose()?;

    let points = props.get("points")
        .map(|v| value_to_points(v, Some(span)))
        .transpose()?;

    let layer = get_enum_opt(props, "layer", parse_layer_spec)?;

    // image_data: accept base64 string or null
    let image_data: Option<Vec<u8>> = if let Some(Value::String(s)) = props.get("image_data") {
        let decoded = base64_decode_simple(s);
        Some(decoded)
    } else {
        None
    };

    Ok(GraphicProperties {
        from,
        to,
        is_solid,
        corner_x_radius,
        corner_y_radius,
        center,
        radius,
        secondary_radius,
        start_angle,
        end_angle,
        points,
        color,
        area_color,
        line_width,
        text,
        font_id,
        at,
        file_name,
        image_data,
        layer,
        width,
        closed,
        show_border,
    })
}

fn compile_pcb_graphic_properties(
    props: &IndexMap<String, Value>,
    span: crate::diagnostic::Span,
) -> Result<PcbGraphicProperties, SpecError> {
    let layer = get_enum_opt(props, "layer", parse_layer_spec)?;
    let width = props.get("width").map(|v| value_to_coord(v, Some(span))).transpose()?;
    let from = props.get("from").map(|v| value_to_coord_point(v, Some(span))).transpose()?;
    let to = props.get("to").map(|v| value_to_coord_point(v, Some(span))).transpose()?;
    let center = props.get("center").map(|v| value_to_coord_point(v, Some(span))).transpose()?;
    let at = props.get("at").map(|v| value_to_coord_point(v, Some(span))).transpose()?;

    let radius = props.get("radius").map(|v| value_to_coord(v, Some(span))).transpose()?;
    let hole_size = props.get("hole_size").map(|v| value_to_coord(v, Some(span))).transpose()?;
    let diameter = props.get("diameter").map(|v| value_to_coord(v, Some(span))).transpose()?;

    let start_angle = get_float_opt(props, "start_angle");
    let end_angle = get_float_opt(props, "end_angle");
    let rotation = get_float_opt(props, "rotation");
    let is_solid = get_bool_opt(props, "is_solid");
    let text = get_string_opt(props, "text");

    let points = props.get("points")
        .map(|v| value_to_points(v, Some(span)))
        .transpose()?;

    Ok(PcbGraphicProperties {
        layer,
        width,
        from,
        to,
        center,
        radius,
        start_angle,
        end_angle,
        points,
        text,
        at,
        rotation,
        hole_size,
        diameter,
        is_solid,
    })
}

/// Minimal base64 decoder (alphabet A-Z a-z 0-9 + /).
/// Only handles standard base64 without whitespace.
fn base64_decode_simple(input: &str) -> Vec<u8> {
    let table: [u8; 128] = {
        let mut t = [255u8; 128];
        for (i, c) in b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
            .iter()
            .enumerate()
        {
            t[*c as usize] = i as u8;
        }
        t
    };
    let input = input.trim_end_matches('=');
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let bytes: Vec<u8> = input
        .bytes()
        .filter(|&b| (b as usize) < 128 && table[b as usize] != 255)
        .map(|b| table[b as usize])
        .collect();
    for chunk in bytes.chunks(4) {
        match chunk.len() {
            4 => {
                let n = (chunk[0] as u32) << 18
                    | (chunk[1] as u32) << 12
                    | (chunk[2] as u32) << 6
                    | chunk[3] as u32;
                out.push((n >> 16) as u8);
                out.push((n >> 8) as u8);
                out.push(n as u8);
            }
            3 => {
                let n = (chunk[0] as u32) << 18
                    | (chunk[1] as u32) << 12
                    | (chunk[2] as u32) << 6;
                out.push((n >> 16) as u8);
                out.push((n >> 8) as u8);
            }
            2 => {
                let n = (chunk[0] as u32) << 18 | (chunk[1] as u32) << 12;
                out.push((n >> 16) as u8);
            }
            _ => {}
        }
    }
    out
}

// ── Anchor placement types ────────────────────────────────────────────────────

/// Which axis a vertical/horizontal edge is fixed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    /// Edge is a vertical line (fixed X, varying Y). Left/Right edges.
    X,
    /// Edge is a horizontal line (fixed Y, varying X). Top/Bottom edges.
    Y,
}

/// Which side of the bounding box the edge belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeSide {
    Left,
    Right,
    Top,
    Bottom,
}

impl EdgeSide {
    fn forward_direction(self) -> i32 {
        // +1 = increasing coordinate along the edge's varying axis.
        match self {
            EdgeSide::Left => -1,  // top to bottom: decreasing Y
            EdgeSide::Right => 1,  // bottom to top: increasing Y
            EdgeSide::Top => 1,    // left to right: increasing X
            EdgeSide::Bottom => -1,// right to left: decreasing X
        }
    }

    fn auto_orientation(self) -> RotationBy90 {
        match self {
            EdgeSide::Left => RotationBy90::Rotate0,    // pin points right
            EdgeSide::Right => RotationBy90::Rotate180, // pin points left
            EdgeSide::Top => RotationBy90::Rotate270,   // pin points down
            EdgeSide::Bottom => RotationBy90::Rotate90, // pin points up
        }
    }
}

/// A resolved edge from a bounding box.
#[derive(Debug, Clone)]
pub struct Edge {
    /// Fixed coordinate value (X for left/right, Y for top/bottom).
    pub position: Coord,
    /// Min and max along the varying axis (Y for X-edges, X for Y-edges).
    pub range: (Coord, Coord),
    /// Which side this edge belongs to.
    pub side: EdgeSide,
}

impl Edge {
    pub fn axis(&self) -> Axis {
        match self.side {
            EdgeSide::Left | EdgeSide::Right => Axis::X,
            EdgeSide::Top | EdgeSide::Bottom => Axis::Y,
        }
    }

    /// Compute point ON the edge at `at_position`.
    fn point_at(&self, at_pos: AnchorPosition) -> Coord {
        let (min, max) = self.range;
        let dir = self.side.forward_direction();
        match at_pos {
            AnchorPosition::Start => {
                if dir > 0 { min } else { max }
            }
            AnchorPosition::Center => {
                Coord::new((min.raw() + max.raw()) / 2)
            }
            AnchorPosition::End => {
                if dir > 0 { max } else { min }
            }
        }
    }
}

/// How to position along the edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorPosition {
    Start,
    Center,
    End,
}

/// Side offset direction (relative to the edge).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementSide {
    Inside,
    Outside,
    Center,
}

/// Resolved anchor value.
#[derive(Debug, Clone)]
pub enum AnchorValue {
    Edge(Edge),
    Point(CoordPoint),
}

// ── Graphic binding map ───────────────────────────────────────────────────────

/// A bound box graphic's geometry extracted for anchor resolution.
#[derive(Debug, Clone)]
pub struct BoxGeometry {
    pub from: CoordPoint,
    pub to: CoordPoint,
}

impl BoxGeometry {
    fn left_edge(&self) -> Edge {
        let x = Coord::new(self.from.x.raw().min(self.to.x.raw()));
        let y_min = Coord::new(self.from.y.raw().min(self.to.y.raw()));
        let y_max = Coord::new(self.from.y.raw().max(self.to.y.raw()));
        Edge { position: x, range: (y_min, y_max), side: EdgeSide::Left }
    }
    fn right_edge(&self) -> Edge {
        let x = Coord::new(self.from.x.raw().max(self.to.x.raw()));
        let y_min = Coord::new(self.from.y.raw().min(self.to.y.raw()));
        let y_max = Coord::new(self.from.y.raw().max(self.to.y.raw()));
        Edge { position: x, range: (y_min, y_max), side: EdgeSide::Right }
    }
    fn top_edge(&self) -> Edge {
        let y = Coord::new(self.from.y.raw().max(self.to.y.raw()));
        let x_min = Coord::new(self.from.x.raw().min(self.to.x.raw()));
        let x_max = Coord::new(self.from.x.raw().max(self.to.x.raw()));
        Edge { position: y, range: (x_min, x_max), side: EdgeSide::Top }
    }
    fn bottom_edge(&self) -> Edge {
        let y = Coord::new(self.from.y.raw().min(self.to.y.raw()));
        let x_min = Coord::new(self.from.x.raw().min(self.to.x.raw()));
        let x_max = Coord::new(self.from.x.raw().max(self.to.x.raw()));
        Edge { position: y, range: (x_min, x_max), side: EdgeSide::Bottom }
    }
    fn center(&self) -> CoordPoint {
        CoordPoint::new(
            Coord::new((self.from.x.raw() + self.to.x.raw()) / 2),
            Coord::new((self.from.y.raw() + self.to.y.raw()) / 2),
        )
    }
    fn corner(&self, side: EdgeSide, start: bool) -> CoordPoint {
        // start = "start" of that edge's forward direction
        let edge = match side {
            EdgeSide::Left => self.left_edge(),
            EdgeSide::Right => self.right_edge(),
            EdgeSide::Top => self.top_edge(),
            EdgeSide::Bottom => self.bottom_edge(),
        };
        let varying_coord = edge.point_at(if start { AnchorPosition::Start } else { AnchorPosition::End });
        match edge.axis() {
            Axis::X => CoordPoint::new(edge.position, varying_coord),
            Axis::Y => CoordPoint::new(varying_coord, edge.position),
        }
    }

    /// Resolve a field name like "left", "right", "top", "bottom", "center",
    /// "top_left", etc. to an AnchorValue.
    pub fn resolve_field(&self, field: &str) -> Option<AnchorValue> {
        match field {
            "left" => Some(AnchorValue::Edge(self.left_edge())),
            "right" => Some(AnchorValue::Edge(self.right_edge())),
            "top" => Some(AnchorValue::Edge(self.top_edge())),
            "bottom" => Some(AnchorValue::Edge(self.bottom_edge())),
            "center" => Some(AnchorValue::Point(self.center())),
            "top_left" | "tl" => Some(AnchorValue::Point(self.corner(EdgeSide::Left, true))),
            "top_right" | "tr" => Some(AnchorValue::Point(self.corner(EdgeSide::Right, false))),
            "bottom_left" | "bl" => Some(AnchorValue::Point(self.corner(EdgeSide::Left, false))),
            "bottom_right" | "br" => Some(AnchorValue::Point(self.corner(EdgeSide::Right, true))),
            _ => None,
        }
    }
}

/// Maps binding name → box geometry (for anchor resolution).
pub type GraphicBindingMap = HashMap<String, BoxGeometry>;

/// Build a GraphicBindingMap by scanning bound box-type graphics in the current scope.
fn build_graphic_binding_map<'a>(
    graphic_decls: impl Iterator<Item = &'a crate::ast::GraphicDecl>,
    scope: &ScopeStack,
) -> Result<GraphicBindingMap, SpecError> {
    let mut map = GraphicBindingMap::new();

    for decl in graphic_decls {
        let binding_name = match &decl.binding {
            Some(b) => b.node.clone(),
            None => continue, // unnamed graphics can't be referenced
        };

        // Only box types have edges.
        let is_box = matches!(
            decl.graphic_type.node.as_str(),
            "rectangle" | "round_rectangle" | "text_frame" | "image"
        );
        if !is_box {
            continue;
        }

        let props = eval_object_to_map(&decl.body.node, scope)?;
        let from = match props.get("from") {
            Some(v) => value_to_coord_point(v, Some(decl.body.span))?,
            None => continue, // can't compute edges without geometry
        };
        let to = match props.get("to") {
            Some(v) => value_to_coord_point(v, Some(decl.body.span))?,
            None => continue,
        };

        map.insert(binding_name, BoxGeometry { from, to });
    }

    Ok(map)
}

// ── Anchor expression parsing ─────────────────────────────────────────────────

/// Parse a raw object's `on:` property as a `(binding_name, edge_field)` pair.
/// Returns `None` if `on:` is absent.
fn extract_on_ref(obj: &crate::ast::Object) -> Option<(String, String)> {
    for item in &obj.items {
        if let crate::ast::ObjectItem::Property(p) = &item.node {
            if p.key.node == "on" {
                return extract_dollar_path_two_part(&p.value.node);
            }
        }
    }
    None
}

/// Extract `(root, field)` from an expression like `$body.left`.
/// Returns `None` if the expression is not a `DollarIdent.field` path.
fn extract_dollar_path_two_part(expr: &crate::ast::Expr) -> Option<(String, String)> {
    if let crate::ast::Expr::Path(base, field) = expr {
        if let crate::ast::Expr::DollarIdent(root) = &base.node {
            return Some((root.clone(), field.node.clone()));
        }
    }
    None
}

/// Parse a raw object's `after:` or `before:` property as a binding name.
fn extract_sequence_ref(obj: &crate::ast::Object, key: &str) -> Option<String> {
    for item in &obj.items {
        if let crate::ast::ObjectItem::Property(p) = &item.node {
            if p.key.node == key {
                if let crate::ast::Expr::DollarIdent(name) = &p.value.node {
                    return Some(name.clone());
                }
            }
        }
    }
    None
}

/// Parse an `at:` string enum from the raw object (for anchor mode: "start", "center", "end").
fn extract_at_position(obj: &crate::ast::Object, scope: &ScopeStack) -> Result<Option<AnchorPosition>, SpecError> {
    for item in &obj.items {
        if let crate::ast::ObjectItem::Property(p) = &item.node {
            if p.key.node == "at" {
                let val = eval_expr(&p.value, scope)?;
                match &val {
                    Value::String(s) => {
                        return Ok(Some(parse_anchor_position(s).ok_or_else(|| {
                            SpecError::no_span(
                                SpecErrorCode::TypeMismatch,
                                format!("invalid anchor position '{}': expected start, center, or end", s),
                            )
                        })?));
                    }
                    Value::CoordPoint(_, _) => {
                        // Absolute coordinate — not anchor mode, let compile_pin handle it.
                        return Ok(None);
                    }
                    other => {
                        return Err(SpecError::no_span(
                            SpecErrorCode::TypeMismatch,
                            format!("at: expected string position or coord point, got {}", other.kind_name()),
                        ));
                    }
                }
            }
        }
    }
    Ok(None)
}

fn parse_anchor_position(s: &str) -> Option<AnchorPosition> {
    match s.to_ascii_lowercase().as_str() {
        "start" => Some(AnchorPosition::Start),
        "center" => Some(AnchorPosition::Center),
        "end" => Some(AnchorPosition::End),
        _ => None,
    }
}

fn parse_placement_side(s: &str) -> Option<PlacementSide> {
    match s.to_ascii_lowercase().as_str() {
        "inside" => Some(PlacementSide::Inside),
        "outside" => Some(PlacementSide::Outside),
        "center" => Some(PlacementSide::Center),
        _ => None,
    }
}

// ── Pin placement resolver ────────────────────────────────────────────────────

/// Intermediate pin data before anchor coordinates are resolved.
struct PendingPin<'a> {
    decl: &'a PinDecl,
    owner_part_id: i32,
    binding_name: Option<String>,
    anchor_mode: PinAnchorMode,
}

enum PinAnchorMode {
    /// Absolute `at:` or no placement — use compile_pin logic.
    Absolute,
    /// `on: $body.edge, at: start|center|end`.
    AtPosition {
        binding: String,
        field: String,
        at_pos: AnchorPosition,
    },
    /// `on: $body.edge, after: $prev_pin, gap: N`.
    After {
        binding: String,
        field: String,
        after_ref: String,
        gap: i32,
    },
    /// `on: $body.edge, before: $next_pin, gap: N`.
    Before {
        binding: String,
        field: String,
        before_ref: String,
        gap: i32,
    },
}

/// Resolve all anchor-based and absolute pins in a pin list.
///
/// Absolute pins are compiled immediately. Anchor-based pins are grouped by
/// (binding, edge) and resolved in topological order within each group.
fn resolve_anchor_pins(
    pin_decls: &[(&PinDecl, i32)],
    binding_map: &GraphicBindingMap,
    scope: &ScopeStack,
) -> Result<Vec<PinSpec>, SpecError> {
    // First pass: classify each pin.
    let mut pending: Vec<PendingPin> = Vec::with_capacity(pin_decls.len());
    for (decl, owner_part_id) in pin_decls {
        let on_ref = extract_on_ref(&decl.body.node);
        let mode = if let Some((binding, field)) = on_ref {
            // Check for after: / before:
            let after = extract_sequence_ref(&decl.body.node, "after");
            let before = extract_sequence_ref(&decl.body.node, "before");
            let at_pos = extract_at_position(&decl.body.node, scope)?;

            // Validate mutual exclusivity.
            let has_at = at_pos.is_some();
            let has_after = after.is_some();
            let has_before = before.is_some();
            let count = has_at as u8 + has_after as u8 + has_before as u8;
            if count > 1 {
                return Err(SpecError::at(
                    SpecErrorCode::TypeMismatch,
                    "at:, after:, and before: are mutually exclusive in anchor mode",
                    decl.body.span,
                ));
            }

            let props_map = eval_object_to_map_skip_anchor_keys(&decl.body.node, scope);
            let gap = if let Ok(ref m) = props_map {
                get_coord_opt(m, "gap")?.map(|c| c.raw()).unwrap_or(0)
            } else {
                0
            };

            if let Some(after_ref) = after {
                PinAnchorMode::After { binding, field, after_ref, gap }
            } else if let Some(before_ref) = before {
                PinAnchorMode::Before { binding, field, before_ref: before_ref, gap }
            } else {
                PinAnchorMode::AtPosition {
                    binding,
                    field,
                    at_pos: at_pos.unwrap_or(AnchorPosition::Center),
                }
            }
        } else {
            PinAnchorMode::Absolute
        };

        let binding_name = decl.binding.as_ref().map(|b| b.node.clone());
        pending.push(PendingPin { decl, owner_part_id: *owner_part_id, binding_name, anchor_mode: mode });
    }

    // Group anchor pins by (binding_name, edge_field) for sequencing.
    // Absolute pins can be compiled immediately.
    // We need to resolve sequenced pins in topo order, then compile all.

    // Map: pin binding name → index in pending
    let binding_index: HashMap<String, usize> = pending.iter().enumerate()
        .filter_map(|(i, p)| p.binding_name.as_ref().map(|b| (b.clone(), i)))
        .collect();

    // Topological sort for after:/before: dependencies.
    let sorted_indices = topo_sort_pins(&pending, &binding_index)?;

    // Build position cache: binding_name -> pin location (both axes).
    // The referencing pin extracts the correct along-edge axis at lookup time.
    let mut position_cache: HashMap<String, CoordPoint> = HashMap::new();
    let mut result_specs: Vec<(usize, PinSpec)> = Vec::with_capacity(pending.len());

    for idx in sorted_indices {
        let p = &pending[idx];
        let pin_spec = compile_one_pin(p, binding_map, scope, &position_cache)?;

        // Store position for after:/before: successors.
        if let Some(ref bn) = p.binding_name {
            position_cache.insert(bn.clone(), pin_spec.location);
        }

        result_specs.push((idx, pin_spec));
    }

    // Return in original declaration order.
    result_specs.sort_by_key(|(i, _)| *i);
    Ok(result_specs.into_iter().map(|(_, s)| s).collect())
}

/// Get an edge from a box geometry by field name.
fn geom_field_to_edge(geom: &BoxGeometry, field: &str) -> Option<Edge> {
    match field {
        "left" => Some(geom.left_edge()),
        "right" => Some(geom.right_edge()),
        "top" => Some(geom.top_edge()),
        "bottom" => Some(geom.bottom_edge()),
        _ => None,
    }
}

/// Topological sort of pins respecting after:/before: dependencies.
fn topo_sort_pins(
    pending: &[PendingPin],
    binding_index: &HashMap<String, usize>,
) -> Result<Vec<usize>, SpecError> {
    let n = pending.len();
    let mut deps: Vec<Vec<usize>> = vec![vec![]; n]; // deps[i] = indices i depends on

    for (i, p) in pending.iter().enumerate() {
        match &p.anchor_mode {
            PinAnchorMode::After { after_ref, binding, field, .. } => {
                if let Some(&dep_idx) = binding_index.get(after_ref) {
                    // Validate same edge.
                    if let Some(dep_edge) = get_dep_edge(&pending[dep_idx], binding) {
                        if !same_edge(field, &dep_edge) {
                            return Err(SpecError::no_span(
                                SpecErrorCode::CrossEdgeReference,
                                format!(
                                    "pin '{}' uses after: ${} but they are on different edges",
                                    pending[i].decl.name.node.as_str(),
                                    after_ref,
                                ),
                            ));
                        }
                    }
                    deps[i].push(dep_idx);
                } else {
                    return Err(SpecError::no_span(
                        SpecErrorCode::UndefinedBinding,
                        format!("after: references undefined binding '${}'", after_ref),
                    ));
                }
            }
            PinAnchorMode::Before { before_ref, binding, field, .. } => {
                if let Some(&dep_idx) = binding_index.get(before_ref) {
                    if let Some(dep_edge) = get_dep_edge(&pending[dep_idx], binding) {
                        if !same_edge(field, &dep_edge) {
                            return Err(SpecError::no_span(
                                SpecErrorCode::CrossEdgeReference,
                                format!(
                                    "pin '{}' uses before: ${} but they are on different edges",
                                    pending[i].decl.name.node.as_str(),
                                    before_ref,
                                ),
                            ));
                        }
                    }
                    // before: A means I depend on A (I need A's position to place before it)
                    deps[i].push(dep_idx);
                } else {
                    return Err(SpecError::no_span(
                        SpecErrorCode::UndefinedBinding,
                        format!("before: references undefined binding '${}'", before_ref),
                    ));
                }
            }
            _ => {}
        }
    }

    // DFS topo sort.
    let mut visited = vec![false; n];
    let mut in_stack = vec![false; n];
    let mut order = Vec::with_capacity(n);

    fn dfs_pins(
        i: usize,
        deps: &Vec<Vec<usize>>,
        visited: &mut Vec<bool>,
        in_stack: &mut Vec<bool>,
        order: &mut Vec<usize>,
    ) -> Result<(), SpecError> {
        if in_stack[i] {
            return Err(SpecError::no_span(
                SpecErrorCode::CircularBinding,
                "circular after:/before: dependency detected",
            ));
        }
        if visited[i] { return Ok(()); }
        in_stack[i] = true;
        for &dep in &deps[i] {
            dfs_pins(dep, deps, visited, in_stack, order)?;
        }
        in_stack[i] = false;
        visited[i] = true;
        order.push(i);
        Ok(())
    }

    for i in 0..n {
        dfs_pins(i, &deps, &mut visited, &mut in_stack, &mut order)?;
    }

    Ok(order)
}

fn get_dep_edge(dep_pin: &PendingPin, _ref_binding: &str) -> Option<String> {
    match &dep_pin.anchor_mode {
        PinAnchorMode::AtPosition { field, .. }
        | PinAnchorMode::After { field, .. }
        | PinAnchorMode::Before { field, .. } => Some(field.clone()),
        PinAnchorMode::Absolute => None,
    }
}

fn same_edge(field_a: &str, field_b: &str) -> bool {
    field_a == field_b
}

/// Compile a single pending pin to a PinSpec.
fn compile_one_pin(
    p: &PendingPin,
    binding_map: &GraphicBindingMap,
    scope: &ScopeStack,
    position_cache: &HashMap<String, CoordPoint>,
) -> Result<PinSpec, SpecError> {
    let decl = p.decl;
    let props = eval_object_to_map_skip_anchor_keys(&decl.body.node, scope)?;

    let name = get_string_opt(&props, "name");
    let electrical = get_enum_opt(&props, "electrical", parse_pin_electrical_type)?;
    let length = get_coord_opt(&props, "length")?;
    let is_hidden = get_bool_opt(&props, "is_hidden");
    let hidden_net_name = get_string_opt(&props, "hidden_net_name");

    match &p.anchor_mode {
        PinAnchorMode::Absolute => {
            let orientation = get_enum_opt(&props, "orientation", parse_rotation_by90)?
                .unwrap_or(RotationBy90::Rotate0);
            let location = if let Some(v) = props.get("at") {
                value_to_coord_point(v, Some(decl.body.span))?
            } else if let Some(x_val) = props.get("x") {
                let x = value_to_coord(x_val, Some(decl.body.span))?;
                let y = props.get("y")
                    .map(|v| value_to_coord(v, Some(decl.body.span)))
                    .transpose()?
                    .unwrap_or(Coord::ZERO);
                CoordPoint::new(x, y)
            } else {
                CoordPoint::zero()
            };
            Ok(PinSpec {
                designator: decl.name.node.as_str(),
                name, electrical, length, location, orientation, is_hidden, hidden_net_name,
                owner_part_id: p.owner_part_id,
            })
        }
        PinAnchorMode::AtPosition { binding, field, at_pos } => {
            let geom = binding_map.get(binding).ok_or_else(|| SpecError::at(
                SpecErrorCode::UndefinedBinding,
                format!("no bound graphic named '${}'", binding),
                decl.body.span,
            ))?;
            let edge = geom_field_to_edge(geom, field).ok_or_else(|| SpecError::at(
                SpecErrorCode::TypeMismatch,
                format!("'{}' is not a valid edge name for anchor placement (use left/right/top/bottom)", field),
                decl.body.span,
            ))?;
            let side = get_enum_opt(&props, "side", parse_placement_side)?
                .unwrap_or(PlacementSide::Outside);
            let gap = get_coord_opt(&props, "gap")?.map(|c| c.raw()).unwrap_or(0);
            let offset = get_coord_point_opt(&props, "offset", decl.body.span)?;
            let pin_length = length.unwrap_or(Coord::from_mils(25));
            let (location, orientation) = resolve_anchor_placement(
                &edge, *at_pos, side, Coord::new(gap), pin_length, offset,
            )?;
            Ok(PinSpec {
                designator: decl.name.node.as_str(),
                name, electrical, length, location, orientation, is_hidden, hidden_net_name,
                owner_part_id: p.owner_part_id,
            })
        }
        PinAnchorMode::After { binding, field, after_ref, gap } => {
            let geom = binding_map.get(binding).ok_or_else(|| SpecError::at(
                SpecErrorCode::UndefinedBinding,
                format!("no bound graphic named '${}'", binding),
                decl.body.span,
            ))?;
            let edge = geom_field_to_edge(geom, field).ok_or_else(|| SpecError::at(
                SpecErrorCode::TypeMismatch,
                format!("'{}' is not a valid edge name", field),
                decl.body.span,
            ))?;
            let prev_pos = position_cache.get(after_ref).copied().ok_or_else(|| {
                SpecError::no_span(
                    SpecErrorCode::UndefinedBinding,
                    format!("after: pin '${after_ref}' not yet resolved"),
                )
            })?;
            let prev_coord = match edge.axis() {
                Axis::X => prev_pos.y,
                Axis::Y => prev_pos.x,
            };
            let dir = edge.side.forward_direction();
            let along = Coord::new(prev_coord.raw() + dir * gap);
            let at_pos = along_coord_to_anchor_position(&edge, along);
            let side = get_enum_opt(&props, "side", parse_placement_side)?
                .unwrap_or(PlacementSide::Outside);
            let extra_offset = get_coord_point_opt(&props, "offset", decl.body.span)?;
            let pin_length = length.unwrap_or(Coord::from_mils(25));
            let (mut location, orientation) = resolve_anchor_placement(
                &edge, at_pos, side, Coord::ZERO, pin_length, extra_offset,
            )?;
            // Override the along-edge coordinate with the computed absolute position.
            match edge.axis() {
                Axis::X => location.y = along,
                Axis::Y => location.x = along,
            }
            // Re-apply side offset.
            let side_offset = compute_side_offset(&edge, side, pin_length);
            match edge.axis() {
                Axis::X => location.x = Coord::new(edge.position.raw() + side_offset),
                Axis::Y => location.y = Coord::new(edge.position.raw() + side_offset),
            }
            Ok(PinSpec {
                designator: decl.name.node.as_str(),
                name, electrical, length, location, orientation, is_hidden, hidden_net_name,
                owner_part_id: p.owner_part_id,
            })
        }
        PinAnchorMode::Before { binding, field, before_ref, gap } => {
            let geom = binding_map.get(binding).ok_or_else(|| SpecError::at(
                SpecErrorCode::UndefinedBinding,
                format!("no bound graphic named '${}'", binding),
                decl.body.span,
            ))?;
            let edge = geom_field_to_edge(geom, field).ok_or_else(|| SpecError::at(
                SpecErrorCode::TypeMismatch,
                format!("'{}' is not a valid edge name", field),
                decl.body.span,
            ))?;
            let next_pos = position_cache.get(before_ref).copied().ok_or_else(|| {
                SpecError::no_span(
                    SpecErrorCode::UndefinedBinding,
                    format!("before: pin '${before_ref}' not yet resolved"),
                )
            })?;
            let next_coord = match edge.axis() {
                Axis::X => next_pos.y,
                Axis::Y => next_pos.x,
            };
            let dir = edge.side.forward_direction();
            // Place BEFORE = reverse direction from the reference pin.
            let along = Coord::new(next_coord.raw() - dir * gap);
            let side = get_enum_opt(&props, "side", parse_placement_side)?
                .unwrap_or(PlacementSide::Outside);
            let extra_offset = get_coord_point_opt(&props, "offset", decl.body.span)?;
            let pin_length = length.unwrap_or(Coord::from_mils(25));
            let (mut location, orientation) = resolve_anchor_placement(
                &edge, AnchorPosition::Center, side, Coord::ZERO, pin_length, extra_offset,
            )?;
            match edge.axis() {
                Axis::X => location.y = along,
                Axis::Y => location.x = along,
            }
            let side_offset = compute_side_offset(&edge, side, pin_length);
            match edge.axis() {
                Axis::X => location.x = Coord::new(edge.position.raw() + side_offset),
                Axis::Y => location.y = Coord::new(edge.position.raw() + side_offset),
            }
            Ok(PinSpec {
                designator: decl.name.node.as_str(),
                name, electrical, length, location, orientation, is_hidden, hidden_net_name,
                owner_part_id: p.owner_part_id,
            })
        }
    }
}

/// Compute the side offset (perpendicular to the edge) for pin location.
///
/// Returns the signed offset from the edge position.
/// For `outside`, the pin's connection point extends away from the body.
fn compute_side_offset(edge: &Edge, side: PlacementSide, pin_length: Coord) -> i32 {
    let len = pin_length.raw();
    match (edge.side, side) {
        // Left edge: outside = left (-X), inside = right (+X)
        (EdgeSide::Left, PlacementSide::Outside) => -len,
        (EdgeSide::Left, PlacementSide::Inside) => len,
        (EdgeSide::Left, PlacementSide::Center) => 0,
        // Right edge: outside = right (+X), inside = left (-X)
        (EdgeSide::Right, PlacementSide::Outside) => len,
        (EdgeSide::Right, PlacementSide::Inside) => -len,
        (EdgeSide::Right, PlacementSide::Center) => 0,
        // Top edge: outside = up (+Y), inside = down (-Y)
        (EdgeSide::Top, PlacementSide::Outside) => len,
        (EdgeSide::Top, PlacementSide::Inside) => -len,
        (EdgeSide::Top, PlacementSide::Center) => 0,
        // Bottom edge: outside = down (-Y), inside = up (+Y)
        (EdgeSide::Bottom, PlacementSide::Outside) => -len,
        (EdgeSide::Bottom, PlacementSide::Inside) => len,
        (EdgeSide::Bottom, PlacementSide::Center) => 0,
    }
}

/// Convert an absolute along-edge coordinate back to an AnchorPosition.
/// Used to bridge after:/before: absolute positions into the `resolve_anchor_placement` API.
fn along_coord_to_anchor_position(_edge: &Edge, _along: Coord) -> AnchorPosition {
    // We override the position directly after calling resolve_anchor_placement,
    // so we just need a placeholder here.
    AnchorPosition::Start
}

/// Resolve anchor-based placement to absolute coordinates and orientation.
///
/// Returns `(location, orientation)` where `location` is the pin's connection point
/// (where wires attach) and `orientation` is the auto-inferred rotation.
pub fn resolve_anchor_placement(
    edge: &Edge,
    at_pos: AnchorPosition,
    side: PlacementSide,
    _gap: Coord,
    pin_length: Coord,
    offset: Option<CoordPoint>,
) -> Result<(CoordPoint, RotationBy90), SpecError> {
    // Step 1: position along the edge.
    let along = edge.point_at(at_pos);

    // Step 2: fixed coordinate (perpendicular) with side offset.
    let side_offset = compute_side_offset(edge, side, pin_length);
    let fixed = Coord::new(edge.position.raw() + side_offset);

    // Step 3: assemble location.
    let mut location = match edge.axis() {
        Axis::X => CoordPoint::new(fixed, along),
        Axis::Y => CoordPoint::new(along, fixed),
    };

    // Step 4: apply offset.
    if let Some(off) = offset {
        location.x = Coord::new(location.x.raw() + off.x.raw());
        location.y = Coord::new(location.y.raw() + off.y.raw());
    }

    // Step 5: auto orientation.
    let orientation = edge.side.auto_orientation();

    Ok((location, orientation))
}

fn get_coord_point_opt(
    props: &IndexMap<String, Value>,
    key: &str,
    span: crate::diagnostic::Span,
) -> Result<Option<CoordPoint>, SpecError> {
    match props.get(key) {
        None => Ok(None),
        Some(v) => Ok(Some(value_to_coord_point(v, Some(span))?)),
    }
}

// ── Layout expansion: Row / Column ───────────────────────────────────────────

/// Absolute direction for rows with no `on:` anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AbsDirection {
    Up,
    Down,
    Left,
    Right,
}

fn parse_abs_direction(s: &str) -> Option<AbsDirection> {
    match s.to_ascii_lowercase().as_str() {
        "up" => Some(AbsDirection::Up),
        "down" => Some(AbsDirection::Down),
        "left" => Some(AbsDirection::Left),
        "right" => Some(AbsDirection::Right),
        _ => None,
    }
}

/// Extract the `pad:` sub-object from a row/grid body map.
fn extract_pad_template(
    props: &IndexMap<String, Value>,
    span: crate::diagnostic::Span,
) -> Result<IndexMap<String, Value>, SpecError> {
    match props.get("pad") {
        Some(Value::Object(m)) => Ok(m.clone()),
        Some(other) => Err(SpecError::at(
            SpecErrorCode::TypeMismatch,
            format!("pad: expected object, got {}", other.kind_name()),
            span,
        )),
        None => Ok(IndexMap::new()),
    }
}

/// Extract a skip list (array of strings/integers) from props.
fn extract_skip_list(props: &IndexMap<String, Value>) -> Vec<String> {
    match props.get("skip") {
        Some(Value::Array(arr)) => arr.iter().filter_map(|v| match v {
            Value::String(s) => Some(s.clone()),
            Value::Integer(n) => Some(n.to_string()),
            _ => None,
        }).collect(),
        _ => vec![],
    }
}

/// Build a PadSpec from template props + computed position override.
fn pad_from_template(
    pad_name: String,
    at: CoordPoint,
    template: &IndexMap<String, Value>,
    span: crate::diagnostic::Span,
) -> Result<PadSpec, SpecError> {
    let shape = get_enum_opt(template, "shape", parse_pad_shape)?;
    let x_size = get_coord_opt(template, "x_size")?;
    let y_size = get_coord_opt(template, "y_size")?;
    let rotation = get_float_opt(template, "rotation");
    let hole_size = get_coord_opt(template, "hole_size")?;
    let is_plated = get_bool_opt(template, "is_plated");
    let layer = get_enum_opt(template, "layer", parse_layer_spec)?;
    let pad_mode = get_enum_opt(template, "pad_mode", parse_pad_stack_mode)?;
    let solder_mask_expansion = get_coord_opt(template, "solder_mask_expansion")?;
    let paste_mask_expansion = get_coord_opt(template, "paste_mask_expansion")?;
    let plane_connection = get_enum_opt(template, "plane_connection", parse_plane_connection)?;
    let relief_conductor_width = get_coord_opt(template, "relief_conductor_width")?;
    let relief_entries = get_integer_opt(template, "relief_entries");
    let relief_air_gap = get_coord_opt(template, "relief_air_gap")?;
    // Use template `at` only if no computed position was given; here `at` is always computed.
    let _ = span; // used for context
    Ok(PadSpec {
        pad_name,
        at,
        shape,
        x_size,
        y_size,
        rotation,
        hole_size,
        is_plated,
        layer,
        pad_mode,
        solder_mask_expansion,
        paste_mask_expansion,
        plane_connection,
        relief_conductor_width,
        relief_entries,
        relief_air_gap,
    })
}

/// Merge explicit override props into an already-built PadSpec.
fn merge_pad_override_from_props(
    pad: &mut PadSpec,
    explicit: &IndexMap<String, Value>,
    span: crate::diagnostic::Span,
) -> Result<(), SpecError> {
    if let Some(v) = explicit.get("at") {
        pad.at = value_to_coord_point(v, Some(span))?;
    }
    if let Ok(Some(v)) = get_enum_opt(explicit, "shape", parse_pad_shape) {
        pad.shape = Some(v);
    }
    if let Ok(Some(v)) = get_coord_opt(explicit, "x_size") {
        pad.x_size = Some(v);
    }
    if let Ok(Some(v)) = get_coord_opt(explicit, "y_size") {
        pad.y_size = Some(v);
    }
    if let Some(v) = get_float_opt(explicit, "rotation") {
        pad.rotation = Some(v);
    }
    if let Ok(Some(v)) = get_coord_opt(explicit, "hole_size") {
        pad.hole_size = Some(v);
    }
    if let Some(v) = get_bool_opt(explicit, "is_plated") {
        pad.is_plated = Some(v);
    }
    if let Ok(Some(v)) = get_enum_opt(explicit, "layer", parse_layer_spec) {
        pad.layer = Some(v);
    }
    if let Ok(Some(v)) = get_enum_opt(explicit, "pad_mode", parse_pad_stack_mode) {
        pad.pad_mode = Some(v);
    }
    if let Ok(Some(v)) = get_coord_opt(explicit, "solder_mask_expansion") {
        pad.solder_mask_expansion = Some(v);
    }
    if let Ok(Some(v)) = get_coord_opt(explicit, "paste_mask_expansion") {
        pad.paste_mask_expansion = Some(v);
    }
    if let Ok(Some(v)) = get_enum_opt(explicit, "plane_connection", parse_plane_connection) {
        pad.plane_connection = Some(v);
    }
    if let Ok(Some(v)) = get_coord_opt(explicit, "relief_conductor_width") {
        pad.relief_conductor_width = Some(v);
    }
    if let Some(v) = get_integer_opt(explicit, "relief_entries") {
        pad.relief_entries = Some(v);
    }
    if let Ok(Some(v)) = get_coord_opt(explicit, "relief_air_gap") {
        pad.relief_air_gap = Some(v);
    }
    Ok(())
}

/// Expand a `row` or `column` declaration into individual PadSpecs.
fn expand_row(decl: &crate::ast::RowDecl, scope: &ScopeStack) -> Result<Vec<PadSpec>, SpecError> {
    let span = decl.body.span;
    let props = eval_object_to_map_skip_row_keys(&decl.body.node, scope)?;

    let count = get_integer_opt(&props, "count").unwrap_or(1) as usize;
    let start = get_integer_opt(&props, "start").unwrap_or(1);
    let pitch = get_coord_opt(&props, "pitch")?.unwrap_or(Coord::ZERO);
    let direction_reverse = get_string_opt(&props, "direction")
        .map(|s| s == "reverse")
        .unwrap_or(false);
    let skip = extract_skip_list(&props);
    let template = extract_pad_template(&props, span)?;

    // Check for `on:` anchor reference.
    let on_ref = extract_row_on_ref(&decl.body.node);

    if let Some((binding, field)) = on_ref {
        // Anchor-based row.
        // Validate: up/down/left/right only allowed for absolute rows.
        if let Some(dir_str) = get_string_opt(&props, "direction") {
            if parse_abs_direction(&dir_str).is_some() {
                return Err(SpecError::at(
                    SpecErrorCode::TypeMismatch,
                    "directions up/down/left/right are only valid for absolute rows (no `on:` anchor)",
                    span,
                ));
            }
        }

        // Resolve anchor position (start/center/end) from `at:` string.
        let at_pos = extract_at_position_from_props(&props)
            .unwrap_or(AnchorPosition::Center);

        // We need the box geometry from the scope. Since we don't have access to the
        // binding map here, we resolve it inline via scope DollarIdent lookup.
        let geom = resolve_binding_geometry_from_scope(&binding, scope, span)?;
        let edge = geom_field_to_edge(&geom, &field).ok_or_else(|| SpecError::at(
            SpecErrorCode::TypeMismatch,
            format!("'{}' is not a valid edge field (use left/right/top/bottom)", field),
            span,
        ))?;

        // Compute center position along edge.
        let center_along = edge.point_at(at_pos);

        // Compute offset of first pad.
        // Total span = (count - 1) * pitch.
        let total_span = pitch * (count as i32 - 1);
        let first_offset = Coord::new(-total_span.raw() / 2);

        // Apply direction: forward follows edge natural direction.
        let dir = if direction_reverse { -edge.side.forward_direction() } else { edge.side.forward_direction() };

        let mut pads = Vec::with_capacity(count);
        let mut pad_counter = 0i32;
        for i in 0..count {
            let along = Coord::new(center_along.raw() + first_offset.raw() + dir * pitch.raw() * i as i32);
            let at = match edge.axis() {
                Axis::X => CoordPoint::new(edge.position, along),
                Axis::Y => CoordPoint::new(along, edge.position),
            };
            let name = generate_pad_name(start, &mut pad_counter, &skip);
            if name.is_empty() { continue; }
            pads.push(pad_from_template(name, at, &template, span)?);
        }
        Ok(pads)
    } else {
        // Absolute row: `at:` is a coordinate, `direction:` is up/down/left/right.
        let first_at = match props.get("at") {
            Some(v) => value_to_coord_point(v, Some(span))?,
            None => CoordPoint::zero(),
        };

        let dir_str = get_string_opt(&props, "direction").unwrap_or_else(|| "right".to_string());
        let abs_dir = parse_abs_direction(&dir_str).ok_or_else(|| SpecError::at(
            SpecErrorCode::TypeMismatch,
            format!("invalid direction '{}': expected up/down/left/right for absolute rows", dir_str),
            span,
        ))?;

        let mut pads = Vec::with_capacity(count);
        let mut pad_counter = 0i32;
        for i in 0..count {
            let at = abs_direction_step(first_at, abs_dir, pitch, i);
            let name = generate_pad_name(start, &mut pad_counter, &skip);
            if name.is_empty() { continue; }
            pads.push(pad_from_template(name, at, &template, span)?);
        }
        Ok(pads)
    }
}

/// Step from first position by abs_direction * pitch * step_index.
fn abs_direction_step(first: CoordPoint, dir: AbsDirection, pitch: Coord, step: usize) -> CoordPoint {
    let n = step as i32;
    match dir {
        AbsDirection::Right => CoordPoint::new(first.x + pitch * n, first.y),
        AbsDirection::Left  => CoordPoint::new(Coord::new(first.x.raw() - pitch.raw() * n), first.y),
        AbsDirection::Up    => CoordPoint::new(first.x, first.y + pitch * n),
        AbsDirection::Down  => CoordPoint::new(first.x, Coord::new(first.y.raw() - pitch.raw() * n)),
    }
}

/// Generate next pad name, skipping any in `skip` list.
/// `counter` tracks how many pads have been emitted so far (used for skip).
/// Returns empty string if the current name is in the skip list.
fn generate_pad_name(start: i32, counter: &mut i32, skip: &[String]) -> String {
    loop {
        let name = (start + *counter).to_string();
        *counter += 1;
        if !skip.contains(&name) {
            return name;
        }
        // Name is in skip — try next
    }
}

/// Resolve a `$binding.field` geometry by looking up the binding in scope.
/// The scope should contain the binding as an Object with `from` and `to` keys.
fn resolve_binding_geometry_from_scope(
    binding: &str,
    scope: &ScopeStack,
    span: crate::diagnostic::Span,
) -> Result<BoxGeometry, SpecError> {
    let val = scope.lookup_dollar(binding)
        .ok_or_else(|| SpecError::at(
            SpecErrorCode::UndefinedBinding,
            format!("no binding '${binding}' in scope"),
            span,
        ))?
        .map_err(|e| e)?
        .clone();
    let map = match val {
        Value::Object(m) => m,
        ref other => return Err(SpecError::at(
            SpecErrorCode::TypeMismatch,
            format!("binding '${binding}' must be an object with from/to, got {}", other.kind_name()),
            span,
        )),
    };
    let from = match map.get("from") {
        Some(v) => value_to_coord_point(v, Some(span))?,
        None => return Err(SpecError::at(
            SpecErrorCode::TypeMismatch,
            format!("binding '${binding}' has no 'from' field"),
            span,
        )),
    };
    let to = match map.get("to") {
        Some(v) => value_to_coord_point(v, Some(span))?,
        None => return Err(SpecError::at(
            SpecErrorCode::TypeMismatch,
            format!("binding '${binding}' has no 'to' field"),
            span,
        )),
    };
    Ok(BoxGeometry { from, to })
}

/// Extract `on: $binding.field` from a row body object.
fn extract_row_on_ref(obj: &crate::ast::Object) -> Option<(String, String)> {
    for item in &obj.items {
        if let crate::ast::ObjectItem::Property(p) = &item.node {
            if p.key.node == "on" {
                return extract_dollar_path_two_part(&p.value.node);
            }
        }
    }
    None
}

/// Evaluate row/grid body skipping anchor keys (`on`, `after`, `before`, `pad`).
/// The `pad:` sub-object is handled separately.
fn eval_object_to_map_skip_row_keys(
    obj: &crate::ast::Object,
    scope: &ScopeStack,
) -> EvalResult<IndexMap<String, Value>> {
    const SKIP: &[&str] = &["on", "after", "before", "pad"];
    let mut result: IndexMap<String, Value> = IndexMap::new();
    for item in &obj.items {
        match &item.node {
            ObjectItem::LetBinding(_) => {}
            ObjectItem::Spread(spread_expr) => {
                let spread_val = eval_expr(spread_expr, scope)?;
                let spread_map = spread_val.into_object(Some(spread_expr.span))?;
                for (k, v) in spread_map {
                    if !SKIP.contains(&k.as_str()) {
                        result.insert(k, v);
                    }
                }
            }
            ObjectItem::Property(prop) => {
                if !SKIP.contains(&prop.key.node.as_str()) {
                    let val = eval_expr(&prop.value, scope)?;
                    result.insert(prop.key.node.clone(), val);
                }
            }
        }
    }
    // Also evaluate pad: sub-object and store as Value::Object for extract_pad_template.
    for item in &obj.items {
        if let ObjectItem::Property(prop) = &item.node {
            if prop.key.node == "pad" {
                if let crate::ast::Expr::Object(inner_obj) = &prop.value.node {
                    let map = eval_object_to_map(inner_obj, scope)?;
                    result.insert("pad".to_string(), Value::Object(map));
                } else {
                    let val = eval_expr(&prop.value, scope)?;
                    result.insert("pad".to_string(), val);
                }
                break;
            }
        }
    }
    Ok(result)
}

/// Extract anchor `at:` as AnchorPosition from an already-evaluated props map.
fn extract_at_position_from_props(props: &IndexMap<String, Value>) -> Option<AnchorPosition> {
    match props.get("at") {
        Some(Value::String(s)) => parse_anchor_position(s),
        _ => None,
    }
}

// ── Layout expansion: Grid ────────────────────────────────────────────────────

/// BGA row letters: A–Z skipping I, O, Q, S, X, Z.
const BGA_LETTERS: &[char] = &[
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H',
    'J', 'K', 'L', 'M', 'N', 'P', 'R', 'T',
    'U', 'V', 'W', 'Y',
];

/// Generate the nth (0-indexed) BGA row letter sequence.
/// 0..19 → A..Y (skipping I,O,Q,S,X,Z), 20..39 → AA..AY, etc.
fn bga_row_letter(n: usize) -> String {
    let base = BGA_LETTERS.len();
    if n < base {
        BGA_LETTERS[n].to_string()
    } else {
        // Double-letter: prefix is bga_row_letter(n / base - 1), suffix is BGA_LETTERS[n % base]
        let prefix = bga_row_letter(n / base - 1);
        format!("{}{}", prefix, BGA_LETTERS[n % base])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GridNaming {
    Numeric,
    Alphanumeric,
}

fn parse_grid_naming(s: &str) -> Option<GridNaming> {
    match s.to_ascii_lowercase().as_str() {
        "numeric" => Some(GridNaming::Numeric),
        "alphanumeric" => Some(GridNaming::Alphanumeric),
        _ => None,
    }
}

/// Expand a `grid` declaration into individual PadSpecs.
fn expand_grid(decl: &crate::ast::GridDecl, scope: &ScopeStack) -> Result<Vec<PadSpec>, SpecError> {
    let span = decl.body.span;
    let props = eval_object_to_map_skip_row_keys(&decl.body.node, scope)?;

    let rows = get_integer_opt(&props, "rows").unwrap_or(1) as usize;
    let cols = get_integer_opt(&props, "cols").unwrap_or(1) as usize;

    // Support pitch (square) or pitch_x/pitch_y (asymmetric).
    let pitch_default = get_coord_opt(&props, "pitch")?.unwrap_or(Coord::ZERO);
    let pitch_x = get_coord_opt(&props, "pitch_x")?.unwrap_or(pitch_default);
    let pitch_y = get_coord_opt(&props, "pitch_y")?.unwrap_or(pitch_default);

    let origin = match props.get("origin") {
        Some(v) => value_to_coord_point(v, Some(span))?,
        None => CoordPoint::zero(),
    };

    let naming = get_enum_opt(&props, "naming", parse_grid_naming)?
        .unwrap_or(GridNaming::Numeric);
    let perimeter_only = get_bool_opt(&props, "perimeter_only").unwrap_or(false);
    let skip = extract_skip_list(&props);
    let template = extract_pad_template(&props, span)?;

    let mut pads = Vec::with_capacity(rows * cols);
    let mut numeric_counter = 1i32;

    for row in 0..rows {
        for col in 0..cols {
            let name = match naming {
                GridNaming::Numeric => {
                    let n = numeric_counter.to_string();
                    numeric_counter += 1;
                    n
                }
                GridNaming::Alphanumeric => {
                    let letter = bga_row_letter(row);
                    format!("{}{}", letter, col + 1)
                }
            };

            // Skip interior pads if perimeter_only.
            if perimeter_only {
                let on_edge = row == 0 || row == rows - 1 || col == 0 || col == cols - 1;
                if !on_edge { continue; }
            }

            if skip.contains(&name) { continue; }

            // x = origin.x + (col - (cols-1)/2.0) * pitch_x
            // y = origin.y + (row - (rows-1)/2.0) * pitch_y  (row 0 = top)
            let x = {
                let offset_num = 2 * col as i32 - (cols as i32 - 1);
                // offset_num / 2 * pitch_x — use raw to avoid rounding issues
                Coord::new(origin.x.raw() + pitch_x.raw() * offset_num / 2)
            };
            let y = {
                let offset_num = 2 * row as i32 - (rows as i32 - 1);
                Coord::new(origin.y.raw() - pitch_y.raw() * offset_num / 2)
            };
            let at = CoordPoint::new(x, y);

            pads.push(pad_from_template(name, at, &template, span)?);
        }
    }

    Ok(pads)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_spec;

    fn compile_schlib(src: &str) -> Result<SchLibSpec, SpecError> {
        let file = parse_spec(src).expect("parse failed");
        match compile_spec(&file, SpecDomain::SchLib)? {
            SpecModel::SchLib(s) => Ok(s),
            other => panic!("expected SchLib, got {:?}", std::mem::discriminant(&other)),
        }
    }

    fn compile_pcblib(src: &str) -> Result<crate::model::PcbLibSpec, SpecError> {
        let file = parse_spec(src).expect("parse failed");
        match compile_spec(&file, SpecDomain::PcbLib)? {
            SpecModel::PcbLib(p) => Ok(p),
            other => panic!("expected PcbLib, got {:?}", std::mem::discriminant(&other)),
        }
    }

    // ── Simple component ───────────────────────────────────────────────────

    #[test]
    fn simple_component_no_pins() {
        let src = r#"
            component R_0603 {
                designator: "R"
                description: "SMD resistor 0603"
            }
        "#;
        let spec = compile_schlib(src).unwrap();
        assert_eq!(spec.components.len(), 1);
        let c = &spec.components[0];
        assert_eq!(c.lib_reference, "R_0603");
        assert_eq!(c.designator.as_deref(), Some("R"));
        assert_eq!(c.description.as_deref(), Some("SMD resistor 0603"));
    }

    // ── Pin compilation with absolute placement ────────────────────────────

    #[test]
    fn pin_absolute_placement() {
        let src = r#"
            component R {
                pin 1 {
                    at: (100mil, 0mil)
                    orientation: "0"
                }
                pin 2 {
                    at: (-100mil, 0mil)
                    orientation: "180"
                }
            }
        "#;
        let spec = compile_schlib(src).unwrap();
        let c = &spec.components[0];
        assert_eq!(c.pins.len(), 2);

        let p1 = &c.pins[0];
        assert_eq!(p1.designator, "1");
        assert_eq!(p1.location.x, Coord::new(1_000_000));
        assert_eq!(p1.location.y, Coord::ZERO);
        assert_eq!(p1.orientation, RotationBy90::Rotate0);

        let p2 = &c.pins[1];
        assert_eq!(p2.designator, "2");
        assert_eq!(p2.orientation, RotationBy90::Rotate180);
    }

    // ── Parameters ────────────────────────────────────────────────────────

    #[test]
    fn parameter_compilation() {
        let src = r#"
            component R_0603 {
                parameter "Value" { text: "10k" }
                parameter "Tolerance" { text: "1%" is_hidden: false }
            }
        "#;
        let spec = compile_schlib(src).unwrap();
        let c = &spec.components[0];
        assert_eq!(c.parameters.len(), 2);
        assert_eq!(c.parameters[0].name, "Value");
        assert_eq!(c.parameters[0].text, "10k");
        assert_eq!(c.parameters[1].name, "Tolerance");
        assert_eq!(c.parameters[1].text, "1%");
        assert_eq!(c.parameters[1].is_hidden, Some(false));
    }

    // ── Aliases ───────────────────────────────────────────────────────────

    #[test]
    fn alias_compilation() {
        let src = r#"
            component RES {
                alias "R"
                alias RESISTOR
            }
        "#;
        let spec = compile_schlib(src).unwrap();
        let c = &spec.components[0];
        assert_eq!(c.aliases, vec!["R".to_string(), "RESISTOR".to_string()]);
    }

    // ── Graphic unique_id: bound ───────────────────────────────────────────

    #[test]
    fn graphic_unique_id_bound() {
        let src = r#"
            component R_0603 {
                body = rectangle {
                    from: (0mil, 0mil)
                    to: (100mil, 50mil)
                }
            }
        "#;
        let spec = compile_schlib(src).unwrap();
        let c = &spec.components[0];
        assert_eq!(c.graphics.len(), 1);
        assert_eq!(c.graphics[0].unique_id, "spec:R_0603:body");
        assert!(matches!(c.graphics[0].graphic_type, GraphicType::Rectangle));
    }

    // ── Graphic unique_id: unnamed ─────────────────────────────────────────

    #[test]
    fn graphic_unique_id_unnamed() {
        let src = r#"
            component R_0603 {
                line { from: (0mil, 0mil) to: (10mil, 10mil) }
                line { from: (20mil, 0mil) to: (30mil, 10mil) }
            }
        "#;
        let spec = compile_schlib(src).unwrap();
        let c = &spec.components[0];
        assert_eq!(c.graphics.len(), 2);
        assert_eq!(c.graphics[0].unique_id, "spec:R_0603:line_0");
        assert_eq!(c.graphics[1].unique_id, "spec:R_0603:line_1");
    }

    // ── Multi-part component ───────────────────────────────────────────────

    #[test]
    fn multi_part_component() {
        let src = r#"
            component LM358 {
                part 1 {
                    pin "IN+" { at: (0mil, 0mil) }
                    pin "IN-" { at: (0mil, -50mil) }
                    pin "OUT" { at: (100mil, -25mil) }
                }
                part 2 {
                    pin "IN+" { at: (0mil, 0mil) }
                    pin "OUT" { at: (100mil, -25mil) }
                }
            }
        "#;
        let spec = compile_schlib(src).unwrap();
        let c = &spec.components[0];
        assert_eq!(c.lib_reference, "LM358");
        assert_eq!(c.parts.len(), 2);
        assert_eq!(c.parts[0].part_number, 1);
        assert_eq!(c.parts[0].pins.len(), 3);
        assert_eq!(c.parts[0].pins[0].owner_part_id, 1);
        assert_eq!(c.parts[1].part_number, 2);
        assert_eq!(c.parts[1].pins.len(), 2);
    }

    // ── Part-scoped unique_id ──────────────────────────────────────────────

    #[test]
    fn part_scoped_unique_id() {
        let src = r#"
            component LM358 {
                part 1 {
                    body = rectangle { from: (0mil, 0mil) to: (100mil, 100mil) }
                }
            }
        "#;
        let spec = compile_schlib(src).unwrap();
        let c = &spec.components[0];
        assert_eq!(c.parts[0].graphics[0].unique_id, "spec:LM358:part1:body");
    }

    // ── Let bindings in component scope ───────────────────────────────────

    #[test]
    fn let_bindings_in_component() {
        let src = r#"
            component R {
                let w = 100mil
                let h = 50mil
                body = rectangle {
                    from: (0mil, 0mil)
                    to: (w, h)
                }
            }
        "#;
        let spec = compile_schlib(src).unwrap();
        let c = &spec.components[0];
        assert_eq!(c.graphics.len(), 1);
        let props = &c.graphics[0].properties;
        // to: (100mil, 50mil) = (1_000_000, 500_000)
        let to = props.to.unwrap();
        assert_eq!(to.x, Coord::new(1_000_000));
        assert_eq!(to.y, Coord::new(500_000));
    }

    // ── File-level let bindings ────────────────────────────────────────────

    #[test]
    fn file_level_let_bindings() {
        let src = r#"
            let pin_length = 200mil
            component R {
                pin 1 {
                    at: (0mil, 0mil)
                    length: pin_length
                }
            }
        "#;
        let spec = compile_schlib(src).unwrap();
        let c = &spec.components[0];
        assert_eq!(c.pins[0].length, Some(Coord::new(2_000_000)));
    }

    // ── FootprintMap compilation ───────────────────────────────────────────

    #[test]
    fn footprint_map_compilation() {
        let src = r#"
            component R_0603 {
                footprint "R_0603_SMD" {
                    map { pin: 1, pad: 1 }
                    map { pin: 2, pad: 2 }
                }
            }
        "#;
        let spec = compile_schlib(src).unwrap();
        let c = &spec.components[0];
        assert_eq!(c.footprints.len(), 1);
        assert_eq!(c.footprints[0].model_name, "R_0603_SMD");
        assert_eq!(c.footprints[0].maps.len(), 2);
        assert_eq!(c.footprints[0].maps[0].pin, "1");
        assert_eq!(c.footprints[0].maps[0].pad, "1");
    }

    // ── Multiple components ────────────────────────────────────────────────

    #[test]
    fn multiple_components() {
        let src = r#"
            component R { }
            component C { }
            component L { }
        "#;
        let spec = compile_schlib(src).unwrap();
        assert_eq!(spec.components.len(), 3);
        assert_eq!(spec.components[0].lib_reference, "R");
        assert_eq!(spec.components[1].lib_reference, "C");
        assert_eq!(spec.components[2].lib_reference, "L");
    }

    // ── PcbLib footprint ───────────────────────────────────────────────────

    #[test]
    fn footprint_compilation() {
        let src = r#"
            footprint "R_0603" {
                description: "SMD resistor 0603"
                pad 1 {
                    at: (0mil, 0mil)
                    shape: "rectangular"
                    x_size: 60mil
                    y_size: 80mil
                    layer: "TopLayer"
                }
                pad 2 {
                    at: (100mil, 0mil)
                    shape: "rectangular"
                    x_size: 60mil
                    y_size: 80mil
                    layer: "TopLayer"
                }
            }
        "#;
        let spec = compile_pcblib(src).unwrap();
        assert_eq!(spec.footprints.len(), 1);
        let fp = &spec.footprints[0];
        assert_eq!(fp.display_name, "R_0603");
        assert_eq!(fp.description.as_deref(), Some("SMD resistor 0603"));
        assert_eq!(fp.pads.len(), 2);
        assert_eq!(fp.pads[0].pad_name, "1");
        assert_eq!(fp.pads[0].at.x, Coord::ZERO);
        assert_eq!(fp.pads[0].at.y, Coord::ZERO);
        assert_eq!(fp.pads[0].shape, Some(PadShape::Rectangular));
        assert_eq!(fp.pads[0].x_size, Some(Coord::new(600_000)));
        assert!(matches!(
            &fp.pads[0].layer,
            Some(LayerSpec::Resolved(lr)) if lr.display_name() == Some("TopLayer")
        ));
        assert_eq!(fp.pads[1].pad_name, "2");
        assert_eq!(fp.pads[1].at.x, Coord::new(1_000_000));
    }

    // ── Enum parsing ───────────────────────────────────────────────────────

    #[test]
    fn pin_electrical_type_parsing() {
        assert!(matches!(parse_pin_electrical_type("input"), Some(PinElectricalType::Input)));
        assert!(matches!(parse_pin_electrical_type("passive"), Some(PinElectricalType::Passive)));
        assert!(matches!(parse_pin_electrical_type("io"), Some(PinElectricalType::InputOutput)));
        assert!(matches!(parse_pin_electrical_type("power"), Some(PinElectricalType::Power)));
        assert!(parse_pin_electrical_type("unknown").is_none());
    }

    #[test]
    fn pad_shape_parsing() {
        assert!(matches!(parse_pad_shape("rectangular"), Some(PadShape::Rectangular)));
        assert!(matches!(parse_pad_shape("round"), Some(PadShape::Round)));
        assert!(matches!(parse_pad_shape("circle"), Some(PadShape::Round)));
        assert!(parse_pad_shape("invalid").is_none());
    }

    // ── Error: undefined binding ───────────────────────────────────────────

    #[test]
    fn error_undefined_binding_in_coord_context() {
        // Bare identifiers not in scope become strings (§7.3 step 3).
        // When a string lands in a coordinate context, it's a TypeMismatch.
        let src = r#"
            component R {
                pin 1 { at: (undefined_var, 0mil) }
            }
        "#;
        let file = parse_spec(src).expect("parse failed");
        let result = compile_spec(&file, SpecDomain::SchLib);
        let err = result.err().expect("expected error");
        assert_eq!(err.code, SpecErrorCode::TypeMismatch);
    }

    #[test]
    fn error_undefined_dollar_binding() {
        // $-prefixed bindings still error when undefined (no string fallback).
        let src = r#"
            component R {
                pin 1 { at: ($undefined_ref, 0mil) }
            }
        "#;
        let file = parse_spec(src).expect("parse failed");
        let result = compile_spec(&file, SpecDomain::SchLib);
        let err = result.err().expect("expected error");
        assert_eq!(err.code, SpecErrorCode::UndefinedBinding);
    }

    // ── Error: type mismatch ───────────────────────────────────────────────

    #[test]
    fn error_type_mismatch_coord() {
        let src = r#"
            component R {
                pin 1 {
                    at: "not_a_coord"
                }
            }
        "#;
        let file = parse_spec(src).expect("parse failed");
        let result = compile_spec(&file, SpecDomain::SchLib);
        assert!(result.is_err());
    }

    // ── Graphic types ──────────────────────────────────────────────────────

    #[test]
    fn all_sch_graphic_types() {
        let src = r#"
            component R {
                line { from: (0mil, 0mil) to: (10mil, 10mil) }
                rectangle { from: (0mil, 0mil) to: (50mil, 50mil) }
                arc { center: (0mil, 0mil) radius: 50mil }
                ellipse { center: (0mil, 0mil) radius: 50mil }
                label { at: (0mil, 0mil) text: "hello" }
            }
        "#;
        let spec = compile_schlib(src).unwrap();
        let c = &spec.components[0];
        assert_eq!(c.graphics.len(), 5);
        assert!(matches!(c.graphics[0].graphic_type, GraphicType::Line));
        assert!(matches!(c.graphics[1].graphic_type, GraphicType::Rectangle));
        assert!(matches!(c.graphics[2].graphic_type, GraphicType::Arc));
        assert!(matches!(c.graphics[3].graphic_type, GraphicType::Ellipse));
        assert!(matches!(c.graphics[4].graphic_type, GraphicType::Label));
    }

    // ── Anchor placement: at: center on left edge ─────────────────────────

    #[test]
    fn anchor_left_center_outside() {
        // body from (-20mil, -10mil) to (20mil, 10mil)
        // left edge: x = -20mil = -200_000, y range (-10mil, 10mil)
        // at: center -> y = 0
        // side: outside -> x = -200_000 + (-250_000) = -450_000 (default length=25mil)
        let src = r#"
            component R {
                body = rectangle { from: (-20mil, -10mil) to: (20mil, 10mil) }
                pin 1 { on: $body.left, at: "center", side: "outside" }
            }
        "#;
        let spec = compile_schlib(src).unwrap();
        let p = &spec.components[0].pins[0];
        assert_eq!(p.location.x, Coord::new(-450_000));
        assert_eq!(p.location.y, Coord::ZERO);
        assert_eq!(p.orientation, RotationBy90::Rotate0);
    }

    #[test]
    fn anchor_right_center_outside() {
        // right edge: x = 20mil = 200_000
        // outside right: side_offset = +pin_length = +250_000
        // location x = 200_000 + 250_000 = 450_000
        let src = r#"
            component R {
                body = rectangle { from: (-20mil, -10mil) to: (20mil, 10mil) }
                pin 1 { on: $body.right, at: "center", side: "outside" }
            }
        "#;
        let spec = compile_schlib(src).unwrap();
        let p = &spec.components[0].pins[0];
        assert_eq!(p.location.x, Coord::new(450_000));
        assert_eq!(p.location.y, Coord::ZERO);
        assert_eq!(p.orientation, RotationBy90::Rotate180);
    }

    #[test]
    fn anchor_top_center_outside() {
        // top edge: y = 10mil = 100_000
        // outside top: side_offset = +pin_length = +250_000
        // location y = 100_000 + 250_000 = 350_000
        let src = r#"
            component R {
                body = rectangle { from: (-20mil, -10mil) to: (20mil, 10mil) }
                pin 1 { on: $body.top, at: "center", side: "outside" }
            }
        "#;
        let spec = compile_schlib(src).unwrap();
        let p = &spec.components[0].pins[0];
        assert_eq!(p.location.x, Coord::ZERO);
        assert_eq!(p.location.y, Coord::new(350_000));
        assert_eq!(p.orientation, RotationBy90::Rotate270);
    }

    #[test]
    fn anchor_bottom_center_outside() {
        // bottom edge: y = -10mil = -100_000
        // outside bottom: side_offset = -pin_length = -250_000
        // location y = -100_000 + (-250_000) = -350_000
        let src = r#"
            component R {
                body = rectangle { from: (-20mil, -10mil) to: (20mil, 10mil) }
                pin 1 { on: $body.bottom, at: "center", side: "outside" }
            }
        "#;
        let spec = compile_schlib(src).unwrap();
        let p = &spec.components[0].pins[0];
        assert_eq!(p.location.x, Coord::ZERO);
        assert_eq!(p.location.y, Coord::new(-350_000));
        assert_eq!(p.orientation, RotationBy90::Rotate90);
    }

    // ── Anchor placement: at: start / end ─────────────────────────────────

    #[test]
    fn anchor_left_start_end_positions() {
        // left edge forward dir = -1 (top-to-bottom = decreasing Y)
        // start = max Y = 10mil = 100_000
        // end   = min Y = -10mil = -100_000
        let src = r#"
            component R {
                body = rectangle { from: (-20mil, -10mil) to: (20mil, 10mil) }
                pin start_pin { on: $body.left, at: "start", side: "outside" }
                pin end_pin   { on: $body.left, at: "end",   side: "outside" }
            }
        "#;
        let spec = compile_schlib(src).unwrap();
        let pins = &spec.components[0].pins;
        assert_eq!(pins[0].designator, "start_pin");
        assert_eq!(pins[0].location.y, Coord::new(100_000));   // start = top
        assert_eq!(pins[1].designator, "end_pin");
        assert_eq!(pins[1].location.y, Coord::new(-100_000));  // end = bottom
    }

    #[test]
    fn anchor_top_start_end_positions() {
        // top edge forward dir = +1 (left-to-right = increasing X)
        // start = min X = -20mil = -200_000
        // end   = max X = 20mil = 200_000
        let src = r#"
            component R {
                body = rectangle { from: (-20mil, -10mil) to: (20mil, 10mil) }
                pin sp { on: $body.top, at: "start", side: "outside" }
                pin ep { on: $body.top, at: "end",   side: "outside" }
            }
        "#;
        let spec = compile_schlib(src).unwrap();
        let pins = &spec.components[0].pins;
        assert_eq!(pins[0].location.x, Coord::new(-200_000));  // start = left
        assert_eq!(pins[1].location.x, Coord::new(200_000));   // end = right
    }

    // ── Anchor placement: side: inside / center ────────────────────────────

    #[test]
    fn anchor_left_inside() {
        // inside left: side_offset = +pin_length = +250_000
        // location x = -200_000 + 250_000 = 50_000
        let src = r#"
            component R {
                body = rectangle { from: (-20mil, -10mil) to: (20mil, 10mil) }
                pin 1 { on: $body.left, at: "center", side: "inside" }
            }
        "#;
        let spec = compile_schlib(src).unwrap();
        let p = &spec.components[0].pins[0];
        assert_eq!(p.location.x, Coord::new(50_000));
        assert_eq!(p.location.y, Coord::ZERO);
    }

    #[test]
    fn anchor_left_center_side() {
        // center side: side_offset = 0
        // location x = -200_000 + 0 = -200_000
        let src = r#"
            component R {
                body = rectangle { from: (-20mil, -10mil) to: (20mil, 10mil) }
                pin 1 { on: $body.left, at: "center", side: "center" }
            }
        "#;
        let spec = compile_schlib(src).unwrap();
        let p = &spec.components[0].pins[0];
        assert_eq!(p.location.x, Coord::new(-200_000));
        assert_eq!(p.location.y, Coord::ZERO);
    }

    // ── Anchor placement: offset: post-placement translation ──────────────

    #[test]
    fn anchor_with_offset() {
        // left+center+outside base location = (-450_000, 0)
        // offset: (5mil, 3mil) -> (-450_000+50_000, 0+30_000) = (-400_000, 30_000)
        let src = r#"
            component R {
                body = rectangle { from: (-20mil, -10mil) to: (20mil, 10mil) }
                pin 1 { on: $body.left, at: "center", side: "outside", offset: (5mil, 3mil) }
            }
        "#;
        let spec = compile_schlib(src).unwrap();
        let p = &spec.components[0].pins[0];
        assert_eq!(p.location.x, Coord::new(-400_000));
        assert_eq!(p.location.y, Coord::new(30_000));
    }

    // ── Anchor placement: after: chaining ─────────────────────────────────

    #[test]
    fn anchor_after_chain_three_pins() {
        // right edge forward dir = +1 (bottom-to-top = increasing Y)
        // p1 at: center -> y = 0
        // p2 after: $p1, gap: 5mil -> y = 0 + 5_0000 = 50_000
        // p3 after: $p2, gap: 5mil -> y = 50_000 + 50_000 = 100_000
        let src = r#"
            component IC {
                body = rectangle { from: (-20mil, -15mil) to: (20mil, 15mil) }
                p1 = pin 1 { on: $body.right, at: "center", side: "outside" }
                p2 = pin 2 { on: $body.right, after: $p1, gap: 5mil, side: "outside" }
                p3 = pin 3 { on: $body.right, after: $p2, gap: 5mil, side: "outside" }
            }
        "#;
        let spec = compile_schlib(src).unwrap();
        let pins = &spec.components[0].pins;
        assert_eq!(pins.len(), 3);
        // right outside: x = 200_000 + 250_000 = 450_000 for all
        assert_eq!(pins[0].location.x, Coord::new(450_000));
        assert_eq!(pins[0].location.y, Coord::ZERO);
        assert_eq!(pins[1].location.x, Coord::new(450_000));
        assert_eq!(pins[1].location.y, Coord::new(50_000));
        assert_eq!(pins[2].location.x, Coord::new(450_000));
        assert_eq!(pins[2].location.y, Coord::new(100_000));
    }

    // ── Error: cross-edge reference ────────────────────────────────────────

    #[test]
    fn error_cross_edge_reference() {
        let src = r#"
            component IC {
                body = rectangle { from: (-20mil, -15mil) to: (20mil, 15mil) }
                p1 = pin 1 { on: $body.left,  at: "center", side: "outside" }
                p2 = pin 2 { on: $body.right, after: $p1,   side: "outside" }
            }
        "#;
        let file = parse_spec(src).expect("parse failed");
        let result = compile_spec(&file, SpecDomain::SchLib);
        let err = result.err().expect("expected error");
        assert_eq!(err.code, SpecErrorCode::CrossEdgeReference);
    }

    // ── Anchor: undefined binding error ───────────────────────────────────

    #[test]
    fn error_anchor_undefined_binding() {
        let src = r#"
            component R {
                pin 1 { on: $missing_body.left, at: "center", side: "outside" }
            }
        "#;
        let file = parse_spec(src).expect("parse failed");
        let result = compile_spec(&file, SpecDomain::SchLib);
        let err = result.err().expect("expected error");
        assert_eq!(err.code, SpecErrorCode::UndefinedBinding);
    }

    // ── SchLib domain ignores footprints ───────────────────────────────────

    #[test]
    fn domain_filtering() {
        let src = r#"
            component R { }
            footprint "R_0603" { }
        "#;
        // SchLib: only component
        let file = parse_spec(src).unwrap();
        let schlib = compile_spec(&file, SpecDomain::SchLib).unwrap();
        match schlib {
            SpecModel::SchLib(s) => assert_eq!(s.components.len(), 1),
            _ => panic!(),
        }
        // PcbLib: only footprint
        let pcblib = compile_spec(&file, SpecDomain::PcbLib).unwrap();
        match pcblib {
            SpecModel::PcbLib(p) => assert_eq!(p.footprints.len(), 1),
            _ => panic!(),
        }
    }

    // ── Layout expansion: Row / Column ─────────────────────────────────────

    #[test]
    fn absolute_row_right_direction() {
        let src = r#"
            footprint "CONN_4" {
                row {
                    at: (0mil, 0mil)
                    pitch: 100mil
                    count: 4
                    start: 1
                    direction: "right"
                    pad: { shape: "round", x_size: 60mil, y_size: 60mil }
                }
            }
        "#;
        let spec = compile_pcblib(src).unwrap();
        let fp = &spec.footprints[0];
        assert_eq!(fp.pads.len(), 4);
        // Pad 1 at (0, 0), pad 2 at (100mil, 0), pad 3 at (200mil, 0), pad 4 at (300mil, 0)
        assert_eq!(fp.pads[0].pad_name, "1");
        assert_eq!(fp.pads[0].at.x, Coord::from_mils(0));
        assert_eq!(fp.pads[1].pad_name, "2");
        assert_eq!(fp.pads[1].at.x, Coord::from_mils(100));
        assert_eq!(fp.pads[2].pad_name, "3");
        assert_eq!(fp.pads[3].pad_name, "4");
        assert_eq!(fp.pads[3].at.x, Coord::from_mils(300));
    }

    #[test]
    fn absolute_row_down_direction() {
        let src = r#"
            footprint "COL_3" {
                row {
                    at: (0mil, 100mil)
                    pitch: 50mil
                    count: 3
                    start: 1
                    direction: "down"
                    pad: { shape: "round" }
                }
            }
        "#;
        let spec = compile_pcblib(src).unwrap();
        let fp = &spec.footprints[0];
        assert_eq!(fp.pads.len(), 3);
        assert_eq!(fp.pads[0].at.y, Coord::from_mils(100));
        assert_eq!(fp.pads[1].at.y, Coord::from_mils(50));
        assert_eq!(fp.pads[2].at.y, Coord::from_mils(0));
    }

    #[test]
    fn row_skip_by_name() {
        let src = r#"
            footprint "CONN_SKIP" {
                row {
                    at: (0mil, 0mil)
                    pitch: 100mil
                    count: 5
                    start: 1
                    direction: "right"
                    skip: [3]
                    pad: { shape: "round" }
                }
            }
        "#;
        let spec = compile_pcblib(src).unwrap();
        let fp = &spec.footprints[0];
        // 5 positions, skip name "3" → emit 1, 2, 4, 5, 6
        let names: Vec<&str> = fp.pads.iter().map(|p| p.pad_name.as_str()).collect();
        assert_eq!(names, vec!["1", "2", "4", "5", "6"]);
    }

    #[test]
    fn row_explicit_pad_override() {
        let src = r#"
            footprint "OVERRIDE" {
                row {
                    at: (0mil, 0mil)
                    pitch: 100mil
                    count: 3
                    start: 1
                    direction: "right"
                    pad: { shape: "round", x_size: 50mil, y_size: 50mil }
                }
                pad 2 { shape: "rectangular", x_size: 80mil, y_size: 30mil }
            }
        "#;
        let spec = compile_pcblib(src).unwrap();
        let fp = &spec.footprints[0];
        assert_eq!(fp.pads.len(), 3);
        // Pad 1: round from template
        assert_eq!(fp.pads[0].shape, Some(PadShape::Round));
        assert_eq!(fp.pads[0].x_size, Some(Coord::from_mils(50)));
        // Pad 2: overridden to rectangular with explicit size
        assert_eq!(fp.pads[1].shape, Some(PadShape::Rectangular));
        assert_eq!(fp.pads[1].x_size, Some(Coord::from_mils(80)));
        assert_eq!(fp.pads[1].y_size, Some(Coord::from_mils(30)));
        // Position from layout, not from explicit pad
        assert_eq!(fp.pads[1].at.x, Coord::from_mils(100));
        // Pad 3: round from template
        assert_eq!(fp.pads[2].shape, Some(PadShape::Round));
    }

    #[test]
    fn column_same_as_row() {
        // column is an alias for row
        let src = r#"
            footprint "COL" {
                column {
                    at: (0mil, 0mil)
                    pitch: 100mil
                    count: 2
                    start: 1
                    direction: "right"
                    pad: { shape: "round" }
                }
            }
        "#;
        let spec = compile_pcblib(src).unwrap();
        assert_eq!(spec.footprints[0].pads.len(), 2);
    }

    // ── Layout expansion: Grid ─────────────────────────────────────────────

    #[test]
    fn grid_numeric_naming() {
        let src = r#"
            footprint "BGA_2x2" {
                grid {
                    origin: (0mil, 0mil)
                    rows: 2
                    cols: 2
                    pitch: 100mil
                    naming: "numeric"
                    pad: { shape: "round", x_size: 40mil, y_size: 40mil }
                }
            }
        "#;
        let spec = compile_pcblib(src).unwrap();
        let fp = &spec.footprints[0];
        assert_eq!(fp.pads.len(), 4);
        let names: Vec<&str> = fp.pads.iter().map(|p| p.pad_name.as_str()).collect();
        assert_eq!(names, vec!["1", "2", "3", "4"]);
    }

    #[test]
    fn grid_alphanumeric_naming() {
        let src = r#"
            footprint "BGA_2x3" {
                grid {
                    origin: (0mil, 0mil)
                    rows: 2
                    cols: 3
                    pitch: 100mil
                    naming: "alphanumeric"
                    pad: { shape: "round" }
                }
            }
        "#;
        let spec = compile_pcblib(src).unwrap();
        let fp = &spec.footprints[0];
        assert_eq!(fp.pads.len(), 6);
        let names: Vec<&str> = fp.pads.iter().map(|p| p.pad_name.as_str()).collect();
        // Row 0 = A, Row 1 = B; col 1..3
        assert_eq!(names, vec!["A1", "A2", "A3", "B1", "B2", "B3"]);
    }

    #[test]
    fn grid_bga_letter_skips_i_o_q_s_x_z() {
        // A(0) B(1) C(2) D(3) E(4) F(5) G(6) H(7) J(8) K(9) L(10) M(11) N(12) P(13) R(14) T(15)...
        assert_eq!(bga_row_letter(7), "H");
        assert_eq!(bga_row_letter(8), "J"); // I is skipped
        assert_eq!(bga_row_letter(9), "K");
        assert_eq!(bga_row_letter(13), "P"); // O is skipped before P
        assert_eq!(bga_row_letter(14), "R"); // Q is skipped before R
    }

    #[test]
    fn grid_skip_by_name() {
        let src = r#"
            footprint "SKIP_GRID" {
                grid {
                    origin: (0mil, 0mil)
                    rows: 2
                    cols: 2
                    pitch: 100mil
                    naming: "alphanumeric"
                    skip: ["A2", "B1"]
                    pad: { shape: "round" }
                }
            }
        "#;
        let spec = compile_pcblib(src).unwrap();
        let fp = &spec.footprints[0];
        let names: Vec<&str> = fp.pads.iter().map(|p| p.pad_name.as_str()).collect();
        assert_eq!(names, vec!["A1", "B2"]);
    }

    #[test]
    fn grid_perimeter_only() {
        let src = r#"
            footprint "PERI" {
                grid {
                    origin: (0mil, 0mil)
                    rows: 3
                    cols: 3
                    pitch: 100mil
                    naming: "numeric"
                    perimeter_only: true
                    pad: { shape: "round" }
                }
            }
        "#;
        let spec = compile_pcblib(src).unwrap();
        let fp = &spec.footprints[0];
        // 3x3 = 9 total, perimeter = 8 (skip center position)
        assert_eq!(fp.pads.len(), 8);
        // Center pad (row=1, col=1) should be skipped - it would be pad "5" in row-major numeric
        let names: Vec<&str> = fp.pads.iter().map(|p| p.pad_name.as_str()).collect();
        assert!(!names.contains(&"5")); // center skipped
    }

    #[test]
    fn grid_asymmetric_pitch() {
        let src = r#"
            footprint "ASYM" {
                grid {
                    origin: (0mil, 0mil)
                    rows: 1
                    cols: 2
                    pitch_x: 200mil
                    pitch_y: 100mil
                    naming: "numeric"
                    pad: { shape: "round" }
                }
            }
        "#;
        let spec = compile_pcblib(src).unwrap();
        let fp = &spec.footprints[0];
        assert_eq!(fp.pads.len(), 2);
        // cols=2: col 0 offset = (0 - 0.5)*200mil = -100mil, col 1 = +100mil
        assert_eq!(fp.pads[0].at.x, Coord::from_mils(-100));
        assert_eq!(fp.pads[1].at.x, Coord::from_mils(100));
    }
}
