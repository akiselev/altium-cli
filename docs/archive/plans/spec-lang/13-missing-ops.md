# 13 - Missing Low-Level Ops

## Context

The spec reconciler needs both Add and Edit operations. The current codebase
has comprehensive Add ops but limited Edit ops. This document inventories what
exists and what must be added.

## Current State

### SchLib Low Ops (sch_ops_core.rs)

| Op | Status | Used by spec |
|----|--------|-------------|
| `CreateComponentRoot` | Exists | Add component |
| `CreateComponentDesignator` | Exists | Add component |
| `CreateComponentComment` | Exists | Add component |
| `AddPin` | Exists | Add pin |
| `AddParameter` | Exists | Add parameter |
| `AddAlias` | Exists | Add alias |
| `RemoveAlias` | Exists | — |
| `RemoveComponent` | Exists | Update fallback (delete+re-add) |
| `EditComponent` | Exists | Update component properties |
| `EditRecord` | Exists | Update pin/parameter/graphic fields |
| `RemoveRecords` | Exists | Update fallback |
| `AddLine` | Exists | Add graphic |
| `AddRectangle` | Exists | Add graphic |
| `AddArc` | Exists | Add graphic |
| `AddEllipticalArc` | Exists | Add graphic |
| `AddEllipse` | Exists | Add graphic |
| `AddPolyline` | Exists | Add graphic |
| `AddPolygon` | Exists | Add graphic |
| `AddBezier` | Exists | Add graphic |
| `AddPie` | Exists | Add graphic |
| `AddRoundRectangle` | Exists | Add graphic |
| `AddLabel` | Exists | Add graphic |
| `AddTextFrame` | Exists | Add graphic |
| `AddImage` | Exists | Add graphic |

**SchLib is well-covered.** `EditComponent` + `EditRecord` (which patches any
record by selector) provide Update capability. No new SchLib ops are strictly
needed for the initial spec implementation.

### PcbLib Low Ops (pcb_ops_core.rs)

| Op | Status | Used by spec |
|----|--------|-------------|
| `AddFootprint` | Exists | Add footprint |
| `AddTrack` | Exists | Add track graphic |
| `AddVia` | Exists | Add via |
| **`AddPad`** | **Missing** | **Add pad** |
| **`EditFootprint`** | **Missing** | **Update footprint properties** |
| **`EditPad`** | **Missing** | **Update pad properties** |
| **`EditTrack`** | **Missing** | **Update track properties** |

### PcbLib High Ops (model.rs)

| Op | Status | Used by spec |
|----|--------|-------------|
| `AddFootprint` | Exists | Add footprint |
| `AddTrack` | Exists | Add track |
| `AddVia` | Exists | Add via |
| **`AddPad`** | **Missing** | **Add pad** |

## Critical Missing: AddPad

The spec language cannot create footprints without pads. `AddPad` is the most
critical missing op and must be implemented before the PcbLib spec path works.

### AddPad Implementation Plan

**High Op** (in `model.rs`):
```rust
#[derive(Debug, Clone, Deserialize, Serialize, OpsSchema)]
#[ops(op = "add_pad", domain = "pcb")]
pub struct AddPadHighOp {
    #[serde(default)]
    pub opid: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[ops(ty = "string", required)]
    pub pad_name: String,
    #[ops(ty = "coord", required)]
    pub at: (i32, i32),
    #[ops(ty = "string")]
    pub shape: Option<String>,           // round, rectangular, octagonal
    #[ops(ty = "dim")]
    pub x_size: Option<i32>,
    #[ops(ty = "dim")]
    pub y_size: Option<i32>,
    #[ops(ty = "dim")]
    pub hole_size: Option<i32>,
    #[ops(ty = "bool")]
    pub is_plated: Option<bool>,
    #[ops(ty = "string")]
    pub layer: Option<String>,
    // ... other pad fields
}
```

**Low Op** (in `pcb_ops_core.rs`):
```rust
pub struct AddPadOp {
    pub opid: String,
    pub id: Option<String>,
    pub pad_name: String,
    pub at: CoordPoint,
    pub shape: PadShape,
    pub x_size: Coord,
    pub y_size: Coord,
    pub hole_size: Coord,
    pub is_plated: bool,
    pub layer: V6Layer,
    pub rotation: f64,
    // ... other fields with defaults
}
```

**Execution**: Call `PcbLib::ops_add_pad()` which creates a `PcbPad` and appends
it to the current footprint's primitive list.

### Composed Op and Lowering

**Composed** (in `lower/mod.rs`):
```rust
ComposedOp::AddPad(AddPadNode)
```

**Lowering** (in `lower/composed_to_pcblib_low.rs`):
```rust
ComposedOp::AddPad(node) => PcbLibLowOp::AddPad(AddPadOp { ... })
```

## Needed Edit Ops (Future Milestones)

These are not required for the initial spec implementation (which uses
delete+re-add fallback for updates) but should be added for efficiency:

### EditPad
Change pad position, shape, size, hole, rotation, layer, mask expansions.

### EditFootprint
Change footprint description, height, pattern.

### EditTrack
Change track start, end, width, layer.

## Implementation Priority

1. **AddPad** — blocks all PcbLib spec functionality. Implement first.
2. **EditComponent/EditRecord** — already exist for SchLib. No work needed.
3. **EditPad/EditFootprint/EditTrack** — not urgent. Delete+re-add works.

## Test Strategy

- AddPad: create pad, verify in footprint, roundtrip save/load
- AddPad with all shape types (round, rectangular, octagonal)
- AddPad with SMD vs through-hole configuration
- AddPad layering (TopLayer, MultiLayer)
- Verify pad appears in correct footprint (context tracking)
