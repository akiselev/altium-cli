# Plan/Apply Redesign — Note 03: Proposed per-document-type CLI

**Session date:** 2026-06-18 (rev. for bidirectional model + `compile` rename)

Concrete CLI proposal for the Terraform-style **bidirectional** model. Builds on
notes `00`–`02` and `05`. The model: *both* the Altium document and the `-spec`
file are editable artifacts (humans edit geometry in Altium's GUI, LLM agents
edit the textual spec), and **each direction** has its own plan/apply review
gate.

The two directions and their verbs:

| Verb | Direction | `plan` previews a diff of… | `apply` writes… |
| ---- | --------- | -------------------------- | --------------- |
| **`compile`** | spec → doc | the **document** (ECO over Altium objects) | the `.{type}` doc |
| **`dump`**    | doc → spec | the **`-spec`** (ECO over spec blocks)     | the `.{type}-spec` |

`compile` replaces the old root `apply` command. Mnemonic: you *compile* a spec
into a document; you *dump* a document into a spec. Each is a `plan`/`apply` pair.

There is no from-scratch, overwrite-the-spec `dump` anymore — `dump apply` always
reconciles against (and merges into) the existing spec (see note 05 §2). A
first-time dump is simply the degenerate case where the existing spec is empty.

---

## 1. Command shape

**Per-document-type command groups, each with two plan/apply pairs:**

```
# spec → doc
altium schdoc compile plan  <spec> [--target doc]                # ECO over the .SchDoc
altium schdoc compile apply <spec> [--target doc] [-o out] [--plan eco.json]

# doc → spec
altium schdoc dump plan     <doc>  --spec <spec>                 # ECO over the .schdoc-spec
altium schdoc dump apply    <doc>  --spec <spec> [-o out] [--plan eco.json]

# same for: pcbdoc, schlib, pcblib, prjpcb
altium pcbdoc  compile plan|apply  /  pcbdoc  dump plan|apply
altium schlib  compile plan|apply  /  schlib  dump plan|apply
…
```

- **Surface:** five per-type groups (`schlib/schdoc/pcblib/pcbdoc/prjpcb`), each
  exposing `compile {plan,apply}` and `dump {plan,apply}`. Matches the user's
  request and the existing `cfb` subcommand-group style; gives each type a home
  for type-specific flags (e.g. `pcbdoc compile apply --instantiate-footprints`).
- **Engine:** one generic core keyed by `SpecDomain`. The domain is also
  inferable from the file extension (`detect_spec_domain`), so the per-type
  groups are thin wrappers (a macro over the `SpecDomain` enum) over a single
  implementation — no five real code paths. Keeps the "one engine" principle
  (note 01 §2).

`validate`, `query`, `info`, `render`, `new`, `cfb`, `format` are unchanged.

---

## 2. `plan` (both directions) — the ECO output

`plan` runs `reconcile()` and renders; it never writes. The ECO type (`eco.rs`)
is shared by both directions; the renderer labels the **target artifact** in the
header ("Target: <doc>" for `compile plan`, "Target: <spec>" for `dump plan`).

Shared additions over today's reconciler/ECO:

- **`Delete`/`Replace`** rows (note 01 §5). Example renders:
  ```
  + ADD     Track  "(TopLayer, 100mil,200mil → 300mil,200mil)"
  ~ UPDATE  Pad    "U1.3"        hole_shape: round → slot
  - DELETE  Track  "(BotLayer, 0,0 → 50mil,0)"
  ± REPLACE Pad    "U1.5"        (identity changed)
  ```
- **Exit codes (Terraform `-detailed-exitcode` convention):**
  `0` = converged (no changes), `2` = drift (changes present), `1` = error.
  (Replaces today's exit-1-on-drift; disambiguates drift from crashes. Breaking
  change to current `plan` — call out in migration.)
- **`--out <eco.json>`**: write the ECO as JSON (already `Serialize`) so the
  matching `apply --plan <eco.json>` executes the reviewed artifact (note 01 §2).
  This is what makes plan==apply literal.
- **`--json`**: machine-readable ECO to stdout.

### Direction-specific reconcile

- **`compile plan`** = `reconcile(spec, doc)` → ECO over **document entities**.
  No `--target` → the default output path for the spec; if it doesn't exist, the
  ECO is all-`Add` (create from scratch).
- **`dump plan`** = `reconcile(doc, existing-spec)` → ECO over **spec blocks**
  (which blocks/props would change in the `-spec`). `--spec` is required; if it
  points at a non-existent file the ECO is all-`Add` (first dump).

Both share the `ResourceAddress` identity layer (note 01 §3) so unchanged
entities render as `Unchanged`/no-op in either direction (fixes the PcbDoc
false-ADD problem, note 00 §3).

---

## 3. `apply` (both directions) — execute the plan

Two modes, identical for `compile` and `dump`:

1. `… apply <args>` — recompute the ECO, then `execute_eco`.
2. `… apply --plan <eco.json>` — execute a previously saved ECO, after a
   **staleness check**: re-reconcile and confirm the change set still matches the
   current target; if not, error "saved plan is stale, re-run plan" (Terraform
   refuses stale plans).

Both directions route through the ECO (note 01 §2): apply is `reconcile →
execute_eco → write`. There is no separate whole-model walker; `executor.rs`
becomes per-`EntityChange` appliers. This permanently kills plan/apply drift
(note 00 §2).

- **`compile apply`** writes the `.{type}` document. `execute_eco` mutates
  document objects.
- **`dump apply`** writes the `.{type}-spec`. `execute_eco` mutates **spec
  blocks**, *merging into the existing spec* — preserving annotation IDs,
  comments (via `trivia.rs`), block ordering, and spec-only constructs
  (`symbol:` refs, `pin -> #NET`). See note 05 §2. Unchanged blocks are
  byte-identical (no churn); annotation IDs are stable, not regenerated (note
  05 §2 — this retires the spec-problems §2 annotation-strip hack).

---

## 4. Authoritative scope, partial specs, and conflicts

**Delete policy** (note 01 §5). Some specs/docs are partial, so `Delete` needs a
guard:

- **Default:** authoritative within *managed kinds* — entities present in the
  target but absent from the source are `Delete`d.
- **`--no-delete` / `--append`:** never delete (today's implicit behavior;
  `spec sync` already has `--append`). For merging multiple sheets into one
  PcbDoc.
- **Unmanaged sections fail loud, never silent:** a section the source schema
  can't yet represent (e.g. PcbDoc layer stack pre-implementation) is marked
  `Unmanaged`; apply **preserves** it (never deletes what it can't express) and
  plan reports it so the user isn't surprised (note 02 fail-loud principle).

**Conflict detection (both sides edited)** — note 05 §4. Each `apply`
re-reconciles; if the *other* artifact has drifted from the last synced baseline,
warn and refuse ("`.SchDoc` has changes not reflected in the spec — run
`schdoc dump plan` first"). Safe concurrent two-author editing needs a
last-synced baseline sidecar (open decision, note 05 §6); single-author works
without it.

- **`-target <address>`** (later): scope plan/apply to one resource address.

---

## 5. Summary of CLI deltas

| Item | Change |
| ---- | ------ |
| Root `apply` | **Renamed to `compile`**; gains `plan`/`apply` subcommands |
| Root `dump`  | Gains `plan`/`apply` subcommands; `dump apply` merges into existing spec (no overwrite) |
| Root `plan`  | Removed as a top-level verb — folded into `compile plan` / `dump plan` |
| Command groups | `schlib/schdoc/pcblib/pcbdoc/prjpcb`, each with `compile {plan,apply}` + `dump {plan,apply}` (thin wrappers over one engine) |
| `plan --out` / `apply --plan` | Saved-ECO review→execute, both directions |
| `plan` exit codes | `0` converged / `2` drift / `1` error |
| `apply` engine | Route through `execute_eco`; retire reconciler/executor split |
| `--no-delete` / `--append` | Delete-policy guard |
| ECO render | Add Delete/Replace/Unmanaged rows; label target artifact (doc vs spec) |
| Identity | Shared `ResourceAddress` (annotation-id → unique-id → natural/structural key) used by both directions |

Next: note `04` sequences this into an incremental roadmap; note `05` is the
authoritative bidirectional model.
