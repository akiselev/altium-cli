# Rules Review Report

**Date:** 2026-03-01
**Scope:** Full workspace (`crates/altium-format/`, `crates/altium-format-derive/`, `crates/altium-format-types/`, `crates/altium-cli/`)

---

## Findings

### CRITICAL

#### R3/R10: `.expect()` / `.unwrap()` in library production code

**R3: `.expect()` in `SchLib::new_blank_ad26()`** — `crates/altium-format/src/schlib.rs:1748`
```rust
let mut component = parse_component_record(&mut params)
    .expect("internal default component parse should not fail");
```
Panics on parse failure instead of returning `Result`. Constructor should propagate errors.
**Fix**: Change return type to `Result<Self>` and use `?` instead of `.expect()`.

---

**R3: `.expect()` in `PcbLib::new_blank_ad26()`** — `crates/altium-format/src/pcblib/mod.rs:981`
```rust
let board_config = crate::board_config::parse_board_config(
    &mut crate::param_collection::ParameterCollection::new(),
)
.expect("empty ParameterCollection should produce valid board config defaults");
```
Panics on parse failure instead of returning `Result`.
**Fix**: Change return type to `Result<Self>` and use `?` instead of `.expect()`.

---

**R3: `.expect()` in `Coord::from_mils()`** — `crates/altium-format-types/src/coord.rs:39`
```rust
pub fn from_mils(mils: i32) -> Self {
    Self(mils.checked_mul(10_000).expect("Coord::from_mils overflow"))
}
```
Panics on integer overflow. `from_mils_f64` silently truncates via `as i32` cast.
**Fix**: Return `Option<Self>` or `Result<Self, CoordOverflow>` and let callers handle overflow.

---

**R10: `assert!()` in `write_wide_string_fixed()`** — `crates/altium-format/src/binary_io.rs:458`
```rust
assert!(
    chars.len() < char_count,
    "Wide string too long: {} chars (max {})",
    chars.len(),
    char_count - 1
);
```
Panics on oversized string input in production serialization code.
**Fix**: Return `Result<()>` with an `AltiumFormatError` instead of panicking.

---

**R10: `unreachable!()` for reachable code** — `crates/altium-format/src/sch_records.rs:2917`
```rust
SchRecord::Sheet(_) => {
    unreachable!("SchSheet serialization is not implemented yet")
}
```
`Sheet` is a legitimate `SchRecord` variant. This is not unreachable — it's unimplemented. Using `unreachable!()` hides the real issue.
**Fix**: Use `todo!("SchSheet serialization")` or return a proper error like `AltiumFormatError::UnsupportedFeature`.

---

**R10: `unreachable!()` at end of finite loop** — `crates/altium-format/src/schlib.rs:1740`
```rust
for suffix in 1..=u32::MAX {
    // ...
    if !used.contains(&candidate) {
        return candidate;
    }
}
unreachable!()
```
While practically unreachable (2^32 attempts), this could be replaced with a proper error for safety.
**Fix**: Return `Result<String>` and return an error if the loop exhausts.

---

**R3: `.unwrap()` in production code** — `crates/altium-format/src/pcbdoc/records.rs:538-540`
```rust
let index = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
let byte_len =
    u32::from_le_bytes(data[offset + 4..offset + 8].try_into().unwrap()) as usize;
```
While logically safe (bounds are checked above at line 527), `.unwrap()` in library code is poor form.
**Fix**: Use `.expect("bounds checked above")` at minimum, or propagate errors properly.

---

**R3: `.unwrap()` in production code** — `crates/altium-format/src/pcbdoc/mod.rs:437`
```rust
if records.len() != 1 {
    return Err(AltiumFormatError::RecordCountMismatch { ... });
}
let mut record = records.into_iter().next().unwrap();
```
Logically safe (length checked == 1) but poor form.
**Fix**: Use `.expect("length checked above")` or destructure with `if let`.

---

**R3: `.unwrap()` in production code** — `crates/altium-format/src/param_collection.rs:324,336`
```rust
let value = self.params.shift_remove(&actual_key).unwrap();
```
Logically safe (key was just found by `find_key`), but still uses `.unwrap()` in library code.
**Fix**: Use `.expect("key found by find_key above")` at minimum.

---

### WARNING

#### R10: `unreachable!()` on `#[non_exhaustive]` enum arms

**R10: `unreachable!()` in macro-generated code** — `crates/altium-format/src/param_value.rs:229`
```rust
_ => unreachable!("unknown {} variant", stringify!($t)),
```
The `impl_string_enum_param_value!` macro generates a wildcard arm for `#[non_exhaustive]` enums. If a new variant is added to the enum but not to the macro invocation, this will panic at runtime instead of returning an error.
**Fix**: Return an error string or `Err` instead of panicking.

---

**R10: `unreachable!()` on `#[non_exhaustive]` enums** — `crates/altium-format/src/param_value.rs:411`
```rust
impl ToParamValue for CornerStyle {
    fn to_param_value(&self) -> String {
        match self {
            Self::Degree90 => "90-Degree",
            Self::Degree45 => "45-Degree",
            Self::Round => "Rounded",
            _ => unreachable!("unknown CornerStyle variant"),
        }.to_owned()
    }
}
```
Same issue — `#[non_exhaustive]` enum with `unreachable!()` wildcard.
**Fix**: Return a fallback or error instead of panicking.

---

**R10: `unreachable!()` on `#[non_exhaustive]` enum** — `crates/altium-format/src/pcblib/sidecar.rs:432`
```rust
fn mask_expansion_mode_to_str(mode: MaskExpansionMode) -> &'static str {
    match mode {
        MaskExpansionMode::NoMask => "None",
        MaskExpansionMode::Rule => "Rule",
        MaskExpansionMode::Manual => "Manual",
        _ => unreachable!("unknown MaskExpansionMode variant"),
    }
}
```
Same pattern — `#[non_exhaustive]` enum with `unreachable!()` wildcard.
**Fix**: Return a fallback or error.

---

#### R8: Hard-coded constants that should use named constants

**R8: Hard-coded `254` instead of `C_MAX_SHORT_STRING_LENGTH`** — multiple files
- `crates/altium-format/src/sch_records.rs:2497` — `encoded.len() > 254`
- `crates/altium-format/src/schlib.rs:928` — `pin.description.len() > 254`
- `crates/altium-format/src/schlib.rs:929` — `&pin.description[254..]`
- `crates/altium-format/src/schlib.rs:1013` — `value.len() > 254`

**Fix**: Import and use `C_MAX_SHORT_STRING_LENGTH` from `altium_format_types::constants::parsing`. Note: the constant is `i32`; cast or create a `usize` companion.

---

**R8: Hard-coded `254` instead of `INSTRUCTION_EXTRA_OBJECT_INDEX`** — record overflow dispatch
- `crates/altium-format/src/schdoc/fileheader.rs:74` — `if record_raw == 254`
- `crates/altium-format/src/schdoc/fileheader.rs:106` — `if record_raw == 254`
- `crates/altium-format/src/schdoc/mod.rs:819` — `if record_raw == 254`
- `crates/altium-format/src/schlib.rs:366` — `if record_raw == 254`

**Fix**: Import and use `INSTRUCTION_EXTRA_OBJECT_INDEX` (value `0xFE = 254`) from `altium_format_types::constants::parsing`.

---

**R8: Hard-coded `"RECORD"` instead of `RECORD` constant** — production code
- `crates/altium-format/src/board_config.rs:265` — `remove_optional::<String>("RECORD")`
- `crates/altium-format/src/board_config.rs:1141` — `params.insert("RECORD", ...)`
- `crates/altium-format/src/api/schdoc_read.rs:32,45,359` — error context keys
- `crates/altium-format/src/api/sch_common.rs:177` — error context key

**Fix**: Import `RECORD` from `altium_format_types::constants::record_structure` and use it instead of the string literal.

---

**R8: Hard-coded `0x8E` and `0x7E`** — `crates/altium-format/src/schlib.rs:1013`
```rust
value.len() > 254 || value.chars().any(|c| c as u32 > 0x7E && c as u32 != 0x8E)
```
`0x8E` should use `C_SCH_SPECIAL_DELIMITER` from `altium_format_types::constants::parsing`. A named constant for `0x7E` (max ASCII printable) may also be appropriate.
**Fix**: Import and use `C_SCH_SPECIAL_DELIMITER`. Consider adding a `C_MAX_ANSI_PRINTABLE` constant for `0x7E`.

---

## Rules That Passed

| Rule | Status | Notes |
|------|--------|-------|
| **R1: Fail Fast** | PASS | No silent skipping found. `assert_exhausted()`, `assert_all_consumed()` properly enforced everywhere. |
| **R2: No Raw Primitive Types** | PASS* | Some `u8` fields in PCB structs (e.g., `track_kind`, `mode`, `flags`) may warrant domain types, but these are ambiguous — could be generic counts/flags rather than Altium domain concepts. No clear violations. |
| **R2b: Strict Encoding** | PASS | No `from_utf8_lossy` anywhere. All `UTF_16LE.decode()` calls check `had_errors`. Windows-1252 decode error flags are correctly ignored (all 256 values valid). |
| **R4: Crate Privacy** | PASS | Internal modules use `mod` (not `pub mod`). Only public API types are exported. |
| **R5: No Unconsumed Data** | PASS | No `mark_consumed` without parsing, no "parse later" suppressions. |
| **R6: Error Type Correctness** | PASS | `anyhow` only in `altium-cli`. `AltiumFormatError` only in `altium-format`. |
| **R7: Domain Types Location** | PASS | Domain types appropriately placed in `altium-format-types`. |
| **R9: Dependency Direction** | PASS | `types → derive → format → spec → cli` verified in all `Cargo.toml` files. |

*Note on R2: Fields like `track_kind: u8`, `advance_mode: u8`, `thermal_relief_rotation_code: u8` in PCB structs could benefit from domain enums, but this requires Altium source investigation to determine the correct type. Flagged for future work, not a clear violation.

---

## Summary

| Severity | Count |
|----------|-------|
| CRITICAL | 9 |
| WARNING  | 6 |
| **Total** | **15** |

### Priority Recommendations

1. **High priority**: Convert `new_blank_ad26()` constructors to return `Result` (schlib.rs:1748, pcblib/mod.rs:981)
2. **High priority**: Replace `assert!()` with `Result` in `write_wide_string_fixed()` (binary_io.rs:458)
3. **High priority**: Replace `unreachable!()` with proper error on `SchRecord::Sheet` (sch_records.rs:2917)
4. **Medium priority**: Replace hard-coded `254` with `C_MAX_SHORT_STRING_LENGTH` / `INSTRUCTION_EXTRA_OBJECT_INDEX` (8 locations)
5. **Medium priority**: Replace hard-coded `"RECORD"` with `RECORD` constant (6 locations)
6. **Low priority**: Replace `unreachable!()` on `#[non_exhaustive]` enum wildcards with error returns (3 locations)
7. **Low priority**: Replace `.unwrap()` with `.expect()` where logically safe (3 locations)
