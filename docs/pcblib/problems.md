# PcbLib: Open Problems

Current state: **34/40 files pass** validation (85%). 6 remaining failures.
Roundtrip: all 34 save successfully, but **all have semantic diff issues** (see below).

Last verified: 2026-03-01

---

## Validation Failures (6 files)

### Problem 1: TextKind=3 invalid enum — 1 file

**File:** `amiryeg-Module-MISC.PcbLib`

**Error:**
```
loading footprint 'LCDRV_SSD1963' (/LCDRV_SSD1963):
  parsing /LCDRV_SSD1963/Data: primitive #47 (Text) at Data offset 0x207F
  (2 subrecords): Invalid enum value: invalid value 3 for enum TextKind
```

**Also affects:** `Synthiam.PcbLib` (same error, different footprint `Bhold2032`).

**Root cause:** `TextKind` enum only handles values 0-2, but value 3 exists in the
wild. Likely a newer Altium text kind (e.g. barcode text or designator variant).

**Investigation needed:**
- Check `AD26-dotnet` for `TTextKind` or `IPCB_Text.GetState_TextKind` to find
  the full enum definition
- Check Delphi exports for the text object ID 12 reader

**Fix:** Add the missing variant to the `TextKind` enum in `altium-format-types`.

---

### Problem 2: PATTERN name Win-1252 mismatch — 1 file

**File:** `miniFOC-foc_pcblib.PcbLib`

**Error:**
```
loading footprint '晶振-3P' (/晶振-3P):
  Invalid parameter value for key 'PATTERN':
  Data stream pattern '¾§Õñ-3P' does not match Parameters PATTERN '晶振-3P'
```

**Root cause:** The footprint name contains Chinese characters (`晶振` = crystal
oscillator). The CFB storage name uses UTF-16LE (which Windows COM APIs decode
correctly), but the binary Data stream's PATTERN field is encoded in Win-1252.
Win-1252 cannot represent CJK characters, so the bytes `¾§Õñ` are a lossy
representation.

The invariant check compares the Win-1252-decoded PATTERN against the UTF-16LE
storage name and they don't match because Win-1252 lacks these codepoints.

**Fix:** When the Data stream PATTERN and Parameters PATTERN disagree, and one
is clearly a Win-1252 mojibake of the other, accept the UTF-16LE (Parameters)
version as authoritative. This is consistent with Altium's own behavior where
the parameter `PATTERN` is the source of truth and the binary Data stream
pattern field is a legacy artifact.

---

### Problem 3: Pad reserved byte 170 non-zero — 1 file

**File:** `Senior-Design-Custom.PcbLib`

**Error:**
```
loading footprint 'MountainSwitch SPST' (/MountainSwitch SPST):
  parsing /MountainSwitch SPST/Data: primitive #0 (Pad) at Data offset 0x18
  (6 subrecords): Invalid parameter value for key 'reserved byte 170':
  expected 0, got 0x0A
```

**Root cause:** Byte 170 in the pad record is currently asserted as zero
(reserved), but this file has `0x0A` there. This is likely a real field that
needs reverse engineering.

**Investigation needed:**
- Check the Delphi pad reader (`PcbApi_ReadPad` or similar) in Ghidra at
  offset 170 to identify what this field represents
- Check `IPCB_Pad` interface in AD26-dotnet for properties that might map here
- Cross-reference with the pad binary layout in `docs/dxp/pcb-records.md`

**Fix:** Replace the zero assertion with a typed field once identified.

---

### Problem 4: Text reserved byte 231 non-zero — 1 file

**File:** `mobinbyn-Socket.PcbLib`

**Error:**
```
loading footprint 'DIP_CON_TSH_3675_Line In' (/DIP_CON_TSH_3675_Line In):
  parsing /DIP_CON_TSH_3675_Line In/Data: primitive #110 (Text) at Data
  offset 0x2B85 (2 subrecords): Invalid parameter value for key
  'reserved byte 231': expected 0, got 0x01
```

**Root cause:** Same class of issue as Problem 3. Byte 231 in the text record
is asserted as zero but is `0x01` in this file. Likely a real field.

**Investigation needed:**
- Check the Delphi text reader in Ghidra at offset 231
- Check `IPCB_Text` interface for properties mapping to this offset

**Fix:** Replace the zero assertion with a typed field once identified.

---

### Problem 5: WideStrings ENCODEDTEXT non-UTF-8 — 1 file

**File:** `TranDangKhoa-LIB.PcbLib`

**Error:**
```
loading footprint 'EDGE-PCIE-X8' (/EDGE-PCIE-X8):
  loading sidecars for /EDGE-PCIE-X8:
  Invalid parameter value for key 'ENCODEDTEXT1':
  decoded bytes are not valid UTF-8: invalid utf-8 sequence of 1 bytes from index 2
```

**Root cause:** The `ENCODEDTEXT` sidecar field uses a comma-separated byte
encoding. After decoding the bytes, the parser calls `from_utf8()` which fails.
The bytes are likely Win-1252 encoded (Vietnamese text, given the author name
`TranDangKhoa`).

**Fix:** Use `WINDOWS_1252.decode()` instead of `from_utf8()` for ENCODEDTEXT
values. All 256 Win-1252 byte values are valid, so this cannot fail.

---

### Problem 6: Binary read past end (truncated/corrupt) — 1 file

**File:** `TranDangKhoa-VioletofSun.PcbLib`

**Error:**
```
Binary read past end: needed 8 bytes at offset 32, only 0 remain
```

**Root cause:** Fails very early during loading — the library-level Data stream
or header is shorter than expected. May be a truncated/corrupt fixture file, or
a format variant in the library-level metadata.

**Investigation needed:**
- Hex dump the file's root streams to check if it's genuinely truncated
- Check if this is a legacy PcbLib version with a different header format

**Fix:** If corrupt, mark as known-bad fixture. If format variant, implement
the variant.

---

## Roundtrip Failures (34 files validate, 0 clean roundtrip)

All 34 files that pass validation also save successfully, but **every file has
semantic diff issues** after roundtrip. The issues fall into 3 categories:

### Roundtrip Problem A: Text primitive byte mismatches — all files

Every footprint's last Text primitive has 2 byte differences:
- **Subrecord 0, offset ~58-64**: A byte that is non-zero in the original becomes
  `0x00` after roundtrip (e.g. `A=0x0c, B=0x00` or `A=0xdc, B=0x00`)
- **Subrecord 1, offset 0**: First byte differs (e.g. `A=0x0b, B=0x2e` or
  `A=0x0d, B=0x27`)

**Root cause:** Likely a text field (possibly `FontId` or a flags byte) that our
serializer writes differently than the original. The offset varies slightly by
file, suggesting it's after a variable-length string field.

**Investigation:** Compare the text primitive serializer against the Delphi/C#
text writer at these offsets to identify which field is wrong.

### Roundtrip Problem B: ComponentBody subrecord length mismatch — all files

Every ComponentBody primitive's subrecord 0 is consistently **~22 bytes longer**
after roundtrip (e.g. 810→832, 870→892, 920→942). The byte at offset 18 also
differs, which is in the embedded 3D model data region.

**Root cause:** The embedded STEP model data (zlib-compressed) is being
recompressed at a different compression level or with different settings,
producing a slightly different (larger) output. Offset 18 is likely within
or just before the compressed payload.

**Investigation:** Check whether the ComponentBody serializer recompresses the
embedded model data. If so, use the same compression level as the original, or
better, preserve the original compressed bytes verbatim when the model data
hasn't changed.

### Roundtrip Problem C: Library/Data stream length mismatch — all files

The `/Library/Data` stream differs in both length and content after roundtrip.
This is the library-level metadata stream.

**Example:** `OpenECADLib-fp-0003.PcbLib`: A=1,041,633 bytes vs B=1,041,287 bytes
(346 bytes shorter after roundtrip).

**Root cause:** Likely related to the embedded model data recompression affecting
the library-level data stream as well, or parameter serialization differences in
the library header.
