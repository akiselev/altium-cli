# Via Binary Record Field Analysis

Comprehensive analysis of the Via (PcbObjectId::Via) binary record format, with special
focus on the 32-byte extension region at offsets 209-240.

## Via Binary Record Layout

The Via record uses a variable-length format: 31-byte legacy or 246+ byte extended.

### Legacy Format (31 bytes)

| Offset | Size | Field | Type |
|--------|------|-------|------|
| 0-12 | 13 | Common header | PcbCommonHeader |
| 13-16 | 4 | location.x | Coord (i32) |
| 17-20 | 4 | location.y | Coord (i32) |
| 21-24 | 4 | diameter | Coord (i32) |
| 25-28 | 4 | hole_size | Coord (i32) |
| 29 | 1 | from_layer | V6Layer (u8) |
| 30 | 1 | to_layer | V6Layer (u8) |

### Extended Format (246 bytes core + optional sections)

#### Core Section (offsets 31-74)

| Offset | Size | Field | Type |
|--------|------|-------|------|
| 31 | 1 | via_properties_version | u8 |
| 32-35 | 4 | thermal_relief_air_gap | Coord |
| 36 | 1 | thermal_relief_conductor_count | u8 |
| 37 | 1 | thermal_relief_rotation_code | u8 |
| 38-41 | 4 | thermal_relief_conductor_width | Coord |
| 42-45 | 4 | power_plane_relief_expansion | Coord |
| 46-49 | 4 | power_plane_clearance | Coord |
| 50-53 | 4 | paste_mask_expansion | Coord |
| 54-57 | 4 | solder_mask_expansion_front | Coord |
| 58-59 | 2 | planes | u16 |
| 60 | 1 | plane_connection_style_valid | TCacheState |
| 61 | 1 | relief_conductor_width_valid | TCacheState |
| 62 | 1 | relief_entries_valid | TCacheState |
| 63 | 1 | relief_air_gap_valid | TCacheState |
| 64 | 1 | power_plane_relief_expansion_valid | TCacheState |
| 65 | 1 | paste_mask_expansion_valid | TCacheState |
| 66 | 1 | solder_mask_expansion_valid | TCacheState |
| 67 | 1 | power_plane_clearance_valid | TCacheState |
| 68 | 1 | planes_valid | TCacheState |
| 69 | 1 | plane_connection_style | PlaneConnectionStyle |
| 70 | 1 | solder_mask_cache_flags | u8 (packed 4x2-bit) |
| 71 | 1 | solder_mask_expansion_mode | u8 |
| 72 | 1 | paste_mask_cache_flags | u8 (packed 4x2-bit) |
| 73 | 1 | paste_mask_expansion_mode | u8 |
| 74 | 1 | via_mode | u8 |

#### Per-Layer Diameters (offsets 75-202)

| Offset | Size | Field | Type |
|--------|------|-------|------|
| 75-202 | 128 | diameters_per_layer[0..32] | [Coord; 32] |

#### Layer Stack Info (offsets 203-208)

| Offset | Size | Field | Type |
|--------|------|-------|------|
| 203-206 | 4 | layer_enum_index | i32 |
| 207 | 1 | stack_start_layer | u8 |
| 208 | 1 | stack_end_layer | u8 |

#### Extension Flags Region (offsets 209-240)

This is the main focus of this analysis. See detailed breakdown below.

#### Trailing Fields (offsets 241-245)

| Offset | Size | Field | Type |
|--------|------|-------|------|
| 241 | 1 | solder_mask_expansion_linked | bool (bit 0) |
| 242-245 | 4 | solder_mask_expansion_back | Coord |

### Optional Sections (after byte 246)

1. **Section 2**: Layer diameter overrides (u32 count + u32 stride=9, then count*9 bytes)
2. **Template link block**: Size-prefixed (41-45 byte payload with GUIDs and tolerances)
3. **Section 4**: Per-layer pad stack entries (u32 count + u32 stride, stride 23/24/29/30)
4. **Section 5**: IPC-4761 via structure (size-prefixed, 4 or 9 byte payload)

## Extension Flags Region: Detailed Analysis

### Background

The 32-byte region at offsets 209-240 was originally interpreted as:
1. Eight `Coord` (i32) fields (extension_coord_209 through extension_coord_237)
2. Then as `[bool; 32]` named `removed_pads_per_layer`

Both interpretations were wrong. Empirical analysis of **41,410 Via records** across
multiple PcbLib and PcbDoc files, combined with C# interface analysis and Delphi attribute
enumeration, reveals these are individual boolean feature flags with 24 reserved bytes.

### Byte-by-Byte Breakdown

| Byte | Offset | Field | Evidence | Confidence |
|------|--------|-------|----------|------------|
| 0 | 209 | **reserved** | Always 0 in 41K records | High |
| 1 | 210 | `is_testpoint_top` | IPCB_Primitive.GetState_IsTestPoint_Top; correlated with byte 2 | High |
| 2 | 211 | `is_testpoint_bottom` | IPCB_Primitive.GetState_IsTestPoint_Bottom | High |
| 3 | 212 | `is_assy_testpoint_top` | IPCB_Primitive.GetState_IsAssyTestPoint_Top; Pad analog at offset 118 | High |
| 4 | 213 | `is_assy_testpoint_bottom` | IPCB_Primitive.GetState_IsAssyTestPoint_Bottom; Pad analog at offset 119 | High |
| 5 | 214 | `solder_mask_override` | Delphi ePrimitiveAttribute enumeration order | Medium-High |
| 6 | 215 | `use_separate_solder_mask_expansion` | TV7_PadCache.UseSeparateExpansions; Pad analog at offset 120 | High |
| 7 | 216 | **reserved** | Always 0 in 41K records | High |
| 8 | 217 | `solder_mask_expansion_from_hole_edge` | IPCB_StackObject interface; Pad analog at offset 125 | High |
| 9-30 | 218-239 | **reserved** | Always 0 in 41K records | High |
| 31 | 240 | `paste_mask_override` | Rare (86/41K = 0.2%), Delphi attribute order | Medium-High |

### Evidence Sources

#### C# Interface Properties (IPCB_Primitive)

The `IPCB_Primitive` interface in `AD26-dotnet` defines:
- `GetState_IsTestPoint_Top` / `SetState_IsTestPoint_Top`
- `GetState_IsTestPoint_Bottom` / `SetState_IsTestPoint_Bottom`
- `GetState_IsAssyTestPoint_Top` / `SetState_IsAssyTestPoint_Top`
- `GetState_IsAssyTestPoint_Bottom` / `SetState_IsAssyTestPoint_Bottom`

These are general primitive properties that apply to both Pads and Vias.

#### Delphi ePrimitiveAttribute Enumeration

The Delphi attribute enum lists properties in declaration order. The solder mask
and paste mask override flags follow the testpoint flags, matching the byte ordering
observed in the binary data.

#### Pad Struct Analogy

The Pad binary record has an analogous extension region with the same flags at
corresponding offsets:
- Pad offset 118 = `is_assy_testpoint_top` (Via offset 212)
- Pad offset 119 = `is_assy_testpoint_bottom` (Via offset 213)
- Pad offset 120 = `use_separate_expansions` (Via offset 215)
- Pad offset 125 = `solder_mask_expansion_from_hole_edge` (Via offset 217)

#### Pairwise Correlation Analysis

Statistical analysis of the 41K records showed:
- Bytes 1-2 (testpoint top/bottom) are positively correlated (often set together)
- Bytes 3-4 (assy testpoint top/bottom) are positively correlated
- Byte 6 (use_separate_solder_mask_expansion) correlates with non-zero
  `solder_mask_expansion_back` in the trailing fields
- All reserved bytes are **strictly zero** across the entire dataset

### Why "Removed Pads Per Layer" Was Wrong

The previous interpretation as per-layer pad removal flags was based on the
`IPCB_Via2.GetProperty_RemovedPads` interface. However:

1. The empirical data shows only 8 specific bytes are ever non-zero, not a
   uniform per-layer distribution
2. The non-zero bytes don't correlate with layer indices
3. The Pad struct has the same flag pattern, and pads don't have "removed pads"
4. The C# interface properties map 1:1 to the observed byte positions

### Implementation Notes

- Reserved bytes (offsets 209, 216, 218-239) are asserted to be zero at parse time.
  If any reserved byte is non-zero, the parser returns a hard error with the offset
  and value, per the project's fail-fast philosophy.
- All flag bytes are read as `u8` and converted to `bool` via `!= 0`.
- The serializer currently only writes the 31-byte legacy format for Vias, so these
  extension flags are not yet written. Full extended-format serialization is a
  separate task.
