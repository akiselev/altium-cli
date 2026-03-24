# Fab Profile Preset System -- Research

Research into PCB fabrication house capabilities to support a `fab: "jlcpcb_4layer"`
preset system in the autorouter spec language.

## Goal

Enable users to write:
```
board {
    fab: "jlcpcb_standard"
    layers: 4
}
```

And have the router automatically derive all manufacturing constraints (min trace, min
space, min drill, annular ring, via rules, solder mask dam, etc.) without manually
specifying dozens of design rules.

---

## Fab Capability Matrix

All values are minimums unless stated otherwise. "Recommended" values are what the fab
suggests for reliable yield; "absolute minimum" is the hard limit they will attempt.

### Budget Tier

#### JLCPCB

| Parameter | 1-2 Layer (1oz) | 4 Layer (1oz) | 6+ Layer (1oz) | 2oz Copper |
|---|---|---|---|---|
| Min trace width | 5 mil / 0.127 mm | 4 mil / 0.1 mm | 3.5 mil / 0.09 mm | 8 mil / 0.2 mm |
| Min spacing | 5 mil / 0.127 mm | 4 mil / 0.1 mm | 3.5 mil / 0.09 mm | 8 mil / 0.2 mm |
| Min drill (PTH) | 0.3 mm / 12 mil | 0.2 mm / 8 mil | 0.2 mm / 8 mil | 0.3 mm / 12 mil |
| Min annular ring | 0.15 mm / 6 mil | 0.15 mm / 6 mil | 0.15 mm / 6 mil | 0.15 mm / 6 mil |
| Min via pad dia | 0.6 mm / 24 mil | 0.45 mm / 18 mil | 0.45 mm / 18 mil | 0.6 mm / 24 mil |
| Min via-to-via | 0.254 mm / 10 mil | 0.127 mm / 5 mil | 0.127 mm / 5 mil | 0.254 mm / 10 mil |
| Via-in-pad (POFV) | No (extra cost) | No (extra cost) | Yes (free, resin-filled copper-capped) | No |
| Blind/buried vias | No | No | No | No |
| Solder mask dam | 0.1 mm / 4 mil | 0.1 mm / 4 mil | 0.1 mm / 4 mil | 0.1 mm / 4 mil |
| Impedance tolerance | +/-10% | +/-10% | +/-10% | +/-10% |
| Layer count | 1-2 | 4 | 6-20 (up to 32) | same |
| Copper weights | 1oz, 2oz outer; 0.5oz, 1oz inner | same | same | -- |
| Board thickness | 0.4-2.0 mm (std 1.6) | 0.8-2.0 mm | 0.8-2.0 mm | same |
| Aspect ratio | <10:1 | <10:1 | <10:1 | <10:1 |
| Board edge clearance | 0.3 mm / 12 mil | 0.3 mm / 12 mil | 0.3 mm / 12 mil | 0.3 mm / 12 mil |

**Standard 4-layer stackup (1.6mm):**
- L1 copper: 35 um (1oz)
- Prepreg 7628: 0.21 mm
- L2 copper: 15.2 um (0.5oz inner)
- Core: ~1.065 mm
- L3 copper: 15.2 um (0.5oz inner)
- Prepreg 7628: 0.21 mm
- L4 copper: 35 um (1oz)
- Core material: NP-155F (Nan Ya), Dk ~4.4

**DFM notes:**
- 3.0-3.5 mil trace/space on multilayer incurs +20% surcharge (4-8L) or +30% (10+L)
- No blind or buried vias
- Drill sizes in 0.05 mm increments
- PTH tolerance: +0.13/-0.08 mm
- Via pad = drill + 0.3 mm minimum (annular ring rule)

#### PCBWay

| Parameter | Standard (1oz) | Advanced (HDI) |
|---|---|---|
| Min trace width | 4 mil / 0.1 mm | 3 mil / 0.076 mm (2/2 mil partial) |
| Min spacing | 4 mil / 0.1 mm | 3 mil / 0.076 mm |
| Min drill (mechanical) | 0.15 mm / 6 mil | 0.15 mm / 6 mil |
| Min drill (laser) | N/A | 0.076 mm / 3 mil |
| Min annular ring | 0.15 mm / 6 mil (std); 0.127 mm / 5 mil (via) | 0.076 mm / 3 mil |
| Via-in-pad | Yes | Yes |
| Blind/buried vias | No (standard) | Yes (HDI 1+N+1 up to 7+N+7) |
| Solder mask dam | 3 mil / 0.076 mm (green); 4 mil / 0.1 mm (black) | 3 mil |
| Impedance tolerance | +/-10% (50 ohm and below: +/-5 ohm) | +/-10% |
| Layer count | 1-14 | up to 64 |
| Copper weights | 1-8oz outer; 1-4oz inner | same |
| Board thickness | 0.2-3.2 mm | 0.21-6.0 mm |
| Aspect ratio | 10:1 | 12:1 |

**DFM notes:**
- Recommended trace/space: 6 mil / 0.15 mm for cost-effective manufacturing
- IPC Class 2 standard; IPC Class 3 available (advanced)
- Laser drill min 4 mil, max 8 mil for standard HDI
- Min prepreg thickness for laser blind via: 0.06 mm
- Min core thickness: 0.1 mm

#### OSH Park

| Parameter | 2-Layer | 4-Layer |
|---|---|---|
| Min trace width | 6 mil / 0.1524 mm | 5 mil / 0.127 mm |
| Min spacing | 6 mil / 0.1524 mm | 5 mil / 0.127 mm |
| Min drill | 10 mil / 0.254 mm | 10 mil / 0.254 mm |
| Min annular ring | 5 mil / 0.127 mm | 4 mil / 0.1016 mm |
| Via-in-pad | No | No |
| Blind/buried vias | No | No |
| Solder mask dam | ~4 mil (estimated) | ~4 mil (estimated) |
| Impedance tolerance | Not guaranteed | Not guaranteed (FR408 enables it) |
| Layer count | 2 | 4 |
| Copper weight | 1oz | 1oz outer, 0.5oz inner |
| Board thickness | 63 mil / 1.6 mm | 63 mil / 1.6 mm |
| Surface finish | ENIG | ENIG |
| Board edge keepout | 15 mil / 0.381 mm | 15 mil / 0.381 mm |

**4-layer stackup:**
- Substrate: FR408-HR (Tg 190, Dk ~3.66 at 1GHz)
- Prepreg: 7.87 mil / 0.2 mm
- Core Er: ~4.0; Prepreg Er: ~3.3

**DFM notes:**
- Purple soldermask, ENIG finish (no options)
- No via-in-pad, no blind/buried vias
- Internal layer clearance: 10 mil / 0.254 mm
- Min slot: 20 mil / 0.508 mm
- Max board: 16x22 inches

#### AllPCB

| Parameter | Standard |
|---|---|
| Min trace width | 4 mil / 0.1 mm (6 mil recommended) |
| Min spacing | 4 mil / 0.1 mm (6 mil recommended) |
| Min drill | 0.2 mm / 8 mil |
| Min annular ring | 0.153 mm / 6 mil |
| Via-in-pad | Not specified |
| Blind/buried vias | Not specified |
| Solder mask dam | 0.1 mm / 4 mil |
| Impedance tolerance | Not specified |
| Layer count | 1-14 |
| Copper weights | 0.5-2oz inner; 1-2oz outer |
| Board thickness | 0.4-3.2 mm |
| Aspect ratio | 6:1 (7-8:1 adds lead time) |
| Board edge clearance | 0.3 mm / 12 mil |

**DFM notes:**
- Via space same net: >= 8 mil; different net: >= 17 mil
- PTH deviation: +/-3 mil
- NPTH deviation: +/-2 mil
- Min slot width: 0.6 mm
- Conservative aspect ratio (6:1 vs industry standard 10:1)

---

### Mid-Tier

#### Eurocircuits

| Parameter | Standard (Pattern Class 6) | Advanced (Pattern Class 8+) |
|---|---|---|
| Min trace width | 0.15 mm / 6 mil | 0.10 mm / 4 mil |
| Min spacing | 0.15 mm / 6 mil | 0.10 mm / 4 mil |
| Min drill (PTH) | 0.35 mm / 14 mil | 0.20 mm / 8 mil |
| Min annular ring (outer) | 0.175 mm / 7 mil | 0.10 mm / 4 mil |
| Min annular ring (inner) | 0.125 mm / 5 mil | 0.075 mm / 3 mil |
| Via-in-pad | No (standard pool) | Consult |
| Blind/buried vias | No (standard pool) | Available (custom) |
| Solder mask dam | ~0.1 mm / 4 mil | ~0.075 mm / 3 mil |
| Impedance tolerance | +/-10% (Defined Impedance pool) | +/-10% |
| Layer count | 1-8 (pool); up to 16+ custom | up to 16+ |
| Copper weights | 18-70 um (0.5-2oz) | same |
| Board thickness | 0.8-2.4 mm | custom |

**DFM notes:**
- Uses Pattern Class + Drill Class classification system
- Standard pool: Pattern Class 6 (0.15mm), Drill Class C (0.35mm)
- Defined Impedance pool: predefined 4/6/8-layer FR-4 stackups with verified Dk
- Annular ring for NPTH: minimum 0.30 mm / 12 mil
- EU-based, IPC Class 2 default

#### Bay Area Circuits

| Parameter | Standard | Advanced |
|---|---|---|
| Min trace width | 4 mil / 0.1 mm | 2 mil / 0.051 mm |
| Min spacing | 4 mil / 0.1 mm | 2 mil / 0.051 mm |
| Min drill (mechanical) | 6 mil / 0.152 mm | 4 mil / 0.1 mm |
| Min drill (laser) | N/A | 3 mil / 0.076 mm |
| Min annular ring | 5 mil / 0.127 mm (Class 2) | 3 mil / 0.076 mm (mech); 1 mil / 0.025 mm (laser) |
| Via-in-pad | Soldermask plugged | Non-conductive or conductive fill |
| Blind/buried vias | No | Yes |
| Solder mask dam | 5 mil / 0.127 mm | 3 mil / 0.076 mm |
| Impedance tolerance | +/-10% | +/-5% |
| Layer count | 1-16 | up to 30 |
| Copper weights | 1-2oz outer; 0.5-2oz inner | 1-5oz outer; 0.3-4oz inner |
| Board thickness | 0.008-0.250 in | 0.008-0.250 in |

**DFM notes:**
- Pad size: hole + 0.015" minimum for reliable manufacturing
- Board edge copper clearance: 0.010" minimum
- Min soldermask/silkscreen feature: 0.003"
- US-based fabrication

#### Advanced Circuits (4PCB)

| Parameter | Standard | Advanced | Development/NPI |
|---|---|---|---|
| Min trace width (1/4-3/8oz) | 3 mil / 0.076 mm | 2 mil / 0.051 mm | 1.5 mil / 0.038 mm |
| Min spacing | 3 mil / 0.076 mm | 2 mil / 0.051 mm | 1.5 mil / 0.038 mm |
| Min drill (mechanical PTH) | 5-6 mil | 5-6 mil | 5-6 mil |
| Min drill (laser) | 4 mil / 0.1 mm | 3 mil / 0.076 mm | 2 mil / 0.051 mm |
| Min annular ring | 8-10 mil / 0.2-0.254 mm | 8-10 mil | 8-10 mil |
| Via-in-pad | Filled and plated-over | same | same |
| Blind/buried vias | Yes | Yes | Yes |
| Solder mask web | 2-4 mil | 2-4 mil | 2-4 mil |
| Impedance tolerance | +/-7-10% | +/-5% | +/-5% |
| Layer count | up to 20 | up to 42+ | 42+ |
| Copper weights | 1/4oz - 4oz | same | same |
| Board thickness | 0.008-0.250 in | same | same |
| Aspect ratio | 10:1 | 12-16:1 | 16:1 |

**DFM notes:**
- Formerly "4PCB", now "AdvancedPCB"
- IPC-A600 Class 2 minimum; Class 3 available
- US-based, multiple facilities
- Via-in-pad min pad: 3-5 mil over drill
- Impedance range: 25-150 ohm

---

### High-End Tier

#### Sierra Circuits

| Parameter | Standard | HDI/Microelectronics |
|---|---|---|
| Min trace width | 3 mil / 0.076 mm | 1.5 mil / 0.038 mm |
| Min spacing | 3 mil / 0.076 mm | 1.5 mil / 0.038 mm |
| Min drill (mechanical) | 6 mil / 0.152 mm | 6 mil / 0.152 mm |
| Min drill (laser) | 3 mil / 0.076 mm | 2 mil / 0.051 mm |
| Min annular ring (through hole) | 5 mil / 0.127 mm | 5 mil / 0.127 mm |
| Min annular ring (blind via) | 2 mil / 0.051 mm | 2 mil / 0.051 mm |
| Min annular ring (buried via) | 2.5 mil / 0.064 mm | 2.5 mil / 0.064 mm |
| Via-in-pad | Yes (filled, capped, plated) | Yes |
| Blind/buried vias | Yes (stacked + staggered) | Yes |
| Solder mask dam (SMT) | 4 mil / 0.102 mm | 3.5 mil / 0.089 mm (BGA) |
| Impedance tolerance | +/-5% with first article | +/-5% |
| Layer count | up to 30 | up to 30 |
| Copper weights | 5um - 3+oz | same |
| Board thickness | 0.005-0.250 in | 0.007-0.250 in |
| Quality standard | IPC Class 2/3 | IPC Class 3 |

**DFM notes:**
- US-based (Silicon Valley), fast-turn prototyping focus
- Conductive and non-conductive filled vias
- Copper plate shut microvias
- Cap plating minimum 12 um / 0.472 mil
- Via protrusion max 50 um / 1.96 mil
- Min 1 mil copper plating inside holes

#### TTM Technologies

| Parameter | Conventional | HDI/Advanced |
|---|---|---|
| Min trace width | 4 mil / 0.1 mm (est) | 2-3 mil (est) |
| Min spacing | 4 mil / 0.1 mm (est) | 2-3 mil (est) |
| Min drill (mechanical) | 6 mil / 0.152 mm | 6 mil |
| Min drill (laser) | Available | Available |
| Min annular ring | Per IPC class | Per IPC class |
| Via-in-pad | Yes (POFV) | Yes |
| Blind/buried vias | Yes (skip microvia) | Yes |
| Impedance tolerance | +/-10% (est) | +/-5% (est) |
| Layer count | up to 60+ | 60+ |
| Copper weights | up to 10oz outer; 12oz inner | same |
| Board thickness | up to 0.450 in | same |
| Aspect ratio | up to 25:1+ | 25:1+ |
| Quality standard | IPC Class 2/3 | IPC Class 3, MIL-PRF-31032 |

**DFM notes:**
- Largest PCB manufacturer in North America
- Military/aerospace grade (MIL-PRF-31032)
- Embedded capacitance materials and planar resistors
- Precision backdrilling for stub length control
- Panel size up to 54 inches
- Production-volume focus (not prototype)

#### Wurth Elektronik

| Parameter | WEdirekt (Online) | Custom |
|---|---|---|
| Min trace width (18um Cu) | 85 um / 3.3 mil | 60 um / 2.4 mil |
| Min trace width (35um Cu) | 100 um / 4 mil | 75 um / 3 mil |
| Min trace width (70um Cu) | 150 um / 6 mil | consult |
| Min spacing (18um Cu) | 85 um / 3.3 mil | 60 um / 2.4 mil |
| Min spacing (35um Cu) | 100 um / 4 mil | 75 um / 3 mil |
| Min spacing (70um Cu) | 192 um / 7.6 mil | consult |
| Min drill | 0.25 mm / 10 mil (online) | consult |
| Annular ring | Pad = Hole + 0.35 mm | consult |
| Solder mask dam | 70 um / 2.8 mil | consult |
| Via-in-pad | Microvia (aspect 1:0.8) | Yes |
| Blind/buried vias | Yes (buried AR max 1:10) | Yes |
| Impedance control | No (online pool) | Yes (custom) |
| Layer count | 1-16 (online up to 8 pool) | 1-16+ |
| Copper weights | 18-105 um (0.5-3oz) | same |
| Board thickness | 0.8-3.2 mm | custom |
| Quality standard | IPC Class 2 (default); Class 3 on request | IPC Class 3 |

**DFM notes:**
- German/EU manufacturer, high reliability focus
- PTH min edge-to-edge: 400 um
- Microvia pad min: 350 um
- FR4 TG150 standard; TG170 available
- Online pool service (WEdirekt) has limited options; custom orders more flexible
- Pad-to-pad spacing >= 170 um (18-35um Cu)

---

## IPC Class 2 vs Class 3 Requirements

IPC-6012 defines three classes of PCB quality:

| Requirement | Class 2 (Commercial) | Class 3 (High Reliability) |
|---|---|---|
| External annular ring min | 90-degree breakout allowed | 2 mil / 0.050 mm (no breakout) |
| Internal annular ring min | Breakout allowed | 1 mil / 0.025 mm (no breakout) |
| Min hole plating thickness | 0.8 mil / 20 um | 1.0 mil / 25 um |
| Conductor width reduction | 20-30% allowed | 10-20% max |
| Practical pad sizing | Via diameter + 8 mil | Via diameter + 10 mil |
| Solder mask registration | Less strict | Tighter alignment required |
| Typical fabricators | JLCPCB, PCBWay, AllPCB | Sierra, TTM, Adv. Circuits, Wurth |

**Router implications:**
- Class 2: Can route tighter (smaller annular rings, more breakout tolerance)
- Class 3: Need larger pads, stricter clearances, higher via cost budget
- Default to Class 2 for budget fabs; Class 3 for high-end fabs

---

## Standard Stackup Dimensions

### 2-Layer 1.6mm

| Layer | Material | Thickness |
|---|---|---|
| Top copper | Cu | 35 um (1oz) |
| Core | FR-4 | ~1.53 mm |
| Bottom copper | Cu | 35 um (1oz) |
| **Total** | | **1.6 mm** |

Dk: ~4.2-4.7 (FR-4)

### 4-Layer 1.6mm (JLCPCB Standard)

| Layer | Material | Thickness |
|---|---|---|
| L1 (Signal) | Cu | 35 um (1oz) |
| Prepreg (7628) | FR-4 | 0.21 mm |
| L2 (GND) | Cu | 15.2 um (0.5oz) |
| Core | FR-4 (NP-155F) | ~1.065 mm |
| L3 (PWR) | Cu | 15.2 um (0.5oz) |
| Prepreg (7628) | FR-4 | 0.21 mm |
| L4 (Signal) | Cu | 35 um (1oz) |
| **Total** | | **~1.6 mm** |

Dk: ~4.4 (NP-155F)

### 4-Layer 1.6mm (OSH Park)

| Layer | Material | Thickness |
|---|---|---|
| L1 (Signal) | Cu | 35 um (1oz) |
| Prepreg | FR408-HR | 0.2 mm (7.87 mil) |
| L2 (GND) | Cu | 17.5 um (0.5oz) |
| Core | FR408-HR | ~1.1 mm |
| L3 (PWR) | Cu | 17.5 um (0.5oz) |
| Prepreg | FR408-HR | 0.2 mm (7.87 mil) |
| L4 (Signal) | Cu | 35 um (1oz) |
| **Total** | | **~1.6 mm** |

Dk: ~3.66 at 1GHz (FR408-HR)

### 6-Layer 1.6mm (Typical)

| Layer | Material | Thickness |
|---|---|---|
| L1 (SIG) | Cu | 35 um (1oz) |
| Prepreg | | 0.1-0.2 mm |
| L2 (GND) | Cu | 17.5 um (0.5oz) |
| Core | | ~0.36 mm |
| L3 (SIG) | Cu | 17.5 um (0.5oz) |
| Prepreg | | 0.1-0.2 mm |
| L4 (SIG) | Cu | 17.5 um (0.5oz) |
| Core | | ~0.36 mm |
| L5 (PWR) | Cu | 17.5 um (0.5oz) |
| Prepreg | | 0.1-0.2 mm |
| L6 (SIG) | Cu | 35 um (1oz) |
| **Total** | | **~1.6 mm** |

---

## Proposed Fab Profile Identifiers

```
# Budget tier
jlcpcb_2layer         # 2L, 5/5 mil, 0.3mm drill, no blind/buried
jlcpcb_4layer         # 4L, 4/4 mil, 0.2mm drill, no blind/buried
jlcpcb_6layer         # 6L, 3.5/3.5 mil, 0.2mm drill, free POFV
pcbway_standard       # 1-14L, 4/4 mil, 0.15mm drill
pcbway_hdi            # HDI, 3/3 mil, laser drill, blind/buried
oshpark_2layer        # 2L, 6/6 mil, 0.254mm drill, ENIG
oshpark_4layer        # 4L, 5/5 mil, 0.254mm drill, FR408, ENIG
allpcb_standard       # 1-14L, 4/4 mil, 0.2mm drill

# Mid-tier
eurocircuits_standard # pool class 6, 6/6 mil, 0.35mm drill
eurocircuits_advanced # class 8+, 4/4 mil, 0.2mm drill
bayarea_standard      # 4/4 mil, 0.152mm drill, Class 2
bayarea_advanced      # 2/2 mil, laser, blind/buried, Class 2/3
advanced_circuits_std # 3/3 mil, blind/buried, Class 2
advanced_circuits_hdi # 2/2 mil, laser, filled via-in-pad

# High-end
sierra_standard       # 3/3 mil, blind/buried, Class 2/3
sierra_hdi            # 1.5/1.5 mil, laser, stacked vias, Class 3
ttm_standard          # 4/4 mil, 60+ layers, Class 2/3
ttm_advanced          # 2-3/2-3 mil, 25:1 AR, mil-spec
wurth_standard        # 4/4 mil (1oz), Class 2
wurth_custom          # 3/3 mil, impedance controlled, Class 3
```

---

## Mapping to Router Parameters

Each fab profile maps to these DRC/routing policy parameters:

```rust
/// Manufacturing constraints derived from a fab profile.
struct FabProfile {
    // -- Trace rules --
    /// Minimum trace width in mm.
    min_trace_width_mm: f64,
    /// Minimum trace-to-trace spacing in mm.
    min_spacing_mm: f64,
    /// Recommended (preferred) trace width in mm.
    preferred_trace_width_mm: f64,

    // -- Drill rules --
    /// Minimum mechanical drill diameter in mm.
    min_drill_mm: f64,
    /// Minimum laser drill diameter in mm (None = no laser drill).
    min_laser_drill_mm: Option<f64>,
    /// Maximum aspect ratio (board thickness / drill diameter).
    max_aspect_ratio: f64,

    // -- Via rules --
    /// Minimum annular ring in mm.
    min_annular_ring_mm: f64,
    /// Minimum via pad diameter in mm (= min_drill + 2 * min_annular_ring).
    min_via_pad_mm: f64,
    /// Minimum via-to-via spacing (edge to edge) in mm.
    min_via_to_via_mm: f64,
    /// Minimum hole-to-hole clearance in mm.
    min_hole_to_hole_mm: f64,

    // -- Via technology --
    /// Via-in-pad support level.
    via_in_pad: ViaInPadSupport,
    /// Blind via support.
    blind_vias: bool,
    /// Buried via support.
    buried_vias: bool,

    // -- Solder mask --
    /// Minimum solder mask dam/bridge in mm.
    min_solder_mask_dam_mm: f64,
    /// Solder mask expansion in mm.
    solder_mask_expansion_mm: f64,

    // -- Board --
    /// Maximum supported layer count.
    max_layers: u32,
    /// Available copper weights in oz.
    copper_weights_oz: Vec<f64>,
    /// Board thickness options in mm.
    board_thickness_mm: Vec<f64>,
    /// Board edge copper clearance in mm.
    board_edge_clearance_mm: f64,

    // -- Quality --
    /// IPC class (2 or 3).
    ipc_class: u8,
    /// Impedance control tolerance (percentage, e.g. 10.0 for +/-10%).
    impedance_tolerance_pct: f64,

    // -- Stackup (optional, layer-count dependent) --
    /// Predefined stackup for common layer counts.
    stackups: BTreeMap<u32, FabStackup>,
}

enum ViaInPadSupport {
    /// No via-in-pad.
    None,
    /// Soldermask plugged only.
    SoldermaskPlugged,
    /// Resin-filled, copper-capped (POFV).
    ResinFilledCopperCapped,
    /// Conductive fill.
    ConductiveFill,
}

struct FabStackup {
    /// Layer assignments with thickness.
    layers: Vec<FabStackupLayer>,
    /// Core material name.
    core_material: String,
    /// Dielectric constant (Dk) at 1 GHz.
    dk_at_1ghz: f64,
}

struct FabStackupLayer {
    /// Layer type.
    kind: StackupLayerKind,
    /// Thickness in mm.
    thickness_mm: f64,
    /// Material name (e.g. "FR-4", "FR408-HR", "7628 prepreg").
    material: String,
}

enum StackupLayerKind {
    Copper { weight_oz: f64 },
    Prepreg,
    Core,
}
```

### Profile-to-DrcPolicy Mapping

When a fab profile is selected, it generates the following `IrDesignRule` entries:

| Fab Profile Field | IR Rule Kind | IR Rule Params |
|---|---|---|
| `min_trace_width_mm` | `Width` | `min_mm`, `preferred_mm` |
| `min_spacing_mm` | `Clearance` | `gap_mm` |
| `min_drill_mm` | `RoutingViaStyle` | `hole_min_mm` |
| `min_annular_ring_mm` | `MinimumAnnularRing` | `min_mm` |
| `min_hole_to_hole_mm` | `HoleToHoleClearance` | `gap_mm` |
| `board_edge_clearance_mm` | `BoardOutlineClearance` | `gap_mm` |
| `min_solder_mask_dam_mm` | `MinimumSolderMaskSliver` | `min_mm` |

User-specified rules override profile defaults (profile provides the floor).

---

## Concrete Profile Values

### `jlcpcb_2layer`

```
min_trace_width_mm: 0.127       # 5 mil
min_spacing_mm: 0.127           # 5 mil
preferred_trace_width_mm: 0.2   # 8 mil
min_drill_mm: 0.3               # 12 mil
min_laser_drill_mm: None
max_aspect_ratio: 10.0
min_annular_ring_mm: 0.15       # 6 mil
min_via_pad_mm: 0.6             # 24 mil
min_via_to_via_mm: 0.254        # 10 mil
min_hole_to_hole_mm: 0.254      # 10 mil
via_in_pad: None
blind_vias: false
buried_vias: false
min_solder_mask_dam_mm: 0.1     # 4 mil
solder_mask_expansion_mm: 0.05
max_layers: 2
copper_weights_oz: [1.0, 2.0]
board_thickness_mm: [0.8, 1.0, 1.2, 1.6, 2.0]
board_edge_clearance_mm: 0.3
ipc_class: 2
impedance_tolerance_pct: 10.0
```

### `jlcpcb_4layer`

```
min_trace_width_mm: 0.1         # 4 mil
min_spacing_mm: 0.1             # 4 mil
preferred_trace_width_mm: 0.15  # 6 mil
min_drill_mm: 0.2               # 8 mil
min_laser_drill_mm: None
max_aspect_ratio: 10.0
min_annular_ring_mm: 0.15       # 6 mil
min_via_pad_mm: 0.45            # 18 mil  (0.15 + 2*0.15 = 0.45... matches JLCPCB spec)
min_via_to_via_mm: 0.127        # 5 mil
min_hole_to_hole_mm: 0.254      # 10 mil
via_in_pad: None
blind_vias: false
buried_vias: false
min_solder_mask_dam_mm: 0.1     # 4 mil
solder_mask_expansion_mm: 0.05
max_layers: 4
copper_weights_oz: [0.5, 1.0, 2.0]
board_thickness_mm: [0.8, 1.0, 1.2, 1.6, 2.0]
board_edge_clearance_mm: 0.3
ipc_class: 2
impedance_tolerance_pct: 10.0
```

### `jlcpcb_6layer`

```
min_trace_width_mm: 0.09        # 3.5 mil
min_spacing_mm: 0.09            # 3.5 mil
preferred_trace_width_mm: 0.127 # 5 mil
min_drill_mm: 0.2               # 8 mil
min_laser_drill_mm: None
max_aspect_ratio: 10.0
min_annular_ring_mm: 0.15       # 6 mil
min_via_pad_mm: 0.45            # 18 mil
min_via_to_via_mm: 0.127        # 5 mil
min_hole_to_hole_mm: 0.254      # 10 mil
via_in_pad: ResinFilledCopperCapped  # free POFV on 6-20L
blind_vias: false
buried_vias: false
min_solder_mask_dam_mm: 0.1     # 4 mil
solder_mask_expansion_mm: 0.05
max_layers: 20
copper_weights_oz: [0.5, 1.0, 2.0]
board_thickness_mm: [0.8, 1.0, 1.2, 1.6, 2.0]
board_edge_clearance_mm: 0.3
ipc_class: 2
impedance_tolerance_pct: 10.0
```

### `pcbway_standard`

```
min_trace_width_mm: 0.1         # 4 mil
min_spacing_mm: 0.1             # 4 mil
preferred_trace_width_mm: 0.15  # 6 mil
min_drill_mm: 0.15              # 6 mil
min_laser_drill_mm: None
max_aspect_ratio: 10.0
min_annular_ring_mm: 0.15       # 6 mil
min_via_pad_mm: 0.45
min_via_to_via_mm: 0.127
min_hole_to_hole_mm: 0.254
via_in_pad: SoldermaskPlugged
blind_vias: false
buried_vias: false
min_solder_mask_dam_mm: 0.076   # 3 mil (green mask)
solder_mask_expansion_mm: 0.05
max_layers: 14
copper_weights_oz: [1.0, 2.0, 3.0, 4.0]
board_thickness_mm: [0.2, 0.4, 0.6, 0.8, 1.0, 1.2, 1.6, 2.0, 2.4, 3.2]
board_edge_clearance_mm: 0.3
ipc_class: 2
impedance_tolerance_pct: 10.0
```

### `pcbway_hdi`

```
min_trace_width_mm: 0.076       # 3 mil
min_spacing_mm: 0.076           # 3 mil
preferred_trace_width_mm: 0.1   # 4 mil
min_drill_mm: 0.15              # 6 mil (mechanical)
min_laser_drill_mm: 0.076       # 3 mil
max_aspect_ratio: 12.0
min_annular_ring_mm: 0.076      # 3 mil
min_via_pad_mm: 0.3
min_via_to_via_mm: 0.1
min_hole_to_hole_mm: 0.2
via_in_pad: ResinFilledCopperCapped
blind_vias: true
buried_vias: true
min_solder_mask_dam_mm: 0.076   # 3 mil
solder_mask_expansion_mm: 0.05
max_layers: 64
copper_weights_oz: [0.5, 1.0, 2.0, 3.0, 4.0]
board_thickness_mm: [0.21, 0.4, 0.6, 0.8, 1.0, 1.2, 1.6, 2.0, 2.4, 3.2, 6.0]
board_edge_clearance_mm: 0.3
ipc_class: 2
impedance_tolerance_pct: 10.0
```

### `oshpark_2layer`

```
min_trace_width_mm: 0.1524      # 6 mil
min_spacing_mm: 0.1524          # 6 mil
preferred_trace_width_mm: 0.254 # 10 mil
min_drill_mm: 0.254             # 10 mil
min_laser_drill_mm: None
max_aspect_ratio: 6.0
min_annular_ring_mm: 0.127      # 5 mil
min_via_pad_mm: 0.508           # 20 mil
min_via_to_via_mm: 0.254        # 10 mil
min_hole_to_hole_mm: 0.254      # 10 mil
via_in_pad: None
blind_vias: false
buried_vias: false
min_solder_mask_dam_mm: 0.1     # ~4 mil
solder_mask_expansion_mm: 0.05
max_layers: 2
copper_weights_oz: [1.0]
board_thickness_mm: [1.6]
board_edge_clearance_mm: 0.381  # 15 mil
ipc_class: 2
impedance_tolerance_pct: 15.0   # not guaranteed
```

### `oshpark_4layer`

```
min_trace_width_mm: 0.127       # 5 mil
min_spacing_mm: 0.127           # 5 mil
preferred_trace_width_mm: 0.2   # 8 mil
min_drill_mm: 0.254             # 10 mil
min_laser_drill_mm: None
max_aspect_ratio: 6.0
min_annular_ring_mm: 0.1016     # 4 mil
min_via_pad_mm: 0.457           # 18 mil
min_via_to_via_mm: 0.254        # 10 mil
min_hole_to_hole_mm: 0.254      # 10 mil
via_in_pad: None
blind_vias: false
buried_vias: false
min_solder_mask_dam_mm: 0.1     # ~4 mil
solder_mask_expansion_mm: 0.05
max_layers: 4
copper_weights_oz: [0.5, 1.0]
board_thickness_mm: [1.6]
board_edge_clearance_mm: 0.381  # 15 mil
ipc_class: 2
impedance_tolerance_pct: 10.0
```

### `allpcb_standard`

```
min_trace_width_mm: 0.1         # 4 mil
min_spacing_mm: 0.1             # 4 mil
preferred_trace_width_mm: 0.15  # 6 mil
min_drill_mm: 0.2               # 8 mil
min_laser_drill_mm: None
max_aspect_ratio: 6.0
min_annular_ring_mm: 0.153      # 6 mil
min_via_pad_mm: 0.506
min_via_to_via_mm: 0.2          # 8 mil (same net)
min_hole_to_hole_mm: 0.432      # 17 mil (different net)
via_in_pad: None
blind_vias: false
buried_vias: false
min_solder_mask_dam_mm: 0.1     # 4 mil
solder_mask_expansion_mm: 0.05
max_layers: 14
copper_weights_oz: [0.5, 1.0, 2.0]
board_thickness_mm: [0.4, 0.6, 0.8, 1.0, 1.2, 1.6, 2.0, 2.4, 3.2]
board_edge_clearance_mm: 0.3
ipc_class: 2
impedance_tolerance_pct: 10.0
```

### `eurocircuits_standard`

```
min_trace_width_mm: 0.15        # 6 mil (Pattern Class 6)
min_spacing_mm: 0.15            # 6 mil
preferred_trace_width_mm: 0.2   # 8 mil
min_drill_mm: 0.35              # 14 mil (Drill Class C)
min_laser_drill_mm: None
max_aspect_ratio: 10.0
min_annular_ring_mm: 0.175      # 7 mil (outer)
min_via_pad_mm: 0.7             # 0.35 + 2*0.175
min_via_to_via_mm: 0.3
min_hole_to_hole_mm: 0.3
via_in_pad: None
blind_vias: false
buried_vias: false
min_solder_mask_dam_mm: 0.1     # 4 mil
solder_mask_expansion_mm: 0.05
max_layers: 8
copper_weights_oz: [0.5, 1.0, 2.0]
board_thickness_mm: [0.8, 1.0, 1.2, 1.6, 2.0, 2.4]
board_edge_clearance_mm: 0.3
ipc_class: 2
impedance_tolerance_pct: 10.0
```

### `bayarea_standard`

```
min_trace_width_mm: 0.1         # 4 mil
min_spacing_mm: 0.1             # 4 mil
preferred_trace_width_mm: 0.2   # 8 mil
min_drill_mm: 0.152             # 6 mil
min_laser_drill_mm: None
max_aspect_ratio: 10.0
min_annular_ring_mm: 0.127      # 5 mil (Class 2)
min_via_pad_mm: 0.406           # drill + 2*AR
min_via_to_via_mm: 0.254
min_hole_to_hole_mm: 0.254
via_in_pad: SoldermaskPlugged
blind_vias: false
buried_vias: false
min_solder_mask_dam_mm: 0.127   # 5 mil
solder_mask_expansion_mm: 0.1   # 4 mil
max_layers: 16
copper_weights_oz: [0.5, 1.0, 2.0]
board_thickness_mm: [0.2, 0.4, 0.6, 0.8, 1.0, 1.2, 1.6, 2.0, 2.4, 3.2, 6.35]
board_edge_clearance_mm: 0.254  # 10 mil
ipc_class: 2
impedance_tolerance_pct: 10.0
```

### `bayarea_advanced`

```
min_trace_width_mm: 0.051       # 2 mil
min_spacing_mm: 0.051           # 2 mil
preferred_trace_width_mm: 0.1   # 4 mil
min_drill_mm: 0.1               # 4 mil (mechanical)
min_laser_drill_mm: 0.076       # 3 mil
max_aspect_ratio: 12.0
min_annular_ring_mm: 0.076      # 3 mil (mech); 1 mil laser
min_via_pad_mm: 0.254
min_via_to_via_mm: 0.127
min_hole_to_hole_mm: 0.2
via_in_pad: ConductiveFill
blind_vias: true
buried_vias: true
min_solder_mask_dam_mm: 0.076   # 3 mil
solder_mask_expansion_mm: 0.05
max_layers: 30
copper_weights_oz: [0.3, 0.5, 1.0, 2.0, 3.0, 4.0, 5.0]
board_thickness_mm: [0.2, 0.4, 0.6, 0.8, 1.0, 1.2, 1.6, 2.0, 2.4, 3.2, 6.35]
board_edge_clearance_mm: 0.254
ipc_class: 2  # Class 3 available
impedance_tolerance_pct: 5.0
```

### `advanced_circuits_std`

```
min_trace_width_mm: 0.076       # 3 mil
min_spacing_mm: 0.076           # 3 mil
preferred_trace_width_mm: 0.15  # 6 mil
min_drill_mm: 0.127             # 5 mil (mechanical)
min_laser_drill_mm: 0.1         # 4 mil
max_aspect_ratio: 10.0
min_annular_ring_mm: 0.2        # 8 mil
min_via_pad_mm: 0.527
min_via_to_via_mm: 0.254
min_hole_to_hole_mm: 0.254
via_in_pad: ResinFilledCopperCapped
blind_vias: true
buried_vias: true
min_solder_mask_dam_mm: 0.076   # 3 mil
solder_mask_expansion_mm: 0.05
max_layers: 42
copper_weights_oz: [0.25, 0.5, 1.0, 2.0, 3.0, 4.0]
board_thickness_mm: [0.2, 0.4, 0.6, 0.8, 1.0, 1.2, 1.6, 2.0, 2.4, 3.2, 6.35]
board_edge_clearance_mm: 0.254
ipc_class: 2
impedance_tolerance_pct: 7.0
```

### `sierra_standard`

```
min_trace_width_mm: 0.076       # 3 mil
min_spacing_mm: 0.076           # 3 mil
preferred_trace_width_mm: 0.127 # 5 mil
min_drill_mm: 0.152             # 6 mil (mechanical)
min_laser_drill_mm: 0.076       # 3 mil
max_aspect_ratio: 12.0
min_annular_ring_mm: 0.127      # 5 mil (through hole)
min_via_pad_mm: 0.406
min_via_to_via_mm: 0.2
min_hole_to_hole_mm: 0.254
via_in_pad: ResinFilledCopperCapped
blind_vias: true
buried_vias: true
min_solder_mask_dam_mm: 0.102   # 4 mil (SMT)
solder_mask_expansion_mm: 0.05
max_layers: 30
copper_weights_oz: [0.125, 0.25, 0.375, 0.5, 1.0, 2.0, 3.0]
board_thickness_mm: [0.127, 0.2, 0.4, 0.6, 0.8, 1.0, 1.2, 1.6, 2.0, 2.4, 3.2, 6.35]
board_edge_clearance_mm: 0.254
ipc_class: 2  # Class 3 available
impedance_tolerance_pct: 5.0
```

### `sierra_hdi`

```
min_trace_width_mm: 0.038       # 1.5 mil
min_spacing_mm: 0.038           # 1.5 mil
preferred_trace_width_mm: 0.076 # 3 mil
min_drill_mm: 0.152             # 6 mil (mechanical)
min_laser_drill_mm: 0.051       # 2 mil
max_aspect_ratio: 15.0
min_annular_ring_mm: 0.051      # 2 mil (microvia)
min_via_pad_mm: 0.152
min_via_to_via_mm: 0.1
min_hole_to_hole_mm: 0.15
via_in_pad: ConductiveFill
blind_vias: true
buried_vias: true
min_solder_mask_dam_mm: 0.089   # 3.5 mil (BGA)
solder_mask_expansion_mm: 0.038
max_layers: 30
copper_weights_oz: [0.125, 0.25, 0.375, 0.5, 1.0, 2.0, 3.0]
board_thickness_mm: [0.178, 0.4, 0.6, 0.8, 1.0, 1.2, 1.6, 2.0, 2.4, 3.2, 6.35]
board_edge_clearance_mm: 0.254
ipc_class: 3
impedance_tolerance_pct: 5.0
```

### `ttm_standard`

```
min_trace_width_mm: 0.1         # 4 mil
min_spacing_mm: 0.1             # 4 mil
preferred_trace_width_mm: 0.15  # 6 mil
min_drill_mm: 0.152             # 6 mil
min_laser_drill_mm: 0.076       # 3 mil (est)
max_aspect_ratio: 25.0
min_annular_ring_mm: 0.127      # 5 mil
min_via_pad_mm: 0.406
min_via_to_via_mm: 0.2
min_hole_to_hole_mm: 0.254
via_in_pad: ResinFilledCopperCapped
blind_vias: true
buried_vias: true
min_solder_mask_dam_mm: 0.1     # 4 mil
solder_mask_expansion_mm: 0.05
max_layers: 60
copper_weights_oz: [0.5, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 8.0, 10.0]
board_thickness_mm: [0.4, 0.6, 0.8, 1.0, 1.2, 1.6, 2.0, 2.4, 3.2, 6.35, 11.43]
board_edge_clearance_mm: 0.254
ipc_class: 2  # Class 3 and MIL-PRF-31032 available
impedance_tolerance_pct: 10.0
```

### `wurth_standard`

```
min_trace_width_mm: 0.1         # 4 mil (35um copper)
min_spacing_mm: 0.1             # 4 mil (35um copper)
preferred_trace_width_mm: 0.15  # 6 mil
min_drill_mm: 0.25              # 10 mil
min_laser_drill_mm: None
max_aspect_ratio: 10.0
min_annular_ring_mm: 0.175      # Pad = Hole + 0.35mm => AR = 0.175
min_via_pad_mm: 0.6
min_via_to_via_mm: 0.4          # PTH edge-to-edge 400um
min_hole_to_hole_mm: 0.4
via_in_pad: None
blind_vias: false
buried_vias: false
min_solder_mask_dam_mm: 0.07    # 2.8 mil
solder_mask_expansion_mm: 0.05
max_layers: 16
copper_weights_oz: [0.5, 1.0, 2.0, 3.0]
board_thickness_mm: [0.8, 1.0, 1.55, 2.0, 2.4, 3.2]
board_edge_clearance_mm: 0.3
ipc_class: 2  # Class 3 on request
impedance_tolerance_pct: 10.0   # custom only
```

---

## Integration Design

### Spec Language Syntax

```
board {
    fab: "jlcpcb_4layer"     // selects profile
    layers: 4                 // validated against profile.max_layers
    thickness: 1.6mm          // validated against profile.board_thickness_mm
}

// Profile provides default rules; user rules override:
rules {
    clearance: 0.15mm         // overrides profile's min_spacing_mm
    width {
        min: 0.127mm          // overrides profile's min_trace_width_mm
        preferred: 0.2mm
    }
}
```

### Override Semantics

1. Fab profile provides **floor values** (manufacturing minimums)
2. User-specified rules can be **more conservative** (larger minimums)
3. User rules **cannot go below** fab profile minimums (error if attempted)
4. Unspecified rules inherit from the profile

### Validation

When `fab:` is specified:
- `layers` must be <= `profile.max_layers`
- `thickness` must be in `profile.board_thickness_mm` (or closest match with warning)
- User rules that violate profile minimums produce hard errors
- Warnings for rules close to manufacturing limits (within 20% of minimum)

---

## How Fabs Publish Capabilities

- **JLCPCB**: Web capabilities page + impedance calculator + DFM checker (jlcdfm.com)
- **PCBWay**: Web capabilities page + DRC checker + advanced capabilities page
- **OSH Park**: docs.oshpark.com with per-service specification pages
- **AllPCB**: Web capabilities page (limited detail)
- **Eurocircuits**: Classification system (Pattern Class + Drill Class) with online visualizer
- **Bay Area Circuits**: Web capabilities page with standard/advanced tiers
- **Advanced Circuits**: Web capabilities page + DFM checking
- **Sierra Circuits**: Technical specs pages + design tools (impedance, stackup calculators)
- **TTM**: Marketing pages only; detailed specs require direct contact
- **Wurth**: WEdirekt online tool + PDF design guides + custom consultation

No fab provides machine-readable DRC/capability files (e.g., JSON or Altium .RUL). All
capability data must be manually transcribed from web pages and PDFs. This is why a
built-in profile system is valuable -- users should not need to look up and enter dozens
of parameters for common fabs.

---

## Sources

- [JLCPCB Capabilities](https://jlcpcb.com/capabilities/pcb-capabilities)
- [JLCPCB Design Rules Guide (Schemalyzer)](https://www.schemalyzer.com/en/blog/manufacturing/jlcpcb/jlcpcb-design-rules)
- [JLCPCB Impedance Stackup](https://jlcpcb.com/impedance)
- [JLCPCB 6-Layer POFV](https://jlcpcb.com/blog/Free-Via-in-Pad-on-6-20-Layer-PCBs-with-POFV)
- [PCBWay Capabilities](https://www.pcbway.com/capabilities.html)
- [PCBWay Advanced Capabilities](https://www.pcbway.com/advanced-pcb-capabilities.html)
- [OSH Park 2-Layer Service](https://docs.oshpark.com/services/two-layer/)
- [OSH Park 4-Layer Service](https://docs.oshpark.com/services/four-layer/)
- [AllPCB Capabilities](https://www.allpcb.com/standard_pcb_manufacturing_capability.html)
- [Eurocircuits Classification](https://www.eurocircuits.com/pcb-design-guidelines-classification/)
- [Bay Area Circuits Capabilities](https://bayareacircuits.com/pcb-capabilities/)
- [Advanced Circuits (4PCB) Capabilities](https://www.advancedpcb.com/en-us/resources/pcb-capabilities-and-expanded-capabilities/)
- [Sierra Circuits Rigid PCB Specs](https://www.protoexpress.com/kb/rigid-pcb/)
- [Sierra Circuits Annular Ring](https://www.protoexpress.com/kb/annular-ring-size/)
- [TTM Technologies Conventional PCB](https://www.ttm.com/en/solutions/printed-circuit-boards/conventional-pcb)
- [Wurth Elektronik Capabilities](https://www.we-online.com/en/products/printed-circuit-boards/capabilities)
- [WEdirekt PCB Technology](https://www.wedirekt.com/en/content/technology/pcbs)
- [IPC Class 2 vs Class 3 (Sierra Circuits)](https://www.protoexpress.com/blog/ipc-class-2-vs-class-3-different-design-rules/)
- [IPC 6012 Class 3 Annular Ring (Altium)](https://resources.altium.com/p/meeting-standards-ipc-6012-class-3-annular-ring)
