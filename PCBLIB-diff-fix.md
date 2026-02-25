# PcbLib Roundtrip Fix Report

Generated: 2026-02-25
Test file: `data/pcblib/28Pins_Project.PcbLib`

## Diff Summary

```
Total issues: 433
  BlockParseError:     52   (PCB binary Data streams unparseable by block parser)
  EntryMissingInB:    113   (sidecar streams not written back)
  MissingParamPair:   214   (parameters dropped or reformatted)
  RawByteMismatch:     27   (first differing byte in raw binary streams)
  StreamLengthMismatch: 26  (stream size differences)
  UpdatedParamValues:    1  (ComponentParamsTOC Description leading \r\n)
```

File sizes: original 3,610,112 bytes, roundtrip 3,506,176 bytes (-103,936 bytes / -2.88%)

---

## Issue Inventory

### Issue 1: Text Primitive Tail Fields Not Serialized [CRITICAL - DATA LOSS]

**Symptoms:** Every footprint's `/Data` stream is shorter. The consistent `A=0xfc B=0xe1`
raw byte mismatch is the u32 subrecord-0 length field: original = 0xFC (252 bytes, AD26
format), roundtrip = 0xE1 (225 bytes, base format). This accounts for 26 of the 27
`StreamLengthMismatch` issues and 26 of the 27 `RawByteMismatch` issues.

**Root cause:** `serialize_text()` at `pcblib/mod.rs:1127-1163` writes only the 225-byte
base format and stops after `barcode_font_name`. The AD26 tail fields (bytes 225-251) are
parsed and stored in `PcbText` but never written back.

**Lost fields (bytes 225-251):**

| Offset | Size | Field | Sample Value |
|--------|------|-------|-------------|
| 225 | 1 | `ttf_inverted_justify` | 1 |
| 226 | 2 | `ttf_offset_from_inverted_rect` | 29 |
| 228 | 1 | `multiline_auto_position` | 2 |
| 229 | 1 | `is_advance_justification_valid` | 1 |
| 230 | 1 | `advance_snapping` | 0 |
| 231 | 1 | reserved | 0 |
| 232 | 4 | `advance_justification_x` (i32) | 0 |
| 236 | 4 | `advance_justification_y` (i32) | 0 |
| 240 | 4 | `use_text_alignment_by_snap` (i32) | 1 |
| 244 | 4 | `snap_point_x` (Coord) | -590000 |
| 248 | 4 | `snap_point_y` (Coord) | -240000 |

**Fix:** Extend `serialize_text()` to always write the AD26 252-byte format. All fields
are already stored in the `PcbText` struct. Append the tail bytes after
`barcode_font_name`. The format is version-progressive (225 < 230 < 244 < 252), but since
we upgrade to latest format on save, always write the full 252 bytes.

**Secondary text issue:** Subrecord 1 (text content) in original has a leading `\r`
character (`\r'.Designator'`) that the serializer omits (`'.Designator'`). Check whether
`PcbText.text` strips the leading `\r` during parse and whether it should be restored on
save.

---

### Issue 2: ComponentBody Missing Parameters [CRITICAL - DATA LOSS]

**Symptoms:** ComponentBody records are consistently ~194 bytes shorter in roundtrip. Multiple
parameters are dropped entirely, and numeric formatting differs.

**Root cause:** `serialize_component_body()` at `pcblib/mod.rs:1326-1369` omits several
parameters that are parsed and stored.

#### 2a. IDENTIFIER not serialized

Parsed at `component_body.rs:136-139` as comma-separated byte values, decoded to string.
Never written back. This is a critical data field used by Altium for cross-referencing.

**Fix:** Add `serialize_identifier()` that re-encodes the identifier string as
comma-separated byte values and inserts `IDENTIFIER=...` into the parameter block.

#### 2b. TEXTURE* parameters not serialized

Parsed fields: `TEXTURE`, `TEXTURECENTERX`, `TEXTURECENTERY`, `TEXTURESIZEX`,
`TEXTURESIZEY`, `TEXTUREROTATION`. Only `TEXTURE` is written; the other 5 are lost.

**Fix:** Serialize all 6 texture parameters. `TEXTUREROTATION` uses scientific notation
format (`0.00000000000000E+0000`).

#### 2c. BODYOVERRIDECOLOR not serialized

Parsed at `component_body.rs:149` as boolean flag. Never written back.

**Fix:** Serialize as `BODYOVERRIDECOLOR=TRUE` or `BODYOVERRIDECOLOR=FALSE`.

#### 2d. MODEL.S{n}X/Y/Z snap points not serialized

The parser reads `MODEL.SNAPCOUNT` and loops to populate `model_snap_points` vector
(lines 191-207). The serializer writes `MODEL.SNAPCOUNT=N` but **never writes the
individual snap point parameters** `MODEL.S0X`, `MODEL.S0Y`, `MODEL.S0Z`, etc.

**Fix:** After writing `MODEL.SNAPCOUNT`, loop over `model_snap_points` and write each
`MODEL.S{i}X`, `MODEL.S{i}Y`, `MODEL.S{i}Z` as internal coordinate values.

#### 2e. Numeric formatting mismatches

| Parameter | Original | Roundtrip | Issue |
|-----------|----------|-----------|-------|
| `BODYOPACITY3D` | `1.000` | `1` | `f64.to_string()` drops trailing zeros |
| `MODEL.2D.ROTATION` | `0.000` | `0` | Same |
| `MODEL.3D.ROTX/Y/Z` | `0.000`/`90.000`/`180.000` | `0`/`90`/`180` | Same |
| `ARCRESOLUTION` | `0.5mil` | `0.5000mil` | `{:.4}mil` adds excess precision |
| `STANDOFFHEIGHT` | `0.5mil` | `0.5000mil` | Same |
| `OVERALLHEIGHT` | `47.744mil` | `47.7440mil` | Same |
| `CAVITYHEIGHT` | `0mil` | `0.0000mil` | Same |

**Fix for rotations/opacity:** Use `format!("{:.3}", value)` to match Altium's 3-decimal
formatting for bare float values. Altium uses `.3f` for rotation and opacity values.

**Fix for mil values:** Match Altium's native formatting. Altium strips trailing zeros from
mil values (e.g., `0.5mil` not `0.5000mil`, `0mil` not `0.0000mil`). Implement a
formatting function that strips unnecessary trailing zeros after the decimal while
preserving at least the minimum needed (e.g., `47.744mil`, `0.5mil`, `0mil`).

---

### Issue 3: Missing Per-Footprint Sidecar Streams [CRITICAL]

**Symptoms:** 113 `EntryMissingInB` issues. Every footprint is missing its sidecar streams.

#### 3a. WideStrings (28 missing streams)

**What:** `/{footprint}/WideStrings` - Unicode string overrides for text primitives.
PcbLib uses parameter-block format (NOT binary TLV like PcbDoc).

**Parse:** `parse_pcblib_wide_strings()` in `wide_strings.rs:21-85`. Parsed and merged
into `PcbText.text` at load time.

**Save:** No serialization code exists.

**Fix:** Add `serialize_pcblib_wide_strings()` that:
1. Iterates footprint primitives
2. For each text primitive whose text contains non-Windows-1252 characters, encodes as
   `ENCODEDTEXT{N}=code1,code2,...` (comma-separated UTF-16LE code units)
3. Writes as a single text block in `/{footprint}/WideStrings`

**Format:**
```
[4-byte block header: flags=0x00, size=N]
|ENCODEDTEXT0=72,101,108,108,111|ENCODEDTEXT5=87,111,114,108,100|\0
```

#### 3b. UniqueIDPrimitiveInformation (75 missing: 25 storages + 25 Data + 25 Header)

**What:** `/{footprint}/UniqueIDPrimitiveInformation/{Header,Data}` - Unique IDs for
primitives used for cross-document tracking and design rule references.

**Parse:** `parse_unique_id_primitive_information()` in `sidecar.rs:30-79`. Merged into
each primitive's `unique_id` field.

**Save:** No serialization code exists.

**Fix:** Add `serialize_unique_id_primitive_information()` that:
1. Creates storage `/{fp}/UniqueIDPrimitiveInformation`
2. Writes Header: `serialize_u32_header(count)`
3. Writes Data: one text block per primitive that has a non-empty `unique_id`, with params:
   `PRIMITIVEINDEX={index}|PRIMITIVEOBJECTID={object_id}|UNIQUEID={uid}`

#### 3c. Library/ModelsNoEmbed (3 missing: 1 storage + Data + Header)

**What:** `/Library/ModelsNoEmbed/{Header,Data}` - Metadata for models not embedded in
the library (external file references).

**Status:** Parsed and stored in `PcbLib.models_no_embed`. **Already has serialization
code** at `pcblib/mod.rs:768-778` via `serialize_model_entries()`.

**Root cause:** The save code writes ModelsNoEmbed only if `!self.models_no_embed.is_empty()`.
The original file likely has empty ModelsNoEmbed streams (Header with count=0, empty Data).
Altium always writes these streams even when empty.

**Fix:** Always write the ModelsNoEmbed storage/streams, even when the list is empty.
Write Header with count=0 and Data as empty bytes.

#### 3d. Library/Textures (3 missing: 1 storage + Data + Header)

**What:** `/Library/Textures/{Header,Data}` - Texture metadata for 3D model rendering.

**Status:** Parsed and stored. **Already has serialization code** at `pcblib/mod.rs:802-817`.
Same issue as ModelsNoEmbed -- only written if non-empty.

**Fix:** Always write the Textures storage/streams, even when empty.

---

### Issue 4: Library/Data V9 Layer Stack Not Serialized [CRITICAL - DATA LOSS]

**Symptoms:** `/Library/Data` shrinks from 95,008 bytes to 176 bytes. The V9 layer stack,
board configuration, design rules, and component-name index are all lost.

**Root cause:** The save code at `pcblib/mod.rs:737-739` only writes the basic library
metadata (FILENAME, KIND, VERSION, DATE, TIME). The `board_config` field (parsed via
`parse_board_config()` in `board_config.rs`) is stored in memory but has no serialization
function.

**What's lost:**
- V9 master stack and substacks (`V9_MASTERSTACK_*`, `V9_STACK_LAYER*_*`)
- V8/V7 layer definitions
- Board dimensions, surface properties, grid settings, viewport config
- Design rules
- Component-name index suffix (binary format after the text block)

**Fix:** Implement `serialize_board_config()` in `board_config.rs` that writes all parsed
fields back as `|KEY=VALUE|` parameters appended to the library metadata text block. Also
re-serialize the component-name index suffix.

**Scope:** This is the single largest piece of work. The board_config parser handles
hundreds of parameters across V9/V8/V7 layer stacks, surface properties, grid settings,
viewport, and more. All must be serialized in the correct order.

---

### Issue 5: Library/Models/Data Formatting Issues [MEDIUM]

**Symptoms:** 152 `MissingParamPair` issues in `/Library/Models/Data`.

#### 5a. MODELSOURCE=Undefined not written

**Root cause:** Parsed at `library.rs:293-295` but not stored in `PcbLibModelEntry` struct.
The field is consumed during parse but discarded.

**Fix:** Add `model_source: String` field to `PcbLibModelEntry`. Parse and store it.
Serialize back in `serialize_model_entries()`.

#### 5b. Rotation values lose `.000` formatting

**Root cause:** Rotations serialized via `entry.rotation_x.to_string()` which produces
`"0"` instead of `"0.000"` and `"90"` instead of `"90.000"`.

**Fix:** Use `format!("{:.3}", value)` for model rotation values in
`serialize_model_entries()`.

---

### Issue 6: PadViaLibrary/Header Count Bug [LOW]

**Symptoms:** `raw byte mismatch at /Library/PadViaLibrary/Header offset 0: A=0x00, B=0x01`

**Root cause:** `pcblib/mod.rs:791` hardcodes `serialize_u32_header(1)`. The original file
has Header = `00 00 00 00` (count = 0). Altium writes count=0 in the header even when Data
has content (the parser ignores the count and reads all available blocks).

**Fix:** Store the original header count during parse and use it during save. Or compute
the block count from the actual Data content (number of text blocks being written).

---

### Issue 7: ComponentParamsTOC Issues [LOW]

**Symptoms:** 5 issues in `/Library/ComponentParamsTOC/Data`:
- `Description=\r\n` (original) vs `Description=` (roundtrip) for first entry
- `Height=0` (original) vs `Height=0.0000mil` (roundtrip)

#### 7a. Description leading `\r\n`

**Root cause:** The original file's Description field for the first component starts with
`\r\n`, but the roundtrip starts with empty string. This is a formatting artifact in the
original -- the first entry's description begins with `\r\n` as a separator, but our
serializer starts the first entry directly.

**Fix:** Prepend `\r\n` to each Description value in the TOC to match Altium's behavior:
```
Description=\r\nName=10118193
```
rather than:
```
Description=Name=10118193
```

Wait -- looking more carefully at the diff, the Description field IS a multi-value field
where each entry starts with `\r\n`. The original has `\r\n` for the FIRST entry's
description (just the separator alone), while the roundtrip produces `""` (empty). This
suggests the description is already being set to empty for the first component, but the
original file writes `\r\n` as the empty description value.

**Fix:** If description is empty, write `\r\n` instead of empty string. Need to verify
against Altium's actual behavior.

#### 7b. Height=0 vs Height=0.0000mil

**Root cause:** Original file stores `Height=0` (bare integer, no unit), roundtrip writes
`Height=0.0000mil` (from `format!("{:.4}mil", height.to_mils())`).

**Fix:** Use the same formatting as Altium. When height is exactly 0, write `0` without
unit suffix. When non-zero, write `{value}mil` with native precision.

---

### Issue 8: Footprint Parameters HEIGHT Formatting [LOW]

**Symptoms:** 56 `MissingParamPair` issues (28 per side). Every footprint's Parameters
stream shows `HEIGHT=0mil` (original) vs `HEIGHT=0.0000mil` (roundtrip).

**Root cause:** Footprint parameters serialize HEIGHT using `format!("{:.4}mil", ...)`.
Altium writes `0mil` for zero values (no decimal places).

**Fix:** Same as Issue 7b -- strip trailing zeros and unnecessary decimal point from mil
formatting. `0mil` not `0.0000mil`, `0.5mil` not `0.5000mil`.

---

### Issue 9: ExtendedPrimitiveInformation / PrimitiveGuids Not Serialized [MEDIUM]

Not visible in the 28Pins diff (this file may not have them), but identified during code
review.

**ExtendedPrimitiveInformation:**
- Parsed at `sidecar.rs:123-194`
- Stored in `PcbFootprint.extended_primitive_info`
- Contains mask expansion modes per primitive
- No serialization code

**PrimitiveGuids:**
- Parsed at `sidecar.rs:203-248`
- Stored in `PcbFootprint.primitive_guids`
- Contains GUIDs for primitives (24-byte binary records)
- No serialization code

**Fix:** Implement serialization for both. Same pattern as UniqueIDPrimitiveInformation.

---

## Fix Priority

| Priority | Issue | Category | Impact | Effort |
|----------|-------|----------|--------|--------|
| P0 | Text tail fields (Issue 1) | DATA LOSS | Snap points, justification lost | Small - append known fields |
| P0 | ComponentBody params (Issue 2) | DATA LOSS | IDENTIFIER, textures, snap points lost | Medium - add missing params |
| P0 | Library/Data board_config (Issue 4) | DATA LOSS | Entire layer stack lost (~95KB) | Large - serialize hundreds of params |
| P1 | WideStrings sidecar (Issue 3a) | DATA LOSS | Unicode text lost | Medium |
| P1 | UniqueIDPrimitiveInformation (Issue 3b) | DATA LOSS | Primitive tracking IDs lost | Medium |
| P1 | ExtendedPrimitiveInformation (Issue 9) | DATA LOSS | Mask expansion settings lost | Medium |
| P1 | PrimitiveGuids (Issue 9) | DATA LOSS | Primitive GUIDs lost | Small |
| P2 | MODELSOURCE parameter (Issue 5a) | Missing field | Model source metadata lost | Small |
| P2 | ModelsNoEmbed/Textures empty (Issue 3c/3d) | Missing streams | Streams not written when empty | Trivial |
| P2 | Rotation formatting (Issue 5b) | Formatting | `.000` precision lost | Trivial |
| P3 | Mil value formatting (Issues 2e, 7b, 8) | Formatting | `0mil` vs `0.0000mil` | Small - custom formatter |
| P3 | PadViaLibrary header (Issue 6) | Wrong count | Count=1 instead of 0 | Trivial |
| P3 | ComponentParamsTOC Description (Issue 7a) | Formatting | Leading `\r\n` | Trivial |
| P3 | Text leading `\r` (Issue 1 secondary) | Formatting | `\r` prefix on text content | Trivial |

---

## Implementation Plan

### Phase 1: Critical data loss fixes (P0)

1. **Text tail fields** - Extend `serialize_text()` to write 252-byte AD26 format
2. **ComponentBody params** - Add IDENTIFIER, TEXTURE*, BODYOVERRIDECOLOR, MODEL.S{n}X/Y/Z
3. **Board config serialization** - Implement `serialize_board_config()` (largest task)

### Phase 2: Sidecar stream serialization (P1)

4. **WideStrings** - `serialize_pcblib_wide_strings()`
5. **UniqueIDPrimitiveInformation** - Header + Data serialization
6. **ExtendedPrimitiveInformation** - Header + Data serialization
7. **PrimitiveGuids** - 24-byte binary record serialization

### Phase 3: Formatting and completeness (P2/P3)

8. **MODELSOURCE field** - Add to PcbLibModelEntry struct + serialize
9. **Empty stream writing** - Always write ModelsNoEmbed/Textures even if empty
10. **Mil formatting** - Implement Altium-native mil formatter (strip trailing zeros)
11. **Float formatting** - Use `.3f` for rotations/opacity
12. **PadViaLibrary header** - Fix hardcoded count
13. **TOC Description** - Fix leading `\r\n`
14. **Text leading `\r`** - Verify and fix text content prefix

### Verification

After each phase, re-run:
```bash
cargo run --release -- save-as data/pcblib/28Pins_Project.PcbLib /tmp/28pins_roundtrip.PcbLib
cargo run --release -- cfb diff --semantic data/pcblib/28Pins_Project.PcbLib /tmp/28pins_roundtrip.PcbLib
```

Target: 0 issues (or only BlockParseError for PCB binary Data streams which are expected
when the block parser hits raw binary records).

---

## Key Source Files

| File | Purpose |
|------|---------|
| `crates/altium-format/src/pcblib/mod.rs:732-841` | PcbLib save implementation |
| `crates/altium-format/src/pcblib/mod.rs:1127-1163` | `serialize_text()` |
| `crates/altium-format/src/pcblib/mod.rs:1326-1369` | `serialize_component_body()` |
| `crates/altium-format/src/pcblib/mod.rs:898-915` | `serialize_component_toc_data()` |
| `crates/altium-format/src/pcblib/primitives/text.rs` | Text primitive parse |
| `crates/altium-format/src/pcblib/primitives/component_body.rs` | ComponentBody parse |
| `crates/altium-format/src/pcblib/library.rs` | Library metadata parse |
| `crates/altium-format/src/pcblib/wide_strings.rs` | WideStrings parse |
| `crates/altium-format/src/pcblib/sidecar.rs` | UniqueID/Extended/Guids parse |
| `crates/altium-format/src/pcblib/footprint.rs` | Footprint load orchestration |
| `crates/altium-format/src/board_config.rs` | Board config parse (no serialize) |
