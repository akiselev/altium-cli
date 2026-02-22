# Additional Stream

The `Additional` stream contains supplementary records that are not part of the main
schematic object hierarchy. In practice, this stream contains RECORD=225 dashed rectangle
overlay records.

## Stream layout

```
Block 0:    Header record (no RECORD key)
Block 1..N: RECORD=225 records (dashed rectangles)
```

## Block 0: Header

| Key | Type | Description |
|-----|------|-------------|
| `HEADER` | string | `Protel for Windows - Schematic Capture Binary File Version 5.0` |
| `Weight` | i32 | Number of records that follow (optional; absent when 0) |

When no supplementary records exist, the `Weight` key is absent and the stream contains
only the header block (75 bytes total).

## RECORD=225: Dashed rectangle overlay

These are rectangular annotation overlays drawn with dashed lines, typically used for
grouping or highlighting regions of the schematic. They appear in the Additional stream
rather than the FileHeader stream.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `RECORD` | i32 | 225 | Always 225 |
| `OWNERPARTID` | i32 | -1 | Always -1 (sheet-level) |
| `INDEXINSHEET` | i32 | | Sequential index |
| `LOCATION.X` + `LOCATION.X_FRAC` | i32 | | Bottom-left corner X |
| `LOCATION.Y` + `LOCATION.Y_FRAC` | i32 | | Bottom-left corner Y |
| `CORNER.X` + `CORNER.X_FRAC` | i32 | | Top-right corner X |
| `CORNER.Y` + `CORNER.Y_FRAC` | i32 | | Top-right corner Y |
| `COLOR` | i32 | 255 | Line color (COLORREF) |
| `AREACOLOR` | i32 | 16777215 | Fill color (COLORREF, typically white/transparent) |
| `LINESTYLE` | i32 | 1 | Line style (1 = dashed) |
| `LINESTYLEEXT` | i32 | 1 | Extended line style |
| `LOCATIONCOUNT` | i32 | 4 | Always 4 (rectangle corners) |
| `X1` + `X1_FRAC` | i32 | | Vertex 1 X |
| `Y1` + `Y1_FRAC` | i32 | | Vertex 1 Y |
| `X2` + `X2_FRAC` | i32 | | Vertex 2 X |
| `Y2` + `Y2_FRAC` | i32 | | Vertex 2 Y |
| `X3` + `X3_FRAC` | i32 | | Vertex 3 X |
| `Y3` + `Y3_FRAC` | i32 | | Vertex 3 Y |
| `X4` + `X4_FRAC` | i32 | | Vertex 4 X |
| `Y4` + `Y4_FRAC` | i32 | | Vertex 4 Y |
| `UNIQUEID` | string | | 8-character unique identifier |

The 4 vertices define the rectangle corners in order (bottom-left, bottom-right,
top-right, top-left).

## Loading behavior

The Additional stream is loaded during the `ImportAdditionalWarehouse` step of the loading
pipeline. Records are added to the `AdditionalWarehouse` list, separate from the main
`BaseWarehouse`. They can reference objects in the BaseWarehouse via `OWNERINDEX`.

## Presence patterns

From 9 LimeSDR SchDoc files:

| Files | Additional stream contents |
|-------|--------------------------|
| 01, 02, 03, 06 | Header only (75 bytes, no RECORD=225) |
| 04 | Header + 4 RECORD=225 entries |
| 05 | Header + 6 RECORD=225 entries |
| 07 | Header + 5 RECORD=225 entries |
| 08 | Header + 6 RECORD=225 entries |
| 09 | Header + 2 RECORD=225 entries |

Simple diagram-only sheets have no dashed rectangles. Complex schematics with component
grouping annotations have several.
