# SchLib Roundtrip Diff Summary

## 1. Overview

| Metric | Count |
|--------|-------|
| Total files tested | 99 |
| Successful save-as | 93 |
| Failed save-as | 6 |
| Success rate | 93.9% |

After a fix for CFB storage path special character encoding, 5 previously-failing files now succeed (aiskylab-Memory, aiskylab-Resistors, JLCPCB-RES, Parts_Library, SMotlaq-Schem_lib). Custom.SchLib also had its CFB path issue fixed but now fails at a later stage with a Pascal string length panic.

All 93 successful roundtrips produce files with a larger CFB container size than the original. No file achieved a byte-identical roundtrip.

## 2. Failures

### 2a. CFB Path Issues with Non-ASCII Characters (2 files)

These files contain component names with non-ASCII characters (Cyrillic, accented Latin) in the CFB storage path. The parser cannot find the storage because the component name read from the FileHeader does not match the actual CFB storage name after character encoding/mapping.

| File | Component Name | Problematic Character | Error |
|------|---------------|----------------------|-------|
| CeleronLib-Connect.SchLib | Cyrillic component names (e.g., "Klemmnik razjomnyj uglovoj 6P") | Non-ASCII / Cyrillic characters | No such storage: "/Klemmnik razjomnyj uglovoj 6P" |
| CeleronLib.SchLib | `SW-PB-N` + U+00D1 (N with tilde) | Non-ASCII (U+00D1) | No such storage: "/SW-PB-N..." |

### 2b. Unknown Parameter Errors (1 file)

| File | Error |
|------|-------|
| TinyShuffle.SchLib | parsing component 'CAPC1608N-0.1UF': record #13 in Data stream: RECORD=34 (SchDesignator): Unknown parameters remaining: ["ShowName"] |

This file contains a `ShowName` parameter on a SchDesignator record (RECORD=34) that the parser does not recognize.

### 2c. Missing Parameter Errors (1 file)

| File | Error |
|------|-------|
| IC_chips.SchLib | Missing required parameter: Size6 |

This file has a font table referencing `Size6` but the parameter is absent from the FileHeader. Version information could not be extracted due to the parse failure.

### 2d. Pascal String Length Panic (2 files)

| File | String Length | Error |
|------|-------------|-------|
| Custom.SchLib | 346 bytes | Pascal string too long: 346 bytes (max 255) |
| ryankurte-electronpowered.SchLib | 336 bytes | Pascal string too long: 336 bytes (max 255) |

These files contain Pascal string fields exceeding the 255-byte maximum, causing a panic in `binary_io.rs:389`. Custom.SchLib previously failed with a CFB path issue (`:` in component name); after the fix, it now parses successfully but panics during save when writing a long string. ryankurte-electronpowered.SchLib (version 5.0, minor version 2) hits the same panic.

## 3. Version Information

All successfully-parsed files report the same header: `Protel for Windows - Schematic Library Editor Binary File Version 5.0`. The minor version varies. Files with minor version < 9 are older and may exhibit parameter key casing differences on roundtrip (older versions used ALL_CAPS keys; the current serializer writes MixedCase keys). Minor version 9 is the latest format version.

| Minor Version | Count | Files |
|---------------|-------|-------|
| 1 | 7 | arthurbenemann-STM32F1, arthurbenemann-STM32F2, arthurbenemann-STM32F3, arthurbenemann-STM32F4, arthurbenemann-STM32L1, General_IC, Synthiam |
| 2 | 60 | 88W8801, aiskylab-Ceramics, aiskylab-Connectors-USB, aiskylab-Electromech, aiskylab-FFC-FPC, aiskylab-Headers-Wire, aiskylab-Inductors, aiskylab-Linear, aiskylab-MCUs, aiskylab-MemoryCon, aiskylab-Memory, aiskylab-Optoelectronics, aiskylab-PowerManage, aiskylab-Resistors, aiskylab-Sensors, aiskylab-Transistors, Arduino, arthurbenemann-Atmel, arthurbenemann-Connectors, arthurbenemann-Diode, arthurbenemann-FET, arthurbenemann-Header, arthurbenemann-Other, arthurbenemann-STM32F0, Connectors, Connectors_ZTZ, Custom, DCDC, Diodes, Headers, ICs, JLCPCB-Cap, JLCPCB-Diode, JLCPCB-FBEAD, JLCPCB-IC, JLCPCB-IUD, JLCPCB-LDO, JLCPCB-LED, JLCPCB-MCU, JLCPCB-MOS, JLCPCB-RES, LEDs, LED_ZTZ, MacRover-0022232021, Mechanical, Modules, NEUACTION, Parts_Library, RAM, Resistors_Caps, ryankurte-ATSAMD21G, ryankurte-EFM32GG12B8xx, ryankurte-electronpowered, S32K, Sika_revb, SIM808, Standard, STM32, Switches, Transistors |
| 4 | 9 | ESP32-DEVKITC, MacRover-13POS, ryankurte-ARJM11, vpodlesnyi-Amplifier, vpodlesnyi-Connector, vpodlesnyi-Driver, vpodlesnyi-GOSTAmplifier, vpodlesnyi-GOSTDiode, vpodlesnyi-GOSTInductor |
| 6 | 2 | lucashudson-AlCap, lucashudson-Diode |
| 8 | 1 | foc_schlib |
| 9 (latest) | 16 | aKaReZa75-BoxHeader, aKaReZa75-Capacitor, aKaReZa75-Connector, aKaReZa75-IC, aKaReZa75-Inductor, aKaReZa75-ModSen, aKaReZa75-Other, aKaReZa75-Resistor, aKaReZa75-SemiConductor, aKaReZa75-Switch, ioelectro, lucashudson-Connectors, lucashudson-Espressif, lucashudson-Random, lucashudson-Sensors, SMotlaq-Schem_lib |
| N/A (parse failed) | 4 | CeleronLib-Connect, CeleronLib, IC_chips, TinyShuffle |

Note: 4 of the 6 failures could not report a version because parsing failed before extracting header info. The other 2 failures (Custom at minor version 2, ryankurte-electronpowered at minor version 2) parsed far enough to extract version info but failed during save.

## 4. Diff Analysis

Of the 93 successful roundtrips, all exhibit a CFB container file size increase. The stream-level differences break down as follows.

### 4a. Files with NO Stream Content Differences (15 files)

These files differ ONLY in CFB container file size (larger output). All individual streams match byte-for-byte. Most are minor version 9 (latest) or files that happened to already have correct casing and no encoding edge cases.

| File | Minor Version |
|------|---------------|
| aKaReZa75-BoxHeader.SchLib | 9 |
| aKaReZa75-Capacitor.SchLib | 9 |
| aKaReZa75-Inductor.SchLib | 9 |
| aKaReZa75-Resistor.SchLib | 9 |
| aKaReZa75-SemiConductor.SchLib | 9 |
| aiskylab-MemoryCon.SchLib | 2 |
| JLCPCB-Cap.SchLib | 2 |
| lucashudson-AlCap.SchLib | 6 |
| lucashudson-Connectors.SchLib | 9 |
| lucashudson-Espressif.SchLib | 9 |
| lucashudson-Random.SchLib | 9 |
| MacRover-0022232021.SchLib | 2 |
| MacRover-13POS.SchLib | 4 |
| ryankurte-ARJM11.SchLib | 4 |
| ryankurte-EFM32GG12B8xx.SchLib | 2 |

### 4b. Parameter Key Casing Differences (41 files)

These files exhibit the ALL_CAPS-to-MixedCase key casing pattern (e.g., `LIBREFERENCE` -> `LibReference`, `PARTCOUNT` -> `PartCount`, `OWNERPARTID` -> `OwnerPartId`, `WEIGHT` -> `Weight`). These are all minor version 1 or 2 files where the original was written with uppercase parameter keys. The roundtrip serializer writes the modern MixedCase form.

This is an expected difference for older-version files that undergo auto-upgrade on save.

### 4c. Windows-1252 to HTML Entity Encoding Differences (44 files)

These files contain non-ASCII text (primarily Chinese/CJK characters, but also Cyrillic and accented Latin) that was originally encoded in Windows-1252. The original file stores the Win1252 byte sequence in the non-UTF8 fallback parameter (e.g., `ComponentDescription=ÌùÆ¬µçÈÝ`), while the roundtripped file writes HTML numeric character references (e.g., `ComponentDescription=&#36148;&#29255;&#30005;&#23481;`).

Both representations decode to the same Unicode text. The `%UTF8%`-prefixed parameter (which carries the canonical UTF-8 value) is preserved identically. This difference reflects a change in the non-UTF8 fallback encoding strategy.

### 4d. Degree Symbol Encoding Differences (9 files)

A subset of files shows degree symbol differences: the original stores the degree sign as the Windows-1252 two-byte sequence `0xA1 0xE3` (which displays as `¡ã`), while the roundtripped file writes the actual `°` character (U+00B0). This appears in temperature-related fields like `Text=-30°C`.

Affected files: aiskylab-Connectors-USB, aiskylab-FFC-FPC, aiskylab-Headers-Wire, aiskylab-Linear, aiskylab-MCUs, aiskylab-Memory, aiskylab-Resistors, aiskylab-Sensors, aiskylab-Transistors.

### 4e. Added Default Parameters (60 files)

Many files show the roundtripped output adding default parameters that were absent in the original:

- **`Text=*` on RECORD=34 (SchDesignator)**: The original omitted the `Text` parameter; the serializer adds the default `Text=*`. Seen in 16 files.
- **`Name=Comment` on RECORD=41 (SchComment)**: The original omitted the `Name` parameter; the serializer adds the default `Name=Comment`. Seen in 37 files.

These are semantically correct additions -- Altium treats the absent parameter as having the default value, so our serializer is being explicit about what was implicit.

### 4f. Binary Sidecar Stream Differences (32 files)

These files have differences in binary-encoded sidecar stream blocks. Breakdown by sidecar type:

| Sidecar Stream | Files Affected | Notes |
|---------------|---------------|-------|
| PinSymbolLineWidth | 33 | Binary blocks often same size but different content |
| PinPackageLength | 23 | Binary blocks often same size but different content |
| PinTextData | 11 | Binary blocks, mixed same/different sizes |
| Storage | 9 | Embedded model/footprint data; often different sizes |
| PinWideText | 5 | Binary blocks, often same size |
| PinFunctionData | 3 | Binary blocks, often different sizes |
| PinFrac | 2 | Binary blocks |

Note: A single file can have differences in multiple sidecar types.

### 4g. Summary Table

| Diff Category | File Count |
|---------------|------------|
| No stream differences (file size only) | 15 |
| Parameter key casing (ALL_CAPS -> MixedCase) | 41 |
| Win1252 -> HTML entity encoding (non-ASCII text) | 44 |
| Degree symbol encoding | 9 |
| Added default parameters (Text=*, Name=Comment) | 60 |
| Binary sidecar stream differences | 32 |

Note: Categories overlap -- a single file can exhibit multiple difference types. For example, many files have casing + entity encoding + added parameters + binary sidecar diffs simultaneously.

## 5. Known Issues

### 5a. CFB Storage Path Non-ASCII Character Handling

**Severity: Blocks parsing/saving of affected files entirely.**
**Status: Partially fixed.**

The previous release could not handle `:`, `*`, `!`, and `"` characters in CFB storage paths. These have now been fixed. Two files still fail due to non-ASCII characters (Cyrillic and accented Latin) in component names that do not match the CFB storage path after character encoding.

**Still-failing characters:**
- Non-ASCII / Cyrillic characters -- 1 file (CeleronLib-Connect)
- Non-ASCII / accented Latin (U+00D1) -- 1 file (CeleronLib)

**Fixed characters (no longer failing):**
- `:` (colon) -- was 3 files, now 0
- `*` (asterisk) -- was 2 files, now 0
- `!` (exclamation mark) -- was 1 file, now 0
- `"` (double quote) -- was 1 file, now 0 (TinyShuffle now fails with a different error)

### 5b. Unknown Parameter: ShowName on SchDesignator

**Severity: Blocks parsing of affected files.**

TinyShuffle.SchLib contains a `ShowName` parameter on RECORD=34 (SchDesignator) that the parser does not recognize. This parameter needs to be added to the SchDesignator record type definition.

### 5c. Missing Required Font Parameters

**Severity: Blocks parsing of affected files.**

IC_chips.SchLib references `Size6` in its font table but the parameter is absent, causing a "Missing required parameter" error. This may indicate a corrupted or hand-edited file, or an incomplete font table parser that does not handle sparse font definitions.

### 5d. Pascal String Length Overflow

**Severity: Panic (crash) during save-as. 2 files affected.**

Custom.SchLib (346 bytes) and ryankurte-electronpowered.SchLib (336 bytes) contain Pascal string fields exceeding the 255-byte maximum, causing a panic in `binary_io.rs:389`. The write path panics rather than returning an error. This should be converted to a proper `Result::Err` return, and the underlying data representation should be investigated (it may need to use a different string encoding or block format for long strings).

### 5e. Parameter Key Casing Normalization

**Severity: Cosmetic -- does not affect data integrity.**

Files with minor version < 9 use ALL_CAPS parameter keys (e.g., `LIBREFERENCE`, `PARTCOUNT`). Our serializer always writes MixedCase keys (e.g., `LibReference`, `PartCount`). Since Altium's parser is case-insensitive for parameter keys, this does not cause data loss but does prevent byte-identical roundtrips for older files. This is an expected consequence of version auto-upgrade. Affects 41 of 93 successful files.

### 5f. Non-UTF8 Fallback String Encoding

**Severity: Low -- semantically equivalent but not byte-identical.**

When a parameter has both a `%UTF8%`-prefixed key (canonical Unicode value) and a non-prefixed fallback, the original file stores the fallback as raw Windows-1252 bytes, while our serializer writes HTML numeric character references (e.g., `&#36148;` instead of the Win1252 bytes). Both decode to the same Unicode text. The `%UTF8%` value (which is the authoritative one) is preserved exactly. Affects 44 of 93 successful files.

### 5g. Degree Symbol Encoding in Win1252 Fallback

**Severity: Low -- semantically equivalent but not byte-identical.**

The degree sign (U+00B0) is encoded as the two-byte Win1252 sequence `0xA1 0xE3` in the original files, but our serializer writes the actual `°` character. This is related to the non-UTF8 fallback encoding issue above. Affects 9 files.

### 5h. Added Default Parameters

**Severity: Low -- semantically equivalent, adds explicit defaults.**

Our serializer writes explicit default values for parameters that Altium treats as implicit when absent:
- `Text=*` on SchDesignator (RECORD=34) records
- `Name=Comment` on SchComment (RECORD=41) records

These additions are semantically correct (Altium would read the same defaults) but produce non-identical roundtrips. Affects 60 of 93 successful files.

### 5i. CFB Container File Size Inflation

**Severity: Cosmetic -- all 93 successful roundtrips produce larger files.**

Every roundtripped file is larger than the original. This is a property of the CFB writing implementation (likely due to different sector allocation strategy or minimum file size). The stream content is preserved; only the container overhead increases.

### 5j. Binary Sidecar Stream Differences

**Severity: Needs investigation.**

32 files show differences in binary-encoded sidecar streams (PinSymbolLineWidth, PinPackageLength, PinTextData, Storage, PinWideText, PinFunctionData, PinFrac). Some binary blocks differ only in content (same size), while others differ in both size and content. The Storage stream differences may relate to embedded model/footprint data serialization. The pin sidecar differences may relate to field encoding, padding, or default value handling.
