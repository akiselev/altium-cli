# P0: Pascal String Panic in binary_io.rs (strings >255 bytes)

## Problem

3 SchLib files crash on `save-as` with:
```
Pascal string too long: N bytes (max 255)
```
at `binary_io.rs:437`.

Affected files:
- `Custom.SchLib` (346 bytes)
- `kmilo17pet-Maxim_Power.SchLib` (355 bytes)
- `ryankurte-electronpowered.SchLib` (336 bytes)

## Current Code Analysis

### The panic site: `BinaryWriter::write_pascal_string`

File: `crates/altium-format/src/binary_io.rs:435-444`

```rust
pub(crate) fn write_pascal_string(&mut self, s: &str) {
    let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(s);
    assert!(
        encoded.len() <= 255,
        "Pascal string too long: {} bytes (max 255)",
        encoded.len()
    );
    self.write_u8(encoded.len() as u8);
    self.buf.extend_from_slice(&encoded);
}
```

### The caller: `serialize_binary_pin`

File: `crates/altium-format/src/sch_records.rs:1990-2022`

```rust
pub(crate) fn serialize_binary_pin(pin: &SchPin) -> Vec<u8> {
    let mut w = BinaryWriter::new();
    // ...
    w.write_pascal_string(&pin.description);  // <-- PANICS HERE (line 2003)
    // ...
    w.write_pascal_string(&pin.name);         // <-- Can also panic (line 2015)
    w.write_pascal_string(&pin.designator);   // <-- Can also panic (line 2016)
    w.write_pascal_string(&pin.swap_id_pin);  // line 2017
    w.write_pascal_string(&pin.swap_id_part); // line 2018
    w.write_pascal_string(&pin.default_value);// line 2019
    w.finish()
}
```

The function writes ALL pin string fields as u8-length Pascal strings without truncation.
When the in-memory model has strings >255 bytes (from PinWideText/PinDesc sidecar merge),
the assertion panics.

### Sidecar write code (already correct)

File: `crates/altium-format/src/schlib.rs`

The sidecar writing functions (`write_pin_desc`, `write_pin_wide_text`) are already
implemented correctly. They check for overflow and emit PinDesc/PinWideText sidecar
streams. The ONLY problem is that `serialize_binary_pin` doesn't truncate strings
before writing them to the binary pin record.

## C# Reference Code Analysis

### How Altium writes pin strings in binary mode

**Key serializer method chain:**

1. `FileFormatV5.ExportPin()` calls `Export_DynamicString()` for Description, Name, and Designator.
2. `SchDataSerializerParam.Export_DynamicString()` (binary mode override):

```csharp
// SchDataSerializerParam.cs:1012-1025
public override void Export_DynamicString(string argN, string argName)
{
    if (mode == 1) // binary mode
    {
        string text = argN ?? string.Empty;
        int num = 254;
        int num2 = ((text.Length > num) ? num : text.Length);
        WriteString(text.Substring(0, num2), argName);  // TRUNCATE to 254
    }
    else
    {
        base.Export_DynamicString(argN, argName);
    }
}
```

3. `SchDataSerializer.WriteString()` (the underlying writer):

```csharp
// SchDataSerializer.cs:166-172
protected virtual void WriteString(string data, string name)
{
    string text = data ?? string.Empty;
    byte[] bytes = DXP.Utils.EncodingDefault.GetBytes(text);
    WriteByte(Convert.ToByte(bytes.Length), name + "_Len");  // u8 length prefix
    Assert(WriteData(bytes, bytes.Length, name) == bytes.Length);
}
```

**Key insight:** Altium's `Export_DynamicString` in binary mode **truncates to 254 characters**
before writing the u8-length Pascal string. The full data is preserved in sidecar streams.

### Sidecar mechanism for overflow data

**PinDesc** (`SchDataExporterLibraryV5.cs:343-354`):
```csharp
private void AddPinLongDescriptionData(ISchDataPin pin, int index, ...)
{
    string text = pin.GetDescription() ?? string.Empty;
    if (text.Length > 254)
    {
        string value = text.Substring(254, text.Length - 254); // overflow ONLY
        // Write to PinDesc sidecar stream
    }
}
```

**PinWideText** (`SchDataExporterLibraryV5.cs:393-409`):
```csharp
private void AddPinWideTextData(ISchDataPin pin, int index, ...)
{
    TryToSetParameterValue(ref parameters, "Desc", pin.GetDescription());
    TryToSetParameterValue(ref parameters, "Name", pin.GetName());
    TryToSetParameterValue(ref parameters, "Desig", pin.GetDesignator());
    TryToSetParameterValue(ref parameters, "SwapId", pin.GetSwapIdPin());
    TryToSetParameterValue(ref parameters, "SwapIDPart", pin.GetSwapIdPartAndPartPin());
    TryToSetParameterValue(ref parameters, "DefValue", pin.GetDefaultValue());
    // Each field written only if NeedToSaveParameter() returns true
}
```

**NeedToSaveParameter** (`SchDataExporterLibraryV5.cs:711-722`):
```csharp
private bool NeedToSaveParameter(string value)
{
    if (!string.IsNullOrEmpty(value))
    {
        if (value.Length < 254)
        {
            return StrUtils.HasNonAnsiSymbols(value);  // non-ASCII chars
        }
        return true;  // length >= 254 always needs sidecar
    }
    return false;
}
```

### Field categorization in FileFormatV5.ExportPin

| Field | Export method | Truncation | Can exceed 255? |
|-------|-------------|------------|-----------------|
| Description | `Export_DynamicString` | Yes (254 chars) | Yes (via PinDesc + PinWideText sidecars) |
| Name | `Export_DynamicString` | Yes (254 chars) | Yes (via PinWideText sidecar) |
| Designator | `Export_DynamicString` | Yes (254 chars) | Yes (via PinWideText sidecar) |
| SwapIdPin | `Export_String` | No (plain Pascal) | Should not exceed 255 in practice |
| SwapIDPart | `Export_String` | No (plain Pascal) | Should not exceed 255 in practice |
| DefaultValue | `Export_String` | No (plain Pascal) | Should not exceed 255 in practice |

## Root Cause

`serialize_binary_pin()` writes pin string fields directly as Pascal strings without
applying the truncation that Altium's `Export_DynamicString` performs in binary mode.

When a file is loaded, the PinWideText sidecar merges the full string into the in-memory
model (e.g., a 346-byte description). When we save it back, `serialize_binary_pin` tries
to write the full 346-byte string as a Pascal string and panics because the u8 length
prefix can only hold 0-255.

The data flow causing the crash:
```
File load:
  1. Binary pin record parsed -> description = first 254 chars
  2. PinDesc sidecar merged  -> description += overflow chars (chars 254+)
  3. PinWideText merged      -> description = full Unicode text (authoritative)
  Result: pin.description = 346 chars (full text)

File save:
  1. serialize_binary_pin() -> write_pascal_string(346-byte string) -> PANIC!
  2. write_pin_desc() would write the overflow sidecar (never reached)
  3. write_pin_wide_text() would write the Unicode sidecar (never reached)
```

## Recommended Fix

### Option A: Truncate in `serialize_binary_pin` (matches C# behavior exactly)

Change `serialize_binary_pin` to truncate `DynamicString` fields to 254 bytes when
encoding as Pascal strings, matching the `SchDataSerializerParam.Export_DynamicString`
behavior exactly.

**Changes needed in `crates/altium-format/src/sch_records.rs`:**

Add a helper function for DynamicString truncation:

```rust
/// Truncates a string to at most 254 Windows-1252 bytes for binary pin serialization.
/// Matches Altium's `SchDataSerializerParam.Export_DynamicString` which truncates
/// DynamicString fields to 254 chars in binary mode. The full text is preserved
/// in PinDesc and PinWideText sidecar streams.
fn write_dynamic_string(w: &mut BinaryWriter, s: &str) {
    let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(s);
    let truncated = if encoded.len() > 254 { &encoded[..254] } else { &encoded };
    w.write_u8(truncated.len() as u8);
    w.write_bytes(truncated);
}
```

Then update `serialize_binary_pin`:

```rust
pub(crate) fn serialize_binary_pin(pin: &SchPin) -> Vec<u8> {
    let mut w = BinaryWriter::new();
    // ...
    write_dynamic_string(&mut w, &pin.description);  // truncate to 254
    // ...
    write_dynamic_string(&mut w, &pin.name);          // truncate to 254
    write_dynamic_string(&mut w, &pin.designator);    // truncate to 254
    w.write_pascal_string(&pin.swap_id_pin);           // plain Pascal (should be short)
    w.write_pascal_string(&pin.swap_id_part);          // plain Pascal (should be short)
    w.write_pascal_string(&pin.default_value);         // plain Pascal (should be short)
    w.finish()
}
```

### Option B: Add `write_pascal_string_truncated` to BinaryWriter

Add a method to `BinaryWriter` that truncates instead of panicking:

```rust
/// Writes a Pascal-style string, truncating to at most `max_len` Windows-1252 bytes.
/// Used for DynamicString fields where overflow is handled by sidecar streams.
pub(crate) fn write_pascal_string_truncated(&mut self, s: &str, max_len: u8) {
    let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(s);
    let len = encoded.len().min(max_len as usize);
    self.write_u8(len as u8);
    self.buf.extend_from_slice(&encoded[..len]);
}
```

### Recommendation: Option A

Option A is preferred because:
1. The truncation logic is localized to pin serialization where the semantic context is clear
2. It explicitly matches the C# `Export_DynamicString` behavior with a comment
3. `BinaryWriter` stays simple and honest -- `write_pascal_string` correctly panics if
   you try to write >255 bytes, which catches bugs elsewhere
4. The `write_pascal_string` panic is actually a good safety net for PCB pascal strings
   and other contexts where strings should genuinely never exceed 255 bytes

### Additional: Convert panic to Result in `write_pascal_string`

Even though Option A prevents the pin-related panics, `write_pascal_string` should
return `Result` instead of panicking, to match the project's error handling philosophy:

```rust
pub(crate) fn write_pascal_string(&mut self, s: &str) -> Result<()> {
    let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(s);
    if encoded.len() > 255 {
        return Err(AltiumFormatError::InvalidParamValue {
            key: "pascal_string".to_owned(),
            detail: format!(
                "string too long: {} bytes (max 255)",
                encoded.len()
            ),
        });
    }
    self.write_u8(encoded.len() as u8);
    self.buf.extend_from_slice(&encoded);
    Ok(())
}
```

This would require updating all callsites to propagate the error with `?`.

## Files That Need to Change

1. **`crates/altium-format/src/sch_records.rs`** (primary fix)
   - `serialize_binary_pin()` (~line 1990): Add truncation for Description, Name, Designator

2. **`crates/altium-format/src/binary_io.rs`** (secondary improvement)
   - `write_pascal_string()` (~line 435): Convert `assert!` to `Result` return

3. **All callers of `write_pascal_string`** (if converting to Result):
   - `crates/altium-format/src/pcb_file_header.rs` (5 callsites)
   - `crates/altium-format/src/pcblib/mod.rs` (8 callsites)
   - `crates/altium-format/src/pcblib/library.rs` (1 callsite)
   - `crates/altium-format/src/sch_records.rs` (remaining non-DynamicString pins: swap_id_pin, swap_id_part, default_value)

## Verification

After fixing, these files should roundtrip without panics:
- `Custom.SchLib` (346-byte description)
- `kmilo17pet-Maxim_Power.SchLib` (355-byte description)
- `ryankurte-electronpowered.SchLib` (336-byte description)

Semantic diff should show no regressions -- the binary pin records will contain
truncated text (matching Altium behavior) and the sidecar streams will carry the
full text.
