# RFC: Canonical Document Graph

Status: Draft

## 1. Summary

This RFC defines a new canonical AutoPCB data model.

The model is a clean-sheet design graph, not a mirror of Altium, KiCad, or any other EDA tool.
Vendor formats are import/export adapters only.

The spec language remains the authored root format, but evolves into a graph-oriented bundle format
that can mix text-authored data with binary artifacts.

## 2. Goals

- Define one canonical runtime and storage model for AutoPCB
- Represent logical, physical, reusable, and asset content in one graph
- Support high-fidelity import from Altium without inheriting Altium's internal structure
- Support multi-document projects, reusable modules, and hybrid text/binary content
- Give `autopcb-shell` a native document graph to host and edit

## 3. Non-Goals

- Reproducing vendor file structures in the canonical model
- Making every imported legacy artifact fully semantic on day one
- Replacing all derived IRs with one massive universal object tree

## 4. Core Principles

### 4.1 Graph First

The canonical model is a graph of nodes and edges.

Files are serialization containers for graph content, not the model itself.

### 4.2 Definitions And Occurrences Are Separate

Reusable content must not be collapsed into placed content.

Examples:

- a part definition is not the same thing as a placed logical component
- a package definition is not the same thing as a placed physical package
- a sheet definition is not the same thing as a sheet occurrence inside a design

### 4.3 Logical And Physical Domains Are Siblings

Logical and physical content belong to the same design graph.

They are linked directly by explicit edges, not by exported netlists or ad hoc string matching.

### 4.4 Preservation Is First-Class

Imported data may be:

- fully semantic
- partially semantic
- opaque but preserved

The canonical model must support all three at once.

### 4.5 Text And Binary Are Both Legitimate Storage Forms

The model should strongly prefer text for authored intent, but it must support binary artifacts
for dense, generated, or opaque imported content.

## 5. Top-Level Model

```text
DesignGraph
  id
  metadata
  documents[]
  definitions[]
  occurrences[]
  connections[]
  constraints[]
  assets[]
  imports[]
```

## 6. Node Families

### 6.1 Document Nodes

Documents are semantic scopes inside the graph.

Examples:

- `logical_document`
- `physical_document`
- `definition_collection`
- `manufacturing_document`
- `analysis_document`

Each document has:

- stable ID
- display name
- role
- ownership scope
- child node list
- local settings and view metadata

### 6.2 Definition Nodes

Definitions describe reusable design content.

Examples:

- `part_definition`
- `package_definition`
- `block_definition`
- `sheet_definition`
- `drawing_block_definition`

Definition nodes are immutable in identity and reusable by many occurrences.

### 6.3 Occurrence Nodes

Occurrences are placed or used instances of definitions within a document scope.

Examples:

- a logical component occurrence
- a physical package occurrence
- a reusable block occurrence
- a sheet occurrence

Each occurrence has:

- stable occurrence ID
- target definition ID
- parent document or occurrence
- instance path
- local overrides

### 6.4 Terminal Nodes

Terminals unify vendor concepts like:

- pin
- pad
- port
- sheet entry
- connector point

Terminal nodes belong either to definitions or occurrences depending on the level of abstraction.

### 6.5 Connection Nodes

Connections represent electrical or logical grouping.

Examples:

- scalar net
- bus
- bundle
- differential pair
- harness-like grouped connection set

Connection nodes own membership edges to terminals.

### 6.6 Geometry Nodes

Geometry nodes capture authored or imported shapes.

Examples:

- logical wires and labels
- physical tracks, vias, zones, keepouts
- fabrication geometry
- assembly geometry
- board outlines and cutouts
- annotation geometry

Geometry is typed and may be:

- native semantic geometry
- structured binary-backed geometry
- opaque preserved geometry

### 6.7 Constraint Nodes

Constraints capture design intent.

Examples:

- electrical rules
- physical rules
- class assignments
- tuning intents
- manufacturing constraints
- assembly constraints

Constraints reference their scope explicitly instead of being hidden in domain-specific side tables.

### 6.8 Asset Nodes

Assets represent non-core payloads and optional storage attachments.

Examples:

- text shards
- binary packs
- images
- 3D models
- imported opaque payloads
- generated caches

Each asset has:

- stable asset ID
- media/storage kind
- provenance
- authority mode
- optional digest
- optional local path mirror

### 6.9 Import Nodes

Import nodes describe source material and its relationship to the canonical graph.

Examples:

- imported Altium document
- imported KiCad project
- imported vendor library

Each import node tracks:

- source tool and version if known
- source file set
- import date
- import policy
- mapping to imported graph nodes

## 7. Edge Families

The canonical graph needs explicit edge kinds.

Minimum edge set:

- `contains`
- `defines`
- `instantiates`
- `maps_to`
- `connects`
- `constrains`
- `references_asset`
- `imported_from`
- `derived_from`

Examples:

- document `contains` occurrence
- occurrence `instantiates` definition
- logical occurrence `maps_to` physical occurrence
- logical terminal `maps_to` physical terminal
- connection `connects` terminal
- geometry `references_asset` binary payload

## 8. Identity Model

The model uses separate stable IDs for different roles.

Required IDs:

- `document_id`
- `definition_id`
- `occurrence_id`
- `terminal_id`
- `connection_id`
- `geometry_id`
- `constraint_id`
- `asset_id`
- `import_id`

Hierarchy identity uses both:

- stable object ID
- stable instance path

Instance path is required because reusable content can appear more than once.

## 9. Logical / Physical Split

The canonical model should not encode "schematic" and "pcb" as the only semantic domains.

Instead:

- logical documents describe functional intent and connectivity representation
- physical documents describe implementation and realization

Typical links:

- `part_definition -> package_definition`
- `logical_occurrence -> physical_occurrence`
- `logical_terminal -> physical_terminal`
- `logical_connection -> physical_connection_membership`

This supports:

- multiple packages per part
- alternate physical implementations
- variants
- multi-board systems
- reusable modules

## 10. Definition Types

### 10.1 Part Definition

Represents the logical identity of a component-like design element.

Contains:

- terminal interface
- semantic metadata
- parameters
- optional symbol views
- package implementation options
- variant options

### 10.2 Package Definition

Represents the physical implementation.

Contains:

- physical terminals
- copper/contact geometry
- fabrication geometry
- assembly geometry
- body geometry
- model references

### 10.3 Block Definition

Represents reusable grouped design content.

Can include:

- nested logical content
- nested physical content
- parameterization
- local assets

### 10.4 Sheet Definition

Represents reusable logical-document content with explicit interface points.

Instances of sheet definitions produce distinct occurrence paths and overrides.

## 11. Storage Model

The spec language remains the root authored format.

But the storage model becomes bundle-oriented.

Example:

```text
example.design-spec
example.design-spec.d/
  manifest.toml
  docs/
  defs/
  assets/
  map/
```

### 11.1 Root Text File

The root file should declare:

- design identity
- included documents
- included definition collections
- imports
- asset references
- storage policy

### 11.2 Text Shards

Small semantic documents may be split into additional text files.

Examples:

- part definition collections
- logical documents
- physical rule sets
- reusable modules

### 11.3 Binary Artifacts

Binary content should be stored as explicit graph assets.

Examples:

- imported dense copper payloads
- opaque legacy vendor payloads
- images
- 3D models
- structured binary geometry packs

## 12. Storage Authority Modes

Each semantic subtree or asset-backed node must declare its storage mode.

Minimum modes:

- `authored_text`
- `generated_text`
- `structured_binary`
- `opaque_binary`
- `external_reference`

This lets the system distinguish between:

- user-authored content
- generated content
- imported preserved content
- linked external assets

## 13. Import Strategy

### 13.1 Vendor Import Is A Translation Into The Graph

Importers should translate vendor formats into canonical graph content.

The importer may attach:

- semantic nodes
- preserved opaque assets
- provenance edges
- mapping metadata

### 13.2 Legacy Concepts Stay At The Boundary

Vendor-specific concepts should remain import/export adapter details unless they are actually
valuable semantics in the canonical model.

### 13.3 Progressive Semantic Upgrade

Imported opaque content should be allowed to become more semantic over time.

Example:

- v1 imports a routed copper block as structured binary
- v2 upgrades it to semantic track/via/zone nodes
- both remain representable in the same canonical graph

## 14. Suggested Rust Shape

This RFC does not lock the final Rust API, but the conceptual split should look like:

```rust
struct DesignGraph {
    identity: DesignIdentity,
    documents: Vec<DocumentNode>,
    definitions: Vec<DefinitionNode>,
    occurrences: Vec<OccurrenceNode>,
    connections: Vec<ConnectionNode>,
    constraints: Vec<ConstraintNode>,
    assets: Vec<AssetNode>,
    imports: Vec<ImportNode>,
    edges: Vec<GraphEdge>,
}
```

The exact storage/index strategy can be decided later.

## 15. `autopcb-shell` Implications

`autopcb-shell` should eventually host this graph directly.

That implies:

- document tabs are views into graph scopes
- inspectors operate on canonical graph nodes
- session restore persists graph references, not only file paths
- project navigation works over graph documents/definitions/assets
- rendering uses derived view models from the graph

## 16. Open Questions

- how strict should immutability be for definitions vs occurrences
- whether connection sets should support hypergraph structure directly
- how much geometry normalization should happen at import time
- whether structured binary assets should be tool-neutral or backend-specific
- how much of the graph should be writable in v1 of the shell

## 17. Decision

Adopt the design graph described here as the target canonical model for AutoPCB.

All future format, shell, and spec-language work should be evaluated against this model rather
than against vendor file taxonomies.
