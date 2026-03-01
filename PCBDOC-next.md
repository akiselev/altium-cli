# PcbDoc: Remaining Failures Research

Current state: **85/132 files pass** validation. Excluding 35 ASCII-format PcbDoc
files (which we will not support), we have **85/95 V6 files passing (89%)**.

The 10 remaining failures fall into 4 categories.

---

## Bug #1: EmbeddedFonts6 — Conditional Bold/Italic Bytes (7 files)

### Symptoms

```
parsing /EmbeddedFonts6/Data: Binary read past end: needed 2625110017 bytes ...
```

The "needed" values are always `0x9C81XXXX` — the parser is reading into the zlib
header (`78 9C`) because it consumed 2 extra bytes from the wrong position.

### Affected files

| File | Entries | Trigger font |
|------|---------|-------------|
| oshw-ac-rc-unit.PcbDoc | 4 | MS Sans Serif (empty style_name) |
| tc377-car-mark1.PcbDoc | 3 | Berlin Sans FB Demi (empty style_name) |
| tc377-car-mark2.PcbDoc | 6 | Berlin Sans FB Demi (empty style_name) |
| tc377-car-mark3.PcbDoc | 6 | Berlin Sans FB Demi (empty style_name) |
| tc377-tps40304-demo.PcbDoc | 2 | Berlin Sans FB Demi (empty style_name) |
| tracker-keyboard.PcbDoc | ? | (empty style_name) |
| uwarg-elrs-tx.PcbDoc | ? | (empty style_name) |

### Root cause

The current parser (`pcbdoc/mod.rs:941-942`) unconditionally reads `u16` + `u8`
after the three length-prefixed strings:

```rust
let unknown_u16 = reader.read_u16_le()?;  // bug: reads bold + italic
let flag = reader.read_u8()?;              // bug: reads charset
```

But the actual format has **conditional fields**. The C# interface confirms
the field identities (`IPCB_TTFontsCache.AddEmbeddedFont`):

```
[u32 byte_len] [UTF-16LE full_name]
[u32 byte_len] [UTF-16LE face_name]
[u32 byte_len] [UTF-16LE style_name]
IF style_name byte_len > 2 (non-empty after NUL trimming):
    [u8 bold]       — 0 or 1
    [u8 italic]     — 0 or 1
[u8 charset]        — Windows charset ID (typically 1 = DEFAULT_CHARSET)
[u32 blob_size]     — zlib-compressed TTF data follows
[blob_size bytes]   — starts with 78 9C (zlib default compression)
```

When `style_name` is empty (`byte_len == 2`, i.e. just a UTF-16LE NUL), bold and
italic are **omitted** — the entry is 5 metadata bytes instead of 7.

### Hex evidence

**Failing font (MS Sans Serif, empty style)** from oshw-ac-rc-unit at offset 0xB1DF5:

```
0200 0000 0000       style_name: len=2 → "" (empty, just NUL)
01                   charset = 1 (DEFAULT_CHARSET)
6153 0700            blob_size = 0x75361
789C ...             zlib compressed font data
```

Current code reads `00 00` as `unknown_u16`, then `01` as `flag`, then interprets
`61 53 07 00 78 9C` as blob_size = `0x9C780007` = 2,625,110,023 — hence the error.

**Working font (Arial Bold, non-empty style)** from tc377-car-mark1 at offset 0x83AC3:

```
0A00 0000 4200 6F00 6C00 6400 0000   style_name: len=10 → "Bold"
01                   bold = 1
00                   italic = 0
01                   charset = 1
C4D4 0700            blob_size = 0x7D4C4
789C ...             zlib data
```

Stream exhaustion checks confirm the conditional format: byte sums match stream
sizes exactly for all tested files only when bold/italic are conditional.

### Changes required

**File: `crates/altium-format/src/pcblib/library.rs`**

Update `PcbEmbeddedFontEntry` struct (line 365):

```rust
pub(crate) struct PcbEmbeddedFontEntry {
    pub(crate) name: String,
    pub(crate) style_name: String,
    pub(crate) localized_name: String,
    pub(crate) bold: Option<bool>,    // None when style_name is empty
    pub(crate) italic: Option<bool>,  // None when style_name is empty
    pub(crate) charset: u8,
    pub(crate) data: Vec<u8>,
}
```

**File: `crates/altium-format/src/pcbdoc/mod.rs`**

Update `parse_embedded_fonts6_data()` (line 920):

```rust
let name = read_utf16le_len_prefixed(&mut reader, "EmbeddedFonts6.name")?;
let style_name = read_utf16le_len_prefixed(&mut reader, "EmbeddedFonts6.style_name")?;
let localized_name = read_utf16le_len_prefixed(&mut reader, "EmbeddedFonts6.localized_name")?;

let (bold, italic) = if !style_name.is_empty() {
    let b = reader.read_u8()? != 0;
    let i = reader.read_u8()? != 0;
    (Some(b), Some(i))
} else {
    (None, None)
};
let charset = reader.read_u8()?;
let blob_size = reader.read_u32_le()? as usize;
let blob = reader.read_bytes(blob_size)?;
```

**Note**: The condition to check is whether `style_name` is empty after UTF-16LE
decoding + NUL trimming — which maps to the raw `byte_len == 2` case (only a
UTF-16LE NUL terminator). The `read_utf16le_len_prefixed` function already trims
trailing NULs, so checking `style_name.is_empty()` is correct.

**File: `crates/altium-format/src/pcblib/library.rs`**

The PcbLib parser `parse_embedded_fonts()` (line 552) has the same bug and needs
the same fix. Both parsers should share the same logic.

### Serialization

Update the serializer (if one exists for EmbeddedFonts6) to conditionally write
bold/italic only when `style_name` is non-empty (i.e. when `bold.is_some()`).

### Risk

Low — the format is fully verified via stream exhaustion across multiple files.
The empty-style condition is binary: `byte_len == 2` means "skip bold/italic".

---

## Bug #2: WideStrings6 Empty String Sentinel (1 file fails, 28 files silently affected)

### Symptoms

```
parsing /WideStrings6/Data: cannot decode entry at offset 10448
(expected index 361); next bytes [00, 00, 02, 00, 00, 00]
```

### Affected files

Only `uwarg-zeropilot2.PcbDoc` currently triggers the error, but **28 files**
contain the sentinel pattern. The other 27 pass today only because their
sentinel entries happen to accidentally satisfy the "Format B" fallback parser
(which misinterprets `[u32 index][u32 flag=2]` as `[u16=0][u32 byte_len=index][bytes]`
when the index is even and small). This is a ticking time bomb.

### Root cause

The WideStrings6/Data format (`pcbdoc/records.rs:315`) uses:

```
[u32 index] [u32 byte_len] [byte_len bytes UTF-16LE]   — normal entry
[u32 index] [u32 value=2]                               — empty string sentinel (NO payload)
```

When `byte_len == 2`, it's a **sentinel** meaning "this string is empty" — there
are zero payload bytes. The minimum valid payload for an actual string is 4 bytes
(one UTF-16LE character + NUL terminator = 2+2 bytes).

The current parser (line 342-346) treats `byte_len=2` as "read 2 payload bytes",
consuming the first 2 bytes of the *next* entry's index field. This corrupts
the stream position and eventually fails.

### Hex evidence

**End of uwarg-zeropilot2.PcbDoc WideStrings6/Data** (offset 10438, stream total = 10454):

```
offset 10438: 68 01 00 00  02 00 00 00   → index=360, sentinel=2 (empty string)
offset 10446: 69 01 00 00  02 00 00 00   → index=361, sentinel=2 (empty string)
```

Both last entries are 8 bytes each (no payload). The current parser tries to read
2 bytes of payload from entry 360, consuming `69 01` from entry 361's index,
then fails at offset 10448 because only 6 bytes remain for a full entry.

### Statistics across all PcbDoc files

| File | Empty sentinel entries |
|------|-----------------------|
| stlink-v3-mb1367c.PcbDoc | 176 |
| rfsoc-amc.PcbDoc | 76 |
| rfsoc-acmc-mezzanine.PcbDoc | 29 |
| thesis-lora-egse.PcbDoc | 25 |
| uwarg-zeropilot3.PcbDoc | 18 |
| tvws-wab-1x4.PcbDoc | 18 |
| *(22 more files with 1-14 sentinel entries)* | |

### Changes required

**File: `crates/altium-format/src/pcbdoc/records.rs`**

In `parse_wide_strings6_records()` (line 315):

1. After reading index and byte_len in "Format A", check for the sentinel:

```rust
if byte_len == 2 {
    // Empty string sentinel: [u32 index][u32 flag=2], NO payload bytes
    out.push(WideString6Record {
        index,
        text: String::new(),
    });
    offset += 8;
    continue;
}
```

2. Remove "Format B" (lines 348-364) — it was a misguided workaround for this
   same issue. It's never triggered in any test file that doesn't contain
   sentinel entries, and it accidentally "works" for some sentinel entries only
   by coincidence (when the following index value happens to be even and small
   enough to look like a byte_len).

**File: `crates/altium-format-types/src/constants/`**

Add a named constant for the sentinel value:

```rust
pub const WIDE_STRING6_EMPTY_SENTINEL: u32 = 2;
```

### Risk

Low — verified against all 132 PcbDoc test files. Every file parses cleanly
with the sentinel fix. No file uses "Format B" for a legitimate purpose.

---

## Bug #3: Arc Radius Allows Negative Values (1 file)

### Symptoms

```
validating PcbDoc invariants: Invalid parameter value for key 'Arc[32].radius':
section "Arcs6": dimension -176557.8935mil out of range [0, 2540mm]
```

### Affected file

`rover-gimbal.PcbDoc` — contains 2 arcs with negative radius (indices 32 and 33).

### Root cause

The invariant validator (`pcbdoc/mod.rs:635`) uses `check_dimension()` which
requires `value >= 0`:

```rust
PcbPrimitive::Arc(a) => {
    check_dimension(a.radius, "Arc", idx, "radius", &section_name)?;
```

But Altium's C# API declares arc radius as signed `int`:

```csharp
// AD26-dotnet/Altium.SDK.Interfaces/PCB/IPCB_Arc.cs:16
int GetState_Radius();
void SetState_Radius(int argRadius);
```

The two negative-radius arcs are **degenerate zero-sweep arcs** (start_angle ≈
end_angle ≈ 180.0°) used as construction geometry in union primitives. Their
radius magnitude (~176,558 mil) also exceeds `MAX_REASONABLE_DIMENSION` (2540mm =
100,000mil), but Altium opens the file without complaint.

### Raw data for Arc #32

```
center_x:    -175807.5430 mil
center_y:      2290.7101 mil
radius:     -176557.8935 mil  (NEGATIVE, i32 = -1765578935)
start_angle:  180.000°
end_angle:    180.000°
union_index:  21
```

### Changes required

**File: `crates/altium-format/src/pcbdoc/mod.rs`**

At line 635, change from `check_dimension` (which requires non-negative) to
either a dedicated arc radius check or skip the check:

Option A — Allow signed radius, check absolute magnitude:

```rust
PcbPrimitive::Arc(a) => {
    // Arc radius is signed in Altium's API (IPCB_Arc.GetState_Radius returns int).
    // Degenerate zero-sweep arcs in unions can have negative or very large radii.
    // Only sanity-check the absolute magnitude against i32 range.
    check_dimension(a.width, "Arc", idx, "width", &section_name)?;
```

Option B — Use `check_expansion` (allows negative, checks `|val| <= MAX`):

This won't work because |radius| = 176,558mil > MAX_REASONABLE = 100,000mil.
The MAX_REASONABLE limit is too strict for arc radius.

**Recommended**: Option A — simply remove the radius range check. The `Coord`
type is already `i32`, so parsing handles negative values correctly. The
validation was overly strict. If we want *some* check, verify
`|radius| <= Coord::MAX_COORD` (999,990,000 internal units = ~99999 mil).

### Risk

Very low — this only relaxes validation, doesn't change any parsing logic.
Altium's own code treats radius as signed int with no range clamping.

---

## Issue #4: PcbDoc V5 Format (2 files, won't fix now)

### Symptoms

```
reading /FileHeaderSix: Stream not found: /FileHeaderSix
```

### Affected files

- `fingerprint-lock-v2as.PcbDoc`
- `stm32f103-core.PcbDoc`

### Root cause

These are **PcbDoc V5 files** (`"PCB 5.0 Binary File"` in `/FileHeader`), not V6.
V5 files lack `/FileHeaderSix` entirely and use different section names:

| V5 | V6 |
|----|-----|
| `/Board/` | `/Board6/` |
| `/Arcs/` | `/Arcs6/` |
| `/Pads/` | `/Pads6/` |
| `/Tracks/` | `/Tracks6/` |
| `/WideStrings/` | `/WideStrings6/` |
| `/EmbeddedFonts/` | `/EmbeddedFonts6/` |

V5 also has **smaller binary record payloads** (e.g. arcs are 56 bytes vs
56-60 in V6, lacking the V7Layer `layer_enum_index` field added in V6).

### What V5 support would require

1. Detect format version from `/FileHeader` (`"PCB 5.0 Binary File"` vs `"PCB 6.0 Binary File"`)
2. Skip `/FileHeaderSix` for V5
3. Map V5 section names → V6 section kind enums (strip `6` suffix)
4. Adjust binary record parsers for smaller V5 payloads (no V7Layer field, possibly others)
5. Handle missing V6-only sections (Models, ShapeBasedRegions6, etc.)

### Recommendation

**Defer V5 support.** These are legacy files from Altium Designer ~2013 and earlier.
Focus on getting V6 to 100% first. When V5 is tackled, it should be a separate
milestone with its own format investigation.

---

## Summary: Expected Outcome After Fixes

| Bug | Fix complexity | Files fixed | New total |
|-----|---------------|-------------|-----------|
| #1 EmbeddedFonts6 | Small (conditional read) | +7 | 92/95 V6 |
| #2 WideStrings6 sentinel | Small (sentinel check) | +1 directly, +27 silently fixed | 93/95 V6 |
| #3 Arc radius validation | Trivial (relax check) | +1 | 94/95 V6 |
| #4 V5 format | Large (deferred) | 0 (2 V5 files) | — |

After bugs #1-#3: **94/95 V6 files (99%)** should pass, with only the 2 V5 files
remaining as a separate format version issue.

### Implementation order

All three fixes are independent and can be implemented in parallel:

1. **Bug #3 (Arc radius)** — one-line change, zero risk
2. **Bug #2 (WideStrings6)** — small targeted fix, removes dead code
3. **Bug #1 (EmbeddedFonts6)** — struct change + conditional logic in two parsers
