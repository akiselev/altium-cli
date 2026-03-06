# KiCad Comparison

## Purpose

KiCad is a useful reference point because its data model and file formats are far cleaner than
Altium's. But it is still an evolved application architecture, not a clean-sheet canonical model.

We want to learn from it without copying its historical split points.

## What KiCad Gets Right

### 1. Explicit Project Container

KiCad treats the project as a container that owns boards, schematics, settings, and related state.

Good idea to keep:

- project-level shared settings
- top-level document registry
- project-local variables and library pinning

What not to copy:

- project metadata spread across several file types and lookup tables

### 2. Stable Object Identity

KiCad uses UUID-like object IDs broadly, and for schematic hierarchy it also uses path-based
instance identity.

Good idea to keep:

- stable object IDs
- instance-path identity for reused/hierarchical content
- distinct definition identity and occurrence identity

This is one of KiCad's strongest ideas and should absolutely survive into the new model.

### 3. Board As A Typed Graph

KiCad does not treat the board as just a flat geometry dump. It has:

- nets
- board items
- footprints
- zones
- groups
- drawings
- connectivity cache

Good idea to keep:

- board as a typed graph with derived caches
- clean separation between persistent objects and derived connectivity/index state

### 4. Libraries As Real Reusable Definitions

KiCad has separate symbol and footprint libraries with stable references.

Good idea to keep:

- reusable definitions distinct from document-local occurrences
- stable references from occurrences to definitions

## What KiCad Gets Wrong For Our Purposes

### 1. File Boundary Is Too Important

KiCad's architecture is still strongly organized around:

- project file
- schematic file
- board file
- symbol library file
- footprint library directory

That is better than Altium's legacy internals, but it is still too storage-shaped for a
clean canonical model.

For us, the primary boundary should be the graph, not the file type.

### 2. Canonical Data Is Duplicated For Recovery

KiCad often caches or embeds library-derived content inside schematics.

That is good for resilience, but bad as a conceptual source of truth.

For our model:

- canonical definitions should be explicit
- local snapshots should exist only as preservation/import artifacts

### 3. Identity Is Sometimes Too Multi-Axis

On the board side, KiCad footprints can carry:

- library ID
- UUID
- schematic association path
- temporary edit-time links
- sheet/file related strings

We should reduce this to a cleaner set:

- `occurrence_id`
- `definition_id`
- optional `source_occurrence_id`
- optional import provenance

### 4. Assets Are Too Path/Search Driven

KiCad leans heavily on:

- environment variables
- project-relative path expansion
- search stacks

That is practical for a desktop application, but not ideal as a canonical design model.

For us:

- assets should be declared in the manifest
- path variables should be compatibility helpers, not the primary mechanism
- content-addressed or manifest-addressed assets are preferable

## The Main Takeaway

KiCad's best idea is:

- hierarchical instance identity over reusable definitions

KiCad's weakest idea is:

- duplicated cached library content plus historically accumulated compatibility fields

## What We Keep

- stable IDs
- definition vs occurrence split
- instance-path identity
- typed board graph
- project-scoped settings
- optional embedded assets

## What We Discard

- file type as model root
- duplicated cached definitions as canonical structure
- search-path-driven asset resolution as the main semantics
- mixed historical identity axes on the same occurrence

## What We Modernize

We should turn KiCad's good ideas into a cleaner graph:

- one canonical design graph
- documents as graph scopes
- reusable definitions as graph nodes
- occurrences as graph nodes
- explicit links between logical and physical domains
- assets declared in graph manifests
- preservation nodes for imported legacy content
