> **Related docs**: [ops-design.md](ops-design.md) | [ops-lang-spec.md](ops-lang-spec.md) | [schlib-ops.md](schlib-ops.md) | [schdoc-ops.md](schdoc-ops.md) | [ops-e2e-gaps.md](ops-e2e-gaps.md) | [ops-lang-checklist.md](ops-lang-checklist.md)

# Ops Language Implementation Checklist

This checklist tracks parser + pass-2 typechecking/compiler completeness against `docs/ops-lang-spec.md`.

## P0 (current sprint)

- [x] `edit` and `remove` are recognized in pass-2 and no longer fail as unknown ops.
- [x] Initial lowering for `edit $name { ... }` and `remove $name` (single op-result selector).
- [x] Add regression tests for the above behavior and explicit diagnostics for unsupported selector shapes.
- [ ] Support generic selector-based mutation lowering (`edit SELECTOR { ... }`, `remove SELECTOR`) beyond `$name`.
  - Target: lower to execution-ready high ops without losing selector semantics.
- [ ] Implement document/entity reference resolution for bare identifiers in expression context (e.g. `U1.location.x`).
- [~] Implement enum resolution by expected field type (case/underscore insensitive), per spec.
  - Implemented for `electrical` in pass-2 object field evaluation.
  - Remaining: generalize across all enum-typed fields as op coverage expands.
- [ ] Expand selector semantic validation table coverage to full SchDoc/SchLib fields and pseudo-classes.

## P1 (high-value completeness)

- [x] Add pass-2 lowering for remaining Sch create ops represented by `HighOp` variants (`add_line`, `add_rectangle`, `add_arc`, etc.).
- [ ] Add pass-2 lowering for spec create ops not yet in high model (`add_wire`, `add_net_label`, `add_power_port`, `add_junction`) or document explicit deferral.
- [x] Extend selector numeric handling to include signed numeric literals in attribute values.
- [ ] Reconcile part-pattern syntax across docs/query parser (`$part` vs `%part`) and enforce one canonical form.
- [ ] Add metamorphic/proptest suites specifically for selector semantics and mutation lowering equivalence.

## P2 (cross-domain + ergonomics)

- [ ] Add pass-2 domain support for `PcbDoc`/`PcbLib`.
- [ ] Add schema-driven diagnostics for all ops via derive metadata, minimizing hand-written field maps.
- [ ] Add golden diagnostics tests for top LLM failure modes (unknown fields, wrong types, unresolved refs, selector mistakes).
- [ ] Add a conformance matrix doc mapping each spec construct to parser/pass-2/executor support.

## Notes

- Current implementation intentionally fails fast with explicit `E2008` diagnostics for selector-based edits/removes not yet representable in a single-reference lowering path.
- This file is a living checklist; keep statuses updated in the same commit as feature work.
