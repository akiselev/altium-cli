# Routing Rules Parameter Audit

Audit of routing-specific PCB rule kinds comparing C# implementations (AD26-dotnet/)
against Rust structs in `crates/altium-format/src/pcbdoc/drc.rs`.

Sources:
- C# COM interfaces in `AD26-dotnet/Altium.SDK.Interfaces/PCB/`
- C# RT_PCB interfaces in `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/`
- C# ConstraintsManager mapping in `AD26-dotnet/ConstraintsManager.Module/.../PcbIntegrationMapper.cs`
- C# default attribute strings in `AD26-dotnet/Altium.Sch.Base/.../DefaultRulesPropertiesValues.cs`
- C# SchDataImporterSheetV4Binary for parameter key string literals

Parameter key names below are the strings used in `Export_ToParameters` / `Import_FromParameters`
(Delphi side), inferred from the attribute strings and interface property names.

---

## RoutingTopology (kind 7)

### C# Parameters (from Export_ToParameters)
- TOPOLOGY (NetTopology enum as string, e.g. "Shortest", "Horizontal", "Vertical", "DaisySimple", "DaisyMidDriven", "DaisyBalanced", "Starburst")

### Rust Parameters (from drc.rs)
- TOPOLOGY (NetTopology)

### Missing in Rust
- **None** -- complete.

---

## RoutingPriority (kind 8)

### C# Parameters (from Export_ToParameters)
- ROUTINGPRIORITY (i32, routing priority value)

### Rust Parameters (from drc.rs)
- ROUTINGPRIORITY (i32)

### Missing in Rust
- **None** -- complete.

---

## RoutingLayers (kind 9)

### C# Parameters (from Export_ToParameters)
Per-layer boolean flags using legacy V5 naming:
- TOP LAYER_V5 (bool)
- MID LAYER 1_V5 through MID LAYER 30_V5 (bool, per mid layer)
- BOTTOM LAYER_V5 (bool)

### Rust Parameters (from drc.rs)
- TOP LAYER_V5 (bool)
- MID LAYER 1_V5 through MID LAYER 30_V5 (bool)
- BOTTOM LAYER_V5 (bool)

### Missing in Rust
- **None** -- complete.

---

## RoutingCornerStyle (kind 10)

### C# Parameters (from Export_ToParameters)
- CORNERSTYLE (CornerStyle enum: "45", "90", "Round")
- MINSETBACK (MilCoord, minimum setback distance)
- MAXSETBACK (MilCoord, maximum setback distance)

### Rust Parameters (from drc.rs)
- CORNERSTYLE (CornerStyle)
- MINSETBACK (MilCoord)
- MAXSETBACK (MilCoord)

### Missing in Rust
- **None** -- complete.

---

## RoutingViaStyle (kind 11)

### C# Parameters (from Export_ToParameters)
- MINHOLEWIDTH (MilCoord, minimum hole width)
- MAXHOLEWIDTH (MilCoord, maximum hole width)
- HOLEWIDTH (MilCoord, preferred hole width -- note key name is "HOLEWIDTH" not "PREFEREDHOLEWIDTH")
- MINWIDTH (MilCoord, minimum via diameter)
- MAXWIDTH (MilCoord, maximum via diameter)
- WIDTH (MilCoord, preferred via diameter -- note key name is "WIDTH" not "PREFEREDWIDTH")
- VIASTYLE (RouteVia enum: "Through Hole", etc.)
- USEVIATEMPLATES (bool, whether to use via templates instead of size limits)
- VIATEMPLATEGUID#N (String, GUID of Nth via template, 1-based index)
- VIATEMPLATENAME#N (String, name of Nth via template, 1-based index)

### Rust Parameters (from drc.rs)
- MINHOLEWIDTH (MilCoord)
- MAXHOLEWIDTH (MilCoord)
- HOLEWIDTH (MilCoord)
- MINWIDTH (MilCoord)
- MAXWIDTH (MilCoord)
- WIDTH (MilCoord)
- VIASTYLE (RouteVia)
- USEVIATEMPLATES (bool)
- VIATEMPLATEGUID#N (String, 1-based)
- VIATEMPLATENAME#N (String, 1-based)

### Missing in Rust
- **None** -- complete.

---

## RoutingNeckDown (kind 67)

### C# Parameters (from Export_ToParameters)
The C# `IPCB_RoutingNeckDownRule` (RT_PCB) interface exposes:
- `GetState_MaxLength()` returning `IPCB_LayerToCoord` -- a **per-layer coord map**
  with key=TV7_Layer and value=i32 coord. This is serialized as per-layer parameters.

The Delphi Export_ToParameters for this rule serializes per-layer max-length values.
Based on the constraint manager mapping code (line 3046):
```
iPCB_RoutingNeckDownRule.GetState_MaxLength().SetData(argKey, length_or_minus1)
```
The layer key is a TV7_Layer identifier. The parameter format is:
- NECKDOWNPERCENTAGE (f64, neck-down percentage -- legacy/simple mode)
- Per-layer max length params using layer V7 keys (format TBD from Delphi, likely `MAXLENGTH_<layerkey>=<coord>`)

### Rust Parameters (from drc.rs)
- NECKDOWNPERCENTAGE (f64)

### Missing in Rust
- **Per-layer max length data** (IPCB_LayerToCoord map) -- the entire per-layer neck-down
  length configuration is missing. The constraint manager writes per-layer length values
  via `GetState_MaxLength().SetData(key, value)`. This is a per-layer coord map that
  gets serialized as parameters. **SEVERITY: HIGH** -- per-layer neck-down lengths will
  be silently dropped.

---

## DifferentialPairsRouting (kind 51)

### C# Parameters (from Export_ToParameters)
Global parameters:
- MINLIMIT (MilCoord, min gap limit -- DiffPairsRoutingRule3.GetState_MinLimit)
- MAXLIMIT (MilCoord, max gap limit -- DiffPairsRoutingRule3.GetState_MaxLimit)
- MOSTFREQGAP (MilCoord, most frequent gap -- DiffPairsRoutingRule3.GetState_MostFrequentGap)
- MOSTFREQUENTWIDTH (MilCoord, most frequent width -- DiffPairsRoutingRule3.SetState_MostFrequentWidth)
- MAXUNCOUPLEDLENGTH (MilCoord, max uncoupled length)
- IMPEDANCEPROFILEDRIVEN (bool, whether impedance-driven)
- IMPEDANCEPROFILEID (String, GUID of impedance profile)
- IMPEDANCEPROFILEVALUE (f64, impedance value -- from IPCB_DifferentialPairsRoutingRule: MinImpedance/MaxImpedance/FavoredImpedance)
- FILTERLAYERSTACKID (String, layer stack filter ID -- DiffPairsRoutingRule3.GetState_FilterLayerStackID)

Per-layer parameters (TOPLAYER, MIDLAYER1..30, BOTTOMLAYER prefixes):
- {PREFIX}_MINWIDTH (MilCoord)
- {PREFIX}_MAXWIDTH (MilCoord)
- {PREFIX}_PREFWIDTH (MilCoord)
- {PREFIX}_MINGAP (MilCoord)
- {PREFIX}_MAXGAP (MilCoord)
- {PREFIX}_PREFGAP (MilCoord)

Per-substack per-layer overrides (indexed by SUBSTACK#N):
- SUBSTACK{N} (String, substack GUID)
- {PREFIX}_{SUBSTACKGUID}_MINWIDTH (MilCoord)
- {PREFIX}_{SUBSTACKGUID}_MAXWIDTH (MilCoord)
- {PREFIX}_{SUBSTACKGUID}_PREFWIDTH (MilCoord)
- {PREFIX}_{SUBSTACKGUID}_MINGAP (MilCoord)
- {PREFIX}_{SUBSTACKGUID}_MAXGAP (MilCoord)
- {PREFIX}_{SUBSTACKGUID}_PREFGAP (MilCoord)

### Rust Parameters (from drc.rs)
- MINLIMIT (MilCoord)
- MAXLIMIT (MilCoord)
- MOSTFREQGAP (MilCoord)
- MAXUNCOUPLEDLENGTH (MilCoord)
- IMPEDANCEPROFILEDRIVEN (Option<bool>)
- IMPEDANCEPROFILEID (Option<String>)
- IMPEDANCEPROFILEVALUE (Option<f64>)
- Per-layer: {PREFIX}_MINWIDTH, _MAXWIDTH, _PREFWIDTH, _MINGAP, _MAXGAP, _PREFGAP
- Per-substack: SUBSTACK{N}, {PREFIX}_{GUID}_MINWIDTH, etc.

### Missing in Rust
- **MOSTFREQUENTWIDTH** (MilCoord, most frequent width) -- exposed via
  `DiffPairsRoutingRule3.GetState_MostFrequentWidth()`. Not parsed. **SEVERITY: MEDIUM** --
  value will be dropped on roundtrip.
- **FILTERLAYERSTACKID** (String, layer stack filter ID) -- exposed via
  `DiffPairsRoutingRule3.GetState_FilterLayerStackID()`. Not parsed. **SEVERITY: MEDIUM** --
  value will be dropped on roundtrip.

---

## FanoutControl (kind 49)

### C# Parameters (from Export_ToParameters)
- FANOUTSTYLE (FanoutStyle enum: "Auto", "BGA", "Rows", "Staggered", "UnderPads")
- FANOUTDIRECTION (FanoutDirection enum: "None", "InOnly", "OutOnly", "InThenOut", "OutThenIn", "Alternating")
- BGADIR (BgaFanoutDirection enum: "Out", "NE", "SE", "SW", "NW", "In")
- BGAVIAMODE (BgaFanoutViaMode enum: "Closest", "Centered")
- VIAGRID (MilCoord, via grid spacing)

### Rust Parameters (from drc.rs)
- BGADIR (BgaFanoutDirection)
- BGAVIAMODE (BgaFanoutViaMode)
- FANOUTSTYLE (FanoutStyle)
- FANOUTDIRECTION (FanoutDirection)
- VIAGRID (MilCoord)

### Missing in Rust
- **None** -- complete.

---

## MatchedLengths (kind 4)

### C# Parameters (from Export_ToParameters)
From IPCB_MatchedNetLengthsConstraint (SDK + RT_PCB):
- TOLERANCE (MilCoord, length tolerance)
- AMPLITUDE (MilCoord, serpentine amplitude)
- GAP (MilCoord, serpentine gap)
- STYLE (LengthenerStyle enum, serpentine style)
- CHECKNETSINDIFFPAIR (bool, check nets within diff pair)
- CHECKDIFFPAIRVSDIFFPAIR (bool, check diff pair vs diff pair)
- CHECKOTHERS / CHECKXSIGNALS (bool, check other electrical objects / between x-signals)
- USEDELAYUNITS (bool, use delay instead of length)
- DELAYTOLERANCE (f64, delay tolerance in seconds)
- TARGETSOURCENAME (String, target source net name)
- PHASEMATCHING (bool, enable phase matching)
- PHASETOLERANCE (MilCoord, phase tolerance)
- PHASEDELAYTOLERANCE (f64, phase delay tolerance in seconds)
- PHASEDISTANCE (MilCoord, phase distance)

### Rust Parameters (from drc.rs)
- TOLERANCE (MilCoord)
- CHECKNETSINDIFFPAIR (bool)
- CHECKDIFFPAIRVSDIFFPAIR (bool)
- CHECKXSIGNALS (bool)
- CHECKOTHERS (bool)
- USEDELAYUNITS (bool)
- DELAYTOLERANCE (f64)
- TARGETSOURCENAME (String)
- PHASEMATCHING (bool)
- PHASETOLERANCE (MilCoord)
- PHASEDELAYTOLERANCE (f64)
- PHASEDISTANCE (MilCoord)

### Missing in Rust
- **AMPLITUDE** (MilCoord, serpentine amplitude) -- from `GetState_Amplitude()`.
  **SEVERITY: HIGH** -- serpentine tuning data will be dropped.
- **GAP** (MilCoord, serpentine gap) -- from `GetState_Gap()`.
  **SEVERITY: HIGH** -- serpentine tuning data will be dropped.
- **STYLE** (LengthenerStyle enum, serpentine style) -- from `GetState_Style()`.
  **SEVERITY: HIGH** -- serpentine tuning data will be dropped.

---

## Length (kind 3)

### C# Parameters (from Export_ToParameters)
From IPCB_MaxMinLengthConstraint (SDK + RT_PCB):
- MAXLIMIT (MilCoord, maximum length)
- MINLIMIT (MilCoord, minimum length)
- USEDELAYUNITS (bool, use delay units instead of length)
- MAXDELAY (f64, maximum delay in seconds)
- MINDELAY (f64, minimum delay in seconds)

### Rust Parameters (from drc.rs)
- MINLIMIT (MilCoord)
- MAXLIMIT (MilCoord)
- USEDELAYUNITS (bool)
- MINDELAY (f64)
- MAXDELAY (f64)

### Missing in Rust
- **None** -- complete.

---

# Summary

| Rule Kind | Kind # | Status | Missing Parameters |
|-----------|--------|--------|-------------------|
| RoutingTopology | 7 | COMPLETE | -- |
| RoutingPriority | 8 | COMPLETE | -- |
| RoutingLayers | 9 | COMPLETE | -- |
| RoutingCornerStyle | 10 | COMPLETE | -- |
| RoutingViaStyle | 11 | COMPLETE | -- |
| RoutingNeckDown | 67 | **INCOMPLETE** | Per-layer max length data (IPCB_LayerToCoord map) |
| DifferentialPairsRouting | 51 | **INCOMPLETE** | MOSTFREQUENTWIDTH, FILTERLAYERSTACKID |
| FanoutControl | 49 | COMPLETE | -- |
| MatchedLengths | 4 | **INCOMPLETE** | AMPLITUDE, GAP, STYLE (serpentine params) |
| Length | 3 | COMPLETE | -- |

## Critical Findings

### 1. MatchedLengths -- Missing serpentine parameters (SEVERITY: HIGH)
The `MatchedLengthsRuleData` struct is missing three serpentine/lengthener parameters:
- `AMPLITUDE` (MilCoord) -- serpentine amplitude
- `GAP` (MilCoord) -- serpentine gap
- `STYLE` (LengthenerStyle enum) -- serpentine style ("Round", "Mitered", etc.)

These are part of the core `IPCB_MatchedNetLengthsConstraint` interface (both SDK and RT_PCB).
They control how the interactive length tuner creates serpentine patterns. If a user has
configured these in their rules, they will be silently dropped on save, corrupting the
design intent.

### 2. RoutingNeckDown -- Missing per-layer max length map (SEVERITY: HIGH)
The `RoutingNeckDownRuleData` struct only has `NECKDOWNPERCENTAGE`. The C# RT_PCB interface
`IPCB_RoutingNeckDownRule` exposes `GetState_MaxLength()` returning an `IPCB_LayerToCoord`
map -- a per-layer coord dictionary. The constraint manager code writes per-layer length
values through this map. The Rust code has no support for this per-layer data.

### 3. DifferentialPairsRouting -- Missing MOSTFREQUENTWIDTH and FILTERLAYERSTACKID (SEVERITY: MEDIUM)
Two parameters from `IPCB_DifferentialPairsRoutingRule3`:
- `MOSTFREQUENTWIDTH` (MilCoord) -- the most frequently used width, used by the router
- `FILTERLAYERSTACKID` (String) -- layer stack filter for sub-stack aware routing

These are newer additions (Rule3 interface) and may not appear in older files, but will be
dropped from files that have them.
