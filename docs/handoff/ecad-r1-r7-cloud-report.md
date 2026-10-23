# Report: altium-format R1–R11 for ecadabra (cloud batch 1, lane C)

Date: 2026-10-23. Lane brief: `docs/handoff/ecad-r1-r7-cloud-lane.md`. Previous handoff:
`docs/handoff/ecad-r1-r7.md`.

## Branch

- Branch `claude/ecad-b1-altium-r1-r7`. Base `08dca3e` (lane brief) on top of `d7fef24` (R2 WIP)
  and `18062d6` (the rev ecadabra pins). No rebase, no pull request.
- R1–R7 were finished at `382fd35`. The coordinator then added R8 (ecadabra lane A) and R9–R11
  (lane B's schematic import); those followed. The last code commit is `b327bde`; this report
  update is the commit after it, the branch head.
- Commits, oldest first (each item, then its review fixes):

| Commit | Item |
|---|---|
| `96f3c8a`, `ec417e5` | R2 polygons: tests, finish, review fixes |
| `0ca7949`, `1955742` | R4 ids, component index, flags; review fixes |
| `c5fc8d1` | R3 polygon membership |
| `03ca15d` | R1 Board6 outline |
| `6f1d69b` | R3 and R1 review fixes, plus the region write duplication fix |
| `187b361`, `b7bc2a2` | R6 layer identity; review fixes |
| `3e7f1e3`, `41e45e8` | R5 pad expansion modes; review fixes |
| `a1fda26`, `382fd35` | R7 region contours; review fixes |
| `df371df`, `d14bb09` | R8 designators without SOURCEDESIGNATOR; review fixes |
| `ce41a3d`, `6e59f48`, `3462b4c` | R9 placed display mode; graphic display modes; second-round fixes |
| `6aa7165`, `3462b4c` | R10 reproducible filler UniqueIDs; review nits |
| `381b197`, `b327bde` | R11 PrjPcb legacy code pages; review fixes |

## State per request

All eleven are done. Each item has unit tests, a filtered corpus test (R11 has only unit tests: no
GBK project is in the corpora) and an independent review, and the review findings were fixed or
are listed below as open.

| Request | State |
|---|---|
| R2 polygons | Done |
| R4 ids, component index, flags | Done. `unique_id` is filled for pads only, see Q2 |
| R3 polygon membership | Done |
| R1 Board6 outline | Done |
| R6 layer identity | Done. Returns `V6Layer`, not `CopperLayer` |
| R5 pad expansion modes | Done. Uses `MaskExpansionMode`, not `PadExpansionMode` |
| R7 region arcs | Done |
| R8 designators without SOURCEDESIGNATOR | Done. `designator` stays SOURCEDESIGNATOR-first, see Q6 |
| R9 placed display mode | Done, plus display modes on every component graphic. Spec syntax open, see Q7 |
| R10 reproducible UniqueIDs | Done (deterministic filler, not `Option`), see Q8 |
| R11 PrjPcb code pages | Done as an explicit `open_with_code_page`; Altium's own choice unverified, see Q9 |

### Public API added

Everything is in `altium_format::api` unless another path is given. Units are raw `Coord`.

```rust
// R2: Polygon (Polygons6), fields added by d7fef24/96f3c8a
pub contour: PcbContour,                    // arc-preserving; `vertices` == contour.to_points()
pub polygon_type: PolygonType,              // "Polygon" | "Split Plane"
pub hatch_style: Option<PolyHatchStyle>,    // None on split planes
pub grid_size: Option<Coord>, pub track_width: Option<Coord>,
pub min_prim_length: Option<Coord>, pub arc_approximation: Option<Coord>, // ARCRESOLUTION
pub remove_dead: Option<bool>, pub pour_over: Option<bool>, pub pour_over_style: Option<u8>,
pub use_octagons: Option<bool>, pub remove_islands_by_area: Option<bool>,
pub area_threshold: Option<f64>, pub remove_necks: Option<bool>,
pub neck_width_threshold: Option<Coord>, pub neck_width_from_rule: Option<bool>,
pub shelved: Option<bool>, pub restore_layer: Option<LayerRef>, pub restore_net: Option<String>,
pub ignore_violations: Option<bool>, pub auto_name: Option<bool>,
pub optimal_void_rotation: Option<bool>, pub obey_polygon_cutout: Option<bool>,
pub copper_invalidate: Option<bool>,
pub selected: bool, pub locked: bool, pub primitive_lock: bool, pub polygon_outline: bool,
pub user_routed: bool, pub keepout: bool, pub union_index: i32,

// R4: Track, Arc, Via, Pad (PcbDocPad), Fill, Text, Region
pub unique_id: Option<String>,      // pads only (UniqueIDPrimitiveInformation); 8 uppercase letters
pub component_index: Option<usize>, // index into PcbDocBoard::components
pub flags: PcbFlags,
// R4: ComponentBody (needed to write bodies of repeated designators)
pub component_index: Option<usize>, pub flags: PcbFlags,
// R4: PcbDocComponent
pub unique_id: String, pub locked: bool, pub primitive_lock: bool,

// R3: Track, Arc, Via, Pad, Fill, Region
pub polygon: Option<usize>,         // index into PcbDocBoard::polygons
// R3: Track, Arc
pub plane_split_line: bool,         // stored polygon index 0xFFFE (split-plane boundary line)
// R3 (needed for correct region writes): Region
pub board_region: bool,             // read from the BoardRegions section

// R1: BoardSettings
pub outline: Option<PcbContour>,    // Board6 KINDn/VXn/VYn/CXn/CYn/SAn/EAn/Rn

// R6: altium_format_types::pcb
impl V7Layer  { pub fn legacy_v6(self) -> Option<V6Layer>; pub fn copper_v6(self) -> Option<V6Layer>; }
impl LayerRef { pub fn legacy_v6(&self) -> Option<V6Layer>; pub fn copper_v6(&self) -> Option<V6Layer>; }
pub mod constants { V7_FAMILY_LAYER, V7_GENUS_{SIGNAL,PLANE,MECHANICAL,UTILITY},
    V7_SPECIES_BOTTOM_SIGNAL, V7_SIGNAL_SPECIES_MAX, V7_PLANE_SPECIES_MAX,
    V7_MECHANICAL_SPECIES_V6_MAX, V7_UTILITY_SPECIES_V6,
    PRIMITIVE_INDEX_NONE /*0xFFFF*/, POLYGON_INDEX_PLANE_SPLIT_LINE /*0xFFFE*/ }

// R5: Pad (PcbLib) and PcbDocPad
pub solder_mask_mode: MaskExpansionMode,   // Manual | Rule (never NoMask on read)
pub paste_mask_mode: MaskExpansionMode,
pub solder_mask_bottom_expansion: Option<Coord>, // separate top/bottom expansions
pub solder_mask_from_hole_edge: bool,
// R5: altium_format_types::pcb
impl TCacheState       { pub fn mask_expansion_mode(self) -> MaskExpansionMode; }
impl MaskExpansionMode { pub fn pad_cache_state(self) -> Option<TCacheState>; }

// R7: Region
pub outline_contour: PcbContour,    // arcs kept (ShapeBasedRegions6)
pub hole_contours: Vec<PcbContour>, // holes are stored as vertices only

// R8: PcbDocComponent
pub source_designator: Option<String>, // stored SOURCEDESIGNATOR; `designator` falls back to the
                                       // component's one designator Text when absent

// R9: SchDocComponent
pub display_mode: i32,              // placed DisplayMode, 0 <= mode < max(display_mode_count, 1)
// R9: every schematic graphic (LineGraphic, ..., TextFrameGraphic), SchDoc and SchLib
pub owner_part_display_mode: i32,   // + Graphic::owner_part_display_mode()

// R11: altium_format::project
impl AltiumProject { pub fn open_with_code_page(path, code_page: &str) -> Result<Self>; }
```

Departures from the request texts, and why:
- **R2.** `remaining_params` is not added. CLAUDE.md forbids capturing unknown fields, so every
  key the corpus uses is typed and an unknown key is an error. The pour settings are `Option`
  because split planes omit them, and the type is the existing `PolyHatchStyle`.
- **R6.** `copper_index() -> Option<CopperLayer>` became `copper_v6() -> Option<V6Layer>`. No
  `CopperLayer` type exists, and CLAUDE.md asks for the user's approval before a new type is
  added. `StackLayer` gets no `v6` field: call `stack_layer.layer.copper_v6()`.
- **R5.** The request's `PadExpansionMode` (FromRule | Manual | None) is the existing
  `MaskExpansionMode` (Rule | Manual | NoMask). A pad cache cannot store NoMask, so the readers
  never produce it and the writers reject it.

### Evidence and corpus counts

- **Corpus.** akiselev/altium-cli-test-pcbdoc `6bc5aa7` has 132 files: 94 CFB PcbDocs open, 36
  are ASCII, and 2 CFB files fail to open. PcbLib corpus: akiselev/altium-cli-test-pcblib
  `c5dfdc7`, 40 libraries.
- **Boards that read.** `PcbDoc::open` followed by `board()` reads 79 of 94 at `18062d6`
  (measured with an external probe crate) and 79 of 94 at the branch head (`corpus_board_reads`).
  It is the same set of boards. The other 15 fail on Dimensions6 `TEXTX` both before and after,
  and the test requires that error. No board that read before fails now.
- **Write checks.** Every `update_board` → `board()` check below covers all 79 readable boards.

| Item | Corpus evidence (test) |
|---|---|
| R2 | 1,618 polygons (105 split planes), 25,070 vertices, 523 arc edges: identical to an independent olefile survey. Before, the 79 readable boards had 0 polygon vertices; now they have 16,587. Every record reads → writes → reads to the same typed polygon (`corpus_r2_polygons`). |
| R4 | 14,409 components (395 locked, 49 without primitive lock), 58,719 pad unique ids, 217,419 primitives linked to a component. Identity, flags and component fields are unchanged by `update_board` (`corpus_r4_identity`). |
| R3 | 100,505 stored polygon indices, all in range; 2,448 split-line markers, all on tracks/arcs on plane layers. On readable boards the only pour members are regions (43,631), none with a net of its own. One region in test-via2 is on the top layer inside a bottom-layer pour. Membership is unchanged by `update_board` (`corpus_r3_polygon_membership`). |
| R1 | 94 outlines, 1,037 vertices, 283 arc edges, 1 open outline. The outline is unchanged by `update_board`, and a replaced 3-vertex outline reads back exactly (`corpus_r1_board_outline`). |
| R6 | 484,977 Track/Arc/Fill layer pairs (all in the full V7 form) satisfy `legacy_v6(v7) == v6`. The 350 stack copper layers map to distinct V6 layers, every copper via and pad layer is in the stack, and primitive layers are unchanged by `update_board` (`corpus_r6_layer_identity`). The 53 distinct pairs are a unit test in altium-format-types. |
| R5 | PcbDoc: 44,614 pads on readable boards, 5,192 manual solder / 3,597 manual paste, 5 separate bottom expansions. Cache states and masks are unchanged by `update_board`, and inserting a pad does not shift the others (`corpus_r5_pad_modes`). PcbLib: 80,866 pads, 19,844 manual solder / 5,954 manual paste. No pad has an ExtendedPrimitiveInformation entry, and states are unchanged by `update_footprint` (`corpus_r5_pcblib_pad_modes`). |
| R7 | 192 regions with 1,017 arc edges on readable boards. Every region contour is unchanged by `update_board`, and a moved outline is written (`corpus_r7_region_contours`). |

| R8 | 603 components lack SOURCEDESIGNATOR (565 ebaz4205-fpga, 12 nanjing-linecard, 2 nanjing-linecard-nfc, 7 heron-payload-ssm, 7 stlink-v3-mb1367c, 9 tvws-agc-calib, 1 test-footprint-no-desig); each has exactly one designator text. ebaz4205: 1,912 pads → 565 distinct owner indices, 565 distinct designators. Designators unchanged by `update_board`; a rename without the stored key fails, with it reads back (`corpus_r8_designators`). |
| R9 | SchDoc corpus `6d0ff22`: 1,202 of 1,617 SchDocs open, 1,130 sheets read; all 33,816 components have a valid DisplayMode; 137 on readable sheets are placed in an alternate mode. All 30 such sheets survive `update_sheet` with every component's and child's (8,674 alternate-mode children) display mode unchanged (`corpus_r9_display_mode`). |
| R10 | 1,202 SchDocs opened twice: 1,713,743 record ids identical (`corpus_r10_unique_ids_reproducible`). |
| R11 | Unit tests: strict GBK decode, refused labels, a GBK project with a Chinese document path opens with `open_with_code_page` and `open` points to it. |

### Defects found and fixed on the way

These go beyond the request texts but were needed for correct round trips. All are covered by
the corpus tests above.

- **Duplicated board region.** `update_board` put every region into one section, so the
  BoardRegions region was duplicated into ShapeBasedRegions6. The layer, flags and polygon of
  every later region then shifted by one record (65 of 79 boards).
- **Discarded region edits.** The region writer copied the old contours back, so an edited
  outline was silently discarded. New regions were written into ShapeBasedRegions6 in the vertex
  format, which cannot be read back.
- **NoLayer for V7 layers.** Primitives with a full V7 layer id were written with V6 layer
  NoLayer. Via layers defaulted to Top/Bottom, which turned a blind via into a through via.
- **Pad ids regenerated.** Pad unique ids were regenerated on every save.
- **Component identity reset.** Component `UNIQUEID`, `LOCKED` and `PRIMITIVELOCK` were reset.
  Components6 records merged by designator, so ten `R14`s all inherited the last one's old keys.
- **Flags and membership lost.** Flags were reset on every write and polygon membership was
  dropped.
- **Pad matching.** PcbLib `update_footprint` matched every repeated pad name to the first pad.
  PcbDoc pad fields were preserved by position, so inserting a pad shifted them.
- **Pad modes.** Both writers ignored the pad mask mode.
- **Source designators.** The writer wrote SOURCEDESIGNATOR for every component (adding a key
  absent from 603 records), and a component rename was silently dropped.
- **Schematic display modes.** The SchDoc writer stored DisplayMode 0 for every component and
  OwnerPartDisplayMode 0 for every graphic, merging all modes' bodies (SchLib graphics too).
- **Filler ids.** SchDoc load filled missing UniqueIDs with random ids.

## Tests run (exact commands, counts)

- **Unit tests, final run.** Command:
  `CARGO_TARGET_DIR=/tmp/target-altium CARGO_PROFILE_DEV_DEBUG=0 cargo test --no-fail-fast -p altium-format -p altium-format-spec -p altium-cli -p altium-format-query -p altium-format-types`
  - altium-format lib: 525 passed, 4 failed. The four are `intlib::tests::*`; they read
    `data/intlib`, which was not cloned, and are not gated behind `test-fixtures`. They already
    failed in this lane's first run (`d7fef24` plus the R2 unit tests), so this is
    pre-existing.
  - altium-format-spec: 248 passed. altium-format-query: 73 passed. altium-format-types: 20
    passed. altium-cli: 0 + 1 + 1 passed.
- **Corpus tests.** Each ran filtered to its own name, with the `data/pcbdoc`, `data/pcblib` and
  `data/schdoc` symlinks pointing at the clones. Command:
  `CARGO_TARGET_DIR=/tmp/target-altium-opt CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_DEV_OPT_LEVEL=2 cargo test -p altium-format --features test-fixtures --lib <name>`
  - Names: `corpus_board_reads`, `corpus_r1_board_outline`, `corpus_r2_polygons`,
    `corpus_r3_polygon_membership`, `corpus_r4_identity`, `corpus_r5_pad_modes`,
    `corpus_r5_pcblib_pad_modes`, `corpus_r6_layer_identity`, `corpus_r7_region_contours`,
    `corpus_r8_designators`, `corpus_r9_display_mode`, `corpus_r10_unique_ids_reproducible`.
  - Final run on `b327bde` (end of R1–R11): 12 of 12 passed, counts as in the table. R7–R10
    ran from a worktree at `b327bde` because R12 work had started in the main tree.
  - Opt-level 2 was used because the debug build takes 440 s per test.
  - Temporary survey tests were run and then deleted before commit: V6/V7 pairs, region forms,
    pad cache states, pad separate expansions, designator texts and SchDoc display modes.
- **Other checks.**
  - `cargo check -p altium-format -p altium-format-spec -p altium-cli -p altium-format-query --tests`
    is clean.
  - `cargo clippy -p altium-format -p altium-format-types -p altium-format-spec -p altium-cli`
    gives 80 warnings, against 85 at `18062d6`. The only new one is `too_many_arguments` on
    `primitive_common_for_board`.
  - `clippy --tests` stops on two deny errors in `binary_io.rs` tests (`approx_constant`). These
    are present at the base too.
- No full fixture suite and no proptests were run.

## Review-agent verdicts

| Item | First review | After fixes |
|---|---|---|
| R2 | complete with nits | fixed: vertices/contour divergence, NaN area, LAYER required, KEEPOUT layer name, tighter corpus test, all-record round trip |
| R4 | complete with nits | fixed: positional pad-id fallback, id validation, Components6 merge by UNIQUEID, body flags |
| R3 | complete with nits | fixed: region round trip compared on all boards (this exposed and fixed the duplication defect), constants module, name-based V6 recovery replaced in R6. Open: spec dump still emits pour members (STATUS) |
| R1 | complete with nits | fixed: first Board6 record only, unit test, doc on `None` |
| R6 | **incomplete** (silent layer fallbacks) | fixed: writer and reader errors, stack check, `LayerRef::is_*`, constants, species high byte. Re-review: complete with nits (PcbLib writer has the same fallback, open) |
| R5 | complete with nits | fixed: spec apply sets Manual, changed rule value → Invalid, pad matching by unique id, bottom/hole-edge exposed, pad ExtendedPrimitiveInformation entries rejected |
| R7 | complete with nits | fixed: wrong STATUS line, open contours closed, ISSHAPEBASED for new regions, unit test. Open: listed under gaps |
| R8 | complete with nits | fixed: silent rename now an error, merge drops an old SOURCEDESIGNATOR, PrimitiveParameters header key, ebaz asserts. Open: Q6 |
| R9 | **incomplete** (graphics lost their display mode) | fixed: display mode on every graphic, child check. Second review **incomplete** (spec loses modes; children unchecked against the count): count checked in both writers, SchLib count derived. Open: spec syntax (Q7) |
| R10 | complete with nits | fixed: doc, test. Open: `update_sheet` re-mints ids of re-synthesized records (Q8) |
| R11 | complete with nits | fixed: UTF-16 and non-text labels refused, keys before the first section are an error, doc. Open: CLI flag (Q9) |

## API breaks

- **Struct literals.** These public structs gained fields, so struct literals of them break:
  Polygon, Track, Arc, Via, Pad (PcbDocPad), Fill, Text (PcbDocText), Region, ComponentBody,
  PcbDocComponent, BoardSettings, the PcbLib `Pad`, SchDocComponent and all 13 schematic graphic
  structs.
- **Filled in this repo.** Literals were filled with defaults in
  - altium-format-spec: `executor.rs`, `reconciler.rs`, `dump.rs`;
  - altium-cli: `main.rs`, where footprint instantiation now also sets `component_index`;
  - altium-format: PcbLib tests.
- **Stricter behaviour.** These used to default silently and are now errors:
  - an unknown layer name in parameter records (Polygons6, Components6, Dimensions6);
  - an out-of-range net, component or polygon index;
  - a stored V6/V7 layer pair that disagrees;
  - a writer layer with no V6 form;
  - an unknown or ambiguous designator without `component_index`;
  - a malformed or repeated component unique id;
  - NoMask on a pad;
  - arcs where a section stores vertices only;
  - an ExtendedPrimitiveInformation entry for a pad;
  - a component designator that differs from what the record would store (R8);
  - a schematic display mode outside the component's modes, for the component or a child (R9);
  - a project key line before the first section header (R11).
- **Signature change.** `PcbLib::add_footprint`/`update_footprint` keep their signatures, but
  their internal conversion is now fallible.
- **Spec.** A spec pad with an explicit mask expansion is written Manual. Before, it was
  written Valid (a rule value).
- **Values.** `LayerRef::is_signal`, `is_copper` and the other `is_*` methods now see full V7
  ids. No caller in the workspace uses them on a `LayerRef`.

## Questions (each with a recommendation)

1. **New domain types.** R2's `pour_over_style: Option<u8>` and the `PadExpansionMode` /
   `CopperLayer` names would each need a new domain type, which needs your approval first.
   Recommendation: add a `PolygonPourOverStyle` enum (corpus value 1 only). Keep
   `MaskExpansionMode` and `copper_v6` as they are, because they are existing types.
2. **Unique ids beyond pads.** PrimitiveGuids (18 files) could supply them, but its
   `IndexForSave` mapping is unverified and the AD26 sources were not available here.
   Recommendation: verify it on the dev machine (ghidra/AD26-dotnet) before exposing anything.
   Until then, ecadabra keeps content-derived ids for non-pad primitives.
3. **Unverified meanings.** Three things are inferred from the corpus, not checked against
   decompiled sources:
   - the 0xFFFE polygon index (split-plane line);
   - the V7→V6 table beyond the observed species (signal and plane 10..31/16);
   - TCacheState Valid/Invalid → Rule.

   Recommendation: confirm them against the decompiled sources on the dev machine. These
   constants could not be checked here: `POLYGON_INDEX_PLANE_SPLIT_LINE`, the `V7_*` constants,
   and `V7_UTILITY_SPECIES_V6`.
4. **Open writer gaps** (STATUS.md):
   - component bodies are written in the vertex format, so a ShapeBasedComponentBodies6 section
     with records gets an unreadable record;
   - bodies and texts go to the legacy section when the modern one exists but is empty;
   - Regions6 keeps stale geometry when ShapeBasedRegions6 exists;
   - region internal fields are preserved by position within each section;
   - the PcbLib writer still writes NoLayer/Top/Bottom for layers without a V6 form;
   - the spec dump emits pour members as authored copper.

   Recommendation: one follow-up lane for the body/text section routing and the PcbLib layer
   fallback. They are the same class of defect as the region fixes made here.
5. **`AREATHRESHOLD` precision.** Altium stores it from an 80-bit Extended, and some values
   (e.g. `250015000224.999968`) lose their last digit through f64. Recommendation: accept this,
   as with every other Altium float this crate parses to f64. It is documented and tested.
6. **R8: which designator is `designator`?** It stays SOURCEDESIGNATOR when present (the
   behaviour ecadabra and the spec code rely on), the designator text only as a fallback. But
   about 93 components (heron 71, tvws-agc 21, one test file) print a different designator on
   the board than their SOURCEDESIGNATOR: the board text is what Altium shows on the PCB.
   Recommendation: switch `designator` to the board designator text when exactly one exists
   (keep `source_designator`), as a deliberate change with a spec check, in a follow-up.
7. **R9: display modes in the spec language.** Dump writes no mode for pins or graphics and
   apply builds everything in mode 0, so a dump→apply of a DeMorgan part collapses its modes
   (pins already did before this lane). Recommendation: add `mode N { … }` blocks to the spec
   grammar (a spec-lane task); until then the dump should refuse alternate-mode children.
8. **R10: filler ids.** Fillers are reproducible now, but they are stored in the same `String`
   as stored ids and `save` writes them; and `update_sheet` gives re-synthesized records
   (Designator, SheetName/SheetFileName, Implementation, ImplementationMap,
   HarnessConnectorType) a fresh random id on every write. Recommendation: carry the existing
   ids through `update_sheet` next; expose `Option<String>` only if ecadabra needs to tell
   filled ids from stored ones.
9. **R11: Altium's code page.** A PrjPcb does not record its code page and AD26's project
   reader could not be checked here, so the caller names it (`gbk` for the three GBK
   projects). Recommendation: confirm on the dev machine whether AD26 reads non-BOM projects
   with `GetACP()`; add an altium-cli `--code-page` flag if the CLI must open them.
10. **Pre-existing BOM data loss.** The board reader never fills `PcbDocComponent::parameters`,
    and the writer rebuilds PrimitiveParameters from them, so every `update_board` empties the
    BOM section. Recommendation: read PrimitiveParameters into `parameters` before the next
    writer change.

## What ecadabra's importer must change (per field)

- **R2.** Build zones from `polygon.contour`, not `vertices` (arcs are now kept). Use
  `polygon_type` to separate split planes and `pour_order` (POURINDEX) for priority. Drop the
  `api.polygon-vertices` loss code.
- **R4.**
  - Key pads and primitives to components by `component_index`, not designator, and drop
    `api.component-ambiguous`.
  - Map `PcbDocComponent::locked` and `primitive_lock`, and drop `api.component-locked`.
  - Use `pad.unique_id` (`Some` for corpus pads) for stable pad ids.
  - `flags` carries selected/locked per primitive.
- **R3.** Skip primitives with `polygon.is_some()` as authored copper: they are pour fill and
  take the zone's net, because pour regions store no net. Skip `plane_split_line` tracks/arcs as
  copper.
- **R1.** Take the board outline from `settings.outline` with arcs; it may be open in one corpus
  board. Use `geometry` only for cut-outs.
- **R6.** Compare layers with `copper_v6()`, both for stack layers and for via and pad layers.
  This replaces the workaround that learned V6 names from primitives.
- **R5.** Map `solder_mask_mode`/`paste_mask_mode` (Rule → FromRule, Manual → Manual, with the
  value in `*_expansion`). Use `solder_mask_bottom_expansion` when it is `Some`. Drop
  `api.pad-expansion-mode`.
- **R7.** Build region and keep-out shapes from `outline_contour`/`hole_contours` (arcs kept). Use
  `board_region` to recognise the board shape region.
- **R8.** Read owners through `component_index` (ebaz4205 now has 565 distinct owners) and use
  `designator`, which is no longer empty for components without SOURCEDESIGNATOR.
- **R9.** Use `SchDocComponent::display_mode` to pick the placed mode, and keep only children
  whose `owner_part_display_mode` (graphics and pins alike) equals it.
- **R10.** Ids of records without a stored UniqueID are now stable across loads; no change
  needed beyond dropping any workaround.
- **R11.** Open the three GBK projects with
  `AltiumProject::open_with_code_page(path, "gbk")`; `open` errors on non-UTF-8 files and says
  so.

## Setup as it ran

- **Host.** `vm`, 4 CPUs, 15 GiB RAM, 252 GB disk (30 GB free). No `/mnt/c`.
- **Toolchain.** rustc 1.97.0 (2d8144b78 2026-07-07), cargo 1.97.0.
- **Corpora.**
  - pcbdoc: cloned to `/tmp/corpus/pcbdoc` at `6bc5aa7`, linked as `data/pcbdoc`.
  - schdoc: cloned to `/tmp/corpus/schdoc` at `6d0ff22`, linked as `data/schdoc` (R9, R10).
  - pcblib: cloned to `/tmp/corpus/pcblib` at `c5dfdc7`, linked as `data/pcblib` (`data/` is
    ignored; no corpus file is committed).
- **Baseline.** A worktree of `18062d6` at `/tmp/base-18062d6`, with a probe crate under `/tmp/probe`
  (the before/after board counts).
- **Survey scripts.** `/tmp/tools/{stream,keys,uiddup}.py`, run with python3 -I against olefile
  installed into `/tmp/pyenv`. All outside the repository.
- **Request texts.** Read from ecadabra `docs/spikes/e2-altium-import.md` on its default branch,
  after the repository was added to the session read-only. The `e2-altium-import` branch named in
  the old handoff does not exist on the remote.
- **Builds.** `CARGO_TARGET_DIR=/tmp/target-altium` (debug), `/tmp/target-altium-opt` (fixture
  runs at opt-level 2), `CARGO_PROFILE_DEV_DEBUG=0`.
