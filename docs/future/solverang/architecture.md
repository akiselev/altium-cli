# Architecture: Solverang PCB Integration

## Pipeline Overview

```
┌─────────────────────────────────────────────────────────────────┐
│  Placement Spec (.pcbdoc-spec)                                  │
│  Written by LLM agent or human                                  │
│  "U1 center, J1 top edge, clearance 0.5mm, optimize ratsnest"  │
└──────────────────────────┬──────────────────────────────────────┘
                           ↓ parse (altium-format-spec parser)
┌─────────────────────────────────────────────────────────────────┐
│  PlacementModel (typed IR)                                      │
│  components: [{designator, region, edge, rotation_options}]     │
│  constraints: [{type, params, scope}]                           │
│  objectives: [{type: ratsnest, weight: 1.0}]                   │
└──────────────────────────┬──────────────────────────────────────┘
                           ↓ + PcbDoc (read board outline, netlist, footprint BBs)
┌─────────────────────────────────────────────────────────────────┐
│  SolverInput                                                    │
│  - Board outline polygon (fixed)                                │
│  - Component bounding boxes (from footprint data)               │
│  - Netlist (from Nets6 + component pad assignments)             │
│  - Design rules (from Rules6/NewRules6)                         │
│  - User constraints (from spec)                                 │
│  - Optimization objectives (from spec)                          │
└──────────────────────────┬──────────────────────────────────────┘
                           ↓ compile to solverang Problem
┌─────────────────────────────────────────────────────────────────┐
│  Solverang ConstraintSystem                                     │
│  Entities:                                                      │
│    PcbComponent(x, y, θ) per component                         │
│  Constraints:                                                   │
│    BoardContainment (hard)                                       │
│    ComponentClearance (hard, pairwise)                           │
│    BoardEdgeClearance (hard)                                     │
│    EdgeAlignment (hard, per edge-placed component)              │
│    RegionContainment (hard, per region-placed component)        │
│    DirectionalOrdering (hard, leftOf/rightOf/above/below)       │
│    NearConstraint (hard)                                        │
│    GroupSeparation (hard)                                        │
│  Objectives:                                                    │
│    SmoothHPWL per net (soft, weighted)                          │
│    ThermalGrouping (soft, weighted)                              │
└──────────────────────────┬──────────────────────────────────────┘
                           ↓ ConstraintSystem::solve() → SystemResult
┌─────────────────────────────────────────────────────────────────┐
│  SystemResult                                                   │
│  status: Solved | PartiallySolved | DiagnosticFailure           │
│  clusters: Vec<ClusterResult> (per independent sub-problem)     │
│  total_iterations, duration                                     │
│  → Extract final (x, y, rotation) from ParamStore               │
│  → Compute metrics: total_hpwl, max_violation                   │
└──────────────────────────┬──────────────────────────────────────┘
                           ↓ emit ECO or apply
┌─────────────────────────────────────────────────────────────────┐
│  ECO (plan mode) or PcbDoc mutation (apply mode)                │
│  ~ MOVE U1 from (100mm, 50mm) → (50mm, 40mm) rotation 0→90    │
│  ~ MOVE J1 from (10mm, 10mm) → (50mm, 78mm) rotation 0→0      │
│  ...                                                            │
└─────────────────────────────────────────────────────────────────┘
```


## Crate Architecture

```
solverang (external, ~/git/solverang/)
  ├── Core: Problem trait, LMSolver, AutoSolver, RobustSolver, ParallelSolver, SparseSolver
  ├── V3 system: ConstraintSystem, Entity/Constraint traits, ParamStore (generational IDs)
  ├── Plugins: sketch2d, sketch3d, assembly (domain-specific entity+constraint libraries)
  ├── Pipeline: 5-phase solve (Decompose → Analyze → Reduce → Solve → PostProcess)
  ├── Macros: #[auto_jacobian] for automatic Jacobian derivation from residual expressions
  └── solverang-pcb (new crate in solverang workspace)
      ├── entities.rs     — PcbComponent, PcbPad, PcbVia, PcbBoardOutline
      ├── constraints.rs  — PCB-specific constraint implementations
      ├── objectives.rs   — HPWL, thermal grouping (soft constraints)
      ├── builder.rs      — Ergonomic API (modeled after Sketch2DBuilder pattern)
      └── drc.rs          — Design rule → constraint mapping

altium-format-spec (this workspace)
  └── src/
      ├── placement.rs    — PlacementModel, PlacementSpec types
      ├── placement_compiler.rs — Compile spec AST → PlacementModel
      └── placement_solver.rs   — PlacementModel + PcbDoc → solverang Problem → Solution
```


## Two Modes of Operation

### Mode 1: Placement (Optimization)

**Goal**: Find optimal (x, y, θ) for each component.

Variables are component positions. The solver iterates from an initial guess
(center of board, random, or current positions) to find positions that:
1. Satisfy all hard constraints (board containment, clearance, user constraints)
2. Minimize soft objectives (wire length)

**Solver choice**: `AutoSolver` (automatically selects `LMSolver` for over-determined
systems, `Solver` for square systems). LM is robust to poor initial guesses and
handles over-determined systems where we have more constraints than variables.
`RobustSolver` tries Newton-Raphson first and falls back to LM on failure.

### Mode 2: DRC Checking (Verification)

**Goal**: Verify that an existing placement satisfies all design rules.

All component positions are **fixed** (not solvable). Design rules become
residual evaluations — if any residual is non-zero, the rule is violated.

This is NOT a solve — it's a single residual evaluation. But formulating DRC
as solverang constraints lets us:
1. Reuse the same constraint code for both placement and verification
2. Get exact violation distances (the residual value = how far off we are)
3. Identify which constraint is violated and by how much
4. Potentially "fix" violations by unfixing the offending component and re-solving

**DRC output format**:
```
DRC REPORT
  PASS  Clearance: U1–U2 gap=2.3mm (min 0.5mm)
  PASS  BoardOutline: U1 edge distance=3.1mm (min 1.0mm)
  FAIL  Clearance: C3–C4 gap=0.3mm (min 0.5mm, violation=-0.2mm)
  FAIL  BoardOutline: J2 edge distance=0.5mm (min 1.0mm, violation=-0.5mm)
  ────
  34 rules checked, 2 violations found
```


## The Rotation Problem

Component rotation is **discrete** (0°, 90°, 180°, 270° for most components,
some connectors are fixed-orientation). This is incompatible with continuous
optimization.

### Strategy: Branch-and-Bound with Warm Start

1. **User specifies allowed rotations** in spec: `rotation: 0 | 90 | 180 | 270`
2. **For N components with discrete rotation choices**, enumerate combinations
3. **For each combination**, solve the continuous placement problem (just x, y)
4. **Pick the combination with lowest objective** (HPWL)

Complexity: For 20 components with 4 rotation options each, that's 4^20 ≈ 10^12
combinations — too many. Mitigations:

- Most components have **constrained rotations** (connectors: 1 option, passives: 2, ICs: 4)
- **Greedy rotation assignment**: Fix rotations one at a time, choosing the best
- **Simulated annealing on rotations**: Random rotation flips with acceptance probability
- **Two-phase**: Solve continuous relaxation first, then snap rotations and re-solve positions

For the initial implementation, **user-specified rotations** (option 1) is simplest
and most LLM-friendly — the agent knows which way connectors face.


## Data Flow: PcbDoc → Solver Input

```rust
/// Extract solver-relevant data from a PcbDoc
fn extract_placement_data(doc: &PcbDoc) -> PlacementData {
    PlacementData {
        // From Board6: board outline polygon
        board_outline: extract_board_outline(&doc.board6),

        // From Components6: current positions + footprint references
        components: doc.components6.iter().map(|c| ComponentData {
            designator: c.get("SOURCEDESIGNATOR"),
            x: c.get_coord("X"),
            y: c.get_coord("Y"),
            rotation: c.get_float("ROTATION"),
            pattern: c.get("PATTERN"),
            // Bounding box from footprint library or computed from child primitives
            bounding_box: compute_bounding_box(c, &doc.pads6),
        }).collect(),

        // From Nets6 + primitive common headers: netlist
        nets: extract_netlist(&doc.nets6, &doc.pads6, &doc.vias6),

        // From Rules6/NewRules6: design rules
        rules: extract_design_rules(&doc.rules6),
    }
}
```


## Performance Targets

| Scenario | Components | Constraints | Target Solve Time |
|----------|-----------|-------------|-------------------|
| Small board (Arduino-like) | 10-20 | ~100 | <10ms |
| Medium board (STM32 dev board) | 50-100 | ~1000 | <100ms |
| Large board (complex product) | 200-500 | ~5000 | <1s |
| DRC check only | any | any | <1ms (single eval) |

Solverang's parallel decomposition (`ParallelSolver` via `rayon`) + sparse matrices
(`SparseSolver` via `faer`) should handle these sizes easily. JIT compilation
(`jit` feature via Cranelift) is available for hot constraint evaluation if needed.

The 5-phase solve pipeline (Decompose → Analyze → Reduce → Solve → PostProcess)
automatically decomposes the problem into independent clusters and solves them
in parallel when possible. `DofAnalysis` provides diagnostic output identifying
under-constrained or over-constrained entities.


## Dependencies

```toml
# In altium-format-spec/Cargo.toml
[dependencies]
solverang = { path = "../../solverang/crates/solverang", features = ["sparse", "parallel", "macros"] }
# Note: "geometry" feature is removed from solverang. PCB-specific geometry
# (bounding boxes, clearance zones) lives in constraint residual functions,
# not in solverang's core. The "macros" feature enables #[auto_jacobian] for
# automatic Jacobian derivation from residual expressions.

# Alternatively, create a bridge crate:
# altium-placement = { path = "../altium-placement" }
```

**Open question**: Should solverang-pcb live in the solverang workspace (domain-agnostic
solver with PCB plugin) or in the altium-cli workspace (Altium-specific placement)?

**Recommendation**: `solverang-pcb` in the solverang workspace (reusable for other
EDA tools), with a thin bridge in `altium-format-spec` that maps Altium types to
solverang-pcb types. The `Sketch2DBuilder` pattern in solverang provides a good
template for the `PcbPlacementBuilder` API.
