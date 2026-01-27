# altium-format-derive

Procedural macros for the `altium-format` crate.

This crate provides three derive macros for generating serialization code for Altium Designer file format records.

## Macros

### AltiumRecord

Generates `FromParams`, `ToParams`, `FromBinary`, and `ToBinary` trait implementations for Altium record types.

**Container attributes:**
- `#[altium(record_id = N)]` - Schematic record type ID
- `#[altium(object_id = Variant)]` - PCB object ID enum variant
- `#[altium(format = "params"|"binary"|"both")]` - Serialization format

**Field attributes:**
- `#[altium(flatten)]` - Flatten a base type's fields
- `#[altium(param = "KEY")]` - Map field to parameter key
- `#[altium(param = "KEY", frac = "KEY_FRAC")]` - Integer with fractional part
- `#[altium(param = "KEY", default)]` - Use `Default::default()` if missing
- `#[altium(param = "KEY", default = value)]` - Use specific default value
- `#[altium(param = "KEY", optional)]` - Wrap in `Option<T>`
- `#[altium(binary, ty = "i32le")]` - Binary field type
- `#[altium(binary, coord_point)]` - Binary coordinate point
- `#[altium(unknown)]` - Store unknown parameters (non-destructive editing)
- `#[altium(unknown_binary)]` - Store unknown binary bytes
- `#[altium(skip)]` - Skip field entirely

**Example:**

```rust
use altium_format_derive::AltiumRecord;

#[derive(AltiumRecord)]
#[altium(record_id = 2, format = "params")]
pub struct SchPin {
    #[altium(flatten)]
    pub base: SchGraphicalBase,

    #[altium(param = "ELECTRICAL", default)]
    pub electrical: PinElectricalType,

    #[altium(param = "PINLENGTH", frac = "PINLENGTH_FRAC")]
    pub pin_length: Coord,

    #[altium(unknown)]
    pub unknown_params: UnknownFields,
}
```

### AltiumBase

Generates `HasXxxBase` traits for composition-based inheritance patterns.

**Attributes:**
- `#[altium(base_name = "Name")]` - Name for generated trait (default: struct name)
- `#[altium(extends = "ParentBase")]` - Parent base type for trait inheritance

**Example:**

```rust
use altium_format_derive::AltiumBase;

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
- `#[altium(repr = "i32"|"u8"|...)]` - Integer representation type
- `#[altium(value = N)]` - Map variant to specific integer value
- `#[altium(default)]` - Mark variant as default for unknown values

**Example:**

```rust
use altium_format_derive::AltiumEnum;

#[derive(AltiumEnum)]
#[altium(repr = "i32")]
pub enum PinElectricalType {
    #[altium(value = 0)]
    Input,
    #[altium(value = 1)]
    InputOutput,
    #[altium(value = 2)]
    Output,
    #[altium(default)]
    Passive = 4,
}
```

## Usage

This crate is automatically used when you depend on `altium-format`. You typically don't need to add it directly to your dependencies.

```toml
[dependencies]
altium-format = "0.1.0"  # Includes altium-format-derive
```

For detailed documentation, see [altium-format](../altium-format/CLAUDE.md).

## License

GPL-3.0-only
