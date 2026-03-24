# autopcb-placement

PCB component placement solver. Accepts a `PcbIr` (millimeter-based intermediate
representation) and a list of `UserConstraint`s, returns a `PlacementResult` with
component positions, rotation, HPWL estimate, and per-phase viewer snapshots.

## Architecture

The solver runs sequentially through up to five phases controlled by `PlacementConfig`:

```
Phase 1+2: Analytical (always)
  ConstraintSystem (solverang LM solver)
  Soft:  SmoothHpwlConstraint per net (log-sum-exp approximation, gamma_start)
  Hard:  BoardContainment, ComponentClearance, RotationDiscretize per component
  User:  EdgePlacement, Directional, Near, RegionContainment, FixedPosition
  → solve() → snap rotations to 90° → re-solve (gamma_end) → greedy overlap push

Phase 2.5: Greedy part swap (allow_part_swap = true)
  swap::greedy_part_swap_pass — exchanges positions of functionally identical components

Phase 3: Simulated annealing (sa_config = Some(...))
  simulated_annealing::refine_with_sa — Metropolis acceptance on Displace/Swap/Rotate moves
  Warm-starts from Phase 2 result; best-tracking guarantees HPWL never regresses

Phase 4.5: Greedy pin swap (allow_pin_swap = true)
  swap::greedy_pin_swap_sweep — reassigns nets to electrically equivalent pads
```

## Design Decisions

**Analytical solver first, SA second.** The solverang LM solver gives a legal, constraint-
satisfying starting placement cheaply. SA then refines it. This separation means MVP boards
(N < 50) finish in milliseconds without SA; SA is opt-in via `PlacementConfig::sa_config`.

**Smooth HPWL via log-sum-exp.** Exact HPWL (max − min) is non-differentiable. The
log-sum-exp approximation with parameter `gamma` provides a smooth surrogate. A two-pass
strategy solves at `gamma_start` (smooth, fast convergence) then re-solves at `gamma_end`
(sharper, closer to exact HPWL).

**Grid-based spatial index over R-tree.** At PCB scale (N < 500), a `HashMap<(i32,i32), Vec<usize>>`
spatial grid gives O(k) neighbor lookup with zero dependencies. R-tree complexity is only
justified at VLSI scale (N > 10K).

**Swap data sourced from PcbIr.** Pin/part swap group IDs (`swap_id_pin`, `swap_id_part`)
live on `IrComponentPad`. This avoids requiring SchLib as a separate solver input — the
PcbDoc already carries back-annotated swap groups from Altium's netlist import.

**Swap overlay file (not inline schematic edit).** Pin/part swaps change net-to-pin
mapping. `write_swap_overlay()` writes swaps to a separate `board-swaps.sch`
file imported by the main `.pcb`. The user can delete the import declaration
to undo all swaps atomically. Inline editing of the source spec would require tracking
each changed net individually to reverse — and would conflate user-authored constraints
with solver-generated net reassignments.

**SA auto-calibrates T₀.** Rather than requiring the user to tune initial temperature,
`refine_with_sa` samples 200 random moves at the initial placement and sets T₀ such that
`initial_acceptance` (default 80%) of uphill moves would be accepted. This makes `SAConfig`
portable across boards of different scales.

## Invariants

- A component with `UserConstraint::FixedPosition` is pinned — the solver adds a hard
  equality constraint, not just a strong soft term. The component cannot drift.
- After any swap pass, `verify_swap_integrity` must confirm: net count unchanged,
  per-net pin count unchanged. Pin/part swaps may only occur within the same swap group.
- `PlacementResult::hpwl_estimate_mm` is a lower bound (exact HPWL at pad centroids,
  ignoring pad size). It is recalculated after each phase so the final value reflects
  the post-swap state.
- `PlacementResult::snapshots` contains one entry per phase named `"initial"`,
  `"continuous"`, `"snapped"`, `"legalized"`, plus SA snapshots every
  `SAConfig::snapshot_interval` steps. The viewer depends on this ordering.
