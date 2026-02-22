# SectionKeys Stream

The optional `/SectionKeys` stream at the CFB root maps full footprint display names to
their (possibly truncated) CFB storage keys.

## When SectionKeys exists

SectionKeys is only present when at least one footprint name exceeds 31 characters — the
maximum length for a CFB storage entry name. If all footprint names are <= 31 characters,
no SectionKeys stream exists and each footprint's CFB storage name IS its display name.

## Binary format

```
[4 bytes] u32 LE: entry count

For each entry:
  [4 bytes] u32 LE: full name block length
  [1 byte]  u8: full name string length (N)
  [N bytes] ASCII full footprint display name

  [4 bytes] u32 LE: truncated key block length
  [1 byte]  u8: truncated key string length (M)
  [M bytes] ASCII truncated CFB storage key
```

Each block uses the same `u32 block_length + u8 string_length + string` framing as other
PcbLib length-prefixed strings.

## Example (from Synthiam.PcbLib)

The Synthiam library has 2 SectionKeys entries for footprints with names > 31 chars:

```
Entry 0:
  Full name (39 chars): "RADIAL CAPACITOR - 10000uF - RightAngle"
  CFB key (31 chars):   "RADIAL CAPACITOR - 10000uF - Ri"

Entry 1:
  Full name (38 chars): "RADIAL CAPACITOR - 100uF - Old Surplus"
  CFB key (31 chars):   "RADIAL CAPACITOR - 100uF - Old "
```

## Name resolution algorithm

When loading a PcbLib:

1. Read SectionKeys (if present) into a `display_name → cfb_key` map.
2. For each top-level CFB storage (excluding system storages):
   a. If the storage name appears as a value in the SectionKeys map, use the corresponding
      key as the display name.
   b. Otherwise, the storage name IS the display name.

When looking up a footprint by name:
1. Check if the name appears in SectionKeys as a full name → use the mapped CFB key.
2. Otherwise, use the name directly as the CFB storage key.

## Shared with SchLib

The SectionKeys format and resolution algorithm is **identical** between PcbLib and SchLib.
The same parsing code should handle both.

## CFB name limitations

CFB (OLE Compound Binary) storage entry names:
- Maximum 31 characters
- Cannot contain: `/\:*?"<>|!`
- Case-preserving but lookups should be case-insensitive
- The truncation simply cuts at 31 characters (no attempt to word-break or hash)
