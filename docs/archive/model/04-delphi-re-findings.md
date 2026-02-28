# Delphi Reverse Engineering Findings

Results from Ghidra analysis of the native Delphi DLLs: `Advpcb.dll` (392K functions) and `AdvSch.dll` (126K functions).

## 1. PCB Object Type Enum (TObjectId)

From `PcbApi_CreateObject` dispatch and cross-referenced with .NET `RT_PCB.TObjectId`:

```
enum TObjectId : byte {
    eNoObject          = 0,   // No object / unknown
    eArcObject         = 1,   // Arc primitive
    ePadObject         = 2,   // Pad primitive
    eViaObject         = 3,   // Via primitive
    eTrackObject       = 4,   // Track primitive
    eTextObject        = 5,   // Text/string primitive
    eFillObject        = 6,   // Fill (solid rectangle) primitive
    eConnectionObject  = 7,   // Connection (ratsnest) line
    eNetObject         = 8,   // Net definition
    eComponentObject   = 9,   // Component (footprint instance)
    ePolyObject        = 10,  // Polygon pour
    eRegionObject      = 11,  // Region (polygon shape)
    eComponentBodyObject = 12, // 3D component body
    eDimensionObject   = 13,  // Dimension annotation
    eCoordinateObject  = 14,  // Coordinate marker
    eClassObject       = 15,  // Design class
    eRuleObject        = 16,  // Design rule
    eFromToObject      = 17,  // FromTo definition
    eDifferentialPairObject = 18, // Differential pair
    eViolationObject   = 19,  // DRC violation marker
    eEmbeddedObject    = 20,  // Embedded object
    eEmbeddedBoardObject = 21, // Embedded board
    eSplitPlaneObject  = 22,  // Split plane region
    eTraceObject       = 23,  // Trace (routed path)
    eSpareViaObject    = 24,  // Spare via
    eBoardObject       = 25,  // Board container
    eBoardOutlineObject = 26, // Board outline
}
```

**Delphi verification** (from `FUN_0469e1e0` type check at vtable offset 0x348):
- `PcbApi_QueryArc`: checks type == 1 (eArcObject)
- `PcbApi_QueryPad`: checks type == 2 (ePadObject)
- `PcbApi_QueryTrack`: checks type == 4 (eTrackObject)
- `PcbApi_QueryText`: checks type == 5 (eTextObject)
- `PcbApi_QueryFill`: checks type == 6 (eFillObject)
- `PcbApi_QueryRegion`: checks type == 11 (eRegionObject)
- `PcbApi_QueryDimension`: checks type == 13 (eDimensionObject)
- `PcbApi_QueryCoordinate`: checks type == 14 (eCoordinateObject)
- `PcbApi_QueryEmbedded`: checks type == 20 (eEmbeddedObject)
- `PcbApi_QueryEmbeddedBoard`: checks type == 21 (eEmbeddedBoardObject)
- `PcbApi_QuerySplitPlane`: checks type == 22 (eSplitPlaneObject)
- `PcbApi_QueryBoard/Sheet`: checks type == 25 (eBoardObject)

`PcbApi_CreateObject` class references by type:
| Type | Address of Class VMT | Object Type |
|------|---------------------|-------------|
| 1 | 0x0137d300 | Arc |
| 2 | 0x045c4070 | Pad |
| 3 | 0x0462ae98 | Via |
| 4 | 0x0133ac80 | Track |
| 5 | 0x0128e250 | Text |
| 6 | 0x0133ba00 | Fill |
| 7 | 0x013a8b50 | Connection |
| 8 | 0x01379430 | Net |
| 9 | 0x0445d8b0 | Component |
| 10 | 0x013551d8 | Polygon |
| 13 | 0x019659c0 | Dimension |
| 14 | 0x020e6f30 | Coordinate |
| 17 | 0x01a33e00 | FromTo |
| 18 | 0x01f8efe0 | DifferentialPair |
| 20 | 0x017a5800 | Embedded |
| 22 | 0x01361be0 | SplitPlane |

## 2. PCB Object Name Strings

From .NET `Consts.cs` `cObjectIdStrings`:

| TObjectId | String Name |
|-----------|-------------|
| eNoObject | "NoObject" |
| eArcObject | "Arc" |
| ePadObject | "Pad" |
| eViaObject | "Via" |
| eTrackObject | "Track" |
| eTextObject | "Text" |
| eFillObject | "Fill" |
| eConnectionObject | "Connection" |
| eNetObject | "Net" |
| eComponentObject | "Component" |
| ePolyObject | "Poly" |
| eRegionObject | "PolyRegion" |
| eComponentBodyObject | "ComponentBody" |
| eDimensionObject | "Dimension" |
| eCoordinateObject | "Coordinate" |
| eClassObject | "Class" |
| eRuleObject | "Rule" |
| eFromToObject | "FromTo" |
| eDifferentialPairObject | "DifferentialPair" |
| eViolationObject | "Violation" |
| eEmbeddedObject | "Embedded" |
| eEmbeddedBoardObject | "EmbeddedBoard" |
| eSplitPlaneObject | "SplitPlane" |
| eTraceObject | "Trace" |
| eSpareViaObject | "SpareVia" |
| eBoardObject | "Board" |
| eBoardOutlineObject | "BoardOutline" |

## 3. PCB Layer ID System (TV6_Layer)

The layer system uses a byte value (`TV6_Layer` enum). In the Delphi code, layers are constructed through a series of helper functions:

### V6 Layer Enum (byte values, 0-based)

```
enum TV6_Layer : byte {
    eV6_NoLayer        = 0,
    eV6_TopLayer       = 1,
    eV6_MidLayer1      = 2,
    eV6_MidLayer2      = 3,
    ...
    eV6_MidLayer30     = 31,
    eV6_BottomLayer    = 32,  // 0x20
    eV6_TopOverlay     = 33,  // 0x21
    eV6_BottomOverlay  = 34,  // 0x22
    eV6_TopPaste       = 35,  // 0x23
    eV6_BottomPaste    = 36,  // 0x24
    eV6_TopSolder      = 37,  // 0x25
    eV6_BottomSolder   = 38,  // 0x26
    eV6_InternalPlane1 = 39,  // 0x27 (also identified in Delphi as 0x38 + n offset)
    eV6_InternalPlane2 = 40,  // 0x28
    ...
    eV6_InternalPlane16 = 54, // 0x36
    eV6_DrillGuide     = 55,  // 0x37
    eV6_KeepOutLayer   = 56,  // 0x38
    eV6_Mechanical1    = 57,  // 0x39 (Delphi: 0x26 + mechanical_number)
    eV6_Mechanical2    = 58,  // 0x3A
    ...
    eV6_Mechanical16   = 72,  // 0x48
    eV6_DrillDrawing   = 73,  // 0x49
    eV6_MultiLayer     = 74,  // 0x4A
    eV6_ConnectLayer   = 75,  // 0x4B
    eV6_BackGroundLayer = 76, // 0x4C
    eV6_DRCErrorLayer  = 77,  // 0x4D
    eV6_HighlightLayer = 78,  // 0x4E
    eV6_GridColor1     = 79,  // 0x4F
    eV6_GridColor10    = 80,  // 0x50
    eV6_PadHoleLayer   = 81,  // 0x51
    eV6_ViaHoleLayer   = 82,  // 0x52
}
```

### V7 Layer Structure (32-bit)

The V7 layer is a 32-bit value with packed fields (from .NET `TV7_Layer` struct):

```
[StructLayout(Explicit, Pack = 1)]
struct TV7_Layer {
    [FieldOffset(0)] uint ID;        // Full 32-bit layer ID
    [FieldOffset(0)] ushort Species; // Lower 16 bits
    [FieldOffset(2)] byte Genus;     // Byte at offset 2
    [FieldOffset(3)] byte Family;    // Byte at offset 3

    // Alternate interpretation:
    [FieldOffset(0)] ushort N;       // Layer number
    [FieldOffset(2)] ushort Flags;   // Flags
}
```

When `Genus == 0` and `Family == 0`, the Species byte matches the V6 layer enum values (backward-compatible).

### Delphi Layer Construction

From `FUN_00fd9fc0` (signal layer constructor):
- Layer 1 = Top Layer (returns internal ID 1)
- Layers 2-30 = Mid Layers (returns the value directly)
- Max layer = Bottom Layer (returns internal ID 0x20)
- Out-of-range layers construct `CONCAT22(0x101, (short)param)` for extended signal layers

From `FUN_00fda2a0` (mechanical layer constructor):
- Mechanical 1-16 = returns `param + 0x26` (values 39-54)
- Extended mechanical: constructs `CONCAT22(0x104, (short)param)` for mechanical layers > 16

From `FUN_00fda410` (internal plane layer constructor):
- Internal plane 1-16 = returns `param + 0x38` (values 57-72 in the V6 scheme)
  - Note: there's an offset discrepancy with V6 enum. The Delphi internal mapping may differ slightly from the published V6 enum ordering.
- Extended internal planes: constructs `CONCAT13(4, (int3)param)` for > 16 planes

### External Layer Byte Mapping

The `PcbApi_QueryBoardLayerInfo` function maps an external "layer byte" (0-82) to internal layer objects. The mapping in full:

| External Byte | Internal Layer | Description |
|---------------|----------------|-------------|
| 0 | Signal(0) | No Layer / Default |
| 1 | Signal(1) | Mid Layer 1 (1-indexed) |
| 2 | Signal(2) | Mid Layer 2 |
| 3 | Signal(3) | Mid Layer 3 |
| 4 | Signal(4) | Mid Layer 4 |
| 5 | Signal(5) | Mid Layer 5 |
| ... | ... | ... |
| 0x14 (20) | Signal(0x14) | Mid Layer 20 |
| 0x15 (21) | Signal(0x15) | Mid Layer 21 |
| ... | ... | ... |
| 0x1F (31) | Signal(0x1F) | Mid Layer 30 |
| 0x20 (32) | BottomLayer | Bottom Layer |
| 0x21 (33) | 0x21 | Top Overlay |
| 0x22 (34) | 0x22 | Bottom Overlay |
| 0x23 (35) | 0x23 | Top Paste |
| 0x24 (36) | 0x24 | Bottom Paste |
| 0x25 (37) | 0x25 | Top Solder |
| 0x26 (38) | 0x26 | Bottom Solder |
| 0x27 (39) | Mechanical(1) | Mechanical Layer 1 |
| 0x28 (40) | Mechanical(2) | Mechanical Layer 2 |
| 0x29 (41) | Mechanical(3) | Mechanical Layer 3 |
| 0x2A (42) | Mechanical(4) | Mechanical Layer 4 |
| ... | ... | ... |
| 0x36 (54) | Mechanical(16) | Mechanical Layer 16 |
| 0x37 (55) | 0x37 | Drill Guide |
| 0x38 (56) | 0x38 | Drill Drawing |
| 0x39 (57) | InternalPlane(1) | Internal Plane 1 |
| 0x3A (58) | InternalPlane(2) | Internal Plane 2 |
| ... | ... | ... |
| 0x48 (72) | InternalPlane(16) | Internal Plane 16 |
| 0x49 (73) | 0x49 | Keep-Out Layer |
| 0x4A (74) | 0x4A | Multi-Layer |
| 0x4B (75) | 0x4B | Connect Layer |
| 0x4C (76) | 0x4C | Background Layer |
| 0x4D (77) | 0x4D | DRC Error Layer |
| 0x4E (78) | 0x4E | Highlight Layer |
| 0x4F (79) | 0x4F | Grid Color 1 |
| 0x50 (80) | 0x50 | Grid Color 10 |
| 0x51 (81) | 0x51 | Pad Hole Layer |
| 0x52 (82) | 0x52 | Via Hole Layer |

## 4. Coordinate System

Both PCB and Schematic use 10,000 internal units per mil:

```
MilsToCoord(mils) = (int)Math.Round(mils * 10000.0)
CoordToMils(coord) = (double)coord / 10000.0
CoordToMM(coord) = (double)coord * 0.0254 / 10000.0
```

This means:
- 1 mil = 10,000 internal units
- 1 mm = 393,701 internal units (approx)
- 1 inch = 10,000,000 internal units
- Coordinates are stored as 32-bit signed integers (int32)
- Maximum representable range: ~214,748 mils = ~5,454 mm

## 5. PCB Pad Shape Enum (TShape)

```
enum TShape : byte {
    eNoShape          = 0,
    eRounded          = 1,  // Round pad
    eRectangular      = 2,  // Rectangular pad
    eOctagonal        = 3,  // Octagonal pad
    eCircleShape      = 4,  // Circle (for regions)
    eArcShape         = 5,  // Arc (for regions)
    eTerminator       = 6,  // Terminator shape
    eRoundRectShape   = 7,  // Round rectangle
    eRotatedRectShape = 8,  // Rotated rectangle
    eRoundedRectangular = 9, // Rounded rectangular
    eCustomShape      = 10, // Custom shape (defined by region)
}
```

## 6. Pad Stack Mode

```
enum TPadMode : byte {
    ePadMode_Simple        = 0,  // Same shape on all layers
    ePadMode_LocalStack    = 1,  // Per-layer shape definition
    ePadMode_ExternalStack = 2,  // References external pad/via library
}
```

## 7. PCB File Format Versions

```
enum TAdvPCBFileFormatVersion : byte {
    ePCBFileFormatNone            = 0,
    eAdvPCBFormat_Binary_V3       = 1,
    eAdvPCBFormat_Library_V3      = 2,
    eAdvPCBFormat_ASCII_V3        = 3,
    eAdvPCBFormat_Binary_V4       = 4,
    eAdvPCBFormat_Library_V4      = 5,
    eAdvPCBFormat_ASCII_V4        = 6,
    eAdvPCBFormat_Binary_V5       = 7,
    eAdvPCBFormat_Library_V5      = 8,
    eAdvPCBFormat_ASCII_V5        = 9,
    eAdvPCBFormat_Binary_V6       = 10,
    eAdvPCBFormat_Library_V6      = 11,
    eAdvPCBFormat_ASCII_V6        = 12,
    eAdvPCBFormat_Binary_V6_CS    = 13, // Circuit Studio format
    eAdvPCBFormat_Binary_V6_CM    = 14, // Circuit Maker format
    eAdvPCBFormat_Binary_V6_PCBWorks = 15, // PCBWorks format
    eAdvPCBFormat_PadViaLibrary_V6 = 16, // Pad/Via library
}
```

## 8. Schematic Object Type Enum and Binary Record Codes

From `Rt_Schematic.TObjectId` and `SchDataUtils.GetBinaryCodeByObjectId`:

| Object ID (Enum) | Binary Record Code | Name |
|-------------------|--------------------|------|
| eSchComponent | 1 | Component/Part |
| ePin | 2 | Pin |
| eSymbol | 3 | Symbol |
| eLabel | 4 | Label (net label style) |
| eBezier | 5 | Bezier curve |
| ePolyline | 6 | Polyline |
| ePolygon | 7 | Polygon |
| eEllipse | 8 | Ellipse |
| ePie | 9 | Pie (sector) |
| eRoundRectangle | 10 | Rounded rectangle |
| eEllipticalArc | 11 | Elliptical arc |
| eArc | 12 | Arc |
| eLine | 13 | Line |
| eRectangle | 14 | Rectangle |
| eSheetSymbol | 15 | Sheet symbol |
| eSheetEntry | 16 | Sheet entry |
| ePowerObject | 17 | Power port |
| ePort | 18 | Port |
| eNoERC | 22 | No-ERC directive |
| eErrorMarker | 23 | Error marker |
| eNetLabel | 25 | Net label |
| eBus | 26 | Bus |
| eWire | 27 | Wire |
| eTextFrame | 28 | Text frame |
| eJunction | 29 | Junction |
| eImage | 30 | Image |
| eSheet | 31 | Sheet properties |
| eSheetName | 32 | Sheet name |
| eSheetFileName | 33 | Sheet file name |
| eDesignator | 34 | Designator |
| eBusEntry | 37 | Bus entry |
| eTemplate | 39 | Template |
| eTaskHolder | 40 | Task holder |
| eParameter | 41 | Parameter |
| eParameterSet | 43 | Parameter set |
| eImplementationsList | 44 | Implementations list |
| eImplementation | 45 | Implementation |
| eImplementationMap | 46 | Implementation map |
| eMapDefiner | 47 | Map definer |
| eParameterList | 48 | Parameter list |
| eHarnessWiringDiagram | 104 | Harness wiring diagram |
| eHarnessLayoutDrawing | 105 | Harness layout drawing |
| eHarnessComponent | 106 | Harness component |
| eHarnessWire | 107 | Harness wire |
| eHarnessSplice | 108 | Harness splice |
| eHarnessLayoutLabel | 109 | Harness layout label |
| eHarnessLayoutConnectionPoint | 110 | Harness connection point |
| eHarnessBundle | 111 | Harness bundle |
| eHarnessLogicalSignal | 112 | Harness logical signal |
| eHarnessPin | 113 | Harness pin |
| eHarnessWireLabel | 114 | Harness wire label |
| eHarnessWireData | 115 | Harness wire data |
| eHarnessSpliceData | 116 | Harness splice data |
| eHarnessShield | 117 | Harness shield |
| eHarnessTwist | 118 | Harness twist |
| eHarnessNoConnect | 119 | Harness no connect |
| eHarnessNoConnectData | 120 | Harness no connect data |
| eHarnessShieldData | 121 | Harness shield data |
| eHarnessTwistData | 122 | Harness twist data |
| eHarnessCable | 123 | Harness cable |
| eHarnessCableData | 124 | Harness cable data |
| eHarnessAssociatedParts | 125 | Harness associated parts |
| eLineView | 126 | Line view |
| eHarnessLibrary | 127 | Harness library |
| eHarnessCovering | 128 | Harness covering |
| eObjectDefinition | 129 | Object definition |
| eHarnessWireBreak | 130 | Harness wire break |
| eAssociatedObjects | 131 | Associated objects |
| eElectronicsSystemDesignDocument | 132 | Electronics system design |
| eFunctionalBlock | 133 | Functional block |
| eFunctionalConnectionLine | 134 | Functional connection |
| eFunctionalTextFrame | 135 | Functional text frame |
| eSchematicBlock | 136 | Schematic block |
| eReuseSheetSymbol | 137 | Reuse sheet symbol |
| eReuseBlockImplementationInfo | 138 | Reuse block implementation |
| eSchLib | 200 | Schematic library header |
| eNote | 209 | Note |
| eProbe | 210 | Probe |
| eCompileMask | 211 | Compile mask |
| eHarnessConnector | 215 | Harness connector |
| eHarnessEntry | 216 | Harness entry |
| eHarnessConnectorType | 217 | Harness connector type |
| eSignalHarness | 218 | Signal harness |
| eHighLevelCodeSymbol | 220 | High-level code symbol |
| eHighLevelCodeEntry | 221 | High-level code entry |
| HighLevelCodeName | 222 | High-level code name* |
| HighLevelCodeFileName | 223 | High-level code filename* |
| eBlanket | 225 | Blanket |
| eHyperlink | 226 | Hyperlink |
| eRichTextDocument | 240 | Rich text document |
| eRTFLink | 241 | RTF link |

*Note: codes 222-223 are computed from special conditions, not directly from TObjectId.

## 9. PCB Viewable Object IDs

The `TViewableObjectID` enum provides a more granular view of PCB objects, particularly for dimensions (which share TObjectId = 13 but have different viewable IDs) and rules:

### Dimension Sub-Types
| Viewable ID | Value | Name |
|-------------|-------|------|
| eViewableObject_LinearDimension | 11 | Linear dimension |
| eViewableObject_AngularDimension | 12 | Angular dimension |
| eViewableObject_RadialDimension | 13 | Radial dimension |
| eViewableObject_LeaderDimension | 14 | Leader dimension |
| eViewableObject_DatumDimension | 15 | Datum dimension |
| eViewableObject_BaselineDimension | 16 | Baseline dimension |
| eViewableObject_CenterDimension | 17 | Center dimension |
| eViewableObject_OriginalDimension | 18 | Original dimension |
| eViewableObject_LinearDiameterDimension | 19 | Linear diameter |
| eViewableObject_RadialDiameterDimension | 20 | Radial diameter |

## 10. Delphi API Architecture

### Object Model
The Delphi PCB API uses a consistent pattern:
1. All PCB objects share a common base with property accessors at fixed vtable offsets
2. `FUN_0469e1e0` reads the object type by calling vtable[0x348] (GetObjectType virtual method)
3. `FUN_0469da80` / `FUN_0469db40` read 16-bit properties (likely layer, flags)
4. `FUN_0469e260` reads 32-bit property (likely selection state or attributes)
5. `FUN_0469ea90` / `FUN_0469e6e0` read 64-bit properties (likely net name/string, coordinates)
6. `FUN_046a0020` reads 8-bit property (likely boolean flag)

### Query API Pattern
All `PcbApi_Query*` functions follow the same pattern:
- `param_1`: Mode byte (0 = set values, 1 = get values, 2 = defaults/set mode)
- `param_2`: Object handle (pointer to internal Delphi object)
- Remaining params: Field value pointers (in/out depending on mode)

### Iterator API
```
PcbApi_CreateIterator()       -> iterator_handle
PcbApi_GetFirstObject(iter)   -> object_handle
PcbApi_GetNextObject(iter)    -> object_handle
PcbApi_DestroyIterator(iter)
```

Spatial iterator variant:
```
PcbApi_CreateSpatialIterator() -> spatial_iter
PcbApi_GetFirstSpatialObject() -> object_handle
PcbApi_GetNextSpatialObject()  -> object_handle
PcbApi_DestroySpatialIterator()
```

### Schematic API Pattern
The schematic API (`SchAPI_*`) follows a similar pattern but uses COM-style interfaces (QueryInterface via `FUN_004604c0`) rather than direct vtable dispatch. Each object type has a GUID-identified interface.

## 11. Key Exported Functions

### Advpcb.dll (PCB - 387 named exports)

Critical functions for file I/O and object manipulation:

| Function | Purpose |
|----------|---------|
| PcbApi_LoadBoardByFullFileName | Load PCB file |
| PcbApi_CloseDocumentByFullFileName | Close PCB file |
| PcbApi_CreateObject | Create new PCB object by type ID |
| PcbApi_DestroyObject | Delete PCB object |
| PcbApi_CreateIterator/DestroyIterator | Iterate all objects |
| PcbApi_CreateSpatialIterator | Iterate objects in region |
| PcbApi_AddObjectToContainer | Add object to parent |
| PcbApi_DeleteObjectFromContainer | Remove from parent |
| PcbApi_QueryPrimitive | Get/set base primitive fields |
| PcbApi_QueryTrack | Get/set track fields |
| PcbApi_QueryArc | Get/set arc fields |
| PcbApi_QueryPad | Get/set pad fields (37 params!) |
| PcbApi_QueryVia | Get/set via fields |
| PcbApi_QueryFill | Get/set fill fields |
| PcbApi_QueryText | Get/set text fields |
| PcbApi_QueryComponent | Get/set component fields |
| PcbApi_QueryPolygon | Get/set polygon fields |
| PcbApi_QueryRegion | Get/set region fields |
| PcbApi_QueryDimension | Get/set dimension fields |
| PcbApi_QueryBoard | Get/set board properties |
| PcbApi_QueryBoardLayerInfo | Get layer properties |
| PcbApi_QueryLayer | Get detailed layer info |
| PcbApi_QueryRule | Get/set design rule |
| PcbApi_SetBoardIsFullyLoaded | Signal loading complete |
| PcbApi_QueryObjectParameters | Get object parameters |

### AdvSch.dll (Schematic - ~100 named exports)

| Function | Purpose |
|----------|---------|
| SchAPI_CreateObject | Create schematic object |
| SchAPI_DestroyObject | Delete schematic object |
| SchAPI_AddObjectToContainer | Add to parent |
| SchAPI_CreateIterator | Iterate objects |
| SchAPI_GetObjectIdFromObjectHandle | Get object type |
| SchAPI_QueryPrimitive | Base primitive fields |
| SchAPI_QueryWire | Wire fields |
| SchAPI_QueryBus | Bus fields |
| SchAPI_QueryPin | Pin fields |
| SchAPI_QuerySchPart | Component/part fields |
| SchAPI_QueryPort | Port fields |
| SchAPI_QueryLabel | Label fields |
| SchAPI_QueryNetLabel | Net label fields |
| SchAPI_QueryText | Text fields |
| SchAPI_QueryArc | Arc fields |
| SchAPI_QueryLine | Line fields |
| SchAPI_QueryRectangle | Rectangle fields |
| SchAPI_QueryPolygon | Polygon fields |
| SchAPI_QuerySheetSymbol | Sheet symbol fields |
| SchAPI_QuerySheetEntry | Sheet entry fields |
| SchAPI_QueryImage | Image fields |
| SchAPI_QueryDocumentOptions | Document options |
| SchAPI_QuerySchematicPreferences | Preferences |

## 12. Schematic Internal-to-External Type Mapping

The Delphi schematic code has a type mapping function `FUN_021dbf70` that converts internal COM interface IDs to external/published type IDs. This confirms the binary record codes are used as the wire format:

| Internal ID | External ID | Object |
|-------------|-------------|--------|
| 4 | 0x2c (44) | (ImplementationsList) |
| 5 | 0x0e (14) | (Coordinate?) |
| 7 | 4 | Label |
| 8 | 1 | SchComponent |
| 9 | 7 | Polygon |
| 10 | 0x2d (45) | (Implementation) |
| 11 | 9 | Pie |
| 12 | 0x25 (37) | BusEntry |
| 13 | 0x36 (54) | (Extended?) |
| 15 | 6 | Polyline |
| 16 | 10 | RoundRectangle |
| 17 | 0x27 (39) | Template |
| 18 | 0x28 (40) | TaskHolder |
| 19 | 0x37 (55) | (Extended?) |
| 20 | 3 | Symbol |
| 21 | 2 | Pin |
| 22 | 11 | EllipticalArc |
| 23 | 12 | Arc |
| 24 | 15 | SheetSymbol |
| 25 | 0x19 (25) | NetLabel |
| 26 | 0x1a (26) | Bus |
| 27 | 0x1b (27) | Wire... |
| ... | ... | ... |

## 13. Summary of Critical Findings for Implementation

### For PCB parsing:
1. Object types are byte-sized enums (0-26), stored in binary records
2. Layer IDs are bytes (V6) or 32-bit packed structs (V7)
3. Coordinates are 32-bit signed integers, 10000 units per mil
4. Pad objects have the most complex field layout (37 parameters in QueryPad)
5. Polygons can be type 10 (ePolyObject) or type 22 (eSplitPlaneObject)
6. The Board object (type 25) serves as the root container

### For Schematic parsing:
1. Binary record codes are NOT sequential with the TObjectId enum - use the explicit mapping table
2. The most common records: Component(1), Pin(2), Wire(27), NetLabel(25), Port(18), PowerObject(17)
3. Harness records use codes 104-138
4. Extended codes (200+) are for library headers and special objects

### Key differences between Delphi and .NET:
- The Delphi code IS the original implementation; the .NET code wraps it via COM interop
- Delphi uses direct vtable dispatch; .NET uses COM QueryInterface
- The Delphi `PcbApi_*` functions are the actual exported DLL API called by plugins
- The .NET `IPCB_*` interfaces map 1:1 to the Delphi object hierarchy
- ObjectId values are identical between Delphi and .NET (confirmed by cross-referencing)
