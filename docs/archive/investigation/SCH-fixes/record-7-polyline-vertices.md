# Extra Vertices >50 for Polyline/Polygon/Bezier/Wire/Bus/Blanket

## Problem

Our `indexed_coords` derive macro and `remove_indexed_coords()` only read `LocationCount`
and then `X1..Xn` / `Y1..Yn`. When a polyline (or polygon, bezier, wire, bus, blanket) has
more than 50 vertices, Altium splits the storage:

- First 50 vertices: `LocationCount=50`, `X1..X50`, `Y1..Y50` (+ `_Frac` variants)
- Overflow vertices: `EXTRALOCATIONCOUNT=N`, `EX51..EX(50+N)`, `EY51..EY(50+N)` (+ `_Frac`)

Our parser reads `LocationCount` vertices and stops, leaving `EXTRALOCATIONCOUNT`, `EXn`,
`EYn`, and their `_Frac` fields unconsumed. This triggers `assert_exhausted()` failures
for any file with >50 vertices on these record types, blocking 84 SchDoc files.

## Record Type Clarification

Despite the task title saying "RECORD=7 (Polyline)", the actual mapping is:
- RECORD=6 = Polyline (`SchPolyline`)
- RECORD=7 = Polygon (`SchPolygon`)

Both (and several others) are affected because they all use the same vertex storage system.

## C# Reference: `SchDataVertices` (authoritative)

Source: `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.Objects/SchDataVertices.cs`

### Internal Storage

```csharp
private readonly List<TLocation> list = new List<TLocation>();
```

A plain dynamic `List<TLocation>` -- no hard limit. The "50" is purely a serialization
split point, not a logical cap.

### Export (write) -- `ExportToFile`

```csharp
public void ExportToFile(ISchDataSerializer serializer)
{
    int num = Math.Min(Count(), 50);                             // cap at 50
    serializer.Export_ShortInt(num, "LocationCount");
    for (int i = 1; i <= num; i++)
    {
        string text = i.ToString();
        TLocation location = GetLocation(i);
        serializer.Export_Coord(location.X, "X" + text);        // X1..X50
        serializer.Export_Coord(location.Y, "Y" + text);        // Y1..Y50
    }
    if (num != Count())                                          // overflow?
    {
        serializer.Export_ShortInt(Count() - num, "EXTRALOCATIONCOUNT");
        for (int j = num + 1; j <= Count(); j++)
        {
            string text2 = j.ToString();
            TLocation location2 = GetLocation(j);
            serializer.Export_Coord(location2.X, "EX" + text2); // EX51, EX52, ...
            serializer.Export_Coord(location2.Y, "EY" + text2); // EY51, EY52, ...
        }
    }
}
```

Key observations:
1. `LocationCount` is capped at `min(count, 50)`.
2. If count > 50, `EXTRALOCATIONCOUNT = count - 50` is written.
3. Extra vertices use prefix `EX`/`EY` with indices *continuing* from the base set
   (51, 52, ..., not 1, 2, ...).
4. `Export_Coord` writes the integer part AND a `_Frac` part (if non-zero):
   - `EX51` + `EX51_Frac`, `EY51` + `EY51_Frac`, etc.

### Import (read) -- `ImportFromFile`

```csharp
public void ImportFromFile(ISchDataSerializer serializer, bool includeExLocations)
{
    Clear();
    int argN = 0;
    serializer.Import_ShortInt(ref argN, "LocationCount");      // base count
    int argN2 = 0;
    if (includeExLocations)
    {
        serializer.Import_ShortInt(ref argN2, "EXTRALOCATIONCOUNT"); // overflow count
    }
    SetCount(argN + argN2);                                     // total vertices
    TLocation location = default(TLocation);
    for (int i = 1; i <= argN; i++)
    {
        string text = i.ToString();
        location.X = 0; location.Y = 0;
        serializer.Import_Coord(ref location.X, "X" + text);   // X1..X(argN)
        serializer.Import_Coord(ref location.Y, "Y" + text);   // Y1..Y(argN)
        SetLocation(i, location);
    }
    if (includeExLocations)
    {
        TLocation location2 = default(TLocation);
        for (int j = argN + 1; j <= argN + argN2; j++)
        {
            string text2 = j.ToString();
            location2.X = 0; location2.Y = 0;
            serializer.Import_Coord(ref location2.X, "EX" + text2);  // EX(argN+1)..
            serializer.Import_Coord(ref location2.Y, "EY" + text2);  // EY(argN+1)..
            SetLocation(j, location2);
        }
    }
}
```

Key observations:
1. `includeExLocations` is a bool parameter that controls whether overflow is parsed.
2. In V5 format (current), ALL vertex-bearing records pass `includeExLocations: true`:
   - `ImportPolygon` (RECORD=7, line 1168)
   - `ImportPolyline` (RECORD=6, line 1216)
   - `ImportBezier` (RECORD=5, line 1248)
   - `ImportWire` (RECORD=27, line 1287)
   - `ImportBus` (RECORD=26, line 1326)
   - `ImportSignalHarness` (RECORD=208, line 2662)
   - `ImportBlanket` (RECORD=225, line 2795)
3. In V4 format (legacy), all pass `includeExLocations: false`.
4. `Import_Coord` automatically reads both `argName` and `argName + "_Frac"`.

### Coord Serialization Detail

`Export_Coord` (base `SchDataSerializer`, line 538-546):
```csharp
public virtual void Export_Coord(int argN, string argName)
{
    SchDataUtils.GetWholeAndFractionalPart_DXP2004SP2_To_DXP2004SP1(argN, out var whole, out var fraction);
    WriteShort(Convert.ToInt16(whole), argName);
    if (fraction != 0)
    {
        WriteInt(fraction, argName + "_Frac");
    }
}
```

So each vertex coordinate produces:
- `Xn` (i16 whole part) + optionally `Xn_Frac` (i32 fractional part)
- `Yn` (i16 whole part) + optionally `Yn_Frac` (i32 fractional part)
- `EXn` (i16 whole part) + optionally `EXn_Frac` (i32 fractional part)
- `EYn` (i16 whole part) + optionally `EYn_Frac` (i32 fractional part)

Our `remove_coord()` already handles the `_Frac` suffix correctly. The only gap is not
reading `EXTRALOCATIONCOUNT` and the `EX`/`EY` prefixed vertices.

## Which Records Use `includeExLocations: true`

From `FileFormatV5.cs`:

| Record Type | RECORD= | V5 `includeExLocations` | Our Rust struct | Has `indexed_coords` |
|---|---|---|---|---|
| Polygon | 7 | true | `SchPolygon` | yes |
| Polyline | 6 | true | `SchPolyline` | yes |
| Bezier | 5 | true | `SchBezier` | yes |
| Wire | 27 | true | `SchWire` | yes |
| Bus | 26 | true | `SchBus` | yes |
| SignalHarness | 208 | true | (not yet implemented) | N/A |
| Blanket | 225 | true | `SchBlanket` | yes |

ALL of these are affected and need the fix.

## Current Implementation

### Derive macro: `#[param(indexed_coords, ...)]`

File: `crates/altium-format-derive/src/lib.rs`

The `indexed_coords` strategy generates:
- **Parse**: calls `params.remove_indexed_coords(count_key, x_prefix, y_prefix)`
- **Serialize**: calls `params.insert_indexed_coords(count_key, x_prefix, y_prefix, &self.field)`

### `remove_indexed_coords` (parse)

File: `crates/altium-format/src/param_collection.rs:321-339`

```rust
pub(crate) fn remove_indexed_coords(
    &mut self,
    count_key: &str,
    x_prefix: &str,
    y_prefix: &str,
) -> Result<Vec<CoordPoint>> {
    let count: usize = self.remove_required(count_key)?;
    let mut points = Vec::with_capacity(count);
    for i in 1..=count {
        let x_key = format!("{x_prefix}{i}");
        let y_key = format!("{y_prefix}{i}");
        let x_frac_key = format!("{x_prefix}{i}_Frac");
        let y_frac_key = format!("{y_prefix}{i}_Frac");
        let x = self.remove_coord(&x_key, &x_frac_key)?;
        let y = self.remove_coord(&y_key, &y_frac_key)?;
        points.push(CoordPoint::new(x, y));
    }
    Ok(points)
}
```

**Bug**: Only reads `count_key` (`LocationCount`) vertices. Does NOT check for
`EXTRALOCATIONCOUNT` or read `EX`/`EY` prefixed overflow vertices.

### `insert_indexed_coords` (serialize)

File: `crates/altium-format/src/param_collection.rs:73-94`

```rust
pub(crate) fn insert_indexed_coords(
    &mut self,
    count_key: &str,
    x_prefix: &str,
    y_prefix: &str,
    points: &[CoordPoint],
) {
    self.insert(count_key, points.len().to_param_value());
    for (i, point) in points.iter().enumerate() {
        let idx = i + 1;
        // ... writes X{idx}/Y{idx} with _Frac
    }
}
```

**Bug**: Writes ALL vertices under `LocationCount` with `X`/`Y` prefix. Does NOT split
at 50 or emit `EXTRALOCATIONCOUNT` / `EX`/`EY` for overflow.

### Existing constants

Already defined in `crates/altium-format-types/src/constants/`:
- `visual.rs:360`: `EXTRA_LOCATION_COUNT = "EXTRALOCATIONCOUNT"`
- `visual.rs:366`: `LOCATION_COUNT = "LocationCount"`
- `record_structure.rs:161`: `EX = "EX"`
- `record_structure.rs:167`: `EY = "EY"`
- `record_structure.rs:149`: `X = "X"`
- `record_structure.rs:155`: `Y = "Y"`

## Recommended Fix

### Option A: Fix `remove_indexed_coords` / `insert_indexed_coords` directly (PREFERRED)

Modify the two methods in `param_collection.rs` to handle the 50-vertex split.
No derive macro changes needed -- the existing `indexed_coords` attribute is sufficient
because the split is purely a serialization concern, not a structural one.

#### Parse (`remove_indexed_coords`)

After reading `count_key` vertices, check for `EXTRALOCATIONCOUNT`. If present, read
additional vertices with `EX`/`EY` prefix, indices continuing from `count+1`.

```rust
pub(crate) fn remove_indexed_coords(
    &mut self,
    count_key: &str,
    x_prefix: &str,
    y_prefix: &str,
) -> Result<Vec<CoordPoint>> {
    let base_count: usize = self.remove_required(count_key)?;
    let extra_count: usize = self.remove_optional::<usize>(EXTRA_LOCATION_COUNT)?
        .unwrap_or(0);
    let total = base_count + extra_count;
    let mut points = Vec::with_capacity(total);

    // Base vertices: X1..X{base_count}, Y1..Y{base_count}
    for i in 1..=base_count {
        let x_key = format!("{x_prefix}{i}");
        let y_key = format!("{y_prefix}{i}");
        let x_frac_key = format!("{x_prefix}{i}_Frac");
        let y_frac_key = format!("{y_prefix}{i}_Frac");
        let x = self.remove_coord(&x_key, &x_frac_key)?;
        let y = self.remove_coord(&y_key, &y_frac_key)?;
        points.push(CoordPoint::new(x, y));
    }

    // Extra vertices: EX{base_count+1}..EX{total}, EY{base_count+1}..EY{total}
    for i in (base_count + 1)..=(base_count + extra_count) {
        let x_key = format!("EX{i}");   // NOTE: hardcoded "EX"/"EY" prefix
        let y_key = format!("EY{i}");
        let x_frac_key = format!("EX{i}_Frac");
        let y_frac_key = format!("EY{i}_Frac");
        let x = self.remove_coord(&x_key, &x_frac_key)?;
        let y = self.remove_coord(&y_key, &y_frac_key)?;
        points.push(CoordPoint::new(x, y));
    }

    Ok(points)
}
```

**Note on prefix**: The extra vertices always use `EX`/`EY` hardcoded prefix (per C#
`SchDataVertices.ExportToFile`). The `x_prefix`/`y_prefix` parameters from the derive
attribute only apply to the base vertices. This is a format-level invariant, not
configurable per-record.

#### Serialize (`insert_indexed_coords`)

Split at 50. First 50 go under `LocationCount` + `X`/`Y`; remainder go under
`EXTRALOCATIONCOUNT` + `EX`/`EY`.

```rust
pub(crate) fn insert_indexed_coords(
    &mut self,
    count_key: &str,
    x_prefix: &str,
    y_prefix: &str,
    points: &[CoordPoint],
) {
    let base_count = points.len().min(50);
    let extra_count = points.len().saturating_sub(50);

    self.insert(count_key, base_count.to_param_value());

    // Base vertices
    for (i, point) in points[..base_count].iter().enumerate() {
        let idx = i + 1;
        if point.x.to_internal() != 0 {
            let x_key = format!("{x_prefix}{idx}");
            let x_frac_key = format!("{x_prefix}{idx}_Frac");
            self.insert_coord(&x_key, &x_frac_key, point.x);
        }
        if point.y.to_internal() != 0 {
            let y_key = format!("{y_prefix}{idx}");
            let y_frac_key = format!("{y_prefix}{idx}_Frac");
            self.insert_coord(&y_key, &y_frac_key, point.y);
        }
    }

    // Extra vertices (if any)
    if extra_count > 0 {
        self.insert(EXTRA_LOCATION_COUNT, extra_count.to_param_value());
        for (i, point) in points[base_count..].iter().enumerate() {
            let idx = base_count + i + 1; // continues from 51
            if point.x.to_internal() != 0 {
                let x_key = format!("EX{idx}");
                let x_frac_key = format!("EX{idx}_Frac");
                self.insert_coord(&x_key, &x_frac_key, point.x);
            }
            if point.y.to_internal() != 0 {
                let y_key = format!("EY{idx}");
                let y_frac_key = format!("EY{idx}_Frac");
                self.insert_coord(&y_key, &y_frac_key, point.y);
            }
        }
    }
}
```

### Option B: New derive attribute (NOT recommended)

Adding a new `#[param(indexed_coords_ex, ...)]` or extra args to the existing attribute
would complicate the macro for what is fundamentally a serialization detail. Since ALL
V5 vertex-bearing records use `includeExLocations: true`, there's no need for per-field
configurability.

## Testing

1. Create a test with >50 vertices, serialize, deserialize, verify roundtrip.
2. Run `cargo test -p altium-format --features test-fixtures` to verify no regressions.
3. The 84 previously-failing SchDoc files should now parse successfully.

## Summary of Changes Required

| File | Change |
|---|---|
| `crates/altium-format/src/param_collection.rs` | Update `remove_indexed_coords` to read `EXTRALOCATIONCOUNT` + `EX`/`EY` overflow |
| `crates/altium-format/src/param_collection.rs` | Update `insert_indexed_coords` to split at 50 and emit `EXTRALOCATIONCOUNT` + `EX`/`EY` |
| No derive macro changes | The existing `indexed_coords` attribute works as-is |
| No struct changes | `Vec<CoordPoint>` already supports arbitrary length |
| No constant additions | `EXTRA_LOCATION_COUNT`, `EX`, `EY` already exist |
