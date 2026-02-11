# Serialization

The `altium-format-derive` crate provides procedural macros that generate
serialization code, mapping between Altium's file format and Rust structs. This
document explains the derive macros, the traits they implement, and the field
attribute syntax.

## Derive Macros

Three derive macros are exported from `altium-format-derive`:

| Macro | Generates | Used on |
|-------|-----------|---------|
| `AltiumRecord` | `FromParams`/`ToParams` and/or `FromBinary`/`ToBinary` | Record structs |
| `AltiumBase` | Composition trait (`HasXxxBase`) + `FromParams`/`ToParams` | Base types |
| `AltiumEnum` | Integer-to-enum conversion + `FromParamValue`/`ToParamValue` | Enums |

## Traits

### Parameter-Based (Schematic)

```rust
// crates/altium-format/src/traits/params.rs

pub trait FromParams: Sized {
    /// Deserialize from a parameter collection.
    fn from_params(params: &ParameterCollection) -> Result<Self>;

    /// Deserialize, returning both the struct and any unknown parameters
    /// (for round-trip preservation).
    fn from_params_preserving(params: &ParameterCollection)
        -> Result<(Self, UnknownFields)>;
}

pub trait ToParams {
    /// Serialize to a new parameter collection.
    fn to_params(&self) -> ParameterCollection;

    /// Append this struct's parameters to an existing collection.
    fn append_to_params(&self, params: &mut ParameterCollection);
}
```

### Binary-Based (PCB)

```rust
// crates/altium-format/src/traits/binary.rs

pub trait FromBinary: Sized {
    /// Read from a binary stream.
    fn read_from<R: Read>(reader: &mut R) -> Result<Self>;

    /// Read from a binary stream, preserving any trailing unknown bytes.
    fn read_from_preserving<R: Read>(reader: &mut R, block_size: usize)
        -> Result<(Self, Vec<u8>)>;
}

pub trait ToBinary {
    /// Write to a binary stream.
    fn write_to<W: Write>(&self, writer: &mut W) -> Result<()>;

    /// Compute the serialized size in bytes.
    fn binary_size(&self) -> usize;
}
```

### Individual Value Conversion

```rust
// crates/altium-format/src/traits/conversion.rs

pub trait FromParamValue: Sized {
    /// Parse a single parameter string value into a typed value.
    fn from_param_value(value: &ParameterValue) -> Result<Self>;
    fn default_value() -> Option<Self>;
}

pub trait ToParamValue {
    /// Serialize a typed value to a parameter string.
    fn to_param_value(&self) -> String;
    fn should_skip_default(&self) -> bool;
}
```

Implemented for: `i32`, `f64`, `bool`, `String`, `Coord`, `Color`, and all
`AltiumEnum` types.

### Polymorphic Traits

```rust
pub trait SchPrimitive: AltiumRecord + FromParams + ToParams {
    const RECORD_ID: i32;
    fn owner_index(&self) -> i32;
    fn set_owner_index(&mut self, index: i32);
    fn calculate_bounds(&self) -> CoordRect;
    fn record_type_name(&self) -> &'static str;
}

pub trait PcbPrimitive: AltiumRecord + FromBinary + ToBinary {
    const OBJECT_ID: PcbObjectId;
    fn layer(&self) -> Layer;
    fn calculate_bounds(&self) -> CoordRect;
}
```

---

## AltiumRecord Derive

### Container Attributes

```rust
#[derive(AltiumRecord)]
#[altium(record_id = 2, format = "params")]  // Schematic record
pub struct SchPin { … }

#[derive(AltiumRecord)]
#[altium(format = "binary")]                  // PCB record
pub struct PcbTrack { … }
```

| Attribute | Values | Meaning |
|-----------|--------|---------|
| `record_id = N` | Integer | Schematic `RECORD` value written on serialize |
| `format = "…"` | `"params"`, `"binary"`, `"both"` | Which traits to generate |

### Field Attributes — Parameter Format

#### Basic field mapping

```rust
#[altium(param = "LIBREFERENCE")]
pub lib_reference: String,
```

Maps the struct field to the parameter key `LIBREFERENCE`. On read, looks up
the key in the `ParameterCollection` and calls `FromParamValue`. On write,
converts the field value to a string and adds it to the collection.

#### Default values

```rust
#[altium(param = "ELECTRICAL", default)]
pub electrical: PinElectricalType,
```

If the parameter is missing, uses `Default::default()` instead of returning an
error. Can also provide a specific default: `default = some_expr`.

#### Optional fields

```rust
#[altium(param = "OWNERPARTID", optional)]
pub owner_part_id: Option<i32>,
```

Wraps the field in `Option<T>`. Missing parameter → `None`.

#### Skip default on write

```rust
#[altium(param = "TEXT", default, skip_default)]
pub text: String,
```

When serializing, omits the parameter if the value equals `Default::default()`.
Reduces output size for common cases (e.g., empty strings, zero values).

#### Fractional coordinates

```rust
#[altium(param = "LOCATION.X", frac = "LOCATION.X_FRAC")]
pub location_x: i32,
```

Reads two parameters (integer + fractional) and combines them into a single
raw coordinate value. See [Coordinate System](coordinates.md) for the encoding.

#### Indexed vertex arrays

```rust
#[altium(indexed_coords, prefix_x = "X", prefix_y = "Y", count = "LOCATIONCOUNT")]
pub vertices: Vec<(i32, i32)>,
```

Reads `LOCATIONCOUNT` to determine the number of vertices, then reads
`X1`/`X1_FRAC`/`Y1`/`Y1_FRAC`, `X2`/`X2_FRAC`/`Y2`/`Y2_FRAC`, etc.
Indices are 1-based.

#### Flatten (composition)

```rust
#[altium(flatten)]
pub graphical: SchGraphicalBase,
```

Recursively reads/writes the nested struct's fields as top-level parameters.
This is how base types are composed into concrete record types — all of
`SchGraphicalBase`'s fields (and transitively `SchPrimitiveBase`'s fields)
appear as top-level keys in the parameter string.

#### Color fields

```rust
#[altium(color)]
pub color: Color,
```

Reads/writes a Win32 COLORREF value (i32 in `0x00BBGGRR` format).

#### List fields

```rust
#[altium(list)]
pub items: Vec<String>,
```

Uses `FromParamList`/`ToParamList` for comma-separated values.

#### Unknown field preservation

```rust
#[altium(unknown)]
pub unknown_params: UnknownFields,
```

After reading all known fields, captures any remaining parameters that weren't
mapped to a field. On write, these are appended back to the parameter
collection. This enables non-destructive round-tripping.

#### Skip

```rust
#[altium(skip)]
pub cached_bounds: CoordRect,
```

Ignored during serialization. Initialized to `Default::default()`.

### Field Attributes — Binary Format

#### Basic binary types

```rust
#[altium(binary, ty = "i32le")]
pub value: i32,
```

Supported types: `i8`, `u8`, `i16le`, `u16le`, `i32le`, `u32le`, `i64le`,
`u64le`, `f32le`, `f64le`, `bool`.

#### Coordinate point

```rust
#[altium(coord_point)]
pub start: CoordPoint,
```

Reads two consecutive `i32` little-endian values as `(x, y)`.

#### Single coordinate

```rust
#[altium(coord)]
pub width: Coord,
```

Reads a single `i32` little-endian value as a `Coord`.

#### String blocks

```rust
#[altium(string_block)]
pub name: String,
```

Reads an `i32` length prefix followed by that many bytes of UTF-8 text.

#### Pascal strings

```rust
#[altium(pascal_string)]
pub label: String,
```

Reads a `u8` length prefix followed by that many bytes.

#### Fixed arrays

```rust
#[altium(array = 32)]
pub sizes: [CoordPoint; 32],
```

Reads 32 consecutive `CoordPoint` values using their `FromBinary`
implementation.

#### Skip bytes

```rust
#[altium(skip_bytes = 10)]
pub _padding: (),
```

Reads and discards N bytes on read; writes N zero bytes on write.

#### Unknown binary preservation

```rust
#[altium(unknown_binary)]
pub unknown: Vec<u8>,
```

Reads all remaining bytes in the block. On write, appends them back. This is the
binary equivalent of `#[altium(unknown)]` for parameter format.

---

## AltiumBase Derive

Generates a composition trait for base types, enabling trait-based access to
shared fields.

```rust
#[derive(AltiumBase)]
#[altium(base_name = "SchGraphicalBase")]
pub struct SchGraphicalBase {
    #[altium(flatten)]
    pub base: SchPrimitiveBase,
    #[altium(param = "LOCATION.X", frac = "LOCATION.X_FRAC")]
    pub location_x: i32,
    #[altium(param = "LOCATION.Y", frac = "LOCATION.Y_FRAC")]
    pub location_y: i32,
    #[altium(param = "COLOR", default)]
    pub color: i32,
    #[altium(param = "AREACOLOR", default)]
    pub area_color: i32,
}
```

This generates:

```rust
pub trait HasSchGraphicalBase {
    fn sch_graphical_base(&self) -> &SchGraphicalBase;
    fn sch_graphical_base_mut(&mut self) -> &mut SchGraphicalBase;

    // Accessor for each non-flattened field:
    fn location_x(&self) -> &i32 { &self.sch_graphical_base().location_x }
    fn set_location_x(&mut self, value: i32) { … }
    // …
}
```

The `extends` attribute chains trait bounds:

```rust
#[altium(base_name = "SchGraphicalBase", extends = "SchPrimitiveBase")]
```

This adds `HasSchPrimitiveBase` as a supertrait of `HasSchGraphicalBase`.

---

## AltiumEnum Derive

Generates bidirectional integer-to-enum conversion.

```rust
#[derive(AltiumEnum)]
#[altium(repr = "u8")]
pub enum PinElectricalType {
    Input = 0,
    InputOutput = 1,
    Output = 2,
    OpenCollector = 3,
    #[altium(default)]
    Passive = 4,
    HiZ = 5,
    OpenEmitter = 6,
    Power = 7,
}
```

This generates:

```rust
impl PinElectricalType {
    pub fn from_int(value: u8) -> Self { … }  // Unknown values → default variant
    pub fn to_int(self) -> u8 { … }
}

impl FromParamValue for PinElectricalType { … }
impl ToParamValue for PinElectricalType { … }
impl Default for PinElectricalType { fn default() -> Self { Self::Passive } }
```

| Attribute | Meaning |
|-----------|---------|
| `#[altium(repr = "u8")]` | Integer type for conversion (default: `i32`) |
| `#[altium(value = N)]` | Explicit integer value for a variant |
| `#[altium(default)]` | Fallback variant for unknown values; also becomes `Default` |

---

## Putting It Together

Here is how `SchPin` maps from file format to Rust struct:

**File format (parameter string):**
```
|RECORD=2|OWNERINDEX=0|LOCATION.X=100|LOCATION.Y=200|COLOR=128|
SYMBOL_INNEREDGE=0|ELECTRICAL=7|PINLENGTH=50|PINLENGTH_FRAC=0|
NAME=VCC|DESIGNATOR=1|
```

**Rust struct (after deserialization):**
```rust
SchPin {
    graphical: SchGraphicalBase {        // ← #[altium(flatten)]
        base: SchPrimitiveBase {         // ← #[altium(flatten)]
            owner_index: 0,              // ← OWNERINDEX
            is_not_accessible: false,    // ← (missing, default)
            owner_part_id: None,         // ← (missing, optional)
            owner_part_display_mode: None,
            graphically_locked: false,
        },
        location_x: 1_000_000,          // ← LOCATION.X=100 * 10000 + 0
        location_y: 2_000_000,          // ← LOCATION.Y=200 * 10000 + 0
        color: 128,                      // ← COLOR=128
        area_color: 0,                   // ← (missing, default)
    },
    symbol_inner_edge: PinSymbol::None,  // ← SYMBOL_INNEREDGE=0
    electrical: PinElectricalType::Power, // ← ELECTRICAL=7
    pin_length: 500_000,                 // ← PINLENGTH=50 * 10000 + 0
    name: "VCC".to_string(),             // ← NAME=VCC
    designator: "1".to_string(),         // ← DESIGNATOR=1
    // … remaining fields from defaults …
}
```

And here is `PcbTrack` from binary:

**File format (binary bytes, hex):**
```
04                          object_id = 4 (Track)
01                          layer = 1 (MidLayer1)
00 00                       flags = 0
[... unique_id ...]
40 42 0F 00                 start.x = 1_000_000 (100 mils)
80 84 1E 00                 start.y = 2_000_000 (200 mils)
C0 C6 2D 00                 end.x   = 3_000_000 (300 mils)
80 84 1E 00                 end.y   = 2_000_000 (200 mils)
A0 86 01 00                 width   = 100_000 (10 mils)
[16 bytes unknown]
```

**Rust struct:**
```rust
PcbTrack {
    common: PcbPrimitiveCommon {
        layer: Layer(1),       // MidLayer1
        flags: PcbFlags(0),
        unique_id: None,
    },
    start: CoordPoint { x: Coord(1_000_000), y: Coord(2_000_000) },
    end: CoordPoint { x: Coord(3_000_000), y: Coord(2_000_000) },
    width: Coord(100_000),     // 10 mils
    unknown: vec![…],
}
```
