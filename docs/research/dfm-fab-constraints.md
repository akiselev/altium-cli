# DFM & Fabrication Constraints for PCB Autorouters

Research findings on fabrication-specific manufacturing constraints and how autorouters
should incorporate them. Written for the autopcb-router DRC manufacturing module.

**Date**: 2026-03-23

---

## Table of Contents

1. [Fabrication Process Constraints That Affect Routing](#1-fabrication-process-constraints-that-affect-routing)
2. [IPC Class-Based Routing Rules](#2-ipc-class-based-routing-rules)
3. [Panelization and Board Edge Constraints](#3-panelization-and-board-edge-constraints)
4. [HDI Constraints](#4-hdi-high-density-interconnect-constraints)
5. [Flex and Rigid-Flex Constraints](#5-flex-and-rigid-flex-constraints)
6. [Fab Capability Profiles](#6-fab-capability-profiles)
7. [DFM Analysis Tools and Common Violations](#7-dfm-analysis-tools-and-common-violations)
8. [Voltage Clearance (IPC-2221)](#8-voltage-clearance-ipc-2221)
9. [Recommendations for autopcb-router](#9-recommendations-for-autopcb-router)

---

## 1. Fabrication Process Constraints That Affect Routing

### 1.1 Minimum Trace Width/Space by Copper Weight

Copper weight directly affects achievable trace width and spacing because thicker copper
requires more etching, increasing undercut. The etching process removes copper isotropically
(sideways as well as down), so thicker copper eats more into the trace edges.

| Copper Weight | Thickness | Min Trace Width | Min Spacing | Notes |
|---|---|---|---|---|
| 0.5 oz | 17.5 um | 3-4 mil (0.076-0.10 mm) | 3-4 mil | Inner layers only at most fabs |
| 1 oz | 35 um | 4-5 mil (0.10-0.127 mm) | 4-5 mil | Standard; most fabs comfortable here |
| 2 oz | 70 um | 6-8 mil (0.15-0.20 mm) | 6-8 mil | +1-2 mil over 1oz due to etching undercut |
| 3 oz | 105 um | 8-10 mil (0.20-0.25 mm) | 8-10 mil | Heavy copper; limited fab availability |
| 4 oz | 140 um | 10-12 mil (0.25-0.30 mm) | 10-12 mil | Specialty heavy copper |

**Key rule**: For each additional ounce of copper, add approximately 1-2 mil to minimum
trace width and spacing requirements. This is because the etch factor (ratio of vertical
etch to lateral undercut) is typically 3:1 to 4:1 for subtractive etching processes.

**Autorouter impact**: The router must know the copper weight per layer and adjust minimum
trace/space constraints accordingly. A design using 2oz outer / 0.5oz inner will have
different minimums on outer vs inner layers.

Sources:
- [YourPCB DFM Design Rules](https://yourpcb.com/tools/reference/dfm-design-rules)
- [JLCPCB Design Rules (Schemalyzer)](https://www.schemalyzer.com/en/blog/manufacturing/jlcpcb/jlcpcb-design-rules)
- [PCB Prime - Copper Thickness](https://pcbprime.com/pcb-tips/how-thick-is-1oz-copper/)

### 1.2 Drill-to-Copper Clearance

The minimum clearance between a drilled hole edge and the nearest copper feature on
internal layers. This is critical because drill registration is imperfect, and a drill
wandering into a copper plane causes shorts.

| IPC Class | Drill-to-Copper Clearance | Notes |
|---|---|---|
| Class 1 | 6 mil (0.15 mm) | Consumer electronics |
| Class 2 | 8 mil (0.20 mm) | General industrial |
| Class 3 | 10 mil (0.25 mm) | High reliability (mil/aero/medical) |

**Layer count impact**: More layers = more lamination cycles = more registration error
accumulation. Recommended clearances increase with layer count:

| Layer Count | Recommended Drill-to-Copper |
|---|---|
| 4-8 layers | 6-7 mil |
| 8-12 layers | 7-8 mil |
| 12+ layers | 8-10 mil |

**Autorouter impact**: When routing near vias or through-holes on internal layers, the
router must maintain drill-to-copper clearance from the hole edge (not pad edge) to the
nearest copper feature. This is separate from and in addition to pad-to-trace clearance.

Sources:
- [Sierra Circuits - DFM Issues](https://www.protoexpress.com/blog/dfm-issues-pcb-manufacturing/)
- [Bittele Electronics - Drill Requirements](https://www.7pcb.com/blog/dimensions-drills-highest-lowest-diameter)

### 1.3 Annular Ring by IPC Class

The annular ring is the copper pad material remaining around a drilled hole. It is
measured differently on external vs internal layers.

**External layers**: Measured from plated hole wall edge to pad edge.
**Internal layers**: Measured from drilled hole edge to pad edge.

| IPC Class | External Annular Ring | Internal Annular Ring | Breakout Allowed |
|---|---|---|---|
| Class 1 | 0 mil (tangency) | 0 mil | Yes, <180 deg |
| Class 2 | 0 mil (tangency) | 0 mil | Yes, <90 deg |
| Class 3 | 2 mil (0.051 mm) | 1 mil (0.025 mm) | No breakout allowed |

**Minimum pad size formula** (IPC-6012):

```
L = a + 2b + c
```

Where:
- `a` = drill hole diameter (internal) or finished hole diameter (external)
- `b` = minimum annular ring for the class
- `c` = fabrication allowance (typically 7-8 mil / 0.178-0.203 mm for Class C tolerance)

**Practical examples**:

| Via Drill | IPC Class | Min External Pad | Min Internal Pad |
|---|---|---|---|
| 0.3 mm (12 mil) | Class 2 | drill + 8 mil = 20 mil | drill + 8 mil = 20 mil |
| 0.3 mm (12 mil) | Class 3 | drill + 10 mil = 22 mil | drill + 10 mil = 22 mil |
| 0.2 mm (8 mil) | Class 2 | drill + 8 mil = 16 mil | drill + 8 mil = 16 mil |
| 0.2 mm (8 mil) | Class 3 | drill + 10 mil = 18 mil | drill + 10 mil = 18 mil |

**Simplified guideline**: Pad = via diameter + 10 mil for Class 3; + 8 mil for Class 1/2.

**Autorouter impact**: Via pad size is computed from drill size + annular ring. The router
must verify that the resulting pad does not violate clearance to adjacent features. Larger
annular rings (Class 3) make vias bigger and harder to fit in tight areas.

Sources:
- [Altium - IPC 6012 Class 3 Annular Rings](https://resources.altium.com/p/meeting-standards-ipc-6012-class-3-annular-ring)
- [Sierra Circuits - IPC Class 2 vs 3](https://www.protoexpress.com/blog/ipc-class-2-vs-class-3-different-design-rules/)
- [RayPCB - IPC-6012 Guide](https://www.raypcb.com/ipc-6012/)

### 1.4 Solder Mask Dam (Minimum Web Between Openings)

The solder mask dam (or web) is the minimum width of solder mask material between two
openings. If the dam is too narrow, the solder mask peels or bridges, exposing copper and
causing solder bridging.

| Mask Color | Minimum Dam Width | Notes |
|---|---|---|
| Green | 4 mil (0.10 mm) | Most forgiving; standard |
| Blue | 5 mil (0.13 mm) | Slightly worse adhesion |
| Red | 5 mil (0.13 mm) | |
| Black | 6 mil (0.15 mm) | Worst adhesion; hardest to inspect |
| White | 7 mil (0.18 mm) | Worst resolution |

**Solder mask expansion**: Typical 2-3 mil per side expansion of pad opening beyond the
copper pad. For high-density designs, reduce to 2 mil (0.05 mm).

**Dam calculation**: If two pads are separated by `gap` mil of copper clearance:
- Mask dam = gap - 2 * solder_mask_expansion
- If dam < minimum, pads must be farther apart or use solder mask defined (SMD) pads

**Autorouter impact**: While the router doesn't directly place solder mask, it controls
trace and via spacing. When routing between fine-pitch pads (e.g., 0.5mm BGA), the router
must ensure that resulting copper gaps leave enough room for solder mask dams after
expansion. If two vias are placed too close together, the solder mask between them may be
too thin and fall off.

Sources:
- [Sierra Circuits - Solder Mask Clearance](https://www.protoexpress.com/blog/pcb-solder-mask-clearance-every-engineer-should-know/)
- [Cadence - Solder Mask Dams](https://resources.pcb.cadence.com/blog/minimum-solder-mask-dams-in-smd-components)

### 1.5 Via-in-Pad Constraints

Via-in-pad places a via directly in a component pad. This is often required for fine-pitch
BGAs (pitch <= 0.8 mm) where there is no room to fan out.

| Via-in-Pad Type | IPC-4761 | Requirements | Cost Impact |
|---|---|---|---|
| Type I - Tented | Covered by mask | Hole <= 12-14 mil, mask covers | Low |
| Type V - Filled (non-conductive) | Epoxy filled | Flat surface, no outgassing | Medium |
| Type VI - Filled + capped | Filled + copper cap | Best for BGA; fully flat | High |
| Type VII - Filled + capped + plated | Full process | Highest reliability | Highest |

**Critical rules**:
- Unfilled via-in-pad causes **solder wicking**: solder flows down the via during reflow,
  starving the joint
- Filled vias must be planarized to within 1 mil of the surface
- Minimum via-in-pad drill: 0.15 mm (6 mil) for laser; 0.3 mm (12 mil) for mechanical
- Minimum via-in-pad to adjacent copper: 6 mil (0.15 mm)
- Via-in-pad adds $2-15/board depending on quantity and process

**When to use via-in-pad**:
- BGA pitch <= 0.8 mm (can't fan out between balls)
- BGA pitch <= 0.5 mm (absolutely required)
- Thermal vias directly under thermal pads
- High-current paths requiring maximum copper area

**Autorouter impact**: The router's fanout module must decide whether to place vias in pads
or fan out. For fine-pitch BGAs, the router should automatically select via-in-pad when
the pitch doesn't allow fanout, and flag the need for filled/capped vias in the DFM
report.

Sources:
- [PCBWay - Via-in-Pad](https://www.pcbway.com/pcb_prototype/PCB_Via_in_Pad.html)
- [Sierra Circuits - Solder Mask Layer](https://www.protoexpress.com/blog/what-is-solder-mask-layer/)

### 1.6 Aspect Ratio Limitations

The aspect ratio is board thickness divided by smallest drill diameter. Higher ratios make
plating difficult because chemistry cannot reach deep into narrow holes uniformly.

| Aspect Ratio | Difficulty | Typical Application |
|---|---|---|
| <= 6:1 | Easy | Standard 2-4 layer boards |
| 6:1 - 8:1 | Standard | Most multilayer boards |
| 8:1 - 10:1 | Challenging | Thick multilayer or small drills |
| 10:1 - 12:1 | Difficult | Requires advanced plating |
| > 12:1 | Specialty | Very few fabs; significant yield impact |

**Practical constraint**: For a 1.6 mm board:
- 8:1 ratio -> min drill 0.2 mm (8 mil)
- 10:1 ratio -> min drill 0.16 mm (6.3 mil)
- 12:1 ratio -> min drill 0.13 mm (5.2 mil)

**Microvia aspect ratio** (laser-drilled, blind):
- Ideal: 0.8:1 (depth:diameter) or better
- Maximum recommended: 1:1
- E.g., 75 um depth -> minimum 75 um diameter (3 mil)
- Exceeding 0.8:1 increases voiding risk by ~30%

**Autorouter impact**: The router must check `board_thickness / via_drill_diameter` against
the fab's maximum aspect ratio. If the aspect ratio is exceeded, the router should either
select a larger drill or flag a DFM violation.

Sources:
- [Epec - Mechanical vs Laser Drilling](https://blog.epectec.com/pcb-mechanical-drilling-vs-laser-aspect-ratios-and-drill-sizing)
- [Sierra Circuits - PCB Drilling](https://www.protoexpress.com/blog/no-chilling-when-it-comes-to-pcb-drilling/)

### 1.7 Registration Tolerance and Its Impact on Clearances

Registration tolerance is the maximum positional error of a drilled hole or layer alignment
relative to the design intent.

| Feature | Tolerance | Notes |
|---|---|---|
| Drill position (CNC) | +/- 3 mil (0.075 mm) | Standard mechanical drilling |
| Drill position (laser) | +/- 1-2 mil (0.025-0.05 mm) | Microvias |
| Layer-to-layer registration | +/- 2-3 mil (0.05-0.075 mm) | Standard lamination |
| Solder mask registration | +/- 2-3 mil (0.05-0.075 mm) | LDI is better: +/- 1 mil |

**Impact on design rules**: Registration tolerance is already baked into the "fabrication
allowance" term in the annular ring formula. However, the router should be aware that:
- **Actual clearance = designed clearance - registration error** in the worst case
- For inner layers with multiple lamination cycles, errors accumulate
- The 8 mil "fabrication allowance" in IPC Class C accounts for up to 7 mil of wander

**Autorouter impact**: The router should add a safety margin to all clearances based on the
fab's registration tolerance. For example, if the design rule says 5 mil clearance but
registration is +/- 3 mil, the effective clearance could be as low as 2 mil -- potentially
causing a short. Conservative practice: add 1-2 mil margin beyond the bare minimum.

Sources:
- [Cadence - PCB Tolerances](https://resources.pcb.cadence.com/blog/2024-common-pcb-tolerances-for-manufacturing)
- [AdvancedPCB - Tolerances](https://www.advancedpcb.com/en-us/resources/tolerances/)

---

## 2. IPC Class-Based Routing Rules

IPC-6012 defines three product classes with progressively stricter requirements:

- **Class 1**: General electronics (consumer, non-critical)
- **Class 2**: Dedicated service electronics (industrial, communications)
- **Class 3**: High-reliability electronics (military, aerospace, medical, life support)

### 2.1 Complete Constraint Comparison Table

| Parameter | Class 1 | Class 2 | Class 3 | Recommended |
|---|---|---|---|---|
| **Trace** | | | | |
| Min trace width | 6 mil | 5 mil | 4 mil | 6-8 mil |
| Min trace spacing | 6 mil | 5 mil | 4 mil | 6-8 mil |
| Trace-to-edge clearance | 10 mil | 10 mil | 10 mil | 15-20 mil |
| Trace-to-pad clearance | 6 mil | 5 mil | 4 mil | 6 mil |
| Trace width tolerance | +/- 1.5 mil | +/- 1 mil | +/- 0.5 mil | +/- 1 mil |
| **Annular Ring** | | | | |
| External annular ring | 0 mil | 0 mil | 2 mil (0.051 mm) | 5-6 mil |
| Internal annular ring | 0 mil | 0 mil | 1 mil (0.025 mm) | 5-6 mil |
| Breakout allowed | < 180 deg | < 90 deg | None | None |
| **Holes & Vias** | | | | |
| Min PTH drill | 8 mil | 8 mil | 6 mil | 10-12 mil |
| Min NPTH drill | 8 mil | 8 mil | 8 mil | 12 mil |
| Min via pad diameter | 16 mil | 18 mil | 16 mil + annular | 20-24 mil |
| Hole-to-hole spacing | 10 mil | 10 mil | 8 mil | 12 mil |
| Hole-to-copper clearance | 8 mil | 8 mil | 6 mil | 10 mil |
| Hole-to-board edge | 10 mil | 10 mil | 10 mil | 15 mil |
| Drill position tolerance | +/- 3 mil | +/- 3 mil | +/- 2 mil | +/- 3 mil |
| Via aspect ratio max | 8:1 | 8:1 | 10:1 | 6:1 |
| **Plating** | | | | |
| Min hole wall Cu thickness | 0.8 mil | 0.8 mil | 1.0 mil | 1.0 mil |
| **Solder Mask** | | | | |
| Mask clearance per side | 3 mil | 3 mil | 2.5 mil | 3-4 mil |
| Minimum mask dam (green) | 4 mil | 4 mil | 3 mil | 4-5 mil |
| Minimum mask opening | 8 mil | 8 mil | 6 mil | 8 mil |
| **Silkscreen** | | | | |
| Min line width | 5 mil | 5 mil | 4 mil | 6 mil |
| Min text height | 32 mil | 32 mil | 32 mil | 40-50 mil |
| Silk-to-pad clearance | 6 mil | 6 mil | 5 mil | 8 mil |
| **Copper Plane** | | | | |
| Min copper-to-edge (outer) | 10 mil | 10 mil | 8 mil | 15 mil |
| Min copper-to-edge (inner) | 15 mil | 15 mil | 12 mil | 20 mil |
| Pour-to-trace spacing | 8 mil | 8 mil | 6 mil | 10 mil |
| Thermal relief spoke width | 8 mil | 8 mil | 8 mil | 10-12 mil |
| **Dielectric** | | | | |
| Min dielectric thickness | 2.56 mil | 2.56 mil | 2.56 mil | 3.5 mil |

### 2.2 How IPC Class Maps to Router Constraints

The autorouter should accept an IPC class parameter and use it to set floor values for:

1. **Trace width minimum** (per copper weight per layer)
2. **Clearance minimum** (trace-to-trace, trace-to-pad, pad-to-pad)
3. **Annular ring minimum** (affects via pad size)
4. **Drill-to-copper clearance** (affects routing near holes on inner layers)
5. **Aspect ratio maximum** (affects minimum via drill size given board thickness)

The IPC class sets the minimum floor; the fab profile (Section 6) may raise these values
further based on actual capabilities.

Sources:
- [Sierra Circuits - IPC Class 2 vs 3](https://www.protoexpress.com/blog/ipc-class-2-vs-class-3-different-design-rules/)
- [ProtoExpress - IPC Class 3 Standards](https://www.protoexpress.com/kb/ipc-class-3-pcb-design-and-manufacturing-standards/)
- [YourPCB DFM Design Rules](https://yourpcb.com/tools/reference/dfm-design-rules)

---

## 3. Panelization and Board Edge Constraints

### 3.1 V-Score Specifications

V-scoring cuts V-shaped grooves into the top and bottom of a panel for board separation.

| Parameter | Value | Notes |
|---|---|---|
| Groove angle | 20, 30, 45, or 60 deg | 30 deg standard |
| Groove depth | 30-40% of board thickness per side | Each side |
| Remaining web | 0.3-0.5 mm | Thinner = easier break, weaker panel |
| Remaining web (t <= 1.0mm) | 0.3 mm typical | |
| Remaining web (t > 1.0mm) | ~1/3 board thickness | |
| Alignment tolerance | +/- 0.05 mm | Between upper and lower blades |
| Web thickness tolerance | +/- 0.10 mm | |
| Min board-to-board spacing | 0-0.5 mm | V-score allows near-zero gap |
| Min PCB size for V-score | 60 mm x 45 mm | Varies by fab |
| Max PCB size for V-score | 600 mm x 1200 mm | |

### 3.2 Tab Routing / Mouse Bite Specifications

| Parameter | Value | Notes |
|---|---|---|
| Board-to-board spacing | 1.0-2.0 mm | Router bit width |
| Mouse bite hole diameter | 0.5-0.6 mm | Typical |
| Mouse bite hole spacing | 0.75-1.0 mm center-to-center | |
| Tab width | 3-5 mm | Per tab |
| Number of tabs per edge | 2-3 minimum | Depends on board size |
| Min skip cut distance | 5 mm | Between separate cuts |

### 3.3 Board Edge Clearances

| Feature | V-Score Clearance | Tab Routing Clearance | Notes |
|---|---|---|---|
| Copper to edge | 0.5 mm (20 mil) | 0.2 mm (8 mil) | Inner layers need more |
| Components to edge | 1.0 mm (40 mil) | 2.0 mm (80 mil) | Tab routing needs more |
| Tall components to edge | 3.175 mm (125 mil) | 3.175 mm (125 mil) | Large caps, connectors |
| Traces to V-score | 0.4 mm (15 mil) | N/A | Prevent shorts at separation |
| Vias to edge | 0.5 mm (20 mil) | 0.5 mm (20 mil) | |

### 3.4 Panel Rail Specifications

| Parameter | Value | Notes |
|---|---|---|
| Rail width | 5-10 mm per side | Gripper area for pick-and-place |
| Fiducial diameter | 1-3 mm | Copper pad, mask opening |
| Fiducial placement | 3 points, L-shape or triangle | On rails |
| Tooling hole diameter | 3 mm | Standard |
| Tooling hole placement | 2 diagonal corners | |

**Autorouter impact**: The router should enforce board edge keepout zones that account for
the intended panelization method. This means:
- Route keepout at board edge = max(board_outline_clearance, panelization_clearance)
- If V-score is planned, copper must be 0.5mm+ from all straight edges
- If tab routing, components must be 2mm+ from routed edges

Sources:
- [FastTurnPCBs - Panelization Guide](https://www.fastturnpcbs.com/guides/pcb-panelization/)
- [Cadence - Board Edge Clearance](https://resources.pcb.cadence.com/blog/2019-multi-board-pcb-edge-clearance-guidelines-and-panelization-tips)
- [JLCPCB - V-Cut Standards](https://jlcpcb.com/blog/technical-guidance-v-cut-panelization-standards)

---

## 4. HDI (High Density Interconnect) Constraints

### 4.1 HDI Build-Up Notation

HDI boards are described using `N+M+N` notation:
- `1+N+1`: One build-up layer per side (single sequential lamination)
- `2+N+2`: Two build-up layers per side
- `3+N+3`: Three build-up layers per side (highest complexity)

Where N is the number of core layers.

### 4.2 Microvia Specifications

| Parameter | Standard | Advanced | Notes |
|---|---|---|---|
| Microvia diameter (laser) | 100-150 um (4-6 mil) | 75-100 um (3-4 mil) | Laser-drilled |
| Microvia depth | Single dielectric layer | Single layer only | ~75 um typical |
| Aspect ratio (depth:diameter) | 0.8:1 max | 1:1 max | IPC standard |
| Microvia pad diameter | 250-300 um (10-12 mil) | 200 um (8 mil) | |
| Microvia capture pad | 50 um annular ring | 50 um min | |
| Microvia target pad | 50 um annular ring | 50 um min | |
| Land diameter | >= 2x via diameter | | Recommended |

### 4.3 Stacked vs Staggered Vias

**Staggered microvias**: Offset between sequential layers to distribute thermal stress.
- Minimum offset: > microvia diameter (center-to-center)
- Better reliability during thermal cycling
- Preferred when layout density allows

**Stacked microvias**: Vertically aligned through multiple build-up layers.
- Required when: BGA pitch is too tight for staggering
- Must be filled with copper or conductive paste (not epoxy alone)
- IPC 2019 warning: aspect ratio failure risk increases during thermal cycling
- CTE mismatch: dielectric expands at 200 ppm vs copper at 16 ppm beyond Tg

### 4.4 HDI Layer Assignment Rules

| Via Type | Layer Span | When to Use |
|---|---|---|
| Through-hole | All layers | Standard signals, power |
| Blind via (laser) | L1-L2 or Ln-1 to Ln | First build-up layer |
| Blind via (mechanical) | L1-L2/L3 | 2-3 layer span max |
| Buried via | Inner layers only | Core connections |
| Skip via (laser) | L1-L3 (skip one) | 2+N+2 or higher |

**BGA Fanout Strategy for HDI**:
- Outer ring of BGA balls: escape on top layer
- Next ring: blind via to L2, route out on L2
- Inner rings: via to deeper layers as needed
- Dog-bone fanout when pitch >= 0.8 mm
- Via-in-pad when pitch < 0.8 mm

**Autorouter impact**: HDI routing requires layer-aware via selection. The router must:
1. Know which via types are available (determined by stackup/build-up)
2. Only place blind/buried vias between their valid layer pairs
3. Prefer staggered over stacked unless forced by density
4. Enforce microvia aspect ratio limits
5. Handle BGA fanout with progressive layer escape

Sources:
- [Sierra Circuits - Stacked and Staggered Vias](https://www.protoexpress.com/blog/design-manufacture-staggered-and-stacked-vias/)
- [PCB Power - Microvia Reliability](https://www.pcbpower.us/blog/microvia-reliability-in-hdi-pcb-fabrication)
- [PCBSync - HDI PCB Design](https://pcbsync.com/hdi-pcb/)

---

## 5. Flex and Rigid-Flex Constraints

### 5.1 Minimum Bend Radius

| Configuration | Bend Radius Formula | Notes |
|---|---|---|
| Single-layer flex | 6x flex thickness | Static bend |
| Double-layer flex | 12x flex thickness | Static bend |
| Multi-layer flex (3+) | 24x flex thickness | Static bend |
| Dynamic flex (any) | 100x finished thickness | Repeated flexing |

Example: A 0.2 mm (8 mil) double-sided flex has min static bend radius of 2.4 mm.

### 5.2 Copper Weight in Flex Regions

| Copper Weight | Thickness | Suitability for Flex |
|---|---|---|
| 0.25 oz (RA or ED) | 9 um | Best flexibility, fragile |
| 0.375 oz | 12 um | Good flexibility |
| 0.5 oz | 17.5 um | Standard flex |
| 1 oz | 35 um | Limited flex, static bends only |
| 2 oz | 70 um | Not recommended in bend areas |

**Rolled annealed (RA) copper** is strongly preferred over electrodeposited (ED) copper
for flex regions because RA copper has a smoother grain structure and better fatigue life.

### 5.3 Routing Rules in Flex Regions

| Rule | Value | Rationale |
|---|---|---|
| Trace orientation to bend | Perpendicular to bend axis | Reduces stress |
| Min trace width in bend | Wider than rigid area | Stress distribution |
| Trace corners in flex | Curved/filleted only | No sharp angles |
| Conductor spacing in flex | > 2x trace width | Reduce stress concentration |
| Staggered traces (multilayer) | Offset, not stacked | Reduce stress at bend |
| Cross-hatch ground plane | 0.015" wide, 0.025" spacing | Maintain flexibility |

### 5.4 Via and Component Placement Rules

| Feature | Min Distance from Bend | Notes |
|---|---|---|
| PTH (plated through-hole) | 20 mil (0.5 mm) | From bend area |
| Via (any type) | 50-60 mil (1.3-1.5 mm) | From rigid-flex transition |
| Via from stiffener edge | 50 mil (1.3 mm) | Prevents cracking |
| Components | 2 mm from flex area | On rigid sections only |
| Stiffener overlap beyond component | 0.5-1.0 mm | Stress relief at solder joints |

### 5.5 Coverlay Requirements

| Copper Weight | Min Coverlay Thickness |
|---|---|
| 0.5 oz or less | 1 mil (25 um) |
| 1 oz | 1.5 mil (38 um) |
| 2 oz | 3 mil (75 um) |

### 5.6 Flex Tolerances

| Parameter | Value |
|---|---|
| Misregistration tolerance | +/- 5 mil (0.127 mm) |
| Warpage limit | 0.75% |
| Flex outline tolerance | +/- 5 mil |

**Autorouter impact**: The router must know which board regions are flex zones and apply:
1. Perpendicular trace routing to bend axis
2. Wider trace width and spacing in flex
3. No vias or through-holes in or near flex/bend zones
4. Curved trace transitions (no right angles)
5. Cross-hatch planes instead of solid copper in flex

Sources:
- [Sierra Circuits - Flex PCB Design](https://www.protoexpress.com/blog/flex-pcb-design-guidelines-for-manufacturing/)
- [Siemens - Rigid-Flex Design](https://blogs.sw.siemens.com/electronic-systems-design/2025/10/15/mastering-the-bend-essential-tips-tricks-for-rigid-flex-pcb-design/)
- [All Flex - Rigid Flex Guide](https://www.allflexinc.com/blog/understanding-rigid-flex-pcb/)

---

## 6. Fab Capability Profiles

### 6.1 Major Fab Capability Comparison

| Parameter | JLCPCB | PCBWay | Eurocircuits | Notes |
|---|---|---|---|---|
| **Trace/Space** | | | | |
| Min trace (standard) | 5/5 mil (2L) | 4/4 mil | 10/10 mil | Eurocircuits more conservative |
| Min trace (advanced) | 3.5/3.5 mil (6+L) | 4/4 mil | 5/5 mil (HDI) | |
| Min trace (ultra) | 3/3 mil | - | 2/2 mil (flex) | Special order |
| **Drill** | | | | |
| Min mechanical drill | 0.2 mm (2L), 0.15mm (4+L) | 0.15 mm | 0.6 mm (PTH) | EC conservative |
| Min laser drill | 0.1 mm | 0.1 mm | Available | HDI pool |
| **Via** | | | | |
| Min via pad (2L) | 0.6 mm (24 mil) | 0.4 mm | 0.45 mm | |
| Min via pad (4+L) | 0.45 mm (18 mil) | 0.4 mm | - | |
| Min annular ring | 0.15 mm (6 mil) | 0.15 mm (6 mil) | IAR + 0.075mm, min 0.2mm | |
| **Solder Mask** | | | | |
| Min dam | 0.1 mm (4 mil) | 0.1 mm (4 mil) | - | |
| Mask expansion | 0.05 mm/side | 0.05 mm/side | - | |
| **Board** | | | | |
| Thickness range | 0.4-2.0 mm | 0.2-3.2 mm | 0.2-3.2 mm | |
| Max aspect ratio | 10:1 | 8:1 (std), 10:1 (adv) | ~8:1 | |
| **Layers** | | | | |
| Max layers | 20 | 14+ | 16 | |
| **Copper** | | | | |
| Outer copper | 1-2 oz | 1-8 oz | 1-3 oz | |
| Inner copper | 0.5-1 oz | 1-4 oz | 0.5-1 oz | |
| **Tolerances** | | | | |
| Drill tolerance (PTH) | +/- 0.08 mm | +/- 0.08 mm | - | |
| Drill tolerance (NPTH) | +/- 0.05 mm | +/- 0.05 mm | - | |
| Board size tolerance | +/- 0.2 mm | +/- 0.2 mm | - | CNC routing |
| **Impedance** | | | | |
| Impedance control | Yes, +/- 10% | Yes, +/- 10% | Yes | |

### 6.2 How EDA Tools Model Fab Capabilities

**Altium Designer**:
- Design rules in Constraint Manager define fabrication limits
- Rules set per net class, layer, or object-to-object
- No native "fab profile" import -- rules are set manually
- DRCOnline and Altium 365 provide some DFM integration

**KiCad**:
- `.kicad_dru` custom design rule files
- Community-maintained fab-specific rule files on GitHub (JLCPCB, OSH Park, etc.)
- Parameters: copper clearance, track width, via diameter, holes, microvias, silkscreen
- Can be imported per project

**Siemens Valor NPI / PCBflow**:
- Most sophisticated: **DFM Profiles** created by manufacturers
- Profile captures all process capabilities as structured constraint sets
- Designer uploads design; system validates against selected manufacturer's profile
- Accepts: ODB++, IPC-2581, Gerber, Altium, Mentor, Zuken formats
- 940+ DFM checks (283 fabrication, 367 assembly, 120 flex, 45 microvia, 38 panel, 87 substrate)

**Cadence Allegro/OrCAD**:
- Constraint Manager with hierarchical rules
- Integration with Valor for DFM
- Object-to-object constraint tables

### 6.3 Proposed Fab Profile Data Model for autopcb-router

Based on the research, a fab capability profile should capture these parameters:

```
FabProfile {
    // Identity
    name: String,                        // e.g., "JLCPCB Standard 4-Layer"

    // Trace/Space (per copper weight per layer type)
    trace_constraints: Vec<TraceConstraint> {
        copper_weight_oz: f64,           // 0.5, 1.0, 2.0, etc.
        layer_type: LayerType,           // Inner, Outer
        min_trace_width_mm: f64,
        min_trace_spacing_mm: f64,
    },

    // Drill
    min_mechanical_drill_mm: f64,        // Smallest CNC drill
    min_laser_drill_mm: f64,             // Smallest laser drill (0.0 if N/A)
    max_drill_mm: f64,
    drill_increment_mm: f64,             // Typically 0.05 mm
    drill_tolerance_pth_mm: f64,         // e.g., +/- 0.08
    drill_tolerance_npth_mm: f64,        // e.g., +/- 0.05

    // Via
    min_via_pad_mm: f64,                 // Minimum via pad diameter
    min_annular_ring_mm: f64,            // Minimum annular ring width
    max_aspect_ratio: f64,               // e.g., 10.0
    via_in_pad_available: bool,
    via_in_pad_min_drill_mm: f64,

    // Solder Mask
    min_solder_mask_dam_mm: f64,         // Minimum web between openings
    solder_mask_expansion_mm: f64,       // Per-side expansion
    solder_mask_registration_mm: f64,    // Registration tolerance

    // Board
    min_board_thickness_mm: f64,
    max_board_thickness_mm: f64,
    max_layers: u32,

    // Registration
    drill_registration_mm: f64,          // Drill position accuracy
    layer_registration_mm: f64,          // Layer-to-layer alignment

    // Edge clearances
    min_copper_to_edge_outer_mm: f64,
    min_copper_to_edge_inner_mm: f64,

    // HDI (optional)
    hdi_available: bool,
    max_sequential_laminations: u32,     // 0, 1, 2, or 3
    min_microvia_diameter_mm: f64,
    max_microvia_aspect_ratio: f64,

    // Impedance
    impedance_control_available: bool,
    impedance_tolerance_percent: f64,    // e.g., 10.0
}
```

Sources:
- [JLCPCB Capabilities](https://jlcpcb.com/capabilities/pcb-capabilities)
- [PCBWay Capabilities](https://www.pcbway.com/capabilities.html)
- [Eurocircuits Classification](https://www.eurocircuits.com/pcb-design-guidelines/classification/)
- [Siemens Valor NPI](https://eda.sw.siemens.com/en-US/pcb/valor/valor-npi/)
- [KiCad Custom Design Rules (GitHub)](https://github.com/Cimos/KiCad-CustomDesignRules)

---

## 7. DFM Analysis Tools and Common Violations

### 7.1 Valor NPI Check Categories

Valor NPI is the industry standard for DFM analysis. Its 940+ checks are organized into:

| Category | Check Count | Examples |
|---|---|---|
| Fabrication | 283 | Trace width, spacing, annular ring, drill, aspect ratio |
| Assembly | 367 | Component spacing, solder paste, tombstoning risk |
| Test | varies | Test point access, ICT coverage |
| Flex/Rigid-Flex | 120 | Bend radius, flex routing, coverlay |
| Microvia/HDI | 45 | Microvia aspect ratio, stacking rules |
| Panel | 38 | Fiducials, tooling holes, rail dimensions |
| Substrate | 87 | Material-specific constraints |

### 7.2 Common DFM Violations in Auto-Routed Boards

These are the violations most frequently found during DFM analysis of auto-routed designs:

#### 7.2.1 Acid Traps

**What**: Acute angles (< 90 degrees) in copper traces that trap etchant during manufacturing.

**Detection**: Any junction where two copper features meet at an angle < 90 degrees. This
includes trace-to-trace, trace-to-pad, and copper pour corners.

**Threshold**: Flag angles < 90 degrees; critical below 45 degrees.

**Prevention**: Use two 45-degree bends instead of one 90-degree turn. Enable teardrop
insertion at pad/via connections.

**Autorouter relevance**: HIGH. Autorouters commonly create acid traps at via connections
and where traces change direction sharply. The router should avoid creating acute angles
and insert teardrops at pad/via entries.

#### 7.2.2 Copper Slivers

**What**: Thin, isolated strips of copper on plane layers that can detach during etching
and redeposit elsewhere, causing shorts.

**Detection**: Any copper feature narrower than the minimum feature size.

**Threshold**: Minimum copper feature width: 4 mil (0.10 mm). Minimum spacing between
isolated pads on plane layers: 8 mil (0.20 mm).

**Autorouter relevance**: MEDIUM. Slivers typically form in copper pours, not routed
traces, but the router's trace placement affects pour geometry. The router should check
that traces don't create narrow copper slivers between the trace clearance zone and
adjacent features.

#### 7.2.3 Starved Thermals

**What**: Thermal relief connections to pads that are too thin or incomplete, causing
poor soldering.

**Detection**: Thermal relief spoke width < minimum; fewer than required spokes connecting
to the pad.

**Threshold**: Minimum spoke width: 8 mil (0.20 mm). Minimum 4 spokes for BGA/SMD; minimum
2 for through-hole.

**Autorouter relevance**: LOW (thermals are usually handled by copper pour, not the router).

#### 7.2.4 Insufficient Annular Ring

**Detection**: `(pad_diameter - drill_diameter) / 2 < minimum_annular_ring`

**Thresholds by IPC Class**:
- Class 2 vias: drill + 7 mil pad minimum
- Class 2 component holes: drill + 9 mil pad minimum
- Class 3 outer: drill + 10 mil pad minimum
- Class 3 inner: drill + 11 mil pad minimum

**Autorouter relevance**: HIGH. The router places vias and must ensure annular ring is met.

#### 7.2.5 Drill-to-Copper Violations

**What**: Copper feature too close to a drilled hole on an inner layer.

**Thresholds**:
- 4-8 layers: 6 mil clearance
- 8-12 layers: 7-8 mil clearance
- Minimum drill-to-drill: 6 mil

**Autorouter relevance**: HIGH. When routing on inner layers near vias, the router must
maintain hole-to-copper clearance, not just pad-to-copper clearance.

#### 7.2.6 Solder Mask Dam Violations

**What**: Solder mask web between adjacent openings too narrow; mask peels off.

**Threshold**: 4 mil minimum (green); 5-7 mil for other colors.

**Autorouter relevance**: MEDIUM. The router controls via-to-via and via-to-pad spacing,
which indirectly determines mask dam width.

#### 7.2.7 Aspect Ratio Violations

**What**: Board too thick relative to smallest drill diameter.

**Threshold**: > 10:1 for standard; > 8:1 for budget fabs.

**Autorouter relevance**: HIGH. The router selects via drill sizes and must check against
board thickness.

#### 7.2.8 Copper Balance Issues

**What**: Uneven copper distribution between layers causes warping during lamination.

**Detection**: Copper density per layer; delta > 20-30% between adjacent layers is risky.

**Autorouter relevance**: LOW-MEDIUM. The router can contribute to copper balance by
distributing routes across layers, but this is not a primary routing concern.

#### 7.2.9 Missing Solder Mask Between Via and SMD Pad

**What**: Via too close to SMD pad; solder mask between them is removed or too thin, causing
solder to wick into the via during reflow.

**Detection**: Distance from via edge to SMD pad opening < mask_dam_min + 2 * mask_expansion

**Autorouter relevance**: HIGH. The router must maintain sufficient distance between vias
and SMD pads to preserve solder mask integrity.

Sources:
- [Sierra Circuits - DFM Issues](https://www.protoexpress.com/blog/dfm-issues-pcb-manufacturing/)
- [Siemens - 4 Less Obvious DFM Violations](https://blogs.sw.siemens.com/pcbflow/2020/03/16/4-less-obvious-pcb-dfm-violations/)
- [Altium - Preventing DFM Errors](https://resources.altium.com/p/preventing-top-dfm-errors-your-pcb-design)
- [Siemens Valor NPI](https://eda.sw.siemens.com/en-US/pcb/valor/valor-npi/)
- [PCBSync - Acid Traps](https://pcbsync.com/acid-traps-pcb/)

---

## 8. Voltage Clearance (IPC-2221)

### 8.1 IPC-2221B Spacing Requirements

| Peak Voltage (V) | Internal Layers (mm) | External Uncoated (mm) | External Coated (mm) |
|---|---|---|---|
| 15 | 0.05 | 0.1 | 0.05 |
| 30 | 0.05 | 0.1 | 0.05 |
| 50 | 0.1 | 0.6 | 0.13 |
| 100 | 0.1 | 0.6 | 0.13 |
| 150 | 0.2 | 0.6 | 0.4 |
| 170 | 0.2 | 1.25 | 0.4 |
| 250 | 0.2 | 1.25 | 0.4 |
| 300 | 0.2 | 1.25 | 0.4 |
| 500 | 0.25 | 2.5 | 0.8 |

Above 500V, formulas are used. For most board designs (< 50V), the DFM minimum
trace/space rules dominate. Voltage clearance only becomes the limiting factor for
power electronics (> 50V) or isolated designs.

**Creepage vs Clearance**:
- **Clearance**: Shortest distance through air between conductors
- **Creepage**: Shortest distance along the surface of insulating material

Creepage requirements depend on pollution degree and material CTI (Comparative Tracking
Index). For FR-4 (CTI group IIIa), creepage requirements are stricter than clearance
for high-humidity environments.

**Autorouter impact**: For designs with high-voltage nets, the router must apply per-net
voltage-based clearances. These override the standard DFM clearance when they are larger.
The DrcPolicy already has `creepage_distance_mm` -- this should be computed from the
voltage table and the environmental conditions.

Sources:
- [IPC-2221B Clearance Table](https://www.smpspowersupply.com/ipc2221pcbclearance.html)
- [Altium - IPC-2221 Calculator](https://resources.altium.com/p/using-an-ipc-2221-calculator-for-high-voltage-design)

---

## 9. Recommendations for autopcb-router

### 9.1 Priority Order for Manufacturing DRC Implementation

Based on autorouter relevance and frequency of real-world DFM violations:

| Priority | Check | Current Status | Effort |
|---|---|---|---|
| **P0** | Annular ring validation | Partial (via.rs) | Low - extend existing |
| **P0** | Aspect ratio check | Not implemented | Low |
| **P0** | Drill-to-copper clearance | Not implemented | Medium |
| **P1** | Solder mask dam violation | Placeholder (manufacturing.rs) | Medium |
| **P1** | Via-to-SMD-pad mask clearance | Not implemented | Medium |
| **P1** | Acid trap / acute angle detection | Partial (geometry.rs) | Medium |
| **P1** | Copper-to-board-edge clearance | Partial (board.rs) | Low - extend |
| **P2** | Copper weight trace/space validation | Not implemented | Low |
| **P2** | Via-in-pad flagging | Not implemented | Low |
| **P2** | Copper sliver detection | Not implemented | High |
| **P2** | HDI layer span validation | Not implemented | Medium |
| **P3** | Flex zone routing rules | Not implemented | High |
| **P3** | Panelization clearances | Not implemented | Medium |
| **P3** | Copper balance analysis | Not implemented | Medium |
| **P3** | Thermal relief validation | Not implemented | Medium |

### 9.2 Proposed FabProfile Integration

The manufacturing DRC module (`drc/manufacturing.rs`) should be driven by a `FabProfile`
struct that captures all fab-specific constraints (see Section 6.3 for the data model).

**Loading**: FabProfiles can be loaded from:
1. Built-in presets (JLCPCB Standard, PCBWay Standard, etc.)
2. JSON/TOML configuration files
3. Derived from IPC class + board parameters

**Override hierarchy** (lowest to highest priority):
1. IPC class defaults (floor values)
2. Fab profile capabilities
3. Design rules from PcbIr
4. Net-class-specific overrides

**Integration with DrcPolicy**: The `FabProfile` should be consulted during `DrcPolicy::build()`
to set floor values that the design rules cannot go below. For example, if the design says
4 mil trace but the fab profile says 5 mil minimum, the effective minimum is 5 mil.

### 9.3 Proposed DrcViolationKind Additions

The current `DrcViolationKind` enum should be extended with:

```rust
// Manufacturing-specific violations
AspectRatioExceeded,           // board_thickness / drill_diameter > max
DrillToCopperClearance,        // hole edge to copper feature too close
CopperSliver,                  // copper feature below minimum width
AcidTrap,                      // acute angle in copper geometry
SolderMaskDamTooNarrow,        // mask web between openings too thin
ViaToSmdMaskClearance,         // via too close to SMD pad (mask integrity)
CopperWeightTraceViolation,    // trace too narrow for copper weight
MicroviaAspectRatio,           // microvia depth:diameter exceeds limit
HdiLayerSpanViolation,         // blind/buried via spans wrong layers
FlexBendRadiusViolation,       // trace/via in flex bend zone
PanelizationClearance,         // copper too close to board edge for panel method
```

### 9.4 What to Add to PcbIR

To support manufacturing DRC, the IR needs:

1. **Per-layer copper weight** (oz) -- needed for trace/space validation
2. **Board thickness** (mm) -- needed for aspect ratio check
3. **Solder mask parameters** (dam width, expansion, color) -- needed for mask checks
4. **Fab profile reference** -- links to capabilities
5. **Flex zone definitions** -- regions with flex-specific rules
6. **Panelization intent** -- V-score, tab routing, or none

### 9.5 Constraint Floor Table (Quick Reference)

These are safe defaults when no fab profile is specified:

| Parameter | Conservative Default | Aggressive Default |
|---|---|---|
| Min trace width (1oz outer) | 6 mil / 0.15 mm | 4 mil / 0.10 mm |
| Min trace spacing (1oz outer) | 6 mil / 0.15 mm | 4 mil / 0.10 mm |
| Min trace width (2oz outer) | 8 mil / 0.20 mm | 6 mil / 0.15 mm |
| Min trace spacing (2oz outer) | 8 mil / 0.20 mm | 6 mil / 0.15 mm |
| Min annular ring | 6 mil / 0.15 mm | 4 mil / 0.10 mm |
| Min drill (mechanical) | 0.3 mm / 12 mil | 0.2 mm / 8 mil |
| Max aspect ratio | 8:1 | 10:1 |
| Min solder mask dam | 4 mil / 0.10 mm | 3 mil / 0.08 mm |
| Solder mask expansion | 3 mil / 0.075 mm | 2 mil / 0.05 mm |
| Drill-to-copper clearance | 8 mil / 0.20 mm | 6 mil / 0.15 mm |
| Min copper-to-edge (outer) | 15 mil / 0.38 mm | 10 mil / 0.25 mm |
| Min copper-to-edge (inner) | 20 mil / 0.50 mm | 15 mil / 0.38 mm |
| Drill position tolerance | +/- 3 mil / 0.075 mm | +/- 2 mil / 0.05 mm |
| Layer registration | +/- 3 mil / 0.075 mm | +/- 2 mil / 0.05 mm |
| Hole-to-hole spacing | 10 mil / 0.25 mm | 8 mil / 0.20 mm |

---

## Appendix A: Unit Conversions

| mil | mm | um |
|---|---|---|
| 1 | 0.0254 | 25.4 |
| 2 | 0.0508 | 50.8 |
| 3 | 0.0762 | 76.2 |
| 4 | 0.1016 | 101.6 |
| 5 | 0.127 | 127.0 |
| 6 | 0.1524 | 152.4 |
| 8 | 0.2032 | 203.2 |
| 10 | 0.254 | 254.0 |
| 12 | 0.3048 | 304.8 |
| 15 | 0.381 | 381.0 |
| 20 | 0.508 | 508.0 |

## Appendix B: Copper Weight to Thickness

| oz | um | mil |
|---|---|---|
| 0.25 | 8.75 | 0.34 |
| 0.375 | 13.1 | 0.52 |
| 0.5 | 17.5 | 0.69 |
| 1.0 | 35.0 | 1.38 |
| 2.0 | 70.0 | 2.76 |
| 3.0 | 105.0 | 4.13 |
| 4.0 | 140.0 | 5.51 |
