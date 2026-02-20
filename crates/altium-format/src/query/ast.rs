//! AST types for the Altium Query Language (AQL).
//!
//! These types represent a parsed query. The parser (Track 5B) converts
//! pest parse trees into this AST, and the evaluator walks the AST to
//! match records.

/// Top-level query node.
#[derive(Debug, Clone)]
pub struct AqlQuery {
    pub expr: AqlExpr,
}

/// Expression: OR/union of terms.
///
/// `Term` is the degenerate case (single term, no OR).
/// `Or` holds two or more terms joined by `OR` or `,`.
#[derive(Debug, Clone)]
pub enum AqlExpr {
    Term(AqlTerm),
    Or(Vec<AqlTerm>),
}

/// Term: AND of factors.
///
/// `Factor` is the degenerate case (single factor, no AND).
/// `And` holds two or more factors joined by `AND`.
#[derive(Debug, Clone)]
pub enum AqlTerm {
    Factor(AqlFactor),
    And(Vec<AqlFactor>),
}

/// Factor: optional NOT wrapping a selector.
#[derive(Debug, Clone)]
pub enum AqlFactor {
    Not(Box<AqlFactor>),
    Selector(AqlSelector),
}

/// A selector, the leaf of the boolean expression tree.
#[derive(Debug, Clone)]
pub enum AqlSelector {
    /// Bare pattern (e.g. `R*`, `~VCC`, `U1:OUT`).
    Pattern(AqlPattern),
    /// Element type without attribute filters (e.g. `component`).
    ElementType(AqlElementType),
    /// Compound: base selector with one or more attribute filters
    /// (e.g. `component[value=10K]`).
    Compound(AqlCompoundSelector),
}

/// Pattern selectors for quick lookups.
#[derive(Debug, Clone)]
pub enum AqlPattern {
    /// Component by designator, possibly with wildcards (e.g. `U1`, `R*`, `C??`).
    Designator(DesignatorPattern),
    /// Net name (e.g. `~VCC`).
    Net(String),
    /// Component value (e.g. `@10K`).
    Value(String),
    /// Part/library reference (e.g. `$LM358`).
    Part(String),
    /// Record ID (e.g. `#42`).
    Id(i32),
    /// Component pin (e.g. `U1:VCC`). First is component, second is pin.
    Pin(String, String),
}

/// Designator pattern with optional wildcard suffix.
#[derive(Debug, Clone)]
pub struct DesignatorPattern {
    /// The alphabetic prefix (e.g. `"U"`, `"R"`, `"C"`).
    pub prefix: String,
    /// What follows the prefix.
    pub suffix: DesignatorSuffix,
}

/// The suffix portion of a designator pattern.
#[derive(Debug, Clone)]
pub enum DesignatorSuffix {
    /// Exact numeric/alphanumeric suffix (e.g. `"1"` in `U1`, `"10"` in C10).
    Exact(String),
    /// Glob wildcard `*` matches any suffix (e.g. `R*`).
    Wildcard,
    /// Single-character wildcard `?` (e.g. `U?` matches U1-U9).
    SingleChar,
    /// Double-character wildcard `??` (e.g. `C??` matches C01-C99).
    DoubleChar,
}

/// Element type selector (case-insensitive keywords).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AqlElementType {
    Component,
    Pin,
    Net,
    Wire,
    Bus,
    Port,
    Power,
    Label,
    NetLabel,
    Junction,
    Sheet,
    Parameter,
    Line,
    Arc,
    Text,
    Polygon,
    Rectangle,
    Pad,
    Via,
    Track,
    Fill,
    Region,
    Rule,
}

/// Compound selector: a base selector plus one or more attribute filters.
#[derive(Debug, Clone)]
pub struct AqlCompoundSelector {
    /// The base selector (pattern or element type).
    pub base: Box<AqlSelector>,
    /// Attribute filters (e.g. `[value=10K]`, `[width>=10mil]`).
    pub filters: Vec<AqlAttrFilter>,
}

/// A single attribute filter: `[field op value]`.
#[derive(Debug, Clone)]
pub struct AqlAttrFilter {
    /// Field name (e.g. `"value"`, `"designator"`, `"x"`).
    pub field: String,
    /// Comparison operator.
    pub op: AqlAttrOp,
    /// The value to compare against.
    pub value: AqlAttrValue,
}

/// Attribute comparison operators.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AqlAttrOp {
    /// `=` exact match
    Eq,
    /// `!=` not equal
    Ne,
    /// `*=` contains substring
    Contains,
    /// `^=` starts with
    StartsWith,
    /// `$=` ends with
    EndsWith,
    /// `~=` word match (whitespace-delimited)
    WordMatch,
    /// `>` greater than
    Gt,
    /// `<` less than
    Lt,
    /// `>=` greater than or equal
    Gte,
    /// `<=` less than or equal
    Lte,
}

/// Attribute filter value.
#[derive(Debug, Clone)]
pub enum AqlAttrValue {
    /// String literal (quoted or bare).
    String(String),
    /// Numeric literal.
    Number(f64),
    /// Boolean literal.
    Bool(bool),
    /// Coordinate with unit (value in the given unit, unit name).
    /// E.g. `10mil` -> `Coord(10.0, "mil")`.
    Coord(f64, String),
}
