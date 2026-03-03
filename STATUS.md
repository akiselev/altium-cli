# Codebase Status Report

Generated: 2026-03-02

## Workspace Overview

11-crate Rust workspace for reading, writing, querying, rendering, and declaratively
specifying Altium Designer files — plus automated PCB placement/routing IR and viewer.

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
altium-cli             (CLI binary: validate, save-as, render, query, plan, apply, dump, inspect, cfb tools)

autopcb-ir             (PCB intermediate representation: mm-based extraction from PcbDocBoard; serde JSON export)
     ↓
autopcb-viewer         (standalone egui/wgpu binary: 2D + 2.5D PCB viewer with pan/zoom/orbit)
```

---

## Document Type Summary

| Document   | Ext     | Parse  | Serialize | High-Level API | Spec Lang | Query | Render    | CLI validate | CLI save-as | CLI new |
| ---------- | ------- | ------ | --------- | -------------- | --------- | ----- | --------- | ------------ | ----------- | ------- |
| **SchLib** | .SchLib | ✅      | ✅         | ✅ Full CRUD    | ✅         | ✅     | ✅ SVG/PNG | ✅            | ✅           | ✅       |
| **SchDoc** | .SchDoc | ✅      | ✅         | ✅ Read/Write   | ✅         | ✅     | ✅ SVG/PNG | ✅            | ✅           | ✅       |
| **PcbLib** | .PcbLib | ✅      | ✅         | ✅ Full CRUD    | ✅         | ✅     | ✅ SVG/PNG | ✅            | ✅           | ✅       |
| **PcbDoc** | .PcbDoc | ✅      | ✅         | ✅ Read/Write   | ✅         | ✅     | ❌         | ✅            | ✅           | ❌       |
| **PrjPcb** | .PrjPcb | ✅      | ✅         | ✅ Read-only    | ✅         | ❌     | ❌         | ✅            | ✅           | ✅       |
| **IntLib** | .IntLib | ❌ Stub | ❌ Stub    | ❌ None         | ❌         | ❌     | ❌         | ⚠️ open only  | ❌           | ❌       |

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

### SchDoc (.SchDoc) — COMPLETE PARSE/SERIALIZE/WRITE

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
`power_objects()`, `ports()`, `junctions()`, `sheet_symbols()`, etc.). Write via
`update_sheet()` with UniqueId-based field preservation — format-internal fields
(`index_in_sheet`, `selection_memory`, `style_id`, `graphically_locked`, etc.) are
preserved across save cycles for all record types via Altium's `UNIQUE_ID` field.
Convenience mutation methods: `add_object()`, `remove_objects()`, `component_mut()`,
`add_component_child()`.

**Spec Language:** Full support — dump, compile, plan, and apply. Supports sheet metadata
(fonts, grid, custom size), component placement, all low-level SchDoc objects (wire, bus,
net_label, power_object, port, junction, no_connect, bus_entry, sheet_symbol, parameter_set,
note, probe, compile_mask, blanket, harness_connector, signal_harness), graphics (label,
rectangle, line, etc.), and parameters. High-level `net` and `power` blocks for declarative
net connectivity. SchDoc specs can import SchLib specs for `$alias.ComponentName` references.

**Query:** Supported. Entity types: schdoc_component, wire, bus, net_label, power_object,
port, junction, no_connect, bus_entry, sheet_symbol, sheet_entry, note, probe, compile_mask,
blanket, harness_connector, signal_harness, parameter_set, parameter, graphic (with subtypes).
Net name queries (`%VCC`) match NetLabel, PowerObject, Port, and SheetEntry objects by name.
Supports attribute filters, pseudo-classes, and combinators.

**Rendering:** SVG and PNG. Full schematic sheet rendering.

**CLI:** `new schdoc`, `validate`, `save-as`, `render`, `query`, `info`, `dump`, `plan`, `apply`.

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
- `PadStack` support: multi-layer pad shapes (top/mid/bot/inner) exposed via `pad.stack`
- `PcbContour` on regions/component bodies: arc-preserving contours instead of `Vec<CoordPoint>`
- Query helpers: `pad()`, `pads_on_layer()`, `plated_through_hole_pads()`,
  `non_plated_through_hole_pads()`, `smd_pads()`, `graphics_on_layer()`,
  `regions()`, `component_bodies()`
- Shared types with PcbDoc via `pcb_common.rs` (PadStack, PcbContour, ContourSegment)

**Spec Language:** Full support — compile, execute, reconcile, dump. Supports pad
templates, spread operators, row/column/grid expansion.

**Query:** Supported. Entity types: footprint, pad, track, arc, fill, region, text, via,
component_body. Supports attribute filters and combinators.

**Rendering:** SVG and PNG. Renders tracks, arcs, vias, pads (round/rect/octagonal/
rounded-rect), fills, regions, text, component bodies.

**CLI:** `validate`, `save-as`, `render`, `plan`, `apply`, `dump`.

---

### PcbDoc (.PcbDoc) — COMPLETE PARSE/SERIALIZE, READ/WRITE API

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

**Serialization:** Complete roundtrip. 94/96 V6 test files save-as successfully
(remaining 2 are V5 format files missing `/FileHeaderSix`). Shared primitive
serializers (Pad, Via, Region, ComponentBody) extracted into
`pcb_primitives_serialize.rs`; PcbDoc-specific serializers for Arc, Track, Fill,
Text in `pcbdoc/mod.rs`.

**Roundtrip fidelity:** Semantic CFB diff shows only acceptable differences:
- **Boolean normalization** — Delphi stores non-zero (0x03, 0x80) for true; we canonicalize to 0x01
- **Font name buffer padding** — Delphi has uninitialized heap garbage in 64-byte fixed buffers; we write clean zeros
- **Pad sub4 format upgrade** — 171-byte → 172-byte variant (adds `has_sub4_extension` field)
- **Via format upgrade** — always writes Section 4/5 (IPC-4761 + template link extension)
- **Rules6 format upgrade** — tier2 serialization for all rules
- **Param key ordering** — `ParameterCollection` uses insertion order; Altium may differ
- **Duplicate param keys** — Board6 LAYER/LOCKED written twice by Altium; ParameterCollection deduplicates

**High-Level API:** Read/Write — `board()` returns typed `PcbDocBoard` with all
cross-references resolved (net indices → names, component indices → designators,
WideStrings6 indices → text strings). `update_board()` writes a `PcbDocBoard`
back into internal sections: parameter sections (Nets6, Components6, Polygons6,
Classes6, DifferentialPairs6, Board6) rebuilt from scratch; primitive sections
(Tracks6, Arcs6, Vias6, Pads6, Fills6, Texts6, Regions6, ComponentBodies6)
rebuilt with format-internal field preservation from existing records at same
index position; WideStrings6 rebuilt with deduplication. Legacy/modern section
detection (ShapeBasedRegions6 vs Regions6, etc.) for write path. Contains
`BoardSettings`, typed collections for all named entities (`Net`,
`PcbDocComponent`, `Polygon`, `NetClass`, `DesignRule`, `DifferentialPair`),
all 8 primitive types (`Track`, `Arc`, `Via`, `Pad`, `Fill`, `Text`, `Region`,
`ComponentBody`), plus `Dimension` and `Model3D`.

**V2 API extensions** (non-breaking):
- `LayerStack`: Full layer stack from Board6 (V9/V8/V7/legacy), with copper thickness,
  dielectric properties, and physical ordering. Convenience methods: `top()`, `bottom()`,
  `layer()`, `inner_layers()`.
- `RuleParams`: Typed parameters for ~35 design rule kinds (clearance, width, mask expansion,
  routing via style, etc.) plus `Other` fallback. `DesignRule` extended with `scope2`,
  `net_scope`, `layer_scope`.
- `PadStack`: Per-layer pad shapes (top/mid/bot) with corner radius, inner layer overrides,
  hole shape, and slot data.
- `BoardGeometry`: Arc-preserving board outline and cutouts from internal region contours.
  `BoardContour` with `Line`/`Arc` segments, keepout zones.
- `BoardConnectivity`: `connectivity()` method groups pads by net with component counts.
- Layer/drill queries: `tracks_on_layer()`, `pads_on_layer()`, `primitives_on_layer()`,
  `vias_by_drill_pair()`, `plated_through_hole_pads()`, `non_plated_through_hole_pads()`,
  `regions_for_polygon()`.

**Internal type fixes:** `PcbStackLayerEntry`, `PcbV7LayerEntry`, `PcbLegacyLayerEntry`,
`PcbSurfaceProperties` fields changed from `String` to `Coord`/`f64` (D3 violation fix).

Legacy query helpers: `net()`, `component()`, `tracks_for_net()`, `pads_for_net()`,
`vias_for_net()`, `pads_for_component()`, `tracks_for_component()`,
`bodies_for_component()`, `rule()`, `rules_for_kind()`. Handles legacy/modern section
pairs (prefers ShapeBasedRegions6 over Regions6, etc.). Board outline auto-extracted from
region primitives.

**Spec Language:** Full support — compile, execute, reconcile, dump. Board settings
(signal_layer_count, snap/visible grid, display_unit), named collections (nets,
components, polygons, rules, classes, differential pairs), and primitives (tracks,
arcs, vias, pads, fills, texts, regions, component_bodies, dimensions). `apply`
requires an existing target file (no `new` for PcbDoc). Dump produces roundtrippable
`.pcbdoc-spec` output. Reconciler uses name-based matching for named collections and
ID-based matching for primitives.

**Query:** Supported. Entity types: pcbdoc_net, pcbdoc_component, pcbdoc_polygon,
pcbdoc_rule, pcbdoc_class, pcbdoc_dimension, pcbdoc_differential_pair, plus reused
PcbLib primitive selectors (track, pad, via, arc, fill, text, region, component_body).
Supports attribute filters on all fields. Pseudo-classes: `:smd`, `:through_hole`
(pad hole detection), `:top`, `:bottom` (layer filtering). Flat query model — all
objects are root-level nodes; component/net relationships queryable via attribute
filters (e.g., `pad[component='U1']`, `track[net='GND']`).

**Rendering:** Not supported.

**CLI:** `validate`, `save-as`, `info`, `query`, `dump`, `plan`, `apply`.

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

**CLI:** `new prjpcb`, `validate`, `save-as`, `plan`, `apply`, `dump`.

---

### IntLib (.IntLib) — STUB

**Status:** Returns `NotImplemented` error on `open()`. No parsing, serialization, API,
or CLI support beyond a basic `validate` call that just attempts to open.

---

## CLI Command Matrix

| Command       | SchLib | SchDoc | PcbLib | PcbDoc | PrjPcb | IntLib      |
| ------------- | ------ | ------ | ------ | ------ | ------ | ----------- |
| `new`         | ✅      | ✅      | ✅      | ❌      | ✅      | ❌           |
| `validate`    | ✅      | ✅      | ✅      | ✅      | ✅      | ⚠️ open only |
| `save-as`     | ✅      | ✅      | ✅      | ✅      | ✅      | ❌           |
| `get version` | ✅      | ❌      | ✅      | ❌      | ❌      | ❌           |
| `render`      | ✅      | ✅      | ✅      | ❌      | ❌      | ❌           |
| `query`       | ✅      | ✅      | ✅      | ✅      | ❌      | ❌           |
| `info`        | ✅      | ✅      | ✅      | ✅      | ❌      | ❌           |
| `plan`        | ✅      | ✅      | ✅      | ✅      | ✅      | ❌           |
| `apply`       | ✅      | ✅      | ✅      | ✅      | ✅      | ❌           |
| `dump`        | ✅      | ✅      | ✅      | ✅      | ✅      | ❌           |
| `cfb ls`      | ✅      | ✅      | ✅      | ✅      | n/a    | ✅           |
| `cfb dump`    | ✅      | ✅      | ✅      | ✅      | n/a    | ✅           |
| `cfb blocks`  | ✅      | ✅      | ✅      | ✅      | n/a    | ✅           |
| `cfb diff`    | ✅      | ✅      | ✅      | ✅      | n/a    | ✅           |
| `cfb cat`     | ✅      | ✅      | ✅      | ✅      | n/a    | ✅           |


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

SchLib, PcbLib, SchDoc, and PcbDoc are queryable. PcbDoc uses a flat query model
(all objects as root nodes) with PcbDoc-specific pseudo-classes (`:smd`, `:through_hole`,
`:top`, `:bottom`).

### altium-format-render-svg / render-png

SVG backend renders 29 schematic record types and 8 PCB primitive types.
PNG backend wraps SVG via usvg/tiny-skia rasterization at configurable scale
(default 4.0 px/mil).

Known gaps: clip regions not applied, embedded image pixel data not preserved,
no PCB layer filtering, no text kerning/shaping.

### altium-format-spec

Declarative DSL for defining libraries and projects. Complete pipeline:
lexer → parser → AST → compiler → `SpecModel` → executor/reconciler/dump.

Supports SchLib, PcbLib, PrjPcb, SchDoc, and PcbDoc (full compile/plan/apply support).

Features: let bindings, arithmetic expressions, dimensional units (`100mil`, `2.54mm`),
color literals (`#FF0000`), spread operators, template interpolation, row/column/grid
pad expansion, multi-part components, pin anchoring, import resolution.

---

## Known Issues & Gaps

### Critical

1. **IntLib is a stub** — returns `NotImplemented`. Violates fail-fast if users expect
   it to work. Should either implement or clearly document as unsupported.

### Moderate

3. **PrjPcb has no public write API** — internal write support exists but isn't
   surfaced through the API module.

4. **PcbDoc rendering not supported** — no SVG/PNG rendering for board designs.

### Minor

6. **PcbDoc validation: 2/96 V6 files still failing** — EmbeddedFonts and
   WideStrings edge cases (tracked in PCBDOC-next.md).

7. **PcbDoc V5 format not supported** — 2 test files deferred.

8. **SVG clip regions not applied** — PushClip/PopClip recorded but skipped in SVG
    generation.

9. **`get version` only works for SchLib and PcbLib** — not PcbDoc, SchDoc, PrjPcb.

10. **`apply --report-json` flag accepted but not used** — dead code.

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

## Code Quality (Rules Review Fix — 2026-03-01)

Fixed all 15 rule violations identified in `rule-review.md`:

- **`Coord::from_mils()` → `Option<Self>`**: Overflow-safe coordinate construction. ~60 call sites updated (small literals use `.expect()`, tests use `.expect()`, CLI uses `?`).
- **`.unwrap()` → `.expect()` with context**: 4 sites in `param_collection.rs`, `pcbdoc/records.rs`, `pcbdoc/mod.rs` now document their invariants.
- **Hard-coded constants → named constants**: `254` (record overflow → `INSTRUCTION_EXTRA_OBJECT_INDEX`), `254` (string length → `C_MAX_SHORT_STRING_LENGTH`), `"RECORD"` → `RECORD`, `0x8E` → `C_SCH_SPECIAL_DELIMITER`.
- **`write_wide_string_fixed()` → `Result<()>`**: `assert!()` replaced with `Err(InvalidParamValue)`, 6 call sites propagate `?`.
- **`fill_record_fields()` → `Result<()>`**: `unreachable!()` for Sheet/Pin replaced with `Err(NotImplemented)`.
- **`generate_unique_key()` → `Result<String>`**: `unreachable!()` replaced with `Err(InvalidParamValue)`, cascaded through `build_section_key_for_name()` and `build_section_keys()`.
- **`mask_expansion_mode_to_str()` → `Result<&str>`**: `unreachable!()` replaced with proper error, cascaded through `serialize_extended_primitive_information()`.
- **Macro/CornerStyle wildcards**: `unreachable!()` → `panic!()` with debug info (`{other:?}`).
- **`SchLib::new_blank_ad26()` / `PcbLib::new_blank_ad26()` → `Result<Self>`**: `.expect()` → `.context()?`, all callers updated.

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

---

## Solverang Integration — Phase 0d/0e (2026-03-02)

### autopcb-ir Enhancements

**Serde JSON export** (`serde` feature flag):
- `altium-format-types`: optional `serde` feature adds `Serialize`/`Deserialize` to `RuleKind`
- `autopcb-ir`: optional `serde` feature adds `Serialize` to all IR types (`PcbIr`, `IrComponent`,
  `IrNet`, `IrTrack`, `IrVia`, `IrFill`, `IrPolygon`, `IrDesignRule`, etc.)
- Handle newtypes (`ComponentId`, `NetId`, etc.) serialize as plain `u32`
- `IdMap<K,V>` serializes as `Vec<V>`

**CLI `inspect ir-json`**: `altium inspect <file> ir-json` outputs full IR as pretty-printed JSON.

### autopcb-viewer Enhancements

**2D rendering improvements:**
- **Layer-colored tracks**: tracks colored by layer name (red=Top, blue=Bottom, etc.)
- **Keepout zones**: semi-transparent red polygons with stroke
- **Copper pour polygons**: semi-transparent layer-colored fill
- **Board fills**: layer-colored rectangles
- **Board cutouts**: punched with background color
- **Net highlighting**: click net in sidebar to highlight matching copper; non-matching dims to ~40% alpha
- **Component→net selection**: selecting a component shows its connected nets

**Sidebar enhancements:**
- Collapsible Components and Nets sections
- Display toggles for keepouts, fills, polygons
- 2D/3D view mode toggle

**Keyboard shortcuts:** F (fit to board), N (toggle ratsnest), L (toggle copper layers),
S (screenshot), Esc (clear selection)

**Screenshot mode:** `autopcb-viewer <file> --screenshot output.png` renders one frame,
saves PNG, exits. Interactive S key also saves `screenshot.png`.

**2.5D wgpu view:**
- Extruded board substrate (FR4 green, 1.6mm thick)
- Copper layers as colored slabs at correct Z positions (top=red, bottom=blue, inner=interpolated)
- Via boxes spanning full board thickness
- Component bounding boxes (red=top, blue=bottom)
- Orbit camera with mouse drag + scroll zoom
- Lambertian shading in WGSL shader for depth perception
- Orthographic projection with configurable yaw/pitch/zoom

---

## AutoPCB Shell — IDE Entry Point (2026-03-03)

### Milestone 1: Shell Foundation

- [x] Create `crates/autopcb-shell` binary and workspace wiring
- [x] Implement shell frame with `egui_tiles` dock layout
- [x] Persist layout/panel state with versioned `eframe` storage keys
- [x] Implement strict command bus + command palette
- [x] Route all M1 user actions through command dispatch
- [x] Add first-class `SelectionState` in Workbench model
- [x] Wire Explorer -> Selection -> PCB pane highlight flow
- [x] Add GPU-ready `PcbCanvasView` abstraction
- [x] Add M1 shortcut map and context-key enable checks
- [x] Add M1 tests and manual smoke validation notes

Manual smoke notes:
- `autopcb-shell <path-to-pcbdoc>` launches IDE shell with sidebar, docked editor tabs, bottom panel, and status bar.
- Command palette opens via `Ctrl/Cmd+Shift+P` and routes commands through registry/dispatcher.
- Explorer component/net selection updates shared selection model and highlights corresponding entities in PCB 2D view.
- Layout and panel visibility persist across restarts via eframe storage keys `shell.layout.v1` and `shell.panels.v1`.

### Milestone 2+: Planned

- [ ] Full command catalog implementation from `docs/gui/commands.md`
- [ ] Command-based undo/redo with inverse model operations
- [ ] File-watcher external change detection + reconciliation flows
- [ ] Job system integration for placement/routing/DRC background execution
- [ ] 3D `PaintCallback` render path migration from placeholder to full scene renderer
- [ ] Spec editor with diagnostics, plan/apply previews, and cross-navigation
