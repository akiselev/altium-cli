# Milestone 3: FileHeader + SectionKeys

**Files**: `crates/altium-format/src/schlib.rs`

**Depends on**: M1 (Derive Macros)

## Requirements

Parse the `/FileHeader` and `/SectionKeys` CFB streams. FileHeader contains the library header string, font table, display settings, and the component index (names, descriptions, part counts, alias counts). SectionKeys maps full component names to their short CFB storage keys for names exceeding the 31-character OLE limit.

## FileHeader Stream Format

Single parameter text block (flags=0x00) containing:

### Header validation
- `HEADER` must equal `"Protel for Windows - Schematic Library Editor Binary File Version 5.0"`
  - Use constant: `file_headers::SCH_LIBRARY_BINARY_HEADER_V50`

### Library metadata
| Key | Type | Notes |
|-----|------|-------|
| `Weight` | i32 | Total primitive count + alias count |
| `MinorVersion` | i32 | Format version (2 or 9) |
| `UniqueID` | String | Library-level unique identifier |

### Font table (1-based indexing)
| Key Pattern | Type | Notes |
|-------------|------|-------|
| `FontIdCount` | i32 | Number of fonts |
| `Size{N}` | i32 | Font size for font N (1-based) |
| `FontName{N}` | String | Font name |
| `Italic{N}` | bool | Italic flag |
| `Bold{N}` | bool | Bold flag |
| `Underline{N}` | bool | Underline flag |
| `StrikeOut{N}` | bool | Strikeout flag |
| `Rotation{N}` | i32 | Rotation angle |

### Display settings
| Key | Type | Default | Notes |
|-----|------|---------|-------|
| `AreaColor` | Color | | Background color |
| `SnapGridSize` | String | | Snap grid (e.g., "10") |
| `VisibleGridSize` | String | | Visible grid |
| `SheetStyle` | i32 | 0 | Sheet style enum |
| `BorderOn` | bool | true | Show borders |
| `UseCustomSheet` | bool | false | Custom sheet dimensions |
| Additional display keys... | | | Parsed but may be skipped initially |

### Component index (0-based)
| Key Pattern | Type | Notes |
|-------------|------|-------|
| `CompCount` | i32 | Number of components |
| `LibRef{N}` | String | Component name (0-based index) |
| `CompDescr{N}` | String | Component description |
| `PartCount{N}` | i32 | Number of parts |
| `AliasCount{N}` | i32 | Number of aliases |
| `Comp{N}Alias{M}` | String | Alias M of component N |

## SectionKeys Stream Format

Single parameter text block (flags=0x00), optional (only present when any component name exceeds 31 characters):

| Key Pattern | Type | Notes |
|-------------|------|-------|
| `RECORD` | i32 | Always 0 |
| `KeyCount` | i32 | Number of mappings |
| `LibRef{N}` | String | Full component name |
| `SectionKey{N}` | String | Short CFB storage key |

## Structs to Define

### SchLibHeader
```rust
pub(crate) struct SchLibHeader {
    pub weight: i32,
    pub minor_version: i32,
    pub unique_id: String,
    pub fonts: Vec<SchFont>,
    pub components: Vec<SchLibComponentIndex>,
    // Display settings as needed
}
```

### SchLibComponentIndex
```rust
pub(crate) struct SchLibComponentIndex {
    pub lib_ref: String,        // full component name
    pub description: String,
    pub part_count: i32,
    pub aliases: Vec<String>,
}
```

## Acceptance Criteria

- FileHeader parses header string and validates against expected constant
- Font table parsed with correct 1-based indexing
- Component index parsed with all names, descriptions, part counts, and aliases
- SectionKeys parsed when present, building name-to-key HashMap
- Missing SectionKeys stream handled gracefully (no mapping needed)
- Error on unexpected header string
- All parameter keys consumed (assert_exhausted)

## Tests

- **Test files**: `crates/altium-format/src/schlib.rs` (inline `#[cfg(test)]` module)
- **Test type**: integration (uses real SchLib test files)
- **Backing**: doc-derived (docs/schlib/fileheader.md, docs/schlib/aliases-and-sectionkeys.md)
- **Scenarios**:
  - Normal: BlankSchlibComponent.SchLib FileHeader parses (1 component, no aliases)
  - Normal: LimeMicroAltiumLib_schLib.SchLib FileHeader parses (200 components)
  - Normal: SectionKeys parsed when present
  - Edge: missing SectionKeys stream returns empty map
  - Error: wrong header string produces descriptive error

## Code Intent

- Expand `crates/altium-format/src/schlib.rs` from stub to implement:
  - `SchLibHeader` struct (hand-written parsing due to indexed fields — font table and component index use numbered key patterns that don't map cleanly to derive)
  - `parse_file_header(data: &[u8]) -> Result<SchLibHeader>` function
  - `parse_section_keys(data: &[u8]) -> Result<HashMap<String, String>>` function
  - Helper: `resolve_component_key(name, section_keys) -> String` for name-to-CFB-key resolution
- FileHeader parsing uses ParameterCollection with indexed access patterns (`remove_indexed`)
- Font table uses 1-based indexing (FontName1, FontName2, ...)
- Component index uses 0-based indexing (LibRef0, LibRef1, ...)
