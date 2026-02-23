# PcbLib Implementation Plan

Read-path parser for Altium PcbLib (PCB Footprint Library) files.

## Overview

PcbLib files are CFB containers holding binary PCB footprint data. Unlike SchLib (pipe-delimited
text records), PcbLib uses packed binary structs dispatched by a `u8` TObjectId byte. The parser
must handle 8 primitive types (Arc, Pad, Via, Track, Text, Fill, Region, ComponentBody), library-wide
metadata, and 4 sidecar stream formats for supplementary per-primitive data.

This plan covers the **read path only** (parsing, validation, CLI queries). Write path is deferred
to a separate plan.

## Planning Context

### Decision Log

| Decision | Reasoning Chain |
|----------|----------------|
| Hybrid module structure | SchLib uses single-file `schlib.rs` (78K) -> PcbLib has fundamentally different parsing (binary vs text) -> top-level mirrors SchLib for consistency, but primitives split into submodules for maintainability since Pad alone is ~500 bytes of binary parsing |
| Read path only | Full read+write doubles scope -> read path validates parsing correctness first -> write path meaningfully depends on read path being stable -> defer write to separate plan |
| Extract shared code to common module | SectionKeys parsing is identical between SchLib and PcbLib (confirmed: `docs/pcblib/sectionkeys.md:58-61` states "The SectionKeys format and resolution algorithm is **identical** between PcbLib and SchLib. The same parsing code should handle both.") -> copy-and-adapt creates maintenance drift -> extract to shared module both import |
| 3D models metadata only | Parsing model GUID references requires ~2 hours -> full STEP decompression is a separate domain (zlib + STEP format) -> metadata-only covers the common use case (listing models, resolving references) |
| Property-based unit tests for binary parsing | Binary parsers have clear input/output contracts (bytes -> struct) -> proptest generates random valid/invalid inputs covering edge cases humans miss -> fewer tests, wider coverage |
| Selected subset for integration tests | 20+ test files (1-93 MB each) -> running all takes minutes -> select ~5-7 representative files (blank, small, medium, large, edge cases with SectionKeys) for fast iteration |
| Semantic round-trip for future write tests | Byte-identical round-trip requires matching Altium's exact serialization order and padding -> semantic equivalence (load -> save -> reload -> compare structures) is sufficient for correctness verification |
| PcbLib WideStrings use parameter-block format | PcbDoc WideStrings use binary TLV -> PcbLib uses `\|ENCODEDTEXT{N}=decimal,bytes\|` format -> these share NO structure and require separate parsers -> existing `wide_strings_tlv.rs` is irrelevant for PcbLib |
| Data stream pattern name block first | Documentation and real files confirm pattern name block precedes all binary records -> parser must read this before entering the binary record dispatch loop |
| Per-primitive trailing_bytes for version tolerance | Records have version-dependent trailing fields (e.g. Arc: 45 bytes legacy, 58 bytes AD26) -> record length tells exact byte count -> store unknown trailing bytes to preserve data we don't yet understand. CLAUDE.md override: the no-opaque-blobs rule is consciously relaxed for trailing bytes ONLY because (a) binary record boundaries are validated via record length, (b) trailing bytes are format-version extensions that don't affect PCB fabrication of the known fields, and (c) M7 integration tests must assert trailing_bytes is empty on all test file primitives — any non-empty trailing bytes trigger investigation and implementation before plan completion. This is tracked technical debt with a concrete completion gate, not silent suppression. |
| Common header has gap byte at offset 1 | Documentation shows offset 0 = layer (1 byte), offset 2 = flags (2 bytes) -> gap byte at offset 1 must be explicitly read -> PcbPrimitiveCommon includes `pad_byte: u8` field at offset 1 to account for it. Total: 1+1+2+4+2+2+1 = 13 bytes |
| Custom Data stream reader (not parse_pcb_binary_records) | `parse_pcb_binary_records()` in pcb_binary_stream.rs reads `u8 type + u32 len + payload` in flat loop -> PcbLib Data stream format is `u8 type` once, then N subrecords each with `u32 len + payload` (N=6 for Pad, 2 for Text, 1 for others) -> type byte is NOT repeated per subrecord -> pcb_binary_stream.rs is designed for PcbDoc homogeneous sections, NOT PcbLib mixed-type Data streams -> M4 requires a custom `parse_pcblib_data_stream()` that knows subrecord counts per type. Subrecord u32 length field has flags in the high byte (per `docs/pcblib/footprint-data-stream.md:62-68`): apply BLOCK_SIZE_MASK to extract lower 24 bits as payload size. In practice, flags byte is usually 0x00 but must be masked for correctness. |
| Error on unimplemented Library sub-storages | LayerKindMapping, PadViaLibrary, EmbeddedFonts, ModelsNoEmbed, Textures all have Header+Data pattern -> parse header count, if 0 entries consume empty Data (this IS full parsing) -> if >0 entries return AltiumFormatError (fail-fast forces implementation when test files have data) -> preserves CLAUDE.md invariant: no raw-byte storage of unparsed data |
| Error on unimplemented primitive types during incremental development | CLAUDE.md prohibits storing raw bytes for data we don't understand -> M4 implements Arc/Track/Via/Fill and returns AltiumFormatError for other types -> M4 test files pre-selected to contain only simple primitives -> M5 adds remaining types before end-to-end validation on complex files |
| FileHeader validation: exact string match | loading-pipeline.md specifies "PCB 6.0 Binary Library File" -> exact match provides strict version gating -> future format versions require explicit code change rather than silently passing -> aligns with fail-fast philosophy |
| PcbLib VersionInfo minor_version mapping | PcbLib stores format version as f64 (e.g. 11.0 = eAdvPCBFormat_Library_V6) -> VersionInfo.minor_version is SchLib-centric (SchLib has integer minor_version in header) -> PcbLib has no equivalent field -> set to 0 with the f64 version available via header string -> callers needing PcbLib version use header field, not minor_version |
| PrimitiveGuids: error if format unclear | CLAUDE.md prohibits storing raw bytes for unparsed data -> if PrimitiveGuids binary layout cannot be determined from documentation + Ghidra analysis, return AltiumFormatError -> red/green loop surfaces this as a test failure for investigation -> no silent data storage |
| UniqueID stored per-primitive | Round-trip requires ID accessible on each primitive for serialization -> footprint-level HashMap requires secondary lookup during write -> per-primitive `unique_id: Option<String>` field is simpler and correct for both read and write paths |
| Library/Data non-standard block framing | docs/pcblib/library-storage.md reports ole-inspect.py block parse errors on this stream and recommends reading entire stream as single pipe-delimited parameter string -> parse as flat param string, not block-by-block -> aligns with implementation note in library-storage.md |
| EmbeddedFonts single-stream structure | docs/pcblib/library-storage.md confirms EmbeddedFonts is a single stream (no Header/Data sub-streams) -> observed as a single block with 0-length payload in all test files -> read as single stream, check if empty, mark consumed. Exception to the Header+Data pattern used by other Library sub-storages |
| Library/Models/N blob streams consumed as opaque bytes | TrackedCfbDocument requires ALL streams consumed -> Library/Models/{0..N} are zlib-compressed STEP model blobs -> 3D models metadata only decision accepts storing blobs without STEP parsing -> iterate 0..count after parsing Models/Data metadata, read each numbered stream, store bytes in model entry |

### Rejected Alternatives

| Alternative | Why Rejected |
|-------------|-------------|
| Single `primitives.rs` file (mirror SchLib exactly) | SchLib's text records are uniform (all pipe-delimited) -> PcbLib has 8 distinct binary layouts with Pad at ~500 bytes -> single file would exceed 2000 lines, hindering navigation |
| Derive macros for binary structs | Would reduce boilerplate for simple types -> significant upfront investment in proc macro code -> `binary_io.rs` helpers already handle the pattern well -> not enough ROI for read-only path |
| Copy SectionKeys parsing into PcbLib module | Faster initial implementation -> creates drift when bugs are fixed or format understanding improves -> extraction costs ~30 min now vs unknown debugging later |
| Include write path in this plan | Comprehensive implementation -> roughly doubles scope -> read path must be stable before write path is meaningful -> deferred to avoid blocking validation workflow |

### Constraints & Assumptions

- PCB types (`PcbObjectId`, `V6Layer`, `PadShape`, etc.) already exist in `altium-format-types/src/pcb.rs`
- `pcb_file_header.rs` already parses the FileHeader format (tested)
- `pcb_binary_stream.rs` reads raw binary records for PcbDoc sections (type + length + payload in flat loop); NOT suitable for PcbLib Data streams which use multi-subrecord framing (see Decision Log)
- `cfb_document.rs`, `binary_io.rs`, `param_collection.rs`, `block_stream.rs` are proven infrastructure
- `TrackedCfbDocument` enforces the fail-fast invariant: all streams must be consumed
- Test files in `data/pcblib/` (~20 files, 1-93 MB) covering blank, simple, and complex libraries
- Encoding: binary fields are Windows-1252 for strings; WideStrings sidecar provides UTF-8

### Known Risks

| Risk | Mitigation | Anchor |
|------|-----------|--------|
| Pad record layout varies by format version and may have undocumented fields | Store trailing bytes after known fields; red/green loop with `altium cfb dump` to inspect real data | `docs/pcblib/binary-primitives.md:86-97` |
| Region vertex count offset is version-dependent | Parse known prefix, use record length to determine vertex data boundaries | `docs/pcblib/binary-primitives.md:226-238` |
| PrimitiveGuids format not fully understood | Return AltiumFormatError if format cannot be determined from docs + Ghidra; red/green loop surfaces for investigation | `docs/pcblib/sidecar-streams.md:82-97` |
| ComponentBody binary layout incompletely documented | binary-primitives.md uses placeholder offsets ("N bytes"); pre-implementation investigation with `altium cfb dump` and Ghidra required before M5 begins | `docs/pcblib/binary-primitives.md:241-258` |
| FileVersionInfo stream not in any milestone explicitly | Present in BlankPcbLib; TrackedCfbDocument assert_all_consumed will force implementation; red/green loop on BlankPcbLib will surface it during M7 integration tests | `docs/pcblib/cfb-structure.md:13-14` |
| Some test files may be corrupt or use unsupported encoding | Skip corrupt files in test subset; report errors clearly for investigation | `CLAUDE.md:179` |

## Invisible Knowledge

### Architecture

```
PcbLib::open(path)
    |
    v
TrackedCfbDocument::open()
    |
    +-- parse FileHeader (format ID, version, key token)
    |
    +-- parse SectionKeys (optional name->key mapping)
    |
    +-- parse Library/ storage
    |     +-- Library/Data (board defaults, layer stack)
    |     +-- Library/ComponentParamsTOC (footprint index)
    |     +-- Library/Models (3D model metadata)
    |     +-- Library/{LayerKindMapping, PadViaLibrary, EmbeddedFonts, ...}
    |
    +-- enumerate footprint storages (exclude FileVersionInfo, Library)
    |
    +-- for each footprint:
          +-- Parameters stream (metadata: PATTERN, HEIGHT, DESCRIPTION)
          +-- Header stream (u32 record count)
          +-- Data stream:
          |     +-- pattern name block (u32 len + u8 strlen + ASCII name)
          |     +-- binary records (u8 type + u32 len + payload per subrecord)
          |           +-- dispatch to type-specific parser
          |           +-- Pad: 6 subrecords, Text: 2 subrecords, others: 1
          |
          +-- WideStrings sidecar (param-block format, merge into Text primitives)
          +-- UniqueIDPrimitiveInformation (merge unique IDs by index)
          +-- ExtendedPrimitiveInformation (merge extended props, rare)
          +-- PrimitiveGuids (merge GUIDs by entry mapping)
```

### Data Flow

```
CFB File --> TrackedCfbDocument --> Stream bytes --> Type-specific parsers --> In-memory structs
                                                         |
                                                         v
                                                    Sidecar merging
                                                         |
                                                         v
                                                    PcbLib { footprints: Vec<PcbFootprint> }
```

### Why This Structure

The `pcblib/` module uses a `primitives/` subdirectory because:
- 8 distinct binary layouts with different field counts and subrecord structures
- Pad alone requires ~200 lines of parsing code (6 subrecords, ~500+ bytes)
- Per-file organization enables targeted testing and isolated changes
- Top-level module (`mod.rs`) handles CFB orchestration, delegating binary parsing to specialists

Shared code (SectionKeys) is extracted because SchLib and PcbLib use identical SectionKeys format.
Duplicating it creates maintenance drift when format understanding improves.

### Invariants

- Every stream in the CFB container must be consumed or return an error (TrackedCfbDocument)
- Binary record lengths include only the payload, not the type or length fields themselves
- Primitive indices are 0-based, sequential, assigned in Data stream parse order
- WideStrings ENCODEDTEXT indices correspond to Text primitive positions (not global indices)
- Pad always has exactly 6 subrecords; Text always has exactly 2; all others have exactly 1
- Pattern name from Data stream must match PATTERN from Parameters stream
- Common header is 13 bytes with a gap byte at offset 1 (between layer and flags)
- PcbLib Data stream framing: type byte once, then N subrecords (NOT repeated per subrecord)
- Library sub-storages with count=0 are valid (empty data); count>0 requires full parsing

### Tradeoffs

- **Trailing bytes preserved vs fully parsed**: Unknown trailing fields are stored as raw bytes
  rather than erroring. This trades some fail-fast purity for version tolerance, but the record
  count still validates stream integrity.
- **Eager vs lazy footprint loading**: All footprints are loaded during `open()` for simplicity.
  Lazy loading would reduce memory for large libraries but adds complexity. Eager is appropriate
  for the validation workflow (need to check everything).

## Milestones

| # | Name | Files | Dependencies |
|---|------|-------|-------------|
| 1 | [Foundation & Module Structure](milestone-1-foundation.md) | pcblib/mod.rs, lib.rs | None |
| 2 | [CFB Metadata & Footprint Enumeration](milestone-2-cfb-metadata.md) | pcblib/mod.rs, section_keys.rs | M1 |
| 3 | [Library Storage](milestone-3-library-storage.md) | pcblib/library.rs | M1 |
| 4 | [Simple Primitives & Data Stream](milestone-4-simple-primitives.md) | pcblib/primitives/*, pcblib/footprint.rs | M2 |
| 5 | [Complex Primitives](milestone-5-complex-primitives.md) | pcblib/primitives/{text,region,pad,component_body}.rs | M4 |
| 6 | [Sidecar Streams](milestone-6-sidecar-streams.md) | pcblib/sidecar.rs, pcblib/wide_strings.rs | M4 |
| 7 | [Validation & CLI Integration](milestone-7-validation-cli.md) | pcblib_ops.rs, main.rs | M3, M5, M6 |

## Milestone Dependencies

```
M1 ──> M2 ──> M4 ──> M5 ──> M7
 |            |       |       ^
 └──> M3      └──> M6 ──────┘
       |                      ^
       └──────────────────────┘
```

M3 (Library Storage) can proceed after M1 (no dependency on M2).
M4 (Simple Primitives) requires M2 (footprint enumeration).
M3 and M4 are independent and can proceed in parallel.
M5 (Complex Primitives) and M6 (Sidecar Streams) can proceed in parallel after M4.
M7 (Validation & CLI) requires M3, M5, and M6.
