# Milestone 5: Binary Pin Parser

**Files**: `crates/altium-format/src/sch_records.rs`

**Depends on**: M2 (Base Types)

**Flags**: `complex-algorithm`, `needs-rationale`

## Requirements

Implement the binary pin parser for RECORD=2 (SchPin). Pins are the only binary record type in SchLib — they use flags=0x01 blocks with a variable-length binary format instead of pipe-delimited parameters. The binary format is compact (18 + N + M + P bytes) with length-prefixed string fields.

## Binary Pin On-Disk Layout

```
Offset    Size  Type    Field                 Notes
0x00      1     u8      binary_code           Always 0x02
0x01      1     u8      symbol_inner_edge     IeeeSymbol enum
0x02      1     u8      symbol_outer_edge     IeeeSymbol enum
0x03      1     u8      symbol_inside         IeeeSymbol enum
0x04      1     u8      symbol_outside        IeeeSymbol enum
0x05      1     u8      description_length    N (0-254 bytes)
0x06      N     bytes   description           ASCII text
0x06+N    1     u8      formal_type           Formal parameter type
0x07+N    1     u8      electrical            PinElectricalType (0-7)
0x08+N    1     u8      pin_conglomerate      Bitmask (see below)
0x09+N    2     i16 LE  pin_length            DXP units
0x0B+N    2     i16 LE  location_x            DXP units
0x0D+N    2     i16 LE  location_y            DXP units
0x0F+N    4     i32 LE  color                 Win32 COLORREF (BGR)
0x13+N    1     u8      name_length           M bytes
0x14+N    M     bytes   name                  ASCII text
0x14+N+M  1     u8      designator_length     P bytes
0x15+N+M  P     bytes   designator            ASCII text
```

Total: 18 + N + M + P bytes.

## PinConglomerate Bitmask

```
Bit 0-1: Orientation (RotationBy90: 0=0°, 1=90°, 2=180°, 3=270°)
Bit 2:   IsHidden (0x04)
Bit 3:   ShowName (0x08)
Bit 4:   ShowDesignator (0x10)
Bit 5:   NOT accessible (0x20, inverted: set = NOT accessible)
Bit 6:   GraphicallyLocked (0x40)
Bit 7:   OwnerIndexAdditionalList (0x80, refers to Additional stream)
```

Use constants from `altium_format_types::constants::pin::*` for bitmask values.

## Coordinate Encoding

Pin coordinates are stored as `i16` in DXP units:
- 1 DXP unit = 100,000 internal units (C_BASE_UNIT)
- Convert: `internal = i16_value as i32 * C_BASE_UNIT`
- Sub-unit precision added later by PinFrac sidecar (Milestone 9)
- Pin length uses same encoding

## SchPin Struct

```rust
pub(crate) struct SchPin {
    // Decoded from binary
    pub symbol_inner_edge: IeeeSymbol,
    pub symbol_outer_edge: IeeeSymbol,
    pub symbol_inside: IeeeSymbol,
    pub symbol_outside: IeeeSymbol,
    pub description: String,
    pub formal_type: u8,
    pub electrical: PinElectricalType,
    pub pin_length: Coord,
    pub location: CoordPoint,
    pub color: Color,
    pub name: String,
    pub designator: String,

    // Decoded from PinConglomerate
    pub orientation: RotationBy90,
    pub is_hidden: bool,
    pub show_name: bool,
    pub show_designator: bool,
    pub is_not_accessible: bool,
    pub graphically_locked: bool,
    pub owner_index_additional_list: bool,

    // Populated by sidecar streams (Milestone 9)
    pub owner_part_id: i32,         // from SchComponent context, not binary
    pub swap_id_pin: String,        // from PinMiscData
    pub swap_id_part: String,       // from PinWideText
    pub default_value: String,      // from PinWideText
    pub pin_symbol_line_width: i32, // from PinSymbolLineWidth
    pub pin_package_length: String, // from PinPackageLength
    pub propagation_delay: String,  // from PinPropagationDelay
    pub selected_functions: Vec<String>,  // from PinFunctionData
    pub defined_functions: Vec<String>,   // from PinFunctionData

    // Text positioning (from PinTextData sidecar)
    pub name_text_data: Option<PinTextPositioning>,
    pub designator_text_data: Option<PinTextPositioning>,
}
```

## Acceptance Criteria

- Binary pin parsing reads all fixed and variable-length fields correctly
- PinConglomerate bitmask decoded into individual boolean/enum fields
- Coordinates converted from i16 DXP to Coord (multiplied by C_BASE_UNIT)
- ASCII strings read with correct length prefixes
- binary_code validated as 0x02 (error otherwise via UnknownBinaryCode)
- BinaryReader exhaustion checked (assert_exhausted) — no trailing bytes
- Electrical type validated via PinElectricalType::try_from
- IeeeSymbol validated for all four symbol fields

## Tests

- **Test files**: `crates/altium-format/src/sch_records.rs` (inline `#[cfg(test)]`)
- **Test type**: unit + integration
- **Backing**: doc-derived (docs/schlib/binary-pin-format.md)
- **Scenarios**:
  - Normal: parse a pin with known binary data (construct test bytes manually)
  - Normal: pin with empty description (N=0), empty name (M=0)
  - Normal: pin with max description length (N=254)
  - Normal: PinConglomerate decoding (all bit combinations)
  - Normal: coordinate DXP-to-internal conversion
  - Edge: all IeeeSymbol variants accepted
  - Edge: all PinElectricalType variants accepted
  - Error: binary_code != 0x02 produces UnknownBinaryCode
  - Error: truncated binary data produces BinaryReadPastEnd
  - Error: trailing bytes produce UnexpectedTrailingData

## Code Intent

- Add to `crates/altium-format/src/sch_records.rs`:
  - `SchPin` struct with all fields (sidecar fields initialized to defaults)
  - `parse_binary_pin(data: &[u8]) -> Result<SchPin>` function using BinaryReader
  - `decode_pin_conglomerate(byte: u8) -> PinConglomerateFields` helper
  - `PinTextPositioning` struct for name/designator text override data
- Hand-written parser (no derive macro) — variable-length format with length-prefixed strings
- BinaryReader used for all reads (position tracking, bounds checking)
- Sidecar fields (swap_id, text_data, etc.) initialized to empty/default values — populated in M9
