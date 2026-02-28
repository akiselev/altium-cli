# Simple Missing Field Fixes

Three issues blocking 59 files total. All are straightforward field additions.

---

## 1. RECORD=29 (Junction) Missing `LOCKED` Boolean -- 29 Files

### Current Code

`crates/altium-format/src/sch_records.rs` lines 1691-1700:

```rust
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchJunction {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(coord_point, x_key = LOCATION_X, x_frac = LOCATION_X_FRAC, y_key = LOCATION_Y, y_frac = LOCATION_Y_FRAC)]
    pub location: CoordPoint,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
}
```

### C# Reference

`AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.FileFormats/FileFormatV5.cs` lines 823-861:

```csharp
// Export order: GraphicalObject, Location.X, Location.Y, Size, Color, Locked, UniqueID
protected override void ExportJunction(ISchDataSerializer argSerializer, ISchDataObject argObject)
{
    ExportGraphicalObject(argSerializer, argObject);
    if (argObject is ISchDataJunction schDataJunction)
    {
        argSerializer.Export_Coord(schDataJunction.GetLocation().X, "Location.X");
        argSerializer.Export_Coord(schDataJunction.GetLocation().Y, "Location.Y");
        argSerializer.Export_Size(schDataJunction.GetSize(), "Size");
        argSerializer.Export_Color(schDataJunction.GetColor(), "Color");
        argSerializer.Export_Boolean(schDataJunction.GetLocked(), "Locked");
        argSerializer.Export_DynamicString(schDataJunction.GetUniqueId(), "UniqueID");
    }
}

// Import: default for Locked is true
bool argN3 = true;
argSerializer.Import_Boolean(ref argN3, "Locked");
schDataJunction.SetLocked(argN3);
```

### Field Type and Semantics

- **Key**: `"Locked"` (constant already exists: `altium_format_types::constants::electrical::LOCKED` and `altium_format_types::constants::locking::LOCKED`)
- **Type**: `bool` (serialized as "T"/"F")
- **Default**: `true` (note: Import_Boolean defaults to true, not false)
- **Semantics**: When true, the junction is locked in place and cannot be moved by the user. This is a separate concept from `GraphicallyLocked` (which is in the base and always reset to false on import).

### Recommended Fix

Add `locked` field to `SchJunction` between `color` and the end. Also note the struct is missing `size` and `unique_id` -- these should be added too for completeness (C# exports them in that order).

The `Size` field uses `TSize` enum exported as a `u8` via `Export_Size`/`Import_Size`. `TSize` appears to be an enum with `eZeroSize` as default. It likely maps to a small set of predefined junction sizes.

```rust
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchJunction {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(coord_point, x_key = LOCATION_X, x_frac = LOCATION_X_FRAC, y_key = LOCATION_Y, y_frac = LOCATION_Y_FRAC)]
    pub location: CoordPoint,
    // Size is exported as u8 via Export_Size. TSize enum with eZeroSize=0 default.
    // Check TSize enum in Delphi/C# for values. For now treat as i32 since
    // Import_Size reads a byte.
    #[param(key = SIZE, default = 0i32)]
    pub size: i32,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(key = LOCKED, default = true)]
    pub locked: bool,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
}
```

**Key detail**: The default for `locked` is `true`, not `false`. The C# code initializes `argN3 = true` before calling `Import_Boolean`, meaning if "Locked" is absent the junction defaults to locked.

**Constants**: Use `LOCKED` from `altium_format_types::constants::locking` (or `electrical` -- they're the same string `"Locked"`). For `SIZE`, use the existing `SIZE` constant from `altium_format_types::constants::visual`. For `UNIQUE_ID`, it's already imported.

---

## 2. RECORD=16 (SheetEntry) Missing `DISTANCEFROMTOP_FRAC1` -- 16 Files

**Note**: The task description labels this as "SheetName" but RECORD=16 is actually `SchSheetEntry`. RECORD=32 is `SchSheetName`.

### Current Code

`crates/altium-format/src/sch_records.rs` lines 1731-1764:

```rust
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchSheetEntry {
    // ...
    #[param(key = DISTANCE_FROM_TOP, default = Coord::from_internal(0))]
    pub distance_from_top: Coord,
    // ...
}
```

The `distance_from_top` field currently reads `DistanceFromTop` as a plain integer (no fractional part). When files contain `DistanceFromTop_Frac1` (or legacy `DistanceFromTop_Frac`), those params are left unconsumed and trigger `assert_exhausted()`.

### C# Reference

**Export** (`SchDataSerializer.cs` line 399):

```csharp
public void ExportDistanceFromTop(int argN, string argName)
{
    int num = 1000000;  // divisor is 1_000_000, NOT standard 100_000
    int num2 = argN / num;
    int num3 = argN - num2 * num;
    WriteShort((short)num2, argName);           // "DistanceFromTop" as i16
    if (num3 != 0)
    {
        WriteInt(num3, argName + "_Frac1");     // "DistanceFromTop_Frac1" as i32
    }
}
```

**Import** (`SchDataSerializer.cs` line 728):

```csharp
public void ImportDistanceFromTop(ref int argN, string argName)
{
    ReadShort(out var value, argName);           // "DistanceFromTop" as i16
    ReadInt(out var value2, argName + "_Frac");  // legacy "_Frac" (i32)
    int value3 = 0;
    if (value2 == 0)
    {
        ReadInt(out value3, argName + "_Frac1"); // new "_Frac1" (i32)
    }
    if (value2 != 0)
    {
        argN = (value * 100000 + value2) * 10;   // legacy: (whole * 100_000 + frac) * 10
    }
    else if (value3 != 0)
    {
        argN = value * 100000 * 10 + value3;     // new: whole * 1_000_000 + frac1
    }
    else
    {
        argN = value * 100000 * 10;               // no frac: whole * 1_000_000
    }
}
```

### Field Type and Semantics

DistanceFromTop uses a **non-standard** fractional encoding:
- **Divisor**: 1_000_000 (10x the standard DXP 100_000)
- **Integer part**: stored as `i16` (short), value = `coord / 1_000_000`
- **Fractional part**: stored as `i32`, TWO possible keys:
  - `DistanceFromTop_Frac` (legacy, old DXP format): if present, `coord = (short * 100_000 + frac) * 10`
  - `DistanceFromTop_Frac1` (new format): if present, `coord = short * 1_000_000 + frac1`

Both representations resolve to the same internal coordinate. The key distinction is that `_Frac` uses the old 100_000 base (multiplied by 10), while `_Frac1` is direct with 1_000_000 base.

### Recommended Fix

Since the `#[param(coord)]` derive attribute uses `remove_coord` (which is based on standard DXP 100_000 divisor), we **cannot** use the derive macro for this field. The DistanceFromTop encoding is custom.

**Option A** (simplest -- manual parsing): Switch `SchSheetEntry` from fully-derived to manual parsing for `distance_from_top`, consuming both `_Frac` and `_Frac1`.

**Option B** (better -- add helper to ParameterCollection): Add a `remove_distance_from_top` helper method to `ParameterCollection` that implements the C# logic, and a corresponding `insert_distance_from_top` for serialization. Then call it manually in a custom parse step.

Recommended: **Option B**.

Add to `ParameterCollection`:

```rust
/// Parses DistanceFromTop which uses a non-standard fractional encoding.
/// Divisor is 1_000_000 (10x standard DXP), with legacy "_Frac" and new "_Frac1" variants.
/// See C# SchDataSerializer.ImportDistanceFromTop / ExportDistanceFromTop.
pub(crate) fn remove_distance_from_top(&mut self, key: &str) -> Result<Coord> {
    let whole: i16 = self.remove_with_default(key, 0i16)?;
    let frac_key = format!("{key}_Frac");
    let frac1_key = format!("{key}_Frac1");
    let frac: i32 = self.remove_with_default(&frac_key, 0i32)?;
    let frac1: i32 = self.remove_with_default(&frac1_key, 0i32)?;

    let coord = if frac != 0 {
        // Legacy format: (whole * 100_000 + frac) * 10
        ((whole as i32) * 100_000 + frac) * 10
    } else if frac1 != 0 {
        // New format: whole * 1_000_000 + frac1
        (whole as i32) * 1_000_000 + frac1
    } else {
        // No frac: whole * 1_000_000
        (whole as i32) * 1_000_000
    };
    Ok(Coord::from_internal(coord))
}

/// Serializes DistanceFromTop using new _Frac1 format (matching C# ExportDistanceFromTop).
pub(crate) fn insert_distance_from_top(&mut self, key: &str, coord: Coord) {
    let val = coord.to_internal();
    let whole = val / 1_000_000;
    let frac1 = val - whole * 1_000_000;
    self.insert(key, (whole as i16).to_param_value());
    if frac1 != 0 {
        let frac1_key = format!("{key}_Frac1");
        self.insert(&frac1_key, frac1.to_param_value());
    }
}
```

Then change `SchSheetEntry` to NOT use `#[param]` for `distance_from_top`. Instead, either:
1. Make `SchSheetEntry` manually parsed (not derived), or
2. Use a custom `FromParams`/`ToParams` impl that calls `remove_distance_from_top`.

Since the struct uses `#[derive(FromParams, ToParams)]`, the simplest approach is to remove the `#[param(key = DISTANCE_FROM_TOP, ...)]` annotation and handle it manually in a custom parse step. However, this breaks the derive macro pattern.

**Practical recommendation**: Add `remove_distance_from_top` / `insert_distance_from_top` to `ParameterCollection`, then convert `SchSheetEntry` to manual parsing (like `SchComponent`). This is the cleanest approach since the encoding is genuinely non-standard.

---

## 3. RECORD=2 (Pin) Missing `Name_CustomPosition_Margin_Frac` and `Designator_CustomPosition_Margin_Frac` -- 14 Files

### Current Code

`crates/altium-format/src/sch_records.rs` lines 450-454 and 488-492:

```rust
// Name margin (line 450):
let custom_position_margin = if position_mode_custom {
    params.remove_optional::<Coord>(NAME_CUSTOM_POSITION_MARGIN)?
} else {
    None
};

// Designator margin (line 488):
let custom_position_margin = if position_mode_custom {
    params.remove_optional::<Coord>(DESIGNATOR_CUSTOM_POSITION_MARGIN)?
} else {
    None
};
```

The problem: `remove_optional::<Coord>` reads only the integer key (e.g., `Name_CustomPosition_Margin`) as a `Coord` via the `FromParamValue` trait. It does NOT consume the companion `_Frac` key (`Name_CustomPosition_Margin_Frac`), which is left in the parameter collection and triggers `assert_exhausted()`.

### C# Reference

`AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.FileFormats/FileFormatV5.cs` lines 506-509, 533-536:

```csharp
// Export
if (schDataPin.GetNamePositionMode() == TPinItemMode.ePinItemMode_Custom)
{
    argSerializer.Export_ASCIIOnlyCoord(schDataPin.GetNameCustomPositionMargin(),
        "Name_CustomPosition_Margin");
}

if (schDataPin.GetDesignatorPositionMode() == TPinItemMode.ePinItemMode_Custom)
{
    argSerializer.Export_ASCIIOnlyCoord(schDataPin.GetDesignatorCustomPositionMargin(),
        "Designator_CustomPosition_Margin");
}
```

`Export_ASCIIOnlyCoord` (SchDataSerializer.cs line 458):

```csharp
public void Export_ASCIIOnlyCoord(int argN, string argName)
{
    SchDataUtils.GetWholeAndFractionalPart_DXP2004SP2_To_DXP2004SP1(argN, out var whole, out var fraction);
    WriteShortAsciiOnly(Convert.ToInt16(whole), argName);        // "Name_CustomPosition_Margin"
    if (fraction != 0)
    {
        WriteIntAsciiOnly(fraction, argName + "_Frac");          // "Name_CustomPosition_Margin_Frac"
    }
}
```

`Import_ASCIIOnlyCoord` (SchDataSerializer.cs line 800):

```csharp
public void Import_ASCIIOnlyCoord(ref int argN, string argName)
{
    ReadShortAsciiOnly(out var value, argName);
    ReadIntAsciiOnly(out var value2, argName + "_Frac");
    argN = SchDataUtils.GetCoord_DXP2004SP1_To_DXP2004SP2(value, value2);
    // = value * 100_000 + value2
}
```

`SchDataUtils.GetCoord_DXP2004SP1_To_DXP2004SP2` (SchDataUtils.cs line 365):

```csharp
public static int GetCoord_DXP2004SP1_To_DXP2004SP2(int whole, int fraction)
{
    return whole * 100000 + fraction;
}
```

### Field Type and Semantics

This uses the **standard DXP fractional encoding**: `whole * 100_000 + frac`.
- Integer key: `"Name_CustomPosition_Margin"` (i16 / short)
- Frac key: `"Name_CustomPosition_Margin_Frac"` (i32)
- Combined: `Coord::from_dxp_frac(whole, frac)`
- Semantics: The margin (offset) of the pin name text from its default position, in internal coordinate units.

This is the exact same encoding used by `LOCATION_X` / `LOCATION_X_FRAC`, so we can use the existing `remove_coord_optional` method.

### Recommended Fix

Replace `remove_optional::<Coord>` with `remove_coord_optional` for both margin fields:

```rust
// Name margin (replace lines 450-454):
let custom_position_margin = if position_mode_custom {
    params.remove_coord_optional(
        NAME_CUSTOM_POSITION_MARGIN,
        &format!("{NAME_CUSTOM_POSITION_MARGIN}_Frac"),
    )?
} else {
    None
};

// Designator margin (replace lines 488-492):
let custom_position_margin = if position_mode_custom {
    params.remove_coord_optional(
        DESIGNATOR_CUSTOM_POSITION_MARGIN,
        &format!("{DESIGNATOR_CUSTOM_POSITION_MARGIN}_Frac"),
    )?
} else {
    None
};
```

**Serialization side**: Also need to update the text pin serialization (if it exists) to use `insert_coord` instead of plain `insert` for these fields. Currently pins in SchDoc are text-format and use `parse_text_pin`, but there is no `serialize_text_pin` -- pins are serialized only as binary for SchLib (via `serialize_binary_pin`) where these fields don't apply (they come from sidecar text data). So the serialization fix is not needed for binary pin format (these fields aren't in the binary layout).

However, if SchDoc serialization ever roundtrips text pins, the `ToParams`-equivalent for `PinTextPositioning` would need to emit `insert_coord` for the margin. This should be verified.

**Frac key constants**: Consider adding `NAME_CUSTOM_POSITION_MARGIN_FRAC` and `DESIGNATOR_CUSTOM_POSITION_MARGIN_FRAC` constants to `altium_format_types::constants::pin`, or just use the `format!()` pattern as shown above.

---

## Summary of Changes

| Issue | Record | Field | Fix Type | Complexity |
|-------|--------|-------|----------|------------|
| 1 | Junction (29) | `LOCKED` | Add bool field with `default = true` | Trivial (1 line in struct) |
| 2 | SheetEntry (16) | `DISTANCEFROMTOP_FRAC1` | Custom coord encoding, manual parse | Medium (new helper + manual parse) |
| 3 | Pin (2) | `*_CustomPosition_Margin_Frac` | Replace `remove_optional` with `remove_coord_optional` | Easy (2 line changes) |

### Priority Order

1. **Pin margin frac** (issue 3) -- simplest, just swap method call
2. **Junction LOCKED** (issue 1) -- add field + also add missing Size/UniqueID
3. **SheetEntry DistanceFromTop** (issue 2) -- requires custom helper and struct refactor
