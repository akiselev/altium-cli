# Identity And Instance Paths

Status: Draft

## Summary

The spec language already provides a strong source-level reference system:

- entity names
- bindings
- import aliases
- scoped `$path` references

Those should remain the primary way users and agents refer to things in source files.

They do not fully replace a canonical identity model in the data model itself.

The design graph therefore needs three separate layers of identity:

1. source references
2. semantic IDs
3. instance paths

## 1. Source References

Source references are what appear in spec files.

Examples:

- `component U1`
- `fp = footprint QFN48 { ... }`
- `import "common.parts.spec" as parts`
- `$parts.Regulator_3V3`
- `$u1.pin.vin`

These are for:

- human readability
- agent ergonomics
- local deterministic linking
- refactorable authored text

### Principle

Users and agents should normally reference objects in source by names, bindings, aliases, and paths.

They should not normally need to write UUIDs in source.

## 2. Semantic IDs

Semantic IDs are canonical graph IDs used by the runtime model.

They exist for:

- rename stability
- cross-document references
- imported opaque content
- unnamed/generated objects
- persistent UI/session selection
- asset attachments
- internal graph edges

Semantic IDs are not primarily a source syntax concern.

### Node Families That Need Semantic IDs

At minimum:

- `document_id`
- `definition_id`
- `occurrence_id`
- `terminal_id`
- `connection_id`
- `geometry_id`
- `constraint_id`
- `asset_id`
- `import_id`

## 3. Instance Paths

Instance paths identify where a reusable thing is instantiated in hierarchy.

This is required because:

- the same definition may be instantiated multiple times
- the same local binding names may appear in multiple instances
- path context determines the identity of an occurrence in a reused subtree

### Principle

Instance paths should be based on stable occurrence identity, not display names.

Names may be included as optional debug/display metadata, but they should not be the canonical basis.

## 4. Why Source References Alone Are Not Enough

Bindings and aliases solve source reference, but not all identity cases.

### 4.1 Rename Stability

If:

```text
component U1 { ... }
```

becomes:

```text
component U101 { ... }
```

we still need a way to represent that this is the same occurrence after rename.

### 4.2 Reused Hierarchy

If a reusable block contains:

```text
component U1 { ... }
```

and that block is instantiated twice, there are now two `U1` occurrences in different paths.

The binding/name is not enough to distinguish them globally.

### 4.3 Unnamed Or Generated Objects

Some objects may have no stable user-facing name:

- generated wire stubs
- auto-created junctions
- imported opaque payload anchors
- internal geometry nodes
- temporary analysis objects

These still need stable graph identity.

### 4.4 Imported Content

Imported vendor content may not map cleanly to source names.

We still need stable IDs to attach:

- provenance
- preservation policy
- asset references
- semantic upgrades over time

## 5. Recommended Model

### 5.1 Source References Are Primary In Files

Spec files should use:

- names
- bindings
- aliases
- `$path` references

as the default and primary authoring/reference mechanism.

### 5.2 Semantic IDs Exist In The Data Model

The canonical graph should always allocate semantic IDs internally.

For authored objects, those IDs should be derived from source structure where possible.

For imported or unnamed objects, IDs may come from import mapping or internal allocation.

### 5.3 Users Should Not Normally Need To Type UUIDs

UUID-like or opaque IDs should generally remain:

- internal
- debug-visible
- inspector-visible
- machine-stable

but not required in normal authoring.

## 6. Exposure Policy

### 6.1 Default Policy

Do not require UUIDs in source files for ordinary authoring.

Users and agents should be able to build designs using only:

- declarations
- names
- bindings
- aliases
- scoped references

More specifically:

- bindings should be the preferred authored reference mechanism whenever possible
- stringly-typed references should be avoided in the canonical authored model
- a rename should usually be a non-event because downstream references resolve through bindings

### 6.2 Optional Visibility

Semantic IDs may still be exposed in:

- debugging output
- inspector views
- diffs
- import diagnostics
- explicit advanced reference forms if ever needed

But they should be optional, not baseline syntax.

### 6.3 Exception Cases

There may be narrow cases where exposing stable internal IDs is useful:

- explicit preservation anchors for imported opaque artifacts
- advanced patching against imported content
- conflict resolution after large refactors
- pinning identity for binary-backed subtrees or payloads that are too large or unwieldy to keep in text

Even then, that should be treated as an advanced mechanism rather than the normal language.

## 7. Proposed Split: Derived Source Identity vs Internal Semantic Identity

Recommended rule:

- source-defined objects get semantic IDs derived from structural source identity
- the graph stores those semantic IDs canonically
- source references resolve onto those IDs
- renames preserve identity when the resolver can prove continuity

This gives:

- human-friendly source files
- stable runtime identity
- no need to expose opaque IDs in normal workflows

## 8. Rename Semantics

Decision:

- renaming a bound entity should preserve semantic identity by default
- creating a new entity with copied content should create a new semantic identity

That means identity follows semantic continuity, not raw spelling.

Operationally:

- if references target the entity through bindings and scoped references, rename should not change identity
- if a user intentionally duplicates/replaces a node rather than renaming it, that creates a new identity
- the authored language should be designed to minimize raw string references so rename safety is the default path

## 9. Definition And Occurrence Identity

Definitions and occurrences must always have separate identities.

Examples:

- a `part_definition` has its own `definition_id`
- each placement/use has its own `occurrence_id`
- an instance path identifies the occurrence's hierarchical context

This is non-negotiable if the model is to support reuse correctly.

## 10. Proposed Path Shape

Canonical instance paths should be occurrence-based.

Conceptually:

```text
/occ:<root-occurrence-id>/occ:<child-occurrence-id>/occ:<leaf-occurrence-id>
```

Human-readable labels may be attached for display:

```text
/power_train[occ:...]/regulator_stage[occ:...]/u1[occ:...]
```

But the occurrence IDs are canonical; the labels are presentation.

## 11. Recommended Decision

The current design direction is:

- source bindings, names, aliases, and `$path` references are the primary authoring identity
- semantic graph IDs are required internally
- instance paths are required for reused hierarchy
- UUID-like IDs should not normally be required in source files
- users/agents should only need them for advanced debugging, import preservation, or repair flows
- the language may later add explicit syntax to pin or expose internal IDs when needed for advanced preservation cases
- binary-backed content may need explicit stable IDs in its own storage format even when the text syntax does not expose them directly

## 12. Implications For The Spec Language

The spec language should remain friendly and text-first.

This means:

- keep the existing reference style as the main user-facing mechanism
- do not force UUID references into ordinary specs
- add advanced identity hooks only if truly necessary

Current direction for those hooks:

- normal authored references should resolve via bindings
- advanced syntax may exist to pin internal identity explicitly
- binary sidecar or binary artifact formats may carry stable IDs directly even when the text source does not

That keeps the language usable while still giving the underlying graph the rigor it needs.
