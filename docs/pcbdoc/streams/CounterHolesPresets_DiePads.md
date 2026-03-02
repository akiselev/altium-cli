# CounterHolesPresetsSection & DiePadsInfo

Two PcbDoc sections related to advanced hole manufacturing: counter-holes
(counterbore/countersink features on pads and vias) and die-pad bonding info
(bare-die chip attachment metadata).

---

## 1. CounterHolesPresetsSection

**CFB storage**: `CounterHolesPresetsSection`
**Section index**: 65 (in the PcbDoc section registry)
**Delphi internal**: (no separate Delphi constant found; referenced alongside
`CounterHolesSection` at index 64 which uses `Section_CounterHoles`)
**Feature flag**: `PCB.CounterHoles` (from `RT_FeatureNames.Consts.cOptionPCBCounterHoles`)

### Purpose

Stores a board-level library of **counter-hole parameter presets** -- saved
configurations for counterbore and countersink hole features that users can apply
to pads. The preset list lives on the board object and is exposed via
`IPCB_BoardEx.GetState_CounterHoleParamsPresetList()`.

### Data Model (from .NET interfaces)

#### Preset List: `IPCB_CounterHoleParamsPresetList`
- `Add(preset)` / `AddNew() -> preset` / `Delete(index)` / `Clear()`
- `GetPreset(index) -> IPCB_CounterHoleParamsPreset`
- `GetCount() -> int`

#### Preset: `IPCB_CounterHoleParamsPreset`
| Field | Type | Notes |
|-------|------|-------|
| `Name` | `string` | Display name |
| `Description` | `string` | Optional description |
| `ParamList` | `IPCB_CounterHoleParamsList` | Counter-hole params attached to this preset |

#### Params List: `IPCB_CounterHoleParamsList`
- `Add(params)` / `AddNew() -> params` / `Delete(index)` / `Clear()`
- `GetParams(index) -> IPCB_CounterHoleParams`
- `GetCount() -> int`
- `IsEqual(other) -> bool`
- `Serialize() -> string` / `Deserialize(string)`
- `Replicate() -> IPCB_CounterHoleParamsList`

#### Params: `IPCB_CounterHoleParams`
| Field | Type | Description |
|-------|------|-------------|
| `Depth` | `int` (Coord) | Counter-hole depth (Altium coords, 10000 = 1 mil) |
| `Diameter` | `int` (Coord) | Counter-hole diameter |
| `Angle` | `double` | Countersink angle in degrees |
| `Direction` | `TCounterHoleDirection` | Top-to-bottom or bottom-to-top |
| `Material` | `TCounterHoleMaterial` | Material in counter-hole cavity |
| `CounterHoleType` | `TCounterHoleType` | Counterbore vs countersink (read-only getter, set implicitly) |

Methods:
- `IsEqual(other)` / `IsEqualWithoutMaterial(other)`
- `CopyFrom(other)`
- `GetDescription(displayUnit) -> string`
- `GetDepthForDiameter(diameter) -> int` (computed from angle for countersink)
- `GetDiameterOnDepth(depth) -> int` (computed from angle for countersink)
- `Serialize() -> string` / `Deserialize(paramStr)`

#### Diameter-on-Depth list: `IPCB_CounterHoleDiameterOnDepthList`
A computed list (from `IPCB_Pad3.GetCounterHoleDiameters()`) giving the hole
diameter profile at each depth through the board:

```csharp
struct TCounterHoleDiameterOnDepth {
    int Depth;      // Coord
    int Diameter;   // Coord
    TCounterHoleMaterial Material;
}
```

Pack = 8 (LayoutKind.Sequential).

### Enums

```
TCounterHoleType : byte {
    eCounterBore  = 0,  // Flat-bottomed enlarged hole
    eCounterSink  = 1,  // Conical/angled enlarged hole
}

TCounterHoleDirection : byte {
    eFromTopToBottom = 0,
    eFromBottomToTop = 1,
}

TCounterHoleMaterial : byte {
    eNoMaterial    = 0,
    eCopperPlated  = 1,
    eSurfaceFinish = 2,
}
```

### Relationship to CounterHolesSection (index 64)

Section 64 (`CounterHolesSection` / `Section_CounterHoles`) stores the per-pad
counter-hole parameter assignments. Section 65 (`CounterHolesPresetsSection`)
stores the board-level preset library that provides reusable configurations.

Both are gated behind feature `PCB.CounterHoles`.

### How Counter-Holes Attach to Primitives

#### Pads (IPCB_Pad3)
- `GetProperty_CounterHoles() -> IPCB_CounterHoleParamsList` (the pad's counter-holes)
- `SetProperty_CounterHoles(list)` (set to null to clear)
- `IsCounterHole() -> bool`
- `GetCounterHoleDiameters() -> IPCB_CounterHoleDiameterOnDepthList` (computed profile)
- `GetFirstTopCounterHole() / GetFirstBottomCounterHole() -> IPCB_CounterHoleParams`
- `UpdateCounterHoles()` / `UntieCounterHoles()`

A pad can have **multiple** counter-holes (e.g., one from top, one from bottom).
Each `IPCB_CounterHoleParams` has a Direction indicating which side it enters from.

#### Vias (IPCB_Via)
- `IsCounterHole() -> bool`
- `GetCounterHole_Params() -> IPCB_CounterHoleParams` (single params, not list)

Vias get a single counter-hole param (not a list) unlike pads.

#### Drill Layer Pairs (IPCB_DrillLayerPair)
- `GetState_DrillLayerPairType() -> TDrillLayerPairType`
- `GetProperty_Params() -> IPCB_CounterHoleParams`
- `IsCounterHole() -> bool`

`TDrillLayerPairType` includes `CounterHole = 3` as a drill pair type:
```
TDrillLayerPairType : byte {
    Regular       = 0,
    MicroViaDrill = 1,
    Backdrill     = 2,
    CounterHole   = 3,
}
```

Counterhole drill pairs are created via:
```csharp
board.AddLayerPairEx2(lowLayer, highLayer, TDrillLayerPairType.CounterHole, counterHoleParams);
```

### Factory (IPCB_CounterHoleFactory)

The board object (`IPCB_Board`) also implements `IPCB_CounterHoleFactory`:
- `CreateCounterHoleParams() -> IPCB_CounterHoleParams`
- `CreateCounterHoleParamsList() -> IPCB_CounterHoleParamsList`
- `CreateCounterHoleParamsPreset() -> IPCB_CounterHoleParamsPreset`
- `CreateCounterHoleParamsPresetList() -> IPCB_CounterHoleParamsPresetList`

### Serialization Format

Both `IPCB_CounterHoleParams` and `IPCB_CounterHoleParamsList` have
`Serialize() -> string` and `Deserialize(string)` methods. The SDK interfaces
pass these as COM dispatch calls. The exact serialization format is likely pipe-
delimited `|KEY=VALUE|` parameter strings (standard Altium text block format),
but the encoding details are in the Delphi DLL.

### UI Properties (from PcbPadDataObject)

The Interactive Properties panel exposes:

| Property | Type | Description |
|----------|------|-------------|
| `TopCounterHoleFeature` | `PadFeatureType` | None / Countersink / Counterbore |
| `TopCounterHoleFeatureSize` | `int` (Coord) | Counter-hole diameter (top) |
| `TopCounterHoleFeatureDepth` | `int` (Coord) | Counter-hole depth (counterbore only) |
| `TopCounterHoleFeatureAngle` | `double` | Counter-hole angle (countersink only) |
| `BottomCounterHoleFeature` | `PadFeatureType` | Same for bottom |
| `BottomCounterHoleFeatureSize` | `int` (Coord) | Counter-hole diameter (bottom) |
| `BottomCounterHoleFeatureDepth` | `int` (Coord) | Counter-hole depth (counterbore) |
| `BottomCounterHoleFeatureAngle` | `double` | Counter-hole angle (countersink) |

Default values:
- `DefaultCounterSinkAngle` = 90.0 degrees
- `DefaultCounterHoleSizeDelta` = 100000 (10 mil) -- added to hole size

```
PadFeatureType {
    None,
    Countersink,
    Counterbore,
}
```

### Drill Manager Classification Fields

Counter-holes add two classification fields to the drill manager:
- `cfCounterHoleDepth` (index 14) -- "Counterhole Depth"
- `cfCounterHoleAngle` (index 15) -- "Counterhole Angle"

Filter constant: `cAllLayerPairsWithoutCounterHoles = -2`

### IHoleSizeInfo Integration

The `IHoleSizeInfo` interface (used in the drill manager) includes:
- `GetCounterHoleDepth() -> int`
- `GetCounterHoleAngle() -> double`
- `DrillType() -> TDrillLayerPairType`

And `IHoleSizeInfoInternal`:
- `SetCounterHoleDepth(depth)`
- `SetCounterHoleAngle(angle)`
- `SetDrillType(drillType)`

### Gerber Output Integration

Counter-hole params are used in Gerber drill file naming:
```csharp
GetState_PlotDrillFileName(drillKind, layer1, layer2, pairType, counterHoleParams)
SetState_PlotDrillFileName(drillKind, layer1, layer2, pairType, counterHoleParams, fileName)
```

---

## 2. DiePadsInfo

**CFB storage**: `DiePadsInfo`
**Section index**: 83 (in the PcbDoc section registry)
**Delphi internal**: `Section_DiePadsInfo`
**Category**: Wirebond / IC Packaging

### Purpose

Stores metadata linking die pads to their associated 3D body components and
Z-axis positioning. This is used for bare-die IC packaging workflows where
bond pads on a semiconductor die need to be associated with specific 3D body
models and layers.

### Data Model (from .NET interfaces)

#### IPCB_DiePadBondInfo

GUID: `{10E6A5C2-06EF-4619-BF59-ADD19BDB2BAC}`

```csharp
interface IPCB_DiePadBondInfo {
    nint I_ObjectAddress();

    bool FindDiePadInfo(
        Guid argDiePadGuid,         // UniqueId of the die pad
        out Guid arg3DBodyGuid,     // UniqueId of the associated 3D body
        out TV7_Layer argLayer,     // Layer the die pad is on
        out int argZOffset          // Z-axis offset (Coord)
    );

    void SetDiePadInfo(
        Guid argDiePadGuid,
        Guid arg3DBodyGuid,
        TV7_Layer argLayer,
        int argZOffset
    );

    void SetDiePad3DBody(Guid argDiePadGuid, Guid arg3DBodyGuid);
    void SetDiePadZCoord(Guid argDiePadGuid, int argZOffset);
}
```

### Record Structure

Each entry is a mapping:
- **Key**: Die pad GUID (UniqueId of a pad primitive)
- **Value**: 3D body GUID + layer + Z offset

This is a lookup table (likely stored as a parameter block or binary map)
indexed by die pad UniqueId GUIDs.

### Fields

| Field | Type | Description |
|-------|------|-------------|
| DiePadGuid | `Guid` (16 bytes) | UniqueId of the pad designated as a die pad |
| 3DBodyGuid | `Guid` (16 bytes) | UniqueId of the associated 3D body component |
| Layer | `TV7_Layer` (u32) | Layer the die pad is placed on |
| ZOffset | `int` (Coord) | Z-axis offset for the die pad |

### Related Layer Kinds

Die pads use dedicated mechanical layer kinds:
```
TMechanicalLayerKind {
    ...
    mlDiePadsTop    = (ordinal ~50),
    mlDiePadsBottom = (ordinal ~51),
}

TMechanicalLayerPairKind {
    ...
    mlpDiePads = (ordinal ~23),
}
```

### Relationship to Other Sections

- **Wirebonds** (section 80): Wire bond connections that route signals from
  die pads to package pads. Die pad info is needed to know the Z position
  of bond pads for 3D routing.
- **3DRoutingData** (section 59): 3D routing paths may use die pad positions
  for wirebond routing geometry.
- **WirebondBodies** (section 82): 3D representations of wirebonds may
  reference die pad Z offsets.

---

## Summary: Key Source Files

### Counter-Holes

| File | Content |
|------|---------|
| `Altium.SDK.Interfaces/PCB/IPCB_Pad3.cs` | Pad3 interface with counter-hole accessors |
| `Altium.SDK.Interfaces/PCB/IPCB_Pad3Helper.cs` | Helper extensions for Pad3 |
| `Altium.SDK.Interfaces/PCB/IPCB_CounterHoleParams.cs` | Single counter-hole params |
| `Altium.SDK.Interfaces/PCB/IPCB_CounterHoleParamsList.cs` | Counter-hole params list |
| `Altium.SDK.Interfaces/PCB/IPCB_CounterHoleParamsPreset.cs` | Named preset |
| `Altium.SDK.Interfaces/PCB/IPCB_CounterHoleParamsPresetList.cs` | Preset list (board-level) |
| `Altium.SDK.Interfaces/PCB/IPCB_CounterHoleFactory.cs` | Factory for creating counter-hole objects |
| `Altium.SDK.Interfaces/PCB/TCounterHoleType.cs` | Counterbore vs Countersink enum |
| `Altium.SDK.Interfaces/PCB/TCounterHoleDirection.cs` | Top-to-bottom vs Bottom-to-top enum |
| `Altium.SDK.Interfaces/PCB/TCounterHoleMaterial.cs` | No material / CopperPlated / SurfaceFinish |
| `Altium.SDK.Interfaces/PCB/SCounterHoleDiameterOnDepth.cs` | Struct: depth+diameter+material |
| `Altium.SDK.Interfaces/PCB/CounterHoleDiameterOnDepth.cs` | Class wrapper for the struct |
| `Altium.SDK.Interfaces/PCB/IPCB_Via.cs` | Via interface (single counter-hole params) |
| `Altium.SDK.Interfaces/PCB/TDrillLayerPairType.cs` | Enum including CounterHole=3 |
| `Altium.Edp.Interfaces/RT_PCB/IPCB_BoardEx.cs` | Board-level preset list accessor |
| `Altium.Edp.Interfaces/RT_PCB/IPCB_DrillLayerPair.cs` | Drill pair with counter-hole params |
| `Altium.Edp.Interfaces/RT_PCB/IPCB_CounterHoleParams.cs` | Edp-level params (typed Direction/Material) |
| `Altium.Edp.Interfaces/DrillManager/Consts.cs` | Classification field names |
| `Altium.Edp.Interfaces/PCBInterfaces/IHoleSizeInfo.cs` | Drill manager hole info |
| `InteractiveProperties...PCB.DataModel/PcbPadDataObject.cs` | UI data object with counter-hole properties |
| `InteractiveProperties...PCB.Views/PadCounterholeFeatureBehavior.cs` | UI behavior/visibility logic |
| `Altium.Dxp.Interfaces/RT_FeatureNames/Consts.cs` | Feature flag `PCB.CounterHoles` |

### Die Pads

| File | Content |
|------|---------|
| `Altium.Edp.Interfaces/RT_PCB/IPCB_DiePadBondInfo.cs` | Die pad bond info interface |
| `Altium.Edp.Interfaces/RT_PCB/TMechanicalLayerKind.cs` | mlDiePadsTop, mlDiePadsBottom |
| `Altium.Edp.Interfaces/RT_PCB/TMechanicalLayerPairKind.cs` | mlpDiePads |

### DispId Constants (SDK)

Counter-hole DispIds span 524912..525185 in `Altium.SDK.Interfaces/DispConsts.cs`:
- Params: 524912-524925, 524966-524967, 524976-524977, 525046
- ParamsList: 524926-524931, 524978, 524994-524995, 525185
- Preset: 524932-524936, 524981
- PresetList: 524937-524942
- Factory: 524943-524946
- DiameterOnDepthList: 525127-525131
- Pad3 CounterHoles: 524964-524965, 525132-525133
- Via CounterHole: 524975
