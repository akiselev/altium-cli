# Delphi Routing Engine Deep Dive

Deep analysis of the Altium interactive routing engine in `Advpcb.dll`, building on the
high-level architecture in `delphi-routing-engine.md`. This document covers the push
algorithm, visibility graph pathfinder, gloss/smoothing engine, and mode dispatch logic.

---

## Push Algorithm

### Overview

The push algorithm displaces existing tracks/primitives to make room for a new route.
It operates on the **Pusher** module (unit name `Pusher` in RTTI), which is a self-contained
subsystem with its own data types.

### Key Pusher Data Types

From RTTI strings in the `Pusher` unit:

```
Pusher.TIntervalRec     -- Interval along a push axis (min/max displacement range)
Pusher.TConstrain       -- Single constraint on a push operation
Pusher.TConstrains      -- Collection of constraints (TList<TConstrain>)
Pusher.TPrimPair        -- Pair of primitives involved in a push (pusher + pushee)
```

Generic containers used:
- `TDictionary<Pusher.TPrimPair, System.Integer>` -- maps prim-pairs to displacement amounts
- `TList<Pusher.TIntervalRec>` -- valid displacement intervals
- `TList<Pusher.TConstrains>` -- constraint sets per direction

### Key Pusher Methods (from RTTI closure names)

| Method | Purpose |
|--------|---------|
| `DoPushIfNecessary` | Main entry: checks if push needed, executes if so |
| `GetViolationsToPath` | Finds clearance violations for a proposed route path |
| `AddViolatorAndSameSubNetViolatorsToViolation` | Collects all violating prims on the same subnet |
| `DetermineEndRoutePoint` | Computes the target point after push displacement |

### FPusher Field

The `FPusher` field appears at several RTTI locations:
- `0x038eb136` (TAdvancedRoute context)
- `0x02ad62a2` (obstacle/clearance context)
- `0x02b34d7d` (another routing context)
- `0x03913475` (state handler context)

### Push Execution Flow

#### Entry Point: FUN_038E9C30 (Push Execute)

Called from `FUN_038FA3B0` (push forward) and `FUN_038FABC0` (push backward).

```
PushExecute(self, targetX, targetY, resultList, ...):
    // Phase 1: Gather push context
    self.clearance = GetPrimClearance(self.primitive, 0x17)  // rule type 0x17
    self.hasRoutingRule = FindRoutingRule(self.primitive, 0x3c) != null  // rule type 0x3c
    if hasRoutingRule:
        self.allowViaPush = GetAllowViaPush(rule)
        self.allowPadPush = GetAllowPadPush(rule)
        self.pushEnabled = GetPushEnabled(rule)
    CheckPushConstraints(self)

    // Phase 2: Determine push direction
    direction_primary = ComputeOctant(primX, primY, targetX, targetY)
    direction_secondary = ComputeSecondaryOctant(primX, primY, targetX, targetY)
    SetDirectionPriority(self, direction_primary, direction_secondary)

    // Phase 3: Try directions in priority order
    if self.canPush[direction_primary]:
        // Direct push in preferred direction
        ExecutePushInDirection(self, direction_primary, resultList)
    else:
        // Try alternative directions
        dirMask = ComputeValidDirectionMask(direction_primary)
        if TryPushDirection(self, dirMask, &chosenDir):
            ExecutePushInDirection(self, chosenDir, resultList)
        else:
            // Try complement direction
            dirMask2 = ComputeComplementMask(direction_primary)
            if TryPushDirection(self, dirMask2, &chosenDir):
                ExecutePushInDirection(self, chosenDir, resultList)
            else if param_7 == 2:
                // Last resort: try remaining directions
                remaining = ALL_DIRS & ~(dirMask | dirMask2)
                TryPushDirection(self, remaining, &chosenDir)
```

#### Direction System (8 Directions)

The push system uses **8 cardinal/diagonal directions**, indexed 0-7.

**FUN_038E5AD0** (direction mapping, at `0x038e5ad0`):
```
ComputePushDirection(octant):
    // Maps octant (1-8) to push direction index (0-7)
    match octant:
        1 -> 2    // North -> push South
        2 -> 1    // NE -> push SW
        3 -> 0    // East -> push West
        4 -> 7    // SE -> push NW
        5 -> 6    // South -> push North
        6 -> 5    // SW -> push NE
        7 -> 4    // West -> push East
        8 -> 3    // NW -> push SE
```

**FUN_038E9F90** (direction priority setup, at `0x038e9f90`):

Sets up 8 priority slots in pairs, interleaving primary and secondary directions,
then rotating outward:

```
SetDirectionPriority(self, dir1, dir2):
    if dir1 == dir2:
        dir2 = (dir1 + 1) % 8
    // Determine rotation direction
    if (dir1 XOR dir2) covers all 8 bits:
        step1 = -1; step2 = +1  // if dir2 < dir1
    else:
        step1 = -1; step2 = +1  // if dir1 < dir2
    // Fill priority array (self+0x6b, 8 bytes)
    for i in 0..4:
        self.priority[i*2] = dir1
        self.priority[i*2+1] = dir2
        dir1 = abs((dir1 + step1) % 8)
        dir2 = abs((dir2 + step2) % 8)
```

**FUN_038E9870** (try push direction, at `0x038e9870`):
```
TryPushDirection(self, validMask, outDir) -> bool:
    for i in 0..8:
        dir = self.priority_order[i]     // self+0x6b
        if self.canPush[dir]             // self+0x60 array
           AND (1 << dir) & validMask != 0:
            *outDir = dir
            return true
    return false
```

#### Push Constraint Analysis (FUN_038E8610 at `0x038e8610`)

This function computes which of the 8 push directions are valid for a given primitive:

```
CheckPushConstraints(self):
    self.validDirMask = 0xFF  // start with all directions valid
    layer = self.layer

    if hasRoutingRule AND NOT allowPadPush:
        self.validDirMask &= 0x55  // disable diagonal directions

    if (hasRoutingRule AND NOT allowViaPush) OR forceConstraint:
        // Check if primitive is wider than tall or vice versa
        width = GetWidth(self.primitive, layer)
        height = GetHeight(self.primitive, layer)
        if width < height:
            self.validDirMask &= 0xBB  // disable certain horizontal pushes
        else:
            self.validDirMask &= 0xEE  // disable certain vertical pushes

    self.allowedDirections = CompactMask(self.validDirMask)
```

**Offset 0x60**: `canPush[8]` -- boolean array, one per direction, whether push is geometrically possible.
**Offset 0x6b**: `priority[8]` -- direction indices in priority order.

#### Push Segment Generation (FUN_038E98E0 at `0x038e98e0`)

Generates the actual displaced track segments:

```
ExecutePushInDirection(self, direction, resultList):
    if self.intervals[direction] is empty: return

    startPoint = GetStartPoint(self.primitive)
    for each interval in self.intervals[direction]:
        endpoint = interval.point
        if self.pushMode == 0:  // forward push
            newTrack = CreateTrack(board, startPoint, endpoint, width, net, layer)
            InsertAtBeginning(resultList, newTrack)
        elif self.pushMode == 1:  // backward push
            newTrack = CreateTrack(board, endpoint, startPoint, width, net, layer)
            Append(resultList, newTrack)
        startPoint = endpoint
```

### Push Integration with Router

**FUN_038FA3B0** (push forward path, at `0x038fa3b0`):

Called when the router's gloss function determines push-mode routing:

```
PushForwardPath(self, startPoint, param):
    // Check if push is enabled for this layer
    if layer is in noPushLayers: return
    if NOT (licensedForPush OR forcePushEnabled): return

    pushTarget = GetPushTarget(self)
    if pushTarget == null: return

    pushEngine = CreatePushEngine(pushTarget, currentWidth)

    // Execute push with direction flags
    success = PushExecute(pushEngine, startX, startY, self.pushResultList)

    if success AND pushResultList.count > 0:
        // Validate pushed path against DRC
        lastPoint = GetEndPoint(pushResultList.last)
        if NOT ValidatePushPath(self, lastPoint, currentLayer, 1):
            ClearList(self.pushResultList)
        else:
            // Mark all pushed segments as pushed (not user-placed)
            for prim in pushResultList:
                prim.isLocked = false
                prim.isPushed = true
            // Record push for undo
            RecordPushOperation(self, pushTarget, pushResultList)
```

**FUN_038FABC0** (push backward path, at `0x038fabc0`) -- similar but pushes in the
reverse direction (from target back toward start).

---

## Visibility Graph PathFinder

### TPolygonPathFinder Architecture

Located in the `PolygonPathFinder` unit (RTTI at `0x054dcf90`). This implements a
**visibility graph** used for walk-around obstacle avoidance routing.

### Data Structures

#### TContourDescr (obstacle contour)

From RTTI at `0x054dd042`, size ~0x60 bytes:

| Offset | Field | Type | Description |
|--------|-------|------|-------------|
| +0x00 | BoundingRect | TRect | Axis-aligned bounding box |
| +0x20 | BoundingRectExpanded | TRect | Expanded AABB (with clearance) |
| +0x40 | Points | TList | Contour vertices |
| +0x48 | Kind | TContourKind | Boundary/Start/End/PathInPolygon |
| +0x50 | Z | Integer | Layer/height value |
| +0x58 | IsHole | Boolean | True if contour is a hole |
| +0x59 | IsSmallHole | Boolean | True if contour is below minimum size |
| +0x5C | ContourIndex | Integer | Index in parent list |

```
enum TContourKind {
    BoundaryContour = 0,
    StartContour = 1,
    EndContour = 2,
    PathInPolygon = 3,
}
```

#### TGraphNode (visibility graph node)

From RTTI at `0x054dd1d2`, size ~0x94 bytes. Fields from RTTI + runtime analysis:

| Offset | Field | Type | Description |
|--------|-------|------|-------------|
| +0x00 | Id | Integer | Node identifier |
| +0x04 | Kind | TContourKind | What this node represents |
| +0x08 | P | TCoordPoint | 2D position (X,Y) |
| +0x28 | Neighbors | TList<PGraphNode> | Adjacent visible nodes |
| +0x30 | InputNode | PGraphNode | Previous node in shortest path |
| +0x38 | Dist | Integer | Distance from source (Dijkstra) |
| +0x40 | Visited | Boolean | Visited flag (Dijkstra) |
| +0x64 | (contour ref) | Integer | Reference to parent contour |
| +0x78 | InputNode (runtime) | PGraphNode | Back-pointer for path reconstruction |
| +0x80 | Visited (runtime) | Boolean | Visited during pathfinding |
| +0x84 | HeapIndex | Integer | Index in priority queue |
| +0x88 | Distance (runtime) | Integer | Current best distance from source |
| +0x8C | ArrivalPoint | TCoordPoint | Coordinate at which this node was reached |

#### TGraphRangeDescr (visibility sector)

Describes a range/sector for visibility testing between graph nodes.

#### TSegmentInfo (path segment)

Represents one segment of the computed shortest path.

### Dijkstra's Algorithm Implementation

**FUN_038CE3A0** (at `0x038ce3a0`) -- the main shortest-path computation:

```
FindShortestPath(self, sourceNode, targetNode, startPoint) -> PathList:
    // Get all graph nodes from the visibility graph
    allNodes = self.glossEngine.GetAllNodes()

    // Initialize Dijkstra
    priorityQueue = TPriorityQueue.Create(capacity=2000, comparator=DistCompare)

    for node in allNodes:
        node.prevNode = null           // +0x78
        if node == sourceNode:
            node.distance = 0          // +0x88
            node.arrivalPoint = startPoint  // +0x8C
        else:
            node.distance = 0x3B9AA2F0  // +0x88, "infinity" = 1,000,000,240
            node.arrivalPoint = node.P   // +0x8C, default to node position
        InsertIntoPQ(priorityQueue, node)

    found = false

    // Main Dijkstra loop
    while priorityQueue.count > 0:
        current = ExtractMin(priorityQueue)   // FUN_038CE380

        if current.distance == 0x3B9AA2F0:
            break  // unreachable nodes remaining

        if current == targetNode:
            found = true
            break

        // Relax edges to neighbors
        neighbors = self.glossEngine.GetNeighbors(current)
        for neighbor in neighbors:
            if NOT neighbor.visited:
                edgeCost = ComputeEdgeCost(self, current, current.arrivalPoint, neighbor)
                newDist = current.distance + edgeCost
                if newDist < neighbor.distance:
                    neighbor.distance = newDist
                    UpdatePQPriority(priorityQueue, neighbor.heapIndex)
                    neighbor.arrivalPoint = <computed point>
                    neighbor.prevNode = current
                    if neighbor == targetNode:
                        found = true

    // Path reconstruction
    result = TList.Create()
    if found:
        node = targetNode
        result.Add(targetNode)
        while node.prevNode != null:
            result.InsertAt(0, node.prevNode)
            node = node.prevNode

    // Cleanup: reset all node state
    for node in allNodes:
        node.visited = false
        node.heapIndex = -1
        node.prevNode = null

    return result
```

**Infinity sentinel**: `0x3B9AA2F0` = 1,000,000,240 (approximately 100 inches in Altium's
coordinate system of 10000 units/mil).

**Priority queue**: Created with capacity 2000 (max nodes in visibility graph), using a
custom comparator function (`FUN_01FC38E0`) that compares distances.

### Edge Cost Function

**FUN_038CE110** (at `0x038ce110`):

```
ComputeEdgeCost(self, fromNode, arrivalPoint, toNode) -> Integer:
    if fromNode == toNode:
        // Self-loop: arrival point is passed through
        return EuclideanDistance(arrivalPoint, toNode.arrivalPoint)

    if fromNode.contourIndex == toNode.contourIndex:
        // Same contour: compute clamped path along contour edge
        (minPt, maxPt) = GetContourEdgeBounds(self, fromNode, toNode)
        // Clamp arrival coordinate to contour edge range
        if arrivalCoord < minPt:
            nextPoint = minPt
        elif arrivalCoord > maxPt:
            nextPoint = maxPt
        else:
            nextPoint = arrivalCoord
        return EuclideanDistance(arrivalPoint, nextPoint)

    else:
        // Different contours: cross-contour edge
        if NOT self.allowCrossContour:
            return 0x3B9AA2F0  // infinity (unreachable)

        layerCost = ComputeLayerTransitionCost(
            fromNode.prevNode.contourIndex, toNode.contourIndex)

        // Add geometric penalty based on bounding box
        (bboxMinX, bboxMinY, bboxMaxX, bboxMaxY) = GetBoundingBox(...)
        crossPenalty = ComputeCrossPenalty(bboxCoords, arrivalPoint)

        return layerCost + crossPenalty + EuclideanDistance(arrivalPoint, toNode.point)
```

---

## Gloss/Smoothing Engine

### Overview

The gloss engine refines route paths by inserting smooth transitions at corners,
converting sharp bends into arcs or diagonal segments. It operates as a post-processing
pass on the computed route.

### Main Gloss Entry Point

**FUN_038F72B0** (route fixup/glossing, at `0x038f72b0`):

```
ApplyGloss(self, param_layer, targetPoint, routePath):
    if routePath.count == 0: return

    // Check if glossing is suppressed
    if self.suppressGloss AND GetRouteMode(self) != WalkAround: return

    routeMode = GetRouteMode(self)

    // Mode 4 (StopAtFirstObstacle): minimal processing only
    if routeMode == 4:
        self.advRouteEngine.GetClearance(&obstacles)
        CheckObstacleCollision(routePath, self.obstacles, 0)
        return

    // Check if mode supports push
    subEngineMode = GetSubEngineMode(self.subEngine)
    isPushMode = subEngineMode in {2, 3}  // Push or HugAndPush

    if NOT suppressGloss AND isPushMode:
        // For push modes: obstacle check only, no smoothing
        self.advRouteEngine.GetClearance(&obstacles)
        CheckObstacleCollision(routePath, self.obstacles, 0)
        return

    // Check cross-layer routing
    if NOT suppressGloss AND IsCrossLayerRouting(self):
        if startLayer != param_layer:
            if IsMultiSegmentMode(self): return
            if routeMode == WalkAround:
                // WalkAround on different layer: obstacle check only
                self.advRouteEngine.GetClearance(&obstacles)
                CheckObstacleCollision(routePath, self.obstacles, 0)
                return

    // === Main Glossing Path ===
    self.advRouteEngine.GetClearance(&glossCtx)

    // Step 1: Check if path can be smoothed with the GlossEngine
    routeMode = GetRouteMode(self)
    canSmooth = glossCtx.CanSmooth(currentLayer, routeMode, routePath)
    if NOT canSmooth: return

    // Step 2: Remove trailing arc if present (will be re-added)
    lastPrim = routePath.Last()
    if GetPrimType(lastPrim) == Arc:
        trailingArc = routePath.RemoveLast()

    // Step 3: Handle WalkAround mode with gloss
    if suppressGloss AND routeMode == WalkAround:
        // Clear route and recompute via walk-around gloss engine
        ClearRoute(self, routePath)
        // ... compute walk-around path with arc fitting ...
        WalkAroundWithGloss(self, targetPoint)
        return

    // Step 4: Normal gloss - process each segment
    if routePath.count > 0:
        startPt = GetStartPoint(routePath[0])
        endPt = GetEndPoint(routePath.Last())
        width = GetWidth(routePath[0])

        canGloss = GlossEngine.CheckGlossability(
            self.glossEngine, startPt, endPt, width)

        if NOT canGloss:
            ClearAllLists(self)
            return

        // Apply corner optimization
        GlossEngine.OptimizeCorners(self.glossEngine, routePath)
        GlossEngine.UpdateTrackGeometry(self.glossEngine, routePath)

        // Remove zero-length segments
        RemoveZeroLengthSegments(routePath)

        // Re-add trailing arc if applicable
        if trailingArc != null:
            if routeMode NOT IN {WalkAround, StopAtFirst}:
                AdjustTrailingArc(self, trailingArc, routePath)
                // ... complex arc fitting logic ...
```

### Corner Optimization (GlossEngine)

**FUN_038D1600** (at `0x038d1600`) -- the main gloss processing loop:

```
GlossProcessRoute(glossEngine, routePath):
    if IsAlreadyGlossed(routePath): return

    glossLevel = GetGlossLevel(globalOptions)
    MarkAsGlossed(routePath)

    startPt = GetStartPoint(routePath[0])
    firstNode = LookupGraphNode(glossEngine, startPt, GetWidth(routePath[0]))
    endPt = GetEndPoint(routePath.Last())

    prevPoint = GetStartPoint(routePath[0])
    lastProcessedNode = -1

    newSegments = TList.Create()

    for each segment in routePath:
        // Find intersection with visibility graph
        intersections = FindIntersections(glossEngine, firstNode, segment)

        if intersections.count == 0 AND NOT reachedEnd:
            // No intersection: create direct track
            nextPoint = GetEndPoint(segment)
            if prevPoint != nextPoint:
                newTrack = CreateTrack(segment, prevPoint, nextPoint)
                newSegments.Add(newTrack)
            prevPoint = nextPoint

        for each intersection:
            if NOT reachedEnd:
                // Before intersection: check graph connectivity
                (splitOk, splitPoint) = FindGlossSplitPoint(glossEngine, ...)
                if NOT splitOk:
                    nextPoint = GetEndPoint(segment)
                else:
                    nextPoint = splitPoint
                    reachedEnd = true

                if prevPoint != nextPoint:
                    newTrack = CreateTrack(segment, prevPoint, nextPoint)
                    newSegments.Add(newTrack)

            if reachedEnd:
                // After intersection: check exit point
                (exitOk, exitPoint) = FindGlossExitPoint(glossEngine, ...)
                if exitOk:
                    if splitPoint != exitPoint:
                        // Insert smoothing arcs/tracks between split and exit
                        SmoothCorner(glossEngine, segment, newSegments,
                                     splitPoint, exitPoint)
                    reachedEnd = false
            prevPoint = nextPoint

    // Ensure first segment starts at original start
    if newSegments.count > 0 AND newSegments[0].start != originalStart:
        InsertConnector(newSegments, originalStart)

    // Ensure last segment ends at original end
    EnsureEndConnection(glossEngine, segment, newSegments)

    // Remove zero-length segments from result
    RemoveZeroLength(newSegments)

    // Replace original path with glossed path
    routePath.Clear()
    routePath.AddAll(newSegments)
```

### Smoothing Corner Insertion

**FUN_038D1CB0** (at `0x038d1cb0`) -- inserts smooth transitions at corners:

```
SmoothCorner(glossEngine, segment, resultList, startPoint, endPoint):
    // Check clearance around corner
    UpdateClearanceContext(board, resultList)

    width = GetWidth(segment)

    // Iteratively try to smooth the corner
    loop:
        success = TryCornerSmooth(glossEngine, startPoint, endPoint, 0,
                                  width, layerWidth,
                                  &newStart, layer, &newEnd, &newLayer)
        if NOT success: break

        // Create intermediate track/arc segments
        CreateIntermediateSegments(board, self.glossBoard,
                                  newStart, newEnd, width, net, layer, resultList)

        // Update clearance context
        UpdateClearanceContext(board, resultList)

        // Try next corner
        success = AdvanceToNextCorner(glossEngine, &newStart, &newEnd, &newLayer)
```

**FUN_038D0340** (at `0x038d0340`) -- the core corner smooth attempt:

Uses the Dijkstra pathfinder (FUN_038CE3A0) to find an optimized path around a corner.
The graph nodes represent contour vertices with visibility edges, and the shortest path
through this graph gives the smoothest route.

```
TryCornerSmooth(glossEngine, fromPt, toPt, ...) -> bool:
    // Set up corner context
    self.fromPoint = fromPt
    self.toPoint = toPt

    // Look up graph nodes for both endpoints
    glossCtx = self.GetGlossContext()
    RegisterEndpoint(glossCtx, toPt, layerWidth)
    fromNode = LookupNode(glossCtx, fromPt, width)
    toNode = LookupNode(glossCtx, toPt, layerWidth)

    if fromNode == null: return false
    if NOT fromNode.CanReach(fromPt): return false

    if fromNode != null AND toNode != null:
        // Run Dijkstra between the two nodes
        path = FindShortestPath(glossEngine, fromNode, toNode, fromPt)

    if path == null OR path.count == 0: return false
    if NOT ValidatePathClearance(glossEngine, path): return false
    if smoothQueue.count == 0: return false

    // Extract first smooth point
    (outStart, outLayer) = smoothQueue.PopFirst()

    // Extract next smooth point if available
    if smoothQueue.count > 0:
        (outEnd, ...) = smoothQueue.PopFirst()

    return true
```

### Neighbor Gloss

**FUN_0390DEB0** (at `0x0390deb0`) -- applies glossing to neighboring tracks:

```
ApplyNeighborGloss(self):
    ctx = PrepareNeighborGlossContext()
    if ctx == null: return

    GatherAffectedNeighbors(ctx)      // FUN_0390DAE0
    FilterGlossableNeighbors(ctx)     // FUN_0390DC60
    ApplyGlossToNeighbors(ctx)        // FUN_0390DD20
    UpdateRouteFromGloss(self, ctx)   // FUN_038F96E0
```

---

## Mode Dispatch Logic

### Top-Level Dispatch

**FUN_039009F0** (core routing dispatch, at `0x039009f0`):

```
CoreRoutingUpdate(self, mousePos, param3, param4):
    // Check re-entrant guard
    if IsReEntrant(self.guard):
        return

    if IsCancelled(self.guard):
        FinalizeRouting(self)
        return

    // Branch on mode flag at offset 0x360
    if self.modeFlag_0x360 == 1:
        // Alternative mode (autoroute current connection)
        AlternativeRoutePath(self, mousePos)        // FUN_039003E0
    else:
        // Normal interactive routing
        NormalInteractiveRoute(self, mousePos, param3, param4)  // FUN_03900CD0

    PostRouteUpdate(self)
    UpdateUI(self)
    UpdateDisplay(self)
```

### Route Mode Reading

**FUN_038F7F00** (GetRouteMode, at `0x038f7f00`):

```
GetRouteMode(self) -> TAdvancedRouteMode:
    if self.overrideFlag[0x260] == 0:
        // Read from global system options chain
        return globalOptions->routingPrefs->advRouteOptions->routeMode
        // ptr chain: [0x62A6C28] -> [+0x148] -> [+0x38] -> [+0x18]
    else:
        return eARIgnoreObstacle (0)
```

### Mode-Dependent Gloss Activation

**FUN_0390D700** (check/apply glossing mode, at `0x0390d700`):

This function reads the sub-engine mode and maps certain mode values to gloss-related
actions:

```
CheckGlossMode(self) -> bool:
    if self.subEngine == null: return false

    subMode = GetSubEngineMode(self.subEngine)  // offset 0x280

    match subMode:
        3 (HugAndPush):
            SetSubEngineMode(self.subEngine, 1)  // -> WalkAround
        4 (StopAtFirst):
            SetSubEngineMode(self.subEngine, 2)  // -> Push
        7:
            SetSubEngineMode(self.subEngine, 5)  // -> AutoCurrentLayer
        8:
            SetSubEngineMode(self.subEngine, 6)  // -> AutoMultiLayer
        _:
            return false

    if self.glossApplied == false:
        self.glossApplied = subMode
    return true
```

This reveals that **HugAndPush mode temporarily switches to WalkAround** for the gloss pass,
and **StopAtFirst temporarily switches to Push**. The original mode is saved in `glossApplied`
for restoration.

### Route Path Computation Dispatch

**FUN_038FD790** (at `0x038fd790`):

```
ComputeRoutePath(self, targetPoint, flag):
    if self.walkAroundMode:       // flag at 0x34A
        WalkAroundPathCompute(self, targetPoint)    // FUN_038FD3E0
    else:
        PrimaryRouteCompute(self, targetPoint)      // FUN_038FBEF0
```

### Walk-Around vs Push in the Route Segment Builder

**FUN_038FB680** (route segment builder, at `0x038fb680`) shows the full mode dispatch:

```
BuildRouteSegments(self, routeList, targetPoint, commitFlag, layerParam):
    saveCount = GetUndoCount(self.board) + 1

    isViaMode = IsViaMode(self)

    // PHASE 1: Follow-mouse mode (offset 0x6F flag)
    if self.followMouseTrail:
        FollowMousePath(self, self.startPoint, targetPoint, *routeList)
        if routeList.count > 0 AND GetRouteMode(self) is track/arc mode:
            // Check each segment for obstacle violations
            for each prim in routeList:
                violations = CheckDRCViolations(prim, obstacles, 1)
                if violations > 0:
                    ClearRoute(self, *routeList)
                    break

    // If no route segments and no pending via/push segments: fail
    if routeList.count == 0 AND pushList.count == 0 AND viaList.count == 0:
        return failure

    // PHASE 2: Handle two-segment (via) case
    if isViaMode:
        if NOT self.isDrag:
            // Check if start/end layers match
            startLayer = GetStartLayer(self.options)
            endLayer = GetEndLayer(self.options)
            if startLayer == endLayer:
                sameLayer = true
            else:
                sameLayer = CheckSingleSegment(*routeList)

            if sameLayer:
                // Compute via insertion point
                viaRoute = ComputeViaRoute(self, *routeList)

        // Call two-segment routing
        TwoSegmentViaRoute(self, routeList, targetPoint, layerParam)

        // Apply gloss if still in via mode
        if IsViaMode(self):
            ApplyGloss(self, layerParam, targetPoint, *routeList)
    else:
        // PHASE 3: Normal single-layer routing
        if NOT self.followMouseTrail
           AND GetRouteMode(self) is track mode
           AND NOT CanProceed(self):
            ApplyGloss(self, layerParam, targetPoint, *routeList)

    // PHASE 4: Apply route to board
    if routeList.count > 0 OR pushList.count > 0 OR viaList.count > 0:
        self.hasRoute = true
        BeginUndoGroup(self.board)

        if IsPushEnabled():
            ApplyPushConnections(self)

        success = ApplyRouteToBoard(self, *routeList, 0)

        if NOT success: return failure

        // Handle via insertion in two-segment mode
        if isViaMode AND NOT self.isDrag:
            builtPrims = BuildRoutedPrimitives(self, saveCount, 1)
            if builtPrims.count > 0:
                if NOT CheckViaSegment(self, builtPrims):
                    RestoreLastRoutePoint(self)
            // ... via segment management ...

        // Save route state for undo
        if commitFlag:
            SaveRouteState(self)

    return success
```

---

## Key Data Structures (New Findings)

### Push Engine Object Layout

At offset `0x90` of the push context:

| Offset | Field | Description |
|--------|-------|-------------|
| +0x08 | primitive | The primitive being pushed |
| +0x10 | clearance | Clearance value from rules |
| +0x14 | allowedDirections | Bitmask of valid push directions |
| +0x15 | hasRoutingRule | Whether a routing rule constrains push |
| +0x16 | pushEnabled | From rule: push feature enabled |
| +0x17 | allowViaPush | From rule: allow pushing vias |
| +0x18 | allowPadPush | From rule: allow pushing pads |
| +0x1C | layer | Current PCB layer |
| +0x20..0x5F | intervals[8] | TList<TIntervalRec> per direction |
| +0x60..0x67 | canPush[8] | Boolean per direction |
| +0x68 | pushMode | 0=forward, 1=backward |
| +0x69 | forceConstraint | Override constraint flag |
| +0x6A | noPush | Push completely disabled |
| +0x6B..0x72 | priority[8] | Direction indices in priority order |
| +0x90 | parent | Pointer to parent push context |
| +0xB0 | pushMode2 | Secondary push mode flag |

### Visibility Graph Node Runtime State

During Dijkstra execution, TGraphNode has these runtime fields:

| Offset | Field | Type | Description |
|--------|-------|------|-------------|
| +0x78 | prevNode | PGraphNode | Back-pointer for path reconstruction |
| +0x80 | visited | Boolean | True if already extracted from PQ |
| +0x84 | heapIndex | Integer | Position in priority queue (for decrease-key) |
| +0x88 | distance | Integer | Current shortest distance from source |
| +0x8C | arrivalPoint | TCoordPoint | Coordinate at which node was reached |

### Priority Queue

The Dijkstra implementation uses a **binary heap** with capacity 2000:
- Created via `FUN_01FD6110` with comparator `FUN_01FC38E0`
- `ExtractMin`: `FUN_038CE380`
- `DecreaseKey`/Update: `FUN_01FD57A0` (takes heap index)
- Insert: `FUN_038CE360`

---

## Updated Open Questions

1. **Exact contour-to-visibility-graph conversion**: How does the router convert TLRO_*
   obstacles into TContourDescr and then into TGraphNode visibility edges? The conversion
   functions are in the TPolygonPathFinder but were not fully decompiled (functions at
   `0x054d3b50`-`0x054d5510` range).

2. **Arc fitting in gloss**: The intermediate functions (`FUN_038D06E0`, `FUN_038D13D0`,
   `FUN_038D1520`) that find smooth points along the visibility graph need deeper analysis
   to understand how arc radius and tangent points are computed.

3. **Push interval computation**: How are the `intervals[8]` arrays in the push engine
   populated? The function chain `FUN_038E8610` -> constraint checking needs more analysis
   to understand the geometric interval calculation.

4. **HugAndPush algorithm details**: The combined hug-and-push mode (mode 3) first applies
   walk-around then selectively pushes. The transition logic between these two phases needs
   investigation.

5. **Differential pair push**: The differential pair router likely has its own push logic.
   The interaction between `TAdvancedDiffPairRoute` and the Pusher module is unknown.

6. **Via push vs track push**: The `allowViaPush`/`allowPadPush` flags suggest different
   push behaviors for different primitive types. The geometric constraints for via pushing
   (circular vs rectangular) need investigation.

7. **Real-time performance**: The router runs on every mouse move. The Dijkstra with
   capacity 2000 and the push algorithm with 8-direction search suggest careful optimization.
   The priority queue implementation and any spatial indexing for obstacle queries need
   investigation.
