# 02 - AST

## Location

`crates/altium-format-ops/src/spec/ast.rs`

## Design

The spec AST represents the parsed structure of a `.sym` or
`.sym` file before type checking, scope resolution, or coordinate
computation. Every node carries a `Span` for error reporting.

## Top-Level

```rust
/// A parsed spec file.
pub struct SpecFile {
    pub items: Vec<Spanned<SpecItem>>,
}

/// Top-level items in a spec file.
pub enum SpecItem {
    Import(ImportDecl),
    LetBinding(LetBinding),
    Component(ComponentDecl),     // SchLib domain
    Footprint(FootprintDecl),     // PcbLib domain
}
```

## Import

```rust
pub struct ImportDecl {
    pub path: Spanned<String>,           // "standard-footprints.sym"
    pub alias: Option<Spanned<String>>,  // as footprints
}
```

## Let Bindings

```rust
pub struct LetBinding {
    pub name: Spanned<String>,
    pub value: Spanned<Expr>,
}
```

## Component (SchLib)

```rust
pub struct ComponentDecl {
    pub binding: Option<Spanned<String>>,  // optional: name = component ...
    pub name: Spanned<EntityName>,         // identity key (lib_reference)
    pub body: Vec<Spanned<ComponentItem>>,
}

pub enum ComponentItem {
    Property(Property),                    // designator: "R?", description: "..."
    LetBinding(LetBinding),
    Part(PartBlock),
    Pin(PinDecl),
    Parameter(ParameterDecl),
    Alias(AliasDecl),
    FootprintMap(FootprintMapDecl),
    Graphic(GraphicDecl),
}
```

## Part Block

```rust
pub struct PartBlock {
    pub binding: Option<Spanned<String>>,
    pub number: Spanned<i32>,              // part 1, part 2, ...
    pub body: Vec<Spanned<PartItem>>,
}

pub enum PartItem {
    LetBinding(LetBinding),
    Pin(PinDecl),
    Graphic(GraphicDecl),
}
```

## Pin

```rust
pub struct PinDecl {
    pub binding: Option<Spanned<String>>,  // p2 = pin 2 { ... }
    pub name: Spanned<EntityName>,         // designator (identity key)
    pub body: Spanned<Object>,             // { properties }
}
```

## Parameter, Alias

```rust
pub struct ParameterDecl {
    pub binding: Option<Spanned<String>>,
    pub name: Spanned<EntityName>,
    pub body: Spanned<Object>,
}

pub struct AliasDecl {
    pub name: Spanned<EntityName>,         // no body
}
```

## Footprint Map (in component)

```rust
pub struct FootprintMapDecl {
    pub name: Spanned<FootprintRef>,       // entity_name or $import.Name
    pub maps: Vec<Spanned<MapEntry>>,
}

pub enum FootprintRef {
    Name(EntityName),                      // literal name: "0603"
    DollarPath(DollarPath),                // $fp.DIP8 or $fp["SOT-23"]
}

pub struct MapEntry {
    pub body: Spanned<Object>,             // { pin: 1, pad: 1 }
}
```

## Footprint (PcbLib)

```rust
pub struct FootprintDecl {
    pub binding: Option<Spanned<String>>,
    pub name: Spanned<EntityName>,         // identity key (display_name)
    pub body: Vec<Spanned<FootprintItem>>,
}

pub enum FootprintItem {
    Property(Property),
    LetBinding(LetBinding),
    Pad(PadDecl),
    Row(RowDecl),
    Column(RowDecl),                       // same structure as Row
    Grid(GridDecl),
    Graphic(GraphicDecl),
}
```

## Pad

```rust
pub struct PadDecl {
    pub binding: Option<Spanned<String>>,
    pub name: Spanned<EntityName>,
    pub body: Spanned<Object>,
}
```

## Row / Column / Grid

```rust
pub struct RowDecl {
    pub body: Spanned<Object>,             // all row properties as key-value
}

pub struct GridDecl {
    pub body: Spanned<Object>,
}
```

## Graphics

```rust
pub struct GraphicDecl {
    pub binding: Option<Spanned<String>>,  // body = rectangle { ... }
    pub graphic_type: Spanned<String>,     // "rectangle", "line", "arc", etc.
    pub body: Spanned<Object>,
}
```

The `graphic_type` is a string because graphic keywords are not lexer tokens —
they are identifiers recognized contextually. The compiler validates that the
string is a known graphic type.

## Entity Names

```rust
pub enum EntityName {
    Ident(String),     // R_0603, VCC, A1
    String(String),    // "My Special Part", "EP"
    Integer(i32),      // 1, 2, 3 (pin designators)
}

impl EntityName {
    /// The string representation used as identity key.
    pub fn as_str(&self) -> String {
        match self {
            EntityName::Ident(s) => s.clone(),
            EntityName::String(s) => s.clone(),
            EntityName::Integer(n) => n.to_string(),
        }
    }
}
```

## Expressions (Shared with ops or duplicated)

```rust
pub enum Expr {
    // Literals
    String(String),
    Template(Vec<TemplatePart>),
    Integer(i32),
    Float(f64),
    Dim(f64, Unit),
    Color(u8, u8, u8),
    Bool(bool),
    Null,

    // References
    Ident(String),                         // let-binding or enum
    DollarIdent(String),                   // $body, $p2
    Path(Box<Spanned<Expr>>, Spanned<String>),  // expr.field
    Index(Box<Spanned<Expr>>, Box<Spanned<Expr>>), // expr[key]

    // Operators
    BinOp(Box<Spanned<Expr>>, BinOp, Box<Spanned<Expr>>),
    UnaryNeg(Box<Spanned<Expr>>),

    // Compound
    Tuple(Box<Spanned<Expr>>, Box<Spanned<Expr>>), // (x, y) coord
    Array(Vec<Spanned<Expr>>),
    Object(Object),
}

pub enum BinOp { Add, Sub, Mul, Div }

pub struct Object {
    pub items: Vec<Spanned<ObjectItem>>,
}

pub enum ObjectItem {
    LetBinding(LetBinding),
    Spread(Spanned<Expr>),                 // ...expr
    Property(Property),                    // key: value
}

pub struct Property {
    pub key: Spanned<String>,
    pub value: Spanned<Expr>,
}

pub struct DollarPath {
    pub root: Spanned<String>,             // $fp, $body
    pub steps: Vec<Spanned<PathStep>>,
}

pub enum PathStep {
    Field(String),                         // .field
    Index(Expr),                           // [expr]
}
```

## Span

```rust
#[derive(Debug, Clone, Copy)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}
```

Reuse the existing `Span` / `Spanned` types from `parser/diagnostic.rs` if
they are accessible, or duplicate (they are tiny).

## Notes

- `Object` is used for entity bodies and let-bound values. It contains
  properties and spreads but NOT child entity declarations. Entity containers
  (component, footprint, part) have their own item enums that include both
  properties (via `Object`-like syntax) and child declarations.

- Template strings in the AST are `Vec<TemplatePart>` where `TemplatePart::Expr`
  contains a sub-expression AST. The parser handles this by recursively parsing
  expression tokens from the lexer's template output.

- The `binding` field on entity declarations is `Option<String>` — if present,
  it registers the entity in the enclosing scope as `$name` for anchor and
  reference access.
