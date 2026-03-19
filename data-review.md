# Data Integrity Review — altium-cli

**Date**: 2026-03-01
**Scope**: All non-test Rust source files in `crates/altium-format/src/`, `crates/altium-format-types/src/`, `crates/altium-format-derive/src/`

---

## Findings

### CRITICAL

**D6: Exhaustion Checks Must Be Called** — `crates/altium-format/src/shared_union.rs:93-102`

Hidden SharedUnion primitives are stored as raw `ParameterCollection` objects without calling
`assert_exhausted()`. The code has an explicit comment acknowledging the skip:

```rust
let child_params = ParameterCollection::from_bytes(&child_prefixed)?;
// We do NOT call assert_exhausted on hidden primitives — they are full
// primitive descriptions with many keys that we store as-is.
hidden.push(child_params);
```

These hidden primitives are full schematic/PCB primitive descriptions with known field structure.
Storing them as opaque `ParameterCollection` objects violates both D6 (no exhaustion check) and
the cardinal rule against retaining opaque format data. Unknown or changed parameters will be
silently preserved.

**Fix**: Define typed structs for hidden primitives based on their `RECORD`/object type and
fully deserialize them with `assert_exhausted()`, or dispatch through the same parsing pipeline
used for regular primitives.

---

**D3: No Raw Parameter String Passthrough** — `crates/altium-format/src/board_config.rs:33`

The `cfg3d` field stores all `CFG3D.*` parameters as an untyped `IndexMap<String, String>`:

```rust
pub(crate) cfg3d: IndexMap<String, String>,
```

CFG3D parameters represent typed 3D board configuration (camera positions, render settings, etc.)
with known keys from Altium's format. Storing them as raw string pairs bypasses type validation
and allows unknown parameters to pass through silently.

**Fix**: Reverse-engineer the CFG3D parameter keys from the C# source, define a typed
`PcbCfg3D` struct, and fully deserialize the parameters with `assert_exhausted()`.

---

**D8: No Lossy Type Conversions** — `crates/altium-format/src/binary_io.rs:204`

Unchecked `i32` to `usize` cast for string block length:

```rust
pub(crate) fn read_string_block(&mut self) -> Result<String> {
    let len = self.read_i32_le()? as usize;
    let bytes = self.read_bytes(len)?;
    ...
}
```

If the file contains a negative i32 length value, the `as usize` cast wraps it to a very large
positive value (~2^63 on 64-bit). While `read_bytes()` will fail with a bounds error, the error
message will be misleading ("needed 18446744073709551614 bytes" instead of "negative length -2").

**Fix**: Add a range check: `let len: usize = len_i32.try_into().map_err(|_| AltiumFormatError::InvalidParamValue { ... })?;`

---

**D8: No Lossy Type Conversions** — `crates/altium-format/src/schlib.rs:495,505,575`

Same pattern — three instances of unchecked `i32 as usize` for sidecar payload lengths:

```rust
// Line 495 (read_sidecar_utf16le_params)
let payload_len = r.read_i32_le()? as usize;

// Line 505 (read_sidecar_ascii_params)
let payload_len = r.read_i32_le()? as usize;

// Line 575 (merge_pin_desc)
let byte_len = r.read_i32_le()? as usize;
```

All three read an i32 length from a sidecar stream and cast directly to usize without
validating non-negative. Same risk as the binary_io.rs finding.

**Fix**: Use `.try_into().map_err(...)` with context identifying the sidecar stream.

---

### WARNING

**D4: No Deserialization Shortcuts via Default Values** — 281 instances across 20 files (INVESTIGATED)

Systematic use of `.unwrap_or_default()`, `.unwrap_or(0)`, `.unwrap_or(false)`, and
`.unwrap_or(-1)` on `remove_optional()` results for struct fields typed as non-`Option`.

**Deep investigation result: ~278 of 281 instances are LEGITIMATE.**

Each file cluster was cross-referenced against the Altium C# source and format documentation.
The vast majority represent genuinely optional parameters due to Altium's format version
evolution (V7→V8→V9 layer stacks), subtype-specific fields (BoardRegion vs normal Region),
or UI preference settings.

#### Confirmed LEGITIMATE categories (not violations):

| Category | Files | Count | Rationale |
|----------|-------|-------|-----------|
| Version-probed layer stacks | `board_config.rs` | ~140 | V9/V8/V7 params inside existence-probe blocks (e.g., only parsed if `V9_MASTERSTACK_STYLE` present) |
| UI preferences (grids, snapping) | `board_config.rs` | ~20 | Grid sizes, snap flags, display modes — user prefs, truly optional |
| 3D model/body metadata | `component_body.rs` | 21 | MODEL.*, BODY* — only present for primitives with 3D models |
| Subtype-specific fields | `region.rs` | 17 | OBJECTKIND, BENDINGLINECOUNT, LOCKED3D etc. — only present for BoardRegion type |
| Library/footprint metadata | `library.rs`, `footprint.rs` | 18 | FILENAME, DESCRIPTION, GUIDs — absent in older files |
| Optional SCH pin fields | `sch_records.rs` | 6 | Explicitly marked "Optional SchDoc pin fields" in code |
| Display settings (Option→concrete) | `schdoc_read.rs` | 16 | Converting `Option<T>` internal fields to concrete API types |
| Sidecar extensions | `sidecar.rs` | 3 | Sidecar fields by definition enhance but don't require core data |
| Serialization (None→wire) | `pcblib/mod.rs`, `pcbdoc/mod.rs` | 9 | Writing Option fields as default wire values — always correct |
| File extension checks | `schlib.rs`, `schdoc/mod.rs` | 3 | `.extension().map(...).unwrap_or(false)` — file system, not format data |
| Fractional companions | `param_collection.rs`, `schlib.rs` | 4 | `_FRAC` parts default to 0 — explicitly acceptable per CLAUDE.md |
| Project config | `project_read.rs` | 5 | Project settings use sensible defaults for missing keys |
| Optional API fields | `schlib_write.rs`, `schdoc_write.rs` | 4 | description.clone().unwrap_or_default() — genuinely optional |

#### Remaining ~3 instances NEEDING INVESTIGATION:

**1. `board_config.rs:266` — `RECORD` defaulting to empty string**

Documentation says RECORD is always `"Board"`. The serialization code conditionally
writes it only if non-empty (`if !config.record.is_empty()`), creating a roundtrip
asymmetry: if absent in old files → parsed as empty → not re-serialized.

**Recommendation**: Either make required (fail if absent) or always write `"Board"` on save.

**2. `board_config.rs:1033` — `BOARDVERSION` defaulting to empty string**

Board version string (e.g., `"26.0"`) is always written unconditionally during
serialization (`params.insert("BOARDVERSION", ...)`). If absent during parsing,
the empty default would be serialized as `BOARDVERSION=`, which may confuse Altium.

**Recommendation**: Verify if ever absent in real files. If always present, make required.
If sometimes absent in legacy files, set a sensible default version string on parse.

**3. `board_config.rs:1022` — `DISPLAYUNIT` defaulting to 0**

Serialized unconditionally. 0=mils (the Altium default), so the default value is
semantically correct. This is likely fine but should be documented.

**Recommendation**: Add a comment: `// 0=mils is Altium's default; absent in very old files`

---

**D6: Exhaustion Checks Must Be Called** — `crates/altium-format/src/schlib.rs:333-343`

`is_end_marker()` creates a `ParameterCollection` but never calls `assert_exhausted()`:

```rust
fn is_end_marker(block: &Block) -> Result<bool> {
    ...
    let mut params = ParameterCollection::from_bytes(&block.data)?;
    let record_val: i32 = match params.remove_optional::<i32>(RECORD)? {
        Some(v) => v,
        None => return Ok(true),   // ← early return, no exhaustion check
    };
    Ok(record_val == 0)            // ← final return, no exhaustion check
}
```

End-marker blocks (RECORD=0) may contain additional parameters that are silently ignored.
If Altium adds fields to end markers in future versions, they'll be invisible to this parser.

**Fix**: Add `params.assert_exhausted()?;` before each return, or at least before the final
`Ok(record_val == 0)` return. For the `None` early return, if end markers can truly be
empty blocks, that path is acceptable.

---

**D9: Block Format Must Be Preserved** — `crates/altium-format/src/pcblib/library.rs:95`

Manual block offset calculation assumes exactly one block exists:

```rust
let bytes_after_first_block = 4 + block.data.len(); // header + payload
let suffix_names = parse_library_data_suffix(&data[bytes_after_first_block..])?;
```

The code reads one block via `iter_blocks()`, drops the iterator, then manually calculates
where the post-block suffix data starts using `4 + block.data.len()`. If the format ever
contains multiple text blocks before the binary suffix, this offset will be wrong — the
parser would try to parse block header bytes as TLV name entries.

**Fix**: Consume ALL blocks from `iter_blocks()` until exhaustion, track the total bytes
consumed, then parse the suffix from the remaining data. Or use the iterator's position
tracking to determine where blocks end.

---

**D9: Block Format Must Be Preserved** — `crates/altium-format/src/pcblib/library.rs:511`

Same pattern in `parse_padvia_library_config()`:

```rust
let config_block_end = 4 + config_block.data.len();
let templates_bytes = &data[config_block_end..];
```

Assumes exactly one config block before the template data. Same risk as above.

**Fix**: Same as above — exhaust the block iterator before calculating the suffix offset.

---

**D3: No Raw Parameter String Passthrough** — `crates/altium-format/src/pcblib/primitives/region.rs:172-174`

`object_kind` stores a typed enum value as a raw `String`:

```rust
let object_kind = params
    .remove_optional::<String>("OBJECTKIND")?
    .unwrap_or_default();
```

`OBJECTKIND` has known values (`"BoardRegion"`, etc.) that control which fields are
present/absent. It should be an enum for type safety and exhaustive matching.

**Fix**: Define `enum RegionObjectKind { BoardRegion, Normal, ... }` in `altium-format-types`
and parse via `remove_optional::<RegionObjectKind>()`.

---

### INFO

**D3: Ambiguous string maps** — `crates/altium-format/src/board_config.rs:249-250`

Two `IndexMap<String, String>` fields for per-layer visual settings:

```rust
pub(crate) layer_opacity: IndexMap<String, String>,
pub(crate) workspace_col_alpha: IndexMap<String, String>,
```

These store dynamically-keyed per-layer configuration (keys are layer-name-dependent).
While they could potentially be parsed into typed `HashMap<LayerId, f64>` structures,
the dynamic keying makes this less clear-cut than the `cfg3d` case. Low risk since these
are visual-only configuration, not structural format data.

**Fix**: Investigate whether the key format is predictable enough to parse into typed
structures. If keys follow `LAYER_<N>_OPACITY` patterns, parse the index and value.

---

## Rules with No Violations Found

| Rule | Description | Status |
|------|-------------|--------|
| D1 | No Silent Data Dropping | CLEAN — strict fail-fast with `assert_exhausted()` and `assert_all_consumed()` throughout |
| D2 | No Raw Byte Passthrough for Structured Data | CLEAN — `PrefixedParamBlock.data` and `PcbBinaryRecord.data` are intermediary structures immediately parsed by callers |
| D5 | No Data Overwriting Without Detection | CLEAN — proper `entry()` API usage, duplicate detection, fresh ParameterCollection creation |
| D7 | No Partial Record Deserialization | CLEAN — no TODO/FIXME/unimplemented markers near record parsing |
| D10 | Sidecar Merge Ordering | CLEAN — primary streams always loaded before sidecar merging, with index/type validation |
| D11 | Encoding Boundary Discipline | CLEAN — no `from_utf8_lossy`, all UTF-16LE decodes check `had_errors`, Windows-1252 correctly used (cannot error) |

---

## Summary

| Severity | Count | Notes |
|----------|-------|-------|
| CRITICAL | 5 | 1x D6 (shared_union), 1x D3 (cfg3d), 4x D8 (i32→usize casts) |
| WARNING  | 5 | 1x D4 (3 suspicious, 278 legitimate), 1x D6 (is_end_marker), 2x D9 (block offset), 1x D3 (object_kind) |
| INFO     | 1 | 1x D3 (layer opacity/alpha maps) |
| **Total** | **11** | |

### D4 Deep Investigation Summary

Of the original 281 `.unwrap_or_default()` instances flagged:
- **278 are LEGITIMATE** — genuinely optional parameters due to format version evolution,
  subtype-specific fields, UI preferences, serialization patterns, or fractional companions.
- **3 need minor fixes** — `RECORD`, `BOARDVERSION`, and `DISPLAYUNIT` in `board_config.rs`
  have roundtrip asymmetries (parsed as optional but serialized unconditionally).

### Priority Recommendations

1. **Highest priority**: Fix the 4 unchecked `i32 as usize` casts (D8) — these are one-line
   fixes that prevent confusing error messages on malformed files.

2. **High priority**: Type the SharedUnion hidden primitives (D6/D3) — this is the most
   significant opaque data retention in the codebase.

3. **Medium priority**: Fix the 3 board_config.rs roundtrip asymmetries (D4) — `RECORD`,
   `BOARDVERSION`, `DISPLAYUNIT` are parsed as optional but serialized unconditionally.

4. **Low priority**: Fix block offset calculations in `library.rs` (D9), type the `cfg3d` map
   (D3), and add exhaustion check to `is_end_marker` (D6).
