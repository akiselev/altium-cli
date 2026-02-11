# Altium Data Model

How Altium Designer files are structured internally, and how `altium-format` maps
that structure to Rust types.

## Contents

| Document | What it covers |
|----------|----------------|
| [Container Format](container-format.md) | OLE/CFB container, block encoding, stream layout per file type |
| [Coordinate System](coordinates.md) | Internal units, DXP fractional encoding, unit conversions |
| [Schematic Records](schematic-records.md) | Parameter-based record format, record type table, key structs (SchPin, SchComponent, SchWire, etc.) |
| [PCB Records](pcb-records.md) | Binary record format, object ID table, key structs (PcbPad, PcbTrack, PcbVia, etc.) |
| [Serialization](serialization.md) | Derive macros, traits (FromParams/ToParams, FromBinary/ToBinary), field mapping attributes |

## Architecture at a Glance

```
┌─────────────────────────────────────────────────────┐
│                  Altium .SchLib file                 │
│            (OLE Compound Document / CFB)             │
├─────────────┬──────────────────────────────────────────┤
│ /Storage    │  Icon storage header                     │
│ /FileHeader │  Component index + metadata              │
│ /R1/Data    │  SchRecord[] (pipe-delimited parameters) │
│ /R2/Data    │  SchRecord[] (pipe-delimited parameters) │
│ ...         │                                          │
└─────────────┴──────────────────────────────────────────┘
         │ read blocks, parse parameters
         ▼
┌─────────────────────────────────────────────────────┐
│           ParameterCollection (IndexMap)             │
│  "|RECORD=2|NAME=VCC|DESIGNATOR=1|ELECTRICAL=7|"    │
└─────────────────────────────────────────────────────┘
         │ FromParams trait (derive macro)
         ▼
┌─────────────────────────────────────────────────────┐
│                 Typed Rust structs                   │
│  SchPin { name: "VCC", designator: "1",             │
│           electrical: PinElectricalType::Power, … } │
└─────────────────────────────────────────────────────┘
```

The PCB path is similar but uses binary structs instead of parameter strings:

```
PcbDoc /Primitives6/Data  →  binary blocks  →  FromBinary  →  PcbTrack, PcbPad, …
```

## Two Serialization Formats

Altium uses two distinct record encodings depending on the domain:

| Aspect | Schematic / Libraries | PCB |
|--------|----------------------|-----|
| Encoding | Pipe-delimited key=value text | Little-endian binary structs |
| Character set | Windows-1252 (with `%UTF8%` prefix for Unicode) | Binary |
| Record type key | `RECORD` parameter (i32) | Object ID byte (u8) |
| Rust traits | `FromParams` / `ToParams` | `FromBinary` / `ToBinary` |
| Examples | `.SchLib`, `.SchDoc` | `.PcbLib`, `.PcbDoc` |

Both formats live inside size-prefixed blocks within OLE/CFB streams. See
[Container Format](container-format.md) for details.

## Dispatch Enums

All record types are unified through dispatch enums that allow polymorphic
access:

- **`SchRecord`** — 33 variants covering every schematic primitive (Component,
  Pin, Wire, Label, etc.) plus an `Unknown` fallback.
- **`PcbRecord`** — 11 variants covering PCB primitives (Pad, Track, Via, etc.)
  plus an `Unknown` fallback.

Dispatch reads the record/object ID, then calls the appropriate `FromParams` or
`FromBinary` implementation to produce a typed struct.

## Ownership Model

**Schematic records** use an `OWNERINDEX` field to form parent-child trees. A
`SchComponent` (Record 1) is the root; its child primitives (pins, lines,
rectangles, labels) reference it via `owner_index`. The value is the index of
the owning record in the current component's primitive list, or `-1` for
top-level records.

**PCB records** use a flat model — primitives are either board-level (in
`/Primitives6/Data`) or component-owned (in per-component streams). There is no
owner-index linking.

## Non-Destructive Round-Tripping

The library preserves unknown parameters and binary data through `UnknownFields`
and `unknown_binary` fields, allowing files to be read, modified, and written
back without losing data the library doesn't yet understand. See
[Serialization](serialization.md) for how this works.
