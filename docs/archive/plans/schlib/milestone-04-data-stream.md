# Milestone 4: Component Data Stream + Record Dispatch

**Files**: `crates/altium-format/src/schlib.rs`

**Depends on**: M2 (Base Types), M3 (FileHeader)

## Requirements

Implement the core parsing loop for per-component `/<key>/Data` streams. Each component's Data stream contains sequential blocks: the SchComponent record (block 0), followed by child primitive records (pins, graphics, text), terminated by a RECORD=0 end marker.

## Block Sequence

```
Block 0:     flags=0x00  RECORD=1  (SchComponent — always first)
Block 1..N:  flags=0x00  RECORD=N  (text-based records: Line, Rect, Label, etc.)
         or  flags=0x01  binary    (binary pin: first byte is 0x02)
Block N+1:   flags=0x00  RECORD=0  (end marker — stop reading)
```

## Record Dispatch Logic

```rust
fn dispatch_record(block: &Block) -> Result<SchRecord> {
    match block.format {
        BlockFormat::Binary => {
            // Binary records: check first byte
            let code = block.data[0];
            match code {
                INSTRUCTION_BINARY_PIN => parse_binary_pin(&block.data),
                _ => Err(AltiumFormatError::UnknownBinaryCode(code)),
            }
        }
        BlockFormat::Text => {
            let mut params = ParameterCollection::from_bytes(&block.data)?;
            let record_type = params.remove_required::<i32>(RECORD)?;

            // Handle RECORD >= 256 extension
            let record_type = if record_type == 254 {
                params.remove_required::<i32>(RECORD_EX)?
            } else {
                record_type
            };

            let record_type = SchRecordType::try_from(record_type)?;

            match record_type {
                SchRecordType::Component => { ... }
                SchRecordType::Label => { ... }
                SchRecordType::Rectangle => { ... }
                SchRecordType::Line => { ... }
                // ... etc
                _ => Err(AltiumFormatError::UnknownRecordType(record_type as i32)),
            }
        }
    }
}
```

## Component Data Parsing Flow

```rust
fn parse_component_data(data: &[u8]) -> Result<SchLibComponent> {
    let blocks = parse_blocks(data)?;
    let mut blocks_iter = blocks.iter();

    // Block 0: must be SchComponent
    let first_block = blocks_iter.next()
        .ok_or(AltiumFormatError::MissingParam("empty Data stream".into()))?;
    let component = parse_component_record(first_block)?;

    // Blocks 1..N: child records until RECORD=0
    let mut records = Vec::new();
    for block in blocks_iter {
        if is_end_marker(block)? {
            break;
        }
        records.push(dispatch_record(block)?);
    }

    Ok(SchLibComponent { component, records })
}
```

## OwnerIndex Handling

In SchLib, OwnerIndex values are relative within each component section:
- SchComponent is at relative index 0
- First child record is at relative index 1
- Children reference parent by relative index

During parsing, records are stored in order. The OwnerIndex field is preserved as-is (relative) since it's meaningful within the component's record list.

## Acceptance Criteria

- Data stream parses into SchComponent + Vec<SchRecord>
- RECORD=0 end marker terminates parsing (not treated as error)
- Binary blocks (flags=0x01) dispatched to binary pin parser
- Text blocks (flags=0x00) dispatched based on RECORD= value
- RECORD=254 + RECORDEX handled for extended record types
- Unknown record types produce `UnknownRecordType` error (fail-fast)
- Unknown binary codes produce `UnknownBinaryCode` error
- `altium validate BlankSchlibComponent.SchLib` progresses past FileHeader into component parsing

## Tests

- **Test files**: `crates/altium-format/src/schlib.rs` (inline `#[cfg(test)]`)
- **Test type**: integration (real SchLib files)
- **Backing**: doc-derived (docs/schlib/component-data-stream.md)
- **Scenarios**:
  - Normal: BlankSchlibComponent.SchLib component parses (simple component)
  - Normal: end marker terminates loop
  - Normal: binary pin blocks dispatched correctly
  - Edge: RECORD=254 + RECORDEX extension dispatched
  - Error: unknown record type fails fast

## Code Intent

- Add to `crates/altium-format/src/schlib.rs`:
  - `parse_component_data(data: &[u8]) -> Result<SchLibComponent>` — main parsing loop
  - `dispatch_record(block: &Block) -> Result<SchRecord>` — record type dispatch
  - `is_end_marker(block: &Block) -> Result<bool>` — check for RECORD=0
  - `parse_component_record(block: &Block) -> Result<SchComponent>` — parse first block
- Record type dispatch initially handles only implemented records (Component from M3, base types from M2)
- Unimplemented record types return `UnknownRecordType` — the red/green loop will surface which records need implementation next
- Each record's `from_params()` call is followed by `params.assert_exhausted()?` at the dispatch site
