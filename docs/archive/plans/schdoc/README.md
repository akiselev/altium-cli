# SchDoc Implementation Plan

Read-path implementation plan for SchDoc support in `altium-format`.

## Overview

Implement SchDoc parsing in `crates/altium-format/src/schdoc/` using existing shared
infrastructure (`TrackedCfbDocument`, block framing, parameter parsing, embedded object
stream parsing) and extending schematic record coverage for SchDoc-only record types.

Scope for this plan:
- Read/validate path (`SchDoc::open`, `SchDocOps::validate`, CLI `validate` flow)
- Full stream consumption with fail-fast behavior
- Record parsing for observed SchDoc records plus documented SchDoc-only records

Out of scope for this plan:
- SchDoc write path / round-trip serialization
- Full support for rare optional streams with undocumented payloads (must still be tracked and surfaced)

## Planning Context

### Decision Log

| Decision | Reasoning |
|---|---|
| Convert `schdoc.rs` stub into `schdoc/` module directory | The implementation will require multiple focused parsers (file header, main records, additional, storage). A single file will become hard to maintain quickly. |
| Keep shared record types in `sch_records.rs` initially | SchLib already uses these types and serializers. Reusing/extending that code avoids duplicate parsing logic while SchDoc stabilizes. |
| Implement SchDoc as read-path first | Current `SchDoc` and `SchDocOps` are stubs; read-path unlocks `validate` and red/green format discovery before write-path complexity. |
| Treat FileHeader as text-only blocks | SchDoc research shows no binary pin blocks in FileHeader; binary blocks are valid only in `Storage`. Any other binary block should be an error. |
| Handle both header/sheet layouts | Research notes legacy files with split Block 0/1 and possible newer combined header+sheet serialization. Parser should accept both without silent key drops. |
| Parse `Additional` and `Storage` in dedicated phases | Mirrors documented loading pipeline and avoids coupling embedded object extraction to base record parsing. |
| Fail on unconsumed streams by default | Matches `CLAUDE.md` fail-fast constraints and avoids masking unsupported optional streams. |

### Constraints & Assumptions

- Use domain constants/types from `altium-format-types` (no hard-coded keys/types where constants exist).
- Every parsing boundary must attach error context.
- `TrackedCfbDocument::assert_all_consumed()` remains a completion gate.
- Target location for new code: `crates/altium-format/src/schdoc/`.
- Existing public API remains `pub struct SchDoc` exported from `altium-format`.

### Known Risks

| Risk | Mitigation |
|---|---|
| Incomplete SchDoc-only record coverage initially | Implement dispatch incrementally with explicit unknown-record errors and test corpus expansion. |
| Header/sheet field placement differs by file version | Parse with a two-source strategy (header block + first sheet block) and validate expected invariants. |
| Optional streams appear in real files | Parse known optional streams where format is documented; otherwise return explicit unsupported errors, never silently consume raw bytes. |
| Owner index validation across base/additional lists | Explicit post-parse validation for `OWNERINDEX` + `OWNERINDEXADDITIONALLIST` consistency. |

## Target Architecture

```text
SchDoc::open(path)
  -> TrackedCfbDocument::open(path)
  -> parse_fileheader_stream("/FileHeader")
       - header metadata
       - base warehouse records (flat list)
       - sheet/template invariants
  -> parse_storage_stream("/Storage")
       - embedded image payloads keyed by filename/id
       - merge into SchImage records
  -> parse_additional_stream("/Additional")
       - RECORD=225 overlays
  -> parse_optional_streams_if_present(...)
  -> validate_owner_indices(...)
  -> tracked.assert_all_consumed()
  -> SchDoc { header, records, additional_records, embedded_objects, ... }
```

## Milestones

| # | Name | Files | Depends On |
|---|---|---|---|
| 1 | [Module Foundation](milestone-01-module-foundation.md) | `crates/altium-format/src/schdoc/`, `crates/altium-format/src/lib.rs` | - |
| 2 | [FileHeader Parse & Dispatch Skeleton](milestone-02-fileheader-dispatch.md) | `crates/altium-format/src/schdoc/fileheader.rs`, `.../mod.rs` | M1 |
| 3 | [SchDoc Record Coverage](milestone-03-record-coverage.md) | `crates/altium-format/src/sch_records.rs`, `.../schdoc/dispatch.rs` | M2 |
| 4 | [Storage Stream Integration](milestone-04-storage-stream.md) | `crates/altium-format/src/schdoc/storage.rs` | M2 |
| 5 | [Additional Stream Integration](milestone-05-additional-stream.md) | `crates/altium-format/src/schdoc/additional.rs` | M2 |
| 6 | [Optional Streams & Final Validation](milestone-06-optional-streams-validation.md) | `crates/altium-format/src/schdoc/mod.rs` | M3, M4, M5 |
| 7 | [Ops/CLI Validation Integration](milestone-07-ops-cli-validation.md) | `crates/altium-format-ops/src/schdoc_ops.rs`, `crates/altium-cli/src/main.rs` | M6 |

## Dependency Graph

```text
M1 -> M2 -> M3 -> M6 -> M7
         \-> M4 -/
         \-> M5 -/
```

