# Altium Format API Review (Feb 2026)

## Executive Summary

The current `altium-format` crate contains three partially overlapping stacks: v1 `io/` + `records/`, v2 `v2/*` (decompiled C# port), and the layered `api/` module. Each stack solves similar problems with different data models and correctness tradeoffs, which makes the public surface confusing and the internal architecture hard to test. The most urgent improvements are to unify correctness (v2) with ergonomics (v1), isolate CFB/stream parsing from record decoding, and provide a single ergonomic document API that is usable by both the CLI and tests.

This review proposes a clean, layered architecture with explicit codecs and typed document models, plus a migration plan that keeps the project usable at each step.

## Primary Findings

- Duplicate implementations: v1 I/O (`crates/altium-format/src/io`) and v2 I/O (`crates/altium-format/src/v2/io`) both parse the same file types with different assumptions. `crates/altium-format/src/api` duplicates CFB and block parsing yet is not used by v1/v2.
- Mixed concerns: parsing and formatting live inside core types (`crates/altium-format/src/types/parameters.rs`), and record definitions contain output concerns (`crates/altium-format/src/records/sch/mod.rs` uses `DumpTree`).
- Fragmented API: `api::AltiumDocument` is a separate entry point from `io::SchLib` / `io::PcbLib`, and the CLI uses ops modules instead of the layered API.
- Incomplete roundtrip story: `api::generic::BinaryRecord` cannot reserialize modified data, and unknown field preservation is spread across `UnknownFields`, `GenericRecord`, and raw byte retention.
- Coordinate ambiguity: v1 uses a single `Coord` with 10k units/mil; v2 uses 100k units/mil for schematics. The correct behavior differs by domain and should be encoded in types, not inferred.

## Goals For A Cleaner Architecture

1. One authoritative implementation for each file format and record type.
2. Clear layer boundaries: CFB → stream blocks → record decoding → typed domain model → views/ops.
3. Testability: every layer should be runnable in memory without filesystem dependencies.
4. Ergonomic API: a single document entry point, typed view helpers, and explicit lossless editing semantics.
5. Explicit correctness: coordinate units, field names, and binary layouts are encoded in types and codecs.

## Proposed Layered Architecture

1. **Storage (CFB)**
   - Purpose: enumerate streams and read raw bytes.
   - Module: `cfb` (either `crates/altium-format/src/cfb` or a new crate).
   - No Altium semantics and no decoding logic.

2. **Stream Codecs**
   - Purpose: parse Altium block framing, compression, and string encodings.
   - Inputs: raw stream bytes.
   - Outputs: `BlockStream` with `Block { flags, data }`.
   - Module: `stream` or `codec::blocks`.
   - Existing candidates: `crates/altium-format/src/io/reader.rs` and `crates/altium-format/src/api/cfb.rs` should be merged and simplified.

3. **Record Codecs**
   - Purpose: decode blocks into record envelopes without domain types.
   - Two codecs: `ParamRecordCodec` and `BinaryRecordCodec`.
   - Output type example: `RecordEnvelope { record_type, payload, raw_block }`.
   - This is where `ParameterCollection` belongs, without any CFB or filesystem coupling.

4. **Typed Domain Model**
   - Purpose: define record structs, enums, and domain-specific coordinate types.
   - Use explicit coordinates:
     - `SchCoord` (100k units/mil)
     - `PcbCoord` (10k units/mil)
   - Keep `UnknownFields` as a first-class concept, but make it consistent across record types.
   - Merge v2 correctness into v1 derive-macro-driven record structs.

5. **Document Model**
   - Purpose: stable, typed representation of a file (SchLib/SchDoc/PcbLib/PcbDoc).
   - Provide `Document` + `DocumentKind` enums and typed views.
   - Allow lazy decoding: store `RawDoc` internally and decode on demand.

6. **Views and Ops**
   - Purpose: query, transformation, JSON/CLI output.
   - Should depend only on the Document model and not on raw stream parsing.

## Proposed Public API Shape

```rust
use altium_format::doc::{Document, DocumentKind};

let doc = Document::open("library.SchLib")?;
match doc.kind() {
    DocumentKind::SchLib(schlib) => {
        for comp in schlib.components() {
            println!("{}", comp.lib_reference);
        }
    }
    _ => {}
}

// Lossless edit flow (typed + unknown fields preserved)
let mut edit = doc.edit();
edit.schlib()?.component_mut("LM358")?.set_value("OPAMP");
edit.save("library_out.SchLib")?;
```

Key API constraints:

- `Document::open` and `Document::from_reader` are the only entry points.
- `DocumentKind` is the only way to branch by file type.
- Typed records always carry an `UnknownFields` companion for lossless edits.

## Specific Structural Cleanups

1. Move `ParameterCollection` into a `params` module and remove its I/O dependencies.
2. Merge `crates/altium-format/src/api/cfb.rs` and `crates/altium-format/src/io/reader.rs` into a single `stream` layer.
3. Replace `api::generic::BinaryRecord` with a codec-backed `BinaryRecord` that can reserialize, or mark it explicitly as read-only.
4. Remove DumpTree implementations from `records` and move them into `dump` as adapters.
5. Collapse v1/v2 into one typed record model with explicit correctness fixes.

## Recommended Migration Plan

1. Introduce `SchCoord` and `PcbCoord` types and update core record structs to use them.
2. Implement record codecs (`ParamRecordCodec`, `BinaryRecordCodec`) and refactor v1 `io/*` to use them.
3. Rebuild `api::AltiumDocument` on top of the new `Document` model and deprecate direct `io::SchLib`/`io::PcbLib` constructors.
4. Port v2 correctness into v1 record definitions and remove `crates/altium-format/src/v2` once parity is reached.
5. Update ops/CLI to use the new document API and delete the older paths.

## Testability Improvements

- Add codec-level tests that operate on raw byte slices (no filesystem).
- Add golden roundtrip tests for each file type using sample fixtures.
- Use property tests for `ParameterCollection` to enforce parse/serialize stability.
- Provide `Document::from_reader` and `Document::write_to` to allow pure in-memory tests.

## Expected Outcomes

- A single, coherent API surface for both library users and internal tools.
- Clear boundaries that allow safe refactoring, easier correctness fixes, and higher test coverage.
- Less duplication and fewer “source of truth” conflicts between v1 and v2.

