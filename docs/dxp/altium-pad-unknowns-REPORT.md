# Altium Pad Subrecord-4: Final Field Mapping (AD26)

Ground truth for this report is from Ghidra decompilation of `Altium.PCB.BinaryLoader.dll`:
- `FUN_018a2900` (pad loader orchestration)
- `FUN_0186d700` (base pad section load)
- `FUN_0187b7c0` (extended 114..171 load)
- `FUN_0187bba0` (sub4 extension header interpretation)
- `FUN_018a28c0`/`FUN_0187da70` (thermal relief entries)

## Resolved offsets in main subrecord-4

| Offset | Size | Meaning | Evidence |
|---|---:|---|---|
| 61 | u8 | `TDaisyChainStyle` (`SetState_DaisyChainStyle`, vtbl `+0x788`) | `FUN_018110d0` from `FUN_0186d700` |
| 62 | u8 | `TPadMode` (`Simple/LocalStack/ExternalStack`) | `FUN_01811390` (`SetState_Mode`) |
| 63 | i32 | internal loader field (`param_1+0x70`) | `FUN_01811110` (direct store) |
| 96..104 | 9*u8 | cache validity states (`PlaneConnectionStyleValid` .. `PlanesValid`) | `FUN_0181a560`..`FUN_0181a6d0` on cache blob |
| 105 | u8 | `SelectionMemoryFlags` (`IPCB_Primitive_SaveLoadParameters`) | `FUN_0184d2a0` |
| 106..109 | i32 | `UnionIndex` | `FUN_0184d380` |
| 110..113 | i32 | `JumperID` (`SetState_JumperID`, vtbl `+0x6b8`) | `FUN_01811460` |
| 114..117 | i32 | `V7Layer` override (`SetState_V7Layer`) | `FUN_0184cc00` |
| 118 | u8 bool | `IsAssyTestPoint_Top` | `FUN_018807f0` -> `FUN_018111c0` |
| 119 | u8 bool | `IsAssyTestPoint_Bottom` | `FUN_018807e0` -> `FUN_018111a0` |
| 120 | u8 bool | `UseSeparateExpansions` (cache bool at struct offset 36) | `FUN_01880820` + `FUN_0181b3f0` |
| 121..124 | i32 | `SolderMaskBottomExpansion` | `FUN_0181b120` selection logic |
| 125 | u8 bool | `SolderMaskExpansionFromHoleEdge` (`IPCB_StackObject`) | `FUN_01880800` -> `FUN_01816540` |
| 126..141 | 16 bytes | template link `LibraryID` GUID bytes | `FUN_01816050` + vtbl `+0x38` |
| 142..157 | 16 bytes | template link `TemplateID` GUID bytes | `FUN_01816050` + vtbl `+0x30` |
| 158..161 | i32 | `PinPackageLength` (`SetState_PinPackageLength`, vtbl `+0x748`) | `FUN_01811480` |
| 162..165 | i32 | `HolePositiveTolerance` (`IPCB_StackObject`) | `FUN_01816410` |
| 166..169 | i32 | `HoleNegativeTolerance` (`IPCB_StackObject`) | `FUN_018163f0` |
| 170 | u8 | still not consumed by `FUN_0186d700`/`FUN_0187b7c0` | not referenced in these loaders |
| 171 | u8 bool | `has_sub4_extension` gate | `FUN_01880810` |

## Sub4 extension block (after offset 171)

If `offset 171 != 0`, loader reads:

1. `u32 extension_header_len` at offset `172`
2. `extension_header` bytes at `176` (`min(0x12, extension_header_len)` used by `FUN_0187bba0`)
3. remaining payload can include thermal-relief entries

### Extension header bytes used by loader

| Header offset | Type | Meaning |
|---|---:|---|
| 0..3 | u32 | thermal relief item count |
| 4..7 | f32 | propagation-delay value passed to `IPCB_PrimDelay` path |
| 8 | u8 bool | read via `FUN_01880880`; affects tenting finalize path |
| 9 | u8 bitfield | controls multiple toggles (below) |
| 10..17 | f64 | propagation-delay double when bit `0x04` is set |

Bitfield at header byte `9`:
- `0x01`/`0x02`: select value fed into `SetState_PasteMaskEnabled` and paired primitive setter
- `0x04`: enables propagation-delay write of header `f64`
- `0x08`: `SetState_IsTopPasteEnabled`
- `0x10`: `SetState_IsBottomPasteEnabled`

(`FUN_00dc6e90` confirmed as bit-test helper: `(byte & mask) != 0`.)

## Thermal relief entry payload (after extension header)

If header count > 0:
- loader reads `u32 entry_size`
- then reads `count * entry_size` entries
- expected entry size is `30` bytes in AD26 flow

30-byte entry maps to:
- `TV7_Layer` (4)
- `TPadViaThermalReliefData` (26)
  - `DefinedType` (1)
  - `ConnectStyle` (1)
  - `AirGapWidth` (4)
  - `ConductorWidth` (4)
  - `Rotation` (1)
  - `Entries` (4)
  - `Expansion` (4)
  - `ConductorByPadEdge` (1)
  - `MinDistance` (4)
  - `EnableMinDistance` (1)
  - `UseCustomRelief` (1)

## GUID confirmations used in this path

- `33B7C5A1-B5E8-4046-B5A1-E72152A791E2` = `IPCB_PolygonThermalRelief`
- `4F8677A0-254A-4DBC-A628-2FE9390ABFE8` = `IPCB_PrimDelay`
- `749F6C82-2386-4BBA-ACDE-025BC536075E` = `IPCB_StackObjectCache`
- `2B8D926A-FB4C-4BEC-8AA4-75FB6C318D64` = `IPCB_Primitive_SaveLoadParameters`
