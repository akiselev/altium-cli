# .NET PCB Types and Data Model

Research from decompiled C# source in `AD26-dotnet/`.

**Key finding:** The PCB core data model lives in **Delphi** (native COM), not .NET. The .NET assemblies provide COM interop wrappers (`IPCB_*` interfaces) and a property panel data model (`InteractiveProperties.Providers.PCB.DataModel`). There is no `Altium.PCB.DataModel` or `Altium.PCB.BinaryLoader` .NET assembly -- binary serialization is entirely Delphi-side.

## Source Assemblies

| Assembly | Role |
|---|---|
| `Altium.SDK.Interfaces/PCB/` | Public SDK COM interface declarations (IPCB_*, enums, structs) |
| `Altium.Edp.Interfaces/RT_PCB/` | Internal COM interface declarations (same types, more complete) |
| `Altium.Edp.Interfaces/Pcbtypes/` | Additional PCB type enums |
| `InteractiveProperties.Providers.PCB.DataModel` | Property panel wrappers around IPCB_* objects |
| `Altium.PCB.FullComponents` | Component variant management (not primitive data) |
| `Altium.PCB.CollaborateMerge.Module` | Collaborative merge UI (not primitive data) |

---

## 1. PCB Object Type IDs (TObjectId)

**Source:** `Altium.Edp.Interfaces/RT_PCB/TObjectId.cs` (byte enum)

```csharp
public enum TObjectId : byte
{
    eNoObject           = 0,
    eArcObject          = 1,
    ePadObject          = 2,
    eViaObject          = 3,
    eTrackObject        = 4,
    eTextObject         = 5,
    eFillObject         = 6,
    eConnectionObject   = 7,
    eNetObject          = 8,
    eComponentObject    = 9,
    ePolyObject         = 10,
    eRegionObject       = 11,
    eComponentBodyObject = 12,
    eDimensionObject    = 13,
    eCoordinateObject   = 14,
    eClassObject        = 15,
    eRuleObject         = 16,
    eFromToObject       = 17,
    eDifferentialPairObject = 18,
    eViolationObject    = 19,
    eEmbeddedObject     = 20,
    eEmbeddedBoardObject = 21,
    eSplitPlaneObject   = 22,
    eTraceObject        = 23,
    eSpareViaObject     = 24,
    eBoardObject        = 25,
    eBoardOutlineObject = 26,
}
```

These are the **record type byte values** stored in PCB binary files. The first byte of each record identifies the primitive type.

## 2. Viewable Object IDs (TViewableObjectID)

**Source:** `Altium.Edp.Interfaces/RT_PCB/TViewableObjectID.cs` (byte enum)

A superset of TObjectId used for the UI/view layer. Notable additional viewable types beyond the basic primitives:

| Value | Name | Notes |
|---|---|---|
| 0 | eViewableObject_None | |
| 1-10 | Arc, Pad, Via, Track, Text, Fill, Connection, Net, Component, Poly | Same as TObjectId |
| 11-20 | LinearDimension..Coordinate | Dimension subtypes |
| 21 | Class | |
| 22-82 | Rule_* variants | Design rule subtypes |
| 83 | FromTo | |
| 84 | DifferentialPair | |
| 85 | Violation | |
| 86 | Board | |
| 87 | BoardOutline | |
| 88 | Group | |
| 89 | Clipboard | |
| 90 | SplitPlane | |
| 91 | EmbeddedBoard | |
| 92 | Region | |
| 93 | ComponentBody | |
| 94-95 | AssyTestPoint rules | |
| 96 | OwnerDraw | |
| 97 | DrillTable | |
| 98 | ViaStitching | |
| 99 | LayerStackTable | |
| 100 | Viewport | |
| 101 | BoardRegion | |
| 103 | AccordionObject | |
| 104 | OLEObject | |
| 106 | ViaShielding | |
| 108 | MultilineText | |
| 110 | CoverlayPoly | |
| 111 | PinPair | |
| 113 | StackedVia | |
| 114 | StaggeredVia | |
| 117 | Rectangle | |
| 119 | Wirebond | |
| 121 | ReuseBlock | |

## 3. ObjectIdOffsets (Interactive Properties Mapping)

**Source:** `InteractiveProperties.Providers.PCB/ObjectIdOffsets.cs`

The property panel uses composite IDs = base TObjectId + offset for specialized views:

```
PcbSmartUnion       = 1000   (eNoObject + 1000 + viewableId)
PcbDocumentSet      = 2000   (eBoardObject + 2000 for library)
PcbDimension        = 3000   (eDimensionObject + 3000 + viewableId)
PcbEmbeddedBoard    = 4000
PcbRegion           = 5000   (eRegionObject + 5000 for BoardRegion)
PcbDocument3D       = 6000   (eBoardObject + 6000 for 3D routed)
PcbKeepOut          = 7000
PcbWirebond         = 8000
PcbDesignator       = 9000
PcbComment          = 10000
PcbTrombone         = 11000
PcbSawtooth         = 12000
```

---

## 4. Layer System

### 4.1 TV6_Layer (Legacy Layer IDs)

**Source:** `Altium.Edp.Interfaces/RT_PCB/TV6_Layer.cs` (byte enum, 0-87)

This is the **original/legacy layer numbering** used in binary file formats:

| Range | Layers |
|---|---|
| 0 | eV6_NoLayer |
| 1 | eV6_TopLayer |
| 2-31 | eV6_MidLayer1..30 |
| 32 | eV6_BottomLayer |
| 33 | eV6_TopOverlay (silkscreen) |
| 34 | eV6_BottomOverlay |
| 35 | eV6_TopPaste |
| 36 | eV6_BottomPaste |
| 37 | eV6_TopSolder |
| 38 | eV6_BottomSolder |
| 39-54 | eV6_InternalPlane1..16 |
| 55 | eV6_DrillGuide |
| 56 | eV6_KeepOutLayer |
| 57-72 | eV6_Mechanical1..16 |
| 73 | eV6_DrillDrawing |
| 74 | eV6_MultiLayer |
| 75 | eV6_ConnectLayer |
| 76 | eV6_BackGroundLayer |
| 77 | eV6_DRCErrorLayer |
| 78 | eV6_HighlightLayer |
| 79 | eV6_GridColor1 |
| 80 | eV6_GridColor10 |
| 81 | eV6_PadHoleLayer |
| 82 | eV6_ViaHoleLayer |

**Critical note:** `eV6_MultiLayer` = 74. Code confirms: `IsMultiLayer => Layer.ID == 74`.

### 4.2 TLayerConstant (Named Constants)

**Source:** `Altium.Edp.Interfaces/RT_PCB/TLayerConstant.cs` (byte enum)

Same numeric values as TV6_Layer with some additions:
- `cNoLayer` = 0, `cIgnoreLayer` = 1 (shifted by 1 vs TV6)
- Additional meta-layers: `cTopPadMasterPlot`, `cBottomPadMasterPlot`, `cV7_MidLayers`, `cAllLayers`, `cSignalLayers`, `cInternalPlaneLayers`, `cElectricalLayers`, `cMechanicalLayers`, `cDielectricLayers`

### 4.3 TV7_Layer (New Extended Layer System)

**Not an enum** -- it's a struct/opaque type passed through COM. The V7 layer system supports more than 256 layers (extended mechanical layers, etc.). Used via `GetState_V7Layer()` / `SetState_V7Layer()` on primitives.

### 4.4 TLayerPartition (V7 Layer Groups)

**Source:** `Altium.Edp.Interfaces/RT_PCB/TLayerPartition.cs`

Groups used for the extended layer system:
```
eV7_NoLayer, eV7_TopLayer, eMidLayers, eV7_BottomLayer,
eInternalPlaneLayers, eMechanicalLayers,
eV7_TopOverlay, eV7_BottomOverlay, eV7_TopPaste, eV7_BottomPaste,
eV7_TopSolder, eV7_BottomSolder, eV7_DrillGuide, eV7_KeepOutLayer,
eV7_DrillDrawing, eV7_MultiLayer, eV7_ConnectLayer,
eV7_BackGroundLayer, eV7_DRCErrorLayer, eV7_HighlightLayer,
eV7_GridColor1, eV7_GridColor10, eV7_PadHoleLayer, eV7_ViaHoleLayer,
eV7_TopPadMasterPlot, eV7_BottomPadMasterPlot, eV7_DRCDetailLayer,
eMidDielectricLayers, eTopCoverlayOutlineLayers, eBottomCoverlayOutlineLayers
```

### 4.5 TLayerClassID

**Source:** `Altium.Edp.Interfaces/RT_PCB/TLayerClassID.cs`

```
eLayerClass_All, eLayerClass_Mechanical, eLayerClass_Physical,
eLayerClass_Electrical, eLayerClass_Dielectric, eLayerClass_Signal,
eLayerClass_InternalPlane, eLayerClass_SolderMask,
eLayerClass_Overlay, eLayerClass_PasteMask
```

### 4.6 TLayerObjectKind

**Source:** `Altium.Edp.Interfaces/Pcbtypes/TLayerObjectKind.cs`

```
eCopperLayer = 0, eDielectric = 1
```

### 4.7 TLayerStackType

```
eBoardLayerStack = 0, eBoardRegionLayerStack = 1
```

---

## 5. Pad/Via Shape Enums

### TShape (Pad/Via Shapes)

**Source:** `Altium.Edp.Interfaces/RT_PCB/TShape.cs` (byte enum)

```
eNoShape            = 0
eRounded            = 1    // round pad
eRectangular        = 2    // rectangle
eOctagonal          = 3
eCircleShape        = 4
eArcShape           = 5
eTerminator         = 6
eRoundRectShape     = 7
eRotatedRectShape   = 8
eRoundedRectangular = 9
eCustomShape        = 10
```

### TShapeSubKind

```
eNoKind = 0, eOctagonalFinger = 1, eRoundedFinger = 2,
eRoundedRectangle = 3, eChamferedRectangle = 4, eDonut = 5
```

### TPadMode

```
ePadMode_Simple        = 0   // same shape all layers
ePadMode_LocalStack    = 1   // per-layer shape definitions
ePadMode_ExternalStack = 2   // template-based
```

### TExtendedDrillType

```
eDrilledHole = 0, ePunchedHole = 1,
eLaserDrilledHole = 2, ePlasmaDrilledHole = 3
```

### TExtendedHoleType

```
eRoundHole = 0, eSquareHole = 1, eSlotHole = 2
```

### TDrillLayerPairType

```
Regular = 0, MicroViaDrill = 1, Backdrill = 2, CounterHole = 3
```

---

## 6. Interface Hierarchy

All PCB primitives implement `IPCB_Primitive` as the base interface.

### IPCB_Primitive (Base)

**Source:** `Altium.Edp.Interfaces/RT_PCB/IPCB_Primitive.cs`

Common state across ALL primitives:
- Board reference, Layer (V6 and V7), ObjectID, ViewableObjectID
- Selection state, Enabled flags, Used flag, DRC error flag
- Misc flags (1-3), EnableDraw, Moveable, UserRouted, TearDrop
- Tenting (top/bottom), TestPoint (top/bottom), AssyTestPoint (top/bottom)
- IsKeepout, AllowGlobalEdit, PolygonOutline
- Container membership: InBoard, InPolygon, InComponent, InNet, InCoordinate, InDimension
- References: Net, Component, Polygon, Coordinate, Dimension
- UniqueId, Handle, Index, UnionIndex
- PowerPlane: ConnectStyle, ReliefConductorWidth, ReliefEntries, ReliefAirGap
- Mask expansions: PasteMaskExpansion, SolderMaskExpansion
- PowerPlaneClearance, PowerPlaneReliefExpansion
- BoundingRectangle (multiple variants)
- Transform: MoveByXY, MoveToXY, RotateBy, FlipXY, Mirror, SwapLayerPairs
- Serialization: Export_ToParameters
- IsSaveable(TAdvPCBFileFormatVersion)

### IPCB_Group : IPCB_Primitive

Adds: XLocation, YLocation, PrimitiveLock, LayerUsed per layer, child primitive iteration (GroupIterator), AddPCBObject/RemovePCBObject

### Primitive Type Interfaces

| Interface | Extends | Key Fields |
|---|---|---|
| `IPCB_Arc` | Primitive | CenterX/Y, Radius, LineWidth, StartAngle, EndAngle, StartX/Y, EndX/Y |
| `IPCB_Track` | Primitive | X1, Y1, X2, Y2, Width, Length |
| `IPCB_Pad` | Primitive | XLocation, YLocation, Mode, Top/Mid/Bot Shape+XSize+YSize, per-layer StackShape+Size, HoleSize, Rotation, Name, Plated, DrillType, HoleType, HoleWidth, PadOffset per layer, HoleRotation, JumperID, Cache (TV7_PadCache), TemplateLink |
| `IPCB_Via` | Primitive | Mode, XLocation, YLocation, LowLayer, HighLayer, StartLayer, StopLayer, HoleSize, Size, per-layer SizeOnLayer/StackSizeOnLayer/ShapeOnLayer, Cache, DrillLayerPairType, Height, Plated, IsBackdrill, CounterHole |
| `IPCB_Fill` | RectangularPrimitive | Width, Length, LocationX/Y (extends Rectangular: X1Location, Y1Location, X2Location, Y2Location, Rotation) |
| `IPCB_Text` | RectangularPrimitive | Size, FontID, Text, Width, Mirror, UnderlyingString, UseTTFonts, Bold, Italic, FontName, Inverted, InvertedTTTextBorder, CharSet, TextKind, BarcodeKind/RenderMode, Multiline, WordWrap, BorderSpaceType |
| `IPCB_Region` | Primitive | Kind (TRegionKind), Name, Area, CavityHeight, GeometricPolygon, MainContour, Holes |
| `IPCB_ComponentBody` | Region | (extends Region for 3D body shapes) |
| `IPCB_Component` | Group | ChannelOffset, ComponentKind, Name/Comment (IPCB_Text), Pattern, NameOn/CommentOn, LockStrings, GroupNum, Rotation, Height, NameAutoPos/CommentAutoPos, Source* fields (Designator, UniqueId, HierarchicalPath, FootprintLibrary, ComponentLibrary, LibReference, Description), DefaultPCB3DModel, IsBGA, FlippedOnLayer, Axes, Vault/Item/Revision GUIDs |
| `IPCB_Polygon` | Group | (polygon pour with child regions) |
| `IPCB_RectangularPrimitive` | Primitive | XLocation, YLocation, X1/Y1/X2/Y2 Location, Rotation |

---

## 7. Key Structs

### TV7_PadCache

**Source:** `Altium.Edp.Interfaces/RT_PCB/TV7_PadCache.cs`

```csharp
[StructLayout(LayoutKind.Sequential, Pack = 8)]
public struct TV7_PadCache
{
    public TPlaneConnectionStyle PlaneConnectionStyle;
    public int ReliefConductorWidth;
    public short ReliefEntries;
    public int ReliefAirGap;
    public int PowerPlaneReliefExpansion;
    public int PowerPlaneClearance;
    public int PasteMaskExpansion;
    public int SolderMaskExpansion;
    public int SolderMaskBottomExpansion;
    public bool UseSeparateExpansions;
    public IPCB_LayerSet Planes;
    public int ViaHeight;
    // Cache validity flags for each field:
    public TCacheState PlaneConnectionStyleValid;
    public TCacheState ReliefConductorWidthValid;
    // ... (one per field)
    public bool IsTentingTop;
    public bool IsTentingBottom;
    public TCacheState IsTentingTopValid;
    public TCacheState IsTentingBottomValid;
    public bool PasteMaskEnabled;
    public ushort InternalPlanes;
    public bool BottomPasteMaskEnabled;
}
```

### CoordPoint / CoordRect

**Source:** `Altium.SDK.Interfaces/PCB/CoordPoint.cs`, `CoordRect.cs`

All coordinates are `int` (32-bit signed integer) in internal units.

- **CoordPoint**: X, Y (int)
- **CoordRect**: Left, Bottom, Right, Top / X1, Y1, X2, Y2 / Lx, Ly, Hx, Hy / Location1, Location2, BottomLeft, TopRight

**Coordinate unit:** 1 internal unit = 1/10000 mil = 0.0001 mil = 2.54 nm. All position and size values throughout the interfaces use this unit.

---

## 8. Additional Enums

### TPlaneConnectStyle / TPlaneConnectionStyle

Two related enums for plane connections:

```
TPlaneConnectStyle:     eReliefConnectToPlane, eDirectConnectToPlane, eNoConnect
TPlaneConnectionStyle:  ePlaneNoConnect, ePlaneReliefConnect, ePlaneDirectConnect
```

### TRegionKind

```
eRegionKind_Copper = 0, eRegionKind_Cutout = 1, eRegionKind_NamedRegion = 2,
eRegionKind_BoardCutout = 3, eRegionKind_Cavity = 4
```

### TPolyHatchStyle

```
ePolyHatch90 = 0, ePolyHatch45 = 1, ePolyVHatch = 2,
ePolyHHatch = 3, ePolyNoHatch = 4, ePolySolid = 5
```

### TPolygonType

```
eSignalLayerPolygon = 0, eSplitPlanePolygon = 1, eCoverlayOutlinePolygon = 2
```

### TTextAutoposition

```
eAutoPos_Manual = 0,
eAutoPos_TopLeft = 1, eAutoPos_CenterLeft = 2, eAutoPos_BottomLeft = 3,
eAutoPos_TopCenter = 4, eAutoPos_CenterCenter = 5, eAutoPos_BottomCenter = 6,
eAutoPos_TopRight = 7, eAutoPos_CenterRight = 8, eAutoPos_BottomRight = 9
```

### TTextKind

```
eText_StrokeFont = 0, eText_TrueTypeFont = 1, eText_BarCode = 2
```

### TBarcodeKind

```
eBarcode39 = 0, eBarCode128 = 1, eBarCode_QrCode = 2, eBarCode_DataMatrix = 3
```

### TBarcodeRenderMode

```
eRender_ByMinWidth = 0, eRender_ByFullWidth = 1
```

### TComponentKind

```
eComponentKind_Standard = 0, eComponentKind_Mechanical = 1,
eComponentKind_Graphical = 2, eComponentKind_NetTie_BOM = 3,
eComponentKind_NetTie_NoBOM = 4, eComponentKind_Standard_NoBOM = 5,
eComponentKind_Jumper = 6
```

### TBoardSide

```
eBoardSide_Top = 0, eBoardSide_Bottom = 1
```

### TMirrorOperation

```
eHMirror = 0, eVMirror = 1
```

### TCacheState

```
eCacheInvalid = 0, eCacheValid = 1, eCacheManual = 2
```

### TAdvPCBFileFormatVersion

```
ePCBFileFormatNone = 0,
eAdvPCBFormat_Binary_V3 = 1, eAdvPCBFormat_Library_V3 = 2, eAdvPCBFormat_ASCII_V3 = 3,
eAdvPCBFormat_Binary_V4 = 4, eAdvPCBFormat_Library_V4 = 5, eAdvPCBFormat_ASCII_V4 = 6,
eAdvPCBFormat_Binary_V5 = 7, eAdvPCBFormat_Library_V5 = 8, eAdvPCBFormat_ASCII_V5 = 9,
eAdvPCBFormat_Binary_V6 = 10, eAdvPCBFormat_Library_V6 = 11, eAdvPCBFormat_ASCII_V6 = 12,
eAdvPCBFormat_Binary_V6_CS = 13, eAdvPCBFormat_Binary_V6_CM = 14,
eAdvPCBFormat_Binary_V6_PCBWorks = 15, eAdvPCBFormat_PadViaLibrary_V6 = 16
```

### SimpleViaType (InteractiveProperties)

```
None = 0, Via = 1, Micro = 2, InvertedMicro = 3, Skip = 4, InvertedSkip = 5
```

### LayerKind (InteractiveProperties)

```
Signal = 0, Paste = 1, Solder = 2, Hole = 3
```

---

## 9. Serialization Model

### Parameter-based serialization

The COM interface `IPCB_PrimitiveSerialize` provides:
```csharp
void ExportToParameters(IWideParameterList argParameters);
void ImportFromParameters(IWideParameterList argParameters);
```

Additionally, `IPCB_Primitive.Export_ToParameters(StringBuilder)` exports to a string parameter format. This is the `|key=value|key=value|` format seen in ASCII files and OLE storage streams.

### Binary serialization

Binary serialization is handled entirely in Delphi. There is **no BinaryReader/BinaryWriter usage in the .NET PCB assemblies**. The .NET side only sees primitives through COM interfaces after they have been deserialized by the Delphi code.

The `IPCB_Primitive_SaveLoadParameters` interface in `Altium.Edp.Interfaces/PCBInterfaces/` provides additional save/load hooks but the actual binary format parsing is in native Delphi DLLs.

---

## 10. InteractiveProperties Data Object Hierarchy

**Source:** `InteractiveProperties.Providers.PCB.DataModel/`

The property panel uses a wrapper hierarchy:

```
BasePcbDataObject (abstract)
  -> PcbPrimitiveDataObject (abstract, wraps IPCB_Primitive)
       -> PcbPrimitiveWithLocationDataObject
            -> PcbTrackDataObject
            -> PcbArcDataObject
            -> PcbFillDataObject
            -> PcbTextDataObject
            -> PcbRegionDataObject
            -> PcbSplitPlaneDataObject
            -> PcbRoomDataObject
            -> PcbPolygonPourDataObject
       -> PcbPadDataObject
       -> PcbViaDataObject
       -> PcbComponentDataObject
       -> PcbConnectionLineDataObject
       -> PcbNetDataObject
       -> PcbGroupDataObject
       -> PcbDimensionDataObject (and subtypes)
       -> PcbDrillTableDataObject
       -> Pcb3DBodyDataObject
       -> PcbStackObjectDataObject
       -> PcbAccordionDataObject
       -> PcbOleObjectDataObject
       -> PcbReuseBlockDataObject
       -> PcbViaShieldingDataObject
       -> PcbViaStitchingDataObject
       -> PcbKeepoutPrimitiveDataObject
       -> PcbBendDataObject
       -> PcbRectangleToolDataObject
  -> PcbBaseDocumentDataObject
       -> PcbBoardDataObject
       -> PcbBoard3DDataObject
       -> PcbLibraryDataObject
       -> PcbDesignViewDataObject
       -> PcbBoardRegionDataObject
  -> BasePcbInteractiveProcessDataObject
       -> BasePcbRoutingInteractiveProcessDataObject
            -> PcbLineRoutingInteractiveProcessDataObject
            -> PcbRoutingInteractiveProcessDataObject
            -> PcbDiffPairRoutingInteractiveProcessDataObject
            -> PcbMultiRouteInteractiveProcessDataObject
       -> PcbSlidingInteractiveProcessDataObject
       -> PcbViaDraggingInteractiveProcessDataObject
       -> PcbLengthTuningInteractiveProcessDataObject
       -> PcbDiffPairLengthTuningInteractiveProcessDataObject
```

### PcbDataObjectFactory

Maps `TObjectId` (+ optional offset) to data object types. The factory:
1. Gets `TObjectId` from `IPCB_Primitive.GetState_ObjectID()`
2. For certain types, adds offsets (dimensions get +3000, embedded boards +4000, etc.)
3. Looks up registered `IDataObject` type by composite ID
4. Creates instance via `Activator.CreateInstance(type, {primitive, helper})`

---

## 11. PcbDoc vs PcbLib Differences

From the factory code:
- `eBoardObject` for a library board gets offset +2000 (maps to `PcbLibraryDataObject`)
- `eBoardObject` for a 3D-routed board gets offset +6000 (maps to `PcbBoard3DDataObject`)
- Regular board gets no offset (maps to `PcbBoardDataObject`)

The `IPCB_Board2.GetState_IsLibrary()` method distinguishes library vs document boards.

For PcbLib files, footprint components are accessed through `IPCB_LibComponent` and iterated via `IPCB_LibraryIterator`.

---

## 12. Coordinate Handling

All coordinate values in the PCB model are **32-bit signed integers** in internal units:
- 1 internal unit = 1/10000 mil = 0.0001 mil = 2.54 nm
- Board origin is stored in `IPCB_Board.GetState_XOrigin()` / `GetState_YOrigin()`
- Display coordinates = internal coordinates - board origin offset
- The `PcbPrimitiveDataObject` provides `TranslateXCoordForGet/Set` and `TranslateYCoordForGet/Set` for this translation

Rotation values are `double` in degrees.

---

## 13. Key Observations for Rust Implementation

1. **Binary format is Delphi-only**: Must reverse engineer the Delphi DLLs for actual binary record layouts. The .NET side only provides the data model (field names, types, relationships).

2. **Two layer systems coexist**: V6 (byte, 0-82) for file format compatibility, V7 (extended struct) for modern features. Both must be supported.

3. **Record type ID is the first byte** of each binary record, matching `TObjectId` values 0-26.

4. **Pad complexity**: Pads have three shape/size sets (Top, Mid, Bot) in simple mode, plus per-layer stack definitions in local stack mode, plus template references in external stack mode. The `TV7_PadCache` struct caches computed values with validity flags.

5. **Polygon regions**: Polygons (`IPCB_Polygon`) are groups containing child `IPCB_Region` primitives (the pour result). The polygon definition is separate from the poured copper.

6. **Component = Group of primitives**: Components extend `IPCB_Group` and contain child pads, tracks, arcs, texts, fills, regions, 3D bodies, etc.

7. **Parameter serialization format**: The `|key=value|` format is used for both ASCII files and OLE compound document streams. Binary format is separate and denser.

8. **File format versions**: V3 through V6 with binary, library, and ASCII variants. V6 is current. CS/CM/PCBWorks are variants.
