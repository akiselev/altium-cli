# Clean-Slate Architecture: Starting from the File Format

## What an Altium File Actually Is

An Altium file is a CFB (Microsoft Compound File Binary) container — a filesystem-inside-a-file with storages (directories) and streams (files). Each file type uses a specific tree layout:

**SchLib:**
```
/FileHeader          → component index params
/SectionKeys         → long-name → storage-key mapping
/{ComponentName}/Data → sequence of record frames (component + pins + graphics)
```

**SchDoc:**
```
/FileHeader          → sequence of record frames (all primitives, flat)
/Additional          → echo of FileHeader metadata
```

**PcbLib:**
```
/FileHeader          → version strings
/Library/Data        → board params + footprint index
/{FootprintName}/Data       → [u8 object_id][binary block] per primitive
/{FootprintName}/Parameters → footprint properties
/{FootprintName}/WideStrings → Unicode text
```

**PcbDoc:**
```
/Board6/Data         → board parameters
/Tracks6/Data        → [u8 4][binary block] per track
/Vias6/Data          → [u8 3][binary block] per via
/Pads6/Data          → [u8 2][multi-block] per pad
/Polygons6/Data      → [param block] per polygon (no object_id!)
/Dimensions6/Data    → [u8 version][u8 flags][param block] per dimension
/Nets6/Data          → [param block] per net
/Rules6/Data         → [binary block] per rule
...
```

Inside each data stream, records are framed as:

```
[i32 size_with_flags][data bytes]

Bits 0-23:  data size
Bits 24-31: flags (0x00 = ASCII params, 0x01 = binary)
```

ASCII data is pipe-delimited: `|RECORD=2|NAME=VCC|ELECTRICAL=6|...\0`

Binary data is sequential typed fields in a fixed order determined by record type.

That's the entire file format. Everything else is built on top.

## The Architecture That Falls Out

The file format has three natural layers. The architecture mirrors them exactly:

```
┌──────────────────────────────────────────────────────┐
│ Layer 3: Typed Records + Operations                   │
│                                                       │
│   SchPin, SchComponent, PcbPad, PcbTrack, ...         │
│   RecordTree (parent-child from owner_index)           │
│   Queries, BOM, netlist, edits — pure functions        │
│                                                       │
│   THIS IS THE FUNCTIONAL CORE                          │
│   Input: typed records. Output: typed records/results. │
│   No bytes, no params, no CFB, no files.               │
├──────────────────────────────────────────────────────┤
│ Layer 2: Record Codec                                  │
│                                                       │
│   parse:     &[u8] → SchPin     (bytes to typed)       │
│   serialize: SchPin → Vec<u8>   (typed to bytes)       │
│                                                       │
│   Uses v2 field knowledge (50+ pin fields, correct     │
│   names, correct coord scale). Uses v2 SchSerializer   │
│   for complex records. Uses derive macros for simple.   │
│   Captures remaining unknown params for roundtrip.     │
├──────────────────────────────────────────────────────┤
│ Layer 1: Container                                     │
│                                                       │
│   open(path) → in-memory CFB                           │
│   read_stream(path) → Vec<RecordFrame>                 │
│   write_stream(path, Vec<RecordFrame>)                 │
│   save(path) → file on disk                            │
│                                                       │
│   RecordFrame = original bytes + optional parsed record │
│                                                       │
│   THIS IS THE IMPERATIVE SHELL                         │
│   Owns bytes. Owns files. Owns the CFB.                │
└──────────────────────────────────────────────────────┘
```

No GenericRecord. No TypedAccessor. No ChangeSet. No RecordRef handles. No dual representation. Three layers, each doing one thing.

## Layer 1: Container

The container opens a CFB file and gives you access to streams as sequences of record frames.

```rust
/// A record frame: the raw bytes of one record in a stream.
/// This is the unit of nondestructive editing — if you don't
/// touch a frame, its bytes are written back verbatim.
struct RecordFrame {
    /// Original bytes (including the size_with_flags header).
    /// None if this frame was created in memory (new record).
    raw: Option<Vec<u8>>,

    /// Parsed typed record, if requested.
    /// Populated lazily on first access.
    parsed: Option<SchRecord>,  // or PcbRecord for PCB files
}
```

The container knows about file-type-specific layouts (SchLib has per-component storages, PcbDoc has type-specific streams). But it doesn't know what's inside the records — that's Layer 2.

```rust
/// Opened Altium file. Owns the CFB and all byte data.
struct AltiumFile {
    cfb: CompoundFile<Cursor<Vec<u8>>>,
}

impl AltiumFile {
    fn open(path: &Path) -> Result<Self>;
    fn save(&self, path: &Path) -> Result<()>;

    /// Read a stream as raw bytes.
    fn read_stream(&mut self, stream_path: &str) -> Result<Vec<u8>>;

    /// Write a stream from raw bytes.
    fn write_stream(&mut self, stream_path: &str, data: &[u8]) -> Result<()>;
}
```

File-type-specific wrappers know the CFB layout:

```rust
struct SchLibFile {
    file: AltiumFile,
    component_names: Vec<String>,      // from FileHeader
    section_keys: HashMap<String, String>, // long name → storage key
    header_params: Vec<u8>,            // raw FileHeader bytes (preserved)
}

impl SchLibFile {
    fn open(path: &Path) -> Result<Self>;

    /// Get the raw record frames for a component's Data stream.
    fn component_frames(&mut self, name: &str) -> Result<Vec<RecordFrame>>;

    /// Replace a component's Data stream with new frames.
    fn set_component_frames(&mut self, name: &str, frames: Vec<RecordFrame>) -> Result<()>;

    fn save(&mut self, path: &Path) -> Result<()>;
}
```

**Nondestructive mechanism**: `save()` writes each stream. For streams that haven't been touched via `set_component_frames()`, the original bytes are written. For streams that have been replaced, the new frames are written — but each frame with `raw: Some(bytes)` writes those original bytes. Only frames where `raw: None` (new) or where the parsed record has been modified and re-serialized get new bytes.

This is the entire nondestructive strategy. No ChangeSet. No dirty flags. Just: does this frame have original bytes? Write them. Does it have new bytes? Write those.

## Layer 2: Record Codec

The codec parses raw frame bytes into typed records and serializes typed records back to bytes.

```rust
/// Parse a record frame's data bytes into a typed schematic record.
fn parse_sch_record(data: &[u8], flags: u8) -> Result<SchRecord> {
    if flags & BLOCK_FLAG_BINARY != 0 {
        // Binary pin format (SchLib only)
        parse_binary_pin(data)
    } else {
        // ASCII parameter format
        let params = ParameterCollection::from_bytes(data);
        SchRecord::from_params(&params)
    }
}

/// Serialize a typed schematic record back to frame data bytes.
fn serialize_sch_record(record: &SchRecord) -> (Vec<u8>, u8) {
    // Returns (data_bytes, flags)
    let params = record.to_params();
    (params.to_bytes(), 0x00) // Always serialize as ASCII
}
```

**This is where v1 and v2 knowledge merges.** The typed record structs use v2's comprehensive field coverage (50+ pin fields, correct names like `SwapIdPin` not `SwapIdGroup`, 100K schematic coords). The serialization uses v2's `SchSerializer` for complex records (Pin, Component) and derive-generated `ToParams` for simple records (Wire, Junction, Label).

Unknown field preservation lives here too, but it's simple:

```rust
struct SchPin {
    // ... all 50+ known fields from v2 ...

    /// Parameters present in the file that we don't have typed fields for.
    /// Captured during parse, replayed during serialize.
    unknown_params: Vec<(String, String)>,  // (key, value) in original order
}
```

During parse: after extracting all known fields from the param string, any remaining key-value pairs go into `unknown_params`. During serialize: after writing all known fields, append the unknown params. This is what v1's `UnknownFields` does, but applied to ALL record types (v1 only used it on SchComponent, not SchPin — that's a bug, not a design choice).

**The codec is the only place serialization concerns exist.** Layer 3 never sees bytes, params, or `ParameterCollection`. Layer 1 never sees typed records (it just holds frames).

## Layer 3: Typed Records + Operations (The Functional Core)

This is everything above the codec. Pure functions on typed data. No bytes, no files, no CFB.

### Data types

The typed records are the same ones we have today — `SchPin`, `SchComponent`, `SchWire`, `PcbPad`, `PcbTrack`, etc. — but with v2's complete field coverage and correct field names. The existing 30+ schematic and 15+ PCB record types, the `SchRecord`/`PcbRecord` dispatch enums, the `SchPrimitive`/`PcbPrimitive` traits — all reused. This is proven, tested code.

The hierarchy is a `RecordTree` built from `owner_index` fields. Also proven, tested.

### Operations

Every operation is a pure function. Two kinds:

**Queries** (read-only, return results):
```rust
fn bom(records: &[SchRecord], tree: &RecordTree<SchRecord>) -> SchDocBom;
fn netlist(records: &[SchRecord]) -> SchDocNetlist;
fn power_map(records: &[SchRecord], tree: &RecordTree<SchRecord>) -> SchDocPowerMap;
fn components(records: &[SchRecord], tree: &RecordTree<SchRecord>) -> Vec<ComponentInfo>;
fn query(records: &[SchRecord], tree: &RecordTree<SchRecord>, selector: &str) -> Vec<QueryMatch>;
```

**Transforms** (produce modified record lists):
```rust
fn move_component(records: &[SchRecord], designator: &str, to: CoordPoint)
    -> Result<Vec<SchRecord>, EditError>;
fn add_wire(records: &[SchRecord], vertices: Vec<CoordPoint>)
    -> Vec<SchRecord>;
fn delete_component(records: &[SchRecord], tree: &RecordTree<SchRecord>, designator: &str)
    -> Result<Vec<SchRecord>, EditError>;
```

That's it. No `DocState`, no `Snapshot`, no `Document` reference. Just `&[SchRecord]` in, results or `Vec<SchRecord>` out.

The `RecordTree` is computed from records when needed. It's cheap (O(n) scan of owner_index values). If you're doing multiple queries on the same data, compute it once and pass it around:

```rust
let records: Vec<SchRecord> = /* parsed from file */;
let tree = RecordTree::from_records(&records);

let bom = bom(&records, &tree);
let netlist = netlist(&records);
let components = components(&records, &tree);
```

## How They Compose: The Full Flow

### Read-only (inspect, query, analyze)

```rust
// Shell (imperative)
let mut file = SchDocFile::open("design.SchDoc")?;
let frames = file.record_frames()?;
let records: Vec<SchRecord> = frames.iter()
    .map(|f| parse_sch_record(&f.data, f.flags))
    .collect::<Result<_>>()?;

// Core (functional)
let tree = RecordTree::from_records(&records);
let bom = ops::bom(&records, &tree);

// Shell (imperative)
output::print(&bom, format)?;
```

### Edit (modify and save)

```rust
// Shell (imperative) — open and parse
let mut file = SchDocFile::open("design.SchDoc")?;
let frames = file.record_frames()?;
let records: Vec<SchRecord> = frames.iter()
    .map(|f| parse_sch_record(&f.data, f.flags))
    .collect::<Result<_>>()?;

// Core (functional) — transform
let modified = ops::move_component(&records, "U1", CoordPoint::from_mils(500.0, 300.0))?;

// Shell (imperative) — diff and save
let new_frames = reconcile(&frames, &records, &modified);
file.set_record_frames(new_frames)?;
file.save("design.SchDoc")?;
```

The `reconcile` function is the bridge between the functional core and the nondestructive shell:

```rust
/// Produce new record frames by comparing original and modified record lists.
/// Records that didn't change keep their original bytes (nondestructive).
/// Records that changed get re-serialized through the codec.
/// New records get serialized from scratch.
fn reconcile(
    original_frames: &[RecordFrame],
    original_records: &[SchRecord],
    modified_records: &[SchRecord],
) -> Vec<RecordFrame> {
    // For each record in modified:
    //   - If it exists at the same index in original AND is equal:
    //     keep original frame (raw bytes preserved)
    //   - If it exists but changed:
    //     re-serialize → new frame with raw = None
    //   - If it's new (index beyond original length, or inserted):
    //     serialize from scratch
    //
    // For records in original but not in modified:
    //   - Omitted (deleted)
}
```

**The reconcile function is the ONLY place where "which records changed" matters.** And it's simple because:
- Records have a natural ordering (their position in the stream)
- Schematic edits (move, add, delete) produce predictable diffs
- `PartialEq` on record types is already derived

For the common case — changing a few fields on a few records without inserting or deleting — reconcile is just a zip of original and modified, comparing element-wise. `O(n)` with no hashing, no ID tracking, no HashMap.

For structural edits (inserting/deleting records), the modified list's length differs from the original. Here you need to match records. The simplest approach: records have a `unique_id` field (most Altium records include `UNIQUEID` in their params). Match by unique_id when present, fall back to position when not. This covers 99% of cases because:
- Components always have unique_id
- Pins inherit their parent's identity via owner_index
- New records (from `add_wire`, `add_component`) don't have original frames to match against — they're always serialized fresh

## What Gets Reused

Almost everything we've built. This isn't a rewrite — it's a reorganization.

**Reused verbatim (proven by tests):**

| What | Where | Why it stays |
|------|-------|-------------|
| All 30+ schematic record types | `records/sch/*.rs` | Battle-tested structs, just add missing v2 fields |
| All 15+ PCB record types | `records/pcb/*.rs` | Same |
| `SchRecord` / `PcbRecord` dispatch enums | `records/*/primitive.rs` | Polymorphism works |
| `SchPrimitive` / `PcbPrimitive` traits | `traits/mod.rs` | Eliminates 85+ match statements |
| `RecordTree` | `tree/mod.rs` | Owner-index hierarchy, proven |
| `SelectorEngine` + `SchQL` query engine | `query/*.rs` | 57 tests, don't touch |
| `FootprintBuilder` | `footprint/*.rs` | PCB footprint creation |
| `ParameterCollection` | `types/parameters.rs` | Param parsing/serialization |
| `FromParams` / `ToParams` derive macros | `altium-format-derive` | Code gen for simple records |
| v2 `SchSerializer` trait + impls | `v2/serializer/*.rs` | Correct field ordering for complex records |
| v2 `format_v5::export_*/import_*` | `v2/serializer/format_v5/*.rs` | 3600 lines of proven codec logic |
| v2 type enums (`PinElectrical`, etc.) | `v2/types.rs` | Correct enum mappings |
| All ops query functions | `ops/queries/*.rs`, `ops/transforms/*.rs` | Already pure |
| `categorize_component()` | `ops/categorization.rs` | Already pure |
| Block read/write primitives | `io/reader.rs`, `io/writer.rs` | Byte framing, zlib, pascal strings |
| All existing tests | `tests/*.rs`, inline tests | Regression guards |

**Reorganized (same logic, different call signature):**

| What | Change |
|------|--------|
| `ops/schdoc.rs` cmd functions | Drop the `open_schdoc(path)` preamble. Take `&[SchRecord]` + `&RecordTree`. Return results directly, not `Result<T, Box<dyn Error>>` for infallible queries. |
| `ops/schdoc_edit.rs` cmd functions | Take `&[SchRecord]`, return `Vec<SchRecord>`. No `EditSession::open(path)`, no `session.save()`. |
| `ops/schlib.rs`, `pcbdoc.rs`, etc. | Same pattern: take parsed data, return results. |
| `io/schlib.rs`, `io/schdoc.rs`, etc. | Become the Layer 1 file-type wrappers. Keep the CFB layout knowledge, drop the record parsing (that's Layer 2 now). |

**Deleted (replaced by simpler mechanism):**

| What | Why |
|------|-----|
| `TypedAccessor<T>` | No dual representation needed. Records are typed. Bytes are in frames. They don't coexist. |
| `GenericRecord` + `IndexMap` backing | Same. The backing store is `RecordFrame.raw`, not a parallel data structure. |
| `api/document.rs` (`AltiumDocument`) | Replaced by `SchLibFile`, `SchDocFile`, etc. — simpler, file-type-specific. |
| `api/generic/` | Layer 2 (GenericRecord, Value, Container) was for schema-less access. If you want schema-less access, read the `ParameterCollection` directly. |
| `api/typed/` | The entire typed accessor machinery. Records ARE typed. That's it. |
| `edit/session.rs` (`EditSession`) | God object. Replace with pure transform functions + thin CLI shell. |

## How This Relates to combine-v1-v2

The combine-v1-v2 plan identified the right problems but proposed complex solutions. This architecture addresses the same problems more simply:

| combine-v1-v2 goal | Their solution | This solution |
|---------------------|---------------|---------------|
| Lossless roundtrip for untouched data | RawStore + ChangeSet + dirty tracking | RecordFrame with raw bytes. Don't touch it, bytes are preserved. |
| Typed access for business logic | RecordRef/RecordRefMut handles borrowed from Document | Just `&[SchRecord]`. No handles, no lifetimes. |
| v2 coord correctness (100K units) | SchCoord newtype, CoordScale migration | Same — add SchCoord/PcbCoord to record types |
| v2 field correctness (SwapIdPin, etc.) | Phase 3 record merge (4 weeks) | Same — update record structs with v2 fields |
| v2 serialization for complex records | Phase 2 codec bridge | Same — SchSerializer used in Layer 2 serialize |
| Query returns handles not copies | RecordRef<T> with location + Document back-reference | Query returns `&SchRecord` borrows from the record slice. Or indices. |
| Builder pattern for creation | ComponentBuilder with .commit() | Build a `SchRecord` directly. Push it onto the list. |
| `Altium::open()` facade | Document + TypedCache + ChangeSet | `SchLibFile::open()` — file-type-specific, no abstraction tax |
| Ops re-hosting | Phase 4, ops take &SchDocView | Ops take `&[SchRecord]` + `&RecordTree` |

## Field Coverage: Closing the v1-v2 Gap

The record structs need v2's field completeness. The gap is real:

**SchPin**: v1 has 20 fields, v2 has 50+. Missing:
- 14 name/designator customization fields (position mode, custom rotation, font, color)
- 3 pin function fields (hide_pin_name_as_function, symbolic_name, show_symbolic)
- 2 accessibility fields (is_schematic_block_object, owner_index_additional_list)
- 1 extended data field (pin_package_length)
- v1 reads but silently drops SwapIdGroup and SwapIdSequence on roundtrip

**Fix**: Add missing fields to SchPin with defaults. Add `unknown_params: Vec<(String, String)>` to SchPin (and all other record types that lack it). This is a struct-level change, not an architectural change.

**Serialization**: Complex records (Pin, Component, Parameter, Implementation) use v2's `export_*/import_*` functions via SchSerializer. Simple records (Wire, Bus, Junction, Label, NetLabel, etc.) keep using derive macros — they're correct and complete for those types.

## Open Questions

1. **Reconcile for structural edits.** Position-based matching works for field changes. For insertions/deletions, use `unique_id` matching where available. Need to define fallback for records without unique_id (graphics primitives, wires). Options: (a) always re-serialize the entire stream if any insertion/deletion occurred, (b) use content hashing as tiebreaker, (c) accept that structural edits re-serialize the whole stream (they're rare and the stream is small).

2. **SchLib per-component granularity.** SchLib stores each component in a separate Data stream. This means editing one component's pin doesn't require re-serializing other components. The `SchLibFile` wrapper knows this — it exposes per-component frame access. SchDoc puts everything in one stream, so any edit re-serializes all records in that stream. This asymmetry is inherent in the file format, not something we should abstract away.

3. **PCB binary records.** PCB primitives are binary-only (no ASCII params). The `RecordFrame` concept works the same way — raw bytes preserved for untouched records — but the codec is different. PCB records use `FromBinary`/`ToBinary` instead of `FromParams`/`ToParams`. The v2 PCB module has Ghidra-verified struct layouts for this.

4. **Parameter normalization.** When we re-serialize a record, the param string encoding must match Altium's conventions (booleans as `T`/`F`, key casing, pipe delimiters). V2's AsciiSerializer already handles this. Make sure the derive-macro-generated `ToParams` for simple records uses the same conventions.

5. **Extended pin streams.** SchLib components can have PinFrac, PinDesc, PinWideText, etc. streams alongside the Data stream. These are separate byte streams — the container layer just preserves them as-is. If we need to modify fractional coordinates, we read/write PinFrac alongside Data. This is new work (neither v1 nor v2 implements it), but the architecture supports it naturally — it's just another stream in the CFB.
