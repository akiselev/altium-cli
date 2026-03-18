# Rules6 Parsing Audit

Audit date: 2026-03-12

Audit of `crates/altium-format/src/pcbdoc/drc.rs` against the decompiled C# source in
`AD26-dotnet/`. The goal is to find any parameters that our Rust implementation silently
drops by using `EmptyRuleData` or by missing fields in typed rule structs.

## EmptyRuleData Audit

Seven rule kinds currently use `EmptyRuleData` (meaning we parse zero kind-specific params).
For each, we checked the C# COM interfaces (`IPCB_*Rule`) and ConstraintsManager data
models to determine whether they truly have no serialized parameters.

### ComponentRotations (RuleKind 25) -- NOT EMPTY, BUG

**C# interface:** `IPCB_ComponentRotationsRule` declares:
- `GetState_AllowedRotations() -> int`
- `SetState_AllowedRotations(int)`

**C# data model:** `ComponentOrientationKind` is a `[Flags] enum : short` with values:
- RotationNone = 0, Rotation0 = 1, Rotation90 = 2, Rotation180 = 8, Rotation270 = 0x10, RotationsAll = 0x20

**ConstraintsManager mapping** (`PcbIntegrationMapper.cs`):
- `((IPCB_ComponentRotationsRule)constraint).SetState_AllowedRotations((int)data.Orientation)`
- Reads back via `(ComponentOrientationKind)iPCB_ComponentRotationsRule.GetState_AllowedRotations()`

**Conclusion:** This rule serializes an `ALLOWEDROTATIONS` (or similar) parameter as an
integer bitmask. We are silently dropping it. **This is a cardinal rule violation.**

**Fix required:** Add a `ComponentRotationsRuleData` struct with an `allowed_rotations: i32`
field (or a typed flags enum). Parse from `ALLOWEDROTATIONS` key. Confirm exact key name via
Ghidra or test file inspection.

### PermittedLayers (RuleKind 26) -- NOT EMPTY, BUG

**C# interface:** `IPCB_PermittedLayersRule` declares:
- `GetState_PermittedLayers() -> TV6_LayerSet`
- `SetState_PermittedLayers(ref TV6_LayerSet)`

**C# data model:** `PermittedLayersData` has `UseTopLayer: bool` and `UseBottomLayer: bool`.
The `TV6_LayerSet` is a Delphi set type with up to 83 elements.

**ConstraintsManager mapping** (`PcbIntegrationMapper.cs`):
- Reads `TV6_LayerSet` from the rule, checks `ContainsValue(eV6_TopLayer)` and
  `ContainsValue(eV6_BottomLayer)`
- The set is serialized to parameters by the Delphi `Export_ToParameters` method

**Conclusion:** This rule serializes permitted layer data. The exact parameter key format
depends on Delphi serialization (likely `PERMITTEDLAYERS` as a hex-encoded layer set, or
individual `TOPLAYER`/`BOTTOMLAYER` booleans). We are silently dropping it.
**This is a cardinal rule violation.**

**Fix required:** Determine exact parameter format via Ghidra or test file inspection,
then add `PermittedLayersRuleData` struct.

### NetsToIgnore (RuleKind 27) -- TRULY EMPTY, OK

**C# interface:** `IPCB_NetsToIgnoreRule` extends `IPCB_Rule` with **no additional methods**.

**C# data model:** `NetsToIgnoreData` has no additional properties beyond `BaseRuleData`.
`GetPropertiesState()` returns an empty list.

**Conclusion:** This rule genuinely has no kind-specific parameters. The rule's effect
comes entirely from its scope expressions (`SCOPE1EXPRESSION` etc.), which are parsed in
`PcbRuleBase`. `EmptyRuleData` is correct.

### LayerStack (RuleKind 38) -- LIKELY EMPTY, OK

**C# interface:** No dedicated `IPCB_LayerStackRule` interface found in the codebase.
The only `IPCB_LayerStack*` interfaces are for the layer stack data structure itself
(`IPCB_LayerStack`, `IPCB_LayerStackBase`, etc.), not for the rule.

**Conclusion:** No evidence of kind-specific parameters. The layer stack rule likely
constrains via scope expressions only. `EmptyRuleData` appears correct, but should be
verified with a test file containing this rule.

### UnconnectedPin (RuleKind 45) -- LIKELY EMPTY, OK

**C# interface:** No dedicated `IPCB_UnconnectedPinRule` interface found in the codebase.
Only the enum value `eRule_UnconnectedPin` and display string "Un-Connected Pin Constraint"
exist.

**Conclusion:** No evidence of kind-specific parameters. This is a unary DRC check --
it flags unconnected pins based on scope. `EmptyRuleData` appears correct.

### SilkToBoardRegionClearance (RuleKind 59) -- UNCERTAIN, NEEDS INVESTIGATION

**C# interface:** No dedicated `IPCB_SilkToBoardRegionRule` interface found.
The rule name string is "SilkToBoardRegionClearance".

**Conclusion:** No interface evidence of parameters, but the name "Clearance" suggests
it might have a gap/distance parameter similar to other clearance rules. Needs
verification via test file or Ghidra decompilation of the Delphi `Export_ToParameters`.
Currently classified as possibly empty but uncertain.

### None (RuleKind 61) -- TRULY EMPTY, OK

**C# interface:** No dedicated interface. The rule name string is simply "None".

**Conclusion:** This is a placeholder/sentinel rule kind. `EmptyRuleData` is correct.


## Missing RuleKind Values

### Rust enum vs C# TRuleKind (SDK version)

Our Rust `RuleKind` enum (in `crates/altium-format-types/src/pcb.rs`) covers values 0-69,
matching the SDK `TRuleKind` exactly:

| Rust enum value | C# SDK enum | Rust value |
|---|---|---|
| Clearance | eRule_Clearance | 0 |
| ... (all match) ... | ... | ... |
| ZAxisClearance | eRule_ZAxisClearance | 69 |

**SDK `TRuleKind` last value:** `eRule_ZAxisClearance` (confirmed by
`TRuleKindConsts.Last = TRuleKind.eRule_ZAxisClearance`).

**No missing values beyond 69.** Our enum is complete for the current format version.

### Note: Pcbtypes (older Delphi) TRuleKind divergence

The older Delphi `Pcbtypes.TRuleKind` includes `eRule_Pcad` (between SilkToBoardRegion
and SMDPADEntry) and lacks `eRule_None`. The SDK version (which is authoritative for the
current file format) does NOT include `eRule_Pcad` and DOES include `eRule_None`. Our
Rust enum correctly follows the SDK version.


## Missing Parameters in Complex Rules

For each rule kind with typed data, we compared our Rust struct fields against the C#
COM interface properties. Only fields that would be serialized to/from parameters are
relevant (runtime-only state like `GetState_Selected` is not persisted).

### Clearance (RuleKind 0)

**C# interface properties beyond IPCB_Rule:**
- `Gap` (int) -- we have `GAP`
- `Mode` (`TClearanceConstraintMode`) -- NOT PARSED
- `IgnorePadToPad` (bool) -- we have `IGNOREPADTOPADCLEARANCEINFOOTPRINT`
- `IsMatrix` (bool) -- NOT PARSED (controls whether OBJECTCLEARANCES matrix is used)

**Missing parameters:**
- `MODE` or `CLEARANCEMODE` -- determines simple gap vs. matrix mode
- `ISMATRIX` -- whether the rule uses a clearance matrix

**Severity:** Medium. The `OBJECTCLEARANCES` matrix is parsed, but the mode flag that
controls whether it's used vs. the simple `GAP` value is missing.

### Width (RuleKind 2) -- COMPLETE

Our implementation parses: MINLIMIT, MAXLIMIT, PREFEREDWIDTH, per-layer overrides,
CHECKCONNECTEDCOPPER, impedance fields (IMPEDANCEDRIVEN, MINIMP, MAXIMP, FAVIMP),
impedance profile fields, and substack overrides.

C# interface confirms: MaxWidth, MinWidth, FavoredWidth (per-layer), ImpedanceDriven,
MinImpedance, MaxImpedance, FavoredImpedance, ImpedanceProfileId, CheckConnectedCopper,
PreferedWidth, MaxLimit, MinLimit, substack variants.

**No missing parameters detected.**

### RoutingViaStyle (RuleKind 11) -- COMPLETE

Our implementation parses: MINHOLEWIDTH, MAXHOLEWIDTH, HOLEWIDTH (preferred),
MINWIDTH, MAXWIDTH, WIDTH (preferred), VIASTYLE, USEVIATEMPLATES, and indexed
VIATEMPLATEGUID#N/VIATEMPLATENAME#N pairs.

C# interface confirms: MinHoleWidth, MaxHoleWidth, PreferedHoleWidth, MinWidth,
MaxWidth, PreferedWidth, ViaStyle, UseViaTemplates, via template management.

**No missing parameters detected.**

### PolygonConnectStyle (RuleKind 20) -- MISSING PARAMETERS

**C# interface properties beyond what we parse:**
- `GetState_MinDistance()` / `SetState_MinDistance(int)` -- NOT PARSED
- `GetState_EnableMinDistance()` / `SetState_EnableMinDistance(bool)` -- NOT PARSED
- `GetState_ConductorByPadEdge()` / `SetState_ConductorByPadEdge(bool)` -- NOT PARSED
- Per-type variants: `MinDistanceByType`, `EnableMinDistanceByType`,
  `ConductorByPadEdgeByType` for `TPolygonConnectPrimitiveType` (ePolyVia, ePolyTHPad,
  ePolySMDPad)

**Missing parameters (likely key names):**
- `MINDISTANCE` -- minimum distance for polygon connect
- `ENABLEMINDISTANCE` -- whether min distance check is active
- `CONDUCTORBYPADEDGE` -- whether conductor width is measured from pad edge
- Per-type variants: `THPAD.MINDISTANCE`, `SMDPAD.MINDISTANCE`, `VIA.MINDISTANCE`, etc.
- Per-type variants: `THPAD.ENABLEMINDISTANCE`, `SMDPAD.ENABLEMINDISTANCE`, etc.
- Per-type variants: `THPAD.CONDUCTORBYPADEDGE`, `SMDPAD.CONDUCTORBYPADEDGE`, etc.

**Severity:** High. These control polygon pour behavior around pads.

### DifferentialPairsRouting (RuleKind 51) -- MISSING PARAMETERS

**C# interface properties beyond what we parse:**
- `GetState_ImpedanceDriven()` / `SetState_ImpedanceDriven(bool)` -- NOT PARSED
- `GetState_MinImpedance()` / `SetState_MinImpedance(double)` -- NOT PARSED
- `GetState_MaxImpedance()` / `SetState_MaxImpedance(double)` -- NOT PARSED
- `GetState_FavoredImpedance()` / `SetState_FavoredImpedance(double)` -- NOT PARSED

Our implementation has `impedance_profile_driven`, `impedance_profile_id`, and
`impedance_profile_value`, but these are DIFFERENT from the impedance-driven fields.
The profile fields control impedance-profile-based routing, while the impedance fields
control direct impedance matching.

**Missing parameters (likely key names):**
- `IMPEDANCEDRIVEN` -- whether impedance matching is active
- `MINIMP` -- minimum impedance
- `MAXIMP` -- maximum impedance
- `FAVIMP` -- favored impedance

**Severity:** High. Missing impedance matching parameters for differential pairs.

### MaxMinHoleSize (RuleKind 42) -- COMPLETE

C# interface (`IPCB_MaxMinHoleSizeConstraintEx`): AbsoluteValues, MaxLimit, MinLimit,
MaxPercent, MinPercent. All present in our struct.

**No missing parameters detected.**

### ComponentClearance (RuleKind 24) -- LIKELY COMPLETE

Our fields: GAP, COLLISIONCHECKMODE, VERTICALGAP, SHOWDISTANCES, DONOTCHECKWITHOUT3DBODY.
No dedicated extended C# interface found with additional properties.

### Other Rules Spot-Checked

- **ParallelSegment**: GAP, LIMIT, PARALLELLENGTH, CHECKPARALLEL, CHECKADJACENTLAYERS -- matches
- **SolderMaskExpansion**: EXPANSION, ISTENTINGTOP, ISTENTINGBOTTOM, SOLDERMASKFROMHOLE -- matches
- **PasteMaskExpansion**: EXPANSION, PERCENT, THPADUSETOPPASTE, THPADUSEBOTTOMPASTE -- matches
- **PowerPlaneConnectStyle**: per-type overrides for PAD/VIA -- matches interface
- **Creepage**: GAP, CHECKDISTANCE, APPLYTOPOLYGONPOUR, VOLTAGE -- need to verify against interface


## Recommendations

### Critical (Cardinal Rule Violations -- Silently Dropping Parameters)

1. **ComponentRotations**: Replace `EmptyRuleData` with a struct parsing `ALLOWEDROTATIONS`
   (int bitmask, `ComponentOrientationKind` flags). Verify exact param key via Ghidra or
   test file dump (`altium cfb dump <file> /Rules6/Data --blocks`).

2. **PermittedLayers**: Replace `EmptyRuleData` with a struct parsing the permitted layer
   set. Verify exact param format via Ghidra or test file dump.

### High Priority (Missing Parameters in Complex Rules)

3. **PolygonConnectStyle**: Add `MINDISTANCE` (Coord), `ENABLEMINDISTANCE` (bool),
   `CONDUCTORBYPADEDGE` (bool), and their per-type variants (THPAD.*, SMDPAD.*, VIA.*).

4. **DifferentialPairsRouting**: Add `IMPEDANCEDRIVEN` (bool), `MINIMP` (f64),
   `MAXIMP` (f64), `FAVIMP` (f64) fields.

### Medium Priority

5. **Clearance**: Investigate and add `MODE`/`CLEARANCEMODE` and `ISMATRIX` parameters.

### Low Priority (Verification Needed)

6. **SilkToBoardRegionClearance**: Verify via Ghidra or test file whether this rule truly
   has no kind-specific parameters, or if it has a clearance gap parameter.

7. **LayerStack**: Verify via test file that no parameters are present.

### Investigation Method

For items requiring verification, use:
```bash
# Dump a PcbDoc file containing the rule type
altium cfb dump <file> /Rules6/Data --blocks

# Or use Ghidra to decompile the Delphi Export_ToParameters for the rule class
ghidra decompile altium26 <binary> <address>
```
