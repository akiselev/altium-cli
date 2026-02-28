# altium-format-layout: Auto-Layout & Auto-Sizing Brainstorm

## Problem Statement

The spec language has solid *mechanical* layout primitives — anchors (`on: $body.left, at: "center"`), `after:`/`before:` relative placement, `row`/`column`/`grid` for PCB footprints. But when an LLM generates a spec, it has no concept of what "looks right" for a schematic symbol. Common failures:

- **Tiny pin lengths** — 20mil instead of 200–300mil for IC pins, 100mil for passives
- **Absurd body rectangles** — 50×50mil instead of something proportional to pin count/names
- **No grid snapping** — coordinates that don't fall on 10 or 50mil boundaries
- **No functional grouping** — power pins mixed with I/O, inputs on wrong side
- **No text clearance** — pin names overlapping body, designators colliding
- **Wrong proportions** — components that look nothing like real Altium symbols

The LLM knows the *logical* structure perfectly (this chip has these pins with these functions) but fails at the *geometric* realization. We need a crate that bridges that gap.

## What altium-format-layout Should Do

A post-processing pass that takes the high-level API types (`api::Component`, `api::Pin`, `api::Graphic`, etc.) and computes proper geometry. The spec language describes *intent*; the layout engine resolves *geometry*.

### Phase 1: SchLib Component Layout (Core)

1. **Auto-size body rectangle** — given pins grouped by side, compute the rectangle dimensions:
   - Width: `max(min_body_width, left_name_width + right_name_width + padding)`
   - Height: `max(left_pin_count, right_pin_count) * pin_pitch + margins`
   - Snap to grid (10mil default)

2. **Auto-place pins** — given a list of pins with functional grouping metadata:
   - Inputs on left, outputs on right (IPC-2612 / Altium convention)
   - Power (VCC/VDD) on top, ground (GND/VSS) on bottom
   - Within each side, group by function with spacing between groups
   - Pin pitch: 100mil default (Altium standard)
   - Pin length: 200–300mil for ICs, 100mil for passives

3. **Grid snap** — all coordinates snapped to configurable grid (default 10mil)

4. **Text clearance** — ensure pin names don't overlap body or each other

5. **Default sizing** — sensible defaults for when the spec says nothing:
   - Passive 2-pin: small rectangle with pins on left/right
   - IC: rectangle sized to fit pins with proper naming
   - Multi-part: per-part body sizing

6. **Pin classification heuristics** — name-pattern matching and electrical type rules

### Phase 2: SchDoc Layout (Future)

- Component placement on sheets
- Wire routing between pins
- Bus formation
- Power rail placement

### Phase 3: PcbDoc (Future)

- Component placement
- Courtyard/silkscreen sizing

## Industry Standards & Conventions

### How EDA Tools Do This

**Altium's Symbol Wizard** uses predefined layout templates with manual refinement:
- Dual in-line, quad side, connector, single in-line, manual
- Users assign pins to functional groups via a pin data table
- Wizard auto-sizes the body to fit
- Pin layout style reverts to "Manual" when user edits positions

**EasyEDA** auto-groups pins by name pattern recognition. It identifies power pins (VCC, GND), bus pins (D0–D7), and signal pins by parsing pin names, then assigns them to sides.

**KiCad** relies on third-party tools and manual placement; no built-in auto-layout.

### Relevant Standards

- **IEEE Std 315** (ANSI Y32.2): Graphic symbols for electrical diagrams. All connection points on a modular grid.
- **IEEE Std 91-1984**: Logic symbol standards with dependency notation and grouping.
- **IPC-2612-1**: Modern standard for generating schematic symbols for complex, high-pin-count devices. Covers pin assignment rules, pin grouping, grid layout, and symbol proportions.

### Industry-Standard Algorithm

From academic literature and tool implementations:

1. **Functional Clustering**: Group pins by function (power, I/O, bus, analog, digital)
   - Name-based heuristics (regex matching for VCC, GND, CLK, D0–D7, etc.)
   - Electrical type classification (input/output/bidirectional/power/passive)
2. **Side Assignment**: Convention-based rules:
   - Inputs on left, outputs on right
   - Power (VCC) on top, ground (GND) on bottom
   - Bidirectional on left or right depending on function
3. **Pin Ordering Within a Side**: Alphabetical, numerical (bus index), or by functional subgroup
4. **Body Sizing**: height = max(left, right) × pitch + padding; width = max(top, bottom) × pitch + padding; width must also accommodate longest pin name text
5. **Multi-Part Decomposition**: For high pin-count (100+), split into functional sub-symbols

## Typical Sizing Conventions

From codebase analysis and Altium conventions:

| Element | LLM Generates | Proper Value | Notes |
|---------|--------------|--------------|-------|
| Pin length (IC) | 20mil | 200–300mil | IPC-2612 convention |
| Pin length (passive) | 20mil | 100mil | |
| Pin pitch | varies | 100mil | Altium standard grid |
| Body margin | 0 | 50mil | Inside body edge to pin attachment |
| Group gap | 0 | 100mil | Extra gap between pin groups |
| Grid snap | none | 10mil | Altium base grid |
| SMD pad (0603) | varies | 60×60mil | From real fixtures |
| TH drill (0603) | varies | 28mil | From real fixtures |
| Char width (approx) | n/a | ~50mil | For text width estimation |

## Rust Crate Ecosystem Survey

### Constraint Solvers

| Crate | Status | Fit | Notes |
|-------|--------|-----|-------|
| **kasuari** | Active (ratatui uses it) | Maybe Phase 2 | Cassowary linear constraint solver. Good for "pin A must be 100mil below pin B" type constraints. Priority system for soft vs hard constraints. Low-level: no notion of rectangles or 2D. |
| cassowary-rs | Unmaintained (2018) | Avoid | Use kasuari instead |
| z3 (bindings) | Active | Avoid | SMT solver. Nuclear option: immensely powerful but 50MB native dep, complex API, slow for interactive use. Only if procedural approach completely fails. |
| good_lp | Active | Avoid | MILP modeling. Non-overlap is non-linear, requires big-M → NP-hard. Overkill. |
| varisat | Maintained | Avoid | SAT solver. Wrong abstraction level for geometric layout. |

### CSS-style Layout

| Crate | Status | Fit | Notes |
|-------|--------|-----|-------|
| **taffy** | Active (Dioxus/Zed) | Poor | Flexbox/CSS Grid. CSS box model is fundamentally wrong for schematic layout — pins radiate *outward* from bodies, not flow *inside* containers. |
| morphorm | Active (vizia) | Poor | Same problem as taffy. |

### Graph Layout

| Crate | Status | Fit | Notes |
|-------|--------|-----|-------|
| **petgraph** | Active, mature | Phase 2 | Graph data structures + algorithms. Needed for SchDoc net connectivity, wire routing. Not needed for component layout. |
| **rust-sugiyama** | Maintained | Phase 2 | Sugiyama layered graph drawing on petgraph. Good for placing components in signal-flow order on schematic sheets. |
| fdg | Stale (2023) | Avoid | Force-directed produces organic non-orthogonal results. Bad for schematics. |

### 2D Geometry

| Crate | Status | Fit | Notes |
|-------|--------|-----|-------|
| euclid | Active (Servo) | Skip | We already have `Coord`/`CoordPoint`/`BoundingBox`. Adding euclid means type conversions everywhere. |
| geo | Active (GeoRust) | Skip | Geospatial-focused, f64-based, heavy deps. We only need AABB ops. |
| rstar | Active | Maybe Phase 2 | R*-tree spatial indexing. Useful for "find overlapping components" in sheet layout. Not needed for single component. |
| parry2d | Active | Skip | Physics collision detection. Massive overkill. |

### Rectangle Packing

| Crate | Status | Fit | Notes |
|-------|--------|-----|-------|
| rectangle-pack | Active | Maybe Phase 2 | Deterministic bin packing. Optimizes *density*, not *readability*. Schematics need whitespace for routing. |

### EDA-Specific

| Crate | Status | Fit | Notes |
|-------|--------|-----|-------|
| Atlantix-EDA | Active, GPL | Avoid | KiCad lib generation. GPL license, Bevy ECS architecture. |
| Substrate2 (UCB) | Active, BSD | Reference only | IC-level (transistor layout), not schematic symbols. |

**Key finding**: No existing Rust crate does schematic symbol auto-layout. We must build it ourselves.

## Architectural Options

### Option A: Constraint-Based (kasuari)

```
Pin assignments → Constraint system → Solve → Grid snap → Validate
```

**Pros**:
- Naturally expresses layout rules as constraints
- Handles conflicting requirements gracefully via priorities
- Extensible — add new rules without rewriting the solver
- Well-proven paradigm (Apple Auto Layout, many GUI frameworks)

**Cons**:
- Learning curve for constraint formulation
- Debugging constraint conflicts can be opaque
- Overkill for simple cases (2-pin passives)
- Non-linear constraints (grid snap) need a 2-pass workaround
- External dependency
- Only linear — can't express `min()`, `max()`, conditionals

### Option B: Imperative Algorithm (no dependencies)

```
Pin assignments → Sort by side/group → Compute pitch → Size body → Place pins → Grid snap
```

**Pros**:
- Simple, predictable, debuggable
- Zero dependencies
- Fast (single pass for simple components)
- Easy to understand and modify
- Matches how Altium's own Symbol Wizard works (predefined layout patterns)
- Grid snapping is trivial (just round coordinates)
- Error messages can be specific: "pin X placed here because: 3rd input, pitch 100mil, body starts at Y=200"

**Cons**:
- Adding new layout rules means modifying code, not adding constraints
- Harder to handle complex interactions (text too long → expand body → move pins → change text positions)
- May need iterative refinement for complex multi-part components

### Option C: Hybrid (recommended)

Start with imperative algorithm (Option B) for Phase 1. Add constraint solving (kasuari) when Phase 2 needs it for SchDoc sheet layout, where components interact and constraints between multiple entities are more natural.

**Rationale**:
1. **Component layout is well-structured** — pins go on 4 sides, body is a rectangle, pitch is fixed. This isn't general constraint satisfaction; it's a specific geometric algorithm.
2. **The spec lang already has anchors** — the compiler already does edge-relative placement. The layout engine fills in *defaults*, not solving arbitrary constraints.
3. **Debuggability matters enormously** — when a component looks wrong, "the algorithm placed pin here because X" beats "the constraint solver converged to this".
4. **Zero deps** — no version conflicts, no type conversions.

## Proposed Architecture

```
altium-format-types (Coord, CoordPoint, BoundingBox, PinElectricalType, etc.)
     ↓
altium-format (api types: Component, Pin, Graphic, etc.)
     ↓
altium-format-layout (NEW: layout engine, zero deps beyond altium-format)
     ↓
altium-format-spec (uses layout as optional post-processing pass)
     ↓
altium-cli (exposes layout commands)
```

### Core Types

```rust
/// Layout configuration with sensible defaults
pub struct LayoutConfig {
    pub grid: Coord,              // Snap grid (default: 10mil)
    pub pin_pitch: Coord,         // Spacing between pins (default: 100mil)
    pub pin_length: Coord,        // Default pin length (default: 200mil)
    pub passive_pin_length: Coord,// Pin length for passives (default: 100mil)
    pub group_gap: Coord,         // Extra gap between pin groups (default: 100mil)
    pub body_margin: Coord,       // Margin inside body edges (default: 50mil)
    pub text_margin: Coord,       // Clearance for pin name text (default: 10mil)
    pub char_width: Coord,        // Approximate character width (default: 50mil)
    pub min_body_width: Coord,    // Minimum body width (default: 200mil)
    pub min_body_height: Coord,   // Minimum body height (default: 200mil)
}

/// Pin classification for auto-placement
pub enum PinRole {
    Input,
    Output,
    Bidirectional,
    PowerPositive,   // VCC, VDD, etc.
    PowerGround,     // GND, VSS, etc.
    Passive,
    Clock,
    Reset,
    Other,
}

/// Pin grouping for layout
pub struct PinGroup {
    pub name: Option<String>,     // Group label (e.g., "SPI", "Port A")
    pub pins: Vec<usize>,         // Indices into the component's pin list
    pub side: Option<Side>,       // User override or auto-assigned
}

/// Which side of the body a pin attaches to
pub enum Side { Left, Right, Top, Bottom }
```

### Key Functions

```rust
/// Full auto-layout: classify, assign, size, place, snap.
/// Modifies the component in-place.
pub fn layout_component(
    component: &mut api::Component,
    config: &LayoutConfig,
) -> Result<LayoutReport, LayoutError>;

/// Classify a pin by name and electrical type.
pub fn classify_pin(pin: &api::Pin) -> PinRole;

/// Assign pins to sides based on classification.
pub fn auto_assign_sides(pins: &[api::Pin]) -> Vec<(usize, Side)>;

/// Compute body dimensions to fit pins on all sides.
pub fn auto_size_body(
    side_assignments: &SideAssignments,
    config: &LayoutConfig,
) -> BoundingBox;

/// Place pins along their assigned edges with proper spacing.
pub fn place_pins(
    pins: &mut [api::Pin],
    body: &BoundingBox,
    side_assignments: &SideAssignments,
    config: &LayoutConfig,
);

/// Snap all coordinates to grid.
pub fn snap_to_grid(component: &mut api::Component, grid: Coord);
```

### The Layout Algorithm

```
1. CLASSIFY: For each pin, determine PinRole from electrical type + name heuristics
   - PinElectricalType::Power + name matches VCC/VDD/V+ → PowerPositive
   - PinElectricalType::Power + name matches GND/VSS/V- → PowerGround
   - PinElectricalType::Input → Input
   - PinElectricalType::Output → Output
   - PinElectricalType::IO → Bidirectional
   - PinElectricalType::Passive → Passive
   - Name heuristics: CLK*/SCK → Clock, RST*/NRST → Reset, etc.

2. GROUP: Cluster pins by functional prefix
   - SPI_MOSI, SPI_MISO, SPI_SCK, SPI_CS → "SPI" group
   - PORTA_0..PORTA_7 → "Port A" group
   - Ungrouped pins stay in a default group

3. ASSIGN SIDES: Map groups to body edges
   - PowerPositive groups → Top
   - PowerGround groups → Bottom
   - Input/Clock/Reset groups → Left
   - Output groups → Right
   - Bidirectional → Right (or Left if nothing else is there)
   - Passive: split evenly Left/Right

4. ORDER: Within each side, sort groups by name, pins within groups by designator/name

5. SIZE BODY:
   - left_height = left_pin_count * pin_pitch + (left_group_count - 1) * group_gap + 2 * body_margin
   - right_height = same for right
   - body_height = max(left_height, right_height), snapped to grid
   - Analogous for width from top/bottom pin counts
   - Also consider: max_left_name_width * char_width + max_right_name_width * char_width + padding
   - body_width = max(text-based width, top/bottom-based width, min_body_width)

6. PLACE PINS: Along each edge, evenly spaced with group gaps
   - Pin connection point = body_edge ± pin_length (outside)
   - Auto-orient: left→0°, right→180°, top→270°, bottom→90°

7. SNAP TO GRID: Round all coordinates to nearest grid multiple

8. VALIDATE: Check for overlaps, out-of-bounds, unreachable pins
```

## Pitfalls & Tricky Issues

### 1. Pin Name Text Width Estimation

We don't have font metrics. Altium's default font is roughly 50mil per character at standard size, but font_id can change this. Strategy: use a conservative approximation (char_count × char_width), accept that it won't be pixel-perfect. Can refine later when we have font table access.

### 2. Multi-Part Components

An LM358 has 2 op-amp parts sharing power pins. Each part needs independent layout, but `owner_part_id == 0` pins are shared. The layout engine must:
- Lay out each part independently
- Handle shared pins (power) appearing in the "all parts" context
- Ensure consistent body sizing across parts (or allow per-part sizing)

### 3. Partial Specification

What if the user specified *some* pin positions but wants the rest auto-placed? Need to:
- Detect which pins already have meaningful coordinates (not default 0,0)
- Respect those positions as fixed constraints
- Auto-place only the unplaced pins
- Possibly resize the body to accommodate both fixed and auto-placed pins

### 4. Pin Naming Heuristics Are Fragile

"MOSI" → output works for SPI master, but "GPIO0" could be anything. Strategy:
- `PinElectricalType` takes precedence when set
- Name heuristics are fallback only
- Unknown → Bidirectional (default to right side)
- Allow user overrides via spec language

### 5. Grid Snapping Can Create Overlaps

If pin pitch is 90mil and grid is 100mil, snapping creates collisions. Detection:
- After snapping, check for duplicate coordinates on the same edge
- If found, increase pitch to next grid multiple and re-layout
- This is why iterative refinement might be needed

### 6. Pin Length Depends on Context

| Component Type | Pin Length | Rationale |
|---------------|-----------|-----------|
| IC (< 20 pins) | 200mil | Standard for small ICs |
| IC (20–64 pins) | 200mil | Same |
| IC (64+ pins) | 300mil | More space for pin names |
| Passive (R, C, L) | 100mil | Compact |
| Connector | 100–200mil | Depends on pin count |
| Power symbol | 50–100mil | Compact, often hidden |

### 7. Interaction with Existing Spec Anchors

The spec language already has `on: $body.left, at: "center"`. If the layout engine resizes the body, anchor positions change. Semantics:
- **Layout runs first** — sizes the body and places pins that have no explicit position
- **Spec anchors override** — if a pin has an explicit anchor, it keeps that position
- **OR: Layout runs after spec** — as a fixup pass that only adjusts things that look wrong

The cleanest approach: layout is an **optional pass** that runs on the `api::Component` output, either before saving or as an explicit CLI command. The spec executor produces the component, then the user can optionally run `layout_component()` to fix up sizing.

### 8. When NOT to Auto-Layout

Some components have intentionally non-standard layout:
- Op-amps (triangle body, not rectangle)
- Logic gates (distinctive shapes)
- Transformers, relays, switches
- Anything with custom graphics

The layout engine should detect when a component already has explicit graphics beyond a simple body rectangle and leave it alone, or only adjust pin positions relative to the existing graphics.

### 9. Future Wire Routing Is Completely Different

Component layout = geometric rule application on a single entity.
Wire routing = graph-based pathfinding with orthogonal constraints across a sheet.

These share almost no code. The crate should be designed with clear module boundaries knowing that `layout::component` and `layout::routing` will use fundamentally different algorithms.

### 10. Altium's Conventions Are Undocumented

Altium's Symbol Wizard uses "predefined layout patterns" but doesn't publish the algorithm. We're reverse-engineering conventions from:
- IPC-2612-1 guidelines
- Real component examples in test fixtures
- The decompiled C#/Delphi code
- Community best practices

## Decision Summary

| Decision | Recommendation | Rationale |
|----------|---------------|-----------|
| External deps (Phase 1)? | Zero | We have our own coord types; algorithm is straightforward |
| Constraint solver? | Not Phase 1; maybe Phase 2 (kasuari) | Component layout is well-structured enough for imperative code |
| Graph library? | Phase 2 (petgraph + rust-sugiyama) | Needed for SchDoc wiring, not component layout |
| Geometry crate? | No — extend existing types | Add methods to `BoundingBox`/`Coord` (~50 lines) |
| Where in pipeline? | Post-processing on `api::Component` | Spec = intent, layout = geometry |
| Grid default? | 10mil | Altium standard |
| Pin pitch default? | 100mil | Industry standard |
| Pin length default? | 200mil (ICs), 100mil (passives) | IPC-2612 conventions |
| Text width? | char_count × fixed_width | Good enough; refine later with font tables |
| Multi-part? | Per-part independent layout with shared pin handling | Matches Altium's model |

## Open Questions

1. Should the layout engine be invocable from the spec language itself? e.g., `component R { layout: auto }` or always as a post-processing pass?

2. Should we support layout "profiles" (e.g., `profile: ic`, `profile: passive`, `profile: connector`) or always infer from pin characteristics?

3. How do we handle the case where an LLM generates a spec with explicit (but terrible) coordinates? Should layout override them, or only fill in missing ones? A `force: true` flag?

4. Should the layout engine produce a LayoutReport with explanations ("pin X placed on left because it's classified as Input") for debugging?

5. For Phase 2 SchDoc layout, should we use Sugiyama (layered) or force-directed for initial component placement? Sugiyama respects signal flow direction; force-directed is simpler but produces less structured results.
