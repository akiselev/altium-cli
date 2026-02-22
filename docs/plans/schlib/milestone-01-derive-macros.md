# Milestone 1: Derive Macros (FromParams)

**Files**: `crates/altium-format-derive/src/lib.rs`

**Flags**: `complex-algorithm`, `needs-rationale`

## Requirements

Implement a `#[derive(FromParams)]` proc macro in `altium-format-derive` that generates parameter-based record deserialization code. The macro eliminates repetitive `ParameterCollection::remove_*` boilerplate across 20+ record types.

## Derive Macro API Design

### Struct-level attribute

```rust
#[derive(FromParams)]
pub(crate) struct SchRectangle {
    // fields...
}
```

Generates:
```rust
impl SchRectangle {
    pub(crate) fn from_params(params: &mut ParameterCollection) -> Result<Self> {
        Ok(Self {
            // field extraction...
        })
    }
}
```

The generated `from_params()` does NOT call `params.assert_exhausted()`. Exhaustion checking is the caller's responsibility (because flattened base types extract a subset of params).

### Field attributes

| Attribute | Generated Code | Use Case |
|-----------|---------------|----------|
| `#[param(key = PATH)]` | `params.remove_required::<T>(PATH)?` | Required field |
| `#[param(key = PATH, default = EXPR)]` | `params.remove_with_default::<T>(PATH, EXPR)?` | Optional with default |
| `#[param(key = PATH, optional)]` | `params.remove_optional::<T>(PATH)?` | `Option<T>` field |
| `#[param(coord, key = K, frac_key = FK)]` | `params.remove_coord(K, FK)?` | DXP fractional coordinate |
| `#[param(coord_point, x_key = XK, x_frac = XF, y_key = YK, y_frac = YF)]` | Calls `remove_coord` twice, constructs `CoordPoint` | Coordinate pair |
| `#[param(indexed_coords, count_key = CK, x_prefix = XP, y_prefix = YP)]` | `params.remove_indexed_coords(CK, XP, YP)?` | Polyline/polygon vertices |
| `#[param(flatten)]` | `T::from_params(params)?` | Compose base types |
| `#[param(list, key = PATH)]` | `params.remove_list::<T>(PATH)?` | Comma-separated values |
| `#[param(list_or_empty, key = PATH)]` | `params.remove_list_or_empty::<T>(PATH)?` | Comma-separated, default empty |

Where `PATH`, `K`, `FK`, etc. are path expressions (typically constants from `altium_format_types::constants::*`) that resolve to `&str` in the calling scope.

### Example usage

```rust
use altium_format_types::constants::{visual::*, record_structure::*, component::*};

#[derive(FromParams)]
pub(crate) struct SchPrimitiveBase {
    #[param(key = OWNER_INDEX, default = -1i32)]
    pub owner_index: i32,

    #[param(key = IS_NOT_ACCESSIBLE, default = false)]
    pub is_not_accessible: bool,

    #[param(key = OWNER_PART_ID, default = -1i32)]
    pub owner_part_id: i32,

    #[param(key = OWNER_PART_DISPLAY_MODE, default = 0i32)]
    pub owner_part_display_mode: i32,

    #[param(key = GRAPHICALLY_LOCKED, default = false)]
    pub graphically_locked: bool,

    #[param(key = INDEX_IN_SHEET, default = -1i32)]
    pub index_in_sheet: i32,
}

#[derive(FromParams)]
pub(crate) struct SchGraphicalBase {
    #[param(flatten)]
    pub primitive: SchPrimitiveBase,

    #[param(coord_point, x_key = LOCATION_X, x_frac = LOCATION_X_FRAC, y_key = LOCATION_Y, y_frac = LOCATION_Y_FRAC)]
    pub location: CoordPoint,

    #[param(key = COLOR, default = Color::black())]
    pub color: Color,

    #[param(key = AREA_COLOR, default = Color::black())]
    pub area_color: Color,
}

#[derive(FromParams)]
pub(crate) struct SchRectangle {
    #[param(flatten)]
    pub base: SchGraphicalBase,

    #[param(coord_point, x_key = CORNER_X, x_frac = CORNER_X_FRAC, y_key = CORNER_Y, y_frac = CORNER_Y_FRAC)]
    pub corner: CoordPoint,

    #[param(key = IS_SOLID, default = false)]
    pub is_solid: bool,

    #[param(key = LINE_WIDTH, default = 1i32)]
    pub line_width: i32,

    #[param(key = TRANSPARENT, default = true)]
    pub transparent: bool,
}
```

### Generated code for SchRectangle

```rust
impl SchRectangle {
    pub(crate) fn from_params(params: &mut ParameterCollection) -> Result<Self> {
        Ok(Self {
            base: SchGraphicalBase::from_params(params)?,
            corner: {
                let x = params.remove_coord(CORNER_X, CORNER_X_FRAC)?;
                let y = params.remove_coord(CORNER_Y, CORNER_Y_FRAC)?;
                CoordPoint::new(x, y)
            },
            is_solid: params.remove_with_default(IS_SOLID, false)?,
            line_width: params.remove_with_default(LINE_WIDTH, 1i32)?,
            transparent: params.remove_with_default(TRANSPARENT, true)?,
        })
    }
}
```

## Implementation Approach

The proc macro parses struct fields and their `#[param(...)]` attributes using `syn`, then generates the `from_params` method body using `quote`.

Key implementation steps:
1. Parse struct definition and validate it's a named-field struct
2. For each field, parse the `#[param(...)]` attribute to determine extraction strategy
3. Generate field initialization expression based on attribute variant
4. Wrap in `impl` block with `from_params` method

The macro must handle:
- **Type inference**: The `T` in `remove_required::<T>` comes from the field's declared type
- **Path expressions**: Key attributes are arbitrary expressions (typically constant paths), emitted as-is
- **Default expressions**: The `default = EXPR` value is emitted as-is into generated code
- **Visibility**: Generated method matches the struct's visibility

## Acceptance Criteria

- `#[derive(FromParams)]` compiles and generates correct `from_params()` methods
- All attribute variants (`key`, `default`, `optional`, `coord`, `coord_point`, `indexed_coords`, `flatten`, `list`, `list_or_empty`) work correctly
- Generated code references constant paths (no string literals in generated output)
- Compile errors for missing required attributes (e.g., `#[param(coord)]` without `key` and `frac_key`)
- Unit tests for each attribute variant using mock ParameterCollection data

## Tests

- **Test files**: `crates/altium-format-derive/tests/from_params.rs` (integration tests for proc macros must be in tests/ directory)
- **Test type**: integration (proc macro tests require separate compilation)
- **Backing**: doc-derived (CLAUDE.md fail-fast philosophy)
- **Scenarios**:
  - Normal: struct with all attribute variants parses correctly
  - Normal: flatten composes base types correctly
  - Normal: coord_point produces correct CoordPoint
  - Normal: indexed_coords handles variable-length vertex lists
  - Edge: default values used when key absent
  - Edge: optional field returns None when key absent
  - Error: required field missing produces MissingParam error
  - Error: invalid value produces InvalidParamValue error

## Code Intent

- New implementation in `crates/altium-format-derive/src/lib.rs`:
  - `#[proc_macro_derive(FromParams, attributes(param))]` entry point
  - Internal parsing of `#[param(...)]` attribute syntax using syn
  - Code generation for each field extraction strategy using quote
  - Error reporting for malformed attributes (compile_error!)
- The generated code references:
  - `ParameterCollection` (from altium-format, must be in scope)
  - `Result` (from altium-format)
  - Constant paths from `altium_format_types::constants::*` (user imports)
  - `CoordPoint::new()` for coord_point variant
  - `Coord` for coord variant
