# altium-format-derive

Procedural macro crate for automatic serialization code generation in altium-format.

## File Index

| File           | What                                        | When                                             |
| -------------- | ------------------------------------------- | ------------------------------------------------ |
| lib.rs         | AltiumRecord, AltiumBase, AltiumEnum macros | Entry point for all derive macros                |
| record.rs      | AltiumRecord implementation                 | Deriving FromParams/ToParams/FromBinary/ToBinary |
| base.rs        | AltiumBase implementation                   | Generating HasXxxBase composition traits         |
| enum_derive.rs | AltiumEnum implementation                   | Deriving integer enum conversions                |
| attrs.rs       | Attribute parsing utilities                 | Extracting #[altium(...)] metadata               |

## Macros

### AltiumRecord

Generates serialization code for Altium record types.

**Container attributes:**
- `#[altium(record_id = N)]` - Schematic record type ID (1-255)
- `#[altium(object_id = Variant)]` - PCB object ID enum variant
- `#[altium(format = "params"|"binary"|"both")]` - Serialization format

**Field attributes:**
- `#[altium(flatten)]` - Flatten base type fields into parent
- `#[altium(param = "KEY")]` - Map field to parameter key
- `#[altium(param = "KEY", frac = "KEY_FRAC")]` - Integer + fractional coordinate
- `#[altium(param = "KEY", default)]` - Use Default::default() if missing
- `#[altium(param = "KEY", optional)]` - Wrap in Option<T>
- `#[altium(binary, ty = "i32le")]` - Binary field with endianness
- `#[altium(binary, coord_point)]` - Binary CoordPoint struct
- `#[altium(unknown)]` - Capture unknown parameters for non-destructive editing
- `#[altium(skip)]` - Skip field during serialization

**Example:**
```rust
#[derive(AltiumRecord)]
#[altium(record_id = 2, format = "params")]
pub struct SchPin {
    #[altium(flatten)]
    pub base: SchGraphicalBase,

    #[altium(param = "ELECTRICAL", default)]
    pub electrical: PinElectricalType,

    #[altium(unknown)]
    pub unknown_params: UnknownFields,
}
```

### AltiumBase

Generates composition traits for base type inheritance.

**Attributes:**
- `#[altium(base_name = "Name")]` - Generated trait name (default: struct name)
- `#[altium(extends = "ParentBase")]` - Parent trait for trait inheritance

**Example:**
```rust
#[derive(AltiumBase)]
#[altium(base_name = "SchPrimitiveBase")]
pub struct SchPrimitiveBase {
    #[altium(param = "OWNERINDEX", default)]
    pub owner_index: i32,
}

#[derive(AltiumBase)]
#[altium(base_name = "SchGraphicalBase", extends = "SchPrimitiveBase")]
pub struct SchGraphicalBase {
    #[altium(flatten)]
    pub base: SchPrimitiveBase,

    #[altium(param = "LOCATION.X", frac = "LOCATION.X_FRAC")]
    pub location_x: i32,
}
```

### AltiumEnum

Generates integer conversion traits for enum types.

**Attributes:**
- `#[altium(repr = "i32"|"u8"|...)]` - Integer representation
- `#[altium(value = N)]` - Map variant to integer value
- `#[altium(default)]` - Default variant for unknown values

**Example:**
```rust
#[derive(AltiumEnum)]
#[altium(repr = "i32")]
pub enum PinElectricalType {
    #[altium(value = 0)]
    Input,
    #[altium(value = 1)]
    InputOutput,
    #[altium(default)]
    Passive = 4,
}
```

## Design Patterns

### Flattening for Composition

Base types are flattened into child structs to avoid deep nesting:

```rust
pub struct SchPin {
    #[altium(flatten)]
    pub base: SchGraphicalBase,  // Contains owner_index, location_x, etc.
}
```

Access: `pin.base.owner_index` instead of `pin.base.base.owner_index`

### Unknown Field Preservation

Non-destructive editing requires preserving unknown parameters:

```rust
#[altium(unknown)]
pub unknown_params: UnknownFields,
```

Roundtrip: `from_params(to_params(x)) == x` even when new fields added to format.

### Coordinate Fractional Parts

Altium splits coordinates into integer + fractional parts:

```rust
#[altium(param = "X", frac = "X_FRAC")]
pub x: Coord,
```

Serializes as two parameters: `X=100` and `X_FRAC=5000` for 100.5 mils.

## Usage in altium-format

These macros are re-exported by altium-format:

```rust
use altium_format::{AltiumRecord, AltiumBase, AltiumEnum};
```

All record types in `altium-format/src/records/` use these derives.
