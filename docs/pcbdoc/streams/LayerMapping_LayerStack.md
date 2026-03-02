# LayerToLayerMapping & LayerStackSection

Research findings for two CFB sections in PcbDoc files related to layer management.

## 1. LayerToLayerMapping

### CFB Stream Names

```
/LayerToLayerMapping/Header   (standard u32 section header)
/LayerToLayerMapping/Data     (layer map payload)
```

**Delphi internal name**: `Section_LayerToLayerMapping`

**Section number in master list**: 70 (from sidecar-streams-deep-dive.md)

### Purpose

Maps layers from one PCB context to another — specifically used when placing a
footprint from a PcbLib into a PcbDoc. The source PcbLib may have a different
layer stack than the target PcbDoc, so this mapping tells Altium how to translate
layer IDs during placement.

The mapping is a **TV7_Layer → TV7_Layer** translation table: for each source
layer, what destination layer it maps to.

### Interface Hierarchy

```
ILayersMap (RT_PCB)
├── GetHash() → int
├── Replicate() → ILayersMap
├── LayerExists(TV7_Layer) → bool
├── SetState_Layer(TV7_Layer source, TV7_Layer dest)
├── GetState_Layer(TV7_Layer source) → TV7_Layer
├── ExportMapToString() → string
└── ImportMapFromString(string)
```

**Key observation**: `ILayersMap` has `ExportMapToString()` / `ImportMapFromString()`.
This strongly suggests the CFB data stream contains a **pipe-delimited parameter string**
encoding the layer map (consistent with how most Altium sections store data). The map
is likely stored as `|KEY=VALUE|` pairs where keys are source TV7_Layer u32 values
and values are destination TV7_Layer u32 values.

### Consumers

- `IPCB_Library2.GetState_LayerToLayerMapping()` — library-level layer mapping
- `IPCB_LibComponent.GetState_LayerToLayerMapping()` — per-component layer mapping
- `IPCB_ElectricalLayerMapping` — runtime mapping for electrical layers between
  source and destination layer stacks

### Data Model

The `ILayersMap` interface is essentially a `HashMap<TV7_Layer, TV7_Layer>`:
- **Source key**: TV7_Layer (u32) from the component/library
- **Mapped value**: TV7_Layer (u32) in the target board

When no mapping is needed (same layer stack), the map may be empty or identity
(each layer maps to itself).

### Serialization Format (Inferred)

Given the `ExportMapToString()` / `ImportMapFromString()` methods, the Data stream
likely uses standard block-encoded format:

```
Block header: [u8 flags | u24 size]  (flags & 0x01 = 0 → text block)
Payload: pipe-delimited |KEY=VALUE| parameter string
```

The parameter string likely contains:
```
|SOURCELAYER1=DESTLAYER1|SOURCELAYER2=DESTLAYER2|...|
```

Where layer values are TV7_Layer u32 integers encoded as decimal strings.

### Observation in Test Files

**Not observed in any test file**. None of our 96+ PcbDoc test files contain a
`/LayerToLayerMapping/` storage. This section only appears when:
1. A PcbLib component has a custom layer mapping different from the board
2. The board was created from a template with different layer assignments

This is a feature primarily used in advanced multi-stackup designs with HDI or
rigid-flex boards where different regions have different layer configurations.

### Standard Section Layout

Like other sections, expected to use:
- `Header` stream: 4-byte u32le record count (likely 1)
- `Data` stream: block-encoded payload

---

## 2. LayerStackSection

### CFB Stream Names

```
/LayerStackSection/Header   (standard u32 section header)
/LayerStackSection/Data     (layer stack definition payload)
```

**Delphi internal name**: None found (no `Section_*` constant identified)

**Section number in master list**: 45 (from sidecar-streams-deep-dive.md)

### Purpose

The "new format" layer stack definition that augments or replaces the legacy
layer stack data stored within the `Board6` section parameters. Whereas Board6
stores layer data as flat `LAYER{i}*` parameters, `LayerStackSection` provides a
richer hierarchical representation supporting:

- **Multiple substacks** (rigid-flex boards with different regions)
- **Board region layer stacks** (different layer counts per board region)
- **Impedance profiles** per substack
- **Custom layer ordering** with full V7 layer IDs
- **Mechanical layer integration** with typed layer kinds

### Interface Hierarchy

The layer stack is managed through a deep interface hierarchy:

```
IPCB_LayerStackBase
├── GetState_Name() → string
├── SetState_IsFlex(bool)
├── GetState_IsFlex() → bool
├── Id() → string
├── StateID() → int
├── Count() → int  (overall or by TLayerClassID)
├── Iterator() → IPCB_LayerObjectIterator
├── First/Last/Next/Previous(TLayerClassID) → IPCB_LayerObject
├── Get_ZTop/ZBottom(IPCB_LayerObject) → int
├── LayerNumberInStack(TV7_Layer) → int
└── GetDisplayName(TLayerNameDisplayMode) → string

IPCB_LayerStack : IPCB_LayerStackBase
├── GetState_ShowTopDielectric() → bool
├── GetState_ShowBotDielectric() → bool
├── Board() → IPCB_Board
├── LayerObject(TV6_Layer | TV7_Layer) → IPCB_LayerObject
├── DielectricTop/Bottom() → IPCB_SolderMaskLayer
├── GetState_LayerStackType() → TLayerStackType
├── GetState_IsService() → bool
├── SetState_Color(u32)
├── GetState_TopSignalLayer() → TV7_Layer
├── GetState_BottomSignalLayer() → TV7_Layer
└── GetState_UsedByPrims() → bool

IPCB_MasterLayerStack : IPCB_LayerStackBase
├── GetState_Substacks(int) → IPCB_LayerStack
├── GetState_LayerStackStyle() → TLayerStackStyle
├── Board() → IPCB_Board
├── CreateLayer(TV7_Layer) → IPCB_LayerObject
├── RemoveLayer(IPCB_LayerObject) → bool
├── InsertOnTop/OnBottom/Below/Above(...)
├── DisableLayer/EnableLayer(substack, layer)
├── CreateSubstack() → IPCB_LayerStack
├── RemoveSubstack(IPCB_LayerStack) → bool
├── Import_FromParameters(StringBuilder)
├── Export_ToParameters(StringBuilder)
├── GetSubstack(string | ILayerSet) → IPCB_LayerStack
└── SubstackCount() → int

IPCB_MasterLayerStack2 : IPCB_MasterLayerStack
├── Export_ToLayerStackManagerParameters(StringBuilder)
├── Import_FromLayerStackManagerParameters(StringBuilder)
├── Merge_FromLayerStackManagerParameters(StringBuilder, StringBuilder)
├── GetImpedanceProfileCount() → int
├── GetImpedanceProfileByIndex/ById(...)
├── CreateImpedanceProfile() → IPCB_ImpedanceProfile
├── ClearAllPhysicalLayers()
├── GetState_CustomData() → string
├── AddMechanicalLayer() → IPCB_MechanicalLayer
├── GetMechanicalLayer(int) → IPCB_MechanicalLayer
├── FindLayerByKind(TMechanicalLayerKind) → IPCB_MechanicalLayer
├── CreateLayerPairByKind(TMechanicalLayerPairKind) → bool
├── GetState_RoughnessModelType() → TRoughnessModelType
├── GetState_SurfaceRoughness() → double
├── GetState_RoughnessFactor() → double
├── HasMicroVias() → bool
├── HasPrintedElectronicLayers() → bool
└── GetAllAvailableLayers() → IPCB_LayerObjectIterator
```

### Key Enums

**TLayerClassID** (u8) — Layer classification filter:
```
0  eLayerClass_All
1  eLayerClass_Mechanical
2  eLayerClass_Physical
3  eLayerClass_Electrical
4  eLayerClass_Dielectric
5  eLayerClass_Signal
6  eLayerClass_InternalPlane
7  eLayerClass_SolderMask
8  eLayerClass_Overlay
9  eLayerClass_PasteMask
```

**TLayerStackStyle** (u8) — Layer pairing style:
```
0  eLayerStack_Pairs         — traditional paired layers (Top/Bottom)
1  eLayerStacks_InsidePairs  — inside-out pairing (for flex)
2  eLayerStackBuildup        — buildup stack (HDI)
3  eLayerStackCustom         — user-defined ordering
```

**TLayerStackType** (u8) — Stack type:
```
0  eBoardLayerStack          — main board stack
1  eBoardRegionLayerStack    — per-region stack (multi-region designs)
```

**TDielectricType** (u8) — Dielectric material type:
```
0  eNoDielectric
1  eCore
2  ePrePreg
3  eSurfaceMaterial
4  eFilm
```

**TComponentPlacementType** (u8) — Component mounting side:
```
0  eComponentPlacement_None
1  eComponentPlacement_BodyUp
2  eComponentPlacement_BodyDown
```

**TRoughnessModelType** (u8) — PCB surface roughness model:
```
0  RoughnessModel_NoRoughness
1  RoughnessModel_MHammerstad
2  RoughnessModel_HuraySnowball
3  RoughnessModel_MGroiss
4  RoughnessModel_Hemispherical
5  RoughnessModel_HurayBracken
```

### Layer Object Properties

Each layer in the stack is an `IPCB_LayerObject` with these properties:

| Property | Type | Description |
|----------|------|-------------|
| `LayerName` | string | User-visible name |
| `UsedByPrims` | bool | Whether any primitives use this layer |
| `V7_LayerID` | TV7_Layer (u32) | V7 layer identifier |
| `V6_LayerID` | TV6_Layer | V6/legacy layer identifier |
| `IsInLayerStack` | bool | Whether layer is in the stack |
| `DisplayInSingleLayerMode` | bool | Show in single-layer mode |
| `Id` | string | Unique ID string |

**Electrical layers** add:
- `CopperThickness` (int, Coord units)

**Signal layers** add:
- `ComponentPlacement` (TComponentPlacementType)

**Internal plane layers** add:
- `PullBackDistance` (int, Coord units)
- `NetName` (string) — assigned net

**Dielectric layers** add:
- `DielectricMaterial` (string, e.g. "FR-4")
- `DielectricType` (TDielectricType)
- `DielectricConstant` (f64, e.g. 4.5)
- `DielectricHeight` (int, Coord units)
- `DielectricLossTangent` (f64)
- `IsStiffener` (bool)

**Mechanical layers** add:
- `MechLayerEnabled` (bool)
- `DisplayInSingleLayerMode` (bool)
- `LinkToSheet` (bool)
- `Kind` (TMechanicalLayerKind)

### Serialization Format

The `Export_ToParameters(StringBuilder)` / `Import_FromParameters(StringBuilder)` pattern
strongly indicates the section stores its data as a **pipe-delimited parameter string**,
identical to the Board6 format but with additional V7/substack parameters.

Based on the Board6 legacy format and the V9_STACK format from PcbLib:
```
Per-layer parameters (index-based):
|LAYERSTACKSTYLE=0|
|V9_STACK_LAYER{N}_NAME=Top Layer|
|V9_STACK_LAYER{N}_LAYERID=16973830|
|V9_STACK_LAYER{N}_USEDBYPRIMS=TRUE|
|V9_STACK_LAYER{N}_COPTHICK=1.4mil|
|V9_STACK_LAYER{N}_DIELCONST=4.500000|
|V9_STACK_LAYER{N}_DIELHEIGHT=12mil|
|V9_STACK_LAYER{N}_DIELMATERIAL=FR-4|
|V9_STACK_LAYER{N}_DIELTYPE=1|
...

Per-substack parameters:
|V9_SUBSTACKCOUNT=1|
|V9_SUBSTACK{N}_ID=...|
|V9_SUBSTACK{N}_NAME=...|
|V9_SUBSTACK{N}_ISFLEX=FALSE|
|V9_SUBSTACK{N}_LAYERSTACKTYPE=0|
...
```

The `Export_ToLayerStackManagerParameters` method may use a different, richer
serialization used by the Layer Stack Manager (LSM) editor — possibly XML-based
given `TLayerStackDocumentFormat` has `dfXml` and `dfXmlExt` variants.

### Relationship to Board6

The Board6 section has always stored basic layer stack data via `LAYER{i}*` parameters.
`LayerStackSection` is a **newer, richer representation** that:

1. Supports the V7 layer ID system (32-bit, vs V6's 8-bit layer bytes)
2. Supports multiple substacks for rigid-flex and multi-region designs
3. Stores impedance profiles per layer
4. Integrates mechanical layers into the physical stack definition
5. Stores roughness model parameters for SI analysis

When `LayerStackSection` is present, it is **authoritative** over the Board6 layer
data. Board6 layer data is maintained for backwards compatibility with older tools.

### Observation in Test Files

**Not observed in any test file**. This section requires advanced board features:
- Multi-region stackup (rigid-flex)
- Board regions with different layer counts
- Layer Stack Manager usage in newer Altium versions

Basic 2-layer and 4-layer designs store all layer data in Board6 and do not
generate a separate `LayerStackSection`.

### Standard Section Layout

- `Header` stream: 4-byte u32le record count
- `Data` stream: block-encoded payload (likely text-mode parameter string)

---

## 3. Related Existing Implementation

### LayerKindMapping (Already Implemented)

The `LayerKindMapping` section is a **different section** that maps mechanical layer
indices to their `TMechanicalLayerKind` values. It is already implemented in our codebase:

- **Parsing**: `parse_layer_kind_mapping()` in `crates/altium-format/src/pcblib/library.rs:427`
- **Serialization**: `serialize_layer_kind_mapping()` in `crates/altium-format/src/pcblib/mod.rs:1506`
- **Data type**: `PcbLayerKindMapping { version, hash, entries: Vec<PcbLayerKindPair> }`
- **PcbDoc integration**: `LayerKindMappingSectionData` in `crates/altium-format/src/pcbdoc/mod.rs:69`

This is NOT the same as `LayerToLayerMapping`. LayerKindMapping maps layer index → kind;
LayerToLayerMapping maps source layer → destination layer.

---

## 4. Layer Stack Manager (LSM) SDK

The Layer Stack Manager is a newer Altium subsystem (`Rt_LayerStackManager.Interfaces`)
that provides a richer API for stackup management. It uses its own type system:

| LSM Type | PCB Equivalent |
|----------|---------------|
| `TLsmSdkStackupType` | `TLayerStackStyle` |
| `TLsmSdkComponentPlacement` | `TComponentPlacementType` |
| `TLsmSdkRoughnessModelType` | `TRoughnessModelType` |
| `TLsmSdkDrillSpanType` | Drill pair types |
| `TLsmSdkViaSpanType` | Via span types |
| `TLsmSdkImpedanceProfileType` | Impedance profile |

The LSM introduces additional concepts not in the legacy stack:
- **Drill spans** — explicit drill pair definitions
- **Via spans** — via span definitions per substack
- **Impedance profiles** — per-substack impedance targets
- **Transmission lines** — SI modeling parameters

The `TLayerStackDocumentFormat` enum suggests the LSM can serialize to:
- `dfDefault` — native binary/parameter format
- `dfXml` — XML format
- `dfXmlExt` — extended XML format

---

## 5. Implementation Priority

Both sections are **not observed in test files**, making them low priority for
initial implementation. When needed:

1. **LayerToLayerMapping** — Implement as a standard param section with a
   `HashMap<u32, u32>` (TV7_Layer source → TV7_Layer dest). Parse the
   `ExportMapToString` format from actual test data when available.

2. **LayerStackSection** — Implement when advanced multi-stackup PcbDoc test
   files become available. Will need to parse the `Export_ToParameters` format
   which likely extends the Board6 `LAYER{i}*` pattern with V7/substack params.

Both sections should follow the standard Header/Data layout and can be added to
`ParamSectionKind` (LayerStackSection) and as a custom-parsed section
(LayerToLayerMapping) in the PcbDoc loader.
