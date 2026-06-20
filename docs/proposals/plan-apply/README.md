# Plan/Apply Redesign Notes

> **Proposal snapshot (2026-06-18; not implemented CLI documentation).** The current
> command surface remains `altium plan`, `altium apply`, and `altium dump`. Use
> [`../../spec-lang/operations/cli.md`](../../spec-lang/operations/cli.md) for current behavior.

Investigation (2026-06-18) into moving the spec pipeline to a **Terraform-style,
bidirectional model**: per document type, every conversion direction has a
`plan` (emits an Engineering Change Order) and an `apply` (executes it). Includes
resolution of the June 2026 spec roundtrip worklog's Problem #1 (SchDoc inline children) under
this framing.

## The model in one picture

Both the Altium document and the `-spec` file are editable artifacts (humans edit
geometry in Altium's GUI; LLM agents edit the textual spec), so there are **two**
plan/apply pairs per document type:

```
            compile plan / compile apply   (spec → doc)
   -spec  ──────────────────────────────────────────────►  document
 (agent edits)  ◄──────────────────────────────────────  (human edits in Altium)
              dump plan / dump apply        (doc → spec)
```

- **`compile`** = spec → doc (this **replaces the old root `apply`** command).
- **`dump`** = doc → spec (now a `plan`/`apply` pair; `dump apply` **merges into**
  the existing spec, it never overwrites it).
- `plan` previews the change to whatever `apply` will write, as an ECO.

| Note | Contents |
| ---- | -------- |
| `00-current-state.md` | How the pipeline works today; the plan/apply two-engine drift bug; ECO model; the missing Delete and ad-hoc identity; per-domain asymmetry |
| `01-terraform-model.md` | Concept-by-concept mapping of Terraform onto altium-cli; plan==apply principle; `ResourceAddress` (identity); why no state file in v1; full action set; bidirectional divergence |
| `02-problem-1-inline-children.md` | "Read schema == write schema" invariant forces Option A (materialize inline children); two-sources-of-truth resolution; fail-loud scaffold path |
| `03-proposed-cli.md` | CLI: per-type command groups, `compile {plan,apply}` + `dump {plan,apply}`, `plan --out` / `apply --plan`, exit codes, delete policy/flags |
| `04-roadmap.md` | Five-phase incremental roadmap, dependency graph, open questions needing a user decision |
| `05-bidirectional.md` | The authoritative bidirectional model: both artifacts editable; `dump apply` merges into the existing spec; stable annotation IDs as the two-way binding; conflict handling |
| `07-open-questions-deep-dive.md` | In-depth treatment of note 06's four open questions (choose `M`, identity, generic `reconcile`, CFB-verify scope) with evidence, options, tradeoffs, and how they interact — incl. the sleeper "partial specs die when `M` goes concrete" |
| `06-architecture.md` | **Rust types & the two ECOs.** The semantic ECO is computed on a *common Rust model* (spec & document are just serializations) in every direction — no text/byte diffs. The CFB `DiffIssue` diff is a *separate, parallel, agent-facing* artifact for verifying file-format support (not nested under the ECO). Which Rust model `M` is the open decision (high-level API model recommended); `Provider` trait + generic `reconcile`; enabling additions |

## TL;DR

- **Bidirectional:** each direction is gated by plan/apply — `compile` (spec→doc)
  and `dump` (doc→spec). `dump apply` *merges into* the existing spec (preserving
  annotation IDs + comments), never regenerates it. The old one-shot dump is gone.
- **Core win is plan==apply:** today `plan` (reconciler.rs) and the write path
  (executor.rs) are *separate engines* that must be hand-kept in sync and have
  already drifted (spec-problems fix #19). Route `apply` through the ECO so the
  plan *is* the instruction set — in both directions.
- **Three things are missing for a real provider:** (1) a uniform resource
  identity/address (`ResourceAddress`), (2) `Delete`/`Replace` in the ECO,
  (3) read-schema == write-schema (= Problem #1). The address and schema-symmetry
  work serve both directions.
- **Problem #1 is resolved by the framing:** under "read schema must equal write
  schema," only Option A (teach `compile` to materialize inline children)
  survives; B/C keep two schemas. With bidirectional editing this is upgraded from
  "should" to a hard correctness prerequisite (else the doc→spec→doc loop leaks).
  Build it incrementally behind a fail-loud scaffold.
- **No state file in v1:** the document and spec are fully readable, so we refresh
  every run; stable annotation IDs (ID↔UniqueId) are the durable two-way binding.
  A small last-synced baseline is only needed later for concurrent two-author
  editing (note 05 §4).

Start with roadmap Phase 1 (`ResourceAddress`, route `compile apply` through the
ECO, `Delete` type, the `compile`/`dump` CLI rename) — pure-internal plus a CLI
rename, unlocks everything, kills the worst latent bug at zero behavioral risk.
