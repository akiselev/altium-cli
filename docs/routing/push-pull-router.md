# Altium Interactive Router — Push-Pull and Conflict Resolution

Reverse-engineered reference for the Altium PCB interactive routing system. All identifiers are from the decompiled C# source in `AD26-dotnet/`.

---

## Overview

Altium's interactive router is a Delphi/COM subsystem exposed to .NET via `RT_PCB` and `PCBInterfaces` namespaces. There is no single "PushPullRouter" class — the concept is spread across:

- **Conflict resolution mode** (`TAdvancedRouteMode`) — per-session setting that controls what happens when the route path hits an obstacle.
- **Process objects** — one COM interface per routing mode (single route, diff-pair, multi-route, via-drag, slide, length-tuning). All share a common base (`IPCB_CustomInteractiveRoutingProcess`).
- **Options page** (`IPCB_RoutingOptionsPage`) — persistent settings repository that the process objects read/write.
- **System options** (`IPCB_SystemOptions`) — legacy `TInteractiveRouteMode` (3-value version, not the current 7-value enum).

There is no evidence of a class or type literally named "PushPull" in the C# code. The user-facing label "HugNPush Obstacles" (from the `.resx`) maps to `TAdvancedRouteMode.eARHugAndPushObstacle`.

---

## Key Interfaces

### `IPCB_CustomInteractiveRoutingProcess`

Base for all interactive routing process COM objects.

File: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_CustomInteractiveRoutingProcess.cs`
GUID: `A1165DEC-03A4-462C-ADD5-8AB5E940C187`
Namespace: `RT_PCB`

Core routing-mode methods (all on the base):

```csharp
TAdvancedRouteMode GetState_RouteMode();
void SetState_RouteMode(TAdvancedRouteMode argValue);
bool GetState_RouteModeEnabled(TAdvancedRouteMode argMode);
void SetState_RouteModeEnabled(TAdvancedRouteMode argMode, bool argValue);

bool GetState_AllowViaPushing();
void SetState_AllowViaPushing(bool argValue);

bool GetState_AutoRemoveLoops();
void SetState_AutoRemoveLoops(bool argValue);

bool GetState_AutoRemoveAntennas();
void SetState_AutoRemoveAntennas(bool argValue);

bool GetState_AutoTerminateRouting();
void SetState_AutoTerminateRouting(bool argValue);

bool GetState_FollowMode();          // diff-pair follow mode
bool GetState_LegacyRouter();        // true = "Quick Route" (legacy 2-layer router)

TGlossEffort GetState_GlossEffort();
void SetState_GlossEffort(TGlossEffort argValue);

TGlossEffort GetState_NeighborGlossEffort();
void SetState_NeighborGlossEffort(TGlossEffort argNewValue);

THuggingStyle GetState_HuggingStyle();
void SetState_HuggingStyle(THuggingStyle argNewValue);

double GetState_MiterSize();
void SetState_MiterSize(double argNewValue);

double GetState_MinimumArcSize();
void SetState_MinimumArcSize(double argValue);

bool GetState_AutoNecking();
void SetState_AutoNecking(bool argValue);

IPCB_RoutingOptionsPage GetRoutingOptions();
```

Corner/width:

```csharp
TRoutingCornerStyle GetState_RoutingCornerStyle();
void SetState_RoutingCornerStyle(TRoutingCornerStyle argValue);
bool GetState_RestrictTo9045();
void SetState_RestrictTo9045(bool argValue);

TRoutingWidthMode GetState_RoutingWidthMode();
void SetState_RoutingWidthMode(TRoutingWidthMode argValue);
TRoutingWidthMode GetTrackWidthMode(bool argCheckPickupTrackWidth);
bool GetState_PickupWidthFromExistingRoutes();

int GetState_Width();
void SetState_Width(int argValue);
```

Via:

```csharp
int GetState_ViaDiameter();
int GetState_HoleSize();
int GetState_ViaLayerPair();
TRoutingWidthMode GetState_ViaSizeMode();
IPCB_PadViaTemplate GetState_ViaTemplate();
IPCB_ViaCombinationManagerInterface GetState_ViaCombinationManager();
```

### `IPCB_InteractiveRoutingProcess`

Extends `IPCB_CustomInteractiveRoutingProcess`. Adds:

File: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_InteractiveRoutingProcess.cs`
GUID: `B52BE2E5-F20C-4E12-9AA8-E2EEC4FA1EFB`

```csharp
IPCB_DifferentialPairsRoutingRule GetState_DiffPairRule();
IPCB_DifferentialPair GetState_DifferentialPair();
bool GetState_ShowLengthGauge();
void SetState_ShowLengthGauge(bool argValue);
bool ProjectAvailableForPinSwap(out string argErrorMessage);
void EditMaxMinWidthRule();
IPCB_MaxMinWidthConstraint GetState_WidthRule();
double GetState_Impedance();
```

### `IPCB_InteractiveDiffPairRoutingProcess`

GUID: `44F1F1CC-C349-4709-BCB7-2BC3DD2B3538`

Adds diff-pair-specific methods on top of `IPCB_CustomInteractiveRoutingProcess`:

```csharp
IPCB_DifferentialPair GetState_DifferentialPair();
TRoutingDiffPairGapMode GetState_GapMode();
TRoutingDiffPairGapMode NextGapMode();
void SetState_GapMode(TRoutingDiffPairGapMode argValue);
IPCB_DifferentialPairsRoutingRule GetState_DiffPairRule();
```

### `IPCB_InteractiveMultiRoutingProcess`

GUID: `353560E5-B456-4E39-A483-7701846693F0`

Adds multi-route (bus routing) methods:

```csharp
IPCB_MaxMinWidthConstraint GetState_WidthRule();
int GetState_BusSpacing();
void SetState_BusSpacing(int argNewValue);
int GetState_GetMinClearanceFromRule();
void EditMaxMinWidthRule();
```

### `IPCB_SlidingRoutingProcess`

Track/segment sliding (drag). GUID: `78855205-FEB0-4A9D-A706-2D523DFA6F91`

Key property: uses `TAdvancedRouteMode` for its `Sliding` mode (same enum as routing conflict resolution):

```csharp
TAdvancedRouteMode GetState_Sliding();
void SetState_Sliding(TAdvancedRouteMode argNewValue);

TVertexAction GetState_VertexAction();
void SetState_VertexAction(TVertexAction argNewValue);

bool GetState_AllowViaPusing();  // note: typo in source
THuggingStyle GetState_HuggingStyle();
TGlossEffort GetState_GlossEffort();
double GetState_MiterSize();
double GetState_MinimumArcSize();
```

### `IPCB_RoutingOptionsPage`

Persistent routing preferences. GUID: `2131E587-7CA7-4E27-B01C-7C4D5CC9FFFE`

File: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_RoutingOptionsPage.cs`

```csharp
// Conflict resolution (same TAdvancedRouteMode enum)
TAdvancedRouteMode GetState_ConflictMode();
void SetState_ConflictMode(TAdvancedRouteMode argValue);
bool GetState_ConflictModeEnabled(TAdvancedRouteMode argMode);
void SetState_ConflictModeEnabled(TAdvancedRouteMode argMode, bool argValue);

// Component interaction during routing
TPushMode GetState_ComponentPushing();
void SetState_ComponentPushing(TPushMode argValue);
bool GetState_ComponentReroute();
bool GetState_ComponentMoveRelevantRouting();
int GetState_ComponentMoveRelevantRoutingPinsLimit();
TNetLineMode GetState_ComponentNetLineMode();

// Drag/slide settings
bool GetState_SmartDrag();
TAvoidObstacleMode GetState_DragAvoidObstacleMode();
void SetState_DragAvoidObstacleMode(TAvoidObstacleMode argValue);
TDragSelectUnselectMode GetState_DragUnselected();
TDragSelectUnselectMode GetState_Dragselected();
TVertexAction GetState_VertexAction();

// Loop/antenna cleanup
bool GetState_AutoRemoveLoops();
bool GetState_AutoRemoveViaLoops();
bool GetState_AutoRemoveAntennas();
bool GetState_AutoTerminateRouting();

// Via pushing
bool GetState_AllowViaPushing();

// Display
bool GetState_DisplayClearanceBounds();
bool GetState_ReduceClearanceDisplayArea();

// Width/clearance
bool GetState_PickupWidthFromExistingRoutes();
TRoutingWidthMode GetState_RoutingWidthMode();
TRoutingWidthMode GetState_ViaSizeMode();
double GetState_PreferredClearanceRatio();
bool GetState_PreferredClearanceApply();
bool GetState_PreferredClearanceAdjustVias();
int GetState_WidthToUse();
int GetState_DiffPairGapToUse();

// Hugging / glossing
TGlossEffort GetState_GlossEffort();
TGlossEffort GetState_NeighborGlossEffort();
THuggingStyle GetState_HuggingStyle();
double GetState_MinimumArcSize();
double GetState_MiterSize();
int GetState_PadEntryStability();

// Diff pair
bool GetState_DifferentialPairMode();
bool GetState_DifferentialPairModeForVias();

// Drag miters
bool GetState_DragWithMiters();
bool GetState_DragMergeParallel();

// Obstacle avoidance for polygon/room
bool GetState_AvoidPolygons();
bool GetState_AvoidRooms();
bool GetState_PreservePath();

// Misc
bool GetState_ShowGaugeDuringDragging();
bool GetState_DisableTraceCenteringWhenDragging();
bool GetState_AutoNecking();
```

### `IPCB_AdvanceRouteCommands`

Low-level commands the router calls on the board during routing.

File: `AD26-dotnet/Altium.Edp.Interfaces/PCBInterfaces/IPCB_AdvanceRouteCommands.cs`
GUID: `F0831499-190D-4429-8B4A-6803D583FC7E`

```csharp
void AddPrimitiveToBoard(IPCB_Primitive argBackConnectedPrim, IPCB_Primitive argPrimToAdd, IPCB_Primitive argForwardConnectedPrim);
void RemovePrimitiveFromBoard(IPCB_Primitive argP);
void ReplaceConnectedPrimitivesInBoard(IPCB_Group argPrimitivesToRemove, IPCB_Group argPrimitivesToAdd);

IPCB_Group GetRoutedPath();
bool IsPushablePrimitive(IPCB_Primitive argP);   // key for push logic

void SetState_BackConnectedPrim(IPCB_Primitive argPrimitive, IPCB_Primitive argBackPrimitive);
IPCB_Primitive GetState_BackConnectedPrim(IPCB_Primitive argPrimitive);
void SetState_ForwardConnectedPrim(IPCB_Primitive argPrimitive, IPCB_Primitive argForwardPrimitive);
IPCB_Primitive GetState_ForwardConnectedPrim(IPCB_Primitive argPrimitive);

int GetWidthFromRouter(IPCB_Primitive argPrimitive);
bool IsUsingAlternativeTargets();
TV6_Layer GetCurrentLayerFromRouter();
bool GetTargetPointForRoute(int argRouteIndex, ref TCoordPoint argTargetPoint);
int GetRoutingFlags();
IPCB_Primitive GetTargetPrimitiveForRoute(int argRouteIndex);
TV7_Layer GetCurrentV7LayerFromRouter();
```

`IsPushablePrimitive` is the predicate the router uses to decide whether a given object can be displaced by push-obstacle mode.

---

## Conflict Resolution Modes

### `TAdvancedRouteMode` (the current enum — 7 values)

File: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TAdvancedRouteMode.cs`
Namespace: `RT_PCB`

| Enum value | UI label (from `.resx`) | Meaning |
|---|---|---|
| `eARIgnoreObstacle` (0) | "Ignore Obstacles" | Route freely, DRC violations accepted |
| `eARWalkAroundObstacle` (1) | "Walkaround Obstacles" | Detour around fixed obstacles, never push |
| `eARPushObstacle` (2) | "Push Obstacles" | Push existing tracks out of the way |
| `eARHugAndPushObstacle` (3) | "HugNPush Obstacles" | Hug the obstacle contour, then push if needed |
| `eARStopAtFirstObstacle` (4) | "Stop At First Obstacle" | Stop routing when any obstacle is hit |
| `eARAutoRouteCurrentLayer` (5) | "AutoRoute Current Layer" | Invoke autorouter on current layer |
| `eARAutoRouteMultiLayer` (6) | "AutoRoute MultiLayer" | Invoke autorouter with layer changes |

**Quick Route (legacy router)** only exposes the first four modes. When `GetState_LegacyRouter()` returns `true`, the UI removes `eARStopAtFirstObstacle`, `eARAutoRouteCurrentLayer`, and `eARAutoRouteMultiLayer` from the available options:

```csharp
// BasePcbRoutingInteractiveProcessDataObject.GetEnabledRoutingModes()
var list = RoutingHelper.GetEnabledRoutingConflictResolutions(options);
if (IsQuickRouting) {
    list.Remove(TAdvancedRouteMode.eARStopAtFirstObstacle);
    list.Remove(TAdvancedRouteMode.eARAutoRouteCurrentLayer);
    list.Remove(TAdvancedRouteMode.eARAutoRouteMultiLayer);
}
```

`IsQuickRouting` is `GetState_LegacyRouter()`.

The **shortcut** to cycle through conflict modes is **Shift+R** (`Shortcut_ConflictResolution` resource key).

### `TInteractiveRouteMode` (legacy/system options — 3 values)

File: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TInteractiveRouteMode.cs` (also in `Pcbtypes/` and `PCB/`)

| Enum value | Meaning |
|---|---|
| `eIgnoreObstacle` (0) | Ignore obstacles |
| `eAvoidObstacle` (1) | Avoid obstacles |
| `ePushObstacle` (2) | Push obstacles |

This is the older 3-value version stored in `IPCB_SystemOptions.GetState_InteractiveRouteMode()`. The modern routing processes use `TAdvancedRouteMode` instead.

### `TSmartRouteMode` (4 values — intermediate version)

File: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TSmartRouteMode.cs`

| Enum value | Meaning |
|---|---|
| `eSRIgnoreObstacle` (0) | Ignore |
| `eSRAvoidObstacle` (1) | Avoid |
| `eSRWalkAroundObstacle` (2) | Walk around |
| `eSRPushObstacle` (3) | Push |

This appears to be a version between `TInteractiveRouteMode` (3-value) and `TAdvancedRouteMode` (7-value). Referenced only in `TSmartRouteModeConsts`.

---

## Additional Enums

### `TPushMode` — component pushing during routing

File: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TPushMode.cs`

| Value | Meaning |
|---|---|
| `eIgnoreOthers` (0) | Don't push components |
| `eStopFirst` (1) | Stop at first component collision |
| `ePushStopLocked` (2) | Push unlocked components, stop at locked |
| `ePushIgnoreLocked` (3) | Push all components regardless of lock |

Used by `IPCB_RoutingOptionsPage.GetState_ComponentPushing()`.

### `TAvoidObstacleMode` — obstacle avoidance for drag operations

File: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TAvoidObstacleMode.cs`

| Value | Meaning |
|---|---|
| `avIgnore` (0) | Ignore obstacles during drag |
| `avAvoidSnap` (1) | Avoid with snap behavior |
| `avAvoid` (2) | Full avoidance |

Used by `IPCB_RoutingOptionsPage.GetState_DragAvoidObstacleMode()`.

### `TGlossEffort` — post-routing glossing/smoothing

File: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TGlossEffort.cs`

| Value | Meaning |
|---|---|
| `eGlossEffort_None` (0) | No glossing after routing |
| `eGlossEffort_Weak` (1) | Light smoothing pass |
| `eGlossEffort_Strong` (2) | Aggressive smoothing |

Two separate gloss effort settings exist: `GlossEffort` (the routed trace) and `NeighborGlossEffort` (adjacent existing traces displaced by push).

### `THuggingStyle` — how pushed traces hug obstacles

File: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/THuggingStyle.cs`

| Value | Meaning |
|---|---|
| `eStyleMixed` (0) | Mix of arcs and straight segments |
| `eStyleRounded` (1) | Arc-based hugging |
| `eStyleDegrees` (2) | Angle-based (45°/90°) hugging |

### `TRoutingCornerStyle` — trace corner geometry

File: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TRoutingCornerStyle.cs`

| Value | Meaning |
|---|---|
| `eRoutingCornerStyle_90` (0) | Right-angle corners |
| `eRoutingCornerStyle_45` (1) | 45° chamfered corners |
| `eRoutingCornerStyle_Any` (2) | Any angle (arc-based) |

### `TRoutingWidthMode` — track width selection

File: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TRoutingWidthMode.cs`

| Value | Meaning |
|---|---|
| `eRoutingWidth_Default` (0) | Use rule default |
| `eRoutingWidth_Min` (1) | Use rule minimum |
| `eRoutingWidth_Preferred` (2) | Use rule preferred |
| `eRoutingWidth_Max` (3) | Use rule maximum |

### `TRoutingDiffPairGapMode` — diff-pair gap selection

File: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TRoutingDiffPairGapMode.cs`

| Value | Meaning |
|---|---|
| `eRoutingDiffPairGap_Min` (0) | Minimum gap |
| `eRoutingDiffPairGap_Preferred` (1) | Preferred gap |
| `eRoutingDiffPairGap_Max` (2) | Maximum gap |

### `TVertexAction` — sliding vertex behavior

File: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TVertexAction.cs`

| Value | Meaning |
|---|---|
| `eDeform` (0) | Deform segment at vertex |
| `eScale` (1) | Scale surrounding segments |
| `eSmooth` (2) | Smooth transition |

### `TRouteHugging` — hugging direction during obstacle navigation

File: `AD26-dotnet/Altium.Edp.Interfaces/xPCBTypes/TRouteHugging.cs`

| Value | Meaning |
|---|---|
| `eRouteNoHug` (0) | No hugging |
| `eRouteHug` (1) | Hug the obstacle boundary |
| `eRouteSpread` (2) | Spread away from obstacle |

### `TPlaceTrackMode` — legacy track placement mode

File: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TPlaceTrackMode.cs`

| Value | Meaning |
|---|---|
| `ePlaceTrackNone` (0) | No mode |
| `ePlaceTrackAny` (1) | Any angle |
| `ePlaceTrack9090` (2) | 90°/90° |
| `ePlaceTrack4590` (3) | 45°/90° |
| `ePlaceTrack90Arc` (4) | 90° with arcs |

### `TAccordionMode` — length tuning meander style

File: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TAccordionMode.cs`

| Value | Meaning |
|---|---|
| `eAccordionMode_Accordion` (0) | Accordion pattern |
| `eAccordionMode_Trombone` (1) | Trombone (U-turns) |
| `eAccordionMode_Sawtooth` (2) | Sawtooth wave |
| `eAccordionMode_Root` (3) | Root/base mode |

### `TDragSelectUnselectMode` — drag behavior for selected/unselected components

| Value | Meaning |
|---|---|
| `dmDrag` (0) | Drag with connected traces |
| `dmMove` (1) | Move without trace following |

### `TNetLineMode` — ratsnest display during component move

| Value | Meaning |
|---|---|
| `NetLineMode_Off` (0) | No ratsnest |
| `NetLineMode_PadToPad` (1) | Pad-to-pad ratsnest |
| `NetLineMode_Reconnection` (2) | Reconnection lines |

---

## Interactive Process Taxonomy

All processes implement `IInteractiveProcess` (base) via `IPCB_InteractiveProcess` and are identified by `TInteractiveProcessId`:

File: `AD26-dotnet/Altium.Edp.Interfaces/RT_InteractiveProcess/TInteractiveProcessId.cs`

| Enum value | Process type | Interface |
|---|---|---|
| `pidUndefined` (0) | N/A | — |
| `pidPcbInteractiveRouting` (1) | Single-net interactive routing | `IPCB_InteractiveRoutingProcess` |
| `pidPcbInteractiveLengthTuning` (2) | Length tuning (meanders) | — |
| `pidPcbInteractiveDiffPairRouting` (3) | Differential pair routing | `IPCB_InteractiveDiffPairRoutingProcess` |
| `pidPcbInteractiveMultiRouting` (4) | Multi-net bus routing | `IPCB_InteractiveMultiRoutingProcess` |
| `pidPcbInteractiveDiffPairLengthTuning` (5) | Diff-pair length tuning | — |
| `pidPcbInteractiveLineRouting` (6) | Line (keepout/mechanical) routing | `IPCB_InteractiveLineRoutingProcess` |
| `pidPcbInteractiveSliding` (7) | Track/segment sliding | `IPCB_SlidingRoutingProcess` |
| `pidPcbInteractiveViaDragging` (8) | Via dragging | — |

### `TRoutingOptionsParentKind` — which router owns an options block

File: `AD26-dotnet/Altium.Edp.Interfaces/xPCBTypes/TRoutingOptionsParentKind.cs`

| Value | Owner |
|---|---|
| `ePCBPreferences` (0) | PCB global preferences |
| `eInteractiveRouter` (1) | Single-net interactive router |
| `eSmartRouter` (2) | Smart (autorouter variant) |
| `eMultiRouter` (3) | Multi-route bus router |
| `eDrag` (4) | Drag/slide process |
| `eDragVia` (5) | Via drag process |
| `eDiffPairRouter` (6) | Diff-pair router |

---

## Data Structures

### `IPCB_RoutingOptionsPage` (persistent settings)

This is the canonical store for all per-process routing preferences. Retrieved via `IPCB_CustomInteractiveRoutingProcess.GetRoutingOptions()`. The process objects read their initial state from this page and write back to it.

The `BasePcbRoutingInteractiveProcessDataObject` C# class (the MVVM data object wrapping a process) holds a reference to `IPCB_RoutingOptionsPage options` and exposes all settings as C# properties. The key routing-mode property:

```csharp
public TAdvancedRouteMode RouteMode {
    get => ParentObject.GetState_RouteMode();
    set => ParentObject.SetState_RouteMode(value);
}
```

### `IPCB_AdvanceRouteParameters`

Per-routing-session metadata:

```csharp
IPCB_Primitive GetSingleRouteStartPrimitive();
uint GetLastHardCommitTimeStamp();
uint GetStartChangeTimeStamp();
IPCB_Group GetMultiRouteStartPrimitives();
int GetMultiRoutesCount();
```

### `IPCB_AccordionMakerSettings`

Length-tuning session state. Stores amplitude, gap, style (`TAccordionMode`), target length, delay, and min/max clamp values. Supports `PushState()`/`PopState()` for undo.

### `ObstacleData` (system design / schematic wire routing)

File: `AD26-dotnet/Altium.Designer.SystemDesign/.../ObstacleData.cs`

This is used by the **schematic wire/connection router** (not PCB), via `AStarMinimumCornersFinder`:

```csharp
public class ObstacleData {
    public Rect Bounds { get; set; }
    public bool IsWalkableHorizontal { get; set; }
    public bool IsWalkableVertical { get; set; }
    public DrawingDocumentItem Item { get; set; }
}
```

The A* path finder (`AStarMinimumCornersFinder`) uses these. When `IgnoreObstacles = true`, it passes `Enumerable.Empty<ObstacleData>()` to the path finder. Otherwise it passes the full obstacle list. If no path is found without ignoring obstacles, it retries with `ignoreObstacle: true`.

---

## Algorithm Notes

### Push-Obstacle Mechanics (`IPCB_AdvanceRouteCommands`)

The key predicate is `IsPushablePrimitive(IPCB_Primitive argP)` on `IPCB_AdvanceRouteCommands`. The Delphi router calls this to check whether each obstacle can be displaced. If pushable, the router calls `RemovePrimitiveFromBoard` + `AddPrimitiveToBoard` on displaced tracks to reroute them out of the way.

The linked-list of primitives along a route is maintained via:
- `SetState_BackConnectedPrim` / `GetState_BackConnectedPrim`
- `SetState_ForwardConnectedPrim` / `GetState_ForwardConnectedPrim`

`GetRoutedPath()` returns the `IPCB_Group` of primitives laid down so far.

### HugNPush (`eARHugAndPushObstacle`)

From the name and related `THuggingStyle` + `TRouteHugging` enums: the router first attempts to walk around the obstacle by hugging its boundary (using `eRouteHug` from `TRouteHugging`), then falls back to pushing if no hugging path exists. The hugging style (`THuggingStyle`) controls whether the hug path uses arcs (`eStyleRounded`), 45°/90° turns (`eStyleDegrees`), or a mix (`eStyleMixed`).

### Gloss Pass

After each segment commit, the router applies a gloss smoothing pass. `TGlossEffort` controls aggressiveness. `NeighborGlossEffort` applies the same smoothing to **adjacent pre-existing traces** that were pushed. `GlossEffort_None` means no post-routing smoothing.

### Auto-Necking

`GetState_AutoNecking()` — when enabled, the router automatically narrows the trace width near pads (using `IPCB_RoutingNeckDownRule`) if the full-width trace would violate clearance. This is a licensed feature gated by:

```csharp
DXP.GlobalVars.Client.GetInternalOptions().ReadFeatureBoolean("PCB.Routing.EnableAutoShrinking", false)
```

### Quick Route vs Advanced Router

`GetState_LegacyRouter()` returns `true` for the legacy "Quick Route" (2-layer DXP-era router). When true:
- Only 4 conflict modes are available (no Stop/AutoRoute variants)
- `AllowViaPushing` UI control is hidden
- `MiterSize`, `GlossEffort`, `AutoNecking`, `SubnetJumperLength`, `PreferredClearance` UI controls are hidden

### Via Pushing

`GetState_AllowViaPushing()` / `SetState_AllowViaPushing()` on both the process object and `IPCB_RoutingOptionsPage`. When true, vias encountered as obstacles can also be displaced in push-obstacle mode.

### Diff-Pair Follow Mode

`GetState_FollowMode()` on `IPCB_CustomInteractiveRoutingProcess`. When the routed net is a diff-pair member, follow mode routes both pair members simultaneously, maintaining the gap defined by `IPCB_DifferentialPairsRoutingRule`.

---

## Differential Pair Routing

### `IPCB_DifferentialPairsRoutingRule`

Per-layer width/gap constraints:

```csharp
int GetState_MaxGap(TV7_Layer argL);
int GetState_MinGap(TV7_Layer argL);
int GetState_MaxWidth(TV7_Layer argL);
int GetState_MinWidth(TV7_Layer argL);
int GetState_MaxUncoupledLength();

// Sub-stack variants (for impedance-controlled layer stacks)
int GetState_MaxGapAtSubStack(TV7_Layer argL, string argStackID);
int GetState_MinGapAtSubStack(TV7_Layer argL, string argStackID);
```

`IPCB_DifferentialPairsRoutingRule2` and `IPCB_DifferentialPairsRoutingRule3` add extended interfaces for backwards-compatibility.

### Routing Topology

File: `AD26-dotnet/Altium.ConstraintsManager/Altium.ConstraintsManager.Implementation.Rules/RoutingTopologyType.cs`

| Value | Description |
|---|---|
| `Shortest` | Minimum spanning tree |
| `Horizontal` | Horizontal daisy-chain |
| `Vertical` | Vertical daisy-chain |
| `DaisyChain_Simple` | Linear daisy-chain |
| `DaisyChain_MidDriven` | Mid-point driven daisy |
| `DaisyChain_Balanced` | Balanced daisy |
| `Starburst` | Star topology |

---

## Feature Gates

| Feature key | Gated behavior |
|---|---|
| `PCB.Routing.EnableAutoShrinking` | AutoNecking |
| `PCB.Routing.AnyAngleDiffPairRouter` | Angle-mode diff-pair routing (`IsAngleDiffPairRouterAvailable`) |
| `IPCBRoutingFeatures.GlossEffort()` | Gloss effort controls |
| `IPCBRoutingFeatures.AdvancedProps()` | Advanced routing properties panel |

---

## Observations / Open Questions

1. **"PushPull" identifier never appears in C# code.** The concept is implemented as `TAdvancedRouteMode.eARPushObstacle` and `eARHugAndPushObstacle`. The Delphi DLLs (not decompiled here) contain the actual routing engine.

2. **`IsPushablePrimitive` logic is in Delphi.** The C# interface `IPCB_AdvanceRouteCommands.IsPushablePrimitive` is a COM call into the Delphi routing engine. The actual logic for which primitives can be pushed (by flag, lock state, net membership, etc.) is not visible from C# alone. The `TPushMode` enum in `IPCB_RoutingOptionsPage` suggests that lock state and "other net" membership are factors.

3. **`TSmartRouteMode` usage unclear.** It appears in only two files (`TSmartRouteMode.cs` and `TSmartRouteModeConsts.cs`) and is never referenced by any process interface. It may be an older intermediate version preserved for serialization compatibility.

4. **`TInteractiveRouteMode` vs `TAdvancedRouteMode` migration.** System options (`IPCB_SystemOptions`) still use the 3-value `TInteractiveRouteMode` for a global default, while process objects use the 7-value `TAdvancedRouteMode`. The mapping between them is not visible in C#.

5. **Accordion/length tuning `PushState()`/`PopState()`.** The `IPCB_AccordionMakerSettings` interface has push/pop state for undo support. The actual undo model (how it integrates with the board's undo stack) is in Delphi.

6. **`IPCB_AdvanceRouteCommands.GetRoutingFlags()` returns `int`.** The bit flags are not decoded in visible C# — need Ghidra analysis of the routing DLL to understand individual flag bits.

7. **`IPCB_RoutingOptionsPage` serialization.** The page implements `Export_ToParameters` / `Import_FromParameters` (inherited from `IPCB_AbstractOptions`) but the actual parameter key names are in Delphi. The PCBDoc/PCBLib file format stores routing options as a parameter block; the exact keys need to be traced from the Delphi source.
