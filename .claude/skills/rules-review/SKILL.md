---
name: rules-review
description: >
    Review code changes against the altium-cli project rules from CLAUDE.md.
    Activate when the user requests a code review, rules check, or compliance audit.
    Checks for: fail-fast violations, raw type usage, privacy leaks across crate boundaries,
    silent error suppression, unconsumed data skipping, missing domain types, and error handling correctness.
---

# altium-cli Rules Review

Review the code changes (staged, unstaged, or a specific commit/PR) against the project rules below. For each violation found, report:

1. **Rule violated** (by name from the checklist)
2. **File and line**
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
Never use raw types where domain types exist in `altium-format-types`. Look for:
- `String` instead of the appropriate Altium string type (Altium uses Windows-1252, UTF-8, and UTF-16)
- `u8` where `PcbObjectId` should be used
- `i32` where `SchRecordType`, `Coord`, or other typed values should be used
- Raw integer literals for constants (e.g., `0xD0` instead of `INSTRUCTION_BINARY`, `0x00FF_FFFF` instead of `BLOCK_SIZE_MASK`)
- Any new struct field using a primitive type when a domain type exists or should be created

#### R3: No Silent Error Dropping
Everything fallible MUST return `Result<T, AltiumFormatError>` (in `altium-format`) or `Result<T, AltiumOpsError>` (in `altium-format-ops`). Look for:
- `.unwrap()` in non-test code
- `.expect()` in library code (acceptable in tests and CLI)
- Swallowing errors with `if let Ok(x) = ...` without handling the error case
- Using `Option` where `Result` is appropriate for fallible operations
- Missing `?` propagation where errors should bubble up

#### R4: Crate Privacy — No Implementation Leaks
`altium-format` implementation details must stay private to the crate. Look for:
- `pub` visibility on internal parsing structs/functions that shouldn't be exposed
- `altium-format-ops` or `altium-cli` directly accessing `altium-format` internals
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
- `altium-format-ops` → `AltiumOpsError`
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
altium-format-types → altium-format-derive → altium-format → altium-format-ops → altium-cli
```
Look for reverse dependencies or circular imports.

#### R10: Result Over Panic
Library code (everything except `altium-cli`) should return `Result` instead of panicking. Look for:
- `panic!()`, `unreachable!()` (when the case IS reachable), `todo!()` in non-test code
- `assert!()` in production code paths (acceptable in tests)

---

## How to Review

1. **Determine scope**: Ask the user what to review (staged changes, a commit, a PR, or specific files). If not specified, review unstaged + staged changes (`git diff HEAD`).
2. **Read the diff**: Use `git diff` or `git show` to get the actual changes.
3. **For each changed file**: Check every hunk against ALL rules above (R1-R10).
4. **Check cross-crate concerns**: Verify new `pub` items don't leak internals (R4), error types match the crate (R6), and dependency direction is correct (R9).
5. **Report findings** in the format specified above, ordered by severity (CRITICAL first).
6. **If no violations found**: Explicitly state the code passes all checks.