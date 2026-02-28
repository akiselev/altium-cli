# 03 - Parser

## Location

`crates/altium-format-ops/src/spec/parser.rs`

## Approach

Hand-written recursive-descent parser, matching the style of the existing ops
parser in `parser/mod.rs`. No parser generator (pest, lalrpop, nom) — the
grammar is simple enough and hand-written parsers give better error messages.

## Public API

```rust
/// Parse a spec file source string into an AST.
pub fn parse_spec(source: &str) -> Result<SpecFile, ParseError> {
    let tokens = lex(source)?;
    let mut parser = SpecParser::new(source, &tokens);
    parser.parse_file()
}
```

## Parser State

```rust
struct SpecParser<'a> {
    source: &'a str,
    tokens: &'a [Token],
    pos: usize,
    /// Stack of bracket depths for newline suppression
    bracket_depth: usize,
}
```

## Parsing Strategy

### File Level

```
spec_file = { spec_item [";"] }
```

The parser loops, consuming items until EOF. Items are separated by newlines
or semicolons. The parser skips noise tokens (`let` before bindings, `;`
after statements).

```rust
fn parse_file(&mut self) -> Result<SpecFile> {
    let mut items = Vec::new();
    self.skip_newlines();
    while !self.at_eof() {
        items.push(self.parse_spec_item()?);
        self.skip_separators();
    }
    Ok(SpecFile { items })
}
```

### Item Dispatch

At the top level, dispatch on the current token:

| Token | Parse as |
|-------|----------|
| `import` | `ImportDecl` |
| `component` | `ComponentDecl` |
| `footprint` | `FootprintDecl` |
| `let` + IDENT + `=` | `LetBinding` |
| IDENT + `=` | `LetBinding` (implicit `let`) |
| IDENT + `=` + `component` | `ComponentDecl` with binding |
| IDENT + `=` + `footprint` | `FootprintDecl` with binding |

The lookahead for binding detection: if we see `IDENT =` and the next token
after `=` is a keyword (`component`, `footprint`), it's an entity with a binding
prefix. If the next token is something else, it's a let binding.

```rust
fn parse_spec_item(&mut self) -> Result<Spanned<SpecItem>> {
    // Skip optional `let`
    let has_let = self.eat_keyword(Let);

    match self.peek_kind() {
        Import => self.parse_import(),
        Component => self.parse_component(None),
        Footprint => self.parse_footprint(None),
        Ident(_) if self.peek_ahead_is(1, Eq) => {
            let name = self.eat_ident()?;
            self.expect(Eq)?;
            // Check if this is a bound entity or a let binding
            match self.peek_kind() {
                Component => self.parse_component(Some(name)),
                Footprint => self.parse_footprint(Some(name)),
                _ => self.parse_let_binding_value(name),
            }
        }
        _ => self.error("expected import, component, footprint, or let binding"),
    }
}
```

### Import

```
import_decl = "import" STRING [ "as" IDENT ]
```

Simple: consume `import`, expect string, optionally consume `as` + ident.

### Component Body

```
component_decl = [binding "="] "component" entity_name "{" { component_item } "}"
```

Inside the braces, dispatch on token:

| Token | Parse as |
|-------|----------|
| `part` | `PartBlock` |
| `pin` | `PinDecl` |
| `parameter` | `ParameterDecl` |
| `alias` | `AliasDecl` |
| `footprint` | `FootprintMapDecl` |
| IDENT + `:` | `Property` (e.g., `designator: "R?"`) |
| IDENT + `=` | Binding — check next token for entity keyword |
| `let` | LetBinding |
| graphic identifier | `GraphicDecl` |

Graphic identifiers (`line`, `rectangle`, `arc`, etc.) are parsed as regular
`Ident` tokens. The parser checks against the known graphic type set.

### Binding Prefix Detection (Uniform Pattern)

Every entity declaration (pin, pad, parameter, graphic) supports the optional
`binding_prefix = [let] IDENT "="` pattern. The parser has a helper:

```rust
/// Try to parse an optional binding prefix: [let] IDENT "=".
/// Returns the binding name if present, and rewinds if not.
fn try_parse_binding(&mut self) -> Option<Spanned<String>> {
    let save = self.pos;
    self.eat_keyword(Let); // skip optional let
    if let Some(ident) = self.try_eat_ident() {
        if self.eat(Eq) {
            return Some(ident);
        }
    }
    self.pos = save;
    None
}
```

### Expression Parsing (Pratt Parser)

Expressions use a Pratt parser with binding powers from spec-lang.md §7.2:

| Precedence | Operators |
|------------|-----------|
| 90 | `.` `[expr]` |
| 70 | unary `-` |
| 60 | `*` `/` |
| 50 | `+` `-` |

```rust
fn parse_expr(&mut self) -> Result<Spanned<Expr>> {
    self.parse_pratt_expr(0)
}

fn parse_pratt_expr(&mut self, min_bp: u8) -> Result<Spanned<Expr>> {
    let mut lhs = self.parse_prefix_expr()?;

    loop {
        let (op, bp) = match self.peek_kind() {
            Dot => (InfixOp::Access, 90),
            LBracket => (InfixOp::Index, 90),
            Star => (InfixOp::Mul, 60),
            Slash => (InfixOp::Div, 60),
            Plus => (InfixOp::Add, 50),
            Minus => (InfixOp::Sub, 50),
            _ => break,
        };
        if bp < min_bp { break; }
        self.advance();
        lhs = self.parse_infix(lhs, op, bp)?;
    }

    Ok(lhs)
}
```

### Object Parsing

Objects (`{ ... }`) contain spreads, properties, and let bindings:

```rust
fn parse_object(&mut self) -> Result<Spanned<Object>> {
    self.expect(LBrace)?;
    let mut items = Vec::new();
    while !self.at(RBrace) && !self.at_eof() {
        self.skip_newlines();
        if self.at(RBrace) { break; }
        items.push(self.parse_object_item()?);
        self.eat_separator(); // comma or newline
    }
    self.expect(RBrace)?;
    Ok(Object { items })
}
```

### Separator Handling

The parser tracks bracket/paren/brace nesting. Newlines inside any bracket
pair are consumed as whitespace. Outside brackets, newlines act as separators
(equivalent to commas).

```rust
fn eat_separator(&mut self) -> bool {
    self.eat(Comma) || self.eat(Newline) || self.eat(Semi)
}

fn skip_newlines(&mut self) {
    while self.eat(Newline) || self.eat(Semi) {}
}
```

### Entity Name Parsing

```rust
fn parse_entity_name(&mut self) -> Result<Spanned<EntityName>> {
    match self.peek_kind() {
        String(s) => { self.advance(); Ok(EntityName::String(s)) }
        Integer(n) => { self.advance(); Ok(EntityName::Integer(n)) }
        Ident(s) => { self.advance(); Ok(EntityName::Ident(s)) }
        _ => self.error("expected entity name (identifier, string, or integer)"),
    }
}
```

## Error Recovery

For the initial implementation, errors are fatal (no recovery). The parser
reports the first error with full source span and stops. This is acceptable
for a DSL primarily consumed by LLMs and CI — fast iteration on errors is
more valuable than parsing past errors.

Future: add recovery points at `}` boundaries to report multiple errors.

## Test Strategy

- Parse every example from spec-lang.md §17 (Examples 1-5)
- Roundtrip: parse -> pretty-print -> parse -> assert AST equality
- Error messages: test that common mistakes produce helpful diagnostics
- Edge cases: empty bodies, trailing commas, noise tokens, forward refs
- Binding prefix: test all combinations (with/without `let`, with/without binding)
