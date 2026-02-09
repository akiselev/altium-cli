# Validation & Testing Plan

Roadmap for testing and validating altium-cli file format support and CLI
correctness, maximizing what can be done **without** a running copy of Altium
Designer. Four phases, ordered by dependency and value.

Current state: 12 end-to-end scenarios implemented with ~25% average pass rate.
5 critical bugs block ~150+ assertions. Extensive decompiled .NET reference code
available for schematics (AD26-dotnet/). PCB format is Delphi-only and
inaccessible.

## Phase 1: Fix Critical Bugs (Unblocks ~150+ assertions)

Fix the three bugs that block the most downstream work. BUG 1 and BUG 2
together account for nearly all routing failures across scenarios 02-12.

### BUG 2: Pin Location Resolution

**Priority:** Fix first (simplest fix, biggest routing unblock).

**Symptom:** All route commands fail with "No route found". Pin absolute
locations resolve to (0, 0) instead of true schematic positions.

**Root cause location:** `crates/altium-format/src/edit/layout.rs:186-241`

- `get_pin_locations()` (line 186) extracts `_base_x`/`_base_y` from the
  component (lines 198-199) but never uses them (prefixed with `_`).
- `calculate_pin_endpoint()` (line 224) reads `pin.graphical.location_x` raw
  without any component offset.

**Investigation steps:**

1. Check whether `transform_primitive()` in `edit/library.rs:281-348` already
   converts pin coordinates to absolute during component instantiation.
2. If pins remain relative after instantiation, the fix is to add
   `component.location + pin.location` in `get_pin_locations()`.
3. If `transform_primitive` should convert but doesn't, fix it there instead.

**.NET reference:** `FileFormatV5.cs:437` — `ExportPin()` exports `Location.X`
and `Location.Y` as simple coordinates. Component-level `Location.X/Y` is
exported separately, confirming pins store component-relative positions.

**Validation (no Altium needed):**
- Scenario 01 routing should succeed (currently works via net labels, but
  pin-to-pin should also work)
- Scenario 06 routing should succeed (currently fails despite designators
  working)
- All `get_pin_locations()` calls should return non-zero coordinates

### BUG 1: Designator Text Persistence

**Priority:** Fix second (more investigation needed, blocks all designator
assertions).

**Symptom:** After `edit add-component ... R1`, saved SchDoc shows empty
designators. `schdoc components` returns `<none>`. ~150+ test failures trace to
this.

**Root cause location:** `crates/altium-format/src/edit/library.rs:199-278`
(instantiate_component, transform_primitive)

**The serialization chain:** SchDesignator (`records/sch/designator.rs:17`)
→ `#[altium(flatten)]` → SchParameter (`records/sch/parameter.rs:13`)
→ `#[altium(flatten)]` → SchLabel (`records/sch/label.rs:11`)
→ `#[altium(param = "TEXT", default, skip_default)]` (label.rs:31)

**Key finding:** The `skip_default` attribute on SchLabel's `text` field means
if text is empty (String default), the TEXT parameter won't be serialized at
all. The derive macro flatten chain itself is correct (generates proper
`append_to_params` calls).

**Investigation steps:**

1. Dump SchDoc JSON after `add-component` — check if the Designator record's
   `label.text` field is populated or empty.
2. If empty: the bug is in `library.rs` instantiation — text isn't being set
   during `transform_primitive()` or `create_designator()`.
3. If populated: the bug is in serialization — check `ToParams` output for the
   Designator record, verify TEXT= appears in the parameter string.
4. Cross-check with .NET: `SchDataDesignator.cs` (52 lines) — `SetText()` calls
   base `SchDataParameter.SetText()` which stores text via `SchDataLabel.text`.

**.NET reference files:**
- `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.Objects/SchDataDesignator.cs`
- `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.Objects/SchDataParameter.cs`
- `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.Serialization/SchDataSerializerParam.cs` (1085 lines — export field order)

**Validation (no Altium needed):**
- `altium-cli schdoc components design.SchDoc --json` should show designator
  text matching what was passed to `add-component`
- Scenarios 02-12 designator assertions should pass

### BUG 3: import-to-pcb Targets Wrong PCB File

**Priority:** Fix third (simple CLI fix, has existing workaround).

**Symptom:** Uses default `PCB1.PcbDoc` from project template instead of
user-added `board.PcbDoc`. Fails with `Os { code: 2, kind: NotFound }`.

**Root cause location:** `crates/altium-cli/src/commands/prjpcb.rs` — the
`import-to-pcb` command calls `prj.primary_pcb()` which defaults to template
name.

**Fix:** Use explicit `--pcb` argument if provided, fall back to first
PCB document found in the project.

**Validation (no Altium needed):**
- All 12 scenarios' import-to-pcb steps should find the correct PCB file.

---

## Phase 2: Lock In Behavior with Regression Tests

Before expanding coverage, prevent regressions on what already works. Estimated
~12 hours of setup.

### 2.1 Snapshot / Golden-File Testing

**What:** Commit scenario JSON dumps to git. Add CI assertions that re-running
scenarios produces identical JSON output.

**Coverage:** 12 scenarios x 6 file formats = up to 72 file states.

**Location:** `crates/altium-format/tests/golden/` (new directory)

**Mechanism:**
- Each test loads a scenario work file, exports JSON, compares against committed
  baseline.
- Use `#[test]` with `include_str!()` for golden files.
- When a bug fix intentionally changes output, re-generate goldens with a bless
  script (e.g., `cargo test -- --bless` pattern or `BLESS=1 cargo test`).

**Catches:** Accidental serialization changes, field order regressions, default
value drift, coordinate precision bugs.

### 2.2 Corpus Roundtrip Harness

**What:** Automated test that discovers all `*.SchLib`, `*.PcbLib`, `*.SchDoc`,
`*.PcbDoc`, `*.PrjPcb` files in the workspace, runs read → write → read,
reports any parse failures or data loss.

**Coverage:** ~15-20 real files of varying complexity (Synthiam 172 components,
scenario files 1-6 components, blank templates).

**Location:** `crates/altium-format/tests/corpus_roundtrip.rs` (new file)

**Existing assets:**
- `Synthiam.SchLib` (566 KB, 172 components) — root
- `Synthiam.PcbLib` (2.9 MB) — root
- `crates/altium-format/data/blank/` — 3 template files
- `crates/altium-format/data/PCB1.PcbDoc` (160 KB)
- `crates/altium-format/data/Plumo-2D.PcbDoc` (7.6 MB)
- `work/scenario-01/` through `work/scenario-12/` — generated files

**Catches:** Parser crashes, missing field preservation, enum decode failures,
coordinate overflow.

### 2.3 CFB Binary Diff Baselines

**What:** Extend `scripts/diff-ole.py` into CI. For each reference file, assert
that the number of changed streams after roundtrip is <= documented baseline.

**Current baseline (from FILE-FORMAT.md):**
- Synthiam.SchLib: 173 of 176 streams have changes (76 cosmetic property-order
  only, 11 substantive issues remaining)

**Location:** `crates/altium-format/tests/cfb_baselines/` (new directory for
baseline JSON files)

**Mechanism:**
- CI runs: read → write → `python3 scripts/diff-ole.py summary original new
  --json` → compare against baseline.
- Baseline JSON records: `max_changed_streams`, `max_substantive_changes`.
- Any increase in changed streams fails CI.

**Catches:** Unintended binary format drift across releases.

### 2.4 Known-Bug Failing Tests

**What:** Write `#[ignore]` tests reproducing all 5 NOTES.md bugs. Convert to
passing as bugs are fixed. Prevents regression.

**Location:** `crates/altium-format/tests/known_bugs.rs` (new file)

**Tests:**
- `test_bug1_designator_persistence` — create component, instantiate, save,
  reload, check designator text != empty.
- `test_bug2_pin_location_nonzero` — instantiate component, get pin locations,
  assert all coordinates are non-zero.
- `test_bug3_import_to_pcb_finds_board` — create project with board.PcbDoc,
  verify import-to-pcb selects it.
- `test_bug4_pcbdoc_component_save` — add component to PcbDoc, save, reload,
  verify component count > 0.
- `test_bug5_netlist_wire_connectivity` — create wires between pins, extract
  netlist, assert net count > 0.

---

## Phase 3: External Validation

Build a test corpus from open-source projects and cross-validate against other
Altium parsers.

### 3.1 Acquire Test Corpus

Clone or download Altium files from open-source hardware projects:

| Source | Repository | Formats | Priority |
|--------|-----------|---------|----------|
| Celestial Altium Library | `issus/altium-library` | SchLib, PcbLib | Tier 1 |
| AlbertaSat Atlas EPS | `AlbertaSat/ex2_atlas_eps_hardware` | All 6 | Tier 1 |
| AlbertaSat Hyperion Solar | `AlbertaSat/ex2_hyperion_solar_panel_hardware` | All 6 | Tier 1 |
| AlbertaSat Apollo UHF | `AlbertaSat/ex2_apollo_uhf_transciever_hardware` | All 6 | Tier 1 |
| CERN OHWR projects | ohwr.org | SchDoc, PcbDoc, PrjPcb | Tier 2 |
| JLCPCB auto-generated | `gsuberland/altium_jlcpcb_libraries` | SchLib | Tier 2 |
| GitHub API search | `filename:*.SchLib` stars>5 | Mixed | Tier 2 |

**Note on Celestial Library:** This is a database-linked library format. The
test value is in the actual SchLib/PcbLib files (likely dozens of libraries
containing 200k+ component database entries), not 200k individual files.

**Corpus organization:**
```
corpus/                        (gitignored, not committed)
  libraries/
    celestial/*.SchLib, *.PcbLib
    jlcpcb/*.SchLib
  projects/
    albertasat-atlas/           (all 6 formats)
    albertasat-hyperion/
    albertasat-apollo/
  cern/
    atfc/
    rhino/
  metadata.json                (file sizes, sources, record type coverage)
```

**Acquisition script:** `scripts/fetch-corpus.sh` — clones repos, extracts
Altium files, builds metadata index.

### 3.2 Run Corpus Through altium-cli

For every file in the corpus:

1. **Parse without crash** — `altium-cli inspect <file>` exits 0.
2. **Extract record types** — catalog which record types appear in real-world
   files vs. which our parser handles.
3. **Roundtrip test** — read → write → binary diff. Measure delta.
4. **JSON export** — verify `schlib json`, `pcblib json`, etc. produce valid
   JSON.

**Metrics:**
- Parse success rate (target: 100% of corpus)
- Record type coverage (what % of observed record types are handled)
- Roundtrip fidelity (average stream-change count)

### 3.3 Differential Testing Against Other Parsers

Cross-validate by parsing the same files with multiple independent
implementations.

| Parser | Language | Formats | Oracle Quality | Independence |
|--------|----------|---------|----------------|-------------|
| KiCad Altium importer | C++ | All formats | Best | Descended from altium2kicad |
| AltiumSharp | C# | SchLib, PcbLib, SchDoc, PcbDoc | High | Independent (Mark Harris) |
| pyAltiumLib | Python | SchLib, PcbLib | Medium | Independent |
| python-altium (vadmium) | Python | SchDoc | Medium | Independent |
| altium.js | JavaScript | SchDoc | Low-Medium | References python-altium |

**Approach:**
- Parse each corpus file with altium-cli → JSON output.
- Parse same file with AltiumSharp / pyAltiumLib → comparable JSON output.
- Compare key fields: component names, pin counts, net names, layer usage.
- Categorize discrepancies: (a) altium-cli bug, (b) other parser bug, (c)
  intentional difference.

**Caveat on KiCad:** Altium import may require GUI interaction (not fully
CLI-automatable). Plan for semi-manual import followed by programmatic
comparison via KiCad's Python API (`pcbnew` module).

**Caveat on bias:** KiCad's importer descended from altium2kicad. AltiumSharp
and python-altium are independently developed. Use at least two independent
parsers to reduce systematic bias risk.

### 3.4 Property-Based Testing

Use `proptest` crate for randomized invariant testing.

**Priority targets:**
- **Coordinate math:** `dxp_frac_to_coord` <-> `coord_to_dxp_frac` roundtrip
  for random coordinates in range [-10M, +10M] with fractional parts [0, 10000).
- **Record roundtrip:** Generate random field values for SchPin, SchComponent,
  PcbPad — verify `from_params(to_params(x))` preserves all fields.
- **Parameter serialization:** Random ParameterCollection values roundtrip
  through string encoding.

**Location:** Add `proptest` to `[dev-dependencies]` in
`crates/altium-format/Cargo.toml`. Tests in existing test modules alongside
the code they test.

### 3.5 Fuzz Testing

Use `cargo-fuzz` to find parser crashes on malformed input.

**Targets:**
- SchLib parser (CFB → record stream → records)
- PcbLib parser (CFB → binary records)
- Parameter string parser (pipe-delimited key=value)
- Binary record parser (fixed-size + variable fields)

**Strategy:** Fuzz at **record level** within valid CFB containers rather than
mutating entire files. This reaches deeper code paths faster.

**Location:** `fuzz/fuzz_targets/` (standard cargo-fuzz layout)

**CI integration:** Nightly workflow runs `cargo fuzz` for 1 hour on main.
Crashes are filed as issues.

### 3.6 Semantic Constraint Validation

Implement validators that check logical invariants without needing Altium as
oracle.

**Invariants to check:**
- All pins within a component have unique designators
- `owner_index` values point to valid parent records (not out of bounds)
- Component bounding box contains all child primitives
- Wire endpoints connect to pin locations (within grid tolerance)
- Net label positions coincide with wire endpoints
- PCB pad coordinates are within board outline
- Layer assignments are valid layer IDs

**Location:** `crates/altium-format/src/ops/validate.rs` (new module)

### 3.7 Targeted .NET Code Review

Use AD26-dotnet/ decompiled code for **specific questions**, not comprehensive
comparison.

**Methodology:** When corpus testing or differential testing reveals a
discrepancy, look up the specific field in the .NET code.

**High-value reference files:**

| File | Lines | What it tells you |
|------|-------|-------------------|
| `SchDataSerializerParam.cs` | 1085 | Export field order for parameter records |
| `FileFormatV5.cs` | 5575 | Record structure, import/export methods for all sch record types |
| `SchDataComponent.cs` | 1177 | Component instantiation logic, designator handling |
| `SchDataExporterLibraryV5.cs` | ~250 | Library export format (lines 163-250) |

**Critical limitation:** The .NET code covers **schematic serialization** only.
PCB format logic lives in Delphi:
- `Advpcb.dll` (114 MB, native) — PCB engine
- `Altium.PCB.DataModel.dll` (14 MB, native) — PCB data model
- `Altium.PCB.BinaryLoader.dll` (54 MB, native) — PCB binary loader

None of these are decompiled. PCB format questions cannot be answered from the
.NET code.

---

## Phase 4: Prepare for Altium Access

Work that can be started now but requires Altium for final validation.

### 4.1 Fix BUG 5: Netlist Extraction

**Symptom:** `schdoc netlist --json` shows `total_nets: 0` after successful wire
routing.

**Root cause location:** `crates/altium-format/src/ops/schdoc.rs:319-356`

**What's missing:** Wire-to-pin spatial matching, junction detection, wire
segment merging, net name propagation from labels.

**.NET reference:** `Altium.Sch.Core/Altium.Sch.Core.DesignUtils/ConnectionsArrays`
has net extraction logic, but this is algorithmic (graph building), not file
format.

**Approach:** Implement wire connectivity graph from first principles:
1. Build adjacency map: wire endpoint → connected pins (spatial match)
2. Merge wire segments at junctions
3. Propagate net names from net labels
4. Group connected pins into nets

**Can validate without Altium:** Unit test with hand-crafted wire/pin
configurations. Scenario 01 should show >= 1 net after routing.

**Requires Altium for final validation:** Compare netlist output against
Altium's own netlist export for the same schematic.

### 4.2 Document BUG 4: PcbDoc Component Save

**Status:** BLOCKED on Altium access. Cannot fix without reference data.

**Why it's blocked:**
- PCB binary format is entirely in Delphi (see 3.7 limitation above)
- No .NET reference code exists for PCB component serialization
- `io/pcbdoc.rs:509-537` — `save_to_file()` only calls `write_rules()` and
  `write_nets()`; component write is stubbed
- Line 693 in `cmd_import_to_pcb()`: `// Would add component here - for now just count`

**What to capture when Altium is available:**
- [ ] Create a PcbDoc in Altium with 1-3 imported components
- [ ] Hex dump `/Components6/Data` stream
- [ ] Compare against blank PcbDoc's `/Components6/Data`
- [ ] Document component record binary structure (field offsets, sizes, types)
- [ ] Repeat with components on different layers
- [ ] Repeat with components that have been rotated/mirrored

### 4.3 Pre-Write Altium Validation Test Suite

Create test scripts now that will execute when Altium is available.

**Write-path acceptance tests:**
- [ ] Open each scenario's SchLib in Altium — verify all components visible
- [ ] Open each scenario's SchDoc in Altium — verify designators, wires, nets
- [ ] Open each scenario's PcbDoc in Altium — verify rules applied
- [ ] Roundtrip: create file in Altium → read with altium-cli → write back →
      open in Altium → verify no data loss

**Format archaeology tests:**
- [ ] Create component in Altium, toggle each property, observe byte-level
      changes in the file → document field semantics for unknown parameters
- [ ] Save same schematic in AD20, AD22, AD24, AD26 → compare binary structures
      → document version-specific format changes

**Netlist/BOM comparison:**
- [ ] Export netlist from Altium for each scenario schematic
- [ ] Export BOM from Altium for each scenario
- [ ] Save as reference data in `crates/altium-format/tests/altium_reference/`
- [ ] Automated comparison: `altium-cli schdoc netlist` vs Altium's netlist

---

## Validation Capability Matrix

### What CAN be validated without Altium

| Validation | How | Phase |
|------------|-----|-------|
| Parse robustness | Corpus of 10k+ files — does it crash? | 3.2 |
| Roundtrip fidelity | Read → write → binary compare to original | 2.2, 2.3 |
| Cross-parser agreement | KiCad + AltiumSharp + pyAltiumLib comparison | 3.3 |
| Semantic consistency | Pin uniqueness, owner indices, bounding boxes | 3.6 |
| Coordinate math correctness | Property-based testing with proptest | 3.4 |
| Crash resilience | Fuzz testing with cargo-fuzz | 3.5 |
| Regression prevention | Golden-file + CFB diff baselines | 2.1, 2.3 |
| Schematic format understanding | AD26 .NET decompiled code review | 3.7 |
| Bug fix correctness (BUG 1, 2, 3) | Scenario re-runs + unit tests | 1 |
| Netlist algorithm correctness | Unit tests with known topologies | 4.1 |

### What REQUIRES Altium

| Validation | Why | Priority |
|------------|-----|----------|
| Write-path acceptance | Does Altium open our files without errors? | Critical |
| Visual correctness | Do components render in the right positions? | Critical |
| PCB format writing (BUG 4) | Delphi-only, no reference code | Critical |
| Cross-version compatibility | Do files work in AD17/AD19/AD21/AD23/AD26? | High |
| Netlist export comparison | Ground truth for netlist extraction | High |
| BOM export comparison | Ground truth for component enumeration | Medium |
| Electrical validation (DRC/ERC) | Altium's analysis engine needed | Medium |
| Format archaeology | Toggle settings, observe byte changes | Low |

---

## Progress Tracking

### Phase 1 Checklist

- [ ] BUG 2 investigated: determine if pins are absolute or relative after
      transform_primitive
- [ ] BUG 2 fixed: pin locations are non-zero in all scenarios
- [ ] BUG 1 investigated: dump JSON to check if TEXT field is populated
- [ ] BUG 1 fixed: designators persist through save/reload cycle
- [ ] BUG 3 fixed: import-to-pcb finds board.PcbDoc
- [ ] Scenarios re-run: pass rate improves from ~25% to ~60%+

### Phase 2 Checklist

- [ ] Golden-file directory created with baseline JSON for passing scenarios
- [ ] Corpus roundtrip test covers all files in workspace
- [ ] CFB binary diff baseline for Synthiam.SchLib (currently 173/176 changed)
- [ ] Known-bug failing tests written for all 5 NOTES.md bugs
- [ ] CI workflow updated to run new test suites

### Phase 3 Checklist

- [ ] Corpus acquisition: Celestial Library cloned
- [ ] Corpus acquisition: AlbertaSat projects cloned (3-4 repos)
- [ ] Corpus acquisition: GitHub API search finds 50+ additional files
- [ ] Corpus parse success rate measured and documented
- [ ] AltiumSharp cross-validation harness built
- [ ] proptest added to dev-dependencies, coordinate roundtrip tests written
- [ ] cargo-fuzz targets created for SchLib and PcbLib parsers
- [ ] Semantic validator module created (ops/validate.rs)

### Phase 4 Checklist

- [ ] BUG 5 netlist algorithm implemented with unit tests
- [ ] BUG 4 documented with Altium capture checklist
- [ ] Altium validation test scripts pre-written
- [ ] Reference data directory structure prepared
      (`tests/altium_reference/`)

---

## Dependencies

```
Phase 1 (bug fixes)
  ├── BUG 2 (pin location) ─── independent
  ├── BUG 1 (designator) ──── independent
  └── BUG 3 (CLI argument) ── independent

Phase 2 (regression tests) ─── depends on Phase 1
  ├── 2.1 Golden files ──────── needs bug fixes for stable output
  ├── 2.2 Corpus roundtrip ──── independent of 2.1
  ├── 2.3 CFB baselines ─────── independent of 2.1
  └── 2.4 Known-bug tests ───── independent, can start immediately

Phase 3 (external validation) ── depends on Phase 2 (for regression safety)
  ├── 3.1 Corpus acquisition ── independent, can start immediately
  ├── 3.2 Corpus testing ────── depends on 3.1
  ├── 3.3 Differential testing ─ depends on 3.1 + 3.2
  ├── 3.4 Property testing ──── independent
  ├── 3.5 Fuzz testing ──────── independent
  ├── 3.6 Semantic validation ── independent
  └── 3.7 .NET code review ──── triggered by 3.2/3.3 findings

Phase 4 (Altium prep) ──── can start in parallel with Phase 3
  ├── 4.1 Netlist fix ────── independent
  ├── 4.2 PcbDoc docs ────── independent
  └── 4.3 Validation suite ── depends on 4.1 completion
```

Items that can start immediately (no dependencies):
- Phase 1: all three bug fixes (parallel)
- Phase 2.4: known-bug failing tests
- Phase 3.1: corpus acquisition
- Phase 3.4: property-based testing
- Phase 3.5: fuzz testing setup
- Phase 4.1: netlist algorithm
- Phase 4.2: PcbDoc documentation
