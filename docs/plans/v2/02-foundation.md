# Phase 1: Foundation Types

**Agents: 6 parallel tracks (1A through 1F)**
**Blocked by: Phase 0**

All 6 tracks can execute simultaneously. They have no dependencies on each other.

---

## Track 1A: Coordinate System

**File: `crates/altium-format/src/v2/coord.rs`**
**Reference: `_v2_reference/coord.rs`, `_v2_reference/pcb/coord.rs`**

### What to Build

1. **`AltiumCoord` trait** — shared interface for coordinate types:
   ```rust
   pub trait AltiumCoord: Copy + Sized + PartialEq + Eq + PartialOrd + Ord + std::fmt::Debug {
       const UNITS_PER_MIL: i32;
       fn from_raw(raw: i32) -> Self;
       fn to_raw(self) -> i32;
       fn from_mils(mils: f64) -> Self {
           Self::from_raw((mils * Self::UNITS_PER_MIL as f64) as i32)
       }
       fn to_mils(self) -> f64 {
           self.to_raw() as f64 / Self::UNITS_PER_MIL as f64
       }
       fn from_mm(mm: f64) -> Self { Self::from_mils(mm / 0.0254) }
       fn to_mm(self) -> f64 { self.to_mils() * 0.0254 }
   }
   ```

2. **`SchCoord`** (100,000 units/mil):
   ```rust
   #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
   pub struct SchCoord(pub(crate) i32);
   ```
   - Implement `AltiumCoord` with `UNITS_PER_MIL = 100_000`
   - Add `to_dxp_parts()`, `from_dxp_parts()`, `to_binary_parts()`, `from_binary_parts()`
   - Implement `Add`, `Sub`, `Neg`, `Mul<i32>`, `Div<i32>` via `impl_coord_ops!` macro
   - Implement `Serialize`, `Deserialize` (as raw i32)

3. **`PcbCoord`** (10,000 units/mil):
   ```rust
   #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
   pub struct PcbCoord(pub(crate) i32);
   ```
   - Implement `AltiumCoord` with `UNITS_PER_MIL = 10_000`
   - Same arithmetic ops via `impl_coord_ops!` macro

4. **`impl_coord_ops!` declarative macro** for arithmetic boilerplate on both types.

5. **Generic `Point<C>` and `Rect<C>`**:
   ```rust
   #[derive(Copy, Clone, Debug, PartialEq, Eq)]
   pub struct Point<C: AltiumCoord> { pub x: C, pub y: C }
   pub type SchPoint = Point<SchCoord>;
   pub type PcbPoint = Point<PcbCoord>;

   #[derive(Copy, Clone, Debug, PartialEq, Eq)]
   pub struct Rect<C: AltiumCoord> { pub min: Point<C>, pub max: Point<C> }
   pub type SchRect = Rect<SchCoord>;
   pub type PcbRect = Rect<PcbCoord>;
   ```
   - `Rect` methods: `width()`, `height()`, `center()`, `contains()`, `intersects()`

6. **Boundary `Measurement<U>` type** (for CLI/DTO layer):
   ```rust
   pub trait Unit { const MILS_PER_UNIT: f64; const ABBREVIATION: &'static str; }
   pub struct Millimeters;
   pub struct Mils;
   pub struct Inches;
   pub struct Measurement<U: Unit>(pub f64, PhantomData<U>);
   pub type Mm = Measurement<Millimeters>;
   pub type Mil = Measurement<Mils>;
   ```
   - `From<Measurement<U>>` impls for both `SchCoord` and `PcbCoord`

### Tests

- `coord_from_mils()` — verify mil→raw conversion for both SchCoord and PcbCoord
- `coord_round_trip_mils()` — round-trip accuracy within tolerance
- `sch_coord_dxp_parts()` — verify DXP binary split
- `sch_coord_binary_parts()` — verify binary format split
- `point_arithmetic()` — basic Point operations
- `rect_contains()` — rectangle containment
- `measurement_conversion()` — Mm/Mil → SchCoord/PcbCoord

### Acceptance Criteria

- [ ] `SchCoord` and `PcbCoord` are separate newtypes with `AltiumCoord` trait
- [ ] All arithmetic ops work on both coord types
- [ ] `Point<C>` and `Rect<C>` are generic over coord type
- [ ] `Measurement<U>` converts to both coord types
- [ ] All unit tests pass
- [ ] `cargo check` passes

---

## Track 1B: Backing Store Types

**File: `crates/altium-format/src/v2/backing_store.rs`**
**Reference: `docs/v2-plan.md` (Core Data Model section)**

### What to Build

1. **`RecordOrigin` enum**:
   ```rust
   pub enum RecordOrigin {
       Param(ParamOrigin),
       Binary(BinaryOrigin),
   }
   ```

2. **`ParamOrigin`**:
   ```rust
   pub struct ParamOrigin {
       pub params: ParameterCollection,
       pub raw_record_text: String,
   }
   ```

3. **`BinaryOrigin`**:
   ```rust
   pub struct BinaryOrigin {
       pub raw_block: Vec<u8>,
       pub field_spans: Vec<FieldSpan>,
   }
   ```

4. **`FieldSpan`**:
   ```rust
   #[derive(Clone, Debug)]
   pub struct FieldSpan {
       pub offset: usize,
       pub size: usize,
   }
   ```

5. **`RecordNode`**:
   ```rust
   pub struct RecordNode {
       pub key: u8,
       pub origin: RecordOrigin,
       pub original_snapshot: Vec<u8>,
       pub dirty: bool,
   }
   ```
   - `new(origin: RecordOrigin)` — creates node from origin, snapshots current bytes
   - `mark_dirty()` — sets dirty flag
   - `is_dirty()` — checks if backing store differs from snapshot
   - `snapshot_bytes()` → `&[u8]` — returns original snapshot for identity writes

6. **`ComponentGroup`**:
   ```rust
   pub struct ComponentGroup {
       pub component: RecordNode,
       pub children: Vec<RecordNode>,
       pub original_indices: Vec<usize>,
   }
   ```

7. **`FootprintGroup`** (for PcbLib):
   ```rust
   pub struct FootprintGroup {
       pub metadata: RecordNode,
       pub primitives: Vec<RecordNode>,
       pub raw_pattern_name_block: Vec<u8>,
       pub original_primitive_order: Vec<PcbPrimitiveRef>,
       pub raw_header: Vec<u8>,
   }

   pub struct PcbPrimitiveRef {
       pub type_id: u8,
       pub index: usize,
   }
   ```

8. **`StreamId`** and **`StreamNode`** (lower-level building blocks):
   ```rust
   pub type StreamId = String;

   pub struct StreamNode {
       pub id: StreamId,
       pub original_bytes: Vec<u8>,
       pub records: Vec<RecordNode>,
   }
   ```

### Helper Methods on RecordOrigin

```rust
impl RecordOrigin {
    pub fn as_param(&self) -> Option<&ParamOrigin> { ... }
    pub fn as_param_mut(&mut self) -> Option<&mut ParamOrigin> { ... }
    pub fn as_binary(&self) -> Option<&BinaryOrigin> { ... }
    pub fn as_binary_mut(&mut self) -> Option<&mut BinaryOrigin> { ... }
    pub fn param(&self) -> &ParamOrigin { self.as_param().expect("expected param origin") }
    pub fn param_mut(&mut self) -> &mut ParamOrigin { ... }
    pub fn binary(&self) -> &BinaryOrigin { ... }
    pub fn binary_mut(&mut self) -> &mut BinaryOrigin { ... }
}
```

### Tests

- `record_node_dirty_tracking()` — verify dirty flag behavior
- `param_origin_access()` — read/write through ParamOrigin
- `binary_origin_field_span()` — read from field span map
- `component_group_split_borrow()` — verify component + children can be borrowed independently

### Acceptance Criteria

- [ ] All backing store types compile and have basic tests
- [ ] `RecordNode` tracks dirty state correctly
- [ ] `ComponentGroup` allows split borrows (component vs children)
- [ ] `cargo check` passes

---

## Track 1C: ParamCodec Trait & Primitive Impls

**File: `crates/altium-format/src/v2/traits.rs`**
**Reference: `docs/v2-plan.md` (Serialization Traits section)**

### What to Build

1. **`ParamCodec` trait**:
   ```rust
   pub trait ParamCodec: Sized {
       fn read(params: &ParameterCollection, key: &str) -> Option<Self>;
       fn write(&self, params: &mut ParameterCollection, key: &str);
   }
   ```

2. **`AltiumEnum` trait**:
   ```rust
   pub trait AltiumEnum: Sized {
       fn from_int(value: i32) -> Self;
       fn to_int(&self) -> i32;
   }
   ```

3. **Blanket `ParamCodec` impl for `AltiumEnum`**:
   ```rust
   impl<T: AltiumEnum> ParamCodec for T {
       fn read(params: &ParameterCollection, key: &str) -> Option<Self> {
           params.get(key).map(|v| Self::from_int(v.as_int_or(0)))
       }
       fn write(&self, params: &mut ParameterCollection, key: &str) {
           params.add_int(key, self.to_int());
       }
   }
   ```

4. **ParamCodec impls for primitives**:
   - `String` — read as string, write as string
   - `i32` — read as int, write as int
   - `i16` — read as int cast to i16
   - `u8` — read as int cast to u8
   - `bool` — read as `T`/`F`, write as `T`/`F`
   - `f64` — read as double, write as double
   - `u32` — read as int (for color values etc.)
   - `Option<T: ParamCodec>` — returns None if key missing
   - `SchCoord` — handles `{key}` int + `{key}_FRAC` composite
   - `PcbCoord` — single i32 value

5. **`RecordType` trait** (marker for macro-generated record types):
   ```rust
   pub trait RecordType {
       const RECORD_ID: u8;
       fn origin(&self) -> &RecordOrigin;
       fn origin_mut(&mut self) -> &mut RecordOrigin;
   }
   ```

6. **`WrapperFamily` trait** (for query type parameters):
   ```rust
   pub trait WrapperFamily {
       type Record: RecordType;
       type View<'a>;
       fn record_id() -> u8 { Self::Record::RECORD_ID }
   }
   ```

### Tests

- `param_codec_string()` — String read/write roundtrip
- `param_codec_int()` — i32 read/write roundtrip
- `param_codec_bool()` — bool read/write roundtrip
- `param_codec_sch_coord()` — SchCoord with FRAC key
- `param_codec_option()` — Option<T> missing key behavior

### Acceptance Criteria

- [ ] `ParamCodec` trait defined with primitive impls
- [ ] `AltiumEnum` trait with blanket `ParamCodec` impl
- [ ] `RecordType` and `WrapperFamily` traits defined
- [ ] All primitive ParamCodec impls have roundtrip tests
- [ ] `cargo check` passes

---

## Track 1D: Domain Newtypes

**File: `crates/altium-format/src/v2/newtypes.rs`**
**Reference: `docs/v2-plan.md` (Domain Newtypes section)**

### What to Build

1. **`Designator`** — the most feature-rich newtype:
   ```rust
   #[derive(Clone, Debug, PartialEq, Eq, Hash)]
   pub struct Designator(String);

   impl Designator {
       pub fn new(s: impl Into<String>) -> Self;
       pub fn as_str(&self) -> &str;
       pub fn prefix(&self) -> &str;
       pub fn is_template(&self) -> bool;
       pub fn number(&self) -> Option<u32>;
       pub fn set_number(&mut self, n: u32);
       pub fn increment(&mut self);
       pub fn resolve(&self, n: u32) -> Designator;
   }
   ```
   - `Deref<Target=str>`, `Display`, `From<&str>`, `From<String>`
   - `ParamCodec` impl (single key, string value)

2. **`LibReference`**:
   ```rust
   pub struct LibReference(String);
   impl LibReference {
       pub fn normalize(&self) -> String;
       pub fn matches_pattern(&self, pattern: &str) -> bool;
   }
   ```

3. **`NetName`**:
   ```rust
   pub struct NetName(String);
   impl NetName {
       pub fn is_power_net(&self) -> bool;
       pub fn prefix(&self) -> &str;
   }
   ```

4. **`UniqueId`**:
   ```rust
   pub struct UniqueId(String);
   impl UniqueId {
       pub fn generate() -> Self;
       pub fn is_valid(&self) -> bool;
   }
   ```

5. **`Description`** — thin wrapper:
   ```rust
   pub struct Description(String);
   ```

6. **`PinName`**:
   ```rust
   pub struct PinName(String);
   impl PinName {
       pub fn is_inverted(&self) -> bool;
       pub fn display_text(&self) -> String;
   }
   ```

7. **Helper macro** `impl_string_newtype!` for the common boilerplate:
   ```rust
   macro_rules! impl_string_newtype {
       ($name:ident) => {
           impl std::ops::Deref for $name {
               type Target = str;
               fn deref(&self) -> &str { &self.0 }
           }
           impl std::fmt::Display for $name {
               fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                   f.write_str(&self.0)
               }
           }
           impl From<&str> for $name {
               fn from(s: &str) -> Self { Self(s.to_string()) }
           }
           impl From<String> for $name {
               fn from(s: String) -> Self { Self(s) }
           }
           impl ParamCodec for $name {
               fn read(params: &ParameterCollection, key: &str) -> Option<Self> {
                   params.get(key).map(|v| Self(v.as_str().to_string()))
               }
               fn write(&self, params: &mut ParameterCollection, key: &str) {
                   params.add(key, &self.0);
               }
           }
       };
   }
   ```

### Tests

- `designator_prefix()` — "R1" → "R", "U10" → "U"
- `designator_number()` — "R1" → Some(1), "R?" → None
- `designator_is_template()` — "U?" → true, "U1" → false
- `designator_increment()` — "R1" → "R2"
- `designator_resolve()` — "U?" + 3 → "U3"
- `pin_name_inverted()` — "~RESET" → inverted
- `unique_id_generate()` — generates valid ID
- `param_codec_designator()` — roundtrip through ParameterCollection

### Acceptance Criteria

- [ ] All 6 newtypes created with `Deref<Target=str>`, `Display`, `From`, `ParamCodec`
- [ ] `Designator` has all domain methods
- [ ] `PinName` handles overbar syntax
- [ ] All newtypes have unit tests
- [ ] `cargo check` passes

---

## Track 1E: Binary Helpers

**File: `crates/altium-format/src/v2/binary_helpers.rs`**
**Reference: `_v2_reference/pcb/primitive.rs`, `io/reader.rs`, `io/writer.rs`**

### What to Build

Common binary read/write functions used by hand-written PCB record parsers:

```rust
pub fn read_i8(data: &[u8], offset: usize) -> i8;
pub fn read_u8(data: &[u8], offset: usize) -> u8;
pub fn read_i16_le(data: &[u8], offset: usize) -> i16;
pub fn read_u16_le(data: &[u8], offset: usize) -> u16;
pub fn read_i32_le(data: &[u8], offset: usize) -> i32;
pub fn read_u32_le(data: &[u8], offset: usize) -> u32;
pub fn read_f64_le(data: &[u8], offset: usize) -> f64;
pub fn read_bool(data: &[u8], offset: usize) -> bool;

pub fn write_i8(data: &mut [u8], offset: usize, value: i8);
pub fn write_u8(data: &mut [u8], offset: usize, value: u8);
pub fn write_i16_le(data: &mut [u8], offset: usize, value: i16);
pub fn write_u16_le(data: &mut [u8], offset: usize, value: u16);
pub fn write_i32_le(data: &mut [u8], offset: usize, value: i32);
pub fn write_u32_le(data: &mut [u8], offset: usize, value: u32);
pub fn write_f64_le(data: &mut [u8], offset: usize, value: f64);
pub fn write_bool(data: &mut [u8], offset: usize, value: bool);

pub fn read_pcb_coord(data: &[u8], offset: usize) -> PcbCoord;
pub fn write_pcb_coord(data: &mut [u8], offset: usize, value: PcbCoord);

pub fn read_pascal_string(data: &[u8], offset: usize) -> (&str, usize); // returns (str, bytes_consumed)
pub fn write_pascal_string(data: &mut Vec<u8>, offset: usize, s: &str) -> usize;

/// PcbCommonHeader: 13-byte binary header shared by all PCB primitives
#[derive(Clone, Debug, PartialEq)]
pub struct PcbCommonHeader {
    pub layer: u8,
    pub flags: u16,
    pub net: u16,
    pub polygon_ref: u16,
    pub component_ref: u16,
    pub ref4: u16,
    pub ref5: u16,
}

impl PcbCommonHeader {
    pub const SIZE: usize = 13;
    pub fn read(data: &[u8], offset: usize) -> Self;
    pub fn write(&self, data: &mut [u8], offset: usize);
}
```

### Tests

- `read_write_i32()` — roundtrip
- `read_write_f64()` — roundtrip
- `read_write_pcb_coord()` — roundtrip
- `pcb_common_header_roundtrip()` — 13-byte header
- `pascal_string_roundtrip()` — string with length prefix

### Acceptance Criteria

- [ ] All helper functions implemented
- [ ] `PcbCommonHeader` struct with 13-byte read/write
- [ ] All functions have roundtrip tests
- [ ] `cargo check` passes

---

## Track 1F: Error Types Update

**File: `crates/altium-format/src/error.rs`**

### What to Build

Update the error type to support v2 query errors and remove any dependencies on v1 types:

```rust
#[derive(Debug, thiserror::Error)]
pub enum AltiumError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("invalid coordinate: {0}")]
    InvalidCoordinate(String),

    #[error("invalid record: {0}")]
    InvalidRecord(String),

    #[error("missing data: {0}")]
    MissingData(String),

    #[error("decompression error: {0}")]
    Decompression(String),

    #[error("encoding error: {0}")]
    Encoding(String),

    #[error("unexpected EOF")]
    UnexpectedEof,

    #[error("parse error: {0}")]
    Parse(String),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("template error: {0}")]
    Template(String),

    // NEW for v2:
    #[error("query error: {0}")]
    Query(String),

    #[error("no match found: {0}")]
    NoMatch(String),

    #[error("ambiguous match: {0} matches found for query '{1}'")]
    AmbiguousMatch(usize, String),

    #[error("CFB error: {0}")]
    Cfb(String),
}
```

Ensure no imports reference removed v1 modules.

### Acceptance Criteria

- [ ] Error type compiles without v1 dependencies
- [ ] New query/match error variants added
- [ ] `cargo check` passes
