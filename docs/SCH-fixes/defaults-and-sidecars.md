# Research Report: Default Value Omission and Missing Sidecar Streams

## Issue 1: Default Value Omission (SymbolType, ShowNetName)

### Problem

Our serializer omits parameters when their value equals the type's `Default::default()`.
Altium always writes certain parameters regardless of value. This causes `MissingParamPair`
diffs in ~60% of SchDoc roundtrips.

### Affected Records

**SchSheetSymbol (RECORD=15) -- `SymbolType`**

- File: `crates/altium-format/src/sch_records.rs:1723`
- Current: `#[param(key = SYMBOL_TYPE, default = SheetSymbolType::Normal)]`
- Serialization tier: T1 (skip when value == `Default::default()`)
- `SheetSymbolType::Normal` IS the `#[default]` variant (line `sch.rs:1216`)
- T1 comparison: `Normal != Normal` = false => SKIPPED
- But C# `ExportSheetSymbol` (FileFormatV5.cs:2219) uses `Export_DynamicString(SheetSymbolTypeToString(...), "SymbolType")` which always writes non-empty strings

**SchPowerObject (RECORD=17) -- `ShowNetName`**

- File: `crates/altium-format/src/sch_records.rs:1557`
- Current: `#[param(key = SHOW_NET_NAME, default = true)]`
- Serialization tier: T1 (skip when value == `Default::default()`)
- `Default::default()` for bool = `false`
- When `show_net_name = false`: `false != false` = doesn't write => SKIPPED
- But C# `ExportPower` (FileFormatV5.cs:1481) uses `Export_Boolean_WithDefault(...)` which always writes both `T` and `F`

### C# Serialization Methods

Two distinct boolean export methods exist in SchDataSerializerParam.cs:

```csharp
// WriteBool (T1 -- only writes TRUE):
protected override void WriteBool(bool value, string name) {
    // mode==1 is binary mode (irrelevant for SchDoc text)
    else if (value) { SetParameter(name, "T"); }  // ONLY writes when true
}

// WriteBoolWithDefault (T2 -- always writes):
protected override void WriteBoolWithDefault(bool value, string name) {
    // mode==1 is binary mode
    else { SetParameter(name, value ? "T" : "F"); }  // ALWAYS writes
}
```

`ShowNetName` uses `Export_Boolean_WithDefault` = T2.

For strings, `Export_DynamicString` -> `WriteString` only skips `null`/empty strings.
`SymbolType=Normal` is a non-empty string, so it always writes.

### Fix

Add `tier2` flag to both fields in the derive macro annotations:

```rust
// sch_records.rs SchSheetSymbol (RECORD=15):
#[param(tier2, key = SYMBOL_TYPE, default = SheetSymbolType::Normal)]
pub symbol_type: SheetSymbolType,

// sch_records.rs SchPowerObject (RECORD=17):
#[param(tier2, key = SHOW_NET_NAME, default = true)]
pub show_net_name: bool,
```

The `tier2` flag makes `to_serialize_tokens` emit unconditionally (no skip check).

### Additional `tier2` Candidates

These fields also use `Export_Boolean_WithDefault` in C# and may need `tier2`:

| Record | Field | C# Method | Location |
|--------|-------|-----------|----------|
| SchNoERC (RECORD=22) | `is_active` | `Export_Boolean_WithDefault` | FileFormatV5.cs:1563 |
| SchNoERC (RECORD=22) | `suppress_all` | `Export_Boolean_WithDefault` | FileFormatV5.cs:1564 |
| SchComponent (RECORD=1) | `part_id_locked` | `Export_Boolean_WithDefault` | FileFormatV5.cs:2892 |

Check if these are already `tier2` in our code; if not, add the flag.

---

## Issue 2: SectionKeys Stream Not Written on Save

### Status: Already Implemented

The `/SectionKeys` stream IS written during SchLib save (`schlib.rs:3084-3087`):

```rust
// 3. /SectionKeys (optional)
if let Some(section_keys_data) = serialize_section_keys(&section_keys) {
    cfb.write_stream(&format!("/{SECTION_KEYS}"), &section_keys_data)?;
}
```

### Root Cause of Roundtrip Diff

The `build_section_keys()` function (schlib.rs:1712) generates entries only when:
```rust
sanitized != default_fallback || sanitized.len() > 31
```

But the C# `SchDataComponentSectionKeyList.AddKey()` (SchDataComponentSectionKeyList.cs:26-48)
uses a simpler check:
```csharp
if (string.IsNullOrEmpty(name) || name.Length < maxKeyLength)
    return;
```

Key differences:
1. C# checks `name.Length >= 31` (raw name length), NOT sanitized length
2. C# does NOT sanitize special characters in the name -- it just truncates to 31 chars
3. C# generates unique keys by truncation + numeric suffix conflict resolution
4. Our code sanitizes `/\:*?"<>|!` to `_` BEFORE checking length

This means: if a component name is exactly 31 chars with no special characters, C# skips it
(length < 31 fails) but our code also skips it. If a name has special characters that change
its effective length... there could be discrepancies.

The actual `EntryMissingInB` errors may be caused by the character sanitization difference:
our Rust code sanitizes the name before truncation while C# truncates the raw name. For
names with special characters shorter than 31 chars, C# won't create a SectionKeys entry
but will use the raw name (with specials) as the CFB key. But CFB storage names can't
contain those characters...

### Recommended Fix

The SectionKeys logic should match C#'s exact behavior:
1. Only generate entries when `name.Length >= 31` (not sanitized name)
2. The truncated key should be `name[0..31]`, not sanitized then truncated
3. CFB storage name resolution should handle the character mapping separately

However, this may require deeper investigation of how Altium resolves CFB keys for names
with special characters that are shorter than 31 chars.

---

## Issue 3: PinMiscData Sidecar -- Two Bugs

### Bug 1: Wrong Field Read/Written

**Parse side** (schlib.rs:636-638):
```rust
// BUG: sets swap_id_pin but should set swap_id_pair
if let Some(v) = params.remove_optional::<String>(PAIR_SWAP_ID)? {
    pins[pin_idx].swap_id_pin = v;  // WRONG! Should be swap_id_pair
}
```

**Serialize side** (schlib.rs:980-982):
```rust
// BUG: reads swap_id_pin but should read swap_id_pair
if pin_field_needs_wide_text(&pin.swap_id_pin) {  // WRONG field
    let mut params = ParameterCollection::new();
    params.insert(PAIR_SWAP_ID, pin.swap_id_pin.clone());  // WRONG field
}
```

**C# reference** (SchDataExporterLibraryV5.cs:356-367):
```csharp
private void AddPinMiscDataData(ISchDataPin pin, int index, ...) {
    if (!string.IsNullOrEmpty(pin.GetSwapIdPair())) {  // SwapIdPAIR, not SwapIdPin
        SetParameterValue(ref parameters, "PairSwapID", pin.GetSwapIdPair());
```

### Bug 2: Wrong Write Condition

**Current**: `pin_field_needs_wide_text(&pin.swap_id_pin)` -- checks needs-wide-text
**Correct**: `!pin.swap_id_pair.is_empty()` -- C# just checks non-empty

The C# code writes PinMiscData whenever `swap_id_pair` is non-empty, regardless of
encoding needs.

### Bug 3: Sidecar Key Case

`StrUtils.SetParameterValue` uppercases the key name:
```csharp
string text = ((name == null) ? string.Empty : name.ToUpper());
```

So the sidecar stores `|PAIRSWAPID=value|` not `|PairSwapID=value|`. Since our
`read_sidecar_utf16le_params` uses case-insensitive lookup, parsing works. But on write
we produce `|PairSwapID=value|` (mixed case) instead of `|PAIRSWAPID=value|` (uppercase).

### Fix

```rust
// Parse: schlib.rs:637
pins[pin_idx].swap_id_pair = v;  // was: swap_id_pin

// Serialize: schlib.rs:979-984
if !pin.swap_id_pair.is_empty() {
    let mut params = ParameterCollection::new();
    params.insert(PAIR_SWAP_ID, pin.swap_id_pair.clone());
    entries.push((i.to_string(), write_sidecar_utf16le_params(&params)));
}
```

For the key case issue, see Issue 4 below (applies to all sidecar streams).

---

## Issue 4: PinWideText Embedded Object Byte Differences

### Root Cause: Sidecar Parameter Key Uppercasing

The `StrUtils.SetParameterValue` method in C# (StrUtils.cs:131-142) uppercases all
parameter keys:

```csharp
public static void SetParameterValue(ref string parameters, string name, string value) {
    string text = ((name == null) ? string.Empty : name.ToUpper());
    parameters = string.Concat("|", text, "=", ReplaceSpecialParameterChars(value));
}
```

This is used by ALL sidecar stream writers (PinWideText, PinMiscData, PinSymbolLineWidth,
PinPackageLength, PinPropagationDelay). So sidecar UTF-16LE params always have UPPERCASE
keys in Altium's output.

Our code writes mixed-case keys because we use the constants directly:
- `DESC = "Desc"` -> file gets `|Desc=...|` but Altium writes `|DESC=...|`
- `NAME = "Name"` -> file gets `|Name=...|` but Altium writes `|NAME=...|`
- etc.

The "case change at offset 8" in the diff report corresponds to the first key in the
UTF-16LE data being `|DESC=...` (uppercase, 10 bytes for `|DESC` in UTF-16LE) vs
`|Desc=...` (mixed, also 10 bytes) -- the `D` at offset 2 is the same but `e` vs `E`
at offset 4 differs.

### Affected Constants

All sidecar-only constants that are passed through `StrUtils.SetParameterValue`:

| Our Constant | Value | Should Be (in sidecar) | Already Correct? |
|-------------|-------|----------------------|-----------------|
| `DESC` | `"Desc"` | `"DESC"` | No |
| `NAME` | `"Name"` | `"NAME"` | No |
| `DESIG` | `"Desig"` | `"DESIG"` | No |
| `SWAP_ID` | `"SwapId"` | `"SWAPID"` | No |
| `SWAP_ID_PART` | `"SwapIDPart"` | `"SWAPIDPART"` | No |
| `DEF_VALUE` | `"DefValue"` | `"DEFVALUE"` | No |
| `PAIR_SWAP_ID` | `"PairSwapID"` | `"PAIRSWAPID"` | No |
| `SIDECAR_SYMBOL_LINE_WIDTH` | `"SYMBOL_LINEWIDTH"` | `"SYMBOL_LINEWIDTH"` | Yes |
| `SIDECAR_PIN_PACKAGE_LENGTH` | `"PINPACKAGELENGTH"` | `"PINPACKAGELENGTH"` | Yes |
| `PIN_PROPAGATION_DELAY_KEY` | `"PinPropagationDelay"` | `"PINPROPAGATIONDELAY"` | No |
| `PIN_SELECTED_FUNCTIONS_COUNT` | `"PinSelectedFunctionsCount"` | `"PINSELECTEDFUNCTIONSCOUNT"` | No |
| `PIN_SELECTED_FUNCTION` | `"PinSelectedFunction"` | `"PINSELECTEDFUNCTION"` | No |
| `PIN_DEFINED_FUNCTIONS_COUNT` | `"PinDefinedFunctionsCount"` | `"PINDEFINEDFUNCTIONSCOUNT"` | No |
| `PIN_DEFINED_FUNCTION` | `"PinDefinedFunction"` | `"PINDEFINEDFUNCTION"` | No |

### Recommended Fix

Two options:

**Option A: Uppercase keys in `write_sidecar_utf16le_params`**

Modify `to_utf16le_bytes` (or a wrapper) to uppercase keys before encoding:

```rust
fn write_sidecar_utf16le_params(params: &ParameterCollection) -> Vec<u8> {
    let utf16_bytes = params.to_utf16le_bytes_uppercase_keys();
    // ...
}
```

This is the cleanest approach because sidecar key case is a serialization concern,
not a data model concern. The constants stay mixed-case for documentation clarity.

**Option B: Define separate uppercase sidecar constants**

Add `SIDECAR_*` uppercase constants and use those in sidecar write functions.

Option A is recommended because it matches C#'s architecture (uppercasing happens at
the serialization boundary in `StrUtils.SetParameterValue`).

### Additional Note: Off-by-one in NeedToSaveParameter

C# `NeedToSaveParameter` (SchDataExporterLibraryV5.cs:711-722):
```csharp
if (value.Length < 254) return StrUtils.HasNonAnsiSymbols(value);
return true;  // length >= 254
```

Our `pin_field_needs_wide_text`:
```rust
value.len() > 254 || value.chars().any(|c| c as u32 > 0x7E && c as u32 != 0x8E)
```

C# threshold: `>= 254` (uses `< 254` as short-circuit).
Rust threshold: `> 254` = `>= 255`.

Fix: change `value.len() > 254` to `value.len() >= 254`.

Also note: `value.len()` in Rust is byte count, but C# `value.Length` is character count.
For ASCII strings they're the same, but for multi-byte UTF-8 they differ. Since the
comparison is against the Pascal string limit (254 chars), it should be character count:
`value.chars().count() >= 254`.

---

## Summary of All Fixes

| Issue | File | Fix | Priority |
|-------|------|-----|----------|
| SymbolType default omission | `sch_records.rs:1723` | Add `tier2` flag | P1 |
| ShowNetName default omission | `sch_records.rs:1557` | Add `tier2` flag | P1 |
| PinMiscData wrong field (parse) | `schlib.rs:637` | Change `swap_id_pin` to `swap_id_pair` | P1 |
| PinMiscData wrong field (write) | `schlib.rs:980-982` | Change `swap_id_pin` to `swap_id_pair` | P1 |
| PinMiscData wrong condition | `schlib.rs:980` | Change to `!pin.swap_id_pair.is_empty()` | P1 |
| Sidecar key uppercasing | `param_collection.rs` or `schlib.rs` | Uppercase keys in `to_utf16le_bytes` | P2 |
| NeedToSaveParameter off-by-one | `schlib.rs:1045` | Change `> 254` to `>= 254`, use char count | P3 |
| SectionKeys char sanitization | `schlib.rs:1712-1743` | Match C#'s raw-name truncation logic | P3 |
