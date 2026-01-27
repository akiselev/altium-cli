//! Selector AST types for querying schematic records.
//!
//! The selector syntax provides ergonomic shortcuts for common queries:
//!
//! | Pattern | Meaning |
//! |---------|---------|
//! | `U1` | Component by designator |
//! | `R*` | Components matching pattern |
//! | `U1:3` | Pin by number |
//! | `U1:VCC` | Pin by name |
//! | `$LM358` | Component by part number |
//! | `~VCC` | Net by name |
//! | `@10K` | Component by value |
//! | `#Power` | Sheet by name |

use super::common::FilterOp;
use super::pattern::Pattern;

/// A parsed selector, which may contain multiple alternatives (comma-separated).
#[derive(Debug, Clone)]
pub struct Selector {
    /// Union of selector chains (comma-separated alternatives).
    pub alternatives: Vec<SelectorChain>,
}

impl Selector {
    /// Create a selector with a single chain.
    pub fn single(chain: SelectorChain) -> Self {
        Self {
            alternatives: vec![chain],
        }
    }

    /// Create a selector matching any record.
    pub fn any() -> Self {
        Self::single(SelectorChain::single(SelectorSegment::any()))
    }

    /// Returns true if this selector has no segments.
    pub fn is_empty(&self) -> bool {
        self.alternatives.is_empty() || self.alternatives.iter().all(|c| c.segments.is_empty())
    }
}

/// A chain of selector segments connected by combinators.
#[derive(Debug, Clone)]
pub struct SelectorChain {
    /// Segments in this chain, each connected to the next by its combinator.
    pub segments: Vec<SelectorSegment>,
}

impl SelectorChain {
    /// Create a chain with a single segment.
    pub fn single(segment: SelectorSegment) -> Self {
        Self {
            segments: vec![segment],
        }
    }

    /// Create an empty chain.
    pub fn empty() -> Self {
        Self { segments: vec![] }
    }

    /// Add a segment to this chain.
    pub fn push(&mut self, segment: SelectorSegment) {
        self.segments.push(segment);
    }
}

/// A single segment in a selector chain.
#[derive(Debug, Clone)]
pub struct SelectorSegment {
    /// What type of record to match.
    pub matcher: RecordMatcher,
    /// Property filters `[prop op value]`.
    pub filters: Vec<PropertyFilter>,
    /// Pseudo-selectors like `:connected`, `:has()`.
    pub pseudo: Vec<PseudoSelector>,
    /// How this segment connects to the next (if any).
    pub combinator: Option<Combinator>,
}

impl SelectorSegment {
    /// Create a segment matching any record.
    pub fn any() -> Self {
        Self {
            matcher: RecordMatcher::Any,
            filters: vec![],
            pseudo: vec![],
            combinator: None,
        }
    }

    /// Create a segment from a matcher.
    pub fn from_matcher(matcher: RecordMatcher) -> Self {
        Self {
            matcher,
            filters: vec![],
            pseudo: vec![],
            combinator: None,
        }
    }

    /// Add a property filter to this segment.
    pub fn with_filter(mut self, filter: PropertyFilter) -> Self {
        self.filters.push(filter);
        self
    }

    /// Add a pseudo-selector to this segment.
    pub fn with_pseudo(mut self, pseudo: PseudoSelector) -> Self {
        self.pseudo.push(pseudo);
        self
    }

    /// Set the combinator for this segment.
    pub fn with_combinator(mut self, combinator: Combinator) -> Self {
        self.combinator = Some(combinator);
        self
    }
}

/// What records a segment matches.
#[derive(Debug, Clone)]
pub enum RecordMatcher {
    /// Match any record type: `*`
    Any,

    /// Match specific record type: `component`, `pin`, `wire`
    Type(RecordType),

    /// Match by designator pattern: `U1`, `R*`, `C??`
    Designator(Pattern),

    /// Match by part number / library reference: `$LM358`, `$STM32*`
    PartNumber(Pattern),

    /// Match by net name: `~VCC`, `~SPI_*`
    Net(Pattern),

    /// Match by value parameter: `@10K`, `@100nF`
    Value(Pattern),

    /// Match by sheet name: `#Power`, `#Analog`
    Sheet(Pattern),

    /// Pin access: `U1:3`, `U1:VCC`, `R*:1`
    Pin {
        /// Component designator pattern
        component: Pattern,
        /// Pin designator or name pattern
        pin: Pattern,
    },

    /// Net connectivity query: `~VCC:pins`, `~GND:components`
    NetConnected {
        /// Net name pattern
        net: Pattern,
        /// What to return
        target: NetConnectedTarget,
    },

    /// Combined designator + value: `R*@10K`
    DesignatorWithValue {
        /// Designator pattern
        designator: Pattern,
        /// Value pattern
        value: Pattern,
    },
}

/// Target for net connectivity queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetConnectedTarget {
    /// Return pins connected to the net
    Pins,
    /// Return components connected to the net
    Components,
}

/// Record types that can be matched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecordType {
    Component,
    Pin,
    Wire,
    NetLabel,
    Port,
    PowerObject,
    Junction,
    Label,
    Rectangle,
    Line,
    Arc,
    Ellipse,
    Polygon,
    Polyline,
    Bezier,
    Image,
    Parameter,
    Sheet,
    Symbol,
    Designator,
    TextFrame,
}

impl RecordType {
    /// Parse a record type from a string.
    pub fn try_parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "component" => Some(Self::Component),
            "pin" => Some(Self::Pin),
            "wire" => Some(Self::Wire),
            "netlabel" | "net_label" => Some(Self::NetLabel),
            "port" => Some(Self::Port),
            "power" | "powerobject" | "power_object" => Some(Self::PowerObject),
            "junction" => Some(Self::Junction),
            "label" => Some(Self::Label),
            "rectangle" | "rect" => Some(Self::Rectangle),
            "line" => Some(Self::Line),
            "arc" => Some(Self::Arc),
            "ellipse" => Some(Self::Ellipse),
            "polygon" => Some(Self::Polygon),
            "polyline" => Some(Self::Polyline),
            "bezier" => Some(Self::Bezier),
            "image" => Some(Self::Image),
            "parameter" | "param" => Some(Self::Parameter),
            "sheet" => Some(Self::Sheet),
            "symbol" => Some(Self::Symbol),
            "designator" => Some(Self::Designator),
            "textframe" | "text_frame" => Some(Self::TextFrame),
            _ => None,
        }
    }

    /// Get the string name of this record type.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Component => "component",
            Self::Pin => "pin",
            Self::Wire => "wire",
            Self::NetLabel => "netlabel",
            Self::Port => "port",
            Self::PowerObject => "power",
            Self::Junction => "junction",
            Self::Label => "label",
            Self::Rectangle => "rectangle",
            Self::Line => "line",
            Self::Arc => "arc",
            Self::Ellipse => "ellipse",
            Self::Polygon => "polygon",
            Self::Polyline => "polyline",
            Self::Bezier => "bezier",
            Self::Image => "image",
            Self::Parameter => "parameter",
            Self::Sheet => "sheet",
            Self::Symbol => "symbol",
            Self::Designator => "designator",
            Self::TextFrame => "textframe",
        }
    }
}

/// A property filter like `[rotation=90]` or `[x>1000]`.
#[derive(Debug, Clone)]
pub struct PropertyFilter {
    /// Property name (case-insensitive)
    pub property: String,
    /// Comparison operator
    pub operator: FilterOperator,
    /// Value to compare against
    pub value: FilterValue,
}

impl PropertyFilter {
    /// Create a new property filter.
    pub fn new(property: impl Into<String>, operator: FilterOperator, value: FilterValue) -> Self {
        Self {
            property: property.into(),
            operator,
            value,
        }
    }

    /// Create an equality filter.
    pub fn eq(property: impl Into<String>, value: impl Into<String>) -> Self {
        Self::new(
            property,
            FilterOperator::Equal,
            FilterValue::String(value.into()),
        )
    }

    /// Create a wildcard filter.
    pub fn matches(property: impl Into<String>, pattern: Pattern) -> Self {
        Self::new(
            property,
            FilterOperator::Wildcard,
            FilterValue::Pattern(pattern),
        )
    }
}

/// Comparison operators for property filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterOperator {
    /// Exact match: `=`
    Equal,
    /// Not equal: `!=`
    NotEqual,
    /// Wildcard/pattern match: `~=`
    Wildcard,
    /// Starts with: `^=`
    StartsWith,
    /// Ends with: `$=`
    EndsWith,
    /// Contains: `*=`
    Contains,
    /// Greater than (numeric): `>`
    GreaterThan,
    /// Less than (numeric): `<`
    LessThan,
    /// Greater or equal (numeric): `>=`
    GreaterOrEqual,
    /// Less or equal (numeric): `<=`
    LessOrEqual,
}

impl FilterOperator {
    /// Parse an operator from its string representation.
    pub fn try_parse(s: &str) -> Option<Self> {
        match s {
            "=" => Some(Self::Equal),
            "!=" => Some(Self::NotEqual),
            "~=" => Some(Self::Wildcard),
            "^=" => Some(Self::StartsWith),
            "$=" => Some(Self::EndsWith),
            "*=" => Some(Self::Contains),
            ">" => Some(Self::GreaterThan),
            "<" => Some(Self::LessThan),
            ">=" => Some(Self::GreaterOrEqual),
            "<=" => Some(Self::LessOrEqual),
            _ => None,
        }
    }

    /// Convert to shared FilterOp type.
    pub fn to_filter_op(&self) -> FilterOp {
        match self {
            Self::Equal => FilterOp::Equals,
            Self::NotEqual => FilterOp::NotEquals,
            Self::Wildcard => FilterOp::WordMatch, // Closest equivalent
            Self::StartsWith => FilterOp::StartsWith,
            Self::EndsWith => FilterOp::EndsWith,
            Self::Contains => FilterOp::Contains,
            Self::GreaterThan => FilterOp::GreaterThan,
            Self::LessThan => FilterOp::LessThan,
            Self::GreaterOrEqual => FilterOp::GreaterOrEqual,
            Self::LessOrEqual => FilterOp::LessOrEqual,
        }
    }

    /// Create from shared FilterOp type.
    pub fn from_filter_op(op: FilterOp) -> Self {
        match op {
            FilterOp::Exists => Self::Equal, // No direct equivalent, use equal
            FilterOp::Equals => Self::Equal,
            FilterOp::NotEquals => Self::NotEqual,
            FilterOp::WordMatch => Self::Wildcard,
            FilterOp::StartsWith => Self::StartsWith,
            FilterOp::EndsWith => Self::EndsWith,
            FilterOp::Contains => Self::Contains,
            FilterOp::GreaterThan => Self::GreaterThan,
            FilterOp::LessThan => Self::LessThan,
            FilterOp::GreaterOrEqual => Self::GreaterOrEqual,
            FilterOp::LessOrEqual => Self::LessOrEqual,
        }
    }
}

/// Value in a property filter.
#[derive(Debug, Clone)]
pub enum FilterValue {
    /// String value
    String(String),
    /// Numeric value
    Number(f64),
    /// Boolean value
    Bool(bool),
    /// Pattern for wildcard matching
    Pattern(Pattern),
}

/// Pseudo-selectors for additional filtering.
#[derive(Debug, Clone)]
pub enum PseudoSelector {
    // === Hierarchy ===
    /// Top-level records with no parent
    Root,
    /// Records with no children
    Empty,
    /// First child of parent
    FirstChild,
    /// Last child of parent
    LastChild,
    /// Nth child (1-indexed)
    NthChild(usize),
    /// Only child of parent
    OnlyChild,

    // === Electrical state (for pins) ===
    /// Connected to a net
    Connected,
    /// Not connected to any net
    Unconnected,
    /// Input pin
    Input,
    /// Output pin
    Output,
    /// Bidirectional pin
    Bidirectional,
    /// Power pin
    Power,
    /// Passive pin
    Passive,
    /// Open collector pin
    OpenCollector,
    /// Open emitter pin
    OpenEmitter,
    /// High-Z pin
    HiZ,

    // === Visibility ===
    /// Not hidden
    Visible,
    /// Hidden records
    Hidden,
    /// Currently selected (UI state)
    Selected,

    // === Combinatorial ===
    /// Does not match the given selector
    Not(Box<Selector>),
    /// Contains a descendant matching the given selector
    Has(Box<Selector>),
    /// Matches any of the given selectors (grouping)
    Is(Box<Selector>),
}

impl PseudoSelector {
    /// Parse a pseudo-selector from its name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "root" => Some(Self::Root),
            "empty" => Some(Self::Empty),
            "first-child" | "firstchild" => Some(Self::FirstChild),
            "last-child" | "lastchild" => Some(Self::LastChild),
            "only-child" | "onlychild" => Some(Self::OnlyChild),
            "connected" => Some(Self::Connected),
            "unconnected" => Some(Self::Unconnected),
            "input" => Some(Self::Input),
            "output" => Some(Self::Output),
            "bidirectional" | "bidir" => Some(Self::Bidirectional),
            "power" => Some(Self::Power),
            "passive" => Some(Self::Passive),
            "open-collector" | "opencollector" | "oc" => Some(Self::OpenCollector),
            "open-emitter" | "openemitter" | "oe" => Some(Self::OpenEmitter),
            "hiz" | "hi-z" | "high-z" => Some(Self::HiZ),
            "visible" => Some(Self::Visible),
            "hidden" => Some(Self::Hidden),
            "selected" => Some(Self::Selected),
            _ => None,
        }
    }
}

/// Combinators connecting selector segments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Combinator {
    /// Descendant (any depth): ` ` (space)
    Descendant,
    /// Direct child only: `>` or `/`
    DirectChild,
}
