# Milestone 11: SchLib Document Assembly

**Files**: `crates/altium-format/src/schlib.rs`, `crates/altium-format-ops/src/schlib_ops.rs`

**Depends on**: All previous milestones

## Requirements

Assemble the complete `SchLib::open()` implementation that orchestrates the 3-phase loading pipeline, enforces stream consumption, and connects to the validate CLI command. This is the integration milestone that ties all previous work together.

## SchLib Public API

```rust
pub struct SchLib {
    header: SchLibHeader,
    components: Vec<SchLibComponent>,
    aliases: Vec<SchLibAlias>,
}

impl SchLib {
    pub fn open(path: &Path) -> Result<Self> { ... }
}
```

## Loading Pipeline Implementation

```rust
pub fn open(path: &Path) -> Result<Self> {
    let mut cfb = TrackedCfbDocument::open(path)?;

    // === Phase 1: ImportBaseWarehouse ===

    // 1a. Parse FileHeader
    let header_data = cfb.read_stream("/FileHeader")?;
    let header = parse_file_header(&header_data)?;

    // 1b. Parse SectionKeys (optional)
    let section_keys = match cfb.read_stream_optional("/SectionKeys")? {
        Some(data) => parse_section_keys(&data)?,
        None => HashMap::new(),
    };

    // 1c. Parse per-component Data streams
    let mut components = Vec::with_capacity(header.components.len());
    for comp_index in &header.components {
        let key = resolve_component_key(&comp_index.lib_ref, &section_keys);
        let data = cfb.read_stream(&format!("/{key}/Data"))?;
        let component = parse_component_data(&data)?;
        components.push(component);
    }

    // === Phase 2: ImportExtendedWarehouse ===

    // 2a. Parse Storage stream (embedded images)
    if let Some(storage_data) = cfb.read_stream_optional("/Storage")? {
        let storage_objects = parse_storage_stream(&storage_data)?;
        for component in &mut components {
            merge_storage_into_records(&storage_objects, &mut component.records);
        }
    }

    // 2b. Parse pin sidecar streams for each component
    for (i, comp_index) in header.components.iter().enumerate() {
        let key = resolve_component_key(&comp_index.lib_ref, &section_keys);
        let pins = collect_pins_mut(&mut components[i].records);
        merge_pin_sidecars(&mut cfb, &key, pins)?;
    }

    // === Phase 3: ImportAdditionalWarehouse ===

    // 3a. Check for LibAdditional
    if let Some(lib_additional_data) = cfb.read_stream_optional("/LibAdditional")? {
        let _lib_additional = parse_lib_additional(&lib_additional_data)?;

        // 3b. Per-component Additional streams
        for (i, comp_index) in header.components.iter().enumerate() {
            let key = resolve_component_key(&comp_index.lib_ref, &section_keys);
            let additional_path = format!("/{key}/Additional");
            if let Some(additional_data) = cfb.read_stream_optional(&additional_path)? {
                let additional_records = parse_additional_stream(&additional_data)?;
                components[i].records.extend(additional_records);
            }
        }
    }

    // === Alias Resolution ===
    let mut aliases = Vec::new();
    for comp_index in &header.components {
        for alias_name in &comp_index.aliases {
            let alias_key = resolve_component_key(alias_name, &section_keys);
            let redir_path = format!("/{alias_key}/Redirection");
            let redir_data = cfb.read_stream(&redir_path)?;
            let canonical = parse_redirection(&redir_data)?;
            aliases.push(SchLibAlias {
                alias_name: alias_name.clone(),
                canonical_name: canonical,
            });
        }
    }

    // === Consumption Check ===
    cfb.assert_all_consumed()?;

    Ok(SchLib { header, components, aliases })
}
```

## SchLibOps Validate Implementation

```rust
impl SchLibOps for SchLib {
    fn validate(&self) -> Result<(), AltiumOperationError> {
        // Opening the file already validates everything (fail-fast parsing)
        // Additional semantic validation can be added here:
        // - Font ID references valid (within font table bounds)
        // - OwnerIndex references valid (within component record bounds)
        // - UniqueIDs are unique within library
        Ok(())
    }
}
```

## Key Integration Points

1. **TrackedCfbDocument** ensures every CFB stream is read or acknowledged
2. **assert_exhausted()** on every ParameterCollection ensures no unknown parameters
3. **SchRecordType::try_from()** on every RECORD= value ensures no unknown records
4. **BinaryReader::assert_exhausted()** on binary pin data ensures no trailing bytes
5. **UnknownBinaryCode** on non-0x02 binary blocks ensures no unknown binary formats

## Red/Green Development Loop

The validate CLI command is the primary feedback mechanism:
```bash
cargo run -- validate data/BlankSchlibComponent.SchLib
cargo run -- validate data/LimeMicroAltiumLib_schLib.SchLib
cargo run -- validate data/Synthiam.SchLib
```

Each run will either:
- **Pass**: all streams, records, and parameters handled
- **Fail with specific error**: unknown record type, unknown parameter, unconsumed stream, etc.

Fix the reported error, re-run. Repeat until all three files validate successfully.

## Acceptance Criteria

- `SchLib::open()` implements the full 3-phase loading pipeline
- `altium validate data/BlankSchlibComponent.SchLib` passes
- `altium validate data/LimeMicroAltiumLib_schLib.SchLib` passes
- `altium validate data/Synthiam.SchLib` passes
- `TrackedCfbDocument::assert_all_consumed()` passes (no unconsumed streams)
- All `ParameterCollection::assert_exhausted()` calls pass (no unknown params)
- `SchLibOps::validate()` returns Ok (not Unimplemented)

## Tests

- **Test files**: `crates/altium-format/src/schlib.rs` (inline `#[cfg(test)]`)
- **Test type**: integration (real SchLib files — cross-milestone integration test)
- **Backing**: user-specified (CLAUDE.md red/green workflow)
- **Dependencies**: M1-M10 (all previous milestones)
- **Scenarios**:
  - Normal: BlankSchlibComponent.SchLib opens and validates (minimal file)
  - Normal: LimeMicroAltiumLib_schLib.SchLib opens and validates (200 components)
  - Normal: Synthiam.SchLib opens and validates (174 components)
  - Normal: component count matches header.components.len()
  - Normal: alias count matches header total alias count
  - Edge: components with no pins (graphics-only)
  - Edge: components with no sidecar streams
  - Edge: components with Additional stream overflow records

## Code Intent

- Rewrite `crates/altium-format/src/schlib.rs`:
  - Replace stub `SchLib::open()` with full 3-phase pipeline implementation
  - Add `collect_pins_mut()` helper to extract mutable references to Pin records
  - Add `parse_lib_additional()` for LibAdditional header stream
- Modify `crates/altium-format-ops/src/schlib_ops.rs`:
  - Replace `Err(Unimplemented)` with actual validation (Ok(()) since parsing is strict)
- The red/green loop during development will likely surface additional unknown parameters and record types that need to be added in earlier milestones — this is expected and part of the workflow
