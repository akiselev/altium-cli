//! Abstract Syntax Tree types for the query language.

use super::common::{FilterOp, FilterValue, compare_filter};

/// Parsed query AST node
#[derive(Debug, Clone, PartialEq)]
pub enum Selector {
    /// Element type selector: `component`, `pin`, `net`, etc.
    Element(ElementType),

    /// ID selector: `#U1`, `#VCC`
    Id(String),

    /// Universal selector: `*`
    Universal,

    /// Attribute selector: `[attr=value]`
    Attribute(AttributeSelector),

    /// Pseudo-selector: `:connected`, `:power`
    Pseudo(PseudoSelector),

    /// Compound selector (multiple conditions on same element)
    Compound(Vec<Selector>),

    /// Combinator: `A B`, `A > B`, `A >> B`
    Combinator {
        left: Box<Selector>,
        combinator: CombinatorType,
        right: Box<Selector>,
    },

    /// Union: `A, B`
    Union(Vec<Selector>),

    /// Negation: `:not(A)`
    Not(Box<Selector>),

    /// Has child: `:has(A)`
    Has(Box<Selector>),
}

impl Selector {
    /// Check if this selector has any result modifiers (count, limit, etc.)
    pub fn has_result_modifier(&self) -> bool {
        match self {
            Selector::Pseudo(p) => p.is_result_modifier(),
            Selector::Compound(parts) => parts.iter().any(|s| s.has_result_modifier()),
            Selector::Combinator { right, .. } => right.has_result_modifier(),
            Selector::Union(selectors) => selectors.iter().any(|s| s.has_result_modifier()),
            _ => false,
        }
    }

    /// Extract result modifiers from this selector
    pub fn get_result_modifiers(&self) -> Vec<&PseudoSelector> {
        let mut modifiers = Vec::new();
        self.collect_result_modifiers(&mut modifiers);
        modifiers
    }

    fn collect_result_modifiers<'a>(&'a self, modifiers: &mut Vec<&'a PseudoSelector>) {
        match self {
            Selector::Pseudo(p) if p.is_result_modifier() => {
                modifiers.push(p);
            }
            Selector::Compound(parts) => {
                for part in parts {
                    part.collect_result_modifiers(modifiers);
                }
            }
            Selector::Combinator { right, .. } => {
                right.collect_result_modifiers(modifiers);
            }
            Selector::Union(selectors) => {
                for sel in selectors {
                    sel.collect_result_modifiers(modifiers);
                }
            }
            _ => {}
        }
    }
}

/// Element types that can be queried
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ElementType {
    /// Component instance (U1, R1, C1, etc.)
    Component,
    /// Pin on a component
    Pin,
    /// Electrical net (computed from connectivity)
    Net,
    /// Inter-sheet port
    Port,
    /// Wire segment
    Wire,
    /// Power symbol (VCC, +5V, etc.)
    Power,
    /// Ground symbol (subset of power with ground style)
    Ground,
    /// Net label
    Label,
    /// Junction point
    Junction,
    /// Component parameter (value, footprint, etc.)
    Parameter,
    /// Designator record
    Designator,
    /// Sheet/document level
    Sheet,
}

impl ElementType {
    /// Parse element type from string
    pub fn try_parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "component" | "comp" | "c" => Some(ElementType::Component),
            "pin" | "p" => Some(ElementType::Pin),
            "net" | "n" => Some(ElementType::Net),
            "port" => Some(ElementType::Port),
            "wire" | "w" => Some(ElementType::Wire),
            "power" | "pwr" => Some(ElementType::Power),
            "ground" | "gnd" => Some(ElementType::Ground),
            "label" | "netlabel" => Some(ElementType::Label),
            "junction" | "junc" => Some(ElementType::Junction),
            "parameter" | "param" => Some(ElementType::Parameter),
            "designator" | "des" => Some(ElementType::Designator),
            "sheet" | "document" | "doc" => Some(ElementType::Sheet),
            _ => None,
        }
    }

    /// Get canonical string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            ElementType::Component => "component",
            ElementType::Pin => "pin",
            ElementType::Net => "net",
            ElementType::Port => "port",
            ElementType::Wire => "wire",
            ElementType::Power => "power",
            ElementType::Ground => "ground",
            ElementType::Label => "label",
            ElementType::Junction => "junction",
            ElementType::Parameter => "parameter",
            ElementType::Designator => "designator",
            ElementType::Sheet => "sheet",
        }
    }
}

/// Attribute comparison operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeOp {
    /// `[attr]` - Has attribute
    Exists,
    /// `[attr=value]` - Exact match
    Equals,
    /// `[attr!=value]` - Not equal
    NotEquals,
    /// `[attr~=value]` - Word match (space-separated list)
    WordMatch,
    /// `[attr^=value]` - Starts with
    StartsWith,
    /// `[attr$=value]` - Ends with
    EndsWith,
    /// `[attr*=value]` - Contains
    Contains,
    /// `[attr>value]` - Greater than (numeric)
    GreaterThan,
    /// `[attr<value]` - Less than (numeric)
    LessThan,
    /// `[attr>=value]` - Greater or equal
    GreaterOrEqual,
    /// `[attr<=value]` - Less or equal
    LessOrEqual,
}

/// Attribute selector
#[derive(Debug, Clone, PartialEq)]
pub struct AttributeSelector {
    /// Attribute name (normalized to lowercase)
    pub name: String,
    /// Comparison operator
    pub op: AttributeOp,
    /// Value to compare (None for Exists)
    pub value: Option<String>,
    /// Case insensitive flag
    pub case_insensitive: bool,
}

impl AttributeSelector {
    /// Create a new attribute selector
    pub fn new(name: impl Into<String>, op: AttributeOp, value: Option<String>) -> Self {
        Self {
            name: name.into().to_lowercase(),
            op,
            value,
            case_insensitive: false,
        }
    }

    /// Test if an attribute value matches this selector
    pub fn matches(&self, attr_value: Option<&str>) -> bool {
        // Convert to shared types and use common comparison logic
        let filter_op = self.op.to_filter_op();
        let filter_value = self
            .value
            .as_ref()
            .map(|v| FilterValue::String(v.clone()))
            .unwrap_or_else(|| FilterValue::String(String::new()));

        compare_filter(attr_value, filter_op, &filter_value, self.case_insensitive)
    }
}

impl AttributeOp {
    /// Convert to shared FilterOp type.
    pub fn to_filter_op(&self) -> FilterOp {
        match self {
            AttributeOp::Exists => FilterOp::Exists,
            AttributeOp::Equals => FilterOp::Equals,
            AttributeOp::NotEquals => FilterOp::NotEquals,
            AttributeOp::WordMatch => FilterOp::WordMatch,
            AttributeOp::StartsWith => FilterOp::StartsWith,
            AttributeOp::EndsWith => FilterOp::EndsWith,
            AttributeOp::Contains => FilterOp::Contains,
            AttributeOp::GreaterThan => FilterOp::GreaterThan,
            AttributeOp::LessThan => FilterOp::LessThan,
            AttributeOp::GreaterOrEqual => FilterOp::GreaterOrEqual,
            AttributeOp::LessOrEqual => FilterOp::LessOrEqual,
        }
    }

    /// Create from shared FilterOp type.
    pub fn from_filter_op(op: FilterOp) -> Self {
        match op {
            FilterOp::Exists => AttributeOp::Exists,
            FilterOp::Equals => AttributeOp::Equals,
            FilterOp::NotEquals => AttributeOp::NotEquals,
            FilterOp::WordMatch => AttributeOp::WordMatch,
            FilterOp::StartsWith => AttributeOp::StartsWith,
            FilterOp::EndsWith => AttributeOp::EndsWith,
            FilterOp::Contains => AttributeOp::Contains,
            FilterOp::GreaterThan => AttributeOp::GreaterThan,
            FilterOp::LessThan => AttributeOp::LessThan,
            FilterOp::GreaterOrEqual => AttributeOp::GreaterOrEqual,
            FilterOp::LessOrEqual => AttributeOp::LessOrEqual,
        }
    }
}

/// Standard attribute names with aliases
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandardAttribute {
    // Component attributes
    Designator,
    Part,
    Value,
    Footprint,
    Description,

    // Pin attributes
    Name,
    Number,
    Type,
    Hidden,

    // Net attributes
    NetName,

    // Port attributes
    IoType,

    // Power attributes
    Style,

    // Location attributes
    X,
    Y,
}

impl StandardAttribute {
    /// Parse attribute name from string (with aliases)
    pub fn try_parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "designator" | "des" | "ref" | "refdes" => Some(Self::Designator),
            "part" | "partname" | "libref" | "libreference" => Some(Self::Part),
            "value" | "val" => Some(Self::Value),
            "footprint" | "fp" | "package" | "pcbfootprint" => Some(Self::Footprint),
            "description" | "desc" => Some(Self::Description),
            "name" | "pinname" => Some(Self::Name),
            "number" | "num" | "pin" | "pinnum" => Some(Self::Number),
            "type" | "electrical" | "elec" => Some(Self::Type),
            "hidden" => Some(Self::Hidden),
            "netname" | "net" => Some(Self::NetName),
            "io" | "iotype" | "direction" | "dir" => Some(Self::IoType),
            "style" => Some(Self::Style),
            "x" | "locx" | "locationx" => Some(Self::X),
            "y" | "locy" | "locationy" => Some(Self::Y),
            _ => None,
        }
    }

    /// Get canonical attribute name
    pub fn canonical_name(&self) -> &'static str {
        match self {
            Self::Designator => "designator",
            Self::Part => "part",
            Self::Value => "value",
            Self::Footprint => "footprint",
            Self::Description => "description",
            Self::Name => "name",
            Self::Number => "number",
            Self::Type => "type",
            Self::Hidden => "hidden",
            Self::NetName => "net",
            Self::IoType => "io",
            Self::Style => "style",
            Self::X => "x",
            Self::Y => "y",
        }
    }
}

/// Pseudo-selector types
#[derive(Debug, Clone, PartialEq)]
pub enum PseudoSelector {
    // Connectivity
    Connected,
    Unconnected,

    // Electrical types
    Power,
    Ground,
    Input,
    Output,
    Bidirectional,
    Passive,
    OpenCollector,
    OpenEmitter,
    HiZ,

    // Visibility
    Hidden,
    Visible,

    // Position selectors
    First,
    Last,
    Nth(usize),
    NthLast(usize),
    Even,
    Odd,

    // Result modifiers
    Count,
    Limit(usize),
    Offset(usize),
}

impl PseudoSelector {
    /// Parse pseudo-selector from string
    pub fn try_parse(s: &str, arg: Option<&str>) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "connected" | "conn" => Some(Self::Connected),
            "unconnected" | "unconn" | "floating" | "nc" => Some(Self::Unconnected),
            "power" | "pwr" => Some(Self::Power),
            "ground" | "gnd" => Some(Self::Ground),
            "input" | "in" => Some(Self::Input),
            "output" | "out" => Some(Self::Output),
            "bidirectional" | "bidir" | "inout" => Some(Self::Bidirectional),
            "passive" | "pass" => Some(Self::Passive),
            "opencollector" | "oc" => Some(Self::OpenCollector),
            "openemitter" | "oe" => Some(Self::OpenEmitter),
            "hiz" | "tristate" => Some(Self::HiZ),
            "hidden" => Some(Self::Hidden),
            "visible" => Some(Self::Visible),
            "first" => Some(Self::First),
            "last" => Some(Self::Last),
            "even" => Some(Self::Even),
            "odd" => Some(Self::Odd),
            "count" => Some(Self::Count),
            "nth" => arg.and_then(|a| a.parse().ok()).map(Self::Nth),
            "nth-last" | "nthlast" => arg.and_then(|a| a.parse().ok()).map(Self::NthLast),
            "limit" => arg.and_then(|a| a.parse().ok()).map(Self::Limit),
            "offset" | "skip" => arg.and_then(|a| a.parse().ok()).map(Self::Offset),
            _ => None,
        }
    }

    /// Check if this is a result modifier (affects output, not filtering)
    pub fn is_result_modifier(&self) -> bool {
        matches!(
            self,
            PseudoSelector::Count
                | PseudoSelector::Limit(_)
                | PseudoSelector::Offset(_)
                | PseudoSelector::First
                | PseudoSelector::Last
                | PseudoSelector::Nth(_)
                | PseudoSelector::NthLast(_)
                | PseudoSelector::Even
                | PseudoSelector::Odd
        )
    }

    /// Check if this is a filter (affects element selection)
    pub fn is_filter(&self) -> bool {
        !self.is_result_modifier()
    }
}

/// Combinator types for relationship queries
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombinatorType {
    /// `A B` - B is descendant of A (owned by, directly or indirectly)
    Descendant,
    /// `A > B` - B is direct child of A
    Child,
    /// `A >> B` - B is electrically connected to A (through nets)
    Connected,
    /// `A ~ B` - B is sibling of A (same owner)
    Sibling,
    /// `A + B` - B immediately follows A in document order
    Adjacent,
    /// `A :: B` - B is on net A (for net queries)
    OnNet,
}

impl CombinatorType {
    /// Get the syntax representation
    pub fn syntax(&self) -> &'static str {
        match self {
            CombinatorType::Descendant => " ",
            CombinatorType::Child => " > ",
            CombinatorType::Connected => " >> ",
            CombinatorType::Sibling => " ~ ",
            CombinatorType::Adjacent => " + ",
            CombinatorType::OnNet => " :: ",
        }
    }
}
