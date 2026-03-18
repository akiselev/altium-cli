# altium-format-spec

Spec DSL for declaratively describing Altium Designer documents. Covers schematic
libraries, PCB libraries, schematic sheets, PCB boards, and PCB component placement.
Provides compile, execute (apply), reconcile (ECO diff), and dump (reverse-generate)
operations for all document types.

## Architecture

```
.spec file text
     |
     v
  lexer.rs       tokenizes with byte-offset spans on every token
     |
     v
  parser.rs      builds typed AST (ast.rs); all AST nodes carry Span {start, end}
     |
     v
  compiler.rs    lowers AST to SpecModel (model.rs); resolves layers, units, refs
     |
     v
  SpecModel      in-memory typed representation of the spec

  SpecModel is then consumed by:

  executor.rs    apply_spec_*(): SpecModel → mutate Altium document
  reconciler.rs  reconcile_*(): SpecModel diff document → EngineeringChangeOrder
  dump.rs        dump_*(): Altium document → .spec text
  spec_rewriter  (altium-cli) rewrite .pcbdoc-spec after autoplace run
```

## Placement Spec

The `placement { }` block is a sub-language within `.pcbdoc-spec` files. It drives
the `autopcb-placement` solver via the bridge in `altium-cli/src/placement_bridge.rs`.

### Constraint semantics

| Spec property | Solver constraint |
|---|---|
| `at: (x,y)` with no `autoplace: true` | `FixedPosition` — component pinned |
| `autoplace: true` (no other hint) | Free solver variable |
| `autoplace: true, edge: top, inset: 2mm` | `EdgePlacement { edge: Top, inset: 2.0 }` |
| `autoplace: true, near: $REF, max_distance: 5mm` | `Near { max_distance: 5.0 }` |
| `autoplace: true, region_name: center` | `RegionContainment` covering center quarter |
| `separate $a, $b { gap: Nmm }` | `Directional` between group centroids |
| `unplaced: autoplace` (default) | Components not in spec added as free variables |
| `unplaced: ignore` | Components not in spec pinned at current PcbDoc position |
| `unplaced: error` | Error if any PcbDoc component is missing from spec |

Named regions for `region_name:`: `center`, `top_half`, `bottom_half`, `left_half`,
`right_half`, `quadrant_tl`, `quadrant_tr`, `quadrant_bl`, `quadrant_br`.

### Autoplace pipeline

`algorithm: full_pipeline` in the `autoplace {}` block enables SA refinement (Phase 3)
and both swap passes. `algorithm: analytical` (default) runs only Phases 1–2.

After the solver runs, `spec_rewriter` (in `altium-cli`) rewrites the `.pcbdoc-spec`
file in place: `autoplace: true` becomes `at: (x, y)` + `rotation: N`, and a
`// autoplace: solved` comment is inserted. All non-placement content is preserved
verbatim using the byte-offset spans stored on AST nodes.

## Design Decisions

**Spec-as-intermediate-representation.** The solver never touches PcbDoc binaries.
The workflow is: write partial spec → solve → rewrite spec → inspect/tweak → reconcile
→ apply to .PcbDoc. This keeps placement decisions human-readable and version-controllable.

**Text-based spec rewriting (not AST round-trip).** Full AST round-trip rewriting would
require preserving all whitespace and comment tokens in the parser — significant
infrastructure. Text-based rewriting using byte-offset spans from the lexer achieves the
same result with far less code. Cost: user comments inside `place` blocks may not survive
a rewrite of that block.

**Spans on all AST nodes.** Every AST node type carries `Span { start: usize, end: usize }`
byte offsets — required for targeted spec rewriting. Without span fields, the rewriter cannot
locate `place` blocks in source text to perform targeted replacement. Amortized cost is
near-zero because the lexer already tracks positions.

**Reconciler tolerance.** Position comparison uses 0.01 mm tolerance; rotation uses 0.1°.
Altium internal coordinates are 10,000 units/mil, so Coord→f64→Coord round-trips introduce
at most ~0.003 mm error. The 0.01 mm threshold (3× round-trip error) suppresses encoding
artifacts while catching real moves. 0.1° equals Altium's minimum UI rotation granularity.

## Invariants

- `compiler.rs` resolves all unit conversions (mm, mil, inch) to internal Altium coords
  before storing in SpecModel. Downstream (executor, reconciler) never parse unit strings.
- Parameter keys in spec are case-insensitive at the compiler level; the compiler
  normalizes to lowercase before storing in SpecModel.
- The reconciler is read-only with respect to the document. Only `apply_spec_*` mutates.
- `dump_*` always sorts output by designator/name for stable diffs.
