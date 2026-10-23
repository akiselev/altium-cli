# Lane altium-format (Copperfield batch 5): R20–R22 and the variant fixes

You are a cloud lane in altium-cli. The Copperfield (ecadabra) batch coordinator, cloud session
`session_01SU3fk4hy1uy9Ye3RWmxkN2`, launched you.

- **Branch and base:** your branch is `claude/ecad-b5-altium-format`. It starts from
  `claude/ecad-b1-altium-r1-r7` at `dd200d4`, the R1–R17 lane, which Copperfield's import lane now
  pins (user, 2026-10-23).
- **Pushing:** push only to your branch, with no pull request. Copperfield moves its pin to your
  branch at its consolidation.

**This repository's `CLAUDE.md` binds you in full:**
- Fail fast. Never retain opaque data, and never skip unconsumed data.
- Use the domain types and constants from `altium-format-types`, checked against the decompiled
  sources.
- Every parsing boundary returns a `Result` with context.
- Keep implementation details private.
- After every step, have an independent agent do a completeness review. You may run the
  fixture tests for the items you touch, with `--features test-fixtures` on the corpus
  repositories.

Read first:
- `docs/handoff/ecad-r1-r7-cloud-report.md`, which records what R1–R17 did and R18–R21's state;
- `docs/handoff/ecad-r1-r7-cloud-lane.md`, the previous lane's brief: setup, corpus and
  evidence rules.

## Items, in order

1. **R22: rule parameters altium-format already parses but doesn't expose.** This comes from
   Copperfield's rules-all lane (its report: `docs/notes/batch5-reports/rules-all.md` on
   ecadabra `claude/ecad-b5-rules-all`).
   - **Urgent: RoomDefinition.** Expose its vertices (`KINDn/VXn/VYn/CXn/CYn/SAn/EAn/Rn`, typed:
     line or arc, with exact coordinates; settle the units of `SAn`/`EAn` from the decompiled
     sources) and `LOCKCOMPONENTS`. Without them, no room rule can be enabled.
   - **Parameters that are parsed but not exposed:**
     - DiffPairsRouting widths;
     - FanoutControl `VIAGRID`;
     - PlaneConnect `RELIEFEXPANSION`;
     - HoleToHole `ALLOWSTACKEDMICROVIAS`;
     - SilkToSolderMask `CLEARANCETOEXPOSEDCOPPER`;
     - ComponentClearance `DONOTCHECKWITHOUT3DBODY`;
     - MatchedLengths' phase and delay keys.
   - **Kinds reported as `Other`.** Type each one fully:
     - test-point style and usage (fabrication and assembly);
     - ComponentRotations, PermittedLayers;
     - the signal-integrity kinds;
     - SilkToBoardRegion, ReturnPath, RoutingNeckDown, WireBonding, ZAxisClearance.
   - **BackDrilling:** read it with AD 26's keys.
2. **R20, the project ERC:** make the level parse fail fast (report §R18–R21).
3. **R21, pad class members:** populated pad classes. Find one in the corpus, or say there is
   none.
4. **Variants and channels** (Copperfield's modularity research, `docs/notes/batch5-reports/
   modularity.md` on ecadabra `claude/ecad-b5-modularity`, requests A1–A5):
   - **A1:** variants are read and written as `[Variant<N>]`
     (`crates/altium-format/src/project.rs:437`, `api/project_write.rs:60`), but Altium writes
     `[ProjectVariantN]`. Real projects' variants are dropped with no error. Fix the read and the
     write, and make an unknown section fail fast.
   - **A2:** `VariationKind` None 0 / NotFitted 1 / Alternate 2 disagrees with KiCad's importer
     (Kind 1 not fitted, Kind 0 alternate). Settle it from the decompiled sources.
   - **A3:** the `ChannelRoomNamingStyle` names disagree with the decompiled enum
     (FlatAlphaWithNames, NumericNamePath, AlphaNamePath).
   - **A4:** SchDocs with reuse-block records 136–138 fail with `UnknownRecordType`. Implement
     them, typed.
   - **A5:** expose `AllowFabrication`, the variation counts, `CHANNELOFFSET` and room vertices,
     `FormatCopy` and `LockComponents`.
5. **The corpus parse failures:** 53 PcbDocs and 487 SchDocs fail at `dd200d4`. Copperfield's
   import lane is grouping their first errors by cause (ecadabra `claude/ecad-b5-import`,
   `docs/notes/batch5-reports/import.md`; the coordinator forwards it when it lands). Fix them
   by cause, the largest group first, for as long as you have context.

Before you stop (near 400k tokens of context, or when you're done), write
`docs/handoff/ecad-b5-altium-format-report.md` and commit it. It gives, per item: what changed,
the evidence (decompiled file and line, or the address), the tests, and the corpus counts before
and after.
