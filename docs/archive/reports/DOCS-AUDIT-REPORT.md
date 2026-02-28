# Documentation Audit Report — Final Synthesis

**Date:** 2026-02-28
**Scope:** All documentation in `docs/`, root-level `.md` files, and cross-reference against codebase
**Files Reviewed:** ~160+ markdown files across 19 directories
**Auditors:** 5 parallel agents (dxp, pcb, sch, plans, loose docs)

---

## Executive Summary

The documentation has grown organically through reverse-engineering cycles, creating a
**hub-and-spoke duplication problem**. `docs/dxp/` is the authoritative hub, but each
format-specific directory (pcbdoc/, pcblib/, schlib/, schdoc/) independently re-derived
the same knowledge. Combined with ~120 one-time investigation artifacts (diff reports,
fix notes, validation reports) scattered across root and subdirectories, the result is
a chaotic landscape where:

- **The same format knowledge exists in 3-4 places** with subtle contradictions
- **~120 files are one-time artifacts** that should be archived
- **Implementation plans are stale** — the code has progressed far beyond what plans describe
- **Production APIs have no documentation** — the high-level API is undocumented
- **Root directory has 10+ loose .md files** that belong elsewhere

### By the Numbers

| Category | Count | Action |
|----------|-------|--------|
| Critical contradictions | 6 | Must resolve (format correctness at stake) |
| Files to archive | ~120 | Move to `docs/archive/` |
| Files to delete | ~5 | Duplicates with no unique content |
| Files to consolidate | ~15 | Merge scattered topics into proper locations |
| Files to move | ~10 | Root .md files → proper subdirectories |
| Missing documentation | 3 | High-level API, implementation status, navigation |
| Files that are fine | ~30 | Keep as-is |

---

## 1. Critical Contradictions (MUST RESOLVE)

These are places where documentation disagrees with itself on format-level details. Since
this project controls PCB fabrication file parsing, contradictions here are bugs.

### 1.1 PCB Common Header Field Layout

| Source | Layout |
|--------|--------|
| `docs/pcbdoc/binary-primitives.md` | layer(1) + flags(2) + net(2) + unknown1(2) + component(2) + polygon(2) + unknown2(2) = 13 bytes |
| `docs/pcblib/binary-primitives.md` | layer(1) + gap(1) + flags(2) + net(4) + polygon(2) + component(2) + unknown(1) = 13 bytes |
| Memory (verified via Ghidra) | layer(1) + flags(2) + net(2) + polygon(2) + component(2) + coordinate(2) + dimension(2) = 13 bytes |

**Three different layouts for the same 13 bytes.** The memory doc (Ghidra-verified) is
authoritative. Both pcbdoc/ and pcblib/ versions need correction.

### 1.2 PcbDoc WideStrings6 Format

| Source | Claims |
|--------|--------|
| `docs/dxp/sidecar-streams-deep-dive.md` | Binary TLV with type tags (0x06, 0x0C, 0x12, 0x14) |
| `docs/pcbdoc/shared-with-pcblib.md` | Simple `[u32 index][u32 length][UTF-16LE]`, says TLV is "older format" |
| `docs/pcblib/shared-with-pcbdoc.md` | Binary TLV is the canonical format |

**The two "shared-with" documents contradict each other.** Need to verify against actual
AD26 hexdumps and C# code.

### 1.3 RECORD Byte Encoding for Schematic

`docs/dxp/schematic-records.md` never mentions that RECORD >= 256 uses the
`RECORD=254 + RECORDEX=<actual>` split encoding. `docs/dxp/invariants.md` documents this
clearly. The main record reference is incomplete.

### 1.4 Pad Thermal Entry Size

`PCBDOC-next.md` documents 30-byte format but test files show 23-byte and 29-byte entries.
`docs/dxp/sidecar-streams-deep-dive.md` doesn't cover the variation at all.

### 1.5 SchLib `binary-pin-format.md` Missing Fields

Documentation is missing 3 fields (`owner_index`, `owner_part_id`,
`owner_part_display_mode`) that the Rust code correctly implements.

### 1.6 PcbLib WideStrings vs PcbDoc WideStrings

All 3 docs agree these are different formats, but the actual format of PcbDoc's version
is disputed (see 1.2 above).

---

## 2. Duplication Map

### 2.1 Format Knowledge (docs/dxp/ vs domain directories)

The following content exists in 2-4 places:

| Topic | Authoritative (dxp/) | Duplicate 1 | Duplicate 2 |
|-------|---------------------|-------------|-------------|
| PCB CFB structure | pcb-files.md §3 | pcbdoc/cfb-structure.md (80% same) | pcblib/cfb-structure.md (80% same) |
| PCB binary records | pcb-records.md | pcbdoc/binary-primitives.md | pcblib/binary-primitives.md |
| PCB sidecar streams | sidecar-streams-deep-dive.md | pcbdoc/sidecar-streams.md | pcblib/sidecar-streams.md |
| PCB stream registry | sidecar-streams-deep-dive.md §3 | pcb-files.md §3.1 | invariants.md §4 |
| TObjectId enum | pcb-records.md §2 | pcb-files.md §2 | pcbdoc/enumerations.md |
| SCH CFB structure | sch-files.md §2-3 | schlib/cfb-structure.md (80% same) | schdoc/cfb-structure.md (75% same) |
| SCH loading pipeline | sch-files.md §5-6 | schlib/loading-pipeline.md | schdoc/loading-pipeline.md |
| SCH fileheader | sch-files.md §9 | schlib/fileheader.md | schdoc/fileheader-stream.md |
| SchLib pin sidecars | sidecar-streams-deep-dive.md §2.1 | schlib/pin-sidecar-streams.md | invariants.md §7 |
| File versions | file-versions.md | file-headers.md (overlap) | — |

**Action:** Establish `docs/dxp/` as single source of truth. Domain directories should
contain ONLY format-specific differences, with cross-references to dxp/.

### 2.2 Scattered Topics

| Topic | Files (scattered) | Consolidate Into |
|-------|-------------------|-----------------|
| Ops language | docs/ops-design.md, docs/ops-lang-spec.md, docs/schlib-ops.md, docs/schdoc-ops.md, ops-e2e-gaps.md, ops-lang-checklist.md | `docs/ops/` directory |
| Spec language | docs/spec-lang.md, docs/spec/, docs/plans/spec-lang/, docs/prjpcb/spec-lang-design.md, walkthrough.md | `docs/spec/` directory |
| SCH fixes/reports | docs/SCH-fixes/, docs/schlib-fixes.md, SCH-report.md, SCH-report2.md | `docs/SCH-fixes/` with summary README |
| PCB fixes/reports | PCBDOC-next.md, PCBDOC-diff-fixes.md, PCBLIB-diff-fix.md | `docs/pcbdoc/` or archive |

---

## 3. Staleness Assessment

### 3.1 Plans (docs/plans/) — HEAVILY STALE

| Plan Area | Milestones | Status | Gap |
|-----------|-----------|--------|-----|
| spec-lang | 15 files | ALL IMPLEMENTED | Plans describe what's already built; archive |
| schlib | 11 files | READ+WRITE DONE | Plans only cover read-path; write-path + high-level API undocumented |
| schdoc | 7 files | READ+WRITE DONE | Same — plans don't mention write-path |
| pcblib | 7 files | ALL DONE | PROGRESS.md not updated; all milestones complete |

**Action:** Archive all milestone files to `docs/archive/plans/`. Create per-format
`IMPLEMENTATION-STATUS.md` files describing actual current state.

### 3.2 Investigation Artifacts — STALE

| Directory/Files | Count | Created | Status |
|----------------|-------|---------|--------|
| docs/schlib-diff/ | 100 files | 2026-02-23 | One-time roundtrip analysis; archive |
| docs/schdoc-diff/ | 20 files | 2026-02-23 | One-time roundtrip analysis; archive |
| docs/SCH-fixes/ | 10 files | 2026-02-26 | Mixed: 3 fixed, 4 still open, 3 evolving |
| Root PCB reports | 3 files | 2026-02-25 | Investigation notes; archive |
| Root SCH reports | 2 files | 2026-02-25 | SCH-report.md superseded by SCH-report2.md |
| PROBLEMS.md | 1 file | Unknown | Stale |

### 3.3 Notes — MIXED

| Note | Status | Action |
|------|--------|--------|
| docs/notes/idempotent-api.md | IMPLEMENTED (became spec-lang) | Archive |
| docs/notes/python-api.md | NOT IMPLEMENTED | Move to docs/future/ |
| docs/notes/solverang/ (8 files) | RESEARCH ONLY, no code | Move to docs/future/ |

### 3.4 Model Docs — ACCURATE

All 10 model docs (`docs/model/01-10`) accurately describe the current architecture.
Keep as architectural reference but consider archiving since `docs/dxp/` now supersedes
most of their content.

---

## 4. Proposed Directory Structure

```
docs/
├── dxp/                          # AUTHORITATIVE format reference (keep as-is, fix contradictions)
│   ├── README.md                 # Updated with reading order and navigation
│   ├── container-format.md       # CFB container structure
│   ├── coordinates.md            # Coordinate systems
│   ├── schematic-records.md      # All schematic record types (add RECORDEX note)
│   ├── sch-files.md              # SchDoc/SchLib file structure
│   ├── pcb-records.md            # All PCB record types (fix header)
│   ├── pcb-files.md              # PcbDoc/PcbLib file structure
│   ├── sidecar-streams-deep-dive.md  # Sidecar streams (resolve WideStrings)
│   ├── invariants.md             # Serialization invariants
│   └── ... (other reference docs)
│
├── pcbdoc/                       # PcbDoc-SPECIFIC details only (trim heavily)
│   ├── README.md                 # Brief intro, links to dxp/
│   ├── board-section.md          # UNIQUE: board-level settings
│   ├── loading-pipeline.md       # UNIQUE: PcbDoc load/save specifics
│   └── fileheader.md             # UNIQUE: PcbDoc FileHeader format
│
├── pcblib/                       # PcbLib-SPECIFIC details only (trim heavily)
│   ├── README.md                 # Brief intro, links to dxp/
│   ├── library-storage.md        # UNIQUE: /Library/ storage
│   ├── footprint-data-stream.md  # UNIQUE: pattern name + packed records
│   ├── CustomShape.md            # UNIQUE: custom pad shapes
│   └── loading-pipeline.md       # UNIQUE: PcbLib load pipeline
│
├── schlib/                       # SchLib-SPECIFIC details only (trim heavily)
│   ├── README.md                 # Brief intro, links to dxp/
│   ├── binary-pin-format.md      # UNIQUE: binary pin encoding (fix missing fields)
│   ├── aliases-and-sectionkeys.md # UNIQUE: SchLib-specific
│   ├── component-data-stream.md  # UNIQUE: component data layout
│   └── text-encoding-investigation.md # UNIQUE: encoding findings
│
├── schdoc/                       # SchDoc-SPECIFIC details only (trim heavily)
│   ├── README.md                 # Brief intro, links to dxp/
│   ├── additional-stream.md      # UNIQUE: Additional stream format
│   ├── storage-stream.md         # UNIQUE: Storage stream format
│   └── shared-with-schlib.md     # USEFUL: difference matrix
│
├── ops/                          # NEW: consolidated ops language docs
│   ├── design.md                 # ← from docs/ops-design.md
│   ├── spec.md                   # ← from docs/ops-lang-spec.md
│   ├── schlib-ops.md             # ← from docs/schlib-ops.md
│   ├── schdoc-ops.md             # ← from docs/schdoc-ops.md
│   ├── e2e-gaps.md               # ← from root ops-e2e-gaps.md
│   └── checklist.md              # ← from root ops-lang-checklist.md
│
├── spec/                         # Spec language (keep, consolidate)
│   ├── spec-lang.md              # ← from docs/spec-lang.md
│   ├── design-questions.md
│   ├── pcb-notes.md
│   ├── schdoc-notes.md
│   └── testing-notes.md
│
├── rendering/                    # Keep as-is (well-organized)
│
├── prjpcb/                       # Keep as-is (small, self-contained)
│
├── designs/                      # Keep (low-level-api.md)
│
├── future/                       # NEW: unimplemented proposals
│   ├── python-bindings.md        # ← from docs/notes/python-api.md
│   └── solverang/                # ← from docs/notes/solverang/
│
├── archive/                      # NEW: historical artifacts
│   ├── plans/                    # ← from docs/plans/ (all completed milestones)
│   ├── model/                    # ← from docs/model/ (accurate but superseded by dxp/)
│   ├── schlib-diff-2026-02-23/   # ← from docs/schlib-diff/
│   ├── schdoc-diff-2026-02-23/   # ← from docs/schdoc-diff/
│   ├── pcb-investigation/        # ← PCBDOC-next.md, PCBDOC-diff-fixes.md, PCBLIB-diff-fix.md
│   ├── sch-investigation/        # ← SCH-report.md, SCH-report2.md (keep v2 in SCH-fixes)
│   └── notes/                    # ← docs/notes/idempotent-api.md
│
└── SCH-fixes/                    # Keep (active bug tracking, add README)
    └── README.md                 # NEW: summary of open vs fixed issues
```

### Files to DELETE (duplicates with zero unique content)

| File | Reason |
|------|--------|
| `docs/pcbdoc/cfb-structure.md` | 80% duplicate of dxp/pcb-files.md §3 |
| `docs/pcblib/cfb-structure.md` | 80% duplicate of dxp/pcb-files.md §3.2 |
| `docs/pcbdoc/shared-with-pcblib.md` | Contradicts its counterpart; merge into consolidated doc |
| `docs/pcblib/shared-with-pcbdoc.md` | Contradicts its counterpart; merge into consolidated doc |
| `walkthrough.md` (root) | Duplicate of docs/spec-lang.md |
| `SCH-report.md` (root) | Superseded by SCH-report2.md |
| `PROBLEMS.md` (root) | Stale |

### Files to HEAVILY TRIM (keep only unique content)

| File | Current | Target | What to keep |
|------|---------|--------|-------------|
| `docs/pcbdoc/binary-primitives.md` | 50+ lines | <20 lines | PcbDoc-specific differences only + link to dxp/ |
| `docs/pcblib/binary-primitives.md` | 50+ lines | <20 lines | PcbLib-specific differences only + link to dxp/ |
| `docs/pcbdoc/sidecar-streams.md` | ~100 lines | <30 lines | PcbDoc-specific sidecar details + link to dxp/ |
| `docs/pcblib/sidecar-streams.md` | ~100 lines | <30 lines | PcbLib-specific sidecar details + link to dxp/ |
| `docs/schlib/cfb-structure.md` | 95 lines | <15 lines | Link to dxp/sch-files.md |
| `docs/schdoc/cfb-structure.md` | 88 lines | <15 lines | Link to dxp/sch-files.md |
| `docs/schlib/loading-pipeline.md` | 355 lines | <30 lines | SchLib-specific pipeline details + link to dxp/ |
| `docs/schdoc/loading-pipeline.md` | 215 lines | <30 lines | SchDoc-specific pipeline details + link to dxp/ |

---

## 5. Action Items (Prioritized)

### P0 — Resolve Contradictions (blocks correctness)

1. **Verify PCB common header** against Ghidra + C# code; update all 3 docs to match
2. **Verify PcbDoc WideStrings6 format** — TLV or simple index+length+UTF16?; update dxp/ and delete contradictory docs
3. **Add RECORDEX encoding** note to `docs/dxp/schematic-records.md`
4. **Fix binary-pin-format.md** — add 3 missing fields
5. **Document thermal entry size variations** (23/29/30 bytes)

### P1 — Archive & Delete (reduces clutter, no content risk)

6. Move 120+ investigation artifacts to `docs/archive/`
7. Delete 7 duplicate/superseded files (listed above)
8. Move root-level .md files to proper locations
9. Create `docs/future/` for unimplemented proposals (Python API, solverang)

### P2 — Consolidate Scattered Topics

10. Create `docs/ops/` directory, move 6 ops-related files
11. Create `docs/SCH-fixes/README.md` summarizing open vs fixed issues
12. Merge `shared-with-pcbdoc.md` + `shared-with-pcblib.md` into single `docs/pcb-shared-formats.md` (after resolving contradictions)
13. Trim 8 heavily-duplicated files to link-only stubs

### P3 — Fill Gaps

14. Create `docs/high-level-api.md` documenting the production API surface (read/write/types for SchLib, SchDoc, PcbLib, projects)
15. Create per-format `IMPLEMENTATION-STATUS.md` replacing stale plans
16. Update `docs/dxp/README.md` with reading order and proper navigation
17. Add validation timestamps to model docs

---

## 6. Content Gaps (Missing Documentation)

| Gap | Impact | Priority |
|-----|--------|----------|
| High-level API reference (read/write/types for all formats) | Users can't discover the public API | HIGH |
| Implementation status docs (replacing stale plans) | No accurate "what's done" reference | MEDIUM |
| PCB shared formats (unified pcbdoc/pcblib comparison) | Contradictory docs cause bugs | HIGH (after P0) |
| docs/ top-level navigation / reading guide | New contributors can't find anything | MEDIUM |
| PcbDoc write-path documentation | Write API exists but undocumented | LOW |

---

## Appendix: Audit Coverage

| Auditor | Scope | Files Read | Key Findings |
|---------|-------|------------|-------------|
| dxp-reviewer | docs/dxp/ (27 files) | All 27 | 3 inconsistencies, 5 duplications, 3 incomplete |
| pcb-reviewer | docs/pcbdoc/ + pcblib/ + root PCB (28 files) | All 28 | 3 contradictions, 400+ duplicated lines, 3 stale |
| sch-reviewer | docs/schlib/ + schdoc/ + SCH-fixes/ + diffs (155 files) | All key files + samples | 75-80% dxp overlap, 120 artifact files, 1 doc bug |
| plans-reviewer | docs/plans/ + model/ + notes/ (48 files) | All 48 | Plans stale, model accurate, 2 unimplemented proposals |
| loose-reviewer | Root .md + spec/ + rendering/ + misc (30 files) | All 30 | 10 stale, 4 misplaced, 3 scattered topics |
