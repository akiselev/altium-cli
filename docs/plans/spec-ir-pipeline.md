# Spec → IR Pipeline: Remove Altium Type Dependencies

## Problem

The current pipeline goes:
```
Spec → SpecModel → apply_spec_pcbdoc(&mut PcbDoc) → PcbIr::extract(&PcbDocBoard) → Router/Placement
```

This causes:
1. **Information loss** — placement constraints, rule scopes, diff pair constraints, net class definitions are dropped when going through PcbDoc
2. **Format coupling** — autopcb-ir depends on altium-format and altium-format-types; autopcb-router directly imports RuleKind from altium-format-types in 13 DRC modules
3. **Blocks multi-format support** — can't add KiCad import when IR is coupled to Altium's type system

## Target Architecture

```
PATH A (spec-only):                          PATH B (spec + existing PcbDoc):
  Spec text                                    Spec text
    → parse → AST                                → parse → AST
    → compile → PcbDocSpec                       → compile → PcbDocSpec
    → lower_to_ir(spec, footprints?)             → apply_spec_pcbdoc(spec, &mut PcbDoc)
    → PcbIr                                      → PcbDocImportAdapter::import(&board)
    │                                            → PcbIr
    ├─► PlacementContext (sibling)               │
    │     (from PlacementSpec)                   ├─► PlacementContext (sibling)
    │                                            │
    ▼                                            ▼
  Router/Placement(ir, ctx)                    Router/Placement(ir, ctx)
```

Both paths converge on `PcbIr` + `PlacementContext`. No altium-format imports in autopcb-ir, autopcb-router, or autopcb-placement.

## Type Ownership

All types IR-native. No shared "ecad-types" crate.

| Current Altium Type | New IR Type | Location | Notes |
|---|---|---|---|
| `RuleKind` | `IrRuleKind` | `autopcb-ir/src/rule.rs` | Exhaustive enum, no `Other` variant (fail-fast) |
| `CornerStyle` | `IrCornerStyle` | `autopcb-ir/src/rule.rs` | 3 variants: Degree90, Degree45, Round |
| `NetTopology` | `IrNetTopology` | `autopcb-ir/src/rule.rs` | 7 variants: Shortest through Starburst |
| `PadShapeKind` | (unchanged) | `autopcb-ir/src/component.rs` | Already IR-native |
| `IrRegionKind` | (unchanged) | `autopcb-ir/src/region.rs` | Already IR-native |
| `CoordPoint` | (removed from types.rs) | conversion in extract.rs only | Helper becomes local fn |

### IrRuleKind

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IrRuleKind {
    // Electrical clearance
    Clearance,
    HoleToHoleClearance,
    BoardOutlineClearance,
    ComponentClearance,
    PowerPlaneClearance,
    Creepage,
    // Routing geometry
    Width,
    RoutingLayers,
    RoutingViaStyle,
    RoutingCornerStyle,
    RoutingTopology,
    RoutingPriority,
    // Net topology / length
    Length,
    MatchedLengths,
    DaisyChainStubLength,
    DiffPairsRouting,
    // Manufacturing
    MinimumAnnularRing,
    MaxMinHoleSize,
    MaxMinHeight,
    MaximumViaCount,
    SolderMaskExpansion,
    PasteMaskExpansion,
    MinimumSolderMaskSliver,
    // Connectivity
    ShortCircuit,
    BrokenNets,
    NetAntennae,
    // Signal integrity
    ParallelSegment,
    AcuteAngle,
    SmdToCorner,
    SmdNeckDown,
    SmdEntry,
    ViasUnderSmd,
    ZAxisClearance,
    // Silk / mask
    SilkToSolderMaskClearance,
    SilkToSilkClearance,
    SilkToBoardRegionClearance,
    // Plane / polygon
    PowerPlaneConnectStyle,
    PolygonConnectStyle,
}
```

## Rule Scopes (Currently Missing — Critical)

`IrDesignRule` gains dual scope fields reflecting Altium's two-selector rule architecture:

```rust
pub struct IrDesignRule {
    pub id: RuleId,
    pub name: String,
    pub kind: IrRuleKind,
    pub priority: i32,
    pub enabled: bool,
    pub scope_a: IrRuleScope,  // first object selector
    pub scope_b: IrRuleScope,  // second object selector (often All)
    pub params: IrRuleParams,
}

pub enum IrRuleScope {
    All,
    NetClass(String),
    Net(String),
    Layer(LayerId),
    Component(ComponentId),
    Between(Box<IrRuleScope>, Box<IrRuleScope>),
}
```

Start flat (no And/Or/Not combinators). Extend when the router actually needs boolean scope expressions.

## Placement Constraints: Sibling Struct

Placement constraints are solver directives, not board state. They live alongside PcbIr, not inside it:

```rust
pub struct PlacementContext {
    pub fixed_positions: Vec<FixedPositionConstraint>,
    pub autoplace_designators: Vec<String>,
    pub ordering: Vec<DirectionalConstraint>,
    pub edge_placements: Vec<EdgePlacementConstraint>,
    pub proximity: Vec<NearConstraint>,
    pub region_containment: Vec<RegionConstraint>,
    pub groups: Vec<ComponentGroup>,
    pub config: PlacementConfig,
    pub unplaced: UnplacedStrategy,
}
```

Solver signature: `solve(ir: &PcbIr, ctx: &PlacementContext)`.

## Coordinate Boundary

All `Coord → f64 mm` conversion at the extraction/lowering boundary. No `Coord` or `CoordPoint` in any IR struct.

## Import Adapter Pattern

```rust
pub trait PcbIrImporter {
    type Error: std::error::Error + Send + Sync + 'static;
    fn import(self) -> Result<PcbIr, Self::Error>;
}
```

- `PcbDocImportAdapter` wraps existing `PcbIr::extract`
- Future `KiCadImportAdapter` conforms to same trait
- Spec lowering (`lower_to_ir`) is a free function, not a trait impl

### IrSource enum for explicit path selection

```rust
pub enum IrSource<'a> {
    PcbDoc { path: &'a Path, spec: Option<&'a PcbDocSpec> },
    Spec { spec: &'a PcbDocSpec, footprints: Option<&'a dyn FootprintResolver> },
}
```

## Migration Steps

| Step | Change | Risk | Effort |
|---|---|---|---|
| 1+2 | Define IrRuleKind/IrCornerStyle/IrNetTopology in autopcb-ir + update all 13 router DRC files via `use autopcb_ir::rule::IrRuleKind as RuleKind` alias | Low | Medium |
| 3 | Remove CoordPoint from types.rs, inline conversion in extract.rs | None | Low |
| 4 | Quarantine: doc-comment enforce extract.rs as sole altium boundary in autopcb-ir | None | Low |
| 5 | Move spec_bridge + extract logic to altium-cli; remove altium-format/autopcb-spec deps from autopcb-ir Cargo.toml | Medium | High |
| 6 | Remove altium-format-types from autopcb-ir Cargo.toml (should be clean after step 5) | None | Trivial |

### Bridge strategy for Steps 1→2

```rust
// In each router DRC file, during transition:
use autopcb_ir::rule::IrRuleKind as RuleKind;
// All existing `RuleKind::Clearance` match arms compile unchanged
```

### Step 5 detail

`PcbIr::extract(&PcbDocBoard)` and `spec_bridge.rs` move to `altium-cli/src/`. `PcbIr` becomes a pure data struct in autopcb-ir with no constructors that reference altium types. All format-specific construction logic lives in the CLI (or a future `autopcb-pipeline` crate).

## Open Questions

1. **Board outline in spec-only path**: Spec-only lowering needs a board outline. Either add `board.outline` to the spec language, or require a target PcbDoc. Until resolved, `lower_to_ir` without a target PcbDoc returns `IrError::NoBoardOutline`.

2. **Pad geometry in spec-only path**: Spec components don't carry pad layout. Needs a `FootprintResolver` trait to resolve pattern → pads from PcbLib. Without it, components are stubs with empty pads.

3. **Rule scope evaluation**: Router's `DrcPolicy` and `RoutingPolicy` need `scope_matches(scope, net, layer) -> bool` helper. Until implemented, all rules default to `IrRuleScope::All` (current behavior, now explicit).

4. **Polygon outlines**: Polygon shapes live in PcbDoc data, not in the spec. Spec-only polygons are stubs with empty outlines.
