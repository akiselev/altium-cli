# Pin & Part Swap Integration for the Autoplacer

Pin swapping, part swapping, and gate swapping are the autoplacer's most
powerful optimization tools. They change the **logical-to-physical mapping**
(which net connects to which pin) without changing the circuit function,
enabling dramatic wirelength and routability improvements.

This document specifies how the autoplacer discovers, evaluates, and applies
swap operations, and how it propagates changes back to upstream schematic
spec files.


## 1. Swap Taxonomy

### 1.1 Pin Swap

Two pins on the **same component** in the **same pin swap group** exchange
their net assignments.

```
Before:                         After:
  U1 pin 3 (group A) → NET_X     U1 pin 3 (group A) → NET_Y
  U1 pin 4 (group A) → NET_Y     U1 pin 4 (group A) → NET_X
```

**When it helps**: Reduces wire crossings when two signals approach a
component from opposite sides but connect to the "wrong" pins.

**Common examples**:
- NAND/NOR gate inputs (logically equivalent)
- Resistor/capacitor terminals (symmetric)
- Bus lines on memory chips (data pins are often swappable)

### 1.2 Part Swap

Two **identical components** in the **same part swap group** exchange ALL
their net assignments. The physical components stay where they are.

```
Before:                         After:
  R1 (group R100K): pin1→NET_A    R1 (group R100K): pin1→NET_C
                    pin2→NET_B                      pin2→NET_D
  R2 (group R100K): pin1→NET_C    R2 (group R100K): pin1→NET_A
                    pin2→NET_D                      pin2→NET_B
```

**When it helps**: When two identical resistors are placed near different
ICs, swapping their logical roles means each resistor connects to the
nearest IC instead of routing across the board.

**Common examples**:
- Identical pull-up/pull-down resistors
- Identical decoupling capacitors
- Identical bypass diodes

### 1.3 Gate Swap

Gates within a **multi-gate IC** in the **same part swap group** exchange
their net assignments. The IC stays put, but the pin mapping changes.

```
Before (quad NAND U1):          After:
  Gate A (pins 1,2,3) → SIG_X    Gate A (pins 1,2,3) → SIG_Y
  Gate B (pins 4,5,6) → SIG_Y    Gate B (pins 4,5,6) → SIG_X
```

**When it helps**: When signals naturally route near different gates of the
same IC, swapping which gate handles which signal reduces crossings.

**Common examples**:
- Quad OpAmp (LM324): 4 identical gates
- Hex inverter (74HC04): 6 identical gates
- Dual comparator (LM393): 2 identical gates


## 2. Swap Groups in Altium's Data Model

### 2.1 Pin-Level Fields (SchLib)

Each schematic pin carries three swap identifiers:

| Field | Constant | Meaning |
|-------|----------|---------|
| `swap_id_pin` | `SWAP_ID_PIN` | Pin swap group within the component |
| `swap_id_part` | `SWAP_ID_PART` | Part/gate swap group ID |
| `swap_id_pair` | `SWAP_ID_PAIR` | Pair swap for differential signals |

Pins with the **same `swap_id_pin`** on the **same component** can be
pin-swapped. Pins with the **same `swap_id_part`** across **different
components** (or gates within a multi-gate IC) can be part/gate-swapped.

### 2.2 Project-Level Settings

```rust
// From PrjPcb [Design] section
pub struct Project {
    pub pin_swap_by_netlabel: bool,  // Allow pin swap based on net labels
    pub pin_swap_by_pin: bool,       // Allow pin swap based on pin properties
}
```

### 2.3 Spec Language (Existing)

The spec language already supports declaring swap groups:

```
component NAND_GATE {
    swap_group input_pins {
        pins: [IN_A, IN_B]
    }

    pin IN_A {
        swap_group: input_pins
    }
    pin IN_B {
        swap_group: input_pins
    }
    pin OUT { }
}
```


## 3. Autoplacer Swap Discovery

### 3.1 Extracting Swap Groups from IR

The autoplacer must build a swap model from the SchLib/SchDoc data:

```rust
/// All swap opportunities for the design
pub struct SwapModel {
    /// Pin swap groups: (component, group_id) → list of swappable pin indices
    pub pin_swap_groups: HashMap<(ComponentId, String), Vec<PinIndex>>,

    /// Part swap groups: group_id → list of component IDs with identical pinouts
    pub part_swap_groups: HashMap<String, Vec<ComponentId>>,

    /// Gate swap groups: (component, group_id) → list of gate descriptors
    pub gate_swap_groups: HashMap<(ComponentId, String), Vec<GateDescriptor>>,
}

pub struct GateDescriptor {
    /// The pins belonging to this gate
    pub pins: Vec<PinIndex>,
    /// Current net assignments
    pub nets: Vec<Option<NetId>>,
}
```

### 3.2 Building the SwapModel

```
Input: PcbIr + SchLib pin data (swap_id_pin, swap_id_part, swap_id_pair)

For each component C in PcbIr:
    For each pin P on C:
        if P.swap_id_pin is non-empty:
            pin_swap_groups[(C.id, P.swap_id_pin)].push(P.index)
        if P.swap_id_part is non-empty:
            part_swap_groups[P.swap_id_part].push(C.id)  // deduplicate

For part_swap_groups:
    Filter to groups with ≥2 components
    Verify all components have identical pin counts and swap IDs
    (Components with different footprints cannot be part-swapped)

For gate_swap_groups:
    Within multi-gate ICs, group pins by swap_id_part
    Each group = one gate
    Gates with same swap_id_part on same component = gate-swappable
```


## 4. Swap Evaluation in the Autoplacer

### 4.1 Pin Swap Evaluation (O(1) per swap)

For each pin swap group on a component, try all pairwise pin swaps and
compute HPWL delta:

```
For component C with pin swap group G = [pin_i, pin_j, ...]:
    For each pair (pin_i, pin_j) in G:
        // Swap their net assignments
        old_hpwl = hpwl(net_of(pin_i)) + hpwl(net_of(pin_j))
        swap nets
        new_hpwl = hpwl(net_of(pin_i)) + hpwl(net_of(pin_j))
        delta = new_hpwl - old_hpwl
        if delta < 0: accept swap (greedy)
        else: restore
```

This is extremely cheap — only 2 nets need HPWL recalculation per swap.

### 4.2 Part Swap Evaluation (O(k) per swap, k = pins per component)

```
For part swap group G = [comp_A, comp_B, ...]:
    For each pair (comp_A, comp_B) in G:
        // Swap ALL net assignments between A and B
        old_hpwl = Σ hpwl(nets touching A) + Σ hpwl(nets touching B)
        swap all pin→net mappings between A and B
        new_hpwl = Σ hpwl(nets touching A) + Σ hpwl(nets touching B)
        delta = new_hpwl - old_hpwl
        if delta < 0: accept swap
        else: restore
```

### 4.3 Gate Swap Evaluation

Same as part swap, but within a single multi-gate IC:

```
For component C with gate swap group = [gate_X, gate_Y]:
    old_hpwl = Σ hpwl(nets on gate_X pins) + Σ hpwl(nets on gate_Y pins)
    swap net assignments between gate_X pins and gate_Y pins
    new_hpwl = recompute
    if improves: accept
```

### 4.4 When to Evaluate Swaps

Swap evaluation fits naturally into the placement pipeline:

| Phase | Swap Type | Rationale |
|-------|-----------|-----------|
| **After Phase 2** (legalization) | Part swap | Components have legal positions; part swap reassigns logic to better-placed components |
| **During Phase 3** (SA) | Pin swap | Add as an SA move type alongside displace/swap/rotate/slide |
| **After Phase 3** (SA) | All swaps | Final greedy sweep to catch remaining improvements |
| **After Phase 4** (refinement) | Pin swap | Last-pass pin optimization with final positions |

### 4.5 SA Move Type: Pin Swap

Add a new move type to SA:

```rust
pub enum Move {
    Displace { comp_idx: usize, dx: f64, dy: f64 },
    Swap { a_idx: usize, b_idx: usize },
    Rotate { comp_idx: usize, delta_deg: i32 },
    Slide { comp_idx: usize, axis: Axis, delta: f64 },
    // NEW: swap two pins' net assignments
    PinSwap { comp_idx: usize, pin_a: PinIndex, pin_b: PinIndex },
    // NEW: swap all nets between two components
    PartSwap { comp_a: usize, comp_b: usize },
}
```

Move probability for swaps in SA:
```
p_pin_swap = 0.1   // 10% of moves are pin swaps
p_part_swap = 0.05  // 5% of moves are part swaps

// Adjusted total: p_displace + p_swap + p_rotate + p_slide + p_pin_swap + p_part_swap = 1.0
```


## 5. Upstream Spec File Propagation

**This is the critical piece.** When the autoplacer swaps pins or parts,
the schematic must be updated to reflect the new net-to-pin mapping.
Since we operate on spec files (not binaries), this means modifying
the upstream `.schlib-spec` or `.schdoc-spec` files.

### 5.1 Swap Changelog

The autoplacer produces a swap changelog alongside the placement result:

```rust
pub struct SwapChangelog {
    pub pin_swaps: Vec<PinSwapRecord>,
    pub part_swaps: Vec<PartSwapRecord>,
    pub gate_swaps: Vec<GateSwapRecord>,
}

pub struct PinSwapRecord {
    pub component: String,      // designator, e.g. "U1"
    pub pin_a: String,          // pin name, e.g. "3"
    pub pin_b: String,          // pin name, e.g. "4"
    pub net_a_before: String,   // "NET_X"
    pub net_b_before: String,   // "NET_Y"
    pub hpwl_improvement_mm: f64,
}

pub struct PartSwapRecord {
    pub comp_a: String,         // "R1"
    pub comp_b: String,         // "R2"
    pub nets_swapped: Vec<(String, String)>,  // [(pin_name, old_net, new_net), ...]
    pub hpwl_improvement_mm: f64,
}

pub struct GateSwapRecord {
    pub component: String,      // "U1" (multi-gate IC)
    pub gate_a: String,         // gate identifier
    pub gate_b: String,
    pub pins_swapped: Vec<(String, String)>,  // (pin_a, pin_b) pairs
    pub hpwl_improvement_mm: f64,
}
```

### 5.2 Schematic Spec Rewriting

The swap changelog drives modifications to the upstream schematic spec:

```
Pipeline:
  1. Autoplacer runs, produces SwapChangelog
  2. For each pin swap:
     - Find the .schdoc-spec file containing the component
     - Update net label assignments on the swapped pins
  3. For each part swap:
     - Find both components in the .schdoc-spec
     - Exchange their net connections
  4. Write updated .schdoc-spec files
```

### 5.3 Spec File Net Assignment Syntax

The schematic spec needs syntax for pin-to-net assignment (if not already
present). The autoplacer writes:

```
// In board.schdoc-spec (or a generated swap overlay file):

// Pin swap: U1 pins 3 and 4 exchanged nets
swap_applied U1 {
    pin 3 { net: "NET_Y" }   // was NET_X
    pin 4 { net: "NET_X" }   // was NET_Y
}

// Part swap: R1 and R2 exchanged all nets
swap_applied R1 {
    pin 1 { net: "NET_C" }   // was NET_A
    pin 2 { net: "NET_D" }   // was NET_B
}
swap_applied R2 {
    pin 1 { net: "NET_A" }   // was NET_C
    pin 2 { net: "NET_B" }   // was NET_D
}
```

### 5.4 Swap Overlay File (Preferred Approach)

Rather than modifying the user's schematic spec directly, produce a
**swap overlay file** that is imported:

```
// board-swaps.schdoc-spec (auto-generated by autoplacer)
// DO NOT EDIT — regenerated by `altium placement autoplace`

// Pin swaps (3 total, saved 42mm HPWL)
swap U1 { pin 3: NET_Y, pin 4: NET_X }

// Part swaps (1 total, saved 28mm HPWL)
swap R1, R2   // exchange all net assignments

// Gate swaps (1 total, saved 15mm HPWL)
swap U3 { gate_a: [1, 2, 3], gate_b: [4, 5, 6] }
```

The main `.pcbdoc-spec` imports this:

```
placement {
    import "board-swaps.schdoc-spec"   // pin/part swap assignments
    // ... placement directives ...
}
```

This approach:
- **Never modifies user-written files** (only generates new overlay)
- **Idempotent** — re-running autoplacer regenerates the overlay
- **Reviewable** — user can see exactly what swaps were made
- **Revertible** — delete the import line to undo all swaps


## 6. Swap Validation

### 6.1 Correctness Invariants

Before applying any swap, verify:

1. **Same swap group**: Both pins/parts MUST be in the same swap group
2. **Compatible pin count**: Part-swapped components must have identical
   pin counts and swap group structure
3. **No power/ground pins**: Never swap power or ground nets (even if
   pins are in a swap group, filter out VCC/GND nets)
4. **No differential pairs**: Don't break diff pair assignments (check
   `swap_id_pair` consistency)
5. **Footprint compatibility**: Part-swapped components must have the
   same footprint (different footprints = different pad geometries)

### 6.2 Net Connectivity Preservation

After all swaps, verify:
- Every net still has the same number of connections
- No net was accidentally duplicated or dropped
- The netlist is still topologically equivalent to the original
  (same connectivity graph, just different pin assignments)

```rust
fn verify_swap_integrity(
    original_netlist: &Netlist,
    swapped_netlist: &Netlist,
) -> Result<(), SwapError> {
    // Net count must be unchanged
    assert_eq!(original_netlist.nets.len(), swapped_netlist.nets.len());

    // Each net must have the same pin count
    for net in &original_netlist.nets {
        let swapped = swapped_netlist.net_by_name(&net.name)
            .ok_or(SwapError::NetLost(net.name.clone()))?;
        if net.pins.len() != swapped.pins.len() {
            return Err(SwapError::PinCountChanged(net.name.clone()));
        }
    }

    Ok(())
}
```


## 7. Spec Language Extensions

### 7.1 New `pcbdoc-spec` Properties

```
placement {
    // Enable/disable swap optimization
    allow_pin_swap: true       // default: true if swap groups exist
    allow_part_swap: true      // default: true if swap groups exist
    allow_gate_swap: true      // default: true if swap groups exist

    // Import swap overlay from previous run
    import "board-swaps.schdoc-spec"

    // Lock specific components against part swapping
    place R5 {
        autoplace: true
        no_part_swap: true     // don't swap this component's nets
    }

    // Lock specific pins against pin swapping
    place U1 {
        autoplace: true
        no_pin_swap: [3, 7]    // pins 3 and 7 must keep their nets
    }
}
```

### 7.2 Swap Report in CLI Output

```
$ altium placement autoplace board.pcbdoc-spec

PLACEMENT RESULT
  14 components placed, HPWL: 1,234mm

SWAP OPTIMIZATION
  Pin swaps:  3 applied, saved 42mm HPWL (3.4%)
    U1: pin 3 ↔ pin 4 (NET_X ↔ NET_Y, saved 18mm)
    U1: pin 7 ↔ pin 8 (NET_P ↔ NET_Q, saved 12mm)
    U3: pin 1 ↔ pin 2 (SIG_A ↔ SIG_B, saved 12mm)

  Part swaps: 1 applied, saved 28mm HPWL (2.3%)
    R1 ↔ R2 (saved 28mm)

  Gate swaps: 1 applied, saved 15mm HPWL (1.2%)
    U3: gate A ↔ gate B (saved 15mm)

  Total swap improvement: 85mm (6.9% of original HPWL)

  Swap overlay written to: board-swaps.schdoc-spec
```


## 8. Integration with Existing Autoplacer Pipeline

```
Phase 0: Clustering (existing)
    ↓
Phase 1: Analytical placement (existing, solverang)
    ↓
Phase 2: Legalization (existing)
    ↓
Phase 2.5: Part swap pass (NEW)
    │  For each part swap group:
    │    Try all pairwise swaps, accept improvements
    │  This reassigns logic to better-positioned components
    ↓
Phase 3: SA with pin swap moves (ENHANCED)
    │  SA move types now include PinSwap and PartSwap
    │  Pin swaps are cheap (O(1)) and explored frequently
    ↓
Phase 4: Final refinement (existing, solverang)
    ↓
Phase 4.5: Final swap sweep (NEW)
    │  Greedy pass over all pin swap groups
    │  Accept any swap that reduces HPWL
    │  This catches improvements from Phase 4 position changes
    ↓
Phase 5: DRC verification (existing)
    ↓
Output: Updated .pcbdoc-spec + board-swaps.schdoc-spec
```


## 9. Data Flow

```
┌─────────────────────────────────────────────────────────────────┐
│  INPUT FILES                                                     │
│                                                                   │
│  board.pcbdoc-spec ── placement constraints, autoplace directives │
│  board.PcbDoc ─────── netlist, board outline, component positions │
│  board.SchLib ─────── pin swap groups (swap_id_pin/part/pair)    │
│  board.SchDoc ─────── net-to-pin assignments (current mapping)   │
└───────────────────────────────┬─────────────────────────────────┘
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│  AUTOPLACER                                                       │
│                                                                   │
│  1. Extract PcbIr from PcbDoc                                    │
│  2. Build SwapModel from SchLib pin swap groups                  │
│  3. Run placement pipeline (Phases 0-5)                          │
│  4. Evaluate + apply swaps at Phases 2.5, 3, 4.5                │
│  5. Produce SwapChangelog                                        │
└───────────────────────────────┬─────────────────────────────────┘
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│  OUTPUT FILES                                                     │
│                                                                   │
│  board.pcbdoc-spec ── updated with explicit positions            │
│  board-swaps.schdoc-spec ── pin/part/gate swap overlay (NEW)    │
│  board-placement.json ── iteration snapshots for viewer          │
└─────────────────────────────────────────────────────────────────┘
```


## 10. Implementation Priority

### Milestone 1: Pin Swap in Final Sweep (simplest, highest impact)
- After Phase 4, greedy sweep over all pin swap groups
- Accept any swap that reduces HPWL
- Write swap changelog to stdout
- **Effort**: 1-2 days

### Milestone 2: Part Swap Pass
- After Phase 2, try all pairwise part swaps
- Write swap overlay file
- **Effort**: 2-3 days

### Milestone 3: SA Pin Swap Moves
- Add PinSwap as SA move type
- Integrate into move probability schedule
- **Effort**: 1-2 days (after SA is implemented)

### Milestone 4: Upstream Spec Rewriting
- Parse/rewrite .schdoc-spec with swap overlay
- CLI `--swap-overlay` flag
- Import mechanism in pcbdoc-spec
- **Effort**: 1 week

### Milestone 5: Gate Swap
- Detect multi-gate ICs from swap_id_part within same component
- Apply gate swap logic (same as part swap, within one component)
- **Effort**: 2-3 days
