# Milestone 2: Base Record Types

**Files**: `crates/altium-format/src/sch_records.rs`

**Depends on**: M1 (Derive Macros)

## Requirements

Define the base composition types shared by all schematic records, plus the `SchRecord` enum for dispatch. These types form the foundation for all subsequent record implementations.

## Record Hierarchy

Altium schematic records use a two-level base composition:

```
SchPrimitiveBase          (ownership, part/display mode, locking)
    |
    v
SchGraphicalBase          (extends Primitive + location, color, area_color)
    |
    v
Concrete records          (SchRectangle, SchLine, SchArc, etc.)
```

Not all records extend SchGraphicalBase — some (like SchImplementationList) only extend SchPrimitiveBase.

## Structs to Define

### SchPrimitiveBase

| Field | Type | Param Key | Default |
|-------|------|-----------|---------|
| `owner_index` | `i32` | `OWNER_INDEX` | -1 |
| `is_not_accessible` | `bool` | `IS_NOT_ACCESSIBLE` | false |
| `owner_part_id` | `i32` | `OWNER_PART_ID` | -1 |
| `owner_part_display_mode` | `i32` | `OWNER_PART_DISPLAY_MODE` | 0 |
| `graphically_locked` | `bool` | `GRAPHICALLY_LOCKED` | false |
| `index_in_sheet` | `i32` | `INDEX_IN_SHEET` | -1 |

### SchGraphicalBase

| Field | Type | Param Key | Default |
|-------|------|-----------|---------|
| `primitive` | `SchPrimitiveBase` | (flatten) | - |
| `location` | `CoordPoint` | `LOCATION_X`/`_FRAC`, `LOCATION_Y`/`_FRAC` | - |
| `color` | `Color` | `COLOR` | black |
| `area_color` | `Color` | `AREA_COLOR` | black |

### SchRecord enum

```rust
pub(crate) enum SchRecord {
    Component(SchComponent),
    Pin(SchPin),
    Label(SchLabel),
    Bezier(SchBezier),
    Polyline(SchPolyline),
    Polygon(SchPolygon),
    Ellipse(SchEllipse),
    Pie(SchPie),
    RoundRectangle(SchRoundRectangle),
    EllipticalArc(SchEllipticalArc),
    Arc(SchArc),
    Line(SchLine),
    Rectangle(SchRectangle),
    TextFrame(SchTextFrame),
    Image(SchImage),
    Designator(SchDesignator),
    Parameter(SchParameter),
    ImplementationList(SchImplementationList),
    Implementation(SchImplementation),
    ImplementationMap(SchImplementationMap),
    MapDefiner(SchMapDefiner),
    ImplementationParameters(SchImplementationParameters),
    // Additional variants added as needed during red/green development
}
```

### SchLibComponent (container for one library component)

```rust
pub(crate) struct SchLibComponent {
    pub component: SchComponent,
    pub records: Vec<SchRecord>,
}
```

## Acceptance Criteria

- SchPrimitiveBase and SchGraphicalBase use `#[derive(FromParams)]` and parse correctly
- SchRecord enum covers all implemented record types
- All types are `pub(crate)` (implementation details of altium-format)
- Module declared in `lib.rs` and accessible from `schlib.rs`

## Tests

- **Test files**: `crates/altium-format/src/sch_records.rs` (inline `#[cfg(test)]` module)
- **Test type**: unit
- **Backing**: doc-derived
- **Scenarios**:
  - Normal: SchPrimitiveBase parses all fields with defaults
  - Normal: SchGraphicalBase flattens SchPrimitiveBase correctly
  - Edge: fractional coordinates reconstruct correctly (e.g., LOCATION.X=100, LOCATION.X_FRAC=50000 -> 10,050,000 internal units)
  - Edge: missing optional fields use defaults

## Code Intent

- New file `crates/altium-format/src/sch_records.rs`:
  - `SchPrimitiveBase` struct with `#[derive(FromParams)]`
  - `SchGraphicalBase` struct with `#[derive(FromParams)]` and `#[param(flatten)]` for primitive base
  - `SchRecord` enum (variants added incrementally as record types are implemented)
  - `SchLibComponent` struct holding component record + child records
- Modify `crates/altium-format/src/lib.rs`:
  - Add `mod sch_records;` declaration
