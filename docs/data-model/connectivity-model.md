# Connectivity Model

Status: Draft

## Summary

The canonical AutoPCB model uses a **hypergraph** as the core connectivity model.

Connectivity is not derived primarily from drawn geometry.
Connectivity is not just a string net name.

Instead:

- connectivity is first-class semantic data
- geometry is one possible realization or presentation of that connectivity
- logical and physical domains share one canonical connection identity

This document records the current design decisions.

## 1. Core Decisions

### 1.1 Canonical Connectivity Is A Hypergraph

Decision:

- canonical connectivity is modeled as a hypergraph

That means:

- one connection node may attach to many terminals directly
- the connection is first-class
- terminals are the attach points

This avoids making drawn wire segments or copper geometry the sole source of truth.

### 1.2 Connectivity Is Semantic First, Geometry Second

Decision:

- geometry is not the canonical connectivity model
- wires, tracks, vias, labels, and similar objects are realization and presentation structures

They may:

- express connectivity visually
- constrain implementation
- document realization

But the canonical electrical/logical identity lives in connection nodes and terminal membership.

### 1.3 Connection Families Are First-Class Node Types

Decision:

- use separate first-class node types rather than one generalized `connection_set` node

Current first-class types:

- `net`
- `bus`
- `bundle`
- `differential_pair`

This is a deliberate choice in favor of semantic clarity.

### 1.4 One Canonical Connection Identity Across Domains

Decision:

- logical and physical domains share one canonical connection identity

That means:

- a schematic connection and its board realization are views or projections of the same underlying connection
- physical routing, classes, tuning, and implementation state attach to the same semantic connection identity

This is preferred because downstream tools like routing and optimization benefit from seeing the full design intent.

### 1.5 Explicit Scoped Globals Are Allowed

Decision:

- explicit scoped globals are allowed in the canonical model
- implicit name-magic globals are not the preferred mechanism

So if global-like behavior exists, it must be declared explicitly and carry an explicit scope.

## 2. Why Hypergraph

The hypergraph model is the right core because:

- a connection naturally joins many terminals
- it avoids rebuilding canonical meaning from geometry fragments
- it works for both logical and physical domains
- it supports reuse, hierarchy, buses, and groups more cleanly

With a geometry-first model:

- wires and junctions become the canonical truth
- many tools must repeatedly derive the same connectivity
- hidden or abstract connectivity becomes awkward

That is the wrong tradeoff for a modern canonical model.

## 3. Canonical Connectivity Objects

## 3.1 Net

A `net` is the canonical scalar connection type.

It represents:

- ordinary electrical connectivity
- named or unnamed scalar connectivity
- the primary attach target for most terminals

Typical members:

- logical pins
- ports
- sheet interface terminals
- physical pads
- vias or copper attach points when represented semantically

## 3.2 Bus

A `bus` is an ordered or named grouped connection family.

A bus is not just a string naming convention.

A bus may:

- contain multiple scalar nets
- define ordering or indexing
- define interface grouping
- support structured expansion

Examples:

- `DATA[0..31]`
- `ADDR[0..23]`

## 3.3 Bundle

A `bundle` is a named grouped connection container whose members are not necessarily indexed
or homogeneous.

Examples:

- a grouped harness-like interface
- a mixed set of control, power, and data lines

A bundle is different from a bus because its semantics are grouping/composition, not indexed parallel signals.

## 3.4 Differential Pair

A `differential_pair` is a first-class connection type with two linked members and specialized
electrical/physical semantics.

It should not be encoded merely as two nets plus a naming convention.

It exists because:

- routing cares about it
- tuning cares about it
- constraint systems care about it
- import/export fidelity cares about it

## 4. Terminal Model

Connectivity attaches only through terminals.

Terminals unify concepts such as:

- symbol pin
- package pad
- port
- sheet/block interface point
- connector terminal
- test point terminal

### Principle

Terminals are the only legal attach points for canonical connectivity.

That gives us a stable graph abstraction across logical and physical views.

## 5. Logical And Physical Domains

The canonical model uses one connection identity across logical and physical domains.

This means a connection may have:

- logical terminals
- physical terminals
- logical geometry views
- physical realization geometry
- constraints at either or both levels

Examples:

- a logical net on a schematic
- the same net realized as routed copper on a board
- the same differential pair represented in both logical interface intent and physical routing

## 6. Geometry Relationship

Geometry does not define canonical connectivity, but it still matters.

### 6.1 Logical Geometry

Logical geometry may include:

- wire drawings
- bus drawings
- labels
- port symbols
- power symbols

These are visual/document structures tied to canonical connectivity.

### 6.2 Physical Geometry

Physical geometry may include:

- tracks
- arcs
- vias
- copper zones
- shape-based conductive regions

These are implementation structures tied to canonical connectivity.

### 6.3 Derived And Checked Relationship

The system should be able to:

- derive connectivity from geometry during import if needed
- validate geometry against canonical connectivity
- generate minimal geometry from canonical connectivity in constrained cases

But the canonical graph remains the authority once the design is in native form.

## 7. Hierarchical Connectivity

### 7.1 Explicit Interface Terminals

Preferred mechanism:

- parent/child document connectivity passes through explicit interface terminals

Examples:

- sheet port
- reusable block interface terminal
- board/module interface connector

This keeps hierarchy inspectable and modular.

### 7.2 Explicit Scoped Globals

Allowed mechanism:

- a connection may be declared global within an explicit scope

Examples of possible scopes:

- local document scope
- subtree scope
- module scope
- design-wide scope

Important rule:

- the scope must be explicit in the model
- no hidden or accidental global behavior from raw name matching alone

### 7.3 Why Allow Scoped Globals

Scoped globals are worth supporting because:

- they are useful for power-style connectivity
- they help with legacy import compatibility
- they can reduce noisy interface wiring where that is intentional

### 7.4 Risk Of Scoped Globals

Scoped globals increase hidden coupling if overused.

So the model should:

- support them
- make them explicit
- expose them clearly in inspectors and queries

## 8. Naming

Names are useful metadata on connection nodes.

But names are not the canonical identity.

This matters because:

- imported designs may rename things
- unnamed nets can exist
- multiple connection families may expose related names
- hierarchical/global semantics must not rely on bare strings alone

## 9. Bus And Bundle Expansion

Because `bus` and `bundle` are first-class node types, the model can express:

- the grouped connection object itself
- the contained members
- ordering/indexing where applicable
- interface semantics

This is preferable to reducing them to:

- naming conventions
- text parsing hacks
- geometry-only constructs

## 10. Differential Pair Semantics

A `differential_pair` should own:

- its identity
- the identity of its positive and negative members
- pair-level constraints and metadata

Examples:

- impedance targets
- skew constraints
- routing class association
- polarity labels

This keeps pair semantics available even before physical routing exists.

## 11. Imported Designs

Imported designs may arrive with connectivity encoded in several ways:

- explicit logical net objects
- implicit schematic geometry topology
- physical copper topology
- naming conventions for buses and differential pairs
- global power semantics

Importers should translate that into:

- canonical connection nodes
- canonical terminal membership
- explicit scoped globals where needed
- provenance links to the original source representation

## 12. Suggested Canonical Structures

Conceptually:

```text
Net
  id
  name?
  scope
  members[terminal_ref]

Bus
  id
  name?
  scope
  members[net_ref]
  ordering/index metadata

Bundle
  id
  name?
  scope
  members[connection_ref]

DifferentialPair
  id
  name?
  scope
  positive_ref
  negative_ref
  pair_constraints
```

## 13. Tradeoffs: First-Class Node Types

Why this was chosen:

- stronger semantic clarity
- clearer UI and inspector behavior
- simpler rule specialization
- easier to avoid invalid states

Known costs:

- larger schema surface
- more specialized traversal code
- more work if future connection families blur boundaries

Current judgement:

- the clarity is worth the cost at this stage

## 14. What This Means For The Spec Language

The spec language should eventually expose connectivity in a way that maps directly onto:

- `net`
- `bus`
- `bundle`
- `differential_pair`

And connectivity references should resolve through:

- bindings
- scoped references
- explicit interface terminals
- explicit scoped global declarations where intended

The language should avoid relying on stringly-typed net naming conventions as the only semantics.

## 15. Implications For `autopcb-shell`

The shell should treat connectivity as a graph-native object family.

That implies:

- selection can target connection nodes directly
- inspectors show members, scope, and realization state
- logical and physical views can both navigate the same connection identity
- autorouting and analysis operate on canonical connection objects rather than reconstructing intent from geometry alone

## 16. Current Design State

The current decided shape is:

- hypergraph connectivity core
- semantic-first, geometry-second
- first-class connection node types
- one canonical connection identity across logical and physical domains
- explicit interface terminals preferred
- explicit scoped globals allowed

This should be treated as the working connectivity model unless later research turns up a stronger alternative.
