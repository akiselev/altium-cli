# altium-format v2: Clean-Slate Architecture

## Intent

Design from scratch for one goal: **lossless, nondestructive editing with an ergonomic imperative API**.

This document intentionally ignores migration constraints. Existing code is treated as a knowledge base and oracle; the new architecture is allowed to break old APIs.

### Why Not FCIS?

The original draft proposed a Functional Core / Imperative Shell architecture. After analysis, FCIS is the wrong organizing principle for a **library**:

- FCIS was designed for **applications** with a main loop that orchestrates side effects. For a library, the consumer IS the shell — there is no main loop to own.
- The "functional" parts (immutable snapshots, produce-new-state) add complexity without benefit. File editing is open → edit → save, not event-sourced state management.
- The concrete benefits we wanted from FCIS (testability, determinism, I/O separation) fall out naturally from good Rust library design without needing FCIS as an architecture.

**What we keep: four design rules that FCIS inspired:**

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
5. **Macro-first generation**: the macro generates record types, getters/setters, lens types (with dirty tracking and validation), and builder APIs. Param types handle their own serialization via `ParamCodec` (single key). Binary records use hand-written parsers with helper functions.
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

### Write Rules

On save, for each record:

- **Unchanged**: backing store bytes match the original snapshot → write the original bytes verbatim. Byte-identical output.
- **Changed**: backing store was mutated by setters → serialize the backing store to bytes.

This applies identically regardless of whether the backing store was loaded from an Altium file or created from a template. There is no separate "rebuild" or "re-serialize" path. Setters always update in place.

### DocumentCore as Struct Member

`DocumentCore` is a **struct**, not a trait. Each document type contains it as a member:

```rust
pub struct SchLib {
    core: DocumentCore,
    section_keys: HashMap<String, String>,
    // SchLib-specific metadata
}

pub struct PcbLib {
    core: DocumentCore,
    // PcbLib-specific metadata
}

pub struct SchDoc {
    core: DocumentCore,
    // SchDoc-specific metadata
}

pub struct PcbDoc {
    core: DocumentCore,
    // PcbDoc-specific metadata (typed stream map, etc.)
}
```

This allows generic operations on `DocumentCore` (iteration, save, test fixture construction) while each document type adds its own semantic layer. Separate types per format because they are semantically different:

- **SchLib**: components containing primitives (tree structure)
- **SchDoc**: flat primitives with OWNERINDEX links (flat + implicit tree)
- **PcbLib**: components containing binary records with multi-block structures
- **PcbDoc**: flat binary records in typed streams (Tracks6, Arcs6, etc.)

## API Shape: Imperative Access with Backing Store

### Two Layers: Record Types and Document Lens

There are two distinct layers:

1. **Record types** (`SchPin`, `SchComponent`, etc.) — own a `RecordOrigin` and have typed getters/setters directly on them. These are the primary API surface. No lifetimes, no lens indirection.
2. **Lens types** (`SchPinRef<'a>`, `SchPinMut<'a>`) — used **only** by the document's closure-based edit API. Deref to the record type so the user calls the same getters/setters, but also provide dirty tracking, drop validation, panic rollback, and read access to sibling records.

The user interacts with the record type's methods. The lens is plumbing.

### Macro Declaration (Source of Truth)

Fields exist in the source code for documentation and IDE autocomplete. The macro reads them but **removes them from the runtime struct**, replacing the struct with a backing-store wrapper. The macro generates typed getters/setters directly on the record type.

Field types should be **domain newtypes**, not raw primitives. A `Designator` is not a `String` — it has structure (prefix + number), helper methods, and validation. Newtypes let the macro generate `update_*` closures that give `&mut` access to the parsed value for in-place modification, avoiding a getter-modify-setter round trip.

The macro passes param key names to the type's `ParamCodec` trait implementation. **Types handle their own serialization** — the macro just orchestrates. This means `SchCoord` knows how to read/write its integer+frac pair, `Designator` knows how to read/write its string, etc. The macro doesn't need special-case logic for `frac`, `bitflags`, or any other encoding detail.

```rust
#[derive(AltiumEntity)]
#[altium(kind = "sch", record_id = 2, codec = "params")]
struct SchPin {
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
pub struct SchPin {
    origin: RecordOrigin,
}

impl SchPin {
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

### Generated Lens Types (Document Closure API Only)

Lens types are wrappers generated by the macro that borrow into the document's storage. They deref to the record type so the same getters/setters are available, but they also provide features that a bare `&mut SchPin` can't:

```rust
// Generated by macro — used only by the document closure API:
pub struct SchPinMut<'a> {
    record: &'a mut RecordNode,      // mutably borrows the target record
    core: &'a DocumentCore,          // read access to sibling records
    dirty: bool,                     // tracks whether any setter was called
    snapshot: Vec<u8>,               // original bytes for rollback on panic
}

impl<'a> Deref for SchPinMut<'a> { type Target = SchPin; }
impl<'a> DerefMut for SchPinMut<'a> { ... }

impl<'a> Drop for SchPinMut<'a> {
    fn drop(&mut self) {
        if self.dirty {
            self.record.mark_dirty();
            // Run validation — warn or panic if invariants are violated
        }
        // If panicking, roll back to snapshot
        if std::thread::panicking() {
            self.record.restore_from(&self.snapshot);
        }
    }
}
```

**Lens features beyond deref:**

| Feature | What | Why a bare `&mut SchPin` can't do it |
|---|---|---|
| **Dirty tracking** | Marks the record dirty on drop if any setter was called | The document needs to know what changed for efficient save — bare ref doesn't report back |
| **Sibling access** | Read-only access to other records in the document via `&DocumentCore` | A bare `&mut SchPin` only sees itself — can't read a related component to check constraints |
| **Drop validation** | Validates record invariants when the closure returns | Catches constraint violations early instead of at save time |
| **Panic rollback** | Restores original bytes if the closure panics | Prevents half-mutated records from corrupting the document |

```rust
impl<'a> SchPinMut<'a> {
    /// Read a sibling record without taking a mutable borrow on it.
    pub fn read_component(&self, id: RecordKey) -> SchComponentRef<'_> { ... }

    /// Check if any setter has been called.
    pub fn is_dirty(&self) -> bool { self.dirty }
}
```

`SchPinRef<'a>` is the read-only counterpart — holds `&'a RecordNode` and `&'a DocumentCore`, derefs to `SchPin` for getters only, no dirty tracking needed.

### Document Access via Closures

The closure API provides lens-wrapped access. Because the lens derefs to the record type, the user calls getters/setters on the record type directly:

```rust
// Edit existing record
doc.with_pin(pin_id, |pin| {
    pin.set_designator(Designator::new("A1"));
    pin.set_pin_length(SchCoord::from_mils(100.0));
    pin.update_pin_conglomerate(|flags| {
        flags.set(PinConglomerateFlags::DISPLAY_NAME_VISIBLE, true);
    });
});

// Create new record — same closure pattern, template is selected internally
doc.insert_pin(component_id, |pin| {
    pin.set_designator(Designator::new("A1"));
    pin.set_pin_length(SchCoord::from_mils(100.0));
});

// Save — writes original bytes for unchanged records, serializes backing store for changed ones
doc.save(path)?;
```

The `with_pin` closure receives a `SchPinMut<'_>` lens. Because the lens derefs to `SchPin`, the user calls the same getters/setters as on a standalone record. But the lens also provides dirty tracking (marks the record on drop if any setter was called), panic rollback (restores original bytes), drop validation, and read access to sibling records. There is no separate `with_pin_mut` — all closures provide mutable access. Read-only access is just calling getters inside the closure.

The `insert_pin` method creates a new record from a template function, gives the closure a lens to configure it, then inserts it into the document. Same closure shape, same setters — the user doesn't need to know whether they're editing or creating.

This is the **first-pass API**. An owned/checkout API for external consumers can be added later.

### Standalone Record Usage (Without a Document)

Because getters/setters live on the record type, records work standalone too:

```rust
// Create from template function
let mut pin = SchPin::new(templates::sch_pin_default());
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
impl From<&SchPin> for SchPinDto {
    fn from(pin: &SchPin) -> Self {
        Self {
            // Context default applied HERE, not in SchPin
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
let pin = SchPin::builder(templates::sch_pin_default)
    .designator(Designator::new("A1"))
    .pin_length(SchCoord::from_mils(100.0))
    .build();

// Desugars to:
let mut pin = SchPin::new(templates::sch_pin_default());
pin.set_designator(Designator::new("A1"));
pin.set_pin_length(SchCoord::from_mils(100.0));
```

The builder takes a template function, not a file path. The macro generates the builder type with the same typed setters as the record type. The builder uses the exact same backing store and setters — there is no separate code path.

## Macro v3 Design

### Goals

1. Remove fields from runtime struct; generate record type with typed getters/setters/updaters.
2. Generate lens types for document closure API (with dirty tracking, validation, and sibling access).
3. Delegate param serialization to types via `ParamCodec` trait — the macro passes a single key per field.
4. Binary records: macro generates getters/setters over field span map; parse/serialize is hand-written per record type.
5. No `default` support in core — defaults come from template functions only.
6. Generate `Builder` type that wraps template function + same setters.
7. For types that don't fit `ParamCodec`, support overriding with `codec_fn = "custom_fn"`.
8. Generate test helpers and `Arbitrary` impls.

### Generated Pieces

From a single `#[derive(AltiumEntity)]` annotation:

- **Record type** (`SchPin`) wrapping `RecordOrigin`, with typed getters/setters/updaters.
- **Lens types** (`SchPinRef<'a>`, `SchPinMut<'a>`) for document closure API. Deref to the record type for getters/setters, but also provide dirty tracking, drop validation, and sibling access.
- **Builder type** (`SchPinBuilder`) — takes a template function, applies typed overrides.
- **Test helpers** (`SchPin::test_fixture()`, `SchPin::assert_roundtrip_identity()`).
- **`Arbitrary` impl** (behind `#[cfg(test)]`) for property-based testing.

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

2. **Cross-record operations**: When an operation needs to read record A while mutating record B (e.g., connecting a wire to a pin), the lens already provides read access to siblings via `&DocumentCore`. But if both records need mutation, options: (a) interior mutability for the second record, (b) explicit multi-record closure API, (c) separate mutations in sequence.

3. **PadLayerStack and similar complex wrappers**: How many of these do we need? Each one is hand-written domain logic. Need an inventory of all complex binary field types across all record types.

4. **Template function coverage**: We need template functions for every record type we want to create. Default values must be extracted from real Altium files. May need a template extraction tool to bootstrap these.

5. **OLE container compatibility**: What sector size, mini-stream cutoff, and directory entry ordering does Altium use? This determines whether we can achieve byte-identical CFB output.

6. **insert vs. update semantics**: `with_pin` and `insert_pin` use the same closure shape. Should we unify them further, or is the semantic distinction (edit existing vs. create new) worth keeping in the API?

## Definition of Done

1. All record types use backing-store access — no runtime typed fields.
2. All getters/setters use proper domain newtypes, coordinates, enums, and bitflags types.
3. Param types handle their own serialization via `ParamCodec` trait (single key). Binary records use hand-written parsers with helper functions.
4. Core types have zero implicit defaults. New records are created from template functions.
5. `UnknownFields` type is removed entirely. Unknown data lives in the backing store.
6. SchCoord (100k/mil) and PcbCoord (10k/mil) are separate types with `AltiumCoord` trait.
7. Boundary `Measurement<U>` type provides type-safe unit conversions via `From` impls.
8. Lens types provide dirty tracking, drop validation, panic rollback, and sibling read access.
9. `Designator` (concrete, SchDoc) and `DesignatorTemplate` (placeholder, SchLib) are separate newtypes.
10. E2E tests rebuild from JSON using templates. In-place edit tests cover each record type.
11. diff-ole.py has exit codes, container-level comparison, and both strict/semantic modes.
12. Existing test files still exist but assert functional behavior with high signal.
13. CLI command redesign starts only after core/test/validation gates are complete.
