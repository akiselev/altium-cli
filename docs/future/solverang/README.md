# Solverang Integration: PCB Placement, Routing & DRC

Design notes for integrating the [solverang](~/git/solverang/) nonlinear constraint
solver with altium-cli for automated PCB component placement, autorouting, and design
rule checking.

## Implementation Plan

**Start here:** [implementation-plan.md](implementation-plan.md) — unified 11-phase roadmap
with dependency graph, from IR crate through GPU acceleration.

## Status Note

These documents are a hybrid:
- a **wishlist / research notebook** (algorithm options and future ideas), and
- a **first-pass implementation plan**.

Where current code differs from an older checklist, treat the codebase as source of
truth and update plan checkboxes accordingly.

## Design Documents

### Foundation
| Document | Description |
|----------|-------------|
| [implementation-plan.md](implementation-plan.md) | **Unified roadmap** — all phases, dependencies, viewer features |
| [ir.md](ir.md) | `altium-format-ir` crate: domain-semantic intermediate representation |

### Placement
| Document | Description |
|----------|-------------|
| [architecture.md](architecture.md) | Overall pipeline: spec → solver → PcbDoc |
| [constraint-types.md](constraint-types.md) | PCB-specific solverang constraint type designs |
| [rotation-and-ratsnest.md](rotation-and-ratsnest.md) | Deep-dive: discrete rotation + HPWL optimization math |
| [placement-algorithms.md](placement-algorithms.md) | Survey of SA, analytical, RL, diffusion + multi-stage pipeline |
| [schdoc-placement.md](schdoc-placement.md) | Schematic auto-layout: Sugiyama + orthogonal wire routing |

### Autorouting
| Document | Description |
|----------|-------------|
| [autorouter.md](autorouter.md) | PCB autorouter: A*, PathFinder, Steiner trees, trace optimization |

### Infrastructure
| Document | Description |
|----------|-------------|
| [design-rules-mapping.md](design-rules-mapping.md) | All 70 Altium design rules mapped to solver categories |
| [spec-grammar.md](spec-grammar.md) | Spec language grammar extensions for placement + DRC |
| [llm-constraint-generation.md](llm-constraint-generation.md) | Phase 0: LLM agent reads PcbDoc + datasheets → intelligent constraints |

## Key Insight

Solverang is a **nonlinear least-squares solver** with multiple backends (Newton-Raphson
via `Solver`, Levenberg-Marquardt via `LMSolver`, auto-selection via `AutoSolver`,
fallback via `RobustSolver`). Its v3 `ConstraintSystem` architecture provides entity/
constraint/param management with generational IDs, automatic cluster decomposition,
and a 5-phase solve pipeline. This means we can naturally blend:

- **Hard constraints** (clearance, board containment) → residuals that must be zero
- **Soft objectives** (minimize wire length, thermal grouping) → weighted residuals

LM minimizes `||r(x)||²`. Hard constraints dominate the residual norm, so the solver
won't sacrifice feasibility for a slight improvement in wire length. This is exactly
the behavior we want for PCB placement.

## Use Cases

### 1. LLM-Driven Placement (Primary)

An LLM agent writes a rough placement spec:
```
placement {
    place U1 { region: center }
    place J1 { edge: top, inset: 10mm }
    left_of $Y1, $U1 { gap: 2mm }
    clearance { all: 0.5mm }
    optimize { ratsnest: true }
}
```

The solver finds optimal (x, y, rotation) for each component.

### 2. Design Rule Checking (DRC)

Verify an existing PcbDoc against Altium design rules by formulating rules as
constraint residuals and checking if all are satisfied (residual ≈ 0).

### 3. Interactive Constraint Editing

User adjusts constraints → solver re-solves in real-time (~5ms for typical boards).

## Prerequisites

- PcbDoc read support: **exists** (all sections parse)
- PcbDoc write support: **exists** (high-level board read/write API available)
- Solverang PCB constraints: **need to be built**
- Spec language PcbDoc support: **exists** (compile/plan/apply/dump supported)
