# CustomReliefs Section

## Overview

The `CustomReliefs` section stores custom thermal relief patterns for pads and vias
connecting to polygons. When a pad/via connects to a copper pour polygon, the thermal
relief defines the pattern of copper connections (spokes) and air gaps between the
pad/via and the polygon fill. The "custom" variant allows users to manually specify
which polygon edges have relief connections instead of using the automatic pattern.

**CFB Stream Name:** `CustomReliefs`
**Section Index:** 71 (in the PcbDoc section ordering)
**Delphi Class:** `TCustomReliefSection` (RTTI at `0x01a8d9d9` in `Altium.PCB.BinaryLoader.dll`)
**Feature Flags (TStorageFeature):**
- `eHasCustomThermalReliefsAtWriteStage` (3) -- controls whether thermal relief data is written
- `eHasCustomReliefInfosAtWriteStage` (12) -- controls whether custom relief info/shapes are written

**Feature Gate:** `PCB.CustomThermalRelief` internal option
(checked via `DXP.GlobalVars.Client.GetInternalOptions().ReadFeatureBoolean`)

## Relationship to Other Sections

Custom reliefs are a **sidecar** to pad/via binary records. The thermal relief
configuration itself is stored in the pad/via binary record's subrecord 4 extension
(`TPadViaThermalReliefData` entries), specifically the `UseCustomRelief` boolean flag.
The `CustomReliefs` section stores the **additional geometric data** (relief point
locations) for pads/vias where `UseCustomRelief = true`.

```
Pad Binary Record (Data stream)
  └── Subrecord 4 Extension
      └── TPadViaThermalReliefData[] (per-layer thermal relief config)
          └── UseCustomRelief = true  ──references──>  CustomReliefs stream entry
                                                        (contains edge/location data)
```

## Data Format

### Stream Framing

The stream uses the standard PcbDoc/PcbLib parameter-block sidecar format:

```
[4 bytes]  u32 LE: entry count
For each entry:
  [4 bytes]  u32 LE: parameter string length (including NUL terminator)
  [N bytes]  NUL-terminated parameter string (pipe-delimited |KEY=VALUE|)
```

This is the same framing as `CustomShapes`, `CustomMaskShapes`, `WideStrings`,
`UniqueIDPrimitiveInformation`, and other sidecar streams.

### Parameter Keys

Based on the `IPCB_CustomThermalRelief` interface and the Delphi `ExportCustomReliefToParameters`/
`ImportCustomReliefFromParameters` methods, each entry contains:

| Key | Type | Description |
|-----|------|-------------|
| `PRIMITIVEINDEX` | integer | 0-based index of the pad/via primitive |

Per-layer relief data uses a layer-prefixed format. The `IPCB_CustomThermalReliefInfo`
interface reveals that custom reliefs are stored as a **list of coordinate locations**:

| Key | Type | Description |
|-----|------|-------------|
| (layer prefix).COUNT | integer | Number of custom relief points on this layer |
| (layer prefix).X{N} | integer | X coordinate of relief point N (Coord units) |
| (layer prefix).Y{N} | integer | Y coordinate of relief point N (Coord units) |

**Note:** The exact parameter key prefixes have not been confirmed from test data.
The interface `IPCB_CustomThermalReliefInfo.ThermalRelieafsCount()` (note original
typo "Relieaf") returns the number of relief locations, and
`GetState_ThermalRelieafLocation(index)` returns `TCoordPoint` for each.

## Key Types

### TPadViaThermalReliefData (RT_PCB)

The core thermal relief configuration struct, stored in pad/via binary records.
Pack = 1, total 30 bytes in current format (when all fields present).

```csharp
[StructLayout(LayoutKind.Sequential, Pack = 1)]
public struct TPadViaThermalReliefData
{
    TThermalReliefDefinedType DefinedType;   // 1 byte
    TPlaneConnectStyle        ConnectStyle;   // 1 byte
    int                       AirGapWidth;    // 4 bytes (Coord)
    int                       ConductorWidth; // 4 bytes (Coord)
    TPolygonReliefAngle       Rotation;       // 1 byte
    uint                      Entries;        // 4 bytes (spoke count)
    int                       Expansion;      // 4 bytes (Coord)
    // -- Fields added in later versions --
    bool                      ConductorByPadEdge;  // 1 byte (added later)
    int                       MinDistance;          // 4 bytes (Coord, added later)
    bool                      EnableMinDistance;    // 1 byte (added later)
    bool                      UseCustomRelief;     // 1 byte (added last)
}
```

Binary layout by offset (within the subrecord 4 entry, after the 4-byte layer field):

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 1 | DefinedType | `TThermalReliefDefinedType` enum |
| 1 | 1 | ConnectStyle | `TPlaneConnectStyle` enum |
| 2 | 4 | AirGapWidth | Gap between pad and polygon (Coord) |
| 6 | 4 | ConductorWidth | Width of relief spokes (Coord) |
| 10 | 1 | Rotation | `TPolygonReliefAngle` enum |
| 11 | 4 | Entries | Number of spokes (conductors) |
| 15 | 4 | Expansion | Expansion from pad edge (Coord) |
| 19 | 1 | ConductorByPadEdge | Spoke shape follows pad edge? |
| 20 | 4 | MinDistance | Minimum distance parameter (Coord) |
| 24 | 1 | EnableMinDistance | Enable minimum distance check? |
| 25 | 1 | UseCustomRelief | Use custom relief pattern? |

Total: 26 bytes of data per entry (plus 4-byte TV7_Layer prefix = 30 bytes per entry).

### TThermalReliefDefinedType

```csharp
public enum TThermalReliefDefinedType : byte
{
    trdRule    = 0,  // Use design rule defaults
    trdUser    = 1,  // User-specified (manual) configuration
    trdDefault = 2   // Default profile
}
```

When `DefinedType == trdUser`, the thermal relief data on this pad/via is manually
configured (not inherited from a polygon pour rule). Custom relief editing is only
available when `DefinedType == trdUser` AND `ConnectStyle == eReliefConnectToPlane`.

### TPlaneConnectStyle

```csharp
public enum TPlaneConnectStyle : byte
{
    eReliefConnectToPlane  = 0,  // Standard thermal relief (spokes)
    eDirectConnectToPlane  = 1,  // Direct solid connection
    eNoConnect             = 2   // No connection to polygon
}
```

### TPolygonReliefAngle

```csharp
public enum TPolygonReliefAngle : byte
{
    ePolygonReliefAngle_45  = 0,  // 45 degree rotation
    ePolygonReliefAngle_90  = 1,  // 90 degree rotation
    ePolygonReliefAngle_0   = 2,  // 0 degree rotation
    ePolygonReliefAngle_135 = 3   // 135 degree rotation
}
```

### TPadViaThermalReliefDataType

```csharp
public enum TPadViaThermalReliefDataType : byte
{
    trtFromRule = 0,  // Thermal relief from design rule
    trtManual   = 1   // Manual thermal relief configuration
}
```

### TPadViaThermalReliefItem

A layer + relief data pair:

```csharp
[StructLayout(LayoutKind.Sequential, Pack = 1)]
public struct TPadViaThermalReliefItem
{
    TV7_Layer                ThermalReliefLayer;  // 4 bytes
    TPadViaThermalReliefData ThermalReliefData;   // 26 bytes
}
```

## Interface Hierarchy

### IPCB_PolygonThermalRelief (per-primitive thermal relief management)

```
IPCB_PolygonThermalRelief
  ├── GetProperty_ThermalReliefDataType(layer) → TPadViaThermalReliefDataType
  ├── SetProperty_ThermalReliefDataType(layer, value)
  ├── GetProperty_ThermalReliefData(layer) → TPadViaThermalReliefData
  ├── SetProperty_ThermalReliefData(layer, data)
  ├── GetProperty_ManualThermalReliefsCount() → int
  ├── GetProperty_ThermalReliefsIterator() → IStackObjectThermalReliefIterator
  ├── RemoveThermalRelief(layer)
  ├── SetAllThermalReliefsToRule()
  ├── EditThermalReliefData(layer, ref data) → bool
  └── EditCustomReliefOnLayer(layer)
```

### IPCB_CustomThermalRelief (custom relief geometry)

```
IPCB_CustomThermalRelief
  ├── GetShapeForCustomRelief(layer) → IPCB_RegionShape
  ├── ClearCustomRelief(layer)
  ├── AddCustomReliefAt(layer, shape, location)
  ├── RemoveCustomReliefAt(layer, shape, location)
  ├── AddCustomRelief(layer, edgeIndex, parameter)
  ├── RemoveCustomRelief(layer, edgeIndex, parameter)
  ├── GetCustomThermalReliefInfo(layer) → IPCB_CustomThermalReliefInfo
  ├── ExportCustomReliefToParameters(parameters)
  └── ImportCustomReliefFromParameters(parameters)
```

### IPCB_CustomThermalReliefInfo (relief point query)

```
IPCB_CustomThermalReliefInfo
  ├── GetState_ThermalRelieafLocation(index) → TCoordPoint
  └── ThermalRelieafsCount() → int
```

Note: "Relieaf" is the original typo in Altium's code (appears in both C# and Delphi).

### IStackObjectThermalReliefIterator

```
IStackObjectThermalReliefIterator
  ├── First() → bool
  ├── Next() → bool
  └── Current() → TPadViaThermalReliefItem
```

## Delphi Implementation

### BinaryLoader.dll -- TCustomReliefSection

**RTTI location:** `0x01a8d9d9` (class name at `0x01a8d9db`)
**VMT start:** `0x01a8d6e0`

VMT virtual methods:

| VMT Offset | Address | Description |
|------------|---------|-------------|
| +0x00 | `FUN_00410e90` | System method (likely TObject.Free) |
| +0x08 | `FUN_01a8e670` | **DataRead** -- main load function |
| +0x10 | `FUN_018a7a60` | Base TSection method |
| +0x18 | `FUN_018a73f0` | Base TSection method |
| +0x20 | `FUN_0189fb10` | Base TSection method |
| +0x28 | `FUN_018a7720` | Base TSection method |

**Loading logic** (decompiled `FUN_01a8e670`):
1. Checks if `eHasCustomThermalReliefsAtWriteStage` feature is enabled
2. Iterates over all board primitives (pads/vias) that support `IPCB_CustomThermalRelief`
3. For each primitive, calls `FUN_01a8e4e0` which:
   a. Checks if the primitive implements the custom relief interface
   b. Calls `ImportCustomReliefFromParameters` to load from stored parameter data
4. Stores loaded entries in an internal dictionary keyed by primitive reference

**Key string references in BinaryLoader.dll:**

| Address | String | Usage |
|---------|--------|-------|
| `0x017f5156` | `CustomReliefs` | CFB stream name |
| `0x017f5091` | `CustomReliefToParameters` | Export method name |
| `0x017f50f4` | `CustomReliefFromParameters` | Import method name |

### Advpcb.dll -- Runtime Implementation

**Key type references:**

| RTTI Pattern | Description |
|-------------|-------------|
| `CustomReliefTypes.TCustomRelief` | Core relief point data type |
| `CustomReliefTypes.TCalculatedReliefData` | Calculated/derived relief geometry |
| `CustomReliefInformation` / `CustomReliefInformation2` | Per-primitive relief info |
| `CustomReliefUtils` | Utility functions for relief operations |
| `CustomReliefEditor` | Interactive relief editing UI |
| `CustomReliefPoints` | Relief point collection |
| `CustomReliefByHandle` | Handle-based relief lookup |
| `PcbCustomRelief.TCustomReliefInfo` | Per-layer relief info |
| `CustomRelief.TPair<TV7_Layer, TCustomReliefInfo>` | Layer-to-info mapping |

**Key locations in Advpcb.dll:**

| Address | String | Usage |
|---------|--------|-------|
| `0x02361ba4` | `CustomReliefInfo` | Relief info class reference |
| `0x02363d5c` | `CustomReliefToParameters$44$0$Intf` | Export closure |
| `0x0236410c` | `CustomReliefFromParameters$46$0$Intf` | Import closure |
| `0x0244cdb8` | `CustomReliefDatas$64$4$Intf` | Data collection closure |
| `0x0253beb8` | `CustomReliefUtils` | Utility class RTTI |
| `0x0253c117` | `CustomReliefByHandle` | Handle lookup |
| `0x0253c23e` | `CustomReliefPoints` | Point collection |
| `0x0253c3e6` | `CustomReliefEditor` | Editor class |

### Altium.PCB.DataModel.dll -- Data Model

Key string references from the `Altium.PCB.DataModel.dll` analysis:

| Address | String | Usage |
|---------|--------|-------|
| `0x00b91b65` | `ReliefConductorWidth` | Property name |
| `0x00b91b8d` | `ReliefEntries` | Property name |
| `0x00b91bae` | `ReliefAirGap` | Property name |
| `0x00b91bd8` | `ReliefExpansion` | Property name |
| `0x00b91d29` | `ReliefConductorWidthValid` | Validation flag |
| `0x00b91d56` | `ReliefEntriesValid` | Validation flag |
| `0x00b91d7c` | `ReliefAirGapValid` | Validation flag |
| `0x00b91dab` | `ReliefExpansionValid` | Validation flag |
| `0x00b9214a` | `ReliefDefinedType` | Type discriminator |
| `0x00b921a8` | `ReliefData` | Relief data reference |
| `0x00b923c8` | `ReliefDataType` | Data type field |
| `0x00b92420` | `ReliefItem` | Item reference |
| `0x00b92450` | `ReliefLayer` | Layer field |
| `0x00b924fd` | `ReliefIterator` | Iterator reference |
| `0x00ce3185` | `ReliefDataKey` | Dictionary key type |
| `0x00ce33c2` | `ReliefDataList` | Data list type |

Internal data model uses `Dictionary<ReliefDataKey, TPadViaThermalReliefData>` for
per-layer thermal relief storage on each pad/via.

## How Custom Reliefs Work

### Conceptual Model

A standard thermal relief creates a symmetric pattern of spokes (conductors) connecting
a pad/via to a polygon pour. The pattern is defined by:
- `ConnectStyle`: Relief, Direct, or No Connect
- `Entries`: Number of spokes (typically 2 or 4)
- `Rotation`: Angular offset of the spoke pattern (0, 45, 90, 135 degrees)
- `AirGapWidth`: Width of gaps between spokes
- `ConductorWidth`: Width of spoke conductors

A **custom relief** overrides this symmetric pattern with user-placed connection points
on specific polygon edges. Instead of automatic spoke placement, the user manually
places and removes relief connections at specific coordinates around the pad.

### Storage Architecture

1. **Binary record** (pad/via Data stream): Stores `TPadViaThermalReliefData` per-layer
   with `UseCustomRelief = true` flag
2. **Sidecar stream** (`CustomReliefs`): Stores the custom relief point locations as
   parameter blocks, keyed by `PRIMITIVEINDEX`
3. **Runtime**: The `IPCB_CustomThermalRelief` interface provides methods to
   add/remove relief points at specific polygon edges or coordinates

### Editing Flow

1. User selects a pad/via and sets thermal relief to "Manual" (`DefinedType = trdUser`)
2. User enables `UseCustomRelief` in the relief settings
3. The `EditCustomReliefOnLayer` method opens the `CustomReliefEditor`
4. User clicks on polygon edges to add/remove relief connection points
5. Points are stored as `TCoordPoint` locations in the `IPCB_CustomThermalReliefInfo`
6. On save, `ExportCustomReliefToParameters` serializes to the `CustomReliefs` stream

## PcbLib Context

In PcbLib files, the `CustomReliefs` stream is stored **per-footprint**:
`<FootprintName>/CustomReliefs`. Same parameter-block format as PcbDoc.

The feature flag `eHasCustomReliefInfosAtWriteStage` (12) controls whether the
stream is written during library save.

## Test Data Status

**No test data observed yet.** The custom relief feature requires:
1. A design with polygon pours
2. Pads/vias with `UseCustomRelief = true`
3. Manually placed relief connection points

To generate test data: Create a PcbDoc with a polygon pour, set a pad's thermal
relief to manual mode, enable custom relief, and use "Edit Points" to place custom
relief locations. Then save and inspect the `CustomReliefs` stream.

## Source References

### C# (AD26-dotnet)

| File | Description |
|------|-------------|
| `Altium.Edp.Interfaces/RT_PCB/IPCB_CustomThermalRelief.cs` | Custom relief interface |
| `Altium.Edp.Interfaces/RT_PCB/IPCB_CustomThermalReliefInfo.cs` | Relief info (point count & locations) |
| `Altium.Edp.Interfaces/RT_PCB/IPCB_PolygonThermalRelief.cs` | Polygon thermal relief management |
| `Altium.Edp.Interfaces/RT_PCB/TPadViaThermalReliefData.cs` | Relief data struct (30 bytes) |
| `Altium.Edp.Interfaces/RT_PCB/TPadViaThermalReliefDataType.cs` | FromRule/Manual enum |
| `Altium.Edp.Interfaces/RT_PCB/TThermalReliefDefinedType.cs` | Rule/User/Default enum |
| `Altium.Edp.Interfaces/RT_PCB/TPlaneConnectStyle.cs` | Relief/Direct/NoConnect enum |
| `Altium.Edp.Interfaces/RT_PCB/TPolygonReliefAngle.cs` | 0/45/90/135 degree enum |
| `Altium.Edp.Interfaces/RT_PCB/TPadViaThermalReliefItem.cs` | Layer + data pair |
| `Altium.Edp.Interfaces/RT_PCB/IStackObjectThermalReliefIterator.cs` | Iterator interface |
| `Altium.Edp.Interfaces/RT_PCB/TStorageFeature.cs` | Feature flags enum (bits 3 and 12) |
| `Altium.Edp.Interfaces/PCBInterfaces/IPCB_FileVersionInfoList.cs` | Version info (AddVersionCustomThermalReliefAreUsed) |
| `Altium.Edp.Interfaces/PCBInterfaces/IPCBCommands.cs` | EditCustomRelief commands (deprecated/DoNotUse) |
| `Altium.Dxp.Interfaces/RT_FeatureNames/Consts.cs` | Feature name: `PCB.CustomThermalRelief` |
| `Altium.SDK.Interfaces/PCB/SPadViaThermalReliefData.cs` | SDK relief data struct (older, no ConductorByPadEdge/MinDistance/UseCustomRelief) |
| `Altium.SDK.Interfaces/PCB/IPadViaThermalReliefData.cs` | SDK relief data interface |
| `Altium.SDK.Interfaces/PCB/PadViaThermalReliefData.cs` | SDK relief data wrapper class |
| `InteractiveProperties.../PcbThermalReliefWrapper.cs` | UI wrapper for thermal relief editing |
| `InteractiveProperties.../ThermalRelieafDataObject.cs` | Data object for thermal relief (note "Relieaf" typo) |
| `InteractiveProperties.../IPcbThermalReliefDataObject.cs` | Thermal relief data interface |
| `InteractiveProperties.../IPcbCustomReliefDataObject.cs` | Custom relief data interface |
| `InteractiveProperties.../PcbStackObjectDataObject.cs` | Stack object with relief support |

### Delphi / Ghidra

| Binary | Address | Description |
|--------|---------|-------------|
| `Altium.PCB.BinaryLoader.dll` | `0x01a8d9d9` | `TCustomReliefSection` RTTI |
| `Altium.PCB.BinaryLoader.dll` | `0x017f5156` | `"CustomReliefs"` stream name string |
| `Altium.PCB.BinaryLoader.dll` | `0x01a8e670` | DataRead virtual method |
| `Altium.PCB.BinaryLoader.dll` | `0x01a8e4e0` | Per-entry load function |
| `Advpcb.dll` | `0x0253beb8` | `CustomReliefUtils` RTTI |
| `Advpcb.dll` | `0x0253c3e6` | `CustomReliefEditor` RTTI |
| `Advpcb.dll` | `0x02361ba4` | `CustomReliefInfo` string |
| `Advpcb.dll` | `0x02363d5c` | `CustomReliefToParameters` closure RTTI |
| `Advpcb.dll` | `0x0236410c` | `CustomReliefFromParameters` closure RTTI |

### Existing Codebase

| File | Description |
|------|-------------|
| `crates/altium-format/src/pcblib/mod.rs:490` | `PcbPadThermalReliefEntry` struct |
| `crates/altium-format/src/pcblib/primitives/pad.rs:405` | `parse_thermal_relief_entry()` function |
| `docs/pcblib/CustomShape.md` | CustomShapes/CustomReliefs/CustomMaskShapes documentation |
| `docs/pcbdoc/stream_table.md:268` | Stream table entry for CustomReliefs |
