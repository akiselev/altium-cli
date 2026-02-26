# /Storage UTF-8 Decode Error Research Report

## Problem Statement

9 SchDoc files fail with UTF-8 decode errors when parsing embedded objects in
the `/Storage` stream. The error originates from `parse_embedded_object()` in
`crates/altium-format/src/embedded_object.rs` at line 39-43.

## Current Parsing Code Analysis

### Where the error occurs

File: `crates/altium-format/src/embedded_object.rs`, function `parse_embedded_object()`

```rust
let id_bytes = reader.read_bytes(id_len)?;
let id = String::from_utf8(id_bytes.to_vec()).map_err(|e| {
    AltiumFormatError::InvalidEmbeddedObject(format!(
        "embedded object id contains invalid UTF-8: {e}"
    ))
})?;
```

The code reads the ID field from the `0xD0` envelope and attempts to decode it
as UTF-8 using `String::from_utf8()`. This fails when the ID contains bytes
outside the valid UTF-8 range (e.g., Windows-1252 characters 0x80-0x9F that are
not valid in UTF-8).

### Call chain

```
SchDoc::open()
  -> parse_storage_stream()              (schdoc/mod.rs:690)
     -> parse_blocks()
     -> parse_embedded_object_stream()   (embedded_object.rs:62)
        -> parse_embedded_object()       (embedded_object.rs:29)  <-- ERROR HERE
           -> String::from_utf8()        (embedded_object.rs:39)
```

### The `0xD0` envelope format

```
[1 byte]  0xD0 tag (INSTRUCTION_BINARY / CEmbeddedStream)
[1 byte]  id_length
[N bytes] id (Windows-1252 encoded string)
[4 bytes] compressed_length (flags | size)
[M bytes] zlib-compressed payload
```

## What Encoding Is Actually Used

### Definitive answer: Windows-1252 (ACP)

The C# source code confirms the encoding chain definitively:

**1. `SchDataEmbeddedObject.ExportToFile()` / `ImportFromFile()`**
(`AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.Objects/SchDataEmbeddedObject.cs`)

```csharp
public virtual void ExportToFile(ISchDataSerializer serializer)
{
    serializer.Export_DynamicString(name, "Name");
    serializer.Export_Binary(data, "Data");
}

public virtual void ImportFromFile(ISchDataSerializer serializer)
{
    serializer.Import_DynamicString(ref name, "Name");
    serializer.Import_Binary(out data, "Data");
}
```

**2. `Export_DynamicString` -> `WriteString`**
(`AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.Serialization/SchDataSerializer.cs`)

```csharp
public virtual void Export_DynamicString(string argN, string argName)
{
    WriteString(argN, argName);
}

protected virtual void WriteString(string data, string name)
{
    string text = data ?? string.Empty;
    byte[] bytes = DXP.Utils.EncodingDefault.GetBytes(text);
    WriteByte(Convert.ToByte(bytes.Length), name + "_Len");
    Assert(WriteData(bytes, bytes.Length, name) == bytes.Length);
}
```

**3. `Import_DynamicString` -> `ReadString`**

```csharp
public virtual void Import_DynamicString(ref string argN, string argName)
{
    ReadString(out argN, argName);
}

protected virtual void ReadString(out string data, string name)
{
    ReadByte(out var data2, name + "_Len");
    Assert(ReadData(out var buffer, data2, name) == data2);
    data = DXP.Utils.EncodingDefault.GetString(buffer);
}
```

**4. `DXP.Utils.EncodingDefault` = Windows Active Code Page**
(`AD26-dotnet/Altium.Dxp.Classes/DXP/Utils.cs`)

```csharp
public static Encoding EncodingDefault => EncodingACP;
public static Encoding EncodingACP => encodingACP.Value;

[DllImport("kernel32.dll")]
private static extern int GetACP();

private static Encoding GetEncodingACP()
{
    return Encoding.GetEncoding(GetACP());
}
```

`GetACP()` returns the Windows Active Code Page, which is **1252 (Windows-1252)**
on all Western Windows systems (the overwhelming majority of Altium users).

### Important nuance: locale-dependent

Technically, `GetACP()` returns whatever code page the system is configured to
use. On Western systems this is 1252 (Windows-1252). On Japanese systems it
would be 932 (Shift-JIS), on Chinese systems 936 (GBK), etc.

However, in practice:
- The vast majority of Altium files in the wild use Windows-1252
- Altium Designer itself primarily targets Western systems
- The parameter string encoding elsewhere in the codebase already uses
  Windows-1252 (confirmed by `container-format.md` and the existing
  `ParameterCollection::from_bytes()` implementation)
- For consistency with how we handle all other string encoding in the format,
  we should use Windows-1252

## Root Cause

**The embedded object ID field is encoded using Windows-1252, but our parser
decodes it as UTF-8.**

The ID field in the `0xD0` envelope is the `Name` property of
`SchDataEmbeddedObject`, written by `WriteString()` which uses
`DXP.Utils.EncodingDefault` (Windows-1252).

For the `/Storage` stream in SchDoc files, the embedded object name is
typically the image filename. When this filename contains characters outside
ASCII that have different byte representations in Windows-1252 vs UTF-8 (e.g.,
accented characters like e-acute `0xE9`, or characters in the 0x80-0x9F range
which are valid Windows-1252 but INVALID UTF-8), `String::from_utf8()` fails.

### Specific problematic byte ranges

Windows-1252 bytes that are NOT valid single-byte UTF-8:
- **0x80-0xBF**: All of these are valid Windows-1252 but are continuation bytes
  in UTF-8 (invalid as leading bytes)
- **0xC0-0xFF**: In UTF-8 these are leading bytes for multi-byte sequences;
  without proper continuation bytes, they are invalid UTF-8

Examples of characters that would trigger this error:
- `0x80` = Euro sign (EUR) in Windows-1252
- `0x85` = horizontal ellipsis
- `0x92` = right single quotation mark
- `0x93`/`0x94` = left/right double quotation marks
- `0xE9` = e-acute (valid Latin-1 but part of multi-byte in UTF-8)
- `0xFC` = u-umlaut
- `0xF1` = n-tilde

## Recommended Fix

### Change: Use `encoding_rs::WINDOWS_1252` instead of `String::from_utf8()`

In `crates/altium-format/src/embedded_object.rs`, replace the UTF-8 decode with
Windows-1252 decode:

**Before (line 38-43):**
```rust
let id_bytes = reader.read_bytes(id_len)?;
let id = String::from_utf8(id_bytes.to_vec()).map_err(|e| {
    AltiumFormatError::InvalidEmbeddedObject(format!(
        "embedded object id contains invalid UTF-8: {e}"
    ))
})?;
```

**After:**
```rust
let id_bytes = reader.read_bytes(id_len)?;
let (id_cow, _encoding_used, _had_replacements) =
    encoding_rs::WINDOWS_1252.decode(id_bytes);
let id = id_cow.into_owned();
```

Note: `encoding_rs::WINDOWS_1252.decode()` is infallible -- all 256 byte values
are valid Windows-1252, so no error handling is needed. This matches the
contract documented in `CLAUDE.md`:
> Windows-1252: `encoding_rs::WINDOWS_1252.decode()` (all 256 byte values are
> valid, cannot error)

### Serialization counterpart

In `serialize_embedded_object()` (line 113-122), the ID is written using
`id.as_bytes()` which produces UTF-8 bytes. This should be changed to encode
as Windows-1252:

**Before:**
```rust
w.write_bytes(id.as_bytes());
```

**After:**
```rust
let (id_bytes, _encoding_used, _had_unmappable) =
    encoding_rs::WINDOWS_1252.encode(id);
w.write_bytes(&id_bytes);
```

Note: `encoding_rs::WINDOWS_1252.encode()` will replace unmappable characters
with `?`. Since Altium only stores Windows-1252 representable strings in
this field, this should be fine. If we want to be strict, we could check
`_had_unmappable` and return an error.

### Impact on other callers

The `parse_embedded_object()` function is also called for:
- SchLib pin sidecar streams (PinFrac, PinDesc, PinWideText, etc.)
- SchDoc Storage stream (embedded images)

In pin sidecar streams, the ID is typically a numeric index string ("0", "1",
"2", ...) which is pure ASCII. The fix is safe for these cases since ASCII
is a subset of both UTF-8 and Windows-1252.

For SchDoc Storage, the ID is the image filename which can contain
Windows-1252 characters. This is the case that triggers the error.

### No changes needed in `ParameterCollection`

The `ParameterCollection::from_bytes()` already correctly uses Windows-1252
decoding for parameter strings. The embedded object ID is NOT parsed through
`ParameterCollection` -- it's read directly in the binary envelope parser.

## Files to Modify

1. `crates/altium-format/src/embedded_object.rs`
   - `parse_embedded_object()`: Change `String::from_utf8()` to `WINDOWS_1252.decode()`
   - `serialize_embedded_object()`: Change `id.as_bytes()` to `WINDOWS_1252.encode()`

## Verification

After fixing, all 9 previously-failing SchDoc files should parse successfully.
Run:
```bash
cargo test -p altium-format --features test-fixtures
```
And validate all SchDoc fixtures:
```bash
for f in data/schdoc/*.SchDoc; do cargo run -p altium-cli -- validate "$f"; done
```
