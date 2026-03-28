use altium_format::PcbDoc;
use altium_format::PcbLib;
use altium_format::SchDoc;
use altium_format::SchLib;
use altium_format::api;

use crate::ast::TypeSelector;
use crate::error::{QueryError, QueryErrorCode};
use crate::value::QueryValue;

/// A queryable node in the document tree.
///
/// Wraps high-level API types so the query engine can inspect them uniformly.
#[derive(Debug, Clone)]
pub enum QueryNode {
    Component(api::Component),
    Pin(api::Pin),
    Parameter(api::Parameter),
    FootprintMap(api::FootprintMap),
    Graphic(api::Graphic),
    // PcbLib types
    Footprint(api::Footprint),
    Pad(api::Pad),
    PcbGraphic(api::PcbGraphic),
    // SchDoc types
    SchDocComponent(api::SchDocComponent),
    Wire(api::Wire),
    Bus(api::Bus),
    NetLabel(api::NetLabel),
    PowerObject(api::PowerObject),
    Port(api::Port),
    Junction(api::Junction),
    NoConnect(api::NoConnect),
    BusEntry(api::BusEntry),
    SheetSymbol(api::SheetSymbol),
    Note(api::Note),
    Probe(api::Probe),
    CompileMask(api::CompileMask),
    Blanket(api::Blanket),
    HarnessConnector(api::HarnessConnector),
    SignalHarness(api::SignalHarness),
    SheetEntry(api::SheetEntry),
    ParameterSet(api::ParameterSet),
    // PcbDoc types
    PcbDocTrack(api::Track),
    PcbDocArc(api::Arc),
    PcbDocVia(api::Via),
    PcbDocPad(api::PcbDocPad),
    PcbDocFill(api::Fill),
    PcbDocText(api::PcbDocText),
    PcbDocRegion(api::Region),
    PcbDocComponentBody(api::ComponentBody),
    PcbDocNet(api::Net),
    PcbDocComponent(api::PcbDocComponent),
    PcbDocPolygon(api::Polygon),
    PcbDocRule(api::DesignRule),
    PcbDocClass(api::NetClass),
    PcbDocDimension(api::Dimension),
    PcbDocDifferentialPair(api::DifferentialPair),
}

/// A matched result from query evaluation.
#[derive(Debug, Clone)]
pub struct QueryMatch {
    pub node: QueryNode,
    /// Path from root to this node (e.g., ["Component 'LM358'", "Pin '1'"])
    pub path: Vec<String>,
}

/// Result of evaluating a query against a document.
pub type QueryResultSet = Vec<QueryMatch>;

/// Trait for document types that can be queried.
pub trait Queryable {
    /// Return all root-level nodes in this document.
    fn root_nodes(&self) -> Result<Vec<QueryNode>, QueryError>;
}

impl Queryable for SchLib {
    fn root_nodes(&self) -> Result<Vec<QueryNode>, QueryError> {
        let components = self.components().map_err(|e| {
            QueryError::new(
                QueryErrorCode::DocumentError,
                format!("failed to read components: {e}"),
            )
        })?;
        Ok(components.into_iter().map(QueryNode::Component).collect())
    }
}

impl Queryable for PcbLib {
    fn root_nodes(&self) -> Result<Vec<QueryNode>, QueryError> {
        let footprints = self.footprints();
        Ok(footprints.into_iter().map(QueryNode::Footprint).collect())
    }
}

impl Queryable for SchDoc {
    fn root_nodes(&self) -> Result<Vec<QueryNode>, QueryError> {
        let sheet = self.sheet().map_err(|e| {
            QueryError::new(
                QueryErrorCode::DocumentError,
                format!("failed to read schematic sheet: {e}"),
            )
        })?;
        let mut nodes = Vec::new();
        for obj in sheet.objects {
            match obj {
                api::SheetObject::Component(c) => nodes.push(QueryNode::SchDocComponent(c)),
                api::SheetObject::Wire(w) => nodes.push(QueryNode::Wire(w)),
                api::SheetObject::Bus(b) => nodes.push(QueryNode::Bus(b)),
                api::SheetObject::NetLabel(n) => nodes.push(QueryNode::NetLabel(n)),
                api::SheetObject::PowerObject(p) => nodes.push(QueryNode::PowerObject(p)),
                api::SheetObject::Port(p) => nodes.push(QueryNode::Port(p)),
                api::SheetObject::Junction(j) => nodes.push(QueryNode::Junction(j)),
                api::SheetObject::NoConnect(n) => nodes.push(QueryNode::NoConnect(n)),
                api::SheetObject::BusEntry(b) => nodes.push(QueryNode::BusEntry(b)),
                api::SheetObject::SheetSymbol(s) => nodes.push(QueryNode::SheetSymbol(s)),
                api::SheetObject::Note(n) => nodes.push(QueryNode::Note(n)),
                api::SheetObject::Probe(p) => nodes.push(QueryNode::Probe(p)),
                api::SheetObject::CompileMask(c) => nodes.push(QueryNode::CompileMask(c)),
                api::SheetObject::Blanket(b) => nodes.push(QueryNode::Blanket(b)),
                api::SheetObject::HarnessConnector(h) => nodes.push(QueryNode::HarnessConnector(h)),
                api::SheetObject::SignalHarness(s) => nodes.push(QueryNode::SignalHarness(s)),
                // Sheet-level graphics and parameters are leaf nodes
                api::SheetObject::Graphic(g) => nodes.push(QueryNode::Graphic(g)),
                api::SheetObject::Parameter(p) => nodes.push(QueryNode::Parameter(p)),
                api::SheetObject::ParameterSet(ps) => nodes.push(QueryNode::ParameterSet(ps)),
            }
        }
        Ok(nodes)
    }
}

impl Queryable for PcbDoc {
    fn root_nodes(&self) -> Result<Vec<QueryNode>, QueryError> {
        let board = self.board().map_err(|e| {
            QueryError::new(
                QueryErrorCode::DocumentError,
                format!("failed to read PCB board: {e}"),
            )
        })?;
        let mut nodes = Vec::new();
        for n in board.nets {
            nodes.push(QueryNode::PcbDocNet(n));
        }
        for c in board.components {
            nodes.push(QueryNode::PcbDocComponent(c));
        }
        for t in board.tracks {
            nodes.push(QueryNode::PcbDocTrack(t));
        }
        for a in board.arcs {
            nodes.push(QueryNode::PcbDocArc(a));
        }
        for v in board.vias {
            nodes.push(QueryNode::PcbDocVia(v));
        }
        for p in board.pads {
            nodes.push(QueryNode::PcbDocPad(p));
        }
        for f in board.fills {
            nodes.push(QueryNode::PcbDocFill(f));
        }
        for t in board.texts {
            nodes.push(QueryNode::PcbDocText(t));
        }
        for r in board.regions {
            nodes.push(QueryNode::PcbDocRegion(r));
        }
        for b in board.component_bodies {
            nodes.push(QueryNode::PcbDocComponentBody(b));
        }
        for p in board.polygons {
            nodes.push(QueryNode::PcbDocPolygon(p));
        }
        for r in board.rules {
            nodes.push(QueryNode::PcbDocRule(r));
        }
        for c in board.classes {
            nodes.push(QueryNode::PcbDocClass(c));
        }
        for d in board.dimensions {
            nodes.push(QueryNode::PcbDocDimension(d));
        }
        for dp in board.differential_pairs {
            nodes.push(QueryNode::PcbDocDifferentialPair(dp));
        }
        Ok(nodes)
    }
}

impl QueryNode {
    /// What type selector matches this node?
    pub fn type_selector(&self) -> TypeSelector {
        match self {
            QueryNode::Component(_) => TypeSelector::Component,
            QueryNode::Pin(_) => TypeSelector::Pin,
            QueryNode::Parameter(_) => TypeSelector::Parameter,
            QueryNode::FootprintMap(_) => TypeSelector::Footprint,
            QueryNode::Graphic(g) => graphic_type_selector(g),
            QueryNode::Footprint(_) => TypeSelector::Footprint,
            QueryNode::Pad(_) => TypeSelector::Pad,
            QueryNode::PcbGraphic(g) => pcb_graphic_type_selector(g),
            // SchDoc types
            QueryNode::SchDocComponent(_) => TypeSelector::SchDocComponent,
            QueryNode::Wire(_) => TypeSelector::Wire,
            QueryNode::Bus(_) => TypeSelector::Bus,
            QueryNode::NetLabel(_) => TypeSelector::NetLabel,
            QueryNode::PowerObject(_) => TypeSelector::PowerObject,
            QueryNode::Port(_) => TypeSelector::Port,
            QueryNode::Junction(_) => TypeSelector::Junction,
            QueryNode::NoConnect(_) => TypeSelector::NoConnect,
            QueryNode::BusEntry(_) => TypeSelector::BusEntry,
            QueryNode::SheetSymbol(_) => TypeSelector::SheetSymbol,
            QueryNode::Note(_) => TypeSelector::Note,
            QueryNode::Probe(_) => TypeSelector::Probe,
            QueryNode::CompileMask(_) => TypeSelector::CompileMask,
            QueryNode::Blanket(_) => TypeSelector::Blanket,
            QueryNode::HarnessConnector(_) => TypeSelector::HarnessConnector,
            QueryNode::SignalHarness(_) => TypeSelector::SignalHarness,
            QueryNode::SheetEntry(_) => TypeSelector::SheetEntry,
            QueryNode::ParameterSet(_) => TypeSelector::ParameterSet,
            // PcbDoc types - primitives reuse PcbLib selectors
            QueryNode::PcbDocTrack(_) => TypeSelector::Track,
            QueryNode::PcbDocArc(_) => TypeSelector::PcbArc,
            QueryNode::PcbDocVia(_) => TypeSelector::Via,
            QueryNode::PcbDocPad(_) => TypeSelector::Pad,
            QueryNode::PcbDocFill(_) => TypeSelector::Fill,
            QueryNode::PcbDocText(_) => TypeSelector::Text,
            QueryNode::PcbDocRegion(_) => TypeSelector::Region,
            QueryNode::PcbDocComponentBody(_) => TypeSelector::ComponentBody,
            // PcbDoc named collections
            QueryNode::PcbDocNet(_) => TypeSelector::PcbDocNet,
            QueryNode::PcbDocComponent(_) => TypeSelector::PcbDocComponent,
            QueryNode::PcbDocPolygon(_) => TypeSelector::PcbDocPolygon,
            QueryNode::PcbDocRule(_) => TypeSelector::PcbDocRule,
            QueryNode::PcbDocClass(_) => TypeSelector::PcbDocClass,
            QueryNode::PcbDocDimension(_) => TypeSelector::PcbDocDimension,
            QueryNode::PcbDocDifferentialPair(_) => TypeSelector::PcbDocDifferentialPair,
        }
    }

    /// Get the specific graphic sub-type selector, if applicable.
    pub fn graphic_type_selector(&self) -> Option<TypeSelector> {
        match self {
            QueryNode::Graphic(g) => Some(graphic_type_selector(g)),
            QueryNode::PcbGraphic(g) => Some(pcb_graphic_type_selector(g)),
            _ => None,
        }
    }

    /// Get the `lib_reference` or `display_name` for pattern matching.
    pub fn lib_reference(&self) -> Option<&str> {
        match self {
            QueryNode::Component(c) => Some(&c.lib_reference),
            QueryNode::Footprint(f) => Some(&f.display_name),
            QueryNode::SchDocComponent(c) => Some(&c.lib_reference),
            QueryNode::PcbDocComponent(c) => Some(&c.source_lib_reference),
            _ => None,
        }
    }

    /// Get the designator for pattern matching.
    pub fn designator(&self) -> Option<&str> {
        match self {
            QueryNode::Component(c) => c.designator.as_deref(),
            QueryNode::Pin(p) => Some(&p.designator),
            QueryNode::Pad(p) => Some(&p.pad_name),
            QueryNode::SchDocComponent(c) => Some(&c.designator),
            QueryNode::NetLabel(n) => Some(&n.text),
            QueryNode::PcbDocPad(p) => Some(&p.pad_name),
            QueryNode::PcbDocComponent(c) => Some(&c.designator),
            _ => None,
        }
    }

    /// Get the direct children of this node.
    pub fn children(&self) -> Vec<QueryNode> {
        match self {
            QueryNode::Component(c) => {
                let mut children = Vec::new();
                for pin in &c.pins {
                    children.push(QueryNode::Pin(pin.clone()));
                }
                for param in &c.parameters {
                    children.push(QueryNode::Parameter(param.clone()));
                }
                for fp in &c.footprints {
                    children.push(QueryNode::FootprintMap(fp.clone()));
                }
                for g in &c.graphics {
                    children.push(QueryNode::Graphic(g.clone()));
                }
                children
            }
            QueryNode::Footprint(f) => {
                let mut children = Vec::new();
                for pad in &f.pads {
                    children.push(QueryNode::Pad(pad.clone()));
                }
                for g in &f.graphics {
                    children.push(QueryNode::PcbGraphic(g.clone()));
                }
                children
            }
            // SchDoc container nodes
            QueryNode::SchDocComponent(c) => {
                let mut children = Vec::new();
                for child in &c.children {
                    match child {
                        api::ComponentChild::Pin(p) => children.push(QueryNode::Pin(p.clone())),
                        api::ComponentChild::Parameter(p) => {
                            children.push(QueryNode::Parameter(p.clone()))
                        }
                        api::ComponentChild::Graphic(g) => {
                            children.push(QueryNode::Graphic(g.clone()))
                        }
                        api::ComponentChild::FootprintMap(f) => {
                            children.push(QueryNode::FootprintMap(f.clone()))
                        }
                    }
                }
                children
            }
            QueryNode::SheetSymbol(s) => {
                let mut children = Vec::new();
                for child in &s.children {
                    match child {
                        api::SheetSymbolChild::Entry(e) => {
                            children.push(QueryNode::SheetEntry(e.clone()));
                        }
                        api::SheetSymbolChild::Parameter(p) => {
                            children.push(QueryNode::Parameter(p.clone()));
                        }
                    }
                }
                children
            }
            QueryNode::HarnessConnector(h) => {
                let mut children = Vec::new();
                for child in &h.children {
                    match child {
                        api::HarnessChild::Entry(e) => {
                            children.push(QueryNode::SheetEntry(e.clone()));
                        }
                        api::HarnessChild::Parameter(p) => {
                            children.push(QueryNode::Parameter(p.clone()));
                        }
                        api::HarnessChild::ConnectorType(_) => {
                            // ConnectorType is a string, not a queryable entity
                        }
                    }
                }
                children
            }
            QueryNode::ParameterSet(ps) => ps
                .parameters
                .iter()
                .map(|p| QueryNode::Parameter(p.clone()))
                .collect(),
            // Leaf nodes have no children
            _ => Vec::new(),
        }
    }

    /// Get all descendants (recursive children).
    pub fn descendants(&self) -> Vec<QueryNode> {
        let mut result = Vec::new();
        for child in self.children() {
            result.extend(child.descendants());
            result.push(child);
        }
        result
    }

    /// Extract a field value by canonical name.
    pub fn get_field(&self, name: &str) -> QueryValue {
        match self {
            QueryNode::Component(c) => get_component_field(c, name),
            QueryNode::Pin(p) => get_pin_field(p, name),
            QueryNode::Parameter(p) => get_parameter_field(p, name),
            QueryNode::FootprintMap(f) => get_footprint_map_field(f, name),
            QueryNode::Graphic(g) => get_graphic_field(g, name),
            QueryNode::Footprint(f) => get_pcb_footprint_field(f, name),
            QueryNode::Pad(p) => get_pad_field(p, name),
            QueryNode::PcbGraphic(g) => get_pcb_graphic_field(g, name),
            // SchDoc types
            QueryNode::SchDocComponent(c) => get_schdoc_component_field(c, name),
            QueryNode::Wire(w) => get_wire_field(w, name),
            QueryNode::Bus(b) => get_bus_field(b, name),
            QueryNode::NetLabel(n) => get_net_label_field(n, name),
            QueryNode::PowerObject(p) => get_power_object_field(p, name),
            QueryNode::Port(p) => get_port_field(p, name),
            QueryNode::Junction(j) => get_junction_field(j, name),
            QueryNode::NoConnect(n) => get_no_connect_field(n, name),
            QueryNode::BusEntry(b) => get_bus_entry_field(b, name),
            QueryNode::SheetSymbol(s) => get_sheet_symbol_field(s, name),
            QueryNode::Note(n) => get_note_field(n, name),
            QueryNode::Probe(p) => get_probe_field(p, name),
            QueryNode::CompileMask(c) => get_compile_mask_field(c, name),
            QueryNode::Blanket(b) => get_blanket_field(b, name),
            QueryNode::HarnessConnector(h) => get_harness_connector_field(h, name),
            QueryNode::SignalHarness(s) => get_signal_harness_field(s, name),
            QueryNode::SheetEntry(e) => get_sheet_entry_field(e, name),
            QueryNode::ParameterSet(ps) => get_parameter_set_field(ps, name),
            // PcbDoc types
            QueryNode::PcbDocTrack(t) => get_pcbdoc_track_field(t, name),
            QueryNode::PcbDocArc(a) => get_pcbdoc_arc_field(a, name),
            QueryNode::PcbDocVia(v) => get_pcbdoc_via_field(v, name),
            QueryNode::PcbDocPad(p) => get_pcbdoc_pad_field(p, name),
            QueryNode::PcbDocFill(f) => get_pcbdoc_fill_field(f, name),
            QueryNode::PcbDocText(t) => get_pcbdoc_text_field(t, name),
            QueryNode::PcbDocRegion(r) => get_pcbdoc_region_field(r, name),
            QueryNode::PcbDocComponentBody(b) => get_pcbdoc_component_body_field(b, name),
            QueryNode::PcbDocNet(n) => get_pcbdoc_net_field(n, name),
            QueryNode::PcbDocComponent(c) => get_pcbdoc_component_field(c, name),
            QueryNode::PcbDocPolygon(p) => get_pcbdoc_polygon_field(p, name),
            QueryNode::PcbDocRule(r) => get_pcbdoc_rule_field(r, name),
            QueryNode::PcbDocClass(cl) => get_pcbdoc_class_field(cl, name),
            QueryNode::PcbDocDimension(d) => get_pcbdoc_dimension_field(d, name),
            QueryNode::PcbDocDifferentialPair(dp) => get_pcbdoc_diff_pair_field(dp, name),
        }
    }

    /// Get a parameter value by name (for `param.Name` field paths).
    pub fn get_parameter(&self, name: &str) -> QueryValue {
        match self {
            QueryNode::Component(c) => {
                for param in &c.parameters {
                    if param.name.eq_ignore_ascii_case(name) {
                        return QueryValue::String(param.text.clone());
                    }
                }
                QueryValue::Null
            }
            QueryNode::SchDocComponent(c) => {
                for child in &c.children {
                    if let api::ComponentChild::Parameter(p) = child {
                        if p.name.eq_ignore_ascii_case(name) {
                            return QueryValue::String(p.text.clone());
                        }
                    }
                }
                QueryValue::Null
            }
            _ => QueryValue::Null,
        }
    }

    /// Get the "Value" parameter (sugar for `@value` pattern).
    pub fn value_parameter(&self) -> QueryValue {
        self.get_parameter("Value")
    }

    /// Get the net name associated with this node, if it represents a net-labeling object.
    pub fn net_name(&self) -> Option<&str> {
        match self {
            QueryNode::NetLabel(n) => Some(&n.text),
            QueryNode::PowerObject(p) => Some(&p.text),
            QueryNode::Port(p) => Some(&p.name),
            QueryNode::SheetEntry(e) => Some(&e.name),
            QueryNode::PcbDocNet(n) => Some(&n.name),
            QueryNode::PcbDocTrack(t) => t.net.as_deref(),
            QueryNode::PcbDocArc(a) => a.net.as_deref(),
            QueryNode::PcbDocVia(v) => v.net.as_deref(),
            QueryNode::PcbDocPad(p) => p.net.as_deref(),
            QueryNode::PcbDocFill(f) => f.net.as_deref(),
            QueryNode::PcbDocRegion(r) => r.net.as_deref(),
            QueryNode::PcbDocPolygon(p) => p.net.as_deref(),
            _ => None,
        }
    }

    /// Display name for error messages and path building.
    pub fn display_name(&self) -> String {
        match self {
            QueryNode::Component(c) => format!("Component '{}'", c.lib_reference),
            QueryNode::Pin(p) => format!("Pin '{}'", p.designator),
            QueryNode::Parameter(p) => format!("Parameter '{}'", p.name),
            QueryNode::FootprintMap(f) => format!("Footprint '{}'", f.model_name),
            QueryNode::Graphic(g) => {
                let uid = g.unique_id().unwrap_or("?");
                format!("Graphic '{uid}'")
            }
            QueryNode::Footprint(f) => format!("Footprint '{}'", f.display_name),
            QueryNode::Pad(p) => format!("Pad '{}'", p.pad_name),
            QueryNode::PcbGraphic(_) => "PcbGraphic".to_string(),
            // SchDoc types
            QueryNode::SchDocComponent(c) => format!("Component '{}'", c.designator),
            QueryNode::Wire(w) => format!("Wire '{}'", w.unique_id),
            QueryNode::Bus(b) => format!("Bus '{}'", b.unique_id),
            QueryNode::NetLabel(n) => format!("NetLabel '{}'", n.text),
            QueryNode::PowerObject(p) => format!("PowerObject '{}'", p.text),
            QueryNode::Port(p) => format!("Port '{}'", p.name),
            QueryNode::Junction(j) => format!("Junction '{}'", j.unique_id),
            QueryNode::NoConnect(n) => format!("NoConnect '{}'", n.unique_id),
            QueryNode::BusEntry(b) => format!("BusEntry '{}'", b.unique_id),
            QueryNode::SheetSymbol(s) => format!("SheetSymbol '{}'", s.sheet_name),
            QueryNode::Note(n) => format!("Note '{}'", n.unique_id),
            QueryNode::Probe(p) => format!("Probe '{}'", p.name),
            QueryNode::CompileMask(c) => format!("CompileMask '{}'", c.unique_id),
            QueryNode::Blanket(b) => format!("Blanket '{}'", b.unique_id),
            QueryNode::HarnessConnector(h) => format!("HarnessConnector '{}'", h.unique_id),
            QueryNode::SignalHarness(s) => format!("SignalHarness '{}'", s.unique_id),
            QueryNode::SheetEntry(e) => format!("SheetEntry '{}'", e.name),
            QueryNode::ParameterSet(ps) => format!("ParameterSet '{}'", ps.name),
            // PcbDoc types
            QueryNode::PcbDocTrack(t) => format!("Track '{}'", t.id),
            QueryNode::PcbDocArc(a) => format!("Arc '{}'", a.id),
            QueryNode::PcbDocVia(v) => format!("Via '{}'", v.id),
            QueryNode::PcbDocPad(p) => format!("Pad '{}'", p.pad_name),
            QueryNode::PcbDocFill(f) => format!("Fill '{}'", f.id),
            QueryNode::PcbDocText(t) => format!("Text '{}'", t.text),
            QueryNode::PcbDocRegion(r) => format!("Region '{}'", r.id),
            QueryNode::PcbDocComponentBody(b) => format!("ComponentBody '{}'", b.id),
            QueryNode::PcbDocNet(n) => format!("Net '{}'", n.name),
            QueryNode::PcbDocComponent(c) => format!("Component '{}'", c.designator),
            QueryNode::PcbDocPolygon(p) => format!("Polygon '{}'", p.name),
            QueryNode::PcbDocRule(r) => format!("Rule '{}'", r.name),
            QueryNode::PcbDocClass(c) => format!("Class '{}'", c.name),
            QueryNode::PcbDocDimension(d) => format!("Dimension '{}'", d.id),
            QueryNode::PcbDocDifferentialPair(dp) => format!("DifferentialPair '{}'", dp.name),
        }
    }
}

// ── Type selector mapping ────────────────────────────────────────────────────

fn graphic_type_selector(g: &api::Graphic) -> TypeSelector {
    match g {
        api::Graphic::Line(_) => TypeSelector::Line,
        api::Graphic::Rectangle(_) => TypeSelector::Rectangle,
        api::Graphic::RoundRectangle(_) => TypeSelector::RoundRectangle,
        api::Graphic::Arc(_) => TypeSelector::Arc,
        api::Graphic::EllipticalArc(_) => TypeSelector::EllipticalArc,
        api::Graphic::Ellipse(_) => TypeSelector::Ellipse,
        api::Graphic::Pie(_) => TypeSelector::Pie,
        api::Graphic::Polyline(_) => TypeSelector::Polyline,
        api::Graphic::Polygon(_) => TypeSelector::Polygon,
        api::Graphic::Bezier(_) => TypeSelector::Bezier,
        api::Graphic::Image(_) => TypeSelector::Image,
        api::Graphic::Label(_) => TypeSelector::Label,
        api::Graphic::TextFrame(_) => TypeSelector::TextFrame,
    }
}

fn pcb_graphic_type_selector(g: &api::PcbGraphic) -> TypeSelector {
    match g {
        api::PcbGraphic::Track(_) => TypeSelector::Track,
        api::PcbGraphic::Arc(_) => TypeSelector::PcbArc,
        api::PcbGraphic::Fill(_) => TypeSelector::Fill,
        api::PcbGraphic::Region(_) => TypeSelector::Region,
        api::PcbGraphic::Text(_) => TypeSelector::Text,
        api::PcbGraphic::Via(_) => TypeSelector::Via,
        api::PcbGraphic::ComponentBody(_) => TypeSelector::ComponentBody,
    }
}

// ── Field extraction ─────────────────────────────────────────────────────────

fn get_component_field(c: &api::Component, name: &str) -> QueryValue {
    match name {
        "lib_reference" => QueryValue::String(c.lib_reference.clone()),
        "designator" => match &c.designator {
            Some(d) => QueryValue::String(d.clone()),
            None => QueryValue::Null,
        },
        "description" => match &c.description {
            Some(d) => QueryValue::String(d.clone()),
            None => QueryValue::Null,
        },
        "component_kind" => match &c.component_kind {
            Some(k) => QueryValue::String(format!("{k:?}")),
            None => QueryValue::Null,
        },
        "part_count" => QueryValue::Integer(c.part_count as i64),
        "show_hidden_pins" => QueryValue::Bool(c.show_hidden_pins),
        _ => QueryValue::Null,
    }
}

fn get_pin_field(p: &api::Pin, name: &str) -> QueryValue {
    match name {
        "designator" => QueryValue::String(p.designator.clone()),
        "name" => QueryValue::String(p.name.clone()),
        "electrical" => QueryValue::String(format!("{:?}", p.electrical)),
        "x" => QueryValue::Coord(p.location.x.raw()),
        "y" => QueryValue::Coord(p.location.y.raw()),
        "length" => QueryValue::Coord(p.length.raw()),
        "orientation" => QueryValue::String(format!("{:?}", p.orientation)),
        "is_hidden" => QueryValue::Bool(p.is_hidden),
        "hidden_net_name" => QueryValue::String(p.hidden_net_name.clone()),
        "owner_part_id" => QueryValue::Integer(p.owner_part_id as i64),
        "show_name" => QueryValue::Bool(p.show_name),
        "show_designator" => QueryValue::Bool(p.show_designator),
        "description" => QueryValue::String(p.description.clone()),
        "unique_id" => QueryValue::String(p.unique_id.clone()),
        "color" => QueryValue::Color(p.color.r(), p.color.g(), p.color.b()),
        "is_not_accessible" => QueryValue::Bool(p.is_not_accessible),
        "graphically_locked" => QueryValue::Bool(p.graphically_locked),
        "owner_part_display_mode" => QueryValue::Integer(p.owner_part_display_mode as i64),
        _ => QueryValue::Null,
    }
}

fn get_parameter_field(p: &api::Parameter, name: &str) -> QueryValue {
    match name {
        "name" => QueryValue::String(p.name.clone()),
        "text" => QueryValue::String(p.text.clone()),
        "is_hidden" => QueryValue::Bool(p.is_hidden),
        "read_only" => QueryValue::String(format!("{:?}", p.read_only)),
        "x" => QueryValue::Coord(p.location.x.raw()),
        "y" => QueryValue::Coord(p.location.y.raw()),
        "orientation" => QueryValue::String(format!("{:?}", p.orientation)),
        "color" => QueryValue::Color(p.color.r(), p.color.g(), p.color.b()),
        "font_id" => QueryValue::Integer(p.font_id as i64),
        "justification" => QueryValue::String(format!("{:?}", p.justification)),
        "is_mirrored" => QueryValue::Bool(p.is_mirrored),
        "show_name" => QueryValue::Bool(p.show_name),
        "unique_id" => QueryValue::String(p.unique_id.clone()),
        "not_auto_position" => QueryValue::Bool(p.not_auto_position),
        "param_type" => QueryValue::String(format!("{:?}", p.param_type)),
        "description" => QueryValue::String(p.description.clone()),
        _ => QueryValue::Null,
    }
}

fn get_footprint_map_field(f: &api::FootprintMap, name: &str) -> QueryValue {
    match name {
        "model_name" => QueryValue::String(f.model_name.clone()),
        "description" => QueryValue::String(f.description.clone()),
        "is_current" => QueryValue::Bool(f.is_current),
        _ => QueryValue::Null,
    }
}

fn get_graphic_field(g: &api::Graphic, name: &str) -> QueryValue {
    // Common fields across all graphic variants
    match name {
        "unique_id" => match g.unique_id() {
            Some(id) => QueryValue::String(id.to_string()),
            None => QueryValue::Null,
        },
        "owner_part_id" => QueryValue::Integer(g.owner_part_id() as i64),
        "x" => graphic_location_x(g),
        "y" => graphic_location_y(g),
        "color" => graphic_color(g),
        "is_solid" => graphic_is_solid(g),
        _ => QueryValue::Null,
    }
}

fn graphic_location_x(g: &api::Graphic) -> QueryValue {
    let coord = match g {
        api::Graphic::Line(l) => l.location.x,
        api::Graphic::Rectangle(r) => r.location.x,
        api::Graphic::RoundRectangle(r) => r.location.x,
        api::Graphic::Arc(a) => a.location.x,
        api::Graphic::EllipticalArc(a) => a.location.x,
        api::Graphic::Ellipse(e) => e.location.x,
        api::Graphic::Pie(p) => p.location.x,
        api::Graphic::Polyline(_) | api::Graphic::Polygon(_) | api::Graphic::Bezier(_) => {
            return QueryValue::Null;
        }
        api::Graphic::Image(i) => i.location.x,
        api::Graphic::Label(l) => l.location.x,
        api::Graphic::TextFrame(t) => t.location.x,
    };
    QueryValue::Coord(coord.raw())
}

fn graphic_location_y(g: &api::Graphic) -> QueryValue {
    let coord = match g {
        api::Graphic::Line(l) => l.location.y,
        api::Graphic::Rectangle(r) => r.location.y,
        api::Graphic::RoundRectangle(r) => r.location.y,
        api::Graphic::Arc(a) => a.location.y,
        api::Graphic::EllipticalArc(a) => a.location.y,
        api::Graphic::Ellipse(e) => e.location.y,
        api::Graphic::Pie(p) => p.location.y,
        api::Graphic::Polyline(_) | api::Graphic::Polygon(_) | api::Graphic::Bezier(_) => {
            return QueryValue::Null;
        }
        api::Graphic::Image(i) => i.location.y,
        api::Graphic::Label(l) => l.location.y,
        api::Graphic::TextFrame(t) => t.location.y,
    };
    QueryValue::Coord(coord.raw())
}

fn graphic_color(g: &api::Graphic) -> QueryValue {
    let color = match g {
        api::Graphic::Line(l) => l.color,
        api::Graphic::Rectangle(r) => r.color,
        api::Graphic::RoundRectangle(r) => r.color,
        api::Graphic::Arc(a) => a.color,
        api::Graphic::EllipticalArc(a) => a.color,
        api::Graphic::Ellipse(e) => e.color,
        api::Graphic::Pie(p) => p.color,
        api::Graphic::Polyline(p) => p.color,
        api::Graphic::Polygon(p) => p.color,
        api::Graphic::Bezier(b) => b.color,
        api::Graphic::Image(i) => i.color,
        api::Graphic::Label(l) => l.color,
        api::Graphic::TextFrame(t) => t.color,
    };
    QueryValue::Color(color.r(), color.g(), color.b())
}

fn graphic_is_solid(g: &api::Graphic) -> QueryValue {
    match g {
        api::Graphic::Rectangle(r) => QueryValue::Bool(r.is_solid),
        api::Graphic::RoundRectangle(r) => QueryValue::Bool(r.is_solid),
        api::Graphic::Ellipse(e) => QueryValue::Bool(e.is_solid),
        api::Graphic::Pie(p) => QueryValue::Bool(p.is_solid),
        api::Graphic::Polygon(p) => QueryValue::Bool(p.is_solid),
        api::Graphic::Image(i) => QueryValue::Bool(i.is_solid),
        api::Graphic::TextFrame(t) => QueryValue::Bool(t.is_solid),
        _ => QueryValue::Null,
    }
}

fn get_pcb_footprint_field(f: &api::Footprint, name: &str) -> QueryValue {
    match name {
        "display_name" => QueryValue::String(f.display_name.clone()),
        "description" => QueryValue::String(f.description.clone()),
        "pattern" => QueryValue::String(f.pattern.clone()),
        "height" => QueryValue::Coord(f.height.raw()),
        _ => QueryValue::Null,
    }
}

fn get_pad_field(p: &api::Pad, name: &str) -> QueryValue {
    match name {
        "pad_name" => QueryValue::String(p.pad_name.clone()),
        "x" => QueryValue::Coord(p.location.x.raw()),
        "y" => QueryValue::Coord(p.location.y.raw()),
        "shape" => QueryValue::String(format!("{:?}", p.shape)),
        "x_size" => QueryValue::Coord(p.x_size.raw()),
        "y_size" => QueryValue::Coord(p.y_size.raw()),
        "rotation" => QueryValue::Float(p.rotation),
        "hole_size" => QueryValue::Coord(p.hole_size.raw()),
        "is_plated" => QueryValue::Bool(p.is_plated),
        "layer" => QueryValue::String(format!("{}", p.layer)),
        "pad_mode" => QueryValue::String(format!("{:?}", p.pad_mode)),
        "solder_mask_expansion" => QueryValue::Coord(p.solder_mask_expansion.raw()),
        "paste_mask_expansion" => QueryValue::Coord(p.paste_mask_expansion.raw()),
        "plane_connection" => QueryValue::String(format!("{:?}", p.plane_connection)),
        "relief_conductor_width" => QueryValue::Coord(p.relief_conductor_width.raw()),
        "relief_entries" => QueryValue::Integer(p.relief_entries as i64),
        "relief_air_gap" => QueryValue::Coord(p.relief_air_gap.raw()),
        _ => QueryValue::Null,
    }
}

// ── SchDoc field extraction ───────────────────────────────────────────────────

fn get_schdoc_component_field(c: &api::SchDocComponent, name: &str) -> QueryValue {
    match name {
        "designator" => QueryValue::String(c.designator.clone()),
        "unique_id" => QueryValue::String(c.unique_id.clone()),
        "lib_reference" => QueryValue::String(c.lib_reference.clone()),
        "source_library_name" => QueryValue::String(c.source_library_name.clone()),
        "design_item_id" => QueryValue::String(c.design_item_id.clone()),
        "library_path" => QueryValue::String(c.library_path.clone()),
        "x" => QueryValue::Coord(c.location.x.raw()),
        "y" => QueryValue::Coord(c.location.y.raw()),
        "orientation" => QueryValue::String(format!("{:?}", c.orientation)),
        "is_mirrored" => QueryValue::Bool(c.is_mirrored),
        "description" => match &c.description {
            Some(d) => QueryValue::String(d.clone()),
            None => QueryValue::Null,
        },
        "component_kind" => QueryValue::String(format!("{:?}", c.component_kind)),
        "part_count" => QueryValue::Integer(c.part_count as i64),
        "current_part_id" => QueryValue::Integer(c.current_part_id as i64),
        "show_hidden_pins" => QueryValue::Bool(c.show_hidden_pins),
        _ => QueryValue::Null,
    }
}

fn get_wire_field(w: &api::Wire, name: &str) -> QueryValue {
    match name {
        "unique_id" => QueryValue::String(w.unique_id.clone()),
        "color" => QueryValue::Color(w.color.r(), w.color.g(), w.color.b()),
        "line_width" => QueryValue::String(format!("{:?}", w.line_width)),
        "line_style" => QueryValue::String(format!("{:?}", w.line_style)),
        _ => QueryValue::Null,
    }
}

fn get_bus_field(b: &api::Bus, name: &str) -> QueryValue {
    match name {
        "unique_id" => QueryValue::String(b.unique_id.clone()),
        "color" => QueryValue::Color(b.color.r(), b.color.g(), b.color.b()),
        "line_width" => QueryValue::String(format!("{:?}", b.line_width)),
        _ => QueryValue::Null,
    }
}

fn get_net_label_field(n: &api::NetLabel, name: &str) -> QueryValue {
    match name {
        "unique_id" => QueryValue::String(n.unique_id.clone()),
        "text" => QueryValue::String(n.text.clone()),
        "x" => QueryValue::Coord(n.location.x.raw()),
        "y" => QueryValue::Coord(n.location.y.raw()),
        "orientation" => QueryValue::String(format!("{:?}", n.orientation)),
        "justification" => QueryValue::String(format!("{:?}", n.justification)),
        "font_id" => QueryValue::Integer(n.font_id as i64),
        "color" => QueryValue::Color(n.color.r(), n.color.g(), n.color.b()),
        "is_mirrored" => QueryValue::Bool(n.is_mirrored),
        _ => QueryValue::Null,
    }
}

fn get_power_object_field(p: &api::PowerObject, name: &str) -> QueryValue {
    match name {
        "unique_id" => QueryValue::String(p.unique_id.clone()),
        "text" => QueryValue::String(p.text.clone()),
        "x" => QueryValue::Coord(p.location.x.raw()),
        "y" => QueryValue::Coord(p.location.y.raw()),
        "orientation" => QueryValue::String(format!("{:?}", p.orientation)),
        "style" => QueryValue::String(format!("{:?}", p.style)),
        "show_net_name" => QueryValue::Bool(p.show_net_name),
        "font_id" => QueryValue::Integer(p.font_id as i64),
        "color" => QueryValue::Color(p.color.r(), p.color.g(), p.color.b()),
        "is_cross_sheet_connector" => QueryValue::Bool(p.is_cross_sheet_connector),
        _ => QueryValue::Null,
    }
}

fn get_port_field(p: &api::Port, name: &str) -> QueryValue {
    match name {
        "unique_id" => QueryValue::String(p.unique_id.clone()),
        "name" => QueryValue::String(p.name.clone()),
        "x" => QueryValue::Coord(p.location.x.raw()),
        "y" => QueryValue::Coord(p.location.y.raw()),
        "io_type" => QueryValue::String(format!("{:?}", p.io_type)),
        "style" => QueryValue::String(format!("{:?}", p.style)),
        "width" => QueryValue::Coord(p.width.raw()),
        "height" => QueryValue::Coord(p.height.raw()),
        "color" => QueryValue::Color(p.color.r(), p.color.g(), p.color.b()),
        "area_color" => QueryValue::Color(p.area_color.r(), p.area_color.g(), p.area_color.b()),
        "text_color" => QueryValue::Color(p.text_color.r(), p.text_color.g(), p.text_color.b()),
        "font_id" => QueryValue::Integer(p.font_id as i64),
        "alignment" => QueryValue::String(format!("{:?}", p.alignment)),
        "harness_type" => QueryValue::String(p.harness_type.clone()),
        "auto_size" => QueryValue::Bool(p.auto_size),
        "port_name_is_hidden" => QueryValue::Bool(p.port_name_is_hidden),
        _ => QueryValue::Null,
    }
}

fn get_junction_field(j: &api::Junction, name: &str) -> QueryValue {
    match name {
        "unique_id" => QueryValue::String(j.unique_id.clone()),
        "x" => QueryValue::Coord(j.location.x.raw()),
        "y" => QueryValue::Coord(j.location.y.raw()),
        "color" => QueryValue::Color(j.color.r(), j.color.g(), j.color.b()),
        _ => QueryValue::Null,
    }
}

fn get_no_connect_field(n: &api::NoConnect, name: &str) -> QueryValue {
    match name {
        "unique_id" => QueryValue::String(n.unique_id.clone()),
        "x" => QueryValue::Coord(n.location.x.raw()),
        "y" => QueryValue::Coord(n.location.y.raw()),
        "color" => QueryValue::Color(n.color.r(), n.color.g(), n.color.b()),
        "orientation" => QueryValue::String(format!("{:?}", n.orientation)),
        "symbol" => QueryValue::String(n.symbol.clone()),
        "is_active" => QueryValue::Bool(n.is_active),
        "suppress_all" => QueryValue::Bool(n.suppress_all),
        _ => QueryValue::Null,
    }
}

fn get_bus_entry_field(b: &api::BusEntry, name: &str) -> QueryValue {
    match name {
        "unique_id" => QueryValue::String(b.unique_id.clone()),
        "x" => QueryValue::Coord(b.location.x.raw()),
        "y" => QueryValue::Coord(b.location.y.raw()),
        "color" => QueryValue::Color(b.color.r(), b.color.g(), b.color.b()),
        "line_width" => QueryValue::String(format!("{:?}", b.line_width)),
        _ => QueryValue::Null,
    }
}

fn get_sheet_symbol_field(s: &api::SheetSymbol, name: &str) -> QueryValue {
    match name {
        "unique_id" => QueryValue::String(s.unique_id.clone()),
        "x" => QueryValue::Coord(s.location.x.raw()),
        "y" => QueryValue::Coord(s.location.y.raw()),
        "x_size" => QueryValue::Coord(s.x_size.raw()),
        "y_size" => QueryValue::Coord(s.y_size.raw()),
        "color" => QueryValue::Color(s.color.r(), s.color.g(), s.color.b()),
        "area_color" => QueryValue::Color(s.area_color.r(), s.area_color.g(), s.area_color.b()),
        "is_solid" => QueryValue::Bool(s.is_solid),
        "symbol_type" => QueryValue::String(format!("{:?}", s.symbol_type)),
        "sheet_name" => QueryValue::String(s.sheet_name.clone()),
        "file_name" => QueryValue::String(s.file_name.clone()),
        _ => QueryValue::Null,
    }
}

fn get_note_field(n: &api::Note, name: &str) -> QueryValue {
    match name {
        "unique_id" => QueryValue::String(n.unique_id.clone()),
        "x" => QueryValue::Coord(n.location.x.raw()),
        "y" => QueryValue::Coord(n.location.y.raw()),
        "text" => QueryValue::String(n.text.clone()),
        "author" => QueryValue::String(n.author.clone()),
        "font_id" => QueryValue::Integer(n.font_id as i64),
        "color" => QueryValue::Color(n.color.r(), n.color.g(), n.color.b()),
        "is_solid" => QueryValue::Bool(n.is_solid),
        "collapsed" => QueryValue::Bool(n.collapsed),
        _ => QueryValue::Null,
    }
}

fn get_probe_field(p: &api::Probe, name: &str) -> QueryValue {
    match name {
        "unique_id" => QueryValue::String(p.unique_id.clone()),
        "x" => QueryValue::Coord(p.location.x.raw()),
        "y" => QueryValue::Coord(p.location.y.raw()),
        "color" => QueryValue::Color(p.color.r(), p.color.g(), p.color.b()),
        "orientation" => QueryValue::String(format!("{:?}", p.orientation)),
        "name" => QueryValue::String(p.name.clone()),
        _ => QueryValue::Null,
    }
}

fn get_compile_mask_field(c: &api::CompileMask, name: &str) -> QueryValue {
    match name {
        "unique_id" => QueryValue::String(c.unique_id.clone()),
        "x" => QueryValue::Coord(c.location.x.raw()),
        "y" => QueryValue::Coord(c.location.y.raw()),
        "color" => QueryValue::Color(c.color.r(), c.color.g(), c.color.b()),
        "collapsed" => QueryValue::Bool(c.collapsed),
        _ => QueryValue::Null,
    }
}

fn get_blanket_field(b: &api::Blanket, name: &str) -> QueryValue {
    match name {
        "unique_id" => QueryValue::String(b.unique_id.clone()),
        "x" => QueryValue::Coord(b.location.x.raw()),
        "y" => QueryValue::Coord(b.location.y.raw()),
        "color" => QueryValue::Color(b.color.r(), b.color.g(), b.color.b()),
        "line_style" => QueryValue::String(format!("{:?}", b.line_style)),
        "collapsed" => QueryValue::Bool(b.collapsed),
        _ => QueryValue::Null,
    }
}

fn get_harness_connector_field(h: &api::HarnessConnector, name: &str) -> QueryValue {
    match name {
        "unique_id" => QueryValue::String(h.unique_id.clone()),
        "x" => QueryValue::Coord(h.location.x.raw()),
        "y" => QueryValue::Coord(h.location.y.raw()),
        "x_size" => QueryValue::Coord(h.x_size.raw()),
        "y_size" => QueryValue::Coord(h.y_size.raw()),
        "color" => QueryValue::Color(h.color.r(), h.color.g(), h.color.b()),
        _ => QueryValue::Null,
    }
}

fn get_signal_harness_field(s: &api::SignalHarness, name: &str) -> QueryValue {
    match name {
        "unique_id" => QueryValue::String(s.unique_id.clone()),
        "color" => QueryValue::Color(s.color.r(), s.color.g(), s.color.b()),
        "line_width" => QueryValue::String(format!("{:?}", s.line_width)),
        _ => QueryValue::Null,
    }
}

fn get_sheet_entry_field(e: &api::SheetEntry, name: &str) -> QueryValue {
    match name {
        "unique_id" => QueryValue::String(e.unique_id.clone()),
        "name" => QueryValue::String(e.name.clone()),
        "io_type" => QueryValue::String(format!("{:?}", e.io_type)),
        "side" => QueryValue::String(format!("{:?}", e.side)),
        "distance_from_top" => QueryValue::Coord(e.distance_from_top.raw()),
        "style" => QueryValue::String(format!("{:?}", e.style)),
        "color" => QueryValue::Color(e.color.r(), e.color.g(), e.color.b()),
        "area_color" => QueryValue::Color(e.area_color.r(), e.area_color.g(), e.area_color.b()),
        "text_color" => QueryValue::Color(e.text_color.r(), e.text_color.g(), e.text_color.b()),
        "text_font_id" => QueryValue::Integer(e.text_font_id as i64),
        _ => QueryValue::Null,
    }
}

fn get_parameter_set_field(ps: &api::ParameterSet, name: &str) -> QueryValue {
    match name {
        "unique_id" => QueryValue::String(ps.unique_id.clone()),
        "x" => QueryValue::Coord(ps.location.x.raw()),
        "y" => QueryValue::Coord(ps.location.y.raw()),
        "color" => QueryValue::Color(ps.color.r(), ps.color.g(), ps.color.b()),
        "orientation" => QueryValue::String(format!("{:?}", ps.orientation)),
        "name" => QueryValue::String(ps.name.clone()),
        "style" => QueryValue::Integer(ps.style as i64),
        _ => QueryValue::Null,
    }
}

// ── PcbDoc field extraction ──────────────────────────────────────────────────

fn get_pcbdoc_track_field(t: &api::Track, name: &str) -> QueryValue {
    match name {
        "id" => QueryValue::String(t.id.clone()),
        "layer" => QueryValue::String(format!("{}", t.layer)),
        "net" => opt_string(&t.net),
        "component" => opt_string(&t.component),
        "width" => QueryValue::Coord(t.width.raw()),
        "start_x" => QueryValue::Coord(t.start.x.raw()),
        "start_y" => QueryValue::Coord(t.start.y.raw()),
        "end_x" => QueryValue::Coord(t.end.x.raw()),
        "end_y" => QueryValue::Coord(t.end.y.raw()),
        _ => QueryValue::Null,
    }
}

fn get_pcbdoc_arc_field(a: &api::Arc, name: &str) -> QueryValue {
    match name {
        "id" => QueryValue::String(a.id.clone()),
        "layer" => QueryValue::String(format!("{}", a.layer)),
        "net" => opt_string(&a.net),
        "component" => opt_string(&a.component),
        "center_x" | "x" => QueryValue::Coord(a.center.x.raw()),
        "center_y" | "y" => QueryValue::Coord(a.center.y.raw()),
        "radius" => QueryValue::Coord(a.radius.raw()),
        "start_angle" => QueryValue::Float(a.start_angle),
        "end_angle" => QueryValue::Float(a.end_angle),
        "width" => QueryValue::Coord(a.width.raw()),
        _ => QueryValue::Null,
    }
}

fn get_pcbdoc_via_field(v: &api::Via, name: &str) -> QueryValue {
    match name {
        "id" => QueryValue::String(v.id.clone()),
        "net" => opt_string(&v.net),
        "component" => opt_string(&v.component),
        "x" => QueryValue::Coord(v.location.x.raw()),
        "y" => QueryValue::Coord(v.location.y.raw()),
        "diameter" => QueryValue::Coord(v.diameter.raw()),
        "hole_size" => QueryValue::Coord(v.hole_size.raw()),
        "from_layer" => QueryValue::String(format!("{}", v.from_layer)),
        "to_layer" => QueryValue::String(format!("{}", v.to_layer)),
        _ => QueryValue::Null,
    }
}

fn get_pcbdoc_pad_field(p: &api::PcbDocPad, name: &str) -> QueryValue {
    match name {
        "id" => QueryValue::String(p.id.clone()),
        "pad_name" | "designator" => QueryValue::String(p.pad_name.clone()),
        "layer" => QueryValue::String(format!("{}", p.layer)),
        "net" => opt_string(&p.net),
        "component" => opt_string(&p.component),
        "x" => QueryValue::Coord(p.location.x.raw()),
        "y" => QueryValue::Coord(p.location.y.raw()),
        "shape" => QueryValue::String(format!("{:?}", p.shape)),
        "x_size" => QueryValue::Coord(p.x_size.raw()),
        "y_size" => QueryValue::Coord(p.y_size.raw()),
        "rotation" => QueryValue::Float(p.rotation),
        "hole_size" => QueryValue::Coord(p.hole_size.raw()),
        "is_plated" => QueryValue::Bool(p.is_plated),
        "pad_mode" => QueryValue::String(format!("{:?}", p.pad_mode)),
        "solder_mask_expansion" => QueryValue::Coord(p.solder_mask_expansion.raw()),
        "paste_mask_expansion" => QueryValue::Coord(p.paste_mask_expansion.raw()),
        "plane_connection" => QueryValue::String(format!("{:?}", p.plane_connection)),
        "relief_conductor_width" => QueryValue::Coord(p.relief_conductor_width.raw()),
        "relief_entries" => QueryValue::Integer(p.relief_entries as i64),
        "relief_air_gap" => QueryValue::Coord(p.relief_air_gap.raw()),
        _ => QueryValue::Null,
    }
}

fn get_pcbdoc_fill_field(f: &api::Fill, name: &str) -> QueryValue {
    match name {
        "id" => QueryValue::String(f.id.clone()),
        "layer" => QueryValue::String(format!("{}", f.layer)),
        "net" => opt_string(&f.net),
        "component" => opt_string(&f.component),
        "rotation" => QueryValue::Float(f.rotation),
        _ => QueryValue::Null,
    }
}

fn get_pcbdoc_text_field(t: &api::PcbDocText, name: &str) -> QueryValue {
    match name {
        "id" => QueryValue::String(t.id.clone()),
        "layer" => QueryValue::String(format!("{}", t.layer)),
        "component" => opt_string(&t.component),
        "text" => QueryValue::String(t.text.clone()),
        "x" => QueryValue::Coord(t.location.x.raw()),
        "y" => QueryValue::Coord(t.location.y.raw()),
        "height" => QueryValue::Coord(t.height.raw()),
        "width" => QueryValue::Coord(t.width.raw()),
        "rotation" => QueryValue::Float(t.rotation),
        "is_mirrored" => QueryValue::Bool(t.is_mirrored),
        "is_designator" => QueryValue::Bool(t.is_designator),
        "is_comment" => QueryValue::Bool(t.is_comment),
        _ => QueryValue::Null,
    }
}

fn get_pcbdoc_region_field(r: &api::Region, name: &str) -> QueryValue {
    match name {
        "id" => QueryValue::String(r.id.clone()),
        "layer" => QueryValue::String(format!("{}", r.layer)),
        "net" => opt_string(&r.net),
        "component" => opt_string(&r.component),
        "kind" => QueryValue::String(format!("{:?}", r.kind)),
        "is_board_cutout" => QueryValue::Bool(r.is_board_cutout),
        "is_keepout" => QueryValue::Bool(r.is_keepout),
        _ => QueryValue::Null,
    }
}

fn get_pcbdoc_component_body_field(b: &api::ComponentBody, name: &str) -> QueryValue {
    match name {
        "id" => QueryValue::String(b.id.clone()),
        "layer" => QueryValue::String(format!("{}", b.layer)),
        "component" => opt_string(&b.component),
        "model_name" => QueryValue::String(b.model_name.clone()),
        "standoff_height" => QueryValue::Coord(b.standoff_height.raw()),
        "overall_height" => QueryValue::Coord(b.overall_height.raw()),
        _ => QueryValue::Null,
    }
}

fn get_pcbdoc_net_field(n: &api::Net, name: &str) -> QueryValue {
    match name {
        "id" => QueryValue::String(n.id.clone()),
        "name" => QueryValue::String(n.name.clone()),
        "color" => QueryValue::Color(n.color.r(), n.color.g(), n.color.b()),
        "visible" => QueryValue::Bool(n.visible),
        _ => QueryValue::Null,
    }
}

fn get_pcbdoc_component_field(c: &api::PcbDocComponent, name: &str) -> QueryValue {
    match name {
        "id" => QueryValue::String(c.id.clone()),
        "designator" => QueryValue::String(c.designator.clone()),
        "pattern" => QueryValue::String(c.pattern.clone()),
        "comment" => QueryValue::String(c.comment.clone()),
        "x" => QueryValue::Coord(c.location.x.raw()),
        "y" => QueryValue::Coord(c.location.y.raw()),
        "rotation" => QueryValue::Float(c.rotation),
        "layer" => QueryValue::String(format!("{}", c.layer)),
        "source_library" => QueryValue::String(c.source_library.clone()),
        "source_lib_reference" => QueryValue::String(c.source_lib_reference.clone()),
        _ => QueryValue::Null,
    }
}

fn get_pcbdoc_polygon_field(p: &api::Polygon, name: &str) -> QueryValue {
    match name {
        "id" => QueryValue::String(p.id.clone()),
        "name" => QueryValue::String(p.name.clone()),
        "net" => opt_string(&p.net),
        "layer" => QueryValue::String(format!("{}", p.layer)),
        "connect_style" => QueryValue::String(format!("{:?}", p.connect_style)),
        "pour_order" => QueryValue::Integer(p.pour_order as i64),
        _ => QueryValue::Null,
    }
}

fn get_pcbdoc_rule_field(r: &api::DesignRule, name: &str) -> QueryValue {
    match name {
        "id" => QueryValue::String(r.id.clone()),
        "name" => QueryValue::String(r.name.clone()),
        "kind" => QueryValue::String(format!("{:?}", r.kind)),
        "enabled" => QueryValue::Bool(r.enabled),
        "priority" => QueryValue::Integer(r.priority as i64),
        "scope" => QueryValue::String(r.scope.clone()),
        "comment" => QueryValue::String(r.comment.clone()),
        _ => QueryValue::Null,
    }
}

fn get_pcbdoc_class_field(c: &api::NetClass, name: &str) -> QueryValue {
    match name {
        "id" => QueryValue::String(c.id.clone()),
        "name" => QueryValue::String(c.name.clone()),
        "kind" => QueryValue::String(format!("{:?}", c.kind)),
        _ => QueryValue::Null,
    }
}

fn get_pcbdoc_dimension_field(d: &api::Dimension, name: &str) -> QueryValue {
    match name {
        "id" => QueryValue::String(d.id.clone()),
        "kind" => QueryValue::String(format!("{:?}", d.kind)),
        "layer" => QueryValue::String(format!("{}", d.layer)),
        _ => QueryValue::Null,
    }
}

fn get_pcbdoc_diff_pair_field(dp: &api::DifferentialPair, name: &str) -> QueryValue {
    match name {
        "id" => QueryValue::String(dp.id.clone()),
        "name" => QueryValue::String(dp.name.clone()),
        "positive_net" => QueryValue::String(dp.positive_net.clone()),
        "negative_net" => QueryValue::String(dp.negative_net.clone()),
        _ => QueryValue::Null,
    }
}

fn opt_string(s: &Option<String>) -> QueryValue {
    match s {
        Some(v) => QueryValue::String(v.clone()),
        None => QueryValue::Null,
    }
}

fn get_pcb_graphic_field(g: &api::PcbGraphic, name: &str) -> QueryValue {
    match g {
        api::PcbGraphic::Track(t) => match name {
            "layer" => QueryValue::String(format!("{}", t.layer)),
            "width" => QueryValue::Coord(t.width.raw()),
            _ => QueryValue::Null,
        },
        api::PcbGraphic::Arc(a) => match name {
            "layer" => QueryValue::String(format!("{}", a.layer)),
            "x" => QueryValue::Coord(a.center.x.raw()),
            "y" => QueryValue::Coord(a.center.y.raw()),
            "radius" => QueryValue::Coord(a.radius.raw()),
            "start_angle" => QueryValue::Float(a.start_angle),
            "end_angle" => QueryValue::Float(a.end_angle),
            "width" => QueryValue::Coord(a.width.raw()),
            _ => QueryValue::Null,
        },
        api::PcbGraphic::Fill(f) => match name {
            "layer" => QueryValue::String(format!("{}", f.layer)),
            "rotation" => QueryValue::Float(f.rotation),
            _ => QueryValue::Null,
        },
        api::PcbGraphic::Region(r) => match name {
            "layer" => QueryValue::String(format!("{}", r.layer)),
            _ => QueryValue::Null,
        },
        api::PcbGraphic::Text(t) => match name {
            "layer" => QueryValue::String(format!("{}", t.layer)),
            "text" => QueryValue::String(t.text.clone()),
            "x" => QueryValue::Coord(t.location.x.raw()),
            "y" => QueryValue::Coord(t.location.y.raw()),
            "rotation" => QueryValue::Float(t.rotation),
            "height" => QueryValue::Coord(t.height.raw()),
            "width" => QueryValue::Coord(t.width.raw()),
            "color" => QueryValue::Color(t.color.r(), t.color.g(), t.color.b()),
            _ => QueryValue::Null,
        },
        api::PcbGraphic::Via(v) => match name {
            "layer" => QueryValue::String(format!("{}", v.layer)),
            "x" => QueryValue::Coord(v.location.x.raw()),
            "y" => QueryValue::Coord(v.location.y.raw()),
            "diameter" => QueryValue::Coord(v.diameter.raw()),
            "hole_size" => QueryValue::Coord(v.hole_size.raw()),
            "from_layer" => QueryValue::String(format!("{}", v.from_layer)),
            "to_layer" => QueryValue::String(format!("{}", v.to_layer)),
            _ => QueryValue::Null,
        },
        api::PcbGraphic::ComponentBody(cb) => match name {
            "layer" => QueryValue::String(format!("{}", cb.layer)),
            _ => QueryValue::Null,
        },
    }
}
