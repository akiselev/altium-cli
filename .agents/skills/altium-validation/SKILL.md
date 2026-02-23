---
name: altium-validation
description: Cross-reference the altium-cli code (types, constants, records, parsing) against the reverse-engineered documentation in docs/dxp/, docs/schlib/, docs/schdoc/, docs/pcblib/, and the Altium C#/.NET source in ./AD26-dotnet/. Finds mismatches, missing implementations, wrong values, ambiguous specs, and documentation drift. Asks the user to resolve ambiguity rather than guessing.
argument-hint: "Scope or extra instructions"
---

# altium-cli Format Validation

Cross-reference the code against the documentation and Altium source to find mismatches.
This is NOT a code-quality review — it checks whether our implementation matches the
documented format specification and the original Altium source code.

When you find **ambiguity** (docs say one thing, C# says another, or neither is clear),
**ask the user** rather than guessing. Flag ambiguities separately from definite errors.

For each finding, report:

1. **Rule violated** (by name from the checklist)
2. **Code location** (file:line)
3. **Documentation or source reference** (doc file + section, or C# file + class/method)
4. **What's wrong** (concrete description of the mismatch)
5. **Suggested fix** (or "AMBIGUOUS — needs user decision" with the options)

At the end, give a summary: total findings grouped by category (MISMATCH / GAP / AMBIGUITY).

---

## Validation Checklist

### MISMATCH — Code contradicts documentation or Altium source

#### V1: Record Type Enum Completeness
The `SchRecordType` and `PcbObjectId` enums must cover all documented record types. Check:
- Compare `SchRecordType` variants in `altium-format-types/src/sch.rs` against the record table in `docs/dxp/schematic-records.md`
- Compare `PcbObjectId` variants in `altium-format-types/src/pcb.rs` against the object ID table in `docs/dxp/pcb-records.md`
- Compare both against the C# source in `AD26-dotnet/` (grep for `SchRecordType`, `TObjectId`, or the relevant enum)
- Every documented record type must have an enum variant, even if parsing isn't implemented yet

#### V2: Constant Values Match Source
Every constant in `altium-format-types/src/constants/` must match the authoritative source. Check:
- Parameter key strings against `FileFormatConsts.cs` in `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.FileFormats/`
- Bitmask values and shifts against the C# or Delphi source
- Stream name constants against `docs/dxp/container-format.md` and `docs/dxp/sch-files.md`
- Magic numbers (block size mask, instruction bytes, etc.) against `docs/dxp/container-format.md`
- **Method**: For each constants module, sample 5-10 constants and verify them against the C# source

#### V3: Parameter Keys Per Record
Each record struct's `#[param(key = "...")]` attributes must use the correct parameter keys. Check:
- Cross-reference each record struct's param keys against the corresponding section in `docs/dxp/schematic-records.md` or `docs/dxp/pcb-records.md`
- Cross-reference against the C# `FileFormatConsts.cs` constants for that record type
- Look for typos, case mismatches, or wrong keys (parameter keys are case-insensitive in Altium, but our constants should match the canonical casing from C#)

#### V4: Parameter Types Match Format
Each parameter must be deserialized into the correct type for how Altium encodes it. Check:
- Coordinate fields use `Coord` with both integer and `_FRAC` companion (per `docs/dxp/coordinates.md`)
- Color fields use `Color` type (Win32 COLORREF `0x00BBGGRR` per `docs/dxp/coordinates.md`)
- Boolean fields parsed from `"T"`/`"F"` strings, not integer 0/1 (unless the specific field uses integer encoding)
- Enum fields use the correct variant mapping (cross-reference with C# enum definitions)
- `RECORD` field dispatched as `SchRecordType` (and `RECORD >= 256` handled via `RECORD=254` + `RECORDEX`)

#### V5: Dispatch Coverage
The record dispatch function must have a case for every implemented record type. Check:
- `dispatch_record()` in `schlib.rs` (or equivalent) against the `SchRecordType` enum
- Every enum variant that has a corresponding struct definition must be dispatched
- Unimplemented variants should produce an explicit error, not be silently skipped
- Compare dispatch branches against the documented record table

#### V6: Stream Names and Layout
CFB stream names and document structure must match the documentation. Check:
- Stream name constants in `altium-format-types/src/constants/streams.rs` against both the cross-cutting docs (`docs/dxp/sch-files.md`, `docs/dxp/pcb-files.md`, `docs/dxp/container-format.md`) and the domain-specific docs:
  - SchLib: `docs/schlib/` describes the `/FileHeader`, `/Storage`, `/SectionKeys`, component `/Key/Data` + `/Key/Additional` layout
  - SchDoc: `docs/schdoc/` describes the SchDoc stream structure
  - PcbLib: `docs/pcblib/` describes the PcbLib stream structure
- Sidecar stream names and load order against `docs/dxp/sidecar-streams-deep-dive.md` AND the domain docs (domain docs may have more detail on specific sidecar handling)

#### V7: Sidecar Merge Behavior
Sidecar stream merging must follow the documented rules. Check:
- Pin sidecar stream list (PinFrac, PinDesc, PinMiscData, PinTextData, PinWideText, PinSymbolLineWidth, PinPackageLength, PinPropagationDelay, PinFunctionData) against `docs/dxp/sidecar-streams-deep-dive.md`
- Merge precedence rules (e.g., PinWideText is authoritative over PinDesc)
- Sidecar load ordering matches documentation
- WideStrings format: PcbLib uses parameter-block format, PcbDoc uses binary TLV — verify code handles the distinction

#### V8: Binary Layout Accuracy
For binary records (PCB), field offsets, sizes, and byte order must match. Check:
- Binary reader field order against the documented struct layout in `docs/dxp/pcb-records.md`
- Endianness (always little-endian per docs)
- Padding bytes accounted for
- Total record size matches documentation

### GAP — Code is missing something the documentation describes

#### V9: Missing Record Fields
Documented fields that aren't implemented in the corresponding struct. Check:
- For each implemented record struct, list its fields
- Compare against the documented parameters for that record type in `docs/dxp/schematic-records.md` or `docs/dxp/pcb-records.md`
- Compare against the C# class fields for that record type
- Any documented field not in the struct is a gap (unless there's a justified reason)

#### V10: Missing Constants
Constants documented or present in `FileFormatConsts.cs` that aren't defined in `altium-format-types/src/constants/`. Check:
- Systematically compare each constants module against its C# source section
- Look for TODO comments indicating known gaps
- Check that constant modules cover all the categories in `FileFormatConsts.cs`

#### V11: Undocumented Code
Code that implements format behavior not described in any documentation. This is a docs gap. Check:
- Record types or fields parsed in code but not documented in `docs/dxp/`
- Special-case handling (workarounds, quirks) without corresponding documentation
- Magic numbers or format quirks in code without explanation
- This is less critical than code gaps but should be flagged for documentation

### AMBIGUITY — Unclear or contradictory specification

#### V12: Documentation vs C# Source Conflict
When docs say one thing and the C# (or Delphi) source says another. Report:
- The documentation claim (with file + section reference)
- The C# source behavior (with file + class/method reference)
- Both possible interpretations
- **Ask the user which is correct** — do NOT guess

#### V13: Underdetermined Format Behavior
Format details that are genuinely unclear from available sources. Report:
- What is known about the field/behavior
- What is ambiguous
- What evidence exists for each possibility
- **Ask the user to investigate further** or make a decision

#### V14: Version-Dependent Behavior
Format features that differ across Altium Designer versions. Report:
- Which versions behave differently
- How the code currently handles it
- Whether the code should target a specific version or handle multiple

---

## How to Validate

### 1. Determine scope

If the user specifies a scope, use it. Otherwise, validate the most critical areas first:

**Priority order:**
1. Record type enums (V1) — quick, high-value check
2. Dispatch coverage (V5) — ensures all defined types are wired up
3. Constant values (V2, V10) — spot-check against C# source
4. Parameter keys per record (V3) — validates the core parsing layer
5. Stream names and layout (V6) — validates document structure
6. Sidecar merge behavior (V7) — validates the tricky sidecar layer
7. Missing fields (V9) — thorough but time-consuming
8. Binary layouts (V8) — relevant once PCB parsing is further along

Acceptable scope specifiers from the user:
- **"records"** — focus on V1, V3, V5, V9 (record types, fields, dispatch)
- **"constants"** — focus on V2, V10 (constant values and coverage)
- **"streams"** — focus on V6, V7 (CFB structure and sidecars)
- **"pcb"** or **"sch"** — focus on one domain
- **A specific record type** (e.g., "Pin", "RECORD=2") — deep-dive that one record
- **A specific file** — validate just that file
- **No argument** — run the priority order top-to-bottom, stopping after significant findings to report

### 2. Cross-reference methodology

For each check, reference **three sources** when available:

| Source | Location | Use for |
|--------|----------|---------|
| **Our code** | `crates/altium-format-types/src/`, `crates/altium-format/src/` | What we implement |
| **Format docs** | `docs/dxp/` (cross-cutting format reference) | Container format, record tables, coordinates, invariants, sidecar deep-dive |
| **Domain docs** | `docs/schlib/`, `docs/schdoc/`, `docs/pcblib/` | Per-format loading pipelines, stream layouts, merge rules, field-level details |
| **Altium source** | `AD26-dotnet/` (C# decompiled source), ghidra-cli for Delphi | Ground truth |

The domain docs (`docs/schlib/`, `docs/schdoc/`, `docs/pcblib/`) often contain more detailed and up-to-date information than the cross-cutting `docs/dxp/` files. Always check the domain-specific docs for the format you're validating:
- Validating SchLib code → check `docs/schlib/` first, then `docs/dxp/`
- Validating SchDoc code → check `docs/schdoc/` first, then `docs/dxp/`
- Validating PcbLib code → check `docs/pcblib/` first, then `docs/dxp/`

When all sources agree: no finding.
When code != docs (either format docs or domain docs): MISMATCH.
When code is missing something from docs or source: GAP.
When format docs != domain docs, or docs != Altium source: AMBIGUITY — ask the user.
When domain docs and format docs contradict each other: AMBIGUITY — the domain docs are likely more recent, but ask the user to confirm.

### 3. Efficient search patterns

| Check | How to search |
|-------|--------------|
| V1 (enums) | Read `sch.rs` and `pcb.rs` enum definitions; grep docs for "RECORD=" tables |
| V2 (constants) | Read constants modules; grep `FileFormatConsts.cs` for matching names |
| V3 (param keys) | Grep for `#[param(key =` in record structs; cross-ref with docs record sections |
| V4 (param types) | Read struct field types; cross-ref `_FRAC` usage, Color, bool encoding |
| V5 (dispatch) | Read dispatch match arms; compare against enum variants |
| V6 (streams) | Read stream constants; compare against docs CFB diagrams |
| V7 (sidecars) | Read sidecar merge functions; compare against `sidecar-streams-deep-dive.md` |
| V9 (fields) | Compare struct fields against docs parameter tables per record |
| V10 (missing consts) | Diff constants modules against `FileFormatConsts.cs` sections |

### 4. Report findings

Group by category (MISMATCH first, then GAP, then AMBIGUITY):

```
## Findings

### MISMATCH

**V2: Constant Values Match Source** — `altium-format-types/src/constants/pin.rs:15`
Constant `PIN_CONGLOMERATE_HIDE_DESIGNATOR_MASK` is `0x08` but `FileFormatConsts.cs`
(line 234) defines `SchPinDesignatorVisibleMask = 0x10`.
**Ref**: `AD26-dotnet/.../FileFormatConsts.cs:234`
**Fix**: Change to `0x10`.

### GAP

**V9: Missing Record Fields** — `altium-format/src/sch_records.rs` `SchPin` struct
The Pin record is documented to have `PINLENGTH_FRAC` (fractional companion for pin
length) in `docs/dxp/schematic-records.md` section "Pin (RECORD=2)", but the struct
has no `_FRAC` field for pin length.
**Ref**: `docs/dxp/schematic-records.md` → Pin section; `FileFormatConsts.cs:198`
**Fix**: Add `pin_length_frac` field with `#[param(coord, frac_key = "PINLENGTH_FRAC")]`.

### AMBIGUITY

**V12: Documentation vs C# Source Conflict** — Pin `PINCONGLOMERATE` bit 4
Our docs (`docs/dxp/schematic-records.md`) say bit 4 means "pin name visible", but
`FileFormatConsts.cs` calls this bit `SchPinDesignatorVisible`.
**Options**: (a) docs are wrong, bit 4 = designator visible; (b) C# naming is misleading
and it really is pin name. Need user to verify against actual Altium UI behavior.

## Summary

| Category   | Count |
|------------|-------|
| MISMATCH   | 1 |
| GAP        | 1 |
| AMBIGUITY  | 1 |
| **Total**  | **3** |
```

### 5. Handling ambiguity

When you encounter something ambiguous:
- Do NOT silently pick an interpretation
- Present both interpretations with evidence
- Use AskUserQuestion to get a decision if the ambiguity is blocking
- If it's non-blocking, list it in the AMBIGUITY section and move on

### 6. If no findings

Explicitly state: "No format validation issues found in the reviewed scope." and list what was checked against what sources.

=====

Extra instructions from user: $ARGUMENTS