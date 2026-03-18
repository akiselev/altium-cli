# Delphi Routing Engine Reverse Engineering

Reverse-engineered from `Advpcb.dll` (392K functions) in the altium26 Ghidra project.

## Binary Inventory

| DLL | Functions | Role |
|-----|-----------|------|
| **Advpcb.dll** | 392,055 | Main PCB editor — contains the entire interactive routing engine |
| Altium.PCB.DataModel.dll | 52,078 | .NET COM wrappers — thin layer that delegates to Advpcb.dll via COM interop |
| Altium.PCB.DataModel.X.dll | 10,717 | .NET extension interfaces |
| Altium.PCB.BinaryLoader.dll | 107,309 | .NET binary file loading |

The routing engine lives **entirely in Delphi** (Advpcb.dll). The .NET layer only provides
COM interface wrappers for properties and settings.

## Routing Architecture Overview

Altium's interactive router follows a **state machine pattern** built on Delphi's
`TInteractiveProcess` framework. The key classes form this hierarchy:

```
TInteractiveProcess (base)
  -> TAdvancedRoute (main interactive routing class)
       Fields:
         +0x090: Board (IPCB_Board)
         +0x270: Obstacles
         +0x280: Sub-engine
         +0x298: Start point (TCoordPoint)
         +0x2B0: End point (TCoordPoint)
         +0x2D0: PCB layer data
         +0x2D8: Connection data
         +0x2E8: State handler
         +0x308: Route path
         +0x348-352: Routing flags
         +0x358: System options interface
         +0x370: Core router engine (TAdvancedRouteEngine subobject)
         +0x378-380: Route mode flags
         +0x3A0: DRC checker
         +0x3A8: Route event handler
         +0x3B8: Advanced route engine
         +0x3C8-3D8: Timer/callback objects
         +0x3F8-400: Primitive lists
         +0x408: Sub-object
         +0x418: Re-entrant process guard
         +0x428: Clearance manager
         +0x430: Via combination manager
         +0x438: Interface ref

  -> TAdvancedRouteEngine
       Fields:
         +0x20: Board
         +0x30: Sub-object A (routing context)
         +0x38: Sub-object B (routing state)
         +0x40: Sub-object C (layer info)
         +0x48: Sub-object D (clearance info)
         +0x50: Sub-object E (connection info)

  -> TAdvancedRouteStateHandler
       Methods: GetInteractiveDataClass, CreateInteractiveStates,
                GetStartInteractiveStateID, GetFinishInteractiveStateID

  -> TAdvancedRouteInteractiveData
       Fields: AdvRoute (ptr), Location (TCoordPoint),
               Zoomed_To_Lens (bool), RouteAborted (bool)

  -> TAdvancedRouteOptions
       (routing options/preferences)
```

### State Machine States

The routing process uses a two-state machine:

| State | RTTI Name | Purpose |
|-------|-----------|---------|
| Route8 | `TInteractiveState_Route8` | Active routing — OnEnterState, OnExitState, CheckExitCondition |
| Route0 | `TInteractiveState_Route0` | Initial/idle state — contains AdvancedRoute reference |

The state transition happens when CheckExitCondition (at `0x0390cdf0`) checks two virtual
methods on the AdvancedRoute object (vtable offsets 0x238 = GoalAchieved and 0x240 = Abort).

## Key Functions

### TAdvancedRoute Published Methods

| Address | Method | Description |
|---------|--------|-------------|
| `0x038EE680` | **Create** | Constructor — takes Board, RouteGuideMode, ALegacyRouter |
| `0x038EEFA0` | **Destroy** | Destructor |
| `0x038F51A0` | **RunRouter** | Entry point — creates TAdvancedRoute, sets up state handler, runs state machine |
| `0x038F4C90` | **RunAutoRouter** | Autoroute entry — takes Locations list |
| `0x038F5070` | **RunQuickRouter** | Quick route — takes ConnLine, StartPoint |
| `0x03908D50` | **EndQuickRouter** | Finish quick route |
| `0x038FE440` | **BeginRouting** | Start routing a connection — initializes start/end points |
| `0x03908DC0` | **EndRouting** | Finish routing |
| `0x03901C60` | **UpdateObject** | **Main routing loop** — called on every mouse move |
| `0x039032A0` | **Handle_ReleasedLB** | Left-click: commit current route segment |
| `0x03904230` | **Handle_ReleasedRB** | Right-click handler |
| `0x039042F0` | **Handle_Backspace** | Undo last segment |
| `0x039056E0` | **Handle_Space** | Switch routing layer (insert via) |
| `0x039044A0` | **Handle_Escape** | Cancel routing |
| `0x039056B0` | **Handle_Return** | Complete routing |
| `0x039049A0` | **Handle_Plus** | Cycle to next route mode / width |
| `0x03904CF0` | **Handle_Multiply** | Multiply handler |
| `0x03904F90` | **Handle_Minus** | Cycle to previous route mode / width |
| `0x03905610` | **Handle_Y** | Y key handler |
| `0x039097D0` | **Handle_OtherKeys** | Generic key handler |
| `0x0390B080` | **Handle_Resume** | Resume routing |
| `0x0390B0B0` | **GoalAchieved** | Check if routing reached target |
| `0x0390B0C0` | **Abort** | Abort routing |
| `0x03907430` | **RegisterHelpShortcuts** | Register UI shortcuts |
| `0x0390BDB0` | **OnEndReEntrantProcess** | Re-entrant process callback |
| `0x0390BE90` | **OnTerminate** | Termination handler |
| `0x0390D290` | **SwitchToFirstAvailableOrDefault** | Switch route mode |
| `0x038FE220` | **GetLastRoutedPrimitive** | Get last placed track/arc |
| `0x038FE2E0` | **GetLastRoutedPrimitiveIgnoreVirtualPad** | Same, ignoring virtual pads |
| `0x0390BBB0` | **ResetViolations** | Clear DRC violations |
| `0x038FDFA0` | **GetLastRoutePoint** | Get last committed route point |
| `0x038FE050` | **ComputeWidth** | Compute track width from rules |
| `0x03902600` | **RestoringPreviousState** | Check if restoring state |
| `0x03902610` | **DoingSecondLegAfterVia** | Check if routing 2nd leg after via |

### TAdvancedRouteEngine Methods

| Address | Method | Description |
|---------|--------|-------------|
| `0x03DB4CF0` | **Create** | Constructor — takes Board, creates sub-engines |

### Internal Helper Functions

| Address | Called From | Purpose |
|---------|------------|---------|
| `0x039009F0` | UpdateObject | Core routing dispatch — branches on route mode |
| `0x03900CD0` | 039009F0 | Main interactive routing path (normal mode) |
| `0x039003E0` | 039009F0 | Alternative routing path (mode flag 0x360 = 1) |
| `0x038FD790` | 03900CD0 | Route path computation dispatch |
| `0x038FBEF0` | 038FD790 | **Primary route computation** — the core pathfinding |
| `0x038FB680` | 038FBEF0 | **Route segment builder** — builds track/arc list |
| `0x038F9F40` | 038FB680 | Route via follow-mouse mode |
| `0x038F94D0` | 038FB680 | Route via two-segment mode |
| `0x038F72B0` | 038FB680 | Route fixup/glossing |
| `0x038FB120` | 038FBEF0 | Build initial route candidate |
| `0x038F5410` | 038FB680 | Clear/remove route primitives |
| `0x038F3530` | BeginRouting | Post-initialization routing setup |
| `0x038F1400` | BeginRouting | Pre-routing initialization |
| `0x038FF3F0` | Multiple | Prepare route state for commit |
| `0x039037F0` | Handle_ReleasedLB | Commit route segment to board |
| `0x038F7F00` | Multiple | Get current route mode (returns TAdvancedRouteMode) |
| `0x038FAB90` | Multiple | Check if route can proceed |
| `0x038F9800` | 038FB680 | Apply route to board |
| `0x038F8070` | Multiple | Build routed primitives list |
| `0x0390D700` | 03900CD0 | Check/apply glossing |
| `0x0390DEB0` | 03900CD0 | Apply gloss to neighbors |
| `0x03900C60` | 03900CD0 | Finalize routing update |
| `0x039017D0` | 03900CD0 | Post-routing cleanup/display |

### Obstacle & Clearance Functions

| Address | Purpose |
|---------|---------|
| `0x02A68EA0` | Create obstacles collection |
| `0x02A65300` | Check obstacle collision |
| `0x029020F0` | QueueRemoveClearanceViolation (exported) |
| `0x03D52E00` | PCBAPI_CheckPrimitivesOverlapWithClearance (exported) |

### Situs (Shape-Based) Router

| Address | Purpose |
|---------|---------|
| `0x05B35820` | Get_ADVPCB_Situs_API_Impl — factory for the Situs autorouter API |

## Algorithm Analysis

### Routing Modes (TAdvancedRouteMode)

From RTTI at `0x00FB9840`:

```
enum TAdvancedRouteMode : u8 {
    eARIgnoreObstacle       = 0,  // Route through obstacles
    eARWalkAroundObstacle   = 1,  // Walk around obstacles
    eARPushObstacle         = 2,  // Push obstacles out of the way
    eARHugAndPushObstacle   = 3,  // Hug obstacle edges + push
    eARStopAtFirstObstacle  = 4,  // Stop when hitting an obstacle
    eARAutoRouteCurrentLayer= 5,  // Autoroute on current layer
    eARAutoRouteMultiLayer  = 6,  // Autoroute across layers
}
```

### Interactive Route Modes (TInteractiveRouteMode)

From RTTI at `0x00FB9652`:

```
enum TInteractiveRouteMode : u8 {
    eIgnoreObstacle = 0,
    eAvoidObstacle  = 1,
    ePushObstacle   = 2,
}
```

### Smart Route Modes (TSmartRouteMode)

From RTTI at `0x00FB97C8`:

```
enum TSmartRouteMode : u8 {
    eSRIgnoreObstacle     = 0,
    eSRAvoidObstacle      = 1,
    eSRWalkAroundObstacle = 2,
    eSRPushObstacle       = 3,
}
```

### Routing Width Mode

```
enum TRoutingWidthMode : u8 {
    eRoutingWidth_Default   = 0,
    eRoutingWidth_Min       = 1,
    eRoutingWidth_Preferred = 2,
    eRoutingWidth_Max       = 3,
}
```

### Corner Style

```
enum TRoutingCornerStyle : u8 {
    eRoutingCornerStyle_90  = 0,  // 90-degree corners
    eRoutingCornerStyle_45  = 1,  // 45-degree mitered corners
    eRoutingCornerStyle_Any = 2,  // Any angle
}
```

### Gloss Effort

```
enum TGlossEffort : u8 {
    eGlossEffort_None   = 0,
    eGlossEffort_Weak   = 1,
    eGlossEffort_Strong = 2,
}
```

### Hugging Style

```
enum THuggingStyle : u8 {
    eStyleMixed   = 0,
    eStyleRounded = 1,
    eStyleDegrees = 2,
}
```

### Track Placement Modes

From RTTI at `0x00FB9980`:

```
enum TPlaceTrackMode : u8 {
    ePlaceTrackNone  = 0,
    ePlaceTrackAny   = 1,
    ePlaceTrack9090  = 2,
    ePlaceTrack4590  = 3,
    ePlaceTrack90Arc = 4,
}
```

### Vertex Actions (for sliding/dragging)

```
enum TVertexAction : u8 {
    eDeform = 0,
    eScale  = 1,
    eSmooth = 2,
}
```

### Routing Options Parent Kind

```
enum TRoutingOptionsParentKind : u8 {
    ePCBPreferences    = 0,
    eInteractiveRouter = 1,
    eSmartRouter       = 2,
    eMultiRouter       = 3,
    eDrag              = 4,
    eDragVia           = 5,
    eDiffPairRouter    = 6,
}
```

### Main Routing Loop (UpdateObject)

The `UpdateObject` method (`0x03901C60`) is called on every mouse move during routing:

```
UpdateObject(self):
    update_display(self)
    if not self.is_dragging:
        core_routing_update(self, current_mouse_pos, 0, 1)
    update_3d_obstacles(self.board, 1)
```

The core routing update (`FUN_039009F0`) dispatches based on route mode:

```
core_routing_update(self, mouse_pos, param3, param4):
    if is_re_entrant(self.guard):
        return
    if is_cancelled(self.guard):
        finalize(self)
        return
    if self.mode_flag_0x360 == 1:
        route_mode_A(self, mouse_pos)      // Alternative mode
    else:
        route_mode_B(self, mouse_pos, param3, param4)  // Normal interactive routing
    post_route_update(self)
    update_ui(self)
    finalize_display(self)
```

### Normal Interactive Routing Path (FUN_03900CD0)

This is the main routing function called on every mouse update:

1. **Reset state**: Clear current route and violations
2. **Snap to target**: Check if mouse is near a target pad/via
3. **Compute route point**: Apply grid snap, obstacle avoidance
4. **Check route mode**: Determine if using push/avoid/ignore/hug
5. **Build route path**: Call the pathfinder to compute track geometry
6. **Apply DRC**: Check clearance violations on proposed route
7. **Handle via insertion**: If on different layer, insert via
8. **Apply glossing**: Smooth/optimize the routed path
9. **Update display**: Show proposed route to user

### Route Path Computation (FUN_038FBEF0)

The primary pathfinding function:

1. Prepares routing state via `FUN_038FF3F0`
2. Calls `FUN_038FB120` to build initial route candidate
3. If route has segments, checks if route needs two legs (via insertion case)
4. Calls `FUN_038FB680` to build the actual route segments
5. If two-segment mode (via), builds first leg then second leg
6. Returns success/failure

### Route Segment Builder (FUN_038FB680)

This is the core pathfinding/segment construction:

1. Gets current layer and checks layer consistency
2. If in follow-mouse mode, calls `FUN_038F9F40` for mouse-following path
3. For via case, calls `FUN_038F94D0` for two-segment routing
4. Calls `FUN_038F72B0` for route fixup/glossing
5. Applies route to board via `FUN_038F9800`
6. Updates route state with width, layer, position data

### Left-Click Commit (Handle_ReleasedLB)

When the user clicks to commit a route segment:

1. Update route event handler (`FUN_038E0200`)
2. Prepare route state (`FUN_038FF3F0`)
3. Commit route to board (`FUN_039037F0`)

### Layer Switch (Handle_Space)

When the user presses Space to switch layers:

1. If not in differential pair mode:
   - Get next layer from routing options
   - Update layer setting
   - Insert via at current point
   - Rebuild route on new layer
2. If in differential pair mode:
   - Different via insertion logic
3. Recalculate route from current point

## Data Structures

### PathFinder Module (TPolygonPathFinder)

The `PolygonPathFinder` unit (RTTI at `0x054DCF90`) implements a contour-based graph
pathfinder used for routing within polygon regions:

```
enum TContourKind {
    BoundaryContour,
    StartContour,
    EndContour,
    PathInPolygon,
}

record TContourDescr {
    // Contour description — boundary or obstacle outline
}

record TGraphNode {
    // Node in the pathfinding graph — visibility graph node
}

record TGraphRangeDescr {
    // Range/sector descriptor for graph traversal
}

record TSegmentInfo {
    // Segment of a computed path
}
```

### Obstacle Types (TLRO_*)

Routing obstacles use a typed hierarchy (RTTI at `0x03347900`):

- **TLRO_Uni** — Universal obstacle (generic shape)
- **TLRO_Arc** — Arc-shaped obstacle
- **TLRO_Poly** — Polygon obstacle

These are stored in dictionaries (`Obstacles.TLRO_Arc>>`) and used for collision
detection during routing.

### Route Commands Interface (COM)

The `IPCB_AdvanceRouteCommands` interface (GUID `F0831499-190D-4429-8B4A-6803D583FC7E`)
provides the COM-visible routing operations:

- `AddPrimitiveToBoard(backPrim, primToAdd, forwardPrim)`
- `RemovePrimitiveFromBoard(prim)`
- `ReplaceConnectedPrimitivesInBoard(toRemove, toAdd)`
- `GetRoutedPath() -> IPCB_Group`
- `IsPushablePrimitive(prim) -> bool`
- `GetWidthFromRouter(prim) -> int`
- `GetCurrentLayerFromRouter() -> TV7_Layer`
- `GetTargetPointForRoute(routeIndex, out targetPoint) -> bool`
- `GetRoutingFlags() -> int`

## Call Graph

```
RunRouter (0x038F51A0)
  -> Create (0x038EE680)
       -> TAdvancedRouteEngine.Create (0x03DB4CF0)
  -> BeginRouting (0x038FE440)
       -> FUN_038F4060 (init routing)
            -> FUN_038F1400 (pre-init)
            -> FUN_038F3530 (post-init)

[Main Loop — called per mouse move]
UpdateObject (0x03901C60)
  -> FUN_039009F0 (core dispatch)
       -> FUN_03900CD0 (normal interactive routing)
            -> FUN_0390BBB0 (reset violations)
            -> FUN_038FD790 (route path computation)
                 -> FUN_038FBEF0 (primary route computation)
                      -> FUN_038FF3F0 (prepare state)
                      -> FUN_038FB120 (build initial candidate)
                      -> FUN_038FB680 (build route segments)
                           -> FUN_038F9F40 (follow-mouse path)
                           -> FUN_038F94D0 (two-segment via path)
                           -> FUN_038F72B0 (route fixup/glossing)
                           -> FUN_038F9800 (apply to board)
                      -> FUN_038F5410 (clear primitives)
            -> FUN_0390D700 (check glossing)
            -> FUN_0390DEB0 (apply neighbor gloss)
            -> FUN_03900C60 (finalize update)
            -> FUN_039017D0 (post-update cleanup)
       -> FUN_039003E0 (alternative routing mode)
  -> FUN_038F2510 (post-update)
  -> FUN_029BADE0 (UI update)
  -> FUN_038FFCB0 (display update)

[User Actions]
Handle_ReleasedLB (0x039032A0)
  -> FUN_038FF3F0 (prepare)
  -> FUN_039037F0 (commit route)

Handle_Space (0x039056E0)
  -> FUN_029B15F0 (insert via)
  -> FUN_039009F0 (recalculate route on new layer)

Handle_Plus (0x039049A0)
  -> FUN_03904500 (cycle route mode)
  -> FUN_039009F0 (recalculate with new mode)

Handle_Escape (0x039044A0)
  -> Cancel routing, restore state

Handle_Backspace (0x039042F0)
  -> Remove last segment

EndRouting (0x03908DC0)
  -> Finalize and commit all segments
```

## COM Interface Hierarchy (from .NET)

```
IInteractiveProcess
  -> IPCB_InteractiveProcess
       -> IPCB_CustomInteractiveRoutingProcess
            Fields: RouteMode, RoutingCornerStyle, GlossEffort,
                    HuggingStyle, MiterSize, MinimumArcSize,
                    Width, ViaDiameter, HoleSize, CurrentLayer,
                    AllowViaPushing, AutoRemoveLoops, AutoRemoveAntennas,
                    FollowMouseTrail, DisplayClearanceBounds,
                    PadEntryStability, NeighborGlossEffort, LegacyRouter
            -> IPCB_InteractiveRoutingProcess
                 Extra: WidthRule, DiffPairRule, DifferentialPair,
                        ShowLengthGauge, Impedance, PinSwapping
            -> IPCB_InteractiveMultiRoutingProcess
            -> IPCB_InteractiveDiffPairRoutingProcess
       -> IPCB_InteractiveLineRoutingProcess
       -> IPCB_SlidingRoutingProcess
            Fields: Sliding (TAdvancedRouteMode), VertexAction,
                    IsSingleNet, NetLength, NetDelay
```

## Exported API Functions

| Export | Address | Notes |
|--------|---------|-------|
| `PcbApi_QueryBoardAdvancedRouterOptions` | `0x03D3D930` | Returns 0 (stub) |
| `PcbApi_QueryBoardSpecctraRouterOptions` | `0x03D3DA00` | Returns 0 (stub) |
| `PCBAPI_CheckPrimitivesOverlapWithClearance` | `0x03D52E00` | DRC clearance check |
| `PcbApi_QueryRuleClearanceConstraint` | `0x03D4C570` | Get clearance rule |
| `PcbApi_QueryRuleComponentClearanceConstraint` | `0x03D4FDD0` | Component clearance rule |
| `QueueRemoveClearanceViolation` | `0x029020F0` | Queue violation removal |
| `Get_ADVPCB_Situs_API_Impl` | `0x05B35820` | Situs autorouter factory |

## Additional Decompilation Details

### GoalAchieved (0x0390B0B0)

Trivial check: returns the byte at `self + 0xD8`. This flag is set elsewhere when the
route reaches its target pad/via.

### Abort (0x0390B0C0)

Checks if the process is re-entrant (via guard at `self + 0x418`), otherwise returns
the byte at `self + 0x4A`.

### GetRouteMode (0x038F7F00)

```
if self.override_flag[0x260] == 0:
    return system_options.global_ptr_chain[0x148][0x38][0x18]  // from global options
else:
    return 0  // eARIgnoreObstacle
```

The route mode is read from a global system options chain unless overridden.

### Two-Segment Via Routing (0x038F94D0)

1. Find via insertion point via `FUN_038F66D0`
2. If via found:
   - Update 3D obstacles for the board
   - Get layer from via primitive
   - Clear current route
   - Rebuild route from via point to target via `FUN_038FB120`

### Route Fixup/Glossing (0x038F72B0)

Branches on route mode:
- **Mode 4 (StopAtFirst)**: Only check obstacles, minimal processing
- **Mode 1 (WalkAround)**: Apply obstacle avoidance without pushing
- **Other modes**: Apply full glossing/smoothing pipeline including:
  - Obstacle collision checking via `FUN_02A65C60`
  - Corner optimization
  - Track smoothing based on GlossEffort setting

## Open Questions

1. **Push algorithm internals**: The functions at `FUN_038F94D0` (two-segment routing) and
   `FUN_038F9F40` (follow-mouse) need deeper analysis to understand how push-obstacle
   resolution works. The push engine likely uses the `FPusher` field and the obstacle
   collections (`TLRO_Arc`, `TLRO_Uni`, `TLRO_Poly`).

2. **Visibility graph**: The `TPolygonPathFinder` with its `TGraphNode` and
   `TGraphRangeDescr` types suggests a visibility-graph-based pathfinder. The exact
   algorithm (Dijkstra, A*, or custom) needs to be determined by decompiling the
   PolygonPathFinder methods.

3. **Glossing/smoothing**: The gloss effort levels (None/Weak/Strong) affect route
   quality via `FUN_038F72B0` and `FUN_0390DEB0`. The exact smoothing algorithm
   (arc fitting, corner optimization) needs investigation.

4. **Situs autorouter**: The shape-based autorouter (`Get_ADVPCB_Situs_API_Impl` at
   `0x05B35820`) is a separate subsystem. Its strategy interface
   (`IPCB_SitusStrategy`) includes Passes, ViaBias, and PreferOrthogonal parameters.

5. **Hugging style implementation**: The `THuggingStyle` enum (Mixed/Rounded/Degrees)
   affects how the router follows obstacle contours. The implementation is likely in
   the obstacle-handling code near the `TLRO_*` types.

6. **Multi-route / differential pair**: The multi-routing and differential pair modes
   use separate process classes. Their specific algorithms need separate investigation.

7. **Legacy router flag**: `GetState_LegacyRouter()` suggests there's an older routing
   engine that can be used as fallback. The `ALegacyRouter` parameter in Create confirms this.

8. **PathFinderResult / PathFinderThroughCopper**: Strings at `0x04A77917` and `0x04A77B7B`
   suggest additional pathfinding result types, possibly used for copper pour routing.
