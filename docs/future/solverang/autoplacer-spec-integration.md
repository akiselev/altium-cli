# Autoplacer ↔ Spec Language Integration Design

The autoplacer operates on `.pcbdoc-spec` files, not PcbDoc binaries directly.
This document defines the spec syntax extensions and the autoplacer's workflow.


## 1. Core Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│  USER writes partial .pcbdoc-spec                                │
│  (locked components, groups, constraints, autoplace directives) │
└──────────────────────────────┬───────────────────────────────────┘
                               ▼
┌──────────────────────────────────────────────────────────────────┐
│  AUTOPLACER reads spec + PcbDoc netlist                          │
│  1. Parse spec → locked positions + constraints + autoplace set  │
│  2. Extract netlist from PcbDoc (connectivity, board outline)    │
│  3. Auto-cluster unplaced components (Phase 0)                   │
│  4. Run analytical solver (Phase 1+2) with constraints           │
│  5. Optionally refine with SA (Phase 3)                          │
│  6. Final refinement (Phase 4)                                   │
└──────────────────────────────┬───────────────────────────────────┘
                               ▼
┌──────────────────────────────────────────────────────────────────┐
│  AUTOPLACER writes UPDATED .pcbdoc-spec                          │
│  Replaces `autoplace: true` with explicit `at: (x, y)` + rotation│
│  Preserves all user-written constraints and locked components    │
│  Result is a complete, human-readable placement specification    │
└──────────────────────────────┬───────────────────────────────────┘
                               ▼
┌──────────────────────────────────────────────────────────────────┐
│  `altium placement apply board.pcbdoc-spec`                      │
│  Reconciler reads spec, writes component positions to PcbDoc     │
└──────────────────────────────────────────────────────────────────┘
```

**Key insight**: The autoplacer is a **spec-to-spec transformer**. It reads a
partial spec with `autoplace: true` directives and produces a complete spec
with explicit positions. The existing reconciler pipeline then applies the
spec to the PcbDoc binary.


## 2. New Spec Syntax: Autoplacer Directives

### 2.1 `autoplace` Property on `place` Blocks

Mark a component (or group of components) for automatic placement:

```
place U1 {
    autoplace: true
}
```

The autoplacer replaces this with:

```
place U1 {
    at: (48.2mm, 39.5mm)
    rotation: 90
    // autoplace: solved by altium-cli autoplacer v0.1.0
}
```

### 2.2 `autoplace` with Partial Constraints

Users can constrain the autoplacer while still requesting auto-placement:

```
// "Place U1 somewhere in the center region, I don't care exactly where"
place U1 {
    autoplace: true
    region: center
    rotation: 0 | 90
}

// "Place these caps near the MCU, figure out exact positions"
place C1, C2, C3, C4 {
    autoplace: true
    near: $U1
    max_distance: 5mm
}

// "Put connectors on the top edge, autoplacer decides spacing"
place J1, J2, J3 {
    autoplace: true
    edge: top
    inset: 2mm
}
```

### 2.3 Locked Components (No `autoplace`)

Components WITHOUT `autoplace: true` that have explicit positions are **locked**:

```
// This component is FIXED — autoplacer must not move it
place J4 {
    at: (2mm, 10mm)
    rotation: 270
}
```

The autoplacer treats locked components as `FixedPositionConstraint` inputs.

Components with `fixed: true` are also locked:

```
place U5 {
    fixed: true
    at: (50mm, 50mm)
    rotation: 0
}
```

### 2.4 Implicit Autoplace: Unmentioned Components

Components that exist in the PcbDoc but are NOT mentioned in the spec at all
have three possible behaviors, controlled by a top-level property:

```
placement {
    // What to do with components not mentioned in this spec:
    unplaced: autoplace    // DEFAULT: auto-place them
    unplaced: ignore       // Leave at their current PcbDoc positions (treat as fixed)
    unplaced: error        // Error if any component is not mentioned
}
```

### 2.5 `autoplace` Block (Global Autoplacer Configuration)

Top-level configuration for the autoplacer algorithm:

```
placement {
    autoplace {
        // Algorithm selection
        algorithm: analytical           // analytical | sa | full_pipeline

        // SA parameters (only if algorithm includes SA)
        sa_cooling: 0.95
        sa_moves_per_temp: 10          // multiplied by N components
        sa_max_steps: 500

        // Optimization
        enable_net_crossings: false     // expensive; off by default
        ratsnest_weight: 0.01

        // Clearance
        default_clearance: 0.5mm
        board_edge_clearance: 1mm

        // Grid snapping
        grid_snap: 0.5mm               // snap final positions to grid

        // Clustering (Phase 0)
        auto_cluster: true              // auto-detect component groups
        cluster_algorithm: bfs          // bfs | spectral
    }
}
```

### 2.6 `autoplace_group` — Auto-Place a Group Together

```
group power_supply { components: [U2, U3, L1, C10, C11, C12] }

place $power_supply {
    autoplace: true
    near: $J4
    max_distance: 20mm
    keep_together: true
    max_spread: 15mm
}
```


## 3. Autoplacer Output Format

The autoplacer rewrites the `.pcbdoc-spec` file, preserving structure and comments.

### 3.1 What Changes

| Input | Output |
|-------|--------|
| `autoplace: true` | Replaced with `at: (x, y)` + `rotation: N` |
| `autoplace: true` + constraints | Constraints preserved, `at:` + `rotation:` added |
| Locked `at: (x, y)` | Preserved unchanged |
| Comments | Preserved |
| Groups, constraints, rules | Preserved unchanged |

### 3.2 Example Transformation

**Input** (user-written):
```
placement {
    target: "my-board.PcbDoc"
    unplaced: autoplace

    autoplace {
        algorithm: full_pipeline
        grid_snap: 0.5mm
    }

    // Connectors are locked
    place J1 {
        at: (50mm, 78mm)
        rotation: 0
    }

    place J4 {
        at: (2mm, 10mm)
        rotation: 270
    }

    // MCU: autoplace in center
    place U1 {
        autoplace: true
        region: center
        rotation: 0 | 90
    }

    // Decoupling caps: near MCU
    place C1, C2, C3, C4 {
        autoplace: true
        near: $U1
        max_distance: 5mm
    }

    // Power section near barrel jack
    group power { components: [U2, U3, L1, C10, C11] }
    place $power {
        autoplace: true
        near: $J4
        max_distance: 15mm
    }

    clearance { all: 0.5mm, edge: 1mm }

    optimize {
        ratsnest: true
        ratsnest_weight: 1.0
    }
}
```

**Output** (autoplacer-generated):
```
placement {
    target: "my-board.PcbDoc"
    unplaced: autoplace

    autoplace {
        algorithm: full_pipeline
        grid_snap: 0.5mm
    }

    // Connectors are locked
    place J1 {
        at: (50mm, 78mm)
        rotation: 0
    }

    place J4 {
        at: (2mm, 10mm)
        rotation: 270
    }

    // MCU: autoplace in center
    place U1 {
        at: (48.5mm, 39.5mm)
        rotation: 90
        region: center
        // autoplace: solved (HPWL contribution: 142mm)
    }

    // Decoupling caps: near MCU
    place C1 {
        at: (45.0mm, 42.0mm)
        rotation: 0
        near: $U1
        max_distance: 5mm
        // autoplace: solved
    }
    place C2 {
        at: (52.0mm, 42.0mm)
        rotation: 0
        near: $U1
        max_distance: 5mm
        // autoplace: solved
    }
    place C3 {
        at: (45.0mm, 37.0mm)
        rotation: 0
        near: $U1
        max_distance: 5mm
        // autoplace: solved
    }
    place C4 {
        at: (52.0mm, 37.0mm)
        rotation: 0
        near: $U1
        max_distance: 5mm
        // autoplace: solved
    }

    // Power section near barrel jack
    group power { components: [U2, U3, L1, C10, C11] }
    place U2 {
        at: (8.5mm, 14.0mm)
        rotation: 0
        // autoplace: solved (group: power)
    }
    place U3 {
        at: (8.5mm, 20.0mm)
        rotation: 0
        // autoplace: solved (group: power)
    }
    place L1 {
        at: (5.0mm, 17.0mm)
        rotation: 90
        // autoplace: solved (group: power)
    }
    place C10 {
        at: (12.0mm, 14.0mm)
        rotation: 0
        // autoplace: solved (group: power)
    }
    place C11 {
        at: (12.0mm, 20.0mm)
        rotation: 0
        // autoplace: solved (group: power)
    }

    // Auto-placed (not in original spec, from unplaced: autoplace)
    place R1 {
        at: (30.0mm, 25.0mm)
        rotation: 0
        // autoplace: solved (unmentioned component)
    }
    place R2 {
        at: (32.0mm, 25.0mm)
        rotation: 0
        // autoplace: solved (unmentioned component)
    }
    // ... remaining auto-placed components ...

    clearance { all: 0.5mm, edge: 1mm }

    optimize {
        ratsnest: true
        ratsnest_weight: 1.0
    }
}
```


## 4. Iterative Refinement Workflow

The spec-based approach enables a natural iterative workflow:

```
Step 1: User writes minimal spec
   placement {
       place J1 { at: (50mm, 78mm), rotation: 0 }
       place U1 { autoplace: true, region: center }
       // everything else: autoplace by default
   }

Step 2: Run autoplacer
   $ altium placement autoplace board.pcbdoc-spec

Step 3: User reviews output spec, adjusts
   // "I don't like where R5 ended up, move it near U1"
   place R5 {
       autoplace: true          // still auto, but constrained
       near: $U1
       max_distance: 3mm
   }

Step 4: Re-run autoplacer (only re-places autoplace:true components)
   $ altium placement autoplace board.pcbdoc-spec

Step 5: User locks in final positions
   // Change autoplace comments to explicit positions
   // Remove `autoplace: true` to lock components

Step 6: Apply to PcbDoc
   $ altium placement apply board.pcbdoc-spec
```


## 5. CLI Commands

```bash
# Run autoplacer on spec file (writes updated spec)
altium placement autoplace board.pcbdoc-spec
altium placement autoplace board.pcbdoc-spec --target my-board.PcbDoc
altium placement autoplace board.pcbdoc-spec --dry-run    # show plan only
altium placement autoplace board.pcbdoc-spec --output board-placed.pcbdoc-spec

# Plan: show what would change without writing
altium placement plan board.pcbdoc-spec

# Apply: write resolved spec positions to PcbDoc binary
altium placement apply board.pcbdoc-spec

# Dump: extract current PcbDoc positions as spec file
altium placement dump my-board.PcbDoc
altium placement dump my-board.PcbDoc --output board.pcbdoc-spec

# DRC
altium drc my-board.PcbDoc --rules board.pcbdoc-spec
```


## 6. Spec Rewriting Rules

The autoplacer must preserve spec file structure when rewriting.

### 6.1 Preservation Rules

1. **Comments**: ALL comments are preserved in their original positions
2. **Locked components**: `place` blocks without `autoplace: true` are never modified
3. **Groups**: Group declarations are preserved unchanged
4. **Constraints**: All relational constraints are preserved
5. **Rules**: Design rule blocks are preserved
6. **Optimize/clearance**: Configuration blocks are preserved
7. **Whitespace**: Indentation style is preserved (detect from existing file)

### 6.2 Modification Rules

1. **`autoplace: true` → `at: (x, y)` + `rotation: N`**: The `autoplace: true` line
   is replaced. A comment `// autoplace: solved` is added for traceability.
2. **Multi-designator `place` blocks are expanded**: `place C1, C2, C3 { autoplace: true }`
   becomes individual `place C1 { at: ... }`, `place C2 { at: ... }`, etc.
   The original constraints (near, max_distance) are duplicated to each.
3. **Unmentioned components**: New `place` blocks are appended at the end of the
   `placement` block, grouped under a comment `// Auto-placed (unmentioned)`.


## 7. Component Classification for Auto-Placement

When `unplaced: autoplace` is set, the autoplacer classifies unmentioned components:

| Classification | Heuristic | Default Behavior |
|---|---|---|
| **Connector** | Footprint starts with "J", "CON", "HEADER", or has `>4` mechanical pads | Place on nearest board edge |
| **Decoupling cap** | C + small value + on same net as IC power pin | Place near associated IC |
| **Pull-up/down resistor** | R + on net with single IC connection | Place near associated IC |
| **Bulk capacitor** | C + large value (>1µF) on power net | Place near power input |
| **IC/active** | U, Q, D prefix with many pins | Place in center/available region |
| **Passive (other)** | R, L, C not classified above | Place near connected components |
| **Test point** | TP prefix | Place at board periphery |
| **Mechanical** | MH prefix, mounting holes | Fixed at current position |

These heuristics generate implicit constraints fed to the solver.


## 8. Implementation Plan

### Phase A: Spec Parsing (prerequisite)
- Parse `autoplace: true` property in `place` blocks
- Parse `autoplace { ... }` configuration block
- Parse `unplaced:` property
- Add to `PlaceSpec`: `autoplace: bool`, `AutoplaceConfig` struct

### Phase B: Autoplacer Core
- Read spec → partition components into locked vs autoplace sets
- Build `UserConstraint` vec from spec (existing logic)
- Add `FixedPositionConstraint` for locked components
- Run existing `solve_placement()` for autoplace components
- Produce `PlacementResult`

### Phase C: Spec Rewriting
- Round-trip spec file with comment/whitespace preservation
- Replace `autoplace: true` with computed positions
- Expand multi-designator blocks
- Append unmentioned component placements

### Phase D: SA Integration (Phase 3)
- Add `algorithm: full_pipeline` support
- Wire SA config from `autoplace { ... }` block
- Run Phase 3 after Phase 1+2

### Phase E: Auto-Clustering (Phase 0)
- When `auto_cluster: true`, run Phase 0 before solving
- Generate implicit `NearConstraint` groups from netlist analysis
- Classify unmentioned components (Section 7 heuristics)
