# Codebase Status Report

Generated: 2026-03-02

## Workspace Overview

9-crate Rust workspace for reading, writing, querying, rendering, and declaratively
specifying Altium Designer files.

```
altium-format-types    (domain types, enums, constants — zero deps)
     ↓
altium-format-derive   (proc macros: FromParams, ToParams, OpsSchema, OpsEnum)
     ↓
altium-format          (core: parsing, serialization, high-level API, rendering infra)
     ├→ altium-format-query       (AQL query language engine)
     ├→ altium-format-render-svg  (SVG rendering backend)
     └→ altium-format-render-png  (PNG rasterization via resvg)
altium-format-spec     (spec DSL: compiler, executor, reconciler, dump)
     ↓
altium-cli             (CLI binary: validate, save-as, render, query, plan, apply, dump, cfb tools)
```

---

## Document Type Summary

| Document   | Ext       | Parse | Serialize | High-Level API | Spec Lang | Query | Render  | CLI validate | CLI save-as | CLI new |
|------------|-----------|-------|-----------|----------------|-----------|-------|---------|--------------|-------------|---------|
| **SchLib** | .SchLib   | ✅    | ✅        | ✅ Full CRUD    | ✅         | ✅     | ✅ SVG/PNG | ✅          | ✅          | ✅      |
| **SchDoc** | .SchDoc   | ✅    | ✅        | ✅ Read, ⚠️ Write | ⚠️ dump only | ❌ | ✅ SVG/PNG | ✅          | ✅          | ✅      |
| **PcbLib** | .PcbLib   | ✅    | ✅        | ✅ Full CRUD    | ✅         | ✅     | ✅ SVG/PNG | ✅          | ✅          | ✅      |
| **PcbDoc** | .PcbDoc   | ✅    | ✅        | ❌ None         | ❌         | ❌     | ❌        | ✅          | ✅          | ❌      |
| **PrjPcb** | .PrjPcb   | ✅    | ✅        | ✅ Read-only    | ✅         | ❌     | ❌        | ✅          | ✅          | ❌      |
| **IntLib** | .IntLib   | ❌ Stub | ❌ Stub | ❌ None         | ❌         | ❌     | ❌        | ⚠️ open only | ❌          | ❌      |

---

## Per-Document Details

### SchLib (.SchLib) — MOST COMPLETE

**Parsing:** Complete. Opens CFB, reads FileHeader, component index, all record
streams, and 9 per-component sidecar streams (PinFrac, PinDesc, PinFunctionData,
PinTextData, PinMiscData, PinWideText, PinSymbolLineWidth, Additional, Redirection).

**Records parsed:** Component (1), Pin (2, binary format), all graphical primitives
(Line, Rectangle, RoundRectangle, Arc, EllipticalArc, Ellipse, Pie, Polyline,
Polygon, Bezier, Image, Label, TextFrame), Sheet (31), Implementation (45),
ImplementationList (44), ImplementationMap (46-48), Parameter (41), ParameterSet (40),
ParameterList (48), Designator, plus embedded images and component aliases.

**Serialization:** Complete roundtrip. Writes back all streams including sidecar
streams. Tested with semantic CFB diff.

**High-Level API:** Full CRUD — `component()`, `components()`, `component_names()`,
`add_component()`, `update_component()`, `remove_component()`. Component type exposes
pins, parameters, graphics, footprint maps.

**Spec Language:** Full support — compile, execute (create/update), reconcile (ECO
generation), dump (reverse-generate spec from document).

**Query:** Supported. Entity types: component, pin, parameter, footprint, graphic
(with subtypes). Supports attribute filters, pseudo-classes (`:power`, `:input`, etc.),
and combinators (`component > pin:power`).

**Rendering:** SVG and PNG. All graphical primitives rendered including pins with
names/designators, IEEE symbols, line styles.

**CLI:** `new schlib`, `validate`, `save-as`, `render`, `query`, `plan`, `apply`, `dump`.

---

### SchDoc (.SchDoc) — COMPLETE PARSE/SERIALIZE, LIMITED WRITE API

**Parsing:** Complete. Reads FileHeader, flat OWNERINDEX-linked record list, Additional
stream, and embedded OLE objects. Converts flat list to nested tree via `sheet()`.

**Records parsed:** All 40+ schematic record types including Wire, Bus, NetLabel, Port,
PowerObject, Junction, NoConnect, BusEntry, SheetSymbol, SheetEntry, Component, Pin,
all graphical primitives, Parameter, ParameterSet, ParameterList, Implementation chain,
Note, Probe, CompileMask, Blanket, HarnessConnector, SignalHarness, and high-level code
records.

**Serialization:** Complete roundtrip. Tree-to-flat conversion via `update_sheet()`.

**High-Level API:** Full read access — `sheet()` returns `SchDocSheet` with typed
accessors for all object types (`components()`, `wires()`, `buses()`, `net_labels()`,
`power_objects()`, `ports()`, `junctions()`, `sheet_symbols()`, etc.). Write is
**sheet-level only** — must rebuild entire `objects` vec and call `update_sheet()`. No
granular per-object add/remove.

**Spec Language:** Dump only (reverse-generate spec from document). Compile/execute not
implemented — returns error "SchDoc spec compilation is not implemented yet."

**Query:** Not supported (marked "Future").

**Rendering:** SVG and PNG. Full schematic sheet rendering.

**CLI:** `new schdoc`, `validate`, `save-as`, `render`, `dump`.

---

### PcbLib (.PcbLib) — COMPLETE

**Parsing:** Complete. Reads file header, section keys, library data, component TOC,
model entries, layer kind mapping, pad/via library config, and all footprints with
their primitives and sidecar streams.

**Primitives parsed (8 types):**
- **Pad** — 6 subrecords; pad stack modes, mask expansion, hole, plating, thermal,
  custom shapes
- **Via** — via structure, plating, size, blind/buried
- **Track** — width, layer, net, routing flags, union indices
- **Arc** — center, radius, angles, width
- **Text** — font, height, rotation, barcode mode, wide-string index
- **Fill** — rectangle corners, rotation
- **Region** — contour vertices (legacy f64 pairs or shape-based TPolySegment)
- **ComponentBody** — body outline/contour, keepout areas

**Sidecar streams:** ExtendedPrimitiveInformation, PrimitiveGUID, CustomShapes,
CustomMaskShapes, CornerRadiusChamfer, WideStrings6 (UTF-16LE TLV).

**Serialization:** Complete roundtrip.

**High-Level API:** Full CRUD — `footprint()`, `footprints()`, `footprint_names()`,
`footprint_count()`, `add_footprint()`, `update_footprint()`, `remove_footprint()`.
Note: ComponentBody graphics cannot be *created* via API but are preserved on update.

**Spec Language:** Full support — compile, execute, reconcile, dump. Supports pad
templates, spread operators, row/column/grid expansion.

**Query:** Not supported (marked "Future").

**Rendering:** SVG and PNG. Renders tracks, arcs, vias, pads (round/rect/octagonal/
rounded-rect), fills, regions, text, component bodies.

**CLI:** `validate`, `save-as`, `render`, `plan`, `apply`, `dump`.

---

### PcbDoc (.PcbDoc) — COMPLETE PARSE/SERIALIZE, NO HIGH-LEVEL API

**Parsing:** Complete. Reads legacy v1 header, v6 file header, and all 18+ section
types from the CFB container.

**Sections parsed (18 types):**
- **Primitive sections (11):** Arcs6, Pads6, Vias6, Tracks6, Texts6, Fills6, Regions6,
  ShapeBasedRegions6, ComponentBodies6, ShapeBasedComponentBodies6, BoardRegions
- **Parameter sections:** Board6, Nets6, Components6, Polygons6, Classes6,
  DifferentialPairs6, FromTos6, EmbeddedBoards6, Embeddeds6, and more
- **Special sections:** WideStrings6, Models/ModelsNoEmbed, EmbeddedFonts,
  PadViaLibrary, LayerKindMapping, SharedUnions, UnionRelations, UnionNames,
  UnionFeatures, SharedUnionParam, PrimitiveParameters, ConstraintManager,
  PrimitiveGuids, DrillManager, LettersGeometry

**DRC support (drc.rs, 122KB):** 39 rule classes and 38 violation classes fully parsed.
Rules, violations, waived violations, and DRC options all stored.

**Serialization:** Complete roundtrip.

**High-Level API:** None. PcbDoc has no public API types beyond `open()`, `save()`,
and `validate_invariants()`.

**Spec Language:** Not supported.

**Query:** Not supported.

**Rendering:** Not supported.

**CLI:** `validate`, `save-as`. No render, query, plan, apply, or dump.

**Validation status:** 94/96 V6 test files passing (97.9%). Known issues tracked in
`PCBDOC-next.md`:
- Bug #1: EmbeddedFonts6 conditional bold/italic (7 files)
- Bug #2: WideStrings6 empty string sentinel (1 fail, 28 affected)
- Bug #3: Arc radius allows negative values (1 file)
- Issue #4: PcbDoc V5 format (2 files, deferred)

---

### PrjPcb (.PrjPcb) — COMPLETE PARSE, READ-ONLY API

**Parsing:** Complete. INI-style text format with indexed sections. Handles BOM UTF-8,
section indexing (Document0..N, Configuration0..N, OutputGroup0..N), and all project
metadata.

**Sections parsed:** [Design], [Preferences], [Document{N}], [Configuration{N}],
[OutputGroup{N}] (with per-output indices), variants, parameters, diff pair suffixes,
modification/difference/ERC levels.

**Serialization:** Complete roundtrip with insertion-order preservation.

**High-Level API:** Read-only — `project()` returns typed `Project` with
`BuildConfiguration`, `OutputGroup`, `OutputJob`, `DocumentRef`,
`AnnotationSettings`, `ClassGenSettings`, `ErcConnectionMatrix`,
`ProjectVariant`, `ProjectParameter`, etc. No public write/mutation API (internal
write exists but is not surfaced).

**Spec Language:** Full support — compile, execute, reconcile, dump. Handles Design
section properties, ERC matrix overrides, documents, output groups, variants.

**CLI:** `validate`, `save-as`, `plan`, `apply`, `dump`.

---

### IntLib (.IntLib) — STUB

**Status:** Returns `NotImplemented` error on `open()`. No parsing, serialization, API,
or CLI support beyond a basic `validate` call that just attempts to open.

---

## CLI Command Matrix

| Command | SchLib | SchDoc | PcbLib | PcbDoc | PrjPcb | IntLib |
|---------|--------|--------|--------|--------|--------|--------|
| `new`          | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| `validate`     | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ open only |
| `save-as`      | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| `get version`  | ✅ | ❌ | ✅ | ❌ | ❌ | ❌ |
| `render`       | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ |
| `query`        | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| `plan`         | ✅ | ✅ | ✅ | ❌ | ✅ | ❌ |
| `apply`        | ✅ | ✅ | ✅ | ❌ | ✅* | ❌ |
| `dump`         | ✅ | ✅ | ✅ | ❌ | ✅ | ❌ |
| `cfb ls`       | ✅ | ✅ | ✅ | ✅ | n/a | ✅ |
| `cfb dump`     | ✅ | ✅ | ✅ | ✅ | n/a | ✅ |
| `cfb blocks`   | ✅ | ✅ | ✅ | ✅ | n/a | ✅ |
| `cfb diff`     | ✅ | ✅ | ✅ | ✅ | n/a | ✅ |
| `cfb cat`      | ✅ | ✅ | ✅ | ✅ | n/a | ✅ |

\* PrjPcb `apply` requires `--target` (no blank creation).

### CFB Tools (Low-Level Debugging)

All CFB subcommands work on any OLE/CFB container regardless of document type:
- `cfb ls` — list streams/storages (tree or `--flat`)
- `cfb dump` — hex+ASCII dump (`--blocks` for block annotation)
- `cfb blocks` — block-level summary or `--block N` for single block detail
- `cfb diff` — byte-level or `--semantic` comparison (order-agnostic params,
  embedded object decompression, categorized issue reporting)
- `cfb cat` — raw bytes to stdout for piping

---

## Crate-Level Details

### altium-format-types

83 public enums, 8 public structs, 13 constant modules. Coverage is comprehensive
with no TODOs or gaps. All Altium domain concepts are typed: `PcbObjectId` (27
variants), `SchRecordType` (92 variants), `V6Layer` (83 layers), `V7Layer`
(extended), `LayerRef` (unified), `RuleKind` (78 DRC rules), `PadShape`,
`HoleType`, `DrillType`, `TextKind`, `RegionKind`, `DimensionKind`, all
schematic pin/port/power types, harness types, project configuration types, etc.

All enums use `#[non_exhaustive]` and `TryFrom` with fail-fast on unknown values.

### altium-format-derive

4 proc macros:
- `FromParams` — deserialize struct from `ParameterCollection`
- `ToParams` — serialize struct to `ParameterCollection` (T1/T2 tier system)
- `OpsSchema` — operation schema metadata generation
- `OpsEnum` — enum variant schema generation

9 parameter strategies: Required, WithDefault, Optional, Coord (split integer+frac),
CoordPoint (X/Y with fracs), IndexedCoords (count+prefix arrays), Flatten
(composition), List (comma-separated), ListOrEmpty.

### altium-format (Core Library)

The heart of the project. Key infrastructure:
- `BinaryReader`/`BinaryWriter` — little-endian binary I/O
- `parse_blocks()` — 4-byte header block stream parsing
- `ParameterCollection` — typed key-value parameter access
- `TrackedCfbDocument` — CFB wrapper with consumption tracking
- `embedded_object` — 0xD0 zlib envelope parsing
- `wide_strings_tlv` — UTF-16LE TLV format for PcbDoc WideStrings6
- `AltiumFormatError` — 18+ error variants with chained context via `ResultExt`
- `test_utils` — semantic CFB diff for roundtrip testing

### altium-format-query

CSS-selector-inspired query language. Supports type selectors, attribute filters
(`[field=value]`, `[field>value]`, etc.), pseudo-classes (`:power`, `:input`),
combinators (descendant, child `>`), and logical operators (AND, OR, NOT, UNION).

SchLib and PcbLib are queryable. SchDoc, PcbDoc marked "Future".

### altium-format-render-svg / render-png

SVG backend renders 29 schematic record types and 8 PCB primitive types.
PNG backend wraps SVG via usvg/tiny-skia rasterization at configurable scale
(default 4.0 px/mil).

Known gaps: clip regions not applied, embedded image pixel data not preserved,
no PCB layer filtering, no text kerning/shaping.

### altium-format-spec

Declarative DSL for defining libraries and projects. Complete pipeline:
lexer → parser → AST → compiler → `SpecModel` → executor/reconciler/dump.

Supports SchLib, PcbLib, and PrjPcb. SchDoc has dump only (no compile/execute).

Features: let bindings, arithmetic expressions, dimensional units (`100mil`, `2.54mm`),
color literals (`#FF0000`), spread operators, template interpolation, row/column/grid
pad expansion, multi-part components, pin anchoring, import resolution.

---

## Known Issues & Gaps

### Critical

1. **IntLib is a stub** — returns `NotImplemented`. Violates fail-fast if users expect
   it to work. Should either implement or clearly document as unsupported.

2. **PcbDoc has no high-level API** — parsing and serialization are complete but there
   are no public API types for programmatic access to boards, nets, components, rules,
   etc.

### Moderate

3. **SchDoc write API is sheet-level only** — no granular per-object add/remove.
   Must rebuild entire objects vec for any mutation.

4. **PrjPcb has no public write API** — internal write support exists but isn't
   surfaced through the API module.

5. **Query only supports SchLib and PcbLib** — SchDoc, PcbDoc entity adapters not
   implemented.

6. **SchDoc spec compilation not implemented** — dump works but compile/execute returns
   error.

7. **PcbDoc rendering not supported** — no SVG/PNG rendering for board designs.

### Minor

8. **PcbDoc validation: 2/96 V6 files still failing** — EmbeddedFonts and
   WideStrings edge cases (tracked in PCBDOC-next.md).

9. **PcbDoc V5 format not supported** — 2 test files deferred.

10. **SVG clip regions not applied** — PushClip/PopClip recorded but skipped in SVG
    generation.

11. **`get version` only works for SchLib and PcbLib** — not PcbDoc, SchDoc, PrjPcb.

12. **`apply --report-json` flag accepted but not used** — dead code.

---

## Documentation State

**Well-organized:**
- `docs/dxp/` — 26 files of exemplary reverse-engineered format reference
- `docs/schlib/`, `docs/schdoc/`, `docs/pcblib/` — per-format documentation
- `examples/spec/hydro/` — working spec DSL example project

**Needs attention:**
- Operations/spec language docs fragmented across 6+ locations
- 80+ roundtrip reports and 19 FPGA analysis files need archiving
- Missing README entry points in several subdirectories
- `PCBDOC-next.md` tracks active PcbDoc validation status
- `AUDIT-SYNTHESIS.md` has detailed doc reorganization recommendations
- `data-review.md` documents 11 Cardinal Rule violations (6 critical)

---

## Test Infrastructure

- **Unit tests** in `#[cfg(test)]` blocks throughout `altium-format`
- **Property tests** (proptest) gated behind `--features proptest` in SchDoc, PcbLib,
  PcbDoc, and CLI
- **Fixture tests** gated behind `--features test-fixtures` for data/ directory files
- **Semantic CFB diff** (`assert_cfb_files_semantic_eq`) for roundtrip validation
- **Spec language tests** — parser, lexer, evaluator, compiler, reconciler, dump, import
- **Derive macro tests** in `derive_tests.rs`
- Default `cargo test` runs fast unit tests only
