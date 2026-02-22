---
name: data-review
description: >
    Audit the altium-cli codebase for data integrity issues: silent data corruption, dropped
    fields, overwritten values, raw byte passthrough, untyped parameter forwarding, and
    deserialization shortcuts. Activate when the user requests a data review, data integrity
    audit, or wants to verify that no data is silently lost or corrupted. Can target the full
    codebase, specific crates, or specific files.
---

# altium-cli Data Integrity Review

Audit Rust source files for data integrity violations — places where file data could be
silently lost, corrupted, overwritten, or passed through without proper deserialization.

For each violation found, report:

1. **Rule violated** (by name from the checklist)
2. **File and line** (e.g., `crates/altium-format/src/schlib.rs:445`)
3. **What's wrong** (what data is at risk and how)
4. **How to fix** (specific suggestion)

At the end, give a summary: total violations found, grouped by severity (CRITICAL / WARNING / INFO).

---

## Review Checklist

### CRITICAL — Data Loss or Corruption

#### D1: No Silent Data Dropping
Every piece of data in an Altium file must be deserialized or produce an error. Look for:
- Sidecar entries skipped when index is out of bounds (e.g., `if idx >= vec.len() { continue; }`)
- Match arms that silently discard variants: `_ => {}`, `_ => Ok(())`, `_ => continue`
- Loop iterations that `continue` past unrecognized data without erroring
- Records or fields read from the stream but never stored in a struct
- Data read into a variable that is never used (`let _data = reader.read_...()`)

#### D2: No Raw Byte Passthrough for Structured Data
Structured data must be deserialized into typed fields, not stored as opaque blobs. Look for:
- `Vec<u8>` struct fields that hold data which has known structure (records, parameters, binary structs)
- Byte slices copied with `.to_vec()`, `clone_from_slice()`, `copy_from_slice()` and stored without parsing
- Data decompressed (e.g., zlib) but then stored as raw bytes instead of being parsed into records
- **Exceptions**: Embedded images (`SchLibEmbeddedImage::data`) and other genuinely opaque binary payloads (e.g., OLE thumbnails, bitmap data) are acceptable as `Vec<u8>`

#### D3: No Raw Parameter String Passthrough
Parameter values must be deserialized into concrete types, not stored as raw strings. Look for:
- Struct fields of type `String` that hold parameter values which should be parsed into domain types (coordinates, colors, booleans, enums, record types)
- `ParameterCollection` or `IndexMap<String, String>` stored in a struct field instead of being fully consumed
- Functions that return raw parameter strings to callers instead of parsed values
- Parameter values forwarded between functions as `&str` or `String` without type conversion
- **Exception**: Parameter values that genuinely ARE free-text strings in Altium (e.g., component descriptions, comments, designators) are fine as `String`

#### D4: No Deserialization Shortcuts via Default Values
Default values must not silently substitute for missing required data. Look for:
- `.unwrap_or(0)`, `.unwrap_or_default()`, `.unwrap_or(false)` on data that should be required
- `Default::default()` used to fill struct fields that should come from the file
- `Option::None` used as a sentinel for "we didn't parse this" rather than "the file didn't have this"
- `#[param(default = ...)]` on fields that are always present in valid Altium files (defaults should only be used for truly optional fields)
- **Distinguishing required vs optional**: Check the Altium C#/.NET source or `FileFormatConsts.cs` to verify whether a field is genuinely optional or always present

#### D5: No Data Overwriting Without Detection
Data must not be silently overwritten. Look for:
- HashMap/IndexMap insertions that overwrite existing keys without checking (use `entry()` API or check for duplicates)
- Sidecar merge functions that overwrite fields without verifying the old value was a default/unset
- Multiple assignment to the same struct field from different sources without clear precedence rules
- Parameter keys parsed twice with different results (first-occurrence-wins rule violated)

### WARNING — Incomplete Deserialization

#### D6: Exhaustion Checks Must Be Called
Every data source must be fully consumed. Look for:
- `ParameterCollection` created but `assert_exhausted()` not called after field extraction
- `BinaryReader` used but `assert_exhausted()` not called at end of parsing
- `TrackedCfbDocument` used but `assert_all_consumed()` not called before returning
- Block streams parsed but trailing blocks not checked
- Any code path that can return early (via `?` or explicit `return`) before exhaustion checks run

#### D7: No Partial Record Deserialization
Every record type must deserialize ALL of its fields. Look for:
- `FromParams` implementations or `#[derive(FromParams)]` structs that are missing fields known to exist in the Altium format (cross-reference with `FileFormatConsts.cs`)
- Binary record readers that don't call `assert_exhausted()` — trailing bytes indicate missed fields
- Struct definitions with fewer fields than the corresponding Altium C# class
- TODO comments indicating fields that haven't been implemented yet

#### D8: No Lossy Type Conversions
Numeric and enum conversions must be checked. Look for:
- `as i32`, `as u8`, `as u16` truncating casts without range validation
- `.try_into().unwrap()` — should use `.try_into()?` or `.try_into().map_err(...)?`
- Enum conversions from integers without exhaustive matching (what happens for unknown values?)
- Floating-point to integer conversions without rounding strategy documentation
- Coordinate conversions that lose precision (DXP fractional encoding is split across two parameters — both parts must be used)

#### D9: Block Format Must Be Preserved
The block framing layer (text vs binary mode) must be respected. Look for:
- Text-mode blocks (`flag 0x00`) being parsed as binary or vice versa
- Block boundaries ignored (reading across block boundaries as if it were a continuous stream)
- Block count or size mismatches between header declaration and actual data

### INFO — Style and Robustness

#### D10: Sidecar Merge Ordering
Sidecar data must be merged in the correct order and into the correct records. Look for:
- Sidecar streams processed before the primary record stream (primary must be parsed first)
- Sidecar data merged into wrong record index (off-by-one errors in index mapping)
- Sidecar merge functions that don't validate the target record type matches expectations

#### D11: Encoding Boundary Discipline
Encoding conversions must happen exactly at parse/serialize boundaries. Look for:
- Encoding conversion in the middle of business logic (should only happen in readers/writers)
- Windows-1252 bytes stored in `String` without decoding
- UTF-16LE decoded without checking `had_errors` flag
- `from_utf8_lossy()` anywhere (always a violation — use strict `from_utf8()`)
- Mixed encoding in a single data path (e.g., half-decoded parameters)

---

## How to Review

### 1. Determine scope

If the user specifies a scope, use it. Otherwise, audit all Rust source files in the parsing crates:

```
crates/altium-format/src/**/*.rs         (primary target — all parsing lives here)
crates/altium-format-derive/src/**/*.rs  (codegen — verify generated patterns are safe)
crates/altium-format-types/src/**/*.rs   (type definitions — verify completeness)
crates/altium-format-ops/src/**/*.rs     (operations — verify they don't bypass parsing)
```

### 2. Scan systematically

Use Grep to find candidate violations, then Read to verify context. Key search patterns:

| Rule | Grep patterns |
|------|--------------|
| D1 | `continue`, `_ => \{\}`, `_ => Ok\(\)`, `let _ =`, `let _[a-z]` in non-test `.rs` files |
| D2 | `Vec<u8>` in struct definitions, `.to_vec()`, `clone_from_slice`, `copy_from_slice` |
| D3 | `String` fields in record structs, `IndexMap<String`, `ParameterCollection` as struct field |
| D4 | `.unwrap_or(`, `.unwrap_or_default()`, `Default::default()`, `default =` in `#[param]` attrs |
| D5 | `.insert(` on maps without `entry()`, multiple assignments to same field |
| D6 | `ParameterCollection::from` without nearby `assert_exhausted`, `BinaryReader::new` without nearby `assert_exhausted`, `TrackedCfbDocument` without `assert_all_consumed` |
| D7 | `TODO`, `FIXME`, `unimplemented`, `todo!()` near record parsing; compare struct field count to C# source |
| D8 | `as i32`, `as u8`, `as u16`, `as u32`, `.try_into().unwrap()` |
| D9 | Block flag checks, `INSTRUCTION_TEXT`, `INSTRUCTION_BINARY` usage |
| D10 | Sidecar merge function ordering in `open()` / loading pipelines |
| D11 | `from_utf8_lossy`, `WINDOWS_1252`, `UTF_16LE`, `had_errors` |

### 3. Reduce false positives with context

- `Vec<u8>` for genuinely opaque data (images, thumbnails) is acceptable — check if the data has known structure
- `String` for free-text fields (descriptions, comments, designators) is correct — check if the value should be a typed enum or coordinate instead
- `.unwrap_or(0)` for `_FRAC` fractional companion parameters is acceptable (absent frac means zero fraction)
- `continue` in loops may be legitimate control flow — check if data is being skipped
- `as` casts on values already known to be in range (e.g., after a bounds check) are fine
- `Default::default()` in tests and test fixtures is fine
- `#[param(default = ...)]` on fields documented as optional in Altium's format spec is fine

### 4. Cross-reference with Altium source

For D7 (partial deserialization), compare against the authoritative sources:
- `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.FileFormats/FileFormatConsts.cs` — lists all known parameter keys per record type
- `altium-format-types/src/constants/` — our mirrored constants (should match C# source)
- Delphi decompilations via ghidra-cli — for binary format structures

### 5. Report findings

Order: CRITICAL first, then WARNING, then INFO. Use this format:

```
## Findings

### CRITICAL

**D1: No Silent Data Dropping** — `crates/altium-format/src/schlib.rs:445`
Pin sidecar entry silently skipped when `pin_idx >= pins.len()`. If a sidecar references
a pin index that doesn't exist, this is a format error that should be reported, not ignored.
The sidecar data is lost.
**Fix**: Return `Err(AltiumFormatError::...)` instead of `continue`.

**D3: No Raw Parameter String Passthrough** — `crates/altium-format/src/foo.rs:78`
Field `location: String` stores a coordinate value as a raw string. This should be parsed
into a `Coord` type at deserialization time.
**Fix**: Change field type to `Coord` and parse via `remove_coord()`.

### WARNING

**D6: Exhaustion Checks Must Be Called** — `crates/altium-format/src/bar.rs:200`
`ParameterCollection` created at line 195 but `assert_exhausted()` never called. If the
file contains parameters we don't recognize, they'll be silently ignored.
**Fix**: Add `params.assert_exhausted()?;` after the last `remove_*()` call.

### INFO

(none)

## Summary

| Severity | Count |
|----------|-------|
| CRITICAL | 2 |
| WARNING  | 1 |
| INFO     | 0 |
| **Total** | **3** |
```

### 6. If no violations found

Explicitly state: "No data integrity violations found in the reviewed scope." and list which files were checked.