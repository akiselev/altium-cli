# Keyword and token reference

Every reserved keyword and punctuation/operator token recognized by the Altium
Spec Language lexer, the `TokenKind` it produces, and the blocks or contexts in
which it is meaningful. This table is authoritative for what the lexer treats as
a keyword versus a plain identifier.

All keyword classification is implemented in the `lex()` keyword match in
[`crates/altium-format-spec/src/lexer.rs`](../../../crates/altium-format-spec/src/lexer.rs)
(the `"import" => TokenKind::Import` block), mirrored by the `is_keyword()`
predicate in the same file. The productions that consume each keyword live in
[`src/parser.rs`](../../../crates/altium-format-spec/src/parser.rs).

**Related pages**

- [Grammar reference](grammar.md) — the full grammar that consumes these tokens
- [Altium mapping](altium-mapping.md) — what each construct produces
- [Syntax](../language/syntax.md) — lexical structure overview
- [Blocks overview](../language/blocks-overview.md) — block-level semantics

## Lexing rules for keywords

A keyword is only produced when a run of identifier characters
(`[A-Za-z_][A-Za-z0-9_]*`) **exactly** matches one of the reserved words below.
Any other identifier run becomes `TokenKind::Ident`. Consequences:

- Keyword matching is **case-sensitive**: `Component` lexes as `Ident("Component")`,
  not the `component` keyword.
- A keyword can still be used as a **property key** inside an object or property
  position — `try_eat_property_key()` in `parser.rs` accepts every keyword token
  and converts it back to its string spelling (so `pin: 1`, `group: "x"`,
  `net: ...` are valid property keys).
- Several keywords double as **expression identifier values** via
  `parse_prefix_expr()`: `power`, `net`, `sheet`, `autoplace`, `group`, and
  `separate` evaluate to `Expr::Ident(<spelling>)` when they appear in value
  position (e.g. `electrical: power`, `unplaced: autoplace`).

Many block names used in the grammar (`document`, `annotation`, `variant`,
`output`, `rule`, `constraint`, `place`, `left_of`, `wire`, `track`, `polygon`,
…) are **not** reserved keywords — they lex as `Ident` and are dispatched by
string comparison inside the parser. Those are listed in
[Contextual identifiers](#contextual-identifiers-not-keywords) below, not here.

## Reserved keywords

| Keyword | `TokenKind` | Introduces / used in | See |
| --- | --- | --- | --- |
| `import` | `Import` | Top-level `import "path" [as alias]` directive (`parse_import`). | [operations: cli](../operations/cli.md), [schlib](../blocks/schlib.md) |
| `as` | `As` | Alias clause of an `import` directive. | [schlib](../blocks/schlib.md) |
| `component` | `Component` | `component NAME { … }` declaration (`parse_component`); SchLib symbol / SchDoc placement. | [schlib](../blocks/schlib.md), [schdoc](../blocks/schdoc.md) |
| `footprint` | `Footprint` | Top-level `footprint NAME { … }` (PcbLib, `parse_footprint`) **and** `footprint REF { … }` map inside a component (`parse_footprint_map`). | [pcblib](../blocks/pcblib.md), [schlib](../blocks/schlib.md) |
| `project` | `Project` | `project NAME { … }` declaration (`parse_project`); PrjPcb. | [prjpcb](../blocks/prjpcb.md) |
| `sheet` | `Sheet` | `sheet { … }` SchDoc metadata block (`parse_sheet`); also an `Expr::Ident` value. | [schdoc](../blocks/schdoc.md) |
| `net` | `Net` | `net NAME { … }` SchDoc net (`parse_net`); also an `Expr::Ident` value. | [schdoc](../blocks/schdoc.md) |
| `power` | `Power` | `power NAME { … }` SchDoc power net (`parse_power`); also an `Expr::Ident` value. | [schdoc](../blocks/schdoc.md) |
| `pin` | `Pin` | `pin NAME { … }` declaration, `pin X -> …` connection, and the `pin` side of a footprint map. | [schlib](../blocks/schlib.md), [schdoc](../blocks/schdoc.md) |
| `pad` | `Pad` | `pad NAME { … }` in a footprint, top-level PcbDoc pad primitive, and the `pad` side of a footprint map. | [pcblib](../blocks/pcblib.md), [pcbdoc](../blocks/pcbdoc.md) |
| `part` | `Part` | `part N { … }` multi-part component block (`parse_part_block`). | [schlib](../blocks/schlib.md) |
| `parameter` | `Parameter` | `parameter NAME { … }` inside a component / SchDoc object, and top-level SchDoc `parameter` object. | [schlib](../blocks/schlib.md), [schdoc](../blocks/schdoc.md) |
| `alias` | `Alias` | `alias NAME` (bodyless) inside a component (`parse_alias`). | [schlib](../blocks/schlib.md) |
| `row` | `Row` | `row { … }` pad-generation block inside a footprint. | [pcblib](../blocks/pcblib.md) |
| `column` | `Column` | `column { … }` pad-generation block inside a footprint. | [pcblib](../blocks/pcblib.md) |
| `grid` | `Grid` | `grid { … }` pad-generation block inside a footprint. | [pcblib](../blocks/pcblib.md) |
| `board` | `Board` | `board NAME { … }` PcbDoc board settings block (`parse_board`). | [pcbdoc](../blocks/pcbdoc.md) |
| `swap_group` | `SwapGroup` | `swap_group NAME { … }` declaration (component/top level) and `swap_group:` property. | [schlib](../blocks/schlib.md) |
| `group` | `Group` | `group NAME { … }` inside a `placement` block; also an annotation key and `Expr::Ident` value. | [placement](../blocks/placement.md) |
| `separate` | `Separate` | `separate $a, $b { … }` inside a `placement` block; also an `Expr::Ident` value. | [placement](../blocks/placement.md) |
| `autoplace` | `Autoplace` | `autoplace { … }` inside a `placement` block; also an `Expr::Ident` value. | [placement](../blocks/placement.md) |
| `pad_net` | `PadNet` | `pad_net PAD: "NET"` PcbDoc pad-to-net assignment inside a component. | [pcbdoc](../blocks/pcbdoc.md) |
| `let` | `Let` | `let NAME = expr` binding (file, component, part, footprint, object, project, sheet, placement scope). | [expressions](../language/expressions.md) |
| `true` | `True` | Boolean literal `Expr::Bool(true)`; also the `stable = true` annotation value. | [types and values](../language/types-and-values.md) |
| `false` | `False` | Boolean literal `Expr::Bool(false)`. | [types and values](../language/types-and-values.md) |
| `null` | `Null` | Null literal `Expr::Null`. | [types and values](../language/types-and-values.md) |

There are **24** reserved keywords. The list matches both the keyword match arm
in `lex()` and the `is_keyword()` predicate exactly.

## Punctuation and operator tokens

These tokens are produced directly by the byte-level scanner in `lex()`.

| Spelling | `TokenKind` | Role |
| --- | --- | --- |
| `{` | `LBrace` | Open a block / object body. |
| `}` | `RBrace` | Close a block / object body. |
| `(` | `LParen` | Open a tuple, grouping, function call, or ERC matrix key. |
| `)` | `RParen` | Close the above. |
| `[` | `LBracket` | Open an array, index, or the `#[…]` annotation. |
| `]` | `RBracket` | Close the above. |
| `:` | `Colon` | Separate a property key from its value. |
| `,` | `Comma` | Separate items / tuple elements / array elements. |
| `.` | `Dot` | Field access on a path / dollar reference. |
| `...` | `DotDotDot` | Spread operator inside an object (`...expr`). Scanned with priority over `.`. |
| `=` | `Eq` | Binding assignment and annotation key assignment. |
| `->` | `Arrow` | Pin-connection arrow (`pin X -> #NET`). Scanned with priority over `-`. |
| `+` | `Plus` | Binary addition (binding power 50/51). |
| `-` | `Minus` | Binary subtraction (50/51) or unary negation (prefix bp 70). |
| `*` | `Star` | Binary multiplication (60/61). |
| `/` | `Slash` | Binary division (60/61). Note: `//` and `/*` start comments instead. |
| `;` | `Semi` | Statement / item separator (equivalent to newline). |
| `#` | `Hash` | Annotation prefix `#[…]` and net-reference prefix `#NET`. Only emitted when `#` is **not** followed by exactly 6 hex digits. |
| `\n` | `Newline` | Item separator; emitted as a token (significant). |
| *(end)* | `Eof` | End-of-input sentinel appended by the lexer. |

Operator binding powers (left/right) are taken from `parse_pratt_expr()`:
`.` and `[]` access bind tightest (90/91), then `*` `/` (60/61), then `+` `-`
(50/51); unary `-` parses its operand at binding power 70. See
[Expressions](../language/expressions.md) and
[Grammar reference](grammar.md#expressions).

## Literal-producing token kinds

Not keywords, but produced by the lexer's literal scanners (listed for
completeness):

| `TokenKind` | Produced from |
| --- | --- |
| `Ident(String)` | An identifier run, or a digit-led run with an unknown/absent unit suffix (e.g. `74LVC1G17`). |
| `DollarIdent(String)` | `$name` reference root. |
| `String(String)` | A `"…"` double-quoted string (escapes `\\ \" \n \r \t`). |
| `Template(Vec<TemplatePart>)` | A `` `…` `` template string with `{expr}` interpolations. |
| `Integer(i32)` | A bare integer literal. |
| `Float(f64)` | A decimal literal with a fractional part. |
| `Dim(f64, Unit)` | A number immediately followed by a known unit: `mil`, `mm`, `in`, `dxp`, `raw`. |
| `Color(u8, u8, u8)` | `#RRGGBB` — `#` followed by exactly 6 hex digits. |

See [Types and values](../language/types-and-values.md) for the value each
literal compiles to.

## Contextual identifiers (not keywords)

These names drive block dispatch in the parser but lex as plain `Ident`. They
are **not** reserved and may be used as ordinary identifiers elsewhere. The
authoritative sets are the `const` arrays in
[`src/ast.rs`](../../../crates/altium-format-spec/src/ast.rs)
(`SCH_GRAPHIC_TYPES`, `PCB_GRAPHIC_TYPES`, `SCHDOC_OBJECT_TYPES`,
`PCBDOC_PRIMITIVE_TYPES`, `PCBDOC_BLOCK_TYPES`) and the literal string matches in
`parser.rs`.

| Context | Contextual identifiers |
| --- | --- |
| Top-level dispatch (`parse_spec_item`) | `placement`, `routing` |
| PcbDoc named blocks (`PCBDOC_BLOCK_TYPES`) | `polygon`, `rule`, `class`, `differential_pair` |
| PcbDoc primitives (`PCBDOC_PRIMITIVE_TYPES`) | `track`, `arc`, `via`, `fill`, `text`, `region`, `component_body`, `dimension` |
| SchDoc objects (`SCHDOC_OBJECT_TYPES`) | `wire`, `bus`, `net_label`, `power_object`, `port`, `junction`, `no_connect`, `bus_entry`, `sheet_symbol`, `parameter_set`, `note`, `probe`, `compile_mask`, `blanket`, `harness_connector`, `signal_harness` |
| SchLib graphics (`SCH_GRAPHIC_TYPES`) | `line`, `rectangle`, `arc`, `elliptical_arc`, `ellipse`, `polyline`, `polygon`, `bezier`, `pie`, `round_rectangle`, `label`, `text_frame`, `image` |
| PcbLib graphics (`PCB_GRAPHIC_TYPES`) | `track`, `arc`, `fill`, `region`, `text`, `via`, `component_body`, `line`, `polyline` |
| `project` block items | `document`, `annotation`, `erc_matrix`, `erc_levels`, `output_group`, `comparison`, `class_gen`, `library_update`, `variant`, and inside them `output`, `rule`, `variation`, `param_variation`, `match_parameter` |
| `sheet` block items | `constraint` (with kinds `edge_placement`, `directional`, `near`, `region`, `fixed_position`), `fonts`, `font` |
| `sheet_symbol` children | `entry` |
| `placement` block items | `place`, `left_of`, `right_of`, `above`, `below`, `optimize`, `minimize` (+ `subject_to`), `clearance` |
| footprint map body | `description` |
| pin connection target | `nc` (no-connect marker) |

Because these are not reserved, the `#[annotation(...)]` attribute and the
`#NET` reference both rely on the standalone `Hash` token rather than on any
keyword.
