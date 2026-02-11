# Phase 5: Query Language (AQL)

**Agents: 3 parallel tracks (5A, 5B, 5C)**
**Blocked by: Phase 0 only** (can run in parallel with Phases 1-3)

The query language is largely independent of the record/document types. It needs to integrate with them (Phase 4), but the grammar, AST, and evaluator can be built against a generic record interface.

**Initial v2 scope:** Pattern selectors + attribute selectors only. Combinators (`>`, `+`, `~`) and pseudo-classes (`:power`, `:input`) are deferred.

---

## Track 5A: Pest Grammar & AST

**Files:**
- `crates/altium-format/src/v2/query/grammar.pest`
- `crates/altium-format/src/v2/query/ast.rs`

**Reference: `docs/query-lang.md` (EBNF grammar)**

### Pest Grammar (grammar.pest)

Translate the EBNF from `docs/query-lang.md` to PEG syntax:

```pest
// Top-level
query = { SOI ~ expr ~ EOI }

// Expression with OR/union
expr = { term ~ (or_op ~ term)* }
or_op = { "OR" | "," }

// Term with AND
term = { factor ~ ("AND" ~ factor)* }

// Factor with NOT
factor = { "NOT" ~ factor | selector }

// Selector types
selector = { compound_selector | pattern }

// Compound: element type or pattern with optional attribute selectors
compound_selector = { (element_type | pattern) ~ attr_sel* }

// Pattern selectors (initial v2 scope)
pattern = {
    pin_pat          // U1:VCC (must be before designator_pat)
    | net_pat        // ~VCC
    | value_pat      // @10K
    | part_pat       // $LM358
    | id_pat         // #42
    | designator_pat // U1, R*, C??
}

designator_pat = @{ ASCII_ALPHA+ ~ (ASCII_ALPHANUMERIC | "_")* ~ ("*" | "??" | "?")? }
net_pat = { "~" ~ ident }
value_pat = { "@" ~ value_literal }
part_pat = { "$" ~ ident }
id_pat = { "#" ~ integer }
pin_pat = { ident ~ ":" ~ ident }

// Element type selectors
element_type = {
    ^"component" | ^"pin" | ^"net" | ^"wire" | ^"bus" | ^"port"
    | ^"power" | ^"label" | ^"netlabel" | ^"junction" | ^"sheet"
    | ^"parameter" | ^"line" | ^"arc" | ^"text" | ^"polygon"
    | ^"rectangle" | ^"pad" | ^"via" | ^"track" | ^"fill" | ^"region"
    | ^"rule"
}

// Attribute selectors
attr_sel = { "[" ~ field_name ~ attr_op ~ attr_value ~ "]" }
field_name = @{ ASCII_ALPHA ~ (ASCII_ALPHANUMERIC | "_" | ".")* }
attr_op = { ">=" | "<=" | "!=" | "*=" | "^=" | "$=" | "~=" | ">" | "<" | "=" }
attr_value = { coord_value | quoted_string | number | boolean | bare_string }

// Value types
coord_value = @{ number ~ ("mil" | "mm" | "in") }
quoted_string = @{ "\"" ~ (!"\"" ~ ANY)* ~ "\"" }
number = @{ "-"? ~ ASCII_DIGIT+ ~ ("." ~ ASCII_DIGIT+)? }
integer = @{ "-"? ~ ASCII_DIGIT+ }
boolean = { ^"true" | ^"false" }
bare_string = @{ (ASCII_ALPHANUMERIC | "_" | "-" | ".")+ }

// Shared
ident = @{ ASCII_ALPHA ~ (ASCII_ALPHANUMERIC | "_")* }
value_literal = @{ (ASCII_ALPHANUMERIC | "_" | "-" | ".")+ }

WHITESPACE = _{ " " | "\t" | "\n" | "\r" }
```

### AST Types (ast.rs)

```rust
/// Top-level query
#[derive(Debug, Clone)]
pub struct AqlQuery {
    pub expr: AqlExpr,
}

/// Expression (OR/union of terms)
#[derive(Debug, Clone)]
pub enum AqlExpr {
    Term(AqlTerm),
    Or(Vec<AqlTerm>),
}

/// Term (AND of factors)
#[derive(Debug, Clone)]
pub enum AqlTerm {
    Factor(AqlFactor),
    And(Vec<AqlFactor>),
}

/// Factor (NOT or selector)
#[derive(Debug, Clone)]
pub enum AqlFactor {
    Not(Box<AqlFactor>),
    Selector(AqlSelector),
}

/// Selector
#[derive(Debug, Clone)]
pub enum AqlSelector {
    Pattern(AqlPattern),
    ElementType(AqlElementType),
    Compound(AqlCompoundSelector),
}

/// Pattern selectors
#[derive(Debug, Clone)]
pub enum AqlPattern {
    Designator(DesignatorPattern),
    Net(String),
    Value(String),
    Part(String),
    Id(i32),
    Pin(String, String), // component, pin
}

/// Designator pattern with optional wildcards
#[derive(Debug, Clone)]
pub struct DesignatorPattern {
    pub prefix: String,
    pub suffix: DesignatorSuffix,
}

#[derive(Debug, Clone)]
pub enum DesignatorSuffix {
    Exact(String),       // "U1" → prefix="U", suffix=Exact("1")
    Wildcard,            // "R*"
    SingleChar,          // "U?"
    DoubleChar,          // "C??"
}

/// Element type
#[derive(Debug, Clone, PartialEq)]
pub enum AqlElementType {
    Component, Pin, Net, Wire, Bus, Port, Power, Label, NetLabel,
    Junction, Sheet, Parameter, Line, Arc, Text, Polygon, Rectangle,
    Pad, Via, Track, Fill, Region, Rule,
}

/// Compound selector (element/pattern + attribute filters)
#[derive(Debug, Clone)]
pub struct AqlCompoundSelector {
    pub base: Box<AqlSelector>,
    pub filters: Vec<AqlAttrFilter>,
}

/// Attribute filter [field op value]
#[derive(Debug, Clone)]
pub struct AqlAttrFilter {
    pub field: String,
    pub op: AqlAttrOp,
    pub value: AqlAttrValue,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AqlAttrOp {
    Eq, Ne, Contains, StartsWith, EndsWith, WordMatch,
    Gt, Lt, Gte, Lte,
}

#[derive(Debug, Clone)]
pub enum AqlAttrValue {
    String(String),
    Number(f64),
    Bool(bool),
    Coord(f64, String), // value + unit
}
```

### Acceptance Criteria

- [ ] Pest grammar compiles
- [ ] AST types cover pattern selectors + attribute selectors
- [ ] Grammar handles all examples from `docs/query-lang.md` (initial scope)
- [ ] `cargo check` passes

---

## Track 5B: Parser & Evaluator

**Files:**
- `crates/altium-format/src/v2/query/mod.rs` (parse function)
- `crates/altium-format/src/v2/query/eval.rs` (evaluator)

### Parser (mod.rs)

```rust
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "v2/query/grammar.pest"]
struct AqlParser;

/// Parse a query string into an AST.
pub fn parse(query: &str) -> crate::Result<AqlQuery> {
    let pairs = AqlParser::parse(Rule::query, query)
        .map_err(|e| crate::AltiumError::Query(e.to_string()))?;
    // Walk pest pairs → build AqlQuery AST
    build_query(pairs)
}

fn build_query(pairs: pest::iterators::Pairs<Rule>) -> crate::Result<AqlQuery> { ... }
fn build_expr(pair: pest::iterators::Pair<Rule>) -> crate::Result<AqlExpr> { ... }
fn build_term(pair: pest::iterators::Pair<Rule>) -> crate::Result<AqlTerm> { ... }
fn build_factor(pair: pest::iterators::Pair<Rule>) -> crate::Result<AqlFactor> { ... }
fn build_selector(pair: pest::iterators::Pair<Rule>) -> crate::Result<AqlSelector> { ... }
fn build_pattern(pair: pest::iterators::Pair<Rule>) -> crate::Result<AqlPattern> { ... }
fn build_attr_sel(pair: pest::iterators::Pair<Rule>) -> crate::Result<AqlAttrFilter> { ... }
```

### Evaluator (eval.rs)

The evaluator takes a parsed query and a record collection, returns matching indices:

```rust
/// Trait that records must implement to be queryable.
pub trait Queryable {
    fn record_id(&self) -> u8;
    fn get_field(&self, field: &str) -> Option<QueryFieldValue>;
}

pub enum QueryFieldValue {
    String(String),
    Int(i32),
    Float(f64),
    Bool(bool),
    Coord(f64), // in mils
}

/// Evaluate a query against a collection of records.
/// Returns indices of matching records.
pub fn evaluate<Q: Queryable>(query: &AqlQuery, records: &[Q]) -> Vec<usize> {
    records.iter()
        .enumerate()
        .filter(|(_, r)| matches_expr(&query.expr, *r))
        .map(|(i, _)| i)
        .collect()
}

fn matches_expr<Q: Queryable>(expr: &AqlExpr, record: &Q) -> bool { ... }
fn matches_term<Q: Queryable>(term: &AqlTerm, record: &Q) -> bool { ... }
fn matches_factor<Q: Queryable>(factor: &AqlFactor, record: &Q) -> bool { ... }
fn matches_selector<Q: Queryable>(selector: &AqlSelector, record: &Q) -> bool { ... }
fn matches_pattern<Q: Queryable>(pattern: &AqlPattern, record: &Q) -> bool { ... }
fn matches_attr_filter<Q: Queryable>(filter: &AqlAttrFilter, record: &Q) -> bool { ... }
```

### Tests

- `parse_simple_designator()` — "U1" → DesignatorPattern
- `parse_wildcard()` — "R*" → Wildcard
- `parse_net()` — "~VCC" → Net
- `parse_attr_filter()` — "[value=10K]"
- `parse_compound()` — "component[value=10K]"
- `parse_and_or()` — "R* OR C*"
- `eval_designator_exact()` — matches U1 against records
- `eval_wildcard()` — matches R* against R1, R2, R3
- `eval_attr_eq()` — matches [value=10K]
- `eval_attr_gt()` — matches [width>=10mil]
- `eval_not()` — NOT :virtual
- `eval_and()` — component AND [value=10K]

### Acceptance Criteria

- [ ] `parse()` correctly parses all pattern selector + attribute selector queries
- [ ] `evaluate()` returns correct matching indices
- [ ] Error messages point to the character that failed
- [ ] All test cases from `docs/query-lang.md` (initial scope) pass
- [ ] `cargo check` passes

---

## Track 5C: Queryable Integration

**File: `crates/altium-format/src/v2/query/integration.rs`**

**Blocked by: Phase 3 (record types) AND Track 5B (evaluator)**

### What to Build

Implement `Queryable` for `RecordNode` by inspecting the backing store:

```rust
impl Queryable for RecordNode {
    fn record_id(&self) -> u8 { self.key }

    fn get_field(&self, field: &str) -> Option<QueryFieldValue> {
        match &self.origin {
            RecordOrigin::Param(p) => {
                // Map field names to param keys
                let key = field_to_param_key(field)?;
                let value = p.params.get(key)?;
                Some(param_to_query_value(value))
            }
            RecordOrigin::Binary(b) => {
                // Map field names to known offsets
                binary_field_lookup(field, b)
            }
        }
    }
}

/// Map user-facing field names to param keys.
/// e.g., "designator" → "DESIGNATOR", "value" → "VALUE"
fn field_to_param_key(field: &str) -> Option<&'static str> {
    match field.to_lowercase().as_str() {
        "designator" => Some("DESIGNATOR"),
        "name" => Some("NAME"),
        "description" | "desc" => Some("DESCRIPTION"),
        "value" => Some("VALUE"),
        "footprint" => Some("FOOTPRINT"),
        "lib_reference" | "libreference" | "libref" => Some("LIBREFERENCE"),
        // ... comprehensive mapping
        _ => None,
    }
}

/// Map record_id to element type for type selectors
fn record_id_to_element_type(id: u8) -> Option<AqlElementType> {
    match id {
        1 => Some(AqlElementType::Component),
        2 => Some(AqlElementType::Pin),
        // ... etc
        _ => None,
    }
}
```

### Acceptance Criteria

- [ ] `RecordNode` implements `Queryable`
- [ ] Field name mapping covers common fields
- [ ] Element type mapping covers all record types
- [ ] Integration tests pass with real records
- [ ] `cargo check` passes
