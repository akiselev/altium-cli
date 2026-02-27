# Spec Language: Gap Analysis & Next Steps

Audit date: 2026-02-27
Compared: `docs/plans/spec-lang/` (16 plan docs) + `docs/spec-lang.md` v0.3 against `crates/altium-format-ops/src/spec/` (12 files, ~11,700 lines)

## Status Summary

| Milestone | Plan | Status | Notes |
|-----------|------|--------|-------|
| M1: Lexer + Parser + AST | `lexer.rs`, `parser.rs`, `ast.rs` | **Complete** | All token types, full recursive-descent parser, Pratt expressions |
| M2: Expression Evaluation | `eval.rs` | **Complete** | Dim arithmetic, spread, scoping, circular binding detection |
| M3: SchLib SpecModel | `compiler.rs`, `model.rs` | **Complete** | Components, pins, parameters, aliases, graphics, footprint maps |
| M4: Reconciler + ECO | `reconciler.rs`, `eco.rs` | **Partial** | SchLib reconciler works; PcbLib reconciler is a stub (see gap G1) |
| M5: Executor + Apply | `executor.rs` | **Complete** | ECO→HighOp for Add/Update/Unchanged, full apply pipeline |
| M6: CLI + Dump (SchLib) | CLI in `main.rs`, `dump.rs` | **Complete** | `spec plan`, `spec apply`, `spec dump` commands all implemented |
| M7: Anchor Placement | `compiler.rs` | **Complete** | `on:`/`at:`/`after:`/`before:`, side offset, auto orientation |
| M8: Import System | `import.rs` | **Complete** | Cycle detection, cross-domain rules, named + bare imports |
| M9: AddPad + PcbLib Path | `pcb_ops_core.rs`, `model.rs` | **Complete** | AddPad at all 3 levels (HighOp, ComposedOp, LowOp), PcbLib apply path |
| M10: Row/Column/Grid | `compiler.rs` | **Complete** | Row, column, grid expansion, BGA naming, skip, pad override |
| M11: PcbLib Dump + CLI | `dump.rs`, CLI | **Complete** | `dump_pcblib`, PcbLib plan/apply/dump all wired up |


## Gaps (ordered by severity)

### G1: PcbLib reconciler does not query existing documents (HIGH)

**Plan (08-reconciler.md):** `reconcile_pcblib(spec, doc)` should query the existing PcbLib for footprints, pads, and graphics, then diff against the spec — producing Add, Update, or Unchanged entries.

**Actual:** `reconcile_pcblib()` delegates directly to `reconcile_pcblib_empty()`, treating every footprint as a new Add regardless of what already exists in the document. The function signature doesn't even accept a `&mut PcbLib` reference.

**Impact:** `spec apply` on an existing PcbLib re-adds every footprint every time (not idempotent). `spec plan` against an existing PcbLib always shows everything as Add.

**What's needed:**
- Query existing PcbLib footprints by `display_name` (case-insensitive)
- Query existing pads by `pad_name` within each footprint
- Diff pad properties (position, shape, size, hole, rotation, layer, mask expansions)
- Diff footprint properties (description, height)
- Diff PCB graphics by unique_id
- Build a `PcbDocView` analogous to the SchLib `DocView`
- This requires PcbLib query low-ops (or direct model access) — need to verify what's available

---

### G2: SchLib reconciler skips pin position/orientation/length comparison (MEDIUM)

**Plan (08-reconciler.md):** The reconciler compares all spec fields against document fields, using ±1 internal unit tolerance for dimensions.

**Actual:** `reconcile_pin()` (line 525) only compares `name`, `electrical`, and `is_hidden`. The `DocPin` struct *stores* `x_mils`, `y_mils`, `length_mils`, `orientation` but `reconcile_pin()` never reads them. The dead_code warnings in diagnostics confirm this:

```
reconciler.rs:187:5 fields `designator`, `owner_part_id`, `x_mils`, `y_mils`,
                    `length_mils`, and `orientation` are never read
```

**Impact:** A pin that moves (different `at:` in spec vs document) or changes length is never detected as Update — it stays Unchanged. Position changes are silently ignored.

**What's needed:**
- Compare `spec.location` against `(doc_pin.x_mils, doc_pin.y_mils)` with ±1 tolerance
- Compare `spec.orientation` against `doc_pin.orientation`
- Compare `spec.length` against `doc_pin.length_mils` with ±1 tolerance
- Compare `spec.hidden_net_name` against doc if applicable

---

### G3: SchLib graphic reconciliation always emits Add (MEDIUM)

**Plan (08-reconciler.md):** Graphics matched by `unique_id` (case-sensitive). Existing graphics with matching unique_id produce Update if properties differ, Unchanged if identical.

**Actual:** `reconcile_component_children()` (line 511-515):
```rust
// Graphics have no doc-side query result, treat as Add (stable via unique_id)
children.push(graphic_to_add(graphic_spec));
```

All graphics are always Add regardless of whether they already exist in the document.

**Impact:** Every `spec apply` re-adds all graphics. For graphics with stable `unique_id` (binding names), this may cause duplicates depending on how the executor handles it. The ECO always over-reports changes.

**What's needed:**
- Query existing graphics from document by `unique_id`
- Compare graphic properties (position, dimensions, colors, etc.)
- Emit Unchanged/Update/Add appropriately

---

### G4: Footprint map reconciliation always emits Add (LOW)

**Plan (08-reconciler.md):** Footprint maps matched by `model_name`. Should detect Update if pin-pad mappings differ.

**Actual:** `reconcile_component_children()` (line 517-519):
```rust
for fp_spec in &spec.footprints {
    children.push(footprint_to_add(fp_spec));
}
```

Always Add, no diff against existing footprint maps.

**Impact:** Footprint maps are small metadata. Re-adding is functionally correct but the ECO over-reports.

---

### G5: No integration tests for spec pipeline (MEDIUM)

**Plan (14-testing.md):** Calls for dedicated integration test files:
- `spec_parse_tests.rs` — parse all §17 examples
- `spec_compile_tests.rs` — compile to SpecModel, verify coordinates
- `spec_reconcile_tests.rs` — reconcile against empty and non-empty
- `spec_apply_tests.rs` — full pipeline
- `spec_roundtrip_tests.rs` — dump → parse → apply → semantic diff

**Actual:** No spec-specific test files in `crates/altium-format-ops/tests/`. All tests are inline unit tests within each module. Existing integration tests (`executor_integration.rs`, `executor_proptest.rs`, etc.) test the general ops executor, not the spec-specific ECO→HighOp→apply path.

**Impact:** No end-to-end tests that verify the full pipeline: parse spec → compile → reconcile → ECO → execute → apply → verify document. Unit tests cover individual modules but don't catch integration bugs.

---

### G6: Footprint validation during reconciliation not implemented (LOW)

**Plan (08-reconciler.md §Footprint validation):** During reconciliation, cross-reference checks should validate:
1. Referenced footprint exists in imported pcblib-spec
2. All mapped pads exist in the footprint definition
3. All mapped pins exist in the component
4. No pad mapped more than once (E_DUPLICATE_MAP)
5. Unmapped pads emit informational note

**Actual:** No validation of footprint maps against imported specs. The compiler resolves the `model_name` from imports but doesn't validate pin/pad existence or detect duplicate maps.

---

### G7: `component_kind` and `show_hidden_pins` not compared in SchLib reconciler (LOW)

**Plan:** Component reconciler should compare all specified properties.

**Actual:** `reconcile_schlib_against_view()` only compares `description` and `part_count` against the document. `component_kind` and `show_hidden_pins` from the spec are not compared. The `DocComponent` struct doesn't even store these fields.

---

### G8: Parameter `is_hidden` not queried from document (LOW)

**Actual:** The `DocParameter` stores `is_hidden` but `query_doc_view()` doesn't populate it from the QueryRecords result — it's always `false`. The reconciler compares against this defaulted value.

---

## Compiler warnings (from diagnostics)

These are all caused by the reconciler gaps above:

| File | Warning | Root Cause |
|------|---------|------------|
| `reconciler.rs:173` | field `lib_reference` never read | DocComponent.lib_reference stored but unused |
| `reconciler.rs:184-192` | fields `designator`, `owner_part_id`, `x_mils`, `y_mils`, `length_mils`, `orientation` never read | DocPin fields stored but not compared (G2) |
| `pcb_ops_core.rs:395` | `ensure_primitive_section` never used | Dead code from earlier refactor |
| `compiler.rs:165` | variable does not need to be mutable | Minor: `let mut` where `let` suffices |
| `compiler.rs:218,279` | methods `compile_part` and `compile_pin` never used | Dead code — stale method signatures |
| `typecheck.rs:68` | fields never read in Value variant | Typecheck Value tuple fields unused |


## What was fully delivered

Despite the gaps above, the implementation is remarkably complete. The following are all fully working:

- **Full lexer** with all token types including template strings, dimensional literals, colors
- **Full parser** covering both SchLib and PcbLib grammars with noise tolerance
- **Expression evaluator** with dimensional arithmetic, spread, scoping, circular detection
- **Import resolver** with cycle detection, cross-domain rules, named/bare imports
- **Compiler** with anchor placement, row/column/grid expansion, BGA naming, unique_id generation, forward references, all graphic types for both domains
- **SpecModel** with complete typed IR for both SchLib and PcbLib
- **ECO output** with both text (box-drawing) and JSON formats
- **Executor** converting ECO to HighOps for all entity types
- **Dump** reverse-generating spec from both SchLib and PcbLib
- **CLI** with `spec plan`, `spec apply`, `spec dump` commands
- **AddPad** at all three levels (HighOp, ComposedOp, LowOp)
- **SchLib reconciliation** against existing documents (components, pins, parameters, aliases)


## Recommended next priorities

1. **G2: Pin position/orientation/length comparison** — quickest fix, eliminates dead_code warnings, makes SchLib reconciliation actually useful for detecting positional changes
2. **G1: PcbLib reconciler** — largest gap, blocks idempotent PcbLib workflow
3. **G3: Graphic reconciliation** — needed for correct ECO reports
4. **G5: Integration tests** — validates the full pipeline end-to-end
5. **G7/G8: Minor reconciler field gaps** — small additions to existing reconciler
6. **G4/G6: Footprint map reconciliation and validation** — correctness improvements
