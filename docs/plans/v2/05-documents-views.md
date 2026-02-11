# Phase 4: Documents, Views & IO

**Agents: 4 parallel tracks (4A, 4B, 4C, 4D)**
**Blocked by: Phase 3 (record types), Phase 5 (query language — for query integration)**

Tracks 4A-4C each implement a document type with its file I/O. Track 4D implements the view/wrapper types. 4D can start as soon as Phase 3 is done; 4A-4C can also start immediately but query integration needs Phase 5.

---

## Track 4A: SchLib Document Type

**Files:**
- `crates/altium-format/src/v2/documents/mod.rs`
- `crates/altium-format/src/v2/documents/schlib.rs`
- `crates/altium-format/src/v2/documents/section_keys.rs` (copy from `_v2_reference/io/section_keys.rs`)

**Reference:**
- `_v2_reference/io/schlib.rs` — Current SchLibV2 implementation
- `_v2_reference/io/section_keys.rs` — SectionKeyList implementation
- `io/reader.rs`, `io/writer.rs` — Low-level CFB functions

### What to Build

1. **`SectionKeyList`** — copy from reference, make self-contained:
   ```rust
   pub struct SectionKeyList {
       entries: Vec<SectionKeyEntry>,
   }
   pub struct SectionKeyEntry {
       pub lib_ref: String,
       pub section_key: String,
   }
   ```

2. **`SchLib` document type**:
   ```rust
   pub struct SchLib {
       pub groups: Vec<ComponentGroup>,
       pub section_keys: SectionKeyList,
       pub header: SchLibHeader,
   }

   pub struct SchLibHeader {
       pub header_text: String,
       pub weight: i32,
       pub minor_version: i32,
       pub unique_id: String,
       pub raw: Option<Vec<u8>>,
   }
   ```

3. **File I/O methods**:
   ```rust
   impl SchLib {
       pub fn open(reader: impl Read + Seek) -> Result<Self> { ... }
       pub fn open_file(path: impl AsRef<Path>) -> Result<Self> { ... }
       pub fn save(&self, writer: impl Write + Seek) -> Result<()> { ... }
       pub fn save_file(&self, path: impl AsRef<Path>) -> Result<()> { ... }
   }
   ```

   **Open logic** (from `_v2_reference/io/schlib.rs`):
   - Open CFB compound file
   - Read FileHeader stream → parse header, extract component list
   - Read SectionKeys stream
   - For each component: read its Data stream → parse records into `RecordNode`s
   - First record is the component record → `ComponentGroup.component`
   - Remaining records → `ComponentGroup.children`
   - Snapshot original bytes for each record

   **Save logic**:
   - For each ComponentGroup:
     - If no records are dirty, write original stream bytes verbatim
     - If any record is dirty, re-serialize all records in the group
   - Write SectionKeys
   - Write FileHeader

4. **Query methods** (integrate with Phase 5 query language):
   ```rust
   impl SchLib {
       pub fn query<T: WrapperFamily>(&mut self, q: &str) -> Result<QueryHandle<'_, T>> { ... }
       pub fn query_all<T: WrapperFamily>(&mut self, q: &str) -> Result<QueryResults<'_, T>> { ... }
       pub fn components(&self) -> &[ComponentGroup] { &self.groups }
       pub fn component_count(&self) -> usize { self.groups.len() }
   }
   ```

5. **QueryHandle and QueryResults**:
   ```rust
   pub struct QueryHandle<'a, T: WrapperFamily> {
       groups: &'a mut [ComponentGroup],
       index: usize,
       _marker: PhantomData<T>,
   }

   impl<'a> QueryHandle<'a, SchComponent> {
       pub fn with_mut<R>(self, f: impl FnOnce(SchComponentView<'_>) -> R) -> R { ... }
   }

   pub struct QueryResults<'a, T: WrapperFamily> {
       groups: &'a mut [ComponentGroup],
       indices: Vec<usize>,
       _marker: PhantomData<T>,
   }

   impl<'a> QueryResults<'a, SchComponent> {
       pub fn for_each_mut(self, f: impl FnMut(SchComponentView<'_>)) { ... }
       pub fn len(&self) -> usize { self.indices.len() }
       pub fn is_empty(&self) -> bool { self.indices.is_empty() }
   }
   ```

### Low-Level IO Helpers

Copy essential functions from `io/reader.rs` and `io/writer.rs` into the document module (or a shared `v2::io_helpers` module):

- `read_block()`, `write_block()` — framed data blocks
- `read_parameters()`, `write_parameters()` — parameter string I/O
- `compress_zlib()`, `decompress_zlib()` — compression
- `encode_windows_1252()` — encoding

These should be self-contained within v2, not importing from v1 modules.

### Serde Support

Add `Serialize`/`Deserialize` to `SchLib` for JSON roundtrip:
```rust
#[derive(Serialize, Deserialize)]
pub struct SchLib { ... }
```

The JSON roundtrip tests depend on this.

### Tests

- `schlib_open_synthiam()` — open Synthiam.SchLib, verify component count
- `schlib_cfb_roundtrip()` — open → save to buffer → re-open → verify identical
- `schlib_json_roundtrip()` — open → serialize JSON → deserialize → verify fields
- `schlib_query_component()` — query single component by designator
- `schlib_query_all()` — query multiple components by pattern

### Acceptance Criteria

- [ ] `SchLib::open()` parses CFB into ComponentGroup storage
- [ ] `SchLib::save()` writes lossless output (unchanged records write original bytes)
- [ ] `query()` and `query_all()` work with WrapperFamily type parameters
- [ ] JSON serialization/deserialization works
- [ ] CFB roundtrip test passes with Synthiam.SchLib
- [ ] `cargo check` passes

---

## Track 4B: SchDoc Document Type

**File: `crates/altium-format/src/v2/documents/schdoc.rs`**
**Reference: `_v2_reference/io/schdoc.rs`**

### What to Build

1. **`SchDoc` document type**:
   ```rust
   pub struct SchDoc {
       pub groups: Vec<ComponentGroup>,
       pub orphan_records: Vec<RecordNode>,
       pub header: SchDocHeader,
   }
   ```

2. **Open logic** — flat stream → OWNERINDEX grouping:
   ```rust
   impl SchDoc {
       pub fn open(reader: impl Read + Seek) -> Result<Self> {
           // Parse flat stream of records
           // Group by OWNERINDEX into ComponentGroups
           // Store original_indices for lossless save
       }
   }
   ```

3. **Save logic** — flatten groups back to original order:
   ```rust
   impl SchDoc {
       pub fn save(&self, writer: impl Write + Seek) -> Result<()> {
           // Flatten groups back using original_indices
           // Dirty records re-serialize; clean records write original bytes
       }
   }
   ```

4. **Query methods** — same API as SchLib.

### Tests

- `schdoc_open()` — open test SchDoc, verify record counts
- `schdoc_cfb_roundtrip()` — open → save → re-open → identical
- `schdoc_ownerindex_grouping()` — verify records group by OWNERINDEX correctly

### Acceptance Criteria

- [ ] `SchDoc::open()` parses flat stream into OWNERINDEX-grouped ComponentGroups
- [ ] `SchDoc::save()` flattens back to original order
- [ ] Same query API as SchLib
- [ ] `cargo check` passes

---

## Track 4C: PcbLib Document Type

**File: `crates/altium-format/src/v2/documents/pcblib.rs`**
**Reference: `_v2_reference/pcb/io/pcblib.rs`**

### What to Build

1. **`PcbLib` document type**:
   ```rust
   pub struct PcbLib {
       pub footprints: Vec<FootprintGroup>,
       pub section_keys: SectionKeyList,
       pub raw_streams: BTreeMap<String, Vec<u8>>,
   }
   ```

2. **Open logic** — parse each CFB storage:
   ```rust
   impl PcbLib {
       pub fn open(reader: impl Read + Seek) -> Result<Self> {
           // Read section keys
           // For each footprint storage:
           //   Read Parameters stream → PcbFootprintRecord (metadata)
           //   Read Header stream → u32 primitive count
           //   Read Data stream → pattern name block + binary primitives
           //   Parse binary primitives by type byte dispatch
           //   Build FootprintGroup
       }
   }
   ```

3. **Binary primitive dispatch** (from `_v2_reference/pcb/io/pcblib.rs`):
   ```rust
   fn parse_primitive(type_byte: u8, data: &[u8]) -> Result<RecordNode> {
       match type_byte {
           1 => parse_arc(data),
           2 => parse_pad(data),
           3 => parse_via(data),
           4 => parse_track(data),
           5 => parse_text(data),
           6 => parse_fill(data),
           11 => parse_region(data),
           12 => parse_component_body(data),
           _ => Ok(RecordNode::unknown(type_byte, data.to_vec())),
       }
   }
   ```

4. **Save logic**, **Query methods** — same pattern as SchLib/SchDoc.

### Tests

- `pcblib_open_synthiam()` — open Synthiam.PcbLib, verify footprint count
- `pcblib_cfb_roundtrip()` — open → save → re-open → identical
- `pcblib_primitive_counts()` — verify track/arc/pad/etc. counts per footprint

### Acceptance Criteria

- [ ] `PcbLib::open()` parses CFB into FootprintGroup storage
- [ ] Binary primitive dispatch handles all known type bytes
- [ ] Unknown type bytes stored as raw bytes for lossless roundtrip
- [ ] `PcbLib::save()` writes lossless output
- [ ] `cargo check` passes

---

## Track 4D: View Types & Wrappers

**Files:**
- `crates/altium-format/src/v2/views/mod.rs`
- `crates/altium-format/src/v2/views/leaf_wrappers.rs`
- `crates/altium-format/src/v2/views/sch_component_view.rs`
- `crates/altium-format/src/v2/views/pcb_footprint_view.rs`
- `crates/altium-format/src/v2/views/child_handle.rs`

**Reference: `docs/v2-plan.md` (Hierarchical Wrapper Types section)**

### What to Build

1. **`impl_leaf_wrapper!` macro**:
   ```rust
   macro_rules! impl_leaf_wrapper {
       ($view:ident<$lt:lifetime> wraps $record:ty) => {
           pub struct $view<$lt> {
               record: &$lt mut $record,
           }

           impl<$lt> std::ops::Deref for $view<$lt> {
               type Target = $record;
               fn deref(&self) -> &$record { self.record }
           }

           impl<$lt> std::ops::DerefMut for $view<$lt> {
               fn deref_mut(&mut self) -> &mut $record {
                   // DerefMut marks dirty
                   self.record
               }
           }
       };
   }
   ```

2. **Leaf wrappers** (one per record type):
   ```rust
   impl_leaf_wrapper!(SchPinView<'a> wraps SchPinRecord);
   impl_leaf_wrapper!(SchArcView<'a> wraps SchArcRecord);
   impl_leaf_wrapper!(SchLineView<'a> wraps SchLineRecord);
   impl_leaf_wrapper!(SchRectangleView<'a> wraps SchRectangleRecord);
   // ... all leaf record types
   impl_leaf_wrapper!(PcbPadView<'a> wraps PcbPadRecord);
   impl_leaf_wrapper!(PcbTrackView<'a> wraps PcbTrackRecord);
   // ... all PCB leaf types
   ```

3. **WrapperFamily marker types**:
   ```rust
   pub enum SchPin {}
   impl WrapperFamily for SchPin {
       type Record = SchPinRecord;
       type View<'a> = SchPinView<'a>;
   }

   pub enum SchComponent {}
   impl WrapperFamily for SchComponent {
       type Record = SchComponentRecord;
       type View<'a> = SchComponentView<'a>;
   }

   // ... for all record types
   ```

4. **`SchComponentView`** (parent wrapper — hand-written):
   ```rust
   pub struct SchComponentView<'a> {
       component: &'a mut SchComponentRecord,
       children: &'a mut [RecordNode],
   }

   impl<'a> Deref for SchComponentView<'a> {
       type Target = SchComponentRecord;
       fn deref(&self) -> &SchComponentRecord { self.component }
   }

   impl<'a> DerefMut for SchComponentView<'a> {
       fn deref_mut(&mut self) -> &mut SchComponentRecord {
           self.component.mark_dirty();
           self.component
       }
   }

   impl<'a> SchComponentView<'a> {
       pub fn for_each_pin_mut(&mut self, f: impl FnMut(SchPinView<'_>)) { ... }
       pub fn query<T: WrapperFamily>(&mut self, q: &str) -> Result<ChildHandle<'_, T>> { ... }
       pub fn query_all<T: WrapperFamily>(&mut self, q: &str) -> Result<ChildResults<'_, T>> { ... }
       pub fn with_child_mut<T: WrapperFamily, R>(
           &mut self, key: ChildKey<T>, f: impl FnOnce(T::View<'_>) -> R
       ) -> R { ... }
       pub fn child_keys<T: WrapperFamily>(&self) -> impl Iterator<Item = ChildKey<T>> { ... }
       pub fn split(&mut self) -> (&mut SchComponentRecord, ChildrenMut<'_>) { ... }
       pub fn pin_count(&self) -> usize { ... }
   }
   ```

5. **ChildHandle, ChildResults, ChildKey** (from v2-plan.md):
   ```rust
   pub struct ChildHandle<'a, T: WrapperFamily> {
       children: &'a mut [RecordNode],
       index: usize,
       _marker: PhantomData<T>,
   }

   impl<'a, T: WrapperFamily> ChildHandle<'a, T> {
       pub fn with_mut<R>(self, f: impl FnOnce(T::View<'_>) -> R) -> R { ... }
   }

   pub struct ChildResults<'a, T: WrapperFamily> {
       children: &'a mut [RecordNode],
       indices: Vec<usize>,
       _marker: PhantomData<T>,
   }

   impl<'a, T: WrapperFamily> ChildResults<'a, T> {
       pub fn for_each_mut(self, f: impl FnMut(T::View<'_>)) { ... }
       pub fn len(&self) -> usize { self.indices.len() }
   }

   pub struct ChildKey<T: WrapperFamily> {
       index: usize,
       _marker: PhantomData<T>,
   }
   ```

6. **`PcbFootprintView`** (parent wrapper for PCB):
   ```rust
   pub struct PcbFootprintView<'a> {
       metadata: &'a mut PcbFootprintRecord,
       primitives: &'a mut [RecordNode],
   }
   // Same pattern as SchComponentView
   ```

### Tests

- `leaf_wrapper_deref()` — getter works through Deref
- `leaf_wrapper_deref_mut()` — setter works through DerefMut
- `component_view_child_access()` — iterate pins
- `component_view_split_borrow()` — split() allows simultaneous parent + child access
- `child_handle_with_mut()` — query → with_mut chain

### Acceptance Criteria

- [ ] `impl_leaf_wrapper!` generates all leaf view types
- [ ] WrapperFamily markers exist for all record types
- [ ] `SchComponentView` has child navigation (for_each_pin_mut, query, split)
- [ ] `PcbFootprintView` has child navigation
- [ ] ChildHandle/ChildResults/ChildKey all work
- [ ] `cargo check` passes
