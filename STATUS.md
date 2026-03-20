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
altium-cli             (CLI binary: validate, save-as, render, query, plan, apply, dump, inspect, cfb tools, placement dump/plan/apply/autoplace)

autopcb-ir             (PCB intermediate representation: mm-based extraction from PcbDocBoard; serde JSON export)
     ↓
autopcb-viewer         (standalone egui/wgpu binary: 2D + 2.5D PCB viewer; spec-centric — accepts ONLY .pcbdoc-spec files, never PcbDoc directly; all rendering from PcbIr via spec bridge)
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
| **IntLib** | .IntLib | ✅ Read | ❌         | ❌ None         | ❌         | ❌     | ❌         | ✅             | ✅ dump      | ❌       |

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

**Serialization:** Complete roundtrip. Shape-based contour geometry text params
(MAINCONTOURVERTEXCOUNT, KINDn, VXn, VYn, etc.) fully parsed into typed
`PolySegment` data and regenerated during serialization. Region and ComponentBody
params conditionally written to match Altium's behavior (LAYER, KEEPOUT,
ISBOARDCUTOUT omitted at defaults; MODEL.SNAPCOUNT omitted when empty;
ARCRESOLUTION duplicate positioned correctly).

**Roundtrip fidelity:** Semantic CFB diff shows only acceptable differences:
- **Font name buffer padding** — Altium leaves garbage after NUL in 64-byte fixed buffer; we zero-fill
- **Text sub#1 WideStrings upgrade** — Win1252 sub-record text upgraded to match authoritative WideStrings content
- **Via format upgrade** — ext_size 42 → 45 (adds trailing RevisionID, per upgrade-to-latest policy)
- **Library/Data stream** — regenerated global stream may differ in size
- **SharedUnion NUL terminator** — minor off-by-one from trailing NUL handling

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

**PrimitiveParameters (BOM Data) — M10:** Implemented. `PcbDocComponent` now carries
a `parameters: Vec<(String, String)>` field. `board_to_internal()` generates the
`PrimitiveParameters` CFB section from component parameters, one group per component
with `SOURCEDESIGNATOR`/`COUNT` header and `NAME`/`VALUE`/`ISIMPORTED` parameter blocks.
`PcbDocComponentSpec` has a matching `parameters: IndexMap<String, String>` field that
flows through: sync projection (`project_pcbdoc_spec` reads from spec; SchDoc projection
already populated from component parameters), diff (`diff_snapshots` compares `parameter:*`
fields), filter (`SyncDirection::Forward` enabled in main.rs), apply
(`apply_sync_changes_to_pcbdoc` updates the spec parameters map), and executor
(`apply_pcbdoc_components` merges spec parameters onto the board component). The text
rewriter silently skips `parameter:*` fields (parameters are not stored as spec properties).

**AutoPCB Placer — Milestone 3 (Reconciler Placement Comparison):** Implemented.
`reconcile_pcbdoc()` now compares component positions from `placement { place ... }`
blocks against PcbDoc component locations. Emits `Update` ECO entries for components
whose position differs by more than 0.01mm (tolerance covers Coord↔f64 round-trip
artifacts). Rotation comparison reserved for when `PlacementPlaceSpec` gains a
`rotation` field (`ROTATION_TOLERANCE_DEG = 0.1` constant defined and ready).
Components in spec but absent from PcbDoc emit a warning and are skipped. Only
`place` blocks with an explicit `at:` coordinate participate in comparison.
6 unit tests cover: moved component, same position, within tolerance, no `at:`,
missing designator, and multi-designator place blocks.

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

### IntLib (.IntLib) — READ-ONLY

**Parsing:** Opens CFB container, decompresses zlib-wrapped embedded SchLib and PcbLib
streams (0x02 prefix + zlib), and parses them using the existing SchLib/PcbLib parsers.
Handles V5 and V6 embedded libraries, optional CKT/MDL/PCB3DLib storages.
Some older IntLib files (pre-AD6) may fail on V5 PcbLib primitives.

**Serialization:** Not implemented. Read-only.

**API:** `IntLib::open(path)`, `.schlibs()`, `.pcblibs()`. Embedded libraries are
full `SchLib`/`PcbLib` objects with all their normal API methods.

**CLI:** `validate` (reports SchLib/PcbLib counts), `dump` (produces separate
`.schlib-spec` and `.pcblib-spec` files from the embedded libraries).

---

## CLI Command Matrix

| Command       | SchLib | SchDoc | PcbLib | PcbDoc | PrjPcb | IntLib      |
| ------------- | ------ | ------ | ------ | ------ | ------ | ----------- |
| `new`         | ✅      | ✅      | ✅      | ❌      | ✅      | ❌           |
| `validate`    | ✅      | ✅      | ✅      | ✅      | ✅      | ✅            |
| `save-as`     | ✅      | ✅      | ✅      | ✅      | ✅      | ❌           |
| `get version` | ✅      | ❌      | ✅      | ❌      | ❌      | ❌           |
| `render`      | ✅      | ✅      | ✅      | ❌      | ❌      | ❌           |
| `query`       | ✅      | ✅      | ✅      | ✅      | ❌      | ❌           |
| `info`        | ✅      | ✅      | ✅      | ✅      | ❌      | ❌           |
| `plan`        | ✅      | ✅      | ✅      | ✅      | ✅      | ❌           |
| `apply`       | ✅      | ✅      | ✅      | ✅      | ✅      | ❌           |
| `dump`        | ✅      | ✅      | ✅      | ✅      | ✅      | ✅           |
| `cfb ls`      | ✅      | ✅      | ✅      | ✅      | n/a    | ✅           |
| `cfb dump`    | ✅      | ✅      | ✅      | ✅      | n/a    | ✅           |
| `cfb blocks`  | ✅      | ✅      | ✅      | ✅      | n/a    | ✅           |
| `cfb diff`    | ✅      | ✅      | ✅      | ✅      | n/a    | ✅           |
| `cfb cat`     | ✅      | ✅      | ✅      | ✅      | n/a    | ✅           |


### Spec Sync Commands

| Command                                                   | Purpose                                      |
| --------------------------------------------------------- | -------------------------------------------- |
| `altium spec sync --forward <schdoc-spec> <pcbdoc-spec>`  | Forward sync: push SchDoc changes to PcbDoc  |
| `altium spec sync --diff <schdoc-spec> <pcbdoc-spec>`     | Diff only: show changes without applying     |
| `altium spec sync ... --dry-run`                          | Show ECO report without writing files        |

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

**Annotation system (M1–M3):** Rust-style block annotations (`#[annotation(id = "...",
stable = true, group = "...")]`) on all spec block types. IDs are 8-character
`[A-Z0-9]` strings matching Altium's UniqueID format. Auto-generated on dump when
absent. Duplicate detection is two-layer: compiler catches within-file duplicates
(fast-fail); validator catches cross-file duplicates (authoritative).

**Sync IR (M4–M6):** `SyncSnapshot` intermediate representation enables spec-to-spec
synchronization. Both `SchDocSpec` and `PcbDocSpec` project into a common normalized
snapshot. `diff_snapshots()` produces a direction-agnostic `Vec<SyncChange>`.
`filter_changes()` applies `SyncPolicy` direction rules and hard-errors on pin variants.
`apply_sync_changes_to_pcbdoc()` applies changes to `PcbDocSpec.boards[0]` in
remove-before-add dependency order.

CLI: `altium spec sync --forward <schdoc-spec> <pcbdoc-spec>` (forward sync),
`altium spec sync --diff <schdoc-spec> <pcbdoc-spec>` (diff only),
`--dry-run` (no file write).

**Validator and resolver (M7):** `validate_schdoc_spec()` / `validate_pcbdoc_spec()`
return `Result<Vec<SpecError>, Vec<SpecError>>` (warnings vs errors). Checks: duplicate
designators, dangling net refs, duplicate annotation IDs, unresolved pin refs (warning).
`resolve_schdoc_spec()` builds designator → footprint map from `SchLibSpec` slices.
Hard errors when a referenced library component cannot be found; bare designators without
library references produce no footprint entry (valid case).

**Constraint & rule extensions (M8):** `constraint <kind> { ... }` blocks are now
parseable inside `sheet { }` blocks. Five typed constraint kinds are supported:
`edge_placement`, `directional`, `near`, `region`, `fixed_position`. Unknown kinds
produce a parse error (typo protection). Annotations (`#[annotation(id = "...")]`) are
accepted before constraint blocks. Compiled constraints surface in `SheetSpec::constraints`
as `Vec<ConstraintSpec>` with `kind: ConstraintKind` and `properties: IndexMap<String, String>`.
`PcbDocRuleSpec` is extended with `properties: IndexMap<String, String>` (freeform
key-value pairs from the rule body) and `scope: Option<String>` (rule scope expression).
The formatter handles constraint blocks in `fmt_sheet_item`. 13 new parser tests cover
all five kinds, empty body, annotation, unknown-kind error, outside-sheet error, rule with
scope, and backward compatibility for existing specs without constraints.

**Pin connections & validated symbol references (M9):** `pin X -> #NET` syntax inside
schdoc component bodies declares pin-to-net connections. The compiler classifies targets
as `Signal`, `Power` (if matching a `power` declaration), or `NoConnect` (`pin X -> nc`).
The executor resolves pin positions from imported SchLib data via `resolve_pin` (name-first,
designator-fallback), transforms orientation (mirror then rotate), and generates Wire stubs
(200mil), NetLabels (signal), PowerObjects (power), or NoConnect markers. `symbol: $alias.Name`
provides compile-time validated symbol references via `Value::ImportRef` provenance tracking —
field access on `Value::ImportObject` returns `ImportRef` instead of `String`, enabling the
compiler to validate that the referenced symbol exists in the imported SchLib. The `Arrow`
(`->`) token, `PinConnectionDecl`/`PinConnectionTarget` AST types, `PinConnectionSpec` model
type, and `SheetSpec::power_declarations` support the full pipeline. CLI threads
`imported_components` from `compile_imported_schlibs` through to `apply_spec_schdoc`.
27 new tests across parser (5), compiler (8), and executor (14).

---

## Known Issues & Gaps

### Critical

1. ~~**IntLib is a stub**~~ — **Fixed.** Read-only IntLib support implemented: `validate`
   and `dump` work. Produces `.schlib-spec` + `.pcblib-spec` from embedded libraries.
   Some older pre-AD6 IntLib files may fail on V5 PcbLib primitive formats.

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

## AutoPCB Placer — Milestone 1: Spec Parser Extensions + Model Updates (2026-03-17)

**Files**: `crates/altium-format-spec/src/{model,ast,lexer,parser,compiler,formatter}.rs`

Extended the spec DSL to support autoplace directives for the upcoming PCB placement solver:

### Lexer (`lexer.rs`)
- Added `Group`, `Separate`, `Autoplace` keyword tokens
- All three also allowed as property keys in objects and as identifier values in expressions

### AST (`ast.rs`)
- Added `PlacementItem::GroupDecl(PlacementGroupDecl)` — `group NAME { components: [...] }`
- Added `PlacementItem::SeparateDecl(PlacementSeparateDecl)` — `separate $a, $b { gap: Nmm }`
- Added `PlacementItem::AutoplaceBlock(Spanned<Object>)` — `autoplace { algorithm: ..., ... }`
- All new AST nodes carry `Span` byte offsets (required by M6 spec rewriter)

### Model (`model.rs`)
- `PlacementSpec`: added `autoplace_config`, `unplaced`, `allow_pin_swap`, `allow_part_swap`, `allow_gate_swap`, `groups`
- `PlacementPlaceSpec`: added `autoplace: bool`, `no_pin_swap: Vec<String>`, `no_part_swap: bool`
- New `UnplacedStrategy` enum: `Autoplace` (default), `Ignore`, `Error`
- New `AutoplaceConfig` struct: `algorithm`, `sa_cooling`, `sa_moves_per_temp`, `sa_max_steps`, `enable_net_crossings`, `default_clearance`, `board_edge_clearance`, `grid_snap`, `auto_cluster`
- New `PlacementGroupSpec` struct: `name`, `components`

### Parser (`parser.rs`)
- `parse_placement`: handles `group`, `separate`, `autoplace` blocks via `Group`/`Separate`/`Autoplace` token kinds
- `parse_placement_group`: parses `group NAME { ... }`
- `parse_placement_separate`: parses `separate $a, $b { ... }` with optional body
- Place body: `autoplace`, `no_pin_swap`, `no_part_swap` properties recognized

### Compiler (`compiler.rs`)
- `compile_placement_decl`: compiles `unplaced`, `allow_*_swap` properties, `AutoplaceBlock` → `AutoplaceConfig`, `GroupDecl` → `PlacementGroupSpec`
- `compile_placement_place`: compiles `autoplace`, `no_pin_swap`, `no_part_swap`
- `compile_autoplace_config`: extracts all 9 config fields from object
- `compile_placement_group`: extracts name and components array

### Formatter (`formatter.rs`)
- `fmt_placement_item`: handles `AutoplaceBlock`, `GroupDecl`, `SeparateDecl`

### Tests: 18 new tests — all passing (262 total, was 244)
- Parser tests: `placement_autoplace_property_in_place_block`, `placement_autoplace_block_full_pipeline`, `placement_autoplace_block_empty`, `placement_unplaced_strategy_autoplace`, `placement_group_decl`, `placement_separate_decl`, `placement_no_pin_swap_in_place_block`, `placement_complete_block_with_all_new_properties`
- Compiler tests: `autoplace_place_flag_compiles`, `autoplace_block_algorithm_compiles`, `autoplace_block_empty_compiles`, `unplaced_strategy_autoplace_compiles`, `unplaced_strategy_ignore_compiles`, `unplaced_strategy_error_compiles`, `unplaced_strategy_invalid_value_produces_error`, `group_decl_compiles`, `no_pin_swap_list_compiles`, `allow_swap_flags_compile`

---

## AutoPCB Placer — Milestone 6: Executor Integration + Spec Rewriter (2026-03-17)

**Files**: `crates/altium-cli/src/spec_rewriter.rs`, `crates/altium-cli/src/main.rs`

Implemented the spec rewriter and `autoplace_spec` orchestrator that closes the loop
from solver output back to an updated `.pcbdoc-spec` file.

### spec_rewriter.rs — AST-based rewriter (updated 2026-03-17)

Public API:
```rust
pub fn rewrite_spec_with_placement(
    original_spec_text: &str,
    result: &PlacementResult,
    autoplace_designators: &[String],
) -> anyhow::Result<RewriteResult>
```

Strategy: AST-based rewriting with trivia (comment) preservation.

1. Parse source via `parse_with_trivia()` → `(SpecFile, TriviaMap)`
2. Find `PlacementDecl` in AST; walk `PlacementItem::Place` children
3. For each `PlaceDecl` with `autoplace: true` and a solvable designator:
   - Build replacement text from original property spans + solver `at:`/`rotation:` lines
   - Re-attach leading trivia (comments before block) and trailing trivia (inline comment on `}`)
   - Record `(span.start, span.end, replacement_text)`
4. Multi-designator blocks expanded to individual blocks; only first block gets leading trivia
5. Unsolved designators in multi-desig blocks: body preserved with `// autoplace: unsolved`
6. Apply replacements in reverse byte order (preserves offsets)
7. Append new blocks for designators not found in any place block
8. Falls back to original text on parse failure; no placement block → output unchanged

Annotations: `// autoplace: solved` and `// autoplace: unsolved` are stable user-facing
markers — downstream tooling may parse these to identify placement status.

### autoplace_spec orchestrator (main.rs)

```rust
pub fn autoplace_spec(
    spec_path: &Path,
    pcbdoc_path: Option<&Path>,
    config: &PlacementConfig,
    dry_run: bool,
    output_path: Option<&Path>,
) -> anyhow::Result<AutoplaceReport>
```

Pipeline: read spec → compile → open PcbDoc → extract IR → build constraints
(via `placement_bridge`) → build `PlacementConfig` (from spec clearance/optimize/autoplace
config + caller-provided overrides) → `solve_placement` → `rewrite_spec_with_placement`
→ write output (unless `--dry-run`).

### CLI command

`altium placement autoplace <spec.pcbdoc-spec>` — run autoplacer, rewrite spec in-place
with solved positions. Flags: `--target`, `--dry-run`, `--output`, `--gamma-start`,
`--gamma-end`, `--max-iters`.

### Tests

13 unit tests in `spec_rewriter::tests` — all passing:
- `autoplace_true_replaced_with_position` — basic rewrite
- `locked_components_unchanged` — locked place blocks preserved verbatim
- `unmentioned_autoplace_components_appended` — appended at end of placement block
- `multi_designator_block_expanded_to_individual` — expansion to individual blocks
- `constraints_and_clearance_blocks_preserved` — non-place content unchanged
- `rotation_nonzero_included_in_output` — non-zero rotation correctly emitted
- `comment_before_place_block_preserved` — leading trivia preserved
- `comment_inside_place_body_preserved` — body properties with comments preserved
- `trailing_comment_on_closing_brace_preserved` — trailing trivia preserved
- `no_placement_block_output_identical_to_input` — edge: no placement block
- `all_components_locked_output_identical_to_input` — edge: nothing to rewrite
- `roundtrip_rewrite_then_reparse` — rewritten output re-parses without errors
- `comment_between_two_place_blocks_preserved` — inter-block comments preserved

---

## AutoPCB Placer — Milestone 5: PlacementSpec → UserConstraint Bridge (2026-03-17)

**File**: `crates/altium-cli/src/placement_bridge.rs`

Implemented the constraint bridge that connects the spec DSL placement model to the
`autopcb-placement` solver's `UserConstraint` format.

### Location rationale

`altium-format-spec` does not depend on `autopcb-placement` and vice versa. The bridge
lives in `altium-cli` (option a from the plan) which already imports both crates.

### Public API

```rust
pub fn placement_spec_to_constraints(
    spec: &PlacementSpec,
    ir: &PcbIr,
) -> anyhow::Result<(Vec<UserConstraint>, Vec<String>)>
```

Returns `(constraints, autoplace_designators)`.

### Constraint mapping

| Spec | Solver |
|------|--------|
| `at:` without `autoplace` | `UserConstraint::FixedPosition` |
| `autoplace: true` | added to autoplace designator list |
| `autoplace: true` + `edge:` | `UserConstraint::EdgePlacement` |
| `autoplace: true` + `near:` + `max_distance:` | `UserConstraint::Near` |
| `autoplace: true` + `region_rect:` | `UserConstraint::RegionContainment` |
| `autoplace: true` + `region_name:` (named preset) | `UserConstraint::RegionContainment` |
| `left_of` / `right_of` / `above` / `below` | `UserConstraint::Directional` |

### Unplaced strategy

- `Autoplace` (default): unmentioned IR components added to autoplace set
- `Ignore`: unmentioned IR components locked at current IR position
- `Error`: error if any IR component not mentioned in spec

### Designator validation

Designators in spec but not in IR: warning emitted + skip (or error if `unplaced: error`).

### Exports

`UnplacedStrategy`, `AutoplaceConfig`, `PlacementGroupSpec` are now exported from
`altium-format-spec/src/lib.rs`.

### Tests

11 unit tests in `placement_bridge::tests` — all passing, no fixture files required.

---

## AutoPCB Placer — Milestone 2: Placement Dump (2026-03-17)

**File**: `crates/altium-format-spec/src/dump.rs`

Added placement dump functionality to the spec DSL dump module:

- `dump_placement_block(out, board)` — public API: emits a `placement { ... }` block from a `PcbDocBoard`
- `dump_placement_block_from_parts(out, components, clearance_gap)` — inner helper testable without constructing a full board
- `designator_key(s)` — numeric-aware sort key: "U2" < "U10" < "U11"
- `format_coord_point_at(pt)` — formats `CoordPoint` as `(Xmm, Ymm)` for `at:` properties
- Clearance: if a `ComponentClearance` design rule with global scope exists, emits `clearance { all: N }`
- 7 new unit tests in `dump::tests` — all passing, no fixture files required

---

## AutoPCB Placer — Milestone 7: Simulated Annealing (Phase 3) (2026-03-17)

**Files**: `crates/autopcb-placement/src/simulated_annealing.rs` (new), `crates/autopcb-placement/src/lib.rs`, `crates/autopcb-placement/Cargo.toml`

Implemented the SA refinement pass (Phase 3) for the autopcb placement pipeline.

### SAConfig

`SAConfig` (serializable) with: `cooling_rate` (0.95), `moves_per_temp` (100), `max_steps` (5000), `initial_acceptance` (0.8), `t_frozen` (0.001), `min_acceptance_steps` (5), `snapshot_interval` (50). Full `Default` impl.

### Internal structures

- `ComponentState` — x/y/rotation/width/height/is_movable + cached pad offsets `(net_idx, local_x, local_y)`
- `SpatialGrid` — `HashMap<(i32,i32), Vec<usize>>` for O(k) AABB overlap checking; `insert`/`remove`/`neighbours`
- `NetComponentIndex` — bidirectional comp↔net mapping built from `PcbIr`; enables incremental HPWL recomputation over only affected nets
- `Move` enum — `Displace { comp_idx, dx, dy }`, `Swap { comp_a, comp_b }`, `Rotate { comp_idx, new_rotation }`

### Algorithm

- `auto_init_temperature`: samples 100 random moves, computes `T₀ = -median_|Δcost| / ln(initial_acceptance)`
- `generate_move`: 50% Displace / 30% Swap / 20% Rotate; displace range proportional to temperature
- `delta_cost`: incremental cost = ΔHPWL (over affected nets only) + ΔAABB-overlap-penalty + Δboard-containment-penalty; all computed without full-placement clone
- Metropolis acceptance: `dc ≤ 0` always accepted; `dc > 0` accepted with prob `exp(-dc/T)`
- Adaptive cooling: `T *= 0.5` if acceptance > 96%, `T *= 0.99` if acceptance < 2%, else `T *= cooling_rate`
- Stopping: `T < t_frozen` OR acceptance < 1% for `min_acceptance_steps` consecutive steps
- Best-tracking: solution with lowest HPWL is preserved; returned result always ≤ input HPWL
- `moves_per_temp == 0`: returns input unchanged (identity pass)

### Integration

- `PlacementConfig` gains `sa_config: Option<SAConfig>` field (serializable, default `None`)
- `solve_placement` calls `refine_with_sa` after Phase 2 legalization when `sa_config` is `Some`
- SA snapshots appended to result's `snapshots` list (phase labels: `"sa_refine"`, `"sa_final"`)
- `rand = "0.9"` added to `autopcb-placement` dependencies

### Tests: 9 unit tests — all passing

- `hpwl_known_4_pin_net` — HPWL = 18.0 for 4 components at corners of 10×8 grid
- `aabb_overlap_detects_overlap` — overlapping AABBs return positive area
- `aabb_overlap_detects_non_overlap` — separated AABBs return 0.0
- `metropolis_high_temp_always_accepts` — exp(-dc/T∞) ≈ 1.0
- `metropolis_zero_temp_rejects_uphill` — exp(-dc/T≈0) ≈ 0.0
- `sa_zero_moves_returns_unchanged` — identity pass verified
- `spatial_grid_insert_remove_query` — insert/remove/query round-trip
- `spatial_grid_exclude_self` — self excluded from neighbour results
- `net_component_index_lookup` — bidirectional index values correct

### Rotation + swap-group follow-up (2026-03-19)

Updated `crates/autopcb-placement/src/simulated_annealing.rs` to make Phase 3
rotation-aware and swap-group-consistent:

- Added rotation-aware world half-extents helper and switched SA overlap/board
  containment cost terms to use world AABBs instead of assuming local width/height
  stay valid after 90° rotation
- `Move::Rotate` now updates the spatial grid and its incremental cost includes
  Δoverlap + Δcontainment, not just ΔHPWL
- Rotate move generation now excludes the component's current rotation, avoiding
  no-op rotate proposals
- `Move::PartSwap` in SA now exchanges full placement state `(x, y, rotation)` so
  part-swap-group moves preserve the orientation of the physical part being
  reassigned, matching the greedy part-swap pass semantics
- Fixed an SA correctness bug where rejected moves were being "reverted" even
  though they had never been applied, which inverted rejected displacements/swaps
- Fixed `cost_after_displace` so geometry penalties still apply to components with
  no nets (previously it returned early and skipped overlap/containment entirely)

### Tests: 13 unit tests in `simulated_annealing::tests` — all passing (22 total in `autopcb-placement`)

- `world_half_extents_swap_on_90_degree_rotation` — 90° swaps the effective AABB axes
- `rotate_delta_cost_includes_rotation_aware_containment` — rotation can improve
  containment cost for elongated parts
- `apply_rotate_updates_spatial_grid_for_new_extents` — accepted rotate moves update
  neighbour queries via the spatial grid
- `part_swap_exchanges_rotation_and_position` — SA part swaps now exchange rotation
  alongside position

---

## AutoPCB Placer — Milestone 8: Pin/Part Swap Optimization (2026-03-17)

**Files**: `crates/autopcb-placement/src/swap.rs` (new), `crates/autopcb-placement/src/lib.rs`, `crates/autopcb-placement/src/simulated_annealing.rs`, `crates/autopcb-ir/src/component.rs`, `crates/autopcb-ir/src/extract.rs`

Implemented greedy pin and part swap optimization passes that run after the analytical solver and optional SA refinement.

### IrComponentPad extensions (`component.rs`)

Added two optional swap ID fields to `IrComponentPad`:
- `swap_id_pin: Option<String>` — pin swap group within a component; pads in the same group are electrically interchangeable
- `swap_id_part: Option<String>` — part swap group across components; components with the same ID have identical pinouts

### Extract (`extract.rs`)

`IrComponentPad` construction now initializes both swap ID fields to `None`. PcbDoc pad records do not carry back-annotated swap group data; swap IDs must be injected externally (e.g. from SchLib parameters) before building a `SwapModel`.

### swap.rs (new)

Public API:
- `build_swap_model(ir: &PcbIr) -> SwapModel` — groups pads and components by swap ID; excludes singleton groups
- `greedy_pin_swap_sweep(placement: &mut PlacementResult, ir: &PcbIr, model: &SwapModel) -> SwapChangelog` — tries all pairwise pad net swaps within each group; accepts improvements; repeats until convergence
- `greedy_part_swap_pass(placement: &mut PlacementResult, ir: &PcbIr, model: &SwapModel) -> SwapChangelog` — tries all pairwise component position swaps within each group; accepts improvements
- `compute_hpwl(placement: &PlacementResult, ir: &PcbIr) -> f64` — exact HPWL from pad world positions
- `compute_hpwl_with_overlay(...)` — HPWL with a net-assignment overlay (used internally by pin sweep)
- `verify_swap_integrity(ir: &PcbIr, before: &HashMap<String, usize>) -> Result<(), SwapError>` — checks net count and per-net pin count unchanged
- `collect_net_pin_counts(ir: &PcbIr) -> HashMap<String, usize>` — baseline snapshot for integrity check
- `write_swap_overlay(changelog: &SwapChangelog) -> String` — generates `.schdoc-spec` overlay text

Pin swap uses a net-assignment overlay (not mutating IR) so HPWL is recomputed without side effects. Part swap exchanges `(x_mm, y_mm, rotation_deg)` in the `PlacementResult`.

### PlacementConfig extensions (`lib.rs`)

- `allow_pin_swap: bool` — enable Phase 4.5 greedy pin sweep (default `false`)
- `allow_part_swap: bool` — enable Phase 2.5 greedy part pass (default `false`)

Wired into `solve_placement`: Phase 2.5 (part swap) runs after legalization; Phase 4.5 (pin sweep) runs after SA or Phase 2 result.

### SA Move types (`simulated_annealing.rs`)

Added two new move types to the SA `Move` enum:
- `PinSwap { comp_idx, pad_a, pad_b }` — swap net indices of two pads on a component; delta cost computed analytically without mutation via `hpwl_for_net_with_swap`
- `PartSwap { comp_a, comp_b }` — exchange positions of two components in the same part swap group

`generate_move` updated: 45% Displace / 25% Swap / 15% Rotate / 7.5% PinSwap / 7.5% PartSwap (when opportunities exist; fallback to Displace when swap lists are empty).

`build_swap_opportunities(ir, comp_designators)` helper builds the SA-internal pin/part swap opportunity lists from IR pad data.

`rebuild_net_index_for_swap` keeps the `NetComponentIndex` consistent after a PinSwap move is applied.

### Tests: 9 unit tests in `swap::tests` — all passing (18 total in autopcb-placement)

- `test_build_swap_model_with_known_groups` — 2-component resistor pair, 2 pin swap groups + 1 part group
- `test_single_pad_group_excluded` — singleton group not in model
- `test_part_swap_runs_without_panic` — swap accepted only if HPWL improves
- `test_pin_swap_runs_without_panic` — HPWL non-increasing after sweep
- `test_verify_swap_integrity_passes_after_no_swaps` — baseline passes
- `test_verify_swap_integrity_detects_pin_count_change` — detects count change
- `test_write_swap_overlay_empty` — "No swaps" message
- `test_write_swap_overlay_with_entries` — overlay contains expected blocks
- `test_collect_net_pin_counts` — correct pin counts from IR

---

## Placer Pipeline Gap Fixes (2026-03-19)

Plan: `docs/plans/placer-gaps.md`

| Milestone | Status | Summary |
|-----------|--------|---------|
| M1: Board6 Merge | Complete | `replace_param_section` → `merge_param_section("DOCUMENTNAME")` preserves ~93KB layer stack |
| M2: Pin→Pad Resolution | Complete | `build_pin_to_pad_map()` in sync.rs resolves pin names to pad designators via SchLib FootprintMapSpec |
| M3: Pad-Net Assignment | Complete | `build_pad_net_map()` discovers sibling schdoc-specs, projects to get pad→net, passes to instantiation |
| M4: Connections6 | Complete | `compute_ratsnest()` generates star-topology ratsnest; `replace_binary_section()` writes Connections6 |
| M5: Footprint Graphics | Complete | `instantiate_footprint_primitives` copies tracks/arcs/fills/texts/regions/bodies from PcbLib |
| M6: SOURCEUNIQUEID | Partial | Fields added to PcbDocComponent, read/write path preserves roundtrip values. NOT YET populated from SchDoc UniqueID data during apply — new components get empty strings. |
| M7: Layer Names | Complete | Component LAYER uses abbreviated "TOP"/"BOTTOM" instead of "TOPLAYER"/"BOTTOMLAYER" |
| M8: Classes6 Members | Complete | Auto-generates "All Components" and "All Nets" classes; uses `merge_param_section("NAME")` |
| M9: Swap Group IDs | Complete | Wires SchLib PinSpec.swap_group → PcbDocPad.swap_id_pin → IrComponentPad.swap_id_pin |
| M10: PrimitiveParameters | Complete | BOM data pipeline: PcbDocComponent.parameters → PrimitiveParameterGroup → write; sync forwards parameters |
| M11: Import-Based Spec | Deferred | Needs RFC — large scope redesign of pcbdoc-spec format |
| M12: UniqueID Rebuild | Complete | `assign_and_rebuild_unique_id_section()` generates UIDs for new pads, rebuilds UniqueIDPrimitiveInformation |

---

## Placer Pipeline Gap Fix — M5: Footprint Graphics Instantiation (2026-03-19)

**File**: `crates/altium-cli/src/main.rs`

Extended the footprint instantiation pipeline to copy all non-pad graphics from PcbLib
footprints into the PcbDoc when `altium apply` is run.

### Changes

- Renamed `instantiate_footprint_pads` → `instantiate_footprint_primitives`
- Added `build_pad_net_map` function (scans sibling `.schdoc-spec` files, projects
  each through `project_schdoc_spec()`, and builds a `(designator, pad_name) → net_name` map)
- Added `transform_point` helper: rotates a footprint-local `CoordPoint` into board space
- Added `transform_contour` helper: maps `PcbContour::to_points()` through `transform_point`,
  returning `Vec<CoordPoint>` for board region/component-body outlines
- Added retain calls to remove component-owned tracks, arcs, fills, texts, regions, and
  component bodies before re-instantiation (idempotent re-apply)
- Added graphics instantiation loop iterating `fp.graphics` after the pad loop:
  - **Track**: transforms start/end; sets `net: None`, `component`
  - **Arc**: transforms center; adds `comp.rotation` to start/end angles
  - **Fill**: transforms corner1/corner2; adds rotation
  - **Text**: transforms location; substitutes `.Designator`/`.DESIGNATOR` with actual designator;
    sets `is_comment: false`, `is_designator: false`
  - **Region**: transforms outline + all holes via `transform_contour`; sets
    `is_board_cutout: false`, `is_keepout: false`
  - **Via**: skipped (handled separately)
  - **ComponentBody**: transforms outline via `transform_contour`

Each instantiated primitive gets a unique ID of the form `{designator}-{type}-{counter}`.

All 24 existing unit tests pass. `cargo check -p altium-cli` clean.

---

## AutoPCB Placer — All Milestones Complete (2026-03-17)

The full autoplacer downstream plumbing (plan: `docs/plans/autopcb-placer.md`) is implemented
across all 9 implementation milestones:

| Milestone | Status | Key deliverable |
|---|---|---|
| M1: Parser Extensions | Complete | `autoplace`, `group`, `separate`, `unplaced` in spec DSL; AST spans |
| M2: Placement Dump | Complete | `dump_placement_block()` — PcbDoc → `.pcbdoc-spec` position export |
| M3: Reconciler Comparison | Complete | `reconcile_pcbdoc()` emits MOVE ECOs for position/rotation changes |
| M4: Viewer File Watch | Complete | `--watch` flag; `notify` watcher on PcbDoc + playback JSON |
| M5: Constraint Bridge | Complete | `placement_spec_to_constraints()` in `altium-cli/placement_bridge.rs` |
| M6: Executor + Rewriter | Complete | `autoplace_spec()` orchestrator; `rewrite_spec_with_placement()` |
| M7: Simulated Annealing | Complete | `refine_with_sa()` with Metropolis acceptance, adaptive cooling |
| M8: Pin/Part Swaps | Complete | `swap.rs`: greedy passes, integrity check, overlay file generation |
| M9: CLI Commands | Complete | `altium placement autoplace/dump/plan/apply` |

### End-to-end pipeline

```
.pcbdoc-spec (partial, autoplace: true)
  → parse (altium-format-spec)
  → placement_spec_to_constraints (altium-cli/placement_bridge.rs)
  → solve_placement (autopcb-placement)
      Phase 1+2: analytical (solverang LM)
      Phase 2.5: greedy part swap (allow_part_swap)
      Phase 3: SA refinement (sa_config)
      Phase 4.5: greedy pin swap (allow_pin_swap)
  → rewrite_spec_with_placement (altium-cli/spec_rewriter.rs)
  → .pcbdoc-spec (at: (x,y) + rotation: N)
  → altium placement apply → .PcbDoc binary
```

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

**File watch (`--watch`):** `autopcb-viewer <file> --watch` sets up a `notify::RecommendedWatcher`
(via `mpsc::channel`) on the spec file, target PcbDoc path, and optional `--playback` JSON path.
On `Modify`/`Create` events the viewer reloads from the spec via `reload_from_spec()`, which
re-parses the spec, opens the PcbDoc via the spec bridge, rebuilds the IR, updates GPU scene
resources, reloads the playback JSON if applicable, and shows a green "Reloaded at HH:MM:SS UTC"
label in the sidebar. Events within 100ms of each other are debounced to a single reload.
`request_repaint()` is called after each reload so egui redraws immediately.

**Spec-centric viewer refactor (2026-03-20):**
- `altium-format` dependency removed from viewer — the viewer never imports PcbDoc or PcbDocBoard
- Viewer accepts ONLY `.pcbdoc-spec` files; rejects raw PcbDoc with a helpful error message
- All PcbDoc access encapsulated in `autopcb-ir::spec_bridge::load_ir_from_spec()` bridge function
- Bridge pipeline: compile spec → open target PcbDoc → apply spec mutations → extract IR → apply placement overrides
- `apply_component_pose` utility moved from viewer to `autopcb-ir::spec_bridge` (shared by bridge and playback)
- New IR primitive types added for rendering: `IrArc`, `IrText`, `IrRegion`/`IrRegionKind`, `IrComponentBody`
- New handle types: `TextId`, `RegionId`, `ComponentBodyId`, `DimensionId`
- `FreeCopperGeometry` now includes `arcs: Vec<IrArc>`
- `IrArc.layer` is `Option<LayerId>` — `None` for non-copper layers
- Region extraction checks `is_board_cutout` flag with priority over `RegionKind`
- `IrDimension` type defined but not yet extracted (Dimension API lacks reference point fields)
- Component-owned copper (tracks/arcs/fills on IrComponent) deferred — see `component.rs` comment

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
- `autopcb-shell <path-to-pcbdoc>` launches IDE shell with sidebar, multi-document editor tabs, bottom panel, and status bar.
- Command palette opens via `Ctrl/Cmd+Shift+P` and routes commands through registry/dispatcher.
- Explorer component/net selection updates shared selection model and highlights corresponding entities in PCB 2D view.
- Layout and panel visibility persist across restarts via eframe storage keys `shell.layout.v1` and `shell.panels.v1`.

Phase 1 hardening (production-readiness):
- [x] Replace single-document model with multi-document workbench data model
- [x] Add typed document identities (`DocumentId`) and tab ordering (`open_editor_tabs`)
- [x] Add active tab semantics (`active_editor_tab`) and command-driven tab activation
- [x] Support mixed document kinds (`Board`, `Spec`) in one editor surface
- [x] Add per-board document view mode (`2D` / `3D`) without global singleton state
- [x] Enforce command enable checks before dispatch (disabled/unknown commands surfaced in output/problems)
- [x] Add keyboard-navigable command palette (filter + up/down + enter)
- [x] Replace hardcoded shortcut handling with command-metadata/default keymap routing
- [x] Add shortcut override persistence (`shell.shortcuts.v1`) and conflict detection
- [x] Add keyboard-shortcuts GUI editor for all registered commands (set/clear/reset + capture mode)
- [x] Replace document-type `match` rendering with provider/factory tab architecture
- [x] Add stable document kind IDs (`document.board`, `document.spec`, `document.keybindings`)
- [x] Add `TabProviderRegistry` and per-document renderer instantiation
- [x] Migrate Board, Spec, and Keyboard Shortcuts tabs to provider-based renderers
- [x] Expand shell tests for dispatch, layout, and tab/document behavior

Phase 1 implementation extension (command/workspace/tab UX):
- [x] Expand command catalog with core `workspace.*`, `file.*`, `view.*`, `editor.*`, `history.*`
- [x] Implement document lifecycle primitives (new/open/save/save-all/revert/close/close-others/reopen-closed)
- [x] Add command-driven tab operations (activate/next/previous/close + close buttons in tab strip)
- [x] Add filesystem-backed explorer in primary sidebar (filter + open file into tab)
- [~] Register split-editor commands in command system (currently scaffolded; split groups not yet wired to multi-pane UI)

Phase 1 automation/control plane:
- [x] Add singleton IPC server for GUI control (`autopcb-shell.sock`)
- [x] Add CLI control commands (`start`, `ping`, `cmd`, `open`, `screenshot`)
- [x] Route IPC requests through existing command bus (`cmd`) and file-open flow
- [x] Add full-window screenshot capture in shell via `ViewportCommand::Screenshot`

Phase 1 UI parity pass (VSCode Dark shell chrome):
- [x] Add tokenized theme module (`ui/theme.rs`) and apply it globally each frame
- [x] Add custom title/menu bar rendering and VSCode-like ordered menu groups
- [x] Add Activity Bar with icon buttons and active-view switching state
- [x] Add icon-rendering module (`ui/icons.rs`) and wire icons into activity bar/explorer/tabs
- [x] Add dedicated tabstrip renderer (`ui/tabstrip.rs`) with icon + dirty marker + close affordance
- [x] Restyle sidebar/bottom panel/status bar toward VSCode dark hierarchy (including blue status bar)
- [x] Extend command registry for activity/status toggles and sidebar panel view commands
- [x] Implement functional split editor groups (right/down) with independent secondary active tab
- [x] Expand top-level menu coverage with `Edit` / `Go` / `Run` / `Terminal` / `Help` command entries

### Milestone 2+: Planned

- [ ] Full command catalog implementation from `docs/gui/commands.md`
- [ ] Command-based undo/redo with inverse model operations
- [ ] File-watcher external change detection + reconciliation flows
- [ ] Job system integration for placement/routing/DRC background execution
- [ ] 3D `PaintCallback` render path migration from placeholder to full scene renderer
- [ ] Spec editor with diagnostics, plan/apply previews, and cross-navigation
