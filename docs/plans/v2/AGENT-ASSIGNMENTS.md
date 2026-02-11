# Agent Assignment & Execution Schedule

This document maps phases and tracks to agents for maximum parallelism.

## Execution Waves

Work proceeds in **waves**. All tracks within a wave can run simultaneously. A wave cannot start until all tracks in the previous wave are complete.

### Wave 1: Scaffolding (1 agent)

| Track | Agent | Description | Est. Files |
|-------|-------|-------------|------------|
| Phase 0 | Agent-0 | Remove v1 from lib.rs, create module skeleton | ~15 files |

**Gate:** `cargo check --workspace` passes with skeleton modules.

---

### Wave 2: Foundation + Query Grammar (9 agents)

These start as soon as Wave 1 completes:

| Track | Agent | Description | Dependencies | Est. Files |
|-------|-------|-------------|--------------|------------|
| 1A | Agent-1A | Coordinate system (SchCoord, PcbCoord) | Phase 0 | 1 file |
| 1B | Agent-1B | Backing store types (RecordOrigin, etc.) | Phase 0 | 1 file |
| 1C | Agent-1C | ParamCodec trait + primitive impls | Phase 0 | 1 file |
| 1D | Agent-1D | Domain newtypes (Designator, etc.) | Phase 0 | 1 file |
| 1E | Agent-1E | Binary helpers | Phase 0 | 1 file |
| 1F | Agent-1F | Error types update | Phase 0 | 1 file |
| 5A | Agent-5A | Pest grammar + AST types | Phase 0 | 2 files |
| 5B | Agent-5B | Query parser + evaluator | Phase 0 | 2 files |

**Note:** Tracks 5A and 5B can start in Wave 2 because they only need the module skeleton, not the foundation types.

**Gate:** All tracks compile. Unit tests pass for each track.

---

### Wave 3: Macro System (2 agents)

| Track | Agent | Description | Dependencies | Est. Files |
|-------|-------|-------------|--------------|------------|
| 2A | Agent-2A | `#[altium_record]` attribute macro | Phase 1 all | 2-3 files |
| 2B | Agent-2B | `#[altium_enum]` attribute macro | Phase 1C | 1-2 files |

**Gate:** Both macros compile. Test records using the macros compile and pass tests.

---

### Wave 4: Record Types (3 agents)

| Track | Agent | Description | Dependencies | Est. Files |
|-------|-------|-------------|--------------|------------|
| 3A | Agent-3A | Core sch records + all enums | Phase 2 | ~18 files |
| 3B | Agent-3B | Additional sch records | Phase 2 | ~21 files |
| 3C | Agent-3C | PCB records | Phase 2 | ~10 files |

**Gate:** All record types compile. Inline roundtrip tests pass.

---

### Wave 5: Documents, Views, Query Integration, Templates (10 agents)

All of these can start as soon as Wave 4 completes:

| Track | Agent | Description | Dependencies | Est. Files |
|-------|-------|-------------|--------------|------------|
| 4A | Agent-4A | SchLib document + IO | Phase 3 | 2-3 files |
| 4B | Agent-4B | SchDoc document + IO | Phase 3 | 1-2 files |
| 4C | Agent-4C | PcbLib document + IO | Phase 3 | 1-2 files |
| 4D | Agent-4D | View types + wrappers | Phase 3 | 4-5 files |
| 5C | Agent-5C | Queryable integration | Phase 3, 5B | 1 file |
| 6A | Agent-6A | Sch template functions | Phase 3 | 1 file |
| 6B | Agent-6B | PCB template functions | Phase 3 | 1 file |
| 6C | Agent-6C | Document-level builders | Phase 3 | 1 file |

**Gate:** Documents open/save. Views work. Templates create valid records. Query evaluates against records.

---

### Wave 6: CLI & Ops (4 agents)

| Track | Agent | Description | Dependencies | Est. Files |
|-------|-------|-------------|--------------|------------|
| 7A | Agent-7A | SchLib ops + CLI | Phase 4A, 6 | 2-3 files |
| 7B | Agent-7B | PcbLib ops + CLI | Phase 4C, 6 | 2-3 files |
| 7C | Agent-7C | SchDoc + PcbDoc ops | Phase 4B, 6 | 2-3 files |
| 7D | Agent-7D | CLI main + commands | Phase 7A-7C | 5-6 files |

**Note:** 7D depends on 7A-7C but can start the clap structure setup immediately.

**Gate:** `cargo build --workspace` succeeds. CLI produces output for all commands.

---

### Wave 7: Tests (4 agents)

| Track | Agent | Description | Dependencies | Est. Files |
|-------|-------|-------------|--------------|------------|
| 8A | Agent-8A | JSON roundtrip tests | Phase 7 | 2 files |
| 8B | Agent-8B | CFB roundtrip tests | Phase 7 | 3 files |
| 8C | Agent-8C | Unit tests + record roundtrips | Phase 7 | 2+ files |
| 8D | Agent-8D | diff-ole.py improvements | Phase 0 (Python, independent) | 1 file |

**Note:** 8D is Python work and can actually start at any time. Listed here for scheduling clarity.

**Gate:** ALL tests pass: `cargo test --workspace` and `cargo test --workspace -- --ignored`.

---

### Wave 8: Cleanup (1 agent)

| Track | Agent | Description | Dependencies | Est. Files |
|-------|-------|-------------|--------------|------------|
| Phase 9 | Agent-9 | Remove v1 files, clean up | Phase 8 all | Deletions |

**Gate:** Final `cargo test --workspace -- --ignored` passes. No v1 code remains.

---

## Merge Strategy

Each wave should be done on a single branch (or feature branches that merge to a wave branch):

```
master
  └── v2-refactoring (base branch)
       ├── v2/wave-1-scaffolding
       ├── v2/wave-2-foundation (merges wave-1)
       ├── v2/wave-3-macros (merges wave-2)
       ├── v2/wave-4-records (merges wave-3)
       ├── v2/wave-5-documents (merges wave-4)
       ├── v2/wave-6-cli (merges wave-5)
       ├── v2/wave-7-tests (merges wave-6)
       └── v2/wave-8-cleanup (merges wave-7, final PR to master)
```

Within each wave, agents work on separate files with minimal merge conflicts. The file-per-record-type approach in Phase 3 ensures agents don't conflict.

## Conflict Avoidance Rules

1. **`v2/mod.rs`** — only Phase 0 creates it. Other phases ADD `pub mod` lines but don't modify existing ones.
2. **`records/mod.rs`** — Track 3A creates it, 3B and 3C add to it. Use alphabetical ordering to minimize conflicts.
3. **`documents/mod.rs`** — Track 4A creates it, 4B and 4C add to it.
4. **`views/mod.rs`** — Track 4D owns it.
5. **`Cargo.toml`** — only Phase 0 modifies it.
6. **`lib.rs`** — only Phase 0 and Phase 9 modify it.

## Total Agent Count by Wave

| Wave | Agents | Duration Factor |
|------|--------|----------------|
| 1 | 1 | Fast (scaffolding) |
| 2 | 8 | Medium (foundation + grammar) |
| 3 | 2 | Medium (macro development) |
| 4 | 3 | Large (many record types) |
| 5 | 8 | Large (documents + IO) |
| 6 | 4 | Medium (CLI adaptation) |
| 7 | 4 | Medium (test writing) |
| 8 | 1 | Fast (cleanup) |

**Peak concurrency: 8 agents** (Wave 2 and Wave 5)
**Total distinct tracks: 31**
