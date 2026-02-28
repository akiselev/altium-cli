# Milestone 9: Pin Sidecar Streams

**Files**: `crates/altium-format/src/schlib.rs`

**Depends on**: M5 (Binary Pin)

**Flags**: `complex-algorithm`, `needs-rationale`

## Requirements

Implement parsing and merging of all 9 pin sidecar streams. Each component can have up to 9 optional sidecar streams that provide extended pin data beyond what the binary pin format stores. Streams must be applied in exact order — PinWideText is authoritative and replaces binary text fields.

## Sidecar Streams (Import Order)

All sidecar streams use the embedded object envelope format (parsed by existing `embedded_object.rs`). Each stream's header block has RECORD=0 + HEADER + Weight. Entry blocks contain per-pin data indexed by pin position.

### 1. PinFrac — Sub-unit coordinate precision

**Format**: 12 bytes per pin (3 × i32 LE)
```
Offset  Type    Field
0x00    i32 LE  location_x_frac
0x04    i32 LE  location_y_frac
0x08    i32 LE  length_frac
```

**Merge**: Additive to binary pin coordinates:
```
pin.location.x = (binary_x * C_BASE_UNIT) + location_x_frac
pin.location.y = (binary_y * C_BASE_UNIT) + location_y_frac
pin.pin_length = (binary_length * C_BASE_UNIT) + length_frac
```

### 2. PinDesc — Description overflow

**Format**: i32 LE text_length + ASCII text bytes

**Merge**: Appends to binary pin description:
```
pin.description = pin.description + overflow_text
```

Only present when description exceeds 254 bytes (binary limit).

### 3. PinMiscData — PairSwapID

**Format**: i32 LE text_length + UTF-16LE parameter string

**Parameters**: `PairSwapID=<value>`

**Merge**: Sets `pin.swap_id_pin = value`

### 4. PinTextData — Custom text positioning/font

**Format**: Two consecutive binary structs (name text, then designator text):

Per struct:
```
Offset  Size  Type   Field
0x00    1     u8     flags
  bit 0: PositionMode (0=default, 1=custom)
  bit 1: RotationAnchor
  bits 2-3: RotationRelative
  bit 4: FontMode (0=default, 1=custom)
(if PositionMode=Custom):
0x01    4     i32    custom_margin
(if FontMode=Custom):
+0      2     i16    custom_font_id
+2      4     i32    custom_color (COLORREF)
```

**Merge**: Sets `pin.name_text_data` and `pin.designator_text_data`

### 5. PinWideText — AUTHORITATIVE text replacement

**Format**: i32 LE text_length + UTF-16LE parameter string

**Parameters**: `Desc`, `Name`, `Desig`, `SwapId`, `SwapIDPart`, `DefValue`

**Merge**: REPLACES binary pin text fields entirely:
```
if present("Desc"):  pin.description = value
if present("Name"):  pin.name = value
if present("Desig"): pin.designator = value
if present("SwapId"):     pin.swap_id_pin = value
if present("SwapIDPart"): pin.swap_id_part = value
if present("DefValue"):   pin.default_value = value
```

PinWideText is authoritative — it fully replaces binary text and any PinDesc overflow.

### 6. PinSymbolLineWidth

**Format**: i32 LE text_length + UTF-16LE parameter string

**Parameters**: `SymBol_LineWidth=<value>` (note exact casing)

**Merge**: Sets `pin.pin_symbol_line_width`

### 7. PinPackageLength

**Format**: i32 LE text_length + UTF-16LE parameter string

**Parameters**: `PinPackageLength=<value>` (internal coordinate units)

**Merge**: Sets `pin.pin_package_length`

### 8. PinPropagationDelay

**Format**: i32 LE text_length + UTF-16LE parameter string

**Parameters**: `PinPropagationDelay=<value>` (scientific notation, e.g., `1.5E-9`)

**Merge**: Sets `pin.propagation_delay`

### 9. PinFunctionData

**Format**: i32 LE text_length + UTF-16LE parameter string

**Parameters**:
- `PinSelectedFunctionsCount`, `PinSelectedFunction1`, `PinSelectedFunction2`, ... (1-based)
- `PinDefinedFunctionsCount`, `PinDefinedFunction1`, `PinDefinedFunction2`, ... (1-based)

**Merge**: Sets `pin.selected_functions` and `pin.defined_functions`

## Sidecar Merge Function

```rust
fn merge_pin_sidecars(
    cfb: &mut TrackedCfbDocument,
    component_key: &str,
    pins: &mut [SchPin],
) -> Result<()> {
    // Apply each sidecar in order
    merge_pin_frac(cfb, component_key, pins)?;
    merge_pin_desc(cfb, component_key, pins)?;
    merge_pin_misc_data(cfb, component_key, pins)?;
    merge_pin_text_data(cfb, component_key, pins)?;
    merge_pin_wide_text(cfb, component_key, pins)?;
    merge_pin_symbol_line_width(cfb, component_key, pins)?;
    merge_pin_package_length(cfb, component_key, pins)?;
    merge_pin_propagation_delay(cfb, component_key, pins)?;
    merge_pin_function_data(cfb, component_key, pins)?;
    Ok(())
}
```

Each `merge_*` function:
1. Reads the stream via `cfb.read_stream_optional()`
2. If present, parses via `parse_embedded_object_stream()`
3. Matches entries to pins by embedded object ID (decimal pin index)
4. Applies the merge semantics described above

## Acceptance Criteria

- All 9 sidecar streams parsed and merged in correct order
- PinFrac coordinates additive to binary pin coordinates
- PinDesc appends to description
- PinWideText replaces binary text fields (authoritative)
- PinTextData binary format decoded correctly (variable-length conditional fields)
- Missing sidecar streams handled gracefully (stream absent = no-op)
- Pin index from embedded object ID maps correctly to pin position in records list
- `altium validate` handles components with and without sidecar streams

## Tests

- **Test files**: `crates/altium-format/src/schlib.rs` (inline `#[cfg(test)]`)
- **Test type**: integration (real SchLib files)
- **Backing**: doc-derived (docs/schlib/pin-sidecar-streams.md)
- **Scenarios**:
  - Normal: PinFrac adds sub-unit precision
  - Normal: PinWideText replaces binary text
  - Normal: PinMiscData sets swap ID
  - Normal: all 9 sidecars applied in order
  - Edge: missing sidecar stream (no-op)
  - Edge: PinDesc append followed by PinWideText replace (final value is PinWideText)
  - Edge: PinTextData with custom font/positioning
  - Edge: PinFunctionData with multiple selected functions

## Code Intent

- Add to `crates/altium-format/src/schlib.rs`:
  - `merge_pin_sidecars()` orchestration function
  - Individual `merge_pin_frac()`, `merge_pin_desc()`, etc. functions (9 total)
  - `parse_pin_text_data(data: &[u8]) -> Result<(PinTextPositioning, PinTextPositioning)>` for PinTextData binary parsing
- Each merge function reads optional stream, parses embedded objects, and modifies pins in-place
- Uses existing `parse_embedded_object_stream()` from embedded_object.rs
- UTF-16LE parameter streams use `ParameterCollection::from_utf16le_bytes()`
- PinFrac uses BinaryReader for 12-byte binary entries
- PinTextData uses BinaryReader for variable-length binary with conditional fields
