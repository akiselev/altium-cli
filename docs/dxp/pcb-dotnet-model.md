# Altium PCB .NET Data Model Reference

Decompiled from Altium Designer 26 .NET assemblies. Primary sources:
- `Altium.SDK.Interfaces/PCB/` - COM SDK interfaces (IDispatch-based, for scripting)
- `Altium.Edp.Interfaces/PCBInterfaces/` - Modern runtime interfaces (vtable-based)
- `Altium.Edp.Interfaces/PCBDataModel.Interfaces.*` - Internal data model layer
- `Altium.Edp.Interfaces/Pcbtypes/` - Constants, enumerations, struct definitions

---

## Object Type Enumeration (TObjectId)

Two versions exist: SDK (`PCB.TObjectId`, int-based) and Pcbtypes (`Pcbtypes.TObjectId`, byte-based).
Pcbtypes is the canonical one used in the binary format:

```
Value  Name                     Description
-----  -----------------------  ---------------------------
0      eIgnoreObject            Null/sentinel
1      eArcObject               Arc primitive
2      ePadObject               Pad primitive
3      eViaObject               Via primitive
4      eTrackObject             Track (line) primitive
5      eTextObject              Text string primitive
6      eFillObject              Solid fill primitive
7      eFromToObject            FromTo (ratsnest endpoint)
8      eNetObject               Net grouping object
9      eComponentObject         Component (footprint instance)
10     ePolygonObject           Copper pour polygon
11     eRegionObject            Region (copper, cutout, cavity)
12     eComponentBodyObject     3D body attached to component
13     eDimensionObject         Dimension annotation
14     eCoordinateObject        Coordinate annotation
15     eClassObject             Object class definition
16     eRuleObject              Design rule definition
17     eManualFromToObject      Manual FromTo definition
18     eDifferentialPairObject  Differential pair definition
19     eViolationObject         DRC violation marker
20     eEmbeddedObject          Embedded object (generic)
21     eEmbeddedBoardObject     Embedded board panel
22     eSplitPlaneObject        Split plane region
23     eTraceObject             Trace (routed path group)
24     eSpareViaObject          Spare via
25     eBoardObject             Board document root
26     eBoardOutlineObject      Board outline shape
```

Constants: `MinObject = eArcObject (1)`, `MaxObject = eBoardObject (25)`.

---

## Layer System

### V6 Layer IDs (TV6_Layer enum)

Legacy layer numbering, still used in binary file format. Ordinal-based:

```
Value  Name                 Category
-----  ------------------   ------------------
0      eV6_NoLayer          -
1      eV6_TopLayer         Signal
2-31   eV6_MidLayer1..30    Signal (inner)
32     eV6_BottomLayer      Signal
33     eV6_TopOverlay        Silkscreen
34     eV6_BottomOverlay     Silkscreen
35     eV6_TopPaste          Paste mask
36     eV6_BottomPaste       Paste mask
37     eV6_TopSolder         Solder mask
38     eV6_BottomSolder      Solder mask
39-54  eV6_InternalPlane1..16  Power/ground planes
55     eV6_DrillGuide        Manufacturing
56     eV6_KeepOutLayer      Keepout
57-72  eV6_Mechanical1..16   Mechanical
73     eV6_DrillDrawing      Manufacturing
74     eV6_MultiLayer        All-layers marker
75     eV6_ConnectLayer      Ratsnest display
76     eV6_BackGroundLayer   UI
77     eV6_DRCErrorLayer     UI
78     eV6_HighlightLayer    UI
79     eV6_GridColor1        UI
80     eV6_GridColor10       UI
81     eV6_PadHoleLayer      UI
82     eV6_ViaHoleLayer      UI
```

String mappings are defined in `Pcbtypes.Consts.cLayerStrings`.

### V7 Layer IDs (IV7_Layer interface)

Modern layer system supporting unlimited layers. The V7 layer is a structured ID:

```csharp
interface IV7_Layer {
    uint   GetID();          // Full 32-bit layer ID
    ushort GetSpecies();     // Layer species (sub-type discriminator)
    byte   GetGenus();       // Layer genus (type group)
    byte   GetFamily();      // Layer family (top-level category)
    uint   GetOrd();         // Ordinal position
    ushort GetN();           // Layer number within type
    ushort GetFlags();       // Layer flags
    TV6_Layer GetDEBUGV6LAYER();  // V6 compatibility mapping
}
```

### Layer Classification (TLayerClassID)

```
eLayerClass_All            All layers
eLayerClass_Mechanical     Mechanical layers
eLayerClass_Physical       Physical layers (signal + plane + dielectric)
eLayerClass_Electrical     Electrical layers (signal + plane)
eLayerClass_Dielectric     Dielectric layers only
eLayerClass_Signal         Signal copper layers only
eLayerClass_InternalPlane  Internal plane layers only
eLayerClass_SolderMask     Solder mask layers
eLayerClass_Overlay        Silkscreen overlay layers
eLayerClass_PasteMask      Paste mask layers
```

### Layer Stack Structure

**IPCB_LayerStackBase** (base for all stacks):
- `GetState_Name() -> string` / `SetState_Name(string)`
- `ID() -> string` - Stack identifier
- `StateID() -> int`
- `Count()` / `Count(type)` / `Count(type, start, stop)` - Layer counts
- `Iterator()` / `Iterator(type)` / `Iterator(type, start, stop)` - Layer iterators
- `First(type)` / `Last(type)` / `Next(type, ref)` / `Previous(type, ref)` - Navigation
- `GetState_IsFlex() -> bool` / `SetState_IsFlex(bool)` - Flex PCB flag
- `Get_ZTop(layer)` / `Get_ZBottom(layer)` - Z-axis positions

**IPCB_LayerStack_V7** (v7 layer stack):
- `GetState_Board()` - Parent board
- `GetState_LayerObject(int)` / `GetState_LayerObject_V7(IV7_Layer)` - Get layer objects
- `GetState_LayerStackStyle() -> TLayerStackStyle`
- `GetState_DielectricTop()` / `GetState_DielectricBottom()` - Dielectric boundaries
- `ShowTopDielectric` / `ShowBotDielectric` - Display flags
- `RemoveFromStack(layer)` / `InsertInStackBelow(ref, layer)` / `InsertInStackAbove(ref, layer)`
- `FirstLayer()` / `NextLayer(layer)` / `PreviousLayer(layer)` / `LastLayer()` - Traversal
- `InsertLayer(int)` / `FirstAvailableSignalLayer()` / `FirstAvailableInternalPlane()`
- `SignalLayerCount()` / `LayersInStackCount()`

**IPCB_MasterLayerStack** (extends LayerStackBase):
- `GetState_Substacks(int)` - Get sub-stacks by index
- `SubstackCount()` - Number of sub-stacks
- `CreateLayer(IV7_Layer)` / `RemoveLayer(layer)` - Layer CRUD
- `InsertOnTop(layer)` / `InsertOnBottom(layer)` / `InsertBelow(ref, layer)` / `InsertAbove(ref, layer)`
- `DisableLayer(substack, layer)` / `EnableLayer(substack, layer)`
- `CreateSubstack()` / `RemoveSubstack(substack)` / `GetSubstack(id)`
- `Import_FromParameters(string)` / `Export_ToParameters(ref string)` - Serialization

**TLayerStackStyle**:
```
eLayerStack_Pairs          Layer pairs (traditional)
eLayerStacks_InsidePairs   Inside pairs
eLayerStackBuildup         Buildup stack
eLayerStackCustom          Custom definition
```

### Layer Object (IPCB_LayerObject / IPCB_LayerObject_V7)

```
GetState_LayerName() -> string     Layer display name
GetState_UsedByPrims() -> bool     Has primitives on this layer
IsInLayerStack() -> bool           Part of the active stack
V7_LayerID() -> IV7_Layer          V7 layer identifier
V6_LayerID() -> int                V6 compatibility ID
LayerStack() -> object             Parent layer stack
GetState_LayerDisplayName(int)     Format-dependent display name
```

IPCB_LayerObject_V7 adds:
```
Dielectric() -> object             Associated dielectric layer
GetState_IsDisplayed(board) -> bool
SetState_IsDisplayed(board, bool)
LayerId() -> int                   Numeric layer ID
```

---

## IPCB_Primitive (Base for All PCB Objects)

Every PCB object implements this interface. All coordinates are in internal units (1 unit = 0.1 mil = 10nm, i.e. `kInternalUnits = 10000` per mil).

### Identity and Hierarchy
```
GetState_Board() -> IPCB_Board     Parent board document
GetState_ObjectID() -> int         TObjectId value
GetState_Layer() / SetState_Layer(int)        V6 layer ID
GetState_V7Layer() / SetState_V7Layer(IV7_Layer)  V7 layer ID
GetState_ObjectIDString() -> string            e.g. "Arc", "Pad"
GetState_Identifier() -> string                Human-readable identifier
GetState_DescriptorString() -> string          Description for UI
GetState_DetailString() -> string              Detailed info string
GetState_UniqueId() / SetState_UniqueID(string)   Persistent unique ID
GetState_Handle() -> string                    Runtime handle
GetState_Index() / SetState_Index(ushort)      Index in parent collection
GetState_UnionIndex() / SetState_UnionIndex(int)  Union membership
```

### Selection and Visibility Flags
```
GetState_Selected() / SetState_Selected(bool)
GetState_IsPreRoute() / SetState_IsPreRoute(bool)
GetState_InSelectionMemory(int) / SetState_InSelectionMemory(int, bool)
GetState_PadCacheRobotFlag() / SetState_PadCacheRobotFlag(bool)
GetState_Enabled() / SetState_Enabled(bool)                    Master enable
GetState_Enabled_Direct() / SetState_Enabled_Direct(bool)      Direct enable
GetState_Enabled_vNet() / SetState_Enabled_vNet(bool)          Net filter enable
GetState_Enabled_vPolygon() / SetState_Enabled_vPolygon(bool)  Polygon filter
GetState_Enabled_vComponent() / SetState_Enabled_vComponent(bool) Component filter
GetState_Enabled_vCoordinate() / SetState_Enabled_vCoordinate(bool)
GetState_Enabled_vDimension() / SetState_Enabled_vDimension(bool)
GetState_EnableDraw() / SetState_EnableDraw(bool)
GetState_DrawAsPreview() / SetState_DrawAsPreview(bool)
```

### State Flags
```
GetState_Used() / SetState_Used(bool)
GetState_DRCError() / SetState_DRCError(bool)
GetState_MiscFlag1() / SetState_MiscFlag1(bool)   General purpose
GetState_MiscFlag2() / SetState_MiscFlag2(bool)
GetState_MiscFlag3() / SetState_MiscFlag3(bool)
GetState_Moveable() / SetState_Moveable(bool)
GetState_UserRouted() / SetState_UserRouted(bool)
GetState_TearDrop() / SetState_TearDrop(bool)
GetState_AllowGlobalEdit() / SetState_AllowGlobalEdit(bool)
```

### Keepout and Test Points
```
GetState_IsKeepout() / SetState_IsKeepout(bool)
GetState_IsTenting() / SetState_IsTenting(bool)           Legacy tenting
GetState_IsTenting_Top() / SetState_IsTenting_Top(bool)   Top tenting
GetState_IsTenting_Bottom() / SetState_IsTenting_Bottom(bool)
GetState_IsTestPoint_Top() / SetState_IsTestPoint_Top(bool)
GetState_IsTestPoint_Bottom() / SetState_IsTestPoint_Bottom(bool)
GetState_IsAssyTestPoint_Top() / SetState_IsAssyTestPoint_Top(bool)
GetState_IsAssyTestPoint_Bottom() / SetState_IsAssyTestPoint_Bottom(bool)
```

### Membership Flags
```
GetState_PolygonOutline() / SetState_PolygonOutline(bool)
GetState_InBoard() / SetState_InBoard(bool)
GetState_InPolygon() / SetState_InPolygon(bool)
GetState_InComponent() / SetState_InComponent(bool)
GetState_InNet() / SetState_InNet(bool)
GetState_InCoordinate() / SetState_InCoordinate(bool)
GetState_InDimension() / SetState_InDimension(bool)
GetState_IsElectricalPrim() -> bool     (read-only)
```

### Electrical Properties (from Pad Cache)
```
GetState_PowerPlaneConnectStyle() -> TPlaneConnectStyle
GetState_ReliefConductorWidth() -> int
GetState_ReliefEntries() -> int
GetState_ReliefAirGap() -> int
GetState_PasteMaskExpansion() -> int
GetState_SolderMaskExpansion() -> int
GetState_PowerPlaneClearance() -> int
GetState_PowerPlaneReliefExpansion() -> int
```

### Parent References
```
GetState_Net() / SetState_Net(object)           Parent net
GetState_Component() / SetState_Component(object)  Parent component
GetState_Polygon() / SetState_Polygon(object)    Parent polygon
GetState_Coordinate() / SetState_Coordinate(object)
GetState_Dimension() / SetState_Dimension(object)
GetState_ViewableObjectID() -> int
```

### Geometry
```
BoundingRectangle() -> ICoordRect
BoundingRectangleForSelection() -> ICoordRect
BoundingRectangleForPainting() -> ICoordRect
BoundingRectangleChildren() -> ICoordRect
IsHidden() -> bool
IsFreePrimitive() -> bool     Not in any group
IsSaveable(int version) -> bool
```

### Modification
```
MoveByXY(int X, int Y)
MoveToXY(int X, int Y)
RotateBy(double angle)
FlipXY(int axis, int mirrorOp)
SwapLayerPairs()
GraphicallyInvalidate()
BeginModify() / EndModify() / CancelModify()
Export_ToParameters(ref string params)    Serialize to parameter string
RequiredParamterSpace() -> int
Replicate() -> IPCB_Primitive             Clone
SetState_Preview(bool enable, uint color, float opacity, int z, int height, float sx, float sy, float sz)
```

### IPCB_Primitive2 (Extended Properties)

```
GetState_PasteMaskExpansion() / SetState_PasteMaskExpansion(int)
GetState_PasteMaskExpansionMode() / SetState_PasteMaskExpansionMode(TMaskExpansionMode)
GetState_SolderMaskExpansion() / SetState_SolderMaskExpansion(int)
GetState_SolderMaskExpansionMode() / SetState_SolderMaskExpansionMode(TMaskExpansionMode)
MaskExpansion(IV7_Layer maskLayer) -> int
HasMaskExpansion(IV7_Layer maskLayer) -> bool
GetState_RoutingMinWidth() -> int
GetState_RoutingViaWidth() -> int
GetState_PasteMaskEnabled() / SetState_PasteMaskEnabled(bool)
GetState_IsEmbeddedComponentCavity() -> bool
GetState_PasteMaskUsePercent() / SetState_PasteMaskUsePercent(bool)
GetState_PasteMaskPercent() / SetState_PasteMaskPercent(double)
GetGUID() -> string
GetState_SharedUnion() / SetState_SharedUnion(object)
CreateSharedUnion()
```

**TMaskExpansionMode**:
```
eMaskExpansionMode_NoMask   No mask opening
eMaskExpansionMode_Rule     Use design rule
eMaskExpansionMode_Manual   Manual override value
```

### IPCB_Primitive3 (Runtime Extended, from PCBInterfaces)

Adds typed return values (not object), plus:
```
IsLogicalComponentAssigned() -> bool
LogicalComponent_SourceDesignator() -> string
IsLogicalNetAssigned() -> bool
LogicalNet_Name() -> string
Import_FromUser(graphicalInterface) -> TChangeScope
GetState_BoundingRectangle_ForJumping(ref LowX, LowY, HighX, HighY)
IsSmartUnionObject() -> bool
SetState_XSizeYSize() -> bool
Violations_IsAssigned() -> bool
Violations_Count() -> int
Violations_At(int) -> IPCB_Violation
RuleKindValidForPrimitive(TRuleKind) -> bool
Replicate_FullCopy() -> IPCB_Primitive
PrimitivesTouch(IPCB_Primitive) -> bool
IsPrevAssigned() / IsNextAssigned() -> bool
LastState() -> IPCB_Primitive3
GetState_vIndex() / SetState_vIndex(ushort)
GetLogicalNet() -> IPCB_Group
ViolationCount() -> int
GetState_X() / GetState_Y() -> int
GetLocalUniqueID() -> uint
HasOwnPropertiesDialog() -> bool
GetCreationTimeStamp() / GetTouchTimeStamp() / GetDeletionTimeStamp() -> uint
CopyTo(IPCB_Primitive dest, TCopyMode)
ClearLastState()
GetDMPrimitive() -> IPCBDM_Primitive
ResetUniqueIDs()
```

---

## IPCB_Group (Base for Compound Objects)

Extends IPCB_Primitive. Used by Component, Net, Polygon, Dimension, Coordinate, SplitPlane.

```
GetState_XLocation() / SetState_XLocation(int)
GetState_YLocation() / SetState_YLocation(int)
GetState_PrimitiveLock() / SetState_PrimitiveLock(bool)
GetState_LayerUsed(int layer) / SetState_LayerUsed(int layer, bool)
FreePrimitives()                              Release all children
GetPrimitiveAt(int index, int objectId)       Get child by index and type
GetPrimitiveCount(ITransportSet objectSet)    Count children by type
SetState_XSizeYSize() / FastSetState_XSizeYSize()   Recalculate bounds
SetState_LayersUsedArray()                    Update layer usage cache
GroupIterator_Create() -> IPCB_GroupIterator  Create child iterator
GroupIterator_Destroy(ref iterator)
AddPCBObject(object pcbObject)               Add child primitive
RemovePCBObject(object pcbObject)            Remove child primitive
ReplicateWithChildren() -> object            Deep clone
```

---

## IPCB_RectangularPrimitive

Extends IPCB_Primitive. Base for Text, Fill, EmbeddedBoard.

```
GetState_XLocation() / SetState_XLocation(int)    Center X
GetState_YLocation() / SetState_YLocation(int)    Center Y
GetState_X1Location() / SetState_X1Location(int)  Corner 1 X
GetState_Y1Location() / SetState_Y1Location(int)  Corner 1 Y
GetState_X2Location() / SetState_X2Location(int)  Corner 2 X
GetState_Y2Location() / SetState_Y2Location(int)  Corner 2 Y
GetState_Rotation() / SetState_Rotation(double)
RotateAroundXY(int X, int Y, double angle)
IsRedundant() -> bool
SetState_XSizeYSize() -> bool
```

---

## Primitive Type Interfaces

### IPCB_Arc (eArcObject = 1)

Extends IPCB_Primitive.

```
GetState_CenterX() / SetState_CenterX(int)
GetState_CenterY() / SetState_CenterY(int)
GetState_Radius() / SetState_Radius(int)
GetState_LineWidth() / SetState_LineWidth(int)
GetState_StartAngle() / SetState_StartAngle(double)    Degrees
GetState_EndAngle() / SetState_EndAngle(double)         Degrees
GetState_StartX() / GetState_StartY() -> int            Computed start point
GetState_EndX() / GetState_EndY() -> int                Computed end point
RotateAroundXY(int X, int Y, double angle)
GetState_StrictHitTest(int hitX, int hitY) -> bool
```

### IPCB_Pad (ePadObject = 2)

Extends IPCB_Primitive. Complex padstack support.

**Location and identity:**
```
GetState_XLocation() / SetState_XLocation(int)
GetState_YLocation() / SetState_YLocation(int)
GetState_Rotation() / SetState_Rotation(double)
GetState_Name() / SetState_Name(string)        Pad designator (e.g. "1", "A1")
GetState_PinDescriptorString() -> string       Full pin descriptor
```

**Padstack mode:**
```
GetState_Mode() / SetState_Mode(TPadMode)
```

TPadMode:
```
ePadMode_Simple          Same shape all layers
ePadMode_LocalStack      Per-layer shapes defined locally
ePadMode_ExternalStack   Shapes from external padstack library
```

**Shape per layer (simple mode - top/mid/bot):**
```
GetState_TopXSize() / SetState_TopXSize(int)
GetState_TopYSize() / SetState_TopYSize(int)
GetState_TopShape() / SetState_TopShape(TShape)
GetState_MidXSize() / SetState_MidXSize(int)
GetState_MidYSize() / SetState_MidYSize(int)
GetState_MidShape() / SetState_MidShape(TShape)
GetState_BotXSize() / SetState_BotXSize(int)
GetState_BotYSize() / SetState_BotYSize(int)
GetState_BotShape() / SetState_BotShape(TShape)
```

**Per-layer access (local stack mode):**
```
GetState_XSizeOnLayer(IV7_Layer) / GetState_YSizeOnLayer(IV7_Layer)  Computed
GetState_ShapeOnLayer(IV7_Layer) -> TShape                           Computed
GetState_XStackSizeOnLayer(IV7_Layer) / SetState_XStackSizeOnLayer(IV7_Layer, int)
GetState_YStackSizeOnLayer(IV7_Layer) / SetState_YStackSizeOnLayer(IV7_Layer, int)
GetState_StackShapeOnLayer(IV7_Layer) / SetState_StackShapeOnLayer(IV7_Layer, TShape)
GetState_XPadOffsetOnLayer(int layer) / SetState_XPadOffsetOnLayer(int, int)
GetState_YPadOffsetOnLayer(int layer) / SetState_YPadOffsetOnLayer(int, int)
GetState_WidthOnLayer(int layer) -> int
ClearStackSizeAndShapes()
DefinitionLayerIterator() -> object
```

**Hole properties:**
```
GetState_HoleSize() / SetState_HoleSize(int)       Hole diameter
GetState_HoleWidth() / SetState_HoleWidth(int)     Slot width (for slot holes)
GetState_HoleRotation() / SetState_HoleRotation(double)
GetState_DrillType() / SetState_DrillType(TExtendedDrillType)
GetState_HoleType() / SetState_HoleType(TExtendedHoleType)
GetState_Plated() / SetState_Plated(bool)
GetState_HolePositiveTolerance() / SetState_HolePositiveTolerance(int)
GetState_HoleNegativeTolerance() / SetState_HoleNegativeTolerance(int)
GetState_SolderMaskExpansionFromHoleEdge() / SetState_SolderMaskExpansionFromHoleEdge(bool)
```

**Pad cache and electrical:**
```
GetState_Cache() / SetState_Cache(IV7_PadCache)
GetState_IsConnectedToPlane(int layer) / SetState_IsConnectedToPlane(int, bool)
PlaneConnectionStyleForLayer(int layer) -> TPlaneConnectionStyle
```

**Swap and part info:**
```
GetState_SwapID_Pad() / SetState_SwapID_Pad(string)
GetState_SwapID_Part() / SetState_SwapID_Part(string)
GetState_SwappedPadName() / SetState_SwappedPadName(string)
GetState_OwnerPart_ID() / SetState_OwnerPart_ID(int)
GetState_JumperID() / SetState_JumperID(int)
```

**Queries:**
```
IsPadStack() -> bool
IsSurfaceMount() -> bool
IsVirtualPin() -> bool
InvalidateSizeShape() / ValidateSizeShape() / ReValidateSizeShape()
UpdateCache() / InvalidateCache()
RotateAroundXY(int X, int Y, double angle)
BoundingRectangleOnLayer(int layer) -> ICoordRect
GetState_TemplateLink() -> object    Linked pad/via template
```

**IPCB_Pad2** (adds corner radius):
```
GetState_CornerRadiusOnLayer(IV7_Layer) -> int
GetState_CRPercentageOnLayer(IV7_Layer) -> byte
GetState_StackCRPctOnLayer(IV7_Layer) / SetState_StackCRPctOnLayer(IV7_Layer, byte)
```

**IPCB_Pad3** (adds counter holes):
```
GetProperty_CounterHoles() / SetProperty_CounterHoles(object)
IsCounterHole() -> bool
GetCounterHoleDiameters() -> object
```

**IPCB_Pad4** exists but was not explored in detail.

### IPCB_Via (eViaObject = 3)

Extends IPCB_Primitive.

```
GetState_Mode() / SetState_Mode(TPadMode)           Padstack mode
GetState_XLocation() / SetState_XLocation(int)
GetState_YLocation() / SetState_YLocation(int)
GetState_Size() / SetState_Size(int)                 Via diameter (simple)
GetState_HoleSize() / SetState_HoleSize(int)         Drill hole diameter
GetState_Height() -> int                              Via height (computed)
GetState_LowLayer() / SetState_LowLayer(IV7_Layer)   Start layer
GetState_HighLayer() / SetState_HighLayer(IV7_Layer)  Stop layer
GetState_StartLayer() / GetState_StopLayer() -> object
GetState_SizeOnLayer(IV7_Layer) -> int                Per-layer diameter
GetState_StackSizeOnLayer(IV7_Layer) / SetState_StackSizeOnLayer(IV7_Layer, int)
GetState_ShapeOnLayer(IV7_Layer) -> TShape
GetState_Cache() / SetState_Cache(IV7_PadCache)
GetState_IsConnectedToPlane(int) / SetState_IsConnectedToPlane(int, bool)
PlaneConnectionStyleForLayer(int) -> TPlaneConnectionStyle
GetState_SolderMaskExpansionFromHoleEdge() / SetState_SolderMaskExpansionFromHoleEdge(bool)
GetState_HolePositiveTolerance() / SetState_HolePositiveTolerance(int)
GetState_HoleNegativeTolerance() / SetState_HoleNegativeTolerance(int)
GetState_TemplateLink() -> object
RotateAroundXY(int X, int Y, double angle)
IntersectLayer(IV7_Layer) -> bool
DefinitionLayerIterator() -> object
ClearStackSizes()
IsCounterHole() -> bool                   (IPCB_Via only, added late)
GetCounterHole_Params() -> object
```

**TViaType**:
```
InvalidVia
Thru            Through-hole via
Blind           Blind via
Buried          Buried via
BackdrillHole   Backdrill hole
MicroVia        Microvia (laser drilled)
SkipVia         Skip via
```

### IPCB_Track (eTrackObject = 4)

Extends IPCB_Primitive. The simplest primitive.

```
GetState_X1() / SetState_X1(int)    Start X
GetState_Y1() / SetState_Y1(int)    Start Y
GetState_X2() / SetState_X2(int)    End X
GetState_Y2() / SetState_Y2(int)    End Y
GetState_Width() / SetState_Width(int)  Track width
```

### IPCB_Text (eTextObject = 5)

Extends IPCB_RectangularPrimitive.

**Core text properties:**
```
GetState_Text() / SetState_Text(string)                    Display text (resolved)
GetState_UnderlyingString() / SetState_UnderlyingString(string)  Raw string (may contain .SpecialString)
GetState_ConvertedString() -> string                        Fully converted display text
GetState_Size() / SetState_Size(int)                        Text height
GetState_Width() / SetState_Width(int)                      Stroke width
GetState_Mirror() / SetState_Mirror(bool)
GetState_FontID() / SetState_FontID(short)                  Stroke font index
```

**TrueType font properties:**
```
GetState_UseTTFonts() / SetState_UseTTFonts(bool)
GetState_Bold() / SetState_Bold(bool)
GetState_Italic() / SetState_Italic(bool)
GetState_FontName() / SetState_FontName(string)
GetState_CharSet() / SetState_CharSet(byte)
GetState_TTFTextWidth() / SetState_TTFTextWidth(int)
GetState_TTFTextHeight() / SetState_TTFTextHeight(int)
```

**Inverted text (text in copper cutout):**
```
GetState_Inverted() / SetState_Inverted(bool)
GetState_InvertedTTTextBorder() / SetState_InvertedTTTextBorder(int)
GetState_InvRectWidth() / SetState_InvRectWidth(int)
GetState_InvRectHeight() / SetState_InvRectHeight(int)
GetState_UseInvertedRectangle() / SetState_UseInvertedRectangle(bool)
GetState_TTFInvertedTextJustify() / SetState_TTFInvertedTextJustify(int)
GetState_TTFOffsetFromInvertedRect() / SetState_TTFOffsetFromInvertedRect(int)
```

**Barcode support:**
```
GetState_BarCodeKind() / SetState_BarCodeKind(TBarcodeKind)
GetState_BarCodeRenderMode() / SetState_BarCodeRenderMode(TBarcodeRenderMode)
GetState_BarCodeFullWidth() / SetState_BarCodeFullWidth(int)
GetState_BarCodeFullHeight() / SetState_BarCodeFullHeight(int)
GetState_BarCodeXMargin() / SetState_BarCodeXMargin(int)
GetState_BarCodeYMargin() / SetState_BarCodeYMargin(int)
GetState_BarCodeMinWidth() / SetState_BarCodeMinWidth(int)
GetState_BarCodeInverted() / SetState_BarCodeInverted(bool)
GetState_BarCodeFontName() / SetState_BarCodeFontName(string)
GetState_BarCodeBitPattern() -> string
GetState_BarCodeShowText() / SetState_BarCodeShowText(bool)
```

**Text kind:**
```
GetState_TextKind() / SetState_TextKind(TTextKind)
```

TTextKind:
```
eText_StrokeFont     Vector stroke font
eText_TrueTypeFont   TrueType font
eText_BarCode        Barcode
```

**Multiline text:**
```
GetState_Multiline() / SetState_Multiline(bool)
GetState_WordWrap() -> bool
GetState_MultilineTextWidth() / SetState_MultilineTextWidth(int)
GetState_MultilineTextHeight() / SetState_MultilineTextHeight(int)
GetState_MultilineTextResizeEnabled() -> bool
GetState_MultilineTextAutoPosition() / SetState_MultilineTextAutoPosition(TTextAutoposition)
GetState_BorderSpaceType() / SetState_BorderSpaceType(int)
CanEditMultilineRectSize() -> bool
```

**Queries:**
```
IsHidden_1() -> bool
IsDesignator() -> bool       Is this the component designator text
IsComment() -> bool          Is this the component comment text
InAutoDimension() -> bool
GetDesignatorDisplayString() -> string
RotationHandle() -> ICoordPoint
GetTrueTypeTextOutlineGeometricPolygon() -> object
ConvertToStrokeArray() -> object
```

**TTextAutoposition**:
```
eAutoPos_Manual         No auto-position
eAutoPos_TopLeft        Left-Above
eAutoPos_CenterLeft     Left-Center
eAutoPos_BottomLeft     Left-Below
eAutoPos_TopCenter      Center-Above
eAutoPos_CenterCenter   Center
eAutoPos_BottomCenter   Center-Below
eAutoPos_TopRight       Right-Above
eAutoPos_CenterRight    Right-Center
eAutoPos_BottomRight    Right-Below
```

### IPCB_Fill (eFillObject = 6)

Extends IPCB_RectangularPrimitive.

```
GetState_Width() / SetState_Width(int)         Rectangle width
GetState_Length() / SetState_Length(int)        Rectangle length
GetState_LocationX() / SetState_LocationX(int) Center X
GetState_LocationY() / SetState_LocationY(int) Center Y
```

Note: X1/Y1/X2/Y2/Rotation are inherited from IPCB_RectangularPrimitive.

### IPCB_Connection (eConnectionObject = 7 equivalent)

Extends IPCB_Primitive. Ratsnest lines.

```
GetState_X1() / SetState_X1(int)
GetState_Y1() / SetState_Y1(int)
GetState_X2() / SetState_X2(int)
GetState_Y2() / SetState_Y2(int)
GetState_Layer1() / SetState_Layer1(int)    V6 layer of pad 1
GetState_Layer2() / SetState_Layer2(int)    V6 layer of pad 2
GetState_Mode() / SetState_Mode(int)        Connection mode (TConnectionMode)
IsRedundant() -> bool
RotateAroundXY(int X, int Y, double angle)
```

### IPCB_Net (eNetObject = 8)

Extends IPCB_Group.

```
GetState_Color() / SetState_Color(uint)
GetState_Name() / SetState_Name(string)
GetState_ConnectsVisible() / SetState_ConnectsVisible(bool)
GetState_ConnectivelyInvalid() -> bool
GetState_RoutedLength() -> int                Total routed trace length
GetState_ViaCount() -> int
GetState_PinCount() -> int
Getstate_PadByName(string) -> object          Find pad by name in net
Getstate_PadByPinDescription(string) -> object
GetState_IsHighlighted() / SetState_IsHighlighted(bool)
GetState_LoopRemoval() / SetState_LoopRemoval(bool)
GetState_InDifferentialPair() -> bool
GetState_LiveHighlightMode() / SetState_LiveHighlightMode(int)
GetState_OverrideColorForDraw() / SetState_OverrideColorForDraw(bool)
GetState_JumpersVisible() / SetState_JumpersVisible(bool)
Rebuild()
HideNetConnects() / ShowNetConnects()
ConnectivelyInValidate()
CancelGroupWarehouseRegistration(object pad)
RegisterWithGroupWarehouse(object pad)
GetLogicalNet() -> object
SubnetIndices_Set() / SubnetIndices_Reset()
GetSubnets() -> object
```

### IPCB_Component (eComponentObject = 9)

Extends IPCB_Group and IPCB_Primitive.

**Identity:**
```
GetState_Pattern() / SetState_Pattern(string)          Footprint name
GetState_Name() -> object                               Designator text object (IPCB_Text)
GetState_Comment() -> object                            Comment text object (IPCB_Text)
GetState_ComponentKind() / SetState_ComponentKind(int)  TComponentKind
GetState_ChannelOffset() / SetState_ChannelOffset(int)
GetState_GroupNum() / SetState_GroupNum(int)
```

**Geometry:**
```
GetState_Rotation() / SetState_Rotation(double)        Rotation in degrees
GetState_Height() / SetState_Height(int)               Component height
GetState_FlippedOnLayer() / SetState_FlippedOnLayer(bool)  Bottom side placement
BoundingRectangleNoNameComment() -> ICoordRect
BoundingRectangleNoNameCommentForSignals() -> ICoordRect
RotateAroundXY(int X, int Y, double angle)
FlipComponent()
Rebuild()
```

**Text auto-positioning:**
```
GetState_NameOn() / SetState_NameOn(bool)
GetState_CommentOn() / SetState_CommentOn(bool)
GetState_LockStrings() / SetState_LockStrings(bool)
GetState_NameAutoPos() / SetState_NameAutoPos(TTextAutoposition)
GetState_CommentAutoPos() / SetState_CommentAutoPos(TTextAutoposition)
ChangeNameAutoposition(int) -> bool
ChangeCommentAutoposition(int) -> bool
AutoPosition_NameComment()
AutoPosition_NameComment_APILike()
ChangeAutopositionNameComment()
```

**Source links (from schematic):**
```
GetState_SourceDesignator() / SetState_SourceDesignator(string)
GetState_SourceUniqueId() / SetState_SourceUniqueId(string)
GetState_SourceHierarchicalPath() / SetState_SourceHierarchicalPath(string)
GetState_SourceFootprintLibrary() / SetState_SourceFootprintLibrary(string)
GetState_SourceComponentLibrary() / SetState_SourceComponentLibrary(string)
GetState_SourceLibReference() / SetState_SourceLibReference(string)
GetState_SourceDescription() / SetState_SourceDescription(string)
GetState_SourceCompDesignItemID() / SetState_SourceCompDesignItemID(string)
GetState_FootprintDescription() / SetState_FootprintDescription(string)
GetState_DefaultPCB3DModel() / SetState_DefaultPCB3DModel(string)
```

**Vault/workspace:**
```
GetState_VaultGUID() / SetState_VaultGUID(string)
GetState_ItemGUID() / SetState_ItemGUID(string)
GetState_ItemRevisionGUID() / SetState_ItemRevisionGUID(string)
```

**Pin swapping:**
```
GetState_EnablePinSwapping() / SetState_EnablePinSwapping(bool)
GetState_EnablePartSwapping() / SetState_EnablePartSwapping(bool)
```

**Configurable footprint:**
```
IsConfigurableFootprint() -> bool
GetState_FootprintConfiguratorName() / SetState_FootprintConfiguratorName(string)
GetState_FootprintConfigurableParameters_Encoded() / SetState_FootprintConfigurableParameters_Encoded(string)
```

**FPGA:**
```
GetState_FPGADisplayMode() / SetState_FPGADisplayMode(int)
```

**Axes (flex PCB):**
```
GetState_Axis(int i) -> object
GetState_AxisCount() -> int
AddAxis() -> object
ClearAxes()
ResetDisplacement()
```

**Other:**
```
GetState_IsBGA() -> bool
Getstate_PadByName(string) -> object
LoadCompFromLibrary() -> bool
LoadFromLibrary(string params) -> bool
SetState_XSizeYSize_1() -> bool
GetState_JumpersVisible() / SetState_JumpersVisible(bool)
SaveModelToFile(string fullPath) -> bool
SaveModelToFileAsPart(string fullPath) -> bool
GetState_ModelHash() -> string
GetState_PackageSpecificHash() -> string
IsFitted() -> bool
IsPackageEqual(object comp) -> bool
IsPackageEqualEx(object comp, ITransportSet options) -> bool
```

### IPCB_Polygon (ePolygonObject = 10)

Extends IPCB_Group.

**Shape definition:**
```
GetState_PointCount() / SetState_PointCount(int)
GetState_Segments(int i) / SetState_Segments(int i, IPolySegment)
GetState_AreaSize() / SetState_AreaSize(double)
GetState_Name() / SetState_Name(string)
GetState_AutoGenerateName() / SetState_AutoGenerateName(bool)
GetDefaultName() -> string
```

**Pour settings:**
```
GetState_PolygonType() / SetState_PolygonType(TPolygonType)
GetState_PolyHatchStyle() / SetState_PolyHatchStyle(TPolyHatchStyle)
GetState_PourOver() / SetState_PourOver(TPolygonPourOver)
GetState_Grid() / SetState_Grid(int)           Pour grid
GetState_TrackSize() / SetState_TrackSize(int) Hatch line width
GetState_MinTrack() / SetState_MinTrack(int)
GetState_BorderWidth() / SetState_BorderWidth(int)
GetState_ArcApproximation() / SetState_ArcApproximation(int)
GetState_PourIndex() / SetState_PourIndex(int)  Pour order priority
GetState_ArcPourMode() / SetState_ArcPourMode(bool)
```

**Pour filtering:**
```
GetState_RemoveDead() / SetState_RemoveDead(bool)
GetState_UseOctagons() / SetState_UseOctagons(bool)
GetState_AvoidObsticles() / SetState_AvoidObsticles(bool)
GetState_ExpandOutline() / SetState_ExpandOutline(bool)
GetState_RemoveIslandsByArea() / SetState_RemoveIslandsByArea(bool)
GetState_IslandAreaThreshold() / SetState_IslandAreaThreshold(double)
GetState_RemoveNarrowNecks() / SetState_RemoveNarrowNecks(bool)
GetState_NeckWidthThreshold() / SetState_NeckWidthThreshold(int)
GetState_ClipAcuteCorners() / SetState_ClipAcuteCorners(bool)
GetState_MitreCorners() / SetState_MitreCorners(bool)
GetState_IgnoreViolations() / SetState_IgnoreViolations(bool)
```

**Display settings:**
```
GetState_DrawRemovedNecks() / SetState_DrawRemovedNecks(bool)
GetState_DrawRemovedIslands() / SetState_DrawRemovedIslands(bool)
GetState_DrawDeadCopper() / SetState_DrawDeadCopper(bool)
```

**Pour state:**
```
GetState_Poured() / SetState_Poured(bool)
GetState_CopperPourInvalid() -> bool
GetState_InRepour() -> bool
SetState_CopperPourInvalid() / SetState_CopperPourValid()
CopperPourValidate()
Rebuild()
```

**Hit testing:**
```
GetState_HitPrimitive(object) -> bool
PrimitiveInsidePoly(object) -> bool
AcceptsLayer(int layer) -> bool
PointInPolygon(int hitX, int hitY) -> bool
GetState_StrictHitTest(int hitX, int hitY) -> bool
GrowPolyshape(int dist)
xBoundingRectangle() -> ICoordRect
```

**TPolygonType**:
```
eSignalLayerPolygon      Standard copper pour
eSplitPlanePolygon       Split plane polygon
eCoverlayOutlinePolygon  Coverlay outline
```

**TPolyHatchStyle**:
```
ePolyHatch90    90-degree crosshatch
ePolyHatch45    45-degree crosshatch
ePolyVHatch     Vertical hatch
ePolyHHatch     Horizontal hatch
ePolyNoHatch    No hatch (outline only)
ePolySolid      Solid fill
```

### IPCB_Region (eRegionObject = 11)

Extends IPCB_Primitive.

```
GetState_Kind() / SetState_Kind(TRegionKind)
GetState_Name() / SetState_Name(string)
GetGeometricPolygon() -> object              Full polygon geometry
GetMainContour() -> object                   Outer boundary contour
GetHoleCount() -> int                        Number of holes
GetHole(int i) -> object                     Hole contour by index
SetOutlineContour(object contour)            Set outer boundary
SetGeometricPolygon(object polygon)          Set full geometry
GetState_Area() -> long                      Area in square internal units
GetState_CavityHeight() / SetState_CavityHeight(int)
IsSimpleRegion() -> bool
RotateAroundXY(int X, int Y, double angle)
```

**TRegionKind**:
```
eRegionKind_Copper        Copper region
eRegionKind_Cutout        Board/polygon cutout
eRegionKind_NamedRegion   Named region
eRegionKind_BoardCutout   Board cutout
eRegionKind_Cavity        Embedded component cavity
```

### IPCB_ComponentBody (eComponentBodyObject = 12)

Extends IPCB_Region.

**3D properties:**
```
GetStandoffHeight() / SetStandoffHeight(int)    Distance above board
GetOverallHeight() / SetOverallHeight(int)      Total height including standoff
GetBodyProjection() / SetBodyProjection(int)    Projection type (T3DPrimitiveKind)
GetBodyColor3D() / SetBodyColor3D(uint)
GetBodyOpacity3D() / SetBodyOpacity3D(float)
GetOverrideColor() / SetOverrideColor(bool)
```

**Texture:**
```
GetTexture() / SetTexture(string)
GetTextureCenter() / SetTextureCenter(ICoordPoint)
GetTextureSize() / SetTextureSize(ICoordPoint)
GetTextureRotation() / SetTextureRotation(double)
```

**3D Model:**
```
GetModel() -> object                    IPCB_Model reference
SetModel(object)
ModelFactory_FromFilename(string, bool) -> object
ModelFactory_CreateCylinder(radius, height, color) -> object
ModelFactory_CreateSphere(radius, color) -> object
ModelFactory_CreateExtruded(minZ, maxZ, color) -> object
ModelFactory_Create(int modelType) -> object
ModelFactory_UpdateModel(radius, height, modelType) -> bool
ModelFactory_Destroy(ref object)
ModelFactory_Replace(int modelType, ref object)
ModelFactory_FromVault(title, vaultGUID, itemGUID, revGUID) -> object
SetState_FromModel()                    Update body from model bounds
GetState_ModelHasChanged() -> bool
SaveModelToFile(string fullPath) -> bool
ValidateMesh()
UniqueName() -> string
Fade(float) / ResetFade()
```

**Snap points:**
```
GetState_SnapCount() / SetState_SnapCount(int)
GetState_SnapPoint(int index) / SetState_SnapPoint(int index, ICoordPoint3D)
```

**Axes (flex):**
```
AddAxis() -> object
AxisCount() -> int
ClearAxes()
GetAxis(int index) -> object
ResetDisplacement()
```

**Identifier:**
```
SetState_Identifier(string)
```

### IPCB_Dimension (eDimensionObject = 13)

Extends IPCB_Group. Many dimension sub-types exist (Linear, Angular, Radial, Leader, etc.).

**TDimensionKind**:
```
eNoDimension
eLinearDimension
eAngularDimension
eRadialDimension
eLeaderDimension
eDatumDimension
eBaselineDimension
eCenterDimension
eOriginalDimension
eLinearDiameterDimension
eRadialDiameterDimension
```

**Core properties:**
```
GetState_DimensionKind() / SetState_DimensionKind(TDimensionKind)
GetState_X1Location() / SetState_X1Location(int)
GetState_Y1Location() / SetState_Y1Location(int)
GetState_Size() / SetState_Size(int)                  Dimension line size
GetState_LineWidth() / SetState_LineWidth(int)
```

**Text formatting:**
```
GetState_TextX() / SetState_TextX(int)
GetState_TextY() / SetState_TextY(int)
GetState_TextHeight() / SetState_TextHeight(int)
GetState_TextWidth() / SetState_TextWidth(int)
GetState_TextFont() / SetState_TextFont(short)
GetState_TextLineWidth() / SetState_TextLineWidth(int)
GetState_TextPosition() / SetState_TextPosition(TDimensionTextPosition)
GetState_TextGap() / SetState_TextGap(int)
GetState_TextFormat() / SetState_TextFormat(string)
GetState_TextDimensionUnit() / SetState_TextDimensionUnit(TDimensionUnit)
GetState_TextPrecision() / SetState_TextPrecision(int)
GetState_TextPrefix() / SetState_TextPrefix(string)
GetState_TextSuffix() / SetState_TextSuffix(string)
GetState_TextValue() / SetState_TextValue(double)
GetState_UseTTFonts() / SetState_UseTTFonts(bool)
GetState_Bold() / SetState_Bold(bool)
GetState_Italic() / SetState_Italic(bool)
GetState_FontName() / SetState_FontName(string)
```

**Arrow and extension:**
```
GetState_ArrowSize() / SetState_ArrowSize(int)
GetState_ArrowLineWidth() / SetState_ArrowLineWidth(int)
GetState_ArrowLength() / SetState_ArrowLength(int)
GetState_ArrowPosition() / SetState_ArrowPosition(TDimensionArrowPosition)
GetState_ExtensionOffset() / SetState_ExtensionOffset(int)
GetState_ExtensionLineWidth() / SetState_ExtensionLineWidth(int)
GetState_ExtensionPickGap() / SetState_ExtensionPickGap(int)
GetState_Style() / SetState_Style(int)
```

**References:**
```
GetState_References(int i) -> IDimensionReference
GetState_References_Count() -> int
SetState_References(int i, IDimensionReference)
References_Add(IDimensionReference)
References_Delete(int index)
References_DeleteLast()
References_IndexOf(object, int) -> int
References_Validate() -> bool
ResetPrefixIfNeeded()
```

### IPCB_Coordinate (eCoordinateObject = 14)

Extends IPCB_Group.

```
GetState_Size() / SetState_Size(int)
GetState_LineWidth() / SetState_LineWidth(int)
GetState_TextHeight() / SetState_TextHeight(int)
GetState_TextWidth() / SetState_TextWidth(int)
GetState_TextFont() / SetState_TextFont(short)
GetState_Style() / SetState_Style(int)
GetState_Rotation() / SetState_Rotation(double)
GetState_UseTTFonts() / SetState_UseTTFonts(bool)
GetState_Bold() / SetState_Bold(bool)
GetState_Italic() / SetState_Italic(bool)
GetState_FontName() / SetState_FontName(string)
SetState_XSizeYSize_1() -> bool
RotateAroundXY(int X, int Y, double angle)
GetState_StrictHitTest(int hitX, int hitY) -> bool
Text() -> object         Child text primitive
Track1() -> object       First axis track
Track2() -> object       Second axis track
```

### IPCB_DifferentialPair (eDifferentialPairObject = 18)

Extends IPCB_Primitive.

```
GetState_Name() / SetState_Name(string)
GetState_PositiveNet() / SetState_PositiveNet(object)   Positive net of pair
GetState_NegativeNet() / SetState_NegativeNet(object)   Negative net of pair
GetState_GatherControl() / SetState_GatherControl(bool)
```

### IPCB_Embedded (eEmbeddedObject = 20)

Extends IPCB_Primitive.

```
GetState_Name() / SetState_Name(string)
GetState_Description() / SetState_Description(string)
```

### IPCB_EmbeddedBoard (eEmbeddedBoardObject = 21)

Extends IPCB_RectangularPrimitive.

```
GetState_RowCount() / SetState_RowCount(int)
GetState_ColCount() / SetState_ColCount(int)
GetState_RowSpacing() / SetState_RowSpacing(int)
GetState_ColSpacing() / SetState_ColSpacing(int)
GetState_DocumentPath() / SetState_DocumentPath(string)
GetState_ChildBoard() -> object
GetState_Mirror() / SetState_Mirror(bool)
GetState_OriginMode() / SetState_OriginMode(TEmbeddedBoardOriginMode)
GetState_TransmitLayersEnabled(IV7_Layer) -> bool
ApplyFilterToIterator(object iterator)
GetForbiddenParameterList() -> object
```

### IPCB_SplitPlane (eSplitPlaneObject = 22)

Extends IPCB_Group.

```
GetState_AreaSize() / SetState_AreaSize(double)
GetState_PointCount() / SetState_PointCount(int)
GetState_Segments(int i) / SetState_Segments(int i, IPolySegment)
GetState_HitPrimitive(object) -> bool
PrimitiveInsidePoly(object) -> bool
SetState_XSizeYSize_1() -> bool
AcceptsLayer(int layer) -> bool
PointInPolygon(int hitX, int hitY) -> bool
xBoundingRectangle() -> ICoordRect
GetState_StrictHitTest(int hitX, int hitY) -> bool
GrowPolyshape(int dist)
RotateAroundXY(int X, int Y, double angle)
Pour()
RemovePour()
GetNegativeRegion() -> object
```

---

## IPCB_Board (Board Document)

Extends IPCB_Primitive. Root object for a PCB document.

### Document info
```
GetState_Window() -> uint                  Window handle
GetState_FileName() -> string              File path
GetState_BoardVersion() -> double          File format version
GetState_BoardID() -> int                  Unique board identifier
HasServerDocument() -> bool
GenerateUniqueID() -> string
```

### Origin and cursor
```
GetState_XOrigin() / SetState_XOrigin(int)
GetState_YOrigin() / SetState_YOrigin(int)
GetState_XCursor() / SetState_XCursor(int)
GetState_YCursor() / SetState_YCursor(int)
GetState_WorldXOrigin() / SetState_WorldXOrigin(int)
GetState_WorldYOrigin() / SetState_WorldYOrigin(int)
GetState_DisplayUnit() / SetState_DisplayUnit(int)     TUnit
```

### Grid settings
```
GetState_SnapGridSize() / SetState_SnapGridSize(double)
GetState_SnapGridSizeX() / SetState_SnapGridSizeX(double)
GetState_SnapGridSizeY() / SetState_SnapGridSizeY(double)
GetState_VisibleGridSize() / SetState_VisibleGridSize(double)
GetState_BigVisibleGridSize() / SetState_BigVisibleGridSize(double)
GetState_TrackGridSize() / SetState_TrackGridSize(double)
GetState_ViaGridSize() / SetState_ViaGridSize(double)
GetState_ComponentGridSize() / SetState_ComponentGridSize(double)
GetState_ComponentGridSizeX() / SetState_ComponentGridSizeX(double)
GetState_ComponentGridSizeY() / SetState_ComponentGridSizeY(double)
GetState_DrawDotGrid() / SetState_DrawDotGrid(bool)
```

### Layer stack
```
GetState_LayerStack() -> IPCB_LayerStack       Legacy layer stack
GetState_LayerStack_V7() -> IPCB_LayerStack_V7  V7 layer stack
GetState_MasterStack() -> IPCB_MasterLayerStack  Master layer stack
GetState_MechanicalPairs() -> object
GetState_CurrentLayer() / GetState_CurrentLayerV7() / SetState_CurrentLayerV7(IV7_Layer)
GetState_LayerColor(int layer) -> uint
GetState_LayerIsDisplayed(IV7_Layer) / SetState_LayerIsDisplayed(IV7_Layer, bool)
GetState_LayerIsUsed(IV7_Layer) / SetState_LayerIsUsed(IV7_Layer, bool)
LayerName(IV7_Layer) -> string
```

### Layer iterators (from board)
```
LayerIterator() -> IPCB_LayerObjectIterator
LayerIterator_IncludeNonEditable() -> IPCB_LayerObjectIterator
MechanicalLayerIterator() -> IPCB_LayerObjectIterator
ElectricalLayerIterator() -> IPCB_LayerObjectIterator
SignalLayerIterator() -> IPCB_LayerObjectIterator
InternalPlaneLayerIterator() -> IPCB_LayerObjectIterator
MechanicalLayers() -> IPCB_LayerSet
ElectricalLayers() -> IPCB_LayerSet
VisibleLayers() -> IPCB_LayerSet
```

### Internal planes
```
GetState_InternalPlaneNetName(IV7_Layer) -> string
SetState_InternalPlaneNetName(IV7_Layer, string)
GetState_InternalPlane1NetName() .. GetState_InternalPlane4NetName()   Legacy
SetState_InternalPlane1NetName(string) .. SetState_InternalPlane4NetName(string)
GetState_AutomaticSplitPlanes() / SetState_AutomaticSplitPlanes(bool)
```

### Drill layer pairs
```
GetState_DrillLayerPairsCount() -> int
GetState_LayerPair(int i) -> IPCB_DrillLayerPair
GetState_LayerPairByPair(int lowLay, int highLay) -> IPCB_DrillLayerPair
GetState_LayerPairByPairEx(IV7_Layer low, IV7_Layer high, int drillType) -> IPCB_DrillLayerPair
AddLayerPair(IV7_Layer low, IV7_Layer high)
```

### Board outline
```
GetState_BoardOutline() -> IPCB_BoardOutline
CreateBoardOutline() -> IPCB_BoardOutline
UpdateBoardOutline()
RebuildBoardOutline(IV7_Layer definingLayer)
```

### Selection
```
GetState_SelectecObjectCount() -> int
GetState_SelectecObject(int i) -> IPCB_Primitive
SelectedObjectsCount() -> int
SelectedObjects_BeginUpdate()
SelectedObjects_Clear()
SelectedObjects_Add(object p)
SelectedObjects_EndUpdate()
```

### Iteration (Board-level)
```
BoardIterator_Create() -> IPCB_BoardIterator
BoardIterator_Destroy(ref object)
SpatialIterator_Create() -> IPCB_SpatialIterator
SpatialIterator_Destroy(ref object)
GetPrimitiveCount(ITransportSet objSet, ITransportSet layerSet, int method) -> int
```

### Object management
```
AddPCBObject(object pcbObject)
RemovePCBObject(object pcbObject)
GetPcbComponentByRefDes(string) -> IPCB_Component
ShowPCBObject(object) / HidePCBObject(object) / InvertPCBObject(object)
EnableAllPrimitives(bool enable)
```

### Design rules
```
FindDominantRuleForObject(object prim, int ruleKind) -> IPCB_Rule
FindDominantRuleForObjectPair(object prim1, object prim2, int ruleKind) -> IPCB_Rule
RuleNameUnique(object rule, string name) -> bool
PrimPrimDistance(object prim1, object prim2) -> int
```

### Net operations
```
ConnectivelyValidateNets()
AnalyzeNet(object net)
CleanNet(object net)
NetNameIsUnique(object net, string name) -> bool
DifferentialPairNameIsUnique(object pair, string name) -> bool
ClassNameIsUnique(object cls, string name) -> bool
```

### Undo/redo
```
ClearUndoRedo()
NewUndo() / EndUndo()
DoUndo() / DoRedo()
```

### View and rendering
```
GetState_ViewConfigIs3D() -> bool
GetState_ViewConfigAsString(out configType, out config) -> bool
SetState_ViewConfigFromString(string configType, string config) -> bool
SetState_ViewConfigFromString_ForceRebuild(string, string) -> bool
GetState_ViewConfigTopSolderMaskColor3D() -> uint
GetState_ViewConfigColor2D(IV7_Layer) -> uint
GetState_ViewConfigOpacity2D(IV7_Layer, int objectId) -> float
GetState_ViewConfigColor3D(IV7_Layer) -> uint
GetState_ViewConfigOpacity3D(IV7_Layer) -> float
GetState_DrawMode(int objectId) / SetState_DrawMode(int objectId, int value)
GetState_DesignatorDisplayMode() / SetState_DesignatorDisplayMode(int)
GetState_ZTopBottom(out float zTop, out float zBottom)
GraphicalView_ZoomRedraw()
GraphicalView_ZoomOnRect(int x1, int y1, int x2, int y2)
WindowBoundingRectangle() -> ICoordRect
GraphicalView_ViewportRect() -> ICoordRect
ViewManager_FullUpdate() / ViewManager_UpdateLayerTabs()
ViewManager_GraphicallyInvalidatePrimitive(object)
SetState_DocumentHasChanged()
GetState_MainGraphicalView() -> object
GetState_Viewport() / SetState_Viewport(object)
```

### Pad/via templates
```
GetState_PadViaCache() -> object
GetState_PadViaLibrary() -> object
LinkToTemplate(object prim, object template)
UnlinkFromTemplate(object prim)
FindLinkedBoardTemplate(object prim) -> object
FindLinkedBoardLibrary(object prim) -> object
```

### Other
```
GetState_PCBSheet() -> object
GetState_BoardLayerSetManager() -> object
GetState_PinPairsManager() -> object
GetState_RoutingOptions() -> object
GetState_SystemOptions() -> object (via ServerInterface)
GetState_OutputOptions() / GetState_ECOOptions() / GetState_GerberOptions() / GetState_PrinterOptions()
GetState_PlacerOptions() -> object
RebuildPadCaches()
RebuildSplitBoardRegions(bool fullRebuild)
RecreateHandles()
InvalidateScopeTester() / ValidateScopeTester()
InvalidatePlane(int layer) / ValidateInvalidPlanes()
GetState_PolygonNameTemplate() / SetState_PolygonNameTemplate(int)
GetState_RouteToolPathLayer() / SetState_RouteToolPathLayer(IV7_Layer)
ReportStackupCompatibilityForEmbeddedBoards(bool) -> bool
GetState_PCB3DMovieManager() -> object
```

---

## IPCB_Library (PCB Library Document)

Not an IPCB_Primitive subtype -- separate interface.

```
GetState_CurrentComponent() / SetState_CurrentComponent(object)
GetState_Board() -> object                   Underlying board
ComponentCount() -> int
GetComponent(int i) -> object                By index
GetComponentByName(string) -> object         By name
AddComponent(string name) -> object          Create and register
CreateNewComponent() -> object               Create without registering
RegisterComponent(object) -> bool
DeRegisterComponent(object) -> bool
RemoveComponent(ref object)                  Delete component
GetUniqueCompName(string test) -> string     Ensure unique name
SetBoardToComponentByName(string) -> bool    Switch active component
Navigate_FirstComponent()
SetCurrentComponentReference(int X, int Y)
LibraryIterator_Create() -> IPCB_LibraryIterator
LibraryIterator_Destroy(ref object)
LibraryLoaderSaver_Create() -> object
LibraryLoaderSaver_Destroy(ref object)
SplitIntoComponents(string dir, bool overwrite, bool cleanNames)
RefreshView()
HasServerDocument() -> bool
GetState_LibraryID() -> int
GetState_IsSingleComponentMode() -> bool
SaveComponentWithLibrary(string compName, string fileName) -> bool
SaveEmptyLibrary(string fileName) -> bool
```

**Vault properties:**
```
GetState_VaultGUID() / SetState_VaultGUID(string)
GetState_FolderGUID() / SetState_FolderGUID(string)
GetState_LifeCycleDefinitionGUID() / SetState_LifeCycleDefinitionGUID(string)
GetState_RevisionNamingSchemeGUID() / SetState_RevisionNamingSchemeGUID(string)
LinkDocumentToReleaseVault(string params, ref string errorMsg) -> bool
ReleaseDocument(string params, ref string errorMsg) -> bool
```

### IPCB_LibComponent (Library Footprint)

Extends IPCB_Group.

```
GetState_Pattern() / SetState_Pattern(string)          Footprint name
GetState_Height() / SetState_Height(int)               Component height
GetState_Description() / SetState_Description(string)  Description text
GetState_ItemGUID() / SetState_ItemGUID(string)
GetState_ItemRevisionGUID() / SetState_ItemRevisionGUID(string)
TransferAllPrimitivesBackFromBoard()
TransferAllPrimitivesOntoBoard()
SaveToFile(string vfsAddress)
CopyTo(object dest, int copyMode)           TCopyMode
ReleasableInterface() -> object
SaveModelToFile(string filename, bool saveAsPart) -> bool
```

---

## IPCB_ServerInterface (PCB Server)

The PCB server singleton. Provides factory methods and global state.

### Object factories
```
PCBObjectFactory(int objectId, int dimensionKind, int creationMode) -> object
PCBClassFactory(int classKind) -> object
PCBClassFactoryByClassMember(int classKind) -> object
PCBRuleFactory(int ruleKind) -> object
PCBContourFactory() -> object
PCBContourMaker() -> object
PCBContourUtilities() -> object
PCBGeometricPolygonFactory() -> object
PCBGeometryMaker() -> object
```

**TObjectCreationMode**:
```
eCreate_Default       Normal creation
eCreate_GlobalCopy    Clone/copy creation
```

### Document access
```
GetCurrentPCBBoard() -> object
GetPCBBoardByPath(string path) -> object
GetPCBBoardByBoardID(int boardID) -> object
LoadPCBBoardByPath(string path) -> object
GetCurrentPCBLibrary() -> object
GetPCBLibraryByPath(string path) -> object
GetPCBLibraryByLibraryID(int libraryID) -> object
LoadPCBLibraryFromFile(string path) -> object
CreatePCBLibrary() -> object
LoadCompFromLibrary(string pattern, string libPath) -> object
CreatePCBLibComp() -> object
```

### Destruction
```
DestroyPCBObject(ref object)
DestroyPCBContour(ref object)
DestroyPCBLibComp(ref object)
DestroyPCBLibrary(ref object)
```

### Utilities
```
GetState_SystemOptions() -> object
GetState_InteractiveRoutingOptions() -> object
GetState_TTFLettersCache() -> object
GetState_TTFontsCache() -> object
GetState_SpecialStringConverter() -> object
GetState_PadViaLibraryManager() -> object
GetParametersFactory() -> object
LayerSet() -> object
LayerUtils() -> object
PrimitiveComparator() -> object
Board3DModelExporter() -> object
GetOccWrapper() -> object
```

### Processing
```
PreProcess() / PostProcess()
PreBatchProcess() / PostBatchProcess()
EnableFastParams() / DisableFastParams()
DocumentLiveHighlight_Start(string path) / DocumentLiveHighlight_Stop(string path)
RefreshDocumentView(string path)
```

### UI
```
RunFontEditorDialog(...) -> bool
PaintFootprintThumbnail(viewName, viewFileAddr, width, height) -> uint
CreateComponentPainter() -> object
CreateDocumentPainter(int mode) -> object
CreateLayerStackupPainter() -> object
CreatePCBWideStringList() -> object
PcbApi_Export_ToPainter(ref object painter, string libRef, string libPath)
```

### Cross-selection
```
SetState_CanFastCrossSelect_Receive(bool) / GetState_CanFastCrossSelect_Receive() -> bool
SetState_CanFastCrossSelect_Emit(bool) / GetState_CanFastCrossSelect_Emit() -> bool
```

---

## Iterator Pattern

### IPCB_AbstractIterator (Base)

```
FirstPCBObject() -> object      Get first matching object
NextPCBObject() -> object       Get next matching object (null = end)
SetState_FilterAll()             Reset to match everything
AddFilter_ObjectSet(ITransportSet)   Filter by object type set
AddFilter_LayerSet(ITransportSet)    Filter by layer set (multiple overloads)
AddFilter_IPCB_LayerSet(object)      Filter by IPCB_LayerSet
AddFilter_Area(int x1, int y1, int x2, int y2)   Spatial filter
AddFilter_AllLayers()                 Include all layers
```

### IPCB_BoardIterator

Extends AbstractIterator. Created via `IPCB_Board.BoardIterator_Create()`.

```
AddFilter_Method(TIterationMethod)   Filter free/component/all primitives
```

**TIterationMethod**:
```
eProcessAll        All primitives
eProcessFree       Free (non-component) primitives only
eProcessComponent  Component primitives only
```

### IPCB_GroupIterator

Extends AbstractIterator. Created via `IPCB_Group.GroupIterator_Create()`. No additional methods.

### IPCB_SpatialIterator

Extends AbstractIterator. Created via `IPCB_Board.SpatialIterator_Create()`.

```
AddFilter_ProcessSpecialLayers()     Include special/system layers
```

### IPCB_LibraryIterator

Extends AbstractIterator. Created via `IPCB_Library.LibraryIterator_Create()`. No additional methods. Iterates over library components.

### IPCB_LayerObjectIterator

Created from board layer iterator methods. Iterates over layer objects in the stack.

### Usage Pattern

```
iterator = board.BoardIterator_Create()
iterator.AddFilter_ObjectSet(objectTypeSet)
iterator.AddFilter_AllLayers()
iterator.AddFilter_Method(eProcessAll)
obj = iterator.FirstPCBObject()
while obj != null:
    // process obj
    obj = iterator.NextPCBObject()
board.BoardIterator_Destroy(ref iterator)
```

---

## IV7_PadCache (Pad/Via Electrical Cache)

Stores resolved electrical properties for pads and vias (from design rules or manual overrides).

```
GetPlaneConnectionStyle() -> TPlaneConnectionStyle
GetReliefConductorWidth() -> int
GetReliefEntries() -> short
GetReliefAirGap() -> int
GetPowerPlaneReliefExpansion() -> int
GetPowerPlaneClearance() -> int
GetPasteMaskExpansion() -> int
GetSolderMaskExpansion() -> int
GetSolderMaskBottomExpansion() -> int
GetUseSeparateExpansions() -> bool
GetViaHeight() -> int
```

Each property has a validity flag:
```
GetPlaneConnectionStyleValid() -> TCacheState
GetReliefConductorWidthValid() -> TCacheState
GetReliefEntriesValid() -> TCacheState
GetReliefAirGapValid() -> TCacheState
GetPowerPlaneReliefExpansionValid() -> TCacheState
GetPasteMaskExpansionValid() -> TCacheState
GetSolderMaskExpansionValid() -> TCacheState
GetPowerPlaneClearanceValid() -> TCacheState
GetPlanesValid() -> TCacheState
GetViaHeightValid() -> TCacheState
```

**TCacheState**:
```
eCacheInvalid    Value not computed yet / stale
eCacheValid      Value computed from design rules
eCacheManual     Value manually overridden by user
```

**TPlaneConnectionStyle** (for cache):
```
ePlaneNoConnect        No connection to plane
ePlaneReliefConnect    Thermal relief connection
ePlaneDirectConnect    Direct solid connection
```

**TPlaneConnectStyle** (for polygon connect rule):
```
eReliefConnectToPlane
eDirectConnectToPlane
eNoConnect
```

---

## Shape Enumeration (TShape)

```
eNoShape             No shape defined
eRounded             Round (circle for equal X/Y)
eRectangular         Rectangle
eOctagonal           Octagon
eCircleShape         Circle (deprecated, use eRounded)
eArcShape            Arc shape
eTerminator          Terminator shape
eRoundRectShape      Rounded rectangle
eRotatedRectShape    Rotated rectangle
eRoundedRectangular  Rounded rectangular (alt)
eCustomShape         Custom shape (complex polygon)
```

---

## PCBDataModel Internal Layer (IPCBDM_Primitive)

The internal data model representation, accessed via `IPCB_Primitive3.GetDMPrimitive()`.

```
UID() -> nint                                     Unique pointer-based ID
GetProperty_DataModel() / SetProperty_DataModel()  Parent data model
GetProperty_ObjectID() / SetProperty_ObjectID(TObjectId)
GetProperty_Layer() / SetProperty_Layer(TV7_Layer)
GetProperty_BoundingRectangle() / SetProperty_BoundingRectangle(TCoordRect)
GetProperty_VNext() / SetProperty_VNext()          Linked-list: next in container
GetProperty_VPrev() / SetProperty_VPrev()          Linked-list: prev in container
GetProperty_VTail() / SetProperty_VTail()          Linked-list: tail pointer
GetProperty_VComponent() / SetProperty_VComponent() Parent component link
GetProperty_VNet() / SetProperty_VNet()             Parent net link
GetProperty_VPolygon() / SetProperty_VPolygon()     Parent polygon link
GetProperty_SharedUnionOwner() / SetProperty_SharedUnionOwner()
InBoard() / SetInBoard(bool)
InNet() / SetInNet(bool)
InComponent() / SetInComponent(bool)
SetState_Default()                     Initialize to defaults
GetState_BoundingRectangle(out lx, ly, hx, hy)
CopyTo(IPCBDM_Primitive dest, TCopyMode)
SetIsKeepout(bool)
SetKeepoutRestrictions(TKeepoutRestrictionsSet)
ClearScopeCache()
```

This reveals the internal linked-list data structure: primitives are chained via VNext/VPrev pointers within their parent container (board, component, net, polygon).

---

## Design Rules (TRuleKind)

Complete enumeration of all design rules:

```
Value  Name                              String ID
-----  --------------------------------  --------------------------
0      eRule_Clearance                   "Clearance"
1      eRule_ParallelSegment             "ParallelSegment"
2      eRule_MaxMinWidth                 "Width"
3      eRule_MaxMinLength                "Length"
4      eRule_MatchedLengths              "MatchedLengths"
5      eRule_DaisyChainStubLength        "StubLength"
6      eRule_PowerPlaneConnectStyle      "PlaneConnect"
7      eRule_RoutingTopology             "RoutingTopology"
8      eRule_RoutingPriority             "RoutingPriority"
9      eRule_RoutingLayers               "RoutingLayers"
10     eRule_RoutingCornerStyle          "RoutingCorners"
11     eRule_RoutingViaStyle             "RoutingVias"
12     eRule_PowerPlaneClearance         "PlaneClearance"
13     eRule_SolderMaskExpansion         "SolderMaskExpansion"
14     eRule_PasteMaskExpansion          "PasteMaskExpansion"
15     eRule_ShortCircuit                "ShortCircuit"
16     eRule_BrokenNets                  "UnRoutedNet"
17     eRule_ViasUnderSMD                "ViasUnderSMD"
18     eRule_MaximumViaCount             "MaximumViaCount"
19     eRule_MinimumAnnularRing          "MinimumAnnularRing"
20     eRule_PolygonConnectStyle         "PolygonConnect"
21     eRule_AcuteAngle                  "AcuteAngle"
22     eRule_ConfinementConstraint       "RoomDefinition"
23     eRule_SMDToCorner                 "SMDToCorner"
24     eRule_ComponentClearance          "ComponentClearance"
25     eRule_ComponentRotations          "ComponentOrientations"
26     eRule_PermittedLayers             "PermittedLayers"
27     eRule_NetsToIgnore                "NetsToIgnore"
28     eRule_SignalStimulus              "SignalStimulus"
29     eRule_Overshoot_FallingEdge       "OvershootFalling"
30     eRule_Overshoot_RisingEdge        "OvershootRising"
31     eRule_Undershoot_FallingEdge      "UndershootFalling"
32     eRule_Undershoot_RisingEdge       "UndershootRising"
33     eRule_MaxMinImpedance             "MaxMinImpedance"
34     eRule_SignalTopValue              "SignalTopValue"
35     eRule_SignalBaseValue             "SignalBaseValue"
36     eRule_FlightTime_RisingEdge       "FlightTimeRising"
37     eRule_FlightTime_FallingEdge      "FlightTimeFalling"
38     eRule_LayerStack                  "LayerStack"
39     eRule_MaxSlope_RisingEdge         "SlopeRising"
40     eRule_MaxSlope_FallingEdge        "SlopeFalling"
41     eRule_SupplyNets                  "SupplyNets"
42     eRule_MaxMinHoleSize              "HoleSize"
43     eRule_TestPointStyle              "FabricationTestpoint"
44     eRule_TestPointUsage              "FabricationTestPointUsage"
45     eRule_UnconnectedPin              "UnConnectedPin"
46     eRule_SMDToPlane                  "SMDToPlane"
47     eRule_SMDNeckDown                 "SMDNeckDown"
48     eRule_LayerPair                   "LayerPairs"
49     eRule_FanoutControl               "FanoutControl"
50     eRule_MaxMinHeight                "Height"
51     eRule_DifferentialPairsRouting    "DiffPairsRouting"
52     eRule_HoleToHoleClearance         "HoleToHoleClearance"
53     eRule_MinimumSolderMaskSliver     "MinimumSolderMaskSliver"
54     eRule_SilkToSolderMaskClearance   "SilkToSolderMaskClearance"
55     eRule_SilkToSilkClearance         "SilkToSilkClearance"
56     eRule_NetAntennae                 "NetAntennae"
57     eRule_AssyTestPointStyle          "AssemblyTestpoint"
58     eRule_AssyTestPointUsage          "AssemblyTestPointUsage"
59     eRule_SilkToBoardRegion           "SilkToBoardRegionClearance"
60     eRule_SMDPADEntry                 "SMDEntry"
61     eRule_None                        (no string)
62     eRule_ModifiedPolygon             "UnpouredPolygon"
63     eRule_BoardOutlineClearance       "BoardOutlineClearance"
64     eRule_BackDrilling                "BackDrilling"
65     eRule_Creepage                    "Creepage"
66     eRule_ReturnPath                  "ReturnPath"
67     eRule_RoutingNeckDown             "RoutingNeckDown"
68     eRule_Wirebonding                 "WireBonding"
69     eRule_ZAxisClearance              "ZAxisClearance"
```

---

## Constants (Pcbtypes.Consts)

### Coordinate System
```
kInternalUnits = 10000       Internal units per mil
k1Mil = 10000                = kInternalUnits
k1Inch = 10000000            = 1000 * kInternalUnits
kMaxCoord = 999990000        Maximum coordinate value
kMinCoord = 0                Minimum coordinate value
kMilAccuracy = 1000          Mil rounding accuracy
kMMAccuracy = 100000         MM rounding accuracy
```

### String Limits
```
kMaxPadNameLength = 20
kMaxNetNameLength = 50
kMaxFreeStringLength = 254
kMaxPadTypeNameLength = 10
kMaxPatternLength = 250
kMaxPolySize = 5000
kMaxStrokes = 2000
kMaxReferenceCount = 200
```

### Bitfield Masks (primitive flags byte)
```
InBoardBitMask = 1           Bit 0: InBoard
InPolygonBitMask = 2         Bit 1: InPolygon
InComponentBitMask = 8       Bit 3: InComponent
InNetBitMask = 16            Bit 4: InNet
InCoordinateBitMask = 32     Bit 5: InCoordinate
InDimensionBitMask = 64      Bit 6: InDimension
```

### Selection/State Bitfield Masks
```
kColorMask = 15               Bits 0-3: Color index
kDRCErrorBitMask = 8          Bit 3: DRC error
kSelectedBitMask = 16         Bit 4: Selected
kAllowGlobEditBitMask = 256   Bit 8: Allow global edit
kTentingBitMask = 1024        Bit 10: Tenting
kTestPoint_TopBitMask = 2048  Bit 11: Top test point
kKeepoutBitMask = 4096        Bit 12: Keepout
kTestPoint_BottomBitMask = 8192  Bit 13: Bottom test point
kTearDropBitMask = 16384      Bit 14: Teardrop
kUserRoutedBitMask = 32768    Bit 15: User routed
```

### BitField3 (additional flags byte)
```
kBitfield3Disabled = 1        Bit 0: Primitive disabled
kBitField3TentingTop = 2      Bit 1: Top tenting
kBitField3TentingBottom = 4   Bit 2: Bottom tenting
```

### Default Layer Stack Values
```
cDefault_CopperLayerHeight = 14000       1.4 mil copper
cDefault_DielectricHeight = 126000       12.6 mil dielectric
cDefault_DielectricConstant = 4.8        FR-4 Er
cDefault_DielectricType = eCore
cDefault_DielectricMaterial = "FR-4"
```

### Messages (PCBM_*)
```
PCBM_NullMessage = 0
PCBM_BeginModify = 1
PCBM_BoardRegisteration = 2
PCBM_EndModify = 3
PCBM_CancelModify = 4
PCBM_Create = 5
PCBM_Destroy = 6
PCBM_ProcessStart = 7
PCBM_ProcessEnd = 8
PCBM_ProcessCancel = 9
PCBM_YieldToRobots = 10
PCBM_CycleEnd = 11
PCBM_CycleStart = 12
PCBM_SystemInvalid = 13
PCBM_SystemValid = 14
PCBM_ViewUpdate = 15
PCBM_UnDoRegister = 16
```

---

## ExtendedPrimitiveInformation

No `ExtendedPrimitiveInformation` type was found in the decompiled .NET code. This concept likely exists only in the Delphi side (binary file format record handling) or as parameter string extensions. The .NET COM interfaces expose extended properties via the `IPCB_Primitive2` interface and per-type extended interfaces (`IPCB_Pad2`, `IPCB_Pad3`, `IPCB_Pad4`, `IPCB_Via2`, etc.).

The binary file format likely stores extended primitive data in sidecar streams (e.g., `ExtendedPrimitiveInformation` OLE stream), but the .NET API abstracts this away through the GetState_/SetState_ property pattern.

---

## IPCB_Factory (Layer Set Factory)

```
CreateLayerSet() -> object                          Empty layer set
CreateLayerSet_1(IV7_Layer) -> object               Single-layer set
AllLayers() -> object                               Set containing all layers
SignalLayers() -> object                            Set of signal layers only
CreateLayerSet_2(ITransportSet) -> object           Copy from transport set
CreateLayerSet_3(ITransportSet) -> object           Copy (variant)
```

---

## Inheritance Hierarchy Summary

```
IPCB_Primitive
  |-- IPCB_Arc
  |-- IPCB_Pad -> IPCB_Pad2 -> IPCB_Pad3 -> IPCB_Pad4
  |-- IPCB_Via
  |-- IPCB_Track
  |-- IPCB_Connection
  |-- IPCB_DifferentialPair
  |-- IPCB_Embedded
  |-- IPCB_RectangularPrimitive
  |     |-- IPCB_Text
  |     |-- IPCB_Fill
  |     |-- IPCB_EmbeddedBoard
  |-- IPCB_Region
  |     |-- IPCB_ComponentBody
  |-- IPCB_Group
  |     |-- IPCB_Component
  |     |-- IPCB_Net
  |     |-- IPCB_Polygon
  |     |-- IPCB_Dimension
  |     |-- IPCB_Coordinate
  |     |-- IPCB_SplitPlane
  |     |-- IPCB_LibComponent
  |-- IPCB_Board (also Group-like: has AddPCBObject/RemovePCBObject)

IPCB_Primitive2    (separate interface, QI'd from primitives)
IPCB_Primitive3    (modern vtable version of IPCB_Primitive)

IPCB_AbstractIterator
  |-- IPCB_BoardIterator
  |-- IPCB_GroupIterator
  |-- IPCB_SpatialIterator
  |-- IPCB_LibraryIterator

IPCB_LayerStackBase
  |-- IPCB_MasterLayerStack

IPCB_LayerObject
  |-- IPCB_LayerObject_V7

IPCB_Library       (standalone, not IPCB_Primitive)
IPCB_ServerInterface  (singleton, factory)
IPCB_Factory       (layer set factory)
```
