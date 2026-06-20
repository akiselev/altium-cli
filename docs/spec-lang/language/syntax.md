# Syntax

The lexical structure of the Altium Spec Language: how source text is broken into
comments, whitespace, and tokens before parsing. This page is the authoritative reference
for every literal form, punctuation mark, and operator the lexer recognizes.

All token rules described here are implemented in
[`crates/altium-format-spec/src/lexer.rs`](../../../crates/altium-format-spec/src/lexer.rs).

**Related pages**

- [Types and values](types-and-values.md) — what each literal evaluates to
- [Expressions](expressions.md) — how tokens combine into expressions
- [Blocks overview](blocks-overview.md) — how tokens form declarations
- [Annotations](annotations.md) — the `#[...]` annotation prefix
- [Grammar reference](../reference/grammar.md)
- [Keyword reference](../reference/keywords.md)

## Source encoding and whitespace

Source is UTF-8. The lexer scans byte-by-byte and only treats ASCII bytes specially;
identifiers and string contents may contain multi-byte UTF-8.

Spaces (`0x20`), tabs (`\t`), and carriage returns (`\r`) are insignificant and skipped.
Newlines (`\n`) are **significant**: the lexer emits a dedicated `Newline` token for each
`\n`. The parser uses newlines as soft statement separators inside blocks.

## Comments

Two comment forms, both captured as side-channel trivia (they never appear as tokens):

| Form | Example | Notes |
| ---- | ------- | ----- |
| Line comment | `// like this` | Runs to end of line. |
| Block comment | `/* like this */` | **Nestable** — `/* outer /* inner */ outer */` is one comment. Unterminated block comments are a lex error (`E1001`). |

Comments are preserved separately for the formatter and the comment-aware rewriter; see
[`src/trivia.rs`](../../../crates/altium-format-spec/src/trivia.rs).

## Identifiers

An identifier starts with an ASCII letter or `_` and continues with ASCII alphanumerics or
`_` (`is_ident_start` / `is_ident_continue` in `lexer.rs`).

```
component   body   ESP32_C6   _internal
```

A token that **begins with a digit** but contains letters and is not a recognized unit
suffix is lexed as an identifier, not a number. This lets part numbers like `74LVC1G17`
work as bare names:

```
74LVC1G17     // Ident("74LVC1G17")
20xyz         // Ident("20xyz") — integer + unknown suffix collapses to ident
```

Identifiers that match a reserved word become keyword tokens instead (see
[Keyword reference](../reference/keywords.md)). Graphic-type names (`line`, `rectangle`,
`arc`, …) and SchDoc/PcbDoc object names (`wire`, `via`, …) are **not** keywords — they
lex as plain identifiers and are dispatched by the parser.

## Literals

### Integers and floats

```
42   0   100        // Integer(i32)
3.14   0.5          // Float(f64)
```

A float requires a digit on both sides of the `.` (`3.14`, not `3.`). Integers are parsed
as `i32`; floats as `f64`.

### Dimensions (units)

A number immediately followed (no space) by a known unit suffix is a dimension literal
`Dim(value, unit)`:

```
100mil   2mm   2.54mm   1in   50dxp   100raw
```

| Suffix | `Unit` | Meaning |
| ------ | ------ | ------- |
| `mil` | `Mil` | thousandths of an inch |
| `mm` | `Mm` | millimetres |
| `in` | `Inch` | inches |
| `dxp` | `Dxp` | DXP fractional units |
| `raw` | `Raw` | raw Altium internal coordinate units |

A **space** between the number and suffix breaks the dimension: `20 mm` lexes as
`Integer(20)` then `Ident("mm")`. A **float** with an unknown suffix (`3.14abc`) is a hard
lex error (`E1003`); an integer with an unknown suffix becomes an identifier (see above).

Unit conversion to internal coordinates happens later, in the evaluator — see
[Types and values](types-and-values.md).

**Maps to Altium:** Altium stores all geometry in internal units of 10,000 per mil;
dimension literals are how spec authors write human-readable measurements that the
compiler normalizes to that internal grid.

### Colors

A `#` followed by **exactly six hex digits** is a color literal `Color(r, g, b)`:

```
#FF0000   #00ff00   #aAbBcC      // case-insensitive hex
```

A `#` that is *not* followed by six hex digits is **not** a color — it becomes a standalone
`Hash` token, which begins an [annotation](annotations.md) (`#[...]`). Thus `#FFGG00`,
`#FFF`, and `#[` all start with a `Hash` token.

**Maps to Altium:** the three bytes are red, green, blue. (Altium's on-disk COLORREF is
BGR; the conversion happens in the compiler/executor, not the lexer.)

### Strings

Double-quoted, with backslash escapes:

```
"hello"   "Resistor"   "line 1\nline 2"
```

Valid escapes: `\\`, `\"`, `\n`, `\r`, `\t`. Any other escape, or an unterminated string,
is a lex error.

### Template strings

Backtick-quoted strings with `{ … }` interpolation holes:

```
`prefix {$body.width} suffix`
```

A template lexes to `Template(Vec<TemplatePart>)`, alternating `Literal` text and `Expr`
holes. The text inside `{ … }` is itself lexed (and later parsed and evaluated) as an
expression. Escapes inside a template: `\\`, `` \` ``, `\n`, `\r`, `\t`, `\{`, `\}`.
Doubled braces are literal braces: `` `{{literal}}` `` produces the text `{literal}`.
Interpolation may nest balanced braces. Unterminated templates and unterminated
interpolations are lex errors.

See [Expressions § template strings](expressions.md#template-strings) for evaluation.

### Booleans and null

`true`, `false`, and `null` are keywords (not identifiers) and lex to dedicated tokens.

## The `$` reference sigil

A `$` must be immediately followed by an identifier; it lexes to `DollarIdent(name)`:

```
$body   $fp   $p2
```

A lone `$`, or `$` followed by whitespace or a non-identifier, is a lex error. `$`-prefixed
names are how the language refers to **bindings** and **imports**; see
[Expressions § references](expressions.md#references).

## Punctuation and operators

| Token | Lexeme | Role |
| ----- | ------ | ---- |
| `LBrace` / `RBrace` | `{` `}` | Block and object delimiters |
| `LParen` / `RParen` | `(` `)` | Grouping, tuples, call args |
| `LBracket` / `RBracket` | `[` `]` | Arrays, index access, annotation prefix `#[` |
| `Colon` | `:` | `key: value` property separator |
| `Comma` | `,` | Element/argument separator |
| `Dot` | `.` | Field access (`$a.b`) |
| `DotDotDot` | `...` | Spread operator |
| `Eq` | `=` | `let` / binding assignment, annotation `key = value` |
| `Arrow` | `->` | Pin-connection arrow (`pin X -> #NET`) |
| `Plus` `Minus` `Star` `Slash` | `+` `-` `*` `/` | Arithmetic operators |
| `Semi` | `;` | Statement separator |
| `Hash` | `#` | Annotation prefix (only when not a 6-hex-digit color) |
| `Newline` | `\n` | Significant line break / soft separator |

`->` is recognized only as the two-byte sequence; a `-` not followed by `>` is `Minus`.
`...` is recognized only as the three-byte sequence; a single `.` is `Dot`.

The four arithmetic operators map to the `BinOp` enum (`Add`, `Sub`, `Mul`, `Div`) in
[`src/diagnostic.rs`](../../../crates/altium-format-spec/src/diagnostic.rs); see
[Expressions](expressions.md) for precedence.

## Complete token-kind table

Every variant of `TokenKind` (`src/lexer.rs`). Keyword tokens are listed in the
[Keyword reference](../reference/keywords.md).

| Category | Token kinds |
| -------- | ----------- |
| Literals | `Ident(String)`, `DollarIdent(String)`, `String(String)`, `Template(Vec<TemplatePart>)`, `Integer(i32)`, `Float(f64)`, `Dim(f64, Unit)`, `Color(u8, u8, u8)` |
| Keywords | `Import`, `As`, `Component`, `Footprint`, `Project`, `Sheet`, `Net`, `Power`, `Pin`, `Pad`, `Part`, `Parameter`, `Alias`, `Row`, `Column`, `Grid`, `Board`, `SwapGroup`, `Group`, `Separate`, `Autoplace`, `PadNet`, `Let`, `True`, `False`, `Null` |
| Punctuation | `LBrace`, `RBrace`, `LParen`, `RParen`, `LBracket`, `RBracket`, `Colon`, `Comma`, `Dot`, `DotDotDot`, `Eq`, `Arrow`, `Plus`, `Minus`, `Star`, `Slash`, `Semi`, `Hash`, `Newline` |
| End marker | `Eof` |

Each emitted `Token` carries a `Span { start, end }` of byte offsets into the source,
used for diagnostics and the text-based rewriter.
