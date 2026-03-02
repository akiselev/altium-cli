# Phase 2: PcbDoc Read Path

## Goal

Implement `PcbDoc::board() -> Result<PcbDocBoard>` — convert internal types to
public API types, resolving cross-references and generating stable IDs.

## File: `api/pcbdoc_read.rs`

### Core Function

```rust
pub(crate) fn board_from_internal(doc: &PcbDoc) -> Result<PcbDocBoard>
```

### Implementation Steps

#### Step 1: Build Index Tables

Before converting primitives, build lookup tables from parameter sections:

```rust
// Net index -> net name
let net_names: Vec<Option<String>> = build_net_name_table(doc);

// Component index -> designator
let component_designators: Vec<Option<String>> = build_component_designator_table(doc);

// Wide string index -> text
let wide_strings: Vec<String> = build_wide_string_table(doc);
```

These are built by scanning `doc.sections` for `ParamSectionKind::Nets6`,
`ParamSectionKind::Components6`, and the `WideStrings` section.

#### Step 2: Convert Named Collections

For each parameter section, extract records and convert to API types:

- `Nets6` records -> `Vec<Net>` (extract NAME, COLOR, VISIBLE)
- `Components6` records -> `Vec<PcbDocComponent>` (extract SOURCEDESIGNATOR,
  PATTERN, X, Y, ROTATION, LAYER, etc.)
- `Polygons6` records -> `Vec<Polygon>` (extract NET, LAYER, vertices, thermal)
- `Classes6` records -> `Vec<NetClass>` (extract NAME, KIND, MEMBERS)
- `Dimensions6` records -> `Vec<Dimension>` (from prefixed params)

DRC rules: convert `doc.rules` -> `Vec<DesignRule>`.

#### Step 3: Convert Primitives

For each `PrimitiveSectionData` in `doc.sections`, convert records to API types.
Each primitive gets:

1. **ID**: `{type}_{index}` where index is position within its section
2. **Layer**: from `common.layer` (V6Layer -> LayerRef)
3. **Net**: resolve `common.net_index` via net_names table
4. **Component**: resolve `common.component_index` via component_designators table
5. **Type-specific fields**: geometry, properties

Conversion functions per type:
```rust
fn track_from_internal(idx: usize, track: &PcbTrack, ctx: &ConvertContext) -> Track
fn arc_from_internal(idx: usize, arc: &PcbArc, ctx: &ConvertContext) -> Arc
fn via_from_internal(idx: usize, via: &PcbVia, ctx: &ConvertContext) -> Via
fn pad_from_internal(idx: usize, pad: &PcbPad, ctx: &ConvertContext) -> Pad
fn fill_from_internal(idx: usize, fill: &PcbFill, ctx: &ConvertContext) -> Fill
fn text_from_internal(idx: usize, text: &PcbText, ctx: &ConvertContext) -> Text
fn region_from_internal(idx: usize, region: &PcbRegion, ctx: &ConvertContext) -> Region
fn body_from_internal(idx: usize, body: &PcbComponentBody, ctx: &ConvertContext) -> ComponentBody
```

Where `ConvertContext` holds the lookup tables and wide strings.

#### Step 4: Handle Section Pairs

PcbDoc has legacy + modern pairs for some sections:
- `Regions6` (legacy) + `ShapeBasedRegions6` (modern) — use ShapeBasedRegions6
  as authoritative, skip Regions6
- `ComponentBodies6` + `ShapeBasedComponentBodies6` — same pattern
- `Texts` (legacy) + `Texts6` (modern) — use Texts6

#### Step 5: Handle Text Wide Strings

`PcbText` has `wide_string_index: i32`. If >= 0, the actual text comes from
the WideStrings6 section at that index. Otherwise use the inline `text` field.

#### Step 6: Board Settings

Extract from Board6 parameter section:
- `DOCUMENTNAME` -> document_name
- Layer count from layer stack parameters
- Grid sizes from SNAPGRIDSIZE, VISIBLEGRIDSIZE
- Board outline: find the Region primitive that defines the board perimeter
  (region_kind = BoardOutline or the board's outline reference)

### Wire-up

Add `board()` method to PcbDoc:
```rust
impl PcbDoc {
    pub fn board(&self) -> Result<crate::api::PcbDocBoard> {
        crate::api::pcbdoc_read::board_from_internal(self)
    }
}
```

### Testing Strategy

1. **Unit test**: Open a real PcbDoc fixture, call `board()`, verify net count,
   component count, primitive counts match expected values.
2. **Smoke test**: Every field accessible without panic.
3. **Cross-reference test**: Verify pads have correct net/component names by
   checking a known pad.

### Estimated Scope

- ~500-800 lines
- Main complexity: parameter extraction from ParameterCollection, section pair handling
- Reuse: can share some primitive conversion logic with pcblib_read.rs

### Dependencies

- Phase 1 (types) must be complete
- Needs access to internal PcbDoc types (already pub(crate))
