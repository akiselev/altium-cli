use altium_format::api;
use altium_format::SchLib;

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
            _ => None,
        }
    }

    /// Get the designator for pattern matching.
    pub fn designator(&self) -> Option<&str> {
        match self {
            QueryNode::Component(c) => c.designator.as_deref(),
            QueryNode::Pin(p) => Some(&p.designator),
            QueryNode::Pad(p) => Some(&p.pad_name),
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
            _ => QueryValue::Null,
        }
    }

    /// Get the "Value" parameter (sugar for `@value` pattern).
    pub fn value_parameter(&self) -> QueryValue {
        self.get_parameter("Value")
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
