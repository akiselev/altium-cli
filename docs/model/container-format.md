# Container Format

All Altium Designer files (`.SchLib`, `.SchDoc`, `.PcbDoc`, `.PcbLib`,
`.PrjPcb`, `.IntLib`) are **OLE Compound Document Binary (CFB)** files — the
same container format used by older Microsoft Office documents (`.doc`, `.xls`).

The `cfb` crate provides low-level access to the compound file. The
`altium-format` library wraps it to read and write the Altium-specific stream
layout.

## Block Encoding

Data within each CFB stream is organized as **size-prefixed blocks**:

```
┌──────────────────────────────────────┐
│  i32 header                          │
│  ┌───────────┬──────────────────┐    │
│  │ flags (8b)│ size (24 bits)   │    │
│  └───────────┴──────────────────┘    │
├──────────────────────────────────────┤
│  payload (size bytes)                │
└──────────────────────────────────────┘
```

- **Size**: lower 24 bits of the header (`header & 0x00FFFFFF`).
- **Flags**: upper 8 bits (`header >> 24`). Flag `0x01` indicates a binary
  record (used in schematic streams to distinguish binary-encoded data from
  parameter text).
- **Payload**: raw bytes — either a Windows-1252 encoded parameter string or a
  binary struct, depending on context.

Implemented in `crates/altium-format/src/io/reader.rs` (`read_block`) and
`crates/altium-format/src/io/writer.rs` (`write_block`).

### Compression

Some blocks use zlib compression (detected by a `0xD0` tag byte). The reader
skips the first 2 bytes (zlib header) and decompresses the remainder. The writer
produces a standard zlib stream.

## Stream Layout by File Type

### SchLib (Schematic Library)

```
CompoundFile
├── /Storage                    "Icon storage" header block
├── /FileHeader                 Component index + metadata
│   └── Block 0: ParameterCollection
│       ├── HEADER = "Protel for Windows - Schematic Library..."
│       ├── WEIGHT = <total primitives + aliases>
│       ├── COMPCOUNT = <number of components>
│       ├── LIBREF0, LIBREF1, …         (component names)
│       ├── PARTCOUNT0, PARTCOUNT1, …   (parts per component)
│       ├── COMPDESCR0, COMPDESCR1, …   (descriptions)
│       └── ALIASCOUNT0, COMP0ALIAS0, … (alias mappings)
├── /SectionKeys                (optional) Maps long names > 31 chars
│   └── KEYCOUNT, LIBREF0/SECTIONKEY0, …
└── /{ComponentName}/           One OLE storage per component
    └── Data                    Stream of SchRecord blocks
        ├── Block 0: SchComponent     (RECORD=1)
        ├── Block 1: SchPin           (RECORD=2)
        ├── Block 2: SchSymbol        (RECORD=3)
        ├── …more primitives…
        └── Block N: (EOF)
```

Component names that exceed the 31-character OLE storage name limit use the
`/SectionKeys` stream to map a short key to the full `LIBREF`.

Aliases are stored as redirect streams: the alias storage contains a single
block with `|SECTIONNAME=<real_component_name>\0`.

### SchDoc (Schematic Document)

```
CompoundFile
├── /Storage                    Icon storage header
├── /FileHeader                 All schematic primitives (flat list)
│   ├── Block 0: ParameterCollection (HEADER, WEIGHT)
│   ├── Block 1: SchSheetHeader     (RECORD=31)
│   ├── Block 2: SchComponent       (RECORD=1)
│   ├── Block 3: SchPin             (RECORD=2)
│   ├── …more primitives…
│   └── Block N: (EOF)
└── /Additional                 (optional) Extra parameters
```

All primitives live in a single flat stream. Parent-child relationships are
encoded via the `OWNERINDEX` parameter (see
[Schematic Records](schematic-records.md)).

### PcbDoc (PCB Document)

```
CompoundFile
├── /Board6/Data                Board-level parameters
├── /Components6/Data           Component metadata
│   └── ParameterCollection blocks (designator, pattern, comment)
├── /Primitives6/Data           Board-level PCB primitives
│   ├── Block 0: u32 record count
│   ├── Block 1: PcbArc          (binary)
│   ├── Block 2: PcbPad          (binary)
│   ├── …more primitives…
│   └── Block N: (EOF)
├── /Nets6/Data                 Net names (string blocks)
├── /Rules6/Data                Design rules (DRC)
└── /Classes6/Data              Net/component classes
```

### PcbLib (PCB Library)

Similar to PcbDoc but organized per-footprint, with each footprint in its own
OLE storage containing binary primitive blocks.

### PrjPcb (PCB Project)

Project files use a parameter-based format listing document paths, build
configurations, and variant definitions.

### IntLib (Integrated Library)

Integrated libraries bundle a SchLib and PcbLib into a single CFB container.
The library extracts embedded sub-files and delegates to the appropriate reader.

## Parameter String Encoding

Schematic records are encoded as pipe-delimited key=value strings in
Windows-1252 encoding:

```
|RECORD=2|OWNERINDEX=0|LOCATION.X=100|LOCATION.Y=200|NAME=VCC|DESIGNATOR=1|
```

Special rules:
- **Nesting**: Level 0 uses `|` as delimiter; level 1 uses backtick (`` ` ``).
- **Unicode**: Values requiring characters outside Windows-1252 use a `%UTF8%`
  prefix in the key.
- **Booleans**: Stored as `T` / `F` (short form) or `TRUE` / `FALSE` (long
  form depending on context).
- **Order**: Parameter order is preserved using `IndexMap` for round-trip
  fidelity.

Implemented in `crates/altium-format/src/types/parameters.rs`
(`ParameterCollection`).

## Reading Flow (Schematic)

```
CFB stream
  → read_block() → raw bytes + flags
  → decode Windows-1252 → pipe-delimited string
  → ParameterCollection::from_string() → IndexMap<String, String>
  → SchRecord::from_params() → match on RECORD value
  → SchPin::from_params() (or other type) → typed Rust struct
```

## Reading Flow (PCB)

```
CFB stream
  → read_block() → raw bytes
  → read u8 object_id → dispatch to type
  → PcbPad::read_from() (FromBinary) → typed Rust struct
```
