# 14 - Testing Strategy

## Test Layers

### Layer 1: Unit Tests (in-module `#[cfg(test)]`)

Each module has focused unit tests:

| Module | Tests |
|--------|-------|
| `spec/lexer.rs` | Token recognition, dimension parsing, color parsing, template strings, comments, error cases |
| `spec/parser.rs` | Parse every grammar production, error messages, noise token handling |
| `spec/import.rs` | Cycle detection, namespace construction, cross-domain validation |
| `spec/compiler.rs` | Binding evaluation, spread expansion, type coercion, anchor resolution, layout expansion, error detection |
| `spec/model.rs` | SpecModel construction, unique_id generation |
| `spec/reconciler.rs` | Identity matching, value normalization, diff computation |
| `spec/executor.rs` | ECO to HighOp mapping |
| `spec/eco.rs` | Text and JSON rendering |
| `spec/dump.rs` | Reverse generation, entity name quoting, coordinate formatting |

### Layer 2: Integration Tests

Location: `crates/altium-format-ops/tests/`

**spec_parse_tests.rs**: Parse every example from spec-lang.md §17 and verify
AST structure.

**spec_compile_tests.rs**: Compile examples to SpecModel, verify absolute
coordinates, verify type resolution.

**spec_reconcile_tests.rs**: Reconcile against empty and non-empty documents,
verify ECO structure.

**spec_apply_tests.rs**: Full pipeline: spec source -> apply to document ->
verify document state.

**spec_roundtrip_tests.rs**: Dump document -> parse spec -> apply to empty
document -> semantic diff against original.

### Layer 3: Property Tests (behind `--features proptest`)

Location: `crates/altium-format-ops/tests/spec_proptest.rs`

**Arbitrary spec generation**: Generate random but valid spec files, verify
they parse without errors.

**Roundtrip property**: For any valid spec, parse -> compile -> reconcile
against empty -> apply -> dump -> parse -> reconcile against result -> all
Unchanged.

**Idempotency**: Apply spec once, apply again -> second apply has zero changes.

### Layer 4: Fixture Tests (behind `--features test-fixtures`)

Using real Altium files from `data/schlib/` and `data/pcblib/`:

**dump_and_reapply**: Load fixture, dump to spec, apply spec to empty document,
semantic diff against fixture (should be minimal differences).

**plan_against_fixture**: Parse fixture, dump to spec, plan against the same
fixture -> all Unchanged (verifies reconciler correctness against real data).

## Key Test Scenarios

### Parser

1. Empty spec file
2. Single component with no children
3. Component with all child types (pins, parameters, aliases, graphics, footprints)
4. Multi-part component
5. Footprint with pads and graphics
6. Footprint with row/column/grid
7. Import declarations (named and bare)
8. Let bindings at file level and inside entities
9. Spread operator in objects
10. All expression types (arithmetic, path, tuple, array, object)
11. Template strings with interpolation
12. All graphic types
13. Noise tokens (`let`, `;`, trailing commas)
14. Comments (line and block, nested)
15. Entity names: unquoted ident, quoted string, integer
16. Binding prefix on every entity type

### Compiler

1. Anchor resolution on all four edges
2. `at: start`, `at: center`, `at: end`
3. `after:` / `before:` chaining
4. `side: inside` / `outside` / `center`
5. Auto orientation
6. Row expansion (anchor-based and absolute)
7. Grid expansion (numeric and alphanumeric naming)
8. Skip semantics
9. Pad override (row + explicit pad)
10. Forward reference resolution
11. Circular reference detection
12. Cross-edge reference error
13. Type coercion (integers → mils, enums)
14. Spread evaluation
15. Nested object evaluation

### Reconciler

1. Empty document → all Add
2. Identical → all Unchanged
3. Single field difference → Update
4. New pin in existing component → Add nested under Update
5. Dimension tolerance (±1 internal unit)
6. Case-insensitive identity matching
7. Multi-part pin matching (`(owner_part_id, designator)`)
8. Footprint map validation (missing pad, duplicate map)
9. Additive semantics (document-only entities preserved)

### Full Pipeline

1. Spec with no target → create new document from scratch
2. Spec with matching target → no changes
3. Spec with different target → ECO shows updates
4. Two sequential applies → second is no-op (idempotency)
5. Apply, modify document externally, re-apply → only spec-declared entities updated
6. Import chain: main spec imports passives and ICs, apply creates combined library

## Test File Organization

```
crates/altium-format-ops/tests/
    spec_parse_tests.rs           # Parse examples from spec
    spec_compile_tests.rs         # Compile to SpecModel
    spec_reconcile_tests.rs       # Reconciler tests
    spec_apply_tests.rs           # Full pipeline tests
    spec_roundtrip_tests.rs       # Dump -> parse -> apply roundtrip
    spec_proptest.rs              # Property-based tests

crates/altium-format-ops/tests/fixtures/
    passives.schlib-spec          # Test fixture: simple passives
    qfp.pcblib-spec               # Test fixture: QFP footprint
    multi-part.schlib-spec        # Test fixture: multi-part IC
    import-main.schlib-spec       # Test fixture: import chain
    import-passives.schlib-spec   # Test fixture: imported file
```

## Feature Flag Gating

```rust
// Tests that read fixture files from data/
#[cfg(feature = "test-fixtures")]
#[test]
fn test_dump_and_reapply_schlib() { ... }

// Property tests
#[cfg(feature = "proptest")]
proptest! {
    #[test]
    fn spec_roundtrip_idempotent(spec in arb_spec()) { ... }
}

// Plain unit tests (no feature flag)
#[test]
fn test_parse_simple_component() { ... }
```

## Regression Seeds

Property test failures are captured in:
```
crates/altium-format-ops/proptest-regressions/spec_proptest/
```

Failed seeds are minimized and committed with the fix.
