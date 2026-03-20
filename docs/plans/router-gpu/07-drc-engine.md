# Comprehensive DRC Engine for the AutoPCB Router

## Overview

This document specifies a complete Design Rule Check (DRC) engine for the AutoPCB
router, supporting both CPU and GPU backends, with full coverage of every Altium
`TRuleKind` relevant to routing and post-routing validation.

The DRC engine serves three distinct purposes in the routing pipeline:

1. **Routing-time DRC**: Fast incremental checks during PathFinder iterations
   that drive history cost updates and rip-up decisions. Runs every iteration
   (or every N iterations per `DrcConfig`). Must complete in < 20% of iteration time.

2. **Post-routing validation DRC**: Comprehensive check after routing converges,
   producing a detailed `DrcReport` included in the `RouteSolution`. Covers all
   rule types, not just clearance. Can take longer.

3. **Interactive DRC**: Used during active/push-pull routing (plan 06) to validate
   candidate trace placements in real-time. Must run in < 5ms on a local subgraph.

**Key principle**: The CPU DRC engine is the reference implementation and source of
truth. The GPU DRC engine must produce identical results for the checks it handles.
Any rule too complex for GPU parallelization stays on CPU.

---

## 1. Complete Altium Rule Kind Inventory

### 1.1 TRuleKind Enum (70 values, 0-69)

Source: `crates/altium-format-types/src/pcb.rs` (`RuleKind` enum), verified against
C# `TRuleKind` in `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TRuleKind.cs`.

| # | RuleKind | IrRuleParams variant | Category |
|---|----------|---------------------|----------|
| 0 | Clearance | `Clearance { gap_mm }` | Copper spacing |
| 1 | ParallelSegment | `Other` | High-speed / crosstalk |
| 2 | Width | `Width { min_mm, max_mm, preferred_mm }` | Copper geometry |
| 3 | Length | `Other` (has params: MINLIMIT, MAXLIMIT, USEDELAYUNITS, MINDELAY, MAXDELAY) | High-speed / timing |
| 4 | MatchedLengths | `MatchedLengths { tolerance_mm }` | High-speed / timing |
| 5 | DaisyChainStubLength | `Other` | Routing topology |
| 6 | PowerPlaneConnectStyle | `Other` | Plane / polygon |
| 7 | RoutingTopology | `RoutingTopology { topology }` | Routing strategy |
| 8 | RoutingPriority | `RoutingPriority { priority }` | Routing strategy |
| 9 | RoutingLayers | `RoutingLayers { allowed }` | Routing strategy |
| 10 | RoutingCornerStyle | `RoutingCornerStyle { style }` | Routing strategy |
| 11 | RoutingViaStyle | `RoutingViaStyle { ... }` | Routing strategy |
| 12 | PowerPlaneClearance | `Other` | Plane / polygon |
| 13 | SolderMaskExpansion | `SolderMaskExpansion { expansion_mm }` | Manufacturing |
| 14 | PasteMaskExpansion | `PasteMaskExpansion { expansion_mm }` | Manufacturing |
| 15 | ShortCircuit | `Other` | Copper spacing |
| 16 | BrokenNets | `Other` | Connectivity |
| 17 | ViasUnderSmd | `Other` | Placement / DFM |
| 18 | MaximumViaCount | `Other` | Via management |
| 19 | MinimumAnnularRing | `MinimumAnnularRing { min_mm }` | Via geometry |
| 20 | PolygonConnectStyle | `Other` | Plane / polygon |
| 21 | AcuteAngle | `Other` | Copper geometry |
| 22 | ConfinementConstraint | `Other` | Placement |
| 23 | SmdToCorner | `Other` | Copper geometry |
| 24 | ComponentClearance | `ComponentClearance { gap_mm }` | Placement |
| 25 | ComponentRotations | `Other` | Placement |
| 26 | PermittedLayers | `Other` | Layer constraint |
| 27 | NetsToIgnore | `Other` (empty) | DRC filtering |
| 28 | SignalStimulus | `Other` | Signal integrity |
| 29 | OvershootFallingEdge | `Other` | Signal integrity |
| 30 | OvershootRisingEdge | `Other` | Signal integrity |
| 31 | UndershootFallingEdge | `Other` | Signal integrity |
| 32 | UndershootRisingEdge | `Other` | Signal integrity |
| 33 | MaxMinImpedance | `Other` | Signal integrity |
| 34 | SignalTopValue | `Other` | Signal integrity |
| 35 | SignalBaseValue | `Other` | Signal integrity |
| 36 | FlightTimeRisingEdge | `Other` | Signal integrity |
| 37 | FlightTimeFallingEdge | `Other` | Signal integrity |
| 38 | LayerStack | `Other` (empty) | Layer constraint |
| 39 | MaxSlopeRisingEdge | `Other` | Signal integrity |
| 40 | MaxSlopeFallingEdge | `Other` | Signal integrity |
| 41 | SupplyNets | `Other` | Net classification |
| 42 | MaxMinHoleSize | `Other` | Manufacturing |
| 43 | FabricationTestpointStyle | `Other` | DFM / test |
| 44 | FabricationTestpointUsage | `Other` | DFM / test |
| 45 | UnconnectedPin | `Other` (empty) | Connectivity |
| 46 | SmdToPlane | `Other` | Copper geometry |
| 47 | SmdNeckDown | `Other` | Routing strategy |
| 48 | LayerPair | `Other` | Via management |
| 49 | FanoutControl | `Other` (has params) | Routing strategy |
| 50 | MaxMinHeight | `Other` | 3D / mechanical |
| 51 | DifferentialPairsRouting | `DiffPairsRouting { gap_mm, max_gap_mm, max_uncoupled_length_mm }` | High-speed |
| 52 | HoleToHoleClearance | `HoleToHoleClearance { gap_mm }` | Manufacturing |
| 53 | MinimumSolderMaskSliver | `Other` | Manufacturing |
| 54 | SilkToSolderMaskClearance | `Other` | Manufacturing |
| 55 | SilkToSilkClearance | `Other` | Manufacturing |
| 56 | NetAntennae | `Other` | Connectivity |
| 57 | AssyTestPointStyle | `Other` | DFM / test |
| 58 | AssyTestPointUsage | `Other` | DFM / test |
| 59 | SilkToBoardRegionClearance | `Other` | Manufacturing |
| 60 | SmdEntry | `Other` | Routing strategy |
| 61 | None | `Other` (sentinel) | N/A |
| 62 | UnpouredPolygon | `Other` | Plane / polygon |
| 63 | BoardOutlineClearance | `BoardOutlineClearance { gap_mm }` | Board geometry |
| 64 | BackDrilling | `Other` | Manufacturing |
| 65 | Creepage | `Other` | Safety / high-voltage |
| 66 | ReturnPath | `Other` | Signal integrity |
| 67 | RoutingNeckDown | `Other` (has params) | Routing strategy |
| 68 | WireBonding | `Other` | Manufacturing |
| 69 | ZAxisClearance | `Other` | 3D / mechanical |

### 1.2 Rule Categories

**Category A: Routing-Relevant (affect trace/via placement during routing)**

These rules are consumed by `RoutingPolicy` (built in `crates/autopcb-router/src/rules.rs`)
and directly influence the routing algorithm's decisions:

| RuleKind | How it affects routing |
|----------|----------------------|
| Clearance (0) | Determines obstacle inflation, sweepline query range, and per-pair distance threshold |
| Width (2) | Sets min/max/preferred trace width per net/layer |
| RoutingTopology (7) | Determines net decomposition into Steiner tree / star / daisy-chain |
| RoutingPriority (8) | Influences net ordering in PathFinder |
| RoutingLayers (9) | Restricts which layers a net may use |
| RoutingCornerStyle (10) | Corner geometry (45-degree, 90-degree, rounded) |
| RoutingViaStyle (11) | Via hole/annular ring sizing constraints |
| RoutingNeckDown (67) | Allows necking down trace width near SMD pads |
| SmdNeckDown (47) | Legacy neckdown between SMD and plane |
| FanoutControl (49) | BGA/SMD fanout pattern and direction |
| SmdEntry (60) | Pad entry angle for SMD pads |
| DifferentialPairsRouting (51) | Diff-pair gap, width, uncoupled length |
| MatchedLengths (4) | Matched-length tolerance and serpentine params |
| Length (3) | Min/max net length constraints |

**Category B: DRC-Checkable (verifiable after routing)**

These rules can be checked against a completed route solution to detect violations:

| RuleKind | What is checked | Geometry type |
|----------|----------------|---------------|
| Clearance (0) | Copper-to-copper minimum distance | Segment-to-segment, segment-to-pad, via-to-via |
| Width (2) | Trace width within min/max bounds | Per-segment scalar check |
| ShortCircuit (15) | Zero-distance overlap between different nets | Occupancy / overlap detection |
| BrokenNets (16) | All net pins connected | Connectivity graph check |
| MinimumAnnularRing (19) | Via annular ring >= minimum | Per-via scalar check |
| AcuteAngle (21) | No acute angle junctions | Per-junction angle computation |
| SmdToCorner (23) | Min distance from SMD pad to first corner | Trace-from-pad distance |
| ComponentClearance (24) | Component courtyard spacing | Courtyard polygon distance |
| HoleToHoleClearance (52) | Via/hole drill-to-drill distance | Point-to-point distance |
| BoardOutlineClearance (63) | Copper-to-board-edge distance | Segment-to-polygon distance |
| ParallelSegment (1) | Max parallel run length at clearance | Segment-pair parallel analysis |
| MaximumViaCount (18) | Max vias per net | Per-net count |
| Length (3) | Net length within min/max bounds | Sum of segment lengths |
| MatchedLengths (4) | Net lengths within tolerance of target | Cross-net length comparison |
| DifferentialPairsRouting (51) | Gap, width, uncoupled length | Per-segment pair analysis |
| Creepage (65) | Minimum creepage distance for high voltage | Surface-path distance |
| NetAntennae (56) | Antenna-effect: copper connected on one layer only | Per-net layer connectivity analysis |
| PowerPlaneClearance (12) | Clearance in split/power planes | Plane-to-copper distance |
| SolderMaskExpansion (13) | Solder mask opening vs pad | Per-pad expansion check |
| PasteMaskExpansion (14) | Paste mask opening vs pad | Per-pad expansion check |
| MinimumSolderMaskSliver (53) | Min solder mask web between pads | Pad-to-pad solder mask distance |
| SilkToSolderMaskClearance (54) | Silkscreen-to-solder-mask distance | Layer-pair distance |
| SilkToSilkClearance (55) | Silkscreen-to-silkscreen distance | Same-layer distance |
| SilkToBoardRegionClearance (59) | Silk-to-board-edge distance | Layer-to-polygon distance |
| MaxMinHoleSize (42) | Drill hole size within bounds | Per-via scalar check |
| ViasUnderSmd (17) | Via placement under SMD pads | Point-in-region check |
| PolygonConnectStyle (20) | Thermal relief pattern validation | Polygon-to-pad geometry |
| DaisyChainStubLength (5) | Max stub length in daisy-chain topology | Per-net topology analysis |
| MaxMinHeight (50) | Component height within bounds | Per-component scalar check |
| ZAxisClearance (69) | 3D clearance between components | 3D bounding-box intersection |

**Category C: Not DRC-Checkable (strategy/filter/simulation rules)**

| RuleKind | Why not checkable |
|----------|------------------|
| RoutingTopology (7) | Strategy input, not verifiable constraint |
| RoutingPriority (8) | Net ordering preference, not verifiable |
| RoutingLayers (9) | Layer permission (enforced during routing, not checked after) |
| RoutingCornerStyle (10) | Corner geometry preference, checked via AcuteAngle instead |
| RoutingViaStyle (11) | Via sizing (enforced during routing via ViaTemplate selection) |
| RoutingNeckDown (67) | Neckdown permission, not verifiable |
| FanoutControl (49) | Fanout strategy, not verifiable |
| SmdNeckDown (47) | Strategy input |
| SmdEntry (60) | Strategy input |
| NetsToIgnore (27) | DRC scope filter (skip certain nets from DRC) |
| SupplyNets (41) | Net classification (no constraint to check) |
| LayerStack (38) | Layer stack constraint (enforced at design level) |
| LayerPair (48) | Via layer pair definition, not a check |
| ComponentRotations (25) | Placement constraint, not routing DRC |
| PermittedLayers (26) | Placement constraint |
| ConfinementConstraint (22) | Placement constraint |
| None (61) | Sentinel / placeholder |
| UnpouredPolygon (62) | Polygon pour trigger, not a check |
| BackDrilling (64) | Manufacturing process, checked by fabricator |
| WireBonding (68) | Manufacturing process |
| ReturnPath (66) | Signal integrity (needs field-solver simulation) |
| SignalStimulus (28) | Simulation input |
| OvershootFallingEdge (29) | Simulation result comparison |
| OvershootRisingEdge (30) | Simulation result comparison |
| UndershootFallingEdge (31) | Simulation result comparison |
| UndershootRisingEdge (32) | Simulation result comparison |
| MaxMinImpedance (33) | Impedance simulation result |
| SignalTopValue (34) | Signal analysis |
| SignalBaseValue (35) | Signal analysis |
| FlightTimeRisingEdge (36) | Signal analysis |
| FlightTimeFallingEdge (37) | Signal analysis |
| MaxSlopeRisingEdge (39) | Signal analysis |
| MaxSlopeFallingEdge (40) | Signal analysis |
| FabricationTestpointStyle (43) | Test point DFM, not routing DRC |
| FabricationTestpointUsage (44) | Test point DFM |
| UnconnectedPin (45) | Schematic-level check |
| SmdToPlane (46) | Via-in-pad / plane connection check |
| AssyTestPointStyle (57) | Assembly DFM |
| AssyTestPointUsage (58) | Assembly DFM |

---

## 2. DRC Engine Architecture

### 2.1 Module Structure

```
crates/autopcb-router/src/
    drc/
        mod.rs                  # DrcEngine trait, DrcReport, DrcViolation types
        policy.rs               # DrcPolicy: built from PcbIr design rules
        clearance.rs            # Clearance checking (segment-to-segment, segment-to-pad, etc.)
        width.rs                # Width min/max checking
        via.rs                  # Via hole size, annular ring, hole-to-hole clearance
        shorts.rs               # Short circuit detection
        connectivity.rs         # Broken nets, net antennae detection
        length.rs               # Min/max length, matched lengths
        geometry.rs             # Acute angle, SMD-to-corner, parallel segment
        board.rs                # Board outline clearance, component clearance
        manufacturing.rs        # Solder mask, paste mask, silk clearance, mask sliver
        diff_pair.rs            # Diff pair gap, uncoupled length checking
        topology.rs             # Daisy chain stub length, topology validation
        report.rs               # DrcReport rendering for CLI output
    gpu/
        drc.rs                  # GpuDrcEngine: wgpu pipeline for GPU-parallelizable checks
        shaders/
            segment_extract.wgsl
            segment_sort.wgsl
            sweepline_check.wgsl
            short_check.wgsl
            width_check.wgsl
            via_check.wgsl
            violation_compact.wgsl
            drc_history_update.wgsl
```

### 2.2 Core Types

```rust
// crates/autopcb-router/src/drc/mod.rs

/// A single DRC violation.
#[derive(Debug, Clone)]
pub struct DrcViolation {
    /// What kind of violation this is.
    pub kind: DrcViolationKind,
    /// The rule that was violated.
    pub rule_kind: RuleKind,
    /// Rule name (from IrDesignRule.name).
    pub rule_name: String,
    /// First object involved (net, component, etc.).
    pub object_a: DrcObject,
    /// Second object involved (for binary rules like clearance).
    pub object_b: Option<DrcObject>,
    /// Where the violation occurs (board coordinates in mm).
    pub location: PointMm,
    /// Layer on which the violation occurs.
    pub layer: Option<LayerId>,
    /// Actual measured value (distance, width, length, etc.) in mm.
    pub actual_mm: f64,
    /// Required value from the rule in mm.
    pub required_mm: f64,
}

/// Classification of DRC violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DrcViolationKind {
    // Category: Copper spacing
    ClearanceViolation,
    ShortCircuit,
    HoleToHoleClearance,
    BoardOutlineClearance,
    ComponentClearance,
    PowerPlaneClearance,
    CreepageViolation,

    // Category: Copper geometry
    WidthBelowMinimum,
    WidthAboveMaximum,
    AcuteAngle,
    SmdToCornerTooClose,
    ParallelSegmentTooLong,

    // Category: Via geometry
    AnnularRingBelowMinimum,
    HoleSizeBelowMinimum,
    HoleSizeAboveMaximum,
    MaximumViaCountExceeded,
    ViaUnderSmd,

    // Category: Connectivity
    BrokenNet,
    NetAntenna,

    // Category: Length / timing
    NetLengthBelowMinimum,
    NetLengthAboveMaximum,
    MatchedLengthOutOfTolerance,
    DaisyChainStubTooLong,

    // Category: Differential pair
    DiffPairGapViolation,
    DiffPairWidthViolation,
    DiffPairUncoupledLengthExceeded,

    // Category: Manufacturing
    SolderMaskExpansionViolation,
    PasteMaskExpansionViolation,
    SolderMaskSliverBelowMinimum,
    SilkToSolderMaskClearance,
    SilkToSilkClearance,
    SilkToBoardRegionClearance,
}

/// Identifies an object involved in a DRC violation.
#[derive(Debug, Clone)]
pub enum DrcObject {
    /// A routed trace segment.
    Segment {
        net_id: NetId,
        layer: LayerId,
        index: usize,
    },
    /// A routed via.
    Via {
        net_id: NetId,
        index: usize,
    },
    /// A component pad.
    Pad {
        component_id: ComponentId,
        pad_id: PadId,
        net_id: Option<NetId>,
    },
    /// The board outline.
    BoardEdge,
    /// A component courtyard.
    Component {
        component_id: ComponentId,
    },
    /// A net (for connectivity / length checks).
    Net {
        net_id: NetId,
    },
    /// A keepout zone.
    Keepout {
        index: usize,
    },
    /// A polygon pour.
    Polygon {
        polygon_id: PolygonId,
    },
}

/// Complete DRC report.
#[derive(Debug, Clone, Default)]
pub struct DrcReport {
    pub violations: Vec<DrcViolation>,
    pub checked_rule_count: u32,
    pub skipped_rule_count: u32,
    pub skipped_rule_kinds: Vec<RuleKind>,
}

impl DrcReport {
    pub fn violation_count(&self) -> u32 {
        self.violations.len() as u32
    }

    pub fn has_violations(&self) -> bool {
        !self.violations.is_empty()
    }

    /// Count violations by kind.
    pub fn count_by_kind(&self, kind: DrcViolationKind) -> usize {
        self.violations.iter().filter(|v| v.kind == kind).count()
    }

    /// Count violations by rule kind.
    pub fn count_by_rule(&self, rule: RuleKind) -> usize {
        self.violations.iter().filter(|v| v.rule_kind == rule).count()
    }
}

/// DRC engine trait: implemented by both CPU and GPU backends.
pub trait DrcEngine {
    /// Run routing-time DRC (clearance + shorts only, fast).
    /// Returns violation count and updates history costs if provided.
    fn check_routing(
        &self,
        solution: &RouteSolution,
        workspace: &RoutingWorkspace,
        history: Option<&mut HistoryArray>,
    ) -> Result<DrcReport, RoutingError>;

    /// Run comprehensive post-routing DRC (all applicable rules).
    fn check_full(
        &self,
        solution: &RouteSolution,
        workspace: &RoutingWorkspace,
        ir: &PcbIr,
    ) -> Result<DrcReport, RoutingError>;
}
```

### 2.3 DrcPolicy: Rule Resolution

`DrcPolicy` is the DRC-side counterpart to `RoutingPolicy`. It is built from the
same `PcbIr` design rules but preserves the full rule set (not just routing-relevant
rules) and supports net-pair-specific clearance queries.

```rust
// crates/autopcb-router/src/drc/policy.rs

/// DRC policy built from PcbIr design rules.
/// Provides fast lookups for clearance, width, via, length, and other
/// constraints needed by DRC checks.
pub struct DrcPolicy {
    /// Per-net-class-pair clearance matrix.
    /// Indexed by (class_a_id, class_b_id) -> clearance_mm.
    clearance_matrix: ClearanceMatrix,

    /// Per-net-class width constraints.
    width_constraints: HashMap<Option<String>, WidthConstraint>,

    /// Global and per-net-class via constraints.
    via_constraints: ViaConstraints,

    /// Board outline clearance (mm).
    board_outline_clearance_mm: f64,

    /// Component-to-copper clearance (mm).
    component_clearance_mm: f64,

    /// Hole-to-hole clearance (mm).
    hole_to_hole_clearance_mm: f64,

    /// Minimum annular ring (mm).
    min_annular_ring_mm: f64,

    /// Min/max hole size (mm).
    hole_size_min_mm: f64,
    hole_size_max_mm: f64,

    /// Per-net length constraints (net_id -> LengthConstraint).
    length_constraints: HashMap<NetId, LengthConstraint>,

    /// Matched length groups (group_name -> MatchedLengthConstraint).
    matched_length_groups: Vec<MatchedLengthGroup>,

    /// Diff pair constraints (net_id -> DiffPairConstraint).
    diff_pair_constraints: HashMap<NetId, DiffPairConstraint>,

    /// Maximum via count per net (0 = unlimited).
    max_via_count: u32,

    /// Parallel segment constraints.
    parallel_segment_gap_mm: f64,
    parallel_segment_max_length_mm: f64,

    /// Acute angle minimum (degrees).
    acute_angle_min_degrees: f64,

    /// SMD-to-corner minimum distance (mm).
    smd_to_corner_min_mm: f64,

    /// Solder mask expansion (mm).
    solder_mask_expansion_mm: f64,

    /// Paste mask expansion (mm).
    paste_mask_expansion_mm: f64,

    /// Minimum solder mask sliver (mm).
    min_solder_mask_sliver_mm: f64,

    /// Silk clearance constraints (mm).
    silk_to_solder_mask_clearance_mm: f64,
    silk_to_silk_clearance_mm: f64,
    silk_to_board_region_clearance_mm: f64,

    /// Creepage distance (mm).
    creepage_distance_mm: f64,

    /// Daisy chain max stub length (mm).
    daisy_chain_stub_max_mm: f64,

    /// Rules that are present but not DRC-checkable (for reporting).
    skipped_rules: Vec<RuleKind>,
}

/// Per-net-class-pair clearance lookup.
pub struct ClearanceMatrix {
    /// Flat array of clearance values: entries[class_a * num_classes + class_b].
    entries: Vec<f64>,
    num_classes: usize,
    /// Map from net class name to class index.
    class_map: HashMap<String, usize>,
    /// Default class index (for nets without explicit class).
    default_class: usize,
}

impl ClearanceMatrix {
    /// Look up clearance between two nets.
    pub fn clearance(&self, net_a_class: Option<&str>, net_b_class: Option<&str>) -> f64 {
        let a = net_a_class
            .and_then(|n| self.class_map.get(n).copied())
            .unwrap_or(self.default_class);
        let b = net_b_class
            .and_then(|n| self.class_map.get(n).copied())
            .unwrap_or(self.default_class);
        self.entries[a * self.num_classes + b]
    }

    /// Maximum clearance in the matrix (used as sweepline query range).
    pub fn max_clearance(&self) -> f64 {
        self.entries.iter().copied().fold(0.0, f64::max)
    }

    /// Convert to GPU-uploadable format (fixed-point u32 in grid cells).
    pub fn to_gpu_buffer(&self, grid_resolution_mm: f64) -> Vec<u32> {
        self.entries.iter().map(|&c| {
            (c / grid_resolution_mm).ceil() as u32
        }).collect()
    }
}

/// Length constraint for a net.
pub struct LengthConstraint {
    pub min_mm: f64,
    pub max_mm: f64,
    pub use_delay_units: bool,
    pub min_delay_ps: f64,
    pub max_delay_ps: f64,
}

/// Matched length group.
pub struct MatchedLengthGroup {
    pub name: String,
    pub net_ids: Vec<NetId>,
    pub tolerance_mm: f64,
    pub use_delay_units: bool,
    pub delay_tolerance_ps: f64,
}

/// Diff pair constraint.
pub struct DiffPairConstraint {
    pub partner_net_id: NetId,
    pub min_gap_mm: f64,
    pub max_gap_mm: f64,
    pub preferred_gap_mm: f64,
    pub max_uncoupled_length_mm: f64,
}
```

---

## 3. CPU DRC Engine

### 3.1 Architecture

The CPU DRC engine uses the R-tree spatial index (already built in `RoutingWorkspace`)
for candidate pair detection, then performs exact geometric checks on each candidate
pair.

```rust
// crates/autopcb-router/src/drc/mod.rs

pub struct CpuDrcEngine {
    policy: DrcPolicy,
}

impl CpuDrcEngine {
    pub fn new(ir: &PcbIr, config: &RoutingConfig) -> Result<Self, RoutingError> {
        let policy = DrcPolicy::build(ir, config)?;
        Ok(CpuDrcEngine { policy })
    }
}
```

### 3.2 Clearance Checking (`clearance.rs`)

**Algorithm**: For each routed segment on each layer, query the R-tree for nearby
objects (pads, pre-routed traces, keepouts) within `max_clearance + max_width`.
For each candidate pair, compute exact segment-to-segment or segment-to-pad
distance and compare against the clearance rule for that net-class pair.

**Geometry primitives needed**:
- `segment_to_segment_distance(s1, s2) -> f64`: Minimum distance between two line segments.
- `segment_to_circle_distance(seg, center, radius) -> f64`: For round pads.
- `segment_to_rectangle_distance(seg, rect, rotation) -> f64`: For rectangular pads.
- `point_to_segment_distance(point, seg) -> f64`: For via-to-trace checks.

**Implementation outline**:

```rust
pub fn check_clearance(
    solution: &RouteSolution,
    workspace: &RoutingWorkspace,
    policy: &DrcPolicy,
) -> Vec<DrcViolation> {
    let mut violations = Vec::new();

    // 1. Build a temporary R-tree of all routed segments + vias
    //    (spatial_index already has fixed obstacles; we need routed copper too)
    let routed_tree = build_routed_rtree(solution);

    // 2. For each routed segment, query for nearby different-net objects
    for (net_id, routed_net) in &solution.nets {
        for (seg_idx, seg) in routed_net.segments.iter().enumerate() {
            let half_width = seg.width_mm / 2.0;
            let max_clearance = policy.clearance_matrix.max_clearance();
            let query_envelope = segment_envelope(seg, half_width + max_clearance);

            for candidate in routed_tree.query(&query_envelope) {
                // Skip same-net
                if candidate.net_id == *net_id { continue; }

                // Skip different layer
                if candidate.layer != seg.layer { continue; }

                let required = policy.clearance_matrix.clearance(
                    net_class_of(*net_id),
                    net_class_of(candidate.net_id),
                );
                let actual = compute_distance(seg, candidate);
                let effective = actual - half_width - candidate.half_width;

                if effective < required {
                    violations.push(DrcViolation {
                        kind: if actual <= 0.0 {
                            DrcViolationKind::ShortCircuit
                        } else {
                            DrcViolationKind::ClearanceViolation
                        },
                        rule_kind: RuleKind::Clearance,
                        actual_mm: effective,
                        required_mm: required,
                        // ... location, objects, etc.
                    });
                }
            }
        }
    }

    violations
}
```

**R-tree spatial index**: Use `rstar::RTree` (already a dependency) with `RTreeObject`
implementations for `TraceSegment` and `RoutedVia`. The tree is rebuilt for each DRC
pass from the current route solution.

### 3.3 Width Checking (`width.rs`)

Simple per-segment check: verify `seg.width_mm >= min_width` and
`seg.width_mm <= max_width` for the net's width constraint on that layer.

```rust
pub fn check_width(
    solution: &RouteSolution,
    policy: &DrcPolicy,
) -> Vec<DrcViolation> {
    let mut violations = Vec::new();
    for (net_id, routed_net) in &solution.nets {
        let constraint = policy.width_constraint(*net_id);
        for (seg_idx, seg) in routed_net.segments.iter().enumerate() {
            if seg.width_mm < constraint.min - EPS {
                violations.push(/* WidthBelowMinimum */);
            }
            if seg.width_mm > constraint.max + EPS {
                violations.push(/* WidthAboveMaximum */);
            }
        }
    }
    violations
}
```

### 3.4 Via Checking (`via.rs`)

For each routed via:
- **Annular ring**: `(via.diameter_mm - via.drill_mm) / 2 >= min_annular_ring_mm`
- **Hole size**: `min_hole_size <= via.drill_mm <= max_hole_size`
- **Hole-to-hole clearance**: For each pair of vias/through-hole pads, check
  center-to-center distance minus both radii >= `hole_to_hole_clearance_mm`.
- **Maximum via count**: Count vias per net; if count exceeds rule, report.

**Hole-to-hole**: Build a temporary point-set of all drill locations (vias + TH pads),
then use a sweep-line or brute-force for small counts (typical PCBs have < 1000 vias).

### 3.5 Short Circuit Detection (`shorts.rs`)

Two approaches:
1. **Occupancy-based** (fast, grid-resolution limited): Check the PathFinder's occupancy
   grid for cells claimed by more than one net.
2. **Geometric** (exact): Detect zero-distance overlaps between segments of different
   nets. This is a byproduct of the clearance check (distance = 0 => short).

For routing-time DRC, the occupancy-based approach is sufficient. For final validation,
use the geometric approach.

### 3.6 Connectivity Checking (`connectivity.rs`)

- **BrokenNets (16)**: Build a connectivity graph from the routed solution. For each
  net, verify that all pins are connected through routed traces and vias. A pin is
  connected if there exists a path from it to every other pin in the same net through
  same-net segments and vias.

- **NetAntennae (56)**: Check for copper islands connected to a net on only one layer
  without a via connecting to other layers.

### 3.7 Length Checking (`length.rs`)

- **Min/Max Length (3)**: Sum all segment lengths for a net; compare against rule.
- **Matched Lengths (4)**: For each matched-length group, compute net lengths;
  verify all are within tolerance of the target (longest net, or explicit target).

```rust
pub fn check_lengths(
    solution: &RouteSolution,
    policy: &DrcPolicy,
) -> Vec<DrcViolation> {
    let mut violations = Vec::new();

    // Per-net length constraints
    for (net_id, constraint) in &policy.length_constraints {
        if let Some(routed) = solution.nets.get(net_id) {
            let length = routed.routed_length_mm;
            if length < constraint.min_mm {
                violations.push(/* NetLengthBelowMinimum */);
            }
            if length > constraint.max_mm {
                violations.push(/* NetLengthAboveMaximum */);
            }
        }
    }

    // Matched length groups
    for group in &policy.matched_length_groups {
        let lengths: Vec<(NetId, f64)> = group.net_ids.iter()
            .filter_map(|id| {
                solution.nets.get(id).map(|n| (*id, n.routed_length_mm))
            })
            .collect();

        if lengths.len() < 2 { continue; }

        let target = lengths.iter().map(|(_, l)| *l).fold(0.0, f64::max);
        for (net_id, length) in &lengths {
            if (target - *length).abs() > group.tolerance_mm {
                violations.push(/* MatchedLengthOutOfTolerance */);
            }
        }
    }

    violations
}
```

### 3.8 Geometry Checks (`geometry.rs`)

- **AcuteAngle (21)**: At each junction where two segments meet, compute the angle
  between them. If the angle is less than the minimum (typically 90 degrees for
  45-degree routing), report a violation.

- **SmdToCorner (23)**: For each SMD pad, find the first corner in the connected
  trace from that pad. If the distance from pad center to the corner is less than
  the rule minimum, report a violation.

- **ParallelSegment (1)**: For each pair of segments from different nets on the
  same layer, if they are nearly parallel (angle < threshold) and close (gap <
  rule gap), compute the parallel run length. If it exceeds the rule maximum,
  report a violation.

### 3.9 Board Geometry Checks (`board.rs`)

- **BoardOutlineClearance (63)**: For each routed segment, compute the minimum
  distance to the board outline polygon. Compare against the rule.

- **ComponentClearance (24)**: For each routed segment, compute the minimum distance
  to each component's world bounding box. Compare against the rule.

### 3.10 Manufacturing Checks (`manufacturing.rs`)

- **SolderMaskExpansion (13)**: Verify that the solder mask opening around each pad
  is at least `expansion_mm` larger than the pad.

- **PasteMaskExpansion (14)**: Same for paste mask.

- **MinimumSolderMaskSliver (53)**: For each pair of pads where solder mask openings
  are close, verify that the remaining solder mask web between them is >= minimum.

- **Silk clearance rules (54, 55, 59)**: Distance checks between silkscreen objects
  and solder mask openings, other silk objects, and the board edge.

Note: Manufacturing checks require layer-specific geometry data (solder mask, paste
mask, silkscreen) that is not currently in `PcbIr`. These checks will be deferred
until the IR is extended.

### 3.11 Diff Pair Checks (`diff_pair.rs`)

- **Gap violation**: For each pair of coupled segments, compute the edge-to-edge
  gap. Must be within `[min_gap, max_gap]`.

- **Width violation**: Each segment in the pair must match the diff pair width rule.

- **Uncoupled length**: Segments where the pair is not coupled (one net routed on a
  different path) contribute to uncoupled length. Total must be <= max.

### 3.12 Topology Checks (`topology.rs`)

- **DaisyChainStubLength (5)**: In a daisy-chain topology, measure the stub length
  (branch from the main chain to a leaf pin). Must be <= max stub length.

---

## 4. GPU DRC Engine

### 4.1 Which Rules Run on GPU

The GPU DRC engine handles a subset of checks that are massively parallelizable:

| Check | GPU-parallelizable? | Method | Notes |
|-------|-------------------|--------|-------|
| Clearance (0) | YES | Parallel sweepline (X-Check) | Core GPU check; see plan 03 |
| ShortCircuit (15) | YES | Occupancy grid overlap | Existing GPU kernel from PathFinder |
| Width (2) | YES | Per-segment scalar check | Trivially parallel: 1 thread per segment |
| MinimumAnnularRing (19) | YES | Per-via scalar check | Trivially parallel: 1 thread per via |
| HoleSizeMinMax (42) | YES | Per-via scalar check | Same kernel as annular ring |
| HoleToHoleClearance (52) | YES | Point-pair distance (sweepline on via positions) | Similar to clearance but simpler (circles only) |
| BoardOutlineClearance (63) | PARTIAL | Per-segment distance to inflated boundary | Can use obstacle inflation from workspace |
| AcuteAngle (21) | YES | Per-junction angle computation | 1 thread per junction |
| Length (3) | NO | Net-level sum (reduction) | Better on CPU; few nets |
| MatchedLengths (4) | NO | Cross-net comparison | Few nets, CPU is fine |
| ParallelSegment (1) | NO | Complex segment-pair analysis | Sequential analysis needed |
| Connectivity (16) | NO | Graph traversal (BFS/DFS) | Not GPU-friendly |
| DiffPairRouting (51) | NO | Paired segment analysis | Sequential per-pair |
| Manufacturing (13,14,53-55,59) | NO | Not yet in IR | Deferred |

### 4.2 GPU DRC Architecture

The GPU DRC engine reuses infrastructure from plan 03 (X-Check GPU DRC) and integrates
with the `GpuRoutingEngine` from plans 01 and 02.

```rust
// crates/autopcb-router/src/gpu/drc.rs

pub struct GpuDrcEngine {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,

    // Shared buffers (from GpuRoutingEngine)
    segment_buffer: wgpu::Buffer,
    via_buffer: wgpu::Buffer,
    obstacle_buffer: wgpu::Buffer,

    // DRC-specific buffers
    clearance_matrix_buffer: wgpu::Buffer,
    violation_buffer: wgpu::Buffer,
    violation_count_buffer: wgpu::Buffer,
    net_violation_counts_buffer: wgpu::Buffer,

    // Pipelines
    clearance_pipeline: DrcClearancePipeline,
    short_pipeline: DrcShortPipeline,
    width_pipeline: DrcWidthPipeline,
    via_pipeline: DrcViaPipeline,
    history_update_pipeline: DrcHistoryUpdatePipeline,

    // Configuration
    config: DrcConfig,
    policy: DrcPolicy,
}
```

### 4.3 GPU Pipeline Stages

See plan 03 (03-xcheck-gpu-drc.md) for the complete WGSL shader pipeline. The
stages are:

1. **Segment extraction** (`segment_extract.wgsl`): Sort segments by layer into
   contiguous arrays.

2. **Sort by Y** (`segment_sort.wgsl`): Radix sort (or CPU sort + upload) segments
   by y-coordinate within each layer.

3. **Parallel sweepline** (`sweepline_check.wgsl`): Three-step parallel prefix
   computation (batch -> sweep -> refine) detecting clearance violations.

4. **Short detection** (`short_check.wgsl`): Occupancy grid overlap detection
   for zero-distance violations.

5. **Width check** (`width_check.wgsl`): Per-segment width validation.
   ```wgsl
   @compute @workgroup_size(256)
   fn check_width(@builtin(global_invocation_id) gid: vec3<u32>) {
       let idx = gid.x;
       if (idx >= params.num_segments) { return; }
       let seg = segments[idx];
       let constraint = width_constraints[seg.net_class];
       if (seg.half_width * 2u < constraint.min_width) {
           report_violation(idx, WIDTH_BELOW_MINIMUM);
       }
       if (seg.half_width * 2u > constraint.max_width) {
           report_violation(idx, WIDTH_ABOVE_MAXIMUM);
       }
   }
   ```

6. **Via check** (`via_check.wgsl`): Per-via annular ring and hole size validation.
   ```wgsl
   @compute @workgroup_size(256)
   fn check_vias(@builtin(global_invocation_id) gid: vec3<u32>) {
       let idx = gid.x;
       if (idx >= params.num_vias) { return; }
       let via = vias[idx];
       let annular_ring = (via.diameter - via.hole_size) / 2u;
       if (annular_ring < params.min_annular_ring) {
           report_violation_via(idx, ANNULAR_RING_BELOW_MINIMUM);
       }
       if (via.hole_size < params.min_hole_size) {
           report_violation_via(idx, HOLE_SIZE_BELOW_MINIMUM);
       }
       if (via.hole_size > params.max_hole_size) {
           report_violation_via(idx, HOLE_SIZE_ABOVE_MAXIMUM);
       }
   }
   ```

7. **Violation compaction** (`violation_compact.wgsl`): Compact scattered violations
   into dense output buffer.

8. **History update** (`drc_history_update.wgsl`): Update PathFinder history costs
   at violation locations.

### 4.4 Dynamic Algorithm Selection

Following X-Check Section 5.1, select CPU or GPU based on data size:

```rust
const GPU_SEGMENT_THRESHOLD: usize = 5_000;

fn select_drc_engine(
    segment_count: usize,
    gpu_available: bool,
) -> DrcBackend {
    if gpu_available && segment_count >= GPU_SEGMENT_THRESHOLD {
        DrcBackend::Gpu
    } else {
        DrcBackend::Cpu
    }
}
```

For boards with fewer than 5,000 segments, GPU kernel launch and data transfer
overhead dominate, making CPU faster.

---

## 5. Complete Rule-to-Implementation Mapping

| # | TRuleKind | IrRuleParams | DRC Check Type | CPU Module | GPU Shader | Priority | Implementation Notes |
|---|-----------|-------------|---------------|------------|------------|----------|---------------------|
| 0 | Clearance | `Clearance { gap_mm }` | Segment-to-segment/pad/via distance | `clearance.rs` | `sweepline_check.wgsl` | P0 (critical) | Per-net-class matrix; GPU sweepline; R-tree on CPU |
| 1 | ParallelSegment | `Other` | Parallel run length at gap | `geometry.rs` | N/A (CPU only) | P2 | Segment-pair angle + length analysis |
| 2 | Width | `Width { min, max, pref }` | Per-segment scalar | `width.rs` | `width_check.wgsl` | P0 (critical) | Trivially parallel on GPU |
| 3 | Length | `Other` | Per-net sum | `length.rs` | N/A | P1 | CPU reduction per net |
| 4 | MatchedLengths | `MatchedLengths { tol }` | Cross-net comparison | `length.rs` | N/A | P1 | Group-level comparison |
| 5 | DaisyChainStubLength | `Other` | Topology analysis | `topology.rs` | N/A | P3 | Requires topology graph |
| 6 | PowerPlaneConnectStyle | `Other` | Polygon-to-pad geometry | N/A (future) | N/A | P4 | Requires polygon pour data |
| 7 | RoutingTopology | `RoutingTopology` | N/A (strategy) | N/A | N/A | -- | Not a DRC check |
| 8 | RoutingPriority | `RoutingPriority` | N/A (strategy) | N/A | N/A | -- | Not a DRC check |
| 9 | RoutingLayers | `RoutingLayers` | N/A (strategy) | N/A | N/A | -- | Enforced during routing |
| 10 | RoutingCornerStyle | `RoutingCornerStyle` | N/A (strategy) | N/A | N/A | -- | See AcuteAngle |
| 11 | RoutingViaStyle | `RoutingViaStyle` | N/A (strategy) | N/A | N/A | -- | Enforced during routing |
| 12 | PowerPlaneClearance | `Other` | Plane-to-copper | N/A (future) | N/A | P4 | Requires plane data |
| 13 | SolderMaskExpansion | `SolderMaskExpansion` | Per-pad scalar | `manufacturing.rs` | N/A | P3 | Requires mask layer data in IR |
| 14 | PasteMaskExpansion | `PasteMaskExpansion` | Per-pad scalar | `manufacturing.rs` | N/A | P3 | Requires mask layer data in IR |
| 15 | ShortCircuit | `Other` | Occupancy overlap | `shorts.rs` | `short_check.wgsl` | P0 (critical) | Grid-based (fast) or geometric (exact) |
| 16 | BrokenNets | `Other` | Graph connectivity | `connectivity.rs` | N/A | P1 | BFS/DFS per net |
| 17 | ViasUnderSmd | `Other` | Point-in-region | `via.rs` | N/A | P3 | Via center vs SMD pad region |
| 18 | MaximumViaCount | `Other` | Per-net count | `via.rs` | N/A | P2 | Simple counter |
| 19 | MinimumAnnularRing | `MinimumAnnularRing` | Per-via scalar | `via.rs` | `via_check.wgsl` | P1 | GPU-parallelizable |
| 20 | PolygonConnectStyle | `Other` | Polygon-to-pad geometry | N/A (future) | N/A | P4 | Requires polygon pour |
| 21 | AcuteAngle | `Other` | Per-junction angle | `geometry.rs` | Possible future | P2 | Angle between connected segments |
| 22 | ConfinementConstraint | `Other` | N/A (placement) | N/A | N/A | -- | Not routing DRC |
| 23 | SmdToCorner | `Other` | Trace-from-pad distance | `geometry.rs` | N/A | P2 | First-corner distance from SMD |
| 24 | ComponentClearance | `ComponentClearance` | Courtyard distance | `board.rs` | N/A | P2 | Component bbox to copper |
| 25 | ComponentRotations | `Other` | N/A (placement) | N/A | N/A | -- | Not routing DRC |
| 26 | PermittedLayers | `Other` | N/A (placement) | N/A | N/A | -- | Not routing DRC |
| 27 | NetsToIgnore | `Other` (empty) | DRC filter | Policy filter | N/A | -- | Nets in scope are excluded from DRC |
| 28-37 | Signal* / Overshoot* / Undershoot* / Impedance* / FlightTime* | `Other` | N/A (simulation) | N/A | N/A | -- | Requires field solver |
| 38 | LayerStack | `Other` (empty) | N/A (layer constraint) | N/A | N/A | -- | Not routing DRC |
| 39-40 | MaxSlope* | `Other` | N/A (simulation) | N/A | N/A | -- | Requires field solver |
| 41 | SupplyNets | `Other` | N/A (classification) | N/A | N/A | -- | Not a check |
| 42 | MaxMinHoleSize | `Other` | Per-via scalar | `via.rs` | `via_check.wgsl` | P1 | GPU-parallelizable |
| 43-44 | *TestpointStyle/Usage | `Other` | N/A (DFM) | N/A | N/A | -- | Not routing DRC |
| 45 | UnconnectedPin | `Other` (empty) | N/A (schematic) | N/A | N/A | -- | Schematic-level |
| 46 | SmdToPlane | `Other` | N/A (strategy) | N/A | N/A | -- | Not routing DRC |
| 47 | SmdNeckDown | `Other` | N/A (strategy) | N/A | N/A | -- | Not routing DRC |
| 48 | LayerPair | `Other` | N/A (definition) | N/A | N/A | -- | Not a check |
| 49 | FanoutControl | `Other` | N/A (strategy) | N/A | N/A | -- | Not routing DRC |
| 50 | MaxMinHeight | `Other` | Per-component scalar | N/A (future) | N/A | P4 | Requires 3D model data |
| 51 | DiffPairsRouting | `DiffPairsRouting` | Paired segment analysis | `diff_pair.rs` | N/A | P1 | Gap, width, uncoupled length |
| 52 | HoleToHoleClearance | `HoleToHoleClearance` | Point-pair distance | `via.rs` | Possible future | P1 | Sweepline on drill points |
| 53 | MinSolderMaskSliver | `Other` | Pad-pair mask distance | `manufacturing.rs` | N/A | P3 | Requires mask layer data |
| 54 | SilkToSolderMaskClearance | `Other` | Layer-pair distance | `manufacturing.rs` | N/A | P3 | Requires silk/mask layers |
| 55 | SilkToSilkClearance | `Other` | Same-layer distance | `manufacturing.rs` | N/A | P3 | Requires silk layer |
| 56 | NetAntennae | `Other` | Layer connectivity | `connectivity.rs` | N/A | P2 | Per-net layer presence check |
| 57-58 | Assy*TestPoint* | `Other` | N/A (DFM) | N/A | N/A | -- | Not routing DRC |
| 59 | SilkToBoardRegionClearance | `Other` | Silk-to-edge distance | `manufacturing.rs` | N/A | P3 | Requires silk layer |
| 60 | SmdEntry | `Other` | N/A (strategy) | N/A | N/A | -- | Not routing DRC |
| 61 | None | `Other` (sentinel) | N/A | N/A | N/A | -- | Placeholder |
| 62 | UnpouredPolygon | `Other` | N/A (pour trigger) | N/A | N/A | -- | Not a check |
| 63 | BoardOutlineClearance | `BoardOutlineClearance` | Segment-to-polygon | `board.rs` | Inflation-based | P1 | Obstacle inflation pre-computed |
| 64 | BackDrilling | `Other` | N/A (manufacturing) | N/A | N/A | -- | Fabrication process |
| 65 | Creepage | `Other` | Surface-path distance | N/A (future) | N/A | P4 | Complex surface-trace analysis |
| 66 | ReturnPath | `Other` | N/A (simulation) | N/A | N/A | -- | Requires impedance analysis |
| 67 | RoutingNeckDown | `Other` | N/A (strategy) | N/A | N/A | -- | Not routing DRC |
| 68 | WireBonding | `Other` | N/A (manufacturing) | N/A | N/A | -- | Not routing DRC |
| 69 | ZAxisClearance | `Other` | N/A (3D) | N/A | N/A | P4 | Requires 3D model data |

### Priority Legend

- **P0**: Critical for router convergence. Must be implemented in Phase 1.
- **P1**: Required for post-routing validation. Phase 1 (CPU) and Phase 2 (GPU).
- **P2**: Important DRC checks. Phase 3.
- **P3**: Manufacturing / DFM checks. Phase 3-4 (requires IR extensions).
- **P4**: Advanced / deferred. Future milestones.
- **--**: Not a DRC check (strategy rule, simulation input, placement constraint, etc.).

---

## 6. Integration with Router Pipeline

### 6.1 Routing-Time DRC (PathFinder Integration)

Position in the PathFinder iteration loop (from `pathfinder/mod.rs`):

```
PathFinder iteration N:
  1. Rip-up (all nets or hot-set)
  2. Order nets (RoutingPriority, congestion heuristic)
  3. Route each net (A* / GPU Bellman-Ford)
  4. Update occupancy grid / detect conflicts
  5. *** DRC pass (clearance + shorts) ***          <-- NEW
  6. Update history costs from DRC violations       <-- NEW
  7. Update pres_fac (exponential growth)
  8. Capture iteration snapshot
  9. Check convergence: conflicts == 0 AND drc_violations == 0
```

**When to run DRC** (from `DrcConfig`):

```rust
/// Configuration for routing-time DRC.
pub struct DrcConfig {
    /// First iteration to run short-circuit detection (default: 1).
    pub short_check_start_iteration: u32,
    /// First iteration to run full clearance DRC (default: 3).
    pub full_drc_start_iteration: u32,
    /// Maximum violations to report per iteration (prevents OOM).
    pub max_violations_per_iteration: u32,
    /// History cost increment per clearance violation.
    pub clearance_history_increment: f64,
    /// History cost increment per short circuit.
    pub short_history_increment: f64,
    /// Neighborhood radius for history spreading (cells).
    pub history_spread_radius: u32,
    /// Minimum segment count to use GPU DRC (below this, use CPU).
    pub gpu_segment_threshold: usize,
}

impl Default for DrcConfig {
    fn default() -> Self {
        DrcConfig {
            short_check_start_iteration: 1,
            full_drc_start_iteration: 3,
            max_violations_per_iteration: 10_000,
            clearance_history_increment: 3.0,
            short_history_increment: 10.0,
            history_spread_radius: 1,
            gpu_segment_threshold: 5_000,
        }
    }
}
```

**Integration code** (added to `pathfinder_route`):

```rust
// After step 4 (occupancy update):
if state.iteration >= config.drc.short_check_start_iteration {
    let drc_mode = if state.iteration >= config.drc.full_drc_start_iteration {
        DrcMode::Full  // clearance + shorts + width
    } else {
        DrcMode::ShortsOnly
    };

    let drc_report = drc_engine.check_routing(
        &current_solution,
        workspace,
        Some(&mut state.history),
    )?;

    // Update history costs at violation locations
    for violation in &drc_report.violations {
        let (gx, gy) = workspace.grid.to_grid(violation.location);
        if let Some(layer) = violation.layer {
            state.history.increment(
                gx, gy,
                layer.raw() as usize,
                match violation.kind {
                    DrcViolationKind::ShortCircuit => config.drc.short_history_increment,
                    _ => config.drc.clearance_history_increment,
                },
            );
            // Spread to neighbors
            for dx in -(config.drc.history_spread_radius as i32)..=(config.drc.history_spread_radius as i32) {
                for dy in -(config.drc.history_spread_radius as i32)..=(config.drc.history_spread_radius as i32) {
                    if dx == 0 && dy == 0 { continue; }
                    let nx = gx as i64 + dx as i64;
                    let ny = gy as i64 + dy as i64;
                    if nx >= 0 && ny >= 0 {
                        state.history.increment(
                            nx as u32, ny as u32,
                            layer.raw() as usize,
                            config.drc.clearance_history_increment / 2.0,
                        );
                    }
                }
            }
        }
    }

    // Add violating nets to hot-set for next iteration
    for violation in &drc_report.violations {
        if let DrcObject::Segment { net_id, .. } | DrcObject::Via { net_id, .. } = violation.object_a {
            hot_set.add(net_id);
        }
        if let Some(DrcObject::Segment { net_id, .. } | DrcObject::Via { net_id, .. }) = &violation.object_b {
            hot_set.add(*net_id);
        }
    }

    // Update snapshot with DRC data
    snapshot.drc_violations = drc_report.violation_count();
}
```

**Convergence condition** (updated):

```rust
fn check_convergence(state: &PathFinderState, drc_violations: u32) -> bool {
    // Both occupancy conflicts AND DRC violations must be zero
    let conflicts = count_conflicts(&occupancy);
    conflicts == 0 && drc_violations == 0
}
```

### 6.2 Post-Routing Validation DRC

After routing converges (or hits max iterations), run comprehensive DRC:

```rust
// In route_board(), after pathfinder_route():
let drc_report = drc_engine.check_full(&solution, workspace, ir)?;
solution.metrics.drc_violations = drc_report.violation_count();

// Attach detailed report for CLI output
solution.drc_report = Some(drc_report);
```

This requires extending `RouteSolution` in `autopcb-routes`:

```rust
pub struct RouteSolution {
    // ... existing fields ...
    /// Detailed DRC report from post-routing validation.
    pub drc_report: Option<DrcReport>,
}
```

### 6.3 DRC in CLI Output

The `altium routing inspect` command should display DRC results:

```
$ altium routing inspect solution.routes
Routing Solution v1
  Nets routed: 142/145 (97.9%)
  Total length: 1234.5 mm
  Total vias: 87
  DRC violations: 3

  DRC Report:
    Clearance violations: 1
      - Net "GND" (seg #12) <-> Net "VCC" (seg #5) on Top Layer at (23.4, 15.2)mm
        actual: 0.15mm, required: 0.20mm
    Width violations: 1
      - Net "DATA0" (seg #3) on Mid Layer 1: width 0.08mm < minimum 0.10mm
    Short circuits: 1
      - Net "CLK" <-> Net "DATA1" on Bottom Layer at (45.6, 32.1)mm
```

### 6.4 DRC Driving Rip-Up Decisions

DRC violations directly influence the PathFinder's rip-up strategy:

1. **Violation-weighted net ordering**: Nets with more violations are routed later
   (giving them access to more routing resources after other nets have been placed).

2. **Hot-set expansion**: All nets involved in DRC violations are added to the
   hot-set for forced rip-up in the next iteration.

3. **History cost amplification**: DRC violations at specific locations increase
   history costs more aggressively than simple occupancy conflicts, because they
   represent geometry-level problems (not just resource contention).

4. **Convergence gating**: The routing loop cannot declare convergence while any
   DRC violations remain. This prevents the router from producing "conflict-free
   but DRC-violating" solutions.

---

## 7. GPU DRC Data Flow

### 7.1 Integration with GpuRoutingEngine

The GPU DRC engine shares buffers with the `GpuRoutingEngine` (from plan 01):

```
GpuRoutingEngine (existing):
  ├── distance_buffer[]
  ├── predecessor_buffer[]
  ├── occupancy_buffer[]
  ├── history_buffer[]              <-- DRC writes here
  └── obstacle_buffer[]

GpuDrcEngine (new, shares device/queue):
  ├── segment_buffer[]              <-- uploaded from RouteSolution per iteration
  ├── via_buffer[]                  <-- uploaded from RouteSolution per iteration
  ├── sorted_segment_buffer[]       <-- internal (sort output)
  ├── clearance_matrix_buffer[]     <-- uploaded once from DrcPolicy
  ├── width_constraints_buffer[]    <-- uploaded once from DrcPolicy
  ├── via_constraints_buffer[]      <-- uploaded once from DrcPolicy
  ├── violation_buffer[]            <-- output
  ├── violation_count_buffer[]      <-- output (single u32)
  └── net_violation_counts_buffer[] <-- output (per-net counts)
```

### 7.2 Per-Iteration GPU DRC Dispatch

```rust
impl GpuDrcEngine {
    fn check_routing_gpu(
        &self,
        solution: &RouteSolution,
        workspace: &RoutingWorkspace,
    ) -> Result<GpuDrcResult, RoutingError> {
        // 1. Upload routed segments and vias to GPU
        self.upload_segments(solution);
        self.upload_vias(solution);

        // 2. Encode DRC compute passes
        let mut encoder = self.device.create_command_encoder(&Default::default());

        // Pass 1: Extract segments per layer
        self.clearance_pipeline.encode_extract(&mut encoder);

        // Pass 2: Sort segments by Y (or CPU sort + upload)
        self.clearance_pipeline.encode_sort(&mut encoder);

        // Pass 3: Parallel sweepline clearance check
        self.clearance_pipeline.encode_sweepline(&mut encoder);

        // Pass 4: Short circuit detection (occupancy overlap)
        self.short_pipeline.encode(&mut encoder);

        // Pass 5: Width check
        self.width_pipeline.encode(&mut encoder);

        // Pass 6: Via check (annular ring + hole size)
        self.via_pipeline.encode(&mut encoder);

        // Pass 7: Update history costs from violations
        self.history_update_pipeline.encode(&mut encoder);

        // 3. Submit and read back violation count
        self.queue.submit(std::iter::once(encoder.finish()));
        let violation_count = self.read_violation_count();

        Ok(GpuDrcResult {
            violation_count,
            // Full violations read back only for final validation or when count is small
        })
    }
}
```

### 7.3 Violation Readback Strategy

- **During routing**: Read back only `violation_count` (1 u32) and
  `net_violation_counts[]` (array of u32, one per net). This avoids the latency
  of reading the full violation buffer. The history update happens on GPU via
  `drc_history_update.wgsl` without CPU involvement.

- **Final validation**: Read back the complete violation buffer and convert to
  `Vec<DrcViolation>` for the `DrcReport`.

---

## 8. IR Extensions Required

The following `IrRuleParams` variants need to be added or extended to support the
full DRC engine:

### 8.1 Existing variants needing no changes

- `Clearance { gap_mm }` -- sufficient for basic clearance
- `Width { min_mm, max_mm, preferred_mm }` -- sufficient
- `MinimumAnnularRing { min_mm }` -- sufficient
- `HoleToHoleClearance { gap_mm }` -- sufficient
- `BoardOutlineClearance { gap_mm }` -- sufficient
- `ComponentClearance { gap_mm }` -- sufficient
- `SolderMaskExpansion { expansion_mm }` -- sufficient
- `PasteMaskExpansion { expansion_mm }` -- sufficient
- `MatchedLengths { tolerance_mm }` -- sufficient for basic check
- `DiffPairsRouting { gap_mm, max_gap_mm, max_uncoupled_length_mm }` -- sufficient

### 8.2 New variants needed for full DRC

```rust
// New IrRuleParams variants needed:
pub enum IrRuleParams {
    // ... existing variants ...

    /// Short circuit rule (currently Other; needs typed variant for DRC-enabled flag).
    ShortCircuit {
        allow_short_circuit: bool,  // some nets intentionally short (e.g. test points)
    },

    /// Broken nets rule.
    BrokenNets {
        check_enabled: bool,
    },

    /// Min/max net length.
    Length {
        min_mm: f64,
        max_mm: f64,
        use_delay_units: bool,
        min_delay_ps: f64,
        max_delay_ps: f64,
    },

    /// Parallel segment constraint.
    ParallelSegment {
        gap_mm: f64,
        max_parallel_length_mm: f64,
    },

    /// Maximum via count per net.
    MaximumViaCount {
        max_count: u32,
    },

    /// Min/max hole size.
    MaxMinHoleSize {
        min_mm: f64,
        max_mm: f64,
    },

    /// Acute angle minimum.
    AcuteAngle {
        min_angle_degrees: f64,
    },

    /// SMD-to-corner minimum distance.
    SmdToCorner {
        min_distance_mm: f64,
    },

    /// Minimum solder mask sliver.
    MinSolderMaskSliver {
        min_mm: f64,
    },

    /// Silk-to-solder-mask clearance.
    SilkToSolderMaskClearance {
        gap_mm: f64,
    },

    /// Silk-to-silk clearance.
    SilkToSilkClearance {
        gap_mm: f64,
    },

    /// Net antennae detection.
    NetAntennae {
        check_enabled: bool,
    },

    /// Vias under SMD restriction.
    ViasUnderSmd {
        allow_under_smd: bool,
    },

    /// Daisy chain stub length.
    DaisyChainStubLength {
        max_stub_mm: f64,
    },

    /// Creepage distance.
    Creepage {
        distance_mm: f64,
        voltage: f64,
    },
}
```

### 8.3 ClearanceMatrix enhancement for Clearance rule

The current `Clearance { gap_mm }` is a single global value. Altium supports a
per-object-type clearance matrix (`TClearanceConstraintMode` with `IsMatrix` flag
and `OBJECTCLEARANCES` parameter). The IR should be extended to support this:

```rust
pub enum IrRuleParams {
    Clearance {
        gap_mm: f64,
        /// Per-object-type clearance overrides.
        /// Key format: "{obj_a_type}-{obj_b_type}", e.g. "Pad-Track", "Via-Via".
        object_clearances: Option<HashMap<String, f64>>,
    },
    // ...
}
```

---

## 9. Implementation Phases

### Phase 1: CPU DRC Baseline (Milestone 7 extension)

**Goal**: Working CPU DRC that runs in the PathFinder loop and drives convergence.

**Files to create/modify**:
- `crates/autopcb-router/src/drc/mod.rs` (replace stub)
- `crates/autopcb-router/src/drc/policy.rs`
- `crates/autopcb-router/src/drc/clearance.rs`
- `crates/autopcb-router/src/drc/width.rs`
- `crates/autopcb-router/src/drc/via.rs`
- `crates/autopcb-router/src/drc/shorts.rs`
- `crates/autopcb-router/src/drc/report.rs`
- `crates/autopcb-router/src/pathfinder/mod.rs` (integrate DRC into loop)
- `crates/autopcb-router/src/config.rs` (add DrcConfig)
- `crates/autopcb-routes/src/lib.rs` (add drc_violations to metrics, DrcReport to solution)

**Checks implemented**:
- Clearance (copper-to-copper, segment-to-pad, via-to-trace)
- Short circuit detection (occupancy-based)
- Width min/max
- Minimum annular ring
- Board outline clearance
- Hole-to-hole clearance
- Hole size min/max
- Maximum via count

**Acceptance criteria**:
- DRC correctly identifies clearance violations on synthetic test boards
- PathFinder convergence uses `drc_violations == 0` as gating condition
- CPU DRC time < 50ms for boards with < 10,000 segments
- DRC violations visible in `RouteSolution.metrics.drc_violations`

### Phase 2: GPU DRC Pipeline (new milestone after Milestone 7)

**Goal**: GPU-accelerated DRC for clearance, shorts, width, and via checks.

**Files to create**:
- `crates/autopcb-router/src/gpu/drc.rs`
- `crates/autopcb-router/src/gpu/shaders/segment_extract.wgsl`
- `crates/autopcb-router/src/gpu/shaders/segment_sort.wgsl`
- `crates/autopcb-router/src/gpu/shaders/sweepline_check.wgsl`
- `crates/autopcb-router/src/gpu/shaders/short_check.wgsl`
- `crates/autopcb-router/src/gpu/shaders/width_check.wgsl`
- `crates/autopcb-router/src/gpu/shaders/via_check.wgsl`
- `crates/autopcb-router/src/gpu/shaders/violation_compact.wgsl`
- `crates/autopcb-router/src/gpu/shaders/drc_history_update.wgsl`

**Acceptance criteria**:
- GPU DRC produces identical results to CPU DRC (verified by property tests)
- GPU DRC is faster than CPU for boards with > 5,000 segments
- Dynamic selection between CPU and GPU based on segment count
- Total DRC overhead < 20% of PathFinder iteration time

### Phase 3: Advanced DRC and Post-Route Validation (Milestone 8)

**Goal**: Comprehensive post-route validation covering all DRC-checkable rules.

**Files to create/modify**:
- `crates/autopcb-router/src/drc/connectivity.rs`
- `crates/autopcb-router/src/drc/length.rs`
- `crates/autopcb-router/src/drc/geometry.rs`
- `crates/autopcb-router/src/drc/board.rs`
- `crates/autopcb-router/src/drc/diff_pair.rs`
- `crates/autopcb-router/src/drc/topology.rs`
- `crates/autopcb-ir/src/rule.rs` (new IrRuleParams variants)
- CLI: `altium routing inspect` DRC output

**Checks added**:
- Broken nets (connectivity graph)
- Net length min/max
- Matched lengths (cross-net tolerance)
- Acute angle detection
- SMD-to-corner distance
- Parallel segment analysis
- Diff pair gap/width/uncoupled length
- Net antennae
- Component clearance

### Phase 4: Manufacturing DRC (Future)

**Goal**: DFM checks requiring manufacturing layer data.

**Prerequisites**: Extend `PcbIr` with solder mask, paste mask, and silkscreen
layer geometry.

**Checks added**:
- Solder mask expansion
- Paste mask expansion
- Minimum solder mask sliver
- Silk-to-solder-mask clearance
- Silk-to-silk clearance
- Silk-to-board-region clearance

---

## 10. Testing Strategy

### 10.1 Unit Tests (per check module)

Each DRC check module has focused unit tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Clearance tests
    #[test]
    fn parallel_traces_no_violation() { /* gap > required */ }
    #[test]
    fn parallel_traces_clearance_violation() { /* gap < required */ }
    #[test]
    fn overlapping_traces_short_circuit() { /* distance = 0 */ }
    #[test]
    fn same_net_no_violation() { /* same-net exclusion */ }
    #[test]
    fn per_class_clearance_matrix() { /* different clearance per net class */ }
    #[test]
    fn diagonal_trace_clearance() { /* 45-degree segments */ }
    #[test]
    fn trace_to_pad_clearance() { /* segment-to-circle distance */ }
    #[test]
    fn via_to_trace_clearance() { /* point-to-segment distance */ }

    // Width tests
    #[test]
    fn width_within_bounds() { /* no violation */ }
    #[test]
    fn width_below_minimum() { /* violation */ }
    #[test]
    fn width_above_maximum() { /* violation */ }

    // Via tests
    #[test]
    fn annular_ring_sufficient() { /* no violation */ }
    #[test]
    fn annular_ring_too_small() { /* violation */ }
    #[test]
    fn hole_to_hole_sufficient() { /* no violation */ }
    #[test]
    fn hole_to_hole_too_close() { /* violation */ }
}
```

### 10.2 CPU/GPU Equivalence Tests

```rust
#[cfg(feature = "proptest")]
proptest! {
    #[test]
    fn gpu_cpu_drc_agreement(
        segments in arb_segments(10..500),
        vias in arb_vias(0..50),
        clearance in 0.1f64..1.0f64,
    ) {
        let policy = DrcPolicy::uniform(clearance);
        let cpu_report = CpuDrcEngine::check(&segments, &vias, &policy);
        let gpu_report = GpuDrcEngine::check(&segments, &vias, &policy);
        prop_assert_eq!(cpu_report.violation_count(), gpu_report.violation_count());
        // Verify same violation pairs (order-independent)
    }
}
```

### 10.3 Integration Tests

- Route a synthetic board with known violations, verify DRC finds them all.
- Route a violation-free board, verify zero violations.
- Verify PathFinder convergence is gated on DRC: a board that routes with occupancy
  conflicts = 0 but clearance violations > 0 does NOT declare convergence until
  the clearance violations are resolved via rip-up/reroute.
- Benchmark DRC time as a percentage of total PathFinder iteration time.
- Verify DRC report rendering in CLI output.

---

## 11. Performance Targets

| Board complexity | Segments | Vias | CPU DRC time | GPU DRC time | Notes |
|-----------------|----------|------|-------------|-------------|-------|
| Simple (2L, 100 nets) | ~2,000 | ~50 | < 5ms | N/A (CPU faster) | GPU overhead dominates |
| Medium (4L, 500 nets) | ~10,000 | ~200 | < 50ms | < 5ms | GPU 10x speedup |
| Complex (6L, 2000 nets) | ~50,000 | ~1,000 | < 500ms | < 15ms | GPU 30x speedup |
| Dense (8L, 5000 nets) | ~200,000 | ~5,000 | ~5s | < 30ms | GPU essential |

The GPU DRC must contribute < 20% overhead to the total PathFinder iteration time.
For a typical 4-layer, 500-net board where an iteration takes ~200ms (routing +
occupancy + history), the DRC budget is ~40ms.

---

## 12. Cross-References

### Codebase Files

| Path | Role in DRC |
|------|-------------|
| `crates/altium-format-types/src/pcb.rs` | `RuleKind` enum (all 70 rule kinds) |
| `crates/autopcb-ir/src/rule.rs` | `IrDesignRule`, `IrRuleParams` (rule data from PcbDoc) |
| `crates/autopcb-ir/src/copper.rs` | `IrTrack`, `IrVia` (segment geometry) |
| `crates/autopcb-ir/src/net.rs` | `IrNet` (net class assignment, diff pair partner) |
| `crates/autopcb-ir/src/board.rs` | `IrBoardGeometry`, `IrKeepoutZone` (board edge, keepouts) |
| `crates/autopcb-ir/src/component.rs` | `IrComponentPad` (pad geometry for clearance) |
| `crates/autopcb-ir/src/polygon.rs` | `IrPolygon` (copper pour regions) |
| `crates/autopcb-ir/src/handles.rs` | `NetId`, `LayerId`, `RuleId` (typed handles) |
| `crates/autopcb-router/src/drc.rs` | Current stub (to be replaced) |
| `crates/autopcb-router/src/rules.rs` | `RoutingPolicy`, `build_policy` (rule resolution) |
| `crates/autopcb-router/src/workspace.rs` | `RoutingWorkspace` (R-tree, obstacle maps) |
| `crates/autopcb-router/src/spatial.rs` | `SpatialIndex` (R-tree wrapper) |
| `crates/autopcb-router/src/pathfinder/mod.rs` | PathFinder loop (DRC integration point) |
| `crates/autopcb-router/src/pathfinder/history.rs` | `HistoryArray` (DRC feeds into this) |
| `crates/autopcb-router/src/pathfinder/hot_set.rs` | `HotSet` (DRC violations add nets) |
| `crates/autopcb-router/src/solution.rs` | `RouteSolutionBuilder` (drc_violations count) |
| `crates/autopcb-routes/src/lib.rs` | `RoutingMetrics` (drc_violations field) |

### Related Plans

| Plan | Relationship |
|------|-------------|
| 01-corolla-bellman-ford.md | GPU routing engine that DRC integrates with |
| 02-gamer-sweep-routing.md | Alternative GPU routing algorithm |
| 03-xcheck-gpu-drc.md | GPU sweepline algorithm details (X-Check, OpenDRC, PDRC) |
| 04-cypress-congestion-feedback.md | Congestion feedback loop (DRC extends this) |
| 05-instantgr-net-batching.md | Net batching for GPU dispatch |
| 06-active-push-pull-routing.md | Interactive routing using shared DRC infrastructure |

### Documentation

| Doc | Relevant sections |
|-----|------------------|
| `docs/routing/active-router.md` | `TRuleKind` enum, rule interfaces, routing modes |
| `docs/routing/routing-rules-params-audit.md` | Parameter-level audit of routing rules |
| `docs/routing/routing-data-model.md` | C# rule interfaces, scope expressions, net topology |
| `docs/routing/rules6-audit.md` | EmptyRuleData audit, missing parameters |

### Research Papers

| Paper | Technique |
|-------|-----------|
| X-Check (ICCAD 2022) | Parallel sweepline via prefix computation |
| OpenDRC (DAC 2023) | Hierarchical GPU DRC acceleration |
| PDRC (DAC 2024) | Non-Manhattan segment handling |
| McMurchie & Ebeling (1995) | PathFinder negotiated congestion routing |

---

## 13. Open Questions

1. **Per-object-type clearance matrix**: Altium's `IPCB_ClearanceConstraint` supports
   a full object-type-by-object-type clearance matrix (Pad-Track, Via-Via, Track-Track,
   etc.) via `TClearanceConstraintMode.IsMatrix` and `OBJECTCLEARANCES`. Should we
   support this in the DRC engine from the start, or defer to Phase 3?

   **Recommendation**: Support only the simple `gap_mm` mode in Phase 1. Add matrix
   mode in Phase 3 after verifying how common it is in real designs.

2. **Scope expressions**: Altium design rules have `Scope1Expression` and
   `Scope2Expression` that determine which objects the rule applies to (e.g.,
   `"InNetClass('Power')"`, `"IsTrack"`). These are not currently in `IrDesignRule`.
   The DRC engine needs scope evaluation to apply rules correctly.

   **Recommendation**: Phase 1 uses global rules only (every rule applies to all
   objects). Phase 2 adds scope expression parsing and evaluation, building on the
   `altium-format-spec` query language if available.

3. **Polygon pour DRC**: Copper polygon pours (from `IrPolygon`) interact with
   clearance rules (polygon edges act as copper for DRC purposes). The polygon
   outline must be decomposed into segments and added to the clearance sweep.

   **Recommendation**: Phase 3. Treat polygon outlines as obstacle segments in
   the R-tree and sweepline.

4. **Via stacking**: Blind/buried/micro-via DRC requires knowledge of the layer
   stack and drill pairs. Currently `IrVia` has `from_layer` and `to_layer` but
   no `ViaType` classification.

   **Recommendation**: Add `via_type: ViaType` to `IrVia` in Phase 2. DRC checks
   via stacking legality (no two vias of the same type occupying the same drill
   span unless explicitly allowed).

5. **Incremental DRC**: Rather than rebuilding the R-tree and re-checking all
   segments every iteration, maintain a dirty-set of modified segments and only
   re-check those. This could significantly reduce DRC time in later PathFinder
   iterations when few nets change.

   **Recommendation**: Defer to Phase 3 (optimization). Full-rebuild DRC is
   simple and correct; incremental DRC is an optimization.
