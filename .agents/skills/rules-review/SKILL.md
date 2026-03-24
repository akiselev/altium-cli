---
name: rules-review
description: >
    Audit the altium-cli codebase (or a specified scope) against project rules from CLAUDE.md.
    Activate when the user requests a rules review, rules check, compliance audit, or codebase audit.
    Checks for: fail-fast violations, raw type usage, privacy leaks across crate boundaries,
    silent error suppression, unconsumed data skipping, missing domain types, and error handling correctness.
    Can target the full codebase, specific crates, files, or git changes.
---

# altium-cli Rules Review

Audit Rust source files against the project rules below. For each violation found, report:

1. **Rule violated** (by name from the checklist)
2. **File and line** (e.g., `crates/altium-format/src/schlib.rs:123`)
3. **What's wrong** (concrete description)
4. **How to fix** (specific suggestion)

At the end, give a summary: total violations found, grouped by severity (CRITICAL / WARNING).

---

## Review Checklist

### CRITICAL Rules (must never be violated)

#### R1: Fail Fast — No Silent Skipping
The parser must never silently skip data it doesn't understand. If a stream, record, or field exists in the file, the parser MUST either fully parse it or return an error. Look for:
- Any form of `skip_known`, `ignore_remaining`, or `_ => {}` catch-all match arms that discard data
- Marking entries as consumed in `TrackedCfbDocument` without actually reading and parsing their contents
- Deferring parsing to "future milestones" by suppressing errors
- Circumventing `assert_all_consumed()` in any way
- `.ok()`, `.unwrap_or_default()`, or `let _ =` on parse results that silently drop errors

#### R2: No Raw Primitive Types
Never use raw primitive integers where domain types exist in `altium-format-types`. Look for:
- `u8` where `PcbObjectId` should be used
- `i32` where `SchRecordType`, `Coord`, or other typed values should be used
- Raw integer literals for constants (e.g., `0xD0` instead of `INSTRUCTION_BINARY`, `0x00FF_FFFF` instead of `BLOCK_SIZE_MASK`)
- Any struct field using a primitive integer type when a domain type exists or should be created
- Note: Rust `String` is correct for decoded text — Altium's own C# code uses plain .NET `string`. The encoding is a property of the serialization context, not the type.

#### R2b: Strict Encoding at Parse Boundaries
All byte-to-string decoding MUST use strict (non-lossy) functions. Look for:
- `from_utf8_lossy()` — ALWAYS a violation; use `from_utf8()` with error propagation or Windows-1252 decode
- Discarded `had_errors` flag on `encoding_rs::UTF_16LE.decode()` calls — MUST be checked
- Note: `encoding_rs::WINDOWS_1252.decode()` maps all 256 byte values and cannot error, so discarding its flags is safe

#### R3: No Silent Error Dropping
Everything fallible MUST return `Result<T, AltiumFormatError>` (in `altium-format`) or `Result<T, SpecError>` (in `autopcb-spec`). Look for:
- `.unwrap()` in non-test code
- `.expect()` in library code (acceptable in tests and CLI)
- Swallowing errors with `if let Ok(x) = ...` without handling the error case
- Using `Option` where `Result` is appropriate for fallible operations
- Missing `?` propagation where errors should bubble up

#### R4: Crate Privacy — No Implementation Leaks
`altium-format` implementation details must stay private to the crate. Look for:
- `pub` visibility on internal parsing structs/functions that shouldn't be exposed
- `autopcb-spec` or `altium-cli` directly accessing `altium-format` internals
- Types or functions that should be `pub(crate)` but are `pub`

#### R5: No Unconsumed Data Suppression
Do NOT mark streams, records, or fields as "consumed" without actually parsing them. Look for:
- Calls to consume/mark-consumed methods without preceding parse logic
- Empty implementations that just mark things as handled
- TODOs or comments like "parse later" with suppressed errors

### WARNING Rules (should be followed, deviations need justification)

#### R6: Error Type Correctness
Each crate uses its own error type:
- `altium-format` → `AltiumFormatError`
- `autopcb-spec` → `SpecError`
- `altium-cli` → `anyhow`
Look for error types used in the wrong crate.

#### R7: Domain Types in altium-format-types
New domain types belong in `altium-format-types`, not inline in other crates. Look for:
- New enums or type aliases defined in `altium-format` that represent Altium concepts
- Constants defined outside of `altium-format-types/src/constants/`

#### R8: Constants from FileFormatConsts
All file format constants should come from `altium_format_types::constants::*` (mirroring `FileFormatConsts.cs`). Look for:
- Hard-coded magic numbers that correspond to known Altium constants
- String literals for stream names, record type names, or parameter keys that should be constants

#### R9: Dependency Direction
The dependency graph must be strictly:
```
altium-format-types → altium-format-derive → altium-format → autopcb-spec → altium-cli
```
Look for reverse dependencies or circular imports.

#### R10: Result Over Panic
Library code (everything except `altium-cli`) should return `Result` instead of panicking. Look for:
- `panic!()`, `unreachable!()` (when the case IS reachable), `todo!()` in non-test code
- `assert!()` in production code paths (acceptable in tests)

---

## How to Review

### 1. Determine scope

If the user specifies a scope, use it. Otherwise, audit all Rust source files across the workspace:

```
crates/altium-format/src/**/*.rs
crates/altium-format-derive/src/**/*.rs
crates/altium-format-types/src/**/*.rs
crates/autopcb-spec/src/**/*.rs
crates/altium-cli/src/**/*.rs
```

Acceptable scope specifiers from the user:
- **A crate name**: e.g., "altium-format" → audit all `.rs` files in that crate
- **A file or glob**: e.g., "schlib.rs" or "src/param_*.rs"
- **"changes"** or **"diff"**: audit only git changes (`git diff HEAD`)
- **A commit or PR**: audit that specific changeset
- **No argument**: audit the entire workspace

### 2. Scan systematically

For each file in scope, use Grep and Read to check for violations. Efficient search patterns:

| Rule | Grep patterns to try |
|------|---------------------|
| R1 | `_ => \{\}`, `_ => Ok\(\)`, `skip`, `ignore_remaining`, `.ok()` on parse results |
| R2 | Struct fields with `: u8`, `: i32`, `: u32`, `: i64`, `: u64`; hex literals `0x[0-9a-fA-F]+` |
| R2b | `from_utf8_lossy`, `decode_without_bom_handling` with discarded `had_errors` on UTF-16 |
| R3 | `.unwrap()`, `.expect(`, `if let Ok(`, `let _ =` |
| R4 | `pub fn` and `pub struct` in altium-format (check if they should be `pub(crate)`) |
| R5 | `mark_consumed`, `set_consumed`, `consumed = true` without adjacent parsing |
| R6 | `anyhow` in library crates, `AltiumFormatError` in spec/ops crates |
| R7 | `enum` definitions in altium-format that look like Altium domain concepts |
| R8 | String literals that look like stream/record names, numeric magic constants |
| R9 | Check `Cargo.toml` files for dependency direction violations |
| R10 | `panic!`, `unreachable!`, `todo!`, `assert!` in non-test code |

### 3. Use context to reduce false positives

- `.unwrap()` and `.expect()` in `#[cfg(test)]` modules or test files are fine
- `assert!` in test code is fine
- `pub` on items that are part of the crate's intended public API is fine — focus on items that look like internal parsing machinery
- Some `_ =>` arms legitimately handle a finite set — check if data is being discarded
- Raw types in proc macro code (`altium-format-derive`) may be acceptable since it generates code

### 4. Report findings

Order by severity (CRITICAL first, then WARNING), then by file path. Use this format:

```
## Findings

### CRITICAL

**R2: No Raw Primitive Types** — `crates/altium-format/src/schlib.rs:45`
Field `component_count: u32` uses raw primitive. Should use a domain type or at minimum document why a raw type is necessary here.
**Fix**: Define an appropriate type in `altium-format-types` or use an existing one.

### WARNING

**R8: Constants from FileFormatConsts** — `crates/altium-format/src/pcblib.rs:112`
Hard-coded string `"Board6"` should use a constant from `altium_format_types::constants`.
**Fix**: Add `pub const BOARD6: &str = "Board6";` to the appropriate constants module.

## Summary

| Severity | Count |
|----------|-------|
| CRITICAL | 1 |
| WARNING  | 1 |
| **Total** | **2** |
```

### 5. If no violations found

Explicitly state: "No rule violations found in the reviewed scope." and list which files were checked.