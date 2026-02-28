# 01 - Lexer

## Location

`crates/altium-format-ops/src/spec/lexer.rs`

## Approach

Write a new lexer for the spec language. The existing `parser/lexer.rs` provides
a reference implementation but the spec lexer is standalone — no code sharing at
the source level (the two grammars have different keyword sets and the cost of
abstraction exceeds the cost of duplication for ~400 lines of lexer code).

Both lexers produce the same fundamental token kinds (strings, numbers, dims,
colors, identifiers) so the `TokenKind` enum is similar but distinct.

## Token Types

```rust
pub enum TokenKind {
    // Literals
    Ident(String),              // [a-zA-Z_][a-zA-Z0-9_]*
    DollarIdent(String),        // $ident
    String(String),             // "..." with escape sequences
    Template(Vec<TemplatePart>), // `...{expr}...`
    Integer(i32),               // 42, -5
    Float(f64),                 // 3.14
    Dim(f64, Unit),             // 20mm, 100mil
    Color(u8, u8, u8),          // #FF0000

    // Keywords (spec-specific)
    Import,                     // import
    As,                         // as
    Component,                  // component
    Footprint,                  // footprint
    Pin,                        // pin
    Pad,                        // pad
    Part,                       // part
    Parameter,                  // parameter
    Alias,                      // alias
    Map,                        // map
    Row,                        // row
    Column,                     // column
    Grid,                       // grid

    // Shared keywords
    Let,                        // let (noise, optional)
    True,                       // true
    False,                      // false
    Null,                       // null

    // Punctuation
    LBrace,                     // {
    RBrace,                     // }
    LParen,                     // (
    RParen,                     // )
    LBracket,                   // [
    RBracket,                   // ]
    Colon,                      // :
    Comma,                      // ,
    Dot,                        // .
    DotDotDot,                  // ...
    Eq,                         // =
    Plus,                       // +
    Minus,                      // -
    Star,                       // *
    Slash,                      // /
    Semi,                       // ; (noise, ignored)
    Newline,                    // significant newline (separator)

    // End
    Eof,
}
```

## Template Strings

```rust
pub enum TemplatePart {
    Literal(String),     // plain text
    Expr(Vec<Token>),    // {expr} — tokens inside interpolation
}
```

Template strings are delimited by backticks. `{expr}` triggers interpolation.
`{{` and `}}` are literal braces. The lexer produces a `Vec<TemplatePart>`;
the parser handles expression parsing within `Expr` parts.

## Keyword Recognition

After lexing an identifier, check against the keyword table:

```
import, as, component, footprint, pin, pad, part, parameter, alias,
map, row, column, grid, let, true, false, null
```

All other identifiers remain `Ident`. Note that `let` and `;` are noise tokens
per spec-lang.md §15.5 — the parser accepts and ignores them.

Graphic type keywords (`line`, `rectangle`, `arc`, `elliptical_arc`, `ellipse`,
`polyline`, `polygon`, `bezier`, `pie`, `round_rectangle`, `label`,
`text_frame`, `image`, `track`, `fill`, `region`, `text`, `via`,
`component_body`) are NOT lexer keywords. They are recognized contextually by
the parser as identifiers in graphic declaration position. This keeps the keyword
set small and avoids conflicts with field names.

## Dimension Lexing

Number immediately followed by unit suffix (no whitespace) = `Dim` token:
- `20mm` → `Dim(20.0, Mm)`
- `100mil` → `Dim(100.0, Mil)`
- `2.54mm` → `Dim(2.54, Mm)`
- `1in` → `Dim(1.0, Inch)`
- `50dxp` → `Dim(50.0, Dxp)`
- `100raw` → `Dim(100.0, Raw)`

`20 mm` (space between) → `Integer(20)` + `Ident("mm")` (not a dim).

## Color Lexing

`#` followed by exactly 6 hex digits → `Color(r, g, b)`. This is the only use
of `#` in the spec language.

## Newline Handling

Newlines are significant as separators (spec-lang.md §15.2). The lexer emits
`Newline` tokens for line breaks that are NOT inside `()`, `[]`, or `{}`.

Implementation: track bracket/paren/brace nesting depth. Emit `Newline` only
when depth = 0.

Actually, the simpler approach (matching the spec): emit ALL newlines as tokens
and let the parser handle suppression inside brackets. The parser already needs
to track nesting for other reasons.

**Decision**: Emit `Newline` tokens for all line breaks. The parser skips them
inside `()`, `[]`, `{}`. Consecutive newlines collapse to one separator.

## Comments

- Line comments: `//` to end of line. Consumed, not emitted.
- Block comments: `/* ... */`. Nest. Consumed, not emitted.

## Error Reporting

Each token carries a `Span(start, end)` byte offset pair for error reporting.
The diagnostic module (`parser/diagnostic.rs`) can be reused for rendering
caret-style error messages.

## Test Strategy

- Unit test every token kind with isolated inputs
- Test dimension disambiguation (`20mm` vs `20 mm`)
- Test newline handling (inside/outside brackets)
- Test template string interpolation
- Test nested block comments
- Test all escape sequences in strings and templates
- Test error cases (unterminated string, invalid hex color, etc.)
