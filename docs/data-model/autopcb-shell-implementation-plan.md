# AutoPCB Shell Implementation Plan

## Purpose

This document translates the document-graph RFC into concrete work needed in `autopcb-shell`.

The goal is to get from the current shell model to a shell that natively hosts the canonical
design graph.

## Current Shell Constraints

Today the shell is centered around:

- board document with reduced PCB IR
- plain text spec document
- schematic preview documents
- schematic library gallery/component preview documents

This means the shell currently lacks:

- a native canonical graph in memory
- first-class logical documents
- first-class physical documents beyond reduced PCB IR
- first-class definition collections
- first-class assets and preservation nodes
- graph-native selections and inspector state

## Implementation Principle

Do not replace everything at once.

Add the canonical graph as a new host model first, then progressively move shell features from:

- file-path-driven views
- reduced IR-only views
- preview-only documents

to:

- graph scope views
- graph node inspection
- graph-native editing intents

## Phase 1: Graph Host In The Workbench

Add a new top-level shell-owned model:

- `DesignGraphDocument`
- or equivalent graph workspace state

Minimum responsibilities:

- load root graph bundle
- keep stable node IDs available to UI
- expose document scopes
- expose definitions and assets

This phase should not remove existing board/spec documents yet.

Result:

- shell can open a graph workspace alongside existing board/spec workflows

## Phase 2: New Document Kinds

Introduce graph-native document kinds for the shell.

Minimum set:

- `document.logical`
- `document.physical`
- `document.definition_collection`
- `document.asset_inspector`

These should be real workbench document kinds, not preview-only wrappers.

Result:

- the tab system can open graph scopes directly

## Phase 3: Graph-Native Selection Model

Replace path/string-based selection with graph-node-based selection.

Selection should be able to target:

- document node
- occurrence node
- definition node
- terminal node
- connection node
- geometry node
- constraint node
- asset node

Selection payload should use stable graph IDs, not only names.

Result:

- inspector and commands can operate on canonical objects

## Phase 4: Inspector Rewrite

The inspector should become a graph inspector.

It should show:

- core identity
- semantic type
- parent/child links
- definition/occurrence mapping
- connection membership
- asset/provenance metadata
- storage authority mode

Result:

- imported opaque content becomes inspectable even before it is fully editable

## Phase 5: Render Adapters

The shell should render from derived graph view models, not directly from import formats.

Needed adapters:

- logical document render adapter
- physical document render adapter
- definition preview adapter

These adapters may internally reuse reduced IRs, but the authoritative source must be the graph.

Result:

- rendering becomes independent from Altium-shaped runtime structures

## Phase 6: Command Pipeline Integration

The shell command pipeline should move to graph-native intents and transactions.

Examples:

- select node
- move occurrence
- update parameter
- attach asset
- convert opaque artifact to semantic geometry
- relink definition

This fits the existing shell intent/resolver/transaction architecture well, but the target state
must be the graph, not a preview file or reduced IR.

Result:

- editing becomes canonical-model-first

## Phase 7: Session And Workspace Persistence

Session restore should persist:

- active graph root
- open graph scopes
- selected node IDs
- UI state tied to graph scopes

It should not rely only on file paths to reconstructed preview documents.

Result:

- shell sessions become stable across design graph workspaces

## Phase 8: Legacy Workflow Migration

Once the graph host is stable:

- keep existing Altium import flows as graph-import entry points
- keep current reduced board IR as a derived adapter
- gradually retire preview-only document kinds

The current workflows should become compatibility surfaces, not the core architecture.

## Required Research Before Phase 1

The following must be nailed down first:

- canonical vocabulary
- identity/path semantics
- graph serialization bundle layout
- connectivity model
- artifact model

Without those, shell integration will just bake uncertainty into UI state and commands.

## Suggested Work Order

1. Finalize RFC terminology and identity rules
2. Add a graph crate with stable node IDs and edge types
3. Add shell workspace support for a graph root
4. Add graph-native document kinds and selection
5. Build graph inspector
6. Build logical and physical render adapters
7. Route commands through graph-native intents
8. Migrate sessions and workspace persistence

## Definition Of Ready For Shell Work

The model is ready for `autopcb-shell` implementation when all of the following exist:

- canonical node/edge vocabulary
- identity and instance-path rules
- storage authority modes
- artifact metadata schema
- graph bundle layout
- minimum logical and physical render adapters

Until then, shell implementation should stay exploratory rather than foundational.
