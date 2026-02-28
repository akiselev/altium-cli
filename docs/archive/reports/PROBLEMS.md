# Property Invariant Failures and Investigation Notes

This file captures the current red-state failures from invariant/property tests and parser validation across document types.

## Scope and evidence

- Proptests run from `crates/altium-format`:
  - `schlib::tests::prop_schlib_invariants_hold_for_fixtures`
  - `schdoc::tests::prop_schdoc_invariants_hold_for_fixtures`
  - `pcbdoc::tests::prop_pcbdoc_invariants_hold_for_fixtures`
- PcbLib evidence gathered with sampled CLI validation over fixtures:
  - `target/debug/altium-cli validate <file.PcbLib>`

## SchLib

### Current failures

- `AllPinCount` invariant failures in many fixtures.
- Missing font table parameter `Size6` in at least one fixture.

### Concrete examples

- `InvalidParamValue { key: "AllPinCount", detail: "component[0] all_pin_count=0 but has 65 Pin records" }`
- `InvalidParamValue { key: "AllPinCount", detail: "component[44] all_pin_count=2 but has 12 Pin records" }`
- `MissingParam("Size6")`

### Why this currently fails

- Parser requires every indexed font `SizeN` key as mandatory.
  - `crates/altium-format/src/schlib.rs:180`
- Invariant enforces strict equality: `component.all_pin_count == actual_pin_records`.
  - `crates/altium-format/src/schlib.rs:3281`

### SchLib deep-dive (AD26 decompiled behavior)

- `AllPinCount` is imported and stored as raw persisted value.
  - `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.FileFormats/FileFormatV5.cs:3075`
  - `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.Objects/SchDataComponent.cs:612`
- AD26 lazily repairs/recomputes only when the stored value is `<= 0`.
  - `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.EngineObjects/SchComponent.cs:2659`
- `GetAllPinCount()` triggers this repair path; export uses that getter.
  - `AD26-dotnet/.../SchComponent.cs:3446`
  - `AD26-dotnet/.../FileFormatV5.cs:2906`

### SchLib assessment

- Definite mismatch with AD26 behavior: our invariant is too strict for persisted legacy/stale positive values.
- `Size6` is currently ambiguous, not a confirmed bug:
  - AD26 `ImportFontTable` reads `SizeN` for every `1..FontIdCount` as required.
    - `AD26-dotnet/.../FileFormatV5.cs:5279` and `:5284`
  - This could be malformed fixture data, truncated data, or an older variant not yet identified.

### SchLib fix direction

- Change invariant from strict equality to AD26-compatible rule:
  - reject negative values;
  - allow stale positive `AllPinCount` (report/warn category);
  - if `AllPinCount <= 0`, allow/expect effective recomputed count semantics.
- Decide policy for missing `SizeN`:
  - strict-fail (align with AD26 import path),
  - or tolerant parse with defaults for corpus compatibility.
- Keep roundtrip writer canonical (emit normalized `AllPinCount` from actual pin graph).

## SchDoc

### Current failures

- Non-CFB `.SchDoc` files in fixtures (ASCII-like content) fail at container open.
- Unknown parameters rejected in `/FileHeader` record parsing.
- Unknown record type in `/Additional` (`RECORD=220`).

### Concrete examples

- `CfbError("Invalid CFB file (wrong magic number): [7c, 48, 45, 41, 44, 45, 52, 3d]")`
- `UnknownParams { keys: ["EXTRALOCATIONCOUNT", "EX51", ... "EY176_FRAC"] }` on `RECORD=7`
- `UnknownParams { keys: ["ALIGNMENT"] }` on `RECORD=209`
- `UnknownRecordType(220)` in `/Additional`

### Why this currently fails

- Optional top-level streams are currently hard-failed as "present but not implemented".
  - `crates/altium-format/src/schdoc/mod.rs:150`
- Strict `assert_exhausted()` on `/FileHeader` and `/Additional` records.
  - `crates/altium-format/src/schdoc/fileheader.rs:117`
  - `crates/altium-format/src/schdoc/mod.rs:692`
- Dispatch rejects unmapped record IDs.
  - `crates/altium-format/src/schdoc/dispatch.rs:96`

### SchDoc deep-dive (AD26 decompiled behavior)

1. Vertex overflow (`EXTRALOCATIONCOUNT`, `EXnn`, `EYnn`) is expected for polygon/polyline/bezier.
   - AD26 polygon/polyline/bezier import calls:
     - `GetVertices().ImportFromFile(..., argIncludeExLocations: true)`
     - `AD26-dotnet/.../FileFormatV5.cs:1168`, `:1214`, `:1247`
   - Overflow keys are emitted/consumed by `SchDataVertices`.
     - `AD26-dotnet/.../SchDataVertices.cs:83` and `:102`
   - Our records currently parse only `Xn/Yn` via `indexed_coords` with `LocationCount`.
     - `crates/altium-format/src/sch_records.rs:994`, `:1020`, `:1038`
   - Result: unknown EX* keys are currently a real parser gap, not a bad fixture.

2. Note record (`RECORD=209`) supports `Alignment` in AD26, but our `SchNote` omits it.
   - AD26 imports Note alignment:
     - `Import_HorizontalAlign(ref ..., "Alignment")`
     - `AD26-dotnet/.../FileFormatV5.cs:2438`
   - AD26 constants include `ParameterNameAlignment = "Alignment"`.
     - `AD26-dotnet/.../FileFormatConsts.cs:121`
   - Our `SchNote` has no `alignment` field.
     - `crates/altium-format/src/sch_records.rs:1801`
   - Result: `UnknownParams { keys: ["ALIGNMENT"] }` is a definite implementation gap.

3. `RECORD=220` in Additional is valid (`HighLevelCodeSymbol`) in AD26.
   - AD26 record-code map:
     - `220 -> eHighLevelCodeSymbol`
     - `AD26-dotnet/.../RtSchematicExt.cs:1149`, `:1246`
   - AD26 object factory maps `case 220` to `SchDataSheetSymbol(..., eHighLevelCodeSymbol)`.
     - `AD26-dotnet/.../FileFormatUtils.cs:347`
   - Our enum already includes `HighLevelCodeSymbol=220`, but SchDoc dispatch has no branch.
     - enum: `crates/altium-format-types/src/sch.rs:113`
     - dispatch fallback: `crates/altium-format/src/schdoc/dispatch.rs:96`
   - Result: `UnknownRecordType(220)` is a definite dispatch-coverage bug.

4. Optional streams are AD26-optional and import is tolerant by existence checks.
   - AD26 `ImportCustomWarehouse` returns early if stream does not exist:
     - `if (!Serializer.StreamExists(...)) return;`
     - `AD26-dotnet/.../SchDataImporterDocumentV5.cs:79`
   - `ReadBinaryBlocksData` also checks `StreamExists` and returns `null` if absent.
     - `AD26-dotnet/.../SchDataImporterDocumentV5.cs:757`
   - AD26 explicitly imports optional streams (`ObjectDefinitions`, `ReuseBlockInfos`, `ReuseBlocks`, `ReuseBlocksV2`, `HarnessConnectionPointConnector`) when present.
   - Our current behavior hard-fails when these optional streams are present.
     - `crates/altium-format/src/schdoc/mod.rs:150`
   - Result: behavior mismatch; should be parsed or at minimum tolerated/preserved.

### SchDoc assessment

- Definite mismatches/gaps:
  - Missing EX vertex overflow handling for RECORD 5/6/7.
  - Missing `Alignment` field on Note (RECORD=209).
  - Missing dispatch mapping for HighLevelCodeSymbol family (at least RECORD=220).
  - Hard-failing valid optional streams.
- Ambiguity:
  - Non-CFB `.SchDoc` fixtures may be non-target format examples in corpus, not parser regressions.
  - Keep as corpus hygiene/fixture-classification issue unless we choose a dual-format parser.

### SchDoc fix direction

- Add overflow-aware vertex parsing for polyline/polygon/bezier:
  - support `EXTRALOCATIONCOUNT`, `EXnn`, `EYnn` and `_FRAC` companions.
- Add `alignment` field to `SchNote` using `ALIGNMENT` key (horizontal-align enum).
- Extend SchDoc dispatch for high-level code records:
  - 220 -> sheet symbol shape
  - 221/222/223 -> sheet entry/name/file-name analogs (same as AD26 mapping family).
- Replace hard-fail on optional streams with:
  - parse if implemented;
  - otherwise preserve as opaque sidecar data and continue invariant validation.
- Keep strict unknown-param checks only after model coverage for known AD26 keys is complete.

## PcbDoc

### Current failures

- Non-CFB `.PcbDoc` fixtures fail at container open.
- Unimplemented storages are treated as hard errors.
- Section record count mismatches cause open failure.
- `PadViaLibrary` binary parsing fails on some fixtures (invalid block header / payload length).

### Concrete examples

- `CfbError("Invalid CFB file (wrong magic number): [7c, 52, 45, 43, 4f, 52, 44, 3d]")`
- `InvalidParamValue { key: "PcbDoc storage", detail: "unimplemented storage /ConstraintManager" }`
- `InvalidParamValue { key: "PcbDoc storage", detail: "unimplemented storage /PrimitiveGuids" }`
- `RecordCountMismatch { section: "PrimitiveParameters", expected: 5, actual: 65 }`
- `InvalidBlockHeader ... parsing /PadViaLibrary/Data`

### Why this currently fails

- Unknown storage names hard-fail.
  - `crates/altium-format/src/pcbdoc/mod.rs:311`
- Count checks are strict in multiple section loaders.
  - `crates/altium-format/src/pcbdoc/mod.rs:238`, `:257`, `:277`, `:297`
- `PadViaLibrary` parser path assumes currently mapped layout only.
  - `crates/altium-format/src/pcbdoc/mod.rs:193`

### PcbDoc deep-dive (docs + decompiled/interface-backed behavior)

1. PcbDoc uses mixed on-disk encodings by section type (not one universal format).
   - Primitive sections are binary-framed records.
     - `docs/pcbdoc/loading-pipeline.md:71-78`
     - `docs/dxp/pcb-files.md:581-589`
   - Parameter sections are length-prefixed `|KEY=VALUE|` blocks.
     - `docs/pcbdoc/loading-pipeline.md:80-90`
   - AD26 interfaces expose a single `IPCB_BinarySection.Import_FromFile(...)` entrypoint, but concrete section types differ (`IPCB_BoardBinarySection`, `IPCB_RequiredBinarySection`, `IPCB_PolygonsBinarySection`, etc.).
     - `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_BinarySection.cs:16`
     - `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_BoardBinarySection.cs:9`
     - `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_RequiredBinarySection.cs:9`
   - Assessment: parser must dispatch by storage kind and decode accordingly; no evidence that the same PcbDoc storage freely flips between param and block framing.

2. `PrimitiveParameters/Header` count semantics are currently mis-modeled.
   - Docs define header count as number of component header groups, not total parameter records.
     - `docs/pcbdoc/sidecar-streams.md:375-377`
     - Hierarchical group layout: `docs/pcbdoc/sidecar-streams.md:382-399`
   - Current code compares header count to flat parsed block count.
     - `crates/altium-format/src/pcbdoc/mod.rs:271-283`
     - `crates/altium-format/src/pcbdoc/records.rs:187-198`
   - Assessment: definite implementation bug causing `expected: 5, actual: 65`-type failures.

3. Valid storages are currently rejected as "unimplemented storage".
   - Current behavior hard-fails unknown top-level storages.
     - `crates/altium-format/src/pcbdoc/mod.rs:311-314`
   - Docs list `ConstraintManager` and `PrimitiveGuids` as real PcbDoc storages/sidecars.
     - `docs/dxp/pcb-files.md:413-417`
     - `docs/dxp/sidecar-streams-deep-dive.md:261`, `:275`
     - `docs/pcbdoc/sidecar-streams.md:18-19`, `:307-309`
   - Assessment: definite coverage gap; these should not be treated as malformed files.

4. `PadViaLibrary*` decoding in PcbDoc is too rigid and likely using the wrong framing path.
   - PcbDoc docs classify `PadViaLibrary`, `PadViaLibraryCache`, `PadViaLibraryLinks` as parameter sections.
     - `docs/pcbdoc/loading-pipeline.md:82-86`
     - `docs/pcbdoc/parameter-sections.md:522-543`
   - Current parser uses `parse_pad_via_library()` which expects block-stream headers (`iter_blocks`).
     - callsite: `crates/altium-format/src/pcbdoc/mod.rs:193-201`
     - parser: `crates/altium-format/src/pcblib/library.rs:432-466`
     - block framing assumption: `crates/altium-format/src/block_stream.rs:1-5`, `:95-107`
   - Assessment: strong mismatch with observed PcbDoc failures (`InvalidBlockHeader`).

5. Primitive section mapping is incomplete for documented section names.
   - Docs include `SplitPlaneRegions6` in primitive section category.
     - `docs/pcbdoc/loading-pipeline.md:73`
   - Current `PrimitiveSectionKind` has no `SplitPlaneRegions6` mapping.
     - `crates/altium-format/src/pcbdoc/records.rs:8-38`
   - Assessment: definite section coverage gap (even if not yet dominant in current fixtures).

### PcbDoc assessment

- Definite mismatches/gaps:
  - `PrimitiveParameters` count invariant uses wrong unit (flat record count instead of component-group count).
  - Valid storages (`ConstraintManager`, `PrimitiveGuids`, likely others) are hard-failed as unimplemented.
  - `PadViaLibrary*` decode path is incompatible with documented PcbDoc parameter-section framing.
  - `SplitPlaneRegions6` is documented but unmapped.
- Ambiguity:
  - Non-CFB `.PcbDoc` fixtures are likely corpus classification issues unless dual-format (ASCII + CFB) support is explicitly in scope.
  - Full binary schema for `ConstraintManager` payload remains unresolved; needs opaque preservation first, typed decoding later.

### PcbDoc fix direction

- Section-format dispatch:
  - Keep strict per-section decoding, but by known storage class (primitive binary vs parameter blocks vs raw binary sidecars).
- `PrimitiveParameters`:
  - implement hierarchical parser (component header + `COUNT` child parameter blocks);
  - validate header count against component header groups, not total blocks.
- Storage handling:
  - replace hard-fail for known-but-unimplemented storages with tolerant load + opaque preserve;
  - continue hard-fail only for truly unknown/unclassified storages (behind strict mode if needed).
- `PadViaLibrary*`:
  - add a PcbDoc-specific parameter-block parser path;
  - keep block-based parser for PcbLib contexts where that framing is actually observed.
- Coverage:
  - add `SplitPlaneRegions6` section mapping and parser plumbing.
- Fixture hygiene:
  - separate non-CFB `.PcbDoc` corpus inputs into a distinct fixture class to avoid conflating format-target failures.

## PcbLib

### Current failures (sampled validation sweep)

- Via parser rejects AD26 extra tail sections (`section3_5`).
- Fill primitive parser reads past end for variant layouts.
- Text enum coverage mismatch (`TextKind=3` invalid in at least one fixture).
- Non-UTF8 pattern-name decoding fails for at least one footprint.

### Concrete examples

- `unmapped AD26 Via sections 3-5 present: 62 bytes remain`
- `primitive #11 (Fill) ... Binary read past end: needed 4 bytes at offset 46, only 0 remain`
- `Invalid enum value: invalid value 3 for enum TextKind`
- `non-UTF8 pattern name: invalid utf-8 sequence of 1 bytes from index 0`

### Why this currently fails

- Via parser intentionally errors if bytes remain after mapped sections 1-2.
  - `crates/altium-format/src/pcblib/primitives/via.rs:150`
- Fill parser assumes either base or full AD26 extension and then exhaustiveness.
  - `crates/altium-format/src/pcblib/primitives/fill.rs:29`
- Text parser enum mapping is strict and does not include observed value(s).
  - `crates/altium-format/src/pcblib/primitives/text.rs`
- Pattern-name decoding assumes UTF-8 where fixtures include non-UTF8 bytes.
  - parse path surfaces as `pattern_name` decode error in runtime validation.

### PcbLib deep-dive (docs + decompiled/ghidra-backed notes)

1. Via trailing sections are known in AD26-era data and we currently fail-fast on them.
   - Our parser explicitly documents unmapped sections 3-5 and errors on remaining bytes.
     - `crates/altium-format/src/pcblib/primitives/via.rs:16`, `:150`
   - Reverse-engineered notes describe multi-section via serialization beyond core+section2:
     - core 246 bytes + section2(N*9) + additional 42-byte block + pad-layer entries (stride 30) + trailing block.
     - `docs/dxp/altium-NOTES.md:1571-1585`
   - Assessment: definite implementation gap (coverage), not random corruption.

2. Fill parser should preserve/handle variable trailing bytes, not assume one fixed extension.
   - Our parser reads 37-byte base, then attempts exactly 13 AD26 bytes and asserts exhausted.
     - `crates/altium-format/src/pcblib/primitives/fill.rs:21-40`
   - Docs explicitly recommend preserving unknown trailing bytes for version variants.
     - `docs/pcblib/binary-primitives.md:261-270`
   - Ghidra-backed notes show AD26 fill trailing fields and shared trailing pattern with track/arc/fill.
     - `docs/dxp/altium-NOTES.md:1587-1619`
   - Assessment: definite parser strategy mismatch (too rigid for variant records).

3. Text kind enum mismatch (`TextKind=3`) is real corpus behavior vs current enum coverage.
   - Current enum only allows 0..2.
     - `crates/altium-format-types/src/pcb.rs:760-775`
   - Corpus has observed value `3` (`Synthiam.PcbLib`).
   - Documentation currently lists only 0=Stroke, 1=TrueType, 2=Barcode.
     - `docs/pcblib/enumerations.md:97-99`
   - AD26 SDK interfaces expose `TTextKind` type, but decompiled interfaces in repo do not clarify an extra variant value.
   - Assessment: ambiguous spec/version behavior; parser should tolerate unknown enum values to avoid hard-fail.

4. Pattern-name decoding should not hard-require UTF-8.
   - Our `Data` pattern name block uses `from_utf8(...)` and fails otherwise.
     - `crates/altium-format/src/pcblib/footprint.rs:149-153`
   - PcbLib docs state ASCII pattern name framing in Data stream.
     - `docs/pcblib/footprint-data-stream.md:83-84`, `docs/dxp/pcb-files.md:848-850`
   - Real corpus has non-ASCII footprint names; display names are often recovered via SectionKeys/Parameters, and Parameters are Win1252.
     - `docs/pcblib/loading-pipeline.md:60-72`
   - Assessment: definite robustness gap; strict UTF-8 assumption is too fragile for real-world files.

5. Additional known PcbLib coverage gaps (even if not hit in this sample run)
   - `CustomShapes` present => hard error.
     - `crates/altium-format/src/pcblib/footprint.rs:242-253`
   - `ModelsNoEmbed` parser intentionally unimplemented for non-empty payload.
     - `crates/altium-format/src/pcblib/mod.rs:550-561`
   - `PrimitiveGuids` PcbLib format remains partially unresolved in docs:
     - `docs/pcblib/sidecar-streams.md:84-97` says format is not fixed and needs more investigation.
     - `docs/dxp/pcb-files.md:865` describes 24-byte/entry (potential conflict).
   - Assessment: clear gaps plus one documentation ambiguity (PrimitiveGuids format details).

### PcbLib assessment

- Definite mismatches/gaps:
  - Via parser rejects known AD26 extra sections instead of preserving/parsing.
  - Fill parser cannot tolerate variant tail layouts.
  - Pattern-name decode path is too strict (`UTF-8` only).
  - `CustomShapes` and non-empty `ModelsNoEmbed` remain unimplemented.
- Ambiguity:
  - `TextKind=3` meaning is not yet confirmed from available AD26 interface docs.
  - PcbLib `PrimitiveGuids` exact binary structure is contradictory across docs and needs focused RE.

### PcbLib fix direction

- Via:
  - parse/preserve sections 3-5 as structured or opaque tails (do not hard-fail on remaining bytes).
- Fill:
  - parse known 37/50 layouts; preserve any additional trailing bytes for forward compatibility.
- Text:
  - introduce tolerant enum handling (unknown numeric variant carried through), then map value 3 once confirmed.
- Pattern names:
  - switch from strict UTF-8 to tolerant decode path (ASCII/Win1252 fallback), with canonical name resolution via Parameters/SectionKeys.
- Sidecars/storage:
  - implement `CustomShapes` passthrough support;
  - implement `ModelsNoEmbed` parsing for non-empty streams;
  - resolve `PrimitiveGuids` format via targeted RE and align docs+code.

## Cross-cutting themes

- We are currently in a good red state for red/green flow: invariants are exposing real gaps.
- Most failures are not random corruption; they indicate version-tolerance and format-coverage gaps.
- Biggest immediate implementation priorities:
  - SchLib `AllPinCount` semantics (AD26-compatible handling).
  - SchDoc tolerant handling for known overflow/aux fields.
  - PcbDoc storage coverage and section-count semantics.
  - PcbLib binary variant support (via/fill/text/encoding).
