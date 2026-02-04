# Hypothesis: Unknown Altium Pad Binary Fields

## Methodology

Based on:
1. .NET IPCB_Pad3 interface properties (known API surface)
2. KiCad parser gaps (skipped bytes)
3. Common PCB design patterns
4. Altium documentation and typical pad properties

## Priority 1: Single-Byte Fields

### offset 61, u8 - FUN_017c4af0
**Hypothesis: HoleType or JumperID flag**
- Location: Immediately after `plated` (offset 60), before `pad_mode` (offset 62)
- Reasoning: This is a critical pad definition area. Likely represents:
  - Hole type enumeration (round/square/slot) OR
  - Jumper ID presence flag OR
  - Daisy chain configuration flag
- **Most likely**: `hole_type_override` or `is_jumper` boolean

### offsets 96-99, u8 - FUN_017caef0, FUN_017cb5b0, FUN_017cb710, FUN_017cb450
**Hypothesis: Test point and assembly test point flags**
- Location: In the "skipped 7 bytes" region before paste/solder mask modes
- Reasoning: .NET interface has:
  - IsAssyTestPoint_Top
  - IsAssyTestPoint_Bottom
  - Additional test point configuration flags
- **Likely mapping**:
  - offset 96: `is_assy_testpoint_top`
  - offset 97: `is_assy_testpoint_bottom`
  - offset 98-99: Reserved or additional test point config

### offset 102, u8 - FUN_017cb920
**Hypothesis: CONFLICT - KiCad already reads this as soldermaskexpansionmode**
- **Action needed**: Verify if KiCad offset calculation is correct
- Possible KiCad has miscalculated offsets in the 86-102 range
- Or this could be a duplicate/override field

### offsets 103-104, u8 - FUN_017cb190, FUN_017cb030
**Hypothesis: Paste mask enable flags or expansion override**
- Location: Immediately after paste/solder mask expansion modes
- Reasoning: .NET interface has:
  - IsTopPasteEnabled
  - IsBottomPasteEnabled
- **Likely mapping**:
  - offset 103: `is_top_paste_enabled`
  - offset 104: `is_bottom_paste_enabled`

### offset 125, u8 - FUN_0185e2c0
**Hypothesis: Layer-specific flag or configuration**
- Location: After holerotation (106-113) and layer range (114-119)
- Could be related to layer configuration or via/blind-buried settings

### offset 170, u8 - FUN_017c8360
**Hypothesis: End-of-structure flag or feature toggle**
- Location: Near the end of the 172-byte core structure
- Likely a feature flag for extended properties or format version indicator

## Priority 2: 4-Byte (i32) Fields

### offset 63, i32 - FUN_017c5330
**Hypothesis: Thermal relief airgap width**
- Location: Right after `pad_mode` (offset 62)
- Reasoning: Thermal relief is a critical pad-to-plane connection property
- Common in the "23 skipped bytes" (63-85) region
- **Most likely**: `thermal_relief_airgap` (in internal units)

### offset 78, i32 - FUN_017cb240
**Hypothesis: Thermal relief conductor width**
- Location: 15 bytes after offset 63
- Reasoning: Complements the airgap width for thermal relief configuration
- **Most likely**: `thermal_relief_conductor_width`

### offset 82, i32 - FUN_017cb0e0
**Hypothesis: Thermal relief conductor count or power plane clearance**
- Location: 4 bytes after conductor width
- Reasoning: Could be:
  - Number of thermal relief spokes (typically 2-8)
  - Power plane clearance override
- **Most likely**: `thermal_relief_conductor_count` or `powerplane_clearance`

### offset 110, i32 - FUN_017c3fd0
**Hypothesis: Union index or polygon index**
- Location: Between holerotation data and layer_enum
- Reasoning: Altium uses union indices for grouping related objects
- **Most likely**: `union_index` or `polygon_union_id`

### offset 121, i32 - FUN_017cb7c0
**Hypothesis: XPadOffsetAllLayers or YPadOffsetAllLayers**
- Location: After layer range data (114-120)
- Reasoning: .NET interface has XPadOffsetAllLayers, YPadOffsetAllLayers
- **Most likely**: `x_offset_all_layers` (X coordinate offset)

### offset 158, i32 - FUN_017c40a0
**Hypothesis: PinPackageLength or SolderMaskExpansionFromHoleEdge**
- Location: Mid-extended structure
- Reasoning: .NET interface has:
  - PinPackageLength (package outline distance)
  - SolderMaskExpansionFromHoleEdge (solder mask expansion from hole edge, not pad edge)
- **Most likely**: `pin_package_length` or `soldermask_expansion_from_hole`

### offset 162, i32 - FUN_017c8600
**Hypothesis: YPadOffsetAllLayers or additional clearance**
- Location: 4 bytes after offset 158
- If offset 121 is X offset, this could be the Y offset
- **Most likely**: `y_offset_all_layers` or secondary clearance value

### offset 166, i32 - FUN_017c85d0
**Hypothesis: Pad-to-die length or propagation delay**
- Location: Near end of structure
- Reasoning: .NET interface has:
  - PropagationDelay (signal propagation delay)
- But KiCad parser shows pad_to_die_length is at offset 202+ in extended records
- **Possible**: Format version-dependent storage location

## Priority 3: Hole Shape Related

### offset 118, u8 - FUN_0185e2b0
### offset 119, u8 - FUN_0185e2a0
**Hypothesis: Slotted hole configuration**
- Location: Near hole rotation and layer data
- Reasoning: Slotted holes need:
  - Slot orientation/rotation override
  - Slot width vs length configuration
- **Likely**:
  - offset 118: `slot_shape_override` or `hole_shape_alt`
  - offset 119: `slot_orientation_mode`

## Cross-Reference Matrix

| .NET Property                      | Offset Hypothesis | Function Address | Confidence |
|------------------------------------|-------------------|------------------|------------|
| HoleType                           | 61                | FUN_017c4af0     | High       |
| IsAssyTestPoint_Top                | 96                | FUN_017caef0     | High       |
| IsAssyTestPoint_Bottom             | 97                | FUN_017cb5b0     | High       |
| IsTopPasteEnabled                  | 103               | FUN_017cb190     | Medium     |
| IsBottomPasteEnabled               | 104               | FUN_017cb030     | Medium     |
| ThermalReliefAirgap                | 63                | FUN_017c5330     | High       |
| ThermalReliefConductorWidth        | 78                | FUN_017cb240     | High       |
| ThermalReliefConductorCount        | 82                | FUN_017cb0e0     | Medium     |
| UnionIndex                         | 110               | FUN_017c3fd0     | Medium     |
| XPadOffsetAllLayers                | 121               | FUN_017cb7c0     | Medium     |
| PinPackageLength                   | 158               | FUN_017c40a0     | Medium     |
| YPadOffsetAllLayers                | 162               | FUN_017c8600     | Medium     |
| SolderMaskExpansionFromHoleEdge    | 166               | FUN_017c85d0     | Low        |
| SlotShapeOverride                  | 118               | FUN_0185e2b0     | Low        |
| SlotOrientationMode                | 119               | FUN_0185e2a0     | Low        |

## Validation Strategy

To confirm these hypotheses:

1. **Decompile getter functions** - Look for patterns like:
   ```c
   return *(uint8_t *)(this + OFFSET);  // Simple getter
   ```

2. **Decompile setter functions** - Look for patterns like:
   ```c
   *(uint8_t *)(this + OFFSET) = value;  // Simple setter
   ```

3. **Find string references** - Property names in Delphi RTTI or .NET interop layer

4. **Cross-reference usage** - Where these getters/setters are called in the binary I/O functions

5. **Compare with binary files** - Examine actual .PcbDoc files with known pad configurations

## Notes

- Offsets assume the KiCad parser baseline is correct
- Some offsets may shift depending on compiler padding/alignment
- Delphi x64 uses 8-byte alignment for structs
- The 172-byte core record size suggests a tightly packed structure with minimal padding
