# altium-format v2: Clean-Slate Architecture

## Intent

Design from scratch for one goal: **lossless, nondestructive editing with an ergonomic imperative API**.

This document intentionally ignores migration constraints. Existing code is treated as a knowledge base and oracle; the new architecture is allowed to break old APIs.

**Goals**

1. **Origin-backed records**: Records store their raw representation. This gives non-destructive editing and lossless roundtrip.
2. **In-place mutation**: Setters patch the backing store directly. No separate patch planning, no PatchOps. The backing store IS the data.
3. **No defaults in core types**: Defaults live in template functions and DTO layers only.
4. **I/O in Document types, not record types**: Records are pure data. Documents handle CFB/OLE.

The result is a **standard imperative library** with a well-designed internal data model.

## Non-Negotiables

1. **Lossless by default**: if nothing is edited, output bytes are byte-identical for every untouched stream.
2. **Edit in place**: setters patch the backing store directly. The backing store is the same type whether it came from an Altium file or a template — there is no separate "create" vs "edit" mechanism.
3. **No core defaults**: contextual defaults live in DTO/interface layers only (CLI/JSON/UI), never in core record types. Template functions provide Altium-correct defaults for new records.
4. **No runtime fields**: macro-declared fields exist for documentation and autocomplete; runtime access is through generated getters/setters over the backing store.
5. **Macro-first generation**: the macro generates `*Record` types with getters/setters and builder APIs. Hierarchical wrapper types are hand-written (Deref to record types for getters/setters, plus dirty tracking, child navigation, and validation). Param types handle their own serialization via `ParamCodec` (single key). Binary records use hand-written parsers with helper functions.
6. **Tests are behavioral**: assertions must prove functional behavior (byte identity, patch locality, invariant enforcement), not just counts.

## Core Data Model

### Backing Store Architecture

Records do not have typed fields at runtime. They have a **backing store** — either a `ParameterCollection` (for param-based records) or a binary blob (for binary records). All reads and writes go through the backing store via generated getters/setters.

Setters write directly into the backing store. There is no separate "patch plan" or PatchOp type. The backing store IS the mutable state, whether it was loaded from an existing Altium file or created from a template function. The same getters/setters work on both.

Caching can be added later as an optimization once the API is stable and all tests pass.

```rust
pub struct DocumentCore {
    streams: Vec<StreamNode>,
}

pub struct StreamNode {
    id: StreamId,
    original_bytes: Vec<u8>,       // snapshot for dirty detection on save
    records: Vec<RecordNode>,
}

pub struct RecordNode {
    key: RecordKey,
    origin: RecordOrigin,
    original_snapshot: Vec<u8>,    // for byte-level dirty detection on save
}
```

```rust
pub enum RecordOrigin {
    Param(ParamOrigin),
    Binary(BinaryOrigin),
}

pub struct ParamOrigin {
    params: ParameterCollection,   // THE authoritative data — reads and writes go here
    raw_record_text: String,       // original serialized text (written verbatim if unchanged)
}

pub struct BinaryOrigin {
    raw_block: Vec<u8>,            // THE authoritative data — setters patch bytes in place
    field_spans: Vec<FieldSpan>,   // decoded span map so setters know where to write
}
```

`UnknownFields` is removed entirely. Unknown data is preserved automatically because we never extract fields out of the backing store — the backing store IS the record. Known fields are accessed through typed getters/setters; unknown fields remain untouched in the backing store.

### ComponentGroup Storage

To enable hierarchical access (component → children) without borrow checker conflicts, the document stores records in **component groups** that separate the parent record from its children as distinct struct fields:

```rust
pub struct ComponentGroup {
    component: RecordNode,           // the component record itself
    children: Vec<RecordNode>,       // pins, lines, arcs, parameters, etc.
    original_indices: Vec<usize>,    // position in original stream (for lossless save)
}
```

Because `component` and `children` are separate fields, Rust allows borrowing them independently — this is the key enabler for the hierarchical wrapper API.

**SchLib**: Each component is a separate CFB stream. One `ComponentGroup` per stream, built naturally during parsing.

**SchDoc**: All records are in one flat stream with OWNERINDEX links. On load, records are grouped by OWNERINDEX into `ComponentGroup`s. On save, the groups are flattened back to the original order using stored indices to preserve byte-identical output for untouched records.

This replaces the generic `DocumentCore` → `StreamNode` → `Vec<RecordNode>` structure for schematic formats. The `ComponentGroup` abstraction unifies SchLib (tree) and SchDoc (flat + OWNERINDEX) behind the same query and closure API. See [Document Type Structure](#document-type-structure) for the full struct definitions.

### Write Rules

On save, for each record:

- **Unchanged**: backing store bytes match the original snapshot → write the original bytes verbatim. Byte-identical output.
- **Changed**: backing store was mutated by setters → serialize the backing store to bytes.

This applies identically regardless of whether the backing store was loaded from an Altium file or created from a template. There is no separate "rebuild" or "re-serialize" path. Setters always update in place.

### Document Type Structure

Each document type is a separate struct. Schematic formats use `ComponentGroup` for hierarchical access; PCB formats may use a different grouping strategy suited to their typed-stream layout.

```rust
pub struct SchLib {
    groups: Vec<ComponentGroup>,         // one per CFB stream
    section_keys: SectionKeyList,
    header: SchLibHeader,
}

pub struct SchDoc {
    groups: Vec<ComponentGroup>,         // grouped by OWNERINDEX on load
    orphan_records: Vec<RecordNode>,     // records not owned by any component
}

pub struct PcbLib {
    core: DocumentCore,
    // PcbLib-specific metadata — per-footprint sections
}

pub struct PcbDoc {
    core: DocumentCore,
    // PcbDoc-specific metadata — typed stream map (Tracks6, Arcs6, etc.)
}
```

Separate types per format because they are semantically different:

- **SchLib**: components containing primitives (tree structure) → `ComponentGroup` per stream
- **SchDoc**: flat primitives with OWNERINDEX links → `ComponentGroup` per owner, built on load
- **PcbLib**: components containing binary records with multi-block structures
- **PcbDoc**: flat binary records in typed streams (Tracks6, Arcs6, etc.)

`DocumentCore` remains as a lower-level building block for PCB formats and for generic operations (iteration, save, test fixtures). Schematic formats use `ComponentGroup` directly for the query and closure API.

## API Shape: Imperative Access with Backing Store

### Three Layers: Records, Hierarchical Wrappers, and Query

There are three distinct layers:

1. **Record types** (`SchPinRecord`, `SchComponentRecord`, etc.) — own a `RecordOrigin` and have typed getters/setters directly on them. Records are **pure data** — they have no knowledge of parent/child/sibling relationships. No lifetimes, no document awareness. The `Record` suffix distinguishes this layer from the wrapper layer. The macro generates these types. **Users never interact with `*Record` types directly.**

2. **Hierarchical wrapper types** (`SchComponent`, `SchPin`, etc.) — hand-written wrappers that borrow into the document's `ComponentGroup` storage. `Deref` to their underlying record type so the user gets all getters/setters for free. Most wrappers are trivial — just a Deref to the record type plus dirty tracking. Only parent types like `SchComponent` add child navigation methods (query, iterate, with_child_mut). Wrappers are **hand-written, not macro-generated**, because this is where API ergonomics lives. **These are the public API types — the only types users see.**

3. **Query API** (`doc.query::<SchComponent>(q)`, `doc.query_all::<SchComponent>(q)`) — returns handles that open closures over matched records. Same query language works at both the document level and within hierarchical wrappers for child access. `query` errors on 0 or 2+ matches; `query_all` returns all matches. Type parameters are wrapper family markers, not record types.

The user interacts with wrapper types everywhere. Record types are internal — the wrapper's Deref makes getters/setters available transparently. Most wrapper types are a single Deref impl plus a Drop for dirty tracking.

### Macro Declaration (Source of Truth)

Fields exist in the source code for documentation and IDE autocomplete. The macro reads them but **removes them from the runtime struct**, replacing the struct with a backing-store wrapper. The macro generates typed getters/setters directly on the record type.

Field types should be **domain newtypes**, not raw primitives. A `Designator` is not a `String` — it has structure (prefix + number), helper methods, and validation. Newtypes let the macro generate `update_*` closures that give `&mut` access to the parsed value for in-place modification, avoiding a getter-modify-setter round trip.

The macro passes param key names to the type's `ParamCodec` trait implementation. **Types handle their own serialization** — the macro just orchestrates. This means `SchCoord` knows how to read/write its integer+frac pair, `Designator` knows how to read/write its string, etc. The macro doesn't need special-case logic for `frac`, `bitflags`, or any other encoding detail.

```rust
#[derive(AltiumEntity)]
#[altium(kind = "sch", record_id = 2, codec = "params")]
struct SchPinRecord {
    #[altium(key = "DESIGNATOR")]
    designator: Designator,

    #[altium(key = "PINLENGTH")]
    pin_length: SchCoord,       // SchCoord internally derives PINLENGTH_FRAC

    #[altium(key = "ELECTRICAL")]
    electrical: PinElectricalType,

    #[altium(key = "PINCONGLOMERATE")]
    pin_conglomerate: PinConglomerateFlags,
}
```

### Domain Newtypes

String-backed fields get newtypes with domain-specific helpers:

```rust
/// Concrete designator in a schematic document (e.g., "R1", "U3", "C12").
/// Always has a prefix and a number.
pub struct Designator(String);

impl Designator {
    pub fn new(s: impl Into<String>) -> Self { ... }
    pub fn as_str(&self) -> &str { &self.0 }
    pub fn prefix(&self) -> &str { ... }           // "U2" → "U"
    pub fn number(&self) -> u32 { ... }            // "U2" → 2
    pub fn set_number(&mut self, n: u32) { ... }   // "U2" → "U5"
    pub fn increment(&mut self) { ... }            // "U2" → "U3"
}

/// Template designator in a schematic library (e.g., "U?", "R?", "C?").
/// The `?` placeholder gets replaced with an incrementing number on placement into a SchDoc.
pub struct DesignatorTemplate(String);

impl DesignatorTemplate {
    pub fn new(s: impl Into<String>) -> Self { ... }
    pub fn as_str(&self) -> &str { &self.0 }
    pub fn prefix(&self) -> &str { ... }           // "U?" → "U"
    pub fn resolve(&self, n: u32) -> Designator { ... }  // "U?" + 3 → Designator("U3")
}
```

Both types implement `ParamCodec` identically (read/write a string to the `DESIGNATOR` key), but they parse the string differently and offer different methods. SchLib `SchComponent` uses `DesignatorTemplate`; SchDoc `SchComponent` uses `Designator`.

```rust
/// Library reference name.
pub struct LibReference(String);

/// Net name.
pub struct NetName(String);

/// Altium unique ID.
pub struct UniqueId(String);
```

Newtypes implement `Deref<Target=str>` for transparent read access, and `From<&str>` / `From<String>` for ergonomic construction.

### Serialization Traits (Inverted Control)

Types handle their own param serialization. The macro calls trait methods, passing a **single key** from the attribute. The type knows its own key pattern and derives any related keys (e.g., `{key}_FRAC`) internally. This eliminates special-case macro logic for `frac`, `bitflags`, composite values, etc.

```rust
/// Trait for types that can read/write themselves from/to parameter collections.
/// The `key` argument is the base param key name. Types that need additional
/// related keys (e.g., SchCoord needs {key}_FRAC) derive them internally.
pub trait ParamCodec: Sized {
    fn read(params: &ParameterCollection, key: &str) -> Option<Self>;
    fn write(&self, params: &mut ParameterCollection, key: &str);
}
```

Implementations for each type family:

```rust
// String newtypes — single key, straightforward
impl ParamCodec for Designator {
    fn read(params: &ParameterCollection, key: &str) -> Option<Self> {
        params.get(key).map(|v| Designator::new(v.as_str()))
    }
    fn write(&self, params: &mut ParameterCollection, key: &str) {
        params.set(key, self.as_str());
    }
}

// Coordinates — type internally derives the _FRAC key from the base key
impl ParamCodec for SchCoord {
    fn read(params: &ParameterCollection, key: &str) -> Option<Self> {
        let int_val = params.get(key)?.as_int_or(0);
        let frac_key = format!("{}_FRAC", key);
        let frac_val = params.get(&frac_key)
            .map(|v| v.as_int_or(0))
            .unwrap_or(0);
        Some(SchCoord::from_dxp_parts(int_val, frac_val))
    }
    fn write(&self, params: &mut ParameterCollection, key: &str) {
        let (int_val, frac_val) = self.to_dxp_parts();
        params.set_int(key, int_val);
        params.set_int(&format!("{}_FRAC", key), frac_val);
    }
}

// Enums — single integer key
impl ParamCodec for PinElectricalType {
    fn read(params: &ParameterCollection, key: &str) -> Option<Self> {
        params.get(key).map(|v| PinElectricalType::from_int(v.as_int_or(0)))
    }
    fn write(&self, params: &mut ParameterCollection, key: &str) {
        params.set_int(key, self.to_int());
    }
}

// Bitflags — single integer key, uses .bits() / from_bits_truncate()
impl ParamCodec for PinConglomerateFlags {
    fn read(params: &ParameterCollection, key: &str) -> Option<Self> {
        params.get(key).map(|v| PinConglomerateFlags::from_bits_truncate(v.as_int_or(0) as u32))
    }
    fn write(&self, params: &mut ParameterCollection, key: &str) {
        params.set_int(key, self.bits() as i32);
    }
}
```

Composite param types (like `MaskExpansion`) that need multiple unrelated keys implement `ParamCodec` by hand or use `codec_fn`.

### Binary Record Serialization (Hand-Written with Helpers)

Binary records (PCB types like `PcbPad`) have complex multi-block structures that don't benefit from a trait abstraction. The real complexity is in the **record-level block structure** (PcbPad has 6 variable-length blocks, each with different fields at different offsets), not in individual field encoding (which is just `i32::from_le_bytes` and similar).

Binary records get **hand-written parse/serialize methods** with common helper functions:

```rust
// Helper functions for common binary patterns
pub mod binary_helpers {
    pub fn read_i32(data: &[u8], offset: usize) -> i32 { ... }
    pub fn write_i32(data: &mut [u8], offset: usize, value: i32) { ... }
    pub fn read_coord(data: &[u8], offset: usize) -> PcbCoord { ... }
    pub fn write_coord(data: &mut [u8], offset: usize, value: PcbCoord) { ... }
    pub fn read_pascal_string(data: &[u8], offset: usize) -> (&str, usize) { ... }
    pub fn write_pascal_string(data: &mut Vec<u8>, offset: usize, s: &str) -> usize { ... }
    pub fn read_layer_array<C: AltiumCoord>(data: &[u8], offset: usize, count: usize) -> Vec<Point<C>> { ... }
    // etc.
}
```

Each binary record type has its own parse/serialize that uses these helpers and understands its block structure. The macro generates getters/setters that index into the field span map, but the span map itself is built by the hand-written parser.

### Escape Hatch: `codec_fn`

For types that don't fit the standard `ParamCodec` trait (e.g., composite types that need multiple unrelated param keys, or unusual encoding patterns), the macro supports overriding with a custom function:

```rust
#[altium(codec_fn = "mask_expansion_codec")]
paste_mask: MaskExpansion,
```

The custom function receives the full `ParameterCollection` and can access whatever keys it needs. This should be rare — the standard `ParamCodec` single-key approach covers ~95% of fields.

### Generated Record Type (Three Access Patterns)

The macro generates three access patterns per field on the record type:

1. **`field()`** — read-only, returns the newtype by value
2. **`set_field(value)`** — full replacement, writes to backing store
3. **`update_field(|f| { ... })`** — closure with `&mut` to the parsed newtype, flushed back on return

```rust
// Generated by macro:
pub struct SchPinRecord {
    origin: RecordOrigin,
}

impl SchPinRecord {
    // 1. Read: calls Designator::from_params() on the backing store
    pub fn designator(&self) -> Designator { ... }

    // 2. Replace: calls value.write_params() on the backing store
    pub fn set_designator(&mut self, value: impl Into<Designator>) { ... }

    // 3. Update in place: parses, gives &mut to closure, flushes back
    pub fn update_designator<R>(&mut self, f: impl FnOnce(&mut Designator) -> R) -> R {
        let mut value = self.designator();  // parse from backing store
        let result = f(&mut value);
        self.set_designator(value);          // flush back
        result
    }

    // Same three patterns for all fields:
    pub fn pin_length(&self) -> SchCoord { ... }
    pub fn set_pin_length(&mut self, value: SchCoord) { ... }
    pub fn update_pin_length<R>(&mut self, f: impl FnOnce(&mut SchCoord) -> R) -> R { ... }

    pub fn electrical(&self) -> PinElectricalType { ... }
    pub fn try_electrical(&self) -> Option<PinElectricalType> { ... }
    pub fn set_electrical(&mut self, value: PinElectricalType) { ... }

    pub fn pin_conglomerate(&self) -> PinConglomerateFlags { ... }
    pub fn set_pin_conglomerate(&mut self, value: PinConglomerateFlags) { ... }
    pub fn update_pin_conglomerate<R>(&mut self, f: impl FnOnce(&mut PinConglomerateFlags) -> R) -> R { ... }
}
```

### Hierarchical Wrapper Types (Hand-Written)

Wrapper types are **hand-written, not macro-generated**. This is where API ergonomics lives — each wrapper is crafted for its specific use case.

Most wrapper types are trivial: they Deref to their underlying record type, add dirty tracking on Drop, and nothing else. Only **parent types** (like `SchComponent`) add child navigation.

#### Leaf Wrappers (Most Types)

Leaf wrappers like `SchPin`, `SchArc`, `SchLine`, etc. are minimal — just a Deref to the record type plus dirty tracking. They have no children and no special methods:

```rust
/// Hand-written. Wraps SchPinRecord with Deref + dirty tracking.
/// No child navigation — pins are leaf records.
pub struct SchPin<'a> {
    record: &'a mut SchPinRecord,
    dirty: bool,
    snapshot: Vec<u8>,
}

impl<'a> Deref for SchPin<'a> { type Target = SchPinRecord; }
impl<'a> DerefMut for SchPin<'a> { ... }

impl<'a> Drop for SchPin<'a> {
    fn drop(&mut self) {
        if self.dirty { self.record.mark_dirty(); }
        if std::thread::panicking() { self.record.restore_from(&self.snapshot); }
    }
}
```

These are boilerplate-heavy but intentionally hand-written. A helper macro (`impl_leaf_wrapper!`) can reduce repetition without going full code generation — this keeps the wrapper layer explicit and easy to customize per type:

```rust
impl_leaf_wrapper!(SchPin<'a> wraps SchPinRecord);
impl_leaf_wrapper!(SchArc<'a> wraps SchArcRecord);
impl_leaf_wrapper!(SchLine<'a> wraps SchLineRecord);
impl_leaf_wrapper!(SchRectangle<'a> wraps SchRectangleRecord);
// ... etc
```

#### Parent Wrappers (SchComponent)

Parent types add child navigation. `SchComponent` is the primary example — it borrows both the component record and its children from separate `ComponentGroup` fields, which is what enables the split-borrow pattern:

```rust
/// Hand-written. Wraps SchComponentRecord + children with child navigation.
pub struct SchComponent<'a> {
    component: &'a mut SchComponentRecord, // borrows ComponentGroup.component
    children: &'a mut [RecordNode],        // borrows ComponentGroup.children
    dirty: bool,
    snapshot: Vec<u8>,
}

impl<'a> Deref for SchComponent<'a> { type Target = SchComponentRecord; }
impl<'a> DerefMut for SchComponent<'a> { ... }

impl<'a> Drop for SchComponent<'a> {
    fn drop(&mut self) {
        if self.dirty { self.component.mark_dirty(); }
        if std::thread::panicking() { self.component.restore_from(&self.snapshot); }
    }
}
```

**Why Deref works here**: Borrows through Deref are temporary — released after each statement. This means getter/setter calls on the component don't conflict with subsequent child access:

```rust
comp.set_description("New description");   // temp borrow via DerefMut, released
comp.for_each_pin_mut(|pin| { ... });      // no conflict — nothing borrows comp
```

The only case that won't compile is holding a reference across child access:

```rust
let lib_ref: &str = comp.lib_reference();  // holds borrow via Deref
comp.for_each_pin_mut(|pin| { ... });      // CONFLICT — comp still borrowed
```

Fix by cloning (common), or use `split()` for zero-copy (rare):

```rust
// Fix 1: clone to release borrow
let lib_ref = comp.lib_reference().to_string();
comp.for_each_pin_mut(|pin| { ... });

// Fix 2: split() for simultaneous parent + child access
let (data, children) = comp.split();
let lib_ref = data.lib_reference();        // borrows data only
children.for_each_pin_mut(|pin| {          // borrows children only — no conflict
    pin.set_name(PinName::new(format!("{}_{}", lib_ref, pin.designator())));
});
```

**Features only parent wrappers have:**

| Feature | What |
|---|---|
| **Child navigation** | Query and iterate child records (pins, lines, etc.) |
| **`split()`** | Returns independent refs to parent record and children view |
| **Child query** | Same AQL scoped to this component's children |

**Features all wrappers have (leaf and parent):**

| Feature | What |
|---|---|
| **Deref to record type** | All getters/setters from the record type, no re-declaration |
| **Dirty tracking** | Marks the record dirty on drop if any setter was called |
| **Drop validation** | Validates record invariants when the closure returns |
| **Panic rollback** | Restores original bytes if the closure panics |

#### Child Access Methods

```rust
impl<'a> SchComponent::Wrapper<'a> {
    /// Iterate all pins with mutable access.
    /// Closure receives SchPin wrapper, not SchPinRecord.
    pub fn for_each_pin_mut(&mut self, f: impl FnMut(SchPin::Wrapper<'_>)) { ... }
    pub fn for_each_pin(&self, f: impl FnMut(SchPin::RefWrapper<'_>)) { ... }

    /// Query children — same AQL syntax, scoped to this component.
    /// T is a WrapperFamily marker (SchPin, SchLine, etc.)
    /// Errors on 0 or 2+ matches.
    pub fn query<T: WrapperFamily>(&mut self, q: &str) -> Result<ChildHandle<'_, T>> { ... }

    /// Query children — returns all matches.
    pub fn query_all<T: WrapperFamily>(&mut self, q: &str) -> Result<ChildResults<'_, T>> { ... }

    /// Access a specific child by key (for when handles are passed around).
    /// Closure receives wrapper type, not record type.
    pub fn with_child_mut<T: WrapperFamily, R>(
        &mut self, key: ChildKey<T>, f: impl FnOnce(T::Wrapper<'_>) -> R,
    ) -> R { ... }
    pub fn with_child_ref<T: WrapperFamily, R>(
        &self, key: ChildKey<T>, f: impl FnOnce(T::RefWrapper<'_>) -> R,
    ) -> R { ... }

    /// Get child keys for external use (passing to other functions, collecting).
    pub fn pin_keys(&self) -> impl Iterator<Item = ChildKey<SchPin>> { ... }

    /// Split borrows escape hatch — returns independent refs to parent record and children.
    pub fn split(&mut self) -> (&mut SchComponentRecord, ChildrenView<'_>) { ... }

    pub fn pin_count(&self) -> usize { ... }
}
```

#### Child Handles (from query)

`ChildHandle` and `ChildResults` are returned by query methods on the wrapper. They hold `&mut` into the children storage and provide `with_mut`/`with_ref`/`for_each_mut` directly — same pattern as the document-level `QueryHandle`. Closures receive **wrapper types**, not record types.

```rust
/// Single child match — from comp.query::<SchPin>("pin[name=VCC]")?
/// T is a WrapperFamily marker (SchPin, SchLine, etc.)
pub struct ChildHandle<'a, T: WrapperFamily> {
    children: &'a mut [RecordNode],
    index: usize,
    _marker: PhantomData<T>,
}

impl<'a, T: WrapperFamily> ChildHandle<'a, T> {
    pub fn with_mut<R>(self, f: impl FnOnce(T::Wrapper<'_>) -> R) -> R { ... }
    pub fn with_ref<R>(self, f: impl FnOnce(T::RefWrapper<'_>) -> R) -> R { ... }
}

/// Multiple child matches — from comp.query_all::<SchPin>("pin:power")?
pub struct ChildResults<'a, T: WrapperFamily> {
    children: &'a mut [RecordNode],
    indices: Vec<usize>,
    _marker: PhantomData<T>,
}

impl<'a, T: WrapperFamily> ChildResults<'a, T> {
    pub fn for_each_mut(self, f: impl FnMut(T::Wrapper<'_>)) { ... }
    pub fn for_each_ref(self, f: impl FnMut(T::RefWrapper<'_>)) { ... }
    pub fn len(&self) -> usize { self.indices.len() }
    pub fn is_empty(&self) -> bool { self.indices.is_empty() }
}
```

#### Child Keys (for passing around)

`ChildKey<T>` is a lightweight typed index with no borrow — safe to collect, store, pass to other functions, and use later with `with_child_mut`. The closure still receives a wrapper type:

```rust
pub struct ChildKey<T: WrapperFamily> {
    index: usize,
    _marker: PhantomData<T>,
}
```

Use `ChildHandle.with_mut()` for inline chains (ergonomic). Use `ChildKey` + `with_child_mut` when handles need to be collected or passed around (flexible).

### Document-Level Query API

The query API is the primary entry point for finding and mutating records in a document.

```rust
impl SchLib {
    /// Find exactly one matching component. Errors on 0 or 2+ matches.
    pub fn query<T: RecordType>(&mut self, q: &str) -> Result<QueryHandle<'_, T>> { ... }

    /// Find all matching components.
    pub fn query_all<T: RecordType>(&mut self, q: &str) -> Result<QueryResults<'_, T>> { ... }
}
```

Both take `&mut self` because the returned handles need mutable access for `with_mut`. The query itself only reads, but the handle must be able to open a mutable closure.

```rust
/// Single match — from doc.query::<SchComponent>("U1")?
/// T is a WrapperFamily marker type (SchComponent, SchPin, etc.)
pub struct QueryHandle<'a, T: WrapperFamily> {
    doc: &'a mut SchLib,   // (or SchDoc, PcbLib, etc.)
    index: usize,
    _marker: PhantomData<T>,
}

impl<'a> QueryHandle<'a, SchComponent> {
    /// Open a mutable closure over the matched component.
    pub fn with_mut<R>(self, f: impl FnOnce(SchComponent::Wrapper<'_>) -> Result<R>) -> Result<R> {
        let group = &mut self.doc.groups[self.index];
        // split borrow: component vs children — separate ComponentGroup fields
        let (comp_node, children) = (&mut group.component, &mut group.children[..]);
        let record = SchComponentRecord::from_origin_mut(&mut comp_node.origin);
        f(/* construct SchComponent wrapper with record + children */)
    }

    /// Read-only access.
    pub fn with_ref<R>(self, f: impl FnOnce(SchComponent::RefWrapper<'_>) -> R) -> R { ... }
}

/// Multiple matches — from doc.query_all::<SchComponent>("R*")?
pub struct QueryResults<'a, T: WrapperFamily> {
    doc: &'a mut SchLib,
    indices: Vec<usize>,
    _marker: PhantomData<T>,
}

impl<'a> QueryResults<'a, SchComponent> {
    pub fn for_each_mut(self, f: impl FnMut(SchComponent::Wrapper<'_>) -> Result<()>) -> Result<()> { ... }
    pub fn for_each_ref(self, f: impl FnMut(SchComponent::RefWrapper<'_>)) { ... }
    pub fn len(&self) -> usize { self.indices.len() }
    pub fn is_empty(&self) -> bool { self.indices.is_empty() }
}
```

### Document Access: Full Example

The user-facing API uses **wrapper type names only** (`SchComponent`, `SchPin`, etc.). Users never write `*Record` types — those are internal.

```rust
let mut doc = SchLib::open("Library.SchLib")?;

// Single component — exact match (errors on 0 or 2+)
doc.query::<SchComponent>("U1")?.with_mut(|comp| {
    // comp: SchComponent<'_> — wrapper, Deref to SchComponentRecord
    // Getters/setters available directly via Deref
    comp.set_lib_reference("LM358N");
    comp.set_description("Dual Op-Amp");

    // Query single child — with_mut directly on the handle
    comp.query::<SchPin>("pin[name=VCC]")?.with_mut(|pin| {
        // pin: SchPin<'_> — wrapper, Deref to SchPinRecord
        pin.set_electrical(PinElectricalType::Power);
    });

    // Query multiple children — for_each_mut on the results
    comp.query_all::<SchPin>("pin:power")?.for_each_mut(|pin| {
        pin.set_electrical(PinElectricalType::Power);
    });

    // Iterate all pins
    comp.for_each_pin_mut(|pin| {
        pin.update_designator(|d| d.increment());
    });

    // Collect keys for later use (keys are lightweight indices, no borrow)
    let pin_keys: Vec<_> = comp.pin_keys().collect();
    for key in pin_keys {
        comp.with_child_mut(key, |pin: SchPin<'_>| {
            pin.set_pin_length(SchCoord::from_mils(100.0));
        });
    }

    Ok(())
})?;

// Multiple components — all resistors
doc.query_all::<SchComponent>("R*")?.for_each_mut(|comp| {
    comp.set_description("Resistor (modified)");
    Ok(())
})?;

// Insert new record — same closure shape, template is selected internally
doc.insert_component(|comp| {
    comp.set_lib_reference("R_NEW");
    comp.set_description("New resistor");
    Ok(())
})?;

doc.save("Library.SchLib")?;
```

**Implementation note**: Since wrapper types like `SchPin<'a>` have lifetimes, they can't be used directly as type parameters in `query::<SchPin>()`. The actual mechanism uses a **wrapper family trait** with an associated record type. Each wrapper family is a zero-sized marker type that maps to the wrapper and record types via GATs:

```rust
pub trait WrapperFamily {
    type Record: RecordType;
    type Wrapper<'a>;
}

// SchPin as a type parameter is this marker — not the wrapper itself
pub enum SchPin {}
impl WrapperFamily for SchPin {
    type Record = SchPinRecord;
    type Wrapper<'a> = SchPinWrapper<'a>;  // the actual wrapper struct
}
```

The exact naming of the internal wrapper struct (e.g., `SchPinWrapper<'a>` vs `SchPinView<'a>`) is an implementation detail. Users only see `SchPin` in type parameters and in closure argument types (via type alias).

### SchDoc Generalization

The same API works identically for SchDoc despite different underlying storage. On load, flat records are grouped by OWNERINDEX into `ComponentGroup`s:

```rust
impl SchDoc {
    fn open(reader: impl Read + Seek) -> Result<Self> {
        let records = parse_flat_stream(reader)?;

        // Group by OWNERINDEX → ComponentGroup
        let mut groups: Vec<ComponentGroup> = Vec::new();
        for (original_index, record) in records.into_iter().enumerate() {
            if record.is_component() {
                groups.push(ComponentGroup {
                    component: record,
                    children: Vec::new(),
                    original_indices: vec![original_index],
                });
            } else {
                let owner = record.owner_index();
                groups[owner].children.push(record);
                groups[owner].original_indices.push(original_index);
            }
        }
        // ...
    }
}
```

On save, groups are flattened back to original order using `original_indices`. Untouched records write original bytes; changed records serialize from the backing store. The user code is identical:

```rust
// Works the same for SchDoc — user doesn't know about OWNERINDEX
let mut doc = SchDoc::open("Design.SchDoc")?;

doc.query::<SchComponent>("U1")?.with_mut(|comp| {
    comp.query_all::<SchPin>("pin:power")?.for_each_mut(|pin| {
        pin.set_electrical(PinElectricalType::Passive);
    });
    Ok(())
})?;

doc.save("Design.SchDoc")?;
```

### Query Language Parsing

The AQL grammar (see `docs/query-lang.md`) is parsed using **pest** (PEG parser generator). The grammar file maps directly from the EBNF spec and produces structured AST nodes that the evaluator walks against record collections.

**Why pest:**
- Grammar file (`.pest`) is a direct translation of the EBNF in `docs/query-lang.md` — readable and maintainable.
- Excellent error messages for user-facing query strings (points to the exact character that failed).
- Operator precedence (`NOT` → `AND` → `OR` → union) is natural in PEG.
- No existing parser dependencies in the project — clean addition.

**Dependencies:**
```toml
[dependencies]
pest = "2.7"
pest_derive = "2.7"
```

**Architecture:**

```
src/query/
    grammar.pest       # PEG grammar (translated from docs/query-lang.md EBNF)
    mod.rs             # parse() → AqlQuery AST
    ast.rs             # AqlQuery, Selector, AttrFilter, PseudoClass, etc.
    eval.rs            # evaluate(query, &[RecordNode]) → Vec<usize> (matched indices)
```

The parser produces an AST. The evaluator walks the AST against a record collection and returns matched indices. Both the document-level `query()` and the component-level `query()` use the same parser and evaluator — the only difference is the input record set.

**Alternatives considered:**
- **winnow**: Faster compilation, smaller binary, but requires hand-writing combinators. Better if parsing becomes a bottleneck. Could switch later without changing the AST/evaluator.
- **nom**: Predecessor to winnow, less ergonomic. No advantage over winnow.
- **chumsky**: Best error recovery, but 780 KiB binary overhead — overkill for a query language.

### Standalone Record Usage (Without a Document)

Because getters/setters live on the record type, records work standalone without any document or wrapper:

```rust
// Create from template function
let mut pin = SchPinRecord::new(templates::sch_pin_default());
pin.set_designator(Designator::new("A1"));
pin.set_pin_length(SchCoord::from_mils(100.0));

// Inspect — Designator derefs to str
println!("{}", pin.designator().as_str());

// Use newtype helpers directly
let prefix = pin.designator().prefix();     // "A"
let number = pin.designator().number();      // Some(1)

// Update in place
pin.update_designator(|d| d.increment());    // "A1" → "A2"
```

This is important for tests, and for library consumers who don't need the full document model.

### Why Backing-Store Access, Not Typed Fields

| Concern | Backing-store getters/setters | Direct struct fields |
|---|---|---|
| Stale data | Impossible — reads always come from backing store | Requires dirty tracking to keep fields in sync with origin |
| Unknown field preservation | Automatic — we never extract, so unknowns stay in place | Requires explicit `UnknownFields` bucket that can get out of sync |
| Roundtrip fidelity | Original bytes preserved until explicitly mutated | Must carefully reconstruct ordering, casing, whitespace |
| Type safety | Getters return typed values; setters accept typed values | Same |
| IDE autocomplete | Methods on the record type are visible in autocomplete | Struct fields are visible |
| Caching | Can be added transparently later inside the getter | Fields ARE the cache, but sync is the problem |

### DTO Boundary

Defaults are applied only in boundary mappers, never in core types:

```rust
impl From<&SchPinRecord> for SchPinDto {
    fn from(pin: &SchPinRecord) -> Self {
        Self {
            // Context default applied HERE, not in SchPinRecord
            electrical: pin.try_electrical().unwrap_or(PinElectricalType::Passive),
        }
    }
}
```

## Template System

### Templates Are Code, Not Files

Templates are **functions in code** that return a `RecordOrigin` with Altium-correct default params/binary values in the correct order. They are NOT external `.params` or `.bin` files, and NOT macro attributes.

Different contexts may need different templates (e.g., a pin in a schematic library vs. a pin in a schematic document may have different default fields). Template functions handle this:

```rust
pub mod templates {
    use super::*;

    /// Default SchPin backing store, matching what Altium produces for a new pin.
    pub fn sch_pin_default() -> RecordOrigin {
        RecordOrigin::Param(ParamOrigin {
            params: params! {
                "RECORD" => "2",
                "OWNERINDEX" => "0",
                "OWNERPARTID" => "1",
                "ELECTRICAL" => "4",
                "PINCONGLOMERATE" => "0",
                "PINLENGTH" => "30",
                "PINLENGTH_FRAC" => "0",
                "DESIGNATOR" => "",
                "NAME" => "",
                // ... all default params in Altium's canonical order
            },
            raw_record_text: String::new(),  // no original text — this is new
        })
    }

    /// SchPin for binary mode (different default fields).
    pub fn sch_pin_binary() -> RecordOrigin { ... }

    /// Default PcbPad backing store.
    pub fn pcb_pad_default() -> RecordOrigin {
        RecordOrigin::Binary(BinaryOrigin {
            raw_block: vec![/* default binary bytes from real Altium file */],
            field_spans: vec![/* decoded spans */],
        })
    }
}
```

Template functions are the **only** way to create new records. There is no `Default` impl.

### Builder API

The builder is a convenience wrapper around "create from template + call setters":

```rust
// These are equivalent:
let pin = SchPinRecord::builder(templates::sch_pin_default)
    .designator(Designator::new("A1"))
    .pin_length(SchCoord::from_mils(100.0))
    .build();

// Desugars to:
let mut pin = SchPinRecord::new(templates::sch_pin_default());
pin.set_designator(Designator::new("A1"));
pin.set_pin_length(SchCoord::from_mils(100.0));
```

The builder takes a template function, not a file path. The macro generates the builder type with the same typed setters as the record type. The builder uses the exact same backing store and setters — there is no separate code path.

## Macro v3 Design

### Goals

1. Remove fields from runtime struct; generate record type with typed getters/setters/updaters.
2. Delegate param serialization to types via `ParamCodec` trait — the macro passes a single key per field.
3. Binary records: macro generates getters/setters over field span map; parse/serialize is hand-written per record type.
4. No `default` support in core — defaults come from template functions only.
5. Generate `Builder` type that wraps template function + same setters.
6. For types that don't fit `ParamCodec`, support overriding with `codec_fn = "custom_fn"`.
7. Generate test helpers and `Arbitrary` impls.

**Not generated by the macro:** Hierarchical wrapper types (`SchComponent<'a>`, `SchPin<'a>`, etc.) are hand-written. The macro only generates the `*Record` types that the wrappers Deref to.

### Generated Pieces

From a single `#[derive(AltiumEntity)]` annotation:

- **Record type** (`SchPinRecord`) wrapping `RecordOrigin`, with typed getters/setters/updaters.
- **Builder type** (`SchPinRecordBuilder`) — takes a template function, applies typed overrides.
- **Test helpers** (`SchPinRecord::test_fixture()`, `SchPinRecord::assert_roundtrip_identity()`).
- **`Arbitrary` impl** (behind `#[cfg(test)]`) for property-based testing.

The hierarchical wrappers (`SchPin<'a>`, `SchComponent<'a>`, etc.) are hand-written separately. Most use `impl_leaf_wrapper!` for the boilerplate; parent types like `SchComponent` are fully hand-written.

### Type Traits the Macro Uses

For param-based records, the macro calls `T::read(params, key)` to read and `value.write(params, key)` to write, passing a single key. The type derives any related keys internally.

```rust
/// Convenience traits that ParamCodec can be derived from:
pub trait AltiumCoord: Copy + Sized {
    const UNITS_PER_MIL: i32;
    fn from_raw(raw: i32) -> Self;
    fn to_raw(self) -> i32;
    // Default impls for from_mils, to_mils, from_mm, to_mm, etc.
}

pub trait AltiumEnum: Sized {
    fn from_int(value: i32) -> Self;
    fn to_int(&self) -> i32;
}
```

`AltiumEnum` types get a blanket `ParamCodec` impl (or a derive macro). `AltiumCoord` types implement `ParamCodec` directly because they need to handle the `{key}_FRAC` pattern. Bitflags types implement `ParamCodec` using `.bits()` / `from_bits_truncate()`. Complex param types implement `ParamCodec` by hand or use `codec_fn`.

For binary-codec records, the macro generates getters/setters that index into a field span map built by the record's hand-written parser. No `BinaryCodec` trait — binary records use helper functions for common patterns (`read_i32`, `read_coord`, `read_pascal_string`, etc.) and hand-write their block structure parsing.

## Coordinate System

### SchCoord (100,000 units/mil) and PcbCoord (10,000 units/mil)

Schematic and PCB use different internal scales. Confirmed from decompiled C# (`SchDataSerializerBinary.Export_Coord` divides by 100,000; PCB uses 10,000).

**Decision: Separate non-generic newtypes with a shared `AltiumCoord` trait.**

```rust
pub struct SchCoord(i32);
impl AltiumCoord for SchCoord {
    const UNITS_PER_MIL: i32 = 100_000;
    fn from_raw(v: i32) -> Self { SchCoord(v) }
    fn to_raw(self) -> i32 { self.0 }
}

pub struct PcbCoord(i32);
impl AltiumCoord for PcbCoord {
    const UNITS_PER_MIL: i32 = 10_000;
    fn from_raw(v: i32) -> Self { PcbCoord(v) }
    fn to_raw(self) -> i32 { self.0 }
}
```

SchCoord also gets domain-specific methods for the DXP binary split format:

```rust
impl SchCoord {
    pub fn to_dxp_parts(self) -> (i32, i32) {
        let whole = self.0 / 100_000;
        let frac = self.0 - 100_000 * whole;
        (whole, frac)
    }

    pub fn from_dxp_parts(whole: i32, frac: i32) -> Self {
        SchCoord(whole * 100_000 + frac)
    }

    pub fn to_binary_parts(self) -> (i16, i32) {
        let (whole, frac) = self.to_dxp_parts();
        (whole as i16, frac)
    }

    pub fn from_binary_parts(whole: i16, frac: i32) -> Self {
        Self::from_dxp_parts(whole as i32, frac)
    }
}
```

Point and rect types become generic:

```rust
pub struct Point<C: AltiumCoord> { pub x: C, pub y: C }
pub type SchPoint = Point<SchCoord>;
pub type PcbPoint = Point<PcbCoord>;

pub struct Rect<C: AltiumCoord> { pub min: Point<C>, pub max: Point<C> }
pub type SchRect = Rect<SchCoord>;
pub type PcbRect = Rect<PcbCoord>;
```

### Why Separate Newtypes, Not PhantomData Generic

Options considered and rejected:

| Approach | Why rejected |
|---|---|
| `Coord<S: CoordScale>` with PhantomData | Error messages show `Coord<SchScale>` not `SchCoord`; can't `impl SchCoord`; PhantomData confuses constructors |
| `Coord<const UNITS: i32>` (const generic) | Error messages show `Coord<100000>` — opaque; leaks implementation detail into type |
| Single `Coord` type for both | Can accidentally mix schematic and PCB coordinates; wrong scale silently corrupts data |

Separate newtypes win because:
- Error messages say `SchCoord` and `PcbCoord` — clear and domain-meaningful.
- Each type can have domain-specific methods (SchCoord has `to_dxp_parts`, PcbCoord doesn't).
- Arithmetic boilerplate is trivially solved with a declarative macro (`impl_coord_ops!`).
- The `AltiumCoord` trait provides a shared interface for generic code where needed.
- The derive macro can match on the concrete type name and call the right methods.

### Boundary Unit Type

`SchCoord` and `PcbCoord` always store values in Altium's native internal units. At the DTO/interface boundary, a unit-aware `Measurement<U>` type provides type-safe conversions:

```rust
pub trait Unit {
    const MILS_PER_UNIT: f64;
    const ABBREVIATION: &'static str;
}

pub struct Millimeters;
impl Unit for Millimeters { const MILS_PER_UNIT: f64 = 1.0 / 0.0254; const ABBREVIATION: &'static str = "mm"; }

pub struct Mils;
impl Unit for Mils { const MILS_PER_UNIT: f64 = 1.0; const ABBREVIATION: &'static str = "mil"; }

pub struct Inches;
impl Unit for Inches { const MILS_PER_UNIT: f64 = 1000.0; const ABBREVIATION: &'static str = "in"; }

pub struct Measurement<U: Unit>(f64, PhantomData<U>);
pub type Mm = Measurement<Millimeters>;
pub type Mil = Measurement<Mils>;

// Conversion FROM boundary type INTO internal coords:
impl<U: Unit> From<Measurement<U>> for SchCoord {
    fn from(m: Measurement<U>) -> Self {
        SchCoord::from_mils(m.0 * U::MILS_PER_UNIT)
    }
}

impl<U: Unit> From<Measurement<U>> for PcbCoord {
    fn from(m: Measurement<U>) -> Self {
        PcbCoord::from_mils(m.0 * U::MILS_PER_UNIT)
    }
}
```

Usage:

```rust
// At the boundary — type-safe unit input
pin.set_pin_length(Mm(2.54).into());       // 2.54mm → SchCoord
pin.set_pin_length(Mil(100.0).into());     // 100mil → SchCoord

// Internal — always native units, no unit type
let length = pin.pin_length();              // SchCoord
println!("{:.2} mm", length.to_mm());       // display conversion
```

`Measurement<U>` lives in the DTO/boundary layer, not in core. Core code uses `SchCoord`/`PcbCoord` exclusively.

## Higher-Level Types

### Domain Newtypes for String-Backed Fields

Raw `String` fields should be newtypes with domain-specific helpers. This enables the `update_*` closure pattern — the user gets `&mut Designator` and can call helpers directly without a getter-modify-setter round trip.

Newtypes implement `Deref<Target=str>` for transparent read access, `Display`, `From<&str>`, and `From<String>` for ergonomic use.

| Newtype | Backing | Helper methods |
|---|---|---|
| `Designator` | `String` | `prefix()`, `number()`, `set_number()`, `increment()` |
| `LibReference` | `String` | `normalize()`, `matches_pattern()` |
| `NetName` | `String` | `is_power_net()`, `prefix()`, `matches()` |
| `UniqueId` | `String` | `generate()`, `is_valid()` |
| `Description` | `String` | (thin wrapper, mainly for type safety) |
| `PinName` | `String` | `is_inverted()`, `display_text()` (handles overbar `~` syntax) |

The full newtype inventory will grow as we type more fields. The rule: if a string field has **any** domain semantics beyond "it's text," it gets a newtype.

### Enum and Composite Types

All Altium enumerated values get proper Rust types:

- **Simple enums** (`PinElectricalType`, `PinSymbol`, `LineWidth`, `PcbPadShape`): implement `AltiumEnum` trait, which provides a blanket `ParamCodec` impl.
- **Bitflags** (`PinConglomerateFlags`, etc.): use the `bitflags` crate. Implement `ParamCodec` using `.bits()` / `from_bits_truncate()`.
- **Composite values** (`MaskExpansion = Auto | Manual(Coord)`, etc.): implement `ParamCodec` or `BinaryCodec` by hand.
- **Layer enums with named ranges**: implement `AltiumEnum` by hand with the semantic layer mapping.

### Wrapper Types for Complex Fields

Fields like `size_layers: [CoordPoint; 32]` need domain-aware wrappers:

```rust
pub struct PadLayerStack {
    // Wraps the raw array, knows that index 0 = top, 1-30 = mid layers, 31 = bottom
}

impl PadLayerStack {
    pub fn top(&self) -> PcbPoint { ... }
    pub fn bottom(&self) -> PcbPoint { ... }
    pub fn mid(&self, layer: MidLayerIndex) -> PcbPoint { ... }
    pub fn all_mids(&self) -> impl Iterator<Item = (MidLayerIndex, PcbPoint)> { ... }
    pub fn set_top(&mut self, value: PcbPoint) { ... }
    // etc.
}
```

These types are used by the hand-written binary record parsers. The macro generates getters/setters that call into the wrapper's methods — no special hooks needed.

## v1/v2 Unification Strategy

- One `DocumentCore` struct and one save path.
- Format-specific codecs implement the same core traits:
  - `DecodeOrigin`: parse raw bytes into `RecordOrigin`
  - `EncodeOrigin`: materialize `RecordOrigin` back to bytes
- v1 and v2 become codec implementations, not separate editing stacks.
- `Coord` (v1, 10k/mil) is deprecated in favor of explicit `SchCoord` / `PcbCoord`.

## Test Strategy

### End-to-End Tests: Rebuild from JSON with Templates

The **primary E2E test strategy** is: export to JSON → reimport → build from scratch using template functions. This proves we understand the format, not just that our reader and writer are symmetric.

```
tests/
  e2e/
    rebuild_schlib.rs      # JSON → template → rebuild → compare (structural)
    rebuild_pcblib.rs
    rebuild_schdoc.rs
    rebuild_pcbdoc.rs
  patching/
    identity_schlib.rs     # Open → save → byte-identical (small set)
    single_field_patch.rs  # Open → mutate → save → only mutation changed (small set)
  units/
    coord_conversion.rs    # SchCoord/PcbCoord math
    param_roundtrip.rs     # Per-record param roundtrip (macro-generated)
  proptests/
    record_roundtrip.rs    # Property-based: random valid records → roundtrip → identical
```

### Patching Tests (Smaller Scope)

A small set of integration tests validates in-place editing:
- Open file → save without changes → byte-identical output
- Open file → change one field → save → diff-ole.py shows only that field changed

~5-10 tests per format, not the full corpus.

### Macro-Generated Test Helpers

Per-record, the macro generates:

- `SchPin::test_fixture()` — creates from default template function
- `SchPin::assert_roundtrip_identity(params)` — parse → re-serialize → compare
- `impl Arbitrary for SchPin` — for proptest

### Anti-Pointless Test Rules

A test is rejected unless it asserts at least one of:

1. Byte/stream identity of untouched data.
2. Exact locality of a change after mutation.
3. Explicit invariant/validation behavior.
4. Explicit I/O boundary contract.

## diff-ole.py Improvements

### Current Gaps

1. No record-level understanding of param streams
2. Binary diffs are opaque hex dumps
3. No exit code for CI
4. No OLE container metadata comparison
5. No tolerance modes

### Required Improvements

1. **Param-aware comparison**: For text streams, compare both raw (byte-for-byte) AND order-normalized. Report which records differ only in order vs. have actual data differences. Binary streams: naive hex comparison only (order matters).

2. **Byte-exact as the criterion**: Format-aware reporting is for human readability. The pass/fail criterion is byte-for-byte identity. The `--semantic` mode (order-insensitive) is separate.

3. **Exit codes**: `--assert-identical` flag returns non-zero on any byte difference. `--assert-semantic` returns non-zero on any data difference (ignoring order).

4. **Container-level comparison**: Compare OLE metadata — sector sizes, directory entry ordering, timestamps, mini-stream cutoff size. This is critical for understanding why our output files differ in size from Altium's (566KB vs 559KB for Synthiam.SchLib). Need to investigate what OLE settings Altium uses.

5. **Both comparison modes**: `--strict` (byte-for-byte) and `--semantic` (order-insensitive). Both available, both testable.

### Keep It in Python

The diff tool stays in Python (separate implementation from the Rust library) as a deliberate cross-validation strategy. If both have the same bug, we won't catch it.

## CLI Surface Plan

Do not remove commands in code right now. Track with explicit freeze/remove/rebuild stages.

### Stage 1: Freeze During Core Refactor

- Freeze feature work on higher-level CLI flows.
- Allow only break/fix changes needed to keep build and baseline tests green.
- Do not introduce new query semantics while core APIs are being rebuilt.

### Stage 2: Validate Core First

- Complete refactoring, test upgrades, and validation gates:
  - no-edit identity guarantees
  - in-place edit locality guarantees
  - DTO-default boundary guarantees
- Keep old high-level commands as temporary wrappers.

### Stage 3: Rip Out and Redesign

- Remove old command implementations, replace with thin adapters over the new core.
- New command APIs use the closure-based access and record type getters/setters.

## Open Questions

1. **Binary record field span map**: For binary records with complex multi-block structures (like PcbPad with its 6 blocks), the hand-written parser builds the field span map. How should this map be structured for records with variable-length blocks? May need a two-level map: block-level + field-level within each block.

2. ~~**Cross-record operations**~~ **RESOLVED**: The hierarchical wrapper solves this. `SchComponentMut` borrows the component record and its children from separate `ComponentGroup` fields, so you can read the parent while mutating children (via `split()`). For mutations across sibling components, use sequential `query().with_mut()` calls — each closure borrows and releases one component at a time.

3. **PadLayerStack and similar complex wrappers**: How many of these do we need? Each one is hand-written domain logic. Need an inventory of all complex binary field types across all record types.

4. **Template function coverage**: We need template functions for every record type we want to create. Default values must be extracted from real Altium files. May need a template extraction tool to bootstrap these.

5. **OLE container compatibility**: What sector size, mini-stream cutoff, and directory entry ordering does Altium use? This determines whether we can achieve byte-identical CFB output.

6. ~~**insert vs. update semantics**~~ **RESOLVED**: The query API provides `query().with_mut()` for editing existing records. `insert_component()` / `insert_pin()` use the same closure shape for creating new records from templates. The semantic distinction (edit vs. create) is worth keeping because insert needs a template function and may need to update indices/keys.

7. **PcbLib/PcbDoc hierarchical model**: PCB formats have different structure — PcbLib has per-footprint sections, PcbDoc has typed streams (Tracks6, Arcs6, etc.). The `ComponentGroup` model fits schematic formats naturally but may need a different grouping strategy for PCB. Should PcbDoc use typed-stream groups instead of OWNERINDEX groups?

8. **Query language implementation scope**: The AQL spec (docs/query-lang.md) is comprehensive. For v2 initial implementation, which subset is required? Pattern selectors and attribute selectors likely cover 90% of use cases. Combinators and pseudo-classes can be deferred.

9. **ChildHandle borrow scope**: `comp.query::<SchPin>(q)?` takes `&mut self` on the wrapper, which means you can't interleave query calls. Each query+with_mut chain must complete before the next one starts. Is this acceptable, or do we need a way to batch multiple child queries?

## Definition of Done

1. All record types use backing-store access — no runtime typed fields.
2. All getters/setters use proper domain newtypes, coordinates, enums, and bitflags types.
3. Param types handle their own serialization via `ParamCodec` trait (single key). Binary records use hand-written parsers with helper functions.
4. Core types have zero implicit defaults. New records are created from template functions.
5. `UnknownFields` type is removed entirely. Unknown data lives in the backing store.
6. SchCoord (100k/mil) and PcbCoord (10k/mil) are separate types with `AltiumCoord` trait.
7. Boundary `Measurement<U>` type provides type-safe unit conversions via `From` impls.
8. Hierarchical wrapper types (`SchComponentMut`, etc.) Deref to record types and provide child navigation via closures and query.
9. `ComponentGroup` storage separates component record from children for split-borrow safety. SchLib builds groups per CFB stream; SchDoc groups by OWNERINDEX.
10. Query API: `query::<T>(q)` errors on 0 or 2+ matches; `query_all::<T>(q)` returns all. Same AQL works at document level and within component wrappers for child access.
11. `QueryHandle` and `ChildHandle` have `with_mut`/`with_ref` directly on them. `ChildKey<T>` + `with_child_mut` available for when handles are passed around.
12. `Designator` (concrete, SchDoc) and `DesignatorTemplate` (placeholder, SchLib) are separate newtypes.
13. E2E tests rebuild from JSON using templates. In-place edit tests cover each record type.
14. diff-ole.py has exit codes, container-level comparison, and both strict/semantic modes.
15. Existing test files still exist but assert functional behavior with high signal.
16. CLI command redesign starts only after core/test/validation gates are complete.
