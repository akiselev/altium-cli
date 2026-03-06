# Artifact Model

Status: Draft

## Summary

The canonical AutoPCB graph needs a first-class artifact model.

Artifacts exist because some design content is best represented as:

- authored text
- generated text
- structured binary
- opaque binary
- external referenced content

This document defines the role of artifacts in the graph and identifies the remaining
decisions required before implementation.

## 1. Why Artifacts Exist

Not all design content belongs directly in semantic graph nodes.

Examples:

- large routed copper payloads
- imported legacy binary structures
- images
- 3D models
- very dense geometry blocks
- source snapshots used for provenance or repair

The graph therefore needs explicit asset/artifact nodes rather than forcing everything into:

- text source
- semantic object trees

## 2. Relationship To The Canonical Graph

Artifacts are not the same thing as semantic nodes.

Semantic nodes capture:

- meaning
- relationships
- constraints
- hierarchy

Artifacts capture:

- storage payload
- binary or text data that may back semantic content
- imported preserved content
- attached resources

An artifact may:

- back a semantic node
- preserve an imported subtree
- serve as an external resource
- represent generated cache-like material

## 3. Artifact Categories

The current working categories are:

### 3.1 Authored Text

Text authored directly by users or agents.

Examples:

- spec shards
- reusable definition fragments
- explicit graph manifests

### 3.2 Generated Text

Text emitted by tools, but still inspectable and diffable.

Examples:

- generated normalized fragments
- dumped compatibility views
- generated documentation snapshots

### 3.3 Structured Binary

Binary with a declared schema and internal structure that AutoPCB understands.

Examples:

- packed routed-geometry payloads
- binary geometry chunks
- indexed realization payloads
- binary tables keyed by stable graph IDs

### 3.4 Opaque Binary

Binary preserved without semantic interpretation beyond metadata.

Examples:

- imported vendor payloads not yet normalized
- raw sidecar payloads
- unknown or version-ambiguous blocks

### 3.5 External Reference

Content referenced from outside the bundle.

Examples:

- external model files
- large shared assets
- optional external manufacturing attachments

## 4. Artifact Node Structure

Every artifact node should have at least:

- `artifact_id`
- `kind`
- `storage_mode`
- `media_type`
- `authority`
- `origin`
- `digest`
- `size`
- `path_or_locator`
- `owner_scope`

Optional fields:

- schema version
- compression
- encryption flag if ever needed
- imported source mapping
- stable local index tables

## 5. Ownership And Attachment

Artifacts should be attachable to any relevant node family.

Examples:

- document-level artifact
- definition-level artifact
- occurrence-level artifact
- geometry-level artifact
- import-level artifact

The attachment edge should be explicit:

- semantic node references artifact
- artifact may declare whether it is authoritative, advisory, preserved, or derived

## 6. Authority Modes

This is one of the most important parts of the artifact model.

Artifacts and semantic nodes can disagree or overlap, so we need explicit authority rules.

Current candidate authority modes:

- `semantic_authoritative`
  - graph semantics are the source of truth
  - artifact is derived or cached
- `artifact_authoritative`
  - artifact is the source of truth
  - semantic layer is partial, projected, or advisory
- `shared_authority`
  - specific fields/regions are owned by graph semantics, others by artifact
- `preserved_only`
  - artifact is retained for lossless export or inspection, but not edited semantically

## 7. Storage Modes

Current candidate storage modes:

- `authored_text`
- `generated_text`
- `structured_binary`
- `opaque_binary`
- `external_reference`

This aligns with the RFC and should likely remain the primary storage-mode vocabulary.

## 8. Binary-Backed Large Content

The motivating case is large routed or hand-authored physical content that is too unwieldy
to keep in main text files.

The graph should support:

- semantic attachment of a large physical realization block
- optional partial semantic indexing into that block
- stable IDs within the binary payload
- explicit pinning from graph nodes to binary content

This is especially important for:

- traces
- vias
- dense copper regions
- large routed sections
- imported vendor realization content

## 9. Identity Inside Artifacts

Artifacts need internal stable identity too.

This follows directly from previous decisions:

- text files should not require opaque IDs for normal authoring
- but binary payloads can and likely should store stable IDs directly

That means a structured binary artifact may contain:

- stable graph node IDs
- stable local payload IDs
- explicit pinning metadata
- cross-index tables

Opaque binary artifacts may only have coarse attachment metadata at first.

## 10. Import Preservation

Artifacts are the main tool for high-fidelity import without polluting the canonical graph
with vendor internals.

Importers should be able to:

- preserve raw vendor payloads as opaque artifacts
- attach those artifacts to canonical semantic nodes
- progressively replace opaque content with structured binary or semantic graph content later

This is the core mechanism that lets us import legacy systems cleanly without becoming them.

## 11. Artifact Families We Already Know We Need

Minimum likely families:

- `graph_text_fragment`
- `geometry_chunk`
- `routing_chunk`
- `image_asset`
- `model_asset`
- `import_snapshot`
- `vendor_preservation_blob`
- `binary_index_table`

These names are provisional, but the categories are real.

## 12. Bundle Layout Direction

The likely bundle structure remains:

```text
design-root.design-spec
design-root.design-spec.d/
  manifest.toml
  docs/
  defs/
  assets/
  map/
```

Artifacts most naturally live under `assets/`.

Likely subdivisions:

- `assets/text/`
- `assets/binary/`
- `assets/models/`
- `assets/import/`

This is still a layout choice, not yet a final storage contract.

## 13. Relationship To Import Sources

Import nodes and artifact nodes are related but distinct.

An import node records:

- source provenance
- mapping policy
- source file set

An artifact node stores:

- actual payload or resource reference

One import may produce many artifacts.
One artifact may be owned by one import but referenced by many semantic nodes.

## 14. What The Model Should Avoid

The artifact model should not:

- force users to hand-author binary IDs in ordinary source
- treat vendor raw payloads as semantic objects unless they actually are
- hide authority conflicts
- rely only on path strings without stable IDs
- assume every artifact is portable text

## 15. Open Decisions

The following decisions are now recorded.

### 15.1 Artifact Addressing

Decision:

- artifacts are addressed by both stable graph IDs and content digests

Rationale:

- graph IDs provide stable internal references
- digests provide integrity and dedup support

### 15.2 External References

Decision:

- external references are allowed depending on artifact kind

Rationale:

- some assets are naturally external and shared
- others should normally be internalized for portability or preservation

### 15.3 Shared Authority

Decision:

- no true shared authority in v1

Rationale:

- one clear authority per attachment is easier to reason about
- shared authority introduces too much ambiguity too early

### 15.4 Structured Binary Normalization

Decision:

- the first structured binary format should be native AutoPCB format

Rationale:

- avoids freezing importer-specific baggage into the core system
- gives the graph a clean native binary realization format from the start

### 15.5 Cross-Artifact References

Decision:

- artifacts may reference other artifacts directly

Rationale:

- some payloads naturally decompose into layered or chained resources
- forcing all artifact relationships through semantic nodes would be unnecessarily rigid

## 16. Current Working Direction

The working direction is:

- artifacts are first-class graph nodes
- text-first authoring remains the default
- binary-backed content is explicitly supported
- stable internal IDs may appear inside binary payloads even when not exposed in normal source
- imported vendor content should often begin as preserved artifacts rather than fake semantic nodes
- artifacts use both graph IDs and content digests
- external references are allowed where appropriate
- v1 should use one clear authority per attachment
- the first structured binary format should be native AutoPCB
- artifacts may reference other artifacts directly

## 17. Immediate Next Step

Before this document is finalized, the remaining open decisions above should be resolved.

After that, the next shell-facing design step is:

- `shell-host-integration.md`

because the shell needs to know how graph nodes and artifact nodes are opened, inspected, and edited.

## 18. Rust Crate Options

This section is not a final dependency decision. It is a shortlist of implementation candidates.

### 18.1 Hashing And Content Digests

Likely candidates:

- `blake3`
- `sha2`

Current recommendation:

- use `blake3` as the primary fast content digest
- optionally support `sha2` for interoperability where needed

Notes:

- `blake3` is well-supported and efficient for large artifact hashing ([docs.rs](https://docs.rs/blake3/))

### 18.2 Stable Internal IDs

Likely candidates:

- `uuid`
- custom newtype IDs over internal numeric keys

Current recommendation:

- use typed internal ID newtypes for graph/runtime keys
- use `uuid` only where globally stable or externally visible identifiers are needed

### 18.3 Graph Storage

Likely candidates:

- `slotmap`
- `petgraph`
- `indexmap`

Current recommendation:

- `slotmap` or a slotmap-style arena for stable node handles
- `indexmap` for deterministic ordered maps in manifests and serialization
- `petgraph` only if we want off-the-shelf traversal algorithms rather than a domain-specific graph layer

Notes:

- `slotmap` remains a strong fit for stable generational handles ([docs.rs](https://docs.rs/crate/slotmap/latest/source/))
- `petgraph` is mature but may be more generic than we need ([GitHub](https://github.com/petgraph/petgraph))

### 18.4 Binary Parsing And Writing

Likely candidates:

- `binrw`
- `zerocopy`
- `deku`

Current recommendation:

- `binrw` for explicit binary file parsing/writing where clarity matters
- `zerocopy` for packed/native AutoPCB binary payloads where layout control and low-copy access matter

Notes:

- `binrw` is a strong fit for external binary format adapters ([docs.rs](https://docs.rs/crate/binrw/latest))
- `zerocopy` is a strong fit for internal structured binary blocks, but only where layout discipline is acceptable

### 18.5 Native Structured Binary Serialization

Likely candidates:

- `postcard`
- `rkyv`
- `ciborium`
- `prost`
- `capnp`

Current recommendation:

- do not commit yet
- evaluate two likely directions:
  - `postcard` or a custom compact format for small/portable structured payloads
  - `rkyv` or a custom indexed binary format for large native realization payloads

Reason:

- the native AutoPCB binary artifact format has not been designed yet
- crate choice should follow access patterns, partial loading requirements, and upgrade strategy

### 18.6 Manifest And Path Handling

Likely candidates:

- `toml_edit`
- `camino`
- `serde`
- `serde_bytes`

Current recommendation:

- `toml_edit` for editable manifests
- `camino` for UTF-8 path handling in bundle code
- `serde` as the common baseline
- `serde_bytes` where binary payload metadata or compact byte serialization is useful

### 18.7 Memory Mapping And Large Payloads

Likely candidate:

- `memmap2`

Current recommendation:

- keep `memmap2` in scope for very large binary artifact reading
- do not require mmap in the first implementation unless profiling shows it is needed

### 18.8 Proposed Bias

If implementation started today, the pragmatic shortlist would be:

- `slotmap`
- `indexmap`
- `uuid`
- `blake3`
- `binrw`
- `zerocopy`
- `toml_edit`
- `camino`
- `serde`
- `serde_bytes`

The serialization crate for native structured binary payloads should remain undecided until the
binary artifact layout is specified.
