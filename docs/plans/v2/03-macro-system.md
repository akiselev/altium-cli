# Phase 2: Macro System

**Agents: 2 parallel tracks (2A, 2B)**
**Blocked by: Phase 1 (all tracks)**

---

## Track 2A: `#[altium_record]` Attribute Macro

**Files:**
- `crates/altium-format-derive/src/lib.rs` (new entry point)
- `crates/altium-format-derive/src/altium_record.rs` (new — main macro logic)
- `crates/altium-format-derive/src/attrs.rs` (rewrite for new attributes)

**Reference: `docs/v2-plan.md` (Macro v3 Design section), existing `record.rs` and `base.rs`**

### What to Build

An **attribute macro** (not derive) that consumes the struct definition and emits a backing-store wrapper with typed getters/setters/updaters.

#### 1. Entry Point

In `lib.rs`, add the new attribute macro alongside (not replacing yet) the old derives:

```rust
#[proc_macro_attribute]
pub fn altium_record(attr: TokenStream, item: TokenStream) -> TokenStream {
    altium_record::expand(attr.into(), item.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
```

#### 2. Attribute Parsing

The macro-level attributes:
```rust
#[altium_record(kind = "sch", record_id = 2, codec = "params")]
#[altium_record(kind = "pcb", object_id = Track, codec = "binary")]
#[altium_record(kind = "pcb", object_id = Pad, codec = "binary",
    parse_fn = "parse_pad", serialize_fn = "serialize_pad")]
```

| Attribute | Required | Values |
|---|---|---|
| `kind` | Yes | `"sch"` or `"pcb"` |
| `record_id` | For sch | Integer — the RECORD param value |
| `object_id` | For pcb | Ident — PcbObjectId variant |
| `codec` | Yes | `"params"` or `"binary"` |
| `parse_fn` | For complex binary | String — custom parse function name |
| `serialize_fn` | For complex binary | String — custom serialize function name |

Field-level attributes:
```rust
#[altium(key = "DESIGNATOR")]          // param key for ParamCodec
#[altium(key = "PINLENGTH")]           // ParamCodec handles {key}_FRAC internally
#[altium(codec_fn = "mask_expansion_codec")]  // escape hatch
#[altium(header)]                       // binary: marks PcbCommonHeader
#[altium(trailing)]                     // binary: marks adaptive trailing fields
#[altium(skip)]                         // skip this field entirely
```

#### 3. Generated Output (Param-Based Records)

Given input:
```rust
#[altium_record(kind = "sch", record_id = 2, codec = "params")]
struct SchPinRecord {
    #[altium(key = "DESIGNATOR")]
    designator: Designator,

    #[altium(key = "PINLENGTH")]
    pin_length: SchCoord,
}
```

Generate:
```rust
pub struct SchPinRecord {
    origin: crate::v2::backing_store::RecordOrigin,
}

impl SchPinRecord {
    // Constructor from origin
    pub fn from_origin(origin: crate::v2::backing_store::RecordOrigin) -> Self {
        Self { origin }
    }

    // Constructor from template function
    pub fn new(origin: crate::v2::backing_store::RecordOrigin) -> Self {
        Self { origin }
    }

    // Getter
    pub fn designator(&self) -> Designator {
        let params = self.origin.param().params;
        <Designator as crate::v2::traits::ParamCodec>::read(&params, "DESIGNATOR")
            .unwrap_or_default()
    }

    // Try-getter (returns Option)
    pub fn try_designator(&self) -> Option<Designator> {
        let params = &self.origin.param().params;
        <Designator as crate::v2::traits::ParamCodec>::read(params, "DESIGNATOR")
    }

    // Setter
    pub fn set_designator(&mut self, value: impl Into<Designator>) {
        let params = &mut self.origin.param_mut().params;
        <Designator as crate::v2::traits::ParamCodec>::write(&value.into(), params, "DESIGNATOR");
    }

    // Updater
    pub fn update_designator<R>(&mut self, f: impl FnOnce(&mut Designator) -> R) -> R {
        let mut value = self.designator();
        let result = f(&mut value);
        self.set_designator(value);
        result
    }

    // Same for pin_length...
    pub fn pin_length(&self) -> SchCoord { ... }
    pub fn set_pin_length(&mut self, value: SchCoord) { ... }
    pub fn update_pin_length<R>(&mut self, f: impl FnOnce(&mut SchCoord) -> R) -> R { ... }
}

// RecordType trait impl
impl crate::v2::traits::RecordType for SchPinRecord {
    const RECORD_ID: u8 = 2;
    fn origin(&self) -> &crate::v2::backing_store::RecordOrigin { &self.origin }
    fn origin_mut(&mut self) -> &mut crate::v2::backing_store::RecordOrigin { &mut self.origin }
}
```

#### 4. Generated Output (Binary Sequential Layout)

Given input:
```rust
#[altium_record(kind = "pcb", object_id = Track, codec = "binary")]
struct PcbTrackRecord {
    #[altium(header)]
    header: PcbCommonHeader,          // 13 bytes

    start_x: PcbCoord,                // 4 bytes at offset 13
    start_y: PcbCoord,                // 4 bytes at offset 17
    end_x: PcbCoord,                  // 4 bytes at offset 21
    end_y: PcbCoord,                  // 4 bytes at offset 25
    width: PcbCoord,                  // 4 bytes at offset 29
}
```

Generate:
```rust
pub struct PcbTrackRecord {
    origin: crate::v2::backing_store::RecordOrigin,
}

impl PcbTrackRecord {
    pub fn start_x(&self) -> PcbCoord {
        crate::v2::binary_helpers::read_pcb_coord(&self.origin.binary().raw_block, 13)
    }
    pub fn set_start_x(&mut self, value: PcbCoord) {
        crate::v2::binary_helpers::write_pcb_coord(&mut self.origin.binary_mut().raw_block, 13, value);
    }
    // ... etc, macro computes offsets from known type sizes
}
```

**Known type sizes for offset computation:**
| Type | Size |
|---|---|
| `u8`, `i8`, `bool` | 1 |
| `u16`, `i16` | 2 |
| `u32`, `i32`, `PcbCoord` | 4 |
| `f64` | 8 |
| `PcbCommonHeader` | 13 |

#### 5. Generated Output (Binary Custom Parser)

For `codec = "binary"` with `parse_fn`/`serialize_fn`:

Generate `const FIELD_*: usize` constants and getters/setters that index into `field_spans`:

```rust
impl PcbPadRecord {
    pub const FIELD_NAME: usize = 0;
    pub const FIELD_POSITION_X: usize = 1;
    // ...

    pub fn position_x(&self) -> PcbCoord {
        let span = &self.origin.binary().field_spans[Self::FIELD_POSITION_X];
        crate::v2::binary_helpers::read_pcb_coord(&self.origin.binary().raw_block, span.offset)
    }
}
```

#### 6. `codec_fn` Escape Hatch

Fields with `#[altium(codec_fn = "name")]` get custom read/write calls:

```rust
// Generated:
pub fn paste_mask(&self) -> MaskExpansion {
    mask_expansion_codec::read(&self.origin.param().params)
}
pub fn set_paste_mask(&mut self, value: MaskExpansion) {
    mask_expansion_codec::write(&value, &mut self.origin.param_mut().params);
}
```

#### 7. Builder Generation

```rust
pub struct SchPinRecordBuilder {
    record: SchPinRecord,
}

impl SchPinRecordBuilder {
    pub fn new(template: fn() -> RecordOrigin) -> Self {
        Self { record: SchPinRecord::new(template()) }
    }
    pub fn designator(mut self, value: impl Into<Designator>) -> Self {
        self.record.set_designator(value);
        self
    }
    pub fn pin_length(mut self, value: SchCoord) -> Self {
        self.record.set_pin_length(value);
        self
    }
    pub fn build(self) -> SchPinRecord {
        self.record
    }
}

impl SchPinRecord {
    pub fn builder(template: fn() -> RecordOrigin) -> SchPinRecordBuilder {
        SchPinRecordBuilder::new(template)
    }
}
```

### Tests

Create a test in `crates/altium-format/src/v2/records/mod.rs` (or a test file) that uses the macro on a simple test record and verifies:
- Getter reads from backing store
- Setter writes to backing store
- Update closure works
- Builder pattern works
- RecordType trait is implemented

### Acceptance Criteria

- [ ] `#[altium_record]` attribute macro generates record types with getters/setters/updaters
- [ ] Param-based codec: calls `ParamCodec::read/write` on the backing store
- [ ] Binary sequential layout: computes offsets, generates direct byte access
- [ ] Binary custom parser: generates `FIELD_*` constants and span-based access
- [ ] `codec_fn` escape hatch works
- [ ] Builder type generated for each record
- [ ] `RecordType` trait impl generated
- [ ] At least one test record compiles and roundtrips correctly
- [ ] `cargo check` passes

---

## Track 2B: `#[altium_enum]` Attribute Macro

**Files:**
- `crates/altium-format-derive/src/lib.rs` (add entry point)
- `crates/altium-format-derive/src/altium_enum_attr.rs` (new — attribute macro version)

**Reference: existing `enum_derive.rs`**

### What to Build

Convert the current `#[derive(AltiumEnum)]` derive macro to an `#[altium_enum]` attribute macro that:

1. Generates `AltiumEnum` trait impl (from_int/to_int)
2. Does NOT generate `Default` impl (no defaults in v2 core)
3. Does NOT generate `FromParamValue`/`ToParamValue` (replaced by blanket `ParamCodec` impl in traits.rs)

```rust
#[proc_macro_attribute]
pub fn altium_enum(attr: TokenStream, item: TokenStream) -> TokenStream { ... }
```

#### Input:
```rust
#[altium_enum]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PinElectricalType {
    Input = 0,
    IO = 1,
    Output = 2,
    OpenCollector = 3,
    Passive = 4,
    HiZ = 5,
    OpenEmitter = 6,
    Power = 7,
}
```

#### Output:
```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PinElectricalType {
    Input = 0,
    IO = 1,
    Output = 2,
    OpenCollector = 3,
    Passive = 4,
    HiZ = 5,
    OpenEmitter = 6,
    Power = 7,
}

impl crate::v2::traits::AltiumEnum for PinElectricalType {
    fn from_int(value: i32) -> Self {
        match value {
            0 => Self::Input,
            1 => Self::IO,
            // ...
            _ => Self::Input, // first variant as fallback
        }
    }
    fn to_int(&self) -> i32 {
        *self as i32
    }
}
```

The struct remains intact (unlike `#[altium_record]` which replaces the struct). The attribute only adds trait impls.

#### Attribute options:
```rust
#[altium_enum(repr = "u8")]           // integer type (default: i32)
#[altium_enum(fallback = "Unknown")]  // specific fallback variant
```

#### Variant attributes:
```rust
#[altium(value = 42)]  // explicit value (overrides discriminant)
```

### Enum Types to Migrate

All existing enums from `_v2_reference/types.rs` need to be recreated with the new `#[altium_enum]` macro in Phase 3. This track just builds the macro; Phase 3 applies it to all enum types.

### Tests

- `enum_from_int()` — known values map correctly
- `enum_to_int()` — variants produce correct integers
- `enum_fallback()` — unknown values use fallback
- `enum_param_codec()` — blanket impl works via ParamCodec trait

### Acceptance Criteria

- [ ] `#[altium_enum]` attribute macro generates `AltiumEnum` impl
- [ ] No `Default` impl generated (v2 principle: no defaults in core)
- [ ] Blanket `ParamCodec` impl (from Track 1C) works with generated `AltiumEnum`
- [ ] Custom fallback variant supported
- [ ] `cargo check` passes
