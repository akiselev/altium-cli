# Placer Pipeline Gap Fixes: Implementation Plan

## Overview

The `altium spec sync` + `altium apply` + placement solver pipeline produces
PcbDocs that Altium can open but not work with. Components and nets exist as
disconnected stubs — 142 pads and 25 nets in the hub board, but zero connections
between them (HPWL=0, all rotations arbitrary).

**Root cause:** Complete absence of netlist assignment — the schematic defines
which pins connect to which nets, the footprint library defines which pins map
to which pads, but nothing threads this data through to PcbDoc pad records.

This plan addresses all gaps identified in the audit (phase-05/placer-gaps.md),
organized into three implementation waves by dependency order and impact.

## Planning Context

### Decision Log

| Decision | Reasoning Chain |
|----------|----------------|
| Board6 merge before netlist work | One-line fix, prevents data destruction on every apply cycle → unblocks all roundtrip testing → must ship first |
| Pad-net threading through instantiate_footprint_pads | The writer already handles `pad.net = Some("LED_POWER")` → problem is filling the pipe, not plumbing → threading net data into the existing instantiation function is minimal-touch |
| Pin name → pad name resolution via SchLib import chain | SchLib `FootprintMapSpec.maps` already carries `PinPadMap { pin, pad }` → just needs a lookup step during sync projection → no new types needed |
| Connections6 generation after pad-net fix | Connections6 binary records are already parsed (43-byte payload, `BinaryLenRecord` in records.rs) → write path exists (`write_binary_section`) → just need to compute ratsnest topology and populate |
| Star topology for ratsnest (not MST) | Altium itself uses star from a central pad → simpler to implement → matches Altium's own initial ratsnest → MST is an optimization for display, not correctness |
| Footprint graphics as separate milestone from pads | Pads are functional (affect connectivity), graphics are visual (silkscreen, courtyard) → different blast radius → graphics can follow independently |
| Import-based `.pcb` spec deferred to Wave 3 | Correct long-term architecture but large scope → current stringly-typed sync works for the immediate fixes → import architecture eliminates the sync command entirely but requires spec language extensions |
| SOURCEUNIQUEID from SchDoc UniqueIDs | Real Altium ECO populates these from schematic `UNIQUE_ID` fields → we have access during sync from SchDoc spec → just need to propagate through the SyncComponent IR |

### Rejected Alternatives

| Alternative | Why Rejected |
|-------------|-------------|
| Fix pad-net in `.pcb` spec format (add pin syntax) | Treating the spec as carrier of pin-level connectivity conflates placement spec with netlist → pad-net belongs in the apply pipeline, derived from the SchDoc+SchLib import chain |
| Write Connections6 from spec | Ratsnest is derived data (computed from pad-net topology) → should be generated during apply, not declared in specs |
| Fix sync policy to Forward for pins | The sync IR doesn't carry pad designators (only pin names) → enabling Forward would propagate wrong data → must fix pin→pad resolution first |
| Skip Board6 fix, generate from scratch | Board6 is ~93KB of layer stack, grid settings, display prefs → regenerating requires implementing the full V7/V8/V9 layer stack writer → merge is one line |

### Constraints & Assumptions

- `instantiate_footprint_pads` in main.rs is the sole pad creation path for apply
- `pcbdoc_write.rs:board_to_internal()` is the sole PcbDoc write entry point
- `FootprintMapSpec` and `PinPadMap` are already parsed from SchLib specs
- `BinaryLenRecord` / `ConnectionCommonHeader` types exist for Connections6
- `merge_param_section` exists and is already used for Nets6, Components6
- `replace_param_section` is what Board6 currently uses (the bug)
- The `SyncSnapshot` / `SyncPin` IR exists but PcbDoc projection sets `pins: IndexMap::new()`
- `PcbDocBoard` exposes pads with `pad.net: Option<String>` and the writer correctly resolves via `resolve_net_index()`
- SchLib `ComponentSpec.pins` carries pin name and designator
- SchLib `ComponentSpec.footprints[0].maps` carries `PinPadMap { pin, pad }`

### Known Risks

| Risk | Mitigation | Anchor |
|------|-----------|--------|
| SchLib pin designator != pad name for some components | `PinPadMap.maps` handles explicit remapping; implicit 1:1 fallback when maps is empty → document both paths | Wave 1 M2 |
| Board6 merge may clobber fields we add | Match key is `DOCUMENTNAME` → our 5 fields overlay onto the ~93KB original → only conflicts if original has same keys with different values → acceptable (our values are authoritative for those keys) | Wave 1 M1 |
| Connections6 record format may vary across Altium versions | Our parser strictly validates 43-byte payloads → if a file has different sizes, parse fails fast → only write the format we understand | Wave 2 M4 |
| Multi-sheet hierarchical designs have path-qualified UniqueIDs | Start with single-sheet (`\UNIQUEID` format) → extend to hierarchical paths (`Sheet1\UNIQUEID`) when multi-sheet sync is implemented | Wave 2 M6 |
| Pin swap groups not in PcbDoc (must come from SchLib) | Documented in extract.rs comments → Wave 3 M9 adds SchLib parameter lookup during IR extraction | Wave 3 M9 |

---

## Wave 1: Critical Fixes (Unblocks Everything)

Three milestones, strict dependency order: M1 → M2 → M3.
Combined effect: pads get net assignments, HPWL becomes non-zero, placement solver produces meaningful results.

### M1: Board6 Merge Instead of Replace

**Problem:** `replace_param_section(doc, ParamSectionKind::Board6, ...)` at
`pcbdoc_write.rs:156` destroys ~93KB of layer stack, board origin, grid settings,
display prefs, copper weights, dielectric constants — replacing with 94 bytes.

**Fix:** Change to `merge_param_section` with match key `"DOCUMENTNAME"`.

**Files:**
- `crates/altium-format/src/api/pcbdoc_write.rs:156` — one-line change

**Implementation:**
```
replace_param_section(doc, ParamSectionKind::Board6, build_board_record(&board.settings));
→
merge_param_section(doc, ParamSectionKind::Board6, build_board_record(&board.settings), "DOCUMENTNAME");
```

Board6 has a single record (the board itself), matched by `DOCUMENTNAME`.
Our 5 fields (`DOCUMENTNAME`, `SIGNALLAYERCOUNT`, `SNAPGRIDSIZE`,
`VISIBLEGRIDSIZE`, `DISPLAYUNIT`) overlay onto the existing record.
All other fields (layer stack V7/V8/V9, board origin, display prefs, etc.)
are preserved from the original.

**Validation:**
- `altium cfb diff --semantic original.PcbDoc saved.PcbDoc` should show Board6
  differences only for our 5 fields, not EntryMissingInB for the rest
- `altium inspect` should report correct copper layer count

**Tests:**
- Unit test: open a reference PcbDoc with full Board6, apply minimal board spec,
  verify Board6 size is ~93KB (not 94 bytes)
- Semantic diff test: roundtrip reference file, verify no Board6 data loss

---

### M2: Pin Name → Pad Name Resolution

**Problem:** Sync projection stores pin **name** ("IO8") as `SyncPin.designator`,
not pad **number** ("10"). The SchLib's `FootprintMapSpec.maps` contains the
pin→pad mapping but is never consulted.

**Resolution chain:**
```
.sch spec pin "IO8" on net "LED_POWER"
  → SchLib component lookup → pin with name "IO8" has designator "10"
  → FootprintMapSpec.maps → PinPadMap { pin: "10", pad: "10" }
  → pad designator is "10"
  → pad "10" gets net "LED_POWER"
```

**Files:**
- `crates/autopcb-spec/src/sync.rs:219` — pin projection in `project_schdoc_spec`
- `crates/autopcb-spec/src/sync.rs:97` — where FootprintMapSpec.model_name is extracted
- `crates/autopcb-spec/src/model.rs:85-93` — `FootprintMapSpec`, `PinPadMap` types

**Implementation:**

1. In `project_schdoc_spec()`, build a pin-name-to-pad-designator lookup from
   the SchLib's `ComponentSpec`:
   - For each `PinConnectionSpec { pin_name, target }` in the schdoc component:
     - Find the matching `PinSpec` in the SchLib component by name → get pin designator
     - Look up pad name in `FootprintMapSpec.maps` using pin designator
     - If `maps` is empty, use implicit 1:1 mapping (pin designator = pad name)
   - Store as `SyncPin { designator: pad_name, net: Some(net_name) }`

2. The `SyncPin.designator` now contains the **pad designator**, not the pin name.
   This is what the PcbDoc needs — pad records are identified by their designator
   within a component.

**Edge cases:**
- `FootprintMapSpec.maps` empty → implicit 1:1: pin designator IS pad name
- Pin name not found in SchLib → error with context (component, pin name)
- Multiple footprints → use first (primary) footprint mapping

**Requires:** Access to SchLib specs during sync projection. The CLI already has
`imported_components` from `compile_imported_schlibs()` — this must be threaded
to `project_schdoc_spec()`.

**Tests:**
- Unit test: project SchDoc spec with known pin→pad mapping, verify SyncPin
  designators are pad names not pin names
- Unit test: implicit 1:1 mapping when FootprintMapSpec.maps is empty
- Unit test: error when pin name not found in SchLib

---

### M3: Pad-to-Net Assignment in instantiate_footprint_pads

**Problem:** `instantiate_footprint_pads` at `main.rs:1382` hardcodes
`net: None` for every pad. The PcbDoc writer at `pcbdoc_write.rs:105` already
handles `pad.net = Some("LED_POWER")` correctly via `resolve_net_index()`.

**Files:**
- `crates/altium-cli/src/main.rs:1307-1406` — `instantiate_footprint_pads`
- `crates/altium-cli/src/main.rs:1293` — call site

**Implementation:**

1. Extend `instantiate_footprint_pads` signature to accept a pad-net mapping:
   ```
   fn instantiate_footprint_pads(
       board: &mut PcbDocBoard,
       pcblib: &PcbLib,
       pad_net_map: &HashMap<(String, String), String>,  // (designator, pad_name) → net_name
   ) -> anyhow::Result<()>
   ```

2. Build `pad_net_map` from the sync output before calling the function:
   - The sync snapshot (after M2 fix) contains `SyncComponent.pins` where each
     `SyncPin { designator: pad_name, net: Some(net_name) }`
   - Collect into `HashMap<(component_designator, pad_designator), net_name>`

3. In the pad creation loop, look up net:
   ```
   let net = pad_net_map
       .get(&(component.designator.clone(), pad.name.clone()))
       .cloned();
   ```

4. Thread the sync snapshot through the apply pipeline:
   - `apply_spec_to_pcbdoc()` already has access to the compiled spec
   - After sync changes are applied, the resulting snapshot contains pin-net data
   - Pass this to `instantiate_footprint_pads`

**Validation:**
- `altium inspect` on the output PcbDoc should show pads with net assignments
- HPWL in placement solver should be > 0
- `altium cfb diff --semantic` should show Pads6 records with non-0xFFFF net_index

**Tests:**
- Integration test: apply spec with known pin-net connectivity, verify pad.net
  values in resulting PcbDocBoard
- Unit test: verify pad_net_map construction from sync snapshot

---

## Wave 2: Altium Interop (Enables Real Workflow)

Four milestones, partially parallelizable: M4 depends on Wave 1; M5-M7 are
independent of each other and of M4.

### M4: Connections6 (Ratsnest) Generation

**Problem:** No ratsnest lines → no visual connectivity feedback in Altium.

**Files:**
- `crates/altium-format/src/api/pcbdoc_write.rs` — add to `board_to_internal()`
- `crates/altium-format/src/pcbdoc/records.rs:399-418` — existing `BinaryLenRecord` type
- `crates/altium-format/src/pcbdoc/mod.rs:1200-1230` — existing `write_binary_section()`

**Implementation:**

1. After pad-net assignment (Wave 1), compute ratsnest:
   - Group pads by net (skip pads with `net: None`)
   - For each net with ≥2 pads, generate star topology:
     - Pick centroid pad (closest to geometric center of all pads in the net)
     - Create `BinaryLenRecord` for each other pad → centroid pad
   - Single-pad nets produce no connections

2. Build `BinaryLenRecord` for each connection:
   ```
   BinaryLenRecord {
       common: ConnectionCommonHeader {
           layer: V6Layer::MultiLayer,
           flags: 0,
           net_index: <net index as i16>,
           unknown_1: 0,
           component_index: -1,  // 0xFFFF
           polygon_index: -1,    // 0xFFFF
           unknown_2: 0,
       },
       from: pad_a.location,
       to: pad_b.location,
       from_layer: pad_a.layer,
       to_layer: pad_b.layer,
       connection_layer_enum: 0,
       from_layer_enum: 0,
       to_layer_enum: 0,
   }
   ```

3. Add to `board_to_internal()` after primitive sections are written:
   ```
   let connections = compute_ratsnest(&board);
   write_binary_section(doc, BinaryLenSectionKind::Connections6, &connections);
   ```

**Validation:**
- `altium cfb diff --semantic` should show Connections6 populated
- Altium Designer should display ratsnest lines when opening the PcbDoc
- Connection count should equal `sum(pads_per_net - 1)` for all nets with ≥2 pads

**Tests:**
- Unit test: 3-pad net → 2 connections (star from centroid)
- Unit test: single-pad net → 0 connections
- Unit test: no nets → empty Connections6

---

### M5: Footprint Graphics Instantiation

**Problem:** Only pads copied from PcbLib. No silkscreen outlines, courtyard
tracks, pin-1 markers, or assembly drawings. Components are invisible pad
clusters.

**Files:**
- `crates/altium-cli/src/main.rs:1307-1406` — `instantiate_footprint_pads` (rename to `instantiate_footprint_primitives`)
- `crates/altium-format/src/api/pcbdoc_write.rs` — Track, Arc, Text, Fill, Region writers

**Implementation:**

1. Extend `instantiate_footprint_pads` to also copy non-pad primitives:
   - **Tracks** (silkscreen outlines, courtyard): transform start/end coords
   - **Arcs** (rounded corners): transform center coord
   - **Texts** (reference designator): transform location, substitute designator
   - **Fills** (assembly rectangles): transform corner coords
   - **Regions** (courtyard polygons): transform all contour vertices

2. For each non-pad primitive from the footprint:
   - Apply component rotation to coordinates (same transform as pads)
   - Apply component offset
   - Set component association
   - Layer mapping: footprint local layers → board layers (e.g., TopOverlay stays TopOverlay)

3. Rename function to `instantiate_footprint_primitives` to reflect broader scope.

**Validation:**
- Visual: rendered PcbDoc should show component outlines
- `altium inspect` should report tracks/arcs/texts with component associations
- `altium cfb diff` against reference should show similar track/arc counts per component

**Tests:**
- Unit test: footprint with 2 tracks + 1 arc → board gets 2 tracks + 1 arc with correct transforms
- Unit test: rotation transforms applied correctly (90, 180, 270 degrees)

---

### M6: SOURCEUNIQUEID / SOURCEHIERARCHICALPATH Population

**Problem:** Empty source tracing fields → Altium's "Update PCB from Schematic"
can't match components → would add all as duplicates.

**Files:**
- `crates/altium-format/src/api/pcbdoc_write.rs:283-284` — hardcoded empty strings
- `crates/autopcb-spec/src/sync.rs` — SyncComponent IR
- `crates/autopcb-spec/src/model.rs` — SchDocComponentSpec

**Implementation:**

1. Extend `SyncComponent` with source tracing fields:
   ```
   pub source_unique_id: Option<String>,
   pub source_hierarchical_path: Option<String>,
   ```

2. In `project_schdoc_spec()`, populate from SchDoc component data:
   - `source_unique_id`: from `SchDocComponentSpec.unique_id` (prefixed with `\`)
   - `source_hierarchical_path`: from the sheet name or hierarchical path

3. Thread through sync → apply → PcbDocComponentSpec → board_to_internal():
   - Add fields to `PcbDocComponentSpec`
   - `build_component_records()` writes them instead of empty strings

4. Use `merge_param_section` for Components6 (already the case) so existing
   values are preserved when not overwritten.

**Validation:**
- `altium cfb diff --semantic` should show SOURCEUNIQUEID and
  SOURCEHIERARCHICALPATH populated with non-empty values
- Altium "Update PCB from Schematic" should recognize existing components

**Tests:**
- Unit test: sync projection populates source_unique_id from SchDoc spec
- Unit test: component record contains `\UNIQUEID` format string

---

### M7: Layer Name Normalization (TOP not TOPLAYER)

**Problem:** Components written with `LAYER=TOPLAYER` instead of canonical
`LAYER=TOP`.

**Files:**
- `crates/altium-format/src/api/pcbdoc_write.rs:262` — component layer field
- `crates/altium-format-types/src/pcb.rs` — V6Layer display/serialization

**Implementation:**

1. Check how V6Layer serializes layer names — if it already uses canonical
   names, the bug is in `build_component_records()` using a string literal.

2. Fix: Use the V6Layer enum's canonical name (which should be `"TOP"`,
   `"BOTTOM"`, etc.) rather than the alternate alias `"TOPLAYER"`.

This is likely a one-line fix in the component record builder.

**Tests:**
- Unit test: component on top layer serializes as `LAYER=TOP`
- Grep existing test fixtures for `TOPLAYER` vs `TOP` to confirm canonical form

---

## Wave 3: Full Feature Parity (Medium-Term)

Five milestones, independent except where noted. These complete the pipeline
but are not blockers for basic Altium interop.

### M8: Classes6 Member Population

**Problem:** Auto-generated classes have empty member lists. Altium's
class-based selection and rule targeting won't work.

**Files:**
- `crates/altium-format/src/api/pcbdoc_write.rs` — Classes6 section builder

**Implementation:**

1. After components are assigned to the board, build class membership:
   - `"All Components"`: all component designators
   - `"Top Side Components"`: components with `layer == TOP`
   - `"Bottom Side Components"`: components with `layer == BOTTOM`
   - `"Inside Board Components"`: all non-mechanical components
   - Net classes: `"All Nets"` with all net names

2. Write member lists as `M0=C1|M1=R2|...` in class records.

3. Preserve any user-defined classes from the original PcbDoc (use merge, not replace).

**Implementation note:** Currently Classes6 uses `replace_param_section` —
change to `merge_param_section` with match key `"NAME"` to preserve custom classes.

**Tests:**
- Unit test: 3 top-side components → "Top Side Components" class has 3 members
- Unit test: user-defined class preserved after apply

---

### M9: Swap Group ID Population from SchLib

**Problem:** `swap_id_pin` and `swap_id_part` always `None` in `extract.rs:252-253`.
The placement solver's swap infrastructure is fully implemented but has no data.

**Files:**
- `crates/autopcb-ir/src/extract.rs:252-253` — hardcoded None
- `crates/altium-cli/src/main.rs` — apply pipeline

**Implementation:**

1. SchLib components carry pin swap group data in parameters:
   - Pin-level: `SWAPIDPIN` parameter on pin records
   - Part-level: `SWAPIDPART` parameter on component records

2. During `instantiate_footprint_primitives` (post-M5), capture swap IDs from
   SchLib pin/component parameters and attach to PcbDoc pad records.

3. During IR extraction (`PcbIr::extract`), read swap IDs from pad records
   and populate `IrComponentPad.swap_id_pin` / `swap_id_part`.

**Requires:** Understanding SchLib pin swap parameter format — may need
reverse engineering of C# `SchLib.Pin.SwapIdPin` property.

**Tests:**
- Unit test: SchLib component with swap groups → IR pads have swap IDs
- Integration test: placement solver uses swap data (HPWL improves after swap pass)

---

### M10: PrimitiveParameters (BOM Data) Generation

**Problem:** 0 bytes of component parameters in PcbDoc — BOM generation empty.

**Files:**
- `crates/altium-format/src/api/pcbdoc_write.rs` — `board_to_internal()`
- `crates/altium-format/src/pcbdoc/mod.rs:1326-1356` — `write_primitive_parameters_section()`

**Implementation:**

1. Source BOM data from SchDoc component parameters:
   - Value, Tolerance, Manufacturer, MPN, Datasheet, etc.
   - These flow through the sync pipeline as `SyncComponent.parameters`

2. Build `PrimitiveParameterGroup` per component:
   - Component header: `SOURCEDESIGNATOR=R1|COUNT=N`
   - N parameter blocks: one per parameter key-value pair

3. Call `write_primitive_parameters_section()` in `board_to_internal()`.

**Requires:** Extending the sync pipeline to carry component parameters through
to the PcbDoc write. Currently `parameters: SyncDirection::None` — must change
to `Forward`.

**Tests:**
- Unit test: component with 3 parameters → 1 group with COUNT=3
- Integration test: roundtrip preserves parameter data

---

### M11: Import-Based `.pcb` Spec Architecture

**Problem:** Stringly-typed sync with no structural validation. The correct
architecture uses imports that carry the full pin→pad→net data chain.

**Files:**
- `crates/autopcb-spec/src/` — spec language extensions
- `crates/altium-cli/src/main.rs` — sync command replacement

**Implementation:**

1. Extend spec language with `.pcb` spec imports:
   ```
   import "hub.sch" as sch
   import "libs/footprints.sym" as fp
   ```

2. Import resolution derives:
   - Components and designators from `.sch` spec
   - Nets and connectivity from `.sch` spec pin connections
   - Footprint patterns from `.sch` spec → SchLib → `.sym` (footprint) chain
   - Pad-net mapping from the full resolution chain

3. Eliminate explicit component/net declarations in the `.pcb` spec — they're
   derived from imports.

4. The `altium spec sync` command becomes unnecessary — imports ARE the sync.

**This is the largest milestone** and effectively redesigns the `.pcb` spec
format. It should be preceded by an RFC-style design document.

**Scope:** Spec language parser, compiler, executor, reconciler, dump all
need import chain changes.

---

### M12: UniqueIDPrimitiveInformation Rebuild

**Problem:** No entries for new pads → pin-level cross-probing broken.

**Files:**
- `crates/altium-format/src/api/pcbdoc_write.rs` — `board_to_internal()`

**Implementation:**

1. When new pads are added, generate UniqueIDPrimitiveInformation entries:
   - Map pad index → unique ID (generate Altium-format 8-char IDs)
   - Write to the UniqueIDPrimitiveInformation section

2. Preserve existing entries for pads that haven't changed.

**Requires:** Understanding the UniqueIDPrimitiveInformation binary format.

---

## Dependency Graph

```
Wave 1 (Critical — sequential):
  M1 Board6 merge
    → M2 Pin→Pad resolution
      → M3 Pad-net assignment

Wave 2 (Interop — parallel after Wave 1):
  M4 Connections6 ←── depends on M3
  M5 Footprint graphics (independent)
  M6 SOURCEUNIQUEID (independent)
  M7 Layer name fix (independent)

Wave 3 (Feature parity — parallel, independent):
  M8  Classes6 members
  M9  Swap group IDs ←── depends on M5 (footprint primitives)
  M10 PrimitiveParameters ←── needs sync parameter forwarding
  M11 Import-based spec ←── large scope, needs RFC first
  M12 UniqueID rebuild
```

## Verification Plan

### After Wave 1 (M1-M3 complete):
```bash
# 1. Apply spec to PcbDoc
altium apply hub.pcb --target template.PcbDoc --output hub.PcbDoc

# 2. Verify pad-net assignment
altium query hub.PcbDoc "pad[net!='']" --count
# Expected: >0 (was 0 before)

# 3. Verify Board6 preservation
altium cfb diff --semantic template.PcbDoc hub.PcbDoc --stream Board6
# Expected: only our 5 fields differ, not 93KB missing

# 4. Run placement solver
altium placement autoplace hub.pcb --target hub.PcbDoc
# Expected: HPWL > 0, non-zero rotations

# 5. Inspect placement result
altium placement dump hub.PcbDoc
# Expected: components at non-trivial positions
```

### After Wave 2 (M4-M7 complete):
```bash
# 6. Verify Connections6
altium cfb blocks hub.PcbDoc Connections6/Data
# Expected: non-zero record count

# 7. Verify graphics
altium query hub.PcbDoc "track[component!='']" --count
# Expected: >0 silkscreen/courtyard tracks

# 8. Verify source tracing
altium query hub.PcbDoc "pcbdoc_component[sourceuniqueid!='']" --count
# Expected: all components have non-empty SOURCEUNIQUEID

# 9. Open in Altium Designer
# Expected: ratsnest visible, component outlines visible, DRC finds no missing connections
```

## Size Estimates

| Milestone | Files touched | Approximate scope |
|-----------|--------------|-------------------|
| M1 Board6 merge | 1 | 1 line change + tests |
| M2 Pin→Pad resolution | 2-3 | ~80 lines new logic + plumbing |
| M3 Pad-net assignment | 2 | ~40 lines signature change + lookup |
| M4 Connections6 | 2 | ~120 lines ratsnest computation + write |
| M5 Footprint graphics | 2 | ~200 lines coordinate transforms + primitive copying |
| M6 SOURCEUNIQUEID | 3-4 | ~60 lines field threading |
| M7 Layer name | 1 | ~5 lines |
| M8 Classes6 | 1 | ~80 lines member list building |
| M9 Swap groups | 3 | ~100 lines SchLib parameter extraction |
| M10 PrimitiveParameters | 3 | ~120 lines BOM data pipeline |
| M11 Import-based spec | 8+ | RFC + major refactor |
| M12 UniqueID rebuild | 2 | ~60 lines ID generation |
