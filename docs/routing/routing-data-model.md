# Routing Data Model — Altium Designer C# Interface Reference

Reverse-engineered from `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/` and
`AD26-dotnet/Altium.Edp.Interfaces/PCBInterfaces/`.

---

## Core Interfaces

### `IPCB_Track` (`RT_PCB/IPCB_Track.cs`, GUID `0C481AF6-283D-4D1C-B559-82C49DAA5EA3`)

Extends `IPCB_Primitive`. A track segment is the atomic routing primitive — a single
line segment from `(X1, Y1)` to `(X2, Y2)` on a given layer with a given width.

**Track-specific properties (beyond the IPCB_Primitive base):**

```csharp
int GetState_X1();
int GetState_Y1();
int GetState_X2();
int GetState_Y2();
int GetState_Width();
int GetState_Length();   // computed / settable
void SetState_X1(int);
void SetState_Y1(int);
void SetState_X2(int);
void SetState_Y2(int);
void SetState_Width(int);
void SetState_Length(int);
void RotateAroundXY(int argX, int argY, double angle);
```

**Routing-relevant flags inherited from `IPCB_Primitive`:**
- `GetState_UserRouted()` / `SetState_UserRouted(bool)` — user-placed vs autorouted
- `GetState_IsPreRoute()` / `SetState_IsPreRoute(bool)` — pre-route (ratsnest placeholder)
- `GetState_TearDrop()` / `SetState_TearDrop(bool)` — teardrop track
- `GetState_PolygonOutline()` — track used as polygon boundary
- `GetState_Net()` / `SetState_Net(IPCB_Net)` — net membership
- `GetState_Layer()` / `SetState_Layer(TV6_Layer)` — physical layer
- `GetState_InNet()` — whether the track belongs to a net

**Important:** There is no `IRoute` or `IRouteSegment` aggregate — Altium models routing
as individual track segments and vias stored flat in the board's primitive list, linked to
nets and components via index fields in their headers.

### `IPCB_Accordion` (`RT_PCB/IPCB_Accordion.cs`, GUID `CA18F5E3-3DA1-4608-9042-EC1AB243EEA8`)

A meander (serpentine) routing element. Extends `IPCB_Primitive`.

```csharp
int GetState_EstimateLength();
int GetState_ConnecitonLength();   // note: typo in original
int GetState_MaxAmplitude();
int GetState_AmplitudeIncrement();
int GetState_Gap();
int GetState_GapIncrement();
TAccordionStyle GetState_Style();
void SetState_MaxAmplitude(int);
void SetState_AmplitudeIncrement(int);
void SetState_Gap(int);
void SetState_GapIncrement(int);
void SetState_Style(TAccordionStyle);
void UpdateNetByPrimitive(IPCB_Primitive);
IPCB_AccordionMakerSettings GetState_Settings();
bool Rebuild();
```

### `IPCB_DifferentialPair` (`RT_PCB/IPCB_DifferentialPair.cs`, GUID `9600882E-3E1F-4BA3-82A7-94D96B49730C`)

Extends `IPCB_Primitive`. Represents a differential pair object on the board.

```csharp
string GetState_Name();
IPCB_Net GetState_PositiveNet();
IPCB_Net GetState_NegativeNet();
bool GetState_GatherControl();
void SetState_Name(string);
void SetState_PositiveNet(IPCB_Net);
void SetState_NegativeNet(IPCB_Net);
void SetState_GatherControl(bool);
void Reroute();
int GetState_PairAverageLength();
```

---

## Design Rule Interfaces

All design rules extend `IPCB_Rule` which itself extends `IPCB_Primitive`.

### `IPCB_Rule` (`RT_PCB/IPCB_Rule.cs`, GUID `3157B6E1-5212-4BE9-AF01-013E7DA372E2`)

Base interface for all PCB design rules.

```csharp
string GetState_Scope1Expression();     // scope filter expression (query language)
string GetState_Scope2Expression();     // second scope (for binary rules)
TRuleKind GetState_RuleKind();
TNetScope GetState_NetScope();
TRuleLayerKind GetState_LayerKind();
string GetState_Comment();
string GetState_Name();
bool GetState_DRCEnabled();
bool GetState_DefinedByLogicalDocument();
bool GetState_IsAdvanced();
ushort Priority();
bool ScopeKindIsValid(TScopeKind);
bool Scope1Includes(IPCB_Primitive);
bool Scope2Includes(IPCB_Primitive);
bool NetScopeMatches(IPCB_Primitive, IPCB_Primitive);
bool CheckBinaryScope(IPCB_Primitive, IPCB_Primitive);
bool CheckUnaryScope(IPCB_Primitive);
int GetState_CollisionExpansion();
IPCB_Violation ActualCheck(IPCB_Primitive, IPCB_Primitive);
bool ActualCheck(IPCB_Primitive, IPCB_Primitive, IInterfaceList violations);
bool IsUnary();
bool IsValid();
```

Extended by `IPCB_Rule1` which adds:
```csharp
void Import_FromParameters(StringBuilder);
ushort GetState_Priority();
void SetState_Priority(ushort);
string GetState_Data();
void SetState_Data(string);
bool CheckExpression(string);
```

### `IPCB_ClearanceConstraint` (`RT_PCB/IPCB_ClearanceConstraint.cs`, GUID `2D455AC5-0388-4723-BC5E-DEB9A53FAAC3`)

Clearance rule between two object classes. Adds to `IPCB_Rule`:

```csharp
int GetState_Gap();                    // default minimum clearance
void SetState_Gap(int);
bool PrimitivesViolate(IPCB_Primitive, IPCB_Primitive);
TClearanceConstraintMode GetState_Mode();
void SetClearance(IPCB_Primitive, IPCB_Primitive, int);
void SetClearance(TObjectClearanceId, TObjectClearanceId, int);
int GetClearance(IPCB_Primitive, IPCB_Primitive);
int GetClearance(TObjectClearanceId, TObjectClearanceId);
bool GetState_IgnorePadToPad();
void SetState_IgnorePadToPad(bool);
bool GetState_IsMatrix();
void SetState_IsMatrix(bool);
```

### `IPCB_MaxMinWidthConstraint` (`RT_PCB/IPCB_MaxMinWidthConstraint.cs`, GUID `1205B18E-827C-431C-ACE1-4F154E8AF5B5`)

Track width rule. Adds to `IPCB_Rule`:

```csharp
int GetState_MaxWidth(TV7_Layer);
int GetState_MinWidth(TV7_Layer);
int GetState_PreferedWidth(TV7_Layer);
bool GetState_ImpedanceDriven();
double GetState_MinImpedance();
double GetState_MaxImpedance();
double GetState_FavoredImpedance();
string GetState_ImpedanceProfileId();
void Invalidate();

// Per-substack variants for impedance-controlled impedance profiles
int GetState_MaxWidthAtSubStack(TV7_Layer, string stackID);
int GetState_MinWidthAtSubStack(TV7_Layer, string stackID);
int GetState_PreferedWidthAtSubStack(TV7_Layer, string stackID);
int GetState_PreferedWidth();     // global preferred width (fallback)
int GetState_MaxLimit();
int GetState_MinLimit();
bool IsLayerDefined(TV7_Layer, string stackID);
bool IsUniformMaxWidth(string LSID, out int width);
bool IsUniformMinWidth(string LSID, out int width);
bool IsUniformPreferredWidth(string LSID, out int width);
IPCB_ImpedanceProfile FindImpedanceProfileForRule(double impedance);
IPCB_ImpedanceProfile GetImpedanceProfile();
```

### `IPCB_RoutingViaStyleRule` (`RT_PCB/IPCB_RoutingViaStyleRule.cs`, GUID `A9946BC1-767D-444B-A10E-A4A038829F99`)

Via style rule for routing. Adds to `IPCB_Rule`:

```csharp
int GetState_MinHoleWidth();
int GetState_MaxHoleWidth();
int GetState_PreferedHoleWidth();
int GetState_MinWidth();
int GetState_MaxWidth();
int GetState_PreferedWidth();
TRouteVia GetState_ViaStyle();
bool GetState_UseViaTemplates();
void SetState_MinHoleWidth(int);
void SetState_MaxHoleWidth(int);
void SetState_PreferedHoleWidth(int);
void SetState_MinWidth(int);
void SetState_MaxWidth(int);
void SetState_PreferedWidth(int);
void SetState_ViaStyle(TRouteVia);
void SetState_UseViaTemplates(bool);
void DeleteAllViaTemplates();
void DeleteMissingViaTemplates();
void AddViaTemplate(string templateGUID, string templateName);
bool IsViaTemplateUsed(string templateGUID);
int GetViaTemplateCount();
IPCB_PadViaTemplate GetViaTemplate(int index);
int GetMissingViaTemplateCount();
void GetMissingViaTemplate(int index, out string templateGUID, out string templateName);
```

### `IPCB_RoutingLayersRule` (`RT_PCB/IPCB_RoutingLayersRule.cs`)

Permitted routing layers rule. Adds to `IPCB_Rule`:

```csharp
bool GetState_RoutingLayers(TV7_Layer signalLayer);
void SetState_RoutingLayers(TV7_Layer signalLayer, bool value);
void ResetRoutingLayers();
```

### `IPCB_RoutingCornerStyleRule` (`RT_PCB/IPCB_RoutingCornerStyleRule.cs`)

Corner style rule for routing. Adds to `IPCB_Rule`:

```csharp
TCornerStyle GetState_Style();
int GetState_MinSetBack();
int GetState_MaxSetBack();
void SetState_Style(TCornerStyle);
void SetState_MinSetBack(int);
void SetState_MaxSetBack(int);
```

### `IPCB_RoutingPriorityRule` (`RT_PCB/IPCB_RoutingPriorityRule.cs`)

Routing priority rule. Adds to `IPCB_Rule`:

```csharp
int GetState_RoutingPriority();
void SetState_RoutingPriority(int);
```

### `IPCB_RoutingTopologyRule` (`RT_PCB/IPCB_RoutingTopologyRule.cs`)

Net topology rule. Adds to `IPCB_Rule`:

```csharp
TNetTopology GetState_Topology();
void SetState_Topology(TNetTopology);
```

### `IPCB_RoutingNeckDownRule` (`RT_PCB/IPCB_RoutingNeckDownRule.cs`)

Neck-down rule (for SMD fanout). Adds to `IPCB_Rule`:

```csharp
IPCB_LayerToCoord GetState_MaxLength();
```

### `IPCB_FanoutControlRule` (`RT_PCB/IPCB_FanoutControlRule.cs`, GUID `9586B81E-E026-4DF0-975A-EC649EF8A218`)

BGA/SMD fanout routing rule. Adds to `IPCB_Rule`:

```csharp
TFanoutStyle GetState_FanoutStyle();
TFanoutDirection GetState_FanoutDirection();
TBGAFanoutDirection GetState_BGAFanoutDirection();
TBGAFanoutViaMode GetState_BGAFanoutViaMode();
int GetState_ViaGrid();
void SetState_FanoutStyle(TFanoutStyle);
void SetState_FanoutDirection(TFanoutDirection);
void SetState_BGAFanoutDirection(TBGAFanoutDirection);
void SetState_BGAFanoutViaMode(TBGAFanoutViaMode);
void SetState_ViaGrid(int);
```

### `IPCB_DifferentialPairsRoutingRule` (`RT_PCB/IPCB_DifferentialPairsRoutingRule.cs`)

Per-layer width and gap for diff pair routing. Adds to `IPCB_Rule`:

```csharp
int GetState_MaxWidth(TV7_Layer);
int GetState_MinWidth(TV7_Layer);
int GetState_PreferedWidth(TV7_Layer);
bool GetState_ImpedanceDriven();
double GetState_MinImpedance();
double GetState_MaxImpedance();
double GetState_FavoredImpedance();
string GetState_ImpedanceProfileId();
void Invalidate();
// Per-substack variants:
int GetState_MaxWidthAtSubStack(TV7_Layer, string stackID);
int GetState_MinWidthAtSubStack(TV7_Layer, string stackID);
int GetState_PreferedWidthAtSubStack(TV7_Layer, string stackID);
int GetState_MaxGapAtSubStack(TV7_Layer, string stackID);
int GetState_MinGapAtSubStack(TV7_Layer, string stackID);
int GetState_PreferedGapAtSubStack(TV7_Layer, string stackID);
void SetState_MaxGapAtSubStack(TV7_Layer, string stackID, int);
// ... setters mirroring getters
```

`IPCB_DifferentialPairsRoutingRule3` (latest version) adds:

```csharp
string GetState_FilterLayerStackID();
void SetState_MostFrequentWidth(int);
int GetState_MostFrequentWidth();
void SetState_MostFrequentGap(int);
int GetState_MostFrequentGap();
bool IsLayerDefined(TV7_Layer, string stackID);
bool IsUniformMaxWidth(string LSID, out int width);
IPCB_ImpedanceProfile FindImpedanceProfileForRule(double);
IPCB_ImpedanceProfile GetImpedanceProfile();
```

### `IPCB_MatchedNetLengthsConstraint` (`RT_PCB/IPCB_MatchedNetLengthsConstraint.cs`, GUID `D1D3EF35-0D26-4E46-BB06-B83B040366C4`)

Length-matching rule. Adds to `IPCB_Rule`:

```csharp
bool Get_CheckNetsInDiffPair();
bool Get_CheckDiffPairVsDiffPair();
bool Get_CheckOtherElectricalObjects();
bool Get_CheckBetweenXSignals();
bool GetState_UseDelayUnits();
double GetState_DelayTolerance();
void SetState_DelayTolerance(double);
string GetState_TargetSourceName();
bool GetState_PhaseMatching();
void SetState_PhaseMatching(bool);
int GetState_PhaseTolerance();
void SetState_PhaseTolerance(int);
double GetState_PhaseDelayTolerance();
int GetState_PhaseDistance();
```

### `IPCB_MaxMinLengthConstraint`

Track length constraint. Adds to `IPCB_Rule`:

```csharp
int GetState_MaxLimit();
int GetState_MinLimit();
bool GetState_UseDelayUnits();
double GetState_MaxDelay();
double GetState_MinDelay();
```

### `IPCB_ParallelSegmentConstraint`

Parallel-segment crosstalk constraint. Adds to `IPCB_Rule`:

```csharp
int GetState_Gap();    // parallel gap
int GetState_Limit();  // length limit
```

---

## Net and Topology Handling

### `IPCB_Net` (`RT_PCB/IPCB_Net.cs`)

Extends `IPCB_Primitive`. Key routing-relevant accessors:

```csharp
bool GetState_IsHighlighted();
bool GetState_LoopRemoval();
nint GetState_DifferentialPair();   // raw pointer to associated diff pair
bool GetState_InDifferentialPair();
TLiveHighlightMode GetState_LiveHighlightMode();
void SetState_Color(uint);
void SetState_Name(string);
void SetState_LoopRemoval(bool);
void SetState_DifferentialPair(nint);
void Rebuild();
IPCB_Group GetLogicalNet();
void SubnetIndices_Set();
void SubnetIndices_Reset();
IPCB_Group GetSubnets();
bool GetState_JumpersVisible();
```

### `TNetTopology` enum

```csharp
eNetTopology_Shortest,
eNetTopology_Horizontal,
eNetTopology_Vertical,
eNetTopology_DaisyChain_Simple,
eNetTopology_DaisyChain_MidDriven,
eNetTopology_DaisyChain_Balanced,
eNetTopology_Starburst
```

### `TNetScope` enum (PCB, `RT_PCB`)

Used on rules to specify which net relationship triggers the rule:

```csharp
eNetScope_DifferentNetsOnly,
eNetScope_SameNetOnly,
eNetScope_AnyNet,
eNetScope_DifferentDiffPairsOnly,
eNetScope_SameDiffPairOnly
```

---

## Via and Layer Handling

### `IPCB_ViaRoutingDataInfo` (GUID `9DE50BC7-4B73-4D40-89A6-B4EED227A529`)

Per-via routing data for a single layer pair:

```csharp
IPCB_DrillLayerPair GetState_DrillLayerPair();
IPCB_PadViaTemplate GetState_Template();
int GetState_ViaSize();
int GetState_ViaSizeOnLayer(TV7_Layer);
int GetState_HoleSize();
IPCB_LayerStack GetState_LayerStack();
IPCB_RoutingViaStyleRule GetState_Rule();
string GetState_Title();
TV7_Layer GetState_HighLayer();
TV7_Layer GetState_LowLayer();
TDrillLayerPairType GetState_PairType();
TViaType GetState_ViaType();
```

### `IPCB_RoutingViaStackInfo` (GUID `9975A39D-5E6B-4EC1-9637-6CB14F268F15`)

Collection of per-layer-pair via data objects for one via "stack":

```csharp
int GetState_ViaDataCount();
IPCB_ViaRoutingDataInfo GetState_ViaDataInfo(int index);
string GetState_Title();
IPCB_ViaRoutingDataInfo FirstViaDataInfo();
```

### `IPCB_ViaCombinationManagerInterface` (GUID `993F4C30-D23B-4A33-AB57-BCE3B6A7E510`)

Manages which via stack is active during interactive routing:

```csharp
int GetState_ViaStackCount();
IPCB_RoutingViaStackInfo GetState_ViaStack(int index);
void NextStack();
void PrevStack();
IPCB_RoutingViaStackInfo GetState_CurrentStack();
void SetState_CurrentStack(IPCB_RoutingViaStackInfo);
```

### `TViaType` enum

```csharp
InvalidVia,
Thru,
Blind,
Buried,
BackdrillHole,
MicroVia,
SkipVia
```

### `TRouteVia` enum

```csharp
eViaThruHole,
eViaBlindBuriedPair,
eViaBlindBuriedAny,
eViaNone
```

### `IPCB_DrillLayerPair` (GUID `937FB5DD-05B2-4644-919E-C7F8870D9B74`)

```csharp
TV6_Layer GetState_LowLayer();
TV6_Layer GetState_HighLayer();
IPCB_LayerObject GetState_StartLayer();
IPCB_LayerObject GetState_StopLayer();
IPCB_Board GetState_Board();
bool GetState_PlotDrillDrawing();
bool GetState_PlotDrillGuide();
```

### `IPCB_BoardLayerSet` (GUID `E586579B-2412-4CDE-8C7A-65548ED63505`)

```csharp
IPCB_LayerSet GetLayers();
string GetName();
```

### `IPCB_BoardRoutingOptions` (GUID `94C123E5-20D8-4932-9D75-B6BC99799944`)

Board-level layer enable/disable for routing:

```csharp
bool GetState_UseLayer(uint layerId);
bool GetState_ShowSignalLayersOnly();
void SetState_UseLayer(uint layerId, bool value);
void SetState_ShowSignalLayersOnly(bool);
void Clear();
string GetState_Value(string key);
void SetState_Value(string key, string value);
```

---

## Router Options / Configuration

### `IPCB_RoutingOptionsPage` (GUID `2131E587-7CA7-4E27-B01C-7C4D5CC9FFFE`)

Persistent interactive routing options (stored in system options):

```csharp
// Conflict resolution
bool GetState_ConflictModeEnabled(TAdvancedRouteMode);
TAdvancedRouteMode GetState_ConflictMode();

// Dragging
bool GetState_SmartDrag();
TAvoidObstacleMode GetState_DragAvoidObstacleMode();
TDragSelectUnselectMode GetState_DragUnselected();
TDragSelectUnselectMode GetState_Dragselected();
TVertexAction GetState_VertexAction();

// Component interaction
TPushMode GetState_ComponentPushing();
bool GetState_ComponentReroute();
bool GetState_ComponentMoveRelevantRouting();
int GetState_ComponentMoveRelevantRoutingPinsLimit();
TNetLineMode GetState_ComponentNetLineMode();

// Autorouting behaviour
bool GetState_AutoTerminateRouting();
bool GetState_AutoRemoveLoops();
bool GetState_AutoRemoveViaLoops();
bool GetState_AutoRemoveAntennas();
bool GetState_AllowViaPushing();

// Display
bool GetState_DisplayClearanceBounds();
bool GetState_ReduceClearanceDisplayArea();

// Width/via sizing
bool GetState_PickupWidthFromExistingRoutes();
TRoutingWidthMode GetState_RoutingWidthMode();
TRoutingWidthMode GetState_ViaSizeMode();

// Gloss and hugging
TGlossEffort GetState_GlossEffort();
TGlossEffort GetState_NeighborGlossEffort();
THuggingStyle GetState_HuggingStyle();
double GetState_MinimumArcSize();
double GetState_MiterSize();
int GetState_PadEntryStability();

// Diff pair
bool GetState_DifferentialPairMode();
bool GetState_DragWithMiters();
bool GetState_DragMergeParallel();
bool GetState_DifferentialPairModeForVias();

// Polygon/room avoidance
bool GetState_AvoidPolygons();
bool GetState_AvoidRooms();
bool GetState_PreservePath();

// Width/gap
int GetState_WidthToUse();
int GetState_DiffPairGapToUse();
double GetState_PreferredClearanceRatio();
bool GetState_PreferredClearanceApply();
bool GetState_PreferredClearanceAdjustVias();

// UI
bool GetState_ShowGaugeDuringDragging();
bool GetState_DisableTraceCenteringWhenDragging();
bool GetState_AutoNecking();
```

### `IPCB_InteractiveRoutingProcess` (GUID `B52BE2E5-F20C-4E12-9AA8-E2EEC4FA1EFB`)

The main interactive routing session. Extends `IPCB_CustomInteractiveRoutingProcess`.

Key state properties beyond the base:

```csharp
double GetState_Impedance();
bool ProjectAvailableForPinSwap(out string errorMessage);
void EditMaxMinWidthRule();
IPCB_MaxMinWidthConstraint GetState_WidthRule();
bool GetState_ShowLengthGauge();
IPCB_DifferentialPairsRoutingRule GetState_DiffPairRule();
IPCB_DifferentialPair GetState_DifferentialPair();
```

### `IPCB_CustomInteractiveRoutingProcess` — base routing process

```csharp
IPCB_Board GetState_Board();
bool GetState_AllowViaPushing();
bool GetState_AutoRemoveAntennas();
bool GetState_AutoRemoveLoops();
bool GetState_AutoTerminateRouting();
bool GetState_CornerRounding();
TV7_Layer GetState_CurrentLayer();
bool GetState_DisplayClearanceBounds();
bool GetState_FollowMouseTrail();
TGlossEffort GetState_GlossEffort();
int GetState_HoleSize();
IPCB_Net GetState_Net();
TV7_Layer GetState_NextLayer();
bool GetState_PickupWidthFromExistingRoutes();
bool GetState_PinSwapping();
bool GetState_RestrictTo9045();
TAdvancedRouteMode GetState_RouteMode();
bool GetState_RouteModeEnabled(TAdvancedRouteMode);
TRoutingCornerStyle GetState_RoutingCornerStyle();
TCoordPoint GetState_RoutingPoint();
IPCB_RoutingViaStyleRule GetState_RoutingViaStyleRule();
TRoutingWidthMode GetState_RoutingWidthMode();
int GetState_SubnetJumperLength();
int GetState_ViaDiameter();
int GetState_ViaLayerPair();
TRoutingWidthMode GetState_ViaSizeMode();
IPCB_PadViaTemplate GetState_ViaTemplate();
int GetState_Width();
TRoutingWidthMode NextRoutingWidthMode();
int NextViaLayerPair();
TRoutingWidthMode NextViaSizeMode();
IPCB_PadViaTemplate NextViaTemplate();
void EditRoutingViaStyleRule();
TRoutingWidthMode GetTrackWidthMode(bool checkPickupTrackWidth);
IPCB_ViaCombinationManagerInterface GetState_ViaCombinationManager();
bool GetState_FollowMode();
int GetState_PadEntryStability();
THuggingStyle GetState_HuggingStyle();
double GetState_MiterSize();
double GetState_MinimumArcSize();
IPCB_NetList GetState_NetList();
TGlossEffort GetState_NeighborGlossEffort();
bool GetState_LegacyRouter();
IPCB_RoutingOptionsPage GetRoutingOptions();
bool GetState_AutoNecking();
```

### `IPCB_InteractiveMultiRoutingProcess`

Multi-net bus routing session. Adds to the custom routing base:

```csharp
IPCB_MaxMinWidthConstraint GetState_WidthRule();
int GetState_BusSpacing();
int GetState_GetMinClearanceFromRule();
void EditMaxMinWidthRule();
```

### `IPCB_SlidingRoutingProcess`

Sliding (interactive push) routing session. Key additions:

```csharp
bool GetState_AllowViaPusing();   // note: typo in original
TGlossEffort GetState_GlossEffort();
THuggingStyle GetState_HuggingStyle();
TAdvancedRouteMode GetState_Sliding();
TVertexAction GetState_VertexAction();
IPCB_Net GetState_Net();
long GetState_NetLength();
double GetState_NetDelay();
double GetState_MinimumArcSize();
bool GetState_IsSingleNet();
IPCB_MaxMinWidthConstraint GetState_WidthRule();
IPCB_RoutingViaStyleRule GetState_RoutingViaStyleRule();
IPCB_MatchedNetLengthsConstraint GetState_MatchedNetLengthsRule();
TV7_Layer GetState_Layer();
void EditMaxMinWidthRule();
void EditRoutingViaStyleRule();
void EditMatchedNetLengthsRule();
TGlossEffort GetState_NeighborGlossEffort();
IPCB_RoutingOptionsPage GetRoutingOptions();
```

### `IPCB_SpecctraRouterOptions` (GUID `7C37270B-3551-40CF-A0F1-D6EE7F2E7331`)

Options for the Specctra autorouter (legacy):

```csharp
// Setback (clearance margins)
int GetState_Setback(int i);
bool GetState_DoSetback(int i);

// Bus routing
bool GetState_DoBus();
bool GetState_BusDiagonal();

// Grid
float GetState_WireGrid();
float GetState_ViaGrid();

// Via seeding
bool GetState_DoSeedVias();
int GetState_SeedViaLimit();

// Passes
int GetState_RoutePasses();
int GetState_CleanPasses();
int GetState_FilterPasses();

// Cost model (layer, wire-width, cross, via, off-grid, off-center, side-exit, squeeze)
TCCTCost GetState_LayerCost(TV6_Layer);
TCCTCost GetState_WwCost();
TCCTCost GetState_CrossCost();
TCCTCost GetState_ViaCost();
float GetState_LayerTax(TV6_Layer);
// ... (all cost/tax pairs)

// Optimisation passes
bool GetState_DoCritic();
bool GetState_DoMiter();
bool GetState_DoRecorner();

// Fanout
bool GetState_DoFanout();
bool GetState_FoPower();
bool GetState_FoSignal();
bool GetState_FoIn();
bool GetState_FoOut();
bool GetState_FoVias();
bool GetState_FoPads();
int GetState_FoPasses();
bool GetState_ForceVias();

// Spread
bool GetState_DoSpread();
TCCTSort GetState_SortKind();
TCCTSortDir GetState_SortDir();

// Version control
int GetState_SpVersion();
bool GetState_MinimizePads();
bool GetState_NoConflicts();
bool GetState_ProtectPreRoutes();
bool GetState_ReorderNets();
```

### `IPCB_AdvanceRouteCommands` (`PCBInterfaces/IPCB_AdvanceRouteCommands.cs`, GUID `F0831499-190D-4429-8B4A-6803D583FC7E`)

Low-level routing command API used by the router engine:

```csharp
void AddPrimitiveToBoard(IPCB_Primitive backConnected, IPCB_Primitive toAdd, IPCB_Primitive forwardConnected);
void RemovePrimitiveFromBoard(IPCB_Primitive);
void ReplaceConnectedPrimitivesInBoard(IPCB_Group toRemove, IPCB_Group toAdd);
IPCB_Group GetRoutedPath();
bool IsPushablePrimitive(IPCB_Primitive);
void SetState_BackConnectedPrim(IPCB_Primitive, IPCB_Primitive back);
IPCB_Primitive GetState_BackConnectedPrim(IPCB_Primitive);
void SetState_ForwardConnectedPrim(IPCB_Primitive, IPCB_Primitive forward);
IPCB_Primitive GetState_ForwardConnectedPrim(IPCB_Primitive);
int GetWidthFromRouter(IPCB_Primitive);
bool IsUsingAlternativeTargets();
TV6_Layer GetCurrentLayerFromRouter();
bool GetTargetPointForRoute(int routeIndex, ref TCoordPoint);
int GetRoutingFlags();
IPCB_Primitive GetTargetPrimitiveForRoute(int routeIndex);
TV7_Layer GetCurrentV7LayerFromRouter();
```

### `IPCB_AdvanceRouteParameters` (`PCBInterfaces/IPCB_AdvanceRouteParameters.cs`, GUID `2FB76A8A-FBEC-40EA-A58E-5FE0974E9788`)

Session-level routing parameters:

```csharp
IPCB_Primitive GetSingleRouteStartPrimitive();
uint GetLastHardCommitTimeStamp();
uint GetStartChangeTimeStamp();
IPCB_Group GetMultiRouteStartPrimitives();
int GetMultiRoutesCount();
```

---

## Meander / Serpentine Routing

### `TAccordionMode` enum

```csharp
eAccordionMode_Accordion,   // standard back-and-forth meander
eAccordionMode_Trombone,    // trombone-style U turns
eAccordionMode_Sawtooth,    // sawtooth/diagonal pattern
eAccordionMode_Root         // root element (container)
```

### `TAccordionStyle` enum

```csharp
asMittered45DegreeLines,   // 45-degree mitered corners
asMitteredArcs,             // arc-mitered corners
asRounded                   // fully rounded corners
```

### `IPCB_AccordionMakerSettings` (GUID `F8D5DAD3-0740-452C-81F9-E20BCEC72BF3`)

State for the accordion-maker tool:

```csharp
// State stack (for undo during interactive accordion editing)
IPCB_AccordionMakerSettings LastState();
void PushState();
void PopState();
void ClearLastState();

// Net context
IPCB_Net GetState_Net();
IPCB_Net GetState_LastSourceNet();
IPCB_DifferentialPair GetState_LastSourceDiffPair();

// Target length mode
TTargetLengthMode GetState_TargetLengthMode();

// Geometry
int GetState_Amplitude();
int GetState_Gap();
int GetState_AmplitudeIncrement();
int GetState_GapIncrement();
int GetState_TargetLength();
TAccordionStyle GetState_Style();

// Mode-specific
TAccordionMode GetState_AccordionMode();
bool GetState_RotationSnapping();
double GetState_SawtoothAngle();
int GetState_SawtoothWidth();
int GetState_SawtoothMinJoint();
int GetState_SawtoothMinHeight();
bool GetState_SawtoothFixedSize();
bool GetState_SingleSide();
bool GetState_UseTargetSource();
int GetState_MinAmplitude();
TAccordionMode GetState_AccordionMode2();
int GetState_OutputTracesCount();

// Serialization
string Serialize();
void Deserialize(string data);
void SetState_ImpExpPrefix(string prefix);
void ModeParameters_ImportFrom_SystemOptions();
void ModeParameters_ExportTo_SystemOptions();
```

### `TTargetLengthMode` enum

```csharp
eTargetLength_Manual,         // user-specified absolute length
eTargetLength_FromNet,        // derived from net's matched length rule
eTargetLength_FromRules,      // derived from board rules
eTargetLength_FromDiffPairs   // derived from diff pair routing
```

---

## Autorouter / Batch Routing

### `TAutorouterMode` enum (`xPCBTypes/TAutorouterMode.cs`)

```csharp
eAutorouteMode_Options,
eAutorouteMode_Pause,
eAutorouteMode_Restart,
eAutorouteMode_Stop,
eAutorouteMode_Start,
eAutorouteMode_Guide,
eAutorouteMode_Connection,
eAutorouteMode_Component,
eAutorouteMode_OnSelectedComponents,
eAutorouteMode_BetweenSelectedComponents,
eAutorouteMode_SingleComponent,
eAutorouteMode_Area,
eAutorouteMode_Border,
eAutorouteMode_ToMouse,
eAutorouteMode_Net,
eAutorouteMode_Room,
eAutorouteMode_SingleRoom,
eAutorouteMode_Pad,
PowerPlaneNets,
SignalNets,
eAutorouteMode_ComponentClass,
eAutorouteMode_NetClass,
eAutorouteMode_ExportData,
eAutorouteMode_RecordTestResults
```

### `TFanoutMode` enum (`xPCBTypes/TFanoutMode.cs`)

```csharp
eFanoutMode_All,
eFanoutMode_Connection,
eFanoutMode_Component,
eFanoutMode_SelectedComponents,
eFanoutMode_SingleComponent,
eFanoutMode_Net,
eFanoutMode_Room,
eFanoutMode_SingleRoom,
eFanoutMode_Pad,
eFanoutMode_PowerPlaneNets,
eFanoutMode_SignalNets
```

### `TAdvancedRouteMode` enum (also conflict resolution mode)

```csharp
eARIgnoreObstacle,
eARWalkAroundObstacle,
eARPushObstacle,
eARHugAndPushObstacle,
eARStopAtFirstObstacle,
eARAutoRouteCurrentLayer,
eARAutoRouteMultiLayer
```

### `TFanoutStyle` / `TFanoutDirection` / `TBGAFanoutDirection` / `TBGAFanoutViaMode`

```csharp
// TFanoutStyle
eFanoutStyle_Auto, eFanoutStyle_Rows, eFanoutStyle_Staggered, eFanoutStyle_BGA, eFanoutStyle_UnderPads

// TFanoutDirection
eFanoutDirection_None, eFanoutDirection_InOnly, eFanoutDirection_OutOnly,
eFanoutDirection_InThenOut, eFanoutDirection_OutThenIn, eFanoutDirection_Alternating

// TBGAFanoutDirection
eBGAFanoutDirection_Out, eBGAFanoutDirection_NE, eBGAFanoutDirection_SE,
eBGAFanoutDirection_SW, eBGAFanoutDirection_NW, eBGAFanoutDirection_In

// TBGAFanoutViaMode
eBGAFanoutVia_Closest, eBGAFanoutVia_Centered
```

---

## Rule Kind Enumeration (`TRuleKind`)

Complete list from `RT_PCB/TRuleKind.cs`:

```csharp
eRule_Clearance,
eRule_ParallelSegment,
eRule_MaxMinWidth,
eRule_MaxMinLength,
eRule_MatchedLengths,
eRule_DaisyChainStubLength,
eRule_PowerPlaneConnectStyle,
eRule_RoutingTopology,
eRule_RoutingPriority,
eRule_RoutingLayers,
eRule_RoutingCornerStyle,
eRule_RoutingViaStyle,
eRule_PowerPlaneClearance,
eRule_SolderMaskExpansion,
eRule_PasteMaskExpansion,
eRule_ShortCircuit,
eRule_BrokenNets,
eRule_ViasUnderSMD,
eRule_MaximumViaCount,
eRule_MinimumAnnularRing,
eRule_PolygonConnectStyle,
eRule_AcuteAngle,
eRule_ConfinementConstraint,
eRule_SMDToCorner,
eRule_ComponentClearance,
eRule_ComponentRotations,
eRule_PermittedLayers,
eRule_NetsToIgnore,
eRule_SignalStimulus,
eRule_Overshoot_FallingEdge,
eRule_Overshoot_RisingEdge,
eRule_Undershoot_FallingEdge,
eRule_Undershoot_RisingEdge,
eRule_MaxMinImpedance,
eRule_SignalTopValue,
eRule_SignalBaseValue,
eRule_FlightTime_RisingEdge,
eRule_FlightTime_FallingEdge,
eRule_LayerStack,
eRule_MaxSlope_RisingEdge,
eRule_MaxSlope_FallingEdge,
eRule_SupplyNets,
eRule_MaxMinHoleSize,
eRule_TestPointStyle,
eRule_TestPointUsage,
eRule_UnconnectedPin,
eRule_SMDToPlane,
eRule_SMDNeckDown,
eRule_LayerPair,
eRule_FanoutControl,
eRule_MaxMinHeight,
eRule_DifferentialPairsRouting,
eRule_HoleToHoleClearance,
eRule_MinimumSolderMaskSliver,
eRule_SilkToSolderMaskClearance,
eRule_SilkToSilkClearance,
eRule_NetAntennae,
eRule_AssyTestPointStyle,
eRule_AssyTestPointUsage,
eRule_SilkToBoardRegion,
eRule_SMDPADEntry,
eRule_None,
eRule_ModifiedPolygon,
eRule_BoardOutlineClearance,
eRule_BackDrilling,
eRule_Creepage,
eRule_ReturnPath,
eRule_RoutingNeckDown,
eRule_Wirebonding,
eRule_ZAxisClearance
```

### `TRuleSet`

Bitfield struct containing a set of `TRuleKind` values (9 bytes raw, covering all 73 rule
kinds). Used as filter when querying applicable rules.

---

## Routing-Related Enumerations Summary

| Enum | Values |
|------|--------|
| `TRoutingWidthMode` | `Default`, `Min`, `Preferred`, `Max` |
| `TRoutingCornerStyle` | `90`, `45`, `Any` |
| `TCornerStyle` (rule) | `90`, `45`, `Round` |
| `TGlossEffort` | `None`, `Weak`, `Strong` |
| `THuggingStyle` | `Mixed`, `Rounded`, `Degrees` |
| `TPlaceTrackMode` | `None`, `Any`, `9090`, `4590`, `90Arc` |
| `TRuleLayerKind` | `SameLayer`, `AdjacentLayer` |

---

## Observations / Open Questions

1. **No IRoute aggregate.** There is no `IRoute`, `IRoutePath`, or `IRouteSegment`
   interface. Routing is represented entirely as individual `IPCB_Track` segments and
   `IPCB_Via` primitives linked via their net/component/polygon indices. The "route" is
   conceptually the set of tracks and vias sharing a net.

2. **Rules are PCB primitives.** Every design rule (`IPCB_Rule`) extends `IPCB_Primitive`
   and lives in the board's primitive list, serialized the same as copper. This means rule
   order/priority is serializable as-is.

3. **Rule scope expressions.** `Scope1Expression` / `Scope2Expression` are strings in
   Altium's board query language (e.g., `"IsTrack"`, `"Net('GND')"`, `"InNet('VCC')"`,
   `"All"`). They are evaluated by the DRC engine at rule check time.

4. **Width rule is per-layer.** `IPCB_MaxMinWidthConstraint` stores `(min, max, preferred)`
   independently per `TV7_Layer` and per layer-stack-ID string. A "global" fallback
   is accessed with `GetState_PreferedWidth()` (no layer argument).

5. **Diff pair rule is per-layer.** Similarly, `IPCB_DifferentialPairsRoutingRule` stores
   width AND gap per layer, with per-substack overrides.

6. **Via templates.** `IPCB_RoutingViaStyleRule` supports both traditional min/max/preferred
   hole+annular constraints and the newer "via template" system. Templates are referenced
   by GUID string; the rule holds a list of template GUIDs.

7. **Accordion / meander is a first-class primitive.** `IPCB_Accordion` extends
   `IPCB_Primitive` and is stored directly in the board. Its child tracks are separate
   `IPCB_Track` primitives linked to it.

8. **Interactive routing options are separate from board-saved state.** `IPCB_RoutingOptionsPage`
   stores session/system preferences (not saved per-board), while `IPCB_BoardRoutingOptions`
   stores the per-board layer enable flags.

9. **`GetState_IsPreRoute` flag.** Tracks flagged as pre-route are ratsnest-level
   placeholders that have not yet been physically routed — they appear as airwires.

10. **Specctra router is the legacy batch autorouter.** The `IPCB_SpecctraRouterOptions`
    interface exposes the full Specctra DSN cost model. Modern Altium uses Situs
    (interactive) as the default autorouter; Specctra export/import may still be present
    for external tools.

11. **`IPCB_AdvanceRouteCommands` / `IPCB_AdvanceRouteParameters`** — these are the
    internal COM bridge used by the Delphi routing engine to communicate with the .NET
    board model. `GetRoutedPath()` returns the group of primitives making up the current
    route attempt. Connected-prim linked-list (`SetState_BackConnectedPrim` /
    `SetState_ForwardConnectedPrim`) forms the doubly-linked chain of segments in the
    in-progress route.

12. **Rule evaluation order.** Rules are prioritized (`ushort Priority()`). Lower numeric
    priority = higher precedence. The `IPCB_RuleManager` provides `SetRulePriority` and
    `UpdateRulePriorities` to reorder rules of a given kind.
