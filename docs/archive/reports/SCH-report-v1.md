# SchLib / SchDoc Validation Report

Generated: 2026-02-26

## Executive Summary

| Metric | SchLib | SchDoc |
|--------|--------|--------|
| Fixtures tested | 132 | 1330 |
| Validate PASS | **132 (100%)** | **900 (67.7%)** |
| Validate FAIL | 0 | 430 (32.3%) |
| Roundtrip PASS (semantic) | 24 (18.2%) | ~12% (sample) |
| Roundtrip SAVE-AS crash | 3 panics + 1 error | ~28% (sample: parse failures) |
| Roundtrip diff issues | 101 files | ~60% of saveable files |

---

## 1. Validation (Parse) Results

### SchLib: 132/132 PASS (100%)

All 129 files in `data/schlib/` plus 3 root-level SchLib fixtures pass validation.
SchLib parsing is complete for the test corpus.

### SchDoc: 900/1330 PASS (67.7%), 430 FAIL

Failures categorized by root cause (each file fails on first error):

| # Files | Record | Missing Parameters | Priority |
|---------|--------|-------------------|----------|
| 145 | RECORD=209 (Hyperlink) | `ALIGNMENT`, `COLLAPSED`, `LineWidth` | P1 |
| 85 | RECORD=18 (Port) | `HARNESSTYPE`, `AUTOSIZE`, `BorderWidth` | P1 |
| 84 | RECORD=7 (Polyline) | `EXTRALOCATIONCOUNT` + `EXn/EYn_FRAC` (n>50) | P2 |
| 29 | RECORD=29 (Junction) | `LOCKED` | P1 |
| 16 | RECORD=16 (SheetName) | `DISTANCEFROMTOP_FRAC1` | P1 |
| 14 | RECORD=2 (Component) | `Name_CustomPosition_Margin_Frac`, `Designator_CustomPosition_Margin_Frac` | P1 |
| 9 | /Storage | embedded object UTF-8 decode error | P2 |
| 8 | RECORD=218 | Unknown record type | P1 |
| 8 | RECORD=215 | Unknown record type | P1 |
| 6 | RECORD=31 (SchSheet) | `TemplateVaultGUID/ItemGUID/RevisionGUID/VaultHRID/RevisionHRID` | P1 |
| 5 | RECORD=45 (Implementation) | `ModelDatafileEntity1`, `ModelDatafileKind1` | P1 |
| 4 | RECORD=27 (TextFrame) | `UNDERLINECOLOR` | P1 |
| 4 | RECORD=31 (SchSheet) | Missing `FontName6`/`FontName7`/`Size8` | P3 |
| 3 | RECORD=34 (Designator) | `NOTALLOWLIBRARYSYNCHRONIZE` | P1 |
| 2 | RECORD=220 | Unknown record type | P1 |
| 2 | RECORD=225 | `COLLAPSED` | P1 |
| 2 | RECORD=26 (PowerPort) | `UNDERLINECOLOR` | P1 |
| 1 | RECORD=25 (NetLabel) | `SelectionMemory` | P1 |
| 1 | RECORD=15 (SheetEntry) | `ShowHiddenFields` | P1 |
| 1 | Invariant | SchImage references missing storage object | P3 |

#### Top Impact Issues (would unblock the most files)

1. **RECORD=209 (Hyperlink) `ALIGNMENT`** — 145 files. Simple enum field (text alignment).
2. **RECORD=18 (Port) `HARNESSTYPE`/`AUTOSIZE`/`BorderWidth`** — 85 files. Port harness support.
3. **RECORD=7 (Polyline) extra location points >50** — 84 files. Parser caps at 50 vertices.
4. **RECORD=29 (Junction) `LOCKED`** — 29 files. Simple boolean field.
5. **Unknown records 215, 218, 220** — 18 files. New record types in `/Additional`.

---

## 2. Roundtrip (save-as + semantic diff) Results

### SchLib Roundtrip: 24/132 PASS, 105 FAIL + 4 SAVE-AS errors

#### Save-As Crashes (4 files)

| File | Error |
|------|-------|
| `Custom.SchLib` | **PANIC**: Pascal string too long: 346 bytes (max 255) |
| `kmilo17pet-Maxim_Power.SchLib` | **PANIC**: Pascal string too long: 355 bytes (max 255) |
| `ryankurte-electronpowered.SchLib` | **PANIC**: Pascal string too long: 336 bytes (max 255) |
| `dungvh03-ICs.SchLib` | CFB error: duplicate storage `/AT25XV041B` |

**P0 Bug**: 3 panics in `binary_io.rs:437` — Pascal string serialization hits 255-byte limit.
This is a crash (not a graceful error) and must be fixed. Likely caused by long component
names or descriptions being serialized as Pascal strings when they should use a different
encoding for strings >255 bytes.

**P1 Bug**: Duplicate component names in `dungvh03-ICs.SchLib` cause CFB storage collision.

#### Semantic Diff Issues (101 files with diffs)

Issue categories across all failing files:

| Category | Cause | Fix Priority |
|----------|-------|-------------|
| **MissingParamPair** (case normalization) | We serialize ALL UPPERCASE keys but original files use mixed-case (e.g., `AllPinCount` → `ALLPINCOUNT`). Since semantic diff is case-sensitive on keys, this shows as missing/added pairs. | P1 — preserve original key casing |
| **MissingParamPair** (encoding) | Windows-1252 high bytes (Chinese chars, Ω, →) are decoded then re-encoded as `&#NNN;` XML entities instead of original Windows-1252 bytes | P1 — round-trip encoding faithfully |
| **EntryMissingInB** (`/SectionKeys`) | `/SectionKeys` stream not written on save | P2 |
| **EntryMissingInB** (`PinMiscData`) | Component PinMiscData sidecar not written | P2 |
| **EmbeddedObjectDataMismatch** | PinWideText binary embedded objects have byte differences (case change at offset 8) | P2 |
| **BinaryBlockMismatch / BlockLengthMismatch** | Pin binary block serialization differs (e.g., FPGA_Xilinx components with many pins) | P2 |
| **UpdatedParamValues** | Value formatting differences for same key (related to encoding diffs above) | P1 (same root cause as encoding) |

#### Root Causes (ordered by blast radius)

1. **Parameter key case normalization** — Our serializer uppercases all parameter keys. Files
   with mixed-case keys (common in newer Altium versions) produce massive diff counts. Example:
   `animevietsub-Schlib1.SchLib` shows 188,831 issues, all from case changes.

2. **Windows-1252 → XML entity encoding on roundtrip** — High-byte Windows-1252 characters
   (Chinese, Greek, arrows) are decoded to Unicode on parse but re-encoded as `&#NNN;` XML
   numeric character references instead of the original Windows-1252 byte values. Altium stores
   both a Windows-1252 version and a `%UTF8%`-prefixed UTF-8 version of strings; we're
   corrupting the Windows-1252 copy.

3. **Missing sidecar streams** — `/SectionKeys` and some `PinMiscData` streams are not
   written back during save.

### SchDoc Roundtrip (20-file sample): ~12% clean, ~60% have diff issues

Of files that pass validation and can be saved:

| Missing Param | # Occurrences | Notes |
|---------------|---------------|-------|
| `SymbolType=Normal` | 19 | RECORD=2 (Component) — default not serialized |
| `ShowNetName=F` | 12 | Power Port — default not serialized |

Both are cases where our serializer omits parameters with default values, but the original
file includes them explicitly.

---

## 3. Prioritized Fix List

### P0 — Crashes

| # | Issue | Impact | Files |
|---|-------|--------|-------|
| 1 | Pascal string panic in `binary_io.rs:437` (strings >255 bytes) | Crash on save-as | 3+ SchLib |

### P1 — Validation Gaps (blocking parse of 430 SchDoc files)

| # | Issue | Impact | Fix Complexity |
|---|-------|--------|---------------|
| 2 | RECORD=209 `ALIGNMENT` | 145 files | Low — add enum field |
| 3 | RECORD=18 `HARNESSTYPE`/`AUTOSIZE`/`BorderWidth` | 85 files | Low-Med — port harness fields |
| 4 | RECORD=7 extra vertices (>50 locations) | 84 files | Med — dynamic vertex array |
| 5 | RECORD=29 `LOCKED` | 29 files | Low — add bool field |
| 6 | Unknown records 215, 218, 220 | 18 files | Med — new record implementations |
| 7 | RECORD=16 `DISTANCEFROMTOP_FRAC1` | 16 files | Low — add coord frac field |
| 8 | RECORD=2 `*_CustomPosition_Margin_Frac` | 14 files | Low — add frac fields |
| 9 | /Storage UTF-8 decode error | 9 files | Med — encoding fallback |

### P1 — Serialization Quality

| # | Issue | Impact | Fix Complexity |
|---|-------|--------|---------------|
| 10 | Key case normalization (uppercase all keys) | 101 SchLib roundtrip failures | Med — preserve original casing |
| 11 | Windows-1252 round-trip encoding | 19+ SchLib files | Med — faithful re-encoding |
| 12 | Default-value parameter omission (`SymbolType`, `ShowNetName`) | ~60% SchDoc roundtrips | Low — always serialize these |

### P2 — Sidecar Gaps

| # | Issue | Impact |
|---|-------|--------|
| 13 | `/SectionKeys` not written on save | EntryMissingInB in roundtrips |
| 14 | `PinMiscData` not written on save | EntryMissingInB in roundtrips |
| 15 | PinWideText embedded object byte diffs | EmbeddedObjectDataMismatch |

### P3 — Edge Cases

| # | Issue | Impact |
|---|-------|--------|
| 16 | SchSheet missing FontName6/7, Size8 | 4 files — older format version |
| 17 | SchImage referencing missing storage | 1 file — corrupt fixture |
| 18 | Duplicate component name CFB collision | 1 SchLib file |

---

## 4. Coverage by Record Type (SchDoc)

Based on the 1330-file test corpus, here are the unknown record types:

| Record | Name (if known) | Status |
|--------|----------------|--------|
| 215 | Unknown | Not implemented |
| 218 | Unknown | Not implemented |
| 220 | Unknown | Not implemented |
| 225 | Partially known | Missing `COLLAPSED` param |

All other record types encountered in the corpus are successfully parsed (when their
parameters are within the implemented set).

---

## 5. Test Commands Used

```bash
# Validation
altium-cli validate <file>

# Roundtrip
altium-cli save-as original.SchLib roundtripped.SchLib
altium-cli cfb diff --semantic original.SchLib roundtripped.SchLib

# Verbose diff
altium-cli cfb diff --semantic --verbose original.SchLib roundtripped.SchLib
```
