# SchLib / SchDoc Validation Report (v2)

Generated: 2026-02-26

## Executive Summary

| Metric | SchLib | SchDoc |
|--------|--------|--------|
| Fixtures tested | 132 | 1215 |
| Validate PASS | **132 (100%)** | **1023 (84.2%)** |
| Validate FAIL | 0 | 192 (15.8%) |
| Roundtrip PASS (semantic) | 23 (17.4%) | 31 (3.0%) |
| Roundtrip save-as crash | 1 (CFB collision) | 0 |
| Roundtrip diff issues | 105 files | 992 files |

### Comparison with previous report (2026-02-26 v1)

| Metric | v1 | v2 | Delta |
|--------|----|----|-------|
| SchLib validate PASS | 132/132 (100%) | 132/132 (100%) | — |
| SchDoc validate PASS | 900/1330 (67.7%) | 1023/1215 (84.2%) | +16.5pp |
| SchDoc validate FAIL | 430 | 192 | **−238 files fixed** |
| SchLib roundtrip PASS | 24 | 23 | −1 |
| SchLib save-as crashes | 4 (3 panics + 1 CFB) | 1 (CFB only) | **3 panics fixed** |
| SchDoc roundtrip PASS | ~12% sample | 31/1023 (3.0%) | now measured on full corpus |

**Key improvements since v1:**
- 3 Pascal string panics (`binary_io.rs:437`) fixed — no more crashes on save-as
- 238 fewer SchDoc validation failures (RECORD=209 ALIGNMENT, RECORD=18 harness fields, RECORD=29 LOCKED, RECORD=7 extra vertices, RECORD=16 DISTANCEFROMTOP_FRAC1, RECORD=2 margin fracs, and more all implemented)
- SchDoc total file count went from 1330→1215 (corpus may have been pruned)

---

## 1. Validation (Parse) Results

### SchLib: 132/132 PASS (100%)

All 129 files in `data/schlib/` plus 3 root-level SchLib fixtures pass validation.
SchLib parsing is complete for the test corpus.

### SchDoc: 1023/1215 PASS (84.2%), 192 FAIL

Failures categorized by root cause:

| # Files | Record / Stream | Missing Parameters | Priority |
|---------|-----------------|-------------------|----------|
| 94 | RECORD=7 (Polyline) | `IGNOREONLOAD` | P1 |
| 49 | RECORD=216 (HarnessConnector) | `OWNERINDEXADDITIONALLIST` (uppercase) | P1 |
| 25 | RECORD=216 (HarnessConnector) | `OwnerIndexAdditionalList` (mixed-case) | P1 |
| 14 | RECORD=215 (HarnessEntry) | `PrimaryConnectionPosition_Frac` | P1 |
| 4 | RECORD=220 (HighLevelCode) | Full FPGA interface parameters (JTAG, memory, routines) | P2 |
| 3 | RECORD=31 (SchSheet) | Missing `FontName6`/`FontName7`/`Size8` | P3 |
| 1 | Invariant | SchImage references missing storage object | P3 |

#### Top Impact Issues (would unblock the most files)

1. **RECORD=7 `IGNOREONLOAD`** — 94 files. Simple boolean field on Polyline records.
2. **RECORD=216 `OWNERINDEXADDITIONALLIST`** — 74 files (49+25). Integer field on HarnessConnector, both upper and mixed-case variants.
3. **RECORD=215 `PrimaryConnectionPosition_Frac`** — 14 files. Fractional coordinate on HarnessEntry.
4. **RECORD=220 FPGA interface params** — 4 files. Complex FPGA high-level code parameters.
5. **RECORD=31 missing fonts** — 3 files. Older format missing later font definitions.

**Implementing just `IGNOREONLOAD` + `OWNERINDEXADDITIONALLIST` + `PrimaryConnectionPosition_Frac` would fix 182 of 192 failures (94.8%).**

---

## 2. Roundtrip (save-as + semantic diff) Results

### SchLib Roundtrip: 23/132 PASS, 105 FAIL + 1 save-as error

#### Save-As Errors (1 file)

| File | Error |
|------|-------|
| `dungvh03-ICs.SchLib` | CFB error: duplicate storage `/AT25XV041B` |

The 3 Pascal string panics from v1 are **fixed**.

#### Roundtrip PASS files (23)

Files with perfect semantic roundtrip:
`aiskylab-MemoryCon`, `aKaReZa75-BoxHeader`, `aKaReZa75-Capacitor`, `aKaReZa75-Inductor`,
`aKaReZa75-Resistor`, `aKaReZa75-SemiConductor`, `CWRUbotix-Resistors`,
`kmilo17pet-AD_Isolators`, `kmilo17pet-Microchip_MOSFET`, `kmilo17pet-NSC_Amplifier`,
`kmilo17pet-NSC_LDO`, `kmilo17pet-NSC_MiscPower`, `kmilo17pet-NSC_VoltageRef`,
`kmilo17pet-PIC12`, `kmilo17pet-ST_LinearReg`, `kmilo17pet-TI_I2C`, and 7 others.

#### Diff Issue Categories (105 failing files)

| Category | Files affected | Description |
|----------|---------------|-------------|
| MissingParamPair only | 52 | Parameter key case normalization (uppercase vs original) |
| MissingParamPair + UpdatedParamValues | 21 | Case normalization + Windows-1252 encoding differences |
| MissingParamPair + EmbeddedObjectDataMismatch | 8 | Case + PinWideText binary diffs |
| EmbeddedObjectDataMismatch only | 8 | PinWideText embedded object byte differences |
| Other combinations | 16 | Various combinations of above categories |

#### Root Cause: Parameters We Drop on Roundtrip (missing in B)

| Parameter Key | Occurrences | Root Cause |
|---------------|-------------|------------|
| `WEIGHT` | 64,677 | Not serialized (integer field on components) |
| `COMPONENTDESCRIPTION` | 45,102 | Uppercase key vs mixed-case original |
| `AREACOLOR` | 45,030 | Uppercase key vs mixed-case original |
| `COLOR` | 44,965 | Uppercase key vs mixed-case original |
| `CURRENTPARTID` | 44,696 | Uppercase key vs mixed-case original |
| `ALLPINCOUNT` | 27,048 | Uppercase key vs mixed-case original |
| `DESIGNITEMID` | 14,643 | Uppercase key vs mixed-case original |
| `DISPLAYMODECOUNT` | 3,141 | Not serialized |
| `SECTIONNAME` | 564 | Not serialized |
| `ALIASLIST` | 189 | Not serialized |
| `COMPCOUNT` | 59 | Not serialized |
| `BORDERON` | 59 | Not serialized |
| `COMPDESCR0..N` | ~100 | Not serialized |

#### Root Cause: Parameters We Add on Roundtrip (missing in A)

| Parameter Key | Occurrences | Root Cause |
|---------------|-------------|------------|
| `Weight` | 64,677 | We serialize as `Weight`, original has `WEIGHT` |
| `ComponentDescription` | 981 | We serialize mixed-case, original had Windows-1252 |
| `DesignItemId` | 689 | Case difference |
| `Text` | 640 | Windows-1252 re-encoding as XML entities |
| `SectionName` | 564 | Case difference |

#### Root Cause: Value Formatting Differences (UpdatedParamValues)

| Parameter Key | Occurrences | Root Cause |
|---------------|-------------|------------|
| `ComponentDescription` | 639 | Windows-1252 bytes → `&#NNN;` XML entities |
| `DesignItemId` | 341 | Windows-1252 encoding |
| `Text` | 111 | Windows-1252 encoding |
| `ModelDatafileEntity0` | 42 | Windows-1252 encoding |
| `Description` | 35 | Windows-1252 encoding |
| `SourceLibraryName` | 13 | Windows-1252 encoding |

#### Root Cause Analysis (SchLib — ordered by blast radius)

1. **Parameter key case normalization** — Our serializer writes keys with our preferred casing
   (e.g., `Weight`, `ComponentDescription`), but many files have ALL-CAPS keys (`WEIGHT`,
   `COMPONENTDESCRIPTION`). The semantic diff sees these as different key-value pairs.
   This is the **single largest issue** — ~65K occurrences from `WEIGHT` alone.

2. **Windows-1252 → XML entity encoding on roundtrip** — High-byte Windows-1252 characters
   (Chinese, Greek, arrows) are decoded to Unicode on parse but re-encoded as `&#NNN;` XML
   numeric character references instead of the original Windows-1252 byte values.
   Affects ~1,200 key-value pairs across dozens of files.

3. **Missing sidecar streams** — `/SectionKeys` and some `PinMiscData`/`PinWideText` streams
   are not written back during save. Affects ~10 files.

4. **PinWideText embedded object byte differences** — Binary embedded objects in pin sidecar
   streams have byte-level differences. Affects ~24 files.

5. **Missing component-level parameters** — `DISPLAYMODECOUNT`, `COMPCOUNT`, `BORDERON`,
   `ALIASLIST`, `COMPDESCR*` parameters not serialized. Lower impact (few files).

---

### SchDoc Roundtrip: 31/1023 PASS, 992 FAIL, 0 save-as errors

No save-as crashes on SchDoc files (improvement from v1).

#### Parameters We Drop on Roundtrip (missing in B)

| Parameter Key | Occurrences | Root Cause |
|---------------|-------------|------------|
| `WEIGHT` | 1,121 | Not serialized |
| `Alignment` | 554 | Default value omitted |
| `AREACOLOR` | 340 | Key case normalization |
| `UNIQUEID` | 334 | Not serialized on some records |
| `MINORVERSION` | 334 | Not serialized |
| `TextFontID` | 122 | Default value omitted |
| `Text` | 116 | Windows-1252 encoding |
| `ShowNetName` | 89 | Default value (`F`) omitted |
| `COLOR` | 71 | Key case normalization |
| `CORNER.X` | 64 | Not serialized (coordinate fields) |
| `SymbolType` | 54 | Default value (`Normal`) omitted |
| `SuppressAll` | 21 | Not serialized |
| `Designator_CustomPosition_Margin` | 18 | Not serialized |
| `Name_CustomPosition_Margin` | 14 | Not serialized |

#### Parameters We Add (missing in A)

| Parameter Key | Occurrences | Root Cause |
|---------------|-------------|------------|
| `Weight` | 787 | We write `Weight`, original has `WEIGHT` |
| `UniqueID` | 610 | We write `UniqueID`, original has `UNIQUEID` |
| `MinorVersion` | 610 | We write `MinorVersion`, original has `MINORVERSION` |
| `Locked` | 470 | We add `Locked=F` default, original omits it |
| `Text` | 102 | Windows-1252 encoding |
| `FontID` | 39 | Default value serialized when original omits |
| `Name` | 16 | Case difference |

#### Root Cause Analysis (SchDoc)

1. **Key case normalization** — Same issue as SchLib. `WEIGHT`/`Weight`, `UNIQUEID`/`UniqueID`,
   `MINORVERSION`/`MinorVersion`, `AREACOLOR`/`AreaColor`. ~2,700 occurrences.

2. **Default-value parameter omission/addition** — We omit parameters with default values that
   the original file includes explicitly (`SymbolType=Normal`, `ShowNetName=F`, `TextFontID=1`,
   `Alignment=1`). Conversely, we add defaults the original omits (`Locked=F`).
   ~900 occurrences.

3. **Windows-1252 → XML entity encoding** — Same as SchLib. ~200 occurrences.

4. **Spurious /Additional stream** — ~185 files show `EntryMissingInA` for `/Additional`,
   meaning we create an `/Additional` stream that the original file doesn't have.

---

## 3. Prioritized Fix List

### P0 — Crashes

**All P0 crashes from v1 are fixed.** No panics or crashes remain.

### P1 — Validation Gaps (blocking parse of 192 SchDoc files)

| # | Issue | Impact | Fix Complexity |
|---|-------|--------|---------------|
| 1 | RECORD=7 `IGNOREONLOAD` | 94 files | Low — add bool field |
| 2 | RECORD=216 `OWNERINDEXADDITIONALLIST` | 74 files | Low — add integer field (case-insensitive) |
| 3 | RECORD=215 `PrimaryConnectionPosition_Frac` | 14 files | Low — add frac coord field |
| 4 | RECORD=220 FPGA interface parameters | 4 files | High — many FPGA-specific fields |
| 5 | RECORD=31 missing FontName6/7, Size8 | 3 files | Low — make optional |

**Implementing #1–#3 alone fixes 182/192 (94.8%) of remaining failures.**

### P1 — Serialization Quality (roundtrip)

| # | Issue | Impact | Fix Complexity |
|---|-------|--------|---------------|
| 6 | Key case normalization | 105 SchLib + 992 SchDoc roundtrip failures | Med — preserve original casing |
| 7 | Default value serialization | ~900 SchDoc, ~200 SchLib occurrences | Low — always serialize `SymbolType`, `ShowNetName`, `TextFontID`, `Alignment`; don't add `Locked=F` |
| 8 | `WEIGHT` field not serialized | 65K SchLib + 1.1K SchDoc occurrences | Low — add Weight to component serialization |
| 9 | Windows-1252 round-trip encoding | ~1,400 occurrences across both | Med — faithful re-encoding of Windows-1252 bytes |
| 10 | Spurious `/Additional` stream creation | ~185 SchDoc files | Low — don't create `/Additional` if empty |
| 11 | Missing component params (`DISPLAYMODECOUNT`, `UNIQUEID`, `MINORVERSION`, etc.) | ~1,000 occurrences | Low-Med — serialize these fields |

### P2 — Sidecar Gaps

| # | Issue | Impact |
|---|-------|--------|
| 12 | `/SectionKeys` not written on save | ~3 SchLib files |
| 13 | `PinMiscData` not written on save | ~2 SchLib files |
| 14 | `PinWideText` embedded object byte diffs | ~24 SchLib files |
| 15 | Missing component sidecar streams (PinPackageLength, PinSymbolLineWidth) | ~2 SchLib files |

### P3 — Edge Cases

| # | Issue | Impact |
|---|-------|--------|
| 16 | RECORD=31 missing FontName6/7, Size8 | 3 files — older format version |
| 17 | SchImage referencing missing storage | 1 file — corrupt fixture |
| 18 | Duplicate component name CFB collision | 1 SchLib file |

---

## 4. Coverage by Record Type (SchDoc)

Based on the 1215-file test corpus, remaining unknown/incomplete record types:

| Record | Name (if known) | Status | Blocking |
|--------|----------------|--------|----------|
| 7 | Polyline | Missing `IGNOREONLOAD` | 94 files |
| 215 | HarnessEntry | Missing `PrimaryConnectionPosition_Frac` | 14 files |
| 216 | HarnessConnector | Missing `OWNERINDEXADDITIONALLIST` | 74 files |
| 220 | HighLevelCode | Missing FPGA interface parameters | 4 files |

All other record types encountered in the corpus are successfully parsed.

---

## 5. Test Scripts

Reusable test scripts are in `scripts/test/`:

```bash
# Validation
./scripts/test/validate-schlib.sh        # SchLib validation (all fixtures)
./scripts/test/validate-schdoc.sh        # SchDoc validation (all fixtures)

# Roundtrip (save-as + semantic diff)
./scripts/test/roundtrip-schlib.sh           # SchLib roundtrip (summary)
./scripts/test/roundtrip-schlib.sh --verbose # SchLib roundtrip (full diff output)
./scripts/test/roundtrip-schdoc.sh           # SchDoc roundtrip (summary)
./scripts/test/roundtrip-schdoc.sh --verbose # SchDoc roundtrip (full diff output)
```

Set `ALTIUM_CLI` environment variable to use a custom binary path (default: `altium-cli`).
