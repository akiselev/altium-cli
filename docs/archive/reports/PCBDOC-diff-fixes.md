# PcbDoc Support: Remaining Fixes and Implementation Gaps

Generated 2026-02-25 by auditing `docs/pcbdoc/`, the implementation in
`crates/altium-format/src/pcbdoc/`, and running `altium validate` across all 132
test fixtures in `data/pcbdoc/`.

## Executive Summary

**0 of 132 test fixtures pass `altium validate`.**

| Error Category | Files | Severity |
|---|---|---|
| Text `reserved_zero` assertion failures in `/Texts/Data` | 88 | **Blocking** |
| Non-CFB (ASCII V5 text format) files | 36 | Low (old format) |
| Text `advance_reserved` assertion failure | 6 | **Blocking** |
| Missing `/FileHeaderSix` stream (V5 binary format) | 2 | Low (old format) |

The two blocking errors are in the same code path (`parse_text_subrecords` in
`pcbdoc/primitives.rs`) and prevent opening **all 94 valid V6 CFB files**.

---

## 1. Blocking Parsing Issues

### 1.1 Text primitive `reserved_zero` assertions are wrong (88 files)

**File:** `crates/altium-format/src/pcbdoc/primitives.rs:500,504,531`

The `parse_text_subrecords` function asserts that certain bytes in the text
binary record are zero, but they contain valid data in real files:

- **Line 500:** `read_reserved_zero(1)` — barcode flag byte. Value is `0x01` in
  the legacy `/Texts/Data` compatibility records.
- **Line 504:** `read_reserved_zero(5)` — barcode tail bytes. Contains non-zero
  values like `01 01 00 00 01`.
- **Line 531:** `read_reserved_zero(1)` — extended header byte. Value is `0x01`
  in records with layer enum index data.

**Error message:** `parsing /Texts/Data: Invalid parameter value for key
'reserved bytes at offset 158': expected 1 zero bytes, got [01]`

**Fix:** These bytes are not reserved — they are real fields that need to be
reverse-engineered from the Delphi/C# source and given proper names and types.
The barcode flag and extended header byte likely control format versioning or
optional features.

### 1.2 Text primitive `advance_reserved` assertion is wrong (6 files)

**File:** `crates/altium-format/src/pcbdoc/primitives.rs:509-515`

In older V6 files (fingerprint-lock-v1, nanjing-linecard, school-adda, etc.),
the "advance reserved byte" at line 509 is `0x37` (ASCII '7'), not zero.

**Error message:** `parsing /Texts/Data: Invalid parameter value for key
'advance reserved byte': expected 0, got 0x37`

**Fix:** Same root cause — the byte is not reserved. Reverse-engineer the field
from the Delphi source.

---

## 2. Unimplemented Primitive Parsers

Three primitive types return hard errors when non-empty sections are encountered.
Since all V6 files fail on Text parsing (issue 1.1) before reaching these, the
actual frequency is unknown but they will block once Text is fixed.

### 2.1 Via parser not implemented

**File:** `crates/altium-format/src/pcbdoc/primitives.rs:240-243`

Returns: `"Via parsing is not implemented without raw payload passthrough"`

**Impact:** Vias6/Data is non-empty in virtually all PCB designs. This will block
nearly all files once Text parsing is fixed.

**Documented binary layout:** `binary-primitives.md` — 316 bytes fixed size with
common header + location + hole_size + per-layer diameter arrays + thermal relief
+ solder mask + stack mode fields.

### 2.2 Region parser not implemented

**File:** `crates/altium-format/src/pcbdoc/primitives.rs:249-252`

Returns: `"Region parsing is not implemented without raw payload passthrough"`

**Impact:** Affects Regions6, ShapeBasedRegions6, and BoardRegions sections.
BoardRegions is present in 94/94 V6 files and often non-empty.

**Documented binary layout:** `binary-primitives.md` — variable-length with
common header + region_kind:u8 + vertex_count:i32 + vertex array (8 bytes each).

### 2.3 ComponentBody parser not implemented

**File:** `crates/altium-format/src/pcbdoc/primitives.rs:253-257`

Returns: `"ComponentBody parsing is not implemented without raw payload passthrough"`

**Impact:** Affects ComponentBodies6 and ShapeBasedComponentBodies6. Present in
all V6 files, non-empty when the design has 3D models.

**Documented binary layout:** `binary-primitives.md` — variable-length with
outline vertices + 3D model reference (GUID) + standoff height + rotation offsets.

---

## 3. Unrecognized CFB Storages

The parser returns a hard error for any unrecognized storage name. The following
storages exist in test fixtures but have no handler:

### 3.1 High-frequency (present in >10% of V6 files)

| Storage | Files (of 94) | Category |
|---|---|---|
| `ConstraintManager` | 61 | Design constraint system |
| `PrimitiveGuids` | 18 | Sidecar (documented in `sidecar-streams.md`) |
| `UnionFeatures` | 15 | Union/grouping features |
| `LettersGeometry` | 14 | Text geometry cache |
| `TClearanceViolation` | 11 | DRC violation storage |
| `TSilkToSilkClearanceViolation` | 10 | DRC violation storage |
| `TSilkToSolderMaskClearanceViola` | 9 | DRC violation storage |
| `TRoutingViaStyleViolation` | 8 | DRC violation storage |
| `TMinSolderMaskSliverViolation` | 8 | DRC violation storage |
| `TBoardOutlineClearanceViolation` | 8 | DRC violation storage |

### 3.2 Medium-frequency (3-7 files)

| Storage | Files | Category |
|---|---|---|
| `TShortCircuitViolation` | 6 | DRC violation |
| `CustomShapes` | 6 | Custom pad shapes |
| `TNetAntennaeViolation` | 5 | DRC violation |
| `TDisconnectedSubnetsViolation` | 5 | DRC violation |
| `CornerRadiusChamfer` | 5 | Pad corner definitions |
| `TModifiedPolygonViolation` | 4 | DRC violation |
| `SharedUnion` | 4 | Union definitions |
| `TUnconnectedPinViolation` | 3 | DRC violation |
| `TDiffPairsViolation` | 3 | DRC violation |
| `TComponentClearanceViolation` | 3 | DRC violation |
| `DrillManager` | 3 | Drill configuration |
| `CustomMaskShapes` | 3 | Custom mask shapes |

### 3.3 Low-frequency (1-2 files)

| Storage | Files | Category |
|---|---|---|
| `ViaStructures` | 1 | Via structure definitions |
| `ViaStructureManager` | 1 | Via structure management |
| `TMaxMinViaHoleSizeViolation` | 1 | DRC violation |
| `TMaxMinLengthViolation` | 1 | DRC violation |
| `TMaxMinComponentHeightViolation` | 1 | DRC violation |
| `THoleToHoleViolation` | 1 | DRC violation |
| `TMinimumAnnularRingViolation` | 2 | DRC violation |
| `TMaxMinPadSlotWidthViolation` | 2 | DRC violation |
| `TMatchedNetLengthsViolation` | 2 | DRC violation |

### 3.4 V5 Legacy Storages (2 files: stm32f103-core, fingerprint-lock-v2as)

These files have V5-format section names (no "6" suffix) plus no `/FileHeaderSix`
stream: `Arcs`, `Board`, `Classes`, `Components`, `ComponentBodies`, `Connections`,
`Coordinates`, `DifferentialPairs`, `Dimensions`, `EmbeddedBoards`, `EmbeddedFonts`,
`Embeddeds`, `Fills`, `FromTos`, `Nets`, `Pads`, `Polygons`, `Regions`, `Rules`,
`Texts`, `Tracks`, `Vias`, `WideStrings`.

None of these (except `Texts` which is already a primitive kind) are recognized by
`PrimitiveSectionKind::from_storage_name` or `ParamSectionKind::from_storage_name`.

---

## 4. Serialization Gaps

### 4.1 Full document save is disabled

**File:** `crates/altium-format/src/pcbdoc/mod.rs:307-313`

`PcbDoc::save()` unconditionally returns an error. Only `AddTrack` operations
work via the ops system, but the result cannot be saved.

### 4.2 Only Track serialization is implemented

**File:** `crates/altium-format/src/pcbdoc/mod.rs:434-457`

`serialize_primitive_payload` handles only `PcbPrimitive::Track`. All other
primitives (Arc, Fill, Pad, Text) return errors.

### 4.3 No sidecar stream serialization

WideStrings6, UniqueIDPrimitiveInformation, ExtendedPrimitiveInformation,
PrimitiveGuids, PrimitiveParameters, UnionNames — none of these sidecar streams
have save logic.

### 4.4 No parameter section serialization

Board6, Nets6, Components6, Polygons6, Classes6, Rules6, Dimensions6, etc. —
none of the parameter sections have serialization logic.

---

## 5. Data Integrity Concerns (per SKILL.md review rules)

### 5.1 D4: Default substitution for missing pad/via library config

**File:** `crates/altium-format/src/pcbdoc/mod.rs:196`

```rust
let config = parse_pad_via_library(&header_data, &data).ok().flatten();
```

`.ok()` silently drops parse errors from PadViaLibrary and PadViaLibraryCache
sections. If the section contains data we don't understand, the error is swallowed
and `config` is set to `None`.

**Severity:** CRITICAL (R3 violation: silent error dropping)

### 5.2 D6: Section record count not validated

**File:** `crates/altium-format/src/pcbdoc/mod.rs:237,249,263,274`

Multiple sections read the `expected_count` from the Header stream but then
discard it with `let _ = expected_count;`. The actual record count parsed from
Data is never compared against the header declaration.

**Severity:** WARNING (header mismatch could indicate corrupt or truncated data)

### 5.3 D2: Model blobs stored as raw `Vec<u8>`

**File:** `crates/altium-format/src/pcbdoc/mod.rs:54`

`ModelsSectionData::blobs` is `Vec<(String, Vec<u8>)>`. The numbered model
streams (`/Models/0`, `/Models/1`, etc.) are binary 3D model files (STEP, etc.)
which are genuinely opaque external data, so this may be acceptable. However,
the `Vec<u8>` should be verified to be actual 3D model data and not structured
Altium format data.

**Severity:** INFO (likely acceptable but should verify)

### 5.4 D2: EmbeddedFonts blob stored as raw `Vec<u8>`

**File:** `PcbEmbeddedFontEntry::data` field

Font data blobs are TrueType font files — genuinely opaque binary data. This is
acceptable per the D2 exception for embedded images/opaque payloads.

**Severity:** INFO (acceptable)

### 5.5 Text `subrecord1_tail` stored as raw `Vec<u8>`

**File:** `crates/altium-format/src/pcbdoc/primitives.rs:543`

```rust
let subrecord1_tail = reader.read_bytes(reader.remaining())?.to_vec();
```

After all known fields are parsed, any remaining bytes in subrecord 1 are
captured as an opaque `Vec<u8>`. This violates the cardinal rule against opaque
format data retention.

**Severity:** CRITICAL (cardinal rule violation: opaque blob retention)

---

## 6. Recommended Fix Priority

### Phase 1: Unblock V6 file validation (fixes 88 files)

1. **Fix Text `reserved_zero` assertions** — reverse-engineer the actual field
   meanings from the Delphi/C# source for the barcode flag, barcode tail, and
   extended header bytes. Replace `read_reserved_zero` calls with proper typed
   field reads.

2. **Fix Text `advance_reserved` assertion** — reverse-engineer the advance
   reserved byte field from older format versions.

3. **Remove `subrecord1_tail` opaque blob** — either parse all remaining fields
   or return a hard error for unknown trailing data.

### Phase 2: Implement missing primitive parsers

4. **Implement Via parser** — 316-byte fixed layout, well-documented in
   `binary-primitives.md`. Per-layer diameter arrays similar to Pad.

5. **Implement Region parser** — variable-length with vertex array. Needed for
   Regions6, ShapeBasedRegions6, and BoardRegions sections.

6. **Implement ComponentBody parser** — variable-length with outline + 3D model
   reference. Needed for ComponentBodies6 and ShapeBasedComponentBodies6.

### Phase 3: Handle unrecognized storages

7. **Add `PrimitiveGuids` sidecar handler** — documented in `sidecar-streams.md`
   as 24-byte packed records. Present in 18 files.

8. **Add DRC violation storage handlers** — the `T*Violation` storages are DRC
   results stored as parameter blocks. There are 15+ violation types. These may
   be droppable on save (Altium regenerates them) but must be parsed on load.

9. **Add `ConstraintManager` handler** — present in 61/94 files. Design
   constraint definitions.

10. **Add remaining storages** — `UnionFeatures`, `LettersGeometry`,
    `CustomShapes`, `CornerRadiusChamfer`, `SharedUnion`, `DrillManager`,
    `CustomMaskShapes`, `ViaStructures`, `ViaStructureManager`.

### Phase 4: V5 format support

11. **Add V5 section name recognition** — map `Arcs` → `Arcs6`, `Board` →
    `Board6`, etc. for the legacy section names without "6" suffix.

12. **Handle missing `FileHeaderSix`** — 2 files have only `/FileHeader` (V5
    legacy format) without `/FileHeaderSix`. The parser currently returns a hard
    error.

### Phase 5: Serialization

13. **Implement full document save** — enable `PcbDoc::save()`.
14. **Implement serialization for all primitive types** (Arc, Fill, Pad, Text,
    Via, Region, ComponentBody).
15. **Implement sidecar stream serialization** (WideStrings6,
    UniqueIDPrimitiveInformation, etc.).
16. **Implement parameter section serialization** (Board6, Nets6, Components6,
    etc.).

### Phase 6: Fix data integrity issues

17. **Fix PadViaLibrary `.ok()` error swallowing** — propagate errors instead of
    silently converting to `None`.
18. **Validate section record counts** — compare parsed count against header
    declaration.

---

## 7. Test Fixture Summary

| Category | Count | Notes |
|---|---|---|
| V6 CFB files (FileHeaderSix present) | 94 | Target for full support |
| ASCII V5 text format (non-CFB) | 36 | `\|RECORD=Board\|...` plaintext format |
| V5 binary CFB (no FileHeaderSix) | 2 | Legacy binary format |
| **Total** | **132** | |

The 36 ASCII V5 files are an entirely different format (pipe-delimited text, not
CFB) and would need a separate parser. The 2 V5 binary CFB files share much of
the same structure but use section names without the "6" suffix and lack the
FileHeaderSix stream.
