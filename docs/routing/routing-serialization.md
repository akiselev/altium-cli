# Routing Data Serialization in Altium PcbDoc/PcbLib

This document covers how routing-related data is persisted in Altium's CFB-based
`.PcbDoc` and `.PcbLib` file formats. It is reference material for implementing
routing rule parsing and serialization in the Rust `altium-format` crate.

Sources: decompiled C# code in `AD26-dotnet/`, existing `docs/dxp/pcb-files.md`.

---

## Persistence / Serialization Overview

Routing-related data falls into four distinct categories inside a PcbDoc CFB:

1. **Design Rules** (`Rules6` / `NewRules6` sections) — routing constraints such
   as width, corner style, via style, topology, priority, neck-down, and
   differential pairs. Stored as pipe-delimited parameter blocks with a 2-byte
   prefix framing.

2. **Router Options** (`Advanced Router Options6` section) — Specctra auto-router
   configuration (passes, via/wire cost, grid, layer taxes, etc.). Stored as
   parameter blocks.

3. **Routing state on primitives** — tracks, vias, and arcs carry per-primitive
   flags (`IsPreRoute`, `UserRouted`) that are part of the binary primitive record
   payload already documented in `docs/dxp/pcb-records.md`.

4. **Interactive Routing Options** — NOT stored in the PcbDoc file itself. Loaded
   and saved through the `IPCB_InteractiveRoutingOptions` COM interface, which
   uses `Export_ToParameters` / `Import_FromParameters` methods. Where exactly
   this goes is not in the dotnet layer (it is Delphi/registry or a separate
   preferences file, not in the PCB CFB).

There are no separate "routing session" or "routing history" CFB streams found in
the dotnet codebase. Active routing is entirely an in-memory, interactive process;
only the resulting geometry (tracks, vias) plus design rules persist in the file.

---

## File Format Streams

### Rules6 (CFB section: `Rules6/Header` + `Rules6/Data`)

- **Format**: Prefixed parameter block records (see `docs/dxp/pcb-files.md` §6.3).
- **Per-record framing**:
  ```
  +--------+----------+----------------------------------+
  | Prefix | Length   | Parameter String                  |
  | 2 bytes| 4 bytes  | N bytes (Win1252, NUL-terminated) |
  +--------+----------+----------------------------------+
  ```
- **Header stream**: u32 LE record count.
- Each parameter string is a pipe-delimited `|KEY=VALUE|` block encoding one rule.
- The `RULEKIND` key identifies which rule type the block represents (see below).

### NewRules6 (CFB section: `NewRules6/Header` + `NewRules6/Data`)

- Same framing as `Rules6`.
- Contains extended rule types added in later format versions. The exact split
  between what goes in `Rules6` vs `NewRules6` is determined at write time based
  on `TStorageFeature` flags:
  - `eHasClearanceByLayerRuleAtWriteStage` (bit 13) — per-layer clearance rules
  - `eHasMatrixRuleAtWriteStage` (bit 14) — clearance matrix rules
  - `eHasNeckDownRuleAtWriteStage` (bit 18) — `RoutingNeckDown` rules
  - `eHasZAxisClearanceRuleAtWriteStage` (bit 25) — Z-axis clearance rules

### Advanced Router Options6 (CFB section: `Advanced Router Options6/Header` + `Advanced Router Options6/Data`)

- Contains Specctra auto-router configuration.
- Stored as parameter blocks via `IPCB_SpecctraRouterOptions.Export_ToParameters()`.
- Interface: `RT_PCB.IPCB_SpecctraRouterOptions` (Guid `7C37270B-3551-40CF-A0F1-D6EE7F2E7331`).
- Key fields from the interface:
  - `GetState_Setback(i)` / `GetState_DoSetback(i)` — setback per layer
  - `GetState_DoBus()` / `GetState_BusDiagonal()` — bus routing options
  - `GetState_WireGrid()` / `GetState_ViaGrid()` — routing grids
  - `GetState_DoSeedVias()` / `GetState_SeedViaLimit()` — via seeding
  - `GetState_RoutePasses()` / `GetState_CleanPasses()` / `GetState_FilterPasses()`
  - `GetState_LayerCost(layer)` / `GetState_LayerWWCost(layer)` — per-layer cost
  - `GetState_WwCost()` / `GetState_CrossCost()` / `GetState_ViaCost()` — global costs
  - `GetState_LayerTax(layer)` / `GetState_ViaTax()` etc. — tax values
  - `GetState_ProtectPreRoutes()` / `GetState_ReorderNets()` / `GetState_NoConflicts()`
- The exact parameter key names used in `Export_ToParameters()` are in Delphi
  (`Advpcb.dll`); the dotnet layer only exposes the interface, not the serialization.

### No "Interactive Routing Options" CFB section

The `IPCB_InteractiveRoutingOptions` interface (Guid `4E613AF2-C436-42A4-965E-6BC117FE892B`,
`TOptionsObjectId.eInteractiveRoutingOptions`) has `Export_ToParameters` and
`Import_FromParameters` methods but these do NOT map to a CFB section in PcbDoc.
They are used for the Delphi preferences subsystem (likely a `.ini` or registry
store), not the PCB file itself. This is confirmed by the absence of any
`"Interactive Routing Options6"` section in the CFB structure tree
(see `docs/dxp/pcb-files.md` §3.1).

---

## Routing Configuration Persistence

Routing configuration (rules) is stored in `Rules6` / `NewRules6` as
pipe-delimited parameter records. Each rule record shares a common set of
envelope keys, then has rule-type-specific keys.

### Common Rule Envelope Keys

These keys appear in every rule record regardless of type:

| Key | Description |
|-----|-------------|
| `RULEKIND` | Rule type string (see table below) |
| `SCOPE1EXPRESSION` | Scope 1 expression string (e.g. `"All"`, `"InNet('GND')"`) |
| `SCOPE2EXPRESSION` | Scope 2 expression string (same format) |
| `NAME` | User-visible rule name string |
| `ENABLED` | `"TRUE"` or `"FALSE"` |
| `PRIORITY` | Integer priority (1 = highest) |
| `COMMENT` | User comment string |
| `UNIQUEID` | Rule unique identifier GUID string |
| `UNIONINDEX` | Integer, union membership index |
| `NETSCOPE` | Net scope enum string (see below) |
| `LAYERKIND` | Layer scope enum string (see below) |
| `SEQUENCE` | Rule sequence number (integer) |
| `DRCENABLED` | Whether DRC is enabled for this rule |

**NETSCOPE values** (from `TNetScope` enum, source `RT_PCB/TNetScope.cs`):

| String | Enum |
|--------|------|
| `"DifferentNets"` | `eNetScope_DifferentNetsOnly` |
| `"SameNet"` | `eNetScope_SameNetOnly` |
| `"AnyNet"` | `eNetScope_AnyNet` |
| `"DifferentDiffPairs"` | `eNetScope_DifferentDiffPairsOnly` |
| `"SameDiffPair"` | `eNetScope_SameDiffPairOnly` |

**LAYERKIND values** (from `TRuleLayerKind` enum, source `RT_PCB/TRuleLayerKind.cs`):

| String | Enum |
|--------|------|
| `"SameLayer"` | `eRuleLayerKind_SameLayer` |
| `"AdjacentLayer"` | `eRuleLayerKind_AdjacentLayer` |

### RULEKIND Values and Rule-Type-Specific Keys

All `RULEKIND` strings come from `cRuleIdStrings` in `RT_PCB/Consts.cs`.
The routing-related rule kinds and their specific parameter keys are:

#### `RULEKIND=RoutingTopology`

Rule kind: `TRuleKind.eRule_RoutingTopology` (value 7).
Interface: `IPCB_RoutingTopologyRule`.

| Key | Description | Example |
|-----|-------------|---------|
| `TOPOLOGY` | Topology type string | `"Shortest"` |

**TOPOLOGY values** (from `TNetTopology` enum, `xPCBTypes/Consts.cs`):
`"Shortest"`, `"Horizontal"`, `"Vertical"`, `"Daisy_Simple"`,
`"Daisy_MidDriven"`, `"Daisy_Balanced"`, `"Starburst"`.

Default attributes: `NETSCOPE=AnyNet|LAYERKIND=SameLayer|TOPOLOGY=Shortest`

#### `RULEKIND=RoutingPriority`

Rule kind: `TRuleKind.eRule_RoutingPriority` (value 8).
Interface: `IPCB_RoutingPriorityRule`.

| Key | Description | Example |
|-----|-------------|---------|
| `PRIORITY` | Routing priority integer | `"0"` |

Interface-specific method: `GetState_RoutingPriority()` / `SetState_RoutingPriority(int)`.

#### `RULEKIND=RoutingLayers`

Rule kind: `TRuleKind.eRule_RoutingLayers` (value 9).
Interface: `IPCB_RoutingLayersRule`.
Interface methods: `GetState_RoutingLayers(TV7_Layer)` / `SetState_RoutingLayers(...)` / `ResetRoutingLayers()`.

Layer keys use legacy V5 layer name strings suffixed with `_V5`:

| Key | Example |
|-----|---------|
| `TOP LAYER_V5` | `"TRUE"` / `"FALSE"` |
| `MID LAYER N_V5` | `"TRUE"` (N = 1..30) |
| `BOTTOM LAYER_V5` | `"TRUE"` |

Default: all layers enabled.

#### `RULEKIND=RoutingCorners`

Rule kind: `TRuleKind.eRule_RoutingCornerStyle` (value 10).
Interface: `IPCB_RoutingCornerStyleRule`.
Source: `RT_PCB/IPCB_RoutingCornerStyleRule.cs`.

| Key | Description | Example |
|-----|-------------|---------|
| `STYLE` | Corner style | `"90"` / `"45"` / `"Round"` |
| `MINSTUBLEN` | Minimum setback (coord) | |
| `MAXSTUBLEN` | Maximum setback (coord) | |

**STYLE values** (from `TCornerStyle` enum, `RT_PCB/TCornerStyle.cs`):
`eCornerStyle_90`, `eCornerStyle_45`, `eCornerStyle_Round`.

Interface methods: `GetState_Style()`, `GetState_MinSetBack()`, `GetState_MaxSetBack()`.

#### `RULEKIND=RoutingVias`

Rule kind: `TRuleKind.eRule_RoutingViaStyle` (value 11).
Interface: `IPCB_RoutingViaStyleRule`.
Source: `RT_PCB/IPCB_RoutingViaStyleRule.cs`.

| Key | Description | Example |
|-----|-------------|---------|
| `WIDTH` | Preferred via outer diameter | `"50mil"` |
| `MINWIDTH` | Minimum via outer diameter | `"50mil"` |
| `MAXWIDTH` | Maximum via outer diameter | `"50mil"` |
| `HOLEWIDTH` | Preferred via hole diameter | `"28mil"` |
| `MINHOLEWIDTH` | Minimum via hole diameter | `"28mil"` |
| `MAXHOLEWIDTH` | Maximum via hole diameter | `"28mil"` |
| `VIASTYLE` | Via style string | `"Through Hole"` |
| `USEVIA TEMPLATES` | Whether to use via templates | `"FALSE"` |

**VIASTYLE values** (from `TRouteVia` enum, `RT_PCB/TRouteVia.cs`):
`eViaThruHole`, `eViaBlindBuriedPair`, `eViaBlindBuriedAny`, `eViaNone`.
String values: `"Through Hole"` corresponds to `eViaThruHole`.

When `USEVIA TEMPLATES=TRUE`, the rule also has via template GUID references.
Interface methods: `AddViaTemplate()`, `GetViaTemplate(index)`, `DeleteAllViaTemplates()`.

Default attributes: `NETSCOPE=AnyNet|LAYERKIND=SameLayer|HOLEWIDTH=28mil|WIDTH=50mil|VIASTYLE=Through Hole|MINHOLEWIDTH=28mil|MINWIDTH=50mil|MAXHOLEWIDTH=28mil|MAXWIDTH=50mil`

#### `RULEKIND=DiffPairsRouting`

Rule kind: `TRuleKind.eRule_DifferentialPairsRouting` (value 51).
Interface: `IPCB_DifferentialPairsRoutingRule` / `IPCB_DifferentialPairsRoutingRule2` / `IPCB_DifferentialPairsRoutingRule3`.
Sources: `RT_PCB/IPCB_DifferentialPairsRoutingRule*.cs`.

Per-layer width/gap keys use the pattern `<LAYERNAME>_MINWIDTH`, `<LAYERNAME>_MAXWIDTH`,
`<LAYERNAME>_PREFWIDTH`, `<LAYERNAME>_MINGAP`, `<LAYERNAME>_MAXGAP`, `<LAYERNAME>_PREFGAP`.
Layer name prefix patterns (from default attributes):
`TOPLAYER`, `MIDLAYER1`..`MIDLAYER30`, `BOTTOMLAYER`.

| Key | Description | Example |
|-----|-------------|---------|
| `MAXLIMIT` | Maximum gap | `"10mil"` |
| `MINLIMIT` | Minimum gap | `"10mil"` |
| `MOSTFREQGAP` | Preferred/most-frequent gap | `"10mil"` |
| `MAXUNCOUPLEDLENGTH` | Maximum uncoupled length | `"500mil"` |
| `IMPEDANCEDRIVEN` | Impedance-driven flag | `"FALSE"` |
| `MINIMPEDANCE` | Min impedance (ohms) | |
| `MAXIMPEDANCE` | Max impedance (ohms) | |
| `FAVOREDIMPEDANCE` | Favored impedance (ohms) | |
| `IMPEDANCEPROFILEID` | Impedance profile GUID | |
| `TOPLAYER_MINWIDTH` | Min width on top layer | `"15mil"` |
| `TOPLAYER_MAXWIDTH` | Max width on top layer | `"15mil"` |
| `TOPLAYER_PREFWIDTH` | Preferred width on top layer | `"15mil"` |
| `MIDLAYER1_MINWIDTH` etc. | Per mid-layer widths | |
| `BOTTOMLAYER_MINWIDTH` etc. | Bottom layer widths | |

#### `RULEKIND=RoutingNeckDown`

Rule kind: `TRuleKind.eRule_RoutingNeckDown` (value 72).
Interface: `IPCB_RoutingNeckDownRule`.
Source: `RT_PCB/IPCB_RoutingNeckDownRule.cs`.
Stored in `NewRules6` when `eHasNeckDownRuleAtWriteStage` feature flag is set.

Interface method: `GetState_MaxLength()` returns `IPCB_LayerToCoord` (per-layer length map).

---

## Active Route Session Storage

There is no "active route session" stream in the PcbDoc CFB. Routing is purely
interactive and in-memory. When a routing session completes, only the resulting
PCB primitives (tracks = `Tracks6`, vias = `Vias6`) are written to the file.

The `IsPreRoute` flag (`GetState_IsPreRoute()` on `IPCB_Primitive`) marks tracks
and arcs that were placed before routing (pre-routes / constraint locks). This flag
is part of the standard 13-byte common header for PCB primitives (bit in the
`flags` u16 field at header offset 1-2). See `memory/pcb-header.md`.

The `UserRouted` flag (`GetState_UserRouted()` on `IPCB_Primitive`) marks whether
a track/via was placed by the user during interactive routing vs. imported or
auto-routed. Also part of the primitive flags.

---

## Routing Configuration Persistence (Router Options Objects)

The `TOptionsObjectId` enum (`RT_PCB/TOptionsObjectId.cs`) lists the recognized
options object types. The routing-related ones are:

| Enum Value | Name | CFB Section |
|------------|------|-------------|
| `eSpecctraRouterOptions` | Specctra auto-router | `Advanced Router Options6` |
| `eAdvancedRouterOptions` | Advanced (Situs) router | (possibly same section or not persisted) |
| `eInteractiveRoutingOptions` | Interactive routing | NOT in PcbDoc CFB |
| `eAdvancedPlacerOptions` | Auto-placer | `Advanced Placer Options6` |
| `eDesignRuleCheckerOptions` | DRC options | `Design Rule Checker Options6` |

The `IPCB_AbstractOptions` base interface (`RT_PCB/IPCB_AbstractOptions.cs`) defines
the serialization contract:

```csharp
void Export_ToParameters(StringBuilder argParameters);
void Import_FromParameters(TUnit argDisplayUnit, StringBuilder argParameters);
void Export_ToParameters_Version4(StringBuilder argParameters);  // V4 format
void Import_FromParameters_Version4(TUnit argDisplayUnit, StringBuilder argParameters);
void Export_ToParameters_Version3(StringBuilder argParameters);  // V3 (legacy)
void Import_FromParameters_Version3(TUnit argDisplayUnit, StringBuilder argParameters);
```

The `IPCB_InteractiveRoutingOptions` additional state (`RT_PCB/IPCB_InteractiveRoutingOptions.cs`)
includes:

- `GetState_PlaceTrackMode()` / `SetState_PlaceTrackMode(TPlaceTrackMode)` — track placement mode
- Coordinate state: `StartX/Y`, `BeginX/Y`, `MidX/Y`, `EndLineX/Y`, `OldTrackArc*` — these are
  transient routing cursor positions, not persisted in the file.
- `Export_ToParameters_GeneralOptions()` / `Export_ToParameters_LayerOptions()` — split export.

**TPlaceTrackMode** values (`RT_PCB/TPlaceTrackMode.cs`): `ePlaceTrackNone`,
`ePlaceTrackAny`, `ePlaceTrack9090`, `ePlaceTrack4590`, `ePlaceTrack90Arc`.

**TAdvancedRouteMode** values (`RT_PCB/TAdvancedRouteMode.cs`):
`eARIgnoreObstacle`, `eARWalkAroundObstacle`, `eARPushObstacle`,
`eARHugAndPushObstacle`, `eARStopAtFirstObstacle`,
`eARAutoRouteCurrentLayer`, `eARAutoRouteMultiLayer`.

**TInteractiveRouteMode** values (`RT_PCB/TInteractiveRouteMode.cs`):
`eIgnoreObstacle`, `eAvoidObstacle`, `ePushObstacle`.

**TSmartRouteMode** values (`RT_PCB/TSmartRouteMode.cs`):
`eSRIgnoreObstacle`, `eSRAvoidObstacle`, `eSRWalkAroundObstacle`, `eSRPushObstacle`.

---

## Design Rule Storage Related to Routing

### Rule Primitive Object ID

Rules are stored as PCB primitives with `TObjectId.eRuleObject`. In the `Rules6`
section, rule records are parameter blocks (not binary structs). The `TObjectId`
byte for a rule is `eRuleObject` (from `xPCBTypes/Consts.cs`: `obj3.Add(TObjectId.eRuleObject, "Rule")`).

### Rules That Route Through PCB (Routing-Specific)

From `TRuleKind` enum (`RT_PCB/TRuleKind.cs`) and `cRuleIdStrings` mapping:

| Enum value | RULEKIND string | Human name |
|------------|-----------------|------------|
| `eRule_RoutingTopology` (7) | `"RoutingTopology"` | Routing Topology |
| `eRule_RoutingPriority` (8) | `"RoutingPriority"` | Routing Priority |
| `eRule_RoutingLayers` (9) | `"RoutingLayers"` | Routing Layers |
| `eRule_RoutingCornerStyle` (10) | `"RoutingCorners"` | Routing Corners |
| `eRule_RoutingViaStyle` (11) | `"RoutingVias"` | Routing Via Style |
| `eRule_BrokenNets` (16) | `"UnRoutedNet"` | Un-Routed Net |
| `eRule_ViasUnderSMD` (18) | `"ViasUnderSMD"` | Vias Under SMD |
| `eRule_MaximumViaCount` (19) | `"MaximumViaCount"` | Maximum Via Count |
| `eRule_SMDNeckDown` (52) | `"SMDNeckDown"` | SMD Neck-Down |
| `eRule_LayerPair` (53) | `"LayerPairs"` | Layer Pairs |
| `eRule_FanoutControl` (54) | `"FanoutControl"` | Fanout Control |
| `eRule_DifferentialPairsRouting` (56) | `"DiffPairsRouting"` | Differential Pairs Routing |
| `eRule_RoutingNeckDown` (72) | `"RoutingNeckDown"` | Routing Neck-Down |

### cOldRuleSection Array

`xPCBTypes/Consts.cs` defines a `cOldRuleSection` array of 67 `TRuleKind` values.
This tracks which rule kinds go into the legacy `Rules6` section vs. the newer
`NewRules6` section. The exact contents are initialized with a static array
(decompiler shows `RuntimeHelpers.InitializeArray` with a `LdMemberToken` that
is not representable in the C# decompilation). Must be resolved via Delphi
analysis if needed.

### cRoutingRules Array

`xPCBTypes/Consts.cs` defines `cRoutingRules` as a 10-element `TRuleKind[]` array.
Again the exact contents require Delphi analysis. This likely contains the 10
routing-specific rule kinds (Topology, Priority, Layers, Corners, Vias, NeckDown,
DiffPairs, plus a few others).

---

## Observations / Open Questions

1. **Rules6 prefix byte**: The 2-byte prefix before each parameter block in
   `Rules6` / `NewRules6` has an unspecified meaning. Based on `pcb-files.md`
   §6.3 it is "section-specific." Needs Delphi reverse engineering to confirm
   semantics (may encode rule category or version).

2. **STYLE key for RoutingCorners**: The decompiled code shows `MinSetBack` /
   `MaxSetBack` as the data model property names but the actual parameter key
   names emitted by `Export_ToParameters` in Delphi may differ (e.g. `MINSTUBLEN`
   / `MAXSTUBLEN`). Confirm from real file inspection.

3. **Interactive Routing Options persistence location**: Where `IPCB_InteractiveRoutingOptions`
   saves its state is not in the dotnet layer. Likely a DXP preferences INI file
   (`DXP.INI` or `PCBPreferences.ini`) or the Windows registry. This is not
   relevant for PcbDoc round-trip support.

4. **NewRules6 vs Rules6 split**: The `cOldRuleSection` array (67 elements) and
   feature flags control which rules go where. The NeckDown rule
   (`eHasNeckDownRuleAtWriteStage` / bit 18) and per-layer clearance
   (`eHasClearanceByLayerRuleAtWriteStage` / bit 13) are confirmed to be in
   `NewRules6`. Other "new" rules need Delphi analysis.

5. **DiffPairs per-layer gap keys**: The interface shows `GetState_MaxGap(TV7_Layer)` /
   `GetState_MinGap(TV7_Layer)` but the default attributes show `MAXLIMIT` /
   `MINLIMIT` for the global values and `TOPLAYER_MINGAP` etc. for per-layer.
   Verify from real file samples whether per-layer gap keys follow the same
   `<LAYERNAME>_MINGAP` pattern as width keys.

6. **Router session recovery**: Altium has a "PCB Recovery" feature. This saves
   a backup `.PcbDoc` as normal; there is no evidence of a separate "routing
   session checkpoint" stream in the CFB format.

7. **IPCB_RoutingViaStackInfo / IPCB_ViaRoutingDataInfo**: These interfaces
   (`RT_PCB/IPCB_RoutingViaStackInfo.cs`, `RT_PCB/IPCB_ViaRoutingDataInfo.cs`)
   relate to via routing data but appear to be runtime-only query interfaces
   (differential pair routing state), not file format elements.

---

## Key Source Files

| File | Purpose |
|------|---------|
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TRuleKind.cs` | Full `TRuleKind` enum |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/Consts.cs` | `cRuleIdStrings` mapping, parameter name constants |
| `AD26-dotnet/Altium.Edp.Interfaces/xPCBTypes/Consts.cs` | `cRoutingRules`, `cOldRuleSection`, `cNetTopologyStrings` |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TOptionsObjectId.cs` | Options object ID enum |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_AbstractOptions.cs` | Options serialization interface |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_InteractiveRoutingOptions.cs` | Interactive routing options |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_SpecctraRouterOptions.cs` | Specctra router options |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_RoutingCornerStyleRule.cs` | Corner style rule interface |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_RoutingViaStyleRule.cs` | Via style rule interface |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_RoutingLayersRule.cs` | Routing layers rule interface |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_RoutingPriorityRule.cs` | Routing priority rule interface |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_RoutingNeckDownRule.cs` | Neck-down rule interface |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_DifferentialPairsRoutingRule.cs` | Diff pairs rule interface |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TAdvancedRouteMode.cs` | Advanced route mode enum |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TInteractiveRouteMode.cs` | Interactive route mode enum |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TCornerStyle.cs` | Corner style enum |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TRouteVia.cs` | Via type enum |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TNetScope.cs` | Net scope enum |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TRuleLayerKind.cs` | Rule layer kind enum |
| `AD26-dotnet/Altium.Sch.Base/Altium.Sch.Base/DefaultRulesPropertiesValues.cs` | Default rule attribute strings |
| `docs/dxp/pcb-files.md` | CFB structure, section types, loading pipeline |
