# SchLib Text Encoding Investigation

## Problem

Our SchLib roundtrip produces incorrect parameter text encoding. The `to_bytes()` method
in `ParameterCollection` does not match Altium's encoding behavior, causing content
mismatches in streams that contain non-ASCII text (e.g., `µ` in "µF").

## Three Encoding Differences

### 1. Pipe Escape Mechanism

**Ours**: `[` and `]` bracket escaping (replaces `|` with `[]`)
**Altium**: Uses `Ž` (byte 0x8E in Windows-1252) as the pipe escape character

In Win1252 context, pipes in values are replaced with byte 0x8E.
In UTF-8 context, pipes in values are replaced with `¦` (U+00A6, broken bar).

### 2. Equals Sign Escaping

**Ours**: `{` and `}` bracket escaping (replaces `=` with `{}`)
**Altium**: Does NOT escape `=` at all — the parser splits on the FIRST `=` only

So `|Text=x=y|` is parsed as key=`Text`, value=`x=y`. No escaping needed.

### 3. Missing %UTF8% Dual-Write

**Ours**: Writes only `|KEY=VALUE|` with bracket escaping
**Altium**: When a value contains non-ASCII characters, writes BOTH:
```
|%UTF8%KEY=VALUE_UTF8|||KEY=VALUE_WIN1252|
```

The `%UTF8%` prefixed version contains the UTF-8 encoded value.
The un-prefixed version contains the Windows-1252 fallback (with lossy encoding).
Both are written for backwards compatibility with older Altium versions.

## C# Investigation Results

All findings below are from the decompiled Altium C# source in `AD26-dotnet/`.

### Answer 1: Pipe Escape Character

**Confirmed**: The pipe escape character is `0x8E` (decimal 142), defined as:

```csharp
// AD26-dotnet/Altium.Edp.Interfaces/Rt_Schematic/Consts.cs:19-29
public const char C_SCH_SPECIAL_DELIMITER_CHAR = '\u008e';   // 142
public const int C_SCH_SPECIAL_DELIMITER_CHAR_ORD = 142;
public const char C_SCH_VERTICAL_BAR = '|';                  // 124
public const int C_SCH_VERTICAL_BAR_ORD = 124;
public const char C_SCH_BROKEN_BAR = '¦';                    // U+00A6 = 166
public const int C_SCH_BROKEN_BAR_ORD = 166;
```

Also confirmed by BOM module:
```csharp
// AD26-dotnet/Altium.BOM.Contracts/Altium.BOM/Consts.cs:5-7
public const char ParameterDelimiter = '|';
public const char SubParameterDelimiter = '¦';   // broken bar = pipe-in-value
```

There are **two distinct escape modes** used in different contexts:

#### Mode A: "Double Escape" (CFB binary container files — SchDoc/SchLib)

Used when `doubleEscape=true` is passed to `ParamList.ToRawString()` and
`AddStringToByteListWithReplace()`.

**Writing** (`ParamList.AddStringToByteListWithReplace`, ParamList.cs:36-77):
```csharp
private void AddStringToByteListWithReplace(StringBuilder data, List<byte> list, bool doubleEscape)
{
    for (int i = 0; i < data.Length; i++)
    {
        char c = data[i];
        switch (c)
        {
        case '\u008e':   // If the char is already 0x8E (or ¦)...
        case '¦':
            list.Add((byte)142);  // write byte 0x8E
            continue;
        case '|':
            list.Add((byte)124);  // write raw pipe byte (NOT escaped here —
            continue;             // the escape happens in the doubleEscape branch below)
        }
        // Encode char to ACP bytes, then process each byte
        byte[] bytes = DXP.Utils.EncodingDefault.GetBytes(new char[1] { c });
        if (doubleEscape)
        {
            foreach (byte b in bytes)
            {
                switch (b)
                {
                case 142:        // 0x8E in encoded output
                    list.Add((byte)142);  // double it: 0x8E 0x8E = literal 0x8E
                    list.Add((byte)142);
                    break;
                case 124:        // 0x7C (pipe) in encoded output
                    list.Add((byte)142);  // replace with single 0x8E = escaped pipe
                    break;
                default:
                    list.Add(b);
                    break;
                }
            }
        }
        else
        {
            list.AddRange(bytes);
        }
    }
}
```

Note the flow: first the switch handles `\u008E`, `¦`, and `|` as .NET chars directly.
Then for all other chars, the char is encoded via `EncodingDefault` (ACP) to bytes, and
in doubleEscape mode each *byte* is checked:
- Byte `0x7C` (pipe) → replaced with single `0x8E` (escaped pipe)
- Byte `0x8E` → doubled to `0x8E 0x8E` (literal 0x8E)

**Reading** (`StrUtils.ReplaceSpecialDelimiterChars`, StrUtils.cs:345-390):
- Single `0x8E` byte → pipe `|` (byte 124)
- Double `0x8E 0x8E` → literal `0x8E` byte
  - UNLESS the data contains `@DESIGNATOR` or `@"DESIGNATOR`, in which case
    `0x8E 0x8E` → `||` (two pipe bytes) — a special Altium designator edge case

#### Mode B: "Broken Bar" (in-memory parameter store / non-doubleEscape contexts)

**Writing** (`SchDataSerializer.GetSafeParamValue`, SchDataSerializer.cs:1277-1301):
```csharp
protected string GetSafeParamValue(string value, bool keepUnchanged)
{
    // ...
    switch (value[i])
    {
    case '|':
        ReplaceChar(ref str, num, '¦');   // pipe → broken bar (U+00A6)
        break;
    case '\u008e':
        if (!keepUnchanged)
            InsertChar(ref str, num, '\u008e');  // double the 0x8E
        break;
    }
}
```

Also `StrUtils.ReplaceSpecialParameterChars` (StrUtils.cs:169-195):
- `|` → `¦` (broken bar)
- `\u008E` → `\u008E\u008E` (doubled)

**Reading** (`StrUtils.ProcessMBCSString`, StrUtils.cs:56-85):
- `\u008E\u008E` → single `\u008E`
- single `\u008E` → `|`
- `¦` → `|`

### Answer 2: Equals Sign — Never Escaped

**Confirmed**: `=` is NEVER escaped. The parser always splits on the first `=`.

High-level parser (`DXP/ParameterList.cs:40-50`):
```csharp
int num = text.IndexOf("=", StringComparison.OrdinalIgnoreCase);  // FIRST =
string name = text.Substring(0, num);        // everything before first =
string value = text.Substring(num + 1);      // everything after (may contain more =)
```

Low-level byte parser (`StrUtils.ReadTill`, StrUtils.cs:294-331):
```csharp
// For keys: ReadTill stops at byte 61 ('=')
string text = ReadTill(ref index, data, dataLength, 61, ...);
// For values: ReadTill stops at byte 124 ('|')
string text2 = ReadTill(ref index, data, dataLength, 124, ...);
```

The key stops at the first `=`, and the value stops at the next `|`, so `=` signs
within the value are naturally included. No escaping needed or implemented.

### Answer 3: %UTF8% Dual-Write Trigger Condition

**The trigger function** (`StrUtils.HasNonAnsiSymbols`, StrUtils.cs:144-147):
```csharp
public static bool HasNonAnsiSymbols(string data)
{
    return data.Any(c => c > '~' && c != '\u008e');
}
```

The condition is: any character in the **value** (not key) has code point `> 0x7E`
AND is not `\u008E` (the escape char itself).

**Important**: The check is on the **in-memory .NET string value** (before byte encoding),
not on the encoded bytes. The in-memory string still contains `¦` (broken bar, U+00A6)
for escaped pipes (see `GetSafeParamValue` above). Since `¦` (U+00A6 = 166) is `> '~'`
(0x7E = 126), **a value containing escaped pipes DOES trigger the dual-write**.

This explains Example 3 from the diff: the value
`?"INITIAL VOLTAGE"¦IC=@"INITIAL VOLTAGE"¦` contains `¦` chars (from pipe escaping),
and `¦` > `~`, so `HasNonAnsiSymbols` returns true even though the original text is
pure ASCII.

The check is **per parameter key-value pair**, not per record.

**Additional trigger in SchDataExporterLibraryV5** (SchDataExporterLibraryV5.cs:712-722):
```csharp
if (value.Length < 254)
    return StrUtils.HasNonAnsiSymbols(value);
return true;  // values >= 254 chars ALWAYS trigger dual-write
```

### Answer 4: UTF-8 Version Encoding

The UTF-8 version is written as (`ParamList.ToRawString`, ParamList.cs:92-94):
```csharp
if (StrUtils.HasNonAnsiSymbols(value))
{
    result.AddRange(Encoding.UTF8.GetBytes($"|{"%UTF8%"}{name}={value.Trim()}||"));
}
```

The entire string `|%UTF8%KEY=VALUE||` is encoded as raw UTF-8 bytes via
`Encoding.UTF8.GetBytes()`. The value is `.Trim()`'d.

For the pipe escape in the UTF-8 version: the in-memory value already has `¦`
(broken bar) in place of `|` (from `GetSafeParamValue`), so when UTF-8 encoded,
`¦` (U+00A6) becomes bytes `C2 A6`. This matches what we see in the hex dumps.

### Answer 5: Win1252 Fallback — Unmappable Characters

The Win1252 version uses `DXP.Utils.EncodingDefault.GetBytes()` per character.
On .NET, `Encoding.GetEncoding(codepage)` returns an encoding with the default
`EncoderReplacementFallback("?")`. **Unmappable characters are replaced with `?`**
(byte 0x3F).

For characters that ARE in the ACP (like `µ` = Win1252 byte 0xB5), they encode
correctly. For characters NOT in the ACP (like CJK, emoji, etc.), they become `?`.

### Answer 6: `[]` and `{}` Bracket Escaping — Not Used by Altium

**Confirmed**: Neither `[]` bracket escaping for pipes nor `{}` bracket escaping for
equals is used ANYWHERE in the Altium C# source code.

These are purely third-party inventions from the `altium-rs` crate we forked from.
They must be replaced with the correct escape mechanisms.

### Answer 7: Exact Byte Sequence for Dual-Write

The format is `|%UTF8%KEY=VALUE_UTF8_TRIMMED|||KEY=VALUE_ACP`:

1. UTF-8 portion: `|%UTF8%KEY=VALUE_TRIMMED||` (trailing `||` = two pipes)
2. ACP portion: `|KEY=VALUE_ACP` (leading `|`)

Concatenated: three pipes `|||` appear between the UTF-8 value and the ACP key.

The `||` trailing the UTF-8 portion is explicit in the format string. The `|` leading
the ACP portion comes from the normal parameter serialization code that always prepends
`|` before each key.

The trailing `||` is NOT an empty parameter — it's just the way the format string is
constructed. The parser handles this via `canSkipLeading: true` on key reads, which
skips consecutive `|` bytes.

## Encoding: Win1252 vs System ACP

### The Code Uses System ACP, NOT Hardcoded Win1252

**Critical finding**: Altium does NOT hardcode Windows-1252. The encoding is the
**Windows Active Code Page (ACP)**, which is system-locale-dependent:

```csharp
// AD26-dotnet/Altium.Dxp.Classes/DXP/Utils.cs:75-77
public static Encoding EncodingDefault => EncodingACP;
public static Encoding EncodingACP => encodingACP.Value;

// Utils.cs:1521-1527
[DllImport("kernel32.dll")]
private static extern int GetACP();

private static Encoding GetEncodingACP()
{
    return Encoding.GetEncoding(GetACP());
}
```

`kernel32.dll`'s `GetACP()` returns the system's default ANSI code page:
- **1252** (Windows-1252) on Western European/US systems
- **932** (Shift-JIS) on Japanese systems
- **936** (GBK) on Chinese Simplified systems
- **949** (EUC-KR) on Korean systems
- **950** (Big5) on Chinese Traditional systems
- etc.

### What This Means for Our Implementation

For **roundtrip correctness with files created on Western systems** (the vast majority
of Altium files), Win1252 is correct. But strictly speaking:

1. The "ANSI" encoding in the ACP fallback portion is locale-dependent.
2. Files created on a Japanese Windows system would use Shift-JIS for the non-`%UTF8%`
   portion.
3. The `%UTF8%` dual-write exists precisely because of this — the UTF-8 version is
   locale-independent and authoritative, while the ACP version is a best-effort
   fallback for older Altium versions that don't understand `%UTF8%`.
4. The `HasNonAnsiSymbols` check (`c > '~'`) is intentionally conservative — it
   triggers on ANY non-ASCII character, not just "non-Win1252" characters. This means
   even characters that ARE representable in Win1252 (like `µ` = 0xB5) get the `%UTF8%`
   treatment, ensuring cross-locale portability.

### Practical Decision for Our Rust Code

**Using Win1252 is the correct choice for our implementation** because:
- It matches the overwhelmingly common case (Western-locale Altium installations)
- We always write the `%UTF8%` version for non-ASCII values, so the Win1252 portion
  is just a backward-compat fallback
- The `%UTF8%` version is authoritative on read (the Win1252 version is ignored if
  `%UTF8%` exists)
- We use `encoding_rs::WINDOWS_1252` which correctly maps all 256 byte values

The only case where this could matter is if someone creates a file on a Japanese
Windows system (ACP=932) and our roundtrip produces Win1252 bytes for the fallback
portion. But since the `%UTF8%` version is always present for non-ASCII values,
modern Altium versions would read the UTF-8 version anyway.

## Data Flow: Write Path (Serialization)

For each parameter key-value pair in `ParamList.ToRawString`:

```
                     ┌─────────────────────────────────────────────────────┐
                     │ In-memory: key="TEXT", value="0.1µF"               │
                     │ (pipes already replaced with ¦ by GetSafeParamValue)│
                     └─────────────┬───────────────────────────────────────┘
                                   │
                    ┌──────────────▼──────────────┐
                    │ HasNonAnsiSymbols(value)?    │
                    │ Any char > 0x7E and != 0x8E? │
                    └──────┬──────────────┬────────┘
                      YES  │              │  NO
                           ▼              │
          ┌────────────────────────┐      │
          │ Emit UTF-8 version:    │      │
          │ Encoding.UTF8.GetBytes │      │
          │ ("|%UTF8%TEXT=0.1µF||")│      │
          │ value is .Trim()'d     │      │
          └────────┬───────────────┘      │
                   │                      │
                   │    ┌─────────────────┘
                   │    │
                   ▼    ▼
          ┌────────────────────────┐
          │ ALWAYS emit ACP version│
          │ via AddStringToByteList│
          │ WithReplace:           │
          │ "|TEXT=0.1µF"          │
          │ (encoded per-char via  │
          │  EncodingDefault/ACP)  │
          │ (doubleEscape: 0x7C→   │
          │  0x8E, 0x8E→0x8E 0x8E)│
          └────────────────────────┘
```

Result bytes: `|%UTF8%TEXT=0.1µF|||TEXT=0.1µF|` where:
- `µ` in UTF-8 portion = bytes `C2 B5`
- `µ` in ACP portion = byte `B5`

## Data Flow: Read Path (Deserialization)

```
          ┌────────────────────────────┐
          │ Raw bytes from CFB stream  │
          └─────────────┬──────────────┘
                        │
            ┌───────────▼──────────────┐
            │ Scan for %UTF8% bytes    │
            │ in the raw data          │
            └───┬──────────────┬───────┘
             FOUND          NOT FOUND
                │               │
                ▼               ▼
    ┌───────────────────┐ ┌───────────────────┐
    │ ParseWideUtfData  │ │ ParseWideNoUtfData│
    │                   │ │ (simple ACP path) │
    │ For each key:     │ └───────────────────┘
    │ - Read key (ACP)  │
    │ - Check %UTF8%    │
    │   prefix          │
    │ - If %UTF8%: read │
    │   value as UTF-8  │
    │ - Else: read as   │
    │   ACP             │
    │                   │
    │ Duplicates:       │
    │ - UTF-8 version   │
    │   ALWAYS wins     │
    │ - ACP duplicate   │
    │   is discarded    │
    └───────────────────┘
```

## Edge Cases

### Trailing escape byte
If the last character of a value is `\u008E` or `¦`, an extra `|` is appended as
a trailer to prevent the escape byte from being misinterpreted at the boundary:
```csharp
// ParamList.cs:101-105
char c = sb[sb.Length - 1];
if (c == '\u008e' || c == '¦')
{
    sb.Append('|');
}
```

### @DESIGNATOR special case
When reading back, if the data contains `@DESIGNATOR` or `@"DESIGNATOR`, the
`0x8E 0x8E` double-escape is interpreted as `||` (two pipes) instead of a literal
`0x8E` byte. This is a special case in `ReplaceSpecialDelimiterChars`
(StrUtils.cs:360-376).

### Value trimming asymmetry
The UTF-8 version applies `.Trim()` to the value. The ACP version does NOT trim.
However, `ParseWideNoUtfData` (the non-UTF8 read path) trims the value after reading
(`text2 = text2.Trim()` at StrUtils.cs:275-276), while `ParseWideUtfData` does NOT
trim — the UTF-8 value was already trimmed on write.

## Concrete Examples from Synthiam.SchLib Diff

83 out of ~200 streams differ. All differences fall into two categories:
1. Missing `Text=*` (skip_default issue, separate from encoding — Phase 2)
2. Text encoding mismatches (this investigation)

### Example 1: Non-ASCII character (`µ` in "µF")

Stream `/Cap/Data`, block 12. Hex dump of the original:

```
# %UTF8% version: µ encoded as UTF-8 bytes C2 B5
00000040: 7c 25 55 54 46 38 25 54  65 78 74 3d 30 2e 31 c2  |%UTF8%Text=0.1.
00000050: b5 46 7c 7c 7c 54 65 78  74 3d 30 2e 31 b5 46 7c  .F|||Text=0.1.F|
#                                  ^^^                  ^^
#                          Win1252 version: µ as single byte B5
```

Decoded: `|%UTF8%Text=0.1µF|||Text=0.1µF|`
- `%UTF8%Text` value: `0.1` + `C2 B5` (UTF-8 µ) + `F`
- `Text` value: `0.1` + `B5` (Win1252 µ) + `F`

Our output: `|Text=0.1µF|` — single version, no `%UTF8%` dual-write.

### Example 2: Pipe escape character (`Ž` / `¦`)

Stream `/Cap/Data`, block 24. This value contains literal `|` pipes in the text that
need escaping. Hex dump of the original:

```
# %UTF8% version: pipe escaped as ¦ (UTF-8: C2 A6)
00000080: 54 41 47 45 22 c2 a6 49  43 3d 40 22 49 4e 49 54  TAGE"..IC=@"INIT
00000090: 49 41 4c 20 56 4f 4c 54  41 47 45 22 c2 a6 7c 7c  IAL VOLTAGE"..||
#                                                      ^^^^
#                                  end of %UTF8% value -+  +- start of Win1252 value
000000a0: 7c 54 65 78 74 3d 40 44  45 53 49 47 4e 41 54 4f  |Text=@DESIGNATO
...
# Win1252 version: pipe escaped as Ž (byte 8E)
000000d0: 22 8e 49 43 3d 40 22 49  4e 49 54 49 41 4c 20 56  ".IC=@"INITIAL V
000000e0: 4f 4c 54 41 47 45 22 8e  7c 7c 4e 61 6d 65 3d 4e  OLTAGE".||Name=N
#                              ^^
#                     Ž = byte 8E (pipe escape in Win1252 context)
```

Decoded:
```
|%UTF8%Text=...?"INITIAL VOLTAGE"¦IC=@"INITIAL VOLTAGE"¦|||Text=...?"INITIAL VOLTAGE"ŽIC=@"INITIAL VOLTAGE"Ž||Name=Netlist|...
```

Our output:
```
|Text=...?"INITIAL VOLTAGE"[]IC{}@"INITIAL VOLTAGE"[]|Name=Netlist|...
```

Note all three encoding differences visible:
1. `[]` instead of `Ž`/`¦` for pipe escape
2. `{}` instead of nothing for `=` escape (Altium has bare `IC=@"..."`)
3. Single version instead of `%UTF8%` + Win1252 dual-write

### Example 3: Pipe escape alone triggers dual-write

The value `?"INITIAL VOLTAGE"ŽIC=@"INITIAL VOLTAGE"Ž` is pure ASCII except for the
`Ž` escape chars (byte 0x8E). The ONLY reason this triggers `%UTF8%` dual-write is
that the in-memory value contains `¦` (broken bar, U+00A6 > 0x7E), which passes the
`HasNonAnsiSymbols` check. This confirms the trigger is on the **in-memory escaped
value** (where pipes are represented as `¦`), not the raw original text.

## Debugging Commands

Reproduce the roundtrip and inspect differences:

```bash
# 1. Roundtrip the file
cargo run -- save-as data/Synthiam.SchLib /tmp/synthiam_rt.SchLib

# 2. Full diff — shows all 83 differing streams with block-level detail
cargo run -- cfb diff data/Synthiam.SchLib /tmp/synthiam_rt.SchLib --blocks

# 3. Focus on a single stream (Cap/Data has both µ and pipe-escape examples)
cargo run -- cfb diff data/Synthiam.SchLib /tmp/synthiam_rt.SchLib --blocks --stream "/Cap/Data"

# 4. Hex dump a specific block to see raw bytes
#    Block 12 = µF example (non-ASCII char, %UTF8% dual-write)
cargo run -- cfb blocks data/Synthiam.SchLib "/Cap/Data" --block 12
#    Block 24 = pipe escape example (Ž in Win1252, ¦ in UTF-8)
cargo run -- cfb blocks data/Synthiam.SchLib "/Cap/Data" --block 24

# 5. Compare a component with only skip_default diffs (no encoding)
cargo run -- cfb diff data/Synthiam.SchLib /tmp/synthiam_rt.SchLib --blocks --stream "/AL8860/Data"

# 6. Compare a component with pipe-escape-only diffs (no µ)
cargo run -- cfb diff data/Synthiam.SchLib /tmp/synthiam_rt.SchLib --blocks --stream "/2N3904/Data"

# 7. See just the list of differing streams (grep-friendly)
cargo run -- cfb diff data/Synthiam.SchLib /tmp/synthiam_rt.SchLib --blocks 2>&1 | grep "stream DIFFERS"

# 8. Run the roundtrip test
cargo test --lib schlib::tests::roundtrip_synthiam_schlib
```

## Key Source Files

| File | Role |
|------|------|
| `AD26-dotnet/Altium.Edp.Interfaces/Rt_Schematic/Consts.cs` | Named constants (`C_SCH_SPECIAL_DELIMITER_CHAR`, `C_SCH_UTF8_PREFIX`, etc.) |
| `AD26-dotnet/Altium.Sch.DataModel/.../ParamList.cs` | **Write path**: `ToRawString()` (UTF-8 + ACP dual-write), `AddStringToByteListWithReplace()` (doubleEscape logic) |
| `AD26-dotnet/Altium.Sch.DataModel/.../StrUtils.cs` | **Read path**: `ParseWideData()` → `ParseWideUtfData()`/`ParseWideNoUtfData()`, `ReadTill()`, `ReplaceSpecialDelimiterChars()`, `HasNonAnsiSymbols()` |
| `AD26-dotnet/Altium.Sch.DataModel/.../SchDataSerializer.cs` | `GetSafeParamValue()` (pipe → broken bar replacement in memory) |
| `AD26-dotnet/Altium.Dxp.Classes/DXP/Utils.cs` | `EncodingDefault` / `EncodingACP` / `GetACP()` — encoding selection |
| `AD26-dotnet/Altium.Dxp.Classes/DXP/ParameterList.cs` | High-level `IndexOf("=")` first-equals splitting |
| `AD26-dotnet/Altium.BOM.Contracts/Altium.BOM/Consts.cs` | `ParameterDelimiter` / `SubParameterDelimiter` constants |
