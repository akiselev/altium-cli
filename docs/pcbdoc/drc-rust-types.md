# DRC Rust Types — Implementation Design

Research document for implementing typed Rust DRC (Design Rule Check) types based on
the C# interface catalog in `drc-types.md` and empirical analysis of real PcbDoc files.

---

## Table of Contents

- [Current State](#current-state)
- [What Already Exists](#what-already-exists)
- [What Needs To Be Built](#what-needs-to-be-built)
- [String-Keyed Enums](#string-keyed-enums)
- [Rule Base Struct](#rule-base-struct)
- [Rule Kind Dispatch](#rule-kind-dispatch)
- [Concrete Rule Type Structs](#concrete-rule-type-structs)
- [Violation Base Struct](#violation-base-struct)
- [Concrete Violation Type Structs](#concrete-violation-type-structs)
- [Waived Violations](#waived-violations)
- [DRC Options](#drc-options)
- [Storage Architecture (Option C)](#storage-architecture-option-c)
- [Complex Data Structures](#complex-data-structures)
  - [Clearance Matrix](#clearance-matrix)
  - [Per-Layer Rule Params](#per-layer-rule-params)
  - [Confinement Polygon Vertices](#confinement-polygon-vertices)
  - [DiffPairs Violation Polygons](#diffpairs-violation-polygons)
  - [Coord-With-Unit Strings](#coord-with-unit-strings)
  - [WaivedViolations UNICODE Keys](#waivedviolations-unicode-keys)
- [Implementation Order](#implementation-order)
- [Resolved Questions](#resolved-questions)

---

## Current State

All DRC-related sections (Rules6, 38 violation storages, WaivedViolations,
DesignRuleCheckerOptions6) are currently loaded as **opaque `ParamSectionData`**:

```rust
pub(crate) struct ParamSectionData {
    pub(crate) kind: records::ParamSectionKind,
    pub(crate) records: Vec<records::StandardParamRecord>,
}

pub(crate) struct StandardParamRecord {
    pub(crate) params: ParameterCollection,  // raw key-value map
}
```

Sections are dispatched by `ParamSectionKind::from_storage_name()` but all collapse
into the same `PcbDocSection::Parameter(ParamSectionData)` variant. No typed access.

---

## What Already Exists

### In `altium-format-types` (pcb.rs)

| Type | Status | Notes |
|------|--------|-------|
| `RuleKind` (u8, 0-69) | Complete | All 70 variants with `TryFrom<u8>`, `Display` |
| `PlaneConnectionStyle` | Complete | 3 variants (NoConnect, Relief, Direct) |
| `CornerStyle` | Complete | 3 variants (Degree90, Degree45, Round) |
| `DaisyChainStyle` | Complete | 3 variants |
| `PcbObjectId` | Complete | Includes `Rule(16)` and `Violation(19)` variants |
| `ViewableObjectId` | Complete | Includes all rule subtypes (82+) |

### In `altium-format` (derive macros)

| Feature | Status | Notes |
|---------|--------|-------|
| `FromParams` derive | Complete | All param strategies supported |
| `ToParams` derive | Complete | T1/T2 serialization tiers |
| `ParameterCollection` | Complete | Remove/insert methods for all types |
| `FromParamValue` trait | Complete | Implemented for primitives and int-keyed enums |
| `parse_standard_param_records()` | Complete | Used by all violation sections |
| `parse_prefixed_param_records()` | Complete | Used by Rules6 |

### What's Missing

1. **String-keyed `FromParamValue`/`ToParamValue` impls** for enums serialized as strings
   (RULEKIND, NETSCOPE, CONNECTSTYLE, etc.)
2. **Typed structs** for rule/violation/waived/options records
3. **Per-rule-kind dispatch** to concrete rule types
4. **Separate `PcbDocSection` variants** for typed sections (or typed accessor methods)

---

## String-Keyed Enums

### The Problem

Most PCB enums serialize as **u8 discriminants** (`LINESTYLE=2`). But DRC enums use
**string identifiers** mapped via constant arrays in the C# source:

| Parameter | Example Value | Maps To |
|-----------|---------------|---------|
| `RULEKIND` | `"Clearance"` | `RuleKind::Clearance` |
| `NETSCOPE` | `"DifferentNets"` | `NetScope::DifferentNetsOnly` |
| `LAYERKIND` | `"SameLayer"` | `RuleLayerKind::SameLayer` |
| `CONNECTSTYLE` | `"Relief"` | `PlaneConnectStyle::Relief` |
| `TOPOLOGY` | `"Shortest"` | `NetTopology::Shortest` |
| `CORNERSTYLE` | `"45-Degree"` | `CornerStyle::Degree45` |
| `POLYGONRELIEFANGLE` | `"90 Angle"` | `PolygonReliefAngle::Angle90` |
| `VIASTYLE` | `"Through Hole"` | `RouteVia::ThruHole` |
| `COLLISIONCHECKMODE` | `"3"` | Integer-keyed (exception) |
| `CONFINEMENTSTYLE` | `"ConfineIn"` | `ConfinementStyle::ConfineIn` |
| `FANOUTSTYLE` | `"Auto"` | `FanoutStyle::Auto` |

### Solution: Custom `FromParamValue`/`ToParamValue` Impls

For `RuleKind` — already exists in `altium-format-types` but needs string-based
param serialization. Add a custom impl (NOT the `impl_enum_param_value!` macro):

```rust
// In crates/altium-format/src/param_value.rs

impl FromParamValue for RuleKind {
    fn from_param_value(key: &str, value: &str) -> Result<Self> {
        // Match against cRuleIdStrings from Consts.cs L1121-1192
        match value {
            "Clearance" => Ok(RuleKind::Clearance),
            "ParallelSegment" => Ok(RuleKind::ParallelSegment),
            "Width" => Ok(RuleKind::Width),
            "Length" => Ok(RuleKind::Length),
            "MatchedLengths" => Ok(RuleKind::MatchedLengths),
            "StubLength" => Ok(RuleKind::DaisyChainStubLength),
            // ... all 70 mappings from cRuleIdStrings
            _ => Err(AltiumFormatError::InvalidParamValue {
                key: key.to_owned(),
                detail: format!("unknown RULEKIND string: {value:?}"),
            }),
        }
    }
}

impl ToParamValue for RuleKind {
    fn to_param_value(&self) -> String {
        match self {
            RuleKind::Clearance => "Clearance",
            RuleKind::ParallelSegment => "ParallelSegment",
            RuleKind::Width => "Width",
            // ... all 70 mappings
        }.to_owned()
    }
}
```

**NOTE**: The string mappings are NON-OBVIOUS. Examples:
- `RuleKind::Width` → `"Width"` (not `"MaxMinWidth"`)
- `RuleKind::BrokenNets` → `"UnRoutedNet"` (not `"BrokenNets"`)
- `RuleKind::ConfinementConstraint` → `"RoomDefinition"`
- `RuleKind::ComponentRotations` → `"ComponentOrientations"`
- `RuleKind::UnpouredPolygon` → `"UnpouredPolygon"` (C# name: `eRule_ModifiedPolygon`)

Full mapping table is in `drc-types.md` → [TRuleKind Enum](#trulekind-enum).

### New Enums Needed in `altium-format-types`

These enums exist in the C# source but not yet in `altium-format-types`:

| Enum | Variants | String Serialization | C# Source |
|------|----------|---------------------|-----------|
| `NetScope` | 5 | `"DifferentNets"`, `"SameNetOnly"`, `"AnyNet"`, `"DifferentPairs"`, `"SameDiffPairs"` | `TNetScope.cs` |
| `RuleLayerKind` | 2 | `"SameLayer"`, `"AdjacentLayers"` | `TRuleLayerKind.cs` |
| `ScopeKind` | 41 | `"Board"`, `"Net"`, `"Advanced"`, etc. | `TScopeKind.cs` |
| `NetTopology` | 7 | `"Shortest"`, `"Horizontal"`, `"Daisy_Simple"`, etc. | `TNetTopology.cs` |
| `RouteVia` | 4 | `"Through Hole"`, etc. | `TRouteVia.cs` |
| `PolygonReliefAngle` | 4 | `"45 Angle"`, `"90 Angle"`, `"0 Angle"`, `"135 Angle"` | `TPolygonReliefAngle.cs` |
| `ConfinementStyle` | 2 | `"ConfineIn"`, `"ConfineOut"` | `TConfinementStyle.cs` |
| `ClearanceConstraintMode` | 2 | `"SingleClearance"`, `"ObjectsClearance"` | `TClearanceConstraintMode.cs` |
| `ObjectClearanceId` | 15 | `"ClearanceObj_Arc"`, etc. | `TObjectClearanceId.cs` |
| `ComponentCollisionCheckMode` | 4 | `"Quick Check Mode"`, etc. | `TComponentCollisionCheckMode.cs` |
| `LengthenerStyle` | 4 | integer-keyed | `TLengthenerStyle.cs` |
| `FanoutStyle` | 5 | `"Auto"`, `"Rows"`, etc. | `TFanoutStyle.cs` |
| `FanoutDirection` | 6 | `"Alternating"`, `"OutThenIn"`, etc. | `TFanoutDirection.cs` |
| `TestpointValid` | 4 | integer-keyed | `TTestpointValid.cs` |
| `StimulusType` | 3 | integer-keyed | `TStimulusType.cs` |
| `SignalLevel` | 2 | integer-keyed | `TSignalLevel.cs` |
| `RuleCategory` | 10 | `"Electrical"`, `"Routing"`, etc. | `TRuleCategory.cs` |
| `BGAFanoutDirection` | ? | string-keyed (`"Out"`, ...) | `TBGAFanoutDirection.cs` |
| `BGAFanoutViaMode` | ? | string-keyed (`"Centered"`, ...) | `TBGAFanoutViaMode.cs` |

**Estimated work**: ~18 new enums, each 2-7 variants, each needing:
1. Enum definition in `altium-format-types/src/pcb.rs` with `#[repr(u8)]`, `TryFrom<u8>`
2. Custom `FromParamValue`/`ToParamValue` in `altium-format/src/param_value.rs`
3. Re-export from `altium-format-types/src/lib.rs`

### Alternative: String-Keyed Enum Macro

Given the volume, a helper macro would be useful:

```rust
macro_rules! impl_string_enum_param_value {
    ($t:ty, $($variant:ident => $s:literal),+ $(,)?) => {
        impl FromParamValue for $t {
            fn from_param_value(key: &str, value: &str) -> Result<Self> {
                match value {
                    $($s => Ok(<$t>::$variant),)+
                    _ => Err(AltiumFormatError::InvalidParamValue {
                        key: key.to_owned(),
                        detail: format!("unknown {} string: {:?}",
                            stringify!($t), value),
                    }),
                }
            }
        }
        impl ToParamValue for $t {
            fn to_param_value(&self) -> String {
                match self {
                    $(<$t>::$variant => $s,)+
                }.to_owned()
            }
        }
    };
}

// Usage:
impl_string_enum_param_value!(RuleKind,
    Clearance => "Clearance",
    ParallelSegment => "ParallelSegment",
    Width => "Width",
    // ... 67 more
);

impl_string_enum_param_value!(NetScope,
    DifferentNetsOnly => "DifferentNets",
    SameNetOnly => "SameNetOnly",
    AnyNet => "AnyNet",
    DifferentDiffPairsOnly => "DifferentPairs",
    SameDiffPairOnly => "SameDiffPairs",
);
```

---

## Rule Base Struct

All rules share a common parameter set (from `IPCB_Rule` base interface). The base
struct uses `#[param(flatten)]` composition:

```rust
/// Common fields shared by ALL rule records.
/// Parsed from the |KEY=VALUE| param record in Rules6/Data.
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct PcbRuleBase {
    // -- Primitive fields (from IPCB_Primitive) --
    #[param(key = "SELECTION", default = false)]
    pub selection: bool,
    #[param(key = "LAYER", default = String::new())]
    pub layer: String,  // TODO: could be V7Layer
    #[param(key = "LOCKED", default = false)]
    pub locked: bool,
    #[param(key = "POLYGONOUTLINE", default = false)]
    pub polygon_outline: bool,
    #[param(key = "USERROUTED", default = true)]
    pub user_routed: bool,
    #[param(key = "KEEPOUT", default = false)]
    pub keepout: bool,
    #[param(key = "UNIONINDEX", default = 0u32)]
    pub union_index: u32,

    // -- Rule identity --
    #[param(key = "RULEKIND")]
    pub rule_kind: RuleKind,
    #[param(key = "NETSCOPE")]
    pub net_scope: NetScope,
    #[param(key = "LAYERKIND", default = RuleLayerKind::SameLayer)]
    pub layer_kind: RuleLayerKind,
    #[param(key = "NAME")]
    pub name: String,
    #[param(key = "COMMENT", default = String::new())]
    pub comment: String,
    #[param(key = "UNIQUEID", default = String::new())]
    pub unique_id: String,

    // -- Scope expressions --
    #[param(key = "SCOPE1EXPRESSION", default = String::new())]
    pub scope1_expression: String,
    #[param(key = "SCOPE2EXPRESSION", default = String::new())]
    pub scope2_expression: String,

    // -- Flags --
    #[param(key = "ENABLED", default = true)]
    pub enabled: bool,
    #[param(key = "PRIORITY", default = 1u16)]
    pub priority: u16,
    #[param(key = "DEFINEDBYLOGICALDOCUMENT", default = false)]
    pub defined_by_logical_document: bool,
}
```

### Rules6 Record: Prefix + Base + Kind-Specific

Each Rules6 record has a **u16 prefix** (from `PrefixedParamRecord`) followed by
the param string. The prefix appears to encode the rule kind index.

```rust
/// A fully typed rule record combining base + kind-specific data.
pub(crate) struct PcbRule {
    /// The u16 prefix from the PrefixedParamRecord framing.
    pub prefix: u16,
    /// Common rule fields (scope, name, priority, etc.)
    pub base: PcbRuleBase,
    /// Kind-specific rule data, dispatched on `base.rule_kind`.
    pub kind_data: PcbRuleKindData,
}
```

---

## Rule Kind Dispatch

After parsing `PcbRuleBase` (which consumes the common keys), the remaining params
are dispatched based on `rule_kind` to a kind-specific struct:

```rust
/// Kind-specific rule data. Each variant holds the extra parameters
/// for that rule type. The base params have already been consumed.
pub(crate) enum PcbRuleKindData {
    Clearance(ClearanceRuleData),
    ParallelSegment(ParallelSegmentRuleData),
    Width(WidthRuleData),
    Length(LengthRuleData),
    MatchedLengths(MatchedLengthsRuleData),
    // ... one variant per RuleKind (70 total)
}
```

### Parsing Flow

```rust
fn parse_rule(prefix: u16, params: &mut ParameterCollection) -> Result<PcbRule> {
    let base = PcbRuleBase::from_params(params)?;
    let kind_data = match base.rule_kind {
        RuleKind::Clearance => PcbRuleKindData::Clearance(
            ClearanceRuleData::from_params(params)?
        ),
        RuleKind::Width => PcbRuleKindData::Width(
            WidthRuleData::from_params(params)?
        ),
        // ... all 70 kinds
        _ => {
            // Fail-fast: unknown rule kind must not pass silently
            return Err(AltiumFormatError::Generic {
                detail: format!("unimplemented rule kind: {:?}", base.rule_kind),
            });
        }
    };
    params.assert_exhausted()?;
    Ok(PcbRule { prefix, base, kind_data })
}
```

---

## Concrete Rule Type Structs

Each rule kind gets a dedicated struct with `#[derive(FromParams, ToParams)]`.
Here are representative examples covering the main patterns:

### Simple Gap/Limit Rules

```rust
/// Clearance (eRule_Clearance = 0)
/// C#: IPCB_ClearanceConstraint
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct ClearanceRuleData {
    #[param(key = "GAP", default = String::from("10mil"))]
    pub gap: String,  // NOTE: Coord-as-string with unit suffix
    #[param(key = "GENERICCLEARANCE", default = String::from("10mil"))]
    pub generic_clearance: String,
    #[param(key = "IGNOREPADTOPADCLEARANCEINFOOTPRINT", default = false)]
    pub ignore_pad_to_pad: bool,
    #[param(key = "OBJECTCLEARANCES", default = String::new())]
    pub object_clearances: String,  // Matrix encoding TBD
}

/// Width (eRule_MaxMinWidth = 2)
/// C#: IPCB_MaxMinWidthConstraint
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct WidthRuleData {
    #[param(key = "MINLIMIT")]
    pub min_limit: String,  // "7mil"
    #[param(key = "MAXLIMIT")]
    pub max_limit: String,
    #[param(key = "PREFEREDWIDTH")]
    pub preferred_width: String,  // Note: Altium's "PREFERED" typo
}
```

### Enum-Valued Rules

```rust
/// Polygon Connect Style (eRule_PolygonConnectStyle = 20)
/// C#: IPCB_PolygonConnectStyleRule
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct PolygonConnectStyleRuleData {
    #[param(key = "CONNECTSTYLE")]
    pub connect_style: PlaneConnectStyle,  // "Relief" / "Direct"
    #[param(key = "RELIEFCONDUCTORWIDTH")]
    pub relief_conductor_width: String,
    #[param(key = "RELIEFENTRIES", default = 4i32)]
    pub relief_entries: i32,
    #[param(key = "POLYGONRELIEFANGLE")]
    pub polygon_relief_angle: PolygonReliefAngle,  // "90 Angle"
    #[param(key = "AIRGAPWIDTH")]
    pub air_gap_width: String,
}

/// Routing Via Style (eRule_RoutingViaStyle = 11)
/// C#: IPCB_RoutingViaStyleRule
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct RoutingViaStyleRuleData {
    #[param(key = "MINHOLEWIDTH")]
    pub min_hole_width: String,
    #[param(key = "MAXHOLEWIDTH")]
    pub max_hole_width: String,
    #[param(key = "HOLEWIDTH")]
    pub preferred_hole_width: String,
    #[param(key = "MINWIDTH")]
    pub min_width: String,
    #[param(key = "MAXWIDTH")]
    pub max_width: String,
    #[param(key = "WIDTH")]
    pub preferred_width: String,
    #[param(key = "VIASTYLE")]
    pub via_style: RouteVia,  // "Through Hole"
}
```

### Complex Rules (Indexed Data)

```rust
/// Room Definition (eRule_ConfinementConstraint = 22)
/// C#: IPCB_ConfinementConstraint
///
/// NOTE: Uses indexed vertex params (KIND0, VX0, VY0, CX0, CY0, SA0, EA0, R0, ...)
/// This is a non-standard indexed pattern that doesn't match existing
/// indexed_coords — may need a custom parser.
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct ConfinementRuleData {
    #[param(key = "CONFINEMENTSTYLE")]
    pub confinement_style: ConfinementStyle,
    #[param(key = "LOCKCOMPONENTS", default = false)]
    pub lock_components: bool,
    // Polygon vertices need custom handling — each vertex has:
    //   KINDn, VXn, VYn, CXn, CYn, SAn, EAn, Rn
    // This is poly-segment format (line/arc) not simple coord points.
    // TODO: May need a Vec<PolySegment> with custom parser
}

/// DiffPairs Routing (eRule_DifferentialPairsRouting = 51)
/// C#: IPCB_DifferentialPairsRoutingRule
///
/// NOTE: Has per-layer params (TOPLAYER_MINGAP, MIDLAYER1_MINGAP, ...,
/// BOTTOMLAYER_MINGAP) × 6 fields (MINGAP, MAXGAP, PREFGAP, MINWIDTH,
/// MAXWIDTH, PREFWIDTH) = potentially 192 layer-specific params.
/// Needs either dynamic extraction or a layer-indexed map.
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct DiffPairsRoutingRuleData {
    #[param(key = "MINLIMIT")]
    pub min_limit: String,
    #[param(key = "MAXLIMIT")]
    pub max_limit: String,
    #[param(key = "MOSTFREQGAP")]
    pub most_freq_gap: String,
    #[param(key = "MAXUNCOUPLEDLENGTH")]
    pub max_uncoupled_length: String,
    // Per-layer fields: would need ~192 optional params or a HashMap approach.
    // Possible approach: parse remaining TOPLAYER_*/MIDLAYER*_*/BOTTOMLAYER_*
    // keys into a HashMap<String, String> or a custom LayerGapMap.
}
```

### Scope-Only Rules (Empty Kind Data)

```rust
/// Nets To Ignore (eRule_NetsToIgnore = 27)
/// Unconnected Pin (eRule_UnconnectedPin = 45)
/// Layer Stack (eRule_LayerStack = 38)
///
/// These rules have NO kind-specific params beyond the base.
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct EmptyRuleData;
// (Could also be a unit variant in the enum rather than a struct)
```

### Estimated Rule Struct Count

| Category | Rule Kinds | Struct Pattern |
|----------|-----------|----------------|
| Simple gap/limit | ~15 | 1-5 params, straightforward |
| Enum-valued | ~10 | String-keyed enum + a few params |
| Boolean-only | ~5 | Single `ALLOWED` or `ENFORCE` param |
| Scope-only (empty) | ~5 | No kind-specific params |
| Per-layer indexed | ~3 | TOPLAYER/MIDLAYER/BOTTOMLAYER × N |
| Complex polygon | ~2 | Indexed poly-segment vertices |
| Signal integrity | ~10 | Mostly single float/double values |
| Testpoint | ~4 | Many grid/size params |

**Total: ~70 structs** (one per `RuleKind` variant), but most are very small (1-5 fields).

---

## Violation Base Struct

All violations share common base parameters:

```rust
/// Common fields shared by ALL violation records.
/// Parsed from |KEY=VALUE| params in T*Violation/Data storages.
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct PcbViolationBase {
    // -- Primitive fields --
    #[param(key = "SELECTION", default = false)]
    pub selection: bool,
    #[param(key = "LAYER", default = String::new())]
    pub layer: String,
    #[param(key = "LOCKED", default = false)]
    pub locked: bool,
    #[param(key = "POLYGONOUTLINE", default = false)]
    pub polygon_outline: bool,
    #[param(key = "USERROUTED", default = true)]
    pub user_routed: bool,
    #[param(key = "KEEPOUT", default = false)]
    pub keepout: bool,
    #[param(key = "UNIONINDEX", default = 0u32)]
    pub union_index: u32,

    // -- Violation-specific --
    #[param(key = "RULEINDEX")]
    pub rule_index: u32,  // Index into Rules6 section
    #[param(key = "PRIM1ID")]
    pub prim1_id: String,  // "Track", "Pad", "Via", "Component", "Net", "Board"
    #[param(key = "PRIM1INDEX")]
    pub prim1_index: u32,
    #[param(key = "DESCRIPTION", default = String::new())]
    pub description: String,
    #[param(key = "INVOLVEDPRIMCOUNT", default = 0u32)]
    pub involved_prim_count: u32,
}
```

### Binary vs Unary Violations

Some violations reference TWO primitives (clearance checks), others only ONE:

```rust
/// Extension for binary violations (clearance, short circuit, etc.)
/// These add PRIM2ID/PRIM2INDEX to the base.
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct BinaryViolationExt {
    #[param(key = "PRIM2ID")]
    pub prim2_id: String,
    #[param(key = "PRIM2INDEX")]
    pub prim2_index: u32,
}
```

---

## Concrete Violation Type Structs

Violations are simpler than rules — most just add location fields to the base.
The violation type is known from the **CFB storage name** (ParamSectionKind),
not from a discriminant in the param data.

### Location Patterns Observed in Real Files

Different violation types use different location parameter schemes:

| Pattern | Parameters | Used By |
|---------|-----------|---------|
| `LOCATION1/2` | `LOCATION1.X`, `LOCATION1.Y`, `LOCATION2.X`, `LOCATION2.Y` | TClearanceViolation, TMaxMinViaHoleSizeViolation, THoleToHoleViolation |
| `LOCATION` | `LOCATION.X`, `LOCATION.Y`, `CIRCLERADIUS` | TNetAntennaeViolation, TSMDPADEntryViolation, TRoutingViaStyleViolation |
| `FX/FY` | `FX1`, `FY1`, `FX2`, `FY2` | TDisconnectedSubnetsViolation, TMaxMinPadSlotWidthViolation |
| `VX/VY` | `VX1`..`VX4`, `VY1`..`VY4` | TShortCircuitViolation (area corners) |
| Polygon | `LAYERCOUNT`, `POLY1.CONTOUR0.VTXCOUNT`, `POLY1.CONTOUR0.VX0`... | TDiffPairsViolation |
| None | (no location) | TMatchedNetLengthsViolation, TMaxMinComponentHeightViolation |

### Representative Violation Structs

```rust
/// Clearance violation (binary: two primitives with two location points).
/// Storage: /TClearanceViolation/
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct ClearanceViolationData {
    #[param(flatten)]
    pub base: PcbViolationBase,
    #[param(flatten)]
    pub binary: BinaryViolationExt,
    // Location points (LOCATION1.X/Y, LOCATION2.X/Y)
    // NOTE: These are coord-with-unit strings ("3992.126mil"), NOT raw Coord.
    // Need to verify if PCB violations use raw coord or formatted string.
    #[param(key = "LOCATION1.X", default = String::new())]
    pub location1_x: String,
    #[param(key = "LOCATION1.Y", default = String::new())]
    pub location1_y: String,
    #[param(key = "LOCATION2.X", default = String::new())]
    pub location2_x: String,
    #[param(key = "LOCATION2.Y", default = String::new())]
    pub location2_y: String,
}

/// Board outline clearance violation (binary, with ObjectClearanceId types).
/// Storage: /TBoardOutlineClearanceViolation/
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct BoardOutlineClearanceViolationData {
    #[param(flatten)]
    pub base: PcbViolationBase,
    #[param(flatten)]
    pub binary: BinaryViolationExt,
    #[param(key = "PRIMID1")]
    pub prim_clearance_id1: ObjectClearanceId,  // "ClearanceObj_Track"
    #[param(key = "PRIMID2")]
    pub prim_clearance_id2: ObjectClearanceId,  // "ClearanceObj_OutlineEdge"
    #[param(key = "LOCATION1.X", default = String::new())]
    pub location1_x: String,
    #[param(key = "LOCATION1.Y", default = String::new())]
    pub location1_y: String,
    #[param(key = "LOCATION2.X", default = String::new())]
    pub location2_x: String,
    #[param(key = "LOCATION2.Y", default = String::new())]
    pub location2_y: String,
}

/// Disconnected subnets violation (unary, FX/FY location scheme).
/// Storage: /TDisconnectedSubnetsViolation/
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct DisconnectedSubnetsViolationData {
    #[param(flatten)]
    pub base: PcbViolationBase,
    #[param(key = "FX1", default = String::new())]
    pub fx1: String,
    #[param(key = "FY1", default = String::new())]
    pub fy1: String,
    #[param(key = "FX2", default = String::new())]
    pub fx2: String,
    #[param(key = "FY2", default = String::new())]
    pub fy2: String,
}

/// DiffPairs violation (complex polygon data).
/// Storage: /TDiffPairsViolation/
/// NOTE: This type has deeply nested indexed polygon params that will
/// require a custom parser, not the derive macro alone.
pub(crate) struct DiffPairsViolationData {
    pub base: PcbViolationBase,
    pub layer_polygons: Vec<LayerPolygon>,  // Custom structure
}
```

### Violation Type Dispatch

Unlike rules (which dispatch on `RULEKIND` inside the record), violations are
dispatched on the **storage name** (already known from `ParamSectionKind`):

```rust
fn parse_violation(
    kind: ParamSectionKind,
    params: &mut ParameterCollection,
) -> Result<PcbViolation> {
    match kind {
        ParamSectionKind::TClearanceViolation => {
            Ok(PcbViolation::Clearance(ClearanceViolationData::from_params(params)?))
        }
        ParamSectionKind::TDisconnectedSubnetsViolation => {
            Ok(PcbViolation::DisconnectedSubnets(
                DisconnectedSubnetsViolationData::from_params(params)?
            ))
        }
        // ... all 38 violation types
        _ => unreachable!("non-violation ParamSectionKind"),
    }
}
```

---

## Waived Violations

WaivedViolations uses a **different parameter set** from violations:

```rust
/// A waived violation entry from WaivedViolations/Data.
/// Completely different structure from T*Violation records.
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct WaivedViolation {
    #[param(key = "UNICODE", default = String::new())]
    pub unicode: String,  // Always "EXISTS"
    #[param(key = "RULEINDEX")]
    pub rule_index: u32,
    #[param(key = "PRIM1KIND")]
    pub prim1_kind: String,  // "DifferentialPair", "Track", "Pad"
    #[param(key = "PRIM1INDEX")]
    pub prim1_index: u32,
    #[param(key = "PRIM2KIND", optional)]
    pub prim2_kind: Option<String>,
    #[param(key = "PRIM2INDEX", optional)]
    pub prim2_index: Option<u32>,
    #[param(key = "CREATEDAT")]
    pub created_at: String,  // ISO 8601: "2020-09-04T13:11:48.000Z"
    #[param(key = "AUTHORID")]
    pub author_id: String,  // GUID
    #[param(key = "AUTHORTITLE")]
    pub author_title: String,
    #[param(key = "SOURCE")]
    pub source: String,
    #[param(key = "COMMENT", default = String::new())]
    pub comment: String,
}
```

**NOTE**: WaivedViolations has `UNICODE__<FIELDNAME>` keys for Unicode codepoint
sequences. The derive macro doesn't support this pattern — it may need special handling
in `from_params()` to strip/consume these keys, or a post-processing step.

---

## DRC Options

```rust
/// Design Rule Checker Options (DesignRuleCheckerOptions6/Data).
/// Single record.
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct DrcOptions {
    #[param(key = "RECORD", default = String::from("DesignRuleCheckerOptions"))]
    pub record: String,
    #[param(key = "DOMAKEDRCFILE", default = true)]
    pub do_make_drc_file: bool,
    #[param(key = "DOMAKEDRCERRORLIST", default = true)]
    pub do_make_drc_error_list: bool,
    #[param(key = "DOSUBNETDETAILS", default = true)]
    pub do_subnet_details: bool,
    #[param(key = "MAXVIOLATIONCOUNT", default = 500u32)]
    pub max_violation_count: u32,
    #[param(list, key = "RULESETTOCHECK")]
    pub rule_set_to_check: Vec<u32>,
    #[param(list, key = "ONLINERULESETTOCHECK")]
    pub online_rule_set_to_check: Vec<u32>,
    #[param(key = "INTERNALPLANEWARNINGS", default = true)]
    pub internal_plane_warnings: bool,
    #[param(key = "VERIFYSHORTINGCOPPER", default = true)]
    pub verify_shorting_copper: bool,
    // ... more boolean flags
}
```

---

## Storage Architecture (Option C)

**Decision: Parse at load time, store typed.** DRC data is parsed into typed structs
during `PcbDoc::load()` and stored in dedicated fields. This gives fail-fast validation
at load time, zero re-parsing overhead, and a clean typed API.

### PcbDoc Struct Changes

```rust
pub(crate) struct PcbDoc {
    // Existing fields unchanged...
    pub(crate) sections: Vec<PcbDocSection>,

    // ── New typed DRC storage ──────────────────────────────────────────

    /// Design rules from Rules6 section. Ordered by record index.
    pub(crate) rules: Vec<PcbRule>,

    /// DRC violations grouped by violation type (CFB storage name).
    /// Key = ParamSectionKind variant (e.g., TClearanceViolation).
    /// Value = all violation records from that storage.
    pub(crate) violations: IndexMap<ParamSectionKind, Vec<PcbViolation>>,

    /// Waived violations from WaivedViolations section.
    pub(crate) waived_violations: Vec<WaivedViolation>,

    /// DRC checker options (single record from DesignRuleCheckerOptions6).
    pub(crate) drc_options: Option<DrcOptions>,
}
```

**Why `IndexMap`**: Preserves insertion order (= file order) for deterministic
serialization roundtrip. Already used by `ParameterCollection` in this codebase.
O(1) lookup by key, iteration in insertion order — best of both worlds.
No `Ord` needed on `ParamSectionKind` (unlike `BTreeMap`).

### Load Path Changes

In `PcbDoc::load()`, where `ParamSectionKind::from_storage_name()` currently dispatches
all param sections uniformly:

```rust
// Before: all param sections → opaque ParamSectionData
if let Some(kind) = records::ParamSectionKind::from_storage_name(&storage_name) {
    // ... generic load into ParamSectionData
    sections.push(PcbDocSection::Parameter(ParamSectionData { kind, records }));
}

// After: dispatch to typed parsing for DRC sections
if let Some(kind) = records::ParamSectionKind::from_storage_name(&storage_name) {
    match kind {
        ParamSectionKind::Rules6 => {
            let records = records::parse_prefixed_param_records(&data)?;
            for (i, rec) in records.into_iter().enumerate() {
                let rule = parse_rule(rec.prefix, rec.params)
                    .with_context(|| format!("rule #{i} in Rules6"))?;
                doc.rules.push(rule);
            }
        }
        kind if kind.is_violation() => {
            let records = records::parse_standard_param_records(&data)?;
            let mut violations = Vec::with_capacity(records.len());
            for (i, rec) in records.into_iter().enumerate() {
                let v = parse_violation(kind, rec.params)
                    .with_context(|| format!("violation #{i} in {kind:?}"))?;
                violations.push(v);
            }
            doc.violations.insert(kind, violations);
        }
        ParamSectionKind::WaivedViolations => {
            let records = records::parse_standard_param_records(&data)?;
            for (i, rec) in records.into_iter().enumerate() {
                let w = WaivedViolation::from_params(rec.params)
                    .with_context(|| format!("waived violation #{i}"))?;
                doc.waived_violations.push(w);
            }
        }
        ParamSectionKind::DesignRuleCheckerOptions6 => {
            let records = records::parse_standard_param_records(&data)?;
            if let Some(rec) = records.into_iter().next() {
                doc.drc_options = Some(DrcOptions::from_params(rec.params)?);
            }
        }
        _ => {
            // Non-DRC param sections: keep opaque for now
            sections.push(PcbDocSection::Parameter(ParamSectionData { kind, records }));
        }
    }
}
```

### Helper Method on ParamSectionKind

```rust
impl ParamSectionKind {
    /// Returns true if this section kind is a DRC violation storage.
    pub(crate) fn is_violation(&self) -> bool {
        matches!(self,
            Self::TAcuteAngleViolation
            | Self::TBackDrillViolation
            | Self::TBoardOutlineClearanceViolation
            // ... all 38 violation variants
            | Self::TZAxisClearanceViolation
        )
    }
}
```

### Save Path

For serialization, reverse the process: iterate `doc.rules`, `doc.violations`, etc.
and write back to their respective CFB storages using `to_params()` + block encoding.

---

## Complex Data Structures

### Clearance Matrix

#### Format (Empirically Verified)

The `OBJECTCLEARANCES` param value is a **semicolon-delimited sparse matrix** encoding
per-object-type clearance overrides:

```
ClearanceObj_Arc-ClearanceObj_Track:137795;ClearanceObj_Arc-ClearanceObj_SMDPad:35000;...
```

Each entry: `{ObjType1}-{ObjType2}:{clearance_internal_units}`

Properties:
- **Symmetric/upper-triangular**: only `Type1 ≤ Type2` (by enum ordinal) stored
- **Sparse**: only overridden pairs present (default = `GENERICCLEARANCE`)
- **Internal coord units**: integer, 10,000 = 1 mil
- **Empty string**: means single-clearance mode (no matrix overrides)

#### Rust Data Structure

```rust
/// Sparse symmetric clearance matrix indexed by object-type pairs.
///
/// Wraps an IndexMap for insertion-order-preserving serialization. Only
/// stores pairs that differ from the generic clearance value. The key
/// pair is always normalized so that the lower ObjectClearanceId comes first.
#[derive(Debug, Clone, Default)]
pub(crate) struct ClearanceMatrix {
    /// Clearance overrides keyed by (type1, type2) where type1 <= type2.
    entries: IndexMap<(ObjectClearanceId, ObjectClearanceId), Coord>,
}

impl ClearanceMatrix {
    /// Get clearance between two object types (order-independent).
    pub fn get(&self, a: ObjectClearanceId, b: ObjectClearanceId) -> Option<Coord> {
        let key = Self::normalize(a, b);
        self.entries.get(&key).copied()
    }

    /// Set clearance between two object types (order-independent).
    pub fn set(&mut self, a: ObjectClearanceId, b: ObjectClearanceId, value: Coord) {
        let key = Self::normalize(a, b);
        self.entries.insert(key, value);
    }

    /// Number of overridden pairs.
    pub fn len(&self) -> usize { self.entries.len() }
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }

    /// Iterate over all (type1, type2, clearance) triples.
    pub fn iter(&self) -> impl Iterator<Item = (ObjectClearanceId, ObjectClearanceId, Coord)> + '_ {
        self.entries.iter().map(|(&(a, b), &v)| (a, b, v))
    }

    fn normalize(a: ObjectClearanceId, b: ObjectClearanceId) -> (ObjectClearanceId, ObjectClearanceId) {
        if (a as u8) <= (b as u8) { (a, b) } else { (b, a) }
    }
}
```

#### Custom FromParamValue / ToParamValue

```rust
impl FromParamValue for ClearanceMatrix {
    fn from_param_value(key: &str, value: &str) -> Result<Self> {
        let mut matrix = ClearanceMatrix::default();
        if value.is_empty() {
            return Ok(matrix);
        }
        for entry in value.split(';') {
            let (pair_str, val_str) = entry.split_once(':')
                .ok_or_else(|| AltiumFormatError::InvalidParamValue {
                    key: key.to_owned(),
                    detail: format!("expected 'Type1-Type2:value', got {entry:?}"),
                })?;
            let (type1_str, type2_str) = pair_str.split_once('-')
                .ok_or_else(|| AltiumFormatError::InvalidParamValue {
                    key: key.to_owned(),
                    detail: format!("expected 'Type1-Type2', got {pair_str:?}"),
                })?;
            let type1 = ObjectClearanceId::from_clearance_string(type1_str)?;
            let type2 = ObjectClearanceId::from_clearance_string(type2_str)?;
            let value = val_str.parse::<i32>().map_err(|_| {
                AltiumFormatError::InvalidParamValue {
                    key: key.to_owned(),
                    detail: format!("invalid clearance value: {val_str:?}"),
                }
            })?;
            matrix.set(type1, type2, Coord::from_internal(value));
        }
        Ok(matrix)
    }
}

impl ToParamValue for ClearanceMatrix {
    fn to_param_value(&self) -> String {
        self.entries.iter()
            .map(|(&(a, b), &v)| format!(
                "{}-{}:{}",
                a.to_clearance_string(),
                b.to_clearance_string(),
                v.to_internal()
            ))
            .collect::<Vec<_>>()
            .join(";")
    }
}
```

#### ObjectClearanceId Helper Methods

```rust
impl ObjectClearanceId {
    /// Parse from the "ClearanceObj_Arc" format used in OBJECTCLEARANCES.
    pub fn from_clearance_string(s: &str) -> Result<Self> {
        match s {
            "ClearanceObj_Arc" => Ok(Self::Arc),
            "ClearanceObj_Track" => Ok(Self::Track),
            "ClearanceObj_SMDPad" => Ok(Self::SmdPad),
            "ClearanceObj_THPad" => Ok(Self::ThPad),
            "ClearanceObj_Via" => Ok(Self::Via),
            "ClearanceObj_Fill" => Ok(Self::Fill),
            "ClearanceObj_Poly" => Ok(Self::Poly),
            "ClearanceObj_Region" => Ok(Self::Region),
            "ClearanceObj_Text" => Ok(Self::Text),
            "ClearanceObj_Hole" => Ok(Self::Hole),
            "ClearanceObj_OutlineEdge" => Ok(Self::OutlineEdge),
            "ClearanceObj_CavityEdge" => Ok(Self::CavityEdge),
            "ClearanceObj_CutoutEdge" => Ok(Self::CutoutEdge),
            "ClearanceObj_SplitBarrior" => Ok(Self::SplitBarrier),  // Altium typo preserved
            "ClearanceObj_SplitContinuation" => Ok(Self::SplitContinuation),
            _ => Err(/* ... */),
        }
    }

    /// Serialize to "ClearanceObj_Arc" format.
    pub fn to_clearance_string(&self) -> &'static str {
        match self {
            Self::Arc => "ClearanceObj_Arc",
            Self::Track => "ClearanceObj_Track",
            // ... all 15
            Self::SplitBarrier => "ClearanceObj_SplitBarrior",  // preserve Altium typo
            Self::SplitContinuation => "ClearanceObj_SplitContinuation",
        }
    }
}
```

#### Usage in ClearanceRuleData

```rust
#[derive(Debug)]
pub(crate) struct ClearanceRuleData {
    pub gap: Coord,
    pub generic_clearance: Coord,
    pub ignore_pad_to_pad: bool,
    /// Per-object-type clearance overrides. Empty = single-clearance mode.
    pub object_clearances: ClearanceMatrix,
}
```

---

### Per-Layer Rule Params

#### The Problem

DiffPairsRouting, Width, and RoutingLayers rules use per-layer parameters with two
different naming conventions:

| Rule Type | Layer Prefix Pattern | Example |
|-----------|---------------------|---------|
| DiffPairsRouting | `TOPLAYER`, `MIDLAYER{1-30}`, `BOTTOMLAYER` | `TOPLAYER_MINWIDTH=15mil` |
| Width (per-substack) | Same + optional `_{GUID}` suffix | `TOPLAYER_MINLIMIT=10mil` |
| RoutingLayers | `TOP LAYER`, `MID LAYER {1-30}`, `BOTTOM LAYER` (spaces!) | `TOP LAYER_V5=TRUE` |

That's 32 signal layers × N suffixes per rule = up to 192+ optional params per rule.

#### Solution: `SignalLayerMap<T>`

A generic wrapper that maps signal layers (V6Layer 1-32) to optional values,
with custom parsing that knows the key prefix conventions:

```rust
/// A map from signal layers (Top, Mid1-30, Bottom) to optional values.
///
/// Backed by a fixed-size array of 32 `Option<T>` slots. Indexed by
/// V6Layer signal layer number (1=Top, 2-31=Mid1-30, 32=Bottom).
///
/// Provides efficient O(1) access by layer and deterministic iteration
/// for serialization.
#[derive(Debug, Clone)]
pub(crate) struct SignalLayerMap<T> {
    /// Index 0 = TopLayer (V6Layer 1), Index 31 = BottomLayer (V6Layer 32).
    layers: [Option<T>; 32],
}

impl<T> SignalLayerMap<T> {
    pub fn new() -> Self {
        Self { layers: std::array::from_fn(|_| None) }
    }

    pub fn get(&self, layer: V6Layer) -> Option<&T> {
        let idx = Self::layer_to_index(layer)?;
        self.layers[idx].as_ref()
    }

    pub fn set(&mut self, layer: V6Layer, value: T) {
        if let Some(idx) = Self::layer_to_index(layer) {
            self.layers[idx] = Some(value);
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (V6Layer, &T)> {
        self.layers.iter().enumerate().filter_map(|(i, v)| {
            v.as_ref().map(|val| (Self::index_to_layer(i), val))
        })
    }

    fn layer_to_index(layer: V6Layer) -> Option<usize> {
        let raw = layer as u8;
        if (1..=32).contains(&raw) { Some((raw - 1) as usize) } else { None }
    }

    fn index_to_layer(i: usize) -> V6Layer {
        V6Layer::try_from((i + 1) as u8).unwrap()
    }
}

impl<T: Default> Default for SignalLayerMap<T> {
    fn default() -> Self { Self::new() }
}
```

#### Key Prefix Table (Compile-Time Constant)

```rust
/// Maps signal layer index (0-31) to the parameter key prefix used by
/// DiffPairsRouting and Width rules (no spaces, e.g. "TOPLAYER").
const SIGNAL_LAYER_PREFIXES: [&str; 32] = [
    "TOPLAYER",
    "MIDLAYER1", "MIDLAYER2", "MIDLAYER3", "MIDLAYER4", "MIDLAYER5",
    "MIDLAYER6", "MIDLAYER7", "MIDLAYER8", "MIDLAYER9", "MIDLAYER10",
    "MIDLAYER11", "MIDLAYER12", "MIDLAYER13", "MIDLAYER14", "MIDLAYER15",
    "MIDLAYER16", "MIDLAYER17", "MIDLAYER18", "MIDLAYER19", "MIDLAYER20",
    "MIDLAYER21", "MIDLAYER22", "MIDLAYER23", "MIDLAYER24", "MIDLAYER25",
    "MIDLAYER26", "MIDLAYER27", "MIDLAYER28", "MIDLAYER29", "MIDLAYER30",
    "BOTTOMLAYER",
];

/// Maps signal layer index (0-31) to the parameter key prefix used by
/// RoutingLayers rules (with spaces, e.g. "TOP LAYER").
const ROUTING_LAYER_PREFIXES: [&str; 32] = [
    "TOP LAYER",
    "MID LAYER 1", "MID LAYER 2", /* ... */ "MID LAYER 30",
    "BOTTOM LAYER",
];
```

#### Custom Parsing for Per-Layer Data

```rust
/// Per-layer constraint data for DiffPairsRouting.
#[derive(Debug, Clone)]
pub(crate) struct DiffPairsLayerParams {
    pub min_gap: Coord,
    pub max_gap: Coord,
    pub pref_gap: Coord,
    pub min_width: Coord,
    pub max_width: Coord,
    pub pref_width: Coord,
}

impl DiffPairsRoutingRuleData {
    pub(crate) fn from_params(params: &mut ParameterCollection) -> Result<Self> {
        // Parse flat (non-per-layer) params first
        let min_limit = params.remove_required::<Coord>("MINLIMIT")?;
        let max_limit = params.remove_required::<Coord>("MAXLIMIT")?;
        let most_freq_gap = params.remove_required::<Coord>("MOSTFREQGAP")?;
        let max_uncoupled_length = params.remove_with_default::<Coord>(
            "MAXUNCOUPLEDLENGTH", Coord::ZERO)?;

        // Parse per-layer params using prefix table
        let mut layer_params = SignalLayerMap::new();
        for (i, prefix) in SIGNAL_LAYER_PREFIXES.iter().enumerate() {
            let layer = V6Layer::try_from((i + 1) as u8).unwrap();
            // Try to extract all 6 per-layer params; skip layer if none present
            let min_width = params.remove_optional::<Coord>(
                &format!("{prefix}_MINWIDTH"))?;
            let max_width = params.remove_optional::<Coord>(
                &format!("{prefix}_MAXWIDTH"))?;
            let pref_width = params.remove_optional::<Coord>(
                &format!("{prefix}_PREFWIDTH"))?;
            let min_gap = params.remove_optional::<Coord>(
                &format!("{prefix}_MINGAP"))?;
            let max_gap = params.remove_optional::<Coord>(
                &format!("{prefix}_MAXGAP"))?;
            let pref_gap = params.remove_optional::<Coord>(
                &format!("{prefix}_PREFGAP"))?;
            // Only store if at least one param present for this layer
            if min_width.is_some() || max_width.is_some() || pref_width.is_some() {
                layer_params.set(layer, DiffPairsLayerParams {
                    min_gap: min_gap.unwrap_or(min_limit),
                    max_gap: max_gap.unwrap_or(max_limit),
                    pref_gap: pref_gap.unwrap_or(most_freq_gap),
                    min_width: min_width.unwrap_or(Coord::ZERO),
                    max_width: max_width.unwrap_or(Coord::ZERO),
                    pref_width: pref_width.unwrap_or(Coord::ZERO),
                });
            }
        }

        Ok(Self {
            min_limit, max_limit, most_freq_gap, max_uncoupled_length,
            layer_params,
        })
    }
}
```

#### RoutingLayers: Boolean Per-Layer Map

```rust
/// RoutingLayers rule (eRule_RoutingLayers = 9).
/// Uses the space-separated layer prefix convention with `_V5` suffix.
#[derive(Debug)]
pub(crate) struct RoutingLayersRuleData {
    /// Which signal layers are enabled for routing.
    pub layers: SignalLayerMap<bool>,
}

impl RoutingLayersRuleData {
    pub(crate) fn from_params(params: &mut ParameterCollection) -> Result<Self> {
        let mut layers = SignalLayerMap::new();
        for (i, prefix) in ROUTING_LAYER_PREFIXES.iter().enumerate() {
            let layer = V6Layer::try_from((i + 1) as u8).unwrap();
            let key = format!("{prefix}_V5");
            if let Some(enabled) = params.remove_optional::<bool>(&key)? {
                layers.set(layer, enabled);
            }
        }
        Ok(Self { layers })
    }
}
```

---

### Confinement Polygon Vertices

#### The Problem

Room definitions (ConfinementConstraint) serialize polygon vertices as indexed
params with 8 fields per vertex:

```
KIND0=0|VX0=1234mil|VY0=5678mil|CX0=0mil|CY0=0mil|SA0=0.00000000000000E+0000|EA0=0.00000000000000E+0000|R0=0mil
KIND1=1|VX1=2000mil|VY1=3000mil|CX1=1500mil|CY1=2500mil|SA1=0.00000000000000E+0000|EA1=3.14159265358979E+0000|R1=500mil
```

This is the **parameter-string form** of the same `TPolySegment` struct already parsed
from binary data in regions/component bodies.

#### Solution: Reuse Existing `PolySegment`

The `PolySegment` type already exists in `crates/altium-format/src/pcblib/primitives/`:

```rust
pub(crate) struct PolySegment {
    pub kind: PolySegmentKind,   // Line=0, Arc=1
    pub vertex: CoordPoint,       // VX, VY
    pub center: CoordPoint,       // CX, CY
    pub radius: Coord,            // R
    pub angle1: f64,              // SA (start angle)
    pub angle2: f64,              // EA (end angle)
}
```

Add a **param-string parser** alongside the existing binary parser:

```rust
impl PolySegment {
    /// Parse poly-segments from indexed parameter strings.
    /// Reads KIND{i}, VX{i}, VY{i}, CX{i}, CY{i}, SA{i}, EA{i}, R{i}
    /// for i in 0..count.
    pub(crate) fn parse_indexed_params(
        params: &mut ParameterCollection,
        count: usize,
    ) -> Result<Vec<PolySegment>> {
        let mut segments = Vec::with_capacity(count);
        for i in 0..count {
            let kind_raw = params.remove_required::<u8>(&format!("KIND{i}"))?;
            let kind = PolySegmentKind::try_from(kind_raw)?;
            let vx = params.remove_required::<Coord>(&format!("VX{i}"))?;
            let vy = params.remove_required::<Coord>(&format!("VY{i}"))?;
            let cx = params.remove_with_default::<Coord>(&format!("CX{i}"), Coord::ZERO)?;
            let cy = params.remove_with_default::<Coord>(&format!("CY{i}"), Coord::ZERO)?;
            let sa = params.remove_with_default::<f64>(&format!("SA{i}"), 0.0)?;
            let ea = params.remove_with_default::<f64>(&format!("EA{i}"), 0.0)?;
            let r = params.remove_with_default::<Coord>(&format!("R{i}"), Coord::ZERO)?;
            segments.push(PolySegment { kind, vertex: CoordPoint::new(vx, vy),
                center: CoordPoint::new(cx, cy), radius: r, angle1: sa, angle2: ea });
        }
        Ok(segments)
    }

    /// Serialize poly-segments back to indexed parameter strings.
    pub(crate) fn write_indexed_params(
        segments: &[PolySegment],
        params: &mut ParameterCollection,
    ) {
        for (i, seg) in segments.iter().enumerate() {
            params.insert(&format!("KIND{i}"), (seg.kind as u8).to_string());
            params.insert(&format!("VX{i}"), seg.vertex.x.to_param_value());
            params.insert(&format!("VY{i}"), seg.vertex.y.to_param_value());
            params.insert(&format!("CX{i}"), seg.center.x.to_param_value());
            params.insert(&format!("CY{i}"), seg.center.y.to_param_value());
            params.insert(&format!("SA{i}"), seg.angle1.to_param_value());
            params.insert(&format!("EA{i}"), seg.angle2.to_param_value());
            params.insert(&format!("R{i}"), seg.radius.to_param_value());
        }
    }
}
```

#### Usage in ConfinementRuleData

```rust
#[derive(Debug)]
pub(crate) struct ConfinementRuleData {
    pub confinement_style: ConfinementStyle,
    pub lock_components: bool,
    pub constraint_layer: V6Layer,
    /// Room boundary as a list of poly-segments (lines and arcs).
    pub boundary: Vec<PolySegment>,
}

impl ConfinementRuleData {
    pub(crate) fn from_params(params: &mut ParameterCollection) -> Result<Self> {
        let confinement_style = params.remove_required::<ConfinementStyle>(
            "CONFINEMENTSTYLE")?;
        let lock_components = params.remove_with_default::<bool>(
            "LOCKCOMPONENTS", false)?;
        let constraint_layer = params.remove_with_default::<V6Layer>(
            "CONSTRAINTLAYER", V6Layer::TopLayer)?;
        let point_count = params.remove_with_default::<usize>("POINTCOUNT", 0)?;
        let boundary = PolySegment::parse_indexed_params(params, point_count)?;
        Ok(Self { confinement_style, lock_components, constraint_layer, boundary })
    }
}
```

---

### DiffPairs Violation Polygons

The `TDiffPairsViolation` storage has deeply nested polygon data:

```
LAYERCOUNT=3
LAYER1=MID7
POLY1.CONTOURCOUNT=6
POLY1.CONTOUR0.VTXCOUNT=67
POLY1.CONTOUR0.VX0=...
POLY1.CONTOUR0.VY0=...
...
```

#### Solution: Custom Nested Parser

```rust
/// A polygon contour on a specific layer, from DiffPairs violations.
#[derive(Debug, Clone)]
pub(crate) struct ViolationLayerPolygon {
    pub layer: String,
    pub contours: Vec<Vec<CoordPoint>>,  // outer + holes
}

impl ViolationLayerPolygon {
    /// Parse POLY{poly_idx}.CONTOUR{c}.VX{v}/VY{v} nested indexed params.
    pub(crate) fn parse_from_params(
        params: &mut ParameterCollection,
        poly_idx: usize,
    ) -> Result<Self> {
        let layer = params.remove_required::<String>(
            &format!("LAYER{poly_idx}"))?;
        let contour_count = params.remove_required::<usize>(
            &format!("POLY{poly_idx}.CONTOURCOUNT"))?;
        let mut contours = Vec::with_capacity(contour_count);
        for c in 0..contour_count {
            let prefix = format!("POLY{poly_idx}.CONTOUR{c}");
            let vtx_count = params.remove_required::<usize>(
                &format!("{prefix}.VTXCOUNT"))?;
            let mut points = Vec::with_capacity(vtx_count);
            for v in 0..vtx_count {
                let x = params.remove_required::<Coord>(
                    &format!("{prefix}.VX{v}"))?;
                let y = params.remove_required::<Coord>(
                    &format!("{prefix}.VY{v}"))?;
                points.push(CoordPoint::new(x, y));
            }
            contours.push(points);
        }
        Ok(Self { layer, contours })
    }
}
```

---

### Coord-With-Unit Strings

#### The Problem

DRC parameters serialize coordinates as **strings with unit suffixes**:
`"7mil"`, `"3992.126mil"`, `"0.0000mil"`. This differs from schematic params
which use raw integer strings (`"70000"`).

#### Investigation Needed

Check whether the existing `Coord::from_param_value()` handles the `"7mil"` format.
If not, two options:

**Option A: Extend `Coord::from_param_value()`** to strip `mil` suffix and convert:
```rust
fn from_param_value(key: &str, value: &str) -> Result<Self> {
    // Try raw integer first (schematic format)
    if let Ok(raw) = value.parse::<i32>() {
        return Ok(Coord::from_internal(raw));
    }
    // Try "Nmil" format (PCB DRC format)
    if let Some(mil_str) = value.strip_suffix("mil") {
        let mils: f64 = mil_str.trim().parse().map_err(|_| ...)?;
        return Ok(Coord::from_mils(mils));
    }
    Err(...)
}
```

**Option B: Use a `MilCoord` newtype** that always parses/serializes with `mil` suffix:
```rust
#[derive(Debug, Clone, Copy)]
pub(crate) struct MilCoord(pub Coord);

impl FromParamValue for MilCoord {
    fn from_param_value(key: &str, value: &str) -> Result<Self> {
        let mil_str = value.strip_suffix("mil")
            .ok_or_else(|| AltiumFormatError::InvalidParamValue {
                key: key.to_owned(),
                detail: format!("expected value with 'mil' suffix, got {value:?}"),
            })?;
        let mils: f64 = mil_str.trim().parse().map_err(|_| ...)?;
        Ok(MilCoord(Coord::from_mils(mils)))
    }
}

impl ToParamValue for MilCoord {
    fn to_param_value(&self) -> String {
        format!("{}mil", self.0.to_mils())
    }
}
```

**Recommendation**: Option A (extend `Coord`) is simpler since PCB rules are the
primary consumer. The `mil` suffix is the only unit observed in rule params.
If other units appear (mm, inch), extend to a `match` on suffix.

---

### WaivedViolations UNICODE Keys

#### The Problem

WaivedViolation records contain `UNICODE__<FIELDNAME>` companion keys that encode
Unicode codepoint sequences for fields with non-Latin characters:

```
AUTHORTITLE=Pawe³ K
UNICODE__AUTHORTITLE=80,97,119,101,322,32,75
```

The `UNICODE__` version contains comma-separated decimal Unicode codepoints and is
authoritative when present (the base field uses Windows-1252 lossy encoding).

#### Solution: Post-Parse Fixup

```rust
impl WaivedViolation {
    pub(crate) fn from_params(mut params: ParameterCollection) -> Result<Self> {
        // First, apply UNICODE__ fixups: for any key starting with "UNICODE__",
        // decode the codepoint sequence and replace the base key's value.
        let unicode_keys: Vec<String> = params.keys()
            .filter(|k| k.starts_with("UNICODE__") && *k != "UNICODE")
            .cloned()
            .collect();
        for ukey in unicode_keys {
            let base_key = &ukey["UNICODE__".len()..];
            if let Some(codepoints_str) = params.remove_optional::<String>(&ukey)? {
                let decoded: String = codepoints_str.split(',')
                    .map(|s| {
                        let cp: u32 = s.trim().parse().map_err(|_| {
                            AltiumFormatError::InvalidParamValue {
                                key: ukey.clone(),
                                detail: format!("invalid codepoint: {s:?}"),
                            }
                        })?;
                        char::from_u32(cp).ok_or_else(|| {
                            AltiumFormatError::InvalidParamValue {
                                key: ukey.clone(),
                                detail: format!("invalid Unicode codepoint: {cp}"),
                            }
                        })
                    })
                    .collect::<Result<String>>()?;
                // Replace the base key's value with the decoded Unicode string.
                // The base key's Windows-1252 value is now superseded.
                params.replace(base_key, decoded);
            }
        }

        // Now parse normally with the fixup-applied params
        let result = Self::from_params_inner(&mut params)?;

        // Consume the leading UNICODE=EXISTS marker
        params.remove_optional::<String>("UNICODE")?;

        params.assert_exhausted()?;
        Ok(result)
    }
}
```

For serialization (`to_params()`), regenerate `UNICODE__` keys for any field
containing non-ASCII characters:

```rust
fn needs_unicode_companion(s: &str) -> bool {
    s.chars().any(|c| !c.is_ascii())
}

fn to_unicode_companion(s: &str) -> String {
    s.chars()
        .map(|c| (c as u32).to_string())
        .collect::<Vec<_>>()
        .join(",")
}
```

---

## Implementation Order

### Phase 1: Foundation (~800 LOC)
1. Add ~18 new enums to `altium-format-types/src/pcb.rs` (`ObjectClearanceId`,
   `NetScope`, `RuleLayerKind`, `ScopeKind`, `NetTopology`, `RouteVia`,
   `PolygonReliefAngle`, `ConfinementStyle`, `ClearanceConstraintMode`,
   `ComponentCollisionCheckMode`, `FanoutStyle`, `FanoutDirection`,
   `BGAFanoutDirection`, `BGAFanoutViaMode`, `TestpointValid`, `StimulusType`,
   `SignalLevel`, `RuleCategory`)
2. Add `impl_string_enum_param_value!` macro to `param_value.rs`
3. Implement string-based `FromParamValue`/`ToParamValue` for all DRC enums
   including `RuleKind` (70 string mappings from `cRuleIdStrings`)
4. Resolve the `Coord` mil-suffix question (extend `FromParamValue` for `Coord`)
5. Add `Hash` to `ParamSectionKind` for use as `IndexMap` key

### Phase 2: Data Structures (~600 LOC)
6. Implement `ClearanceMatrix` with `FromParamValue`/`ToParamValue`
7. Implement `SignalLayerMap<T>` with prefix table constants
8. Add `PolySegment::parse_indexed_params()` / `write_indexed_params()`
9. Add `ViolationLayerPolygon` for DiffPairs violation polygons
10. Unit tests for all custom parsers (matrix roundtrip, layer map, poly-segments)

### Phase 3: Rule Types (~2200 LOC)
11. Define `PcbRuleBase` struct with `FromParams`/`ToParams`
12. Define `PcbRuleKindData` enum (70 variants)
13. Define concrete rule data structs:
    - Simple: ~40 structs with derive macros (1-5 fields each)
    - Custom: ~5 structs with manual `from_params()` (DiffPairs, Width, Confinement,
      RoutingLayers, Clearance matrix)
    - Empty: ~5 scope-only rules (unit variants)
14. Wire up `parse_rule()` dispatch function
15. Test against real Rules6 data from fixture files

### Phase 4: Violation Types (~1500 LOC)
16. Define `PcbViolationBase` struct
17. Define `PcbViolation` enum (38 variants)
18. Define concrete violation data structs:
    - Simple: ~30 structs with derive macros (base + 2-4 location fields)
    - Custom: DiffPairsViolation (nested polygon parser)
19. Wire up violation dispatch by `ParamSectionKind`
20. Test against real violation data from fixture files

### Phase 5: Supporting Types (~300 LOC)
21. Define `WaivedViolation` struct with UNICODE fixup
22. Define `DrcOptions` struct
23. Add `ParamSectionKind::is_violation()` helper

### Phase 6: Integration (~400 LOC)
24. Add typed DRC fields to `PcbDoc` struct
25. Update `PcbDoc::load()` with typed parsing dispatch
26. Update serialization path for save/roundtrip
27. Add proptest coverage for rule/violation roundtrip
28. Integration test: load all fixture files, verify DRC data parses

---

## Resolved Questions

### 1. Coordinate Value Format → Extend `Coord::from_param_value()`

DRC params use `"7mil"` format. Extend `FromParamValue for Coord` to handle the
`mil` suffix by stripping it and converting from mils to internal units
(`value * 10_000`). The raw integer format used by schematic params is tried first
for backward compatibility.

### 2. PrefixedParamRecord Prefix = Rule Kind Index

Verified: the u16 prefix matches the `RuleKind` discriminant. Example: `0x16 = 22`
for `RuleKind::ConfinementConstraint = 22`. Store as `prefix: u16` on `PcbRule`
but assert `prefix == base.rule_kind as u16` during parsing for validation.

### 3. Per-Layer Params → `SignalLayerMap<T>` with Custom Parser

Solved with a fixed-size `[Option<T>; 32]` array indexed by signal layer, plus a
compile-time prefix table. Custom `from_params()` iterates the 32 prefixes and
extracts optional params for each layer. See [Per-Layer Rule Params](#per-layer-rule-params).

### 4. Confinement Polygon → Reuse `PolySegment` with Param Parser

The `PolySegment` struct already exists for binary region data. Added
`parse_indexed_params()` / `write_indexed_params()` methods for the parameter-string
form (`KIND{i}`, `VX{i}`, etc.). See [Confinement Polygon Vertices](#confinement-polygon-vertices).

### 5. OBJECTCLEARANCES → `ClearanceMatrix` Sparse Map

Format is semicolon-delimited `ClearanceObj_Type1-ClearanceObj_Type2:value` pairs.
Solved with `ClearanceMatrix` wrapping `IndexMap<(ObjectClearanceId, ObjectClearanceId), Coord>`.
Normalized key ordering (type1 ≤ type2) ensures symmetric access.
See [Clearance Matrix](#clearance-matrix).

### 6. Validation Scope → Mandatory at Load Time

Per project fail-fast philosophy: ALL rule/violation types must be fully parsed at
load time. Implementation order handles this: all 70 rule kinds and 38 violation
types get structs before integration. Unknown params trigger `assert_exhausted()`
failure.

---

## Estimated Effort

| Component | Items | Complexity | Est. LOC |
|-----------|-------|-----------|----------|
| New enums in types crate | ~18 | Low | ~400 |
| String param value impls + macro | ~20 | Low | ~500 |
| `ClearanceMatrix` type + parser | 1 | Medium | ~150 |
| `SignalLayerMap<T>` + prefix tables | 1 | Medium | ~150 |
| `PolySegment` param parser/writer | 1 | Low | ~80 |
| `ViolationLayerPolygon` parser | 1 | Medium | ~80 |
| Rule base + 70 kind structs | 71 | Medium | ~2000 |
| Rule dispatch + parsing | 1 | Medium | ~200 |
| Violation base + 38 structs | 39 | Medium | ~1200 |
| Violation dispatch | 1 | Low | ~100 |
| WaivedViolation + UNICODE fixup | 1 | Medium | ~120 |
| DrcOptions struct | 1 | Low | ~80 |
| PcbDoc integration (Option C) | 1 | Medium | ~300 |
| Tests (unit + fixture) | ~25 | Medium | ~600 |
| **Total** | | | **~5960** |
