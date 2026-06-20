# Annotations

The `#[annotation(...)]` attribute attaches sync metadata to a block: a stable identity,
a stability flag, and an optional group. Annotations are what let the sync system match
spec blocks to Altium objects across edits without relying on names or positions.

Annotation parsing is in
[`src/parser.rs`](../../../crates/altium-format-spec/src/parser.rs) (`parse_annotation`),
the AST in [`src/ast.rs`](../../../crates/altium-format-spec/src/ast.rs)
(`BlockAnnotation`, `AnnotationKey`), and compilation/ID logic in
[`src/annotation.rs`](../../../crates/altium-format-spec/src/annotation.rs).

**Related pages**

- [Blocks overview](blocks-overview.md) — where annotations attach
- [Syntax § the `$` reference sigil](syntax.md) and the `#` / `Hash` token
- [Sync operations](../operations/sync.md)
- [Apply and plan](../operations/apply-and-plan.md)

## Syntax

```
#[annotation(key = value, key = value, ...)]
BLOCK
```

The prefix begins with a `Hash` token (`#`) followed by `[` — note this only lexes as a
`Hash` because `#` followed by six hex digits would instead be a color literal (see
[Syntax § colors](syntax.md#colors)). Keys are comma-separated `key = value` pairs; a
trailing comma and surrounding newlines are tolerated. An empty annotation `#[annotation()]`
is valid (it just auto-generates an ID).

```
#[annotation(id = "AB12CD34")] component R1 {}
#[annotation(id = "AB12CD34", stable = true, group = "power")] net VCC {}
#[annotation()] component R1 {}
```

An annotation may precede any block declaration; it is stored in that block's `annotation:
Option<Spanned<BlockAnnotation>>` field and compiled to a `CompiledAnnotation`.

## Predefined keys

Only a fixed set of keys is accepted (`AnnotationKey`). An unknown key is a **parse error**
(`unknown annotation key '…'`), by design:

> If arbitrary keys were allowed, a typo like `stabl = true` would be silently accepted and
> have no effect. With a predefined enum the parser rejects unknown keys immediately.
> — `src/ast.rs`

| Key | Type | Required | Description |
| --- | ---- | -------- | ----------- |
| `id` | string | No | The 8-character short ID. Auto-generated when omitted. |
| `stable` | boolean (`true`/`false`) | No | Defaults to `false`. When `true`, sync apply will not overwrite this block. |
| `group` | string | No | A group name for clustering related blocks. |
| `source_id` | string | No | The Altium `UNIQUE_ID` of the source component (opaque, not validated). |

The `id` and `group` values must be string literals; `stable` must be a boolean literal.
A value of the wrong kind is a parse error. (`group` is also a keyword token; the parser
accepts it as a key name specially.)

## Short-ID format and validation

An annotation ID is an Altium-style **8-character** string drawn from the alphabet
`[A-Z0-9]` (uppercase letters and digits only). Validation is `validate_short_id` in
`src/annotation.rs`:

- Exactly 8 characters — shorter or longer is rejected.
- Every character must be an ASCII uppercase letter or digit. Lowercase, mixed case, or
  punctuation is rejected.

```
AB12CD34   ZZZZZZZZ   00000000   A1B2C3D4   // valid
ab12cd34   AB12CD3    AB12!D34                // invalid
```

Domain-prefixed IDs like `FP000001`, `NET00001`, `BRD00001`, `PLY00001`, `RUL00001`,
`CLS00001` are valid because they are still 8 chars from the alphabet — the prefix is a
human convention, not enforced by the validator.

## Auto-generation vs manual IDs

If `id` is omitted, `compile_annotation` generates one with `generate_short_id()`: a random
8-character string from `[A-Z0-9]` (36⁸ ≈ 2.8 trillion combinations, negligible collision
risk at spec scale). A related deterministic helper, `generate_source_id(seed)`, derives a
stable ID by FNV-1a hashing a seed string, used to make sync idempotent.

```
#[annotation()]            net GND {}   // id auto-generated, e.g. "K7QW2M9P"
#[annotation(stable=true)] net GND {}   // stable flag set; id still auto-generated
```

## Duplicate-ID detection

`compile_annotation` takes a `seen_ids: HashSet<String>` and errors with
`DuplicateAnnotationId` if the same ID appears twice in one compile pass. This is a
two-layer design:

- The **compiler** check fast-fails on within-file duplicates during single-file
  compilation.
- The **validator** (Phase 3) performs the authoritative cross-file duplicate check for
  multi-file projects.

See the module docs in `src/annotation.rs`.

## The `stable` flag

`stable` controls overwrite behaviour during sync apply. When `true`, the executor will not
overwrite the block when reconciling the spec against an existing document — the block is
treated as a fixed, user-authored anchor. When `false` (the default), the block participates
in normal sync updates. See [Sync operations](../operations/sync.md) and
[Apply and plan](../operations/apply-and-plan.md).

## The `group` field

`group` is an optional free-form string used to cluster related blocks (for example,
grouping all power nets under `group = "power"`). It is carried through compilation on
`CompiledAnnotation.group` and consumed by the sync/reporting layers.

## Compiled form

The parsed `BlockAnnotation` (with optional spanned `id`, `stable`, `group`, `source_id`)
compiles to a `CompiledAnnotation`:

```rust
pub struct CompiledAnnotation {
    pub id: String,             // always present (generated if absent)
    pub stable: bool,           // defaults to false
    pub group: Option<String>,
    pub source_id: Option<String>,
}
```

This compiled annotation is attached to 15+ spec model types and is emitted before each
block by the `dump` command (`#[annotation(...)]` lines) so round-tripped specs retain
their stable identities.
