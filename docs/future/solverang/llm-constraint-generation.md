# Phase 0: LLM-Driven Intelligent Constraint Generation

The key insight: the LLM agent doesn't just write a static placement spec from
vibes. It uses **altium-cli** to read the actual PcbDoc, then combines that data
with design knowledge (datasheets, application notes, mechanical drawings) to
generate precise, informed constraints — **before** solverang's global phase.

This is our version of what PCBAgent (ASPDAC 2025) does with RL + LLM, but
cleaner: the LLM generates a constraint spec, the solver makes it precise.


## The LLM Advantage Over Traditional Autoplacers

Traditional autoplacers know NOTHING about design intent:
- They see nets, pads, and clearance rules
- They optimize wire length blindly
- A USB connector and a debug header are equivalent objects

An LLM agent knows EVERYTHING about design intent:
- "This is an STM32F407 — the HSE pins need a crystal within 10mm"
- "This is a USB-C connector — it goes on a board edge, impedance-matched traces"
- "These are decoupling caps — each one pairs with a specific power pin"
- "The HDMI connector has a specific mechanical footprint from the datasheet"
- "The barrel jack and voltage regulators form a power section that should cluster"


## Phase 0 Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│  LLM Agent (Claude, GPT, etc.)                                  │
│                                                                  │
│  Inputs:                                                         │
│    1. altium-cli inspect → component list, netlist, footprints  │
│    2. Design notes / requirements doc                            │
│    3. Datasheet knowledge (from training data or RAG)           │
│    4. Board mechanical constraints (from mech drawing)          │
│    5. Previous placement (if iterating)                          │
│                                                                  │
│  Process:                                                        │
│    1. Identify functional blocks from netlist topology           │
│    2. Look up connector placement rules from datasheets          │
│    3. Determine power topology → power section grouping          │
│    4. Apply PCB design best practices                            │
│    5. Generate precise constraints with real dimensions          │
│                                                                  │
│  Output: .pcb file with placement constraints           │
└──────────────────────────┬──────────────────────────────────────┘
                           ↓
              ┌────────────────────────┐
              │  Phase 1: Solverang    │
              │  (global placement)    │
              └────────────────────────┘
```


## What the LLM Agent Can Query via altium-cli

```bash
# List all components with designator, footprint, and description
altium inspect my-board.PcbDoc --components

# Show netlist (which pins connect to which nets)
altium inspect my-board.PcbDoc --nets

# Show component footprint sizes (bounding boxes)
altium inspect my-board.PcbDoc --footprints

# Show board outline dimensions
altium inspect my-board.PcbDoc --board-outline

# Show existing design rules
altium inspect my-board.PcbDoc --rules

# Show current placement (if any)
altium inspect my-board.PcbDoc --placement
```

From this data, the LLM can determine:
- Which components are connectors (by footprint/description)
- Which components are ICs (by pin count, package type)
- Which components are passives (by footprint: 0402, 0603, SOT-23, etc.)
- Net topology (star, bus, differential pairs)
- Power nets (VCC, GND, 3V3, 5V)
- High-speed nets (from net class names or known IC pin functions)


## Example: LLM Reasoning for an STM32 Dev Board

The LLM agent examines the PcbDoc and reasons:

```markdown
## Board Analysis

Components: 47 total
  - 1× STM32F407VGT6 (U1, QFP-100, 14×14mm)
  - 1× HDMI-A connector (J1, 13.9×11.2mm)
  - 1× USB-C connector (J2, 8.9×7.4mm)
  - 1× SD card slot (J3, 14×15mm)
  - 1× Barrel jack (J4, 9×14mm)
  - 2× LM1117 3.3V regulators (U2, U3, SOT-223)
  - 1× 8MHz crystal (Y1, HC49/SMD, 5×3.2mm)
  - 12× 100nF decoupling caps (C1-C12, 0402)
  - 4× LEDs (D1-D4, 0603)
  - ... 24 other passives

Board: 80mm × 60mm rectangle

## Design Knowledge Applied

1. **STM32F407 crystal placement** (from datasheet AN4488):
   - Y1 must be within 10mm of OSC_IN/OSC_OUT pins (pins 12, 13)
   - Load capacitors C5, C6 must be between crystal and MCU
   - Guard ring ground pour recommended → keep area clear

2. **USB-C connector** (from USB-C spec):
   - Must be on board edge (mechanical requirement)
   - CC pins need 5.1kΩ pulldowns → R1, R2 near connector
   - D+/D- differential pair → route directly, place ESD diode near connector

3. **HDMI connector** (from HDMI spec):
   - Must be on board edge
   - TMDS differential pairs → short trace lengths
   - ESD protection near connector pads

4. **Power section** (from LM1117 datasheet):
   - Input cap within 10mm of VIN pin
   - Output cap within 5mm of VOUT pin
   - Thermal pad needs copper pour → place with clearance

5. **Decoupling caps** (from STM32 PCB design guide):
   - One 100nF per VDD pin, as close as possible
   - 4.7µF bulk cap within 20mm

6. **Board edge connectors** (mechanical):
   - HDMI: top edge, centered (user-facing)
   - USB-C: left edge, upper area (cable management)
   - SD card: right edge (user access)
   - Barrel jack: left edge, lower area (away from signals)
```

This produces the constraint spec:

```
placement {
    target: "my-board.PcbDoc"

    // ── From STM32 datasheet: crystal placement ────────
    place Y1 {
        near: $U1
        max_distance: 10mm      // AN4488: "within 10mm of OSC pins"
    }
    left_of $Y1, $U1 { gap: 1mm }  // crystal on pin 12/13 side

    place C5, C6 {                   // crystal load caps
        near: $Y1
        max_distance: 3mm           // between crystal and MCU
    }

    // ── From USB-C spec: edge placement ────────────────
    place J2 {
        edge: left
        inset: 0mm                  // flush with board edge (mechanical)
        bias: top
        rotation: 270               // pins face inward
    }

    place U4 {                       // USB ESD protection
        near: $J2
        max_distance: 5mm           // ESD: as close as possible
    }

    place R1, R2 {                   // CC pulldown resistors
        near: $J2
        max_distance: 8mm
    }

    // ── From HDMI spec: edge + short traces ────────────
    place J1 {
        edge: top
        inset: 0mm
        align: center
        rotation: 0
    }

    place U5 {                       // HDMI ESD/level shifter
        near: $J1
        max_distance: 8mm           // short TMDS traces
    }

    // ── From LM1117 datasheet: input/output caps ──────
    place C7 {                       // U2 input cap
        near: $U2
        max_distance: 10mm          // datasheet: "within 10mm of VIN"
    }

    place C8 {                       // U2 output cap
        near: $U2
        max_distance: 5mm           // datasheet: "within 5mm of VOUT"
    }

    // ── From STM32 PCB guide: decoupling ───────────────
    place C1, C2, C3, C4, C9, C10, C11, C12 {
        near: $U1
        max_distance: 5mm           // as close as possible to VDD pins
    }

    // ... (connector edges, LED cluster, etc.)
}
```

**Every dimension comes from a real source** (datasheet, spec, app note) —
not guessed. The LLM agent cites its sources in comments.


## Constraint Types the LLM Can Generate

### From Datasheets

| Source | Constraint | Example |
|--------|-----------|---------|
| Crystal app note | `near: $MCU, max_distance: 10mm` | STM32 AN4488 |
| Regulator datasheet | `near: $REG, max_distance: 5mm` | LM1117 cap placement |
| ESD protection | `near: $CONNECTOR, max_distance: 5mm` | USB ESD IC |
| Impedance matching | `near: $CONNECTOR` + routing constraint | USB D+/D- |

### From Mechanical Drawings

| Source | Constraint | Example |
|--------|-----------|---------|
| Enclosure CAD | `edge: left, inset: 2mm` | Connector cutout position |
| Mounting holes | `fixed: true, at: (5mm, 5mm)` | M3 standoff locations |
| Keep-out zone | `distance $comp, $hole { min: 3mm }` | Around mounting holes |
| Height limit | component height check | Under LCD, under heatsink |

### From Design Best Practices

| Practice | Constraint | Source |
|----------|-----------|--------|
| Analog/digital separation | `separate $analog, $digital { gap: 10mm }` | PCB design guides |
| Power section clustering | `group power { components: [...] }` | Power integrity |
| High-speed trace length | `near: $IC` (minimize stub) | SI guidelines |
| Thermal spreading | `distance $hot_A, $hot_B { min: 15mm }` | Thermal management |
| EMC: clock near MCU | `near: $MCU, max_distance: 3mm` | EMC guidelines |

### From Net Topology Analysis

The LLM can analyze the netlist to identify:

1. **Differential pairs** → place both ends close together
2. **Power trees** → cluster regulator + caps + load
3. **Bus structures** → align components along bus direction
4. **Star topology** → central IC with radial passives
5. **Daisy chains** → linear component arrangement


## LLM Agent Prompt Template

```markdown
You are a PCB placement engineer. Given the following board data from
altium-cli, generate a .pcb placement file.

## Board Data
{output of altium inspect --components}
{output of altium inspect --nets}
{output of altium inspect --board-outline}

## Design Requirements
{user's design notes, or empty}

## Instructions
1. Identify all connectors and place them on appropriate board edges
2. For each IC, look up placement guidelines from its datasheet
3. Place decoupling capacitors near their associated power pins
4. Group functionally related components
5. Separate analog and digital sections if applicable
6. Add clearance constraints based on component types
7. Enable HPWL optimization for overall wire length
8. Use exact dimensions from datasheets where possible
9. Cite your sources in comments

Generate a complete .pcb file.
```


## Multi-Round Iteration

The LLM doesn't have to get it right on the first try:

```
Round 1:  LLM generates initial spec from netlist analysis
          → Solverang solves → LLM reviews placement report

Round 2:  LLM sees "HPWL=2,340mm, J2 clearance violation"
          → Adjusts constraints (relax J2 inset, add group for power)
          → Solverang re-solves → better result

Round 3:  LLM sees "all constraints satisfied, HPWL=1,890mm"
          → Adds fine-tuning constraints ("move C3 closer to U1 pin 42")
          → Final solve → placement complete
```

This is exactly what PCBAgent (ASPDAC 2025) demonstrated: multi-round
LLM-driven optimization outperforms single-shot approaches.


## Integration with altium-cli

### New CLI Commands

```bash
# Generate placement spec from PcbDoc (LLM-assisted)
altium placement generate my-board.PcbDoc
    --model claude-sonnet-4-6       # LLM to use
    --design-notes notes.md     # optional design requirements
    --output board-layout.pcb

# Solve placement from spec
altium placement solve board-layout.pcb
    --target my-board.PcbDoc
    --phases 0,1,2,3,4          # which phases to run
    --output placement-report.txt

# Interactive: LLM + solver loop
altium placement interactive my-board.PcbDoc
    --model claude-sonnet-4-6
    --max-rounds 5
```

### Machine-Readable Solve Output

The solver produces structured output the LLM can parse for iteration:

```json
{
  "status": "solved",
  "phases": {
    "global": { "iterations": 42, "time_ms": 23 },
    "legalization": { "time_ms": 2 },
    "sa": { "iterations": 15000, "time_ms": 340 },
    "refinement": { "iterations": 8, "time_ms": 5 }
  },
  "metrics": {
    "total_hpwl_mm": 1890.3,
    "max_constraint_violation_mm": 0.0,
    "net_crossings": 12,
    "component_count": 47,
    "placed_count": 47
  },
  "violations": [],
  "placements": [
    { "designator": "U1", "x_mm": 40.2, "y_mm": 30.1, "rotation": 0 },
    { "designator": "J1", "x_mm": 40.0, "y_mm": 58.0, "rotation": 0 },
    ...
  ]
}
```


## Why This Works Better Than Pure Algorithmic Approaches

| Approach | Quality | Speed | Constraint Handling |
|----------|---------|-------|-------------------|
| SA only | Good | Slow | Penalty functions (soft) |
| Analytical only | Good | Fast | Hard constraints, no design intent |
| RL (Google) | Good | Needs training | Learned from data, not explicit |
| **LLM + Solverang** | **Best** | **Fast** | **Explicit from datasheets + hard solver** |

The LLM encodes more design knowledge than any training dataset because it has
read thousands of datasheets, app notes, and PCB design guides. It generates
constraints that encode DESIGN INTENT, not just geometric relationships. The
solver then makes those constraints mathematically precise.

This is the PCB placement equivalent of:
- RL: "Learn to play chess by playing millions of games"
- LLM: "Read every chess book ever written, then tell me your strategy"
- Solver: "Given the strategy, find the exact optimal moves"
