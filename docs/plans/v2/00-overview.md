# V2 Refactoring: Implementation Plan Overview

## Goal

Replace the entire v1 API with the v2 backing-store architecture described in `docs/v2-plan.md`. By the end:

- **All v1 modules removed** from the module hierarchy (`types/`, `traits/`, `records/`, `io/`, `ops/`, `templates/`, `dump/`)
- **No deprecated functionality** — everything uses the v2 API
- **Original roundtrip tests pass** (JSON export/import, CFB read/write/re-read)
- **All CLI commands work** through the new v2 API

## Phase Dependency Graph

```
Phase 0: Scaffolding (MUST be first - single agent)
    │
    ├──────────────────────────────────────────────────────┐
    ▼                                                      ▼
Phase 1: Foundation Types                          Phase 5: Query Language
(6 parallel tracks: 1A-1F)                         (3 parallel tracks: 5A-5C)
    │                                                      │
    ▼                                                      │
Phase 2: Macro System                                      │
(2 parallel tracks: 2A-2B)                                 │
    │                                                      │
    ▼                                                      │
Phase 3: Record Types                                      │
(3 parallel tracks: 3A-3C)                                 │
    │                                                      │
    ├──────────────────────────────────────────────────────┘
    ▼
Phase 4: Documents, Views & IO
(4 parallel tracks: 4A-4D)
    │
    ▼
Phase 6: Templates & Builders
(3 parallel tracks: 6A-6C)
    │
    ▼
Phase 7: CLI & Ops Migration
(4 parallel tracks: 7A-7D)
    │
    ▼
Phase 8: Tests & Validation
(4 parallel tracks: 8A-8D)
    │
    ▼
Phase 9: Cleanup & Final Validation
(single agent)
```

## Parallelism Summary

| Phase | Parallel Tracks | Total Agents | Blocking? |
|-------|----------------|--------------|-----------|
| 0: Scaffolding | 1 | 1 | Yes - must complete first |
| 1: Foundation | 6 (1A-1F) | 6 | Blocked by Phase 0 |
| 2: Macros | 2 (2A-2B) | 2 | Blocked by Phase 1 |
| 3: Records | 3 (3A-3C) | 3 | Blocked by Phase 2 |
| 4: Docs/Views/IO | 4 (4A-4D) | 4 | Blocked by Phase 3, 5 |
| 5: Query Language | 3 (5A-5C) | 3 | Blocked by Phase 0 only |
| 6: Templates | 3 (6A-6C) | 3 | Blocked by Phase 3 |
| 7: CLI/Ops | 4 (7A-7D) | 4 | Blocked by Phase 4, 6 |
| 8: Tests | 4 (8A-8D) | 4 | Blocked by Phase 7 |
| 9: Cleanup | 1 | 1 | Blocked by Phase 8 |

**Maximum concurrent agents: 9** (Phase 1 + Phase 5 running simultaneously)

## Key Principles

1. **v1 modules are removed from `lib.rs` immediately** (Phase 0). Builds break. This is intentional — it forces all work to target v2.

2. **v1 source files are NOT deleted** until Phase 9. They serve as a knowledge base for field names, param keys, binary offsets, enum values, etc. Agents reference them but never import from them.

3. **The current `v2/` module is also replaced.** The current v2 uses typed struct fields. The new v2 uses backing-store access. Current v2 files serve as knowledge base alongside v1 files.

4. **Each phase has clear acceptance criteria.** A phase is not complete until all criteria pass.

5. **Agents should run `cargo check` frequently** to validate their work compiles against the shared module structure.

## File Conventions

All new v2 code lives under `crates/altium-format/src/`:

```
src/
  lib.rs                          # Rewritten in Phase 0
  error.rs                        # Updated in Phase 1F
  format/                         # Kept as-is (binary constants)
  v2/                             # NEW - complete rewrite
    mod.rs                        # Phase 0
    backing_store.rs              # Phase 1B
    coord.rs                      # Phase 1A
    traits.rs                     # Phase 1C (ParamCodec, AltiumCoord, etc.)
    binary_helpers.rs             # Phase 1E
    newtypes.rs                   # Phase 1D
    records/                      # Phase 3 (macro-generated record types)
      mod.rs
      sch_pin.rs
      sch_component.rs
      sch_arc.rs
      ... (one file per record type)
      pcb_pad.rs
      pcb_track.rs
      ...
    views/                        # Phase 4D (hand-written wrapper types)
      mod.rs
      sch_component_view.rs
      leaf_wrappers.rs
      pcb_footprint_view.rs
    documents/                    # Phase 4A-4C (document types + IO)
      mod.rs
      schlib.rs
      schdoc.rs
      pcblib.rs
    query/                        # Phase 5 (AQL parser)
      mod.rs
      grammar.pest
      ast.rs
      eval.rs
    templates.rs                  # Phase 6A-6B
    builders.rs                   # Phase 6C
    ops/                          # Phase 7 (CLI operations)
      mod.rs
      schlib.rs
      schdoc.rs
      pcblib.rs
      pcbdoc.rs
```

## Cargo.toml Changes (Phase 0)

Add to `crates/altium-format/Cargo.toml`:
```toml
pest = "2.7"
pest_derive = "2.7"
```

## Reference: Current v1/v2 Module Map

Files to use as **knowledge base** (read, don't import):

| Current File | Knowledge It Contains |
|---|---|
| `v2/fields/pin.rs` | All 50+ pin field names, param keys, types |
| `v2/fields/component.rs` | Component field definitions |
| `v2/fields/primitives.rs` | Arc, Line, Rectangle, etc. field defs |
| `v2/fields/schematic.rs` | Wire, Bus, Port, Power, etc. |
| `v2/types.rs` | All enum type definitions with values |
| `v2/coord.rs` | V2Coord implementation (100K units/mil) |
| `v2/pcb/pad.rs` | PcbPad binary structure, field offsets |
| `v2/pcb/primitive.rs` | PcbCommonHeader, trailing fields |
| `v2/io/schlib.rs` | SchLib CFB structure, section keys |
| `v2/io/schdoc.rs` | SchDoc flat stream parsing |
| `v2/pcb/io/pcblib.rs` | PcbLib CFB structure |
| `v2/serializer/ascii.rs` | Parameter string format details |
| `v2/serializer/binary.rs` | Binary field format details |
| `types/parameters.rs` | ParameterCollection implementation |
| `types/unknown.rs` | UnknownFields (to be eliminated) |
| `types/coord.rs` | v1 Coord (10K, reference only) |
| `types/layer.rs` | Layer constants (reuse in v2) |
| `types/color.rs` | Color type (reuse in v2) |
| `io/reader.rs` | Low-level CFB reading functions |
| `io/writer.rs` | Low-level CFB writing functions |
| `ops/schlib.rs` | CLI operation logic for SchLib |
| `ops/pcblib.rs` | CLI operation logic for PcbLib |
