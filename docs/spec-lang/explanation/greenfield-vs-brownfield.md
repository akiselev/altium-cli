# Greenfield vs brownfield: who is authoritative?

The spec language serves two fundamentally different workflows. Which one you are
in decides what `dump` / `compile` / `plan` / `apply` should *mean* — especially
how a SchDoc component's inline children (pins, graphics, parameters) are treated.
This is the single most important conceptual distinction in the spec pipeline, so
read this before changing the SchDoc executor, dump, or reconciler.

**Related pages:** [Design rationale](design-rationale.md) ·
[Annotations](../language/annotations.md) · [Introduction](../01-introduction.md)

## The two cases

### Greenfield — the spec is authoritative

The `*-spec` files own the design. They `import` and reference each other; a SchDoc
component is an *instance* of a symbol defined in an imported `*.schlib-spec`. The
Altium files are a generated artifact.

`dump` / `plan` / `apply` exist here as a **GUI escape hatch**: the user opens the
generated document in Altium Designer to do work the spec/agents cannot yet do well
— spatial reasoning, placement, routing — and then persists those edits *back into
the spec*. Because the spec is the source of truth, the round-trip that matters is:

```
spec → apply → (user edits in Altium GUI) → dump → spec   (deltas folded back in)
```

The hard problem in greenfield is **divergence tracking**. A materialized pin or
graphic is normally identical to its imported symbol template, so it should not be
re-inlined on dump — only the *delta the user introduced in the GUI* should be
captured (as an override on the instance). To compute that delta we need stable
**identity** linking each placed object back to the spec entity it came from
(see "Identity cascade" below). Because we own the Altium file in this mode, we may
embed our own typed identity metadata into it.

### Brownfield — the Altium files are authoritative

An existing Altium project is the source of truth. `dump` / `compile` / `apply`
exist so agents can read an existing project into spec text, make changes, and
persist them back to the Altium binaries. There is no library to resolve a
component against — the component's **inline children are the truth** and must
materialize verbatim and round-trip losslessly:

```
Altium doc → dump → spec → (agent edits) → apply → Altium doc   (lossless)
```

In brownfield we must **not** pollute the user's Altium file with our own metadata.
Identity for diffing comes from Altium-native fields (UniqueId) plus structural
matching only.

## How this resolves the inline-children fork

(Previously "Decision #1" in `docs/spec-problems.md`.) The earlier framing asked
whether to teach `apply` to materialize inline children (Option A), make `dump`
authoring-only (Option B), or treat inline children as advisory (Option C). The
two-case model resolves it: **it is both A and a greenfield override layer**,
selected per component by *whether the component resolves to an imported symbol*:

| Component in spec | Mode | Behavior |
| --- | --- | --- |
| No resolvable `symbol:` import (only inline children, or bare `lib_reference:`) | brownfield | Materialize inline children **verbatim**; lossless round-trip. Reuse the SchLib `ComponentChild` machinery (`schdoc_write.rs` already serializes children). |
| Resolvable `symbol: $lib.Name` import | greenfield | Reconstruct geometry from the resolved symbol as the **base**, apply inline children as **overrides** on top. On dump, emit only fields that diverge from the resolved template. |

This answers the old open question "who wins when both inline children *and* a
`symbol:` reference appear?" — **the import is the base, inline children are
overrides layered on top.**

### Immediate fail-fast fix (independent of mode)

Today inline `pin` / `graphic` / `part` / `footprint_map` blocks inside a SchDoc
component *parse* (`ast.rs` `ComponentItem`) but are **silently dropped at compile**
(`compiler.rs::compile_schdoc_component` only consumes `LetBinding`, `Property`,
`Parameter`, `PinConnection`). A silently dropped block is a fail-fast violation.
Until the materialization path above lands, unhandled inline children must be a
hard compile error, not a silent drop.

## Identity cascade (greenfield)

To map a placed object back to its spec entity for divergence tracking, resolve
identity in this precedence order — cheapest/least-invasive first:

1. **Native Altium UniqueId.** Components carry a `UniqueId`; use it first. No data
   written to the file. Works well for component-level identity.
2. **Embedded typed spec params.** When native identity is insufficient (e.g. pin-
   or graphic-level identity, or UniqueId churns across GUI edits), write our spec
   identity — the 8-char `#[annotation(...)]` IDs — as **reserved, fully-typed**
   parameters we understand end to end. Never opaque (CARDINAL RULE still applies:
   typed in/out, no raw retention). Only in greenfield, where we own the file.
3. **Structural match.** As a last resort, match by designator / position against
   the resolved symbol.

Start with (1); add (2) only where (1) demonstrably fails; (3) is the fallback.

## Open investigation — mode detection

**Unresolved:** how does the pipeline know it is in greenfield vs brownfield mode?
Two candidate mechanisms, both gated on an Altium round-trip question:

- **File-level marker.** Write a document-level metadata value marking the file
  "greenfield" (and possibly carrying the identity map). Requires that **Altium
  preserves our metadata across a GUI save** — otherwise the marker is lost the
  first time the user saves in Altium, exactly the round-trip greenfield depends on.
- **Inferred.** Derive mode per-component from whether `symbol:` resolves to an
  imported symbol. No file metadata needed, but more implicit.

**Investigation required before locking this in** (reverse-engineering task, see
CLAUDE.md "Reverse engineering Altium"): determine empirically and from the
decompiled source whether Altium Designer preserves, on a GUI save:

1. unknown **document-level header parameters**,
2. unknown **component parameters** (needed for identity cascade step 2),
3. unknown **CFB streams** (the sidecar alternative).

The answer decides whether the file-level marker is viable, whether embedded spec
params survive (identity cascade step 2), and ultimately whether mode can be a
durable file property or must be inferred. Default until resolved: **brownfield**
(never embed our metadata into a user's file unless they opted into greenfield).

## Two-sided change sets (plan / apply touch both files)

`plan` / `apply` must be able to report and write changes to **both** the source
spec *and* the destination document — not just the document. The motivating case is
greenfield identity: when a spec entity is first applied (or first linked back from
a GUI edit), the **source spec gains the linking annotation** that ties it to the
placed object, while the **destination document gains the actual design change**.

So a single `plan` produces a change set with two halves:

- **Source-side changes** — e.g. inserting/updating `#[annotation(<id>)]` lines so
  spec entities and placed objects stay linked (the spec-side carrier of the
  [identity cascade](#identity-cascade-greenfield) step 2), folding GUI-introduced
  deltas back into the spec as overrides, etc.
- **Destination-side changes** — the ECO against the Altium document (add / modify /
  remove primitives), as today.

Implications to design for:

- `plan` output should clearly separate "changes to `foo.schdoc-spec`" from
  "changes to `foo.SchDoc`" so the user sees both before approving.
- `apply` must write both atomically (or fail both): a destination edit whose
  linking annotation never made it back into the spec breaks the next round-trip.
- This generalizes the existing span-based spec rewriting used by `format` / `sync`
  (see [Design rationale](design-rationale.md) "Text-based rewriting") — annotation
  insertion is another targeted source rewrite.
- Brownfield rarely needs source-side writes (no identity to embed), but the
  mechanism is the same; keep it mode-aware rather than greenfield-only in code.
