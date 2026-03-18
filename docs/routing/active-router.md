# Altium Active Router — Reverse Engineering Notes

**Source**: Decompiled C# from AD26-dotnet (`Altium.Edp.Interfaces/RT_PCB/`, `PCBInterfaces/`,
`ConstraintsManager.Contracts/`). The actual routing *algorithms* live in undecompiled Delphi
DLLs; the C# layer is purely the COM interface glue and UI ViewModel layer.

---

## Overview

Altium's "Active Router" is really a family of interactive routing processes, each exposed
as a Delphi COM interface to the C# / Script layer. All processes share a common base
(`IPCB_CustomInteractiveRoutingProcess`) and are identified by a `TInteractiveProcessId` enum.

There is **no single "ActiveRouter" interface**. The UI resource keys
`PcbActiveRouteSingleEndedLine`, `PcbActiveRouteDiffPairLine`, etc. are just toolbar icon
names for the interactive routing tool.

The main routing processes are:

| Process interface | `TInteractiveProcessId` value | Purpose |
|---|---|---|
| `IPCB_InteractiveRoutingProcess` | `pidPcbInteractiveRouting` | Single-ended interactive route |
| `IPCB_InteractiveDiffPairRoutingProcess` | `pidPcbInteractiveDiffPairRouting` | Diff-pair interactive route |
| `IPCB_InteractiveMultiRoutingProcess` | `pidPcbInteractiveMultiRouting` | Bus / multi-net route |
| `IPCB_SlidingRoutingProcess` | `pidPcbInteractiveSliding` | Slide existing trace segments |
| `IPCB_InteractiveLineRoutingProcess` | `pidPcbInteractiveLineRouting` | Raw line placement (keepout mode) |
| `IPCB_ViaDraggingProcess` | `pidPcbInteractiveViaDragging` | Drag a via along connected traces |
| `IPCB_AccordionMakerSettings` (not a process) | `pidPcbInteractiveLengthTuning` / `pidPcbInteractiveDiffPairLengthTuning` | Tuning / meander insertion |

All process interfaces inherit from `IInteractiveProcess` (namespace `RT_InteractiveProcess`)
and `IPCB_InteractiveProcess` (namespace `RT_PCB`).

---

## Key Interfaces

All in namespace `RT_PCB` unless noted. Files in
`AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/`.

### `IPCB_CustomInteractiveRoutingProcess`
GUID: `A1165DEC-03A4-462C-ADD5-8AB5E940C187`

The base shared by all three main routing processes.  Key methods (getters and matching
setters exist for all state properties):

```csharp
// Board and net context
IPCB_Board GetState_Board()
IPCB_Net GetState_Net()
IPCB_NetList GetState_NetList()

// Layer management
TV7_Layer GetState_CurrentLayer()
TV7_Layer GetState_NextLayer()
void SetState_CurrentLayer(TV7_Layer)
void SetState_NextLayer(TV7_Layer)
void SetState_InternalNextLayer(TV7_Layer)  // internal bypass

// Obstacle/conflict mode
TAdvancedRouteMode GetState_RouteMode()
bool GetState_RouteModeEnabled(TAdvancedRouteMode argMode)
void SetState_RouteMode(TAdvancedRouteMode)
void SetState_RouteModeEnabled(TAdvancedRouteMode, bool)

// Width
int GetState_Width()
TRoutingWidthMode GetState_RoutingWidthMode()
TRoutingWidthMode NextRoutingWidthMode()
bool GetState_PickupWidthFromExistingRoutes()
TRoutingWidthMode GetTrackWidthMode(bool argCheckPickupTrackWidth)
bool GetState_AutoNecking()

// Via
int GetState_ViaDiameter()
int GetState_HoleSize()
int GetState_ViaLayerPair()
TRoutingWidthMode GetState_ViaSizeMode()
IPCB_PadViaTemplate GetState_ViaTemplate()
bool GetState_ViaTemplateChanged()
IPCB_ViaCombinationManagerInterface GetState_ViaCombinationManager()
TRoutingWidthMode NextViaSizeMode()
int NextViaLayerPair()
IPCB_PadViaTemplate NextViaTemplate()
IPCB_RoutingViaStyleRule GetState_RoutingViaStyleRule()
void EditRoutingViaStyleRule()

// Corner style
TRoutingCornerStyle GetState_RoutingCornerStyle()

// Gloss / hugging / arc
TGlossEffort GetState_GlossEffort()
TGlossEffort GetState_NeighborGlossEffort()
THuggingStyle GetState_HuggingStyle()
double GetState_MiterSize()
double GetState_MinimumArcSize()
int GetState_PadEntryStability()

// Interactive options toggles
bool GetState_AllowViaPushing()
bool GetState_AutoRemoveAntennas()
bool GetState_AutoRemoveLoops()
bool GetState_AutoTerminateRouting()
bool GetState_CornerRounding()
bool GetState_DisplayClearanceBounds()
bool GetState_FollowMouseTrail()
bool GetState_FollowMode()
bool GetState_LegacyRouter()
bool GetState_PinSwapping()
bool GetState_ReduceClearanceDisplayArea()
bool GetState_RestrictTo9045()
TCoordPoint GetState_RoutingPoint()
int GetState_SubnetJumperLength()

// Options page access
IPCB_RoutingOptionsPage GetRoutingOptions()
```

### `IPCB_InteractiveRoutingProcess`
GUID: `B52BE2E5-F20C-4E12-9AA8-E2EEC4FA1EFB`
Inherits `IPCB_CustomInteractiveRoutingProcess`. Additional methods specific to single-ended:

```csharp
double GetState_Impedance()
IPCB_MaxMinWidthConstraint GetState_WidthRule()
bool GetState_ShowLengthGauge()
IPCB_DifferentialPairsRoutingRule GetState_DiffPairRule()
IPCB_DifferentialPair GetState_DifferentialPair()
bool ProjectAvailableForPinSwap(out string argErrorMessage)
void EditMaxMinWidthRule()
```

### `IPCB_InteractiveDiffPairRoutingProcess`
GUID: `44F1F1CC-C349-4709-BCB7-2BC3DD2B3538`
Inherits `IPCB_CustomInteractiveRoutingProcess`. Additional methods specific to diff-pair:

```csharp
IPCB_DifferentialPair GetState_DifferentialPair()
TRoutingDiffPairGapMode GetState_GapMode()
TRoutingDiffPairGapMode NextGapMode()
void SetState_GapMode(TRoutingDiffPairGapMode)
IPCB_DifferentialPairsRoutingRule GetState_DiffPairRule()
bool GetState_ShowLengthGauge()
bool ProjectAvailableForPinSwap(out string argErrorMessage)
```

### `IPCB_InteractiveMultiRoutingProcess`
GUID: `353560E5-B456-4E39-A483-7701846693F0`
Inherits `IPCB_CustomInteractiveRoutingProcess`. Additional methods for bus routing:

```csharp
IPCB_MaxMinWidthConstraint GetState_WidthRule()
int GetState_BusSpacing()
void SetState_BusSpacing(int)
int GetState_GetMinClearanceFromRule()
void EditMaxMinWidthRule()
```

### `IPCB_SlidingRoutingProcess`
GUID: `78855205-FEB0-4A9D-A706-2D523DFA6F91`
Inherits `IPCB_InteractiveProcess` only (not `IPCB_CustomInteractiveRoutingProcess`).
For *sliding* existing trace segments:

```csharp
double GetState_MiterSize()
int GetState_PadEntryStability()
bool GetState_ShowLengthGauge()
bool GetState_ReduceClearanceDisplayArea()
bool GetState_DisplayClearanceBoundaries()
bool GetState_AllowViaPusing()   // note: typo in original ("Pusing")
TGlossEffort GetState_GlossEffort()
TGlossEffort GetState_NeighborGlossEffort()
THuggingStyle GetState_HuggingStyle()
TAdvancedRouteMode GetState_Sliding()
TVertexAction GetState_VertexAction()
IPCB_Net GetState_Net()
long GetState_NetLength()
double GetState_NetDelay()
double GetState_MinimumArcSize()
bool GetState_IsSingleNet()
TV7_Layer GetState_Layer()
IPCB_MaxMinWidthConstraint GetState_WidthRule()
IPCB_RoutingViaStyleRule GetState_RoutingViaStyleRule()
IPCB_MatchedNetLengthsConstraint GetState_MatchedNetLengthsRule()
IPCB_RoutingOptionsPage GetRoutingOptions()
void EditMaxMinWidthRule()
void EditRoutingViaStyleRule()
void EditMatchedNetLengthsRule()
```

### `IPCB_InteractiveLineRoutingProcess`
GUID: `8C29C6EF-9BC7-40AC-8CCD-E53C9C76A9B2`
For placing lines (keepout or non-electrical):

```csharp
bool GetState_PlacingKeepOuts()
TV7_Layer GetState_CurrentLayer()
int GetState_LineWidth()
```

### `IPCB_ViaDraggingProcess`
GUID: `5C571ED9-DD48-449A-BB30-0EB3591A9717`

```csharp
IPCB_RoutingOptionsPage GetRoutingOptions()
```

### `IPCB_RoutingOptionsPage`
GUID: `2131E587-7CA7-4E27-B01C-7C4D5CC9FFFE`
Persistent user preferences for all interactive routing operations:

```csharp
// Conflict resolution
TAdvancedRouteMode GetState_ConflictMode()
bool GetState_ConflictModeEnabled(TAdvancedRouteMode)

// Smart drag
bool GetState_SmartDrag()
TAvoidObstacleMode GetState_DragAvoidObstacleMode()
TDragSelectUnselectMode GetState_DragUnselected()
TDragSelectUnselectMode GetState_Dragselected()
TVertexAction GetState_VertexAction()

// Component interaction
TPushMode GetState_ComponentPushing()
bool GetState_ComponentReroute()
bool GetState_ComponentMoveRelevantRouting()
int GetState_ComponentMoveRelevantRoutingPinsLimit()
TNetLineMode GetState_ComponentNetLineMode()

// Routing auto-actions
bool GetState_AutoTerminateRouting()
bool GetState_AutoRemoveLoops()
bool GetState_AutoRemoveViaLoops()
bool GetState_AutoRemoveAntennas()
bool GetState_AllowViaPushing()

// Display
bool GetState_DisplayClearanceBounds()
bool GetState_ReduceClearanceDisplayArea()
bool GetState_ShowGaugeDuringDragging()

// Width / via
bool GetState_PickupWidthFromExistingRoutes()
TRoutingWidthMode GetState_RoutingWidthMode()
TRoutingWidthMode GetState_ViaSizeMode()
int GetState_WidthToUse()
bool GetState_AutoNecking()

// Gloss / corner
TGlossEffort GetState_GlossEffort()
TGlossEffort GetState_NeighborGlossEffort()
THuggingStyle GetState_HuggingStyle()
double GetState_MinimumArcSize()
double GetState_MiterSize()
int GetState_PadEntryStability()

// Diff-pair
bool GetState_DifferentialPairMode()
bool GetState_DragWithMiters()
bool GetState_DragMergeParallel()
bool GetState_DifferentialPairModeForVias()
int GetState_DiffPairGapToUse()

// Clearance preference
double GetState_PreferredClearanceRatio()
bool GetState_PreferredClearanceApply()
bool GetState_PreferredClearanceAdjustVias()

// Obstacle avoidance
bool GetState_AvoidPolygons()
bool GetState_AvoidRooms()
bool GetState_PreservePath()

// Dragging
bool GetState_DisableTraceCenteringWhenDragging()
```

### `IPCB_AdvanceRouteCommands`
GUID: `F0831499-190D-4429-8B4A-6803D583FC7E`
Low-level primitive manipulation API used by the router to build paths:

```csharp
void AddPrimitiveToBoard(IPCB_Primitive back, IPCB_Primitive toAdd, IPCB_Primitive forward)
void RemovePrimitiveFromBoard(IPCB_Primitive p)
void ReplaceConnectedPrimitivesInBoard(IPCB_Group toRemove, IPCB_Group toAdd)
IPCB_Group GetRoutedPath()
bool IsPushablePrimitive(IPCB_Primitive p)
void SetState_BackConnectedPrim(IPCB_Primitive, IPCB_Primitive back)
IPCB_Primitive GetState_BackConnectedPrim(IPCB_Primitive)
void SetState_ForwardConnectedPrim(IPCB_Primitive, IPCB_Primitive forward)
IPCB_Primitive GetState_ForwardConnectedPrim(IPCB_Primitive)
int GetWidthFromRouter(IPCB_Primitive)
bool IsUsingAlternativeTargets()
TV6_Layer GetCurrentLayerFromRouter()
bool GetTargetPointForRoute(int routeIndex, ref TCoordPoint targetPoint)
int GetRoutingFlags()
IPCB_Primitive GetTargetPrimitiveForRoute(int routeIndex)
TV7_Layer GetCurrentV7LayerFromRouter()
```

### `IPCB_AdvanceRouteParameters`
GUID: `2FB76A8A-FBEC-40EA-A58E-5FE0974E9788`

```csharp
IPCB_Primitive GetSingleRouteStartPrimitive()
uint GetLastHardCommitTimeStamp()
uint GetStartChangeTimeStamp()
IPCB_Group GetMultiRouteStartPrimitives()
int GetMultiRoutesCount()
```

### `IPCB_AccordionMakerSettings`
GUID: `F8D5DAD3-0740-452C-81F9-E20BCEC72BF3`
Configuration object for length tuning (meander insertion):

```csharp
// Target length
TTargetLengthMode GetState_TargetLengthMode()
int GetState_TargetLength()
double GetState_TargetDelay()
bool GetState_UseDelayUnits()

// Meander geometry
TAccordionMode GetState_AccordionMode()
TAccordionStyle GetState_Style()
int GetState_Amplitude()
int GetState_Gap()
int GetState_AmplitudeIncrement()
int GetState_GapIncrement()
bool GetState_ClipToTargetLength()
bool GetState_SingleSide()
bool GetState_RotationSnapping()
double GetState_MitterRadiusRatio()
int GetState_Tolerance()

// Sawtooth-specific
double GetState_SawtoothAngle()
int GetState_SawtoothWidth()
int GetState_SawtoothMinJoint()
int GetState_SawtoothMinHeight()
bool GetState_SawtoothFixedSize()

// Source / rules
IPCB_Net GetState_Net()
IPCB_MaxMinLengthConstraint GetState_LengthRule()
IPCB_MatchedNetLengthsConstraint GetState_MatchedLengthRule()
IPCB_MatchedNetLengthsConstraint GetState_ActualMatchedLengthRule()

// Range / validity
int GetState_MinValidLength()
int GetState_MaxValidLength()
bool GetState_RangeIsValid()
int GetState_MinLength()
int GetState_MaxLength()
int GetState_OldLength()
int GetState_OriginalNetLength()
int GetState_OutputTracesCount()

// Length gauge
bool GetState_ShowLengthGauge()

// Helper operations
void PreviousStyle()
void NextStyle()
void ToggleAmplitudeDirection()
void RecalculateTargetRange()
void InitilizeTargetRange()
void UpdatePrimitive(IPCB_Primitive)
void UpdatePinPairs(IPCB_Primitive)
void UpdatePinPairsNet(IPCB_Net)
long CalculateNetLength(IPCB_Net)
long CalculatePinPairLength(IPCB_PinPair)
double ConvertLengthToDelay_PicoSeconds(int length)
int ConvertDelayToLength_PicoSeconds(double delay)
double ConvertLengthToDelay(int length)
int ConvertDelayToLength(double delay)
string Serialize()
void Deserialize(string data)

// Persistence
void Import_FromParameters(IParameterList)
void Export_ToParameters(IParameterList)
void ImportFrom_SystemOptions()
void ExportTo_SystemOptions()
void ModeParameters_ImportFrom_SystemOptions()
void ModeParameters_ExportTo_SystemOptions()
void SetState_Default()
```

### `IPCB_Accordion` / `IPCB_Accordion2`
The in-board meander/accordion *primitive* (stored as a PCB object). `IPCB_Accordion2` adds:

```csharp
// Geometry
IPCBDM_PolygonalShape GetState_BoundingPolygon()
IPCBDM_PolygonalShape GetState_CentralLine()
TCoordPoint GetState_StartPoint()
TCoordPoint GetState_EndPoint()
IContainer GetState_SegmentsToTune()
IPCB_Primitive GetState_SeedPrimitive()

// Primitive list (the underlying traces)
int PrimListCount()
IPCB_Primitive PrimListAt(int i)
void AddToPrimList(IPCB_Primitive)
void RemovePrimList()

// Status
bool GetState_IsDiffPair()
bool GetState_IsInternal()
bool GetState_Valid()
int GetState_AccordionLength()
int GetState_EstimateLength()
int GetState_ConnecitonLength()   // note: typo "Conneciton" in original

// Rebuild
bool Rebuild()
void UpdateLayer(TV7_Layer)
void UpdateAfterStateChanged()
void ReportLengths(int signalLength, int routedLength)
void ReportLengthsDiffPair(int sigLen, int routedLen, int dpSigLen, int dpRoutedLen)

// Serialization
string GetState_CentralLineData()
string GetState_BoundingPolygonData()
string Serialize()
void Deserialize(string data)
string HashedState()
```

### Via Management

**`IPCB_ViaCombinationManagerInterface`** — manages a stack of `IPCB_RoutingViaStackInfo`
objects (one per layer transition combination):

```csharp
int GetState_ViaStackCount()
IPCB_RoutingViaStackInfo GetState_ViaStack(int index)
void NextStack()
void PrevStack()
IPCB_RoutingViaStackInfo GetState_CurrentStack()
void SetState_CurrentStack(IPCB_RoutingViaStackInfo)
```

**`IPCB_RoutingViaStackInfo`** — one stack = one layer pair combination:

```csharp
int GetState_ViaDataCount()
IPCB_ViaRoutingDataInfo GetState_ViaDataInfo(int index)
string GetState_Title()
IPCB_ViaRoutingDataInfo FirstViaDataInfo()
```

**`IPCB_ViaRoutingDataInfo`** — one via entry within a stack:

```csharp
IPCB_DrillLayerPair GetState_DrillLayerPair()
IPCB_PadViaTemplate GetState_Template()
int GetState_ViaSize()
int GetState_ViaSizeOnLayer(TV7_Layer)
int GetState_HoleSize()
IPCB_LayerStack GetState_LayerStack()
IPCB_RoutingViaStyleRule GetState_Rule()
string GetState_Title()
TV7_Layer GetState_HighLayer()
TV7_Layer GetState_LowLayer()
TDrillLayerPairType GetState_PairType()
TViaType GetState_ViaType()
```

### `IPCB_InteractiveRoutingOptions`
GUID: `4E613AF2-C436-42A4-965E-6BC117FE892B`
Stores historic state of the interactive routing tool (arc and track placement midpoints),
serialized in/out via parameter strings. Fields:

```
PlaceTrackMode, OldTrackDrawLayer, TrackArcX/Y/Radius/Angle1/Angle2,
OldTrackArc*, OldTrackDrawSize, Midx/y, Cx/y, EndLineX/Y, StartX/Y, Beginx/y
```

---

## Routing Rules (IPCB_Rule subinterfaces)

All rule interfaces inherit `IPCB_Rule` and `IPCB_Primitive`. Files in
`Altium.Edp.Interfaces/RT_PCB/`.

| Interface | GUID | Key unique fields |
|---|---|---|
| `IPCB_MaxMinWidthConstraint` | — | `MaxWidth(layer)`, `MinWidth(layer)`, `FavoredWidth(layer)`, per-substack variants, `ImpedanceDriven`, impedance range, `CheckConnectedCopper` |
| `IPCB_DifferentialPairsRoutingRule` | `80F9A031-24B8-4171-B488-D7AEE5B7DF1C` | `MaxGap(layer)`, `MinGap(layer)`, `PreferedGap(layer)`, `MaxUncoupledLength`, width fields per layer, `ImpedanceDriven`, `ImpedanceProfileId` |
| `IPCB_RoutingViaStyleRule` | `A9946BC1-767D-444B-A10E-A4A038829F99` | `MinHoleWidth`, `MaxHoleWidth`, `PreferedHoleWidth`, `MinWidth`, `MaxWidth`, `PreferedWidth`, `ViaStyle`, `UseViaTemplates`, template list |
| `IPCB_RoutingCornerStyleRule` | `179BAE37-6B2E-423F-B8DD-426A5C49B4DC` | `Style` (`TCornerStyle`), `MinSetBack`, `MaxSetBack` |
| `IPCB_RoutingLayersRule` | `FCF35E64-1FF1-4390-8564-EDFE061EAE67` | per-layer allowed routing directions via `TRouteLayer` |
| `IPCB_RoutingTopologyRule` | — | `Topology` (`TNetTopology`) |
| `IPCB_RoutingPriorityRule` | — | routing priority integer |
| `IPCB_RoutingNeckDownRule` | `98EAD02D-F352-4966-96EC-D973A7EA918F` | `MaxLength` (`IPCB_LayerToCoord`) |
| `IPCB_FanoutControlRule` | — | `FanoutStyle`, `FanoutDirection`, `BGAFanoutDirection`, `BGAFanoutViaMode`, `ViaGrid` |
| `IPCB_MatchedNetLengthsConstraint` | — | `Amplitude`, `Gap`, `Style` (`TLengthenerStyle`), `Tolerance`, `DelayTolerance`, `UseDelayUnits`, `TargetSourceName`, `PhaseMatching`, `PhaseTolerance`, `PhaseDelayTolerance`, `PhaseDistance` |
| `IPCB_MaxMinLengthConstraint` | — | length min/max |

The complete list of rule kinds:

```csharp
public enum TRuleKind : byte {
    eRule_Clearance, eRule_ParallelSegment, eRule_MaxMinWidth, eRule_MaxMinLength,
    eRule_MatchedLengths, eRule_DaisyChainStubLength, eRule_PowerPlaneConnectStyle,
    eRule_RoutingTopology, eRule_RoutingPriority, eRule_RoutingLayers,
    eRule_RoutingCornerStyle, eRule_RoutingViaStyle, eRule_PowerPlaneClearance,
    eRule_SolderMaskExpansion, eRule_PasteMaskExpansion, eRule_ShortCircuit,
    eRule_BrokenNets, eRule_ViasUnderSMD, eRule_MaximumViaCount,
    eRule_MinimumAnnularRing, eRule_PolygonConnectStyle, eRule_AcuteAngle,
    eRule_ConfinementConstraint, eRule_SMDToCorner, eRule_ComponentClearance,
    eRule_ComponentRotations, eRule_PermittedLayers, eRule_NetsToIgnore,
    eRule_SignalStimulus, eRule_Overshoot_FallingEdge, eRule_Overshoot_RisingEdge,
    eRule_Undershoot_FallingEdge, eRule_Undershoot_RisingEdge, eRule_MaxMinImpedance,
    eRule_SignalTopValue, eRule_SignalBaseValue, eRule_FlightTime_RisingEdge,
    eRule_FlightTime_FallingEdge, eRule_LayerStack, eRule_MaxSlope_RisingEdge,
    eRule_MaxSlope_FallingEdge, eRule_SupplyNets, eRule_MaxMinHoleSize,
    eRule_TestPointStyle, eRule_TestPointUsage, eRule_UnconnectedPin,
    eRule_SMDToPlane, eRule_SMDNeckDown, eRule_LayerPair, eRule_FanoutControl,
    eRule_MaxMinHeight, eRule_DifferentialPairsRouting, eRule_HoleToHoleClearance,
    eRule_MinimumSolderMaskSliver, eRule_SilkToSolderMaskClearance,
    eRule_SilkToSilkClearance, eRule_NetAntennae, eRule_AssyTestPointStyle,
    eRule_AssyTestPointUsage, eRule_SilkToBoardRegion, eRule_SMDPADEntry,
    eRule_None, eRule_ModifiedPolygon, eRule_BoardOutlineClearance,
    eRule_BackDrilling, eRule_Creepage, eRule_ReturnPath, eRule_RoutingNeckDown,
    eRule_Wirebonding, eRule_ZAxisClearance
}
```

---

## Data Structures

### Routing-relevant enums (namespace `RT_PCB`)

```csharp
// Obstacle handling mode (the "Conflict Resolution" mode)
enum TAdvancedRouteMode : byte {
    eARIgnoreObstacle = 0,
    eARWalkAroundObstacle = 1,
    eARPushObstacle = 2,
    eARHugAndPushObstacle = 3,
    eARStopAtFirstObstacle = 4,
    eARAutoRouteCurrentLayer = 5,
    eARAutoRouteMultiLayer = 6
}

// Legacy simple routing mode (stored in IPCB_SystemOptions)
enum TInteractiveRouteMode : byte {
    eIgnoreObstacle = 0,
    eAvoidObstacle = 1,
    ePushObstacle = 2
}

// Smart route mode (for smart auto-complete)
enum TSmartRouteMode : byte {
    eSRIgnoreObstacle = 0,
    eSRAvoidObstacle = 1,
    eSRWalkAroundObstacle = 2,
    eSRPushObstacle = 3
}

// Track width selection
enum TRoutingWidthMode : byte {
    eRoutingWidth_Default = 0,
    eRoutingWidth_Min = 1,
    eRoutingWidth_Preferred = 2,
    eRoutingWidth_Max = 3
}

// Corner style during routing
enum TRoutingCornerStyle : byte {
    eRoutingCornerStyle_90 = 0,
    eRoutingCornerStyle_45 = 1,
    eRoutingCornerStyle_Any = 2
}

// Persisted corner style in rules
enum TCornerStyle : byte {
    eCornerStyle_90 = 0,
    eCornerStyle_45 = 1,
    eCornerStyle_Round = 2
}

// Diff pair gap selection
enum TRoutingDiffPairGapMode : byte {
    eRoutingDiffPairGap_Min = 0,
    eRoutingDiffPairGap_Preferred = 1,
    eRoutingDiffPairGap_Max = 2
}

// Gloss effort (how hard to optimize corners after routing)
enum TGlossEffort : byte {
    eGlossEffort_None = 0,
    eGlossEffort_Weak = 1,
    eGlossEffort_Strong = 2
}

// Hugging corner shape
enum THuggingStyle : byte {
    eStyleMixed = 0,
    eStyleRounded = 1,
    eStyleDegrees = 2
}

// Vertex drag action
enum TVertexAction : byte {
    eDeform = 0,
    eScale = 1,
    eSmooth = 2
}

// Via type selection
enum TRouteVia : byte {
    eViaThruHole = 0,
    eViaBlindBuriedPair = 1,
    eViaBlindBuriedAny = 2,
    eViaNone = 3
}

// Layer routing direction for Routing Layers rule
enum TRouteLayer : byte {
    eRLLayerNotUsed = 0,
    eRLRouteHorizontal = 1,
    eRLRouteVertical = 2,
    eRLRouteSingleLayer = 3,
    eRLRoute_1_OClock = 4,
    eRLRoute_2_OClock = 5,
    eRLRoute_4_OClock = 6,
    eRLRoute_5_OClock = 7,
    eRLRoute_45_Up = 8,
    eRLRoute_45_Down = 9,
    eRLRouteFanout = 10,
    eRLRouteAuto = 11
}

// Net topology for Routing Topology rule
enum TNetTopology : byte {
    eNetTopology_Shortest = 0,
    eNetTopology_Horizontal = 1,
    eNetTopology_Vertical = 2,
    eNetTopology_DaisyChain_Simple = 3,
    eNetTopology_DaisyChain_MidDriven = 4,
    eNetTopology_DaisyChain_Balanced = 5,
    eNetTopology_Starburst = 6
}

// Fanout style
enum TFanoutStyle : byte {
    eFanoutStyle_Auto = 0,
    eFanoutStyle_Rows = 1,
    eFanoutStyle_Staggered = 2,
    eFanoutStyle_BGA = 3,
    eFanoutStyle_UnderPads = 4
}

// Fanout direction
enum TFanoutDirection : byte {
    eFanoutDirection_None = 0,
    eFanoutDirection_InOnly = 1,
    eFanoutDirection_OutOnly = 2,
    eFanoutDirection_InThenOut = 3,
    eFanoutDirection_OutThenIn = 4,
    eFanoutDirection_Alternating = 5
}

// BGA fanout direction
enum TBGAFanoutDirection : byte {
    eBGAFanoutDirection_Out = 0,
    eBGAFanoutDirection_NE = 1,
    eBGAFanoutDirection_SE = 2,
    eBGAFanoutDirection_SW = 3,
    eBGAFanoutDirection_NW = 4,
    eBGAFanoutDirection_In = 5
}

// BGA fanout via placement mode
enum TBGAFanoutViaMode : byte {
    eBGAFanoutVia_Closest = 0,
    eBGAFanoutVia_Centered = 1
}

// Obstacle avoidance during dragging
enum TAvoidObstacleMode : byte {
    avIgnore = 0,
    avAvoidSnap = 1,
    avAvoid = 2
}

// Component push during routing
enum TPushMode : byte {
    eIgnoreOthers = 0,
    eStopFirst = 1,
    ePushStopLocked = 2,
    ePushIgnoreLocked = 3
}

// Net ratsnest line mode
enum TNetLineMode : byte {
    NetLineMode_Off = 0,
    NetLineMode_PadToPad = 1,
    NetLineMode_Reconnection = 2
}

// Track placement corner mode (for interactive options)
enum TPlaceTrackMode : byte {
    ePlaceTrackNone = 0,
    ePlaceTrackAny = 1,
    ePlaceTrack9090 = 2,
    ePlaceTrack4590 = 3,
    ePlaceTrack90Arc = 4
}

// Accordion/meander tuning pattern type
enum TAccordionMode : byte {
    eAccordionMode_Accordion = 0,  // standard meander
    eAccordionMode_Trombone = 1,
    eAccordionMode_Sawtooth = 2,
    eAccordionMode_Root = 3
}

// Accordion corner style
enum TAccordionStyle : byte {
    asMittered45DegreeLines = 0,
    asMitteredArcs = 1,
    asRounded = 2
}

// Length tuning target source
enum TTargetLengthMode : byte {
    eTargetLength_Manual = 0,
    eTargetLength_FromNet = 1,
    eTargetLength_FromRules = 2,
    eTargetLength_FromDiffPairs = 3
}

// Lengthener / meander style stored in MatchedNetLengthsConstraint rule
enum TLengthenerStyle : byte {
    eLengthenerStyle_90 = 0,
    eLengthenerStyle_45 = 1,
    eLengthenerStyle_Round = 2,
    eLengthenerStyle_Mitered90 = 3
}

// Net classification for accordion tuner
enum TNetClassification : byte {
    eDiffPairNet = 0,
    eSameLengthRule = 1,
    eSameMatchedLengthRule = 2,
    eSameNetClass = 3,
    eOtherBoardNets = 4
}
```

---

## Configuration / Settings

### `IPCB_InteractiveRoutingOptions` (persisted per-session state)
Serialized to/from parameter strings via `Import_FromParameters` / `Export_ToParameters`.
Versioned: Version3 and Version4 variants exist.

Tracks:
- `PlaceTrackMode` — current corner mode
- Old and new arc geometry (x, y, radius, angle1, angle2)
- Start / end / mid / begin point coordinates

### `IPCB_BoardRoutingOptions`
Board-level routing layer enable/disable mask. Accessed via key/value pairs:

```csharp
bool GetState_UseLayer(uint layerId)
bool GetState_ShowSignalLayersOnly()
string GetState_Value(string key)
```

### Specctra Autorouter options (`IPCB_SpecctraRouterOptions`)
Full cost/tax matrix for the batch autorouter. Not part of active routing but stored in the
same PCB options system. Key settings: `WireGrid`, `ViaGrid`, `RoutePasses`, `CleanPasses`,
`FilterPasses`, `LayerCost(layer)`, `LayerWWCost(layer)`, `ViaCost`, `WwCost`, `CrossCost`,
fanout control flags.

---

## Routing Modes / Algorithms

### Conflict Resolution (Active Routing)

Stored in `IPCB_RoutingOptionsPage.ConflictMode` and per-mode enable flags.
The `TAdvancedRouteMode` enum governs how the active router handles obstacles:

| Value | Meaning |
|---|---|
| `eARIgnoreObstacle` | Route through everything, create DRC violations |
| `eARWalkAroundObstacle` | Navigate around obstacles without disturbing them |
| `eARPushObstacle` | Push other traces out of the way |
| `eARHugAndPushObstacle` | Hug obstacles tightly, push if needed |
| `eARStopAtFirstObstacle` | Stop routing when obstacle encountered |
| `eARAutoRouteCurrentLayer` | Hand off to single-layer autorouter for current segment |
| `eARAutoRouteMultiLayer` | Hand off to multi-layer autorouter |

The user cycles through enabled modes interactively; `IPCB_CustomInteractiveRoutingProcess.
GetState_RouteModeEnabled(mode)` controls which modes appear in the cycle.

### Length Tuning (Accordion/Meander)

Four tuning patterns (`TAccordionMode`):
- **Accordion** — standard back-and-forth meander
- **Trombone** — single-direction extension
- **Sawtooth** — diagonal teeth pattern (configurable angle, width, joint/height min)
- **Root** — internal mode (exact semantics unclear from C# layer)

Three corner styles (`TAccordionStyle`):
- `asMittered45DegreeLines` — standard 45-degree chamfer
- `asMitteredArcs` — arc-filleted miters
- `asRounded` — smooth curves

Target length can come from: manual entry, net length, design rules
(`MatchedNetLengths` / `MaxMinLength` rules), or the other net in a diff pair
(`TTargetLengthMode`).

### Gloss (Post-Route Optimization)

After placing each segment the router "glosses" nearby traces. Controlled by:
- `TGlossEffort`: `None` / `Weak` / `Strong`
- `NeighborGlossEffort`: separate setting for neighboring traces

### Sliding

`pidPcbInteractiveSliding` / `IPCB_SlidingRoutingProcess`: slides one or more existing trace
segments while maintaining connections at both ends. Uses `TVertexAction` to control vertex
handling: `eDeform` / `eScale` / `eSmooth`.

### Diff-Pair Routing

Handled by `IPCB_InteractiveDiffPairRoutingProcess`. Controls gap between the two conductors
via `TRoutingDiffPairGapMode` (Min/Preferred/Max from `IPCB_DifferentialPairsRoutingRule`).
The rule stores per-layer width and gap ranges plus `MaxUncoupledLength`.

### Bus/Multi Routing

`IPCB_InteractiveMultiRoutingProcess` — routes multiple nets simultaneously. Has an
additional `BusSpacing` parameter (uniform spacing between bus members).

### Specctra Batch Autorouter

Separate from interactive routing. Configured via `IPCB_SpecctraRouterOptions` and invoked
separately. Not part of the "active router" flow.

---

## Schematic Router (Layer2 / Auto-Route Wires)

Distinct from PCB routing. Located in `Altium.Sch.Layer2Base/`.

Interface: `IRouter` with three methods:

```csharp
void Route()
List<Point> CreateRoute(Point src, Orientations srcOri, Point dst, Orientations dstOri, TObjectId connectionType)
void RegisterTransformedObject(Layer2Id id)
```

`Router` (abstract base class) implements orthogonal obstacle-avoiding wire routing for
schematic connections. Algorithm: `RouterPathSearch` using an obstacle grid
(`ObstacleBuilder`). Points are snapped to schematic grid before routing.

---

## Observations / Open Questions

1. **"Active Router" name**: The marketing name "ActiveRoute" refers to the *interactive*
   routing tool, not a distinct software component. It is the combination of
   `IPCB_InteractiveRoutingProcess` + `TAdvancedRouteMode::eARAutoRouteCurrentLayer/MultiLayer`
   (the "auto-complete" sub-mode). There is no separate `IActiveRouter` interface.

2. **Algorithm location**: All pathfinding is in undecompiled Delphi DLLs (likely
   `Routing.dll` or `PCB.dll`). The C# interfaces are thin COM wrappers for UI/Script access.
   Use `ghidra` on the Altium PCB Delphi DLLs to reverse the actual algorithms.

3. **`IPCB_Accordion.GetState_ConnecitonLength()`**: Typo in the original — "Conneciton"
   (missing 't'). This appears in both `IPCB_Accordion` and `IPCB_Accordion2`.

4. **`TInteractiveRouteModeConsts.First/Last`**: The "simple" `TInteractiveRouteMode`
   (3 values) appears to be an older/legacy mode stored in `IPCB_SystemOptions`. The
   more modern interactive routing uses `TAdvancedRouteMode` (7 values). It's unclear if
   they map 1:1 or coexist.

5. **`IPCB_RoutingNeckDownRule`**: The rule only exposes `GetState_MaxLength()` as its
   unique property (returns `IPCB_LayerToCoord`). The actual neckdown width seems to come
   from a separate width rule applied in the neckdown context, not from this rule.

6. **`DoNotUse_*` methods**: Several methods in `IPCB_Accordion_SaveLoadParameters` are
   explicitly marked `DoNotUse_` with an ordinal suffix (e.g. `DoNotUse_GetState_NetName_14`).
   These are deprecated COM slot placeholders that must not be removed (would break binary
   compatibility) but must not be called.

7. **Accordion serialization**: `IPCB_Accordion2.Serialize()` / `Deserialize()` and
   `HashedState()` are used for PCB file save/load. The serialized form also appears in
   `GetState_CentralLineData()` and `GetState_BoundingPolygonData()`. These need
   investigation when implementing accordion read/write.

8. **`RoutingTopologyType.Custom`** (in `ConstraintsManager.Contracts`) has no equivalent in
   `TNetTopology` (which only goes to `Starburst`). The UI layer adds a `Custom` entry
   beyond what the Delphi enum defines — likely handled via a `RoutingTopologyCustomData`
   payload object.

9. **`IPCB_AdvancedPlacerOptions`**: Component-level fanout placer options referenced from
   `IPCB_Board`. Not a routing interface per se but related to pre-routing fanout placement.
