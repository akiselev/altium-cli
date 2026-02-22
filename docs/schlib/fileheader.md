# FileHeader Stream

The `/FileHeader` stream is a **single parameter text block** (flags=0x00) containing all
library-wide header data as pipe-delimited `key=value` pairs in Windows-1252 encoding.

## Header string

The `HEADER` key must equal exactly:

```
Protel for Windows - Schematic Library Editor Binary File Version 5.0
```

## Keys in the FileHeader block

### Core header fields

| Key | Type | Description |
|-----|------|-------------|
| `HEADER` | string | Must equal the header string above |
| `Weight` | i32 | Total primitive count + alias count across all components |
| `MinorVersion` | i32 | Format minor version (9 for current files, 2 for older) |
| `UniqueID` | string | Library-level unique identifier |

### Font table

Fonts use 1-based indexing. `FontIdCount` gives the total count.

| Key | Type | Description |
|-----|------|-------------|
| `FontIdCount` | i32 | Number of fonts in the table |
| `Size{N}` | i32 | Font size in points |
| `Rotation{N}` | i32 | Font rotation |
| `Underline{N}` | bool | Underline flag |
| `Italic{N}` | bool | Italic flag |
| `Bold{N}` | bool | Bold flag |
| `StrikeOut{N}` | bool | Strikethrough flag |
| `FontName{N}` | string | Font face name (e.g. "Times New Roman") |

### Display settings

| Key | Type | Description |
|-----|------|-------------|
| `UseMBCS` | bool | Use multi-byte character set (always T) |
| `IsBOC` | bool | (always T) |
| `SheetStyle` | i32 | Sheet style enum (9 = custom) |
| `BorderOn` | bool | Show border |
| `SheetNumberSpaceSize` | i32 | Sheet number space size |
| `AreaColor` | i32 | Background area color (COLORREF) |
| `SnapGridOn` | bool | Snap grid enabled |
| `SnapGridSize` | i32 | Snap grid size |
| `VisibleGridOn` | bool | Visible grid enabled |
| `VisibleGridSize` | i32 | Visible grid size |
| `CustomX` | i32 | Custom sheet width |
| `CustomY` | i32 | Custom sheet height |
| `UseCustomSheet` | bool | Use custom sheet size (always T) |
| `ReferenceZonesOn` | bool | Show reference zones |
| `Display_Unit` | i32 | Display unit enum |

### Component index

The component index enumerates every component in the library. Indices are 0-based.
Alias indices within each component are also 0-based.

| Key | Type | Description |
|-----|------|-------------|
| `CompCount` | i32 | Number of components |
| `LibRef{N}` | string | Component name (the library reference) |
| `CompDescr{N}` | string | Human-readable description |
| `PartCount{N}` | i32 | Number of parts in this component |
| `AliasCount{N}` | i32 | Number of aliases for this component |
| `Comp{N}Alias{M}` | string | Alias M of component N (0-based M) |

### Weight calculation

`Weight` = sum over all components of:
- Number of primitives in the component's `Data` stream (excluding the SchComponent
  record itself and any end marker)
- Plus the number of aliases for that component

## Example values from real files

| File | CompCount | Weight | MinorVersion | FontIdCount |
|------|-----------|--------|--------------|-------------|
| BlankSchlibComponent.SchLib | 1 | 5 | 9 | 1 |
| LimeMicroAltiumLib.SchLib | 200 | 12461 | 2 | 8 |
| Synthiam.SchLib | 173 | 5381 | 9 | 4 |
