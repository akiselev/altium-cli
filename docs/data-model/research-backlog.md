# Research Backlog

## Purpose

This document tracks the remaining research needed to make the document graph ready for
implementation in `autopcb-shell`.

## Priority 0: Freeze The Canonical Vocabulary

Before implementation starts, we need to finalize the naming and scope of the core node kinds.

Questions:

- should `part_definition` and `package_definition` be the final public names
- should `logical_document` and `physical_document` be the final names
- should `connection_set` be one type with variants, or multiple node kinds
- should `block_definition` and `sheet_definition` both exist, or should one subsume the other

Deliverable:

- glossary doc or RFC update with fixed terminology

## Priority 1: Identity And Path Semantics

We need a precise identity model before any shell or storage work starts.

Questions:

- exact stable ID format
- instance-path semantics for repeated/reused content
- whether IDs are opaque UUIDs, deterministic hashes, or both
- how imported IDs and native IDs coexist
- how to identify nodes before they have been saved

Deliverable:

- identity RFC with example IDs and path rules

## Priority 2: Definition / Occurrence Mapping

We need to formalize how definitions and occurrences relate.

Questions:

- can one logical occurrence map to multiple physical occurrences
- how variants affect occurrence mapping
- whether sheet instances and block instances share one abstraction
- where override data lives

Deliverable:

- mapping semantics doc with examples for:
  - simple resistor
  - multi-gate IC
  - one logical part with multiple package options
  - hierarchical repeated block

## Priority 3: Connectivity Model

We need a connection model that is stronger than "a net is a string".

Questions:

- net, bus, bundle, and differential-pair representation
- whether the connection graph is a hypergraph
- how named and unnamed connectivity are represented
- how logical and physical connectivity cross-link
- how local labels/global labels/ports-like concepts should be expressed canonically

Deliverable:

- connectivity RFC with graph examples

## Priority 4: Geometry Model

We need to define the canonical geometry layer before shell rendering can target it.

Questions:

- exact geometry primitives to support in v1
- arc preservation vs normalized curves
- how board outlines, cutouts, and zones are represented
- how logical annotation geometry differs from physical geometry
- how much geometry is semantic vs presentation-only

Deliverable:

- geometry schema doc with examples from logical and physical documents

## Priority 5: Asset And Binary Artifact Model

This is required for the hybrid text/binary workflow.

Questions:

- exact asset metadata schema
- whether assets are path-addressed, digest-addressed, or both
- difference between `structured_binary` and `opaque_binary`
- how asset authority is resolved when text and binary both exist
- which binary payloads should remain raw on first import

Deliverable:

- artifact RFC plus example bundle layout

## Priority 6: Import Preservation Policy

We need explicit policy on how much of the original import is retained.

Questions:

- when to preserve raw payloads
- when semantic content replaces raw payloads
- how to mark partial semantic coverage
- how to warn on lossy export

Deliverable:

- preservation policy document with import/export guarantees

## Priority 7: Shell Host Model

This is the implementation bridge for `autopcb-shell`.

Questions:

- what in-memory graph container the shell uses
- whether shell opens the entire graph or lazily loads document scopes
- how selections reference graph nodes
- how tabs map to graph scopes
- how sessions persist graph references
- how command intents target graph nodes atomically

Deliverable:

- shell integration design doc

Status:

- `shell-host-integration.md` now exists
- next step is turning it into concrete crate/type/API decisions

## Priority 8: Derived View Models

We need a plan for the derived models that sit on top of the graph.

Needed derived layers:

- render model for logical documents
- render model for physical documents
- placement/routing IR
- search/index model
- rule evaluation model

Deliverable:

- dependency diagram showing canonical graph vs derived models

## Priority 9: Importers To Study Next

We already looked at Altium and KiCad at a high level.

Still worth researching:

- Cadence OrCAD / Allegro
- Autodesk Eagle / Fusion Electronics
- gEDA / lepton
- IPC-2581 and EDIF style interchange models
- open-source netlist and package standards used in manufacturing pipelines

Reason:

- these can reveal useful neutral abstractions for connectivity, physical packaging, and assets

Deliverable:

- short comparative notes per tool/standard

## Suggested Sequence

1. Finalize vocabulary
2. Finalize identity/path model
3. Finalize definitions/occurrences/connectivity
4. Finalize geometry and asset model
5. Finalize preservation/import policy
6. Design `autopcb-shell` host model
7. Implement a minimal graph crate
8. Rework spec-language root format around the graph
9. Rebuild shell tabs/inspectors against graph scopes

## Immediate Next Design Work

- minimum graph crate API for shell integration
- `SessionTabRef` vNext and graph-backed restore semantics
- graph-native selection and inspector Rust types
- logical and physical render adapter contracts
- command migration inventory for `autopcb-shell`
