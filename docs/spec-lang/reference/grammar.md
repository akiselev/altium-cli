# Grammar reference

A complete EBNF-style grammar for the Altium Spec Language, reconstructed from
the recursive-descent parser. Each production cites the `parse_*` method that
implements it; **the parser is authoritative** where this prose and the grammar
disagree.

The parser lives in
[`crates/altium-format-spec/src/parser.rs`](../../../crates/altium-format-spec/src/parser.rs);
token kinds are defined in
[`src/lexer.rs`](../../../crates/altium-format-spec/src/lexer.rs) and AST nodes in
[`src/ast.rs`](../../../crates/altium-format-spec/src/ast.rs).

**Related pages**

- [Keyword reference](keywords.md) — every reserved word and token
- [Altium mapping](altium-mapping.md) — what each construct produces
- [Syntax](../language/syntax.md) — lexical structure
- [Expressions](../language/expressions.md) — expression semantics
- [Blocks overview](../language/blocks-overview.md) — block semantics

## Notation

- `lower_snake` — a grammar nonterminal (production).
- `UPPERCASE` — a terminal token kind (see [keywords](keywords.md)): `IDENT`,
  `STRING`, `TEMPLATE`, `INTEGER`, `FLOAT`, `DIM`, `COLOR`, `DOLLAR_IDENT`, plus
  literal keyword/punctuation spellings in `'quotes'`.
- `a?` optional, `a*` zero-or-more, `a+` one-or-more, `a | b` alternation,
  `( … )` grouping.
- `SEP` — an item separator: a comma, newline, or semicolon (`skip_separators` /
  `eat_separator`). The parser is largely separator-tolerant: leading/trailing
  separators are skipped and either a comma or a newline ends an item.
- `NL?` — optional newlines/semicolons consumed by `skip_newlines`.

## Spec file → items

The entry point is `parse_spec` → `parse_file` → `parse_spec_item`.

```
spec_file   = NL? ( spec_item SEP* )*
spec_item   = import_decl
            | annotation? block_decl
            | binding_decl
            | let_binding
            | placement_decl
            | routing_decl
            | pcbdoc_named_block
            | pcbdoc_primitive
            | schdoc_object
```

`parse_spec_item` dispatches in this fixed order:

1. `'import'` → `import_decl`.
2. Optional `annotation` prefix (`parse_block_annotation`); if present, the next
   token **must** begin a block declaration (`component`, `footprint`, `sheet`,
   `net`, `power`, `board`, `placement`, `routing`, or a PcbDoc named block) or
   it is an error.
3. Keyword-led blocks: `component`, `footprint`, `project`, `sheet`, `net`,
   `power`, `board`, top-level `pad`, top-level `parameter`, `swap_group`.
4. Optional `'let'`, then `IDENT '='` lookahead → `binding_decl` (a binding
   prefix in front of `component`/`footprint`/`project`/`swap_group`) or a
   plain `let_binding`.
5. Identifier-led blocks: `placement`, `routing`, then PcbDoc named/primitive
   types, then SchDoc object / graphic types.

```
binding_decl = 'let'? IDENT '=' ( component_decl | footprint_decl
                                | project_decl | swap_group_decl
                                | expr )           (* parse_spec_item *)
```

The `IDENT '='` prefix (the *binding*) names the resulting entity so a later
`$name` reference can target it; if the right side is none of the four block
keywords it is an ordinary `let_binding`.

## Annotation prefix

`parse_block_annotation`. Only the four predefined keys are accepted.

```
annotation = '#' '[' 'annotation' '(' annotation_args? ')' ']'
annotation_args = annotation_pair ( ',' annotation_pair )*
annotation_pair = annotation_key '=' annotation_value
annotation_key  = 'id' | 'stable' | 'group' | 'source_id'
annotation_value =
      STRING        (* for id, group, source_id *)
    | 'true' | 'false'   (* for stable *)
```

See [Annotations](../language/annotations.md).

## Import and let binding

```
import_decl = 'import' STRING ( 'as' IDENT )?           (* parse_import *)
let_binding = 'let'? IDENT '=' expr                     (* parse_let_binding *)
```

`let` is optional everywhere a binding is accepted; a bare `IDENT '=' expr`
is equivalent.

## SchLib: component, part, pin, parameter, alias, footprint map

```
component_decl = 'component' entity_name NL? '{' component_item* '}'   (* parse_component *)

component_item =
      part_block
    | pin_connection_decl              (* 'pin' name '->' … lookahead *)
    | pin_decl
    | parameter_decl
    | alias_decl
    | footprint_map_decl
    | let_binding
    | swap_group_decl                  (* 'swap_group' NAME, not 'swap_group:' *)
    | pad_net_decl
    | binding '=' ( pin_decl | parameter_decl | part_block
                  | graphic_decl | swap_group_decl | expr )
    | graphic_decl                     (* bare graphic type ident *)
    | property                         (* IDENT ':' value, or 'swap_group:' *)

part_block      = 'part' INTEGER NL? '{' part_item* '}'   (* parse_part_block *)
part_item       = pin_decl | let_binding | graphic_decl | property
                | binding '=' ( pin_decl | graphic_decl | expr )

pin_decl        = 'pin' entity_name NL? object           (* parse_pin *)
parameter_decl  = 'parameter' entity_name NL? object      (* parse_parameter *)
alias_decl      = 'alias' entity_name                     (* parse_alias; no body *)

pin_connection_decl = 'pin' pin_name '->' pin_target      (* parse_component_item *)
pin_name        = IDENT | INTEGER | STRING
pin_target      = '#' IDENT | 'nc'

pad_net_decl    = 'pad_net' entity_name ':' STRING        (* PcbDoc context *)

footprint_map_decl =
      'footprint' footprint_ref                            (* implicit 1:1 *)
    | 'footprint' footprint_ref '{' ( map_desc | pin_pad_pair )* '}'
footprint_ref   = entity_name | dollar_path
map_desc        = 'description' ':' STRING
pin_pad_pair    = pin_pad_ref ':' pin_pad_ref
pin_pad_ref     = ( 'pin' | 'pad' ) entity_name | dollar_path
```

`binding` is an `IDENT` captured before `'='`. See [schlib blocks](../blocks/schlib.md).

## PcbLib: footprint, pad, row/column/grid, graphics

```
footprint_decl = 'footprint' entity_name NL? '{' footprint_item* '}'  (* parse_footprint *)

footprint_item =
      pad_decl
    | 'row' NL? object
    | 'column' NL? object
    | 'grid' NL? object
    | let_binding
    | binding '=' ( pad_decl | graphic_decl | expr )
    | graphic_decl
    | property

pad_decl       = 'pad' entity_name NL? object             (* parse_pad *)
```

See [pcblib blocks](../blocks/pcblib.md).

## Graphics

```
graphic_decl = binding? graphic_type NL? object           (* parse_graphic *)
graphic_type = IDENT in SCH_GRAPHIC_TYPES ∪ PCB_GRAPHIC_TYPES
```

Graphic type names are contextual identifiers, not keywords; see
[keywords: contextual identifiers](keywords.md#contextual-identifiers-not-keywords).

## SchDoc: sheet, net, power, components, objects

```
sheet_decl  = 'sheet' NL? '{' sheet_item* '}'             (* parse_sheet *)
sheet_item  = let_binding
            | annotation? constraint_decl                  (* annotation ⇒ constraint only *)
            | font_block
            | property

constraint_decl = 'constraint' constraint_kind NL? object  (* parse_constraint_decl *)
constraint_kind = 'edge_placement' | 'directional' | 'near'
                | 'region' | 'fixed_position'

font_block  = 'fonts' NL? '{' font_decl* '}'              (* parse_font_block *)
font_decl   = 'font' INTEGER NL? object                   (* parse_font_decl *)

net_decl    = 'net' entity_name NL? object                (* parse_net *)
power_decl  = 'power' entity_name NL? object              (* parse_power *)
```

A placed SchDoc component reuses `component_decl`; its `pin X -> #NET` /
`pin X -> nc` connections are the `pin_connection_decl` production above.

SchDoc free objects are identifier-dispatched:

```
schdoc_object = schdoc_object_type schdoc_name? NL? '{' schdoc_object_item* '}'
              | 'parameter' entity_name? NL? '{' schdoc_object_item* '}'  (* keyword form *)
schdoc_object_type = IDENT in SCHDOC_OBJECT_TYPES ∪ SCH_GRAPHIC_TYPES
schdoc_name        = entity_name      (* only net_label, power_object, port,
                                         sheet_symbol, parameter_set, probe,
                                         and keyword 'parameter' take a name *)
schdoc_object_item =
      parameter_decl
    | let_binding
    | entry_decl                       (* 'entry' inside sheet_symbol *)
    | graphic_decl                     (* graphic type not followed by ':' *)
    | property

entry_decl  = 'entry' entity_name NL? object              (* parse_entry *)
```

See [schdoc blocks](../blocks/schdoc.md).

## PcbDoc: board, primitives, named blocks, pad-net

```
board_decl  = 'board' entity_name NL? '{' board_item* '}' (* parse_board *)
board_item  = let_binding | property

pcbdoc_primitive =
      pcbdoc_prim_type entity_name? NL? object             (* parse_pcbdoc_primitive *)
    | 'pad' entity_name? NL? object                        (* keyword form, top level *)
pcbdoc_prim_type = IDENT in PCBDOC_PRIMITIVE_TYPES

pcbdoc_named_block =                                        (* parse_pcbdoc_named_block *)
      'polygon' entity_name NL? object
    | 'rule' entity_name NL? object
    | 'class' entity_name NL? object
    | 'differential_pair' entity_name NL? object
```

The PcbDoc named-block types are checked **before** SchDoc object types so that
`polygon` (which exists in both) resolves to the PcbDoc form. See
[pcbdoc blocks](../blocks/pcbdoc.md).

## Routing and placement

```
routing_decl = 'routing' NL? object                       (* parse_routing_decl *)

placement_decl = 'placement' NL? '{' placement_item* '}'  (* parse_placement *)
placement_item =
      let_binding
    | annotation? 'place' entity_name ( ',' entity_name )* NL? object
    | directional_constraint
    | 'optimize' NL? object
    | 'minimize' IDENT ( 'subject_to' NL? object )?
    | 'clearance' NL? object
    | group_decl
    | separate_decl
    | 'autoplace' NL? object
    | property

directional_constraint =                                   (* parse_placement_directional_constraint *)
      ( 'left_of' | 'right_of' | 'above' | 'below' )
      dollar_path ',' dollar_path object?

group_decl    = 'group' ( IDENT | STRING ) NL? object      (* parse_placement_group *)
separate_decl = 'separate' dollar_path ( ',' dollar_path )* object?   (* parse_placement_separate *)
```

An `annotation` inside a placement block is valid **only** before a `place`
block. See [placement blocks](../blocks/placement.md).

## PrjPcb: project and its items

```
project_decl = 'project' entity_name NL? '{' project_item* '}'  (* parse_project *)

project_item =
      let_binding
    | property
    | IDENT '=' expr                   (* let binding without 'let' *)
    | 'document'  document_block
    | 'annotation' annotation_block
    | 'erc_matrix' erc_matrix_block
    | 'erc_levels' erc_levels_block
    | 'output_group' output_group_block
    | 'comparison' comparison_block
    | 'class_gen' property_block
    | 'library_update' property_block
    | 'variant' variant_block

document_block      = entity_name NL? '{' property* '}'         (* parse_document_block *)

annotation_block    = NL? '{' ( match_parameter_decl | property )* '}'  (* parse_annotation_block *)
match_parameter_decl = 'match_parameter' INTEGER NL? object

erc_matrix_block    = NL? '{' erc_matrix_entry* '}'            (* parse_erc_matrix_block *)
erc_matrix_entry    = '(' IDENT ',' IDENT ')' ':' IDENT

erc_levels_block    = NL? '{' erc_level_entry* '}'            (* parse_erc_levels_block *)
erc_level_entry     = IDENT ':' expr

output_group_block  = entity_name NL? '{' ( output_block | property )* '}'  (* parse_output_group_block *)
output_block        = 'output' entity_name NL? '{' property* '}'

comparison_block    = NL? '{' comparison_rule* '}'           (* parse_comparison_block *)
comparison_rule     = 'rule' entity_name NL? object

variant_block       = entity_name NL? '{' ( variation_decl
                                          | param_variation_decl
                                          | property )* '}'   (* parse_variant_block *)
variation_decl       = 'variation' entity_name NL? object
param_variation_decl = 'param_variation' entity_name NL? object

property_block      = NL? '{' property* '}'                  (* parse_property_block *)
```

The `document`, `annotation`, `variant`, `output`, `rule`, `match_parameter`,
`variation`, and `param_variation` lead words are contextual identifiers, not
reserved keywords. See [prjpcb blocks](../blocks/prjpcb.md).

## Swap group

```
swap_group_decl = binding? 'swap_group' entity_name NL? object  (* parse_swap_group_decl *)
```

## Objects, properties, spreads

`parse_object` / `parse_object_item` / `parse_property`. An object body requires
a separator between items.

```
object      = '{' ( object_item ( SEP+ ) )* object_item? '}'  (* separator required between items *)
object_item = '...' expr                  (* spread, parse_object_item *)
            | let_binding                  (* 'let' IDENT '=' expr, or IDENT '=' expr *)
            | property
property    = property_key ':' expr        (* parse_property *)
property_key = IDENT | KEYWORD             (* any keyword token, via try_eat_property_key *)
```

A property key may be any reserved keyword spelled as text (e.g. `pin:`,
`net:`, `group:`).

## Entity names

```
entity_name = IDENT | STRING | INTEGER     (* parse_entity_name *)
```

## Dollar paths and references

```
dollar_path = DOLLAR_IDENT path_step*       (* parse_dollar_path_reference *)
path_step   = '.' IDENT | '[' expr ']'      (* parse_dollar_path_tail *)
```

A `dollar_path` is a structural reference to a previously-bound entity
(`$name.field[index]…`).

## Expressions

`parse_expr` → `parse_pratt_expr(0)` (a Pratt parser) → `parse_prefix_expr`.

```
expr        = pratt_expr
pratt_expr(min_bp):                          (* parse_pratt_expr *)
            prefix_expr ( infix_op )*        (* climbing by binding power *)

infix_op    = '.'  IDENT                      (* Path access,  bp 90/91 *)
            | '['  expr ']'                   (* Index,        bp 90/91 *)
            | '*'  pratt_expr                 (* Mul,          bp 60/61 *)
            | '/'  pratt_expr                 (* Div,          bp 60/61 *)
            | '+'  pratt_expr                 (* Add,          bp 50/51 *)
            | '-'  pratt_expr                 (* Sub,          bp 50/51 *)

prefix_expr =                                  (* parse_prefix_expr *)
      STRING | TEMPLATE | INTEGER | FLOAT | DIM | COLOR
    | 'true' | 'false' | 'null'
    | DOLLAR_IDENT path_tail                   (* $ref with .field / [idx] tail *)
    | ( 'power' | 'net' | 'sheet'              (* keyword-as-value ⇒ Expr::Ident *)
      | 'autoplace' | 'group' | 'separate' )
    | IDENT call_args? path_tail               (* ident, call, or ref + path tail *)
    | '-' pratt_expr(70)                       (* unary negation *)
    | '(' expr ( ',' expr )? ')'               (* grouping or 2-tuple/coord *)
    | '[' ( expr SEP )* ']'                    (* array *)
    | object                                   (* nested object *)

path_tail   = ( '.' IDENT | '[' expr ']' )*
call_args   = '(' ( arg ( SEP arg )* )? ')'    (* parse_call_args *)
arg         = expr                              (* positional *)
            | IDENT ':' expr                    (* named; must follow all positionals *)
```

Notes on expressions:

- A parenthesized `(a, b)` with one comma is a **2-tuple** (`Expr::Tuple`), used
  for coordinates; a single parenthesized expression is just grouping.
- A function call is `IDENT '(' … ')'`; named arguments (`name: value`) must come
  after all positional arguments.
- Unary `-` parses its operand at binding power 70, so `-a.b` negates `a.b`.

The `BinOp` kinds (`Add`, `Sub`, `Mul`, `Div`) come from
[`src/diagnostic.rs`](../../../crates/altium-format-spec/src/diagnostic.rs); the
expression AST (`Expr`) is in `ast.rs`. See [Expressions](../language/expressions.md)
and [Types and values](../language/types-and-values.md).
