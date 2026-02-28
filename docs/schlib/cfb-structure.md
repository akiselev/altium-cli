> **Authoritative reference**: See [../../dxp/sch-files.md](../../dxp/sch-files.md)
> for the canonical format specification. This document covers SchLib-specific details.

# CFB Structure

SchLib files use the OLE Compound Binary (CFB / Structured Storage) format. The storage
tree looks like this:

```
Root Storage
 |
 +-- FileHeader                  (library header + component index + font table)
 +-- Storage                     (global embedded binary objects - images)
 +-- SectionKeys                 (optional - name-to-key mapping for long names)
 +-- LibAdditional               (optional - header for per-component additional data)
 |
 +-- <ComponentKey>/             (one CFB sub-storage per component)
 |    +-- Data                   (component records - the main stream)
 |    +-- Additional             (optional - per-component additional records)
 |    +-- PinFrac                (optional)
 |    +-- PinDesc                (optional)
 |    +-- PinMiscData            (optional)
 |    +-- PinTextData            (optional)
 |    +-- PinWideText            (optional)
 |    +-- PinSymbolLineWidth     (optional)
 |    +-- PinPackageLength       (optional)
 |    +-- PinPropagationDelay    (optional)
 |    +-- PinFunctionData        (optional)
 |
 +-- <AliasKey>/                 (CFB sub-storage for alias)
      +-- Redirection            (redirect to canonical component name)
```

## Block encoding format

Every stream in a SchLib is made up of sequential blocks. Each block begins with a 4-byte
little-endian header:

```
bits [23:0]  (lower 24 bits) = payload size in bytes
bits [31:24] (upper 8 bits)  = flags byte
```

Flags values:
- `0x00` - parameter text block: NUL-terminated pipe-delimited `key=value` pairs encoded
  in Windows-1252. The `RECORD` key identifies the record type.
- `0x01` - binary data block: raw binary payload. In SchLib `Data` streams, binary blocks
  are always pin records (first byte = `0x02`).

## Component storage key rules

CFB storage names are limited to 31 characters and must not contain the characters
`` /\:*?"<>|! ``. The mapping from component name to CFB key follows these rules:

1. Invalid characters (`` /\:*?"<>|! ``) are replaced with `_`.
2. Names longer than 31 characters are truncated to 31 characters.
3. If truncation produces a collision, a numeric suffix is appended (making it unique
   within 31 chars).
4. When any component name exceeds 31 characters, the full name-to-key mapping is stored
   in the `/SectionKeys` stream.

For names <= 31 characters (after character replacement), the name itself is used directly
as the CFB storage key and no SectionKeys entry is created.

## Storage stream (embedded images)

The `/Storage` stream holds global embedded binary objects referenced by `SchImage`
records. It uses the embedded object envelope format:

```
[header block, flags=0x00]: |RECORD=0|HEADER=Icon storage|Weight=<count>|
[entry blocks, flags=0x01]: one per image
```

Each entry block payload starts with `0xD0` (the embedded object tag). The object data
that follows is zlib-compressed. The name embedded in the object matches the `FILENAME`
parameter of the corresponding `SchImage` record.

## Pin sidecar streams

All 9 pin sidecar streams (`PinFrac`, `PinDesc`, `PinMiscData`, `PinTextData`,
`PinWideText`, `PinSymbolLineWidth`, `PinPackageLength`, `PinPropagationDelay`,
`PinFunctionData`) also use the embedded object envelope format with `0xD0` tag entries.
See [pin-sidecar-streams.md](pin-sidecar-streams.md) for their detailed formats.

## Alias sub-storages

Each alias gets its own CFB sub-storage at `/<AliasKey>/` containing only a `Redirection`
stream. The `Redirection` stream is a single parameter text block (flags=0x00):

```
|RECORD=0|SectionName=<canonical_component_name>|
```

The `SectionName` value is the canonical component's full name (not its CFB key). During
loading, encountering a `Redirection` stream means the requested name is an alias; load
the canonical component instead.
