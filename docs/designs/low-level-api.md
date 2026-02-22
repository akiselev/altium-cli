# Low-Level API Design

## Overview

Five layers compose the parsing stack inside `altium-format`. Each layer has a single
responsibility and a clean boundary with its neighbors. All types are `pub(crate)` — the
public API surface is the document types (`SchDoc`, `PcbDoc`, etc.) and their record types,
not the parsing machinery.

```
┌─────────────────────────────────────────────────────┐
│  Document Loader (SchDoc::open, PcbDoc::open, …)    │  ← public API
│  Orchestrates layers, merges sidecars, builds trees  │
├─────────────────────────────────────────────────────┤
│  Layer 5: Record Parsing                             │
│  FromParams / FromBinary traits + dispatch           │
│  POLICY: assert_exhausted at dispatch boundary       │
├─────────────────────────────────────────────────────┤
│  Layer 4: ParameterCollection / BinaryReader         │
│  Structured data access with consumption tracking    │
│  MECHANISM: remove-on-read, remaining_keys/bytes     │
├─────────────────────────────────────────────────────┤
│  Layer 3: Stream Parsers                             │
│  Block framing, binary headers, TLV, envelopes       │
│  Multiple format-specific parsers, not just blocks   │
├─────────────────────────────────────────────────────┤
│  Layer 2: Stream Consumption Tracking                │
│  Wraps CFB access, tracks which streams are read     │
│  POLICY: assert_all_consumed at end of loading       │
├─────────────────────────────────────────────────────┤
│  Layer 1: CFB Document                               │
│  Open file, enumerate storages/streams, read bytes   │
└─────────────────────────────────────────────────────┘
```

Data flows bottom-up: file → tracked access → bytes → blocks/TLV/headers → params/binary → typed records.

### Exhaustion at every level

The fail-fast philosophy applies uniformly across all five layers:

| Level | What is tracked | Exhaustion check | Error on violation |
|---|---|---|---|
| Layer 1 | File is valid CFB | `cfb` crate validates | `CfbError` |
| Layer 2 | Every stream/storage in the CFB | `assert_all_consumed()` | `UnconsumedStreams` |
| Layer 3 | Every byte in a stream | Block framing validates sizes; `Weight`/count checks | `InvalidBlockHeader`, `RecordCountMismatch` |
| Layer 4 | Every key / every byte in a record | `assert_exhausted()` on ParameterCollection/BinaryReader | `UnknownParams`, `UnexpectedTrailingData` |
| Layer 5 | Every record type discriminant | Dispatch match with no wildcard | `UnknownRecordType`, `UnknownObjectId` |

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

    /// Recursively enumerate ALL entries (storages and streams) in the entire file.
    /// Returns absolute paths like "/FileHeader", "/Board6/Data", etc.
    pub fn enumerate_all_entries(&self) -> Result<HashSet<String>>;
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

## Layer 2: Stream Consumption Tracking

**Module**: `tracked_cfb`

**Responsibility**: Wraps `CfbDocument` to track which streams and storages have been
accessed during document loading. At the end of loading, `assert_all_consumed()` verifies
that every entry in the CFB container was either explicitly read or explicitly acknowledged.
This closes the gap between "we validate every byte within a stream" and "we validate every
stream within a file."

### Why this layer exists

Without stream-level tracking, if Altium adds a new stream (e.g. a new sidecar format, a
new metadata section, a new constraint system), our parser would silently ignore it. That
stream might carry electrical connectivity data, design rules, or fabrication constraints.
Silently dropping it violates the fail-fast philosophy.

The principle is the same pattern used everywhere else in the stack:

| What | Mechanism | Exhaustion check |
|---|---|---|
| Parameter keys | remove-on-read from `ParameterCollection` | `assert_exhausted()` — unknown keys are errors |
| Binary bytes | position tracking in `BinaryReader` | `assert_exhausted()` — trailing bytes are errors |
| Record types | dispatch match with no wildcard | unknown discriminant is an error |
| **CFB streams** | **read/acknowledge tracking** | **`assert_all_consumed()` — unknown streams are errors** |

### API

```rust
/// Wraps CfbDocument with stream/storage consumption tracking.
///
/// Every entry in the CFB container must be explicitly consumed (read) or
/// acknowledged (skip_known) during document loading. After loading,
/// assert_all_consumed() verifies nothing was missed.
pub(crate) struct TrackedCfbDocument {
    inner: CfbDocument,
    /// All entries discovered at open time (recursive enumeration).
    all_entries: HashSet<String>,
    /// Entries that have been read or explicitly acknowledged.
    consumed: HashSet<String>,
}

impl TrackedCfbDocument {
    /// Open a CFB file and enumerate all entries for tracking.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let inner = CfbDocument::open(path)?;
        let all_entries = inner.enumerate_all_entries()?;
        Ok(Self {
            inner,
            all_entries,
            consumed: HashSet::new(),
        })
    }

    /// Read a required stream. Marks it as consumed.
    /// Returns Err(StreamNotFound) if the stream does not exist.
    pub fn read_stream(&mut self, path: &str) -> Result<Vec<u8>> {
        self.consumed.insert(path.to_string());
        self.inner.read_stream(path)
    }

    /// Read an optional stream. Marks it as consumed if it exists.
    /// Returns Ok(None) if the stream does not exist.
    pub fn read_stream_optional(&mut self, path: &str) -> Result<Option<Vec<u8>>> {
        self.consumed.insert(path.to_string());
        self.inner.read_stream_optional(path)
    }

    /// Check whether an entry exists. Does NOT mark it as consumed.
    pub fn exists(&self, path: &str) -> bool {
        self.inner.exists(path)
    }

    /// List immediate child entries under a path.
    /// Marks the parent storage as consumed (it has been inspected).
    pub fn list_entries(&mut self, path: &str) -> Result<(Vec<String>, Vec<String>)> {
        self.consumed.insert(path.to_string());
        self.inner.list_entries(path)
    }

    /// Explicitly acknowledge a known stream/storage without reading it.
    ///
    /// Use this for entries that are:
    /// - Known but not yet implemented (must include a TODO comment at call site)
    /// - Known to be irrelevant for our use case (e.g. printer settings)
    /// - Storage nodes that are implicitly consumed by reading their children
    ///
    /// This forces explicit acknowledgement rather than silent ignorance.
    /// The call site documents *why* the stream is being skipped.
    pub fn skip_known(&mut self, path: &str) {
        self.consumed.insert(path.to_string());
    }

    /// Mark multiple entries as consumed at once.
    /// Convenience for acknowledging a batch of known-but-unimplemented streams.
    pub fn skip_known_many(&mut self, paths: &[&str]) {
        for path in paths {
            self.consumed.insert(path.to_string());
        }
    }

    /// FAIL FAST: Error if any entry in the CFB was never consumed or acknowledged.
    ///
    /// Must be called at the end of every document loader. An unknown stream
    /// could carry fabrication-critical data — we must never silently ignore it.
    pub fn assert_all_consumed(&self) -> Result<()> {
        let unconsumed: Vec<_> = self.all_entries
            .difference(&self.consumed)
            .sorted() // deterministic error messages
            .cloned()
            .collect();
        if unconsumed.is_empty() {
            Ok(())
        } else {
            Err(AltiumFormatError::UnconsumedStreams { paths: unconsumed })
        }
    }
}
```

### How this drives the red/green workflow

Stream tracking is the outermost discovery loop:

1. Open a real Altium file → `assert_all_consumed()` fails, listing unknown streams
2. Investigate what those streams contain (ghidra, hex dumps, docs)
3. Either implement the parser (`read_stream` + full parsing) or call `skip_known` with a
   comment explaining why it's safe to skip
4. Run again → passes, or fails on the next unknown stream

This is exactly parallel to how `assert_exhausted()` on `ParameterCollection` drives
discovery of unknown parameter keys within records.

### Storage nodes vs stream nodes

CFB containers have two types of entries: storages (directories) and streams (files).
When the document loader reads all streams under a storage (e.g. `/Arcs6/Header` and
`/Arcs6/Data`), it must also acknowledge the storage node itself (`/Arcs6`). The
`list_entries` method handles this automatically by marking the parent as consumed.
The root storage `/` is always implicitly consumed.

---

## Layer 3: Stream Parsers

**Module**: `block_stream`, `binary_headers`, `wide_strings_tlv`

**Responsibility**: Parse the various stream-level formats that Altium uses within CFB
streams. This is NOT a single format — Altium uses at least five distinct stream-level
encodings. This layer handles framing only, yielding payloads for Layer 4.

### Format A: Block-framed streams (most common)

**Used by**: SchDoc/SchLib record streams, PcbDoc text sections, pin sidecar streams,
Storage streams, all streams with pipe-delimited parameter data.

Every block-framed Altium stream is a sequence of size-prefixed blocks:

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
indicator — it is the embedded object envelope marker (see Layer 4). Layer 3 does NOT
perform decompression. Decompression is the responsibility of the embedded object
envelope parser in Layer 4, which knows from context whether inner data is compressed
(e.g. `/Storage` entries are zlib-compressed, pin sidecar entries are not).

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
/// Validates that the entire stream is consumed (no trailing bytes).
pub(crate) fn parse_blocks(stream_data: &[u8]) -> Result<Vec<Block>>;

/// Parse blocks, returning an iterator for lazy consumption.
pub(crate) fn iter_blocks(stream_data: &[u8]) -> BlockIter<'_>;
```

### Format B: PCB binary record streams

**Used by**: PcbDoc/PcbLib primitive sections (Arcs6, Pads6, Tracks6, Vias6, Texts6,
Fills6, Connections6, Regions6, ShapeBasedRegions6, SplitPlaneRegions6, ComponentBodies6,
ShapeBasedComponentBodies6).

These are NOT block-framed. Each record is:

```
┌───────────────────────────┐
│ u8 object_id              │  TObjectId enum value (1=Arc, 2=Pad, etc.)
├───────────────────────────┤
│ u32 LE record_length      │  payload size (high byte may contain flags;
│                           │  mask with SIZE_FLAG_MASK)
├───────────────────────────┤
│ payload (length bytes)    │  packed little-endian binary struct
└───────────────────────────┘
```

The section's `Header` sub-stream contains a u32 record count. The `Data` sub-stream
contains the packed records. After parsing, the actual record count must match the
header count — mismatch is a hard error.

```rust
/// A raw PCB binary record before dispatch.
pub(crate) struct PcbBinaryRecord {
    pub object_id: u8,
    pub data: Vec<u8>,
}

/// Parse all PCB binary records from a section Data stream.
///
/// Validates that the stream is fully consumed (no trailing bytes).
pub(crate) fn parse_pcb_binary_records(stream_data: &[u8]) -> Result<Vec<PcbBinaryRecord>>;

/// Read the record count from a PCB section Header stream.
/// Header is always exactly 4 bytes: u32 LE count.
pub(crate) fn parse_pcb_section_header(header_data: &[u8]) -> Result<u32>;
```

### Format C: PCB prefixed parameter blocks

**Used by**: PcbDoc sections Rules6, NewRules6, Dimensions6, Coordinates6.

Similar to block-framed but with a u16 prefix before each block:

```
┌───────────────────────────┐
│ u16 LE prefix             │  purpose varies by section
├───────────────────────────┤
│ u32 LE payload_size       │  length of the parameter string
├───────────────────────────┤
│ payload (size bytes)      │  NUL-terminated pipe-delimited parameter string
└───────────────────────────┘
```

```rust
/// A prefixed parameter block from Rules6/Dimensions6/etc.
pub(crate) struct PrefixedParamBlock {
    pub prefix: u16,
    pub data: Vec<u8>,
}

/// Parse all prefixed parameter blocks from a section Data stream.
pub(crate) fn parse_prefixed_param_blocks(stream_data: &[u8]) -> Result<Vec<PrefixedParamBlock>>;
```

### Format D: WideStrings6 binary TLV

**Used by**: PcbDoc `/WideStrings6/Data` only. NOT block-framed.

Binary type-length-value encoding for Unicode string replacement:

```
┌───────────────────────────┐
│ type byte                 │  0x06, 0x0C, 0x12, or 0x14
├───────────────────────────┤
│ length field              │  u8 for type 0x06; u32 LE for others
│                           │  0x12 stores char count, others store byte count
├───────────────────────────┤
│ string data               │  ASCII (0x06/0x0C), UTF-16LE (0x12), UTF-8 (0x14)
└───────────────────────────┘
```

Each entry corresponds to a primitive by position index. The entries replace the
ASCII text field on primitives that have Unicode characters.

```rust
/// A single WideStrings6 TLV entry.
pub(crate) struct WideStringEntry {
    pub text: String,
}

/// Parse the WideStrings6 binary TLV stream.
/// Returns entries indexed by position (0-based).
/// Validates that the entire stream is consumed.
pub(crate) fn parse_wide_strings_tlv(stream_data: &[u8]) -> Result<Vec<WideStringEntry>>;
```

**Critical distinction**: PcbLib per-footprint `WideStrings` uses parameter-block format
(Format A), NOT this TLV format. They are completely different encodings despite similar
names.

### Format E: Binary file headers

**Used by**: PcbDoc `FileHeader`, PcbDoc `FileHeaderSix`, PcbLib `FileHeader`.

These are fixed-layout binary structures, not block-framed.

#### PcbDoc FileHeader (legacy, 24 bytes)

```
┌───────────────────────────┐
│ u32 LE char_count         │  NOTE: character count, not byte count
├───────────────────────────┤
│ UTF-16LE string           │  "PCB 5.0 Binary File" (char_count × 2 bytes)
└───────────────────────────┘
```

Known quirk: the u32 stores the character count (19), not the byte count (38).

#### PcbDoc FileHeaderSix / PcbLib FileHeader (pascal-block format)

```
┌───────────────────────────┐
│ u32 LE outer_length       │  total length of inner block
├───────────────────────────┤
│ u8 string_length          │  pascal string length
├───────────────────────────┤
│ ASCII string (N bytes)    │  version string
├───────────────────────────┤
│ f64 LE version            │  5.01 (always)
├───────────────────────────┤
│ u32 LE outer_length       │  total length of inner block
├───────────────────────────┤
│ u8 string_length          │  pascal string length
├───────────────────────────┤
│ ASCII string (N bytes)    │  UniqueID (8-char alpha for PcbLib, GUID for PcbDoc)
└───────────────────────────┘
```

Version strings:
- PcbDoc: `"PCB 6.0 Binary File"`
- PcbLib: `"PCB 6.0 Binary Library File"`

```rust
/// Parsed PCB file header.
pub(crate) struct PcbFileHeader {
    pub version_string: String,
    pub version: f64,
    pub unique_id: String,
}

/// Parse a PcbDoc FileHeaderSix or PcbLib FileHeader (pascal-block format).
pub(crate) fn parse_pcb_file_header(data: &[u8]) -> Result<PcbFileHeader>;

/// Parse a PcbDoc legacy FileHeader (24-byte UTF-16LE format).
pub(crate) fn parse_pcb_legacy_header(data: &[u8]) -> Result<String>;
```

### Format F: PcbLib footprint parameters

**Used by**: PcbLib per-footprint `Parameters` stream.

```
┌───────────────────────────┐
│ u32 LE payload_length     │  length of the parameter block
├───────────────────────────┤
│ u8 string_length          │  pascal string length (redundant with payload_length)
├───────────────────────────┤
│ Win-1252 param string     │  pipe-delimited key=value pairs
└───────────────────────────┘
```

Both length fields are always consistent (pascal block pattern). After reading,
the parameter string is parsed via `ParameterCollection::from_bytes`.

### Format G: WriteBinaryBlocksData (instruction-tagged envelopes)

**Used by**: SchDoc `ReuseBlocks`, `ReuseBlocksV2`, `HarnessConnectionPointConnector`.

These streams use 0xD0-tagged instruction envelopes (same as embedded objects) but
are NOT standard block-framed streams. They use the `WriteBinaryBlocksData` format
from Delphi's `SchDataEmbeddedObject.WriteData`.

Parsed via `parse_embedded_object_stream` after block framing.

### Format H: SchDoc Files stream (0xE3-tagged)

**Used by**: SchDoc `Files` stream only.

Uses 0xE3-tagged `SchDataFileObject` entries for embedded image parameter model files
and harness layout drawings. This is a distinct envelope format from the 0xD0 embedded
objects.

### PcbLib pattern name prefix

PcbLib footprint Data streams have a Pascal-string pattern name before the packed
binary records. This is a stream-level prefix that the document loader strips before
passing the remaining bytes to the binary record parser.

```rust
/// Read a Pascal-string prefix (u8 length + ASCII bytes) from the start of a
/// byte buffer, returning (name, remaining_bytes).
pub(crate) fn read_pascal_prefix(data: &[u8]) -> Result<(String, &[u8])>;
```

### Record count validation

Several stream formats include an explicit record count that must be validated:

| Source | Count location | Validated against |
|---|---|---|
| SchDoc `FileHeader` block 0 | `Weight` parameter | Number of primitive blocks that follow |
| SchLib per-component `Data` | Loop until `RECORD=0` sentinel | N/A (sentinel-terminated) |
| SchLib pin sidecars | `Weight` in header block | Number of 0xD0 entry blocks |
| PcbDoc section `Header` sub-stream | u32 LE count | Number of records in `Data` sub-stream |
| PcbLib footprint `Header` sub-stream | u32 LE count | Number of records in `Data` sub-stream |

Mismatches are hard errors (`RecordCountMismatch`).

---

## Layer 4: ParameterCollection / BinaryReader

**Module**: `param_collection`, `binary_io`

**Responsibility**: Structured access to the contents of a single block or record payload.
This layer provides the *mechanism* for consumption tracking — it knows what data has been
read and what remains — but does not enforce any policy about unknown fields. That policy
lives in Layer 5.

### ParameterCollection

The primary text serialization format. Pipe-delimited key=value pairs used by SchDoc,
SchLib, PcbDoc text sections (Nets6, Rules6, Classes6, Board6), and pin sidecar streams.

Parameter strings appear in two encodings depending on context:
- **Windows-1252**: Main record blocks (flags=0x00 blocks in Data/FileHeader streams)
- **UTF-16LE**: Pin sidecar streams (PinMiscData, PinWideText, PinSymbolLineWidth,
  PinPackageLength, PinPropagationDelay, PinFunctionData) and PcbLib WideStrings

Both produce the same pipe-delimited `key=value` format once decoded to a Rust String.

#### Parameter string syntax

```
|KEY1=VALUE1|KEY2=VALUE2|...|KEYn=VALUEn|\0
```

| Rule | Detail |
|---|---|
| Delimiter | `\|` between key=value pairs |
| Escaping | `[]` decodes as `\|` (literal pipe); `{}` decodes as `=` (literal equals) |
| Text encoding | Windows-1252 by default |
| Unicode keys | `%UTF8%` prefix on key name indicates UTF-8 encoded value |
| Booleans | `T`/`F` in schematic records; `TRUE`/`FALSE` in PCB text sections |
| Key case | Case-insensitive matching; first occurrence wins for duplicates |
| NUL terminator | Every parameter string ends with `\0` |
| Extended records | `RECORD=254` means actual type is in `RECORDEX` (i32) |

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
    /// - Pipe delimiter splitting (with `[]` → `|` and `{}` → `=` unescaping)
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
    /// The frac key is omitted when zero; range is 0..99_999.
    /// One DXP unit = 100,000 internal units = 10 mils.
    pub fn remove_coord(&mut self, key: &str, frac_key: &str) -> Result<Coord>;

    /// Remove an indexed coordinate array (X1, Y1, X2, Y2, … with COUNT key).
    /// Also removes fractional parts (X1_FRAC, Y1_FRAC, …) when present.
    /// Each coordinate is reconstructed as: integer * 100_000 + frac.
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

Cursor-based binary I/O for packed little-endian data. Used by:
- **PcbDoc/PcbLib**: Primary sections (Arcs6, Pads6, Tracks6, etc.)
- **SchLib**: Binary pin records (flags=0x01 blocks in Data streams)
- **All file types**: Embedded object envelopes (0xD0), sidecar binary payloads (PinFrac,
  PinTextData)

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

    /// Read a Coord (i32 LE in internal units, 10,000 units = 1 mil).
    /// Used for PCB binary coordinates which store values directly in internal units.
    /// NOTE: SchLib binary pins use i16 with different scaling (see coordinate note below).
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

### Coordinate representations

There are three distinct coordinate encodings in Altium files. All ultimately produce the
same `Coord(i32)` in internal units (10,000 = 1 mil), but they arrive via different paths:

| Context | On-disk format | Reconstruction | BinaryReader method |
|---|---|---|---|
| **PCB binary** | i32 LE, already in internal units | value directly | `read_coord()` |
| **Schematic text** | Two params: `KEY=N` + `KEY_FRAC=F` | `N * 100_000 + F` | N/A (ParameterCollection) |
| **SchLib binary pins** | i16 LE in DXP units (1 DXP = 100,000 internal) | `i16 * 100_000` + PinFrac sidecar | `read_i16_le()` + post-processing |

The schematic DXP base unit is `Rt_Schematic.Consts.cBaseUnit = 100_000` internal units
(= 10 mils). The `_FRAC` part ranges 0..99,999. Binary pin coordinates are truncated to
DXP units (i16); the PinFrac sidecar provides the remainder for full precision.

`BinaryReader::read_coord()` is specifically for the PCB case (i32 directly in internal
units). SchLib binary pin coordinates must use `read_i16_le()` and reconstruct manually.

### Embedded object envelope

The `0xD0` embedded object format is used by `/Storage` streams (embedded images) and all
9 SchLib pin sidecar streams. It appears inside binary blocks (flags=0x01) as a framing
layer that wraps stream-specific inner data.

```
0xD0          (1 byte)  embedded object tag
id_length     (1 byte)  length of id string
id            (N bytes) ASCII identifier (e.g. pin index "0", "15", or image filename)
inner_header  (4 bytes) same format as block header: bits[23:0]=size, bits[31:24]=flags
inner_data    (M bytes) format varies by stream (see below)
```

```rust
/// A single entry parsed from a 0xD0 embedded object envelope.
pub(crate) struct EmbeddedObject {
    /// The identifier string (pin index for sidecars, filename for images).
    pub id: String,
    /// Format of the inner data (from inner_header flags).
    pub inner_format: BlockFormat,
    /// The inner payload bytes (NOT decompressed — caller decides based on context).
    pub inner_data: Vec<u8>,
}

/// Parse the 0xD0 envelope from a binary block payload.
pub(crate) fn parse_embedded_object(data: &[u8]) -> Result<EmbeddedObject>;

/// Parse a stream that uses the embedded object envelope pattern:
/// header block (flags=0x00, params with RECORD=0 + Weight) followed by
/// N entry blocks (flags=0x01, each containing a 0xD0 envelope).
///
/// Validates that the number of entry blocks matches the Weight parameter.
///
/// Returns the header params and the parsed envelope entries.
pub(crate) fn parse_embedded_object_stream(
    blocks: &[Block],
) -> Result<(ParameterCollection, Vec<EmbeddedObject>)>;
```

**Decompression is context-dependent**: `/Storage` entries contain zlib-compressed image
data (decompress with `flate2`). Pin sidecar entries are NOT compressed. The caller
(document loader) knows which context it's in and decompresses accordingly.

**Inner data formats by stream**:

| Stream | Inner data format | Parser |
|---|---|---|
| `/Storage` | zlib-compressed image binary | `flate2::decompress` |
| `PinFrac` | 12 bytes: 3 × i32 LE | `BinaryReader` |
| `PinDesc` | u32 LE length + ASCII text | `BinaryReader` |
| `PinTextData` | 2-22 bytes variable binary | `BinaryReader` |
| `PinMiscData` | u32 LE length + UTF-16LE params | `ParameterCollection::from_utf16le_bytes` |
| `PinWideText` | u32 LE length + UTF-16LE params | `ParameterCollection::from_utf16le_bytes` |
| `PinSymbolLineWidth` | u32 LE length + UTF-16LE params | `ParameterCollection::from_utf16le_bytes` |
| `PinPackageLength` | u32 LE length + UTF-16LE params | `ParameterCollection::from_utf16le_bytes` |
| `PinPropagationDelay` | u32 LE length + UTF-16LE params | `ParameterCollection::from_utf16le_bytes` |
| `PinFunctionData` | u32 LE length + UTF-16LE params | `ParameterCollection::from_utf16le_bytes` |

---

## Layer 5: Record Parsing

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
    ├── Text block (flags=0x00)
    │   │
    │   ▼
    │   Parse ParameterCollection from bytes
    │   Read discriminant: RECORD=N param (schematic) or section identity (PCB)
    │   │
    │   ├── RECORD=0 → return None (end sentinel / header block)
    │   ├── RECORD=254 → read RECORDEX for actual type code, dispatch
    │   ├── RECORD=N → dispatch to RecordType::from_params(&mut params)
    │   │               params.assert_exhausted()
    │   │               return Ok(Some(SchRecord))
    │   └── PCB section → dispatch to SectionType::from_params(&mut params)
    │                      params.assert_exhausted()
    │                      return Ok(PcbNet/PcbRule/etc.)
    │
    └── Binary block (flags=0x01)
        │
        ▼
        Create BinaryReader from bytes
        Read discriminant: first byte
        │
        ├── Schematic: binary_code byte (0x02=Pin)
        │   dispatch to SchPin::from_binary(&mut reader)
        │   reader.assert_exhausted()
        │   return Ok(Some(SchRecord::Pin))
        │
        └── PCB: object_id byte + u32 length → sub_reader
            dispatch to PcbType::from_binary(&mut sub_reader)
            sub_reader.assert_exhausted()
            return Ok(PcbRecord)
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

SchLib Data streams contain **mixed-format blocks**: text blocks (flags=0x00) for most
record types, and binary blocks (flags=0x01) for pins. The dispatcher must handle both,
using different discriminant mechanisms:

- **Text blocks**: Record type determined by the `RECORD` parameter key
- **Binary blocks**: Record type determined by the first byte (binary code: `0x02` = pin)

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
    /// Parse a single schematic record from a block.
    ///
    /// Handles both text (flags=0x00) and binary (flags=0x01) blocks:
    ///
    /// - Text: parses ParameterCollection, dispatches on RECORD=N key
    /// - Binary: reads binary code byte, dispatches on code value
    ///
    /// Returns Ok(None) for the RECORD=0 end-of-stream sentinel.
    pub fn from_block(block: &Block) -> Result<Option<Self>> {
        match block.format {
            BlockFormat::Text => {
                let mut params = ParameterCollection::from_bytes(&block.data)?;
                let record_id: i32 = params.remove_required("RECORD")?;

                if record_id == 0 {
                    // RECORD=0 is the end-of-stream sentinel, not a real record.
                    // Remaining params (HEADER, Weight, etc.) are ignored.
                    return Ok(None);
                }

                // Handle extended record types (RECORD=254 → actual code in RECORDEX)
                let effective_id = if record_id == 254 {
                    params.remove_required::<i32>("RECORDEX")?
                } else {
                    record_id
                };

                let record = match effective_id {
                    1 => SchRecord::Component(SchComponent::from_params(&mut params)?),
                    25 => SchRecord::NetLabel(SchNetLabel::from_params(&mut params)?),
                    27 => SchRecord::Wire(SchWire::from_params(&mut params)?),
                    // ... exhaustive match over all known RECORD values
                    _ => return Err(AltiumFormatError::UnknownRecordType(effective_id)),
                };

                params.assert_exhausted()?;
                Ok(Some(record))
            }
            BlockFormat::Binary => {
                let mut reader = BinaryReader::new(&block.data);
                let binary_code = reader.read_u8()?;

                let record = match binary_code {
                    0x02 => SchRecord::Pin(SchPin::from_binary(&mut reader)?),
                    // 0x02 is the only binary code in Data streams.
                    // 0xD0 (embedded object) appears in Storage/sidecar streams
                    // but those are NOT dispatched through SchRecord.
                    _ => return Err(AltiumFormatError::UnknownBinaryCode(binary_code)),
                };

                reader.assert_exhausted()?;
                Ok(Some(record))
            }
        }
    }
}
```

**Note on SchPin**: Pins implement BOTH `FromParams` and `FromBinary` because they can
appear in either format. In SchLib Data streams they are always binary (flags=0x01). In
SchDoc FileHeader streams they could theoretically appear as text (RECORD=2). The pin
struct is the same either way — the binary format stores a subset of fields, with the
remainder filled in from sidecar streams during post-processing.

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
    /// Parse a single PCB record from a binary record payload.
    ///
    /// 1. Creates a BinaryReader over the record data
    /// 2. Dispatches to the concrete type's FromBinary based on object_id
    /// 3. Asserts the reader is exhausted
    /// 4. Returns the polymorphic PcbRecord
    ///
    /// The object_id and length have already been parsed by the Layer 3
    /// binary record parser (parse_pcb_binary_records).
    pub fn from_record(object_id: u8, data: &[u8]) -> Result<Self> {
        let mut reader = BinaryReader::new(data);

        let record = match object_id {
            1 => PcbRecord::Arc(PcbArc::from_binary(&mut reader)?),
            2 => PcbRecord::Pad(PcbPad::from_binary(&mut reader)?),
            3 => PcbRecord::Via(PcbVia::from_binary(&mut reader)?),
            4 => PcbRecord::Track(PcbTrack::from_binary(&mut reader)?),
            // ... exhaustive match over all known object IDs
            _ => return Err(AltiumFormatError::UnknownObjectId(object_id)),
        };

        reader.assert_exhausted()?;  // ← strict validation happens HERE
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

### Pattern 1: Stream of blocks (Layer 3)

The most common pattern. A stream contains N blocks; each block is one record.

```
/FileHeader stream:
  Block 0 → header record (RECORD=0 with HEADER, Weight, MinorVersion, UniqueID)
  Block 1 → sheet record (RECORD=31 with font table, sheet settings)
  Block 2 → first primitive
  ...
  Block N → last primitive
  Block N+1 → end sentinel (RECORD=0)

/Arcs6/Data stream:
  Record 0 → section header (binary, all zeros or metadata)
  Record 1 → first arc
  Record 2 → second arc
  ...
```

**Handled by**: `parse_blocks()` / `parse_pcb_binary_records()` in Layer 3. The document
loader iterates and dispatches each block through Layers 4+5.

**Exhaustion**: The document loader validates record counts (Weight for schematic,
Header count for PCB). Mismatches are hard errors.

### Pattern 2: Indexed parameter families (Layer 4 + derive macros)

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

### Pattern 3: Comma-separated values (Layer 4)

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

### Pattern 4: Binary fixed-size arrays (Layer 4)

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

### Pattern 5: Binary subrecords (Layer 4)

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

### Pattern 6: Embedded object sidecar streams (Document loader)

Some data is split across sidecar streams that use the `0xD0` embedded object envelope.
Entries are **sparse** (only pins with non-default data get an entry) and addressed by
**explicit pin index** in the envelope's `id` field, not by array position.

```
/ComponentName/Data    → [comp, pin0, pin1, pin2, ...]  ← primary records
/ComponentName/PinFrac → header block + entry blocks:
                          entry "0" → 12 bytes (pin 0 frac data)
                          entry "2" → 12 bytes (pin 2 frac data)
                          (pin 1 has no entry — uses defaults)
```

**Handled by**: The document loader using Layer 4's `parse_embedded_object_stream` helper.
The loader:
1. Parses the primary Data stream into records via Layers 3-5
2. Reads each sidecar stream via `parse_embedded_object_stream`
3. For each `EmbeddedObject`, parses `id` as pin index, parses inner data per stream type
4. Merges into the pin record at that index

Sidecar streams bypass Layer 5's record dispatch — they use Layer 4 types directly
(BinaryReader for PinFrac/PinTextData, ParameterCollection for UTF-16LE param streams).

```rust
/// Load a pin sidecar stream and apply it to the component's pins.
fn apply_pin_sidecar(
    cfb: &mut TrackedCfbDocument,
    component_key: &str,
    stream_name: &str,
    pins: &mut [SchPin],
    apply: impl Fn(&EmbeddedObject, &mut SchPin) -> Result<()>,
) -> Result<()> {
    let stream_path = format!("/{component_key}/{stream_name}");
    let Some(stream_data) = cfb.read_stream_optional(&stream_path)? else {
        return Ok(()); // sidecar is optional — skip if absent
    };

    let blocks = parse_blocks(&stream_data)?;
    let (_header, entries) = parse_embedded_object_stream(&blocks)?;

    for entry in &entries {
        let pin_index: usize = entry.id.parse().map_err(|_| {
            AltiumFormatError::InvalidParamValue {
                key: "embedded_object_id".into(),
                detail: format!("expected pin index, got {:?}", entry.id),
            }
        })?;
        apply(entry, &mut pins[pin_index])?;
    }

    Ok(())
}

// Usage: apply PinFrac sidecar
apply_pin_sidecar(cfb, key, "PinFrac", pins, |entry, pin| {
    let mut reader = BinaryReader::new(&entry.inner_data);
    pin.location_x_frac = reader.read_i32_le()?;
    pin.location_y_frac = reader.read_i32_le()?;
    pin.length_frac = reader.read_i32_le()?;
    reader.assert_exhausted()
})?;

// Usage: apply PinWideText sidecar (UTF-16LE params)
apply_pin_sidecar(cfb, key, "PinWideText", pins, |entry, pin| {
    let mut reader = BinaryReader::new(&entry.inner_data);
    let text_len = reader.read_u32_le()? as usize;
    let text_bytes = reader.read_bytes(text_len)?;
    let mut params = ParameterCollection::from_utf16le_bytes(&text_bytes)?;
    if let Some(name) = params.remove_optional::<String>("Name")? {
        pin.name = name;
    }
    if let Some(desig) = params.remove_optional::<String>("Desig")? {
        pin.designator = desig;
    }
    if let Some(desc) = params.remove_optional::<String>("Desc")? {
        pin.description = desc;
    }
    // ... other optional keys ...
    params.assert_exhausted()?;
    reader.assert_exhausted()
})?;
```

### Pattern 7: PcbDoc global sidecar streams (Document loader)

PcbDoc has its own sidecar pattern using different formats:
- `/WideStrings6/Data`: Binary TLV (Layer 3 Format D) with type codes 0x06/0x0C/0x12/0x14
- `/UniqueIDPrimitiveInformation/Data`: ParameterCollection blocks with `PRIMITIVEINDEX`
- `/ExtendedPrimitiveInformation/Data`: ParameterCollection blocks with `PRIMITIVEINDEX`
- `/PrimitiveGuids/Data`: Binary array of 24-byte structs per primitive

These match by `PRIMITIVEINDEX` or position to the primitive's index in its section, not
by 0xD0 envelope. They are parsed by the document loader using Layer 3 + Layer 4 directly.

### Summary of collection patterns

| Pattern | Where | Example | Parsed by |
|---|---|---|---|
| Stream of blocks | Multiple blocks in stream | SchDoc primitives, PcbDoc text sections | Layer 3 `parse_blocks` + Layer 5 dispatch |
| PCB binary records | Packed records in Data stream | PcbDoc arcs, pads, tracks | Layer 3 `parse_pcb_binary_records` + Layer 5 dispatch |
| Indexed param family | Keys in one ParameterCollection | Vertices, font table, component index | Layer 4 `remove_indexed` + derive macro |
| Comma-separated list | Single parameter value | Pin delay values | Layer 4 `remove_list` |
| Binary fixed array | Contiguous in binary record | Pad per-layer shapes (×32) | Layer 4 `read_array` + derive macro |
| Binary subrecords | Length-prefixed within record | PcbPad subrecords | Layer 4 `sub_reader` + record's `FromBinary` |
| Embedded object sidecars | 0xD0 envelopes, matched by id | PinFrac, PinWideText, Storage | Layer 4 `parse_embedded_object_stream` + loader |
| PcbDoc global sidecars | Blocks/TLV matched by index | WideStrings6, UniqueIDs, PrimitiveGuids | Layer 3 parsers + loader |

---

## Stream manifests: What each file type contains

These tables document every stream/storage in each file type. The document loader must
read or `skip_known` every one of them. `assert_all_consumed()` enforces this.

### SchDoc streams

| Stream | Format | Required | Load order |
|---|---|---|---|
| `/FileHeader` | Block-framed params (Format A) | Yes | 1 |
| `/Storage` | Embedded object stream (0xD0, zlib) | Yes | 2 |
| `/ReuseBlocks` | WriteBinaryBlocksData (Format G) | No | 3 |
| `/ReuseBlocksV2` | WriteBinaryBlocksData (Format G) | No | 4 |
| `/HarnessConnectionPointConnector` | WriteBinaryBlocksData (Format G) | No | 5 |
| `/Additional` | Block-framed params (Format A) | No | 6 |
| `/ObjectDefinitions` | Block-framed params (Format A) | No | 7 |
| `/ReuseBlockInfos` | Block-framed params (Format A) | No | 8 |
| `/Files` | 0xE3-tagged file objects (Format H) | No | — |

**FileHeader block structure**:
- Block 0: Header (RECORD=0, HEADER string, Weight, MinorVersion=13, UniqueID)
- Block 1: Sheet record (RECORD=31, font table via FontIdCount, sheet settings)
- Blocks 2..N: Schematic primitives (flat list, parent-child via OwnerIndex)
- Final block: End sentinel (RECORD=0)

`Weight` in block 0 gives the exact count of records to follow (including block 1).
The loader must verify the actual count matches Weight.

### SchLib streams

**Root-level:**

| Stream | Format | Required |
|---|---|---|
| `/FileHeader` | Single block-framed param block | Yes |
| `/Storage` | Embedded object stream (0xD0, zlib) | Yes |
| `/SectionKeys` | Block-framed params | No (needed if any component name > 31 chars) |
| `/LibAdditional` | Block-framed params (header only) | No |

**Per-component** (`/<SectionKey>/`):

| Stream | Format | Required |
|---|---|---|
| `Data` | Block-framed mixed text/binary | Yes (identifies a canonical component) |
| `Additional` | Block-framed params | No |
| `Redirection` | Single param block | No (alias components only — replaces Data) |
| `PinFrac` | Embedded object (binary, 12 bytes/pin) | No |
| `PinDesc` | Embedded object (length-prefixed ASCII) | No |
| `PinMiscData` | Embedded object (UTF-16LE params) | No |
| `PinTextData` | Embedded object (binary, 2-22 bytes) | No |
| `PinWideText` | Embedded object (UTF-16LE params) | No |
| `PinSymbolLineWidth` | Embedded object (UTF-16LE params) | No |
| `PinPackageLength` | Embedded object (UTF-16LE params) | No |
| `PinPropagationDelay` | Embedded object (UTF-16LE params) | No |
| `PinFunctionData` | Embedded object (UTF-16LE params) | No |

**Component discovery**: Enumerate storages under root via `list_entries("/")`. A storage
is a canonical component if it contains a `Data` stream. A storage is an alias if it
contains a `Redirection` stream. System storages (`Storage`) are skipped.

**SectionKeys mapping**: OLE storage names are limited to 31 characters. Components with
longer names use an obfuscated key in the storage name. `/SectionKeys` maps display names
to storage keys.

**Pin sidecar load order** (must be applied in this exact sequence):
1. PinFrac → 2. PinDesc → 3. PinMiscData → 4. PinTextData → 5. PinWideText →
6. PinSymbolLineWidth → 7. PinPackageLength → 8. PinPropagationDelay → 9. PinFunctionData

**PinWideText is authoritative**: It is loaded after PinDesc and fully replaces the
Name/Designator/Description fields that PinDesc may have set.

### PcbDoc streams

**Root-level headers:**

| Stream | Format | Required |
|---|---|---|
| `/FileHeader` | Binary 24-byte UTF-16LE (Format E) | Yes |
| `/FileHeaderSix` | Binary pascal-block (Format E) | Yes |

**Primitive sections** (binary records, Format B — each has `/Header` + `/Data`):

| Storage | TObjectId | Notes |
|---|---|---|
| `Arcs6` | 1 (Arc) | |
| `Pads6` | 2 (Pad) | |
| `Vias6` | 3 (Via) | |
| `Tracks6` | 4 (Track) | |
| `Texts6` | 5 (Text) | |
| `Fills6` | 6 (Fill) | |
| `Connections6` | 7 (Connection) | Transient ratsnest data |
| `Regions6` | 11 (Region) | |
| `ShapeBasedRegions6` | 11 | Shape-based variant |
| `SplitPlaneRegions6` | 11 | Split plane variant |
| `ComponentBodies6` | 12 (ComponentBody) | |
| `ShapeBasedComponentBodies6` | 12 | Shape-based variant |

**Text parameter sections** (block-framed, Format A — each has `/Header` + `/Data`):

| Storage | Content |
|---|---|
| `Board6` | Board-level settings, metadata, feature flags |
| `Nets6` | Net definitions |
| `Components6` | Component instances |
| `Polygons6` | Polygon pour definitions |
| `Classes6` | Object class definitions |
| `DifferentialPairs6` | Differential pair definitions |
| `FromTos6` | From-to/ratsnest definitions |
| `EmbeddedBoards6` | Embedded board array definitions |
| `Embeddeds6` | Embedded objects |

**Prefixed parameter sections** (Format C — each has `/Header` + `/Data`):

| Storage | Content |
|---|---|
| `Rules6` | Design rules |
| `NewRules6` | Extended design rules |
| `Dimensions6` | Dimension annotations |
| `Coordinates6` | Coordinate annotations |

**Sidecar streams** (loaded after all primitives):

| Storage | Format | Content |
|---|---|---|
| `WideStrings6` | Binary TLV (Format D) | Unicode string replacements |
| `UniqueIDPrimitiveInformation` | Block-framed params | Per-primitive unique IDs |
| `ExtendedPrimitiveInformation` | Block-framed params | Mask expansion overrides |
| `PrimitiveGuids` | Raw binary (24 bytes/entry) | Primitive GUID assignments |

**Settings/metadata sections** (block-framed params):

| Storage | Content |
|---|---|
| `Advanced Placer Options6` | Auto-placer settings |
| `Advanced Router Options6` | Auto-router settings |
| `Design Rule Checker Options6` | DRC settings |
| `Pin Swap Options6` | Pin-swap settings |
| `PadViaLibrary` | Pad/via template library |
| `PadViaLibraryCache` | Pad/via template cache |
| `PadViaLibraryLinks` | Pad/via template links |
| `PinPairsSection` | Pin pair definitions |
| `SignalClasses` | Signal class definitions |
| `SmartUnions` | Smart union definitions |
| `UnionRelations` | Union relation mappings |
| `WaivedViolations` | Waived DRC violations |
| `PrimitiveParameters` | Primitive parameter overrides |

**Raw binary sections:**

| Storage | Content |
|---|---|
| `EmbeddedFonts6` | Embedded font data |
| `FileVersionInfo` | File version history |
| `LayerKindMapping` | Mechanical layer kind map |
| `ModelsNoEmbed` | Model references without embedded data |
| `Textures` | Texture image data |
| `UnionNames` | Union name strings |
| `ConstraintManager` | Constraint manager data |

**Models section** (special structure):

| Stream | Content |
|---|---|
| `Models/Header` | u32 count |
| `Models/Data` | Model metadata parameter blocks |
| `Models/0` .. `Models/N` | Raw 3D model binary blobs (STEP etc.) |

**Primary load order** (from `RegisterAllSectionsForExporting`, 23 sections):
1-7: Board6, ECO Options6, Output Options6, Printer Options6, Gerber Options6,
     Advanced Placer Options6, Design Rule Checker Options6
8-17: Classes6, Nets6, Components6, Polygons6, Dimensions6, Coordinates6,
      Connections6, Rules6, FromTos6, Embeddeds6
18-23: Arcs6, Pads6, Vias6, Tracks6, Texts6, Fills6

Then sidecars: WideStrings6 → UniqueIDPrimitiveInformation →
ExtendedPrimitiveInformation → PrimitiveGuids

### PcbLib streams

**Root-level:**

| Stream | Format | Required |
|---|---|---|
| `/FileHeader` | Binary pascal-block (Format E) | Yes |
| `/SectionKeys` | Binary (u32 count + name/key pairs) | No |

**Library-global** (`/Library/`):

| Stream | Content |
|---|---|
| `Library/Header` | u32 count |
| `Library/Data` | Library-wide parameter data |
| `Library/EmbeddedFonts` | Binary font data |
| `Library/ComponentParamsTOC/Header` + `Data` | Component parameter TOC |
| `Library/LayerKindMapping/Header` + `Data` | Mechanical layer kind mapping |
| `Library/Models/Header` + `Data` + `0..N` | 3D model pool |
| `Library/ModelsNoEmbed/Header` + `Data` | Model refs without blobs |
| `Library/PadViaLibrary/Header` + `Data` | Pad/via templates |
| `Library/Textures/Header` + `Data` | Texture data |

**Per-footprint** (`/<FootprintName>/` or obfuscated `/<Key>/`):

| Stream | Format | Required |
|---|---|---|
| `Parameters` | Binary pascal-block params (Format F) | Yes |
| `Header` | u32 count + version info | Yes |
| `Data` | Pattern name prefix + packed binary records (Format B) | Yes |
| `WideStrings` | Block-framed params (Format A, NOT binary TLV!) | No |
| `PrimitiveGuids/Header` + `Data` | Binary, 24 bytes/entry | No |
| `UniqueIDPrimitiveInformation/Header` + `Data` | Block-framed params | No |
| `ExtendedPrimitiveInformation/Header` + `Data` | Block-framed params | No |

**Footprint discovery**: Enumerate storages under root. A storage is a footprint if it
is NOT one of the system names (`FileHeader`, `SectionKeys`, `Library`, `FileVersionInfo`)
AND contains a `Data` sub-stream.

**Critical format distinction**: PcbLib per-footprint `WideStrings` uses parameter-block
format (Format A) with `ENCODEDTEXT{N}=comma,separated,integers`. This is completely
different from PcbDoc's `WideStrings6` binary TLV (Format D).

---

## Composition: How layers work together

### Example: Loading a SchDoc

```rust
impl SchDoc {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        // Layer 1+2: Open CFB container with tracking
        let mut cfb = TrackedCfbDocument::open(path)?;

        // Layer 2+3: Read and parse the FileHeader stream
        let stream_data = cfb.read_stream("/FileHeader")?;
        let blocks = parse_blocks(&stream_data)?;

        // Block 0 is the header record (Weight, MinorVersion, etc.)
        let mut header_params = ParameterCollection::from_bytes(&blocks[0].data)?;
        let weight: usize = header_params.remove_required("Weight")?;
        // ... parse remaining header fields ...
        header_params.assert_exhausted()?;

        // Block 1 is the sheet record (RECORD=31, font table, etc.)
        let sheet = SchRecord::from_block(&blocks[1])?;

        // Blocks 2..N are the schematic primitives
        let mut records = Vec::new();
        for block in &blocks[2..] {
            if let Some(record) = SchRecord::from_block(block)? {
                records.push(record);
            }
        }

        // Validate record count matches Weight
        if records.len() + 1 != weight {  // +1 for sheet record
            return Err(AltiumFormatError::RecordCountMismatch {
                section: "FileHeader".into(),
                expected: weight,
                actual: records.len() + 1,
            });
        }

        // Read sidecar streams (all tracked by Layer 2)
        let storage_data = cfb.read_stream("/Storage")?;
        // ... parse embedded images ...

        // Read optional streams (tracked even if absent)
        let additional = cfb.read_stream_optional("/Additional")?;
        let object_defs = cfb.read_stream_optional("/ObjectDefinitions")?;
        let reuse_block_infos = cfb.read_stream_optional("/ReuseBlockInfos")?;
        let reuse_blocks = cfb.read_stream_optional("/ReuseBlocks")?;
        let reuse_blocks_v2 = cfb.read_stream_optional("/ReuseBlocksV2")?;
        let harness = cfb.read_stream_optional("/HarnessConnectionPointConnector")?;
        let files = cfb.read_stream_optional("/Files")?;
        // ... parse each if present ...

        // FAIL FAST: verify every stream in the CFB was accounted for
        cfb.assert_all_consumed()?;

        // Post-processing: build ownership tree from OWNERINDEX values
        // Post-processing: validate all OwnerIndex references

        Ok(SchDoc { header, records })
    }
}
```

### Example: Loading a SchLib component

```rust
/// Load a single component from a SchLib Data stream.
/// Demonstrates mixed text/binary block handling and sidecar merging.
fn load_component(
    cfb: &mut TrackedCfbDocument,
    component_key: &str,
    component_base_offset: usize,
) -> Result<(SchComponent, Vec<SchRecord>)> {
    let stream_data = cfb.read_stream(&format!("/{component_key}/Data"))?;
    let blocks = parse_blocks(&stream_data)?;

    // Block 0: SchComponent (always text, always first)
    let mut params = ParameterCollection::from_bytes(&blocks[0].data)?;
    let _record_id: i32 = params.remove_required("RECORD")?; // always 1
    let component = SchComponent::from_params(&mut params)?;
    params.assert_exhausted()?;

    // Blocks 1..N: mixed text (RECORD=N) and binary (code=0x02 pins)
    let mut records = Vec::new();
    let mut pins = Vec::new();
    for block in &blocks[1..] {
        match SchRecord::from_block(block)? {
            None => break, // RECORD=0 end sentinel
            Some(SchRecord::Pin(pin)) => {
                pins.push(pin);
                records.push(SchRecord::Pin(pin));
            }
            Some(record) => {
                // Adjust relative OwnerIndex to absolute
                // record.owner_index += component_base_offset;
                records.push(record);
            }
        }
    }

    // Phase 2: Apply pin sidecar streams in exact order
    // (PinFrac, PinDesc, PinMiscData, PinTextData, PinWideText,
    //  PinSymbolLineWidth, PinPackageLength, PinPropagationDelay, PinFunctionData)
    apply_pin_sidecar(cfb, component_key, "PinFrac", &mut pins, |entry, pin| {
        let mut reader = BinaryReader::new(&entry.inner_data);
        pin.location_x += reader.read_i32_le()?; // additive frac
        pin.location_y += reader.read_i32_le()?;
        pin.pin_length += reader.read_i32_le()?;
        reader.assert_exhausted()
    })?;
    // ... remaining 8 sidecars ...

    // Read optional Additional stream
    let _additional = cfb.read_stream_optional(
        &format!("/{component_key}/Additional")
    )?;

    // Note: Redirection stream is checked during component discovery,
    // not here (alias components don't have Data streams)

    Ok((component, records))
}
```

### Example: Loading a PcbDoc section

```rust
fn load_pcb_binary_section(
    cfb: &mut TrackedCfbDocument,
    section: &str,
) -> Result<Vec<PcbRecord>> {
    // Layer 2+3: Read header for expected record count
    let header_data = cfb.read_stream(&format!("/{section}/Header"))?;
    let expected_count = parse_pcb_section_header(&header_data)?;

    // Layer 2+3: Read and parse binary records
    let data = cfb.read_stream(&format!("/{section}/Data"))?;
    let raw_records = parse_pcb_binary_records(&data)?;

    // Validate record count (first record is section header, skip it)
    let primitive_records = &raw_records[1..];
    if primitive_records.len() != expected_count as usize {
        return Err(AltiumFormatError::RecordCountMismatch {
            section: section.into(),
            expected: expected_count as usize,
            actual: primitive_records.len(),
        });
    }

    // Layer 5: Dispatch each record to its typed parser
    let mut records = Vec::new();
    for raw in primitive_records {
        let record = PcbRecord::from_record(raw.object_id, &raw.data)?;
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

    // Layer 2 (stream consumption tracking)
    #[error("Unconsumed streams/storages in CFB container: {paths:?}")]
    UnconsumedStreams { paths: Vec<String> },

    // Layer 3 (stream parsing)
    #[error("Invalid block header at offset {offset}: {detail}")]
    InvalidBlockHeader { offset: usize, detail: String },
    #[error("Record count mismatch in {section}: expected {expected}, got {actual}")]
    RecordCountMismatch { section: String, expected: usize, actual: usize },

    // Layer 4 (structured data access)
    #[error("Missing required parameter: {0}")]
    MissingParam(String),
    #[error("Decompression failed: {0}")]
    DecompressionError(String),
    #[error("Invalid parameter value for key '{key}': {detail}")]
    InvalidParamValue { key: String, detail: String },
    #[error("Binary read past end: needed {needed} bytes at offset {offset}, only {available} remain")]
    BinaryReadPastEnd { offset: usize, needed: usize, available: usize },

    // Layer 4 (embedded object envelope)
    #[error("Invalid embedded object: {0}")]
    InvalidEmbeddedObject(String),

    // Layer 5 (strict validation)
    #[error("Unknown record type: {0}")]
    UnknownRecordType(i32),
    #[error("Unknown PCB object ID: {0}")]
    UnknownObjectId(u8),
    #[error("Unknown binary code in schematic block: 0x{0:02X}")]
    UnknownBinaryCode(u8),
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
| **All streams accounted for** | **2** | **`assert_all_consumed()` returns `Err(UnconsumedStreams)`** |
| Block framing is valid | 3 | `parse_blocks` validates headers, sizes |
| Binary record framing valid | 3 | `parse_pcb_binary_records` validates object_id, lengths |
| PCB header version valid | 3 | `parse_pcb_file_header` validates version string |
| Record count matches header | 3 | Loader validates `Weight` / `Header` count against actual |
| WideStrings6 TLV valid | 3 | `parse_wide_strings_tlv` validates type codes, lengths |
| Embedded object envelope valid | 4 | `parse_embedded_object` validates 0xD0 tag, lengths |
| Sidecar entry count matches Weight | 4 | `parse_embedded_object_stream` validates |
| Decompression succeeds | 4 | `flate2` returns `Err` on corrupt zlib (called by loader for Storage) |
| Parameter syntax is valid | 4 | `ParameterCollection::from_bytes` validates encoding + delimiters |
| Parameter escaping correct | 4 | `from_bytes` handles `[]` → `\|` and `{}` → `=` |
| Parameter value parses to type | 4 | `remove_required` / `FromParamValue` returns `Err` |
| Binary data has enough bytes | 4 | `BinaryReader::read_*` checks `remaining()` |
| Record type is known (text) | 5 | Dispatch `match` on RECORD=N — returns `Err(UnknownRecordType)` |
| Extended record type handled | 5 | RECORD=254 → read RECORDEX, dispatch on effective ID |
| Binary code is known (sch) | 5 | Dispatch `match` on binary code — returns `Err(UnknownBinaryCode)` |
| Object ID is known (pcb) | 5 | Dispatch `match` on object_id — returns `Err(UnknownObjectId)` |
| End sentinel handled | 5 | RECORD=0 returns `Ok(None)`, not dispatched as unknown |
| All fields consumed | 5 | Dispatcher calls `assert_exhausted()` after `FromParams`/`FromBinary` |
| All bytes consumed | 5 | Dispatcher calls `assert_exhausted()` on sub-reader |
