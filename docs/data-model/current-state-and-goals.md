# Current State And Goals

## Why This Exists

The current codebase has three different model layers, and none of them is the right canonical
home for a long-term design model:

- `altium-format`
  - good for reverse-engineered format parsing and writing
  - too tied to Altium storage, record layout, sidecars, and legacy semantics
- `autopcb-ir`
  - intentionally reduced and PCB-only
  - useful for placement/routing/analysis
  - not suitable as the canonical source of truth
- `autopcb-spec`
  - closest to a human-authored model
  - still organized around Altium file types and partial coverage

The replacement needs to be a clean-sheet canonical design graph.

## Current Gaps

### 1. The Model Is Still File-Type-Shaped

The current spec model is rooted in:

- `SchLib`
- `SchDoc`
- `PcbLib`
- `PcbDoc`
- `PrjPcb`

That is still a tool/file taxonomy, not a design taxonomy.

What we actually need is a model rooted in:

- reusable definitions
- document scopes
- occurrences
- connectivity
- geometry
- rules and classes
- assets
- import provenance

### 2. `autopcb-ir` Is Not A Canonical Model

`autopcb-ir` is deliberately simplified:

- PCB only
- no schematic or library IR
- arcs flattened into point lists
- rules partially typed
- layer stack reduced
- no preservation of import identity, sidecar state, or source structure

This is correct for a derived working IR, but wrong for a canonical design graph.

### 3. `autopcb-shell` Does Not Yet Host A Real Native Design Graph

`autopcb-shell` today is centered around:

- board document
- text spec document
- schematic preview
- schematic library gallery/component preview

That is enough for the current workflow, but not enough for:

- multi-document native projects
- native libraries as first-class graph nodes
- shared definitions across documents
- graph-level asset tracking
- hybrid text/binary storage
- import provenance and preservation policy

### 4. Import Fidelity Is Not Yet Expressed As A Model Concern

The current stack can import and dump many things, but import fidelity is mostly implicit.

The canonical model needs explicit representation for:

- stable imported identity
- ownership and hierarchy
- unresolved or opaque imported payloads
- binary attachments
- source references
- preservation policies

Without these, full-fidelity import remains an implementation detail instead of a guaranteed model property.

## Design Goals

### 1. Clean Break From Legacy Terminology

We should not let Altium's internal names define the model.

Instead of "SchLib/PcbLib/SchDoc/PcbDoc", the model should prefer terms like:

- `definition_collection`
- `logical_document`
- `physical_document`
- `part_definition`
- `package_definition`
- `occurrence`
- `terminal`
- `connection_set`
- `artifact`

Importers/exporters can translate between canonical terms and vendor terms.

### 2. Spec Language As The Authored Root

The spec language remains the user-facing authored format.

But it should evolve from "one file per vendor-shaped document" into:

- a design graph root file
- optional included text shards
- optional binary artifacts
- explicit graph-level imports and asset references

### 3. Full-Fidelity Import Without Full-Fidelity Legacy Modeling Everywhere

We do not want to model every piece of vendor cruft as first-class semantics.

We do want to preserve imported material with enough fidelity that we can:

- re-export safely
- inspect it later
- gradually replace opaque imported content with cleaner semantic forms

That means the canonical model needs both:

- semantic nodes
- preservation nodes

### 4. Explicit Definition / Occurrence Split

The model must distinguish between:

- what something is
- where and how it is used

This applies to:

- logical parts
- physical packages
- reusable blocks/modules
- sheets/boards and their instances

### 5. Cross-Domain Links Must Be First-Class

A modern design graph must directly represent:

- part definition to package definition mapping
- logical occurrence to physical occurrence mapping
- logical terminal to physical terminal mapping
- net/class/rule relationships across logical and physical domains

This must not be hidden in strings or import-time heuristics.

### 6. Text/Binary Hybrid Storage Must Be Intentional

Some content is naturally text-authored:

- metadata
- rules
- connectivity intent
- reusable definitions
- document structure

Some content is naturally binary or dense:

- very large routed copper sets
- imported legacy payloads
- images
- 3D models
- opaque vendor-specific blocks

The model must let both coexist cleanly.

## Outcome We Want

The end state is:

- import Altium designs into a modern canonical graph
- edit that graph through text-first spec files
- preserve large or legacy-heavy content through attached artifacts
- host the graph natively in `autopcb-shell`
- derive focused IRs from the graph for rendering, placement, routing, and analysis
