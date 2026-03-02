use crate::diagnostic::{Span, Spanned, Unit};

/// Top-level parsed query.
#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    pub expr: Spanned<QueryExpr>,
}

/// A query expression — the top-level grammar node.
#[derive(Debug, Clone, PartialEq)]
pub enum QueryExpr {
    /// Union of multiple queries (`,` separator)
    Union(Vec<Spanned<QueryExpr>>),
    /// Logical OR
    Or(Vec<Spanned<QueryExpr>>),
    /// Logical AND
    And(Vec<Spanned<QueryExpr>>),
    /// Logical NOT
    Not(Box<Spanned<QueryExpr>>),
    /// A selector chain (the leaf expression type)
    Selector(SelectorChain),
}

/// A chain of selectors connected by combinators.
///
/// `component > pin:power` → two segments:
///   1. `{ combinator: None, selector: component }`
///   2. `{ combinator: Child, selector: pin:power }`
#[derive(Debug, Clone, PartialEq)]
pub struct SelectorChain {
    pub segments: Vec<SelectorSegment>,
}

/// One segment in a selector chain.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectorSegment {
    pub combinator: Combinator,
    pub selector: Spanned<CompoundSelector>,
}

/// How this segment relates to the previous one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Combinator {
    /// First segment in chain (no combinator).
    None,
    /// Descendant (whitespace separator).
    Descendant,
    /// Direct child (`>`).
    Child,
}

/// A compound selector: base selector + optional attribute filters + pseudo-classes.
#[derive(Debug, Clone, PartialEq)]
pub struct CompoundSelector {
    pub base: Spanned<BaseSelector>,
    pub attrs: Vec<Spanned<AttributeFilter>>,
    pub pseudos: Vec<Spanned<PseudoClass>>,
}

/// The base part of a selector.
#[derive(Debug, Clone, PartialEq)]
pub enum BaseSelector {
    /// Type selector: `component`, `pin`, `track`, etc.
    Type(TypeSelector),
    /// Designator pattern: `R*`, `U1`, `C??`
    DesignatorPattern(DesignatorPattern),
    /// Part number lookup: `$LM358`
    PartNumber(String),
    /// Value pattern: `@10K`
    ValuePattern(String),
    /// Net name: `~VCC`
    NetName(String),
    /// Record ID: `#42`
    RecordId(i64),
    /// Component:pin pattern: `U1:VCC`
    ComponentPin {
        component: String,
        pin: String,
    },
    /// Universal selector (matches anything): `*`
    Any,
}

/// A designator pattern with optional wildcards.
#[derive(Debug, Clone, PartialEq)]
pub struct DesignatorPattern {
    pub prefix: String,
    pub wildcard: Wildcard,
}

/// Wildcard type in a designator pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Wildcard {
    /// No wildcard — exact match
    None,
    /// `*` suffix — match any suffix
    Star,
    /// One or more `?` — match exactly that many characters
    Fixed(usize),
}

/// An attribute filter: `[field op value]`
#[derive(Debug, Clone, PartialEq)]
pub struct AttributeFilter {
    pub field: FieldPath,
    pub op: CompareOp,
    pub value: Spanned<FilterValue>,
}

/// A dotted field path with optional prefix.
///
/// `field.designator` → prefix=Some("field"), name="designator"
/// `designator` → prefix=None, name="designator"
/// `param.Value` → prefix=Some("param"), name="Value"
#[derive(Debug, Clone, PartialEq)]
pub struct FieldPath {
    pub prefix: Option<String>,
    pub name: String,
    pub span: Span,
}

/// Comparison operator in an attribute filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    Eq,         // =
    NotEq,      // !=
    Contains,   // *=
    StartsWith, // ^=
    EndsWith,   // $=
    WordMatch,  // ~=
    Gt,         // >
    Lt,         // <
    Gte,        // >=
    Lte,        // <=
}

/// A value in an attribute filter.
#[derive(Debug, Clone, PartialEq)]
pub enum FilterValue {
    String(String),
    Integer(i64),
    Float(f64),
    Bool(bool),
    Dim(f64, Unit),
    Regex(String),
    /// Bare identifier used as a string value (e.g., `[electrical=power]`)
    Ident(String),
}

/// Pseudo-class filters applied to pins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PseudoClass {
    // Pin electrical types
    Power,
    Input,
    Output,
    Io,
    Passive,
    HiZ,
    OpenCollector,
    OpenEmitter,

    // Component state
    Virtual,
}

/// Known type selectors mapping to high-level API types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeSelector {
    // SchLib types
    Component,
    Pin,
    Parameter,
    Footprint,
    Graphic,
    Line,
    Rectangle,
    RoundRectangle,
    Arc,
    EllipticalArc,
    Ellipse,
    Pie,
    Polyline,
    Polygon,
    Bezier,
    Image,
    Label,
    TextFrame,

    // PcbLib types
    Pad,
    Track,
    Via,
    Fill,
    Region,
    Text,
    PcbArc,
    ComponentBody,

    // SchDoc types
    SchDocComponent,
    Wire,
    Bus,
    NetLabel,
    PowerObject,
    Port,
    Junction,
    NoConnect,
    BusEntry,
    SheetSymbol,
    Note,
    Probe,
    CompileMask,
    Blanket,
    HarnessConnector,
    SignalHarness,
}

impl TypeSelector {
    /// The set of known type selector keyword strings (lowercase).
    pub const KNOWN: &'static [(&'static str, TypeSelector)] = &[
        ("component", TypeSelector::Component),
        ("pin", TypeSelector::Pin),
        ("parameter", TypeSelector::Parameter),
        ("footprint", TypeSelector::Footprint),
        ("graphic", TypeSelector::Graphic),
        ("line", TypeSelector::Line),
        ("rectangle", TypeSelector::Rectangle),
        ("round_rectangle", TypeSelector::RoundRectangle),
        ("arc", TypeSelector::Arc),
        ("elliptical_arc", TypeSelector::EllipticalArc),
        ("ellipse", TypeSelector::Ellipse),
        ("pie", TypeSelector::Pie),
        ("polyline", TypeSelector::Polyline),
        ("polygon", TypeSelector::Polygon),
        ("bezier", TypeSelector::Bezier),
        ("image", TypeSelector::Image),
        ("label", TypeSelector::Label),
        ("text_frame", TypeSelector::TextFrame),
        ("pad", TypeSelector::Pad),
        ("track", TypeSelector::Track),
        ("via", TypeSelector::Via),
        ("fill", TypeSelector::Fill),
        ("region", TypeSelector::Region),
        ("text", TypeSelector::Text),
        ("pcb_arc", TypeSelector::PcbArc),
        ("component_body", TypeSelector::ComponentBody),
        // SchDoc types
        ("schdoc_component", TypeSelector::SchDocComponent),
        ("wire", TypeSelector::Wire),
        ("bus", TypeSelector::Bus),
        ("net_label", TypeSelector::NetLabel),
        ("power_object", TypeSelector::PowerObject),
        ("port", TypeSelector::Port),
        ("junction", TypeSelector::Junction),
        ("no_connect", TypeSelector::NoConnect),
        ("bus_entry", TypeSelector::BusEntry),
        ("sheet_symbol", TypeSelector::SheetSymbol),
        ("note", TypeSelector::Note),
        ("probe", TypeSelector::Probe),
        ("compile_mask", TypeSelector::CompileMask),
        ("blanket", TypeSelector::Blanket),
        ("harness_connector", TypeSelector::HarnessConnector),
        ("signal_harness", TypeSelector::SignalHarness),
    ];

    /// Try to parse a type selector from a keyword string (case-insensitive).
    pub fn from_keyword(s: &str) -> Option<TypeSelector> {
        let lower = s.to_ascii_lowercase();
        Self::KNOWN
            .iter()
            .find(|(k, _)| *k == lower.as_str())
            .map(|(_, v)| *v)
    }
}
