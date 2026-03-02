# Phase 3: PcbDoc Write Path

## Goal

Implement `PcbDoc::update_board(&mut self, board: &PcbDocBoard) -> Result<()>` —
convert public API types back to internal types, preserving format-internal fields.

## File: `api/pcbdoc_write.rs`

### Core Function

```rust
pub(crate) fn update_board_internal(
    doc: &mut PcbDoc,
    board: &PcbDocBoard,
) -> Result<()>
```

### Implementation Strategy

Follow the SchDoc `update_sheet()` pattern:

1. **Read the current state** via `board_from_internal()` to get the existing
   API-level view (needed for preservation matching)
2. **Build reverse lookup tables**: net name -> index, designator -> index
3. **Convert each API collection back to internal sections**
4. **Replace the relevant sections** in `doc.sections`
5. **Validate invariants** after replacement

### Preservation Model

Format-internal fields NOT in the public API must be preserved during updates.
Matching by `id` (or positional fallback) determines which existing records to
pull preserved fields from.

**Primitive fields to preserve:**
- `PcbPrimitiveCommon::flags` (except bits reflected in API)
- `PcbPrimitiveCommon::coordinate_index`, `dimension_index`
- `PcbPrimitiveCommon::polygon_index`
- `subpoly_index`, `user_routed`, `union_index`
- `keepout_restrictions`
- Pad: full pad stack data, thermal relief entries, template links, cache
- Via: per-layer diameters, IPC-4761 structure, template links, cache states
- Text: font details, barcode properties, sentinels, snap points
- Region: shape-based flag, arc resolution, pad index, board region specifics

**Named collection fields to preserve:**
- Components6: all GUID/path fields (SOURCEUNIQUEID, SOURCEHIERARCHICALPATH, etc.)
- Nets6: per-layer minimum routing widths
- Polygons6: all advanced pour settings not in API
- Rules: all rule-specific data beyond what DesignRule exposes

### Conversion Functions

```rust
fn track_to_internal(track: &Track, existing: Option<&PcbTrack>, ctx: &WriteContext) -> PcbTrack
fn arc_to_internal(arc: &Arc, existing: Option<&PcbArc>, ctx: &WriteContext) -> PcbArc
// ... etc for each type
```

Where `existing` provides the preservation source (None for new objects).

### ID-Based Matching

For each primitive type:
1. Build a map: `id -> (section_index, record)` from the current internal state
2. For each API object, look up its `id` in the map
3. If found: update with preservation
4. If not found: create new (no preservation)

### Section Replacement

Replace entire sections rather than patching individual records:
```rust
fn rebuild_primitive_section(
    kind: PrimitiveSectionKind,
    primitives: &[ParsedPrimitiveRecord],
) -> PcbDocSection
```

### Sidecar Regeneration

After rebuilding primitive sections, regenerate sidecar sections:
- WideStrings6: rebuild from all text primitives
- UniqueIDPrimitiveInformation: rebuild from all primitives with unique_ids
- PrimitiveGuids: rebuild from all primitives

### Testing Strategy

1. **Roundtrip test**: `board()` -> `update_board()` -> `save()` -> semantic diff
   against original. Should be identical (no changes).
2. **Mutation test**: Modify a net name, save, reopen, verify change persists.
3. **Preservation test**: Modify one field, verify all other fields preserved.

### Estimated Scope

- ~600-1000 lines
- Main complexity: preservation matching, sidecar regeneration
- This is the hardest phase due to the number of fields to preserve

### Dependencies

- Phase 1 (types) and Phase 2 (read) must be complete
- Read path needed for building the "existing" state for preservation
