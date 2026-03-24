# 15 - Implementation Milestones

## Ordering Rationale

The milestones are ordered by the dependency graph of the pipeline stages.
Each milestone is independently testable and delivers visible functionality.
SchLib is prioritized over PcbLib because it has better existing op coverage.

## Milestone 1: Lexer + Parser + AST

**Goal**: Parse valid spec files into an AST.

**Files created**:
- `spec/mod.rs`
- `spec/lexer.rs`
- `spec/ast.rs`
- `spec/parser.rs`

**Deliverable**: `parse_spec(source) -> Result<SpecFile, ParseError>`

**Tests**:
- Parse all 5 examples from spec-lang.md §17
- Parse error messages for common mistakes
- Noise token handling

**Dependencies**: None

**Estimated scope**: ~1500 lines (lexer ~400, AST ~300, parser ~800)

---

## Milestone 2: Expression Evaluation + Let Bindings

**Goal**: Evaluate expressions, resolve let bindings, expand spreads.

**Files created**:
- `spec/eval.rs` (expression evaluator)

**Deliverable**: Given an AST, evaluate all expressions to typed values.
Handle arithmetic, dimension units, colors, path references, and spread.

**Tests**:
- Arithmetic: `100mil + 2.54mm`
- Spread: `{ ...defaults, shape: rectangular }`
- Let binding: `let x = 5`, use `x` in expression
- Circular binding detection
- Type coercion (integer → mil in dim context)

**Dependencies**: Milestone 1

**Estimated scope**: ~600 lines

---

## Milestone 3: SchLib SpecModel Compiler (No Anchors)

**Goal**: Compile SchLib specs with absolute placement to SpecModel.

**Files created**:
- `spec/model.rs`
- `spec/compiler.rs`

**Deliverable**: `compile_spec(ast, SchLib) -> Result<SchLibSpec, SpecError>`

This milestone handles:
- Component declaration compilation
- Pin compilation with absolute `at: (x, y)` placement
- Parameter and alias compilation
- Graphic compilation
- Footprint map compilation
- Scope management (component-level, part-level)
- Forward reference resolution
- unique_id generation

Does NOT handle: anchor-based placement, imports.

**Tests**:
- Compile Example 1 (passives) with absolute pin placement
- Multi-part component (Example 4 without anchors)
- All graphic types
- Binding names → unique_ids

**Dependencies**: Milestones 1, 2

**Estimated scope**: ~800 lines

---

## Milestone 4: Reconciler + ECO Output (SchLib)

**Goal**: Diff SpecModel against SchLib document, produce ECO.

**Files created**:
- `spec/reconciler.rs`
- `spec/eco.rs`

**Deliverable**:
- `reconcile_schlib(spec, doc) -> ECO`
- `eco.render_text()` and `eco.render_json()`

**Tests**:
- Empty document: all Add
- Matching document: all Unchanged
- Partial match: mixed Add/Update/Unchanged
- Value normalization (dimension tolerance, case-insensitive enums)
- Text and JSON rendering

**Dependencies**: Milestone 3

**Estimated scope**: ~700 lines

---

## Milestone 5: Executor + Apply Pipeline (SchLib)

**Goal**: Apply ECO to SchLib documents.

**Files created**:
- `spec/executor.rs`

**Extended**:
- `spec/mod.rs` (public API: `apply_spec_to_schlib`)

**Deliverable**:
- Full pipeline: spec source → parse → compile → reconcile → execute → mutated document
- Idempotency: applying same spec twice is a no-op

**Tests**:
- Create new SchLib from spec
- Update existing SchLib from spec
- Idempotency test
- Roundtrip: apply → save → load → reconcile → all Unchanged

**Dependencies**: Milestone 4

**Estimated scope**: ~400 lines

---

## Milestone 6: CLI Commands (plan + apply + dump)

**Goal**: CLI interface for spec operations.

**Extended**:
- `altium-cli/src/main.rs` (new commands)

**Files created**:
- `spec/dump.rs`

**Deliverable**:
- `altium plan foo.sym`
- `altium apply foo.sym`
- `altium dump foo.SchLib`

**Tests**:
- Integration tests via CLI
- Dump → apply roundtrip
- JSON output validation

**Dependencies**: Milestone 5

**Estimated scope**: ~600 lines (dump ~400, CLI ~200)

---

## Milestone 7: Anchor-Based Placement

**Goal**: Support `on: $body.left, at: center` pin placement.

**Extended**:
- `spec/compiler.rs` (anchor resolution)

**Deliverable**:
- Edge anchor computation
- `at: start|center|end` positioning
- `after:`/`before:` sequencing
- `side: inside|outside|center`
- Auto orientation
- Error checking (cross-edge, mutual exclusivity)

**Tests**:
- All four edges with all positions
- Chained `after:` references
- Error: cross-edge reference
- Example 1 (passives with anchors) and Example 4 (multi-part IC with anchors)

**Dependencies**: Milestone 5

**Estimated scope**: ~500 lines

---

## Milestone 8: Import System

**Goal**: Support `import` declarations.

**Files created**:
- `spec/import.rs`

**Deliverable**:
- Named imports (`import "file" as fp`)
- Bare imports (`import "file"`)
- Cycle detection
- Cross-domain validation
- Namespace resolution (`$fp.DIP8`)

**Tests**:
- Named import with footprint reference
- Bare import merge
- Cycle detection error
- Collision detection for bare imports
- Example 4 (import footprints) and Example 5 (composable files)

**Dependencies**: Milestone 5

**Estimated scope**: ~400 lines

---

## Milestone 9: AddPad Op + PcbLib Path

**Goal**: Create footprints with pads via spec.

**Extended**:
- `pcb_ops_core.rs` (AddPad low op)
- `ops/model.rs` (AddPadHighOp)
- `ops/lower/` (AddPad composed and lowering)
- `spec/compiler.rs` (PcbLib compilation)
- `spec/reconciler.rs` (PcbLib reconciliation)
- `spec/executor.rs` (PcbLib execution)

**Deliverable**:
- `AddPad` op at all three levels (high, composed, low)
- PcbLib spec compilation
- PcbLib reconciliation
- Full pipeline: pcblib-spec → apply → PcbLib file

**Tests**:
- Create footprint with SMD pads
- Create footprint with TH pads
- All pad shapes
- Example 2 (QFP - without rows, manual pads)

**Dependencies**: Milestone 5

**Estimated scope**: ~500 lines

---

## Milestone 10: Row / Column / Grid Expansion

**Goal**: Layout blocks for regular pad patterns.

**Extended**:
- `spec/compiler.rs` (layout expansion)

**Deliverable**:
- Row expansion (anchor-based and absolute)
- Column expansion
- Grid expansion (numeric and alphanumeric naming)
- Skip semantics
- Pad override merging
- Direction control

**Tests**:
- QFP with 4 rows (Example 2)
- BGA with grid (Example 3)
- DIP with absolute rows
- Skip and override semantics

**Dependencies**: Milestone 9

**Estimated scope**: ~500 lines

---

## Milestone 11: PcbLib Dump + CLI

**Goal**: Complete PcbLib CLI support.

**Extended**:
- `spec/dump.rs` (PcbLib dump)
- CLI PcbLib paths

**Deliverable**:
- `altium dump foo.PcbLib`
- `altium plan foo.sym`
- `altium apply foo.sym`

**Tests**:
- Dump PcbLib with various pad/graphic types
- Full roundtrip

**Dependencies**: Milestone 10

**Estimated scope**: ~300 lines

---

## Summary Table

| # | Milestone | New Lines (est.) | Depends On | Delivers |
|---|-----------|-----------------|------------|----------|
| 1 | Lexer + Parser + AST | ~1500 | — | Parse spec files |
| 2 | Expression Evaluation | ~600 | 1 | Evaluate expressions |
| 3 | SchLib SpecModel (absolute) | ~800 | 1, 2 | Compile SchLib specs |
| 4 | Reconciler + ECO | ~700 | 3 | Diff + plan output |
| 5 | Executor + Apply (SchLib) | ~400 | 4 | Apply specs to SchLib |
| 6 | CLI + Dump | ~600 | 5 | plan/apply/dump commands |
| 7 | Anchor Placement | ~500 | 5 | on:/at:/after: support |
| 8 | Import System | ~400 | 5 | File composition |
| 9 | AddPad + PcbLib Path | ~500 | 5 | PcbLib spec support |
| 10 | Row/Column/Grid | ~500 | 9 | Layout expansion |
| 11 | PcbLib Dump + CLI | ~300 | 10 | PcbLib CLI support |
| **Total** | | **~6800** | | |

## Parallelization

Milestones 7, 8, and 9 are independent of each other (all depend on 5).
They can be developed in parallel.

Milestone 10 depends only on 9, and milestone 11 depends only on 10. These
form a separate PcbLib track.

```
1 → 2 → 3 → 4 → 5 → 6  (main SchLib track)
                    ├→ 7  (anchor placement)
                    ├→ 8  (imports)
                    └→ 9 → 10 → 11  (PcbLib track)
```

## Definition of Done (per milestone)

- [ ] All code compiles with no warnings
- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Examples from spec-lang.md that are in scope parse/compile/reconcile correctly
- [ ] Error messages include source spans
- [ ] No `unwrap()` in non-test code
- [ ] No opaque/raw fields (per CLAUDE.md cardinal rule)
