# Spec Language Testing Gap Analysis

Status: as of 2026-02-27, covering `crates/altium-format-spec/` v0.2.0

## Current Coverage (258 unit tests)

| Module | Tests | Notes |
|--------|-------|-------|
| Lexer | 56 | Good: all token types, edge cases, escape sequences |
| Parser | 90 | Good: expressions, components, footprints, imports, operators |
| Compiler | 43 | Good: component/pin/footprint compilation, scoping, graphics |
| Eval | 28 | Good: arithmetic, units, let bindings, circular refs, spread |
| Reconciler | 10 | All use mock `DocView` structs — never real documents |
| Dump | 16 | Formatting helpers only (`format_coord_mils`, `quote_string`, etc.) |
| Import | 8 | File resolution, cycle detection, cross-domain validation |
| ECO | 4 | Timestamp serialization only |
| **Executor** | **0** | **Completely untested** |
| AST/Model/Diagnostic | 0 | Data types only — acceptable |

## Reusable Infrastructure from altium-format / altium-format-ops

These exist and should be reused by spec tests:

- `SchLib::validate_invariants()` — validates header/component count, owner indices, ownership chains
- `PcbLib::validate_invariants()` — validates header version, footprint names, storage keys, section keys
- `save_reopen_schlib(lib)` — validates before save, saves, reopens, validates again, two-save semantic diff
- `save_reopen_pcblib(lib)` — same pattern for PcbLib
- `assert_cfb_files_semantic_eq(path_a, path_b)` — semantic CFB diff (order-agnostic params, decompressed embedded objects)
- `diff_cfb_files_semantic(path_a, path_b)` — returns `CfbSemanticDiffReport` for inspection
- Proptest strategies in `executor_proptest.rs` — random HighOp generation with `edge_i32`, `norm_u8`, `poly_points`


## Gap 1: Zero Integration Tests (spec → document → validate)

No test takes a spec string, parses it, compiles it, applies it to a real
`SchLib`/`PcbLib`, and calls `validate_invariants()`. This is the most basic
end-to-end validation.

**Needed tests:**

- Simple component spec → empty SchLib → `validate_invariants()`
- Multi-component spec → empty SchLib → `validate_invariants()`
- Component with all 13 SchLib graphic types → `validate_invariants()`
- Multi-part component (LM358-style with shared + part-scoped pins) → `validate_invariants()`
- Simple footprint spec → empty PcbLib → `validate_invariants()`
- Footprint with multiple pads (different shapes, layers) → `validate_invariants()`

**Example pattern:**
```rust
#[test]
fn spec_apply_schlib_validates() {
    let source = r#"
        component R {
            designator: "R?"
            description: "Resistor"
            body = rectangle { from: (-20mil, -10mil), to: (20mil, 10mil), is_solid: true }
            pin 1 { at: (-30, 0), orientation: 0, electrical: passive, length: 25 }
            pin 2 { at: (30, 0), orientation: 180, electrical: passive, length: 25 }
            parameter Value { text: "10K" }
        }
    "#;
    let ast = parse_spec(source);
    let model = compile_spec(&ast, SpecDomain::SchLib).unwrap();
    let spec = match model { SpecModel::SchLib(s) => s, _ => panic!() };
    let mut doc = SchLib::empty();
    apply_spec_schlib(&spec, &mut doc).unwrap();
    doc.validate_invariants().unwrap();
}
```


## Gap 2: Zero Roundtrip Tests (spec → apply → save → reopen → validate)

The `save_reopen_schlib` / `save_reopen_pcblib` harness exists in
altium-format-ops but is never used by the spec crate. No test verifies that
spec-generated documents survive save/reopen with semantic CFB equality.

**Needed tests:**

- Apply spec → save → reopen → `validate_invariants()` → second save → `assert_cfb_files_semantic_eq`
- Same for PcbLib

**Example pattern:**
```rust
#[test]
fn spec_schlib_roundtrip() {
    let source = r#"
        component R { designator: "R?", pin 1 { at: (-30, 0), orientation: 0 } }
    "#;
    let ast = parse_spec(source);
    let model = compile_spec(&ast, SpecDomain::SchLib).unwrap();
    let spec = match model { SpecModel::SchLib(s) => s, _ => panic!() };
    let mut doc = SchLib::empty();
    apply_spec_schlib(&spec, &mut doc).unwrap();
    // Uses the existing two-save-compare pattern with semantic CFB diff
    save_reopen_schlib(&doc);
}
```


## Gap 3: Zero Idempotency Tests (the core spec promise)

The spec's #1 design goal is "applying the same spec twice is a no-op." This
is completely untested.

**Needed tests:**

- Apply spec to empty doc → apply same spec again → verify no LowOps emitted
  (or all `EntityChange::Unchanged` in reconciler output)
- Apply spec → save → reopen → apply spec again → `validate_invariants()`

**Example pattern:**
```rust
#[test]
fn spec_apply_is_idempotent() {
    let source = r#"
        component R { designator: "R?", pin 1 { at: (-30, 0), orientation: 0 } }
    "#;
    let ast = parse_spec(source);
    let model = compile_spec(&ast, SpecDomain::SchLib).unwrap();
    let spec = match model { SpecModel::SchLib(s) => s, _ => panic!() };

    let mut doc = SchLib::empty();
    apply_spec_schlib(&spec, &mut doc).unwrap();
    doc.validate_invariants().unwrap();

    // Second application
    let eco = reconcile_schlib(&spec, &mut doc, "test.SchLib".into(), "test.schlib-spec".into()).unwrap();
    for change in &eco.changes {
        assert!(matches!(change, EntityChange::Unchanged { .. }),
            "expected all Unchanged after second apply, got: {change:?}");
    }
}
```


## Gap 4: Zero Dump Roundtrip Tests

No test verifies that `dump(doc) → parse → compile → apply(empty)` produces a
valid document. The dump output could produce syntactically invalid or
semantically wrong spec source.

**Needed tests:**

- Open fixture SchLib → `dump_schlib` → `parse_spec` → `compile_spec` →
  `apply_spec_schlib(empty)` → `validate_invariants()`
- Open fixture PcbLib → `dump_pcblib` → `parse_spec` → `compile_spec` →
  `apply_spec_pcblib(empty)` → `validate_invariants()`

These should be gated behind `#[cfg(feature = "test-fixtures")]`.

**Example pattern:**
```rust
#[cfg(feature = "test-fixtures")]
#[test]
fn dump_schlib_roundtrip_fixture() {
    let lib = SchLib::open(fixture_path("Misc.SchLib")).unwrap();
    let spec_source = dump_schlib(&lib);

    // The dumped source must parse and compile cleanly
    let ast = parse_spec(&spec_source);
    let model = compile_spec(&ast, SpecDomain::SchLib).unwrap();
    let spec = match model { SpecModel::SchLib(s) => s, _ => panic!() };

    // Apply to empty doc and validate
    let mut doc = SchLib::empty();
    apply_spec_schlib(&spec, &mut doc).unwrap();
    doc.validate_invariants().unwrap();
}
```


## Gap 5: Zero Property-Based Tests

No proptests exist despite `proptest` being a dev-dependency in Cargo.toml.

### Proptest 5a: Random valid spec models → apply → validate invariants

Generate random `SchLibSpec` / `PcbLibSpec` models (not source strings — build
the model directly), apply to empty documents, validate invariants. This
catches executor bugs that produce invalid document structures.

Should reuse `SchLib::validate_invariants()` and `PcbLib::validate_invariants()`.

**Strategy approach** (similar to `executor_proptest.rs`):
```rust
proptest! {
    #![proptest_config(ProptestConfig::with_cases(36))]
    #[test]
    fn prop_spec_schlib_apply_validates(
        plans in prop::collection::vec(
            prop::collection::vec(
                (0u8..20, -5000i32..5000, -5000i32..5000, -5000i32..5000, -5000i32..5000),
                0..8
            ),
            1..4
        )
    ) {
        let spec = build_random_schlib_spec(plans);
        let mut doc = SchLib::empty();
        apply_spec_schlib(&spec, &mut doc).unwrap();
        doc.validate_invariants().unwrap();
    }
}
```

The `build_random_schlib_spec` helper would generate components with:
- Random lib_reference (unique per component)
- Random designator patterns
- Random pins at valid coordinates with valid electrical types
- Random parameters with names/text
- Random graphics (rectangle, line, arc, etc.) with valid coords
- Optional multi-part structure

### Proptest 5b: Random spec → apply → save → reopen → validate

Same as 5a but with roundtrip. Catches serialization bugs in spec-generated entities.

```rust
proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]
    #[test]
    fn prop_spec_schlib_roundtrip(plans in /* same strategy */) {
        let spec = build_random_schlib_spec(plans);
        let mut doc = SchLib::empty();
        apply_spec_schlib(&spec, &mut doc).unwrap();
        save_reopen_schlib(&doc); // validates + semantic CFB diff
    }
}
```

### Proptest 5c: Idempotency property

For any spec S and empty doc D: applying S twice produces all-Unchanged ECO.

```rust
proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]
    #[test]
    fn prop_spec_idempotent(plans in /* same strategy */) {
        let spec = build_random_schlib_spec(plans);
        let mut doc = SchLib::empty();
        apply_spec_schlib(&spec, &mut doc).unwrap();

        let eco = reconcile_schlib(&spec, &mut doc, ...);
        for change in &eco.changes {
            prop_assert!(matches!(change, EntityChange::Unchanged { .. }));
        }
    }
}
```

### Proptest 5d: Parser fuzz (never panics)

Similar to the existing `prop_parser_never_panics_on_random_text` in
altium-format-ops parser. Arbitrary strings → `parse_spec()` must return
`Ok` or meaningful error, never panic.

```rust
proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]
    #[test]
    fn prop_parser_never_panics(input in "\\PC{0,500}") {
        let _ = parse_spec(&input); // must not panic
    }
}
```

### Proptest 5e: Lexer fuzz (never panics)

```rust
proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]
    #[test]
    fn prop_lexer_never_panics(input in "\\PC{0,500}") {
        let _ = lex(&input); // must not panic
    }
}
```

### Proptest 5f: Dump → reparse → recompile roundtrip

For generated specs, `dump(apply(empty, spec))` produces a valid spec that
recompiles to an equivalent model.

```rust
proptest! {
    #![proptest_config(ProptestConfig::with_cases(16))]
    #[test]
    fn prop_dump_recompile_roundtrip(plans in /* strategy */) {
        let spec = build_random_schlib_spec(plans);
        let mut doc = SchLib::empty();
        apply_spec_schlib(&spec, &mut doc).unwrap();

        let dumped = dump_schlib(&doc);
        let ast = parse_spec(&dumped); // must not error
        let model = compile_spec(&ast, SpecDomain::SchLib).unwrap();
        let respec = match model { SpecModel::SchLib(s) => s, _ => panic!() };

        let mut doc2 = SchLib::empty();
        apply_spec_schlib(&respec, &mut doc2).unwrap();
        doc2.validate_invariants().unwrap();
    }
}
```

### Proptest 5g: ECO summary accuracy

For any spec and empty doc, `reconcile_empty` should produce all-Adds with
counts matching the spec's entity count.

```rust
proptest! {
    #[test]
    fn prop_eco_summary_accurate(plans in /* strategy */) {
        let spec = build_random_schlib_spec(plans);
        let eco = reconcile_schlib_empty(&spec, ...);

        let comp_summary = eco.summary.by_kind.get(&EntityKind::Component).unwrap();
        prop_assert_eq!(comp_summary.adds, spec.components.len());
        prop_assert_eq!(comp_summary.updates, 0);
        prop_assert_eq!(comp_summary.unchanged, 0);
    }
}
```


## Gap 6: No Reconciler Tests Against Real Documents

All 10 reconciler tests build mock `DocView` structs. None test
`reconcile_schlib()` against a real `SchLib` document (which exercises
`query_doc_view`).

**Needed tests:**

- Create SchLib via spec → `reconcile_schlib` with same spec → all Unchanged
- Create SchLib via spec A → `reconcile_schlib` with superset spec B → correct
  adds/updates
- Open fixture SchLib → `reconcile_schlib` with spec covering some components →
  correct mix of Add/Update/Unchanged

The first test (create then reconcile) is the integration form of the
idempotency property. The fixture-based test should be gated behind
`#[cfg(feature = "test-fixtures")]`.


## Gap 7: Zero Executor Tests

`executor.rs` has zero `#[cfg(test)]` blocks. The entire
`build_schlib_low_ops` / `build_pcblib_low_ops` / `emit_*` chain is untested
in isolation.

**Needed tests:**

- `build_schlib_low_ops` for add-new-component: verify correct LowOp sequence
  (CreateComponentRoot, CreateComponentDesignator, CreateComponentComment,
  AddPin × N, AddParameter × N, etc.)
- `build_schlib_low_ops` for update-existing-component: verify EditComponent /
  EditPin / EditParameter emitted only when values differ
- `build_pcblib_low_ops`: verify AddFootprint + AddPad sequence

These are unit tests for the executor's LowOp generation, independent of the
integration tests that exercise the full pipeline.


## Gap 8: No Fixture-Based Integration Tests

No test applies a spec to an existing fixture library from `data/schlib/` or
`data/pcblib/` to verify additive semantics work on real-world documents.

**Needed tests (gated behind `test-fixtures`):**

- Open fixture SchLib → apply spec adding one new component → validate
  invariants → verify original components untouched
- Open fixture PcbLib → apply spec adding one new footprint → validate
  invariants → verify original footprints untouched


## Implementation Priority

| Priority | Gap | What | Why |
|----------|-----|------|-----|
| P0 | Gap 1 | Integration: spec → apply → validate | Catches executor bugs, validates fundamental correctness |
| P0 | Gap 2 | Integration: spec → apply → save → reopen | Catches serialization bugs, uses existing roundtrip harness |
| P1 | Gap 5a,b | Proptest: random specs → apply → validate/roundtrip | Broad fuzzing of executor, reuses `validate_invariants` |
| P1 | Gap 3 | Integration + proptest: idempotency | Core spec promise, completely untested |
| P2 | Gap 5d,e | Proptest: parser/lexer fuzz | Panic safety |
| P2 | Gap 4 | Dump roundtrip against fixtures | Dump correctness |
| P3 | Gap 6 | Reconciler against real docs | Reconciler correctness beyond mock DocViews |
| P3 | Gap 7 | Executor unit tests | LowOp generation correctness |
| P3 | Gap 5f,g | Proptest: dump roundtrip, ECO accuracy | Full-stack property coverage |
| P4 | Gap 8 | Fixture-based additive semantics | Real-world document compatibility |


## Test File Organization

Suggested structure:

```
crates/altium-format-spec/
  tests/
    integration.rs              # P0/P1: end-to-end parse → apply → validate
    roundtrip.rs                # P0: save → reopen → semantic diff
    idempotency.rs              # P1: apply twice → all unchanged
    dump_roundtrip.rs           # P2: dump → reparse → apply → validate (fixture-gated)
    fixture_additive.rs         # P4: spec on existing fixture docs
  src/
    executor.rs                 # P3: add #[cfg(test)] unit tests
    parser.rs                   # P2: add proptest fuzz block
    lexer.rs                    # P2: add proptest fuzz block
    reconciler.rs               # P3: add real-document reconciler tests
  proptest-regressions/         # Created automatically by proptest
```

All proptest blocks should be gated with `#[cfg(feature = "proptest")]`.
All fixture-dependent tests should be gated with `#[cfg(feature = "test-fixtures")]`.
