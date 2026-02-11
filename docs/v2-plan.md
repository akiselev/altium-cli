# altium-format v2: Clean-Slate Architecture

## Intent

Design from scratch for one goal: **lossless, nondestructive editing with an ergonomic imperative API**.

This document intentionally ignores migration constraints. Existing code is treated as a knowledge base and oracle; the new architecture is allowed to break old APIs.

**Goals**

1. **Origin-backed records**: Records store their raw representation. This gives non-destructive editing and lossless roundtrip.
2. **In-place mutation**: Setters patch the backing store directly. The backing store IS the data.
3. **No defaults in core types**: Defaults live in template functions and DTO layers only.
4. **I/O in Document types, not record types**: Records are pure data. Documents handle CFB/OLE.


## Non-Negotiables

1. **Lossless by default**: if nothing is edited, output bytes are byte-identical for every untouched stream.
2. **Edit in place**: setters patch the backing store directly. The backing store is the same type whether it came from an Altium file or a template — there is no separate "create" vs "edit" mechanism.
3. **No core defaults**: contextual defaults live in DTO/interface layers only (CLI/JSON/UI), never in core record types. Template functions provide Altium-correct defaults for new records.
4. **No runtime fields**: macro-declared fields exist for documentation and autocomplete; runtime access is through generated getters/setters over the backing store.
5. **Macro-first generation**: the macro generates `*Record` types with getters/setters and builder APIs. Hierarchical view types (`*View`) are hand-written (Deref/DerefMut to record types for getters/setters, DerefMut marks dirty, child navigation for parent types). Param types handle their own serialization via `ParamCodec` (single key). Binary records use sequential-layout macro or hand-written parsers with helper functions.
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
    key: u8,                       // RECORD param value (sch) or type byte (pcb)
    origin: RecordOrigin,
    original_snapshot: Vec<u8>,    // for byte-level dirty detection on save
    dirty: bool,                   // set by DerefMut on wrapper types
}
```

```rust
pub enum RecordOrigin {
    Param(ParamOrigin),
    Binary(BinaryOrigin),
}

pub struct ParamOrigin {
    params: ParameterCollection,   // THE authoritative data — reads and writes go here
                                   // Order-preserving: keys maintain original insertion order
                                   // for lossless roundtrip of untouched records
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

Each document type is a separate struct. Schematic formats use `ComponentGroup` for hierarchical access; PcbLib uses `FootprintGroup`. PcbDoc is deferred.

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
    footprints: Vec<FootprintGroup>,     // one per CFB storage
    section_keys: SectionKeyList,        // maps long names → truncated storage names
    raw_streams: BTreeMap<String, Vec<u8>>,  // other CFB streams (verbatim roundtrip)
}
```

Separate types per format because they are semantically different:

- **SchLib**: components containing param-based primitives (tree structure) → `ComponentGroup` per stream
- **SchDoc**: flat param-based primitives with OWNERINDEX links → `ComponentGroup` per owner, built on load
- **PcbLib**: footprints containing binary primitives (tree structure) → `FootprintGroup` per CFB storage
- **PcbDoc**: deferred (flat binary records in typed streams — Tracks6, Arcs6, etc.)

`DocumentCore` remains as a lower-level building block for generic operations (iteration, save, test fixtures).

## API Shape: Imperative Access with Backing Store

### Three Layers: Records, Hierarchical Wrappers, and Query

There are three distinct layers:

1. **Record types** (`SchPinRecord`, `SchComponentRecord`, etc.) — own a `RecordOrigin` and have typed getters/setters directly on them. Records are **pure data** — they have no knowledge of parent/child/sibling relationships. No lifetimes, no document awareness. The `Record` suffix distinguishes this layer from the wrapper layer. The macro generates these types. **Users never interact with `*Record` types directly.**

2. **Hierarchical view types** (`SchComponentView`, `SchPinView`, etc.) — hand-written wrappers that borrow into the document's `ComponentGroup` storage. `Deref`/`DerefMut` to their underlying record type so the user gets all getters/setters for free. `DerefMut` marks the record dirty. Most views are trivial — just Deref/DerefMut to the record type. Only parent types like `SchComponentView` add child navigation methods (query, iterate, with_child_mut). Views are **hand-written, not macro-generated**, because this is where API ergonomics lives. Users interact with view types through closures (e.g., `with_mut(|comp| { ... })`) and with marker types as type parameters (e.g., `query::<SchComponent>(...)`).

3. **Query API** (`doc.query::<SchComponent>(q)`, `doc.query_all::<SchComponent>(q)`) — returns handles that open closures over matched records. Same query language works at both the document level and within hierarchical wrappers for child access. `query` errors on 0 or 2+ matches; `query_all` returns all matches. Type parameters are wrapper family markers, not record types.

The user interacts with marker types (for type parameters) and view types (in closures). Record types are internal — the view's Deref makes getters/setters available transparently. Most view types are a single Deref/DerefMut impl. Mut-only for now — no read-only view variants.

### Macro Declaration (Source of Truth)

Fields exist in the source code for documentation and IDE autocomplete. The macro reads them but **removes them from the runtime struct**, replacing the struct with a backing-store wrapper. The macro generates typed getters/setters directly on the record type.

Field types should be **domain newtypes**, not raw primitives. A `Designator` is not a `String` — it has structure (prefix + number), helper methods, and validation. Newtypes let the macro generate `update_*` closures that give `&mut` access to the parsed value for in-place modification, avoiding a getter-modify-setter round trip.

The macro passes param key names to the type's `ParamCodec` trait implementation. **Types handle their own serialization** — the macro just orchestrates. This means `SchCoord` knows how to read/write its integer+frac pair, `Designator` knows how to read/write its string, etc. The macro doesn't need special-case logic for `frac`, `bitflags`, or any other encoding detail.

```rust
#[altium_record(kind = "sch", record_id = 2, codec = "params")]
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

**Why an attribute macro, not derive**: Derive macros can only add impl blocks — they cannot modify the struct. Since we need to replace the struct's fields with a `RecordOrigin` backing store, this must be an **attribute macro** (`#[altium_record]`). The attribute macro consumes the struct definition (using the fields for documentation and code generation) and emits a new struct wrapping `RecordOrigin` plus all the generated getters/setters/updaters.

### Domain Newtypes

String-backed fields get newtypes with domain-specific helpers:

```rust
/// Designator string — covers both concrete ("R1", "U3") and template ("U?", "R?") forms.
/// A single newtype that interprets the string based on content.
pub struct Designator(String);

impl Designator {
    pub fn new(s: impl Into<String>) -> Self { ... }
    pub fn as_str(&self) -> &str { &self.0 }
    pub fn prefix(&self) -> &str { ... }           // "U2" → "U", "U?" → "U"
    pub fn is_template(&self) -> bool { ... }      // has "?" placeholder
    pub fn number(&self) -> Option<u32> { ... }    // "U2" → Some(2), "U?" → None
    pub fn set_number(&mut self, n: u32) { ... }   // "U2" → "U5"
    pub fn increment(&mut self) { ... }            // "U2" → "U3" (panics on template)
    pub fn resolve(&self, n: u32) -> Designator { ... }  // "U?" + 3 → "U3"
}
```

A single `Designator` type for both SchLib (template form `U?`) and SchDoc (concrete form `U1`). The `is_template()` method distinguishes them. Both forms read/write identically via `ParamCodec` — just a string to the `DESIGNATOR` key.

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
pub struct SchPinView<'a> {
    record: &'a mut SchPinRecord,
}

impl<'a> Deref for SchPinView<'a> { type Target = SchPinRecord; }
impl<'a> DerefMut for SchPinView<'a> {
    fn deref_mut(&mut self) -> &mut SchPinRecord {
        self.record.mark_dirty();  // any &mut access marks dirty
        self.record
    }
}
```

Dirty tracking uses `DerefMut`: Rust dispatches `Deref` for shared borrows and `DerefMut` for mutable borrows, so read-only access through getters does NOT mark dirty. Only setter calls (which require `&mut self`) trigger `DerefMut` and mark the record. No snapshot, no panic rollback — if a closure panics, dirty state is left as-is.

These are boilerplate-heavy but intentionally hand-written. A helper macro (`impl_leaf_wrapper!`) can reduce repetition without going full code generation — this keeps the wrapper layer explicit and easy to customize per type:

```rust
impl_leaf_wrapper!(SchPinView<'a> wraps SchPinRecord);
impl_leaf_wrapper!(SchArcView<'a> wraps SchArcRecord);
impl_leaf_wrapper!(SchLineView<'a> wraps SchLineRecord);
impl_leaf_wrapper!(SchRectangleView<'a> wraps SchRectangleRecord);
// ... etc
```

#### Parent Wrappers (SchComponent)

Parent types add child navigation. `SchComponent` is the primary example — it borrows both the component record and its children from separate `ComponentGroup` fields, which is what enables the split-borrow pattern:

```rust
/// Hand-written. Wraps SchComponentRecord + children with child navigation.
pub struct SchComponentView<'a> {
    component: &'a mut SchComponentRecord, // borrows ComponentGroup.component
    children: &'a mut [RecordNode],        // borrows ComponentGroup.children
}

impl<'a> Deref for SchComponentView<'a> { type Target = SchComponentRecord; }
impl<'a> DerefMut for SchComponentView<'a> {
    fn deref_mut(&mut self) -> &mut SchComponentRecord {
        self.component.mark_dirty();
        self.component
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
| **Dirty tracking** | DerefMut marks the record dirty on any mutable access |

#### Child Access Methods

```rust
impl<'a> SchComponentView<'a> {
    /// Iterate all pins with mutable access.
    /// Closure receives SchPinView wrapper, not SchPinRecord.
    pub fn for_each_pin_mut(&mut self, f: impl FnMut(SchPinView<'_>)) { ... }

    /// Query children — same AQL syntax, scoped to this component.
    /// T is a WrapperFamily marker (SchPin, SchLine, etc.)
    /// Errors on 0 or 2+ matches.
    pub fn query<T: WrapperFamily>(&mut self, q: &str) -> Result<ChildHandle<'_, T>> { ... }

    /// Query children — returns all matches.
    pub fn query_all<T: WrapperFamily>(&mut self, q: &str) -> Result<ChildResults<'_, T>> { ... }

    /// Access a specific child by key (for when handles are passed around).
    /// Closure receives wrapper type, not record type.
    pub fn with_child_mut<T: WrapperFamily, R>(
        &mut self, key: ChildKey<T>, f: impl FnOnce(T::View<'_>) -> R,
    ) -> R { ... }

    /// Get child keys for external use (passing to other functions, collecting).
    pub fn child_keys<T: WrapperFamily>(&self) -> impl Iterator<Item = ChildKey<T>> { ... }

    /// Split borrows escape hatch — returns independent refs to parent record and children.
    pub fn split(&mut self) -> (&mut SchComponentRecord, ChildrenMut<'_>) { ... }

    pub fn pin_count(&self) -> usize { ... }
}
```

#### Child Handles (from query)

`ChildHandle` and `ChildResults` are returned by query methods on the wrapper. They hold `&mut` into the children storage and provide `with_mut`/`for_each_mut` directly — same pattern as the document-level `QueryHandle`. Closures receive **wrapper view types**, not record types. Closures are generic over return type — they pass through whatever the user returns.

```rust
/// Single child match — from comp.query::<SchPin>("pin[name=VCC]")?
/// T is a WrapperFamily marker (SchPin, SchLine, etc.)
pub struct ChildHandle<'a, T: WrapperFamily> {
    children: &'a mut [RecordNode],
    index: usize,
    _marker: PhantomData<T>,
}

impl<'a, T: WrapperFamily> ChildHandle<'a, T> {
    pub fn with_mut<R>(self, f: impl FnOnce(T::View<'_>) -> R) -> R { ... }
}

/// Multiple child matches — from comp.query_all::<SchPin>("pin:power")?
pub struct ChildResults<'a, T: WrapperFamily> {
    children: &'a mut [RecordNode],
    indices: Vec<usize>,
    _marker: PhantomData<T>,
}

impl<'a, T: WrapperFamily> ChildResults<'a, T> {
    pub fn for_each_mut(self, f: impl FnMut(T::View<'_>)) { ... }
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
    pub fn query<T: WrapperFamily>(&mut self, q: &str) -> Result<QueryHandle<'_, T>> { ... }

    /// Find all matching components.
    pub fn query_all<T: WrapperFamily>(&mut self, q: &str) -> Result<QueryResults<'_, T>> { ... }
}
```

`query()` and `query_all()` return `Result` with our error type (parse errors, zero/multiple matches). Both take `&mut self` because the returned handles need mutable access for `with_mut`. The query itself only reads, but the handle must be able to open a mutable closure.

Closure return types are **generic passthrough** — the closure returns whatever the user wants (`R`), and the handle passes it through. This lets users return custom types, `Result`, or `()` as needed. Only the query functions themselves return our `Result` type.

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
    /// Closure return type is generic — passes through whatever the user returns.
    pub fn with_mut<R>(self, f: impl FnOnce(SchComponentView<'_>) -> R) -> R {
        let group = &mut self.doc.groups[self.index];
        // split borrow: component vs children — separate ComponentGroup fields
        let (comp_node, children) = (&mut group.component, &mut group.children[..]);
        let record = SchComponentRecord::from_origin_mut(&mut comp_node.origin);
        f(/* construct SchComponentView with record + children */)
    }
}

/// Multiple matches — from doc.query_all::<SchComponent>("R*")?
pub struct QueryResults<'a, T: WrapperFamily> {
    doc: &'a mut SchLib,
    indices: Vec<usize>,
    _marker: PhantomData<T>,
}

impl<'a> QueryResults<'a, SchComponent> {
    pub fn for_each_mut(self, f: impl FnMut(SchComponentView<'_>)) { ... }
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
    // comp: SchComponentView<'_> — Deref to SchComponentRecord
    // Getters/setters available directly via Deref
    comp.set_lib_reference("LM358N");
    comp.set_description("Dual Op-Amp");

    // Query single child — with_mut directly on the handle
    comp.query::<SchPin>("pin[name=VCC]")?.with_mut(|pin| {
        // pin: SchPinView<'_> — Deref to SchPinRecord
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
    let pin_keys: Vec<_> = comp.child_keys::<SchPin>().collect();
    for key in pin_keys {
        comp.with_child_mut(key, |pin| {
            pin.set_pin_length(SchCoord::from_mils(100.0));
        });
    }
});

// Multiple components — all resistors
doc.query_all::<SchComponent>("R*")?.for_each_mut(|comp| {
    comp.set_description("Resistor (modified)");
});

// Build new component with children from templates
doc.build_component(templates::sch_component_default, |comp| {
    comp.set_lib_reference("R_NEW");
    comp.set_description("New resistor");
    comp.add_pin(templates::sch_pin_default, |pin| {
        pin.set_designator(Designator::new("1"));
        pin.set_name(PinName::new("A"));
    });
    comp.add_pin(templates::sch_pin_default, |pin| {
        pin.set_designator(Designator::new("2"));
        pin.set_name(PinName::new("B"));
    });
})?;

doc.save("Library.SchLib")?;
```

**Implementation note**: Since view types like `SchPinView<'a>` have lifetimes, they can't be used directly as type parameters in `query::<SchPin>()`. The actual mechanism uses a **wrapper family trait** with an associated record type. Each wrapper family is a zero-sized marker type that maps to the view and record types via GATs:

```rust
pub trait WrapperFamily {
    type Record: RecordType;
    type View<'a>;
}

// SchPin as a type parameter is this marker — not the view itself
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
```

Users see `SchPin` in type parameters (e.g., `query::<SchPin>(...)`) and `SchPinView<'_>` in closure arguments. The `*View` suffix is used consistently for all internal wrapper structs.

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
});

doc.save("Design.SchDoc")?;
```

### PcbLib Design

PcbLib uses the same hierarchical pattern as schematic formats. Each footprint is a CFB storage containing binary primitive records (pads, tracks, arcs, etc.). The `FootprintGroup` is analogous to `ComponentGroup`:

```rust
pub struct FootprintGroup {
    metadata: RecordNode,                      // from Parameters stream (param-based)
    primitives: Vec<RecordNode>,               // from Data stream (binary records)
    raw_pattern_name_block: Vec<u8>,           // original name block for lossless roundtrip
    original_primitive_order: Vec<PcbPrimitiveRef>,  // type + index for ordering preservation
    raw_header: Vec<u8>,                       // original Header stream (u32 count)
}
```

**CFB layout per footprint:**

```
/{FootprintName}/
    Header          → u32 primitive count
    Data            → pattern name block + mixed binary primitive records
    Parameters      → parametric properties (|KEY=VALUE| text)
    WideStrings     → optional extended strings
```

**Record dispatch:** Each binary record starts with a type byte (u8) mapped to `PcbObjectId`:

| Type byte | Record | Framing |
|-----------|--------|---------|
| 1 | Arc | `type + u32 len + data` |
| 2 | Pad | `type + 6 subrecords (each u32 len + data)` — NO outer length |
| 3 | Via | `type + u32 len + data` (multi-section internally) |
| 4 | Track | `type + u32 len + data` |
| 5 | Text | `type + 2 subrecords (each u32 len + data)` — NO outer length |
| 6 | Fill | `type + u32 len + data` |
| 11 | Region | `type + u32 len + binary header + params + vertices` (hybrid) |
| 12 | ComponentBody | `type + u32 len + data` (hybrid like Region) |

Unknown type bytes are stored as raw bytes for lossless roundtrip.

**All binary records share a 13-byte common header:**

```rust
pub struct PcbCommonHeader {
    layer: u8,              // TLayer enum (0-82)
    flags: u16,             // bitmask (locked, teardrop, tent, etc.)
    net: u16,               // 0xFFFF = no ref
    polygon_ref: u16,       // 0xFFFF = no ref
    component_ref: u16,     // 0xFFFF = no ref
    ref4: u16,              // 0xFFFF = no ref
    ref5: u16,              // 0xFFFF = no ref
}
```

**Query API — same pattern as schematic:**

```rust
let mut lib = PcbLib::open("Library.PcbLib")?;

lib.query::<PcbFootprint>("SOIC-8")?.with_mut(|fp| {
    fp.query_all::<PcbPad>("pad")?.for_each_mut(|pad| {
        pad.set_hole_size(PcbCoord::from_mils(40.0));
    });

    fp.query_all::<PcbTrack>("track[width>=10mil]")?.for_each_mut(|track| {
        track.set_width(PcbCoord::from_mils(12.0));
    });
});

lib.save("Library.PcbLib")?;
```

**Wrapper families for PCB:**

```rust
pub enum PcbFootprint {}
impl WrapperFamily for PcbFootprint {
    type Record = PcbFootprintRecord;     // metadata from Parameters stream
    type View<'a> = PcbFootprintView<'a>; // parent wrapper with child navigation
}

pub enum PcbPad {}
impl WrapperFamily for PcbPad {
    type Record = PcbPadRecord;
    type View<'a> = PcbPadView<'a>;       // leaf wrapper
}

// Same for PcbTrack, PcbArc, PcbVia, PcbFill, PcbText, PcbRegion
```

`PcbFootprintView` is a parent wrapper (like `SchComponentView`) — it Derefs to `PcbFootprintRecord` for metadata access and provides child navigation over its primitives. Leaf wrappers (`PcbPadView`, `PcbTrackView`, etc.) use `impl_leaf_wrapper!`.

**PcbDoc is deferred.** PcbDoc has a fundamentally different structure (flat typed streams like Tracks6, Arcs6 rather than per-component grouping) that needs separate design work. PcbLib is sufficient for initial library editing workflows.

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

**Initial scope (v2):** Pattern selectors + attribute selectors only. These cover ~90% of use cases. Combinators (`>`, `+`, `~`, `,`) and pseudo-classes (`:power`, `:input`, etc.) are deferred to a later version.

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

**Not generated by the macro:** Hierarchical wrapper types (`SchComponentView<'a>`, `SchPinView<'a>`, etc.) are hand-written. The macro only generates the `*Record` types that the view types Deref to.

### Generated Pieces

From a single `#[altium_record]` attribute macro:

- **Record type** (`SchPinRecord`) wrapping `RecordOrigin`, with typed getters/setters/updaters.
- **Builder type** (`SchPinRecordBuilder`) — takes a template function, applies typed overrides.
- **Test helpers** (`SchPinRecord::test_fixture()`, `SchPinRecord::assert_roundtrip_identity()`).
- **`Arbitrary` impl** (behind `#[cfg(test)]`) for property-based testing.

The hierarchical view types (`SchPinView<'a>`, `SchComponentView<'a>`, etc.) are hand-written separately. Most use `impl_leaf_wrapper!` for the boilerplate; parent types like `SchComponentView` are fully hand-written.

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

`AltiumEnum` types get an `#[altium_enum]` attribute macro that generates the `AltiumEnum` trait impl (from_int/to_int mapping), plus a blanket `impl<T: AltiumEnum> ParamCodec for T` that handles the read/write. `AltiumCoord` types implement `ParamCodec` directly because they need to handle the `{key}_FRAC` pattern. Bitflags types implement `ParamCodec` using `.bits()` / `from_bits_truncate()`. Complex param types implement `ParamCodec` by hand or use `codec_fn`.

### Binary Record Macro Attributes

For binary-codec records, the `#[altium_record]` macro supports two modes:

**Sequential layout (simple records — Track, Arc, Fill):** Fields are declared in order. The macro computes offsets from type sizes. No explicit offsets needed.

```rust
#[altium_record(kind = "pcb", object_id = Track, codec = "binary")]
struct PcbTrackRecord {
    #[altium(header)]
    header: PcbCommonHeader,          // 13 bytes, always first

    start_x: PcbCoord,                // 4 bytes at offset 13
    start_y: PcbCoord,                // 4 bytes at offset 17
    end_x: PcbCoord,                  // 4 bytes at offset 21
    end_y: PcbCoord,                  // 4 bytes at offset 25
    width: PcbCoord,                  // 4 bytes at offset 29
    subpoly_index: u16,              // 2 bytes at offset 33

    #[altium(trailing)]
    trailing: PcbTrailingFields,     // adaptive trailing fields (1-14 bytes)
}
```

The macro knows the size of each type (`PcbCoord` = 4 bytes, `u16` = 2 bytes, `f64` = 8 bytes, `u8` = 1 byte, `bool` = 1 byte, `PcbCommonHeader` = 13 bytes) and computes offsets automatically. The `#[altium(header)]` attribute marks the common header. The `#[altium(trailing)]` attribute marks adaptive trailing fields parsed from remaining bytes.

**Custom parser (complex multi-block records — Pad, Via, Text, Region):** For records with variable-length blocks or multi-subrecord structures, the macro generates getters/setters but delegates parse/serialize to hand-written functions.

```rust
#[altium_record(kind = "pcb", object_id = Pad, codec = "binary",
    parse_fn = "parse_pad", serialize_fn = "serialize_pad")]
struct PcbPadRecord {
    name: PadName,
    position_x: PcbCoord,
    position_y: PcbCoord,
    top_size: PcbPoint,
    mid_size: PcbPoint,
    bot_size: PcbPoint,
    hole_size: PcbCoord,
    top_shape: PcbPadShape,
    mid_shape: PcbPadShape,
    bot_shape: PcbPadShape,
    rotation: f64,
    is_plated: bool,
    // ... all fields declared for documentation + getter/setter generation
}
```

The hand-written `parse_pad` function reads the 6 subrecords, builds a `BinaryOrigin` with a field span map, and returns a `RecordOrigin`. The hand-written `serialize_pad` function writes the subrecords back from the span map. The macro generates typed getters/setters that read/write through the span map using field IDs derived from field declaration order.

**Field span map:** For custom-parser records, each field gets an auto-generated `usize` ID based on declaration order (0, 1, 2, ...). The hand-written parser builds a `Vec<FieldSpan>` indexed by these IDs:

```rust
pub struct FieldSpan {
    offset: usize,    // byte offset in raw_block
    size: usize,      // field size in bytes
}

// Generated getter:
pub fn position_x(&self) -> PcbCoord {
    let span = &self.origin.binary().field_spans[Self::FIELD_POSITION_X]; // ID = 1
    PcbCoord::from_raw(i32::from_le_bytes(
        self.origin.binary().raw_block[span.offset..span.offset+4].try_into().unwrap()
    ))
}
```

The macro generates `const FIELD_*: usize` constants so the hand-written parser can use them to build the span map consistently.

### Insert / Builder API

**Record-level builders** are generated by `#[altium_record]`. They wrap `new(template) + set_*()` in a fluent chain:

```rust
let pin = SchPinRecord::builder(templates::sch_pin_default)
    .designator(Designator::new("A1"))
    .pin_length(SchCoord::from_mils(100.0))
    .build();
```

No external builder crate — the macro generates this trivially since it already knows all fields and setters.

**Document-level insertion** is hand-written on each document type because it involves domain logic (index management, hierarchy construction, ordering):

```rust
impl SchLib {
    /// Build a new component with children from templates.
    pub fn build_component(
        &mut self,
        template: fn() -> RecordOrigin,
        build: impl FnOnce(&mut ComponentBuilder),
    ) -> Result<()> {
        let mut builder = ComponentBuilder::new(template());
        build(&mut builder);
        self.groups.push(builder.into_group());
        Ok(())
    }
}

/// Builder for constructing a component with children.
/// Hand-written — handles hierarchy construction.
pub struct ComponentBuilder {
    component: RecordNode,
    children: Vec<RecordNode>,
}

impl ComponentBuilder {
    /// Deref to the component record for setting fields.
    pub fn set_lib_reference(&mut self, v: impl Into<LibReference>) { ... }

    /// Add a child record from a template.
    pub fn add_pin(
        &mut self,
        template: fn() -> RecordOrigin,
        build: impl FnOnce(&mut SchPinRecord),
    ) {
        let mut node = RecordNode::new(template());
        build(node.as_record_mut());
        self.children.push(node);
    }

    // add_line, add_arc, add_rectangle, etc.
}
```

For SchDoc, the builder also assigns OWNERINDEX values automatically. For PcbLib, `FootprintBuilder` handles the pattern name block, header, and primitive ordering.

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
| `Designator` | `String` | `prefix()`, `number()`, `is_template()`, `set_number()`, `increment()`, `resolve()` |
| `LibReference` | `String` | `normalize()`, `matches_pattern()` |
| `NetName` | `String` | `is_power_net()`, `prefix()`, `matches()` |
| `UniqueId` | `String` | `generate()`, `is_valid()` |
| `Description` | `String` | (thin wrapper, mainly for type safety) |
| `PinName` | `String` | `is_inverted()`, `display_text()` (handles overbar `~` syntax) |

The full newtype inventory will grow as we type more fields. The rule: if a string field has **any** domain semantics beyond "it's text," it gets a newtype.

### Enum and Composite Types

All Altium enumerated values get proper Rust types:

- **Simple enums** (`PinElectricalType`, `PinSymbol`, `LineWidth`, `PcbPadShape`): use `#[altium_enum]` attribute macro to generate `AltiumEnum` trait impl. A blanket `impl<T: AltiumEnum> ParamCodec for T` provides serialization for free.
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

## v1/v2 Migration Strategy

When v2 work begins, remove v1 from the module hierarchy entirely. Only migrate what is needed from v1 into the v2 module structure — do NOT copy everything. The v1 code serves as a knowledge base and oracle, not as a codebase to preserve.

**Steps:**
1. Remove v1 module exports — builds break immediately, which is intentional.
2. Build v2 module structure from scratch (backing store, record types, wrappers, query).
3. As each record type is implemented in v2, pull knowledge from the corresponding v1 code (field names, param keys, binary offsets, enum mappings). Delete the v1 source file once v2 replaces it.
4. Format-specific codecs implement core traits:
   - `DecodeOrigin`: parse raw bytes into `RecordOrigin`
   - `EncodeOrigin`: materialize `RecordOrigin` back to bytes
5. `Coord` (v1, 10k/mil) is removed in favor of explicit `SchCoord` / `PcbCoord`.

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

- `SchPinRecord::test_fixture()` — creates from default template function
- `SchPinRecord::assert_roundtrip_identity(params)` — parse → re-serialize → compare
- `impl Arbitrary for SchPinRecord` — for proptest (creates from template, then applies random valid values via setters using per-field proptest strategies)

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

1. ~~**Binary record field span map**~~ **RESOLVED**: Simple binary records use sequential field layout in the macro (offsets computed from type sizes). Complex multi-block records (Pad, Via, Text, Region) use `parse_fn`/`serialize_fn` with hand-written parsers that build a flat `Vec<FieldSpan>` indexed by macro-generated `FIELD_*` constants. See [Binary Record Macro Attributes](#binary-record-macro-attributes).

2. ~~**Cross-record operations**~~ **RESOLVED**: The hierarchical wrapper solves this. `SchComponentView` borrows the component record and its children from separate `ComponentGroup` fields, so you can read the parent while mutating children (via `split()`). For mutations across sibling components, use sequential `query().with_mut()` calls — each closure borrows and releases one component at a time.

3. **PadLayerStack and similar complex wrappers**: These are hand-written domain types used by hand-written binary record parsers (NOT generated by the macro — the macro generates `*Record` types only, wrappers are hand-written). Need an inventory of all complex binary field types across PcbLib record types during implementation.

4. **Template function coverage**: We need template functions for every record type we want to create. Default values must be extracted from real Altium files. Template extraction tooling will be built later — for initial implementation, extract defaults manually from test fixtures.

5. **OLE container compatibility**: What sector size, mini-stream cutoff, and directory entry ordering does Altium use? This is critical for byte-identical CFB output. Will be investigated during implementation — examine real Altium files with diff-ole.py and patch the `cfb` crate configuration as needed.

6. ~~**insert vs. update semantics**~~ **RESOLVED**: The query API provides `query().with_mut()` for editing existing records. `build_component()` / `add_pin()` use a builder closure pattern for creating new records from templates. See [Insert / Builder API](#insert--builder-api).

7. ~~**PcbLib/PcbDoc hierarchical model**~~ **RESOLVED**: PcbLib uses `FootprintGroup` (analogous to `ComponentGroup`). PcbDoc is deferred. See [PcbLib Design](#pcblib-design).

8. ~~**Query language implementation scope**~~ **RESOLVED**: Pattern selectors + attribute selectors for initial v2 implementation. Combinators and pseudo-classes deferred.

9. ~~**ChildHandle borrow scope**~~ **RESOLVED**: Sequential-only constraint is acceptable. Each `query().with_mut()` chain must complete before the next. `ChildKey<T>` + `child_keys()` provides a collect-and-iterate pattern for when handles need to be passed around. No query batching.

## Definition of Done

1. All record types use backing-store access — no runtime typed fields.
2. All getters/setters use proper domain newtypes, coordinates, enums, and bitflags types.
3. Param types handle their own serialization via `ParamCodec` trait (single key). Binary records use hand-written parsers with helper functions or sequential-layout macro generation.
4. Core types have zero implicit defaults. New records are created from template functions.
5. `UnknownFields` type is removed entirely. Unknown data lives in the backing store.
6. SchCoord (100k/mil) and PcbCoord (10k/mil) are separate types with `AltiumCoord` trait.
7. Boundary `Measurement<U>` type provides type-safe unit conversions via `From` impls.
8. Hierarchical view types (`SchComponentView`, `SchPinView`, etc.) Deref/DerefMut to record types and provide child navigation via closures and query. DerefMut marks dirty.
9. `ComponentGroup` storage separates component record from children for split-borrow safety. SchLib builds groups per CFB stream; SchDoc groups by OWNERINDEX. PcbLib uses `FootprintGroup`.
10. Query API: `query::<T>(q)` errors on 0 or 2+ matches; `query_all::<T>(q)` returns all. Same AQL works at document level and within parent wrappers for child access. Closures are generic over return type.
11. `QueryHandle` and `ChildHandle` have `with_mut` directly on them. `ChildKey<T>` + `with_child_mut` available for when handles are passed around. Mut-only for now.
12. `Designator` is a single newtype covering both concrete (`U1`) and template (`U?`) forms.
13. E2E tests rebuild from JSON using templates. In-place edit tests cover each record type.
14. diff-ole.py has exit codes, container-level comparison, and both strict/semantic modes.
15. Existing test files still exist but assert functional behavior with high signal.
16. CLI command redesign starts only after core/test/validation gates are complete.
17. AQL parser uses pest. Initial scope: pattern selectors + attribute selectors.
18. `#[altium_enum]` attribute macro generates `AltiumEnum` impl with blanket `ParamCodec`.
19. v1 module hierarchy removed at start. v2 built from scratch, pulling knowledge from v1 as needed.
