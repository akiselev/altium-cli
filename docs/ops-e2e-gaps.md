> **Related docs**: [ops-design.md](ops-design.md) | [ops-lang-spec.md](ops-lang-spec.md) | [schlib-ops.md](schlib-ops.md) | [schdoc-ops.md](schdoc-ops.md) | [ops-e2e-gaps.md](ops-e2e-gaps.md) | [ops-lang-checklist.md](ops-lang-checklist.md)

# Ops E2E Gap Analysis (Parser/Typecheck vs Runtime)

## Pipeline layers

1. `.ops` parser/typecheck emits `Vec<HighOp>` (`compile_ops_to_high_*`).
2. `HighOp -> ComposedOp` lowering.
3. `ComposedOp -> SchDocLowOp/SchLibLowOp` lowering.
4. `apply_*_low_ops` execution in `altium-format::sch_ops_core`.

## Critical gaps

1. SchDoc runtime does not support most ops that parser/typecheck now compiles.
   - Parser/typecheck accepts many ops: `edit/remove`, graphics, records, aliases, etc.
   - SchDoc executor rejects them with `"operation is not supported for SchDoc"`.
   - This means compile success does not imply runtime success for SchDoc.

2. Selector semantics mismatch between parser/typecheck and runtime query evaluator.
   - Parser/typecheck accepts rich selector AST + validation.
   - Runtime query implementation supports a tiny subset (`component`, `component[designator=...]`, and for SchLib also `component[lib_reference=...]`) with simple string extraction.
   - Valid selectors at parse/typecheck can still fail at execution with `"unsupported query selector"`.

3. Generic `edit/remove SELECTOR` is still not representable end-to-end.
   - Current pass-2 only lowers `edit/remove` when selector is a plain `$op_result` reference.
   - Rich selectors are rejected early by pass-2.

4. `.ops` E2E path is not wired into current `apply_*` API surface.
   - Existing executor tests are YAML/JSON spec -> `HighOp` -> apply.
   - No first-class helper yet for `.ops` source -> compile -> apply in one call.

## Layer coverage matrix

Legend: `Y` supported, `N` unsupported, `P` partial/restricted.

| Capability | Parser/Typecheck | High->Composed | Composed->Low | SchDoc runtime | SchLib runtime |
|---|---|---|---|---|---|
| `add_component` (+pins/footprint chain) | Y | Y | Y | Y | Y |
| `add_pin` | Y | Y | Y | Y | Y |
| `query` | Y | Y | Y | P (subset selectors) | P (subset selectors) |
| `add_parameter` | Y | Y | Y | N | Y |
| `add_alias/remove_alias` | Y | Y | Y | N | Y |
| `remove_component` | Y | Y | Y | N | Y |
| `edit_component` | Y | Y | Y | N | Y |
| `edit_record/remove_records/query_records` | Y | Y | Y | N | Y |
| `query_components/query_pins` | Y | Y | Y | N | Y |
| graphics/text/image create ops | Y | Y | Y | N | Y |
| `edit/remove` with generic selector | P (`$name` only) | Y | Y | N | N |

## What is good for E2E testing now

### SchDoc pass-now subset
- `add_component`, `add_pin`, `query` with runtime-supported selectors only:
  - `component`
  - `component[designator=...]`

### SchLib pass-now subset
- Most current `HighOp` variants are executable end-to-end.
- `query` still limited by runtime selector subset.

### Expected-fail E2E tests (valuable)
- Any SchDoc op in the unsupported list should fail with clear runtime error.
- Selectors accepted by parser/typecheck but outside runtime subset should fail at runtime with `"unsupported query selector"`.
- `edit/remove` with non-`$name` selectors should fail in pass-2 with explicit diagnostics.

## Immediate testing recommendation

1. Add `.ops`-source E2E helpers:
   - `apply_ops_source_schdoc(doc, source)`
   - `apply_ops_source_schlib(lib, source)`
   which call `compile_ops_to_high_*` then `apply_*`.
2. Add two E2E suites:
   - `ops_e2e_schdoc.rs` (small pass-now + expected-fail matrix)
   - `ops_e2e_schlib.rs` (broad pass-now coverage + selector-limit expected-fail)
3. Keep runtime-selector expected-fails explicit so parser/typecheck can evolve independently without masking executor gaps.
