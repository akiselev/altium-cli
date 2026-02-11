# V2 Refactoring Progress

## Wave Status

| Wave | Phases | Status | Started | Completed |
|------|--------|--------|---------|-----------|
| 1 | Phase 0: Scaffolding | COMPLETE | 2026-02-10 | 2026-02-10 |
| 2 | Phase 1A-1F, 5A, 5B: Foundation + Query | COMPLETE | 2026-02-10 | 2026-02-10 |
| 3 | Phase 2A-2B: Macro System | COMPLETE | 2026-02-10 | 2026-02-10 |
| 4 | Phase 3A-3C: Record Types | COMPLETE | 2026-02-10 | 2026-02-10 |
| 5 | Phase 4A-4D, 5C, 6A-6C: Docs/Views/Query/Templates | COMPLETE | 2026-02-10 | 2026-02-11 |
| 6 | Phase 7A-7D: CLI & Ops | NOT STARTED | - | - |
| 7 | Phase 8A-8D: Tests | NOT STARTED | - | - |
| 8 | Phase 9: Cleanup | NOT STARTED | - | - |

## Phase Detail

### Wave 1
- [x] Phase 0: Scaffolding & v1 removal from lib.rs

### Wave 2
- [x] Track 1A: Coordinate system (SchCoord, PcbCoord) — 10 tests
- [x] Track 1B: Backing store types — 10 tests
- [x] Track 1C: ParamCodec trait + primitive impls — 13 tests
- [x] Track 1D: Domain newtypes — 17 tests
- [x] Track 1E: Binary helpers — 14 tests
- [x] Track 1F: Error types update
- [x] Track 5A: Pest grammar + AST — 25 tests
- [x] Track 5B: Query parser + evaluator — 28 tests

### Wave 3
- [x] Track 2A: `#[altium_record]` attribute macro — 9 tests
- [x] Track 2B: `#[altium_enum]` attribute macro — 6 tests

### Wave 4
- [x] Track 3A: Core sch records + enums — 23 enums, 15 records, 45 tests
- [x] Track 3B: Additional sch records — 20 records, 40 tests
- [x] Track 3C: PCB records — 11 PCB enums, 9 records, 36 tests

### Wave 5
- [x] Track 4A: SchLib document + IO — 6 tests
- [x] Track 4B: SchDoc document + IO — 3 tests
- [x] Track 4C: PcbLib document + IO — 6 tests
- [x] Track 4D: View types + wrappers — 5 tests
- [x] Track 5C: Queryable integration — 14 tests
- [x] Track 6A: Sch template functions — 29 templates
- [x] Track 6B: PCB template functions — 9 templates
- [x] Track 6C: Document-level builders — 10 tests

### Wave 6
- [ ] Track 7A: SchLib ops + CLI
- [ ] Track 7B: PcbLib ops + CLI
- [ ] Track 7C: SchDoc + PcbDoc ops
- [ ] Track 7D: CLI main + commands

### Wave 7
- [ ] Track 8A: JSON roundtrip tests
- [ ] Track 8B: CFB roundtrip tests
- [ ] Track 8C: Unit tests + record roundtrips
- [ ] Track 8D: diff-ole.py improvements

### Wave 8
- [ ] Phase 9: Cleanup & final validation

## Gate Results

| Wave | Gate Command | Result | Notes |
|------|-------------|--------|-------|
| 1 | `cargo check --workspace` | PASS | All compiles, v1 removed from tree |
| 2 | `cargo check --workspace` + unit tests | PASS | 117 tests pass, 0 warnings |
| 3 | `cargo check --workspace` + macro tests | PASS | 132 tests pass, 0 warnings |
| 4 | `cargo check --workspace` + record tests | PASS | 253 tests pass, 0 warnings |
| 5 | `cargo check -p altium-format` + lib tests | PASS | 297 tests pass, 1 warning (expected dead_code) |
| 6 | `cargo build --workspace` + CLI output | - | - |
| 7 | `cargo test --workspace -- --ignored` | - | - |
| 8 | Final validation, no v1 code remains | - | - |
