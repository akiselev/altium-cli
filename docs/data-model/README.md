# Data Model Docs

This directory defines the clean-sheet data model direction for AutoPCB.

The goal is not to mirror Altium, KiCad, or any legacy tool's file structure.
The goal is to define a modern canonical design graph that:

- is clean enough to build new tooling on directly
- is rich enough to import almost any Altium design with minimal loss
- keeps the spec language as the authored root format
- supports mixed text and binary content where that is the practical choice

Documents in this directory:

- `current-state-and-goals.md`
  - Summary of the current model gaps in `autopcb-ir`, `autopcb-shell`, and `altium-format-spec`
  - Design goals for the replacement model
- `kicad-comparison.md`
  - What KiCad's model gets right
  - What should be discarded instead of copied
- `identity-and-paths.md`
  - Source references vs semantic IDs vs instance paths
  - UUID exposure policy for authored files
- `connectivity-model.md`
  - Hypergraph-first canonical connectivity
  - First-class node types for net, bus, bundle, and differential pair
- `artifact-model.md`
  - First-class artifact nodes for text, binary, external, and preserved content
  - Authority and storage-mode framework for hybrid text/binary designs
- `shell-host-integration.md`
  - Graph-native host model for `autopcb-shell`
  - Tabs, selection, sessions, commands, and render adapters over graph scopes
- `rfc-document-graph.md`
  - The proposed canonical document graph model
  - Node kinds, edge kinds, identity, assets, hierarchy, and storage modes
- `research-backlog.md`
  - Open research items required before implementation in `autopcb-shell`
  - Suggested work sequence
- `autopcb-shell-implementation-plan.md`
  - Concrete migration path from the current shell model to a graph-native shell

These docs assume:

- Altium remains an import/export boundary, not the canonical runtime model
- KiCad remains a reference point, not the canonical runtime model
- The spec language remains the authored root format, but evolves into a bundle-based graph format
