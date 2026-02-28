# Milestone 6: Graphical Primitive Records

**Files**: `crates/altium-format/src/sch_records.rs`

**Depends on**: M2 (Base Types)

## Requirements

Implement parameter-based parsing for all graphical primitive record types. These records represent the visual shapes that compose schematic symbols. All use `#[derive(FromParams)]` with `SchGraphicalBase` flattened.

## Record Types

### SchLine (RECORD=13)

| Field | Type | Param Key | Default |
|-------|------|-----------|---------|
| base | SchGraphicalBase | (flatten) | - |
| corner | CoordPoint | `CORNER.X`/`_FRAC`, `CORNER.Y`/`_FRAC` | - |
| line_width | i32 | `LINEWIDTH` | 1 (Small) |
| line_style | i32 | `LINESTYLE` | 0 (Solid) |

### SchRectangle (RECORD=14)

| Field | Type | Param Key | Default |
|-------|------|-----------|---------|
| base | SchGraphicalBase | (flatten) | - |
| corner | CoordPoint | `CORNER.X`/`_FRAC`, `CORNER.Y`/`_FRAC` | - |
| is_solid | bool | `ISSOLID` | false |
| line_width | i32 | `LINEWIDTH` | 1 |
| transparent | bool | `TRANSPARENT` | true |
| corner_x_radius | i32 | `CORNERXRADIUS` | 0 |
| corner_y_radius | i32 | `CORNERYRADIUS` | 0 |

### SchRoundRectangle (RECORD=10)

Same fields as SchRectangle (corner radii are meaningful here).

### SchArc (RECORD=12)

| Field | Type | Param Key | Default |
|-------|------|-----------|---------|
| base | SchGraphicalBase | (flatten) | - |
| radius | Coord | `RADIUS`/`RADIUS_FRAC` | - |
| start_angle | f64 | `STARTANGLE` | 0.0 |
| end_angle | f64 | `ENDANGLE` | 360.0 |
| line_width | i32 | `LINEWIDTH` | 1 |

Note: Angles are stored as Real48 (Borland Turbo Pascal format) in some contexts, but in SchLib parameter text they are plain decimal strings.

### SchEllipticalArc (RECORD=11)

| Field | Type | Param Key | Default |
|-------|------|-----------|---------|
| base | SchGraphicalBase | (flatten) | - |
| radius | Coord | `RADIUS`/`RADIUS_FRAC` | - |
| secondary_radius | Coord | `SECONDARYRADIUS`/`SECONDARYRADIUS_FRAC` | - |
| start_angle | f64 | `STARTANGLE` | 0.0 |
| end_angle | f64 | `ENDANGLE` | 360.0 |
| line_width | i32 | `LINEWIDTH` | 1 |

### SchEllipse (RECORD=8)

| Field | Type | Param Key | Default |
|-------|------|-----------|---------|
| base | SchGraphicalBase | (flatten) | - |
| radius | Coord | `RADIUS`/`RADIUS_FRAC` | - |
| secondary_radius | Coord | `SECONDARYRADIUS`/`SECONDARYRADIUS_FRAC` | - |
| is_solid | bool | `ISSOLID` | false |
| line_width | i32 | `LINEWIDTH` | 1 |
| transparent | bool | `TRANSPARENT` | true |

### SchPie (RECORD=9)

Like SchEllipse + start_angle/end_angle.

### SchPolyline (RECORD=6)

| Field | Type | Param Key | Default |
|-------|------|-----------|---------|
| base | SchGraphicalBase | (flatten) | - |
| vertices | Vec<CoordPoint> | `LOCATIONCOUNT`, `X{N}`, `Y{N}` (indexed) | - |
| line_width | i32 | `LINEWIDTH` | 1 |
| line_style | i32 | `LINESTYLE` | 0 |
| line_shape | i32 | `LINESHAPE` | 0 |
| start_line_shape | i32 | `STARTLINESHAPE` | 0 |
| end_line_shape | i32 | `ENDLINESHAPE` | 0 |

Note: Vertex coordinates use indexed params: `X1`, `Y1`, `X2`, `Y2`, etc. (1-based). Use `remove_indexed_coords()`.

### SchPolygon (RECORD=7)

Like SchPolyline + `ISSOLID`, `TRANSPARENT`.

### SchBezier (RECORD=5)

Like SchPolyline (4 control points = 4 vertices).

### SchImage (RECORD=30)

| Field | Type | Param Key | Default |
|-------|------|-----------|---------|
| base | SchGraphicalBase | (flatten) | - |
| corner | CoordPoint | `CORNER.X`/`_FRAC`, `CORNER.Y`/`_FRAC` | - |
| embed_image | bool | `EMBEDIMAGE` | false |
| filename | String | `FILENAME` | "" |
| keep_aspect | bool | `KEEPASPECT` | true |

Image binary data comes from the `/Storage` stream (matched by FILENAME), handled in Milestone 10.

## Acceptance Criteria

- All graphical primitive records parse via `#[derive(FromParams)]`
- Coordinate pairs use coord/coord_point attributes for DXP fractional encoding
- Indexed vertices (Polyline, Polygon, Bezier) use `indexed_coords` attribute
- All parameter keys consumed (assert_exhausted at dispatch site)
- `altium validate` progresses past graphical records in test files
- Unknown parameters surface as errors for investigation

## Tests

- **Test files**: `crates/altium-format/src/sch_records.rs` (inline `#[cfg(test)]`)
- **Test type**: unit + integration
- **Backing**: doc-derived (docs/schlib/record-types.md)
- **Scenarios**:
  - Normal: each graphical record type parses from constructed parameter data
  - Normal: polyline with multiple vertices
  - Normal: arc with start/end angles
  - Edge: rectangle with corner radii (round rectangle variant)
  - Edge: polygon with ISSOLID=T
  - Edge: elliptical arc with secondary radius
  - Error: missing required field produces MissingParam

## Code Intent

- Add to `crates/altium-format/src/sch_records.rs`:
  - `SchLine`, `SchRectangle`, `SchRoundRectangle`, `SchArc`, `SchEllipticalArc`, `SchEllipse`, `SchPie`, `SchPolyline`, `SchPolygon`, `SchBezier`, `SchImage` structs
  - All use `#[derive(FromParams)]` with `#[param(flatten)]` for base
  - Add variants to `SchRecord` enum
  - Add arms to `dispatch_record()` match in schlib.rs
- Vertex-based records use `#[param(indexed_coords, ...)]` for vertex lists
- SchImage stores filename only — binary data merged from Storage in M10
