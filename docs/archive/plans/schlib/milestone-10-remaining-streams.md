# Milestone 10: Storage, Additional Streams, Aliases

**Files**: `crates/altium-format/src/schlib.rs`

**Depends on**: M4 (Data Stream), M6 (Graphical — for SchImage), M7 (Text), M8 (Implementation)

## Requirements

Implement the remaining CFB streams: `/Storage` for embedded images, per-component `/<key>/Additional` for overflow records, and `/<alias>/Redirection` for alias resolution. These complete the 3-phase loading pipeline.

## 1. Storage Stream (Phase 2)

The `/Storage` stream contains embedded binary objects (images) using the embedded object envelope format. Each entry corresponds to a SchImage record matched by filename.

### Format
- Block 0: text header with `RECORD=0`, `HEADER`, `Weight`
- Blocks 1..N: embedded objects, each containing:
  - `0xD0` tag + ID + inner data
  - Inner data is the raw image bytes (PNG, BMP, etc.)

### Merge Logic
```
for each embedded_object in storage:
    find SchImage record where image.filename == embedded_object.id
    image.data = embedded_object.inner_data
```

### Notes
- Storage stream is optional (only present if component has SchImage records)
- The embedded object ID is the filename, not a numeric index
- Must read via `cfb.read_stream_optional()` and mark as consumed

## 2. Additional Streams (Phase 3)

Some primitives have `OwnerIndexAdditionalList=true` (bit 7 of PinConglomerate for pins). These records are stored in a separate `/<key>/Additional` stream instead of the main Data stream.

### LibAdditional Header
- `/LibAdditional` stream: single text block with `RECORD=0` + library-level additional metadata
- If absent, skip the entire additional phase

### Per-Component Additional
- `/<key>/Additional` stream: same block format as Data stream
- Records parsed via same `dispatch_record()` function
- OwnerIndex values are relative to the Additional stream's scope
- Records appended to the component's record list

### Flow
```
if cfb.exists("/LibAdditional"):
    read and consume /LibAdditional
    for each component:
        if cfb.exists("/<key>/Additional"):
            parse_additional_stream() -> Vec<SchRecord>
            append to component.records
```

## 3. Alias Resolution

Aliases are alternate names that redirect to canonical components. Each alias has a CFB sub-storage with a Redirection stream.

### Alias Discovery
- Aliases listed in FileHeader component index (`AliasCount{N}`, `Comp{N}Alias{M}`)
- Each alias name mapped to a CFB key (same truncation rules as components)

### Redirection Stream Format
Single text block: `|RECORD=0|SectionName=<canonical_component_name>|`

Where `SectionName` is the full canonical component name (not the CFB key).

### SchLib Alias Storage
```rust
pub(crate) struct SchLibAlias {
    pub alias_name: String,
    pub canonical_name: String,
}
```

Aliases stored in `SchLib.aliases: Vec<SchLibAlias>` — simple name mapping.

### Flow
```
for each component in header.components:
    for each alias in component.aliases:
        alias_key = resolve_component_key(alias, section_keys)
        read /<alias_key>/Redirection stream
        parse -> SectionName value
        store alias -> canonical_name mapping
```

## Acceptance Criteria

- Storage stream parsed and images merged into SchImage records by filename
- Additional streams parsed and records appended to components
- LibAdditional stream handled (consumed or skipped if absent)
- Alias Redirection streams parsed and alias mappings stored
- All alias CFB storages consumed (no unconsumed streams from aliases)
- Missing optional streams handled gracefully
- `altium validate` passes all stream consumption checks

## Tests

- **Test files**: `crates/altium-format/src/schlib.rs` (inline `#[cfg(test)]`)
- **Test type**: integration (real SchLib files)
- **Backing**: doc-derived (docs/schlib/loading-pipeline.md, docs/schlib/aliases-and-sectionkeys.md)
- **Scenarios**:
  - Normal: Storage stream with embedded images
  - Normal: Additional stream with overflow records
  - Normal: alias resolves to canonical component
  - Edge: no Storage stream (component has no images)
  - Edge: no Additional streams (no overflow records)
  - Edge: no aliases (empty alias list)
  - Edge: alias name > 31 chars (uses SectionKeys)

## Code Intent

- Add to `crates/altium-format/src/schlib.rs`:
  - `parse_storage_stream(data: &[u8]) -> Result<Vec<EmbeddedObject>>` — parse embedded images
  - `merge_storage_into_records(storage: &[EmbeddedObject], records: &mut [SchRecord])` — match images to SchImage records
  - `parse_additional_stream(data: &[u8]) -> Result<Vec<SchRecord>>` — parse overflow records (reuses dispatch_record)
  - `parse_redirection(data: &[u8]) -> Result<String>` — extract SectionName from Redirection stream
  - `SchLibAlias` struct
- Storage uses existing `parse_embedded_object_stream()` from embedded_object.rs
- Additional stream uses same block parsing and record dispatch as Data stream
- All streams read through TrackedCfbDocument to ensure consumption tracking
