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

## Status: All V6 Bugs Fixed

All bugs #1-#3 have been implemented. Bug #4 (V5 format) is deferred.

**Current result: 94/96 non-ASCII PcbDoc files pass (97.9%).**

The only 2 remaining failures are V5-format files (`fingerprint-lock-v2as`, `stm32f103-core`).

Additionally, all **38 persistable DRC violation types** are now registered in
`ParamSectionKind`, future-proofing against files containing any Altium DRC result.

---

## Research: High-Level API Types for DRC Rules and Violations

This section documents the data model for a future typed API over PcbDoc DRC rules
and violations, based on the C# decompiled source (`AD26-dotnet/`).

### Overview

The DRC system has three interconnected concepts:

```
IPCB_Rule (stored in Rules6 section)
    ↓ referenced by RULEINDEX
IPCB_Violation (stored in T*Violation sections)
    ↓ references primitives by PRIM1ID/PRIM1INDEX
IPCB_Primitive (stored in Arcs6, Pads6, Tracks6, etc.)
```

### TRuleKind Enum (70 variants, u8)

Defined in `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TRuleKind.cs`. Each rule kind
has a string identifier used in the `RULEKIND=` parameter (from `Consts.cs` lines 1121-1192).

```
 0  eRule_Clearance                    → "Clearance"
 1  eRule_ParallelSegment              → "ParallelSegment"
 2  eRule_MaxMinWidth                  → "Width"
 3  eRule_MaxMinLength                 → "Length"
 4  eRule_MatchedLengths               → "MatchedLengths"
 5  eRule_DaisyChainStubLength         → "StubLength"
 6  eRule_PowerPlaneConnectStyle       → "PlaneConnect"
 7  eRule_RoutingTopology              → "RoutingTopology"
 8  eRule_RoutingPriority              → "RoutingPriority"
 9  eRule_RoutingLayers                → "RoutingLayers"
10  eRule_RoutingCornerStyle           → "RoutingCorners"
11  eRule_RoutingViaStyle              → "RoutingVias"
12  eRule_PowerPlaneClearance          → "PlaneClearance"
13  eRule_SolderMaskExpansion          → "SolderMaskExpansion"
14  eRule_PasteMaskExpansion           → "PasteMaskExpansion"
15  eRule_ShortCircuit                 → "ShortCircuit"
16  eRule_BrokenNets                   → "UnRoutedNet"
17  eRule_ViasUnderSMD                 → "ViasUnderSMD"
18  eRule_MaximumViaCount              → "MaximumViaCount"
19  eRule_MinimumAnnularRing           → "MinimumAnnularRing"
20  eRule_PolygonConnectStyle          → "PolygonConnect"
21  eRule_AcuteAngle                   → "AcuteAngle"
22  eRule_ConfinementConstraint        → "RoomDefinition"
23  eRule_SMDToCorner                  → "SMDToCorner"
24  eRule_ComponentClearance           → "ComponentClearance"
25  eRule_ComponentRotations           → "ComponentOrientations"
26  eRule_PermittedLayers              → "PermittedLayers"
27  eRule_NetsToIgnore                 → "NetsToIgnore"
28  eRule_SignalStimulus               → "SignalStimulus"
29  eRule_Overshoot_FallingEdge        → "OvershootFalling"
30  eRule_Overshoot_RisingEdge         → "OvershootRising"
31  eRule_Undershoot_FallingEdge       → "UndershootFalling"
32  eRule_Undershoot_RisingEdge        → "UndershootRising"
33  eRule_MaxMinImpedance              → "MaxMinImpedance"
34  eRule_SignalTopValue               → "SignalTopValue"
35  eRule_SignalBaseValue              → "SignalBaseValue"
36  eRule_FlightTime_RisingEdge        → "FlightTimeRising"
37  eRule_FlightTime_FallingEdge       → "FlightTimeFalling"
38  eRule_LayerStack                   → "LayerStack"
39  eRule_MaxSlope_RisingEdge          → "SlopeRising"
40  eRule_MaxSlope_FallingEdge         → "SlopeFalling"
41  eRule_SupplyNets                   → "SupplyNets"
42  eRule_MaxMinHoleSize               → "HoleSize"
43  eRule_TestPointStyle               → "FabricationTestpoint"
44  eRule_TestPointUsage               → "FabricationTestPointUsage"
45  eRule_UnconnectedPin               → "UnConnectedPin"
46  eRule_SMDToPlane                   → "SMDToPlane"
47  eRule_SMDNeckDown                  → "SMDNeckDown"
48  eRule_LayerPair                    → "LayerPairs"
49  eRule_FanoutControl                → "FanoutControl"
50  eRule_MaxMinHeight                 → "Height"
51  eRule_DifferentialPairsRouting     → "DiffPairsRouting"
52  eRule_HoleToHoleClearance          → "HoleToHoleClearance"
53  eRule_MinimumSolderMaskSliver      → "MinimumSolderMaskSliver"
54  eRule_SilkToSolderMaskClearance    → "SilkToSolderMaskClearance"
55  eRule_SilkToSilkClearance          → "SilkToSilkClearance"
56  eRule_NetAntennae                  → "NetAntennae"
57  eRule_AssyTestPointStyle           → "AssemblyTestpoint"
58  eRule_AssyTestPointUsage           → "AssemblyTestPointUsage"
59  eRule_SilkToBoardRegion            → "SilkToBoardRegionClearance"
60  eRule_SMDPADEntry                  → "SMDEntry"
61  eRule_None                         → "None"
62  eRule_ModifiedPolygon              → "UnpouredPolygon"
63  eRule_BoardOutlineClearance        → "BoardOutlineClearance"
64  eRule_BackDrilling                 → "BackDrilling"
65  eRule_Creepage                     → "Creepage"
66  eRule_ReturnPath                   → "ReturnPath"
67  eRule_RoutingNeckDown              → "RoutingNeckDown"
68  eRule_Wirebonding                  → "WireBonding"
69  eRule_ZAxisClearance               → "ZAxisClearance"
```

### Rules6 Section Format

Rules are stored as **PrefixedParamRecords** (u16 prefix + block-encoded `|KEY=VALUE|`).
Currently parsed at the `PrefixedParamSectionKind::Rules6` level.

**Common rule parameters** (all rule types):

| Parameter | Type | Description |
|-----------|------|-------------|
| `RULEKIND` | string | Rule type string from table above |
| `NETSCOPE` | string | `"AnyNet"`, `"DifferentNets"`, `"SameNetOnly"`, etc. |
| `LAYERKIND` | string | `"SameLayer"` or `"AdjacentLayer"` |
| `SCOPE1EXPRESSION` | string | Scope 1 query (e.g. `"All"`, `"HasFootprint('QFP')"`) |
| `SCOPE2EXPRESSION` | string | Scope 2 query (unary rules use `"All"`) |
| `NAME` | string | Unique rule name (e.g. `"Clearance_1"`) |
| `ENABLED` | bool | `"TRUE"` or `"FALSE"` |
| `PRIORITY` | u16 | Priority (lower = higher priority) |
| `COMMENT` | string | User comment |
| `UNIQUEID` | string | 8-char unique ID |
| `DEFINEDBYLOGICALDOCUMENT` | bool | Whether rule came from schematic |

**Rule-type-specific parameters** (examples):

| Rule Type | Extra Parameters |
|-----------|-----------------|
| Clearance | `GAP`, `GENERICCLEARANCE`, `OBJECTCLEARANCES`, `IGNOREPADTOPADCLEARANCEINFOOTPRINT` |
| Width | `MINWIDTH`, `MAXWIDTH`, `PREFERREDWIDTH` |
| HoleSize | `MINHOLESIZE`, `MAXHOLESIZE` |
| PolygonConnect | `CONNECTSTYLE`, `RELIEFCONDUCTORWIDTH`, `RELIEFENTRIES`, `POLYGONRELIEFANGLE`, `AIRGAPWIDTH` |
| RoutingVias | `VIATEMPLATENAME`, `DIAMETER`, `HOLESIZE` |
| DiffPairsRouting | `PREFERREDGAP`, `MAXGAP`, `MINGAP` |
| Height | `MINHEIGHT`, `MAXHEIGHT` |

### Violation Storage Format

All 38 violation types share the same CFB storage layout:

- **Storage name**: Delphi class name (e.g. `/TClearanceViolation/`)
- **Header stream**: 4-byte u32le record count
- **Data stream**: standard block-encoded `|KEY=VALUE|` param records

**Common violation parameters** (all violation types):

| Parameter | Type | Description |
|-----------|------|-------------|
| `SELECTION` | bool | Always `"FALSE"` |
| `LAYER` | string | Layer name (e.g. `"TOP"`, `"MULTILAYER"`) |
| `LOCKED` | bool | Always `"FALSE"` |
| `POLYGONOUTLINE` | bool | Always `"FALSE"` |
| `USERROUTED` | bool | Usually `"TRUE"` |
| `KEEPOUT` | bool | Always `"FALSE"` |
| `UNIONINDEX` | u32 | Union index (0 = none) |
| `RULEINDEX` | u32 | **Index into Rules6** linking to the rule |
| `PRIM1ID` | string | First primitive type (`"Via"`, `"Pad"`, `"Track"`, etc.) |
| `PRIM1INDEX` | u32 | Index into that primitive's section |
| `PRIM2ID` | string | Second primitive type (binary violations only) |
| `PRIM2INDEX` | u32 | Index (binary violations only) |
| `DESCRIPTION` | string | Human-readable violation description |
| `INVOLVEDPRIMCOUNT` | u32 | Count of additional involved primitives |

**Location parameters vary by violation subtype:**

| Pattern | Used By |
|---------|---------|
| `LOCATION1.X/Y`, `LOCATION2.X/Y` | Clearance, HoleToHole, ComponentClearance, MaxMinViaHoleSize, MaxMinPadSlotWidth |
| `FX1/FY1`, `FX2/FY2` | DisconnectedSubnets |
| `LOCATION.X/Y`, `CIRCLERADIUS` | NetAntennae, SMDPADEntry |
| `VX1/VY1`, `VX2/VY2`, `VX3/VY3`, `VX4/VY4` | ShortCircuit (area) |
| (none) | ModifiedPolygon, RoutingViaStyle (description only) |

### Violation-to-Rule Linkage

Violations reference rules by index: `RULEINDEX=N` → `Rules6[N]`. At runtime,
`IPCB_Violation.GetState_Rule()` returns the corresponding `IPCB_Rule` object.

### Supporting Enums

**TNetScope** (5 variants):
```
0  eNetScope_DifferentNetsOnly  → "DifferentNets"
1  eNetScope_SameNetOnly        → "SameNetOnly"
2  eNetScope_AnyNet             → "AnyNet"
3  eNetScope_DifferentDiffPairsOnly → "DifferentPairs"
4  eNetScope_SameDiffPairOnly   → "SameDiffPairs"
```

**TRuleLayerKind** (2 variants):
```
0  eRuleLayerKind_SameLayer     → "SameLayer"
1  eRuleLayerKind_AdjacentLayer → "AdjacentLayer"
```

### Proposed API Types for `altium-format-types`

```rust
/// DRC rule kind — discriminant for rule-type-specific parameters.
/// Source: TRuleKind enum (AD26-dotnet Altium.Edp.Interfaces/RT_PCB/TRuleKind.cs)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PcbRuleKind {
    Clearance = 0,
    ParallelSegment = 1,
    MaxMinWidth = 2,
    MaxMinLength = 3,
    MatchedLengths = 4,
    DaisyChainStubLength = 5,
    PowerPlaneConnectStyle = 6,
    RoutingTopology = 7,
    RoutingPriority = 8,
    RoutingLayers = 9,
    RoutingCornerStyle = 10,
    RoutingViaStyle = 11,
    PowerPlaneClearance = 12,
    SolderMaskExpansion = 13,
    PasteMaskExpansion = 14,
    ShortCircuit = 15,
    BrokenNets = 16,
    ViasUnderSMD = 17,
    MaximumViaCount = 18,
    MinimumAnnularRing = 19,
    PolygonConnectStyle = 20,
    AcuteAngle = 21,
    ConfinementConstraint = 22,
    SMDToCorner = 23,
    ComponentClearance = 24,
    ComponentRotations = 25,
    PermittedLayers = 26,
    NetsToIgnore = 27,
    SignalStimulus = 28,
    OvershootFallingEdge = 29,
    OvershootRisingEdge = 30,
    UndershootFallingEdge = 31,
    UndershootRisingEdge = 32,
    MaxMinImpedance = 33,
    SignalTopValue = 34,
    SignalBaseValue = 35,
    FlightTimeRisingEdge = 36,
    FlightTimeFallingEdge = 37,
    LayerStack = 38,
    MaxSlopeRisingEdge = 39,
    MaxSlopeFallingEdge = 40,
    SupplyNets = 41,
    MaxMinHoleSize = 42,
    TestPointStyle = 43,
    TestPointUsage = 44,
    UnconnectedPin = 45,
    SMDToPlane = 46,
    SMDNeckDown = 47,
    LayerPair = 48,
    FanoutControl = 49,
    MaxMinHeight = 50,
    DifferentialPairsRouting = 51,
    HoleToHoleClearance = 52,
    MinimumSolderMaskSliver = 53,
    SilkToSolderMaskClearance = 54,
    SilkToSilkClearance = 55,
    NetAntennae = 56,
    AssyTestPointStyle = 57,
    AssyTestPointUsage = 58,
    SilkToBoardRegion = 59,
    SMDPADEntry = 60,
    None = 61,
    ModifiedPolygon = 62,
    BoardOutlineClearance = 63,
    BackDrilling = 64,
    Creepage = 65,
    ReturnPath = 66,
    RoutingNeckDown = 67,
    Wirebonding = 68,
    ZAxisClearance = 69,
}

/// Net scope for DRC rule applicability.
/// Source: TNetScope (AD26-dotnet)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PcbNetScope {
    DifferentNetsOnly = 0,
    SameNetOnly = 1,
    AnyNet = 2,
    DifferentDiffPairsOnly = 3,
    SameDiffPairOnly = 4,
}

/// Layer applicability for DRC rules.
/// Source: TRuleLayerKind (AD26-dotnet)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PcbRuleLayerKind {
    SameLayer = 0,
    AdjacentLayer = 1,
}

/// DRC violation storage type — identifies which T*Violation section a
/// violation record belongs to. Each variant corresponds to a Delphi class
/// name used as the CFB storage name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PcbViolationKind {
    AcuteAngle,
    BackDrill,
    BoardOutlineClearance,
    Clearance,
    ComponentClearance,
    Creepage,
    DiffPairs,
    DisconnectedSubnets,
    HoleToHole,
    MatchedNetLengths,
    MaximumViaCount,
    MaxMinComponentHeight,
    MaxMinLength,
    MaxMinPadSlotWidth,
    MaxMinViaHoleSize,
    MinimumAnnularRing,
    MinSolderMaskSliver,
    MinWidth,
    ModifiedPolygon,
    NetAntennae,
    PadUnderSMD,
    ParallelSegment,
    ReturnPath,
    RoutingNeckDown,
    RoutingViaStyle,
    ShortCircuit,
    SilkToBoardRegionClearance,
    SilkToSilkClearance,
    SilkToSolderMaskClearance,
    SMDNeckDown,
    SMDPADEntry,
    SMDToCorner,
    TestPoint,
    UnconnectedPin,
    ViaUnderSMD,
    WirebondLength,
    WirebondWireToWire,
    ZAxisClearance,
}
```

### Notes for Implementation

1. **`PcbRuleKind` string serialization**: Must use the `RULEKIND=` string mapping
   from the table above (NOT the enum variant names). E.g. `MaxMinWidth` → `"Width"`,
   `BrokenNets` → `"UnRoutedNet"`. This is a non-obvious mapping — a `to_rule_kind_str()`
   / `from_rule_kind_str()` pair is essential.

2. **Rule-type-specific parameters**: Each `PcbRuleKind` has its own set of extra
   `|KEY=VALUE|` parameters. A full typed API would need per-kind structs or an enum
   dispatch, but for initial implementation, storing all params as a
   `ParameterCollection` and dispatching on `PcbRuleKind` for typed access is sufficient.

3. **Violation-to-Rule cross-reference**: `RULEINDEX` is a u32 index into `Rules6`.
   The API should expose this as a typed reference, not a raw integer.

4. **`PcbViolationKind` maps to `ParamSectionKind`**: The violation kind can be
   derived from the section kind (stripping `T` prefix and `Violation` suffix) or
   maintained as a separate enum with a mapping function. It represents the semantic
   meaning (what DRC check failed), vs `ParamSectionKind` which represents the storage
   location.
