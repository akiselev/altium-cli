# Shell Host Integration

## Purpose

This document translates the canonical document graph into a concrete host model for
`autopcb-shell`.

The goal is to make the shell a native host for the design graph without breaking the current
intent, resolver, command transaction, session, and tab architecture all at once.

This is not a UI mockup document.
It is an application architecture document for how the shell should own, expose, mutate, persist,
and render graph-backed design state.

## Scope

This document covers:

- in-memory shell host model
- graph-backed workbench documents
- graph-backed tab references
- graph-native selection and inspector state
- command pipeline integration
- session and workspace persistence
- render adapter boundaries
- phased migration from the current shell

This document does not define:

- the final graph crate schema in full
- the final geometry schema
- the final structured binary artifact format
- detailed UI layout changes

## Current Shell Constraints

Today `autopcb-shell` is still centered around file-shaped documents:

- `Board`
- `Spec`
- `SchDocPreview`
- `SchLibGallery`
- `SchLibComponent`
- `Keybindings`

The current workbench state in
[`crates/autopcb-shell/src/workbench.rs`](/home/kiselev/git/altium-cli-simplified/crates/autopcb-shell/src/workbench.rs)
is document-tab oriented and largely file-path driven.

The current project graph in
[`crates/autopcb-shell/src/project_graph.rs`](/home/kiselev/git/altium-cli-simplified/crates/autopcb-shell/src/project_graph.rs)
is also file/project shaped:

- board docs
- schematic docs
- spec docs
- file-path edges between them

The current session model in
[`crates/autopcb-shell/src/session.rs`](/home/kiselev/git/altium-cli-simplified/crates/autopcb-shell/src/session.rs)
persists:

- board paths
- spec paths or untitled IDs
- keybindings
- board-centric selection

The current command architecture in
[`crates/autopcb-shell/src/pipeline.rs`](/home/kiselev/git/altium-cli-simplified/crates/autopcb-shell/src/pipeline.rs)
is already structurally good:

- typed intents
- pure-ish resolution with `ResolveContext`
- explicit `CommandTransaction`
- atomic `apply_command(...)`

That pipeline should be preserved.
The main change is what state the pipeline targets.

## Integration Principles

### 1. Keep The Existing Command Architecture

The shell should continue to follow:

`Intent -> resolve_intent(...) -> CommandTransaction -> apply_command(...)`

Graph integration must fit that flow.
Do not bypass the resolver and do not let UI code mutate graph state directly.

### 2. Add The Graph As A New Host Layer

Do not replace existing board/spec documents immediately.

First add a graph-native workspace host next to the current file-shaped host model.
Then migrate tabs, selection, inspector, and commands gradually.

### 3. Make Graph Scope The Unit Of Editing

The shell should stop thinking in terms of vendor file types and start thinking in terms of graph
scopes:

- logical document scope
- physical document scope
- definition collection scope
- asset scope
- import scope

Tabs, selections, inspectors, and commands should target those scopes.

### 4. Persist Stable Graph References

Runtime-only `DocumentId` values are not enough.

Sessions and tab restore must persist references to:

- graph root
- scope identity
- selected node identity
- optional instance path

### 5. Derived Views Stay Derived

The graph is authoritative.

Render adapters, reduced IRs, search indexes, and import caches are all derived layers owned by the
host, not alternate canonical state stores.

## Target Host Model

The shell should gain a graph-native workspace model that sits alongside existing UI/session state.

Suggested shape:

```text
ShellHostModel
  workbench_ui_state
  active_workspace_ref?
  graph_host?
  open_tabs[]
  active_tab?
  secondary_tab?
  selection
  inspector_state
  derived_views
  problems
  jobs
```

Suggested graph-owned portion:

```text
GraphHost
  root_ref
  graph_store
  open_scopes[]
  asset_store
  import_registry
  derived_render_cache
  search_index
  dirty_state
```

Minimum responsibilities of `GraphHost`:

- load a graph bundle from spec-root plus attached artifacts
- keep canonical node IDs and scope IDs stable in memory
- expose graph scopes that can be opened in tabs
- provide node lookup for selection, inspector, and commands
- manage dirty tracking at graph and scope levels
- manage derived caches for rendering and indexing
- resolve asset references and artifact authority

## Graph Root And Workspace Identity

The shell should treat a design graph root as the primary workspace identity.

Suggested concepts:

- `WorkspaceRef`
  - stable reference to the opened design root
- `GraphRootRef`
  - root spec file or bundle entry
- `ScopeRef`
  - stable reference to an openable graph scope
- `NodeRef`
  - stable reference to a graph node

These should be separate concepts.

Do not collapse:

- workspace identity
- scope identity
- node identity
- tab identity

into one path or one runtime integer.

## Graph-Backed Workbench Documents

The shell should evolve away from vendor/file-shaped document kinds and toward graph scope
documents.

Recommended graph-native document kinds:

- `document.design_overview`
- `document.logical`
- `document.physical`
- `document.definition_collection`
- `document.asset`
- `document.import`
- `document.search_results`

These are shell document kinds, not graph node kinds.
They are tab/view wrappers around graph scopes or graph queries.

Suggested payload shape:

```text
DocumentKind::GraphScope {
  workspace: WorkspaceRef,
  scope: ScopeRef,
  presentation: ScopePresentation,
}

DocumentKind::GraphAsset {
  workspace: WorkspaceRef,
  asset: AssetRef,
}

DocumentKind::GraphImport {
  workspace: WorkspaceRef,
  import: ImportRef,
}
```

This lets the shell keep its tab/document model while making the underlying identity graph-native.

## Scope Model

The host needs a small number of explicit openable scope kinds.

Recommended scope kinds for v1:

- `design`
- `logical_document`
- `physical_document`
- `definition_collection`
- `part_definition`
- `package_definition`
- `block_definition`
- `asset_group`
- `import_group`

Important constraint:

Openable scope does not mean canonical node kind.
Some scopes are views over multiple nodes.

Example:

- a `logical_document` tab may open one document node
- a `definition_collection` tab may open a definition collection node
- a `part_definition` tab may open one part plus linked symbol/package summaries
- an `asset_group` tab may open a filtered asset query rather than one node

## Tab Identity And Session References

The current session model persists path-oriented `SessionTabRef` values.
That is not enough once one root design can expose many openable scopes and many non-file-backed
tabs.

The shell should move toward a graph-backed session tab reference model.

Recommended direction:

```text
SessionTabRefVNext
  DesignOverview { workspace, scope }
  LogicalScope { workspace, scope }
  PhysicalScope { workspace, scope }
  DefinitionScope { workspace, scope }
  AssetScope { workspace, asset }
  ImportScope { workspace, import }
  SpecText { path | untitled_id }
  Keybindings
```

Important details:

- persist graph scopes by stable graph references, not runtime `DocumentId`
- allow spec text tabs to continue existing during migration
- keep tab identity independent from current selection
- keep active and secondary tabs valid even if some scopes fail to restore

If a scope cannot be restored:

- do not panic
- surface a problem entry
- keep the workspace open
- prune only the invalid tab reference

## Selection Model

The current selection model is too board-specific:

- `Component(String)`
- `Net(String)`
- `Pad { component, pad }`
- `Rule(String)`

The shell needs a graph-native selection envelope.

Recommended direction:

```text
SelectionState
  primary: SelectionTarget
  secondary: Vec<SelectionTarget>
  locked: bool

SelectionTarget
  None
  Node(NodeRef)
  Terminal(TerminalRef, InstancePath?)
  Connection(ConnectionRef, InstancePath?)
  Geometry(GeometryRef)
  Asset(AssetRef)
  Scope(ScopeRef)
```

Key points:

- selection must be graph-ID-based
- optional instance path is required when selecting reused content in context
- selection should distinguish semantic node selection from rendered geometry selection
- cross-probing should target canonical node references, not names

String names may still be used for user-visible search and command input, but they should resolve
to stable node refs before any command is applied.

## Inspector Integration

The inspector should become a graph inspector with a small number of standard panels.

Recommended inspector sections:

- identity
  - node kind
  - stable semantic ID
  - instance path if applicable
  - authored binding/path if available
- containment
  - parent scope
  - child summaries
- mapping
  - definition to occurrence mapping
  - logical to physical mapping
  - terminal mapping
- connectivity
  - direct connection memberships
  - scoped global memberships
  - bus/bundle/differential-pair relationships
- artifacts
  - attached artifacts
  - authority mode
  - digest
  - storage kind
- provenance
  - import source
  - semantic coverage status

This is important because imported opaque content must still be inspectable even before it is
editable semantically.

## Render Adapter Boundary

The shell should not render directly from importer-native objects or from raw artifact payloads.

It should render from derived graph view models.

Recommended derived adapters:

- `LogicalRenderModel`
- `PhysicalRenderModel`
- `DefinitionPreviewModel`
- `AssetPreviewModel`

The graph host is responsible for building and caching these.

Important constraint:

- geometry and copper can be binary-backed
- rendering code should not need to know whether the source content came from text or binary
- the derived adapter boundary hides that storage distinction

This is where current reduced models such as `PcbIr` can temporarily survive.
They should be treated as derived adapters for physical scopes, not as the host's authoritative
document model.

## Command Pipeline Integration

The existing resolver/transaction model is already the correct shape.
What changes is the command vocabulary and command target state.

### Resolver Expectations

`ResolveContext` will need graph-aware context such as:

- active workspace ref
- active graph scope kind
- current selection target kind
- whether selected nodes are editable
- whether selected content is semantic, structured-binary-backed, or opaque
- whether selected nodes are in a reusable definition or an occurrence context

### Command Target Principles

Each command should mutate one coherent part of graph-backed state.

Examples:

- select node
- open scope
- rename binding
- update parameter
- rebind occurrence to definition
- attach artifact
- change artifact authority
- create scoped global
- add terminal to connection
- move physical occurrence

Examples of things that should stay out of direct execution code:

- string command parsing
- ad hoc graph traversal from UI event handlers
- geometry mutation logic hidden inside render code

### Transactions

Transactions remain the right place for one-to-many decompositions.

Examples:

- deleting an occurrence may remove multiple mapping edges
- importing a library may create documents, definitions, assets, and import nodes
- converting an opaque artifact region into semantic geometry may create many nodes plus updated
  artifact references

### Telemetry And Rejections

Graph-native commands should continue the current shell discipline:

- rejected in resolver when context is invalid
- surfaced in problems/logs
- recorded in telemetry

There should be no silent failures for missing scopes, stale refs, or unsupported edit targets.

## Session And Persistence Integration

The session system in
[`crates/autopcb-shell/src/session.rs`](/home/kiselev/git/altium-cli-simplified/crates/autopcb-shell/src/session.rs)
should remain the only persistence path for user-facing shell state.

What changes is the payload.

### Persisted Workspace State

Persist:

- workspace root or graph root ref
- active graph bundle root
- active import root if the workspace is still import-oriented

### Persisted Document State

Persist graph-backed documents by stable graph references:

- open scope refs
- active/secondary scope refs
- asset inspector refs
- import inspector refs

Continue to persist:

- untitled spec documents
- keybindings

during migration.

### Persisted Selection State

Persist:

- selected node refs
- selected scope ref
- instance path if required

Do not persist:

- transient hover state
- transient render cache handles
- importer scratch objects

### Restore Order

Restore ordering should remain deterministic and aligned with the project rules:

1. prefs/theme
2. workspace root / graph root
3. graph host load
4. documents/scopes
5. tabs and split state
6. selection
7. secondary derived caches

If graph load fails:

- fail soft
- preserve the rest of shell startup
- record a surfaced problem

## Dirty State And Save Model

The host needs a clearer dirty model than the current file-shaped one.

Recommended layers:

- graph-root dirty
- scope dirty
- asset dirty
- unsaved spec text dirty

Reasons:

- some changes affect only text-authored scopes
- some affect attached artifacts
- some affect import metadata or authority mode only

The save flow should support:

- save current scope
- save workspace
- save all dirty scopes and artifacts

The session system should still snapshot restorable unsaved text where needed, but graph and
artifact save should be explicit shell commands, not hidden autosave-only state.

## Problems And Failure Surfacing

Graph hosting introduces new failure classes that the shell must expose clearly:

- missing graph root
- missing artifact payload
- digest mismatch
- unresolved binding
- stale node ref in session restore
- stale scope ref in tab restore
- unsupported edit against opaque artifact authority
- failed render adapter build

These should be surfaced through the existing shell problem pipeline, not hidden in logs only.

## Minimum New Shell Types

The exact Rust API can change, but the shell likely needs equivalents of:

```text
WorkspaceRef
GraphRootRef
ScopeRef
NodeRef
TerminalRef
ConnectionRef
GeometryRef
AssetRef
ImportRef
InstancePath
```

And new workbench-side wrappers such as:

```text
GraphHost
GraphDocumentState
GraphSelectionState
InspectorTarget
DerivedRenderCache
```

The current `DocumentId` can remain as a runtime tab/container ID, but it should no longer be the
only persisted identity handle for graph-backed content.

## Phased Migration

### Phase 1: Add Graph Host Without Replacing Existing Documents

Add:

- `GraphHost`
- graph root loading
- graph-backed workspace state
- graph scope lookup

Do not remove:

- board documents
- spec text documents
- preview documents

Goal:

- open a graph workspace in the shell without changing most UI surfaces yet

### Phase 2: Add Graph Scope Tabs

Add graph-backed document kinds and session refs.

Goal:

- open logical and physical scopes directly

### Phase 3: Replace Selection And Inspector

Move from board-specific string selection to graph-native selection refs.

Goal:

- inspector works on any node family

### Phase 4: Add Render Adapters

Back logical and physical views from graph-derived render models.

Goal:

- UI rendering no longer depends on vendor-native object trees

### Phase 5: Move Commands To Graph Targets

Extend intents/resolution/execution to operate on graph refs and scope refs.

Goal:

- editing is canonical-model-first

### Phase 6: Retire Preview-Only Host Shapes

After graph-native tabs, selection, renderers, and sessions are stable:

- keep import adapters
- keep text spec editing
- retire vendor-preview document types as core concepts

## Definition Of Ready

`autopcb-shell` is ready for foundational graph-host implementation when these are fixed enough:

- canonical vocabulary
- identity and instance-path semantics
- connectivity model
- artifact model
- graph root / bundle layout
- minimum graph crate skeleton
- minimum render adapter contract

Without those, shell work will still be exploratory and likely to hard-code the wrong boundaries.

## Immediate Follow-On Work

After this document, the next implementation-facing design work should be:

1. define the minimum graph crate API the shell depends on
2. define `SessionTabRef` vNext for graph-backed scopes
3. define graph-native selection/inspector Rust types
4. define the render adapter contract for logical and physical scopes
5. audit current commands and classify them as:
   - still valid as-is
   - graph-aware with small changes
   - board-specific and needing replacement
