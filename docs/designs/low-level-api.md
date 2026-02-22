# Low-Level API Design

## Overview

Four layers compose the parsing stack inside `altium-format`. Each layer has a single
responsibility and a clean boundary with its neighbors. All types are `pub(crate)` — the
public API surface is the document types (`SchDoc`, `PcbDoc`, etc.) and their record types,
not the parsing machinery.

```
┌─────────────────────────────────────────────────────┐
│  Document Loader (SchDoc::open, PcbDoc::open, …)    │  ← public API
│  Orchestrates layers, merges sidecars, builds trees  │
├─────────────────────────────────────────────────────┤
│  Layer 4: Record Parsing                             │
│  FromParams / FromBinary traits + dispatch           │
│  POLICY: assert_exhausted at dispatch boundary       │
├─────────────────────────────────────────────────────┤
│  Layer 3: ParameterCollection / BinaryReader         │
│  Structured data access with consumption tracking    │
│  MECHANISM: remove-on-read, remaining_keys/bytes     │
├─────────────────────────────────────────────────────┤
│  Layer 2: Block Stream                               │
│  Parse block framing, decompress, iterate blocks     │
├─────────────────────────────────────────────────────┤
│  Layer 1: CFB Document                               │
│  Open file, enumerate storages/streams, read bytes   │
└─────────────────────────────────────────────────────┘
```

Data flows bottom-up: file → bytes → blocks → params/binary → typed records.

---

## Layer 1: CFB Document

**Module**: `cfb_document`

**Responsibility**: Wraps the `cfb` crate to provide Altium-aware access to CFB (OLE
Compound Document) files. Handles the fact that all Altium files (SchDoc, SchLib, PcbDoc,
PcbLib, IntLib) are CFB containers, with the single exception of PrjPcb (plain-text INI).

```rust
/// Thin wrapper over cfb::CompoundFile providing Altium-specific ergonomics.
pub(crate) struct CfbDocument {
    inner: cfb::CompoundFile<std::io::Cursor<Vec<u8>>>,
}

impl CfbDocument {
    /// Open a CFB file from a filesystem path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self>;

    /// Read an entire stream into a byte buffer.
    /// Returns Err if the stream does not exist.
    pub fn read_stream(&mut self, path: &str) -> Result<Vec<u8>>;

    /// Read a stream, returning Ok(None) if it does not exist.
    pub fn read_stream_optional(&mut self, path: &str) -> Result<Option<Vec<u8>>>;

    /// Check whether a stream or storage exists at the given path.
    pub fn exists(&self, path: &str) -> bool;

    /// List immediate child entries (storages and streams) under a path.
    /// Returns (storages, streams) as separate vectors of names.
    pub fn list_entries(&self, path: &str) -> Result<(Vec<String>, Vec<String>)>;
}
```

### Design notes

- We read the entire file into memory via `Cursor<Vec<u8>>` rather than holding a file
  handle. Altium files are typically <100 MB; the simplicity is worth it.
- No file-type awareness at this layer. `CfbDocument` does not know whether it holds a
  SchDoc or PcbDoc — that knowledge lives in the document loaders above.
- Stream paths use forward slashes (`/FileHeader`, `/Board6/Data`). The `cfb` crate
  accepts these natively.

---

## Layer 2: Block Stream

**Module**: `block_stream`

**Responsibility**: Parse the block framing that Altium uses within CFB streams. Every
Altium stream (except PrjPcb text and a few raw binary sidecars) is a sequence of
size-prefixed blocks. This layer handles the framing only, yielding individual block
payloads tagged with their format.

### Block framing format

```
┌───────────────────────────┐
│ i32 LE header             │  bits 0-23: payload size
│                           │  bits 24-31: flags (0x00=text, 0x01=binary)
├───────────────────────────┤
│ payload (size bytes)      │  text: Windows-1252 parameter string
│                           │  binary: packed struct data
└───────────────────────────┘
```

**Important**: `0xD0` as the first byte of a binary block payload is NOT a compression
indicator — it is the embedded object envelope marker (see Layer 3). Layer 2 does NOT
perform decompression. Decompression is the responsibility of the embedded object
envelope parser in Layer 3, which knows from context whether inner data is compressed
(e.g. `/Storage` entries are zlib-compressed, pin sidecar entries are not).

### Types

```rust
/// Discriminant for block payload format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlockFormat {
    /// Pipe-delimited key=value parameter string (Windows-1252 or UTF-8).
    Text,
    /// Packed little-endian binary struct.
    Binary,
}

/// A single block extracted from a stream.
#[derive(Debug)]
pub(crate) struct Block {
    /// Whether the payload is text parameters or binary data.
    pub format: BlockFormat,
    /// The raw payload bytes (already decompressed if applicable).
    pub data: Vec<u8>,
}

/// Parse all blocks from a raw stream byte buffer.
///
/// The stream is consumed sequentially. Each block's header is read,
/// the payload extracted, and the block appended to the output vec.
/// No decompression is performed — payloads are returned as-is.
pub(crate) fn parse_blocks(stream_data: &[u8]) -> Result<Vec<Block>>;

/// Parse blocks, returning an iterator for lazy consumption.
pub(crate) fn iter_blocks(stream_data: &[u8]) -> BlockIter<'_>;
```

### PcbLib pattern name prefix

PcbLib footprint streams have a Pascal-string pattern name before the block sequence.
This is not part of the block framing — it is a stream-level prefix that the document
loader strips before passing the remaining bytes to `parse_blocks`.

```rust
/// Read a Pascal-string prefix (u8 length + ASCII bytes) from the start of a
/// byte buffer, returning (name, remaining_bytes).
pub(crate) fn read_pascal_prefix(data: &[u8]) -> Result<(String, &[u8])>;
```

---

## Layer 3: ParameterCollection / BinaryReader

**Module**: `param_collection`, `binary_io`

**Responsibility**: Structured access to the contents of a single block. This layer
provides the *mechanism* for consumption tracking — it knows what data has been read and
what remains — but does not enforce any policy about unknown fields. That policy lives in
Layer 4.

### ParameterCollection

The primary text serialization format. Pipe-delimited key=value pairs used by SchDoc,
SchLib, PcbDoc text sections (Nets6, Rules6, Classes6, Board6), and pin sidecar streams.

Parameter strings appear in two encodings depending on context:
- **Windows-1252**: Main record blocks (flags=0x00 blocks in Data/FileHeader streams)
- **UTF-16LE**: Pin sidecar streams (PinMiscData, PinWideText, PinSymbolLineWidth,
  PinPackageLength, PinPropagationDelay, PinFunctionData) and PcbLib WideStrings

Both produce the same pipe-delimited `key=value` format once decoded to a Rust String.

```rust
/// Ordered map of string key-value pairs parsed from a pipe-delimited parameter block.
///
/// Designed for consume-on-read access: each successful `remove_*` call removes the
/// key from the map. After all expected keys are consumed, `assert_exhausted()` can
/// verify nothing remains.
pub(crate) struct ParameterCollection {
    /// Preserves insertion order for deterministic round-trip output.
    params: IndexMap<String, String>,
}

impl ParameterCollection {
    /// Parse a parameter string from Windows-1252 encoded bytes.
    ///
    /// Handles:
    /// - Windows-1252 → String conversion
    /// - Pipe delimiter splitting
    /// - `%UTF8%` prefix on keys (UTF-8 encoded values)
    /// - Leading/trailing pipe stripping
    /// - NUL terminator stripping
    pub fn from_bytes(data: &[u8]) -> Result<Self>;

    /// Parse a parameter string from UTF-16LE encoded bytes.
    ///
    /// Used by pin sidecar streams and PcbLib WideStrings. Same pipe-delimited
    /// format, different source encoding.
    pub fn from_utf16le_bytes(data: &[u8]) -> Result<Self>;

    // ── Consuming accessors ──────────────────────────────────────────

    /// Remove a required key and parse its value.
    /// Errors if the key is missing or the value fails to parse.
    pub fn remove_required<T: FromParamValue>(&mut self, key: &str) -> Result<T>;

    /// Remove an optional key and parse its value.
    /// Returns Ok(None) if the key is absent.
    /// Errors only if the key is present but fails to parse.
    pub fn remove_optional<T: FromParamValue>(&mut self, key: &str) -> Result<Option<T>>;

    /// Remove a key with a fallback default.
    /// Returns the default if the key is absent.
    pub fn remove_with_default<T: FromParamValue>(
        &mut self,
        key: &str,
        default: T,
    ) -> Result<T>;

    /// Remove a DXP fractional coordinate pair (e.g. LOCATION.X + LOCATION.X_FRAC).
    /// Reconstructs the full coordinate: integer_part * 100_000 + frac_part.
    pub fn remove_coord(&mut self, key: &str, frac_key: &str) -> Result<Coord>;

    /// Remove an indexed coordinate array (X1, Y1, X2, Y2, … with COUNT key).
    pub fn remove_indexed_coords(
        &mut self,
        count_key: &str,
        x_prefix: &str,
        y_prefix: &str,
    ) -> Result<Vec<CoordPoint>>;

    // ── Exhaustion checking ──────────────────────────────────────────

    /// Return the keys that have not been consumed.
    pub fn remaining_keys(&self) -> impl Iterator<Item = &str>;

    /// Return the number of unconsumed keys.
    pub fn remaining_count(&self) -> usize;

    /// Error if any keys remain unconsumed.
    /// The error message includes the list of remaining keys.
    pub fn assert_exhausted(&self) -> Result<()>;
}
```

#### FromParamValue / ToParamValue

Conversion traits for individual parameter values. Implemented for all primitive types
that appear in parameter strings.

```rust
/// Parse a typed value from a parameter string value.
pub(crate) trait FromParamValue: Sized {
    fn from_param_value(value: &str) -> Result<Self>;
}

/// Serialize a typed value to a parameter string value.
pub(crate) trait ToParamValue {
    fn to_param_value(&self) -> String;
}

// Implementations for:
//   i32, u32, i16, u16, i8, u8, f64    – decimal text
//   bool                                – "T"/"F" and "TRUE"/"FALSE"
//   String                              – identity
//   Coord                               – i32 in internal units
//   Color                               – Win32 COLORREF as decimal i32
//   UniqueId                            – GUID string
//   Enums via #[derive(AltiumEnum)]     – integer ↔ variant
```

### BinaryReader / BinaryWriter

The PCB binary serialization format. Packed little-endian structs. Used by PcbDoc primary
sections (Arcs6, Pads6, Tracks6, etc.) and PcbLib footprint data.

```rust
/// Cursor over a byte slice with position tracking.
///
/// Every read advances the position. After all expected fields are read,
/// `assert_exhausted()` can verify no trailing bytes remain.
pub(crate) struct BinaryReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> BinaryReader<'a> {
    pub fn new(data: &'a [u8]) -> Self;

    // ── Primitive reads ──────────────────────────────────────────────

    pub fn read_u8(&mut self) -> Result<u8>;
    pub fn read_i8(&mut self) -> Result<i8>;
    pub fn read_u16_le(&mut self) -> Result<u16>;
    pub fn read_i16_le(&mut self) -> Result<i16>;
    pub fn read_u32_le(&mut self) -> Result<u32>;
    pub fn read_i32_le(&mut self) -> Result<i32>;
    pub fn read_u64_le(&mut self) -> Result<u64>;
    pub fn read_i64_le(&mut self) -> Result<i64>;
    pub fn read_f32_le(&mut self) -> Result<f32>;
    pub fn read_f64_le(&mut self) -> Result<f64>;
    pub fn read_bool(&mut self) -> Result<bool>;

    // ── Compound reads ───────────────────────────────────────────────

    /// Read a Coord (i32 LE, 10000 units = 1 mil).
    pub fn read_coord(&mut self) -> Result<Coord>;

    /// Read a CoordPoint (two consecutive i32 LE: x, y).
    pub fn read_coord_point(&mut self) -> Result<CoordPoint>;

    /// Read a length-prefixed string (i32 LE length + UTF-8 bytes).
    pub fn read_string_block(&mut self) -> Result<String>;

    /// Read a Pascal string (u8 length + bytes).
    pub fn read_pascal_string(&mut self) -> Result<String>;

    /// Read exactly N bytes into a vec.
    pub fn read_bytes(&mut self, count: usize) -> Result<Vec<u8>>;

    /// Skip N bytes (read and discard).
    pub fn skip(&mut self, count: usize) -> Result<()>;

    /// Create a sub-reader over the next `len` bytes.
    /// Advances this reader's position by `len`.
    /// The sub-reader has its own independent position starting at 0.
    pub fn sub_reader(&mut self, len: usize) -> Result<BinaryReader<'a>>;

    // ── Position / exhaustion ────────────────────────────────────────

    /// Bytes remaining from current position to end.
    pub fn remaining(&self) -> usize;

    /// Current read position.
    pub fn position(&self) -> usize;

    /// Error if any bytes remain unread.
    pub fn assert_exhausted(&self) -> Result<()>;
}

/// Byte buffer writer with position tracking.
pub(crate) struct BinaryWriter {
    buf: Vec<u8>,
}

impl BinaryWriter {
    pub fn new() -> Self;

    pub fn write_u8(&mut self, val: u8);
    pub fn write_i32_le(&mut self, val: i32);
    pub fn write_coord(&mut self, val: Coord);
    pub fn write_coord_point(&mut self, val: CoordPoint);
    pub fn write_string_block(&mut self, val: &str);
    pub fn write_pascal_string(&mut self, val: &str);
    pub fn write_bytes(&mut self, data: &[u8]);
    // ... mirrors BinaryReader

    /// Consume the writer, returning the assembled byte buffer.
    pub fn finish(self) -> Vec<u8>;
}
```

---

## Layer 4: Record Parsing

**Module**: `sch::records`, `pcb::records`

**Responsibility**: Domain-specific record parsing with strict validation. This is where
the "fail on unknown field" policy is enforced. The key architectural decision is *where*
the exhaustion check happens.

### The exhaustion boundary

`assert_exhausted()` is called **at the dispatch boundary** — the point where a raw block
is converted into a typed record — **not** inside individual `FromParams`/`FromBinary`
implementations. This is because:

1. **Base types compose via flatten.** `SchPrimitiveBase`, `SchGraphicalBase`,
   `PcbPrimitiveCommon` use `FromParams`/`FromBinary` to read their own fields from a
   shared ParameterCollection/BinaryReader. They must NOT check exhaustion because the
   containing record has more fields to consume.

2. **The dispatcher knows it's at the top level.** Only the top-level dispatch function
   knows that all fields should now be consumed.

3. **Single point of enforcement.** Having the check in one place (the dispatcher) rather
   than scattered across every record type is simpler and less error-prone.

```
Block arrives at dispatcher
    │
    ▼
Dispatcher reads record type discriminant (RECORD=N or object_id byte)
    │
    ▼
Dispatcher calls RecordType::from_params(&mut params) or ::from_binary(&mut reader)
    │  ├─ Base types read their fields (params/reader partially consumed)
    │  └─ Record type reads its fields (params/reader further consumed)
    │
    ▼
Dispatcher calls params.assert_exhausted() / reader.assert_exhausted()
    │
    ▼
Returns Result<SchRecord> or Result<PcbRecord>
```

### Parsing traits

```rust
/// Parse a record's fields from a ParameterCollection.
///
/// Implementations consume their known keys via remove_required / remove_optional.
/// They do NOT call assert_exhausted — that is the dispatcher's job.
pub(crate) trait FromParams: Sized {
    fn from_params(params: &mut ParameterCollection) -> Result<Self>;
}

/// Serialize a record's fields to a ParameterCollection.
pub(crate) trait ToParams {
    fn to_params(&self, params: &mut ParameterCollection);
}

/// Parse a record's fields from a BinaryReader.
///
/// Implementations read their known fields sequentially.
/// They do NOT call assert_exhausted — that is the dispatcher's job.
pub(crate) trait FromBinary: Sized {
    fn from_binary(reader: &mut BinaryReader<'_>) -> Result<Self>;
}

/// Serialize a record's fields to a BinaryWriter.
pub(crate) trait ToBinary {
    fn to_binary(&self, writer: &mut BinaryWriter);
}
```

### Schematic record dispatch

```rust
/// All schematic primitive types.
pub(crate) enum SchRecord {
    Component(SchComponent),
    Pin(SchPin),
    Wire(SchWire),
    NetLabel(SchNetLabel),
    PowerObject(SchPowerObject),
    // ... one variant per RECORD type, no Unknown/Fallback variant
}

impl SchRecord {
    /// Parse a single schematic record from a text block.
    ///
    /// 1. Parses the block bytes into a ParameterCollection
    /// 2. Removes the RECORD key to determine the record type
    /// 3. Dispatches to the concrete type's FromParams
    /// 4. Asserts the ParameterCollection is exhausted
    /// 5. Returns the polymorphic SchRecord
    pub fn from_block(block: &Block) -> Result<Self> {
        assert!(block.format == BlockFormat::Text);
        let mut params = ParameterCollection::from_bytes(&block.data)?;
        let record_id: i32 = params.remove_required("RECORD")?;

        let record = match record_id {
            1 => SchRecord::Component(SchComponent::from_params(&mut params)?),
            2 => SchRecord::Pin(SchPin::from_params(&mut params)?),
            25 => SchRecord::NetLabel(SchNetLabel::from_params(&mut params)?),
            27 => SchRecord::Wire(SchWire::from_params(&mut params)?),
            // ... exhaustive match over all known RECORD values
            _ => return Err(AltiumFormatError::UnknownRecordType(record_id)),
        };

        params.assert_exhausted()?;  // ← strict validation happens HERE
        Ok(record)
    }
}
```

### PCB record dispatch

```rust
/// All PCB primitive types.
pub(crate) enum PcbRecord {
    Arc(PcbArc),
    Pad(PcbPad),
    Via(PcbVia),
    Track(PcbTrack),
    Text(PcbText),
    // ... one variant per object ID, no Unknown/Fallback variant
}

impl PcbRecord {
    /// Parse a single PCB record from a binary block.
    ///
    /// 1. Creates a BinaryReader over the block data
    /// 2. Reads the object_id byte
    /// 3. Reads the record length and creates a sub-reader
    /// 4. Dispatches to the concrete type's FromBinary
    /// 5. Asserts the sub-reader is exhausted
    /// 6. Returns the polymorphic PcbRecord
    pub fn from_block(block: &Block) -> Result<Self> {
        assert!(block.format == BlockFormat::Binary);
        let mut reader = BinaryReader::new(&block.data);
        let object_id = reader.read_u8()?;
        let length = reader.read_u32_le()? as usize;
        let mut sub_reader = reader.sub_reader(length)?;

        let record = match object_id {
            1 => PcbRecord::Arc(PcbArc::from_binary(&mut sub_reader)?),
            2 => PcbRecord::Pad(PcbPad::from_binary(&mut sub_reader)?),
            3 => PcbRecord::Via(PcbVia::from_binary(&mut sub_reader)?),
            4 => PcbRecord::Track(PcbTrack::from_binary(&mut sub_reader)?),
            // ... exhaustive match over all known object IDs
            _ => return Err(AltiumFormatError::UnknownObjectId(object_id)),
        };

        sub_reader.assert_exhausted()?;  // ← strict validation happens HERE
        Ok(record)
    }
}
```

### PCB text-format records

Some PCB sections use text parameters rather than binary (Nets6, Rules6, Classes6, Board6).
These follow the same pattern as schematic records but dispatch on section identity rather
than a RECORD key:

```rust
/// Parse a PCB net from a text parameter block.
/// The caller (document loader) knows this block came from /Nets6/Data.
pub(crate) fn parse_pcb_net(block: &Block) -> Result<PcbNet> {
    assert!(block.format == BlockFormat::Text);
    let mut params = ParameterCollection::from_bytes(&block.data)?;
    let net = PcbNet::from_params(&mut params)?;
    params.assert_exhausted()?;
    Ok(net)
}
```

### Derive macros

The `altium-format-derive` crate generates `FromParams`/`ToParams` and
`FromBinary`/`ToBinary` implementations from struct definitions. The macros handle:

- Field-to-parameter-key mapping (`#[altium(param = "KEY")]`)
- Fractional coordinate pairs (`#[altium(param = "X", frac = "X_FRAC")]`)
- Indexed coordinate arrays (`#[altium(indexed_coords, ...)]`)
- Binary type mapping (`#[altium(binary, ty = "i32le")]`)
- Base type flattening (`#[altium(flatten)]`)
- Optional vs required vs default fields

The macros generate consume-based code (calling `remove_required`, `remove_optional`,
etc.) but do **not** generate `assert_exhausted` calls — that remains the dispatcher's
responsibility.

---

## Collection patterns: Lists within records

Data is nested at multiple levels. Not every list is a stream of blocks — some are
embedded *within* a single block. The format uses several distinct collection patterns,
and each is handled at a different layer.

### Pattern 1: Stream of blocks (Layer 2)

The most common pattern. A stream contains N blocks; each block is one record.

```
/FileHeader stream:
  Block 0 → header record
  Block 1 → first primitive
  Block 2 → second primitive
  ...

/Arcs6/Data stream:
  Block 0 → section header
  Block 1 → first arc
  Block 2 → second arc
  ...
```

**Handled by**: `parse_blocks()` in Layer 2. The document loader iterates and dispatches
each block through Layers 3+4.

**Exhaustion**: The document loader consumes all blocks. If a stream has blocks left over
after the expected structure, that's a document-level error, not a Layer 2 concern.

### Pattern 2: Indexed parameter families (Layer 3 + derive macros)

A single ParameterCollection contains a count key and N copies of a key pattern with
numeric suffixes. This is the schematic format's way of encoding variable-length arrays
within a single record.

**Examples**:

```
Vertices (1-based):
  |LOCATIONCOUNT=3|X1=100|Y1=200|X2=300|Y2=400|X3=500|Y3=600|

Font table (1-based):
  |FontIdCount=2|Size1=10|FontName1=Arial|Bold1=T|Size2=12|FontName2=Times|Bold2=F|

Component index (0-based):
  |CompCount=2|LibRef0=Resistor|CompDescr0=...|LibRef1=Capacitor|CompDescr1=...|
```

**Handled by**: ParameterCollection provides a general indexed-removal method. The derive
macros generate calls to it.

```rust
impl ParameterCollection {
    /// Remove an indexed family of sub-records.
    ///
    /// Reads count_key to get N, then for each index i in base..base+N,
    /// calls `parse_one(self, i)` which removes that index's keys.
    ///
    /// `base` is typically 0 or 1 depending on the parameter family.
    pub fn remove_indexed<T>(
        &mut self,
        count_key: &str,
        base: usize,
        parse_one: impl Fn(&mut Self, usize) -> Result<T>,
    ) -> Result<Vec<T>>;
}
```

Usage in a manual `FromParams` implementation:

```rust
impl FromParams for SchDocHeader {
    fn from_params(params: &mut ParameterCollection) -> Result<Self> {
        // ... other fields ...

        // Font table: indexed family with compound sub-records
        let fonts = params.remove_indexed("FontIdCount", 1, |params, i| {
            Ok(Font {
                size: params.remove_required(&format!("Size{i}"))?,
                name: params.remove_required(&format!("FontName{i}"))?,
                bold: params.remove_with_default(&format!("Bold{i}"), false)?,
                italic: params.remove_with_default(&format!("Italic{i}"), false)?,
                underline: params.remove_with_default(&format!("Underline{i}"), false)?,
                strikeout: params.remove_with_default(&format!("StrikeOut{i}"), false)?,
                rotation: params.remove_with_default(&format!("Rotation{i}"), 0)?,
            })
        })?;

        // ...
    }
}
```

The derive macro supports this with an `indexed` attribute on `Vec<T>` fields:

```rust
#[derive(AltiumRecord)]
struct SchDocHeader {
    /// Font table: Vec of Font, indexed 1-based by FontIdCount
    #[altium(indexed, count = "FontIdCount", base = 1)]
    fonts: Vec<Font>,
}

/// Each font's fields use the index as a suffix
#[derive(AltiumRecord)]
#[altium(indexed_suffix)]
struct Font {
    #[altium(param = "Size")]      // becomes "Size1", "Size2", ...
    size: i32,
    #[altium(param = "FontName")]
    name: String,
    #[altium(param = "Bold", default = false)]
    bold: bool,
    // ...
}
```

`remove_indexed_coords` (already in the design) is a specialization of this pattern for
the common coordinate-array case, avoiding the overhead of a closure + format strings for
the hot path.

### Pattern 3: Comma-separated values (Layer 3)

Some parameters encode a list as a single comma-separated value string.

```
|PINPROPAGATIONDELAY=100,200,300|
```

**Handled by**: `FromParamValue` implementation for `Vec<T>` that splits on commas, or a
dedicated `remove_list` method on ParameterCollection.

```rust
impl ParameterCollection {
    /// Remove a key whose value is a comma-separated list, parsing each element.
    pub fn remove_list<T: FromParamValue>(&mut self, key: &str) -> Result<Vec<T>>;

    /// Same as remove_list but returns empty vec if key is absent.
    pub fn remove_list_or_empty<T: FromParamValue>(&mut self, key: &str) -> Result<Vec<T>>;
}
```

### Pattern 4: Binary fixed-size arrays (Layer 3)

PCB binary records contain fixed-length arrays of typed elements. PcbPad has 32 per-layer
entries for size, shape, corner radius, offsets, etc.

```
Pad binary layout (excerpt):
  ... [CoordPoint × 32 sizes] [u8 × 32 shapes] [u8 × 32 corner_radii] ...
```

**Handled by**: BinaryReader loop or a helper method.

```rust
impl BinaryReader<'_> {
    /// Read a fixed-size array by calling `read_one` N times.
    pub fn read_array<T, const N: usize>(
        &mut self,
        read_one: impl Fn(&mut Self) -> Result<T>,
    ) -> Result<[T; N]>;
}
```

The derive macro supports this with an `array` attribute:

```rust
#[derive(AltiumRecord)]
struct PcbPad {
    /// Per-layer sizes, 32 entries (one per signal layer)
    #[altium(binary, array = 32, ty = "coord_point")]
    size_layers: [CoordPoint; 32],

    /// Per-layer shapes, 32 entries
    #[altium(binary, array = 32, ty = "u8")]
    shape_layers: [PadShape; 32],
}
```

### Pattern 5: Binary subrecords (Layer 3)

Some PCB records (notably PcbPad) are internally divided into subrecords, each with its
own length prefix. The record's total binary payload contains multiple variable-length
sections.

```
PcbPad binary layout:
  [subrecord 0: common fields]          ← u32 length + payload
  [subrecord 1: size/shape arrays]      ← u32 length + payload
  [subrecord 2: hole/thermal data]      ← u32 length + payload
  ...
```

**Handled by**: `BinaryReader::sub_reader()` at the record's `FromBinary` implementation.
Each subrecord gets its own sub-reader with independent exhaustion checking.

```rust
impl FromBinary for PcbPad {
    fn from_binary(reader: &mut BinaryReader<'_>) -> Result<Self> {
        // Subrecord 0: common fields
        let len0 = reader.read_u32_le()? as usize;
        let mut sr0 = reader.sub_reader(len0)?;
        let layer = sr0.read_u8()?;
        let flags = sr0.read_u16_le()?;
        // ... read all subrecord 0 fields ...
        sr0.assert_exhausted()?;  // ← each subrecord is independently strict

        // Subrecord 1: per-layer arrays
        let len1 = reader.read_u32_le()? as usize;
        let mut sr1 = reader.sub_reader(len1)?;
        // ...
        sr1.assert_exhausted()?;

        // ...
    }
}
```

Note: for subrecords, `assert_exhausted` IS called inside `FromBinary`, because each
subrecord boundary is known to the record type itself — it's not composition via flatten.
This is different from the dispatcher-level exhaustion check, which validates the *outer*
record boundary.

### Pattern 6: Sidecar parallel arrays (Document loader)

Some data is split across parallel streams matched by index. PinFrac has one 12-byte
record per pin; UniqueIDPrimitiveInformation has one entry per primitive.

```
/ComponentName/Data    → [pin0, pin1, pin2, ...]     ← primary records
/ComponentName/PinFrac → [frac0, frac1, frac2, ...]  ← 12 bytes per pin
```

**Handled by**: The document loader, not Layers 1-4. The loader:
1. Parses the primary stream into `Vec<SchRecord>` via Layers 2-4
2. Parses the sidecar stream into `Vec<PinFracData>` (using BinaryReader directly)
3. Merges by index: `records[i].merge_pin_frac(sidecar[i])`

The merge step is a post-processing concern. Sidecar records use the same Layer 3 types
(BinaryReader/ParameterCollection) but bypass Layer 4's record dispatch — they have their
own small struct types parsed directly.

```rust
/// 12-byte per-pin fractional coordinate sidecar.
struct PinFracEntry {
    location_x_frac: i32,
    location_y_frac: i32,
    length_frac: i32,
}

impl FromBinary for PinFracEntry {
    fn from_binary(reader: &mut BinaryReader<'_>) -> Result<Self> {
        Ok(Self {
            location_x_frac: reader.read_i32_le()?,
            location_y_frac: reader.read_i32_le()?,
            length_frac: reader.read_i32_le()?,
        })
    }
}

/// Parse a sidecar stream as a flat array of fixed-size records.
fn parse_sidecar_array<T: FromBinary>(data: &[u8], record_size: usize) -> Result<Vec<T>> {
    let mut reader = BinaryReader::new(data);
    let mut entries = Vec::new();
    while reader.remaining() > 0 {
        let mut sub = reader.sub_reader(record_size)?;
        entries.push(T::from_binary(&mut sub)?);
        sub.assert_exhausted()?;
    }
    Ok(entries)
}
```

### Summary of collection patterns

| Pattern | Where | Example | Parsed by |
|---|---|---|---|
| Stream of blocks | Multiple blocks in stream | SchDoc primitives, PcbDoc arcs | Layer 2 `parse_blocks` + Layer 4 dispatch |
| Indexed param family | Keys in one ParameterCollection | Vertices, font table, component index | Layer 3 `remove_indexed` + derive macro |
| Comma-separated list | Single parameter value | Pin delay values | Layer 3 `remove_list` |
| Binary fixed array | Contiguous in binary record | Pad per-layer shapes (×32) | Layer 3 `read_array` + derive macro |
| Binary subrecords | Length-prefixed within record | PcbPad subrecords | Layer 3 `sub_reader` + record's `FromBinary` |
| Sidecar parallel array | Separate stream, matched by index | PinFrac, UniqueIDs | Document loader post-processing |

---

## Composition: How layers work together

### Example: Loading a SchDoc

```rust
impl SchDoc {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        // Layer 1: Open CFB container
        let mut cfb = CfbDocument::open(path)?;

        // Layer 1: Read the FileHeader stream
        let stream_data = cfb.read_stream("/FileHeader")?;

        // Layer 2: Parse block framing
        let blocks = parse_blocks(&stream_data)?;

        // Block 0 is the header record (sheet properties, font table, etc.)
        let header = SchDocHeader::from_block(&blocks[0])?;

        // Blocks 1+ are the schematic primitives
        let mut records = Vec::new();
        for block in &blocks[1..] {
            // Layer 3 + 4: ParameterCollection → SchRecord (strict)
            let record = SchRecord::from_block(block)?;
            records.push(record);
        }

        // Post-processing: build ownership tree from OWNERINDEX values
        // Post-processing: merge sidecar streams (/Storage, /Additional, etc.)

        Ok(SchDoc { header, records })
    }
}
```

### Example: Loading a PcbDoc section

```rust
fn load_pcb_section(cfb: &mut CfbDocument, section: &str) -> Result<Vec<PcbRecord>> {
    // Layer 1: Read the section stream
    let stream_data = cfb.read_stream(&format!("/{section}/Data"))?;

    // Layer 2: Parse block framing
    let blocks = parse_blocks(&stream_data)?;

    // Block 0 is typically a header block (may be text or binary)
    // Blocks 1+ are the section records
    let mut records = Vec::new();
    for block in &blocks[1..] {
        // Layer 3 + 4: BinaryReader → PcbRecord (strict)
        let record = PcbRecord::from_block(block)?;
        records.push(record);
    }

    Ok(records)
}
```

---

## Error types

All layers use `AltiumFormatError` from `altium-format`. Relevant variants:

```rust
#[derive(Debug, thiserror::Error)]
pub enum AltiumFormatError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    // Layer 1
    #[error("CFB format error: {0}")]
    CfbError(String),
    #[error("Stream not found: {0}")]
    StreamNotFound(String),

    // Layer 2
    #[error("Invalid block header at offset {offset}: {detail}")]
    InvalidBlockHeader { offset: usize, detail: String },
    #[error("Decompression failed: {0}")]
    DecompressionError(String),

    // Layer 3
    #[error("Missing required parameter: {0}")]
    MissingParam(String),
    #[error("Invalid parameter value for key '{key}': {detail}")]
    InvalidParamValue { key: String, detail: String },
    #[error("Binary read past end: needed {needed} bytes at offset {offset}, only {available} remain")]
    BinaryReadPastEnd { offset: usize, needed: usize, available: usize },

    // Layer 4 (strict validation)
    #[error("Unknown record type: {0}")]
    UnknownRecordType(i32),
    #[error("Unknown PCB object ID: {0}")]
    UnknownObjectId(u8),
    #[error("Unknown parameters remaining: {keys:?}")]
    UnknownParams { keys: Vec<String> },
    #[error("Unexpected trailing data: {count} bytes remaining at offset {offset}")]
    UnexpectedTrailingData { offset: usize, count: usize },
}
```

---

## Summary: Who enforces what

| Concern | Layer | Mechanism |
|---|---|---|
| File is valid CFB | 1 | `cfb` crate returns `Err` on corrupt container |
| Stream exists | 1 | `read_stream` returns `Err(StreamNotFound)` |
| Block framing is valid | 2 | `parse_blocks` validates headers, sizes, checksums |
| Decompression succeeds | 2 | `flate2` returns `Err` on corrupt zlib data |
| Parameter syntax is valid | 3 | `ParameterCollection::from_bytes` validates encoding + delimiters |
| Parameter value parses to type | 3 | `remove_required` / `FromParamValue` returns `Err` |
| Binary data has enough bytes | 3 | `BinaryReader::read_*` checks `remaining()` |
| Record type is known | 4 | Dispatch `match` has no wildcard — returns `Err(UnknownRecordType)` |
| All fields consumed | 4 | Dispatcher calls `assert_exhausted()` after `FromParams`/`FromBinary` |
| All bytes consumed | 4 | Dispatcher calls `assert_exhausted()` on sub-reader |
