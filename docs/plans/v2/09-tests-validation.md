# Phase 8: Tests & Validation

**Agents: 4 parallel tracks (8A, 8B, 8C, 8D)**
**Blocked by: Phase 7 (CLI/Ops)**

This phase migrates all existing tests to use the v2 API and adds new tests required by the v2 plan. The CRITICAL tests are the JSON roundtrip and CFB roundtrip tests — these must pass for the refactoring to be considered complete.

---

## Track 8A: JSON Roundtrip Tests (CRITICAL)

**Files:**
- `crates/altium-format/tests/v2_schlib_roundtrip.rs` (rewrite)
- `crates/altium-format/tests/v2_pcblib_roundtrip.rs` (rewrite)

**Reference:**
- Current `tests/v2_schlib_roundtrip.rs` — existing test logic
- Current `tests/v2_pcblib_roundtrip.rs` — existing test logic

### What to Build

These tests prove that the v2 API correctly reads, serializes to JSON, and deserializes back.

#### SchLib JSON Roundtrip

```rust
#[test]
#[ignore] // requires Synthiam.SchLib fixture
fn json_roundtrip_synthiam_schlib() {
    // 1. Open SchLib with v2 API
    let lib = SchLib::open_file("../../Synthiam.SchLib").unwrap();

    // 2. Serialize to JSON
    let json = serde_json::to_string_pretty(&lib).unwrap();

    // 3. Deserialize from JSON
    let lib2: SchLib = serde_json::from_str(&json).unwrap();

    // 4. Compare field-by-field
    assert_eq!(lib.component_count(), lib2.component_count());
    for (g1, g2) in lib.groups.iter().zip(lib2.groups.iter()) {
        // Compare component records
        let c1 = SchComponentRecord::from_origin(&g1.component.origin);
        let c2 = SchComponentRecord::from_origin(&g2.component.origin);
        assert_eq!(c1.lib_reference().as_str(), c2.lib_reference().as_str());
        assert_eq!(c1.description().as_str(), c2.description().as_str());
        assert_eq!(c1.part_count(), c2.part_count());

        // Compare children count
        assert_eq!(g1.children.len(), g2.children.len());

        // Compare each child's record_id and key params
        for (ch1, ch2) in g1.children.iter().zip(g2.children.iter()) {
            assert_eq!(ch1.key, ch2.key);
            // Compare param values
        }
    }
}
```

#### PcbLib JSON Roundtrip

```rust
#[test]
#[ignore]
fn json_roundtrip_synthiam_pcblib() {
    let lib = PcbLib::open_file("../../Synthiam.PcbLib").unwrap();
    let json = serde_json::to_string_pretty(&lib).unwrap();
    let lib2: PcbLib = serde_json::from_str(&json).unwrap();

    assert_eq!(lib.footprints.len(), lib2.footprints.len());
    for (f1, f2) in lib.footprints.iter().zip(lib2.footprints.iter()) {
        assert_eq!(f1.primitives.len(), f2.primitives.len());
        // Compare primitive types and counts
        // Handle f64 angle comparisons with tolerance
    }
}
```

### Key Assertions (from existing tests)

The existing tests check:
- Component count matches
- `lib_ref`, `description`, `part_count` match for each component
- `aliases` list matches
- `record_id` and `record_id_ex` match for each record
- Params roundtrip correctly
- Pin fields are preserved
- f64 angles match within tolerance (1e-10)

### Acceptance Criteria

- [ ] `json_roundtrip_synthiam_schlib` passes with Synthiam.SchLib
- [ ] `json_roundtrip_synthiam_pcblib` passes with Synthiam.PcbLib
- [ ] Field-level assertions match existing test coverage
- [ ] `cargo test --test v2_schlib_roundtrip` passes
- [ ] `cargo test --test v2_pcblib_roundtrip` passes

---

## Track 8B: CFB Roundtrip Tests (CRITICAL)

**Files:**
- `crates/altium-format/tests/v2_schlib_cfb_roundtrip.rs` (rewrite)
- `crates/altium-format/tests/v2_pcblib_cfb_roundtrip.rs` (rewrite)
- `crates/altium-format/tests/v2_schdoc_cfb_roundtrip.rs` (rewrite)

**Reference:**
- Current CFB roundtrip test files

### What to Build

These tests prove that opening a file and saving it produces identical (or near-identical) output:

```rust
#[test]
#[ignore]
fn cfb_roundtrip_synthiam_schlib() {
    // 1. Read original file
    let original_bytes = std::fs::read("../../Synthiam.SchLib").unwrap();

    // 2. Open with v2 API
    let lib = SchLib::open(Cursor::new(&original_bytes)).unwrap();

    // 3. Save to buffer (no changes made)
    let mut output = Vec::new();
    lib.save(Cursor::new(&mut output)).unwrap();

    // 4. Re-open saved output
    let lib2 = SchLib::open(Cursor::new(&output)).unwrap();

    // 5. Structural comparison
    assert_eq!(lib.component_count(), lib2.component_count());
    for (g1, g2) in lib.groups.iter().zip(lib2.groups.iter()) {
        let c1 = SchComponentRecord::from_origin(&g1.component.origin);
        let c2 = SchComponentRecord::from_origin(&g2.component.origin);
        assert_eq!(c1.lib_reference().as_str(), c2.lib_reference().as_str());
        assert_eq!(c1.part_count(), c2.part_count());
        assert_eq!(g1.children.len(), g2.children.len());
    }
}
```

### Lossless Write Verification

For the identity test (open → save with no changes → byte-identical):

```rust
#[test]
#[ignore]
fn identity_save_synthiam_schlib() {
    let original_bytes = std::fs::read("../../Synthiam.SchLib").unwrap();
    let lib = SchLib::open(Cursor::new(&original_bytes)).unwrap();

    let mut output = Vec::new();
    lib.save(Cursor::new(&mut output)).unwrap();

    // Stream-level comparison (each CFB stream should be identical)
    // Note: CFB container metadata may differ (timestamps, etc.)
    // Compare at the stream level, not the file level
    // Use diff-ole.py for detailed analysis if this fails
}
```

### Acceptance Criteria

- [ ] CFB roundtrip tests pass for SchLib, PcbLib
- [ ] Structural comparison passes (component counts, record counts, field values)
- [ ] No data loss during roundtrip
- [ ] `cargo test --test v2_schlib_cfb_roundtrip` passes
- [ ] `cargo test --test v2_pcblib_cfb_roundtrip` passes

---

## Track 8C: Unit Tests & Record Roundtrips

**Files:**
- `crates/altium-format/tests/v2_schlib_typed.rs` (rewrite)
- Inline tests in `v2/records/*.rs` (already created in Phase 3)
- Inline tests in `v2/documents/*.rs` (already created in Phase 4)

**Reference:**
- Current `tests/v2_schlib_typed.rs`

### What to Build

1. **Typed record access tests** — verify that opening a SchLib and accessing typed fields works:
   ```rust
   #[test]
   #[ignore]
   fn typed_fields_accessible() {
       let mut lib = SchLib::open_file("../../Synthiam.SchLib").unwrap();
       lib.query::<SchComponent>("EZ-B v4/2")
           .unwrap()
           .with_mut(|comp| {
               assert!(!comp.lib_reference().as_str().is_empty());
               assert!(!comp.description().as_str().is_empty());
               assert!(comp.pin_count() > 0);

               comp.for_each_pin_mut(|pin| {
                   // Pin fields should be populated
                   assert!(!pin.designator().as_str().is_empty());
               });
           });
   }
   ```

2. **Coordinate system tests** — verify 100K units/mil:
   ```rust
   #[test]
   fn coord_from_mils_100k() {
       let c = SchCoord::from_mils(1.0);
       assert_eq!(c.to_raw(), 100_000);
   }
   ```

3. **Per-record param roundtrip tests** — for each record type, verify:
   - Parse from ParameterCollection → create record
   - Read field via getter → correct value
   - Write field via setter → value persists in backing store
   - Re-read → same value

4. **Binary serializer tests** — verify binary field read/write:
   - PCB track roundtrip
   - PCB common header roundtrip
   - PCB trailing fields roundtrip

### Acceptance Criteria

- [ ] Typed field access works for real SchLib fixtures
- [ ] Coordinate system uses 100K units/mil
- [ ] Per-record roundtrip tests pass for all record types
- [ ] `cargo test` passes (excluding ignored fixture-dependent tests)

---

## Track 8D: diff-ole.py Improvements

**Files:**
- `diff-ole.py` (or wherever the diff tool lives)

**Reference: `docs/v2-plan.md` (diff-ole.py Improvements section)**

### What to Build

This track is Python work, independent of the Rust code.

1. **Exit codes**:
   - `--assert-identical` → exit 0 if byte-identical, exit 1 if any difference
   - `--assert-semantic` → exit 0 if semantically identical, exit 1 if data differs

2. **Param-aware comparison**:
   - For text streams: compare both raw bytes AND order-normalized
   - Report which records differ only in order vs. have actual data changes

3. **Container-level comparison**:
   - Compare OLE metadata: sector sizes, directory entry ordering, timestamps
   - Report mini-stream cutoff size differences

4. **Both comparison modes**:
   - `--strict` — byte-for-byte comparison
   - `--semantic` — order-insensitive comparison

### Tests

- Test with Synthiam.SchLib: open → save → diff should show no semantic differences
- Test with intentionally modified file: diff should detect changes

### Acceptance Criteria

- [ ] `--assert-identical` exit codes work
- [ ] `--assert-semantic` exit codes work
- [ ] Param-aware comparison distinguishes order vs. data differences
- [ ] Container metadata comparison works
- [ ] Tool runs correctly on Synthiam fixtures
