# PcbDoc: Open Problems

Current state: **93/96 files pass** validation (97%).
Roundtrip testing: not yet run (PcbDoc save-as not fully implemented).

Last verified: 2026-03-01

---

## Problem 1: PcbDoc V5 format — 2 files (deferred)

**Files:** `fingerprint-lock-v2as.PcbDoc`, `stm32f103-core.PcbDoc`

**Error:**
```
reading /FileHeaderSix: Stream not found: /FileHeaderSix
```

**Root cause:** These are **PcbDoc V5 files** (`"PCB 5.0 Binary File"` in
`/FileHeader`), not V6. V5 files lack `/FileHeaderSix` entirely and use
different section names (e.g. `/Arcs/` instead of `/Arcs6/`, `/Pads/` instead
of `/Pads6/`).

V5 also has **smaller binary record payloads** (e.g. arcs are 56 bytes vs 56-60
in V6, lacking the V7Layer `layer_enum_index` field).

**What V5 support would require:**
1. Detect format version from `/FileHeader` (`"PCB 5.0 Binary File"` vs
   `"PCB 6.0 Binary File"`)
2. Skip `/FileHeaderSix` for V5
3. Map V5 section names → V6 section kind enums (strip `6` suffix)
4. Adjust binary record parsers for smaller V5 payloads (no V7Layer, etc.)
5. Handle missing V6-only sections (Models, ShapeBasedRegions6, etc.)

**Status:** Deferred. These are legacy files from Altium Designer ~2013 and
earlier. Focus on V6 100% first.

---

## Problem 2: MaskExpansionMode=7 invalid enum — 1 file

**File:** `rfsoc-amc.PcbDoc`

**Error:**
```
parsing /Vias6/Data: Invalid enum value: invalid value 7 for enum MaskExpansionMode
```

**Root cause:** The `MaskExpansionMode` enum in `altium-format-types` only
defines values 0-6, but this file contains a via with value 7. This is likely
a newer Altium version adding a new expansion mode variant.

**Investigation needed:**
- Check `AD26-dotnet` for `TMaskExpansionMode` or the via interface
  `IPCB_Via.GetState_SolderMaskExpansionMode` to find the full enum definition
- Cross-reference with Delphi via reader to confirm valid range

**Fix:** Add the missing variant to the `MaskExpansionMode` enum in
`altium-format-types`.

---

## Resolved Issues

The following issues from `PCBDOC-next.md` Bugs #1-#3 have been implemented:

### Bug #1 (RESOLVED): EmbeddedFonts6 conditional bold/italic — was 7 files

EmbeddedFonts6 parser now correctly handles the conditional bold/italic bytes
when `style_name` is empty.

### Bug #2 (RESOLVED): WideStrings6 empty string sentinel — was 1 file (28 affected)

WideStrings6 parser now handles the `byte_len == 2` empty string sentinel
correctly. Format B fallback was removed.

### Bug #3 (RESOLVED): Arc radius allows negative values — was 1 file

Arc radius validation now accepts negative values (signed `int` per Altium's
C# API).

### Previously known issues now resolved

- ShapeBasedRegions6 trailing data — fixed
- PadViaLibrary multi-record — fixed
- Unsupported storage names — fixed (38 DRC violation types registered)
- WideStrings6 edge cases — fixed
- EmbeddedFonts6 format variant — fixed

---

## Research: DRC Rules and Violations

See `PCBDOC-next.md` for detailed documentation of the DRC data model:
- `TRuleKind` enum (70 variants)
- Rules6 section format and parameters
- Violation storage format (38 `T*Violation` section types)
- Proposed `PcbRuleKind`, `PcbNetScope`, `PcbRuleLayerKind`, `PcbViolationKind`
  types for `altium-format-types`
