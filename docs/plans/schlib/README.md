# SchLib Implementation Plan

## Overview

Implement full SchLib (Schematic Library) document parsing in the `altium-format` crate, including derive macros for parameter-based record deserialization. SchLib files are OLE/CFB compound documents containing reusable schematic components with pins, graphical primitives, text annotations, and footprint assignments.

The low-level API (layers 1-4: CFB I/O, consumption tracking, block framing, parameter/binary parsing) is production-ready. This plan builds layer 5: SchLib-specific record types, stream parsers, and document assembly.

## Planning Context

### Decision Log

| Decision | Reasoning Chain |
|----------|----------------|
| `#[derive(FromParams)]` macro for parameter records | 20+ record types averaging 10+ fields each -> 200+ lines of repetitive `remove_required`/`remove_with_default` calls -> derive macro makes struct definitions declarative and self-documenting while eliminating copy-paste bugs |
| Hand-written binary pin parser (no derive) | Only one binary record type (Pin, RECORD=2) -> variable-length format with length-prefixed strings makes derive complex for a single use -> hand-written parser is clearer and more maintainable |
| `from_params()` does NOT call `assert_exhausted()` | Base types (SchPrimitiveBase, SchGraphicalBase) extract subset of params via flatten -> exhaustion check must happen at top level after ALL fields extracted -> caller handles exhaustion at dispatch site |
| Flat enum dispatch over trait objects | All record types known at compile time -> enum dispatch avoids heap allocation and dynamic dispatch -> pattern matching gives exhaustiveness checking -> Altium record set is closed (no user-defined records) |
| Follow documented 3-phase loading pipeline | C# source uses ImportBaseWarehouse -> ImportExtendedWarehouse -> ImportAdditionalWarehouse -> pin sidecar merging must happen in exact order (PinWideText is authoritative) -> deviating from pipeline risks data corruption |
| Component-relative OwnerIndex | SchLib stores OwnerIndex relative to each component section -> must track base offset per component during loading -> absolute index = relative + component_base_offset |
| Derive macro uses path expressions for keys | Constants in altium-format-types are &str values -> macro emits code referencing constant paths (e.g., `visual::COLOR`) -> compile-time verification that key names are valid -> no string typos |
| SchLib module stays in single file initially | All parsing is internal to altium-format -> start in schlib.rs, extract to submodules only when file exceeds 500 lines -> avoids premature file proliferation |
| Pin sidecar merge modifies Pin in-place | Sidecar streams provide additive/replacement data for existing pins -> collecting pins first then applying sidecars in-order matches C# loading pipeline -> PinWideText replaces binary text fields entirely |
| No round-trip/write support in this plan | CLAUDE.md says no round-trip preservation -> read-only parsing sufficient for validate command -> write support is a separate future plan |

### Rejected Alternatives

| Alternative | Why Rejected |
|-------------|-------------|
| serde-based deserialization | Altium's parameter format (pipe-delimited, case-insensitive, Windows-1252, fractional coords) is too far from serde's model -> custom derive is simpler and more correct than fighting serde's assumptions |
| Trait objects for record types | Adds heap allocation, dynamic dispatch, and loses exhaustiveness checking -> enum dispatch with pattern matching is faster and safer for a closed set of ~20 record types |
| Single monolithic parse function | Would exceed 500 lines -> per-record-type parsing functions with enum dispatch keeps each function focused and testable |
| Parse all record types before any testing | Red/green workflow requires incremental progress -> implement one record type at a time, validate against real files, fix unknowns as they appear |
| Store raw ParameterCollection in records | Violates fail-fast philosophy -> unknown parameters must cause errors, not be silently carried -> typed structs with assert_exhausted() enforcement |
| Derive macro for binary pin format | Only one binary record type -> variable-length strings with conditional fields make declarative binary format description complex -> hand-written BinaryReader code is clearer |

### Constraints & Assumptions

- Rust 2024 edition, MSRV 1.85+
- All parsing must return `Result<T, AltiumFormatError>` (never panic, never silently skip)
- Constants from `altium-format-types::constants::*` must be used for all parameter keys (no string literals)
- Test data: `data/BlankSchlibComponent.SchLib` (1 component), `data/LimeMicroAltiumLib_schLib.SchLib` (200 components), `data/Synthiam.SchLib` (174 components)
- Derive crate currently empty (just Cargo.toml with proc-macro2/quote/syn dependencies)
- `altium validate <file.SchLib>` is the primary red/green feedback loop
- No write/serialization support needed (read-only)
- Privacy: all implementation details are `pub(crate)`, only `SchLib` struct and its public API are `pub`

### Known Risks

| Risk | Mitigation | Anchor |
|------|-----------|--------|
| Unknown parameter keys in real SchLib files | `assert_exhausted()` on ParameterCollection will surface them immediately -> add to struct or return error | `param_collection.rs:assert_exhausted()` |
| Pin sidecar import order matters | Follow exact 9-stream order from docs/schlib/pin-sidecar-streams.md -> PinWideText (stream 5) is authoritative and replaces binary text | docs/schlib/pin-sidecar-streams.md |
| Component names > 31 chars truncated in CFB | SectionKeys stream maps full names to CFB keys -> must parse SectionKeys before component enumeration | docs/schlib/cfb-structure.md |
| PartCount stored as actual+1 in wire format | C# source confirms `PARTCOUNT` = actual_parts + 1 -> subtract 1 during parsing | docs/schlib/record-types.md |
| PinFrac coordinates are additive, not replacement | Binary pin i16 values multiplied by C_BASE_UNIT (100,000) then PinFrac i32 added -> not replacing the coordinate | docs/schlib/binary-pin-format.md |
| RECORD >= 256 uses RECORD=254 + RECORDEX | Dispatch must check for RECORD=254 and read RECORDEX for actual type -> SchRecordType supports all extended values | docs/dxp/schematic-records.md |

## Architecture

```
SchLib::open(path)
    |
    v
TrackedCfbDocument::open(path)
    |
    v
Phase 1: parse_file_header()        -> SchLibHeader (fonts, component index)
         parse_section_keys()        -> HashMap<String, String> (name -> cfb key)
         for each component:
           parse_component_data()    -> SchComponent + Vec<SchRecord>
    |
    v
Phase 2: parse_storage_stream()     -> merge images into SchImage records
         for each component:
           merge_pin_sidecars()      -> modify SchPin records in-place (9 streams)
    |
    v
Phase 3: for each component:
           parse_additional_stream() -> append overflow records
    |
    v
tracked_cfb.assert_all_consumed()   -> error if any stream unhandled
    |
    v
SchLib { header, components: Vec<SchLibComponent> }
```

### Data Flow

```
CFB File
  -> TrackedCfbDocument (stream tracking)
    -> /FileHeader -> parse_blocks -> ParameterCollection -> SchLibHeader
    -> /SectionKeys -> parse_blocks -> ParameterCollection -> name-to-key map
    -> /<key>/Data -> parse_blocks -> dispatch_record() per block
         flags=0x00 -> ParameterCollection -> FromParams -> SchRecord variant
         flags=0x01 -> BinaryReader -> parse_binary_pin() -> SchPin
    -> /Storage -> parse_embedded_object_stream -> merge into SchImage records
    -> /<key>/PinFrac..PinFunctionData -> parse_embedded_object_stream -> merge into SchPin
    -> /<key>/Additional -> parse_blocks -> dispatch_record() -> append to component
    -> /<alias>/Redirection -> ParameterCollection -> alias resolution
```

### Invariants

- Every CFB stream must be consumed or explicitly skipped (TrackedCfbDocument enforces this)
- Every parameter key must be consumed or error (assert_exhausted enforces this)
- OwnerIndex values are component-relative in SchLib; conversion to absolute happens at parse time
- Pin sidecar streams must be applied in exact order: PinFrac -> PinDesc -> PinMiscData -> PinTextData -> PinWideText -> PinSymbolLineWidth -> PinPackageLength -> PinPropagationDelay -> PinFunctionData
- PinWideText fully replaces binary pin text fields (Name, Description, Designator) when present
- RECORD=0 is an end marker, not an error
- Binary pin blocks always have flags=0x01; all other records use flags=0x00

## Milestones

See individual milestone files for full details.

| Milestone | Name | Files | Depends On |
|-----------|------|-------|------------|
| 1 | [Derive Macros (FromParams)](milestone-01-derive-macros.md) | `crates/altium-format-derive/src/lib.rs` | - |
| 2 | [Base Record Types](milestone-02-base-types.md) | `crates/altium-format/src/sch_records.rs` | M1 |
| 3 | [FileHeader + SectionKeys](milestone-03-fileheader.md) | `crates/altium-format/src/schlib.rs` | M1 |
| 4 | [Component Data Stream + Record Dispatch](milestone-04-data-stream.md) | `crates/altium-format/src/schlib.rs` | M2, M3 |
| 5 | [Binary Pin Parser](milestone-05-binary-pin.md) | `crates/altium-format/src/sch_records.rs` | M2 |
| 6 | [Graphical Primitives](milestone-06-graphical-primitives.md) | `crates/altium-format/src/sch_records.rs` | M2 |
| 7 | [Text + Annotation Records](milestone-07-text-records.md) | `crates/altium-format/src/sch_records.rs` | M2 |
| 8 | [Implementation Records](milestone-08-implementation-records.md) | `crates/altium-format/src/sch_records.rs` | M2 |
| 9 | [Pin Sidecar Streams](milestone-09-pin-sidecars.md) | `crates/altium-format/src/schlib.rs` | M5 |
| 10 | [Storage, Additional, Aliases](milestone-10-remaining-streams.md) | `crates/altium-format/src/schlib.rs` | M4, M6, M7, M8 |
| 11 | [SchLib Document Assembly](milestone-11-assembly.md) | `crates/altium-format/src/schlib.rs`, `crates/altium-format-ops/src/schlib_ops.rs` | All |

## Milestone Dependencies

```
M1 (Derive Macros)
 |
 +---> M2 (Base Types)
 |      |
 |      +---> M5 (Binary Pin) -----> M9 (Pin Sidecars) --+
 |      |                                                  |
 |      +---> M6 (Graphical Primitives) --+                |
 |      |                                 |                |
 |      +---> M7 (Text Records) ---------+---> M10 ------+---> M11 (Assembly)
 |      |                                 |                |
 |      +---> M8 (Implementation) -------+                |
 |                                                         |
 +---> M3 (FileHeader) ---> M4 (Data Stream) ------------+
```

Independent milestones that can execute in parallel after M2:
- M5, M6, M7, M8 (all record type implementations)

Sequential dependencies:
- M1 -> M2 -> M4 (derive -> base types -> dispatch)
- M1 -> M3 -> M4 (derive -> fileheader -> dispatch)
- M5 -> M9 (binary pin -> pin sidecars)
- All record types -> M10 -> M11 (records -> remaining streams -> assembly)
