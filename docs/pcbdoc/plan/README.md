# PcbDoc High-Level API Implementation Plan

## Overview

This plan implements the PcbDoc high-level API as designed in `../high-level-api.md`.
The work is split into phases that each deliver testable, usable functionality.

The PcbDoc API is the biggest remaining gap in the codebase. Parsing and serialization
are complete (94/96 test files passing). This plan adds the public API layer that
unlocks query, rendering, spec language, and programmatic access.

## Phase Dependencies

```
Phase 1: API Types (pcbdoc_types.rs)          ~300 lines
    ↓
Phase 2: Read Path (pcbdoc_read.rs)           ~500-800 lines
    ↓
Phase 3: Write Path (pcbdoc_write.rs)         ~600-1000 lines
    ├──→ Phase 4: CLI Integration             ~600-1000 lines
    │    ├── 4a: info command (~50)
    │    ├── 4b: query adapter (~300-500)
    │    └── 4c: dump (~200-400)
    ├──→ Phase 5: Spec Language               ~1250 lines
    │    ├── 5a: parser (~150)
    │    ├── 5b: model types (~200)
    │    ├── 5c: compiler (~300)
    │    ├── 5d: executor (~200)
    │    └── 5e: reconciler (~400)
    └──→ Phase 6: Rendering                   ~400-600 lines
```

**Minimum viable path**: Phases 1-2 + 4a (info) gives read-only board access.
**Query/render path**: + Phase 4b/4c + Phase 6.
**Full spec path**: + Phase 3 + Phase 5.

Phases 4-6 can proceed in parallel once Phase 3 is complete.
Phase 4a (info) and Phase 6 (render) only need Phase 2 (read-only).

## Key Design Decisions

1. **Typed vectors, not enum**: Primitives stored in `Vec<Track>`, `Vec<Via>`, etc.
   (not a single `BoardObject` enum). Matches the file format's section-per-type
   storage and makes type-safe queries natural.

2. **Stable human-readable IDs**: Every object gets an `id: String` field. Auto-
   generated from type + section index (`track_0`), or user-provided via block-level
   names in the spec language (`track main_bus { ... }`).

3. **Resolved cross-references**: Net/component indices resolved to names at read
   time. `net: Option<String>` rather than `net_index: u16`.

4. **Separate types from PcbLib**: PcbDoc `Track` is different from PcbLib
   `TrackGraphic` (has net, component context). Shared conversion helpers for
   common fields.

5. **Positional-index reconciler fallback**: Handles ID renames gracefully without
   losing Altium's internal UniqueID/GUID.

## Files

- [phase1-types.md](phase1-types.md) — API type definitions
- [phase2-read.md](phase2-read.md) — Internal-to-public conversion
- [phase3-write.md](phase3-write.md) — Public-to-internal conversion
- [phase4-cli.md](phase4-cli.md) — CLI integration (info, query, dump)
- [phase5-spec.md](phase5-spec.md) — Spec language support
- [phase6-render.md](phase6-render.md) — SVG/PNG rendering

## Related Documents

- `../high-level-api.md` — API design, type definitions, ID strategy
- `../../spec-lang.md` §5b — PcbDoc spec syntax
- `../parameter-sections.md` — Board6/Nets6/Components6 parameter formats
- `../binary-primitives.md` — PCB primitive binary record layouts
