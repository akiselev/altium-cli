# Spec Language Grammar Extensions for PcbDoc Placement & DRC

Extends the existing spec language (`.sym`, `.sym`) with a new
`.pcb` domain for board-level placement constraints and design rule
specifications.


## 1. New File Extension

| Extension | Domain | Output file |
|-----------|--------|-------------|
| `.pcb` | PcbDoc | `.PcbDoc` (same base name) |

The `.pcb` file declares:
- Component placement constraints
- Design rule overrides
- Optimization objectives
- Board geometry references


## 2. Design Principles

### Agent-Friendly Syntax

The placement spec is designed for LLM agents to generate. Key properties:

1. **Natural language concepts**: `edge: top`, `near: $U1`, `left_of`, `region: center`
2. **Minimal boilerplate**: Only specify what you care about; everything else is default
3. **Forgiving**: Constraints can be approximate — the solver finds exact positions
4. **Readable error messages**: "U1 cannot be placed in center: board too small for clearance"

### Relationship to Existing Spec Language

- Shares the same lexer, expression evaluator, and scope system
- Adds new keywords: `placement`, `place`, `rule`, `group`, `near`, `left_of`,
  `right_of`, `above`, `below`, `separate`, `optimize`, `board`
- Reuses: `let`, `import`, `{...}`, coord tuples, dimensional values, spread


## 3. Top-Level Grammar

```ebnf
(* ================================================================ *)
(* PcbDoc spec file                                                  *)
(* ================================================================ *)

pcbdoc_spec     = { pcbdoc_item [";"] } ;

pcbdoc_item     = import_decl                (* existing *)
                | let_binding                (* existing *)
                | placement_decl
                | rule_decl ;

(* ================================================================ *)
(* Placement block                                                   *)
(* ================================================================ *)

placement_decl  = "placement" "{" { placement_item [sep] } "}" ;

placement_item  = let_binding
                | property                    (* target, default_clearance, etc. *)
                | place_decl
                | group_decl
                | constraint_decl
                | optimize_decl ;

(* ================================================================ *)
(* Component placement                                               *)
(* ================================================================ *)

place_decl      = "place" designator_list object ;

designator_list = entity_name { "," entity_name } ;

(* Properties inside place { ... }:
   region:    center | top_half | bottom_half | left_half | right_half
              | quadrant_tl | quadrant_tr | quadrant_bl | quadrant_br
              | rectangle { from: (x, y), to: (x, y) }
   edge:     top | bottom | left | right
   inset:    Dim (distance from edge)
   align:    start | center | end (position along edge)
   bias:     top | bottom | left | right (preference within region)
   near:     $ref (entity reference)
   max_distance: Dim (for near constraint)
   rotation: Int { "|" Int }  (allowed rotations, e.g., 0 | 90 | 180 | 270)
   fixed:    true | false (lock position, don't solve)
   at:       Coord (explicit position, implies fixed)
   side:     top | bottom (board side for SMD)
*)

(* ================================================================ *)
(* Relational constraints                                            *)
(* ================================================================ *)

constraint_decl = directional_constraint
                | clearance_constraint
                | separate_constraint
                | distance_constraint ;

directional_constraint = ("left_of" | "right_of" | "above" | "below")
                         dollar_path "," dollar_path [object] ;

(* left_of $Y1, $U1 { gap: 2mm }
   → Y1 must be to the left of U1 with ≥ 2mm gap *)

clearance_constraint = "clearance" object ;

(* clearance { all: 0.5mm, connectors: 2mm, edge: 1mm }
   → default clearance, per-class clearance, board edge clearance *)

separate_constraint = "separate" dollar_path "," dollar_path [object] ;

(* separate $analog, $digital { gap: 10mm }
   → minimum gap between group convex hulls *)

distance_constraint = "distance" dollar_path "," dollar_path object ;

(* distance $U1, $U2 { min: 5mm, max: 20mm }
   → constrain center-to-center distance *)

(* ================================================================ *)
(* Component groups                                                  *)
(* ================================================================ *)

group_decl      = "group" entity_name object ;

(* group analog { components: [U5, R10, R11, C20, C21] }
   → named group for separation/clustering constraints *)

(* Properties inside group { ... }:
   components: Array of designators
   keep_together: true | false (default true — cluster group members)
   max_spread: Dim (maximum diameter of group's bounding circle)
*)

(* ================================================================ *)
(* Optimization objectives                                           *)
(* ================================================================ *)

optimize_decl   = "optimize" object ;

(* optimize {
       ratsnest: true
       ratsnest_weight: 1.0
       thermal: true
       thermal_components: [U2, Q1, Q2]
   } *)

(* Properties inside optimize { ... }:
   ratsnest:    Bool (minimize HPWL, default true)
   ratsnest_weight: Float (relative weight, default 1.0)
   thermal:     Bool (distribute heat sources, default false)
   thermal_components: Array (components to spread apart)
   thermal_weight: Float (relative weight, default 0.5)
*)

(* ================================================================ *)
(* Design rules (DRC specification)                                  *)
(* ================================================================ *)

rule_decl       = "rule" entity_name object ;

(* rule Clearance_Default {
       kind: clearance
       gap: 0.254mm
       scope1: all
       scope2: all
       net_scope: different_nets
       layer_scope: same_layer
   }

   rule Width_Signal {
       kind: width
       min: 0.127mm
       max: 0.5mm
       preferred: 0.254mm
       scope1: InNetClass("Signal")
   }

   rule ComponentClearance_Default {
       kind: component_clearance
       gap: 0.5mm
       scope1: all
       scope2: all
   }

   rule BoardOutline {
       kind: board_outline_clearance
       gap: 1mm
   }

   rule HoleSpacing {
       kind: hole_to_hole_clearance
       gap: 0.254mm
   }

   rule AnnularRing {
       kind: minimum_annular_ring
       min: 0.127mm
   }
*)

(* Properties inside rule { ... }:
   kind:        Enum (maps to TRuleKind)
   gap:         Dim (for clearance rules)
   min:         Dim (minimum bound)
   max:         Dim (maximum bound)
   preferred:   Dim (preferred value)
   expansion:   Dim (for mask expansion rules)
   scope1:      Expr (scope query expression, or 'all')
   scope2:      Expr (scope query expression, or 'all')
   net_scope:   Enum (any_net, different_nets, same_net)
   layer_scope: Enum (any_layer, same_layer, adjacent_layers)
   enabled:     Bool (default true)
   priority:    Int (lower = higher priority)
*)
```


## 4. Complete Example: LLM-Generated Placement Spec

```
// board-layout.pcb
// Generated by LLM agent for STM32F4 development board

placement {
    // Target board
    target: "my-board.PcbDoc"

    // ── MCU (center of board) ──────────────────────────
    place U1 {
        region: center
        rotation: 0 | 90
    }

    // ── Connectors (board edges) ───────────────────────
    place J1 {                          // HDMI
        edge: top
        inset: 2mm
        align: center
        rotation: 0
    }

    place J2 {                          // USB-C
        edge: left
        inset: 2mm
        bias: top
        rotation: 270
    }

    place J3 {                          // SD Card
        edge: right
        inset: 2mm
        align: center
        rotation: 90
    }

    place J4 {                          // Power barrel jack
        edge: left
        inset: 2mm
        bias: bottom
        rotation: 270
    }

    // ── Power section (near barrel jack) ───────────────
    place U2, U3 {                      // Voltage regulators
        near: $J4
        max_distance: 15mm
        rotation: 0
    }

    // ── Decoupling caps (close to MCU) ─────────────────
    place C1, C2, C3, C4 {
        near: $U1
        max_distance: 5mm
    }

    // ── Crystal (left of MCU) ──────────────────────────
    left_of $Y1, $U1 { gap: 2mm }

    // ── Debug header (bottom edge) ─────────────────────
    place J5 {                          // SWD header
        edge: bottom
        inset: 3mm
        align: end
        rotation: 0
    }

    // ── LED indicator cluster ──────────────────────────
    group leds { components: [D1, D2, D3, D4] }
    place D1, D2, D3, D4 {
        edge: top
        inset: 8mm
        bias: right
    }

    // ── Analog section separation ──────────────────────
    group analog { components: [U5, R10, R11, R12, C20, C21, C22] }
    group digital { components: [U1, U2, U3, U6] }
    separate $analog, $digital { gap: 8mm }

    // ── Clearance rules ────────────────────────────────
    clearance {
        all: 0.5mm
        connectors: 2mm
        edge: 1mm
    }

    // ── Optimization ───────────────────────────────────
    optimize {
        ratsnest: true
        ratsnest_weight: 1.0
        thermal: true
        thermal_components: [U2, U3]
        thermal_weight: 0.5
    }
}

// ── Design rules (optional, override board defaults) ──

rule Clearance_Default {
    kind: clearance
    gap: 0.254mm
    scope1: all
    scope2: all
    net_scope: different_nets
}

rule Width_Power {
    kind: width
    min: 0.3mm
    max: 1mm
    preferred: 0.5mm
    scope1: InNetClass("Power")
}

rule BoardOutline {
    kind: board_outline_clearance
    gap: 1mm
}
```


## 5. Rule Kind Enum Values (for `kind:` field)

```
clearance                    → TRuleKind 0
parallel_segment             → TRuleKind 1
width                        → TRuleKind 2
length                       → TRuleKind 3
matched_lengths              → TRuleKind 4
stub_length                  → TRuleKind 5
plane_connect                → TRuleKind 6
routing_topology             → TRuleKind 7
routing_priority             → TRuleKind 8
routing_layers               → TRuleKind 9
routing_corners              → TRuleKind 10
routing_vias                 → TRuleKind 11
plane_clearance              → TRuleKind 12
solder_mask_expansion        → TRuleKind 13
paste_mask_expansion         → TRuleKind 14
short_circuit                → TRuleKind 15
unrouted_net                 → TRuleKind 16
vias_under_smd               → TRuleKind 17
max_via_count                → TRuleKind 18
minimum_annular_ring         → TRuleKind 19
polygon_connect              → TRuleKind 20
acute_angle                  → TRuleKind 21
room_definition              → TRuleKind 22
smd_to_corner                → TRuleKind 23
component_clearance          → TRuleKind 24
component_orientations       → TRuleKind 25
permitted_layers             → TRuleKind 26
nets_to_ignore               → TRuleKind 27
hole_size                    → TRuleKind 42
height                       → TRuleKind 50
diff_pairs_routing           → TRuleKind 51
hole_to_hole_clearance       → TRuleKind 52
minimum_solder_mask_sliver   → TRuleKind 53
silk_to_solder_mask_clearance → TRuleKind 54
silk_to_silk_clearance       → TRuleKind 55
silk_to_board_clearance      → TRuleKind 59
smd_entry                    → TRuleKind 60
board_outline_clearance      → TRuleKind 63
back_drilling                → TRuleKind 64
creepage                     → TRuleKind 65
return_path                  → TRuleKind 66
routing_neck_down            → TRuleKind 67
z_axis_clearance             → TRuleKind 69
```


## 6. Scope Expression Mini-Language

Altium scope expressions are used to select which objects a rule applies to.
We support a subset in the spec language:

```ebnf
scope_expr = "all"
           | "InNet" "(" STRING ")"
           | "InNetClass" "(" STRING ")"
           | "InComponent" "(" STRING ")"
           | "InComponentClass" "(" STRING ")"
           | "OnLayer" "(" STRING ")"
           | "HasFootprint" "(" STRING ")"
           | "IsVia"
           | "IsPad"
           | "IsTrack"
           | "IsFill"
           | "IsRegion"
           | "IsComponent"
           | scope_expr "and" scope_expr
           | scope_expr "or" scope_expr
           | "not" scope_expr
           | "(" scope_expr ")" ;
```

**Example scope expressions**:
```
scope1: InNetClass("Power")
scope2: all
scope1: InComponent("U1") or InComponent("U2")
scope1: IsVia and OnLayer("TopLayer")
```

**Implementation**: Scope expressions are evaluated during constraint generation
(not during solving). They filter the object set to produce (A, B) pairs that
the constraint applies to.


## 7. Clearance Block Shorthand

The `clearance { ... }` block inside `placement` is syntactic sugar for
multiple design rule declarations:

```
clearance {
    all: 0.5mm              → rule _auto_clearance_all { kind: component_clearance, gap: 0.5mm }
    connectors: 2mm         → rule _auto_clearance_conn { kind: component_clearance, gap: 2mm,
                                                           scope1: IsConnector }
    edge: 1mm               → rule _auto_edge_clearance { kind: board_outline_clearance, gap: 1mm }
}
```


## 8. CLI Commands

```bash
# Plan: show placement ECO without mutating
altium placement plan board-layout.pcb
altium placement plan board-layout.pcb --target my-board.PcbDoc

# Apply: solve + write component positions
altium placement apply board-layout.pcb

# DRC: check design rules against existing board
altium drc my-board.PcbDoc                           # use rules from board
altium drc my-board.PcbDoc --rules rules.pcb # use rules from spec

# DRC with JSON output
altium drc my-board.PcbDoc --json

# Dump: extract current placement as spec
altium placement dump my-board.PcbDoc                # → my-board.pcb
```

**Placement plan output**:
```
╔══════════════════════════════════════════════════════════════╗
║  PLACEMENT PLAN                                              ║
║  Board: my-board.PcbDoc                                      ║
║  Spec:  board-layout.pcb                             ║
║  Solver: LM (42 iterations, 3.2ms)                          ║
║  HPWL: 1,234mm (estimated wire length)                       ║
╚══════════════════════════════════════════════════════════════╝

CHANGES

  ~ MOVE U1 "STM32F407VGT6"
    │ position: (50.0mm, 40.0mm) → (48.2mm, 39.5mm)
    │ rotation: 0° → 90°

  ~ MOVE J1 "HDMI-A"
    │ position: (10.0mm, 10.0mm) → (50.0mm, 78.0mm)
    │ rotation: 0° → 0°

  = U2 "LM1117" (unchanged)

  14 components placed, 2 moved, 12 unchanged

CONSTRAINT SATISFACTION
  ✓ All 47 hard constraints satisfied
  ✓ Board containment: all components inside board
  ✓ Component clearance: min gap 0.8mm (required 0.5mm)
  ✓ Board edge clearance: min gap 1.2mm (required 1.0mm)
  ○ HPWL: 1,234mm (soft objective, weight 1.0)

END OF PLAN
```

**DRC report output**:
```
╔══════════════════════════════════════════════════════════════╗
║  DESIGN RULE CHECK REPORT                                    ║
║  Board: my-board.PcbDoc                                      ║
║  Rules: 12 enabled, 3 disabled                               ║
╚══════════════════════════════════════════════════════════════╝

VIOLATIONS (3)

  ✗ Clearance [Clearance_Default]
    │ C3 pad 1 ↔ C4 pad 2
    │ gap: 0.18mm (min: 0.254mm, violation: -0.074mm)
    │ layer: TopLayer
    │ location: (32.5mm, 41.2mm)

  ✗ BoardOutlineClearance [BoardOutline]
    │ J2 body
    │ gap: 0.6mm (min: 1.0mm, violation: -0.4mm)
    │ edge: left
    │ location: (1.4mm, 55.0mm)

  ✗ Width [Width_Power]
    │ track segment on net VCC3P3
    │ width: 0.2mm (min: 0.3mm, violation: -0.1mm)
    │ layer: TopLayer
    │ location: (45.0mm, 30.0mm)

PASSES (156)

  ✓ Clearance: 142 pairs checked, all pass
  ✓ HoleToHoleClearance: 8 pairs checked, all pass
  ✓ MinimumAnnularRing: 6 vias checked, all pass

────
  159 checks, 3 violations, 156 passes
```


## 9. Parser Extension Points

### New Keywords to Add to Lexer

```rust
// In lexer.rs, keyword matching:
"placement" => TokenKind::Placement,
"place" => TokenKind::Place,
"rule" => TokenKind::Rule,
"group" => TokenKind::Group,
"near" => TokenKind::Near,          // or handle as Ident
"left_of" => TokenKind::LeftOf,
"right_of" => TokenKind::RightOf,
"above" => TokenKind::Above,
"below" => TokenKind::Below,
"separate" => TokenKind::Separate,
"optimize" => TokenKind::Optimize,
"clearance" => TokenKind::Clearance,
"distance" => TokenKind::Distance,
```

**Alternative**: Keep directional constraints as identifiers and match in the
parser. This avoids polluting the keyword set and is more LLM-tolerant.

### New AST Nodes

```rust
pub enum SpecItem {
    // existing
    Import(ImportDecl),
    LetBinding(LetBinding),
    Component(ComponentDecl),
    Footprint(FootprintDecl),
    Project(ProjectDecl),
    // new
    Placement(PlacementDecl),
    Rule(RuleDecl),
}

pub struct PlacementDecl {
    pub body: Vec<Spanned<PlacementItem>>,
}

pub enum PlacementItem {
    Property(Property),           // target, default settings
    LetBinding(LetBinding),
    Place(PlaceDecl),
    Group(GroupDecl),
    Constraint(ConstraintDecl),
    Optimize(Object),
    Clearance(Object),            // clearance shorthand
}

pub struct PlaceDecl {
    pub designators: Vec<Spanned<EntityName>>,
    pub body: Object,
}

pub struct GroupDecl {
    pub name: Spanned<EntityName>,
    pub body: Object,
}

pub enum ConstraintDecl {
    LeftOf { a: Spanned<Expr>, b: Spanned<Expr>, props: Option<Object> },
    RightOf { a: Spanned<Expr>, b: Spanned<Expr>, props: Option<Object> },
    Above { a: Spanned<Expr>, b: Spanned<Expr>, props: Option<Object> },
    Below { a: Spanned<Expr>, b: Spanned<Expr>, props: Option<Object> },
    Separate { a: Spanned<Expr>, b: Spanned<Expr>, props: Option<Object> },
    Distance { a: Spanned<Expr>, b: Spanned<Expr>, props: Object },
}

pub struct RuleDecl {
    pub name: Spanned<EntityName>,
    pub body: Object,
}
```

### New Model Types

```rust
pub struct PcbDocSpec {
    pub placement: Option<PlacementSpec>,
    pub rules: Vec<RuleSpec>,
}

pub struct PlacementSpec {
    pub target: Option<String>,
    pub places: Vec<PlaceSpec>,
    pub groups: Vec<GroupSpec>,
    pub constraints: Vec<ConstraintSpec>,
    pub optimize: OptimizeSpec,
    pub clearance: ClearanceSpec,
}

pub struct PlaceSpec {
    pub designators: Vec<String>,
    pub region: Option<Region>,
    pub edge: Option<BoardEdge>,
    pub inset: Option<Coord>,
    pub align: Option<Alignment>,
    pub bias: Option<BoardEdge>,
    pub near: Option<String>,         // designator reference
    pub max_distance: Option<Coord>,
    pub rotation_options: Vec<i32>,   // allowed rotations
    pub fixed: bool,
    pub at: Option<CoordPoint>,
    pub side: Option<BoardSide>,
}

pub enum Region {
    Center,
    TopHalf, BottomHalf, LeftHalf, RightHalf,
    QuadrantTL, QuadrantTR, QuadrantBL, QuadrantBR,
    Custom { from: CoordPoint, to: CoordPoint },
}

pub struct GroupSpec {
    pub name: String,
    pub components: Vec<String>,
    pub keep_together: bool,
    pub max_spread: Option<Coord>,
}

pub enum ConstraintSpec {
    LeftOf { a: String, b: String, gap: Coord },
    RightOf { a: String, b: String, gap: Coord },
    Above { a: String, b: String, gap: Coord },
    Below { a: String, b: String, gap: Coord },
    Separate { a: String, b: String, gap: Coord },
    Distance { a: String, b: String, min: Option<Coord>, max: Option<Coord> },
}

pub struct OptimizeSpec {
    pub ratsnest: bool,
    pub ratsnest_weight: f64,
    pub thermal: bool,
    pub thermal_components: Vec<String>,
    pub thermal_weight: f64,
}

pub struct ClearanceSpec {
    pub all: Option<Coord>,
    pub connectors: Option<Coord>,
    pub edge: Option<Coord>,
}

pub struct RuleSpec {
    pub name: String,
    pub kind: RuleKind,
    pub gap: Option<Coord>,
    pub min: Option<Coord>,
    pub max: Option<Coord>,
    pub preferred: Option<Coord>,
    pub expansion: Option<Coord>,
    pub scope1: Option<ScopeExpr>,
    pub scope2: Option<ScopeExpr>,
    pub net_scope: Option<NetScope>,
    pub layer_scope: Option<LayerScope>,
    pub enabled: bool,
    pub priority: Option<i32>,
}
```


## 10. Domain Detection

The spec language already supports domain detection via file extension:
- `.sym` → SchLib
- `.sym` → PcbLib
- `.proj` → PrjPcb (already exists)

Add:
- `.pcb` → PcbDoc

```rust
pub enum SpecDomain {
    SchLib,
    PcbLib,
    PrjPcb,
    PcbDoc,  // NEW
}
```

The parser dispatches based on domain:
- `PcbDoc` domain allows `placement` and `rule` top-level items
- Other domains reject these items with a clear error
