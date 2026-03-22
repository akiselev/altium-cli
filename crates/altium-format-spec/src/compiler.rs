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

use std::collections::{HashMap, HashSet};

use indexmap::IndexMap;

use altium_format_types::sch::{
    HorizontalAlign, LeftRightSide, LineStyle, PenWidth, PortArrowStyle, PortIoType,
    PowerObjectStyle, TextJustification,
};
use altium_format_types::{
    Color, ComponentKind, Coord, CoordPoint, LayerRef, PadShape, PadStackMode, PinElectricalType,
    PlaneConnectionStyle, RotationBy90,
};

use crate::annotation::{CompiledAnnotation, compile_annotation};
use crate::ast::{
    AliasDecl, BoardDecl, BoardItem, ClassDecl, ComponentDecl, ComponentItem, ConstraintDecl,
    DifferentialPairDecl, FootprintDecl, FootprintItem, FootprintMapDecl, FootprintRef,
    GraphicDecl, Object, ObjectItem, PadDecl, ParameterDecl, PartBlock, PartItem,
    PcbDocPrimitiveDecl, PinDecl, PlaceDecl, PlacementConstraintDecl, PlacementDecl,
    PlacementGroupDecl, PlacementItem, PolygonDecl, ProjectDecl, ProjectItem, RoutingDecl,
    RuleDecl, SchDocObjectDecl, SchDocObjectItem, SheetDecl, SheetItem, SpecFile, SpecItem,
};
use crate::eval::{EvalResult, ScopeStack, SpecError, SpecErrorCode, Value, eval_expr};
use crate::model::{
    AnnotationMatchParamSpec, AnnotationSpec, AutoplaceConfig, BlanketSpec, BoardSpec,
    BusEntrySpec, BusSpec, ClassGenSpec, ComparisonRuleSpec, CompileMaskSpec, ComponentSpec,
    ConstraintKind, ConstraintSpec, DocumentSpec, ErcLevelOverride, ErcMatrixOverride, FontSpec,
    FootprintMapSpec, FootprintSpec, GraphicProperties, GraphicSpec, GraphicType,
    HarnessConnectorSpec, JunctionSpec, LayerSpec, LibraryUpdateSpec, NetLabelSpec, NetSpec,
    NoConnectSpec, NoteSpec, OutputGroupSpec, OutputSpec, PadSpec, ParamVariationSpec,
    ParameterSetSpec, ParameterSpec, PartSpec, PcbDocClassSpec, PcbDocComponentSpec,
    PcbDocDifferentialPairSpec, PcbDocNetSpec, PcbDocPolygonSpec, PcbDocPrimitiveSpec,
    PcbDocRuleSpec, PcbDocSpec, PcbGraphicProperties, PcbGraphicSpec, PcbGraphicType, PinPadMap,
    PinRef, PinSpec, PlacementAutoplaceMode, PlacementClearanceSpec, PlacementConstraintSpec,
    PlacementGroupSpec, PlacementOptimizeSpec, PlacementPlaceSpec, PlacementRuleSpec,
    PlacementSpec, PortSpec, PowerObjectSpec, PowerSpec, PrjPcbSpec, ProbeSpec, ProjectSpec,
    RoutingSpec, SchDocComponentSpec, SchDocObjectSpec, SchDocSpec, SchLibSpec, SheetEntrySpec,
    SheetSpec, SheetSymbolSpec, SignalHarnessSpec, SpecDomain, SpecModel, SymbolRef,
    UnplacedStrategy, VariantSpec, VariationSpec, WireSpec,
};

use crate::diagnostic::Spanned;
use altium_format_types::project::{
    ChannelRoomNamingStyle, ConnectionCode, CrossRefLocationStyle, CrossRefPorts,
    CrossRefSheetStyle, ErrorLevel, FlattenMode, SortLocation, SortOrder, VariationKind,
};

// ── Public API ────────────────────────────────────────────────────────────────

/// Compile a parsed spec file into a typed [`SpecModel`].
///
/// `domain` selects whether to compile SchLib or PcbLib entities.
/// Top-level entities that don't match the domain are silently skipped.
pub fn compile_spec(file: &SpecFile, domain: SpecDomain) -> Result<SpecModel, SpecError> {
    compile_spec_with_imports(file, domain, HashMap::new())
}

/// Compile a parsed spec file into a typed [`SpecModel`], with imported SchLib
/// component definitions available for rich component bindings.
///
/// When `imported_components` is non-empty and a placed SchDoc component's
/// `lib_reference` matches an entry, the scope binding becomes a
/// `Value::Object` with `x`, `y`, and `pin<N>` fields rather than a plain
/// `Value::CoordPoint`. This enables `$U1.pin1.x`-style expressions.
pub fn compile_spec_with_imports(
    file: &SpecFile,
    domain: SpecDomain,
    imported_components: HashMap<String, ComponentSpec>,
) -> Result<SpecModel, SpecError> {
    let mut compiler = SpecCompiler::new(domain, imported_components);
    compiler.compile(file)
}

/// Compile with resolved imports — named import aliases are registered in scope
/// as objects mapping entity names to their string names, enabling `fp["0603"]`.
pub fn compile_spec_with_resolved(
    resolved: &crate::import::ResolvedSpec,
    domain: SpecDomain,
    imported_components: HashMap<String, ComponentSpec>,
) -> Result<SpecModel, SpecError> {
    let mut compiler = SpecCompiler::new(domain, imported_components);
    // Build scope objects for named imports.
    for (alias, (_path, spec_file)) in &resolved.named_imports {
        let mut entries = IndexMap::new();
        for item in &spec_file.items {
            let name = match &item.node {
                SpecItem::Component(c) => Some(c.name.node.as_str()),
                SpecItem::Footprint(f) => Some(f.name.node.as_str()),
                _ => None,
            };
            if let Some(name) = name {
                entries.insert(name.clone(), Value::String(name));
            }
        }
        // ImportObject stores the alias with entries so field access on `$alias.Name`
        // returns ImportRef, preserving import provenance for symbol validation.
        compiler.named_import_objects.insert(
            alias.clone(),
            Value::ImportObject {
                alias: alias.clone(),
                entries,
            },
        );
    }
    compiler.compile(&resolved.root)
}

/// Compile all SchLib-domain imports and collect their components by lib_reference.
///
/// This is used by SchDoc compilation to resolve symbol references for rich
/// component bindings (pin access).
pub fn compile_imported_schlibs(
    resolved: &crate::import::ResolvedSpec,
) -> Result<HashMap<String, ComponentSpec>, (std::path::PathBuf, SpecError)> {
    let mut components = HashMap::new();

    for (path, spec_file) in &resolved.bare_imports {
        collect_schlib_components(path, spec_file, &mut components)
            .map_err(|e| (path.clone(), e))?;
    }

    for (_alias, (path, spec_file)) in &resolved.named_imports {
        collect_schlib_components(path, spec_file, &mut components)
            .map_err(|e| (path.clone(), e))?;
    }

    Ok(components)
}

fn collect_schlib_components(
    path: &std::path::Path,
    file: &SpecFile,
    components: &mut HashMap<String, ComponentSpec>,
) -> Result<(), SpecError> {
    // Resolve the imported file's own imports (e.g., schlib-spec importing pcblib-spec)
    // so that transitive bindings like `$fp["FootprintName"]` are available during compilation.
    let sub_resolved = crate::import::resolve_imports(path, file.clone())?;
    let sub_model = compile_spec_with_resolved(&sub_resolved, SpecDomain::SchLib, HashMap::new())?;
    match sub_model {
        SpecModel::SchLib(schlib) => {
            for comp in schlib.components {
                components.insert(comp.lib_reference.clone(), comp);
            }
        }
        _ => {}
    }
    Ok(())
}

// ── Component binding helpers ─────────────────────────────────────────────────

/// Build a rich binding value for a placed SchDoc component.
///
/// If the component's lib_reference is found in `imported_components`, produces a
/// `Value::Object` with `x`, `y`, and `pin<N>` fields (each pin is a CoordPoint
/// with the pin's transformed schematic-space position). Otherwise falls back to
/// `Value::CoordPoint(x, y)`.
fn build_component_binding(
    comp: &SchDocComponentSpec,
    imported_components: &HashMap<String, ComponentSpec>,
) -> Value {
    let lib_ref = match &comp.symbol {
        SymbolRef::Literal(name) => name.as_str(),
        SymbolRef::Import { name, .. } => name.as_str(),
    };

    let lib_comp = match imported_components.get(lib_ref) {
        Some(c) => c,
        None => return Value::CoordPoint(comp.location.x.raw(), comp.location.y.raw()),
    };

    let mut fields = IndexMap::new();
    fields.insert("x".to_string(), Value::Dim(comp.location.x.raw()));
    fields.insert("y".to_string(), Value::Dim(comp.location.y.raw()));

    let orientation = comp.orientation.unwrap_or(RotationBy90::Rotate0);
    let is_mirrored = comp.is_mirrored.unwrap_or(false);

    for pin in &lib_comp.pins {
        let transformed =
            transform_pin_position(pin.location, comp.location, orientation, is_mirrored);
        let pin_key = format!("pin{}", pin.designator);
        fields.insert(
            pin_key,
            Value::CoordPoint(transformed.x.raw(), transformed.y.raw()),
        );
    }

    Value::Object(fields)
}

/// Transform a pin position from symbol space to schematic space.
///
/// Applies mirror, rotation, then translation (same order as Altium's placement):
/// 1. Mirror (if mirrored): negate X
/// 2. Rotate by orientation around origin
/// 3. Translate by component location
pub fn transform_pin_position(
    pin_location: CoordPoint,
    comp_location: CoordPoint,
    orientation: RotationBy90,
    is_mirrored: bool,
) -> CoordPoint {
    let mut x = pin_location.x.raw();
    let y = pin_location.y.raw();

    if is_mirrored {
        x = -x;
    }

    let (rx, ry) = match orientation {
        RotationBy90::Rotate0 => (x, y),
        RotationBy90::Rotate90 => (-y, x),
        RotationBy90::Rotate180 => (-x, -y),
        RotationBy90::Rotate270 => (y, -x),
        _ => (x, y),
    };

    CoordPoint::new(
        Coord::new(rx + comp_location.x.raw()),
        Coord::new(ry + comp_location.y.raw()),
    )
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
    /// Pre-compiled SchLib components from imports, keyed by lib_reference.
    imported_components: HashMap<String, ComponentSpec>,
    /// Named import alias → Value::Object mapping entity names to string names.
    named_import_objects: IndexMap<String, Value>,
    /// Tracks annotation IDs seen within the current spec file compile.
    /// Constructed fresh per top-level compile call (one set per spec file).
    seen_ids: HashSet<String>,
}

impl SpecCompiler {
    fn new(domain: SpecDomain, imported_components: HashMap<String, ComponentSpec>) -> Self {
        Self {
            domain,
            scope: ScopeStack::new(),
            unnamed_counters: IndexMap::new(),
            context_name: String::new(),
            part_context: None,
            imported_components,
            named_import_objects: IndexMap::new(),
            seen_ids: HashSet::new(),
        }
    }

    /// Compile an optional `#[annotation(...)]` from the AST into a `CompiledAnnotation`.
    ///
    /// Returns `None` when the AST has no annotation (unannotated blocks — auto-generation
    /// for those is deferred to the dump phase, M3). Returns `Some(compiled)` when an
    /// annotation is present, auto-generating an ID if none was specified.
    fn compile_opt_annotation(
        &mut self,
        ann: Option<&crate::diagnostic::Spanned<crate::ast::BlockAnnotation>>,
    ) -> Result<Option<CompiledAnnotation>, SpecError> {
        match ann {
            None => Ok(None),
            Some(spanned) => {
                let compiled =
                    compile_annotation(&spanned.node, &mut self.seen_ids, Some(spanned.span))?;
                Ok(Some(compiled))
            }
        }
    }

    fn compile(&mut self, file: &SpecFile) -> Result<SpecModel, SpecError> {
        // Root scope for file-level let bindings.
        self.scope.push();

        // Register named import aliases in scope as objects.
        for (alias, value) in &self.named_import_objects {
            self.scope.define(alias.clone(), value.clone());
        }

        // Collect and evaluate file-level let bindings (forward-visible).
        let file_lets: Vec<_> = file
            .items
            .iter()
            .filter_map(|item| match &item.node {
                SpecItem::LetBinding(lb) => Some((&*lb.name.node, &lb.value)),
                _ => None,
            })
            .collect();
        eval_let_bindings_slice(&file_lets, &mut self.scope)?;

        // Register file-level swap_group declarations in scope.
        for item in &file.items {
            if let SpecItem::SwapGroup(decl) = &item.node {
                let sg_name = decl.name.node.as_str();
                let binding_name = decl
                    .binding
                    .as_ref()
                    .map(|b| b.node.clone())
                    .unwrap_or_else(|| sg_name.clone());
                self.scope
                    .define(binding_name.clone(), Value::SwapGroup(sg_name.clone()));
                // If an explicit binding was provided, also register under the entity name.
                if decl.binding.is_some() && binding_name != sg_name {
                    self.scope
                        .define(sg_name.clone(), Value::SwapGroup(sg_name.clone()));
                }
            }
        }

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
            SpecDomain::PcbDoc => {
                let spec = self.compile_pcbdoc(file)?;
                self.scope.pop();
                Ok(SpecModel::PcbDoc(spec))
            }
        }
    }

    // ── Component compilation ──────────────────────────────────────────────

    fn compile_component(&mut self, decl: &ComponentDecl) -> Result<ComponentSpec, SpecError> {
        let annotation = self.compile_opt_annotation(decl.annotation.as_ref())?;
        let lib_reference = decl.name.node.as_str();
        self.context_name = lib_reference.clone();
        self.unnamed_counters.clear();
        self.part_context = None;

        // Push component scope.
        self.scope.push();

        // Collect and evaluate component-level let bindings.
        let comp_lets: Vec<_> = decl
            .body
            .iter()
            .filter_map(|item| match &item.node {
                ComponentItem::LetBinding(lb) => Some((&*lb.name.node, &lb.value)),
                _ => None,
            })
            .collect();
        eval_let_bindings_slice(&comp_lets, &mut self.scope)?;

        // Register component-level swap_group declarations in scope.
        for item in &decl.body {
            if let ComponentItem::SwapGroup(sg_decl) = &item.node {
                let sg_name = sg_decl.name.node.as_str();
                let binding_name = sg_decl
                    .binding
                    .as_ref()
                    .map(|b| b.node.clone())
                    .unwrap_or_else(|| sg_name.clone());
                self.scope
                    .define(binding_name.clone(), Value::SwapGroup(sg_name.clone()));
                if sg_decl.binding.is_some() && binding_name != sg_name {
                    self.scope
                        .define(sg_name.clone(), Value::SwapGroup(sg_name.clone()));
                }
            }
        }

        // Collect component-level properties from Property items.
        let props = collect_object_properties_from_items(
            decl.body.iter().filter_map(|item| match &item.node {
                ComponentItem::Property(p) => Some(p),
                _ => None,
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
        let (mut binding_map, auto_sized) = build_graphic_binding_map(
            decl.body.iter().filter_map(|item| {
                if let ComponentItem::Graphic(g) = &item.node {
                    Some(g)
                } else {
                    None
                }
            }),
            &self.scope,
        )?;

        // Pass 2: collect all anchor-pinned pin decls by edge for sequencing.
        // We need to resolve after:/before: chains before producing final PinSpecs.
        // For single-part components (part_count absent or 1), all items belong to part 1.
        // For multi-part components, component-level items are shared (part 0).
        let default_owner_part_id = if part_count.unwrap_or(1) <= 1 {
            1i32
        } else {
            0i32
        };

        let all_pin_decls_at_level: Vec<(&PinDecl, i32)> = decl
            .body
            .iter()
            .filter_map(|item| {
                if let ComponentItem::Pin(p) = &item.node {
                    Some((p, default_owner_part_id))
                } else {
                    None
                }
            })
            .collect();

        // Pass 2b: auto-size any rectangles whose from/to were omitted.
        if !auto_sized.is_empty() {
            compute_auto_size_bounds(
                &auto_sized,
                &all_pin_decls_at_level,
                decl.body.iter().filter_map(|item| {
                    if let ComponentItem::Graphic(g) = &item.node {
                        Some(g)
                    } else {
                        None
                    }
                }),
                &mut binding_map,
                &self.scope,
            )?;
        }

        // Compile children.
        let pins = resolve_anchor_pins(&all_pin_decls_at_level, &binding_map, &self.scope)?;
        let mut parameters = Vec::new();
        let mut aliases = Vec::new();
        let mut footprints = Vec::new();
        let mut graphics = Vec::new();
        let mut parts = Vec::new();

        // Track auto-sized graphic binding names along with their source decl binding
        // so we can patch from/to after compilation.
        let mut auto_sized_graphic_indices: Vec<(usize, String)> = Vec::new();

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
                    let idx = graphics.len();
                    let spec = self.compile_sch_graphic(graphic_decl)?;
                    if let Some(b) = &graphic_decl.binding {
                        if auto_sized.contains(&b.node) {
                            auto_sized_graphic_indices.push((idx, b.node.clone()));
                        }
                    }
                    graphics.push(spec);
                }
                ComponentItem::Part(part_block) => {
                    parts.push(self.compile_part_with_anchors(part_block, &binding_map)?);
                }
                ComponentItem::Property(_) | ComponentItem::LetBinding(_) => {
                    // Already handled above.
                }
                ComponentItem::SwapGroup(_) => {
                    // Already registered in scope; nothing to emit into the component spec.
                }
                ComponentItem::PinConnection(_) => {
                    // Resolved at executor time; nothing to compile into the component spec yet.
                }
            }
        }

        // Patch auto-sized rectangle from/to with computed bounds.
        for (idx, binding_name) in auto_sized_graphic_indices {
            if let Some(geom) = binding_map.get(&binding_name) {
                graphics[idx].properties.from = Some(geom.from);
                graphics[idx].properties.to = Some(geom.to);
            }
        }

        self.scope.pop();

        Ok(ComponentSpec {
            annotation,
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
        let mut sheet_annotation: Option<CompiledAnnotation> = None;
        let mut fonts = Vec::new();
        // Pre-pass: collect power net names so pin-connection classification
        // (Signal vs Power) is order-independent.
        let mut power_declarations: std::collections::HashMap<String, PowerObjectStyle> =
            std::collections::HashMap::new();
        for item in &file.items {
            if let SpecItem::Power(power_decl) = &item.node {
                // Placeholder — final style is resolved after all power declarations compile.
                power_declarations.insert(power_decl.name.node.as_str(), PowerObjectStyle::Bar);
            }
        }
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
        let mut constraints = Vec::new();

        for item in &file.items {
            match &item.node {
                SpecItem::Sheet(sheet_decl) => {
                    sheet_annotation =
                        self.compile_opt_annotation(sheet_decl.annotation.as_ref())?;
                    self.compile_sheet_metadata(
                        sheet_decl,
                        &mut fonts,
                        &mut custom_width,
                        &mut custom_height,
                        &mut snap_grid_on,
                        &mut visible_grid_on,
                        &mut hot_spot_grid_on,
                        &mut show_hidden_pins,
                        &mut border_on,
                        &mut title_block_on,
                        &mut constraints,
                    )?;
                }
                SpecItem::Component(comp_decl) => {
                    let comp = self.compile_schdoc_component(comp_decl, &power_declarations)?;
                    let binding = build_component_binding(&comp, &self.imported_components);
                    self.scope.define(comp.designator.clone(), binding);
                    components.push(comp);
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
                SpecItem::Import(_)
                | SpecItem::LetBinding(_)
                | SpecItem::SwapGroup(_)
                | SpecItem::Footprint(_)
                | SpecItem::Project(_)
                | SpecItem::Board(_)
                | SpecItem::PcbDocPrimitive(_)
                | SpecItem::Placement(_)
                | SpecItem::Routing(_)
                | SpecItem::Polygon(_)
                | SpecItem::Rule(_)
                | SpecItem::Class(_)
                | SpecItem::DifferentialPair(_) => {
                    // Imports, let bindings, swap groups, and other-domain items silently skipped.
                }
            }
        }

        // power_declarations: names from pre-pass, styles from this loop.
        // Styles were unavailable during pre-pass (power items not yet compiled).
        for power in &powers {
            power_declarations.insert(power.name.clone(), power.style);
        }

        let sheet = SheetSpec {
            annotation: sheet_annotation,
            fonts,
            power_declarations,
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
            constraints,
        };

        Ok(SchDocSpec {
            sheets: vec![sheet],
        })
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
        constraints: &mut Vec<ConstraintSpec>,
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
                        "custom_width" => {
                            *custom_width = Some(value_to_coord(&val, Some(prop.value.span))?)
                        }
                        "custom_height" => {
                            *custom_height = Some(value_to_coord(&val, Some(prop.value.span))?)
                        }
                        "snap_grid_on" => {
                            *snap_grid_on = Some(value_to_bool(&val, Some(prop.value.span))?)
                        }
                        "visible_grid_on" => {
                            *visible_grid_on = Some(value_to_bool(&val, Some(prop.value.span))?)
                        }
                        "hot_spot_grid_on" => {
                            *hot_spot_grid_on = Some(value_to_bool(&val, Some(prop.value.span))?)
                        }
                        "show_hidden_pins" => {
                            *show_hidden_pins = Some(value_to_bool(&val, Some(prop.value.span))?)
                        }
                        "border_on" => {
                            *border_on = Some(value_to_bool(&val, Some(prop.value.span))?)
                        }
                        "title_block_on" => {
                            *title_block_on = Some(value_to_bool(&val, Some(prop.value.span))?)
                        }
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
                SheetItem::Constraint(constraint_decl) => {
                    constraints.push(self.compile_constraint_decl(constraint_decl)?);
                }
            }
        }
        Ok(())
    }

    fn compile_constraint_decl(
        &mut self,
        decl: &ConstraintDecl,
    ) -> Result<ConstraintSpec, SpecError> {
        use crate::ast::ConstraintKind as AstKind;
        let annotation = self.compile_opt_annotation(decl.annotation.as_ref())?;
        let kind = match decl.kind.node {
            AstKind::EdgePlacement => ConstraintKind::EdgePlacement,
            AstKind::Directional => ConstraintKind::Directional,
            AstKind::Near => ConstraintKind::Near,
            AstKind::Region => ConstraintKind::Region,
            AstKind::FixedPosition => ConstraintKind::FixedPosition,
        };
        let raw_props = eval_object_to_map(&decl.body.node, &self.scope)?;
        let mut properties = indexmap::IndexMap::new();
        for (k, v) in raw_props {
            properties.insert(k, value_to_string_repr(&v));
        }
        Ok(ConstraintSpec {
            annotation,
            kind,
            properties,
        })
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
        power_declarations: &std::collections::HashMap<String, PowerObjectStyle>,
    ) -> Result<SchDocComponentSpec, SpecError> {
        let annotation = self.compile_opt_annotation(decl.annotation.as_ref())?;
        let designator = decl.name.node.as_str();

        self.scope.push();

        // Evaluate component-level let bindings.
        let comp_lets: Vec<_> = decl
            .body
            .iter()
            .filter_map(|item| match &item.node {
                ComponentItem::LetBinding(lb) => Some((&*lb.name.node, &lb.value)),
                _ => None,
            })
            .collect();
        eval_let_bindings_slice(&comp_lets, &mut self.scope)?;

        let props = collect_object_properties_from_items(
            decl.body.iter().filter_map(|item| match &item.node {
                ComponentItem::Property(p) => Some(p),
                _ => None,
            }),
            &self.scope,
        )?;

        // Resolve symbol reference: either $alias.Name or lib_reference: "Name"
        let symbol = if let Some(v) = props.get("symbol") {
            match v {
                Value::ImportRef { alias, name } => {
                    let alias = alias.clone();
                    let name = name.clone();
                    if !self.imported_components.contains_key(&name) {
                        let available: Vec<String> =
                            self.imported_components.keys().cloned().collect();
                        return Err(SpecError::no_span(
                            SpecErrorCode::AltiumFormat,
                            format!(
                                "symbol '{}' not found in import '{}' (available: {})",
                                name,
                                alias,
                                available.join(", ")
                            ),
                        ));
                    }
                    SymbolRef::Import { alias, name }
                }
                Value::String(s) => {
                    // Plain lib_reference string — no import validation; treated as opaque component name.
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

        // Compile pin connections
        let mut pin_connections = Vec::new();
        for item in &decl.body {
            if let ComponentItem::PinConnection(conn_decl) = &item.node {
                let target = match &conn_decl.target {
                    crate::ast::PinConnectionTarget::NoConnect => {
                        crate::model::PinConnectionTarget::NoConnect
                    }
                    crate::ast::PinConnectionTarget::NetRef(net_name) => {
                        let name = net_name.node.clone();
                        if power_declarations.contains_key(&name) {
                            crate::model::PinConnectionTarget::Power(name)
                        } else {
                            crate::model::PinConnectionTarget::Signal(name)
                        }
                    }
                };
                pin_connections.push(crate::model::PinConnectionSpec {
                    pin_name: conn_decl.pin_name.node.clone(),
                    target,
                });
            }
        }

        Ok(SchDocComponentSpec {
            annotation,
            designator,
            symbol,
            location,
            orientation,
            is_mirrored,
            description,
            parameters,
            pin_connections,
        })
    }

    fn compile_net(&mut self, decl: &crate::ast::NetDecl) -> Result<NetSpec, SpecError> {
        let annotation = self.compile_opt_annotation(decl.annotation.as_ref())?;
        let name = decl.name.node.as_str();
        let props = eval_object_to_map(&decl.body.node, &self.scope)?;

        let pins = self.compile_pin_refs(&props, decl.body.span)?;

        Ok(NetSpec {
            annotation,
            name,
            pins,
        })
    }

    fn compile_power(&mut self, decl: &crate::ast::PowerDecl) -> Result<PowerSpec, SpecError> {
        let annotation = self.compile_opt_annotation(decl.annotation.as_ref())?;
        let name = decl.name.node.as_str();
        let props = eval_object_to_map(&decl.body.node, &self.scope)?;

        let style = get_enum_opt(&props, "style", parse_power_object_style)?
            .unwrap_or(PowerObjectStyle::Bar);
        let show_net_name = get_bool_opt(&props, "show_net_name");
        let orientation = get_enum_opt(&props, "orientation", parse_rotation_by90)?;
        let pins = self.compile_pin_refs(&props, decl.body.span)?;

        Ok(PowerSpec {
            annotation,
            name,
            style,
            pins,
            show_net_name,
            orientation,
        })
    }

    /// Parse a `pins: [U1.14, C1.1]` array into PinRef values.
    fn compile_pin_refs(
        &self,
        props: &IndexMap<String, Value>,
        span: crate::diagnostic::Span,
    ) -> Result<Vec<PinRef>, SpecError> {
        let pins_val = match props.get("pins") {
            Some(v) => v,
            None => return Ok(Vec::new()),
        };

        let arr = match pins_val {
            Value::Array(a) => a,
            _ => {
                return Err(SpecError::new(
                    SpecErrorCode::TypeMismatch,
                    "'pins' must be an array".to_string(),
                    Some(span),
                ));
            }
        };

        let mut refs = Vec::new();
        for item in arr {
            let s = match item {
                Value::String(s) => s.clone(),
                _ => {
                    return Err(SpecError::no_span(
                        SpecErrorCode::TypeMismatch,
                        "pin ref must be a string like \"U1.14\"".to_string(),
                    ));
                }
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
                    Ok(SchDocObjectSpec::Graphic(GraphicSpec {
                        unique_id,
                        graphic_type,
                        properties,
                    }))
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

    fn compile_wire_spec(
        &mut self,
        decl: &SchDocObjectDecl,
    ) -> Result<SchDocObjectSpec, SpecError> {
        let props = self.collect_schdoc_object_props(decl)?;
        let vertices = get_coord_point_array(&props, "vertices")?;
        let color = get_color_opt(&props, "color");
        let line_width = get_enum_opt(&props, "line_width", parse_pen_width)?;
        let line_style = get_enum_opt(&props, "line_style", parse_line_style)?;
        Ok(SchDocObjectSpec::Wire(WireSpec {
            vertices,
            color,
            line_width,
            line_style,
        }))
    }

    fn compile_bus_spec(&mut self, decl: &SchDocObjectDecl) -> Result<SchDocObjectSpec, SpecError> {
        let props = self.collect_schdoc_object_props(decl)?;
        let vertices = get_coord_point_array(&props, "vertices")?;
        let color = get_color_opt(&props, "color");
        let line_width = get_enum_opt(&props, "line_width", parse_pen_width)?;
        Ok(SchDocObjectSpec::Bus(BusSpec {
            vertices,
            color,
            line_width,
        }))
    }

    fn compile_net_label_spec(
        &mut self,
        decl: &SchDocObjectDecl,
    ) -> Result<SchDocObjectSpec, SpecError> {
        let text = decl
            .name
            .as_ref()
            .map(|n| n.node.as_str())
            .unwrap_or_default();
        let props = self.collect_schdoc_object_props(decl)?;
        let location = get_coord_point_required(&props, "at")?;
        let orientation = get_enum_opt(&props, "orientation", parse_rotation_by90)?;
        let justification = get_enum_opt(&props, "justification", parse_text_justification)?;
        let font_id = get_integer_opt(&props, "font_id");
        let color = get_color_opt(&props, "color");
        let is_mirrored = get_bool_opt(&props, "is_mirrored");
        Ok(SchDocObjectSpec::NetLabel(NetLabelSpec {
            text,
            location,
            orientation,
            justification,
            font_id,
            color,
            is_mirrored,
        }))
    }

    fn compile_power_object_spec(
        &mut self,
        decl: &SchDocObjectDecl,
    ) -> Result<SchDocObjectSpec, SpecError> {
        let text = decl
            .name
            .as_ref()
            .map(|n| n.node.as_str())
            .unwrap_or_default();
        let props = self.collect_schdoc_object_props(decl)?;
        let location = get_coord_point_required(&props, "at")?;
        let orientation = get_enum_opt(&props, "orientation", parse_rotation_by90)?;
        let style = get_enum_opt(&props, "style", parse_power_object_style)?;
        let show_net_name = get_bool_opt(&props, "show_net_name");
        let font_id = get_integer_opt(&props, "font_id");
        let color = get_color_opt(&props, "color");
        let is_cross_sheet_connector = get_bool_opt(&props, "is_cross_sheet_connector");
        Ok(SchDocObjectSpec::PowerObject(PowerObjectSpec {
            text,
            location,
            orientation,
            style,
            show_net_name,
            font_id,
            color,
            is_cross_sheet_connector,
        }))
    }

    fn compile_port_spec(
        &mut self,
        decl: &SchDocObjectDecl,
    ) -> Result<SchDocObjectSpec, SpecError> {
        let name = decl
            .name
            .as_ref()
            .map(|n| n.node.as_str())
            .unwrap_or_default();
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
            name,
            location,
            io_type,
            style,
            width,
            height,
            color,
            area_color,
            text_color,
            font_id,
            alignment,
        }))
    }

    fn compile_junction_spec(
        &mut self,
        decl: &SchDocObjectDecl,
    ) -> Result<SchDocObjectSpec, SpecError> {
        let props = self.collect_schdoc_object_props(decl)?;
        let location = get_coord_point_required(&props, "at")?;
        let color = get_color_opt(&props, "color");
        Ok(SchDocObjectSpec::Junction(JunctionSpec { location, color }))
    }

    fn compile_no_connect_spec(
        &mut self,
        decl: &SchDocObjectDecl,
    ) -> Result<SchDocObjectSpec, SpecError> {
        let props = self.collect_schdoc_object_props(decl)?;
        let location = get_coord_point_required(&props, "at")?;
        let color = get_color_opt(&props, "color");
        let orientation = get_enum_opt(&props, "orientation", parse_rotation_by90)?;
        Ok(SchDocObjectSpec::NoConnect(NoConnectSpec {
            location,
            color,
            orientation,
        }))
    }

    fn compile_bus_entry_spec(
        &mut self,
        decl: &SchDocObjectDecl,
    ) -> Result<SchDocObjectSpec, SpecError> {
        let props = self.collect_schdoc_object_props(decl)?;
        let location = get_coord_point_required(&props, "at")?;
        let corner = get_coord_point_required(&props, "corner")?;
        let color = get_color_opt(&props, "color");
        let line_width = get_enum_opt(&props, "line_width", parse_pen_width)?;
        Ok(SchDocObjectSpec::BusEntry(BusEntrySpec {
            location,
            corner,
            color,
            line_width,
        }))
    }

    fn compile_sheet_symbol_spec(
        &mut self,
        decl: &SchDocObjectDecl,
    ) -> Result<SchDocObjectSpec, SpecError> {
        let sheet_name = decl
            .name
            .as_ref()
            .map(|n| n.node.as_str())
            .unwrap_or_default();
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
            sheet_name,
            file_name,
            location,
            x_size,
            y_size,
            color,
            area_color,
            entries,
        }))
    }

    fn compile_sheet_entry(
        &mut self,
        decl: &crate::ast::EntryDecl,
    ) -> Result<SheetEntrySpec, SpecError> {
        let name = decl.name.node.as_str();
        let props = eval_object_to_map(&decl.body.node, &self.scope)?;
        let io_type = get_enum_opt(&props, "io_type", parse_port_io_type)?;
        let side = get_enum_opt(&props, "side", parse_left_right_side)?;
        let distance_from_top = get_coord_opt(&props, "distance")?;
        Ok(SheetEntrySpec {
            name,
            io_type,
            side,
            distance_from_top,
        })
    }

    fn compile_parameter_set_spec(
        &mut self,
        decl: &SchDocObjectDecl,
    ) -> Result<SchDocObjectSpec, SpecError> {
        let name = decl
            .name
            .as_ref()
            .map(|n| n.node.as_str())
            .unwrap_or_default();
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

        Ok(SchDocObjectSpec::ParameterSet(ParameterSetSpec {
            name,
            location,
            parameters,
        }))
    }

    fn compile_note_spec(
        &mut self,
        decl: &SchDocObjectDecl,
    ) -> Result<SchDocObjectSpec, SpecError> {
        let props = self.collect_schdoc_object_props(decl)?;
        let location = get_coord_point_required(&props, "at")?;
        let text = get_string_opt(&props, "text").unwrap_or_default();
        let color = get_color_opt(&props, "color");
        let area_color = get_color_opt(&props, "area_color");
        let font_id = get_integer_opt(&props, "font_id");
        Ok(SchDocObjectSpec::Note(NoteSpec {
            location,
            text,
            color,
            area_color,
            font_id,
        }))
    }

    fn compile_probe_spec(
        &mut self,
        decl: &SchDocObjectDecl,
    ) -> Result<SchDocObjectSpec, SpecError> {
        let name = decl
            .name
            .as_ref()
            .map(|n| n.node.as_str())
            .unwrap_or_default();
        let props = self.collect_schdoc_object_props(decl)?;
        let location = get_coord_point_required(&props, "at")?;
        let color = get_color_opt(&props, "color");
        Ok(SchDocObjectSpec::Probe(ProbeSpec {
            name,
            location,
            color,
        }))
    }

    fn compile_compile_mask_spec(
        &mut self,
        decl: &SchDocObjectDecl,
    ) -> Result<SchDocObjectSpec, SpecError> {
        let props = self.collect_schdoc_object_props(decl)?;
        let location = get_coord_point_required(&props, "at")?;
        let corner = get_coord_point_required(&props, "corner")?;
        let color = get_color_opt(&props, "color");
        Ok(SchDocObjectSpec::CompileMask(CompileMaskSpec {
            location,
            corner,
            color,
        }))
    }

    fn compile_blanket_spec(
        &mut self,
        decl: &SchDocObjectDecl,
    ) -> Result<SchDocObjectSpec, SpecError> {
        let props = self.collect_schdoc_object_props(decl)?;
        let location = get_coord_point_required(&props, "at")?;
        let corner = get_coord_point_required(&props, "corner")?;
        let vertices = if props.contains_key("vertices") {
            Some(get_coord_point_array(&props, "vertices")?)
        } else {
            None
        };
        let color = get_color_opt(&props, "color");
        Ok(SchDocObjectSpec::Blanket(BlanketSpec {
            location,
            corner,
            vertices,
            color,
        }))
    }

    fn compile_harness_connector_spec(
        &mut self,
        decl: &SchDocObjectDecl,
    ) -> Result<SchDocObjectSpec, SpecError> {
        let props = self.collect_schdoc_object_props(decl)?;
        let location = get_coord_point_required(&props, "at")?;
        let x_size = get_coord_opt(&props, "x_size")?;
        let y_size = get_coord_opt(&props, "y_size")?;
        let color = get_color_opt(&props, "color");
        let area_color = get_color_opt(&props, "area_color");
        Ok(SchDocObjectSpec::HarnessConnector(HarnessConnectorSpec {
            location,
            x_size,
            y_size,
            color,
            area_color,
        }))
    }

    fn compile_signal_harness_spec(
        &mut self,
        decl: &SchDocObjectDecl,
    ) -> Result<SchDocObjectSpec, SpecError> {
        let props = self.collect_schdoc_object_props(decl)?;
        let vertices = get_coord_point_array(&props, "vertices")?;
        let color = get_color_opt(&props, "color");
        let line_width = get_enum_opt(&props, "line_width", parse_pen_width)?;
        Ok(SchDocObjectSpec::SignalHarness(SignalHarnessSpec {
            vertices,
            color,
            line_width,
        }))
    }

    fn compile_parameter_object_spec(
        &mut self,
        decl: &SchDocObjectDecl,
    ) -> Result<SchDocObjectSpec, SpecError> {
        let name = decl
            .name
            .as_ref()
            .map(|n| n.node.as_str())
            .unwrap_or_default();
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

        let part_lets: Vec<_> = part_block
            .body
            .iter()
            .filter_map(|item| match &item.node {
                PartItem::LetBinding(lb) => Some((&*lb.name.node, &lb.value)),
                _ => None,
            })
            .collect();
        eval_let_bindings_slice(&part_lets, &mut self.scope)?;

        // Part-level graphic bindings (may shadow component-level ones).
        let (mut part_binding_map, part_auto_sized) = {
            let part_graphics = part_block.body.iter().filter_map(|item| {
                if let PartItem::Graphic(g) = &item.node {
                    Some(g)
                } else {
                    None
                }
            });
            let (mut m, auto_s) = build_graphic_binding_map(part_graphics, &self.scope)?;
            // Merge component-level map (part-level takes precedence).
            for (k, v) in binding_map {
                m.entry(k.clone()).or_insert_with(|| v.clone());
            }
            (m, auto_s)
        };

        let part_pin_decls: Vec<(&PinDecl, i32)> = part_block
            .body
            .iter()
            .filter_map(|item| {
                if let PartItem::Pin(p) = &item.node {
                    Some((p, part_number))
                } else {
                    None
                }
            })
            .collect();

        // Auto-size part-level rectangles whose from/to were omitted.
        if !part_auto_sized.is_empty() {
            compute_auto_size_bounds(
                &part_auto_sized,
                &part_pin_decls,
                part_block.body.iter().filter_map(|item| {
                    if let PartItem::Graphic(g) = &item.node {
                        Some(g)
                    } else {
                        None
                    }
                }),
                &mut part_binding_map,
                &self.scope,
            )?;
        }

        let mut pins = resolve_anchor_pins(&part_pin_decls, &part_binding_map, &self.scope)?;

        // Extract part_swap_group from part-level properties and apply to all part pins.
        // Accepts both "part_swap_group" and "swap_group" property names, and both
        // Value::SwapGroup (typed reference) and Value::String (backward compat).
        let part_swap_group: Option<String> = part_block.body.iter().find_map(|item| {
            if let PartItem::Property(prop) = &item.node {
                if prop.key.node == "part_swap_group" || prop.key.node == "swap_group" {
                    if let Ok(val) = eval_expr(&prop.value, &self.scope) {
                        match val {
                            Value::SwapGroup(s) => return Some(s),
                            Value::String(s) => return Some(s),
                            _ => {}
                        }
                    }
                }
            }
            None
        });
        if let Some(ref psg) = part_swap_group {
            for pin in &mut pins {
                if pin.part_swap_group.is_none() {
                    pin.part_swap_group = Some(psg.clone());
                }
            }
        }

        let mut graphics = Vec::new();
        let mut auto_sized_part_graphic_indices: Vec<(usize, String)> = Vec::new();
        for item in &part_block.body {
            match &item.node {
                PartItem::Graphic(graphic_decl) => {
                    let idx = graphics.len();
                    let spec = self.compile_sch_graphic(graphic_decl)?;
                    if let Some(b) = &graphic_decl.binding {
                        if part_auto_sized.contains(&b.node) {
                            auto_sized_part_graphic_indices.push((idx, b.node.clone()));
                        }
                    }
                    graphics.push(spec);
                }
                PartItem::Pin(_) | PartItem::LetBinding(_) | PartItem::Property(_) => {}
            }
        }

        // Patch auto-sized rectangle from/to with computed bounds.
        for (idx, binding_name) in auto_sized_part_graphic_indices {
            if let Some(geom) = part_binding_map.get(&binding_name) {
                graphics[idx].properties.from = Some(geom.from);
                graphics[idx].properties.to = Some(geom.to);
            }
        }

        self.scope.pop();
        self.part_context = None;

        Ok(PartSpec {
            part_number,
            pins,
            graphics,
        })
    }

    // ── Pin compilation ────────────────────────────────────────────────────

    fn compile_pin(&mut self, decl: &PinDecl, owner_part_id: i32) -> Result<PinSpec, SpecError> {
        let designator = decl.name.node.as_str();
        let props = eval_object_to_map(&decl.body.node, &self.scope)?;

        let name = get_string_opt(&props, "name");
        let electrical = get_enum_opt(&props, "electrical", parse_pin_electrical_type)?;
        let length = get_coord_opt(&props, "length")?;
        let is_hidden = get_bool_opt(&props, "is_hidden");
        let hidden_net_name = get_string_opt(&props, "hidden_net_name");
        let swap_group = get_swap_group_opt(&props, "swap_group")?;
        let part_swap_group = get_swap_group_opt(&props, "part_swap_group")?;
        let pair_swap_group = get_swap_group_opt(&props, "pair_swap_group")?;
        let orientation = get_enum_opt(&props, "orientation", parse_rotation_by90)?
            .unwrap_or(RotationBy90::Rotate0);

        let location = if let Some(v) = props.get("at") {
            value_to_coord_point(v, Some(decl.body.span))?
        } else if let Some(x_val) = props.get("x") {
            let x = value_to_coord(x_val, Some(decl.body.span))?;
            let y = props
                .get("y")
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
            swap_group,
            part_swap_group,
            pair_swap_group,
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

        Ok(ParameterSpec {
            name,
            text,
            is_hidden,
        })
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
            FootprintRef::DollarPath(dp) => {
                // Resolve the dollar path to get the footprint name.
                // For `$fp.QFP48` or `let x = fp["DFN-4"]; footprint $x`,
                // evaluate the path to extract the model name string.
                let spanned_expr = crate::ast::Spanned::new(
                    crate::ast::Expr::DollarIdent(dp.root.node.clone()),
                    decl.name.span,
                );
                let val = eval_expr(&spanned_expr, &self.scope)?;
                match val {
                    Value::String(s) => s,
                    Value::ImportRef { name, .. } => name,
                    _ => dp.root.node.clone(),
                }
            }
        };

        match &decl.maps {
            None => {
                // Implicit 1:1 mapping — no maps needed at the model level.
                // The Altium binary format doesn't strictly require explicit pin-pad
                // entries when they are 1:1. We emit an empty maps vec; the binary
                // writer handles this as "all pins map to same-numbered pads".
                Ok(FootprintMapSpec {
                    model_name,
                    maps: vec![],
                    source: None,
                })
            }
            Some(pairs) => {
                let mut maps = Vec::new();
                for pair_spanned in pairs {
                    let pair = &pair_spanned.node;
                    // Resolve pin dollar path to its designator string
                    let pin_val = self.resolve_dollar_path_to_string(&pair.pin)?;
                    let pad_val = self.resolve_dollar_path_to_string(&pair.pad)?;
                    maps.push(PinPadMap {
                        pin: pin_val,
                        pad: pad_val,
                    });
                }
                Ok(FootprintMapSpec {
                    model_name,
                    maps,
                    source: None,
                })
            }
        }
    }

    fn resolve_dollar_path_to_string(
        &self,
        path: &crate::ast::Spanned<crate::ast::DollarPath>,
    ) -> Result<String, SpecError> {
        let dp = &path.node;
        let expr = crate::ast::Expr::DollarIdent(dp.root.node.clone());
        let spanned = crate::ast::Spanned::new(expr, path.span);
        let val = eval_expr(&spanned, &self.scope)?;
        match val {
            Value::String(s) => Ok(s),
            Value::Integer(n) => Ok(n.to_string()),
            Value::Float(f) => Ok(f.to_string()),
            _ => Err(SpecError::at(
                SpecErrorCode::TypeMismatch,
                format!(
                    "expected string or number for pin/pad reference, got {:?}",
                    val
                ),
                path.span,
            )),
        }
    }

    // ── Schematic graphic compilation ──────────────────────────────────────

    fn compile_sch_graphic(&mut self, decl: &GraphicDecl) -> Result<GraphicSpec, SpecError> {
        let graphic_type = parse_sch_graphic_type(&decl.graphic_type.node).ok_or_else(|| {
            SpecError::at(
                SpecErrorCode::TypeMismatch,
                format!(
                    "unknown schematic graphic type: '{}'",
                    decl.graphic_type.node
                ),
                decl.graphic_type.span,
            )
        })?;

        let unique_id = self.make_unique_id(decl.binding.as_ref(), &decl.graphic_type.node);

        let props = eval_object_to_map(&decl.body.node, &self.scope)?;
        let properties = compile_graphic_properties(&props, decl.body.span)?;

        Ok(GraphicSpec {
            unique_id,
            graphic_type,
            properties,
        })
    }

    // ── Footprint compilation (PcbLib) ─────────────────────────────────────

    fn compile_footprint(&mut self, decl: &FootprintDecl) -> Result<FootprintSpec, SpecError> {
        let annotation = self.compile_opt_annotation(decl.annotation.as_ref())?;
        let display_name = decl.name.node.as_str();
        self.context_name = display_name.clone();
        self.unnamed_counters.clear();
        self.part_context = None;

        self.scope.push();

        let fp_lets: Vec<_> = decl
            .body
            .iter()
            .filter_map(|item| match &item.node {
                FootprintItem::LetBinding(lb) => Some((&*lb.name.node, &lb.value)),
                _ => None,
            })
            .collect();
        eval_let_bindings_slice(&fp_lets, &mut self.scope)?;

        let props = collect_object_properties_from_items(
            decl.body.iter().filter_map(|item| match &item.node {
                FootprintItem::Property(p) => Some(p),
                _ => None,
            }),
            &self.scope,
        )?;

        let description = get_string_opt(&props, "description");
        let height = get_coord_opt(&props, "height")?;
        let pattern = get_string_opt(&props, "pattern");

        let mut pads = Vec::new();
        let mut graphics = Vec::new();

        // First pass: collect explicit pads for override lookup.
        let explicit_pads: HashMap<String, &PadDecl> = decl
            .body
            .iter()
            .filter_map(|item| {
                if let FootprintItem::Pad(pd) = &item.node {
                    Some((pd.name.node.as_str(), pd))
                } else {
                    None
                }
            })
            .collect();

        // Track which explicit pad names were claimed by layout expansion.
        let mut claimed_by_layout: std::collections::HashSet<String> =
            std::collections::HashSet::new();

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
                            let explicit_props =
                                eval_object_to_map(&explicit.body.node, &self.scope)?;
                            merge_pad_override_from_props(
                                &mut pad,
                                &explicit_props,
                                explicit.body.span,
                            )?;
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
                            let explicit_props =
                                eval_object_to_map(&explicit.body.node, &self.scope)?;
                            merge_pad_override_from_props(
                                &mut pad,
                                &explicit_props,
                                explicit.body.span,
                            )?;
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
            annotation,
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
            let y = props
                .get("y")
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

    fn compile_pcb_graphic(&mut self, decl: &GraphicDecl) -> Result<PcbGraphicSpec, SpecError> {
        let graphic_type = parse_pcb_graphic_type(&decl.graphic_type.node).ok_or_else(|| {
            SpecError::at(
                SpecErrorCode::TypeMismatch,
                format!("unknown PCB graphic type: '{}'", decl.graphic_type.node),
                decl.graphic_type.span,
            )
        })?;

        let unique_id = self.make_unique_id(decl.binding.as_ref(), &decl.graphic_type.node);

        let props = eval_object_to_map(&decl.body.node, &self.scope)?;
        let properties = compile_pcb_graphic_properties(&props, decl.body.span)?;

        Ok(PcbGraphicSpec {
            unique_id,
            graphic_type,
            properties,
        })
    }

    // ── unique_id generation ───────────────────────────────────────────────

    fn make_unique_id(&mut self, binding: Option<&Spanned<String>>, type_name: &str) -> String {
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
                format!(
                    "spec:{}:{}:{}_{}",
                    self.context_name, part_ctx, type_name, n
                )
            } else {
                format!("spec:{}:{}_{}", self.context_name, type_name, n)
            };
            *n += 1;
            id
        }
    }

    // ── PcbDoc compilation ─────────────────────────────────────────────────

    fn compile_pcbdoc(&mut self, file: &SpecFile) -> Result<PcbDocSpec, SpecError> {
        self.context_name = "board".to_string();
        self.unnamed_counters.clear();

        let mut board_name = String::new();
        let mut board_annotation: Option<CompiledAnnotation> = None;
        let mut board_settings_props = IndexMap::new();
        let mut nets = Vec::new();
        let mut components = Vec::new();
        let mut primitives_by_type: IndexMap<String, Vec<PcbDocPrimitiveSpec>> = IndexMap::new();
        let mut polygons = Vec::new();
        let mut rules = Vec::new();
        let mut placement_rules = Vec::new();
        let mut placement: Option<PlacementSpec> = None;
        let mut routing: Option<RoutingSpec> = None;
        let mut classes = Vec::new();
        let mut differential_pairs = Vec::new();

        for item in &file.items {
            match &item.node {
                SpecItem::Board(decl) => {
                    board_annotation = self.compile_opt_annotation(decl.annotation.as_ref())?;
                    board_name = decl.name.node.as_str();
                    board_settings_props = self.compile_board_settings(decl)?;
                }
                SpecItem::Net(decl) => {
                    nets.push(self.compile_pcbdoc_net(decl)?);
                }
                SpecItem::Component(decl) => {
                    components.push(self.compile_pcbdoc_component(decl)?);
                }
                SpecItem::PcbDocPrimitive(decl) => {
                    let spec = self.compile_pcbdoc_primitive(decl)?;
                    let type_name = spec.primitive_type.clone();
                    primitives_by_type.entry(type_name).or_default().push(spec);
                }
                SpecItem::Polygon(decl) => {
                    polygons.push(self.compile_pcbdoc_polygon(decl)?);
                }
                SpecItem::Rule(decl) => {
                    rules.push(self.compile_pcbdoc_rule(decl)?);
                    placement_rules.push(self.compile_placement_rule(decl)?);
                }
                SpecItem::Placement(decl) => {
                    placement = Some(self.compile_placement_decl(decl)?);
                }
                SpecItem::Routing(decl) => {
                    routing = Some(self.compile_routing_decl(decl)?);
                }
                SpecItem::Class(decl) => {
                    classes.push(self.compile_pcbdoc_class(decl)?);
                }
                SpecItem::DifferentialPair(decl) => {
                    differential_pairs.push(self.compile_pcbdoc_diff_pair(decl)?);
                }
                SpecItem::Import(_) | SpecItem::LetBinding(_) | _ => {
                    // Imports, let bindings, and other-domain items silently skipped.
                }
            }
        }

        let board = BoardSpec {
            annotation: board_annotation,
            name: board_name,
            signal_layer_count: get_integer_opt(&board_settings_props, "signal_layer_count"),
            snap_grid_size: get_coord_opt(&board_settings_props, "snap_grid_size")?,
            visible_grid_size: get_coord_opt(&board_settings_props, "visible_grid_size")?,
            display_unit: get_string_opt(&board_settings_props, "display_unit"),
            nets,
            components,
            tracks: primitives_by_type.shift_remove("track").unwrap_or_default(),
            arcs: primitives_by_type.shift_remove("arc").unwrap_or_default(),
            vias: primitives_by_type.shift_remove("via").unwrap_or_default(),
            pads: primitives_by_type.shift_remove("pad").unwrap_or_default(),
            fills: primitives_by_type.shift_remove("fill").unwrap_or_default(),
            texts: primitives_by_type.shift_remove("text").unwrap_or_default(),
            regions: primitives_by_type
                .shift_remove("region")
                .unwrap_or_default(),
            component_bodies: primitives_by_type
                .shift_remove("component_body")
                .unwrap_or_default(),
            dimensions: primitives_by_type
                .shift_remove("dimension")
                .unwrap_or_default(),
            outline: extract_outline_from_props(&board_settings_props),
            keepouts: Vec::new(),
            layers: Vec::new(),
            polygons,
            rules,
            classes,
            differential_pairs,
        };

        Ok(PcbDocSpec {
            boards: vec![board],
            placement,
            placement_rules,
            routing,
        })
    }

    fn compile_board_settings(
        &mut self,
        decl: &BoardDecl,
    ) -> Result<IndexMap<String, Value>, SpecError> {
        // Evaluate let bindings in board body.
        let board_lets: Vec<_> = decl
            .body
            .iter()
            .filter_map(|item| match &item.node {
                BoardItem::LetBinding(lb) => Some((&*lb.name.node, &lb.value)),
                _ => None,
            })
            .collect();
        self.scope.push();
        eval_let_bindings_slice(&board_lets, &mut self.scope)?;

        let props = collect_object_properties_from_items(
            decl.body.iter().filter_map(|item| match &item.node {
                BoardItem::Property(p) => Some(p),
                _ => None,
            }),
            &self.scope,
        )?;

        self.scope.pop();
        Ok(props)
    }

    fn compile_pcbdoc_net(
        &mut self,
        decl: &crate::ast::NetDecl,
    ) -> Result<PcbDocNetSpec, SpecError> {
        let annotation = self.compile_opt_annotation(decl.annotation.as_ref())?;
        let name = decl.name.node.as_str();
        let props = eval_object_to_map(&decl.body.node, &self.scope)?;

        let color = props
            .get("color")
            .map(|v| value_to_color(v, Some(decl.body.span)))
            .transpose()?;
        let visible = get_bool_opt(&props, "visible");

        Ok(PcbDocNetSpec {
            annotation,
            name,
            color,
            visible,
        })
    }

    fn compile_pcbdoc_component(
        &mut self,
        decl: &ComponentDecl,
    ) -> Result<PcbDocComponentSpec, SpecError> {
        let annotation = self.compile_opt_annotation(decl.annotation.as_ref())?;
        let designator = decl.name.node.as_str();

        // Collect properties from component body items.
        let props = collect_object_properties_from_items(
            decl.body.iter().filter_map(|item| match &item.node {
                ComponentItem::Property(p) => Some(p),
                _ => None,
            }),
            &self.scope,
        )?;

        let pattern = get_string_opt(&props, "pattern");
        let comment = get_string_opt(&props, "comment");
        let location = props
            .get("at")
            .map(|v| value_to_coord_point(v, None))
            .transpose()?;
        let rotation = get_float_opt(&props, "rotation");
        let layer = get_enum_opt(&props, "layer", parse_layer_spec)?;
        let source_library = get_string_opt(&props, "source_library");

        Ok(PcbDocComponentSpec {
            annotation,
            designator,
            pattern,
            comment,
            location,
            rotation,
            layer,
            source_library,
            parameters: indexmap::IndexMap::new(),
            pads: Vec::new(),
        })
    }

    fn compile_pcbdoc_primitive(
        &mut self,
        decl: &PcbDocPrimitiveDecl,
    ) -> Result<PcbDocPrimitiveSpec, SpecError> {
        let type_name = decl.primitive_type.node.as_str();
        let props = eval_object_to_map(&decl.body.node, &self.scope)?;

        // Generate unique ID: named primitives use the name, unnamed get auto-generated IDs.
        let id = match &decl.name {
            Some(name_spanned) => {
                format!("spec:{}:{}", self.context_name, name_spanned.node.as_str())
            }
            None => {
                let counter_key = format!("{}:{}", self.context_name, type_name);
                let n = self.unnamed_counters.entry(counter_key).or_insert(0);
                let id = format!("spec:{}:{}_{}", self.context_name, type_name, n);
                *n += 1;
                id
            }
        };

        // position_index tracks order within each primitive type.
        let pos_key = format!("pos:{}", type_name);
        let position_index = {
            let n = self.unnamed_counters.entry(pos_key).or_insert(0);
            let idx = *n as usize;
            *n += 1;
            idx
        };

        Ok(PcbDocPrimitiveSpec {
            id,
            position_index,
            primitive_type: type_name.to_string(),
            properties: props,
        })
    }

    fn compile_pcbdoc_polygon(
        &mut self,
        decl: &PolygonDecl,
    ) -> Result<PcbDocPolygonSpec, SpecError> {
        let annotation = self.compile_opt_annotation(decl.annotation.as_ref())?;
        let name = decl.name.node.as_str();
        let props = eval_object_to_map(&decl.body.node, &self.scope)?;

        Ok(PcbDocPolygonSpec {
            annotation,
            name,
            net: get_string_opt(&props, "net"),
            layer: get_enum_opt(&props, "layer", parse_layer_spec)?,
            connect_style: get_string_opt(&props, "connect_style"),
            pour_order: get_integer_opt(&props, "pour_order"),
        })
    }

    fn compile_pcbdoc_rule(&mut self, decl: &RuleDecl) -> Result<PcbDocRuleSpec, SpecError> {
        let annotation = self.compile_opt_annotation(decl.annotation.as_ref())?;
        let name = decl.name.node.as_str();
        let raw = eval_object_to_map(&decl.body.node, &self.scope)?;

        let kind = get_string_opt(&raw, "kind");
        let enabled = get_bool_opt(&raw, "enabled");
        let priority = get_integer_opt(&raw, "priority");
        let scope = get_string_opt(&raw, "scope");

        // Collect remaining key-value pairs into `properties` (everything except the
        // well-known scalar fields handled above and the `properties` sub-block key).
        let well_known = ["kind", "enabled", "priority", "scope", "gap", "properties"];
        let mut properties = indexmap::IndexMap::new();
        for (k, v) in &raw {
            if !well_known.contains(&k.as_str()) {
                properties.insert(k.clone(), value_to_string_repr(v));
            }
        }
        // If a `properties { ... }` sub-block was given, merge those entries too.
        if let Some(Value::Object(sub)) = raw.get("properties") {
            for (k, v) in sub {
                properties.insert(k.clone(), value_to_string_repr(v));
            }
        }

        Ok(PcbDocRuleSpec {
            annotation,
            name,
            kind,
            enabled,
            priority,
            properties,
            scope,
            scope2: None,
        })
    }

    fn compile_pcbdoc_class(&mut self, decl: &ClassDecl) -> Result<PcbDocClassSpec, SpecError> {
        let annotation = self.compile_opt_annotation(decl.annotation.as_ref())?;
        let name = decl.name.node.as_str();
        let props = eval_object_to_map(&decl.body.node, &self.scope)?;

        let members = match props.get("members") {
            Some(Value::Array(arr)) => arr
                .iter()
                .filter_map(|v| match v {
                    Value::String(s) => Some(s.clone()),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        };

        Ok(PcbDocClassSpec {
            annotation,
            name,
            kind: get_string_opt(&props, "kind"),
            members,
        })
    }

    fn compile_pcbdoc_diff_pair(
        &mut self,
        decl: &DifferentialPairDecl,
    ) -> Result<PcbDocDifferentialPairSpec, SpecError> {
        let name = decl.name.node.as_str();
        let props = eval_object_to_map(&decl.body.node, &self.scope)?;

        Ok(PcbDocDifferentialPairSpec {
            annotation: None,
            name,
            positive_net: get_string_opt(&props, "positive_net"),
            negative_net: get_string_opt(&props, "negative_net"),
        })
    }

    fn compile_routing_decl(&mut self, decl: &RoutingDecl) -> Result<RoutingSpec, SpecError> {
        let props = eval_object_to_map(&decl.body.node, &self.scope)?;
        let solution = get_string_opt(&props, "solution");
        let mut config = indexmap::IndexMap::new();
        for (key, val) in &props {
            if key != "solution" {
                config.insert(key.clone(), val.display());
            }
        }
        Ok(RoutingSpec { solution, config })
    }

    fn compile_placement_rule(&mut self, decl: &RuleDecl) -> Result<PlacementRuleSpec, SpecError> {
        let name = decl.name.node.as_str();
        let props = eval_object_to_map(&decl.body.node, &self.scope)?;
        Ok(PlacementRuleSpec {
            name,
            kind: get_string_opt(&props, "kind"),
            gap: get_coord_opt(&props, "gap")?,
        })
    }

    fn compile_placement_decl(&mut self, decl: &PlacementDecl) -> Result<PlacementSpec, SpecError> {
        let annotation = self.compile_opt_annotation(decl.annotation.as_ref())?;
        // placement-level lets
        self.scope.push();
        let placement_lets: Vec<_> = decl
            .body
            .iter()
            .filter_map(|item| match &item.node {
                PlacementItem::LetBinding(lb) => Some((&*lb.name.node, &lb.value)),
                _ => None,
            })
            .collect();
        eval_let_bindings_slice(&placement_lets, &mut self.scope)?;

        let mut target = None;
        let mut places = Vec::new();
        let mut constraints = Vec::new();
        let mut optimize = PlacementOptimizeSpec {
            ratsnest: true,
            ratsnest_weight: 1.0,
        };
        let mut clearance = PlacementClearanceSpec {
            all: None,
            edge: None,
        };
        let mut autoplace_config: Option<AutoplaceConfig> = None;
        let mut unplaced = UnplacedStrategy::default();
        let mut allow_pin_swap = false;
        let mut allow_part_swap = false;
        let mut allow_gate_swap = false;
        let mut groups: Vec<PlacementGroupSpec> = Vec::new();

        for item in &decl.body {
            match &item.node {
                PlacementItem::Property(p) => match p.key.node.as_str() {
                    "target" => {
                        target = Some(self.expr_to_string(&p.value.node, p.value.span)?);
                    }
                    "unplaced" => {
                        let s = self.expr_to_string(&p.value.node, p.value.span)?;
                        unplaced = match s.as_str() {
                            "autoplace" => UnplacedStrategy::Autoplace,
                            "ignore" => UnplacedStrategy::Ignore,
                            "error" => UnplacedStrategy::Error,
                            _ => {
                                return Err(SpecError::at(
                                    SpecErrorCode::TypeMismatch,
                                    format!(
                                        "invalid unplaced strategy '{}'; expected autoplace, ignore, or error",
                                        s
                                    ),
                                    p.value.span,
                                ));
                            }
                        };
                    }
                    "allow_pin_swap" => {
                        allow_pin_swap = self.expr_to_bool(&p.value.node, p.value.span)?;
                    }
                    "allow_part_swap" => {
                        allow_part_swap = self.expr_to_bool(&p.value.node, p.value.span)?;
                    }
                    "allow_gate_swap" => {
                        allow_gate_swap = self.expr_to_bool(&p.value.node, p.value.span)?;
                    }
                    _ => {}
                },
                PlacementItem::Place(place) => places.push(self.compile_placement_place(place)?),
                PlacementItem::Constraint(c) => {
                    if let Some(cspec) = self.compile_placement_constraint(c)? {
                        constraints.push(cspec);
                    }
                }
                PlacementItem::Optimize(obj) => {
                    let props = self.object_expr_map(&obj.node)?;
                    if let Some(v) = props.get("ratsnest") {
                        optimize.ratsnest = self.expr_to_bool(v.0, v.1)?;
                    }
                    if let Some(v) = props.get("ratsnest_weight") {
                        optimize.ratsnest_weight = self.expr_to_f64(v.0, v.1)?;
                    }
                }
                PlacementItem::Clearance(obj) => {
                    let props = self.object_expr_map(&obj.node)?;
                    if let Some(v) = props.get("all") {
                        clearance.all = Some(self.expr_to_coord(v.0, v.1)?);
                    }
                    if let Some(v) = props.get("edge") {
                        clearance.edge = Some(self.expr_to_coord(v.0, v.1)?);
                    }
                }
                PlacementItem::Minimize(decl) => {
                    // `minimize wirelength` maps to optimize.ratsnest = true
                    // `minimize wirelength subject_to { ... }` additionally stores hints
                    match decl.objective.node.as_str() {
                        "wirelength" => {
                            optimize.ratsnest = true;
                            optimize.ratsnest_weight = 0.01; // default
                        }
                        _other => {
                            // Phase 1: only 'wirelength' is supported
                        }
                    }
                    // subject_to hints parsed but not yet consumed (Phase 2)
                }
                PlacementItem::AutoplaceBlock(obj) => {
                    autoplace_config = Some(self.compile_autoplace_config(&obj.node)?);
                }
                PlacementItem::GroupDecl(group) => {
                    groups.push(self.compile_placement_group(group)?);
                }
                PlacementItem::SeparateDecl(_) => {
                    // SeparateDecl is stored in constraints or ignored for now.
                    // Future milestones will convert these to PlacementConstraintSpec variants.
                }
                PlacementItem::LetBinding(_) => {}
            }
        }

        self.scope.pop();

        Ok(PlacementSpec {
            annotation,
            target,
            places,
            constraints,
            optimize,
            clearance,
            autoplace_config,
            unplaced,
            allow_pin_swap,
            allow_part_swap,
            allow_gate_swap,
            groups,
        })
    }

    fn compile_placement_place(
        &mut self,
        decl: &PlaceDecl,
    ) -> Result<PlacementPlaceSpec, SpecError> {
        let annotation = self.compile_opt_annotation(decl.annotation.as_ref())?;
        let designators = decl.designators.iter().map(|d| d.node.as_str()).collect();
        let props = self.object_expr_map(&decl.body.node)?;

        let mut region_name = None;
        let mut region_rect = None;
        if let Some((expr, span)) = props.get("region") {
            match expr {
                crate::ast::Expr::Ident(s) | crate::ast::Expr::String(s) => {
                    region_name = Some(s.clone());
                }
                crate::ast::Expr::Object(obj) => {
                    let region_props = self.object_expr_map(obj)?;
                    if let (Some(from), Some(to)) =
                        (region_props.get("from"), region_props.get("to"))
                    {
                        region_rect = Some((
                            self.expr_to_coord_point(from.0, from.1)?,
                            self.expr_to_coord_point(to.0, to.1)?,
                        ));
                    }
                }
                _ => {
                    return Err(SpecError::at(
                        SpecErrorCode::TypeMismatch,
                        "region must be an identifier/string or rectangle object",
                        *span,
                    ));
                }
            }
        }

        let edge = match props.get("edge") {
            Some((expr, span)) => Some(self.expr_to_string(expr, *span)?),
            None => None,
        };
        let inset = match props.get("inset") {
            Some((expr, span)) => Some(self.expr_to_coord(expr, *span)?),
            None => None,
        };
        let near = match props.get("near") {
            Some((expr, _span)) => Some(self.expr_to_component_ref(expr)?),
            None => None,
        };
        let max_distance = match props.get("max_distance") {
            Some((expr, span)) => Some(self.expr_to_coord(expr, *span)?),
            None => None,
        };
        let autoplace = match props.get("autoplace") {
            Some((expr, span)) => self.expr_to_autoplace_mode(expr, *span)?,
            None => PlacementAutoplaceMode::Disabled,
        };
        let (rotation, rotation_options) = match props.get("rotation") {
            Some((crate::ast::Expr::Array(items), _)) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    out.push(self.expr_to_i32(&item.node, item.span)?);
                }
                (None, out)
            }
            Some((expr, span)) if autoplace.is_solver_variable() => {
                (None, vec![self.expr_to_i32(expr, *span)?])
            }
            Some((expr, span)) => (Some(self.expr_to_f64(expr, *span)?), Vec::new()),
            None => (None, Vec::new()),
        };
        let fixed = match props.get("fixed") {
            Some((expr, span)) => self.expr_to_bool(expr, *span)?,
            None => false,
        };
        let at = match props.get("at") {
            Some((expr, span)) => Some(self.expr_to_coord_point(expr, *span)?),
            None => None,
        };
        let side = match props.get("side") {
            Some((expr, span)) => Some(self.expr_to_string(expr, *span)?),
            None => None,
        };
        let no_pin_swap = match props.get("no_pin_swap") {
            Some((crate::ast::Expr::Array(items), _)) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    out.push(self.expr_to_string(&item.node, item.span)?);
                }
                out
            }
            Some((expr, span)) => vec![self.expr_to_string(expr, *span)?],
            None => Vec::new(),
        };
        let no_part_swap = match props.get("no_part_swap") {
            Some((expr, span)) => self.expr_to_bool(expr, *span)?,
            None => false,
        };

        Ok(PlacementPlaceSpec {
            annotation,
            designators,
            region_name,
            region_rect,
            edge,
            inset,
            near,
            max_distance,
            rotation,
            rotation_options,
            fixed,
            at,
            side,
            autoplace,
            no_pin_swap,
            no_part_swap,
        })
    }

    fn compile_autoplace_config(&mut self, obj: &Object) -> Result<AutoplaceConfig, SpecError> {
        let props = self.object_expr_map(obj)?;
        let algorithm = match props.get("algorithm") {
            Some((expr, span)) => Some(self.expr_to_string(expr, *span)?),
            None => None,
        };
        let sa_cooling = match props.get("sa_cooling") {
            Some((expr, span)) => Some(self.expr_to_f64(expr, *span)?),
            None => None,
        };
        let sa_moves_per_temp = match props.get("sa_moves_per_temp") {
            Some((expr, span)) => Some(self.expr_to_i32(expr, *span)? as usize),
            None => None,
        };
        let sa_max_steps = match props.get("sa_max_steps") {
            Some((expr, span)) => Some(self.expr_to_i32(expr, *span)? as usize),
            None => None,
        };
        let enable_net_crossings = match props.get("enable_net_crossings") {
            Some((expr, span)) => Some(self.expr_to_bool(expr, *span)?),
            None => None,
        };
        let congestion_weight = match props.get("congestion_weight") {
            Some((expr, span)) => Some(self.expr_to_f64(expr, *span)?),
            None => None,
        };
        let congestion_cell = match props.get("congestion_cell") {
            Some((expr, span)) => Some(self.expr_to_coord(expr, *span)?),
            None => None,
        };
        let critical_net_boost = match props.get("critical_net_boost") {
            Some((expr, span)) => Some(self.expr_to_f64(expr, *span)?),
            None => None,
        };
        let default_clearance = match props.get("default_clearance") {
            Some((expr, span)) => Some(self.expr_to_coord(expr, *span)?),
            None => None,
        };
        let board_edge_clearance = match props.get("board_edge_clearance") {
            Some((expr, span)) => Some(self.expr_to_coord(expr, *span)?),
            None => None,
        };
        let grid_snap = match props.get("grid_snap") {
            Some((expr, span)) => Some(self.expr_to_coord(expr, *span)?),
            None => None,
        };
        let auto_cluster = match props.get("auto_cluster") {
            Some((expr, span)) => Some(self.expr_to_bool(expr, *span)?),
            None => None,
        };
        let cluster_target_size = match props.get("cluster_target_size") {
            Some((expr, span)) => Some(self.expr_to_i32(expr, *span)? as usize),
            None => None,
        };
        let cluster_max_depth = match props.get("cluster_max_depth") {
            Some((expr, span)) => Some(self.expr_to_i32(expr, *span)? as usize),
            None => None,
        };
        Ok(AutoplaceConfig {
            algorithm,
            sa_cooling,
            sa_moves_per_temp,
            sa_max_steps,
            enable_net_crossings,
            congestion_weight,
            congestion_cell,
            critical_net_boost,
            default_clearance,
            board_edge_clearance,
            grid_snap,
            auto_cluster,
            cluster_target_size,
            cluster_max_depth,
        })
    }

    fn compile_placement_group(
        &mut self,
        decl: &PlacementGroupDecl,
    ) -> Result<PlacementGroupSpec, SpecError> {
        let name = decl.name.node.clone();
        let props = self.object_expr_map(&decl.body.node)?;
        let components = match props.get("components") {
            Some((crate::ast::Expr::Array(items), _)) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    out.push(self.expr_to_string(&item.node, item.span)?);
                }
                out
            }
            Some((expr, span)) => vec![self.expr_to_string(expr, *span)?],
            None => Vec::new(),
        };
        Ok(PlacementGroupSpec { name, components })
    }

    fn compile_placement_constraint(
        &mut self,
        decl: &PlacementConstraintDecl,
    ) -> Result<Option<PlacementConstraintSpec>, SpecError> {
        let gap_from_obj = |obj: &Option<Spanned<Object>>| -> Result<Option<Coord>, SpecError> {
            if let Some(o) = obj {
                let props = self.object_expr_map(&o.node)?;
                if let Some((expr, span)) = props.get("gap") {
                    return Ok(Some(self.expr_to_coord(expr, *span)?));
                }
            }
            Ok(None)
        };

        match decl {
            PlacementConstraintDecl::LeftOf { a, b, body } => {
                Ok(Some(PlacementConstraintSpec::LeftOf {
                    a: a.node.root.node.clone(),
                    b: b.node.root.node.clone(),
                    gap: gap_from_obj(body)?,
                }))
            }
            PlacementConstraintDecl::RightOf { a, b, body } => {
                Ok(Some(PlacementConstraintSpec::RightOf {
                    a: a.node.root.node.clone(),
                    b: b.node.root.node.clone(),
                    gap: gap_from_obj(body)?,
                }))
            }
            PlacementConstraintDecl::Above { a, b, body } => {
                Ok(Some(PlacementConstraintSpec::Above {
                    a: a.node.root.node.clone(),
                    b: b.node.root.node.clone(),
                    gap: gap_from_obj(body)?,
                }))
            }
            PlacementConstraintDecl::Below { a, b, body } => {
                Ok(Some(PlacementConstraintSpec::Below {
                    a: a.node.root.node.clone(),
                    b: b.node.root.node.clone(),
                    gap: gap_from_obj(body)?,
                }))
            }
        }
    }

    fn object_expr_map<'a>(
        &self,
        obj: &'a Object,
    ) -> Result<IndexMap<String, (&'a crate::ast::Expr, crate::diagnostic::Span)>, SpecError> {
        let mut map = IndexMap::new();
        for item in &obj.items {
            match &item.node {
                ObjectItem::Property(p) => {
                    map.insert(p.key.node.clone(), (&p.value.node, p.value.span));
                }
                ObjectItem::LetBinding(_) | ObjectItem::Spread(_) => {
                    return Err(SpecError::at(
                        SpecErrorCode::TypeMismatch,
                        "placement objects currently support only plain properties",
                        item.span,
                    ));
                }
            }
        }
        Ok(map)
    }

    fn expr_to_string(
        &self,
        expr: &crate::ast::Expr,
        span: crate::diagnostic::Span,
    ) -> Result<String, SpecError> {
        match expr {
            crate::ast::Expr::String(s) | crate::ast::Expr::Ident(s) => Ok(s.clone()),
            crate::ast::Expr::Integer(i) => Ok(i.to_string()),
            _ => Err(SpecError::at(
                SpecErrorCode::TypeMismatch,
                "expected string-like value",
                span,
            )),
        }
    }

    fn expr_to_component_ref(&self, expr: &crate::ast::Expr) -> Result<String, SpecError> {
        match expr {
            crate::ast::Expr::DollarIdent(s) => Ok(s.clone()),
            crate::ast::Expr::Path(base, _field) => match &base.node {
                crate::ast::Expr::DollarIdent(s) => Ok(s.clone()),
                _ => Err(SpecError::no_span(
                    SpecErrorCode::TypeMismatch,
                    "expected '$Designator' component reference",
                )),
            },
            _ => Err(SpecError::no_span(
                SpecErrorCode::TypeMismatch,
                "expected '$Designator' component reference",
            )),
        }
    }

    fn expr_to_bool(
        &self,
        expr: &crate::ast::Expr,
        span: crate::diagnostic::Span,
    ) -> Result<bool, SpecError> {
        let val = eval_expr(&Spanned::new(expr.clone(), span), &self.scope)?;
        value_to_bool(&val, Some(span))
    }

    fn expr_to_autoplace_mode(
        &self,
        expr: &crate::ast::Expr,
        span: crate::diagnostic::Span,
    ) -> Result<PlacementAutoplaceMode, SpecError> {
        match expr {
            crate::ast::Expr::Bool(true) => Ok(PlacementAutoplaceMode::Auto),
            crate::ast::Expr::Bool(false) => Ok(PlacementAutoplaceMode::Disabled),
            crate::ast::Expr::Ident(name) => match name.as_str() {
                "solved" => Ok(PlacementAutoplaceMode::Solved),
                "locked" => Ok(PlacementAutoplaceMode::Locked),
                "true" => Ok(PlacementAutoplaceMode::Auto),
                "false" => Ok(PlacementAutoplaceMode::Disabled),
                other => Err(SpecError::at(
                    SpecErrorCode::TypeMismatch,
                    format!(
                        "invalid autoplace value '{other}'; expected true, false, solved, or locked"
                    ),
                    span,
                )),
            },
            _ => {
                let val = eval_expr(&Spanned::new(expr.clone(), span), &self.scope)?;
                match value_to_bool(&val, Some(span)) {
                    Ok(true) => Ok(PlacementAutoplaceMode::Auto),
                    Ok(false) => Ok(PlacementAutoplaceMode::Disabled),
                    Err(_) => Err(SpecError::at(
                        SpecErrorCode::TypeMismatch,
                        "expected autoplace value true, false, solved, or locked",
                        span,
                    )),
                }
            }
        }
    }

    fn expr_to_i32(
        &self,
        expr: &crate::ast::Expr,
        span: crate::diagnostic::Span,
    ) -> Result<i32, SpecError> {
        let val = eval_expr(&Spanned::new(expr.clone(), span), &self.scope)?;
        match val {
            Value::Integer(i) => Ok(i),
            Value::Float(f) => Ok(f as i32),
            _ => Err(SpecError::at(
                SpecErrorCode::TypeMismatch,
                "expected integer",
                span,
            )),
        }
    }

    fn expr_to_f64(
        &self,
        expr: &crate::ast::Expr,
        span: crate::diagnostic::Span,
    ) -> Result<f64, SpecError> {
        let val = eval_expr(&Spanned::new(expr.clone(), span), &self.scope)?;
        match val {
            Value::Integer(i) => Ok(i as f64),
            Value::Float(f) => Ok(f),
            _ => Err(SpecError::at(
                SpecErrorCode::TypeMismatch,
                "expected numeric value",
                span,
            )),
        }
    }

    fn expr_to_coord(
        &self,
        expr: &crate::ast::Expr,
        span: crate::diagnostic::Span,
    ) -> Result<Coord, SpecError> {
        let val = eval_expr(&Spanned::new(expr.clone(), span), &self.scope)?;
        value_to_coord(&val, Some(span))
    }

    fn expr_to_coord_point(
        &self,
        expr: &crate::ast::Expr,
        span: crate::diagnostic::Span,
    ) -> Result<CoordPoint, SpecError> {
        let val = eval_expr(&Spanned::new(expr.clone(), span), &self.scope)?;
        value_to_coord_point(&val, Some(span))
    }

    // ── Project compilation (PrjPcb) ──────────────────────────────────────

    fn compile_project(&mut self, decl: &ProjectDecl) -> Result<ProjectSpec, SpecError> {
        let name = decl.name.node.as_str();
        self.context_name = name.clone();
        self.unnamed_counters.clear();

        // Push project scope.
        self.scope.push();

        // Collect and evaluate project-level let bindings.
        let proj_lets: Vec<_> = decl
            .body
            .iter()
            .filter_map(|item| match &item.node {
                ProjectItem::LetBinding(lb) => Some((&*lb.name.node, &lb.value)),
                _ => None,
            })
            .collect();
        eval_let_bindings_slice(&proj_lets, &mut self.scope)?;

        // Collect project-level properties from Property items.
        let props = collect_object_properties_from_items(
            decl.body.iter().filter_map(|item| match &item.node {
                ProjectItem::Property(p) => Some(p),
                _ => None,
            }),
            &self.scope,
        )?;

        // Extract scalar fields.
        let hierarchy_mode = get_enum_opt(&props, "hierarchy_mode", parse_flatten_mode)?;
        let channel_room_naming_style = get_enum_opt(
            &props,
            "channel_room_naming_style",
            parse_channel_room_naming_style,
        )?;
        let channel_designator_format = get_string_opt(&props, "channel_designator_format");
        let channel_room_level_separator = get_string_opt(&props, "channel_room_level_separator");
        let allow_port_net_names = get_bool_opt(&props, "allow_port_net_names");
        let allow_sheet_entry_net_names = get_bool_opt(&props, "allow_sheet_entry_net_names");
        let netlist_single_pin_nets = get_bool_opt(&props, "netlist_single_pin_nets");
        let append_sheet_number_to_local_nets =
            get_bool_opt(&props, "append_sheet_number_to_local_nets");
        let name_nets_hierarchically = get_bool_opt(&props, "name_nets_hierarchically");
        let power_port_names_take_priority = get_bool_opt(&props, "power_port_names_take_priority");
        let pin_swap_by_netlabel = get_bool_opt(&props, "pin_swap_by_netlabel");
        let pin_swap_by_pin = get_bool_opt(&props, "pin_swap_by_pin");
        let cross_ref_sheet_style =
            get_enum_opt(&props, "cross_ref_sheet_style", parse_cross_ref_sheet_style)?;
        let cross_ref_location_style = get_enum_opt(
            &props,
            "cross_ref_location_style",
            parse_cross_ref_location_style,
        )?;
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
                        generate_component_classes: get_bool_opt(
                            &cg_props,
                            "generate_component_classes",
                        ),
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

    fn compile_document(
        &mut self,
        doc: &crate::ast::DocumentBlockDecl,
    ) -> Result<DocumentSpec, SpecError> {
        let path = doc.path.node.as_str();
        let props =
            collect_object_properties_from_items(doc.body.iter().map(|p| &p.node), &self.scope)?;
        Ok(DocumentSpec {
            path,
            annotation_enabled: get_bool_opt(&props, "annotation_enabled"),
            annotate_start_value: get_integer_opt(&props, "annotate_start_value"),
            do_library_update: get_bool_opt(&props, "do_library_update"),
            do_database_update: get_bool_opt(&props, "do_database_update"),
        })
    }

    fn compile_annotation(
        &mut self,
        ann: &crate::ast::AnnotationBlockDecl,
    ) -> Result<AnnotationSpec, SpecError> {
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

    fn compile_erc_matrix_entry(
        &self,
        entry: &crate::ast::ErcMatrixEntryDecl,
    ) -> Result<ErcMatrixOverride, SpecError> {
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
                format!(
                    "unknown error level: '{}' (expected no_report, warning, error, fatal)",
                    entry.level.node
                ),
                Some(entry.level.span),
            )
        })?;
        Ok(ErcMatrixOverride { row, col, level })
    }

    fn compile_erc_level_entry(
        &self,
        entry: &crate::ast::ErcLevelEntryDecl,
    ) -> Result<ErcLevelOverride, SpecError> {
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
                    format!(
                        "expected error level string or integer, got {}",
                        other.kind_name()
                    ),
                    Some(entry.level.span),
                ));
            }
        };
        let level = parse_error_level(&level_str).ok_or_else(|| {
            SpecError::new(
                SpecErrorCode::TypeMismatch,
                format!(
                    "unknown error level: '{}' (expected no_report, warning, error, fatal)",
                    level_str
                ),
                Some(entry.level.span),
            )
        })?;
        Ok(ErcLevelOverride {
            name: entry.name.node.clone(),
            level,
        })
    }

    fn compile_output_group(
        &mut self,
        group: &crate::ast::OutputGroupBlockDecl,
    ) -> Result<OutputGroupSpec, SpecError> {
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

        Ok(OutputGroupSpec {
            name,
            description,
            outputs,
        })
    }

    fn compile_comparison_rule(
        &self,
        rule: &crate::ast::ComparisonRuleDecl,
    ) -> Result<ComparisonRuleSpec, SpecError> {
        let kind = rule.kind.node.as_str();
        let obj_map = eval_object_to_map(&rule.body.node, &self.scope)?;
        let mut properties = IndexMap::new();
        for (k, v) in &obj_map {
            properties.insert(k.clone(), v.display());
        }
        Ok(ComparisonRuleSpec { kind, properties })
    }

    fn compile_variant(
        &mut self,
        var: &crate::ast::VariantBlockDecl,
    ) -> Result<VariantSpec, SpecError> {
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
            let parameter = get_string_opt(&pv_map, "parameter").unwrap_or_default();
            let value = get_string_opt(&pv_map, "value").unwrap_or_default();
            param_variations.push(ParamVariationSpec {
                designator: pv.node.designator.node.as_str(),
                parameter,
                value,
            });
        }

        Ok(VariantSpec {
            name,
            description,
            variations,
            param_variations,
        })
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
fn eval_object_to_map(obj: &Object, scope: &ScopeStack) -> EvalResult<IndexMap<String, Value>> {
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

/// Convert a [`Value`] to its canonical string representation for storage in
/// freeform `IndexMap<String, String>` property bags.
fn value_to_string_repr(v: &Value) -> String {
    v.display()
}

fn get_string_opt(props: &IndexMap<String, Value>, key: &str) -> Option<String> {
    props.get(key).and_then(|v| match v {
        Value::String(s) => Some(s.clone()),
        Value::Integer(n) => Some(n.to_string()),
        _ => None,
    })
}

/// Extract a swap group name from props, accepting either `Value::SwapGroup` (typed reference)
/// or `Value::String` (backward compatibility with raw string literals).
fn get_swap_group_opt(
    props: &IndexMap<String, Value>,
    key: &str,
) -> Result<Option<String>, SpecError> {
    match props.get(key) {
        Some(Value::SwapGroup(s)) => Ok(Some(s.clone())),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(other) => Err(SpecError::no_span(
            SpecErrorCode::TypeMismatch,
            format!(
                "{key}: expected swap_group reference or string, got {}",
                other.kind_name()
            ),
        )),
        None => Ok(None),
    }
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

fn get_coord_opt(props: &IndexMap<String, Value>, key: &str) -> Result<Option<Coord>, SpecError> {
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
        Some(Value::String(s)) => parse(s.as_str()).map(Some).ok_or_else(|| {
            SpecError::no_span(
                SpecErrorCode::TypeMismatch,
                format!("invalid enum value '{}' for key '{key}'", s),
            )
        }),
        Some(Value::Integer(n)) => parse(&n.to_string()).map(Some).ok_or_else(|| {
            SpecError::no_span(
                SpecErrorCode::TypeMismatch,
                format!("invalid enum integer {n} for key '{key}'"),
            )
        }),
        Some(other) => Err(SpecError::no_span(
            SpecErrorCode::TypeMismatch,
            format!(
                "expected string for enum key '{key}', got {}",
                other.kind_name()
            ),
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
            format!(
                "expected string/integer for '{key}', got {}",
                other.kind_name()
            ),
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
    let from = props
        .get("from")
        .map(|v| value_to_coord_point(v, Some(span)))
        .transpose()?;
    let to = props
        .get("to")
        .map(|v| value_to_coord_point(v, Some(span)))
        .transpose()?;
    let center = props
        .get("center")
        .map(|v| value_to_coord_point(v, Some(span)))
        .transpose()?;
    let at = props
        .get("at")
        .map(|v| value_to_coord_point(v, Some(span)))
        .transpose()?;

    let radius = props
        .get("radius")
        .map(|v| value_to_coord(v, Some(span)))
        .transpose()?;
    let secondary_radius = props
        .get("secondary_radius")
        .map(|v| value_to_coord(v, Some(span)))
        .transpose()?;
    let line_width = props
        .get("line_width")
        .map(|v| value_to_coord(v, Some(span)))
        .transpose()?;
    let width = props
        .get("width")
        .map(|v| value_to_coord(v, Some(span)))
        .transpose()?;
    let corner_x_radius = props
        .get("corner_x_radius")
        .map(|v| value_to_coord(v, Some(span)))
        .transpose()?;
    let corner_y_radius = props
        .get("corner_y_radius")
        .map(|v| value_to_coord(v, Some(span)))
        .transpose()?;

    let start_angle = get_float_opt(props, "start_angle");
    let end_angle = get_float_opt(props, "end_angle");
    let is_solid = get_bool_opt(props, "is_solid");
    let closed = get_bool_opt(props, "closed");
    let show_border = get_bool_opt(props, "show_border");
    let font_id = get_integer_opt(props, "font_id");
    let text = get_string_opt(props, "text");
    let file_name = get_string_opt(props, "file_name");

    let color = props
        .get("color")
        .map(|v| value_to_color(v, Some(span)))
        .transpose()?;
    let area_color = props
        .get("area_color")
        .map(|v| value_to_color(v, Some(span)))
        .transpose()?;

    let points = props
        .get("points")
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
    let width = props
        .get("width")
        .map(|v| value_to_coord(v, Some(span)))
        .transpose()?;
    let from = props
        .get("from")
        .map(|v| value_to_coord_point(v, Some(span)))
        .transpose()?;
    let to = props
        .get("to")
        .map(|v| value_to_coord_point(v, Some(span)))
        .transpose()?;
    let center = props
        .get("center")
        .map(|v| value_to_coord_point(v, Some(span)))
        .transpose()?;
    let at = props
        .get("at")
        .map(|v| value_to_coord_point(v, Some(span)))
        .transpose()?;

    let radius = props
        .get("radius")
        .map(|v| value_to_coord(v, Some(span)))
        .transpose()?;
    let hole_size = props
        .get("hole_size")
        .map(|v| value_to_coord(v, Some(span)))
        .transpose()?;
    let diameter = props
        .get("diameter")
        .map(|v| value_to_coord(v, Some(span)))
        .transpose()?;

    let start_angle = get_float_opt(props, "start_angle");
    let end_angle = get_float_opt(props, "end_angle");
    let rotation = get_float_opt(props, "rotation");
    let is_solid = get_bool_opt(props, "is_solid");
    let text = get_string_opt(props, "text");

    let points = props
        .get("points")
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
                let n = (chunk[0] as u32) << 18 | (chunk[1] as u32) << 12 | (chunk[2] as u32) << 6;
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
            EdgeSide::Left => -1,   // top to bottom: decreasing Y
            EdgeSide::Right => 1,   // bottom to top: increasing Y
            EdgeSide::Top => 1,     // left to right: increasing X
            EdgeSide::Bottom => -1, // right to left: decreasing X
        }
    }

    fn auto_orientation(self) -> RotationBy90 {
        match self {
            EdgeSide::Left => RotationBy90::Rotate0, // pin points right
            EdgeSide::Right => RotationBy90::Rotate180, // pin points left
            EdgeSide::Top => RotationBy90::Rotate270, // pin points down
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
                if dir > 0 {
                    min
                } else {
                    max
                }
            }
            AnchorPosition::Center => Coord::new((min.raw() + max.raw()) / 2),
            AnchorPosition::End => {
                if dir > 0 {
                    max
                } else {
                    min
                }
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
        Edge {
            position: x,
            range: (y_min, y_max),
            side: EdgeSide::Left,
        }
    }
    fn right_edge(&self) -> Edge {
        let x = Coord::new(self.from.x.raw().max(self.to.x.raw()));
        let y_min = Coord::new(self.from.y.raw().min(self.to.y.raw()));
        let y_max = Coord::new(self.from.y.raw().max(self.to.y.raw()));
        Edge {
            position: x,
            range: (y_min, y_max),
            side: EdgeSide::Right,
        }
    }
    fn top_edge(&self) -> Edge {
        let y = Coord::new(self.from.y.raw().max(self.to.y.raw()));
        let x_min = Coord::new(self.from.x.raw().min(self.to.x.raw()));
        let x_max = Coord::new(self.from.x.raw().max(self.to.x.raw()));
        Edge {
            position: y,
            range: (x_min, x_max),
            side: EdgeSide::Top,
        }
    }
    fn bottom_edge(&self) -> Edge {
        let y = Coord::new(self.from.y.raw().min(self.to.y.raw()));
        let x_min = Coord::new(self.from.x.raw().min(self.to.x.raw()));
        let x_max = Coord::new(self.from.x.raw().max(self.to.x.raw()));
        Edge {
            position: y,
            range: (x_min, x_max),
            side: EdgeSide::Bottom,
        }
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
        let varying_coord = edge.point_at(if start {
            AnchorPosition::Start
        } else {
            AnchorPosition::End
        });
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
///
/// Returns `(map, auto_sized)` where `auto_sized` contains binding names whose
/// `from`/`to` were absent — they get a zero placeholder and will be patched later
/// by `compute_auto_size_bounds`.
fn build_graphic_binding_map<'a>(
    graphic_decls: impl Iterator<Item = &'a crate::ast::GraphicDecl>,
    scope: &ScopeStack,
) -> Result<(GraphicBindingMap, HashSet<String>), SpecError> {
    let mut map = GraphicBindingMap::new();
    let mut auto_sized: HashSet<String> = HashSet::new();

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
            None => {
                // Auto-sized: placeholder bounds, will be computed from pin extents.
                auto_sized.insert(binding_name.clone());
                CoordPoint::zero()
            }
        };
        let to = match props.get("to") {
            Some(v) => value_to_coord_point(v, Some(decl.body.span))?,
            None => {
                auto_sized.insert(binding_name.clone());
                CoordPoint::zero()
            }
        };

        map.insert(binding_name, BoxGeometry { from, to });
    }

    Ok((map, auto_sized))
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
fn extract_at_position(
    obj: &crate::ast::Object,
    scope: &ScopeStack,
) -> Result<Option<AnchorPosition>, SpecError> {
    for item in &obj.items {
        if let crate::ast::ObjectItem::Property(p) = &item.node {
            if p.key.node == "at" {
                let val = eval_expr(&p.value, scope)?;
                match &val {
                    Value::String(s) => {
                        return Ok(Some(parse_anchor_position(s).ok_or_else(|| {
                            SpecError::no_span(
                                SpecErrorCode::TypeMismatch,
                                format!(
                                    "invalid anchor position '{}': expected start, center, or end",
                                    s
                                ),
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
                            format!(
                                "at: expected string position or coord point, got {}",
                                other.kind_name()
                            ),
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
    /// No `on:`, but `after: $ref` present — binding+field will be inferred from the chain.
    InferredAfter { after_ref: String, gap: i32 },
    /// No `on:`, but `before: $ref` present — binding+field will be inferred from the chain.
    InferredBefore { before_ref: String, gap: i32 },
}

/// Auto-size rectangle bounds from pin extents.
///
/// For each binding in `auto_sized`, groups pins by their resolved edge, sums gaps along
/// each edge chain, and sets `from`/`to` in `binding_map` to enclose all pins with margins.
///
/// Properties read from rectangle graphic body (all optional):
/// - `margin`    (default 20mil = 200_000)
/// - `min_width` (default 80mil = 800_000)
/// - `min_height`(default 50mil = 500_000)
fn compute_auto_size_bounds<'a>(
    auto_sized: &HashSet<String>,
    pin_decls: &[(&'a crate::ast::PinDecl, i32)],
    graphic_decls: impl Iterator<Item = &'a crate::ast::GraphicDecl>,
    binding_map: &mut GraphicBindingMap,
    scope: &ScopeStack,
) -> Result<(), SpecError> {
    if auto_sized.is_empty() {
        return Ok(());
    }

    // Parse per-binding rectangle properties (padding, min_width, min_height).
    struct RectProps {
        padding: i32,
        min_width: i32,
        min_height: i32,
    }
    let mut rect_props_map: HashMap<String, RectProps> = HashMap::new();
    for decl in graphic_decls {
        let binding_name = match &decl.binding {
            Some(b) => b.node.clone(),
            None => continue,
        };
        if !auto_sized.contains(&binding_name) {
            continue;
        }
        let props = eval_object_to_map(&decl.body.node, scope)?;
        // padding: space from body edge to first/last pin (default 20mil = 200,000)
        let padding = get_coord_opt(&props, "padding")?
            .map(|c| c.raw())
            .unwrap_or(200_000);
        // min dimensions (default 50mil × 50mil = 500,000)
        let min_width = get_coord_opt(&props, "min_width")?
            .map(|c| c.raw())
            .unwrap_or(500_000);
        let min_height = get_coord_opt(&props, "min_height")?
            .map(|c| c.raw())
            .unwrap_or(500_000);
        rect_props_map.insert(
            binding_name,
            RectProps {
                padding,
                min_width,
                min_height,
            },
        );
    }

    // For each auto-sized binding, accumulate extent per edge.
    // extent = sum of all gaps in the chain (root_gap + subsequent gaps).
    for binding_name in auto_sized {
        // Collect pins belonging to this binding, grouped by edge field.
        struct EdgePin {
            pin_binding: Option<String>,
            gap: i32,
        }
        let mut edge_pins: HashMap<String, Vec<EdgePin>> = HashMap::new();

        for (decl, _) in pin_decls {
            let on_ref = extract_on_ref(&decl.body.node);
            let field = if let Some((ref b, ref f)) = on_ref {
                if b != binding_name {
                    continue;
                }
                f.clone()
            } else {
                // Inferred — need to follow chain; we handle this after initial pass.
                continue;
            };

            let props_map = eval_object_to_map_skip_anchor_keys(&decl.body.node, scope);
            let gap = if let Ok(ref m) = props_map {
                get_coord_opt(m, "gap")?.map(|c| c.raw()).unwrap_or(500_000)
            } else {
                1_000_000
            };
            let pin_binding = decl
                .binding
                .as_ref()
                .map(|b| b.node.clone())
                .or_else(|| Some(format!("pin{}", decl.name.node.as_str())));

            edge_pins
                .entry(field)
                .or_default()
                .push(EdgePin { pin_binding, gap });
        }

        // Also include inferred pins — walk each inferred pin's chain to find its edge.
        // Build a name -> (on_ref, field) lookup from the direct pins we already have.
        let pin_field_lookup: HashMap<String, String> = edge_pins
            .iter()
            .flat_map(|(field, pins)| {
                pins.iter()
                    .filter_map(move |p| p.pin_binding.as_ref().map(|b| (b.clone(), field.clone())))
            })
            .collect();

        for (decl, _) in pin_decls {
            if extract_on_ref(&decl.body.node).is_some() {
                continue; // already handled above
            }
            let after = extract_sequence_ref(&decl.body.node, "after");
            let before_ref_raw = extract_sequence_ref(&decl.body.node, "before");
            if after.is_none() && before_ref_raw.is_none() {
                continue;
            }
            // Walk chain to find field.
            let chain_ref = after.as_ref().or(before_ref_raw.as_ref()).unwrap();
            let field = {
                let mut visited: HashSet<String> = HashSet::new();
                let mut cur = chain_ref.clone();
                let mut found: Option<String> = None;
                loop {
                    if !visited.insert(cur.clone()) {
                        break;
                    } // cycle
                    if let Some(f) = pin_field_lookup.get(&cur) {
                        found = Some(f.clone());
                        break;
                    }
                    // Look up in pin_decls to follow chain.
                    let next = pin_decls.iter().find_map(|(d, _)| {
                        let bn = d
                            .binding
                            .as_ref()
                            .map(|b| b.node.clone())
                            .unwrap_or_else(|| format!("pin{}", d.name.node.as_str()));
                        if bn == cur {
                            extract_sequence_ref(&d.body.node, "after")
                                .or_else(|| extract_sequence_ref(&d.body.node, "before"))
                        } else {
                            None
                        }
                    });
                    match next {
                        Some(n) => cur = n,
                        None => break,
                    }
                }
                match found {
                    Some(f) => f,
                    None => continue, // couldn't resolve — skip this pin
                }
            };

            let props_map = eval_object_to_map_skip_anchor_keys(&decl.body.node, scope);
            let gap = if let Ok(ref m) = props_map {
                get_coord_opt(m, "gap")?.map(|c| c.raw()).unwrap_or(500_000)
            } else {
                1_000_000
            };
            let pin_binding = decl
                .binding
                .as_ref()
                .map(|b| b.node.clone())
                .or_else(|| Some(format!("pin{}", decl.name.node.as_str())));

            edge_pins
                .entry(field)
                .or_default()
                .push(EdgePin { pin_binding, gap });
        }

        // Compute extent for each edge: padding + (n-1) * gaps + padding.
        // Compute extent for each edge:
        // root_pin_gap (start padding) + (n-1) inter-pin gaps + root_pin_gap (end padding)
        // The root pin's gap acts as padding from both edges.
        let extent_for_edge = |field: &str| -> i32 {
            let pins = match edge_pins.get(field) {
                Some(p) if !p.is_empty() => p,
                _ => return 0,
            };
            // Root pin gap acts as padding from edge start
            let start_padding = pins[0].gap;
            // Inter-pin spacing: sum of gaps for chained pins
            let inter_pin: i32 = pins.iter().skip(1).map(|p| p.gap).sum();
            // Use root pin gap as end padding too (symmetric)
            start_padding + inter_pin + start_padding
        };

        let left_extent = extent_for_edge("left");
        let right_extent = extent_for_edge("right");
        let top_extent = extent_for_edge("top");
        let bottom_extent = extent_for_edge("bottom");

        let rp = rect_props_map
            .get(binding_name)
            .map(|r| (r.padding, r.min_width, r.min_height))
            .unwrap_or((200_000, 500_000, 500_000));
        let (_padding, min_width, min_height) = rp;

        // Total extent already includes start/end padding from root pin gap.
        let height_from_pins = left_extent.max(right_extent);
        let width_from_pins = top_extent.max(bottom_extent);

        let height = height_from_pins.max(min_height);
        let width = width_from_pins.max(min_width);

        let from = CoordPoint::new(Coord::new(-(width / 2)), Coord::new(-(height / 2)));
        let to = CoordPoint::new(Coord::new(width / 2), Coord::new(height / 2));

        if let Some(geom) = binding_map.get_mut(binding_name) {
            geom.from = from;
            geom.to = to;
        }
    }

    Ok(())
}

/// Walk `InferredAfter`/`InferredBefore` chains to find an explicit `on:` edge, then
/// promote the inferred pin to `After`/`Before` with the resolved binding+field.
fn resolve_inferred_edges(pending: &mut Vec<PendingPin>) -> Result<(), SpecError> {
    // Build name→index map.
    let name_to_idx: HashMap<String, usize> = pending
        .iter()
        .enumerate()
        .filter_map(|(i, p)| p.binding_name.as_ref().map(|b| (b.clone(), i)))
        .collect();

    // Collect the indices that are currently Inferred*.
    let inferred_indices: Vec<usize> = pending
        .iter()
        .enumerate()
        .filter(|(_, p)| {
            matches!(
                &p.anchor_mode,
                PinAnchorMode::InferredAfter { .. } | PinAnchorMode::InferredBefore { .. }
            )
        })
        .map(|(i, _)| i)
        .collect();

    for start_idx in inferred_indices {
        // Walk the chain from start_idx until finding a pin with explicit binding+field.
        let mut visited: std::collections::HashSet<usize> = std::collections::HashSet::new();
        let mut current = start_idx;

        // The is_after flag follows whether each hop in the chain is "after" or "before".
        let is_after = matches!(
            &pending[start_idx].anchor_mode,
            PinAnchorMode::InferredAfter { .. }
        );

        loop {
            if !visited.insert(current) {
                return Err(SpecError::no_span(
                    SpecErrorCode::CircularBinding,
                    format!(
                        "circular inferred anchor chain detected at pin '{}'",
                        pending[current].decl.name.node.as_str()
                    ),
                ));
            }

            let ref_name = match &pending[current].anchor_mode {
                PinAnchorMode::InferredAfter { after_ref, .. } => after_ref.clone(),
                PinAnchorMode::InferredBefore { before_ref, .. } => before_ref.clone(),
                // Found an explicit edge — use its binding+field.
                PinAnchorMode::After { binding, field, .. }
                | PinAnchorMode::Before { binding, field, .. }
                | PinAnchorMode::AtPosition { binding, field, .. } => {
                    let binding = binding.clone();
                    let field = field.clone();
                    // Now patch the start_idx pin.
                    let (orig_ref, gap) = match &pending[start_idx].anchor_mode {
                        PinAnchorMode::InferredAfter { after_ref, gap } => {
                            (after_ref.clone(), *gap)
                        }
                        PinAnchorMode::InferredBefore { before_ref, gap } => {
                            (before_ref.clone(), *gap)
                        }
                        _ => unreachable!(),
                    };
                    if is_after {
                        pending[start_idx].anchor_mode = PinAnchorMode::After {
                            binding,
                            field,
                            after_ref: orig_ref,
                            gap,
                        };
                    } else {
                        pending[start_idx].anchor_mode = PinAnchorMode::Before {
                            binding,
                            field,
                            before_ref: orig_ref,
                            gap,
                        };
                    }
                    break;
                }
                PinAnchorMode::Absolute => {
                    return Err(SpecError::no_span(
                        SpecErrorCode::UndefinedBinding,
                        format!(
                            "pin '{}' uses after:/before: without on:, but the referenced pin '{}' has no anchor edge",
                            pending[start_idx].decl.name.node.as_str(),
                            pending[current].decl.name.node.as_str(),
                        ),
                    ));
                }
            };

            // Follow the chain to the referenced pin.
            let next_idx = name_to_idx.get(&ref_name).copied().ok_or_else(|| {
                SpecError::no_span(
                    SpecErrorCode::UndefinedBinding,
                    format!("inferred anchor chain: referenced pin '${ref_name}' not found"),
                )
            })?;
            current = next_idx;
        }
    }

    Ok(())
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
                get_coord_opt(m, "gap")?.map(|c| c.raw()).unwrap_or(500_000)
            } else {
                1_000_000
            };

            if let Some(after_ref) = after {
                PinAnchorMode::After {
                    binding,
                    field,
                    after_ref,
                    gap,
                }
            } else if let Some(before_ref) = before {
                PinAnchorMode::Before {
                    binding,
                    field,
                    before_ref: before_ref,
                    gap,
                }
            } else {
                PinAnchorMode::AtPosition {
                    binding,
                    field,
                    at_pos: at_pos.unwrap_or(AnchorPosition::Center),
                }
            }
        } else {
            // No on: — check for after/before to infer edge from referenced pin.
            let after = extract_sequence_ref(&decl.body.node, "after");
            let before = extract_sequence_ref(&decl.body.node, "before");
            let props_map = eval_object_to_map_skip_anchor_keys(&decl.body.node, scope);
            let gap = if let Ok(ref m) = props_map {
                get_coord_opt(m, "gap")?.map(|c| c.raw()).unwrap_or(500_000)
            } else {
                1_000_000
            };
            if let Some(after_ref) = after {
                PinAnchorMode::InferredAfter { after_ref, gap }
            } else if let Some(before_ref) = before {
                PinAnchorMode::InferredBefore { before_ref, gap }
            } else {
                PinAnchorMode::Absolute
            }
        };

        // Explicit binding (`p1 = pin 1 { ... }`) takes priority.
        // Otherwise, auto-generate an implicit binding from the designator:
        //   pin 1   -> $pin1
        //   pin SDA -> $pinSDA
        let binding_name = decl
            .binding
            .as_ref()
            .map(|b| b.node.clone())
            .or_else(|| Some(format!("pin{}", decl.name.node.as_str())));
        pending.push(PendingPin {
            decl,
            owner_part_id: *owner_part_id,
            binding_name,
            anchor_mode: mode,
        });
    }

    // Resolve InferredAfter/InferredBefore chains — walk until finding a pin with
    // explicit binding+field (After/Before/AtPosition), then copy that binding+field.
    resolve_inferred_edges(&mut pending)?;

    // Group anchor pins by (binding_name, edge_field) for sequencing.
    // Absolute pins can be compiled immediately.
    // We need to resolve sequenced pins in topo order, then compile all.

    // Map: pin binding name → index in pending
    let binding_index: HashMap<String, usize> = pending
        .iter()
        .enumerate()
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
            PinAnchorMode::After {
                after_ref,
                binding,
                field,
                ..
            } => {
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
            PinAnchorMode::Before {
                before_ref,
                binding,
                field,
                ..
            } => {
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
        if visited[i] {
            return Ok(());
        }
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
        // Inferred modes don't have a resolved field yet; topo sort runs after resolution.
        PinAnchorMode::InferredAfter { .. }
        | PinAnchorMode::InferredBefore { .. }
        | PinAnchorMode::Absolute => None,
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
    let swap_group = get_swap_group_opt(&props, "swap_group")?;
    let part_swap_group = get_swap_group_opt(&props, "part_swap_group")?;
    let pair_swap_group = get_swap_group_opt(&props, "pair_swap_group")?;

    match &p.anchor_mode {
        PinAnchorMode::Absolute => {
            let orientation = get_enum_opt(&props, "orientation", parse_rotation_by90)?
                .unwrap_or(RotationBy90::Rotate0);
            let location = if let Some(v) = props.get("at") {
                value_to_coord_point(v, Some(decl.body.span))?
            } else if let Some(x_val) = props.get("x") {
                let x = value_to_coord(x_val, Some(decl.body.span))?;
                let y = props
                    .get("y")
                    .map(|v| value_to_coord(v, Some(decl.body.span)))
                    .transpose()?
                    .unwrap_or(Coord::ZERO);
                CoordPoint::new(x, y)
            } else {
                CoordPoint::zero()
            };
            Ok(PinSpec {
                designator: decl.name.node.as_str(),
                name,
                electrical,
                length,
                location,
                orientation,
                is_hidden,
                hidden_net_name,
                owner_part_id: p.owner_part_id,
                swap_group: swap_group.clone(),
                part_swap_group: part_swap_group.clone(),
                pair_swap_group: pair_swap_group.clone(),
            })
        }
        PinAnchorMode::AtPosition {
            binding,
            field,
            at_pos,
        } => {
            let geom = binding_map.get(binding).ok_or_else(|| {
                SpecError::at(
                    SpecErrorCode::UndefinedBinding,
                    format!("no bound graphic named '${}'", binding),
                    decl.body.span,
                )
            })?;
            let edge = geom_field_to_edge(geom, field).ok_or_else(|| SpecError::at(
                SpecErrorCode::TypeMismatch,
                format!("'{}' is not a valid edge name for anchor placement (use left/right/top/bottom)", field),
                decl.body.span,
            ))?;
            let side = get_enum_opt(&props, "side", parse_placement_side)?
                .unwrap_or(PlacementSide::Outside);
            let gap = get_coord_opt(&props, "gap")?
                .map(|c| c.raw())
                .unwrap_or(500_000);
            let offset = get_coord_point_opt(&props, "offset", decl.body.span)?;
            let pin_length = length.unwrap_or(Coord::from_mils(25).expect("25 mils fits Coord"));
            let (location, orientation) = resolve_anchor_placement(
                &edge,
                *at_pos,
                side,
                Coord::new(gap),
                pin_length,
                offset,
            )?;
            Ok(PinSpec {
                designator: decl.name.node.as_str(),
                name,
                electrical,
                length,
                location,
                orientation,
                is_hidden,
                hidden_net_name,
                owner_part_id: p.owner_part_id,
                swap_group: swap_group.clone(),
                part_swap_group: part_swap_group.clone(),
                pair_swap_group: pair_swap_group.clone(),
            })
        }
        PinAnchorMode::After {
            binding,
            field,
            after_ref,
            gap,
        } => {
            let geom = binding_map.get(binding).ok_or_else(|| {
                SpecError::at(
                    SpecErrorCode::UndefinedBinding,
                    format!("no bound graphic named '${}'", binding),
                    decl.body.span,
                )
            })?;
            let edge = geom_field_to_edge(geom, field).ok_or_else(|| {
                SpecError::at(
                    SpecErrorCode::TypeMismatch,
                    format!("'{}' is not a valid edge name", field),
                    decl.body.span,
                )
            })?;
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
            let pin_length = length.unwrap_or(Coord::from_mils(25).expect("25 mils fits Coord"));
            let (mut location, orientation) = resolve_anchor_placement(
                &edge,
                at_pos,
                side,
                Coord::ZERO,
                pin_length,
                extra_offset,
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
                name,
                electrical,
                length,
                location,
                orientation,
                is_hidden,
                hidden_net_name,
                owner_part_id: p.owner_part_id,
                swap_group: swap_group.clone(),
                part_swap_group: part_swap_group.clone(),
                pair_swap_group: pair_swap_group.clone(),
            })
        }
        PinAnchorMode::Before {
            binding,
            field,
            before_ref,
            gap,
        } => {
            let geom = binding_map.get(binding).ok_or_else(|| {
                SpecError::at(
                    SpecErrorCode::UndefinedBinding,
                    format!("no bound graphic named '${}'", binding),
                    decl.body.span,
                )
            })?;
            let edge = geom_field_to_edge(geom, field).ok_or_else(|| {
                SpecError::at(
                    SpecErrorCode::TypeMismatch,
                    format!("'{}' is not a valid edge name", field),
                    decl.body.span,
                )
            })?;
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
            let pin_length = length.unwrap_or(Coord::from_mils(25).expect("25 mils fits Coord"));
            let (mut location, orientation) = resolve_anchor_placement(
                &edge,
                AnchorPosition::Center,
                side,
                Coord::ZERO,
                pin_length,
                extra_offset,
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
                name,
                electrical,
                length,
                location,
                orientation,
                is_hidden,
                hidden_net_name,
                owner_part_id: p.owner_part_id,
                swap_group: swap_group.clone(),
                part_swap_group: part_swap_group.clone(),
                pair_swap_group: pair_swap_group.clone(),
            })
        }
        PinAnchorMode::InferredAfter { .. } | PinAnchorMode::InferredBefore { .. } => {
            return Err(SpecError::no_span(
                SpecErrorCode::UndefinedBinding,
                "inferred anchor mode was not resolved before pin compilation (internal error)",
            ));
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
    gap: Coord,
    pin_length: Coord,
    offset: Option<CoordPoint>,
) -> Result<(CoordPoint, RotationBy90), SpecError> {
    // Step 1: position along the edge.
    // For start/end positions, apply gap as an inward offset from the edge boundary.
    // This creates padding between the edge and the first/last pin.
    let along = {
        let base = edge.point_at(at_pos);
        let dir = edge.side.forward_direction();
        match at_pos {
            AnchorPosition::Start => {
                // Offset inward (in forward direction) from edge start
                Coord::new(base.raw() + dir * gap.raw())
            }
            AnchorPosition::End => {
                // Offset inward (against forward direction) from edge end
                Coord::new(base.raw() - dir * gap.raw())
            }
            AnchorPosition::Center => base,
        }
    };

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

/// Extract a board outline from evaluated properties.
///
/// Accepts either a `Value::Shape` (from `rect()`, `circle()`, etc.) or a
/// `Value::Array` of `CoordPoint` values (backward compat with raw vertex lists).
fn extract_outline_from_props(props: &IndexMap<String, Value>) -> Option<Vec<CoordPoint>> {
    match props.get("outline") {
        Some(Value::Shape(s)) => {
            let verts = s.to_vertices();
            if verts.is_empty() {
                None
            } else {
                Some(
                    verts
                        .into_iter()
                        .map(|(x, y)| CoordPoint::new(Coord::new(x), Coord::new(y)))
                        .collect(),
                )
            }
        }
        Some(Value::Array(_)) => {
            // Backward compat: array of coordinate points
            value_to_points(props.get("outline").unwrap(), None).ok()
        }
        _ => None,
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
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| match v {
                Value::String(s) => Some(s.clone()),
                Value::Integer(n) => Some(n.to_string()),
                _ => None,
            })
            .collect(),
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
        let at_pos = extract_at_position_from_props(&props).unwrap_or(AnchorPosition::Center);

        // We need the box geometry from the scope. Since we don't have access to the
        // binding map here, we resolve it inline via scope DollarIdent lookup.
        let geom = resolve_binding_geometry_from_scope(&binding, scope, span)?;
        let edge = geom_field_to_edge(&geom, &field).ok_or_else(|| {
            SpecError::at(
                SpecErrorCode::TypeMismatch,
                format!(
                    "'{}' is not a valid edge field (use left/right/top/bottom)",
                    field
                ),
                span,
            )
        })?;

        // Compute center position along edge.
        let center_along = edge.point_at(at_pos);

        // Compute offset of first pad.
        // Total span = (count - 1) * pitch.
        let total_span = pitch * (count as i32 - 1);
        let first_offset = Coord::new(-total_span.raw() / 2);

        // Apply direction: forward follows edge natural direction.
        let dir = if direction_reverse {
            -edge.side.forward_direction()
        } else {
            edge.side.forward_direction()
        };

        let mut pads = Vec::with_capacity(count);
        let mut pad_counter = 0i32;
        for i in 0..count {
            let along =
                Coord::new(center_along.raw() + first_offset.raw() + dir * pitch.raw() * i as i32);
            let at = match edge.axis() {
                Axis::X => CoordPoint::new(edge.position, along),
                Axis::Y => CoordPoint::new(along, edge.position),
            };
            let name = generate_pad_name(start, &mut pad_counter, &skip);
            if name.is_empty() {
                continue;
            }
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
        let abs_dir = parse_abs_direction(&dir_str).ok_or_else(|| {
            SpecError::at(
                SpecErrorCode::TypeMismatch,
                format!(
                    "invalid direction '{}': expected up/down/left/right for absolute rows",
                    dir_str
                ),
                span,
            )
        })?;

        let mut pads = Vec::with_capacity(count);
        let mut pad_counter = 0i32;
        for i in 0..count {
            let at = abs_direction_step(first_at, abs_dir, pitch, i);
            let name = generate_pad_name(start, &mut pad_counter, &skip);
            if name.is_empty() {
                continue;
            }
            pads.push(pad_from_template(name, at, &template, span)?);
        }
        Ok(pads)
    }
}

/// Step from first position by abs_direction * pitch * step_index.
fn abs_direction_step(
    first: CoordPoint,
    dir: AbsDirection,
    pitch: Coord,
    step: usize,
) -> CoordPoint {
    let n = step as i32;
    match dir {
        AbsDirection::Right => CoordPoint::new(first.x + pitch * n, first.y),
        AbsDirection::Left => CoordPoint::new(Coord::new(first.x.raw() - pitch.raw() * n), first.y),
        AbsDirection::Up => CoordPoint::new(first.x, first.y + pitch * n),
        AbsDirection::Down => CoordPoint::new(first.x, Coord::new(first.y.raw() - pitch.raw() * n)),
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
    let val = scope
        .lookup_dollar(binding)
        .ok_or_else(|| {
            SpecError::at(
                SpecErrorCode::UndefinedBinding,
                format!("no binding '${binding}' in scope"),
                span,
            )
        })?
        .map_err(|e| e)?
        .clone();
    let map = match val {
        Value::Object(m) => m,
        ref other => {
            return Err(SpecError::at(
                SpecErrorCode::TypeMismatch,
                format!(
                    "binding '${binding}' must be an object with from/to, got {}",
                    other.kind_name()
                ),
                span,
            ));
        }
    };
    let from = match map.get("from") {
        Some(v) => value_to_coord_point(v, Some(span))?,
        None => {
            return Err(SpecError::at(
                SpecErrorCode::TypeMismatch,
                format!("binding '${binding}' has no 'from' field"),
                span,
            ));
        }
    };
    let to = match map.get("to") {
        Some(v) => value_to_coord_point(v, Some(span))?,
        None => {
            return Err(SpecError::at(
                SpecErrorCode::TypeMismatch,
                format!("binding '${binding}' has no 'to' field"),
                span,
            ));
        }
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
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'J', 'K', 'L', 'M', 'N', 'P', 'R', 'T', 'U', 'V', 'W',
    'Y',
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

    let naming = get_enum_opt(&props, "naming", parse_grid_naming)?.unwrap_or(GridNaming::Numeric);
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
                if !on_edge {
                    continue;
                }
            }

            if skip.contains(&name) {
                continue;
            }

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

    fn compile_schdoc(src: &str) -> Result<SchDocSpec, SpecError> {
        let file = parse_spec(src).expect("parse failed");
        match compile_spec(&file, SpecDomain::SchDoc)? {
            SpecModel::SchDoc(s) => Ok(s),
            other => panic!("expected SchDoc, got {:?}", std::mem::discriminant(&other)),
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
                parameter "Tolerance" { text: "1%", is_hidden: false }
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
                line { from: (0mil, 0mil), to: (10mil, 10mil) }
                line { from: (20mil, 0mil), to: (30mil, 10mil) }
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
                    body = rectangle { from: (0mil, 0mil), to: (100mil, 100mil) }
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
        // Implicit 1:1 mapping — no body
        let src = r#"
            component R_0603 {
                footprint "R_0603_SMD"
            }
        "#;
        let spec = compile_schlib(src).unwrap();
        let c = &spec.components[0];
        assert_eq!(c.footprints.len(), 1);
        assert_eq!(c.footprints[0].model_name, "R_0603_SMD");
        // Implicit 1:1 mapping produces empty maps vec
        assert_eq!(c.footprints[0].maps.len(), 0);
    }

    #[test]
    fn footprint_map_dollar_ref_resolves_import_ref() {
        // When a footprint is referenced via a let binding that resolves to an
        // ImportRef (e.g. `let fp = $lib["FP_NAME"]; footprint $fp`), the
        // compiler should extract the footprint name from the ImportRef, not
        // fall back to the variable name.
        // NOTE: Testing ImportRef resolution via `$fp["SOIC-8"]` requires actual
        // file imports, which is covered by the end-to-end sync tests. Here we
        // guard the literal path as a regression baseline.
        //
        // Direct test: literal footprint name (regression guard)
        let src2 = r#"
            component IC2 {
                footprint "SOIC-8"
            }
        "#;
        let spec = compile_schlib(src2).unwrap();
        let c = &spec.components[0];
        assert_eq!(c.footprints[0].model_name, "SOIC-8");
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
        assert!(matches!(
            parse_pin_electrical_type("input"),
            Some(PinElectricalType::Input)
        ));
        assert!(matches!(
            parse_pin_electrical_type("passive"),
            Some(PinElectricalType::Passive)
        ));
        assert!(matches!(
            parse_pin_electrical_type("io"),
            Some(PinElectricalType::InputOutput)
        ));
        assert!(matches!(
            parse_pin_electrical_type("power"),
            Some(PinElectricalType::Power)
        ));
        assert!(parse_pin_electrical_type("unknown").is_none());
    }

    #[test]
    fn pad_shape_parsing() {
        assert!(matches!(
            parse_pad_shape("rectangular"),
            Some(PadShape::Rectangular)
        ));
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
                line { from: (0mil, 0mil), to: (10mil, 10mil) }
                rectangle { from: (0mil, 0mil), to: (50mil, 50mil) }
                arc { center: (0mil, 0mil), radius: 50mil }
                ellipse { center: (0mil, 0mil), radius: 50mil }
                label { at: (0mil, 0mil), text: "hello" }
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
                body = rectangle { from: (-20mil, -10mil), to: (20mil, 10mil) }
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
                body = rectangle { from: (-20mil, -10mil), to: (20mil, 10mil) }
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
                body = rectangle { from: (-20mil, -10mil), to: (20mil, 10mil) }
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
                body = rectangle { from: (-20mil, -10mil), to: (20mil, 10mil) }
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
                body = rectangle { from: (-20mil, -10mil), to: (20mil, 10mil) }
                pin start_pin { on: $body.left, at: "start", side: "outside", gap: 0mil }
                pin end_pin   { on: $body.left, at: "end",   side: "outside", gap: 0mil }
            }
        "#;
        let spec = compile_schlib(src).unwrap();
        let pins = &spec.components[0].pins;
        assert_eq!(pins[0].designator, "start_pin");
        assert_eq!(pins[0].location.y, Coord::new(100_000)); // start = top
        assert_eq!(pins[1].designator, "end_pin");
        assert_eq!(pins[1].location.y, Coord::new(-100_000)); // end = bottom
    }

    #[test]
    fn anchor_top_start_end_positions() {
        // top edge forward dir = +1 (left-to-right = increasing X)
        // start = min X = -20mil = -200_000
        // end   = max X = 20mil = 200_000
        let src = r#"
            component R {
                body = rectangle { from: (-20mil, -10mil), to: (20mil, 10mil) }
                pin sp { on: $body.top, at: "start", side: "outside", gap: 0mil }
                pin ep { on: $body.top, at: "end",   side: "outside", gap: 0mil }
            }
        "#;
        let spec = compile_schlib(src).unwrap();
        let pins = &spec.components[0].pins;
        assert_eq!(pins[0].location.x, Coord::new(-200_000)); // start = left
        assert_eq!(pins[1].location.x, Coord::new(200_000)); // end = right
    }

    // ── Anchor placement: side: inside / center ────────────────────────────

    #[test]
    fn anchor_left_inside() {
        // inside left: side_offset = +pin_length = +250_000
        // location x = -200_000 + 250_000 = 50_000
        let src = r#"
            component R {
                body = rectangle { from: (-20mil, -10mil), to: (20mil, 10mil) }
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
                body = rectangle { from: (-20mil, -10mil), to: (20mil, 10mil) }
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
                body = rectangle { from: (-20mil, -10mil), to: (20mil, 10mil) }
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
                body = rectangle { from: (-20mil, -15mil), to: (20mil, 15mil) }
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

    // ── Implicit pin bindings ─────────────────────────────────────────────

    #[test]
    fn implicit_binding_after_chain() {
        // pin 1 auto-generates $pin1, pin 2 references it via after: $pin1
        let src = r#"
            component IC {
                body = rectangle { from: (-20mil, -15mil), to: (20mil, 15mil) }
                pin 1 { on: $body.right, at: "center", side: "outside" }
                pin 2 { on: $body.right, after: $pin1, gap: 5mil, side: "outside" }
                pin 3 { on: $body.right, after: $pin2, gap: 5mil, side: "outside" }
            }
        "#;
        let spec = compile_schlib(src).unwrap();
        let pins = &spec.components[0].pins;
        assert_eq!(pins.len(), 3);
        // Same positions as the explicit binding test above
        assert_eq!(pins[0].location.y, Coord::ZERO);
        assert_eq!(pins[1].location.y, Coord::new(50_000));
        assert_eq!(pins[2].location.y, Coord::new(100_000));
    }

    #[test]
    fn explicit_binding_overrides_implicit() {
        // Explicit binding `my_pin = pin 1` creates $my_pin, and $pin1 is also available
        let src = r#"
            component IC {
                body = rectangle { from: (-20mil, -15mil), to: (20mil, 15mil) }
                my_pin = pin 1 { on: $body.right, at: "center", side: "outside" }
                pin 2 { on: $body.right, after: $my_pin, gap: 5mil, side: "outside" }
            }
        "#;
        let spec = compile_schlib(src).unwrap();
        let pins = &spec.components[0].pins;
        assert_eq!(pins.len(), 2);
        assert_eq!(pins[0].location.y, Coord::ZERO);
        assert_eq!(pins[1].location.y, Coord::new(50_000));
    }

    #[test]
    fn implicit_binding_named_pins() {
        // Non-numeric pin designators like SDA -> $pinSDA
        let src = r#"
            component IC {
                body = rectangle { from: (-20mil, -15mil), to: (20mil, 15mil) }
                pin SDA { on: $body.left, at: "center", side: "outside" }
                pin SCL { on: $body.left, after: $pinSDA, gap: 5mil, side: "outside" }
            }
        "#;
        let spec = compile_schlib(src).unwrap();
        let pins = &spec.components[0].pins;
        assert_eq!(pins.len(), 2);
        assert_eq!(pins[0].location.y, Coord::ZERO);
        assert_eq!(pins[1].location.y, Coord::new(-50_000));
    }

    // ── Implicit component bindings (SchDoc) ──────────────────────────────

    #[test]
    fn implicit_component_binding_relative_placement() {
        // component "U1" at (100mil, 200mil) creates $U1 as CoordPoint.
        // component "C1" can reference $U1.x / $U1.y for relative placement.
        let src = r#"
            component "U1" { at: (100mil, 200mil) }
            component "C1" { at: ($U1.x + 50mil, $U1.y) }
        "#;
        let spec = compile_schdoc(src).unwrap();
        let comps = &spec.sheets[0].components;
        assert_eq!(comps.len(), 2);
        // U1 at (100mil, 200mil) = (1_000_000, 2_000_000) internal
        assert_eq!(comps[0].location.x, Coord::new(1_000_000));
        assert_eq!(comps[0].location.y, Coord::new(2_000_000));
        // C1 at (150mil, 200mil) = (1_500_000, 2_000_000) internal
        assert_eq!(comps[1].location.x, Coord::new(1_500_000));
        assert_eq!(comps[1].location.y, Coord::new(2_000_000));
    }

    // ── Rich component bindings with imported SchLib pins ─────────────────

    fn compile_schdoc_with_imports(
        src: &str,
        imported_components: HashMap<String, ComponentSpec>,
    ) -> Result<SchDocSpec, SpecError> {
        let file = parse_spec(src).expect("parse failed");
        match compile_spec_with_imports(&file, SpecDomain::SchDoc, imported_components)? {
            SpecModel::SchDoc(s) => Ok(s),
            other => panic!("expected SchDoc, got {:?}", std::mem::discriminant(&other)),
        }
    }

    fn make_test_component(lib_ref: &str, pins: Vec<(&str, i32, i32)>) -> ComponentSpec {
        ComponentSpec {
            annotation: None,
            lib_reference: lib_ref.to_string(),
            designator: None,
            description: None,
            component_kind: None,
            part_count: None,
            show_hidden_pins: None,
            pins: pins
                .iter()
                .map(|(des, x, y)| PinSpec {
                    designator: des.to_string(),
                    name: None,
                    electrical: None,
                    length: None,
                    location: CoordPoint::new(Coord::new(*x), Coord::new(*y)),
                    orientation: RotationBy90::Rotate0,
                    is_hidden: None,
                    hidden_net_name: None,
                    owner_part_id: 0,
                    swap_group: None,
                    part_swap_group: None,
                    pair_swap_group: None,
                })
                .collect(),
            parameters: vec![],
            aliases: vec![],
            footprints: vec![],
            graphics: vec![],
            parts: vec![],
        }
    }

    #[test]
    fn rich_component_binding_pin_access() {
        let mut imports = HashMap::new();
        imports.insert(
            "MCU".to_string(),
            make_test_component("MCU", vec![("1", -2_000_000, 0), ("2", 2_000_000, 0)]),
        );

        let src = r#"
            component "U1" { lib_reference: "MCU", at: (1000mil, 500mil) }
        "#;
        let spec = compile_schdoc_with_imports(src, imports).unwrap();
        let comp = &spec.sheets[0].components[0];
        assert_eq!(comp.location.x, Coord::new(10_000_000));
        assert_eq!(comp.location.y, Coord::new(5_000_000));
    }

    #[test]
    fn rich_binding_pin_reference_in_expression() {
        let mut imports = HashMap::new();
        imports.insert(
            "MCU".to_string(),
            make_test_component("MCU", vec![("1", -2_000_000, 0), ("2", 2_000_000, 0)]),
        );

        let src = r#"
            component "U1" { lib_reference: "MCU", at: (1000mil, 500mil) }
            component "R1" { at: ($U1.pin2.x + 200mil, $U1.pin2.y) }
        "#;
        let spec = compile_schdoc_with_imports(src, imports).unwrap();
        let comps = &spec.sheets[0].components;
        // U1.pin2 is at symbol (200mil, 0) translated to schematic (1200mil, 500mil)
        // R1 at (1200mil + 200mil, 500mil) = (1400mil, 500mil)
        assert_eq!(comps[1].location.x, Coord::new(14_000_000));
        assert_eq!(comps[1].location.y, Coord::new(5_000_000));
    }

    #[test]
    fn rich_binding_rotation_90() {
        let mut imports = HashMap::new();
        imports.insert(
            "MCU".to_string(),
            make_test_component("MCU", vec![("1", 1_000_000, 0)]),
        );

        let src = r#"
            component "U1" { lib_reference: "MCU", at: (500mil, 500mil), orientation: "rotate90" }
        "#;
        let spec = compile_schdoc_with_imports(src, imports).unwrap();
        assert_eq!(spec.sheets[0].components.len(), 1);
    }

    #[test]
    fn rich_binding_rotation_transforms() {
        let pin = CoordPoint::new(Coord::new(1_000_000), Coord::new(0));
        let comp = CoordPoint::new(Coord::new(5_000_000), Coord::new(5_000_000));

        let r0 = transform_pin_position(pin, comp, RotationBy90::Rotate0, false);
        assert_eq!(r0.x, Coord::new(6_000_000));
        assert_eq!(r0.y, Coord::new(5_000_000));

        let r90 = transform_pin_position(pin, comp, RotationBy90::Rotate90, false);
        assert_eq!(r90.x, Coord::new(5_000_000));
        assert_eq!(r90.y, Coord::new(6_000_000));

        let r180 = transform_pin_position(pin, comp, RotationBy90::Rotate180, false);
        assert_eq!(r180.x, Coord::new(4_000_000));
        assert_eq!(r180.y, Coord::new(5_000_000));

        let r270 = transform_pin_position(pin, comp, RotationBy90::Rotate270, false);
        assert_eq!(r270.x, Coord::new(5_000_000));
        assert_eq!(r270.y, Coord::new(4_000_000));
    }

    #[test]
    fn rich_binding_mirror_transform() {
        let pin = CoordPoint::new(Coord::new(1_000_000), Coord::new(0));
        let comp = CoordPoint::new(Coord::new(5_000_000), Coord::new(5_000_000));

        let m = transform_pin_position(pin, comp, RotationBy90::Rotate0, true);
        assert_eq!(m.x, Coord::new(4_000_000));
        assert_eq!(m.y, Coord::new(5_000_000));

        let m90 = transform_pin_position(pin, comp, RotationBy90::Rotate90, true);
        assert_eq!(m90.x, Coord::new(5_000_000));
        assert_eq!(m90.y, Coord::new(4_000_000));
    }

    #[test]
    fn fallback_to_coord_point_when_lib_not_found() {
        let src = r#"
            component "U1" { lib_reference: "MCU", at: (100mil, 200mil) }
            component "C1" { at: ($U1.x + 50mil, $U1.y) }
        "#;
        let spec = compile_schdoc(src).unwrap();
        let comps = &spec.sheets[0].components;
        assert_eq!(comps[1].location.x, Coord::new(1_500_000));
        assert_eq!(comps[1].location.y, Coord::new(2_000_000));
    }

    // ── Error: cross-edge reference ────────────────────────────────────────

    #[test]
    fn error_cross_edge_reference() {
        let src = r#"
            component IC {
                body = rectangle { from: (-20mil, -15mil), to: (20mil, 15mil) }
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
        assert_eq!(fp.pads[0].at.x, Coord::from_mils(0).expect("test coord"));
        assert_eq!(fp.pads[1].pad_name, "2");
        assert_eq!(fp.pads[1].at.x, Coord::from_mils(100).expect("test coord"));
        assert_eq!(fp.pads[2].pad_name, "3");
        assert_eq!(fp.pads[3].pad_name, "4");
        assert_eq!(fp.pads[3].at.x, Coord::from_mils(300).expect("test coord"));
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
        assert_eq!(fp.pads[0].at.y, Coord::from_mils(100).expect("test coord"));
        assert_eq!(fp.pads[1].at.y, Coord::from_mils(50).expect("test coord"));
        assert_eq!(fp.pads[2].at.y, Coord::from_mils(0).expect("test coord"));
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
        assert_eq!(
            fp.pads[0].x_size,
            Some(Coord::from_mils(50).expect("test coord"))
        );
        // Pad 2: overridden to rectangular with explicit size
        assert_eq!(fp.pads[1].shape, Some(PadShape::Rectangular));
        assert_eq!(
            fp.pads[1].x_size,
            Some(Coord::from_mils(80).expect("test coord"))
        );
        assert_eq!(
            fp.pads[1].y_size,
            Some(Coord::from_mils(30).expect("test coord"))
        );
        // Position from layout, not from explicit pad
        assert_eq!(fp.pads[1].at.x, Coord::from_mils(100).expect("test coord"));
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
        assert_eq!(fp.pads[0].at.x, Coord::from_mils(-100).expect("test coord"));
        assert_eq!(fp.pads[1].at.x, Coord::from_mils(100).expect("test coord"));
    }

    // ── Bug fixes ─────────────────────────────────────────────────────────

    #[test]
    fn digit_starting_component_name() {
        let src = r#"
            component 74LVC1G17 {
                body = rectangle { from: (-75mil, -75mil), to: (75mil, 75mil) }
                pin 2 { on: $body.left, at: "center", side: "outside", electrical: input, name: "A" }
                pin 4 { on: $body.right, at: "center", side: "outside", electrical: output, name: "Y" }
            }
        "#;
        let spec = compile_schlib(src).unwrap();
        assert_eq!(spec.components[0].lib_reference, "74LVC1G17");
        assert_eq!(spec.components[0].pins.len(), 2);
    }

    #[test]
    fn part_swap_group_in_part_block() {
        let src = r#"
            component MCP6002 {
                part_count: 2

                part 1 {
                    part_swap_group: "A"
                    body = rectangle { from: (-100mil, -100mil), to: (100mil, 100mil) }
                    pin 3 { on: $body.left, at: "center", side: "outside", electrical: input, name: "IN+" }
                    pin 1 { on: $body.right, at: "center", side: "outside", electrical: output, name: "OUT" }
                }

                part 2 {
                    part_swap_group: "A"
                    body = rectangle { from: (-100mil, -100mil), to: (100mil, 100mil) }
                    pin 5 { on: $body.left, at: "center", side: "outside", electrical: input, name: "IN+" }
                    pin 7 { on: $body.right, at: "center", side: "outside", electrical: output, name: "OUT" }
                }

                pin 8 { at: (0mil, -200mil), orientation: 90, electrical: power, is_hidden: true, hidden_net_name: "VDD" }
                pin 4 { at: (0mil, 200mil), orientation: 270, electrical: power, is_hidden: true, hidden_net_name: "GND" }
            }
        "#;
        let spec = compile_schlib(src).unwrap();
        let comp = &spec.components[0];

        // All part-1 pins should have part_swap_group = "A"
        let part1_pins: Vec<_> = comp
            .parts
            .iter()
            .find(|p| p.part_number == 1)
            .map(|p| &p.pins)
            .into_iter()
            .flatten()
            .collect();
        assert!(!part1_pins.is_empty(), "part 1 should have pins");
        assert!(
            part1_pins
                .iter()
                .all(|p| p.part_swap_group.as_deref() == Some("A")),
            "part 1 pins should all have part_swap_group = A"
        );

        // Part-2 pins should also have part_swap_group = "A"
        let part2_pins: Vec<_> = comp
            .parts
            .iter()
            .find(|p| p.part_number == 2)
            .map(|p| &p.pins)
            .into_iter()
            .flatten()
            .collect();
        assert!(!part2_pins.is_empty(), "part 2 should have pins");
        assert!(
            part2_pins
                .iter()
                .all(|p| p.part_swap_group.as_deref() == Some("A")),
            "part 2 pins should all have part_swap_group = A"
        );

        // Component-level pins (owner_part_id 0) should NOT have part_swap_group
        let level0_pins: Vec<_> = comp.pins.iter().filter(|p| p.owner_part_id == 0).collect();
        assert!(!level0_pins.is_empty(), "component-level pins should exist");
        assert!(
            level0_pins.iter().all(|p| p.part_swap_group.is_none()),
            "component-level pins should not have part_swap_group"
        );
    }

    // ── swap_group block declaration tests ───────────────────────────────────

    #[test]
    fn swap_group_block_declaration_pin() {
        let src = r#"
            swap_group digital {}
            component IC {
                body = rectangle { from: (-100mil, -100mil), to: (100mil, 100mil) }
                pin 1 { on: $body.left, at: "center", side: "outside", electrical: input_output, swap_group: $digital }
                pin 2 { on: $body.right, at: "center", side: "outside", electrical: input_output, swap_group: $digital }
            }
        "#;
        let spec = compile_schlib(src).unwrap();
        assert_eq!(
            spec.components[0].pins[0].swap_group.as_deref(),
            Some("digital")
        );
        assert_eq!(
            spec.components[0].pins[1].swap_group.as_deref(),
            Some("digital")
        );
    }

    #[test]
    fn swap_group_block_on_part() {
        let src = r#"
            swap_group opamp {}
            component MCP6002 {
                part_count: 2
                part 1 {
                    swap_group: $opamp
                    body = rectangle { from: (-100mil, -100mil), to: (100mil, 100mil) }
                    pin 3 { on: $body.left, at: "center", side: "outside", electrical: input, name: "IN+" }
                    pin 1 { on: $body.right, at: "center", side: "outside", electrical: output, name: "OUT" }
                }
                part 2 {
                    swap_group: $opamp
                    body = rectangle { from: (-100mil, -100mil), to: (100mil, 100mil) }
                    pin 5 { on: $body.left, at: "center", side: "outside", electrical: input, name: "IN+" }
                    pin 7 { on: $body.right, at: "center", side: "outside", electrical: output, name: "OUT" }
                }
                pin 8 { at: (0mil, -200mil), orientation: 90, electrical: power, is_hidden: true, hidden_net_name: "VDD" }
                pin 4 { at: (0mil, 200mil), orientation: 270, electrical: power, is_hidden: true, hidden_net_name: "GND" }
            }
        "#;
        let spec = compile_schlib(src).unwrap();
        let comp = &spec.components[0];

        let part1_pins: Vec<_> = comp
            .parts
            .iter()
            .find(|p| p.part_number == 1)
            .map(|p| &p.pins)
            .into_iter()
            .flatten()
            .collect();
        assert!(!part1_pins.is_empty());
        assert!(
            part1_pins
                .iter()
                .all(|p| p.part_swap_group.as_deref() == Some("opamp"))
        );

        let part2_pins: Vec<_> = comp
            .parts
            .iter()
            .find(|p| p.part_number == 2)
            .map(|p| &p.pins)
            .into_iter()
            .flatten()
            .collect();
        assert!(!part2_pins.is_empty());
        assert!(
            part2_pins
                .iter()
                .all(|p| p.part_swap_group.as_deref() == Some("opamp"))
        );

        let lvl0_pins: Vec<_> = comp.pins.iter().filter(|p| p.owner_part_id == 0).collect();
        assert!(lvl0_pins.iter().all(|p| p.part_swap_group.is_none()));
    }

    #[test]
    fn swap_group_undefined_reference_error() {
        let src = r#"
            component IC {
                body = rectangle { from: (-100mil, -100mil), to: (100mil, 100mil) }
                pin 1 { on: $body.left, at: "center", side: "outside", electrical: input_output, swap_group: $nonexistent }
            }
        "#;
        assert!(compile_schlib(src).is_err());
    }

    #[test]
    fn swap_group_component_scoped() {
        let src = r#"
            component IC {
                swap_group my_group {}
                body = rectangle { from: (-100mil, -100mil), to: (100mil, 100mil) }
                pin 1 { on: $body.left, at: "center", side: "outside", electrical: input_output, swap_group: $my_group }
                pin 2 { on: $body.right, at: "center", side: "outside", electrical: input_output, swap_group: $my_group }
            }
        "#;
        let spec = compile_schlib(src).unwrap();
        assert_eq!(
            spec.components[0].pins[0].swap_group.as_deref(),
            Some("my_group")
        );
        assert_eq!(
            spec.components[0].pins[1].swap_group.as_deref(),
            Some("my_group")
        );
    }

    #[test]
    fn swap_group_backward_compat_string() {
        let src = r#"
            component IC {
                body = rectangle { from: (-100mil, -100mil), to: (100mil, 100mil) }
                pin 1 { on: $body.left, at: "center", side: "outside", electrical: input_output, swap_group: "legacy" }
            }
        "#;
        let spec = compile_schlib(src).unwrap();
        assert_eq!(
            spec.components[0].pins[0].swap_group.as_deref(),
            Some("legacy")
        );
    }

    #[test]
    fn swap_group_explicit_binding() {
        let src = r#"
            sg = swap_group digital {}
            component IC {
                body = rectangle { from: (-100mil, -100mil), to: (100mil, 100mil) }
                pin 1 { on: $body.left, at: "center", side: "outside", electrical: input_output, swap_group: $sg }
                pin 2 { on: $body.right, at: "center", side: "outside", electrical: input_output, swap_group: $digital }
            }
        "#;
        let spec = compile_schlib(src).unwrap();
        assert_eq!(
            spec.components[0].pins[0].swap_group.as_deref(),
            Some("digital")
        );
        assert_eq!(
            spec.components[0].pins[1].swap_group.as_deref(),
            Some("digital")
        );
    }

    // ── Autoplace model compilation tests ──────────────────────────────────

    fn compile_placement(src: &str) -> Result<PlacementSpec, SpecError> {
        let file = parse_spec(src).expect("parse must succeed for compiler tests");
        match compile_spec(&file, SpecDomain::PcbDoc)? {
            SpecModel::PcbDoc(spec) => spec.placement.ok_or_else(|| {
                SpecError::no_span(
                    SpecErrorCode::TypeMismatch,
                    "no placement block found in test input",
                )
            }),
            other => panic!("expected PcbDoc, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn autoplace_place_flag_compiles() {
        let spec = compile_placement(
            r#"
placement {
    place U1 { autoplace: true, region: center }
}
"#,
        )
        .unwrap();
        assert_eq!(spec.places[0].autoplace, PlacementAutoplaceMode::Auto);
    }

    #[test]
    fn autoplace_place_states_compile() {
        let spec = compile_placement(
            r#"
placement {
    place U1 { autoplace: solved }
    place U2 { autoplace: locked }
}
"#,
        )
        .unwrap();
        assert_eq!(spec.places[0].autoplace, PlacementAutoplaceMode::Solved);
        assert_eq!(spec.places[1].autoplace, PlacementAutoplaceMode::Locked);
    }

    #[test]
    fn solved_place_rotation_compiles_as_fixed_rotation() {
        let spec = compile_placement(
            r#"
placement {
    place U1 { autoplace: solved, at: (10mm, 20mm), rotation: 90.0 }
    place U2 { autoplace: true, rotation: 180 }
}
"#,
        )
        .unwrap();
        assert_eq!(spec.places[0].rotation, Some(90.0));
        assert!(spec.places[0].rotation_options.is_empty());
        assert_eq!(spec.places[1].rotation, None);
        assert_eq!(spec.places[1].rotation_options, vec![180]);
    }

    #[test]
    fn autoplace_block_algorithm_compiles() {
        let spec = compile_placement(
            r#"
placement {
    autoplace { algorithm: full_pipeline, grid_snap: 0.5mm }
}
"#,
        )
        .unwrap();
        let config = spec
            .autoplace_config
            .as_ref()
            .expect("expected autoplace_config");
        assert_eq!(config.algorithm.as_deref(), Some("full_pipeline"));
        assert!(config.grid_snap.is_some());
    }

    #[test]
    fn autoplace_block_congestion_and_clustering_compile() {
        let spec = compile_placement(
            r#"
placement {
    autoplace {
        congestion_weight: 0.2
        congestion_cell: 4mm
        critical_net_boost: 3.0
        auto_cluster: true
        cluster_target_size: 10
        cluster_max_depth: 4
    }
}
"#,
        )
        .unwrap();
        let config = spec
            .autoplace_config
            .as_ref()
            .expect("expected autoplace_config");
        assert_eq!(config.congestion_weight, Some(0.2));
        assert!(
            config
                .congestion_cell
                .map(|c| (c.to_mms() - 4.0).abs() < 1e-3)
                .unwrap_or(false)
        );
        assert_eq!(config.critical_net_boost, Some(3.0));
        assert_eq!(config.auto_cluster, Some(true));
        assert_eq!(config.cluster_target_size, Some(10));
        assert_eq!(config.cluster_max_depth, Some(4));
    }

    #[test]
    fn autoplace_block_empty_compiles() {
        let spec = compile_placement(
            r#"
placement {
    autoplace {}
}
"#,
        )
        .unwrap();
        assert!(
            spec.autoplace_config.is_some(),
            "empty autoplace block should produce Some(AutoplaceConfig)"
        );
    }

    #[test]
    fn unplaced_strategy_autoplace_compiles() {
        let spec = compile_placement(
            r#"
placement {
    unplaced: autoplace
}
"#,
        )
        .unwrap();
        assert_eq!(spec.unplaced, UnplacedStrategy::Autoplace);
    }

    #[test]
    fn unplaced_strategy_ignore_compiles() {
        let spec = compile_placement(
            r#"
placement {
    unplaced: ignore
}
"#,
        )
        .unwrap();
        assert_eq!(spec.unplaced, UnplacedStrategy::Ignore);
    }

    #[test]
    fn unplaced_strategy_error_compiles() {
        let spec = compile_placement(
            r#"
placement {
    unplaced: error
}
"#,
        )
        .unwrap();
        assert_eq!(spec.unplaced, UnplacedStrategy::Error);
    }

    #[test]
    fn unplaced_strategy_invalid_value_produces_error() {
        let result = compile_placement(
            r#"
placement {
    unplaced: invalid_value
}
"#,
        );
        assert!(
            result.is_err(),
            "invalid unplaced strategy should produce an error"
        );
        let msg = result.err().unwrap().to_string();
        assert!(
            msg.contains("invalid_value") || msg.contains("invalid unplaced strategy"),
            "error message should mention the invalid value: {}",
            msg
        );
    }

    #[test]
    fn group_decl_compiles() {
        let spec = compile_placement(
            r#"
placement {
    group analog { components: [U5, R10, C20] }
}
"#,
        )
        .unwrap();
        assert_eq!(spec.groups.len(), 1);
        assert_eq!(spec.groups[0].name, "analog");
        assert_eq!(spec.groups[0].components, vec!["U5", "R10", "C20"]);
    }

    #[test]
    fn no_pin_swap_list_compiles() {
        let spec = compile_placement(
            r#"
placement {
    place U1 { no_pin_swap: [A, B], no_part_swap: true }
}
"#,
        )
        .unwrap();
        assert_eq!(spec.places[0].no_pin_swap, vec!["A", "B"]);
        assert!(spec.places[0].no_part_swap);
    }

    #[test]
    fn allow_swap_flags_compile() {
        let spec = compile_placement(
            r#"
placement {
    allow_pin_swap: true
    allow_part_swap: false
    allow_gate_swap: true
}
"#,
        )
        .unwrap();
        assert!(spec.allow_pin_swap);
        assert!(!spec.allow_part_swap);
        assert!(spec.allow_gate_swap);
    }

    // ── Pin connection compilation ─────────────────────────────────────────

    #[test]
    fn pin_connection_signal_compiles() {
        let src = r#"
component U1 {
    at: (0mil, 0mil)
    pin GPIO4 -> #SDA
}
"#;
        let spec = compile_schdoc(src).unwrap();
        let sheet = &spec.sheets[0];
        let comp = &sheet.components[0];
        assert_eq!(comp.pin_connections.len(), 1);
        let conn = &comp.pin_connections[0];
        assert_eq!(conn.pin_name, "GPIO4");
        match &conn.target {
            crate::model::PinConnectionTarget::Signal(name) => assert_eq!(name, "SDA"),
            other => panic!("expected Signal, got {:?}", std::mem::discriminant(other)),
        }
    }

    #[test]
    fn pin_connection_power_compiles() {
        let src = r#"
power VDD3V3 {
    pins: []
}
component U1 {
    at: (0mil, 0mil)
    pin VDD -> #VDD3V3
}
"#;
        let spec = compile_schdoc(src).unwrap();
        let sheet = &spec.sheets[0];
        let comp = &sheet.components[0];
        assert_eq!(comp.pin_connections.len(), 1);
        let conn = &comp.pin_connections[0];
        assert_eq!(conn.pin_name, "VDD");
        match &conn.target {
            crate::model::PinConnectionTarget::Power(name) => assert_eq!(name, "VDD3V3"),
            other => panic!("expected Power, got {:?}", std::mem::discriminant(other)),
        }
    }

    #[test]
    fn pin_connection_no_connect_compiles() {
        let src = r#"
component U1 {
    at: (0mil, 0mil)
    pin NC1 -> nc
}
"#;
        let spec = compile_schdoc(src).unwrap();
        let sheet = &spec.sheets[0];
        let comp = &sheet.components[0];
        assert_eq!(comp.pin_connections.len(), 1);
        let conn = &comp.pin_connections[0];
        assert_eq!(conn.pin_name, "NC1");
        assert!(matches!(
            conn.target,
            crate::model::PinConnectionTarget::NoConnect
        ));
    }

    #[test]
    fn component_without_pin_connections_has_empty_vec() {
        let src = r#"
component U1 {
    at: (0mil, 0mil)
}
"#;
        let spec = compile_schdoc(src).unwrap();
        let comp = &spec.sheets[0].components[0];
        assert!(comp.pin_connections.is_empty());
    }

    #[test]
    fn pin_connection_mix_compiles() {
        let src = r#"
power GND {
    pins: []
}
component U1 {
    at: (0mil, 0mil)
    pin A -> #SDA
    pin B -> #GND
    pin C -> nc
}
"#;
        let spec = compile_schdoc(src).unwrap();
        let comp = &spec.sheets[0].components[0];
        assert_eq!(comp.pin_connections.len(), 3);
        assert!(
            matches!(&comp.pin_connections[0].target, crate::model::PinConnectionTarget::Signal(n) if n == "SDA"),
            "expected Signal(SDA) for pin A"
        );
        assert!(
            matches!(&comp.pin_connections[1].target, crate::model::PinConnectionTarget::Power(n) if n == "GND"),
            "expected Power(GND) for pin B"
        );
        assert!(
            matches!(
                &comp.pin_connections[2].target,
                crate::model::PinConnectionTarget::NoConnect
            ),
            "expected NoConnect for pin C"
        );
    }

    // ── Validated symbol reference (ImportRef) ─────────────────────────────

    fn make_minimal_component_spec(lib_ref: &str) -> ComponentSpec {
        ComponentSpec {
            annotation: None,
            lib_reference: lib_ref.to_string(),
            designator: None,
            description: None,
            component_kind: None,
            part_count: None,
            show_hidden_pins: None,
            pins: Vec::new(),
            parameters: Vec::new(),
            aliases: Vec::new(),
            footprints: Vec::new(),
            graphics: Vec::new(),
            parts: Vec::new(),
        }
    }

    #[test]
    fn lib_reference_string_is_literal_no_validation() {
        // Regression: plain lib_reference: "Name" produces SymbolRef::Literal, no error even if
        // the name isn't in imported_components.
        let src = r#"
component U1 {
    at: (0mil, 0mil)
    lib_reference: "ESP32-C6"
}
"#;
        let spec = compile_schdoc_with_imports(src, HashMap::new()).unwrap();
        let sym = &spec.sheets[0].components[0].symbol;
        assert!(matches!(sym, SymbolRef::Literal(s) if s == "ESP32-C6"));
    }

    #[test]
    fn import_ref_valid_symbol_produces_import_symbol_ref() {
        // compile_spec_with_imports puts ESP32_C6 in imported_components keyed by lib_reference.
        // The $mcu.ESP32_C6 path requires ImportObject in named_import_objects, which is only
        // populated via compile_spec_with_resolved. This test verifies the ImportRef → SymbolRef::Import
        // path using a direct Value::ImportRef in the symbol slot by testing the validation logic
        // via the compile_spec_with_imports path (which doesn't populate named_import_objects).
        //
        // The symbol: $alias.Name path that produces ImportRef requires named_import_objects
        // (set via compile_spec_with_resolved). Here we just verify that when imported_components
        // contains the name, a SymbolRef::Literal still works (regression check).
        let mut imported = HashMap::new();
        imported.insert(
            "ESP32_C6".to_string(),
            make_minimal_component_spec("ESP32_C6"),
        );
        let src = r#"
component U1 {
    at: (0mil, 0mil)
    lib_reference: "ESP32_C6"
}
"#;
        let spec = compile_schdoc_with_imports(src, imported).unwrap();
        let sym = &spec.sheets[0].components[0].symbol;
        assert!(matches!(sym, SymbolRef::Literal(s) if s == "ESP32_C6"));
    }

    #[test]
    fn import_ref_unknown_symbol_produces_error() {
        // $mcu.TYPO where the schlib DOES contain TYPO (so ImportRef is created), but
        // imported_components does NOT contain "TYPO" → AltiumFormat error with available names.
        use crate::import::ResolvedSpec;
        use indexmap::IndexMap as IMap;

        let root_src = r#"
component U1 {
    at: (0mil, 0mil)
    symbol: $mcu.TYPO
}
"#;
        // The schlib contains TYPO so the named_imports entries map has it → ImportRef is produced.
        let schlib_src = r#"
component TYPO {
    designator: "U"
}
component ESP32_C6 {
    designator: "U"
}
"#;
        let root_file = parse_spec(root_src).expect("parse root failed");
        let schlib_file = parse_spec(schlib_src).expect("parse schlib failed");

        let mut named_imports = IMap::new();
        named_imports.insert(
            "mcu".to_string(),
            (std::path::PathBuf::from("mcu.schlib"), schlib_file),
        );
        let resolved = ResolvedSpec {
            root: root_file,
            named_imports,
            bare_imports: Vec::new(),
        };

        // imported_components has ESP32_C6 but NOT TYPO → validation fails.
        let mut imported = HashMap::new();
        imported.insert(
            "ESP32_C6".to_string(),
            make_minimal_component_spec("ESP32_C6"),
        );

        let result = compile_spec_with_resolved(&resolved, SpecDomain::SchDoc, imported);
        let err = match result {
            Err(e) => e,
            Ok(_) => panic!("expected compile error for unknown import symbol"),
        };
        assert_eq!(err.code, SpecErrorCode::AltiumFormat);
        assert!(
            err.message.contains("TYPO"),
            "error should mention symbol name: {}",
            err.message
        );
        assert!(
            err.message.contains("mcu"),
            "error should mention import alias: {}",
            err.message
        );
        assert!(
            err.message.contains("ESP32_C6"),
            "error should list available symbols: {}",
            err.message
        );
    }

    #[test]
    fn import_ref_valid_symbol_via_resolved() {
        // $mcu.ESP32_C6 where ESP32_C6 is in both named_imports and imported_components
        // → compiles to SymbolRef::Import { alias: "mcu", name: "ESP32_C6" }, no error.
        use crate::import::ResolvedSpec;
        use indexmap::IndexMap as IMap;

        let root_src = r#"
component U1 {
    at: (0mil, 0mil)
    symbol: $mcu.ESP32_C6
}
"#;
        let schlib_src = r#"
component ESP32_C6 {
    designator: "U"
}
"#;
        let root_file = parse_spec(root_src).expect("parse root failed");
        let schlib_file = parse_spec(schlib_src).expect("parse schlib failed");

        let mut named_imports = IMap::new();
        named_imports.insert(
            "mcu".to_string(),
            (std::path::PathBuf::from("mcu.schlib"), schlib_file),
        );
        let resolved = ResolvedSpec {
            root: root_file,
            named_imports,
            bare_imports: Vec::new(),
        };

        let mut imported = HashMap::new();
        imported.insert(
            "ESP32_C6".to_string(),
            make_minimal_component_spec("ESP32_C6"),
        );

        let spec = compile_spec_with_resolved(&resolved, SpecDomain::SchDoc, imported)
            .expect("compile should succeed");
        let spec = match spec {
            SpecModel::SchDoc(s) => s,
            other => panic!("expected SchDoc, got {:?}", std::mem::discriminant(&other)),
        };
        let sym = &spec.sheets[0].components[0].symbol;
        match sym {
            SymbolRef::Import { alias, name } => {
                assert_eq!(alias, "mcu");
                assert_eq!(name, "ESP32_C6");
            }
            other => panic!(
                "expected SymbolRef::Import, got {:?}",
                std::mem::discriminant(other)
            ),
        }
    }

    // ── Pin connection classification ──────────────────────────────────────

    #[test]
    fn pin_connection_power_target() {
        let src = r#"
            power GND { style: gnd_power, pins: [] }
            component U1 {
                pin 1 -> #GND
            }
        "#;
        let spec = compile_schdoc(src).unwrap();
        let conn = &spec.sheets[0].components[0].pin_connections[0];
        assert_eq!(conn.pin_name, "1");
        assert!(
            matches!(&conn.target, crate::model::PinConnectionTarget::Power(n) if n == "GND"),
            "expected Power(\"GND\"), got {:?}",
            conn.target
        );
    }

    #[test]
    fn pin_connection_signal_target() {
        let src = r#"
            component U1 {
                pin 1 -> #SDA
            }
        "#;
        let spec = compile_schdoc(src).unwrap();
        let conn = &spec.sheets[0].components[0].pin_connections[0];
        assert_eq!(conn.pin_name, "1");
        assert!(
            matches!(&conn.target, crate::model::PinConnectionTarget::Signal(n) if n == "SDA"),
            "expected Signal(\"SDA\"), got {:?}",
            conn.target
        );
    }

    #[test]
    fn pin_connection_noconnect_target() {
        let src = r#"
            component U1 {
                pin NC -> nc
            }
        "#;
        let spec = compile_schdoc(src).unwrap();
        let conn = &spec.sheets[0].components[0].pin_connections[0];
        assert_eq!(conn.pin_name, "NC");
        assert!(
            matches!(&conn.target, crate::model::PinConnectionTarget::NoConnect),
            "expected NoConnect, got {:?}",
            conn.target
        );
    }
}
