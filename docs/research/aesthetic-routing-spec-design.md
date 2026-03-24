# Aesthetic Routing & Spec Language Design

Research synthesis from 10 parallel research agents covering: post-route optimization
algorithms, parallel trace bundling, trace alignment, spacing/symmetry, commercial tool
aesthetics, bus type presets, CSS-like constraint DSLs, DFM integration, fab profiles,
and quality tier pipelines.

## Design Philosophy

Three principles from the research:

1. **Spec says WHAT, router says HOW** — the spec declares semantic identity ("this is a
   DDR4 data bus", "this board targets JLCPCB"), the router derives implementation
   (pitch, flow direction, trace width, corridors).

2. **Presets encode domain knowledge** — bus types (DDR3/USB/PCIe), fab profiles
   (JLCPCB/Sierra), and quality tiers (draft/production) carry dozens of parameters each.
   The user writes one line; the router gets a complete constraint set.

3. **Net classes are CSS** — classes define routing style properties that cascade to member
   nets/buses. Multiple classes can be applied. Most-specific wins.

---

## I. Spec Language Extensions

### 1. `bus` Block (Semantic Bus Declaration)

Declares an ordered group of nets as a specific bus type. The type carries all routing
knowledge as a built-in preset.

```
bus "DDR4_DQ_BYTE0" {
    type: DDR4_DQ
    nets: [DQ0, DQ1, DQ2, DQ3, DQ4, DQ5, DQ6, DQ7]
    strobe: DQS0                    // DDR-specific: associated strobe pair
}

bus "ETH_ADDR" {
    type: parallel                   // generic parallel bus
    nets: [ETH_ADDR0, ..., ETH_ADDR14]
}

bus "SPI_BOOT" {
    type: SPI
    nets: [BOOT_SPI_CLK, BOOT_SPI_MOSI, BOOT_SPI_MISO, BOOT_SPI_CS]
}
```

**Bus without a type** defaults to `parallel` (route together, equalize spacing, no
length matching).

**Override preset values** with explicit properties:
```
bus "TIGHT_DDR" {
    type: DDR4_DQ
    nets: [...]
    strobe: DQS0
    max_skew: 0.05mm              // override DDR4 default of 0.127mm
}
```

### 2. Extended `class` Block (CSS-like Routing Style)

Extend the existing `class` to carry routing style properties that cascade to all members.

```
class "high_speed" {
    members: [...]

    // Routing style properties (all optional — cascade from preset/default)
    spacing: equalize              // equalize | minimize | maximize | hug
    gloss: aggressive              // none | light | aggressive
    max_vias: 4                    // per-net via budget (null = unlimited)
    corner_style: rounded          // degree45 | rounded
    impedance: 50                  // target ohms (router computes width from stackup)
}

class "power" {
    members: [GND, "3V3", "5V", "12V"]
    spacing: maximize
    width: 0.5mm
}
```

**Cascading rules** (like CSS specificity):
1. Board-level defaults (lowest priority)
2. Bus type preset defaults
3. Class properties (multiple classes: last in source order wins)
4. Explicit per-bus overrides (highest priority)

**Classes can reference bus names** — the class properties cascade to all nets in the bus:
```
class "memory_interface" {
    members: ["DDR4_DQ_BYTE0", "DDR4_DQ_BYTE1", "DDR4_ADDR"]
    gloss: aggressive
}
```

### 3. `fab` Declaration (Manufacturing Profile)

Replaces dozens of individual DRC rules with a single declaration.

```
board "myboard" {
    fab: "jlcpcb_4layer"           // built-in preset
    signal_layer_count: 4
}
```

Equivalent to manually specifying: min trace 4mil, min space 4mil, min drill 0.3mm,
min annular ring 5mil, max aspect ratio 8:1, no blind/buried vias, IPC Class 2,
+/-10% impedance tolerance, and the JLCPCB JLC2313 stackup dimensions.

**Override specific fab values:**
```
board "myboard" {
    fab: "jlcpcb_4layer"
    min_trace: 5mil                // more conservative than fab minimum
    impedance_tolerance: 8         // tighter than default 10%
}
```

### 4. `routing.aesthetics` Block (Quality Level)

Single knob for optimization pipeline control.

```
routing {
    solution: "board.routes"

    aesthetics {
        quality: production        // draft | standard | production | showroom
    }
}
```

Quality tiers map to specific optimization passes (from commercial tool research):

| Tier | Time Budget | Passes |
|------|-------------|--------|
| `draft` | <5s | Merge segments, basic corners, quick via reduction |
| `standard` | <30s | + 3-pass via optimization, 3-pass straightening, pad entry |
| `production` | <5min | + spread/center, rip-up worst 10%, 5x outer cycles |
| `showroom` | <30min | + global re-optimization, symmetry, 10x outer cycles |

### 5. `stackup` Block (Impedance-Driven Routing)

Bridge between fab profile (materials) and impedance control (computed trace widths).

```
stackup {
    // Layer order with dielectric properties
    layer "Top" { copper: 1oz }
    dielectric { height: 0.2mm, er: 4.4 }
    layer "Inner1" { copper: 0.5oz }
    dielectric { height: 1.0mm, er: 4.4 }
    layer "Inner2" { copper: 0.5oz }
    dielectric { height: 0.2mm, er: 4.4 }
    layer "Bottom" { copper: 1oz }
}
```

If `fab` is specified, the stackup can be omitted — the fab profile carries standard
stackup dimensions. If both are specified, the explicit stackup overrides.

### 6. `dfm` Block (Manufacturing Optimization)

```
dfm {
    teardrops: enabled             // enabled | curved | disabled
    spacing_margin: 1.25           // route at 1.25x minimum clearance
    acid_trap_threshold: 90        // minimum interior angle (degrees)
    copper_balance_target: 0.5     // 50% copper coverage target
    redundant_vias: enabled        // post-route redundant via insertion
}
```

---

## II. Bus Type Presets (Router-Side Domain Knowledge)

Presets are Rust code in the router, not in the spec language. They encode constraints
from JEDEC/USB-IF/PCI-SIG/IEEE standards.

### Preset Lookup Table (Key Parameters)

| Preset | Z_SE | Z_Diff | Group Skew | Pair Space | Inter Space | Style | Topology |
|--------|------|--------|-----------|------------|-------------|-------|----------|
| DDR3_DQ | 50Ω | — | ±0.635mm | — | 3x | ParBus | P2P |
| DDR3_DQS | — | 100Ω | ±0.635mm | 2x | 3x | Diff | P2P |
| DDR3_CMD | 50Ω | — | ±0.635mm | — | 3x | ParBus | FlyBy |
| DDR4_DQ | 40Ω | — | ±0.127mm | — | 4x | ParBus | P2P |
| DDR4_DQS | — | 80Ω | ±0.127mm | 2x | 4x | Diff | P2P |
| DDR5_DQ | 36Ω | — | ±0.051mm | — | 5x | ParBus | P2P |
| USB2 | 45Ω | 90Ω | — | 2x | 3x | Diff | P2P |
| USB3_GEN2 | 42.5Ω | 85Ω | — | 2x | 5x | Diff | P2P |
| PCIE_GEN4 | 42.5Ω | 85Ω | — | 2x | 5x | Diff | P2P |
| PCIE_GEN5 | 42.5Ω | 85Ω | — | 2x | 6x | Diff | P2P |
| RGMII | 50Ω | — | ±0.254mm | — | 3x | ParBus | P2P |
| SPI | 50Ω | — | ±1.27mm* | — | 2x | ParBus | P2P |
| I2C | — | — | — | — | 2x | Indep | Bus |
| LVDS | 50Ω | 100Ω | — | 2x | 4x | Diff | P2P |
| CAN | 60Ω | 120Ω | — | 2x | 2x | Diff | Bus |
| eMMC_HS400 | 50Ω | — | ±0.127mm | — | 3x | ParBus | P2P |

*SPI group skew only matters at >25 MHz clock speeds.

Full preset data in `docs/research/bus-type-presets.md` covers 35+ presets with all
parameters from the relevant standards.

---

## III. Fab Profile Presets

### Budget Fabs

| Profile | Trace/Space | Drill | Annular Ring | Aspect | Blind/Buried | Impedance |
|---------|------------|-------|-------------|--------|-------------|-----------|
| jlcpcb_2layer | 5/5 mil | 0.3mm | 5mil | 6:1 | No | ±10% |
| jlcpcb_4layer | 4/4 mil | 0.3mm | 5mil | 8:1 | No | ±10% |
| jlcpcb_6layer | 3.5/3.5 mil | 0.25mm | 4mil | 8:1 | No | ±10% |
| pcbway_standard | 5/5 mil | 0.2mm | 6mil | 10:1 | No | ±10% |
| pcbway_advanced | 3.5/3.5 mil | 0.15mm | 4mil | 12:1 | Yes | ±8% |
| oshpark_2layer | 6/6 mil | 0.254mm | 5mil | 6:1 | No | ±10% |
| oshpark_4layer | 5/5 mil | 0.254mm | 5mil | 6:1 | No | ±10% |

### Mid-Tier / High-End Fabs

| Profile | Trace/Space | Drill | Annular Ring | Aspect | Blind/Buried | Impedance |
|---------|------------|-------|-------------|--------|-------------|-----------|
| eurocircuits_standard | 4/4 mil | 0.2mm | 5mil | 8:1 | No | ±10% |
| eurocircuits_advanced | 3/3 mil | 0.15mm | 4mil | 10:1 | Yes | ±5% |
| advanced_circuits_std | 4/4 mil | 0.2mm | 5mil | 8:1 | No | ±10% |
| advanced_circuits_adv | 3/3 mil | 0.15mm | 3mil | 12:1 | Yes | ±5% |
| sierra_standard | 3/3 mil | 0.2mm | 3mil | 10:1 | Yes | ±5% |
| sierra_hdi | 2/2 mil | 0.1mm | 2mil | 15:1 | Yes (stacked) | ±5% |
| wurth_standard | 4/4 mil | 0.2mm | 5mil | 8:1 | Yes | ±8% |

Full profile data with stackup dimensions in `docs/routing/fab-profiles-research.md`.

---

## IV. Post-Route Optimization Pipeline

### Recommended Pass Order (from FreeRouting, KiCad, Cadence research)

```
optimize_solution(quality_tier):
    1. Segment merging              // always — foundation for all passes
    2. Staircase elimination        // existing
    3. Enhanced pull-tight          // step-reduction [8,4,2,1], replaces rubber_band
    4. Jog elimination              // short segments between parallel neighbors
    5. Detour elimination           // rip-up/reroute nets with length > 1.5× manhattan
    6. Via nudging                  // gradient-descent via positions
    7. Via elimination              // single-layer reroute attempts
    8. Corner conversion            // existing, on cleaner geometry
    9. Pad entry optimization       // center and straighten last segment before pad
    10. Spacing equalization        // force-directed or LP spreading
    11. Segment merging             // final consolidation
```

### Passes per Quality Tier

| Pass | draft | standard | production | showroom |
|------|-------|----------|------------|----------|
| Segment merge | 1 | 1 | 1 | 1 |
| Staircase | 1 | 1 | 1 | 1 |
| Pull-tight | 1 step | 3 steps, 3 iter | 5 steps, 5 iter | 10 steps, 10 iter |
| Jog elimination | — | 1 | 3 | 5 |
| Detour elimination | — | — | top 10% nets, 3 cycles | all nets, 5 cycles |
| Via nudging | — | 1 | 3 | 5 |
| Via elimination | 1 | 3 | 5+3 post-spread | 10+5 post-spread |
| Corners | 1 | 1 | 2 | 5 |
| Pad entry | — | 1 | 2 | 5 |
| Spacing equalization | — | — | 3 | 5 |
| Outer cycles | 1 | 2 | 5 | 10 |
| Convergence threshold | — | 1% | 0.5% | 0.1% |
| Time limit | 5s | 30s | 5min | 30min |

### Key Ordering Principles (from literature)

1. **Via optimization before straightening** — removing vias opens straightening paths
2. **Straightening before spreading** — shorten first, then distribute space
3. **Re-run via opt after spreading** — spreading creates new via elimination opportunities
4. **Rip-up only at production+** — expensive, only helps where initial topology was wrong
5. **Segment merge first AND last** — cleans up artifacts from both PathFinder and passes

---

## V. A* Cost Function Extensions

### Current Cost Function
```
C(n) = base_cost × direction_penalty × corridor_penalty
     + hist_weight × history[n]
     + pres_fac × max(0, usage[n] - 1)
```

### Extended Cost Function (from DFM + aesthetics research)
```
C(n) = base_cost
     × direction_penalty(layer, move_dir)      // existing: 1.0 or 1.5
     × corridor_penalty(global_route)           // existing: 1.0 or 1.5
     × bend_cost(parent_dir, move_dir)          // NEW: acid trap prevention
     + hist_weight × (history[n] + edge_history[p→n])  // existing
     + pres_fac × max(0, usage[n] - 1)          // existing
     + via_penalty(n)                            // EXTEND: mfg-aware, not flat
     + spacing_pressure(n)                       // NEW: repel from nearby traces
     + centering_pressure(n)                     // NEW: repel from obstacles
     + ref_plane_penalty(n)                      // NEW: reference plane continuity
     + bus_affinity(n, bus_group)                // NEW: parallel routing bonus
     + layer_balance(n)                          // NEW: copper density equalization
```

**New terms explained:**

- **bend_cost**: Multiplicative. 1.0 for straight, 1.2 for ≥90°, 5.0 for 45-90° (acid
  trap risk), 100.0 for <45° (hairpin). From DFM acid trap research.

- **spacing_pressure**: Additive. `k_spacing / distance²` from each nearby trace.
  Naturally spreads traces apart during routing. From force-directed spacing research.

- **centering_pressure**: Additive. `k_center / distance²` from each nearby obstacle.
  Centers traces in channels. From Voronoi/medial-axis research.

- **ref_plane_penalty**: Additive. High cost (50-100x) for cells over plane splits/voids.
  From impedance/SI research.

- **bus_affinity**: Multiplicative discount. 0.3-0.5x for cells adjacent to already-routed
  bus group members. Heavy penalty for crossing another group member's lane. From
  bus routing research.

- **layer_balance**: Additive. `k_balance × |density[layer] - target|`. Steers routing
  toward under-utilized layers. From copper balance research.

**DFM terms must be additive with constant weight** (not iteration-adaptive) to avoid
disrupting PathFinder convergence. Bus affinity is multiplicative because it modifies
the base movement cost, not congestion negotiation.

---

## VI. DFM Post-Route Pipeline

After routing optimization, run DFM-specific passes:

```
dfm_post_route():
    1. Teardrop insertion           // at all pad/via junctions
    2. Return via insertion         // at reference plane changes
    3. Redundant via insertion      // MIS formulation, where space permits
    4. Copper pour filling          // boolean polygon ops, thermal reliefs
    5. Via stitching                // ground + thermal patterns
    6. Copper thieving              // density equalization fill
    7. DFM validation               // final check against fab profile
```

---

## VII. CSS-like Property Reference

Properties settable per net class, from cross-tool analysis (Altium, Allegro, KiCad,
Xpedition):

### Geometry
- `width`: min/max/preferred trace width (mm)
- `clearance`: copper-to-copper spacing (mm)
- `corner_style`: degree45 | rounded
- `neckdown`: percent or min width near pads
- `routing_layers`: which copper layers allowed

### Electrical
- `impedance`: target ohms
- `max_length` / `min_length`: absolute length bounds (mm)
- `max_skew`: within-group length matching tolerance (mm)
- `max_parallel_run`: crosstalk guard distance (mm)

### Router Tuning
- `spacing`: equalize | minimize | maximize | hug
- `gloss`: none | light | aggressive
- `max_vias`: per-net via budget
- `via_cost`: A* penalty weight override
- `routing_priority`: net ordering for autorouter

---

## VIII. Complete Example Spec

```
board "stm32_ddr3" {
    fab: "jlcpcb_4layer"
    signal_layer_count: 4
}

// --- Style classes ---
class "power" {
    members: [GND, "3V3", "1V35", VTT]
    spacing: maximize
}

// --- Buses ---
bus "DDR3_DQ0" {
    type: DDR3_DQ
    nets: [DQ0, DQ1, DQ2, DQ3, DQ4, DQ5, DQ6, DQ7]
    strobe: DQS0
}

bus "DDR3_DQ1" {
    type: DDR3_DQ
    nets: [DQ8, DQ9, DQ10, DQ11, DQ12, DQ13, DQ14, DQ15]
    strobe: DQS1
}

bus "DDR3_ADDR" {
    type: DDR3_CMD
    nets: [A0, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11, A12,
           BA0, BA1, BA2, RAS, CAS, WE, CKE, ODT, RESET]
}

bus "SPI_FLASH" {
    type: SPI
    nets: [FLASH_CLK, FLASH_MOSI, FLASH_MISO, FLASH_CS]
}

// --- Diff pairs (existing syntax, unchanged) ---
differential_pair "DDR3_CLK" { positive_net: CLK_P, negative_net: CLK_N }
differential_pair "DDR3_DQS0" { positive_net: DQS0_P, negative_net: DQS0_N }
differential_pair "DDR3_DQS1" { positive_net: DQS1_P, negative_net: DQS1_N }
differential_pair "USB" { positive_net: USB_DP, negative_net: USB_DN }

// --- DFM ---
dfm {
    teardrops: enabled
    spacing_margin: 1.25
}

// --- Routing ---
routing {
    solution: "board.routes"
    aesthetics {
        quality: production
    }
}
```

---

## IX. Key Academic References

| Topic | Paper/Source | Year |
|-------|-------------|------|
| Negotiated routing (PathFinder) | McMurchie & Ebeling, FPGA | 1995 |
| Topological routing | Dayan PhD thesis (rubber-band) | 1997 |
| Wire spreading (LP) | Cho et al., ISPD | 2005 |
| Bus routing + lane swap | Wu & Wong, IEEE TCAD | 2013 |
| RSMT (FLUTE) | Chu & Wong, DAC | 2004 |
| Via minimization | Chang & Du, IEEE TCAD | 1991 |
| Compaction | Schlag et al., IEEE TCAD | 1987 |
| Equal-spacing (force) | Lienig & Thulasiraman | 1997 |
| Symmetric routing | Lin et al., ISPD | 2009 |
| Global routing | Pan & Chu, FastRoute | 2006 |

### Open-Source References
| Tool | Language | Key Feature | Files to Study |
|------|----------|------------|----------------|
| KiCad PNS | C++ | Push-and-shove, optimizer | `pns_optimizer.cpp` |
| FreeRouting | Java | Multi-pass optimization | `BatchOptRoute.java` |
| TopoR | Commercial | Topological routing | (Dayan thesis) |

---

## X. Implementation Priority

### Phase 1: Spec Language (enables everything else)
1. `bus` block — parser, compiler, model, IR
2. Extended `class` — add style properties to existing class
3. `fab` declaration — preset profiles
4. `routing.aesthetics.quality` — single quality knob

### Phase 2: Router Core (A* modifications)
5. Bus-aware A* cost (affinity bonus, crossing penalty)
6. Spacing/centering pressure in A* cost
7. Manufacturing-aware via cost (not flat 10.0)
8. Bend cost for acid trap prevention

### Phase 3: Post-Route Pipeline
9. Segment merging pass
10. Enhanced pull-tight (step-reduction)
11. Jog elimination
12. Via optimization (nudge + eliminate)
13. Pad entry cleanup
14. Spacing equalization (force-directed)

### Phase 4: DFM Integration
15. Teardrop insertion
16. Fab profile validation
17. Copper balance tracking
18. Redundant via insertion

### Phase 5: Advanced
19. Detour elimination (per-net reroute)
20. Multi-pass glossing
21. Bus parallel alignment post-pass
22. Symmetry optimization
