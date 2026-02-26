# SchLib Serialization Quality: Key Case & Windows-1252 Encoding

## Summary

Two roundtrip fidelity issues affect 101+ SchLib files:

1. **Key case normalization** -- serializer uppercases ALL parameter keys but original
   files use mixed-case (PascalCase) keys
2. **Windows-1252 roundtrip encoding** -- high-byte Windows-1252 characters decoded to
   Unicode are re-encoded as `&#NNN;` NCR entities instead of original byte values

---

## Issue 1: Key Case Normalization

### Root Cause Analysis

**This issue may NOT actually exist in the current code.** After thorough research,
the code already preserves key case correctly through the entire pipeline:

#### Parse Path (from_bytes)
- `ParameterCollection::from_bytes()` (`param_collection.rs:170-211`)
- Keys are decoded from Windows-1252 and stored with **original case** in an `IndexMap`
- `%UTF8%` prefixed keys: the prefix is stripped but original case of the remaining key
  is preserved
- Lookups use `find_key()` which does case-insensitive comparison but returns the
  **original-case** key string
- The C# `ParamList` constructor (`ParamList.cs:15-20`) similarly stores keys with
  original case in a `Dictionary<string, string>` with `OrdinalIgnoreCase` comparer

#### Serialize Path (to_bytes)
- `ParameterCollection::to_bytes()` (`param_collection.rs:105-145`)
- Iterates `self.params` (IndexMap) and outputs keys verbatim -- no uppercasing
- The `%UTF8%` dual-write also uses the key as-is (`param_collection.rs:124`)

#### Constants
- All key constants in `altium-format-types/src/constants/` use correct mixed case:
  - `OWNER_INDEX = "OwnerIndex"` (not `"OWNERINDEX"`)
  - `ALL_PIN_COUNT = "AllPinCount"` (not `"ALLPINCOUNT"`)
  - `LIB_REFERENCE = "LibReference"` (not `"LIBREFERENCE"`)
  - etc.
- The derive macro `ToParams` uses `#[param(key = CONSTANT)]` which embeds the
  constant's string value directly -- no case transformation occurs

#### C# Reference Behavior
- `SchDataSerializerParam.SetParameter()` (`SchDataSerializer.cs:1256-1265`) stores
  keys with original case (only trims whitespace, no `.ToUpper()`)
- `ParamList.ToRawString()` (`ParamList.cs:79-113`) outputs keys as-is when
  `deleteNewLineChars` is false
- The standard SchLib/SchDoc binary serialization path
  (`SchDataSerializerParam.cs:224`) calls `GetParamsAsBytes(deleteNewLineChars: false)`,
  preserving original case
- Only `SchDataSerializerParamAscii` (ASCII export, not binary CFB) uppercases keys
  (`ParamList.cs:89`)
- The separate `StrUtils.SetParameterValue()` (`StrUtils.cs:131-142`) does `.ToUpper()`
  but this is a utility for string-based parameter manipulation, not the CFB serializer

**Conclusion**: If key case normalization issues ARE being observed in semantic diffs, the
most likely cause is that our Rust constants have incorrect casing for specific keys.
Cross-check any failing keys against the C# `FileFormatV5.cs` Export methods to verify
the exact string literal Altium uses.

### Verification Steps

To verify whether this issue actually exists:
```bash
# Run semantic diff on a known-failing file
altium cfb diff --semantic --verbose original.SchLib roundtripped.SchLib

# Look for UpdatedParamValues or MissingParamPair issues that differ only in case
# e.g. "AllPinCount=4" vs "ALLPINCOUNT=4"
```

If case issues ARE found, fix the corresponding constant in
`crates/altium-format-types/src/constants/`.

---

## Issue 2: Windows-1252 Roundtrip Encoding (NCR Entities)

### Root Cause

`encoding_rs::WINDOWS_1252.encode()` follows the WHATWG encoding standard, which
specifies that **unmappable Unicode characters** (those not in the Windows-1252 codepage)
are replaced with **decimal Numeric Character References** (NCR) like `&#8220;` instead
of a replacement byte.

This causes roundtrip corruption when:
1. A value is read from a `%UTF8%`-prefixed parameter (decoded as UTF-8)
2. The value contains Unicode characters that DO NOT exist in Windows-1252
3. The value is re-serialized with `to_bytes()`, which calls
   `encoding_rs::WINDOWS_1252.encode()` on the Win-1252 version
4. The unmappable characters become `&#NNN;` NCR entities in the output bytes

**Example**: If a `%UTF8%` value contains CJK characters (U+4E16 etc.), the UTF-8
version is correct, but the mandatory Win-1252 fallback version will contain NCR
entities instead of the original garbled bytes.

### How Altium Handles It (C# Reference)

In `ParamList.AddStringToByteListWithReplace()` (`ParamList.cs:36-77`):
1. Iterates character by character
2. Special-cases `\u008E`, `\u00A6`, and `|`
3. For all other chars: `DXP.Utils.EncodingDefault.GetBytes(new char[1] { c })`
4. .NET's Windows-1252 `Encoding.GetBytes()` replaces unmappable chars with `?` (0x3F)

**Key difference**: Altium produces `?` for unmappable chars; our code produces
`&#NNN;` NCR entities. The `?` is the correct behavior for the Win-1252 fallback
version because:
- The Win-1252 version is the fallback for parsers that don't understand `%UTF8%`
- The `%UTF8%` version (emitted first) carries the true Unicode data
- A `?` in the Win-1252 version is tolerable; NCR entities are not (they're multi-byte
  ASCII sequences that corrupt the value length and content)

### Code Flow

```
In-memory value (Rust String, UTF-8)
    |
    v
to_bytes() [param_collection.rs:105-145]
    |
    +--> For values with chars > '~': emit %UTF8% version first (UTF-8 bytes, correct)
    |
    +--> ALWAYS emit Win-1252 version:
         escape_for_win1252() -> encoding_rs::WINDOWS_1252.encode()
                                  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
                                  THIS produces &#NNN; for unmappable chars
```

### Recommended Fix

Replace `encoding_rs::WINDOWS_1252.encode()` in `to_bytes()` with a custom encoder
that replaces unmappable characters with `?` (0x3F), matching Altium's .NET behavior.

The simplest approach: use `encoding_rs::WINDOWS_1252.new_encoder()` with the
`encode_from_utf8()` method in `EncoderResult::Unmappable` mode, substituting `?`
for each unmappable character. Alternatively, implement a direct char-by-char
encoder:

```rust
fn encode_win1252_with_fallback(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    for c in s.chars() {
        // Try to encode as Windows-1252
        let mut buf = [0u8; 1];
        let (result, _, _) = encoding_rs::WINDOWS_1252.encode(&c.to_string());
        if result.len() == 1 && !result.starts_with(b"&#") {
            out.push(result[0]);
        } else {
            // Unmappable: use '?' like .NET Encoding.GetBytes()
            out.push(b'?');
        }
    }
    out
}
```

However, a more efficient approach uses the `encoding_rs` encoder API properly:

```rust
fn encode_win1252_lossy(s: &str) -> Vec<u8> {
    let mut encoder = encoding_rs::WINDOWS_1252.new_encoder();
    let mut out = vec![0u8; s.len() * 2]; // generous allocation
    let mut total = 0;
    let mut src = s;
    loop {
        let (result, read, written, _) =
            encoder.encode_from_utf8_without_replacement(src, &mut out[total..], true);
        total += written;
        match result {
            encoding_rs::EncoderResult::InputEmpty => break,
            encoding_rs::EncoderResult::OutputFull => {
                out.resize(out.len() * 2, 0);
            }
            encoding_rs::EncoderResult::Unmappable(_) => {
                out[total] = b'?';
                total += 1;
            }
        }
        src = &src[read..];
    }
    out.truncate(total);
    out
}
```

**Where to apply this fix**:
- `param_collection.rs:131` -- key encoding
- `param_collection.rs:134` -- value encoding
- Any other call site that uses `encoding_rs::WINDOWS_1252.encode()` for serialization

**Note**: This ONLY affects the Win-1252 fallback version. The `%UTF8%` version
(emitted first for values with non-ANSI chars) correctly preserves all Unicode data.
The fix ensures that the Win-1252 version degrades gracefully with `?` instead of
expanding into multi-byte NCR entities.

### Impact on Semantic Diff

The NCR entities would show up as `UpdatedParamValues` in semantic diffs where:
- Original file has `%UTF8%` + Win-1252 with raw high bytes
- Roundtripped file has `%UTF8%` + Win-1252 with `&#NNN;` entities

The semantic diff parser reads the `%UTF8%` version first (if present), so the
**semantic value** may still match. But the raw bytes differ, causing byte-level
diff failures and potentially `MissingParamPair`/`UpdatedParamValues` if the diff
tool compares both versions independently.

---

## Summary of Code Locations

| File | Lines | Role |
|------|-------|------|
| `param_collection.rs` | 17-21 | `ParameterCollection` struct (IndexMap, original-case keys) |
| `param_collection.rs` | 105-145 | `to_bytes()` -- serialization to Win-1252 with %UTF8% |
| `param_collection.rs` | 170-211 | `from_bytes()` -- parsing from Win-1252 with %UTF8% |
| `param_collection.rs` | 473-479 | `find_key()` -- case-insensitive lookup |
| `param_collection.rs` | 513-517 | `escape_for_win1252()` -- pipe/Ž escaping |
| `altium-format-derive/src/lib.rs` | 69-104 | `ToParams` derive -- uses key constants verbatim |
| `altium-format-types/src/constants/` | various | Key string constants (PascalCase) |
| C# `ParamList.cs` | 36-77 | `AddStringToByteListWithReplace` -- reference encoder |
| C# `ParamList.cs` | 79-113 | `ToRawString` -- reference serializer |
| C# `StrUtils.cs` | 210-262 | `ParseWideUtfData` -- reference parser |
| C# `SchDataSerializer.cs` | 1256-1265 | `SetParameter` -- reference key storage |
| C# `SchDataSerializerParam.cs` | 224 | Binary CFB serialization entry point |

---

## Priority

- **Windows-1252 NCR encoding**: HIGH -- causes real byte-level corruption
- **Key case normalization**: LOW/VERIFY -- may not actually be an issue in current code;
  verify with semantic diff before investing effort
