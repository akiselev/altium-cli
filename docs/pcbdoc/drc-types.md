# PcbDoc DRC Types Reference

Complete reference for DRC (Design Rule Check) rules, violations, and supporting types
in the Altium PcbDoc file format, sourced from the decompiled C# interfaces.

**Base directory for all C# interfaces:**
`AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/`

---

## Table of Contents

- [Rules6 Section Format](#rules6-section-format)
- [IPCB_Rule Base Interface](#ipcb_rule-base-interface)
- [Rule Kind Enum (TRuleKind)](#trulekind-enum)
- [Concrete Rule Types](#concrete-rule-types)
- [Violation Base Interface](#ipcb_violation-base-interface)
- [Concrete Violation Types](#concrete-violation-types)
- [DRC Result Interfaces](#drc-result-interfaces)
- [Waived Violation Interfaces](#waived-violation-interfaces)
- [Supporting Enums](#supporting-enums)
- [Serialization Constants](#serialization-constants)

---

## Rules6 Section Format

Rules are stored in the `Rules6` CFB storage as **PrefixedParamRecords**:
- `Header`: 4-byte u32le record count
- `Data`: records, each prefixed with u16le, then standard block-encoded `|KEY=VALUE|` params

Common parameters shared by all rules:

| Parameter | Type | Source |
|-----------|------|--------|
| `RULEKIND` | string | cRuleIdStrings (see [serialization constants](#cRuleIdStrings)) |
| `NETSCOPE` | string | cNetScopeStrings |
| `LAYERKIND` | string | cRuleLayerKindStrings |
| `SCOPE1EXPRESSION` | string | Query expression |
| `SCOPE2EXPRESSION` | string | Query expression |
| `NAME` | string | Unique rule name |
| `ENABLED` | bool | `"TRUE"` / `"FALSE"` |
| `PRIORITY` | u16 | Lower = higher priority |
| `COMMENT` | string | Optional |
| `UNIQUEID` | string | 8-char ID |
| `DEFINEDBYLOGICALDOCUMENT` | bool | If schematic-defined |

---

## IPCB_Rule Base Interface

**File:** `IPCB_Rule.cs`, lines 329-416
**Extends:** `IPCB_Primitive`

```csharp
// Scope
string GetState_Scope1Expression();
string GetState_Scope2Expression();
void SetState_Scope1Expression(string argValue);
void SetState_Scope2Expression(string argValue);

// Kind and classification
TRuleKind GetState_RuleKind();
void SetState_RuleKind(TRuleKind argValue);
TNetScope GetState_NetScope();
void SetState_NetScope(TNetScope argValue);
TRuleLayerKind GetState_LayerKind();
void SetState_LayerKind(TRuleLayerKind argValue);

// Identity
string GetState_Comment();
void SetState_Comment(string argValue);
string GetState_Name();
void SetState_Name(string argValue);

// Flags
bool GetState_DRCEnabled();
void SetState_DRCEnabled(bool argValue);
bool GetState_DefinedByLogicalDocument();
void SetState_DefinedByLogicalDocument(bool argValue);
bool GetState_IsAdvanced();
void SetState_IsAdvanced(bool argValue);

// Priority
ushort Priority();

// Scope checking (runtime)
bool ScopeKindIsValid(TScopeKind argScopeKind);
bool Scope1Includes(IPCB_Primitive argP);
bool Scope2Includes(IPCB_Primitive argP);
bool NetScopeMatches(IPCB_Primitive argP1, IPCB_Primitive argP2);
bool CheckBinaryScope(IPCB_Primitive argP1, IPCB_Primitive argP2);
bool CheckUnaryScope(IPCB_Primitive argP);
bool IsUnary();

// Display
int GetState_CollisionExpansion();
string GetState_DataSummaryString();
string GetState_ShortDescriptorString();
string GetState_ScopeDescriptorString();
string GetState_ViolationDescriptorString(IPCB_Violation argV);
string GetWhatsThisHelpString();
bool IsValid();

// Check execution
IPCB_Violation ActualCheck(IPCB_Primitive argP1, IPCB_Primitive argP2);
bool ActualCheck(IPCB_Primitive argP1, IPCB_Primitive argP2, IInterfaceList argViolations);

// Serialization
void Export_ToParameters_IPCB_Rule(StringBuilder argParams);
```

### IPCB_Rule1 (Extended)

**File:** `IPCB_Rule1.cs`, lines 418-429
**Extends:** `IPCB_Rule, IPCB_Primitive`

```csharp
void Import_FromParameters(StringBuilder argParameters);
ushort GetState_Priority();
void SetState_Priority(ushort argPriority);
string GetState_Data();
void SetState_Data(string argValue);
bool CheckExpression(string argExpression);
```

---

## TRuleKind Enum

**File:** `TRuleKind.cs` — `byte`, 70 variants (0-69)

| Value | Name | RULEKIND String |
|-------|------|-----------------|
| 0 | `eRule_Clearance` | `"Clearance"` |
| 1 | `eRule_ParallelSegment` | `"ParallelSegment"` |
| 2 | `eRule_MaxMinWidth` | `"Width"` |
| 3 | `eRule_MaxMinLength` | `"Length"` |
| 4 | `eRule_MatchedLengths` | `"MatchedLengths"` |
| 5 | `eRule_DaisyChainStubLength` | `"StubLength"` |
| 6 | `eRule_PowerPlaneConnectStyle` | `"PlaneConnect"` |
| 7 | `eRule_RoutingTopology` | `"RoutingTopology"` |
| 8 | `eRule_RoutingPriority` | `"RoutingPriority"` |
| 9 | `eRule_RoutingLayers` | `"RoutingLayers"` |
| 10 | `eRule_RoutingCornerStyle` | `"RoutingCorners"` |
| 11 | `eRule_RoutingViaStyle` | `"RoutingVias"` |
| 12 | `eRule_PowerPlaneClearance` | `"PlaneClearance"` |
| 13 | `eRule_SolderMaskExpansion` | `"SolderMaskExpansion"` |
| 14 | `eRule_PasteMaskExpansion` | `"PasteMaskExpansion"` |
| 15 | `eRule_ShortCircuit` | `"ShortCircuit"` |
| 16 | `eRule_BrokenNets` | `"UnRoutedNet"` |
| 17 | `eRule_ViasUnderSMD` | `"ViasUnderSMD"` |
| 18 | `eRule_MaximumViaCount` | `"MaximumViaCount"` |
| 19 | `eRule_MinimumAnnularRing` | `"MinimumAnnularRing"` |
| 20 | `eRule_PolygonConnectStyle` | `"PolygonConnect"` |
| 21 | `eRule_AcuteAngle` | `"AcuteAngle"` |
| 22 | `eRule_ConfinementConstraint` | `"RoomDefinition"` |
| 23 | `eRule_SMDToCorner` | `"SMDToCorner"` |
| 24 | `eRule_ComponentClearance` | `"ComponentClearance"` |
| 25 | `eRule_ComponentRotations` | `"ComponentOrientations"` |
| 26 | `eRule_PermittedLayers` | `"PermittedLayers"` |
| 27 | `eRule_NetsToIgnore` | `"NetsToIgnore"` |
| 28 | `eRule_SignalStimulus` | `"SignalStimulus"` |
| 29 | `eRule_Overshoot_FallingEdge` | `"OvershootFalling"` |
| 30 | `eRule_Overshoot_RisingEdge` | `"OvershootRising"` |
| 31 | `eRule_Undershoot_FallingEdge` | `"UndershootFalling"` |
| 32 | `eRule_Undershoot_RisingEdge` | `"UndershootRising"` |
| 33 | `eRule_MaxMinImpedance` | `"MaxMinImpedance"` |
| 34 | `eRule_SignalTopValue` | `"SignalTopValue"` |
| 35 | `eRule_SignalBaseValue` | `"SignalBaseValue"` |
| 36 | `eRule_FlightTime_RisingEdge` | `"FlightTimeRising"` |
| 37 | `eRule_FlightTime_FallingEdge` | `"FlightTimeFalling"` |
| 38 | `eRule_LayerStack` | `"LayerStack"` |
| 39 | `eRule_MaxSlope_RisingEdge` | `"SlopeRising"` |
| 40 | `eRule_MaxSlope_FallingEdge` | `"SlopeFalling"` |
| 41 | `eRule_SupplyNets` | `"SupplyNets"` |
| 42 | `eRule_MaxMinHoleSize` | `"HoleSize"` |
| 43 | `eRule_TestPointStyle` | `"FabricationTestpoint"` |
| 44 | `eRule_TestPointUsage` | `"FabricationTestPointUsage"` |
| 45 | `eRule_UnconnectedPin` | `"UnConnectedPin"` |
| 46 | `eRule_SMDToPlane` | `"SMDToPlane"` |
| 47 | `eRule_SMDNeckDown` | `"SMDNeckDown"` |
| 48 | `eRule_LayerPair` | `"LayerPairs"` |
| 49 | `eRule_FanoutControl` | `"FanoutControl"` |
| 50 | `eRule_MaxMinHeight` | `"Height"` |
| 51 | `eRule_DifferentialPairsRouting` | `"DiffPairsRouting"` |
| 52 | `eRule_HoleToHoleClearance` | `"HoleToHoleClearance"` |
| 53 | `eRule_MinimumSolderMaskSliver` | `"MinimumSolderMaskSliver"` |
| 54 | `eRule_SilkToSolderMaskClearance` | `"SilkToSolderMaskClearance"` |
| 55 | `eRule_SilkToSilkClearance` | `"SilkToSilkClearance"` |
| 56 | `eRule_NetAntennae` | `"NetAntennae"` |
| 57 | `eRule_AssyTestPointStyle` | `"AssemblyTestpoint"` |
| 58 | `eRule_AssyTestPointUsage` | `"AssemblyTestPointUsage"` |
| 59 | `eRule_SilkToBoardRegion` | `"SilkToBoardRegionClearance"` |
| 60 | `eRule_SMDPADEntry` | `"SMDEntry"` |
| 61 | `eRule_None` | `"None"` |
| 62 | `eRule_ModifiedPolygon` | `"UnpouredPolygon"` |
| 63 | `eRule_BoardOutlineClearance` | `"BoardOutlineClearance"` |
| 64 | `eRule_BackDrilling` | `"BackDrilling"` |
| 65 | `eRule_Creepage` | `"Creepage"` |
| 66 | `eRule_ReturnPath` | `"ReturnPath"` |
| 67 | `eRule_RoutingNeckDown` | `"RoutingNeckDown"` |
| 68 | `eRule_Wirebonding` | `"WireBonding"` |
| 69 | `eRule_ZAxisClearance` | `"ZAxisClearance"` |

**Source:** cRuleIdStrings at `RT_PCB/Consts.cs` lines 1121-1192.
Display names: cRuleStrings lines 1193-1264. Tree view names: cRuleTreeStrings lines 1265-1336.

---

## Concrete Rule Types

Each rule type extends `IPCB_Rule` (and `IPCB_Primitive`). Only type-specific members
are listed below (all inherit the base IPCB_Rule members above).

### Clearance (eRule_Clearance = 0)

**Interface:** `IPCB_ClearanceConstraint` — `IPCB_ClearanceConstraint.cs` L11
**Key params:** `GAP`, `GENERICCLEARANCE`, `OBJECTCLEARANCES`, `IGNOREPADTOPADCLEARANCEINFOOTPRINT`

```csharp
int GetState_Gap();                                          // L418
void SetState_Gap(int argValue);                             // L420
TClearanceConstraintMode GetState_Mode();                    // L425
void SetClearance(TObjectClearanceId o1, TObjectClearanceId o2, int val); // L429
int GetClearance(TObjectClearanceId o1, TObjectClearanceId o2);           // L433
bool GetState_IgnorePadToPad();                              // L436
void SetState_IgnorePadToPad(bool argValue);                 // L438
bool GetState_IsMatrix();                                    // L441
void SetState_IsMatrix(bool argValue);                       // L443
```

Inheritance chain: `ClearanceConstraint` → `ClearanceGapByLayerConstraint` → `ClearanceMatrixConstraint`

### Parallel Segment (eRule_ParallelSegment = 1)

**Interface:** `IPCB_ParallelSegmentConstraint` — `IPCB_ParallelSegmentConstraint.cs` L11

```csharp
int GetState_Gap();              // L418
int GetState_Limit();            // L420
void SetState_Gap(int val);      // L422
void SetState_Limit(int val);    // L424
```

### Width (eRule_MaxMinWidth = 2)

**Interface:** `IPCB_MaxMinWidthConstraint` — `IPCB_MaxMinWidthConstraint.cs` L11
**Key params:** `MINLIMIT`, `MAXLIMIT`, `PREFEREDWIDTH`

```csharp
int GetState_MaxWidth(TV7_Layer argL);                       // L418
int GetState_MinWidth(TV7_Layer argL);                       // L420
int GetState_FavoredWidth(TV7_Layer argL);                   // L422
bool GetState_ImpedanceDriven();                             // L425
double GetState_MinImpedance();                              // L427
double GetState_MaxImpedance();                              // L429
double GetState_FavoredImpedance();                          // L431
bool GetState_CheckConnectedCopper();                        // L434
string GetState_ImpedanceProfileId();                        // L452
// Per-substack overloads: GetState_MaxWidthAtSubStack(), etc.
void SetState_PreferedWidth(int argValue);                   // L477
int GetState_PreferedWidth();                                // L479
void SetState_MaxLimit(int argValue);                        // L481
int GetState_MaxLimit();                                     // L483
void SetState_MinLimit(int argValue);                        // L485
int GetState_MinLimit();                                     // L487
```

### Length (eRule_MaxMinLength = 3)

**Interface:** `IPCB_MaxMinLengthConstraint` — `IPCB_MaxMinLengthConstraint.cs` L11

```csharp
int GetState_MaxLimit();                 // L418
int GetState_MinLimit();                 // L420
bool GetState_UseDelayUnits();           // L427
double GetState_MaxDelay();              // L429
double GetState_MinDelay();              // L431
```

### Matched Lengths (eRule_MatchedLengths = 4)

**Interface:** `IPCB_MatchedNetLengthsConstraint` — `IPCB_MatchedNetLengthsConstraint.cs` L11

```csharp
int GetState_Amplitude();                    // L418
int GetState_Gap();                          // L420
TLengthenerStyle GetState_Style();           // L422
int GetState_Tolerance();                    // L424
bool GetState_UseDelayUnits();               // L461
double GetState_DelayTolerance();            // L463
string GetState_TargetSourceName();          // L469
bool GetState_PhaseMatching();               // L474
int GetState_PhaseTolerance();               // L478
double GetState_PhaseDelayTolerance();       // L482
int GetState_PhaseDistance();                // L486
```

### Stub Length (eRule_DaisyChainStubLength = 5)

**Interface:** `IPCB_DaisyChainStubLengthConstraint` — `IPCB_DaisyChainStubLengthConstraint.cs` L11

```csharp
int GetState_Limit();            // L418
```

### Power Plane Connect Style (eRule_PowerPlaneConnectStyle = 6)

**Interface:** `IPCB_PowerPlaneConnectStyleRule` — `IPCB_PowerPlaneConnectStyleRule.cs` L11

```csharp
TPlaneConnectStyle GetState_PlaneConnectStyle();             // L418
int GetState_ReliefExpansion();                              // L420
int GetState_ReliefConductorWidth();                         // L422
int GetState_ReliefEntries();                                // L424
int GetState_ReliefAirGap();                                 // L426
bool GetState_SamePadAndViaParams();                         // L439
// ByType overloads for TPlaneConnectPrimitiveType
```

### Routing Topology (eRule_RoutingTopology = 7)

**Interface:** `IPCB_RoutingTopologyRule` — `IPCB_RoutingTopologyRule.cs` L11

```csharp
TNetTopology GetState_Topology();            // L418
```

### Routing Priority (eRule_RoutingPriority = 8)

**Interface:** `IPCB_RoutingPriorityRule` — `IPCB_RoutingPriorityRule.cs` L11

```csharp
int GetState_RoutingPriority();              // L418
```

### Routing Layers (eRule_RoutingLayers = 9)

**Interface:** `IPCB_RoutingLayersRule` — `IPCB_RoutingLayersRule.cs` L11

```csharp
bool GetState_RoutingLayers(TV7_Layer argSignalLayer);       // L419
void ResetRoutingLayers();                                    // L423
```

### Routing Corner Style (eRule_RoutingCornerStyle = 10)

**Interface:** `IPCB_RoutingCornerStyleRule` — `IPCB_RoutingCornerStyleRule.cs` L11

```csharp
TCornerStyle GetState_Style();               // L418
int GetState_MinSetBack();                   // L420
int GetState_MaxSetBack();                   // L422
```

### Routing Via Style (eRule_RoutingViaStyle = 11)

**Interface:** `IPCB_RoutingViaStyleRule` — `IPCB_RoutingViaStyleRule.cs` L11
**Key params:** `HOLEWIDTH`, `WIDTH`, `VIASTYLE`, `MINHOLEWIDTH`, `MAXHOLEWIDTH`, `MINWIDTH`, `MAXWIDTH`

```csharp
int GetState_MinHoleWidth();                 // L418
int GetState_MaxHoleWidth();                 // L420
int GetState_PreferedHoleWidth();            // L422
int GetState_MinWidth();                     // L424
int GetState_MaxWidth();                     // L426
int GetState_PreferedWidth();                // L428
TRouteVia GetState_ViaStyle();               // L430
bool GetState_UseViaTemplates();             // L433
int GetViaTemplateCount();                   // L460
IPCB_PadViaTemplate GetViaTemplate(int i);   // L463
```

### Power Plane Clearance (eRule_PowerPlaneClearance = 12)

**Interface:** `IPCB_PowerPlaneClearanceRule` — `IPCB_PowerPlaneClearanceRule.cs` L11

```csharp
int GetState_Clearance();                    // L418
```

### Solder Mask Expansion (eRule_SolderMaskExpansion = 13)

**Interface:** `IPCB_SolderMaskExpansionRule` — `IPCB_SolderMaskExpansionRule.cs` L11

```csharp
int GetState_Expansion();                            // L418
int GetState_ExpansionBottom();                      // L422
bool GetState_FromHoleEdge();                        // L427
bool GetState_UseSeparateExpansions();               // L432
```

### Paste Mask Expansion (eRule_PasteMaskExpansion = 14)

**Interface:** `IPCB_PasteMaskExpansionRule` — `IPCB_PasteMaskExpansionRule.cs` L11

```csharp
int GetState_Expansion();                    // L418
bool GetState_UsePaste();                    // L423
bool GetState_UsePercent();                  // L428
double GetState_Percent();                   // L432
bool GetState_THPadUseTopPaste();            // L437
bool GetState_THPadUseBottomPaste();         // L442
```

### Short Circuit (eRule_ShortCircuit = 15)

**Interface:** `IPCB_ShortCircuitConstraint` — `IPCB_ShortCircuitConstraint.cs` L11

```csharp
bool GetState_Allowed();                     // L419
```

### Broken Nets / Unrouted Net (eRule_BrokenNets = 16)

**Interface:** `IPCB_BrokenNetRule` — `IPCB_BrokenNetRule.cs` L11

```csharp
bool GetState_HighlightPolygons();           // L419
bool GetState_CheckBadConnections();         // L424
```

### Vias Under SMD (eRule_ViasUnderSMD = 17)

**Interface:** `IPCB_ViasUnderSMDConstraint` — `IPCB_ViasUnderSMDConstraint.cs` L11

```csharp
bool GetState_Allowed();                     // L419
```

### Maximum Via Count (eRule_MaximumViaCount = 18)

**Interface:** `IPCB_MaximumViaCountRule` — `IPCB_MaximumViaCountRule.cs` L11

```csharp
int GetState_Limit();                        // L418
```

### Minimum Annular Ring (eRule_MinimumAnnularRing = 19)

**Interface:** `IPCB_MinimumAnnularRing` — `IPCB_MinimumAnnularRing.cs` L11

```csharp
int GetState_Minimum();                      // L418
```

### Polygon Connect Style (eRule_PolygonConnectStyle = 20)

**Interface:** `IPCB_PolygonConnectStyleRule` — `IPCB_PolygonConnectStyleRule.cs` L11
**Key params:** `CONNECTSTYLE`, `RELIEFCONDUCTORWIDTH`, `RELIEFENTRIES`, `POLYGONRELIEFANGLE`, `AIRGAPWIDTH`

```csharp
TPlaneConnectStyle GetState_ConnectStyle();                  // L418
int GetState_ReliefConductorWidth();                         // L420
int GetState_ReliefEntries();                                // L422
TPolygonReliefAngle GetState_PolygonReliefAngle();           // L424
int GetState_ReliefAirGap();                                 // L434
bool GetState_SamePadAndViaParams();                         // L439
// ByType overloads for TPolygonConnectPrimitiveType (THPad, SMDPad, Via)
int GetState_MinDistance();                                   // L461
bool GetState_EnableMinDistance();                            // L464
bool GetState_ConductorByPadEdge();                          // L480
```

### Acute Angle (eRule_AcuteAngle = 21)

**Interface:** `IPCB_AcuteAngle` — `IPCB_AcuteAngle.cs` L11

```csharp
double GetState_Minimum();                   // L418
bool GetState_CheckTracksOnly();             // L423
```

### Confinement / Room Definition (eRule_ConfinementConstraint = 22)

**Interface:** `IPCB_ConfinementConstraint` — `IPCB_ConfinementConstraint.cs` L11

```csharp
int GetState_XLocation();                    // L418
int GetState_YLocation();                    // L420
TConfinementStyle GetState_Kind();           // L422 — eConfineIn(0) or eConfineOut(1)
TV6_Layer GetState_ConstraintLayer();        // L424
TCoordRect GetState_BoundingRectangle();     // L426
int GetState_PointCount();                   // L428
TPolySegment GetState_Segments(int i);       // L430
bool GetState_LockComponents();              // L433
```

### SMD To Corner (eRule_SMDToCorner = 23)

**Interface:** `IPCB_SMDToCornerConstraint` — `IPCB_SMDToCornerConstraint.cs` L11

```csharp
int GetState_Distance();                     // L418
```

### Component Clearance (eRule_ComponentClearance = 24)

**Interface:** `IPCB_ComponentClearanceConstraint` — `IPCB_ComponentClearanceConstraint.cs` L11

```csharp
int GetState_Gap();                                          // L418
int GetState_VerticalGap();                                  // L420
TComponentCollisionCheckMode GetState_CollisionCheckMode();  // L422
bool GetState_ShowDistances();                               // L425
bool GetState_DoNotCheckWithout3DBody();                     // L436
bool GetState_CheckComponentBoundary();                      // L441
int GetState_HorizontalGap();                                // L447
```

### Component Rotations (eRule_ComponentRotations = 25)

**Interface:** `IPCB_ComponentRotationsRule` — `IPCB_ComponentRotationsRule.cs` L11

```csharp
int GetState_AllowedRotations();             // L418
```

### Permitted Layers (eRule_PermittedLayers = 26)

**Interface:** `IPCB_PermittedLayersRule` — `IPCB_PermittedLayersRule.cs` L11

```csharp
TV6_LayerSet GetState_PermittedLayers();     // L418
```

### Nets To Ignore (eRule_NetsToIgnore = 27)

**Interface:** `IPCB_NetsToIgnoreRule` — `IPCB_NetsToIgnoreRule.cs` L11
*Empty interface — scope-only rule, no own members.*

### Signal Stimulus (eRule_SignalStimulus = 28)

**Interface:** `IPCB_SignalStimulus` — `IPCB_SignalStimulus.cs` L11
**Key params:** `KIND`, `LEVEL`, `STARTTIME`, `STOPTIME`, `PERIODTIME`

```csharp
TStimulusType GetState_Kind();               // L418
TSignalLevel GetState_Level();               // L420
double GetState_StartTime();                 // L422
double GetState_StopTime();                  // L424
double GetState_PeriodTime();                // L426
```

### Overshoot Falling Edge (eRule_Overshoot_FallingEdge = 29)

**Interface:** `IPCB_MaxOvershootFall` — `IPCB_MaxOvershootFall.cs` L11

```csharp
double GetState_Maximum();                   // L418
```

### Overshoot Rising Edge (eRule_Overshoot_RisingEdge = 30)

**Interface:** `IPCB_MaxOvershootRise` — `IPCB_MaxOvershootRise.cs` L11

```csharp
double GetState_Maximum();                   // L418
```

### Undershoot Falling Edge (eRule_Undershoot_FallingEdge = 31)

**Interface:** `IPCB_MaxUndershootFall` — `IPCB_MaxUndershootFall.cs` L11

```csharp
double GetState_Maximum();                   // L418
```

### Undershoot Rising Edge (eRule_Undershoot_RisingEdge = 32)

**Interface:** `IPCB_MaxUndershootRise` — `IPCB_MaxUndershootRise.cs` L11

```csharp
double GetState_Maximum();                   // L418
```

### Max Min Impedance (eRule_MaxMinImpedance = 33)

**Interface:** `IPCB_RuleMaxMinImpedance` — `IPCB_RuleMaxMinImpedance.cs` L11

```csharp
double GetState_Minimum();                   // L418
double GetState_Maximum();                   // L420
```

### Signal Top Value (eRule_SignalTopValue = 34)

**Interface:** `IPCB_RuleMinSignalTopValue` — `IPCB_RuleMinSignalTopValue.cs` L11

```csharp
double GetState_Minimum();                   // L418
```

### Signal Base Value (eRule_SignalBaseValue = 35)

**Interface:** `IPCB_RuleMaxSignalBaseValue` — `IPCB_RuleMaxSignalBaseValue.cs` L11

```csharp
double GetState_Maximum();                   // L418
```

### Flight Time Rising Edge (eRule_FlightTime_RisingEdge = 36)

**Interface:** `IPCB_RuleFlightTime_RisingEdge` — `IPCB_RuleFlightTime_RisingEdge.cs` L11

```csharp
double GetState_MaximumFlightTime();         // L418
```

### Flight Time Falling Edge (eRule_FlightTime_FallingEdge = 37)

**Interface:** `IPCB_RuleFlightTime_FallingEdge` — `IPCB_RuleFlightTime_FallingEdge.cs` L11

```csharp
double GetState_MaximumFlightTime();         // L418
```

### Max Slope Rising Edge (eRule_MaxSlope_RisingEdge = 39)

**Interface:** `IPCB_RuleMaxSlopeRisingEdge` — `IPCB_RuleMaxSlopeRisingEdge.cs` L11

```csharp
double GetState_MaxSlope();                  // L418
```

### Max Slope Falling Edge (eRule_MaxSlope_FallingEdge = 40)

**Interface:** `IPCB_RuleMaxSlopeFallingEdge` — `IPCB_RuleMaxSlopeFallingEdge.cs` L11

```csharp
double GetState_MaxSlope();                  // L418
```

### Supply Nets (eRule_SupplyNets = 41)

**Interface:** `IPCB_RuleSupplyNets` — `IPCB_RuleSupplyNets.cs` L11

```csharp
double GetState_Voltage();                   // L418
```

### Hole Size (eRule_MaxMinHoleSize = 42)

**Interface:** `IPCB_MaxMinHoleSizeConstraint` — `IPCB_MaxMinHoleSizeConstraint.cs` L11

```csharp
bool GetState_AbsoluteValues();              // L419
int GetState_MaxLimit();                     // L421
int GetState_MinLimit();                     // L423
double GetState_MaxPercent();                // L425
double GetState_MinPercent();                // L427
```

### Test Point Style (eRule_TestPointStyle = 43)

**Interface:** `IPCB_TestPointStyleRule` — `IPCB_TestPointStyleRule.cs` L11

```csharp
bool GetState_TestpointUnderComponent();     // L419
int GetState_MinSize();                      // L421
int GetState_MaxSize();                      // L423
int GetState_PreferedSize();                 // L425
int GetState_MinHoleSize();                  // L427
int GetState_MaxHoleSize();                  // L429
int GetState_PreferedHoleSize();             // L431
bool GetState_UseGrid();                     // L434
TCoordPoint GetState_GridOrigin();           // L436
int GetState_TestpointGrid();                // L438
int GetState_GridTolerance();                // L440
int GetState_MinSpacing();                   // L442
int GetState_CompBodyClearance();            // L444
int GetState_BoardEdgeClearance();           // L446
TTestpointAllowedSideSet GetState_AllowedSide(); // L448
int GetState_DistanceToViaHoleCenter();      // L480
int GetState_DistanceToPadHoleCenter();      // L482
```

### Test Point Usage (eRule_TestPointUsage = 44)

**Interface:** `IPCB_TestPointUsage` — `IPCB_TestPointUsage.cs` L11

```csharp
TTestpointValid GetState_Valid();            // L418
bool GetState_AllowMultipleOnNet();          // L421
```

### Unconnected Pin (eRule_UnconnectedPin = 45)

**Interface:** `IPCB_UnConnectedPinRule` — `IPCB_UnConnectedPinRule.cs` L11
*Empty interface — scope-only rule, no own members.*

### SMD To Plane (eRule_SMDToPlane = 46)

**Interface:** `IPCB_SMDToPlaneConstraint` — `IPCB_SMDToPlaneConstraint.cs` L11

```csharp
int GetState_Distance();                     // L418
```

### SMD Neck Down (eRule_SMDNeckDown = 47)

**Interface:** `IPCB_SMDNeckDownConstraint` — `IPCB_SMDNeckDownConstraint.cs` L11

```csharp
double GetState_Percent();                   // L418
```

### Layer Pair (eRule_LayerPair = 48)

**Interface:** `IPCB_LayerPairsRule` — `IPCB_LayerPairsRule.cs` L11

```csharp
bool GetState_EnforceLayerPairs();           // L419
```

### Fanout Control (eRule_FanoutControl = 49)

**Interface:** `IPCB_FanoutControlRule` — `IPCB_FanoutControlRule.cs` L11

```csharp
TFanoutStyle GetState_FanoutStyle();                 // L418
TFanoutDirection GetState_FanoutDirection();          // L420
TBGAFanoutDirection GetState_BGAFanoutDirection();    // L422
TBGAFanoutViaMode GetState_BGAFanoutViaMode();       // L424
int GetState_ViaGrid();                               // L426
```

### Height (eRule_MaxMinHeight = 50)

**Interface:** `IPCB_MaxMinHeightConstraint` — `IPCB_MaxMinHeightConstraint.cs` L11

```csharp
int GetState_MaxHeight();                    // L418
int GetState_MinHeight();                    // L420
int GetState_PreferedHeight();               // L422
```

### Differential Pairs Routing (eRule_DifferentialPairsRouting = 51)

**Interface:** `IPCB_DifferentialPairsRoutingRule` — `IPCB_DifferentialPairsRoutingRule.cs` L11
**Key params:** `MAXLIMIT`, `MINLIMIT`, `MOSTFREQGAP`, per-layer `TOPLAYER_MINWIDTH`, etc.

```csharp
int GetState_MaxGap(TV7_Layer argL);                 // L418
int GetState_MinGap(TV7_Layer argL);                 // L420
int GetState_PreferedGap(TV7_Layer argL);            // L422
int GetState_MaxUncoupledLength();                    // L424
int GetState_MaxWidth(TV7_Layer argL);               // L434
int GetState_MinWidth(TV7_Layer argL);               // L436
int GetState_PreferedWidth(TV7_Layer argL);          // L438
bool GetState_ImpedanceDriven();                      // L441
double GetState_MinImpedance();                       // L443
double GetState_MaxImpedance();                       // L445
double GetState_FavoredImpedance();                   // L447
string GetState_ImpedanceProfileId();                 // L463
// Per-substack overloads
```

### Hole to Hole Clearance (eRule_HoleToHoleClearance = 52)

**Interface:** `IPCB_HoleToHoleClearanceRule` — `IPCB_HoleToHoleClearanceRule.cs` L11

```csharp
int GetState_Gap();                          // L418
bool GetState_AllowStackedMicroVias();       // L421
```

### Minimum Solder Mask Sliver (eRule_MinimumSolderMaskSliver = 53)

**Interface:** `IPCB_MinimumSolderMaskSliverRule` — `IPCB_MinimumSolderMaskSliverRule.cs` L11

```csharp
int GetState_MinSolderMaskSliver();          // L418
```

### Silk to Solder Mask Clearance (eRule_SilkToSolderMaskClearance = 54)

**Interface:** `IPCB_SilkToSolderMaskClearanceRule` — `IPCB_SilkToSolderMaskClearanceRule.cs` L11

```csharp
int GetState_SilkToMaskGap();                        // L418
bool GetState_IsClearanceToExposedCopper();           // L423
```

### Silk to Silk Clearance (eRule_SilkToSilkClearance = 55)

**Interface:** `IPCB_SilkToSilkClearanceRule` — `IPCB_SilkToSilkClearanceRule.cs` L11

```csharp
int GetState_SilkToSilkClearance();          // L418
```

### Net Antennae (eRule_NetAntennae = 56)

**Interface:** `IPCB_CheckNetAntennaeRule` — `IPCB_CheckNetAntennaeRule.cs` L11

```csharp
int GetState_NetAntennaeTolerance();         // L418
```

### Board Outline Clearance (eRule_BoardOutlineClearance = 63)

**Interface:** `IPCB_BoardOutlineClearanceConstraint` — `IPCB_BoardOutlineClearanceConstraint.cs` L11

```csharp
int GetState_Gap();                          // L418
void SetClearance(TObjectClearanceId o1, TObjectClearanceId o2, int val); // L422
int GetClearance(TObjectClearanceId o1, TObjectClearanceId o2);           // L424
```

### Back Drilling (eRule_BackDrilling = 64)

**Interface:** `IPCB_BackDrillingRule` — `IPCB_BackDrillingRule.cs` L11

```csharp
bool GetState_UseTopLayer();                             // L419
bool GetState_UseBottomLayer();                          // L424
int GetState_MaxStubLength();                            // L428
int GetState_BackDrillOverSize();                        // L432
int GetState_BackDrillOverSizePositiveTolerance();       // L436
int GetState_BackDrillOverSizeNegativeTolerance();       // L440
```

### Creepage (eRule_Creepage = 65)

**Interface:** `IPCB_CreepageRule` — `IPCB_CreepageRule.cs` L11

```csharp
int GetState_CheckDistance();                // L418
bool GetState_IgnoreInternalLayers();        // L423
bool GetState_ApplyToPolygonPour();          // L428
```

### Return Path (eRule_ReturnPath = 66)

**Interface:** `IPCB_ReturnPathRule` — `IPCB_ReturnPathRule.cs` L11

```csharp
int GetState_GapLimit();                                 // L418
string GetState_ImpedanceProfileId();                    // L422
bool GetState_UseAntiPads();                             // L427
int GetState_MaxStitchViaDistance();                      // L431
bool GetState_MaxStitchViaDistanceEnabled();             // L436
```

### Routing Neck Down (eRule_RoutingNeckDown = 67)

**Interface:** `IPCB_RoutingNeckDownRule` — `IPCB_RoutingNeckDownRule.cs` L11

```csharp
IPCB_LayerToCoord GetState_MaxLength();      // L419
```

### Wirebonding (eRule_Wirebonding = 68)

**Interface:** `IPCB_WirebondRule` — `IPCB_WirebondRule.cs` L11

```csharp
int GetState_WireToWireGap();                // L418
int GetState_MinWireLength();                // L422
int GetState_MaxWireLength();                // L426
int GetState_BondFingerSpace();              // L430
int GetState_BondFingerMargin();             // L434
double GetState_Angle();                     // L438
bool GetState_BondFingerToWireAlignment();   // L443
```

### Z-Axis Clearance (eRule_ZAxisClearance = 69)

**Interface:** `IPCB_ZAxisClearanceRule` — `IPCB_ZAxisClearanceRule.cs` L11

```csharp
int GetState_ZAxisClearance();               // L418
```

### Modified Polygon (eRule_ModifiedPolygon = 62)

**Interface:** `IPCB_ModifiedPolygonRule` — `IPCB_ModifiedPolygonRule.cs` L11

```csharp
bool GetState_AllowModified();               // L419
bool GetState_AllowShelved();                // L424
```

### Layer Stack (eRule_LayerStack = 38)

*No dedicated interface found — likely uses base IPCB_Rule only.*

### Assembly Test Point Style/Usage (eRule_AssyTestPointStyle = 57, eRule_AssyTestPointUsage = 58)

*Reuse IPCB_TestPointStyleRule / IPCB_TestPointUsage interfaces.*

### Silk to Board Region (eRule_SilkToBoardRegion = 59)

*No dedicated interface found — likely uses a generic gap constraint.*

### SMD PAD Entry (eRule_SMDPADEntry = 60)

**Interface:** `IPCB_SMDPADEntryConstraint` — `IPCB_SMDPADEntryConstraint.cs` L11

```csharp
bool GetState_Can_Side();                    // L419
bool GetState_Can_Corner();                  // L422
bool GetState_Can_AnyAngle();                // L425
bool GetState_Can_IgnoreFirstCorner();       // L428
```

---

## IPCB_Violation Base Interface

**File:** `IPCB_Violation.cs`, lines 328-363
**Extends:** `IPCB_Primitive`

```csharp
string GetState_Name();                                          // L328
IPCB_Primitive GetState_Rule();                                  // L331
IPCB_Primitive GetState_Primitive1();                            // L334
IPCB_Primitive GetState_Primitive2();                            // L337
string GetState_Description();                                   // L339
void SetState_Description(string argValue);                      // L354
bool IsRedundant();                                              // L342
string GetState_ShortDescriptorString();                         // L344
bool GetState_IsWaived();                                        // L347
IPCB_WaivedViolationInfo GetState_WaivedInfo();                  // L350
void SetState_WaivedInfo(IPCB_WaivedViolationInfo argValue);     // L352
void AddInvolvedPrimitive(IPCB_Primitive argPrimitive);          // L356
int InvolvedPrimitivesCount();                                    // L358
IPCB_Primitive GetState_InvolvedPrimitiveItem(int argIndex);     // L361
void InvolvedPrimitivesClear();                                   // L363
```

**Serialized as** standard `|KEY=VALUE|` param records in `T*Violation` CFB storages.

Common parameters:

| Parameter | Description |
|-----------|-------------|
| `RULEINDEX` | u32 index into Rules6 |
| `PRIM1ID` | Primitive type string (`"Via"`, `"Pad"`, `"Track"`, etc.) |
| `PRIM1INDEX` | u32 index in primitive's section |
| `PRIM2ID` / `PRIM2INDEX` | Second primitive (binary violations) |
| `DESCRIPTION` | Human-readable violation text |
| `INVOLVEDPRIMCOUNT` | Count of additional involved primitives |

---

## Concrete Violation Types

### Inheritance Hierarchy

```
IPCB_Violation
  +-- IPCB_AcuteAngleViolation
  +-- IPCB_BackDrillViolation
  +-- IPCB_BoardOutlineClearanceViolation
  +-- IPCB_ClearanceViolation
  |     +-- IPCB_ComponentClearanceViolation
  |     +-- IPCB_WirebondWireToWireViolation
  |     +-- IPCB_ZAxisClearanceViolation
  +-- IPCB_CreepageViolation
  +-- IPCB_DiffPairsViolation
  +-- IPCB_DisconnectedSubnetsViolation
  +-- IPCB_MatchedNetLengthsViolation
  +-- IPCB_MaxMinLengthViolation
  +-- IPCB_MinWidthStubViolation_Base
  +-- IPCB_MinWidthViolation
  +-- IPCB_NetAntennaeViolation
  +-- IPCB_RoutingNeckDownViolation
  +-- IPCB_ShortCircuitViolation
  +-- IPCB_SMDNeckDownViolation
  +-- IPCB_SMDPADEntryViolation
  +-- IPCB_TestpointViolation_Base
  +-- IPCB_ViaUnderSMDViolation
  |     +-- IPCB_PadUnderSMDViolation
  +-- IPCB_WirebondLengthViolation
```

### IPCB_AcuteAngleViolation — `IPCB_AcuteAngleViolation.cs` L365

```csharp
double GetState_AcuteAngle();                // L365
IPCB_Primitive GetState_HelperPrim();        // L368
TCoordPoint GetState_Location();             // L370
```

### IPCB_BackDrillViolation — `IPCB_BackDrillViolation.cs` L365

```csharp
int GetState_StubLength();                   // L365
bool GetState_TopOrBottom();                 // L368
TCoordRect GetState_ViolationArea();         // L370
```

### IPCB_BoardOutlineClearanceViolation — `IPCB_BoardOutlineClearanceViolation.cs` L365

```csharp
TObjectClearanceId GetState_PrimitiveID1();  // L365
TObjectClearanceId GetState_PrimitiveID2();  // L367
TCoordPoint GetState_ViolationPt1();         // L369
TCoordPoint GetState_ViolationPt2();         // L371
```

### IPCB_ClearanceViolation — `IPCB_ClearanceViolation.cs` L365

```csharp
TCoordPoint GetState_ViolationPt1();         // L365
TCoordPoint GetState_ViolationPt2();         // L367
bool GetState_IsHoleClearanceViolation();    // L370
bool GetState_ValueIsIntit();                // L373
int GetState_RuleValue();                    // L375
```

### IPCB_ComponentClearanceViolation — `IPCB_ComponentClearanceViolation.cs` L387
**Extends:** `IPCB_ClearanceViolation`

```csharp
TDoublePoint3D GetState_Point1();            // L387
TDoublePoint3D GetState_Point2();            // L389
```

### IPCB_CreepageViolation — `IPCB_CreepageViolation.cs` L365

```csharp
TCoordPoint GetState_ViolationPathItem(int i); // L365
int GetState_ActualDistance();               // L369
void Clear();                                // L373
int Count();                                 // L375
void AddPoint(TCoordPoint pt);              // L377
```

### IPCB_DiffPairsViolation — `IPCB_DiffPairsViolation.cs` L365

```csharp
IPCB_GeometricPolygon GetState_Item(TV7_Layer layer); // L366
IPCB_LayerIterator LayerIterator();          // L371
void Clear();                                // L373
int Count();                                 // L375
```

### IPCB_DisconnectedSubnetsViolation — `IPCB_DisconnectedSubnetsViolation.cs` L365

Serialized with `FX1/FY1/FX2/FY2` parameters.

```csharp
TCoordPoint GetState_ViolationPt1();         // L365
TCoordPoint GetState_ViolationPt2();         // L367
```

### IPCB_MatchedNetLengthsViolation — `IPCB_MatchedNetLengthsViolation.cs` L365

```csharp
IPCB_PhaseMatchingViolatedArea CreatePhaseMatchingViolatedArea(); // L366
void Add(IPCB_PhaseMatchingViolatedArea area); // L368
void Remove(int index);                      // L370
void Clear();                                // L372
int Count();                                 // L374
IPCB_PhaseMatchingViolatedArea GetItem(int i); // L377
```

### IPCB_MaxMinLengthViolation — `IPCB_MaxMinLengthViolation.cs` L365

```csharp
double GetState_ViolDelay();                 // L365
int GetState_ViolLength();                   // L367
```

### IPCB_MinWidthStubViolation_Base — `IPCB_MinWidthStubViolation_Base.cs` L365

```csharp
IPCB_Primitive GetState_HelperPrim();        // L366
```

Also has `IPCB_MinWidthStubViolation_Base_SaveLoadParameters` (`PCBInterfaces/` L11):
```csharp
TCoordPoint GetState_MidPoint();             // L11
```

### IPCB_MinWidthViolation — `IPCB_MinWidthViolation.cs` L365

```csharp
IPCB_Primitive GetState_HelperPrim();        // L366
TCoordRect GetState_ViolationArea();         // L370
```

### IPCB_NetAntennaeViolation — `IPCB_NetAntennaeViolation.cs` L365

Serialized with `LOCATION.X/Y` and `CIRCLERADIUS` parameters.

```csharp
int GetState_CircleRadius();                 // L365
TCoordPoint GetState_Location();             // L367
```

### IPCB_RoutingNeckDownViolation — `IPCB_RoutingNeckDownViolation.cs` L365

```csharp
IPCB_GeometricPolygon GetState_BoundPolygon(); // L366
int GetState_NeckDownLength();               // L368
```

### IPCB_ShortCircuitViolation — `IPCB_ShortCircuitViolation.cs` L365

Serialized with `VX1/VY1/VX2/VY2/VX3/VY3/VX4/VY4` (area corners).

```csharp
TCoordRect GetState_ViolationArea();         // L365
```

### IPCB_SMDNeckDownViolation — `IPCB_SMDNeckDownViolation.cs` L365

```csharp
TCoordPoint GetState_Location();             // L365
```

### IPCB_SMDPADEntryViolation — `IPCB_SMDPADEntryViolation.cs` L365

```csharp
int GetState_CircleRadius();                 // L365
TCoordPoint GetState_Location();             // L367
```

### IPCB_TestpointViolation_Base — `IPCB_TestpointViolation_Base.cs` L365

```csharp
IPCB_Primitive GetState_HelperPrim();        // L366
```

### IPCB_ViaUnderSMDViolation — `IPCB_ViaUnderSMDViolation.cs` L365

```csharp
IPCB_Primitive GetState_HelperPrim();        // L366
```

### IPCB_PadUnderSMDViolation — `IPCB_PadUnderSMDViolation.cs` L365
**Extends:** `IPCB_ViaUnderSMDViolation`
*Empty — inherits HelperPrim from parent.*

### IPCB_WirebondLengthViolation — `IPCB_WirebondLengthViolation.cs` L365

```csharp
int GetState_ActualLength();                 // L365
```

### IPCB_WirebondWireToWireViolation — `IPCB_WirebondWireToWireViolation.cs` L387
**Extends:** `IPCB_ClearanceViolation`

```csharp
int GetState_ActualClosestDistance();        // L387
TCoordPoint3D GetProperty_ViolationPt3D1();  // L391
TCoordPoint3D GetProperty_ViolationPt3D2();  // L395
```

### IPCB_ZAxisClearanceViolation — `IPCB_ZAxisClearanceViolation.cs` L387
**Extends:** `IPCB_ClearanceViolation`

*All 8 methods are deprecated `DoNotUse_*` vtable slots (L387-401).*

---

## DRC Result Interfaces

### IPCB_DRCResult — `IPCB_DRCResult.cs` L10

```csharp
int GetState_CheckResult();                  // L10
IPCB_Rule GetState_Rule();                   // L13
string GetState_ScopeObjectID();             // L16
IPCB_Violation GetState_Violation();         // L19
string GetState_ScopeObjectName();           // L22
TObjectId GetState_ScopeObjectKind();        // L24
IPCB_Primitive GetState_Prim1();             // L27
bool GetState_IsBinary();                    // L30
```

### IPCB_DRCBinaryResult — `IPCB_DRCBinaryResult.cs` L33
**Extends:** `IPCB_DRCResult`

```csharp
IPCB_Primitive GetState_Prim2();             // L33
```

### IPCB_DRCResultSingleValue — `IPCB_DRCResultSingleValue.cs` L32
**Extends:** `IPCB_DRCResult`

```csharp
int GetState_Value();                        // L32
```

---

## Waived Violation Interfaces

### IPCB_WaivedViolationInfo — `IPCB_WaivedViolationInfo.cs` L11

```csharp
bool CanChangeAuthor();                      // L11
IPCB_AuthorInfo GetAuthor();                 // L14
double GetCreatedAt();                       // L16 — OLE Automation date
string GetComment();                         // L18
void SetAuthor(IPCB_AuthorInfo argValue);    // L20
void SetComment(string argValue);            // L22
void SetCreatedAt(double argValue);          // L24
IPCB_WaivedViolationInfo Clone();            // L27
void CopyTo(IPCB_WaivedViolationInfo dest);  // L29
```

### IPCB_WaivedViolationManager — `PCBInterfaces/IPCB_WaivedViolationManager.cs` L11

```csharp
void Clear();                                                    // L11
void Add(IPCB_Primitive p1, IPCB_Primitive p2,
         IPCB_Rule rule, IPCB_WaivedViolationInfo info);         // L13
```

### IPCB_ViolationsFactory — `IPCB_ViolationsFactory.cs` L11

```csharp
IPCB_WaivedViolationInfo CreateWaivedViolationInfo();            // L11
IPCB_WaivedViolationInfo CreateDefaultWaivedViolationInfo();     // L14
IPCB_AuthorInfo CreateAuthorInfo(string id, string title, string source); // L17
IPCB_AuthorInfo CreateCurrentAuthorInfo();                       // L20
IPCB_Violation CreateViolation(string className, IPCB_Primitive rule,
                                IPCB_Primitive p1, IPCB_Primitive p2); // L23
```

### IPCB_ViolationSection — `IPCB_ViolationSection.cs` L81
**Extends:** `IPCB_BinarySection`

```csharp
int GetState_ViolationCount();               // L81
```

---

## Supporting Enums

All files in `RT_PCB/` unless noted. Underlying type is `byte` for all.

### TNetScope — `TNetScope.cs`

| Value | Name | Serialization String |
|-------|------|---------------------|
| 0 | `eNetScope_DifferentNetsOnly` | `"DifferentNets"` |
| 1 | `eNetScope_SameNetOnly` | `"SameNetOnly"` |
| 2 | `eNetScope_AnyNet` | `"AnyNet"` |
| 3 | `eNetScope_DifferentDiffPairsOnly` | `"DifferentPairs"` |
| 4 | `eNetScope_SameDiffPairOnly` | `"SameDiffPairs"` |

Source: `xPCBTypes/Consts.cs` L588-594

### TRuleLayerKind — `TRuleLayerKind.cs`

| Value | Name | String |
|-------|------|--------|
| 0 | `eRuleLayerKind_SameLayer` | `"SameLayer"` |
| 1 | `eRuleLayerKind_AdjacentLayer` | `"AdjacentLayers"` |

Source: `xPCBTypes/Consts.cs` L579-582

### TScopeKind — `TScopeKind.cs` (41 variants, 0-40)

| Value | Name | String |
|-------|------|--------|
| 0 | `eScopeKindBoard` | `"Board"` |
| 1 | `eScopeKindLayerClass` | `"LayerClass"` |
| 2 | `eScopeKindLayer` | `"Layer"` |
| 3 | `eScopeKindObjectKind` | `"ObjectKind"` |
| 4 | `eScopeKindFootprint` | `"Footprint"` |
| 5 | `eScopeKindComponentClass` | `"ComponentClass"` |
| 6 | `eScopeKindComponent` | `"Component"` |
| 7 | `eScopeKindNetClass` | `"NetClass"` |
| 8 | `eScopeKindNet` | `"Net"` |
| 9 | `eScopeKindFromToClass` | `"FromToClass"` |
| 10 | `eScopeKindFromTo` | `"FromTo"` |
| 11 | `eScopeKindPadClass` | `"PadClass"` |
| 12 | `eScopeKindPadSpec` | `"PadSpec"` |
| 13 | `eScopeKindViaSpec` | `"ViaSpec"` |
| 14 | `eScopeKindFootprintPad` | `"FootprintPad"` |
| 15 | `eScopeKindPad` | `"Pad"` |
| 16 | `eScopeKindRegion` | `"Region"` |
| 17 | `eScopeKindSignalClass` | `"xSignal"` |
| 18 | `eScopeKindLayerStackRegion` | `"LayerStackRegion"` |
| 19 | `eScopeKindDiffPair` | `"DiffPair"` |
| 20 | `eScopeKindDiffPairClass` | `"DiffPairClass"` |
| 21 | `eScopeKindPackage` | `"Package"` |
| 22 | `eScopeKindDrillPair` | `"DrillPair"` |
| 23 | `eScopeKindInPolygon` | `"InPolygon"` |
| 24 | `eScopeKindPadType` | `"PadType"` |
| 25 | `eScopeKindWithinRoom` | `"WithinRoom"` |
| 26 | `eScopeKindNetAndLayer` | `"NetAndLayer"` |
| 27 | `eScopeKindNetAndFootPrint` | `"NetAndFootprint"` |
| 28 | `eScopeKindLayerAndDiffPairClass` | `"LayerAndDiffPairClass"` |
| 29 | `eScopeKindPackageAndComponentClass` | `"PackageAndComponentClass"` |
| 30 | `eScopeKindFootPrintAndLayer` | `"FootPrintAndLayer"` |
| 31 | `eScopeKindNetAndPadType` | `"NetAndPadType"` |
| 32 | `eScopeKindPolygonAndLayer` | `"PolygonAndLayer"` |
| 33 | `eScopeKindPadClassAndLayer` | `"PadClassAndLayer"` |
| 34 | `eScopeKindObjectKindAndLayer` | `"ObjectKindAndLayer"` |
| 35 | `eScopeKindNetAndDrillPair` | `"NetAndDrillPair"` |
| 36 | `eScopeKindNetClassAndDrillPair` | `"NetClassAndDrillPair"` |
| 37 | `eScopeKindSignalClassAndLayer` | `"xSignalClassAndLayer"` |
| 38 | `eScopeKindComponentAndLayer` | `"ComponentAndLayer"` |
| 39 | `eScopeKindPackageAndLayer` | `"PackageAndLayer"` |
| 40 | `eScopeKindAdvanced` | `"Advanced"` |

Source: `xPCBTypes/Consts.cs` L524-566

### TClearanceConstraintMode — `TClearanceConstraintMode.cs`

| Value | Name | String |
|-------|------|--------|
| 0 | `eClearanceConstraintMode_SingleClearance` | `"SingleClearance"` |
| 1 | `eClearanceConstraintMode_ObjectsClearance` | `"ObjectsClearance"` |

Source: `RT_PCB/Consts.cs` L1442-1445

### TObjectClearanceId — `TObjectClearanceId.cs` (15 variants)

| Value | Name | String |
|-------|------|--------|
| 0 | `eObjectClearanceID_Arc` | `"ClearanceObj_Arc"` |
| 1 | `eObjectClearanceID_Track` | `"ClearanceObj_Track"` |
| 2 | `eObjectClearanceID_SMDPad` | `"ClearanceObj_SMDPad"` |
| 3 | `eObjectClearanceID_THPad` | `"ClearanceObj_THPad"` |
| 4 | `eObjectClearanceID_Via` | `"ClearanceObj_Via"` |
| 5 | `eObjectClearanceID_Fill` | `"ClearanceObj_Fill"` |
| 6 | `eObjectClearanceID_Poly` | `"ClearanceObj_Poly"` |
| 7 | `eObjectClearanceID_Region` | `"ClearanceObj_Region"` |
| 8 | `eObjectClearanceID_Text` | `"ClearanceObj_Text"` |
| 9 | `eObjectClearanceID_Hole` | `"ClearanceObj_Hole"` |
| 10 | `eObjectClearanceID_OutlineEdge` | `"ClearanceObj_OutlineEdge"` |
| 11 | `eObjectClearanceID_CavityEdge` | `"ClearanceObj_CavityEdge"` |
| 12 | `eObjectClearanceID_CutoutEdge` | `"ClearanceObj_CutoutEdge"` |
| 13 | `eObjectClearanceID_SplitBarrier` | `"ClearanceObj_SplitBarrior"` |
| 14 | `eObjectClearanceID_SplitContinuation` | `"ClearanceObj_SplitContinuation"` |

Source: `RT_PCB/Consts.cs` L1425-1441. Note: "SplitBarrior" is a typo in the original.

### TPlaneConnectStyle — `TPlaneConnectStyle.cs`

| Value | Name | String |
|-------|------|--------|
| 0 | `eReliefConnectToPlane` | `"Relief"` |
| 1 | `eDirectConnectToPlane` | `"Direct"` |
| 2 | `eNoConnect` | `"NoConnect"` |

### TPolygonConnectPrimitiveType — `TPolygonConnectPrimitiveType.cs`

| Value | Name |
|-------|------|
| 0 | `ePolyPadAndVia` |
| 1 | `ePolyTHPad` |
| 2 | `ePolySMDPad` |
| 3 | `ePolyVia` |

### TPolygonReliefAngle — `TPolygonReliefAngle.cs`

| Value | Name | String |
|-------|------|--------|
| 0 | `ePolygonReliefAngle_45` | `"45 Angle"` |
| 1 | `ePolygonReliefAngle_90` | `"90 Angle"` |
| 2 | `ePolygonReliefAngle_0` | `"0 Angle"` |
| 3 | `ePolygonReliefAngle_135` | `"135 Angle"` |

Source: `xPCBTypes/Consts.cs` L652-657

### TNetTopology — `TNetTopology.cs`

| Value | Name | String |
|-------|------|--------|
| 0 | `eNetTopology_Shortest` | `"Shortest"` |
| 1 | `eNetTopology_Horizontal` | `"Horizontal"` |
| 2 | `eNetTopology_Vertical` | `"Vertical"` |
| 3 | `eNetTopology_DaisyChain_Simple` | `"Daisy_Simple"` |
| 4 | `eNetTopology_DaisyChain_MidDriven` | `"Daisy_MidDriven"` |
| 5 | `eNetTopology_DaisyChain_Balanced` | `"Daisy_Balanced"` |
| 6 | `eNetTopology_Starburst` | `"Starburst"` |

Source: `xPCBTypes/Consts.cs` L595-603

### TCornerStyle — `TCornerStyle.cs`

| Value | Name | String |
|-------|------|--------|
| 0 | `eCornerStyle_90` | `"90-Degree"` |
| 1 | `eCornerStyle_45` | `"45-Degree"` |
| 2 | `eCornerStyle_Round` | `"Round"` |

### TRouteVia — `TRouteVia.cs`

| Value | Name | String |
|-------|------|--------|
| 0 | `eViaThruHole` | `"Through Hole"` |
| 1 | `eViaBlindBuriedPair` | `"Blind Buried (Adjacent Layers)"` |
| 2 | `eViaBlindBuriedAny` | `"Blind Buried (Any Layer Pair)"` |
| 3 | `eViaNone` | `"xxx"` |

### TConfinementStyle — `TConfinementStyle.cs`

| Value | Name |
|-------|------|
| 0 | `eConfineIn` |
| 1 | `eConfineOut` |

### TComponentCollisionCheckMode — `TComponentCollisionCheckMode.cs`

| Value | Name | String |
|-------|------|--------|
| 0 | `eQuickCheck` | `"Quick Check Mode"` |
| 1 | `eMultiLayerCheck` | `"Multi-Layer Check Mode"` |
| 2 | `eFullCheck` | `"Full Check Mode"` |
| 3 | `eComponentBodyCheck` | `"Component Body Mode"` |

Source: `RT_PCB/Consts.cs` L1357-1362

### TLengthenerStyle — `TLengthenerStyle.cs`

| Value | Name |
|-------|------|
| 0 | `eLengthenerStyle_90` |
| 1 | `eLengthenerStyle_45` |
| 2 | `eLengthenerStyle_Round` |
| 3 | `eLengthenerStyle_Mitered90` |

### TFanoutStyle — `TFanoutStyle.cs`

| Value | Name |
|-------|------|
| 0 | `eFanoutStyle_Auto` |
| 1 | `eFanoutStyle_Rows` |
| 2 | `eFanoutStyle_Staggered` |
| 3 | `eFanoutStyle_BGA` |
| 4 | `eFanoutStyle_UnderPads` |

### TFanoutDirection — `TFanoutDirection.cs`

| Value | Name |
|-------|------|
| 0 | `eFanoutDirection_None` |
| 1 | `eFanoutDirection_InOnly` |
| 2 | `eFanoutDirection_OutOnly` |
| 3 | `eFanoutDirection_InThenOut` |
| 4 | `eFanoutDirection_OutThenIn` |
| 5 | `eFanoutDirection_Alternating` |

### TTestpointValid — `TTestpointValid.cs`

| Value | Name |
|-------|------|
| 0 | `eRequire` |
| 1 | `eInvalid` |
| 2 | `eIgnore` |
| 3 | `eRequireAtLeafs` |

### TStimulusType — `TStimulusType.cs`

| Value | Name |
|-------|------|
| 0 | `eConstantLevel` |
| 1 | `eSinglePulse` |
| 2 | `ePeriodicPulse` |

### TSignalLevel — `TSignalLevel.cs`

| Value | Name |
|-------|------|
| 0 | `eLowLevel` |
| 1 | `eHighLevel` |

### TRuleCategory — `xPCBTypes/TRuleCategory.cs`

| Value | Name | String |
|-------|------|--------|
| 0 | `eRuleCategory_Electrical` | `"Electrical"` |
| 1 | `eRuleCategory_Routing` | `"Routing"` |
| 2 | `eRuleCategory_SMT` | `"SMT"` |
| 3 | `eRuleCategory_PasteAndSolderMask` | `"Mask"` |
| 4 | `eRuleCategory_PowerPlane` | `"Plane"` |
| 5 | `eRuleCategory_Testpoint` | `"Testpoint"` |
| 6 | `eRuleCategory_OtherManufacturing` | `"Manufacturing"` |
| 7 | `eRuleCategory_HighSpeed` | `"High Speed"` |
| 8 | `eRuleCategory_ComponentPlacement` | `"Placement"` |
| 9 | `eRuleCategory_SignalIntegrity` | `"Signal Integrity"` |

Source: `xPCBTypes/Consts.cs` L658-669

### TViolationKind — `TViolationKind.cs` (wirebond-specific only)

| Value | Name |
|-------|------|
| 0 | `eViolation_NotSpecified` |
| 1 | `eViolation_WirebondLength` |
| 2 | `eViolation_WireToWireClearance` |
| 3 | `eViolation_BondFingerSpaceClearance` |
| 4 | `eViolation_BondFingerMarginClearance` |
| 5 | `eViolation_WirebondAngleClearance` |
| 6 | `eViolation_DieToBondFingerClearance` |
| 7 | `eViolation_BondFingerToWireAlignment` |
| 8 | `eViolation_WirebondShortCircuit` |

---

## Serialization Constants

### cRuleIdStrings

**File:** `RT_PCB/Consts.cs` lines 1121-1192

Maps `TRuleKind` enum values to the string used in `RULEKIND=` parameters.
See the [TRuleKind table](#trulekind-enum) above for the complete mapping.

### cNetScopeStrings

**File:** `xPCBTypes/Consts.cs` lines 588-594. See [TNetScope](#tnetscope--tnetscopecs).

### cRuleLayerKindStrings

**File:** `xPCBTypes/Consts.cs` lines 579-582. See [TRuleLayerKind](#trulelayerkind--trulelayerkindcs).

### cRuleScopeStrings

**File:** `xPCBTypes/Consts.cs` lines 524-566. See [TScopeKind](#tscopekind--tscopekindcs-41-variants-0-40).

### cObjectClearanceIdStrings

**File:** `RT_PCB/Consts.cs` lines 1425-1441. See [TObjectClearanceId](#tobjectclearanceid--tobjectclearanceidcs-15-variants).

### cClearanceConstraintModeIdStrings

**File:** `RT_PCB/Consts.cs` lines 1442-1445. See [TClearanceConstraintMode](#tclearanceconstraintmode--tclearanceconstraintmodecs).

### cComponentCollisionCheckModeStings

**File:** `RT_PCB/Consts.cs` lines 1357-1362. Note: "Stings" is a typo in the original.

### cRoutingDiffPairGapModeStrings

**File:** `RT_PCB/Consts.cs` lines 1391-1395

| Value | Name | String |
|-------|------|--------|
| 0 | `eRoutingDiffPairGap_Min` | `"Rule Minimum"` |
| 1 | `eRoutingDiffPairGap_Preferred` | `"Rule Preferred"` |
| 2 | `eRoutingDiffPairGap_Max` | `"Rule Maximum"` |
