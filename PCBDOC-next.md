# PcbDoc: Next Steps Investigation Guide

Updated 2026-02-28. Includes quick wins implementation results and
detailed research on Issues 1-4 (ConstraintManager, PadViaLibrary,
ShapeBasedRegions6, PrimitiveGuids).

---

## Current State

**5 of 132 test fixtures pass `altium validate`** (up from 4 earlier today, 0 on 2026-02-25).

Passing files: `fingerprint-lock-v1.PcbDoc`, `heron-4layer.PcbDoc`,
`kitsprout-template.PcbDoc`, `test-textfonts.PcbDoc`, `test-textsize.PcbDoc`.

### What has been completed

1. **Text parser fully rewritten** — barcode fields, advance_snapping/advance_mode.
2. **Via/Region/ComponentBody/Pad parsers unified with pcblib** — shared code.
3. **Common header unified** — 13-byte layout shared between pcbdoc and pcblib.
4. **Pad thermal entry sizes** — variable sizes (23, 29, 30) handled.
5. **Pad trailing bytes without sub4 flag** — resolved.
6. **Additional storages**: `SharedUnions`, `UnionNames`, `UnionRelations`,
   `Connections6`, `LayerKindMapping`, `EmbeddedFonts6`, `Models`,
   `PadViaLibrary`/`PadViaLibraryCache`.

### Quick wins implemented (this session)

9. **f64 whitespace trimming** — `param_value.rs` now trims before `f64::parse()`.
   Fixes `MODEL.3D.ROTX= 3.3E-0314` style values. (2 files unblocked, now hit
   ConstraintManager)

10. **New ParamSectionKind storages** — added `CustomShapes`,
    `TClearanceViolation`, `TShortCircuitViolation`,
    `TSilkToSilkClearanceViolation`, `TRoutingViaStyleViolation`.

11. **PrimitiveParameters hierarchical parser** — new `parse_primitive_parameter_records()`
    reads component headers with `COUNT=N` then N child blocks. Header count validates
    against group count, not flat block count. (3 files unblocked, now hit
    ShapeBasedComponentBodies6)

12. **UnionFeatures indexed format** — discovered `[u32 union_index][u32 len][payload]`
    format (not standard param blocks). New `parse_indexed_param_records()` parser
    with dedicated `PcbDocSection::UnionFeatures` variant.

13. **SharedUnion hierarchical format** — discovered three variants: with
    `HIDDENPRIMITIVESCOUNT=N` (child blocks), with `PRIMITIVESCOUNT=N` (inline refs),
    or neither (no child data). New `parse_shared_union_param_records()` parser.

---

## Error Distribution (132 files, post-quick-wins)

| Error Category | Count | Notes |
|---|---|---|
| PASS | 5 | Full parse + invariant validation |
| ASCII V5 text (not CFB) | 36 | Out of scope — entirely different format |
| V5 binary (no FileHeaderSix) | 2 | Low priority — legacy binary format |
| **ConstraintManager** | **26** | See Issue 1 — complex UTF-16LE base64/zlib |
| **PadViaLibrary templates** | **18** | See Issue 2 — multi-record template format |
| **ShapeBasedRegions6** | **15** | See Issue 3 — extended vertex format |
| **PrimitiveGuids** | **11** | See Issue 4 — binary 24-byte records |
| **ShapeBasedComponentBodies6** | **8** | Same root cause as Issue 3 |
| **EmbeddedFonts6 variant** | **7** | See Issue 5 — font entry format varies |
| **DrillManager** | **3** | See Issue 6 — specialized format |
| WideStrings6 edge case | 1 | See Issue 7 |

**Note**: Each file fails on its FIRST error. The previous "43 unsupported storage"
category has been eliminated — those files now progress past the newly-supported
storages and hit deeper issues, revealing the true error distribution.

---

## ISSUE 1: ConstraintManager (26 files)

### Current status

**Not implemented.** PcbDoc dispatch at `pcbdoc/mod.rs:383` returns hard error
`"unsupported storage '/ConstraintManager'"`.

### CFB storage layout

```
/ConstraintManager/
  +-- Header    (4 bytes: u32 LE, value 0x00000001 in all test files)
  +-- Data      (single text block: [u32 header][UTF-16LE payload][u16 NUL])
```

### Encoding pipeline (verified from C# source + hex dump)

The Data stream contains a **single text block** with this encoding chain:

1. **XML document** (UTF-8) — serialized by `ConstraintDocumentXmlSerializer`
   - Namespace: `http://altium.com/ns/ConstraintManager2` (v2.0 format)
   - Root element: `<ConstraintDocument>` with `<ConstraintSets>` and `<Constraints>`
2. **Zlib compress** the UTF-8 XML bytes (standard deflate, header `78 DA`)
3. **Base64 encode** the compressed bytes (RFC 4648)
4. **Write as UTF-16LE** text block in the CFB stream

**Reverse pipeline (decode):**
Read text block → decode UTF-16LE → base64 decode → zlib decompress → parse XML

### Hex dump (rover-arm.PcbDoc, all 30 bytes)

```
Header: 01 00 00 00              (u32 = 1)
Data:   1a 00 00 00              (block header: text, 26 bytes)
        65 00 4e 00 6f 00 44 00  "eNoDAAAAAAE=" as UTF-16LE
        41 00 41 00 41 00 41 00
        41 00 41 00 45 00 3d 00
        00 00                     NUL terminator
```

- UTF-16LE decodes to: `eNoDAAAAAAE=`
- Base64 decodes to: `78 da 03 00 00 00 00 01` (9 bytes, zlib header)
- Zlib decompresses to: **empty** (0 bytes)

**All 26 test files contain empty constraint documents** (zlib decompresses to 0 bytes).
This is normal for designs created without using the constraint manager feature.
Real designs with constraints will have actual XML content.

### C# source locations

| File | Content |
|------|---------|
| `AD26-dotnet/ConstraintsManager.Module/.../ConstraintDocumentXmlSerializer.cs` | v2.0 XML serializer (~5774 lines) |
| `AD26-dotnet/Altium.ConstraintsManager/.../ConstraintDocumentXmlSerializer.cs` | v1.0 XML serializer (~3200 lines) |
| `AD26-dotnet/Altium.ConstraintsManager.Abstractions/` | `IConstraintDocument`, `IRuleData`, `RuleType` (40+ rule types) |
| `AD26-dotnet/.../ConstraintsServerPcbDocument.cs` | PCBDoc ↔ ConstraintManager integration |

### XML schema (v2.0, key elements)

```xml
<ConstraintDocument xmlns="http://altium.com/ns/ConstraintManager2"
    SerializerVersion="1.0" DocumentName="..." UserName="..." CreationDateTime="...">
  <ConstraintSets>
    <ClearanceMatrixItem>...</ClearanceMatrixItem>
    <PhysicalItem>...</PhysicalItem>
    <ElectricalNetsItem>...</ElectricalNetsItem>
    <ElectricalDiffPairsItem>...</ElectricalDiffPairsItem>
    <AdvancedItem>...</AdvancedItem>
  </ConstraintSets>
  <Constraints>...</Constraints>
</ConstraintDocument>
```

### Implementation approach

**Minimal (unblock 26 files):** Parse the text block → decode UTF-16LE → base64 decode →
zlib decompress → store the decompressed XML bytes as `String`. No XML parsing needed
initially — just validate the decompression pipeline and store the raw XML. This
preserves fail-fast behavior while unblocking files.

**Full (design rules support):** Parse the XML into typed Rust structs for clearance,
physical, electrical, diff-pair, and advanced constraint sets. This is a large effort
(40+ rule types) and can be deferred.

**Dependencies:** `base64` crate (already in workspace?), `flate2` or `miniz_oxide`
for zlib, `encoding_rs` for UTF-16LE (already used).

---

## ISSUE 2: PadViaLibrary Multi-Record Template Format (18 files)

**Files affected:** 18 (rover-*, heron-*, thesis-lora-* series)

### Current status

Parser at `pcblib/library.rs:454-495` reads one standard text block (config params)
and errors if additional blocks exist. PcbDoc dispatch at `pcbdoc/mod.rs:219-230`
propagates this error.

### Verified format (hex dump: rover-arm.PcbDoc)

**Header:** `02 00 00 00` → u32 LE = 2 (template count, NOT total block count)

**Data stream (2708 bytes total):**

```
Offset 0x000 — Config block (standard text block framing):
  [u32 0x0000007f = 127]  block header (flags=0x00, size=127)
  [127 bytes]             |PADVIALIBRARY.LIBRARYID={...}|
                          |PADVIALIBRARY.LIBRARYNAME=<Local>|
                          |PADVIALIBRARY.DISPLAYUNITS=1|NUL

Offset 0x083 — Template 1 (custom framing, NOT standard blocks):
  [u8  0x02]              template index = 2
  [u32 0x000005B3 = 1459] param string length
  [1459 bytes]            |TEMPLATE.EXTERNALLINK.LIBRARYID={...}|
                          |TEMPLATE.TEMPLATENAME=c152hn76|
                          |TEMPLATE.PAD.ISMULTILAYER=TRUE|
                          |TEMPLATE.VIA.HOLESIZE=16mil|...NUL

Offset 0x63B — Template 2 (same custom framing):
  [u8  0x03]              template index = 3
  [u32 0x00000454 = 1108] param string length
  [1108 bytes]            |TEMPLATE.*|...NUL

Total: 131 + 1464 + 1113 = 2708 bytes ✓
```

### Key observations

1. **Config block** uses standard text block framing `[u32 header][payload]`
2. **Template blocks** use DIFFERENT framing: `[u8 index][u32 param_len][params]`
3. Template indices start at 2 and increment (2, 3, 4...)
4. The Header u32 count = number of templates (NOT config blocks + templates)
5. **Block parser misreads templates** — the byte `0x02` (template index) gets
   incorporated into the u32 size field, producing garbage size 373506

### C# interface

`IPCB_PadViaLibrary` (`AD26-dotnet/.../IPCB_PadViaLibrary.cs`):
- `GetState_Count()` → template count
- `GetState_Template(int index)` → `IPCB_PadViaTemplate`
- `GetState_LibraryID()`, `GetState_LibraryName()`, `GetState_DisplayUnits()`

`IPCB_PadViaTemplate` (`AD26-dotnet/.../IPCB_PadViaTemplate.cs`):
- `GetState_ObjectID()` → `TObjectId` (Pad or Via)
- `GetState_TemplateName()` → string
- `Export_ToParameters()` → exports with `TEMPLATE.` prefix

### Implementation approach

Extend `parse_pad_via_library()` in `pcblib/library.rs`:
1. Read config block as before (standard text block)
2. Read N template blocks with `[u8 index][u32 len][params]` framing
3. Store templates as `Vec<PadViaTemplate>` in `PcbPadViaLibraryConfig`
4. Validate template count against Header u32

### Code locations

| File | Lines | What |
|------|-------|------|
| `pcblib/library.rs` | 351-355 | `PcbPadViaLibraryConfig` struct (needs `templates` field) |
| `pcblib/library.rs` | 454-495 | `parse_pad_via_library()` (needs template parsing) |
| `pcbdoc/mod.rs` | 219-230 | PcbDoc dispatch for PadViaLibrary |
| `pcblib/mod.rs` | 1498-1504 | `serialize_pad_via_library()` (needs template serialization) |
| `docs/pcblib/library-storage.md` | 147-157 | Documentation (incomplete, no templates) |

---

## ISSUE 3: ShapeBasedRegions6 / ShapeBasedComponentBodies6 Format (23 files)

**Files affected:** 15 (ShapeBasedRegions6) + 8 (ShapeBasedComponentBodies6) = 23 total

### Current status

Region parser at `pcblib/primitives/region.rs` uses `read_f64_contour()` (line 33)
which reads `i32 count + count × (f64 x, f64 y)` = 16 bytes per vertex. This works
for Regions6 (legacy) but fails for ShapeBasedRegions6 (extended vertex format).

Both section kinds dispatch to the same `parse_region()` function via
`pcbdoc/primitives.rs:137-140` (both map to `PcbObjectId::Region`).

### Verified format: `TPolySegment` (C# struct, Pack=1, 37 bytes)

**Source:** `AD26-dotnet/Altium.Edp.Interfaces/Pcbtypes/TPolySegment.cs`

```csharp
[StructLayout(LayoutKind.Sequential, Pack = 1)]
public struct TPolySegment {
    public TPolySegmentType Kind;  // byte (u8): 0=Line, 1=Arc
    public int vx;                 // i32 LE: vertex X (internal units)
    public int vy;                 // i32 LE: vertex Y (internal units)
    public int cx;                 // i32 LE: arc center X (0 for lines)
    public int cy;                 // i32 LE: arc center Y (0 for lines)
    public int Radius;             // i32 LE: arc radius (0 for lines)
    public double Angle1;          // f64 LE: arc start angle in degrees
    public double Angle2;          // f64 LE: arc end angle in degrees
}
// Total: 1 + 4×5 + 8×2 = 37 bytes
```

**`TPolySegmentType` enum** (`Pcbtypes/TPolySegmentType.cs`):
```csharp
public enum TPolySegmentType : byte {
    ePolySegmentLine = 0,
    ePolySegmentArc = 1
}
```

### Hex dump verification (artiq-hvsup-isol.PcbDoc, /ShapeBasedRegions6/Data)

**Header:** `5f 00 00 00` = 95 records
**Regions6 header also:** `5f 00 00 00` = 95 records (same count — both sections coexist)

**Record 0 (398 bytes payload, ISSHAPEBASED=TRUE, 6 edges):**

```
Offset 0x8c: vertex_count = 06 00 00 00 (i32 = 6)

Vertex 0 (line) at 0x90:   [00] [40 2e 1c 06] [3f f1 cc 03] [00×4] [00×4] [00×4] [00×8] [00×8]
Vertex 1 (line) at 0xb5:   [00] [0a 32 28 06] [3f f1 cc 03] [00×4] [00×4] [00×4] [00×8] [00×8]
...
Vertex 4 (ARC!) at 0x124:  [01] [40 2e 1c 06] [23 b1 e6 03] [d4 ac 17 06] [ab dc dc 03]
                            [36 d0 0a 00] [76 8d 3d 35 fd 69 72 40] [78 ca 09 2b 0b 58 50 40]
                            kind=Arc   x=0x061c2e40  y=0x03e6b123  cx=0x0617acd4  cy=0x03dcdcab
                            radius=0x000ad036  angle1≈293°  angle2≈65°
...
Vertex 6 (CLOSING) at 0x16e: [00] [40 2e 1c 06] [3f f1 cc 03] [00...] — same as vertex 0
```

**Stride: 37 bytes per vertex. Verified: 0xb5 - 0x90 = 0x25 = 37 ✓**

### Critical: N+1 vertex convention

ShapeBasedRegions6 stores **N+1 vertices** for a count of N:
- `i32 vertex_count = N` (number of edges)
- `(N + 1) × TPolySegment` records follow (closing vertex duplicates first vertex)
- **Total contour bytes:** `4 + (N + 1) × 37`

Verification: 6 edges → 7 × 37 = 259 bytes + 4 = 263 bytes. Remaining payload
after header/kind/holes/params = 263 bytes ✓

**Legacy Regions6 stores exactly N vertices** (closing vertex implied):
- `i32 vertex_count = N`
- `N × (f64 x, f64 y)` pairs follow
- **Total contour bytes:** `4 + N × 16`

### Same applies to hole contours

Each hole in a ShapeBasedRegion also uses N+1 TPolySegment format.
The `hole_count` (i32) at payload offset 14-17 tells how many hole contours follow.

### Implementation approach

**Option A (recommended): Pass section kind through to region parser.**

1. Add a `is_shape_based_section: bool` parameter to `parse_region()`
2. When true, use `read_polysegment_contour()` instead of `read_f64_contour()`
3. `read_polysegment_contour()` reads N+1 TPolySegment records (37 bytes each)
4. Store edge metadata (kind, cx, cy, radius, angle1, angle2) in the PcbRegion struct
5. Thread the section kind from `pcbdoc/primitives.rs` dispatch

**Note:** The parameter string also contains the shape data (`KIND0`, `VX0`, `CX0`,
etc.) — currently consumed but discarded at `region.rs:146-178`. With binary parsing,
the param values become redundant (they're the same data in text form). They should
still be consumed to pass `assert_exhausted()`.

### Code locations

| File | Lines | What |
|------|-------|------|
| `pcblib/primitives/region.rs` | 33-60 | `read_f64_contour()` — legacy vertex reader |
| `pcblib/primitives/region.rs` | 74-217 | `parse_region()` — main parser |
| `pcblib/primitives/region.rs` | 146-178 | Shape param consumption (discards edge metadata) |
| `pcbdoc/primitives.rs` | 137-140 | Section kind → ObjectId dispatch |
| `pcbdoc/records.rs` | 15-18 | `PrimitiveSectionKind::ShapeBasedRegions6` |

### C# interfaces

| Interface | File | Purpose |
|-----------|------|---------|
| `IPCB_RegionShape` | `RT_PCB/IPCB_RegionShape.cs` | Old shape API using `TPolySegment` |
| `IPCB_RegionShape2` | `RT_PCB/IPCB_RegionShape2.cs` | New shape API using `IPCB_ShapeEdge` |
| `TPCB_ShapeEdgeType` | `PCB/TPCB_ShapeEdgeType.cs` | Edge types: Line, Circle, Ellipse, Parabola, BSpline |

Note: `TPCB_ShapeEdgeType` has 5 values (Line/Circle/Ellipse/Parabola/BSpline)
but `TPolySegmentType` only has 2 (Line/Arc). The binary format uses `TPolySegmentType`
(u8, 0 or 1). Advanced edge types (Ellipse/Parabola/BSpline) are likely only used
through the `IPCB_RegionShape2` COM API, not in the binary file format.

---

## ISSUE 4: PrimitiveGuids Binary Format (11 files)

### Current status

**PcbLib: implemented** at `pcblib/sidecar.rs:203-248` — parses 24-byte records,
converts ObjectId i32 → u8 → `ViewableObjectId`.

**PcbDoc: not implemented** — hard error at `pcbdoc/mod.rs:383-388`:
`"unsupported storage '/PrimitiveGuids' encountered; typed parser required"`

### Verified format (artiq-hvsup-isol.PcbDoc)

**Header:** `e6 28 00 00` → u32 LE = 10470 entries
**Data:** 251,280 bytes = 10470 × 24 ✓ — **format is 24-byte packed records, NOT block-framed**

The `cfb blocks` tool misreads this as a single 520-byte text block because the first
4 bytes happen to look like a block header. The data is raw binary with no block framing.

### Record format (24 bytes, same struct as PcbLib `TPrimitiveGUID`)

```
i32  ObjectId       (4 bytes, LE)
i32  IndexForSave   (4 bytes, LE)
u8   GUID[16]       (16 bytes, raw Windows GUID)
```

### PcbDoc ObjectId differs from PcbLib

In PcbLib, ObjectId values fit in u8 (0-119, ViewableObjectId range).
In PcbDoc, the i32 carries **additional metadata** in the upper bytes:

```
Record 0: ObjectId = 0x00000208  →  low byte 0x08 = Group
Record 1: ObjectId = 0x00001004  →  low byte 0x04 = Track
Record 2: ObjectId = 0x00001105  →  low byte 0x05 = Text
Record 3: ObjectId = 0x00000D01  →  low byte 0x01 = Arc
Record 4: ObjectId = 0x00000E02  →  low byte 0x02 = Pad
```

The low byte is consistently a valid `ViewableObjectId`. The upper bytes (0x02, 0x10,
0x11, 0x0D, 0x0E) appear to be section-related metadata (possibly the section index
within the PcbDoc storage list, or a sub-type discriminator). The upper 16 bits are
always 0x0000 in all observed records.

### Implementation approach

1. Reuse the existing `parse_primitive_guids()` from `pcblib/sidecar.rs`
2. Modify it to handle PcbDoc's wider ObjectId: store the full i32 or extract only the
   low byte as `ViewableObjectId`
3. Add a `PcbDocSection::PrimitiveGuids` variant and dispatch handler in `pcbdoc/mod.rs`
4. The Data stream is raw binary (NOT block-framed) — read directly, not through
   block iterator

### Code locations

| File | Lines | What |
|------|-------|------|
| `pcblib/sidecar.rs` | 93-103 | `PrimitiveGuidEntry` struct |
| `pcblib/sidecar.rs` | 196-197 | `PRIMITIVE_GUID_RECORD_SIZE = 24` constant |
| `pcblib/sidecar.rs` | 203-248 | `parse_primitive_guids()` — PcbLib parser (needs ObjectId fix for PcbDoc) |
| `pcblib/sidecar.rs` | 470-483 | `serialize_primitive_guids()` |
| `pcbdoc/mod.rs` | 383-388 | Hard error for unsupported PrimitiveGuids |
| `altium-format-types/src/pcb.rs` | 132-265 | `ViewableObjectId` enum (0-119) |

### Open question

What do the upper bytes of ObjectId represent in PcbDoc? Candidates:
- Section index in PcbDoc storage list
- Sub-type discriminator within the object type
- Version/flags field

This needs investigation via the Delphi `TPrimitiveGUID` struct in ghidra or by
correlating the upper byte values with the actual section registry positions.

---

## ISSUE 5: EmbeddedFonts6 Format Variant (7 files)

**Files affected:** tc377-car-mark1, tc377-car-mark2, tc377-car-mark3,
tc377-tps40304-demo, oshw-ac-rc-unit, tracker-keyboard

### Root cause

The EmbeddedFonts6 parser reads: `[u32 name_len][name UTF-16LE]` ×3 (name, style,
localized), then `[u16 unknown][u8 flag][u32 blob_size][blob]`.

In the failing files, the third font entry (e.g. "Berlin Sans FB Demi Bold") has
a different record layout after the localized name field. The parser reads bytes
from the zlib-compressed font blob as the `unknown_u16`/`flag`/`blob_size` fields,
resulting in a nonsensical blob size of ~2.6 billion bytes.

### Likely explanation

The font entry format varies by font type or encoding. When the localized name is
empty (2-byte UTF-16LE with just NUL), the subsequent fields may have a different
structure — possibly omitting the `unknown_u16` and `flag` fields, or using a
different blob size encoding. The `78 9C` zlib header appears immediately after
what appears to be a `01 00` prefix.

### Investigation needed

1. Hex dump the working fonts vs the failing font in the same file to find the
   exact field layout difference
2. Check the C# `IPCB_EmbeddedFonts` interface for font entry structure
3. Check if the format difference correlates with font type (TrueType vs OpenType)

---

## ISSUE 6: DrillManager (3 files)

Uses 8-byte binary prefix (two u32s) before standard param blocks. The first u32
is `0xFFFFFFFF` (-1), the second appears to be a type/count value. Needs investigation.

---

## ISSUE 7: WideStrings6 Edge Case (1 file)

**File affected:** uwarg-zeropilot2

### Root cause

The WideStrings6 parser fails at offset 10448 with "expected index 361". The stream
has 362 entries (0x016A from Header) but the last entry appears to have a truncated
or differently-encoded payload. The bytes at the failure point are
`[00 00 02 00 00 00]` — only 6 bytes remaining for what should be an 8+ byte entry
header.

### Investigation needed

Check whether this is a truncated stream, an encoding variant (the `00 00` could be
a sentinel for implicit sequential indexing), or an off-by-one in index tracking
within the parser.

---

## Key Files Reference

| File | Purpose |
|---|---|
| `crates/altium-format/src/pcbdoc/primitives.rs` | All primitive parsers. Via/Region/ComponentBody/Pad delegate to pcblib. Arc/Track/Fill/Text parsed locally. |
| `crates/altium-format/src/pcbdoc/mod.rs` | PcbDoc loading pipeline, section dispatch, invariant validation |
| `crates/altium-format/src/pcbdoc/records.rs` | Section name → enum mapping, param/WideStrings/UnionNames/Connections parsers |
| `crates/altium-format/src/pcblib/primitives/` | Shared primitive parsers: pad.rs, via.rs, region.rs, component_body.rs |
| `crates/altium-format/src/pcblib/mod.rs` | PcbPrimitiveCommon and all shared PCB structs |
| `crates/altium-format/src/pcblib/library.rs` | PadViaLibrary/LayerKindMapping/Models parsers |
| `crates/altium-format/src/param_value.rs` | FromParamValue trait (float parsing issue lives here) |
| `docs/pcbdoc/binary-primitives.md` | Binary record layout docs (common header needs update) |
| `docs/pcbdoc/sidecar-streams.md` | WideStrings6, UniqueID, PrimitiveGuids, PrimitiveParameters docs |

## Test Fixtures

| Category | Count | Notes |
|---|---|---|
| V6 CFB (target for support) | 94 | Has FileHeaderSix |
| ASCII V5 text (non-CFB) | 36 | Different format entirely |
| V5 binary (no FileHeaderSix) | 2 | Legacy binary |

## Quick Validation Commands

```bash
# Validate all files
for f in data/pcbdoc/*.PcbDoc; do
  result=$(target/release/altium-cli validate "$f" 2>&1)
  if echo "$result" | grep -q "^Validation passed"; then
    echo "PASS: $(basename $f)"
  else
    echo "FAIL: $(basename $f) | $(echo "$result" | grep "^Error:" | head -1)"
  fi
done

# Aggregate by error type
for f in data/pcbdoc/*.PcbDoc; do
  result=$(target/release/altium-cli validate "$f" 2>&1)
  if echo "$result" | grep -q "^Validation passed"; then echo "PASS"
  else echo "$result" | grep "^Error:" | head -1; fi
done | sort | uniq -c | sort -rn
```

## Recommended Fix Priority (post-research)

| Priority | Issue | Files Blocked | Effort | Format Verified? |
|---|---|---|---|---|
| 1 | ConstraintManager (minimal: decode pipeline only) | 26 | Low-Medium | Yes — hex + C# |
| 2 | ShapeBasedRegions6 + ShapeBasedComponentBodies6 | 23 | Medium | Yes — TPolySegment struct + hex |
| 3 | PadViaLibrary multi-record templates | 18 | Low-Medium | Yes — hex verified boundary |
| 4 | PrimitiveGuids binary parser | 11 | Low | Yes — 24-byte records confirmed |
| 5 | EmbeddedFonts6 format variant | 7 | Medium | No — needs hex investigation |
| 6 | DrillManager specialized format | 3 | Low | No — needs hex investigation |
| 7 | WideStrings6 edge case | 1 | Low | No |

**Notes**: Priority reordered after research. ConstraintManager's *minimal* fix
(decode pipeline without XML parsing) is now estimated Low-Medium effort — the
encoding chain is fully understood and all test files have empty documents.
Issues 1-4 are fully format-verified and ready for implementation.
Fixing issues 1-4 would bring the pass count to ~20-30 of 94 V6 files.

---

## Open Issues From Other Formats

The following issues were consolidated from archived investigation reports
(PROBLEMS.md, PCBDOC-diff-fixes.md, PCBLIB-diff-fix.md) on 2026-02-28.

---

## ISSUE 8: PcbDoc Data Integrity Violations

### 8a. PadViaLibrary silent error dropping (CRITICAL)

**File:** `crates/altium-format/src/pcbdoc/mod.rs:196`

```rust
let config = parse_pad_via_library(&header_data, &data).ok().flatten();
```

`.ok()` silently drops parse errors. If PadViaLibrary contains data we don't understand,
the error is swallowed and config becomes `None`. This violates the fail-fast rule.

**Fix:** Propagate errors instead of converting to `None`.

### 8b. Text subrecord1_tail opaque blob (CARDINAL RULE VIOLATION)

**File:** `crates/altium-format/src/pcbdoc/primitives.rs:543`

```rust
let subrecord1_tail = reader.read_bytes(reader.remaining())?.to_vec();
```

After parsing known fields, remaining bytes are captured as `Vec<u8>`. This violates
the cardinal rule against opaque data retention.

**Fix:** Parse all remaining fields or return a hard error for unknown trailing data.

### 8c. Section record count not validated

**File:** `crates/altium-format/src/pcbdoc/mod.rs` (multiple locations)

Multiple sections read `expected_count` from Header but discard it with
`let _ = expected_count;`. The actual record count is never compared against the header.

**Fix:** Add count validation (or at minimum log warnings) for all sections.

---

## ISSUE 9: PcbDoc Serialization Not Implemented

Full document save is disabled (`PcbDoc::save()` returns an error unconditionally).

**Current state:**
- Only `AddTrack` ops work via the ops system
- Only Track serialization is implemented in `serialize_primitive_payload`
- No sidecar stream serialization (WideStrings6, UniqueIDPrimitiveInformation, etc.)
- No parameter section serialization (Board6, Nets6, Components6, Polygons6, etc.)

**Fix:** Implement incrementally — each primitive type + sidecar + parameter section.
This is the largest body of remaining PcbDoc work.

---

## ISSUE 10: PcbLib Roundtrip — Board Config Serialization (CRITICAL DATA LOSS)

**File:** `crates/altium-format/src/pcblib/mod.rs:737-739`

`/Library/Data` shrinks from ~95KB to 176 bytes on save. The V9 layer stack, board
configuration, design rules, and component-name index are all lost.

**What's lost:**
- V9 master stack and substacks (`V9_MASTERSTACK_*`, `V9_STACK_LAYER*_*`)
- V8/V7 layer definitions
- Board dimensions, surface properties, grid settings, viewport config
- Design rules
- Component-name index suffix (binary format after the text block)

**Fix:** Implement `serialize_board_config()` in `board_config.rs`. This is the single
largest piece of PcbLib serialization work — hundreds of parameters across V9/V8/V7
layer stacks, surface properties, grid settings, viewport, and more.

### PcbLib minor roundtrip issues

- **ComponentParamsTOC Description `\r\n`**: First entry's description starts with
  `\r\n` in original, empty string in roundtrip. Trivial formatting fix.
- **Text leading `\r`**: Subrecord 1 text content has `\r` prefix in original that
  serializer omits. Needs verification against actual test fixture.

---

## ISSUE 11: PcbLib Parsing Gaps

### 11a. Via AD26 extra sections (sections 3-5)

Parser rejects AD26 vias with extra tail sections. Error: "unmapped AD26 Via sections
3-5 present: 62 bytes remain". Known additional sections include a 42-byte block +
pad-layer entries (stride 30) + trailing block.

**File:** `crates/altium-format/src/pcblib/primitives/via.rs:150`

### 11b. Fill variant record sizes

Fill parser assumes either base (37 bytes) or full AD26 (50 bytes) and asserts
exhausted. Some fixtures have intermediate sizes.

**File:** `crates/altium-format/src/pcblib/primitives/fill.rs:29`

### 11c. TextKind=3 unrecognized

Current enum only allows 0-2 (Stroke, TrueType, Barcode). Corpus has value 3
(`Synthiam.PcbLib`). AD26 SDK exposes `TTextKind` but does not clarify value 3.

**File:** `crates/altium-format-types/src/pcb.rs:760-775`

### 11d. Non-UTF8 pattern names

Pattern name decoding uses strict `from_utf8()`. Real corpus has non-ASCII footprint
names (Windows-1252). Should use `WINDOWS_1252.decode()` like other string boundaries.

**File:** `crates/altium-format/src/pcblib/footprint.rs:149-153`

### 11e. CustomShapes / ModelsNoEmbed unimplemented

- `CustomShapes` present → hard error (`footprint.rs:242-253`)
- `ModelsNoEmbed` non-empty payload → unimplemented (`pcblib/mod.rs:550-561`)

---

## ISSUE 12: SchDoc Parsing Gaps

### 12a. Missing vertex overflow handling (RECORD 5/6/7)

Polygons, polylines, and beziers with >255 vertices use overflow keys
(`EXTRALOCATIONCOUNT`, `EXnn`, `EYnn`, `EXnn_FRAC`, `EYnn_FRAC`). Parser only reads
`Xn/Yn` via `indexed_coords` with `LocationCount`, causing `UnknownParams` errors.

**AD26 reference:** `SchDataVertices.cs:83,:102` — `ImportFromFile(..., argIncludeExLocations: true)`

**File:** `crates/altium-format/src/sch_records.rs:994,:1020,:1038`

### 12b. Missing ALIGNMENT field on Note (RECORD 209)

`SchNote` has no `alignment` field. AD26 imports it via `Import_HorizontalAlign`.

**AD26 reference:** `FileFormatV5.cs:2438`, constant `ParameterNameAlignment = "Alignment"`

### 12c. Missing dispatch for HighLevelCodeSymbol (RECORD 220)

Enum `SchRecordType` includes `HighLevelCodeSymbol=220` but SchDoc dispatch has no
branch. AD26 maps 220→`eHighLevelCodeSymbol` via `SchDataSheetSymbol`.

**AD26 reference:** `RtSchematicExt.cs:1149,:1246`, `FileFormatUtils.cs:347`

### 12d. Hard-failing on valid optional streams

AD26 treats streams like `ObjectDefinitions`, `ReuseBlockInfos`, `ReuseBlocks`,
`ReuseBlocksV2`, `HarnessConnectionPointConnector` as optional (returns early if
absent). Our parser hard-fails when they ARE present.

**AD26 reference:** `SchDataImporterDocumentV5.cs:79,:757` — `StreamExists` guard

**File:** `crates/altium-format/src/schdoc/mod.rs:150`

---

## ISSUE 13: SchLib Parsing Gaps

### 13a. AllPinCount invariant too strict

Invariant enforces `component.all_pin_count == actual_pin_records`. AD26 only repairs
when stored value is `<= 0`; stale positive values are tolerated.

**Fix:** Allow stale positive values (warn category), reject negative, recompute on save.

**AD26 reference:** `SchComponent.cs:2659` (lazy repair), `FileFormatV5.cs:3075` (import)

### 13b. Missing font table parameter Size6

Parser requires every `SizeN` key for `1..FontIdCount` as mandatory. At least one
fixture is missing `Size6`. AD26 `ImportFontTable` also reads as required
(`FileFormatV5.cs:5279`), so this may be a corrupt fixture — needs classification.

---

## Full Validation Sweep (2026-02-28)

Fresh sweep across all test fixtures to verify which PROBLEMS.md issues are resolved
and identify new failures. Commands: `altium validate` on all fixtures, plus
`altium save-as` + `altium cfb diff --semantic` for roundtrip testing on SchLib and PcbLib.

---

### SchLib: 129/129 PASS (100%)

**All fixtures pass validation.** Issues 13a (AllPinCount) and 13b (Size6) are RESOLVED.

**SchLib roundtrip** (129 files save, 1 save failure):

After fixing the semantic diff to use **case-insensitive key comparison** by default
(matching Altium's documented behavior), results are: **33 PASS, 92 FAIL** out of 125
diffed files (excluding 4 binary-data grep issues).

**Remaining real roundtrip issues by category:**

1. **Non-ASCII text encoding** (~60+ files: aiskylab-\*, aKaReZa75-\*, vpodlesnyi-\*,
   Switches, etc.): Original has Win-1252 encoded Chinese/Russian/Vietnamese chars
   (e.g. `ÌùÆ¬µçÈÝ` = 贴片电容), roundtrip produces HTML numeric entities
   (`&#36148;&#29255;...`). **Root cause:** writer encodes non-ASCII chars as HTML
   entities instead of preserving Win-1252 bytes. **Fix:** re-encode to Win-1252 on
   write, use `%UTF8%` prefix for chars outside Win-1252 range.

2. **`ALLPINCOUNT` added on save** (~20+ files: arthurbenemann-\*, chilaboard,
   General_IC, ioelectro, kmilo17pet-\*, yu0316): Roundtrip adds `ALLPINCOUNT=N` param
   that wasn't in the original. This is intentional normalize-on-save behavior (Altium
   does the same). Not a bug, but triggers diff noise.

3. **Embedded object case mismatch** (~21 files: kmilo17pet-\*, amiryeg-\*,
   ryankurte, SMotlaq, etc.): PinWideText sidecar embedded objects differ at byte 8:
   `0x45` (E) vs `0x65` (e) — case difference inside compressed binary embedded
   objects. Same root cause as key casing but in sidecar serialization.

4. **Duplicate component storage** (1 file: `dungvh03-ICs.SchLib`): Save fails with
   "Cannot create storage at '/AT25XV041B' because a storage already exists there".
   File has duplicate component names — needs dedup or error handling.

5. **Structural mismatches** (~6 files: dungvh03-CERN_\*, Sika_revb, Custom,
   Mohamadkhosravi-MonoLine): Binary block length/type mismatches and missing
   streams — deeper format variant issues needing investigation.

---

### SchDoc: 1023/1215 PASS (84%), 192 FAIL

| Error Category | Count | Status vs PROBLEMS.md |
|---|---|---|
| `OwnerIndexAdditionalList` on RECORD=216 | 74 | **NEW** — not in PROBLEMS.md |
| `IGNOREONLOAD` on RECORD=7 (polygon) | 94 | **NEW** — not in PROBLEMS.md |
| RECORD=220 unknown params (HighLevelCodeSymbol) | 4 | Known (Issue 12c) — params now identified |
| `PrimaryConnectionPosition_Frac` on RECORD=215 | 14 | **NEW** — fractional position field |
| Missing `FontName6`/`FontName7` (legacy FPGA boards) | 4 | Related to Issue 13b (font table) |
| Missing `Size8` (legacy font table) | 1 | Related to Issue 13b |
| Embedded SchImage missing storage object | 1 | **NEW** — external image path reference |

**Resolved since PROBLEMS.md:**
- EX/EY vertex overflow (RECORD 5/6/7) — no longer hits as first error (may be masked
  by IGNOREONLOAD firing first on same polygon records, or may be fixed)
- ALIGNMENT on Note (RECORD 209) — no longer failing
- Hard-failing on optional streams (Issue 12d) — no longer failing (1023 files pass)

**New issues to add to Issue 12:**

#### 12e. Missing IGNOREONLOAD parameter on Polygon (RECORD=7) — 94 files

All from the `a3ng7n_Altium-Schematic-Parser` test corpus (FPGA embedded software
schematics). Polygon records contain an `IGNOREONLOAD` parameter not in our parser.

**AD26 reference:** Likely `FileFormatConsts.cs` — needs investigation.

#### 12f. Missing OwnerIndexAdditionalList on RECORD=216 — 74 files

Records in `/Additional` stream with `RECORD=216` have an `OwnerIndexAdditionalList`
parameter not handled by our parser. Affects the `raphaelchang`, `uwrobotics`,
`UBC-Thunderbots`, and `qfsae` fixture families.

#### 12g. Missing PrimaryConnectionPosition_Frac on RECORD=215 — 14 files

Port records (`RECORD=215`) in `/Additional` have a `PrimaryConnectionPosition_Frac`
parameter (DXP fractional coordinate) not handled by our parser. Affects all 14
`BFH-AudioStreamer` fixtures.

#### 12h. Missing FontName6/7/Size8 in legacy SchSheet (RECORD=31) — 5 files

Legacy FPGA board schematics have fewer fonts in their font table than the parser
requires. Files: `FPGA_Actel_ProASIC3*`, `FPGA_Memec_Virtex4*`, `bspd_001`.

#### 12i. SchImage references external storage path — 1 file

`d-el_PS3604L` contains a SchImage record referencing an external path
(`D:\Radio\Картинки\...`) that doesn't exist as a CFB storage object.

---

### PcbDoc: 5/132 PASS (4%), 127 FAIL

| Error Category | Count | Status vs PCBDOC-next Issues |
|---|---|---|
| Unsupported storage name | 40 | Known (Issue 1) |
| Non-CFB (ASCII V5 text) | 36 | Out of scope |
| ShapeBasedRegions6 trailing data | 22 | Known (Issue 2) |
| ShapeBasedRegions6 binary read past end | 10 | Known (Issue 2) |
| PadViaLibrary multi-record | 17 | Known (Issue 3) |
| EmbeddedFonts6 format variant | 6 | Known (Issue 4) |
| V5 binary (no FileHeaderSix) | 2 | Known (low priority) |
| WideStrings6 edge case | 1 | Known (Issue 7) |

**New passing file:** `test-textsize.PcbDoc` (previously failed on text parsing).

No new error categories vs PCBDOC-next Issues 1-7. PcbDoc status is accurate.

---

### PcbLib: 28/43 PASS (65%), 15 FAIL

| Error Category | Count | Status vs PROBLEMS.md |
|---|---|---|
| Via `hole_positive_tolerance` out of range | 8 | **NEW** — invariant too strict |
| `TextKind=3` invalid enum | 2 | Known (Issue 11c) |
| Pad `reserved byte 170` assertion | 1 | **NEW** — pad format variant |
| Text `reserved byte 231` assertion | 1 | **NEW** — text format variant |
| WideStrings `ENCODEDTEXT` non-UTF8 | 1 | **NEW** — encoding issue |
| Pattern name Win1252 mismatch | 1 | Known (Issue 11d) |
| Binary read past end (unknown) | 1 | **NEW** — needs investigation |

**Resolved since PROBLEMS.md:**
- Via AD26 extra sections 3-5 — no longer failing
- Fill variant record sizes — no longer failing
- CustomShapes present → hard error — no longer failing
- ModelsNoEmbed non-empty → unimplemented — no longer failing

**PcbLib roundtrip:** All 28 passing files also complete save-as successfully. Semantic
diff shows 0 issues for all 28 — **perfect roundtrip fidelity**.

**New issues to add to Issue 11:**

#### 11f. Via hole_positive_tolerance invariant too strict — 8 files

Via records with `hole_positive_tolerance = 214748.3647mil` (which is `0x7FFFFFFF`
in internal units — i32 max) fail the range check `[-2540mm, 2540mm]`. This value
likely represents "unset" or "use default" in Altium, not an actual expansion value.

**Files:** SMotlaq-PCB_lib, mobinbyn-Module, mobinbyn-Capacitor, lucashudson-Voltage-Regulators,
LimeMicroAltiumLib_pcbLib, FragasLab-Footprint, elk-pi-Sika_revb, amiryeg-IC-SMD-QUAD.

**Fix:** Treat `0x7FFFFFFF` as sentinel for "auto/default" (similar to `0xFFFF` for net_index).

#### 11g. Pad reserved byte 170 non-zero — 1 file

`Senior-Design-Custom.PcbLib` has a pad record where byte 170 (currently asserted as
reserved zero) is `0x0A`. Likely a real field that needs reverse engineering.

#### 11h. Text reserved byte 231 non-zero — 1 file

`mobinbyn-Socket.PcbLib` has a text record where byte 231 is `0x01`. Same class of
issue as PcbDoc text reserved_zero assertions — likely a format variant field.

#### 11i. WideStrings ENCODEDTEXT non-UTF8 — 1 file

`TranDangKhoa-LIB.PcbLib` has `ENCODEDTEXT1` that decodes to bytes which are not
valid UTF-8. The comma-separated byte values reconstruct to a non-UTF8 sequence.
Parser should use `WINDOWS_1252.decode()` instead of `from_utf8()`.

#### 11j. Unknown binary read past end — 1 file

`TranDangKhoa-VioletofSun.PcbLib` fails with "needed 8 bytes at offset 32, only 0
remain" during initial loading. May be a truncated/corrupt fixture or a format variant
in the library-level Data stream.

---

### Summary: What's Changed Since PROBLEMS.md

| Format | PROBLEMS.md (2026-02-24) | Now (2026-02-28) | Delta |
|---|---|---|---|
| **SchLib** | Multiple failures (AllPinCount, Size6) | **129/129 PASS (100%)** | **FULLY RESOLVED** |
| **SchDoc** | ~4 known issues, unknown pass rate | **1023/1215 PASS (84%)** | 6 new error categories identified |
| **PcbDoc** | 0/132 PASS | **5/132 PASS (4%)** | +5, 7 blocking categories tracked |
| **PcbLib** | ~4 known issues (Via/Fill/Text/encoding) | **28/43 PASS (65%)** | Via/Fill/CustomShapes/ModelsNoEmbed resolved; 5 new issues |
| **PcbLib roundtrip** | 106 diff issues on 28Pins_Project | **0 issues on all 28 passing files** | **PERFECT ROUNDTRIP** |
| **SchLib roundtrip** | not tested | 128/129 save OK; 33/125 clean diff (26%) | Win-1252 encoding + ALLPINCOUNT + sidecar casing |

### Semantic Diff Tool: Case-Insensitive Default (FIXED)

The `cfb diff --semantic` tool now compares parameter keys **case-insensitively** by
default, matching Altium's documented behavior. Use `--case-sensitive-keys` to opt into
the old behavior. This eliminated ~42K phantom `MissingParamPair` issues caused by our
serializer writing MixedCase keys vs originals' UPPERCASE keys.
