# Report: altium-format for Copperfield batch 5 (R20–R23, the variant fixes, corpus failures)

Date: 2026-10-23. Lane brief: `docs/handoff/ecad-b5-altium-format-lane.md` (on
`claude/ecad-b1-altium-r1-r7` `e611f8f`). Coordinator: cloud session
`session_01SU3fk4hy1uy9Ye3RWmxkN2`.

## Branch

- `claude/ecad-b5-altium-format`, from `e611f8f` (the lane brief on top of `dd200d4`, R1–R17).
  No rebase, no pull request.
- Commits:

| Commit | Content |
|---|---|
| `dbe7367` | R22: every rule kind typed; room outlines and flags; parsed-but-hidden keys |
| `704229b` | R22 review fixes; R20; R21 (+ Q15 classes and diff pairs); A1, A3–A5; ASCII SchDoc; IgnoreOnLoad; harness owners; Dimensions6 (R23) |
| `c8f8fa8` | A2: VariationKind 0 and 2 refused (coordinator decision) |
| `8a8f04a` | second review fixes |
| next | R1–R17 corpus tests kept on their 79 base boards |
| this commit | this report |

## Sources read

- Copperfield reports:
  - ecadabra `claude/ecad-b5-rules-all` `docs/notes/batch5-reports/rules-all.md`;
  - `claude/ecad-b5-modularity` `…/modularity.md`;
  - `claude/ecad-b5-import` `…/import.md`, already landed, so item 5 used its grouping.
- **Decompiled evidence: altium-re `3e7f5e9`**, added to the session read-only. AD26-dotnet is not
  in this checkout. Used:
  - `port/Ppcb1-rules/rule-params.tsv`: AD 26 rule readers and writers per kind (Advpcb.dll 26.2.0.7);
  - `port/Psch2-erc/erc-catalog.tsv` and `src/erc.rs.md` (ERCConnectionMatrix.ParseFromStrings);
  - `port/Psch1-net-derivation/structs.md` (TChannelRoomNamingStyle, ilspy of RT_Workspace).
- Corpora: pcbdoc `6bc5aa7`, schdoc `6d0ff22`, pcblib `c5dfdc7`. They are linked as `data/*`
  (ignored) and are not committed.

## Per item

### R22: rule parameters (done; RoutingNeckDown refused)

- **What changed:**
  - **No more `Other`.** Every rule kind the reader accepts has a typed `RuleParams` variant;
    `Other` remains only for `RuleKind::None`.
  - **Rooms.** `ConfinementConstraint` gains:
    - `outline: PcbContour`, read by the shared Board6/Polygons6 contour reader (exact
      coordinates, line/arc typed, partial vertex or gap is an error, empty outline is an
      error);
    - `layer`, `lock_components` (LOCKCOMPONENTS) and `format_copy` (FORMATCOPY), all required.
  - **Newly exposed:**
    - DiffPairsRouting and Width per-layer widths/gaps (`V6Layer` per key prefix), substacks
      and impedance;
    - FanoutControl `via_grid`;
    - PlaneConnect `relief_expansion` plus the `PAD.`/`VIA.` settings (new
      `ConnectStyleTarget` enum); PolygonConnect `THPAD.`/`SMDPAD.`/`VIA.`;
    - HoleToHole `allow_stacked_microvias`; SilkToSolderMask `clearance_to_exposed_copper`;
      ComponentClearance `do_not_check_without_3d_body`/`show_distances`;
    - MatchedLengths phase and delay keys.
  - **Kinds that were `Other`:** test-point style/usage (fabrication, assembly),
    ComponentRotations, PermittedLayers, SignalStimulus, the SI limits (`SignalMaximum`,
    `SignalMinimum`, `MaxMinImpedance`, `MaxSlope`), SupplyNets, ReturnPath, WireBonding,
    ZAxisClearance, SilkToBoardRegion, NetsToIgnore, UnconnectedPin, LayerStack.
  - **BackDrilling** reads AD 26's six keys (MaxStubLength, UseTop/BottomBackdrill,
    HoleOverSize, HoleTolPlus/Minus); BACKDRILLINGDEPTH is now an error.
  - **The key rule.** Keys follow AD 26's readers and the corpus:
    - a key every corpus record of that kind stores is required;
    - a key some records omit is `Option` (AD 26 keeps its constructor default for an absent
      key, which is unread: Ppcb1-rules B-DRC-1);
    - keys that neither AD 26 nor the corpus has are removed (they now fail): BACKDRILLINGDEPTH,
      StubLength MAXLIMIT, Creepage GAP/VOLTAGE, ParallelSegment PARALLELLENGTH/CHECK*,
      MatchedLengths AMPLITUDE/GAP/STYLE, DiffPairs MOSTFREQWIDTH/IMPEDANCEDRIVEN, the old SI
      keys (MAXOVERSHOOT…, MINVALUE/MAXVALUE, STIMULUSTYPE/SIGNALLEVEL);
    - per-kind checks: fabrication test points need SIDE, VALID and ALLOWMULTIPLE; assembly
      test points refuse SIDE and BOARDEDGECLEARANCE; plane/polygon connect needs all five
      top-level keys or per-object settings only.
  - **Fixed defects:**
    - SMDToCorner/SMDToPlane `DISTANCE` is an int in internal units (AD 26 helper 0x120bf30);
      it was read as mils, so `40157` was a metre.
    - MaximumViaCount reads `LIMIT` (rule-params.tsv ord 18); it read MAXVIACOUNT.
    - Silent defaults were removed from the public conversion: 19 PlaneConnect rules with only
      `PAD.`/`VIA.` settings read as `connect_style` default with 4 spokes.
    - AcuteAngle MINIMUM is written in Altium's float form.
  - **SI values** (`SignalValue`) are parsed strictly and accepted only in the stored
    4-significant-digit form that writes back identically (" 51.00", "-1.000").
- **RoutingNeckDown is refused.** AD 26 stores one `LayerMaxLength` string (import 0x4ca5df0)
  whose encoding is unknown, and no corpus rule exists.
- **SA/EA units: degrees.** The decompiled sources reachable here do not settle them (the
  room vertex reader is not in the Ppcb1-rules packet). The corpus does: of 806 arc vertices
  in Board6 and Polygons6, which share the KINDn layout, 805 have both ends on the arc when
  read as degrees and 0 when read as radians. No corpus room has an arc.
- **Evidence:** rule-params.tsv rows cited in the doc comments of
  `crates/altium-format/src/pcbdoc/drc.rs`; a survey of all corpus Rules6 keys per kind.
- **Tests:**
  - unit tests in `pcbdoc/drc.rs`: room with an arc and round trip; partial vertex, missing
    LOCKCOMPONENTS and no vertices refused; SMD internal units; BackDrilling; RoutingNeckDown
    refused; SI values and SI round trip; per-object PlaneConnect; MaximumViaCount;
    per-kind test points;
  - `param_value` `signal_value_round_trips_corpus_forms`;
  - corpus `corpus_r22_rules`.
- **Corpus:** 4,168 rules in all 94 openable PcbDocs convert, none untyped.
  - Readable boards: 94 rooms (59 enabled), 823 vertices, all line edges, none locked.
  - 17 PlaneConnect and 37 PolygonConnect rules use per-object settings.
  - Every rule writes and re-reads to the same public params.
  - Counts per variant are asserted.

### R20: project ERC (done)

- **What changed:**
  - `parse_erc_levels` returns a `Result`. A key must be `Type1`..`Type165`, or
    `MultiChannelAlternate` / `AlternateItemFail` (kinds 119 and 120, which are stored by
    name); the level must be 0–3. Anything else is an error. The constants
    `ERC_ERROR_KIND_COUNT` and `ERC_LEVEL_NAMED_KEYS` are new in altium-format-types
    `project.rs`.
  - `[Modification Levels]` (0/1) and `[Difference Levels]` were silently skipped through
    `filter_map`; they are now strict too.
  - The ERC matrix: an absent section is all NoReport, as in Altium; a present section needs
    L1..L17 exactly.
  - `ErcConnectionMatrix` documents that Altium reads it transposed.
- **Evidence:** erc-catalog.tsv (165 kinds, store keys) and erc.rs.md
  (`ParseFromStrings`, transposed `ErrorLevel`).
- **Tests:** `erc_levels_and_matrix_parse_strictly` (the AD 26 blank project parses with 165
  levels; bad keys, values and rows are refused). No PrjPcb is in the corpora.

### R21: pad class members (none in the corpus)

- **Finding:** the corpus has no populated user pad class. All 79 pad classes on readable
  boards are the built-in `All Pads` superclass; one of them (textbook-smart-mcu) stores a single
  member, `M0=PowerSupply`. Since no member form can be verified, pad members stay the stored
  strings.
- **Classes6 fixes on the way (Q15):**
  - read: KIND, NAME and SUPERCLASS are required, a member parse error propagates, a gap in
    Mn is an error, and `UNICODE__Mn` sidecars are applied (36 members);
  - new `NetClass::super_class`;
  - the writer (`merge_class_section`) matches by name and kind, which fixes repeated names
    across kinds, and drops stale `Mn` and Unicode keys.
- **DifferentialPairs6 fixes on the way (Q15):** the reader reads POSITIVENETNAME and
  NEGATIVENETNAME (716 pairs read empty nets before), and the writer writes those keys,
  merged by NAME so UNIQUEID and GATHERCONTROL survive.
- **Tests:** `corpus_r21_classes_and_pairs` checks:
  - every pair's nets are board nets;
  - classes and pairs are unchanged by `update_board`, and stored pair records match exactly;
  - renaming a Unicode member and dropping one reads back.

### A1: variants (done; Kind values unverified)

- **What changed:**
  - `[ProjectVariantN]` on read and write; the old `[VariantN]` is now an unknown section.
  - An unknown section, a key in one, or a line that is neither a header nor KEY=VALUE is an
    error. They were silently dropped by both read and write.
  - Strict variant parse:
    - known keys only;
    - `Variation1..n` need Designator, UniqueId, Kind and AlternatePart;
    - `ParamVariation1..n` need Designator, ParameterName and VariantValue;
    - VariationCount must match the lines; a gap in the numbering is an error;
    - `ParameterCount > 0` is refused, because its layout is unverified.
  - `ProjectVariant::allow_fabrication: Option<bool>`; `overwrite_pcb_footprint` is now
    `Option<bool>`.
- **Evidence:** `docs/spec-lang/blocks/prjpcb.md` and KiCad's importer, per the modularity
  report. No PrjPcb with variants is in any corpus, so the key set beyond the existing code is
  not verified against a file.
- **Tests:** `project_variants_read_and_write_as_project_variant_sections`,
  `test_unknown_section_refused`.

### A2: VariationKind (fail fast; settled only for Kind 1)

- **The gap:** no decompiled source and no PrjPcb was reachable. Our enum (None 0 / NotFitted
  1 / Alternate 2) agrees with KiCad's importer on Kind 1 = not fitted and disagrees on 0
  (KiCad: alternate).
- **Coordinator's decision:** keep NotFitted = 1. Kind 0 and Kind 2 are now hard errors with
  context ("unverified VariationKind Kind=N, see the A2 request"); tests in
  `project_variants_read_and_write_as_project_variant_sections`.
- **Two ways to settle it:**
  - altium-re reads WorkspaceManager.dll's PrjPcb variant reader and `IsFitted` (modularity's
    altium-re request);
  - the user adds a real PrjPcb with a not-fitted part and an alternate part to a corpus.

### A3: ChannelRoomNamingStyle (done)

- **What changed:** values 1–3 are renamed to FlatAlphaWithNames, NumericNamePath and
  AlphaNamePath. The spec strings change with them: `flat_alpha_with_names`,
  `numeric_name_path`, `alpha_name_path`.
- **Evidence:** altium-re Psch1-net-derivation `structs.md`: `TChannelRoomNamingStyle`
  FlatNumericWithNames 0, FlatAlphaWithNames 1, NumericNamePath 2, AlphaNamePath 3,
  MixedNamePath 4.

### A4: reuse-block records 136–138 (not implemented; refused with context)

- No corpus SchDoc or SchLib contains RECORD=136, 137 or 138 (a byte search of all files).
- The AD 26 serializer of SchematicBlock and ReuseSheetSymbol is not available here: no
  AD26-dotnet, no altium-re packet. Neither are their base-class keys, or where
  ReuseBlockImplementationInfo attaches. Typing them would mean guessing keys.
- They now fail with `RECORD=136 (SchematicBlock): reuse-block records are not supported
  (serializer not yet reverse engineered; no fixture)`.
- The import report's "record type 44" was not RECORD=44. It is the `SchRecord` variant ordinal
  of `HarnessEntry` (see item 5).

### A5: exposures (done)

- `AllowFabrication` (A1).
- The variation counts are checked, not stored.
- Room vertices, `FormatCopy` and `LockComponents` (R22).
- `PcbDocComponent::channel_offset: Option<u32>` (CHANNELOFFSET; absent on 597 of 14,409
  records). It is read and written, and no longer a default-only key.

### Item 5: corpus parse failures

**PcbDoc:** every CFB PcbDoc that opens now reads, 94 of 94 (79 at `dd200d4`).

- **Fixed: Dimensions6 (R23).** The 15 boards failed on `TEXTX=3800mil`. All 92 corpus
  dimension records store TEXTX, TEXTY, TEXTHEIGHT and TEXTWIDTH as `<decimal>mil`. They are now
  required `ExactMilCoord`s, and DIMENSIONKIND is required and checked; an unknown kind used to
  read as NoDimension.
- **Still failing:** 36 ASCII PcbDocs (R24) and 2 CFB files without FileHeaderSix (R25). These
  were not started.

**SchDoc:** 1,293 of 1,617 open (1,202 at `dd200d4`); 1,293 sheets (1,130). Every opened SchDoc
now converts.

- **Fixed:**
  - ASCII SchDoc (R26): read through the binary record path, saved as CFB. A non-empty Icon
    storage or additional section is refused (none in the corpus).
  - `IgnoreOnLoad` (R27, 98 files): typed on the record base.
  - Harness owners: what the import report filed as R28 "RECORD=44" (147 sheets) was
    HarnessEntry records in `/Additional` whose owner is in the additional list.
    `OwnerIndexAdditionalList` was honoured for pins only, so entries attached to main records:
    sheet level, components, sheet symbols, or wires, where they were silently dropped.
  - `SELECTION` on RECORD=7 (R27): not fixed separately. That file fails earlier.

| First error now | Files | Note |
|---|---|---|
| DISPLAYMODE holds a footprint name | 170 | EasyEDA/LCSC exports (19-char `gge…` ids); not Altium-written, refused |
| ASCII-only keys: TRANSPARENT/LINESTYLE on RECORD=10/12, pin ISNOTACCESIBLE/FONTID, ALIGMENT (RECORD=18), NOTUSEDDBTABLENAME (RECORD=1), ORIENTATION/ISMIRRORED (RECORD=28), SECONDARYRADIUS (RECORD=12) | about 96 | the binary records omit these; settling them needs AD 26's ASCII writer |
| embedded image with no storage object | 20 | R31; a refusal, as before |
| RECORD=220 code-symbol routine keys (EXPORTEDROUTINECOUNT…) | 14 | R29; not typed |
| RECORD=215 `PrimaryConnectionPosition_Frac` | 14 | R29; not typed |
| `%UTF8%%UTF8%TEXT` (doubled prefix, no TEXT) | 6 | malformed third-party export |
| sheet font table entries missing | 5 | R30; not started |
| SELECTION (RECORD=22), STARTLINESHAPE empty, merge conflict | 3 | |

### Open: R1–R17 checks on the 15 newly readable boards

The R1–R17 corpus tests stay on the 79 boards that read at the base rev; they skip the 15
boards in `BOARDS_FAILING_AT_BASE` explicitly. On the 15 new boards, two R-items' checks
fail:

- **R3 (an observation, not a defect):** tvws-agc-calib has tracks inside a polygon
  (`track_1541`, polygon 0). On the 79 base boards the only pour members were regions, so
  the importer must expect track/arc pour members (hatched pours).
- **R5 (a possible writer defect, not investigated):** on tvws-wab-1x4, after
  `corpus_r5_pad_modes` inserts a pad, the first pad's solder cache state reads Invalid
  instead of Valid. `merge_mask_cache_state` marks a rule pad Invalid when its expansion value
  differs, so a value read differently on that board is the likely cause.
- **Recommendation:** the next format lane runs every R-item check over all 94 boards and fixes
  R5 there.

## Tests (exact commands)

- **Unit tests:**
  `CARGO_TARGET_DIR=/tmp/target-altium CARGO_PROFILE_DEV_DEBUG=0 cargo test --no-fail-fast -p altium-format -p altium-format-spec -p altium-cli -p altium-format-types`
  - altium-format: 548 passed. The 4 failures are the `intlib::tests::*` tests that need
    the uncloned `data/intlib`; they fail at the base too.
  - altium-format-spec: 248 passed; altium-format-types: 20 passed; altium-cli: 1 + 1 passed.
- **Corpus tests** (all `corpus_*`, the coordinator-approved fixture set):
  `CARGO_TARGET_DIR=/tmp/target-altium-opt2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_DEV_OPT_LEVEL=2 cargo test -p altium-format --features test-fixtures --lib corpus_ -- --test-threads 3`
  Result: __CORPUS__
- `cargo check --workspace --tests` is clean; `cargo fmt --all --check` is clean.
- No proptests and no other fixture tests were run.

## Review verdicts

| Step | Verdict | Fixed |
|---|---|---|
| R22 (`dbe7367`) | complete with nits, 2 blocking: MaximumViaCount key; SignalValue rounding on save | both blocking, plus: removed unevidenced keys, per-kind test-point checks, all-or-none connect keys, corpus test over every file's rules, unit tests. Not fixed: raw `SIDE`/`VALID`/stimulus ints (Q3); rule-base `KEEPOUT` default (pre-existing, Q5) |
| Second commit (`704229b`) | **incomplete**, 3 blocking | All 3 fixed in `8a8f04a`: (1) Classes6 writer dropped Unicode sidecars, so a class keeps its stored record when unchanged and a changed one with a non-Windows-1252 member is refused; (2) the SchDoc validator kept a pins-only `owner_ref`, now one shared helper; (3) fixture-free tests for harness owners, IgnoreOnLoad, Classes6, DifferentialPairs6 and CHANNELOFFSET. Nits left open: ASCII errors say "/FileHeader block #N", not line N; the in-memory header string stays ASCII after an ASCII read; Classes6/DiffPairs6 do not exhaust other keys; DiffPairs6 name sidecars not applied; harness children are written to /FileHeader after an `update_sheet`; Dimensions6 is read-only; the variant key set and section list are unverified against a real PrjPcb (they may reject real projects, e.g. `[GeneratedDocumentN]` or `ParamDesignatorN`, if those exist); IgnoreOnLoad is not visible in the sheet API |

## API breaks

- **`RuleParams`:**
  - New variants; a match without a wildcard breaks.
  - Changed fields:
    - Clearance `ignore_pad_to_pad: Option<bool>`;
    - Length delay fields Option;
    - MatchedLengths optional fields;
    - ParallelSegment `gap, limit` only;
    - `DaisyChainStubLength { limit }` (was `max_limit`);
    - BrokenNets/UnpouredPolygon Options;
    - SolderMask/PasteMask Options;
    - Power/PolygonConnect top-level fields are Options, plus `per_object`;
    - RoutingViaStyle templates;
    - ComponentClearance extras;
    - BackDrilling's six fields (was `depth`);
    - CreepageDistance `check_distance` (was `gap`);
    - AcuteAngle `check_tracks_only`.
  - `rule_params_from_internal` is fallible, so `board()` fails on a bad room layer.
- `NetClass::super_class`, `PcbDocComponent::channel_offset`, `ProjectVariant::allow_fabrication`,
  and `overwrite_pcb_footprint: Option<bool>` are new fields: struct literals break (filled in
  spec and cli).
- `ChannelRoomNamingStyle` variants are renamed, along with their spec strings.
- **Stricter:**
  - unknown PrjPcb sections and lines;
  - variant keys;
  - ERC/modification/difference levels;
  - Classes6 KIND/NAME/SUPERCLASS;
  - DifferentialPairs6 keys;
  - Dimensions6 keys;
  - the rule keys above;
  - RoutingNeckDown.
- `ConnectStyleTarget` is a new type in altium-format-types; it needs the user's approval (Q2).

## Questions (each with a recommendation)

1. **A2 VariationKind:** answered by the coordinator, and done as above. Open until one of the
   two ways above settles Kind 0 and Kind 2.
2. **New domain type `ConnectStyleTarget`** (per-object rule prefix). Recommendation: approve.
3. **Raw integers:** test-point `side` and `valid`, and SignalStimulus `kind` and `level`, stay
   stored integers; corpus `SIDE=3` looks like a top/bottom mask. Recommendation: altium-re
   reads `TTestPointStyleRule.Side` and `TSignalStimulus.Kind/Level`, then add enums.
4. **RoutingNeckDown `LayerMaxLength`.** Recommendation: altium-re decodes 0x4ca5df0 and
   0x4ca5f60; until then the rule is refused.
5. **Rule base defaults (pre-existing):** `PcbRuleBase` fills absent KEEPOUT/LAYER/… with
   defaults (KEEPOUT is absent on about 6% of records). Recommendation: make them `Option` in
   the next rules pass.
6. **ASCII-only SchDoc keys (about 96 files).** Recommendation: have altium-re read AD 26's
   ASCII export of those records before typing them. Until then they are refused.
7. **Reuse blocks 136–138.** Recommendation: a dev-machine lane with the C# serializer and an
   AD-made fixture.

## What Copperfield's importer must change

- **Rooms:** build the `RuleArea` from `ConfinementConstraint::outline` on `layer`; map
  `lock_components`; enable the 59 enabled rooms. This retires `api.rule-room-area`.
- **Other rules:** map the new variants and the per-object connect settings.
  `api.rule-kind-params` should only remain for RoutingNeckDown.
- **Optional fields:** treat `None` as "not stored". Do not substitute a default.
- **SMD distances:** SMDToCorner and SMDToPlane `distance` is now correct (internal units).
- **Diff pairs:** they now carry nets, which retires `model.diffpair-missing-net` (716).
- **Dimensions:** the 15 boards that failed on Dimensions6 import now.
- **SchDocs:** 91 more open, including ASCII; harness connectors carry their entries.
- **Variants:** the PrjPcb import reads `[ProjectVariantN]`.

## Setup as it ran

- Host: cloud `vm`, 4 CPUs, 15 GiB, 30 GB free; no `/mnt/c`; rustc 1.97.0.
- Corpora are shallow clones in `/tmp/corpus/{pcbdoc,schdoc,pcblib}`, linked as `data/*`.
- Survey scripts in `/tmp/tools` (python3 -I, olefile from the repo's wheel in `/tmp/pyenv`),
  outside the repository.
- Builds: `/tmp/target-altium` (debug), `/tmp/target-altium-opt2` (fixture runs, opt-level 2).
