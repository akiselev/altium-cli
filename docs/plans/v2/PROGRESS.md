# V2 Refactoring Progress

## Wave Status

| Wave | Phases | Status | Started | Completed |
|------|--------|--------|---------|-----------|
| 1 | Phase 0: Scaffolding | COMPLETE | 2026-02-10 | 2026-02-10 |
| 2 | Phase 1A-1F, 5A, 5B: Foundation + Query | IN PROGRESS | 2026-02-10 | - |
| 3 | Phase 2A-2B: Macro System | NOT STARTED | - | - |
| 4 | Phase 3A-3C: Record Types | NOT STARTED | - | - |
| 5 | Phase 4A-4D, 5C, 6A-6C: Docs/Views/Query/Templates | NOT STARTED | - | - |
| 6 | Phase 7A-7D: CLI & Ops | NOT STARTED | - | - |
| 7 | Phase 8A-8D: Tests | NOT STARTED | - | - |
| 8 | Phase 9: Cleanup | NOT STARTED | - | - |

## Phase Detail

### Wave 1
- [x] Phase 0: Scaffolding & v1 removal from lib.rs

### Wave 2
- [ ] Track 1A: Coordinate system (SchCoord, PcbCoord)
- [ ] Track 1B: Backing store types
- [ ] Track 1C: ParamCodec trait + primitive impls
- [ ] Track 1D: Domain newtypes
- [ ] Track 1E: Binary helpers
- [ ] Track 1F: Error types update
- [ ] Track 5A: Pest grammar + AST
- [ ] Track 5B: Query parser + evaluator

### Wave 3
- [ ] Track 2A: `#[altium_record]` attribute macro
- [ ] Track 2B: `#[altium_enum]` attribute macro

### Wave 4
- [ ] Track 3A: Core sch records + enums
- [ ] Track 3B: Additional sch records
- [ ] Track 3C: PCB records

### Wave 5
- [ ] Track 4A: SchLib document + IO
- [ ] Track 4B: SchDoc document + IO
- [ ] Track 4C: PcbLib document + IO
- [ ] Track 4D: View types + wrappers
- [ ] Track 5C: Queryable integration
- [ ] Track 6A: Sch template functions
- [ ] Track 6B: PCB template functions
- [ ] Track 6C: Document-level builders

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
| 2 | `cargo check --workspace` + unit tests | - | - |
| 3 | `cargo check --workspace` + macro tests | - | - |
| 4 | `cargo check --workspace` + record tests | - | - |
| 5 | Documents open/save + views + templates | - | - |
| 6 | `cargo build --workspace` + CLI output | - | - |
| 7 | `cargo test --workspace -- --ignored` | - | - |
| 8 | Final validation, no v1 code remains | - | - |
