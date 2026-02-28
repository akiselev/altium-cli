# Altium Design Rules → Solverang Mapping

Complete mapping of all 70 Altium design rule types (TRuleKind 0–69) to
solverang constraint categories.

## Category Legend

| Category | Description | Solverang Role |
|----------|-------------|----------------|
| **GEOMETRIC** | Distance/dimension constraint, expressible as `f(x) ≥ 0` or `f(x) = 0` | Inequality or equality constraint |
| **LOGICAL** | Boolean/membership check, not a continuous function | Evaluated as pass/fail predicate, NOT a solver constraint |
| **ELECTRICAL** | Signal integrity / impedance / timing | Out of scope (requires SPICE-level simulation) |
| **PLACEMENT** | Directly relevant to component placement optimization | Priority constraint for autoplacer |
| **N/A** | Sentinel or unused | Skip |


## Complete Mapping (IDs 0–69)

### Clearance Rules (Geometric, Core)

| ID | Name | Category | Solverang Constraint | Residual |
|----|------|----------|---------------------|----------|
| 0 | **Clearance** | GEOMETRIC | `CopperClearance` | `dist(A, B) - gap ≥ 0` (inequality via slack) |
| 12 | **PowerPlaneClearance** | GEOMETRIC | `PlaneClearance` | `dist(obj, plane_edge) - clearance ≥ 0` |
| 24 | **ComponentClearance** | GEOMETRIC+PLACEMENT | `ComponentClearance` | `bbox_dist(A, B) - gap ≥ 0` |
| 52 | **HoleToHoleClearance** | GEOMETRIC | `HoleToHoleClearance` | `center_dist(h1, h2) - gap ≥ 0` |
| 63 | **BoardOutlineClearance** | GEOMETRIC+PLACEMENT | `BoardEdgeClearance` | `dist(obj, outline) - gap ≥ 0` |
| 65 | **Creepage** | GEOMETRIC | `CreepageClearance` | `creepage_dist(A, B) - gap ≥ 0` (surface distance, not straight-line) |
| 69 | **ZAxisClearance** | GEOMETRIC | `ZClearance` | `z_dist(A, B) - gap ≥ 0` (3D, height-based) |

**Note on Clearance (ID 0)**: This is the most complex rule. Parameters include:
- `GAP` — minimum clearance distance
- `COLLISIONCHECKMODE` — how collision geometry is computed (Multi-Layer, SameNetOnly, etc.)
- `VERTICALGAP` — vertical (3D) clearance component
- Scope expressions filter which object pairs to check

For the autoplacer, ComponentClearance (24) is more important than copper Clearance (0)
since placement operates on component bounding boxes, not individual copper features.


### Silkscreen / Mask Rules (Geometric, Manufacturing)

| ID | Name | Category | Solverang Constraint | Residual |
|----|------|----------|---------------------|----------|
| 13 | **SolderMaskExpansion** | GEOMETRIC | `MaskExpansion` | `expansion - target = 0` (equality, per pad) |
| 14 | **PasteMaskExpansion** | GEOMETRIC | `PasteExpansion` | `expansion - target = 0` (equality, per pad) |
| 53 | **MinimumSolderMaskSliver** | GEOMETRIC | `MaskSliverWidth` | `sliver_width - min ≥ 0` |
| 54 | **SilkToSolderMaskClearance** | GEOMETRIC | `SilkMaskClearance` | `dist(silk, mask_opening) - gap ≥ 0` |
| 55 | **SilkToSilkClearance** | GEOMETRIC | `SilkSilkClearance` | `dist(silk_A, silk_B) - gap ≥ 0` |
| 59 | **SilkToBoardRegionClearance** | GEOMETRIC | `SilkBoardClearance` | `dist(silk, board_edge) - gap ≥ 0` |


### Width / Length / Size Rules (Geometric, Bounds)

| ID | Name | Category | Solverang Constraint | Residual |
|----|------|----------|---------------------|----------|
| 2 | **Width** | GEOMETRIC | `TrackWidthBounds` | `width - min ≥ 0` AND `max - width ≥ 0` (two inequalities) |
| 3 | **Length** | GEOMETRIC | `NetLengthBounds` | `length - min ≥ 0` AND `max - length ≥ 0` |
| 5 | **DaisyChainStubLength** | GEOMETRIC | `StubLengthMax` | `max - stub_length ≥ 0` |
| 19 | **MinimumAnnularRing** | GEOMETRIC | `AnnularRingMin` | `(diameter - hole) / 2 - min ≥ 0` |
| 42 | **MaxMinHoleSize** | GEOMETRIC | `HoleSizeBounds` | `hole - min ≥ 0` AND `max - hole ≥ 0` |
| 50 | **MaxMinHeight** | GEOMETRIC+PLACEMENT | `HeightBounds` | `height - min ≥ 0` AND `max - height ≥ 0` |


### Via Rules (Geometric)

| ID | Name | Category | Solverang Constraint | Residual |
|----|------|----------|---------------------|----------|
| 11 | **RoutingViaStyle** | GEOMETRIC | `ViaBounds` | Width, hole width min/max bounds |
| 64 | **BackDrilling** | GEOMETRIC | — | Manufacturing process rule, not geometric |


### Pad / SMD Rules (Geometric)

| ID | Name | Category | Solverang Constraint | Residual |
|----|------|----------|---------------------|----------|
| 21 | **AcuteAngle** | GEOMETRIC | `MinAngle` | `angle(seg_A, seg_B) - min_angle ≥ 0` |
| 23 | **SmdToCorner** | GEOMETRIC | `SmdCornerClearance` | `dist(smd_pad, corner) - gap ≥ 0` |
| 46 | **SmdToPlane** | GEOMETRIC | `SmdPlaneClearance` | `dist(smd_pad, plane) - gap ≥ 0` |
| 47 | **SmdNeckDown** | GEOMETRIC | `NeckDown` | Neck-down width/length constraints |
| 49 | **FanoutControl** | GEOMETRIC | `Fanout` | Fanout trace geometry constraints |
| 60 | **SmdEntry** | GEOMETRIC | `SmdEntryAngle` | Entry angle constraints on SMD pads |
| 67 | **RoutingNeckDown** | GEOMETRIC | `RoutingNeckDown` | Neck-down from via to trace |


### Differential Pair Rules (Geometric)

| ID | Name | Category | Solverang Constraint | Residual |
|----|------|----------|---------------------|----------|
| 51 | **DiffPairsRouting** | GEOMETRIC | `DiffPairSpacing` | `gap - min ≥ 0`, `max - gap ≥ 0`, `skew ≤ max` |
| 4 | **MatchedLengths** | GEOMETRIC | `LengthMatch` | `|length_A - length_B| - tolerance ≤ 0` |


### Polygon / Plane Rules (Geometric + Logical)

| ID | Name | Category | Solverang Constraint | Residual |
|----|------|----------|---------------------|----------|
| 6 | **PowerPlaneConnectStyle** | LOGICAL | — | Configuration: Direct/Relief/NoConnect |
| 20 | **PolygonConnectStyle** | GEOMETRIC | `ThermalRelief` | Relief conductor width, entries, angle, air gap |


### Routing Rules (Logical, Autorouter Config)

| ID | Name | Category | Solverang Constraint | Notes |
|----|------|----------|---------------------|-------|
| 1 | **ParallelSegment** | LOGICAL | — | Parallel segment routing constraint (autorouter) |
| 7 | **RoutingTopology** | LOGICAL | — | Topology: shortest, horizontal, vertical, star, daisy |
| 8 | **RoutingPriority** | LOGICAL | — | Net routing priority ordering |
| 9 | **RoutingLayers** | LOGICAL | — | Which layers allow routing per net |
| 10 | **RoutingCornerStyle** | LOGICAL | — | 90° / 45° / any-angle corners |
| 26 | **PermittedLayers** | LOGICAL | — | Layer restrictions |


### Component Placement Rules (Logical + Geometric)

| ID | Name | Category | Solverang Constraint | Notes |
|----|------|----------|---------------------|-------|
| 22 | **ConfinementConstraint** | LOGICAL→GEOMETRIC | `RegionContainment` | Room definition — component must be inside a region. **Expressible as containment constraint!** |
| 25 | **ComponentRotations** | LOGICAL | — | Allowed rotation angles (discrete, not continuous) |


### Connectivity Rules (Logical)

| ID | Name | Category | Solverang Constraint | Notes |
|----|------|----------|---------------------|-------|
| 15 | **ShortCircuit** | LOGICAL | — | Unintended net connections (graph analysis) |
| 16 | **BrokenNets** | LOGICAL | — | Unrouted connections (graph analysis) |
| 17 | **ViasUnderSmd** | LOGICAL | — | Spatial query + boolean |
| 18 | **MaximumViaCount** | LOGICAL | — | Counting check per net |
| 27 | **NetsToIgnore** | LOGICAL | — | DRC exclusion filter |
| 45 | **UnconnectedPin** | LOGICAL | — | Connectivity check |
| 56 | **NetAntennae** | LOGICAL | — | Antenna effect detection |
| 62 | **UnpouredPolygon** | LOGICAL | — | State check |
| 66 | **ReturnPath** | LOGICAL | — | Return current path analysis |


### Test Point Rules (Logical)

| ID | Name | Category | Solverang Constraint | Notes |
|----|------|----------|---------------------|-------|
| 43 | **FabricationTestpointStyle** | LOGICAL | — | Test point geometry style |
| 44 | **FabricationTestpointUsage** | LOGICAL | — | Test point usage rules |
| 57 | **AssyTestPointStyle** | LOGICAL | — | Assembly test point style |
| 58 | **AssyTestPointUsage** | LOGICAL | — | Assembly test point usage |


### Layer Rules (Logical)

| ID | Name | Category | Solverang Constraint | Notes |
|----|------|----------|---------------------|-------|
| 38 | **LayerStack** | LOGICAL | — | Layer stackup configuration |
| 48 | **LayerPair** | LOGICAL | — | Via layer pair assignments |


### Electrical / Signal Integrity Rules (Out of Scope)

| ID | Name | Category | Notes |
|----|------|----------|-------|
| 28 | **SignalStimulus** | ELECTRICAL | SPICE stimulus definition |
| 29 | **OvershootFallingEdge** | ELECTRICAL | Overshoot % on falling edge |
| 30 | **OvershootRisingEdge** | ELECTRICAL | Overshoot % on rising edge |
| 31 | **UndershootFallingEdge** | ELECTRICAL | Undershoot % on falling edge |
| 32 | **UndershootRisingEdge** | ELECTRICAL | Undershoot % on rising edge |
| 33 | **MaxMinImpedance** | ELECTRICAL | Impedance bounds (ohms) |
| 34 | **SignalTopValue** | ELECTRICAL | Peak voltage limit |
| 35 | **SignalBaseValue** | ELECTRICAL | Base voltage limit |
| 36 | **FlightTimeRisingEdge** | ELECTRICAL | Rising edge flight time (ns) |
| 37 | **FlightTimeFallingEdge** | ELECTRICAL | Falling edge flight time (ns) |
| 39 | **MaxSlopeRisingEdge** | ELECTRICAL | Edge rate limit (V/ns) |
| 40 | **MaxSlopeFallingEdge** | ELECTRICAL | Edge rate limit (V/ns) |
| 41 | **SupplyNets** | LOGICAL/ELECTRICAL | Supply net identification |


### Miscellaneous

| ID | Name | Category | Notes |
|----|------|----------|-------|
| 61 | **None** | N/A | Sentinel value, no rule |
| 68 | **WireBonding** | GEOMETRIC | Specialized (BGA/CSP wirebond) |


## Summary Statistics

| Category | Count | Solverang-Applicable |
|----------|-------|---------------------|
| GEOMETRIC (distance/dimension) | 32 | **32** (all expressible as constraints) |
| LOGICAL (boolean/membership) | 22 | **1** (ConfinementConstraint → RegionContainment) |
| ELECTRICAL (signal integrity) | 13 | **0** (requires SPICE) |
| PLACEMENT (directly for autoplacer) | 4 | **4** (Clearance, BoardOutline, Height, Confinement) |
| N/A | 2 | 0 |
| **Total** | **70** | **33** |

**Bottom line**: 33 of 70 rules can be expressed as solverang geometric constraints.
The remaining 37 are logical checks (evaluated as predicates) or electrical analyses
(out of scope).


## Priority for Implementation

### Phase 1: Placement-Critical (must have for autoplacer)

1. **ComponentClearance** (24) — minimum distance between component bounding boxes
2. **BoardOutlineClearance** (63) — components inside board with margin
3. **ConfinementConstraint** (22) — room/region containment
4. **ComponentRotations** (25) — allowed rotations (discrete, handled outside solver)

### Phase 2: Core DRC (most common violations)

5. **Clearance** (0) — copper-to-copper clearance
6. **Width** (2) — track width bounds
7. **HoleToHoleClearance** (52) — drill spacing
8. **MinimumAnnularRing** (19) — annular ring width
9. **SilkToSolderMaskClearance** (54) — silkscreen clearance
10. **SilkToSilkClearance** (55) — silkscreen spacing

### Phase 3: Manufacturing DRC

11. **SolderMaskExpansion** (13)
12. **PasteMaskExpansion** (14)
13. **MinimumSolderMaskSliver** (53)
14. **MaxMinHoleSize** (42)
15. **SmdToCorner** (23)

### Phase 4: Routing DRC

16. **AcuteAngle** (21)
17. **DiffPairsRouting** (51)
18. **MatchedLengths** (4)
19. **Length** (3)
20. **RoutingViaStyle** (11)

### Phase 5: Advanced

21–33: Remaining geometric rules (SmdToPlane, NeckDown, Fanout, Creepage, etc.)


## Scope Expressions

Altium rules use scope expressions to select which objects they apply to:

```
SCOPE1EXPRESSION=InNet('GND')
SCOPE2EXPRESSION=All
NETSCOPE=DifferentNets
LAYERKIND=SameLayer
```

For solverang, scope filtering happens BEFORE constraint generation:
1. Parse scope expressions
2. Evaluate against the object set to get (object_A, object_B) pairs
3. Generate one constraint per pair
4. Solver sees only the generated constraints (no scope awareness)

This keeps the solver domain-agnostic — all Altium-specific logic is in the
constraint generation layer.
