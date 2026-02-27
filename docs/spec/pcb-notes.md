# Spec Language Gap Analysis: DRC & Autorouting Metadata

Research date: 2026-02-27

## Overview

The spec language handles **layout geometry** well (positions, shapes, sizes) but is
almost entirely missing the **electrical intent** and **manufacturing metadata** that
Altium needs for DRC checks and autorouting. There are also pad properties that are
*defined in the spec model types but never wired through* to reconciliation or execution.

The spec language targets **library files** (SchLib/PcbLib), not **design files**
(SchDoc/PcbDoc). Most DRC rules live at the PCB level, not in the library. The
library's role is to provide correct **metadata** so the rules can reference it.

---

## 1. Low-Hanging Fruit: Pad Properties Defined but Not Wired

These fields already exist in `PadSpec` (spec model) but are **never reconciled,
never executed, and not accepted by `AddPadOp`**:

| Field | PadSpec? | Reconciler? | AddPadOp? | Impact |
|-------|----------|-------------|-----------|--------|
| `pad_mode` | Yes | No | No | Per-layer pad stack control |
| `solder_mask_expansion` | Yes | No | No | DRC mask opening |
| `paste_mask_expansion` | Yes | No | No | Manufacturing stencil |
| `plane_connection` | Yes | No | No | Power plane thermal relief |
| `relief_conductor_width` | Yes | No | No | Thermal spoke width |
| `relief_entries` | Yes | No | No | Number of thermal spokes |
| `relief_air_gap` | Yes | No | No | Relief isolation gap |

**Fix**: Wire these 7 fields through reconciler -> executor -> `AddPadOp` ->
serialization. The model types exist.

**Files to modify:**
- `crates/altium-format-ops/src/spec/reconciler.rs` — `pad_spec_to_add` only checks
  8 properties, missing these 7
- `crates/altium-format-ops/src/spec/executor.rs` — doesn't pass them to AddPadOp
- `crates/altium-format/src/pcb_ops_core.rs` — `AddPadOp` only has 8 fields

---

## 2. Missing Pin Properties (HIGH priority for autorouting)

The spec language supports only: `name`, `electrical`, `length`, `at`, `orientation`,
`is_hidden`, `hidden_net_name`.

### 2.1 Pin Swap Groups (critical for autorouter)

The autorouter needs swap IDs to know which pins are functionally interchangeable.
Defined in SchPin record and PinMiscData sidecar.

| Field | Format Key | Spec Lang | Purpose |
|-------|-----------|-----------|---------|
| `swap_id_pin` | `SWAPIDGROUP` | Missing | Pin-level swap group (e.g., NAND gate inputs) |
| `swap_id_part` | `SWAPIDPART` | Missing | Part-level swap (e.g., which gate in 74HC00) |
| `swap_id_pair` | PinMiscData `PairSwapID` | Missing | Differential pair swap |

Proposed spec syntax:
```
pin 1 { electrical: input, swap_group: "A", swap_part: 1 }
pin 2 { electrical: input, swap_group: "A", swap_part: 1 }  // swappable with pin 1
```

### 2.2 IEEE Pin Symbols (medium priority for ERC)

These visual symbols carry electrical meaning (inversion dot, clock edge, active-low,
Schmitt trigger, open collector/emitter markers). Defined in PinSymbol enum (0-33+).

| Field | Format Key | Spec Lang |
|-------|-----------|-----------|
| `symbol_inner_edge` | `SYMBOL_INNEREDGE` | Missing |
| `symbol_outer_edge` | `SYMBOL_OUTEREDGE` | Missing |
| `symbol_inside` | `SYMBOL_INSIDE` | Missing |
| `symbol_outside` | `SYMBOL_OUTSIDE` | Missing |

### 2.3 Pin Function/Mode Data (medium priority)

Multi-function pins (GPIOs that can be SPI/I2C/UART) store alternate functions
in the PinFunctionData sidecar stream.

| Field | Format Location | Spec Lang |
|-------|----------------|-----------|
| `defined_functions` | PinFunctionData sidecar | Missing |
| `selected_functions` | PinFunctionData sidecar | Missing |

### 2.4 Signal Integrity Properties (lower priority)

| Field | Format Location | Spec Lang |
|-------|----------------|-----------|
| `propagation_delay` | PinPropagationDelay sidecar | Missing |
| `pin_package_length` | PinMiscData sidecar | Missing |

---

## 3. Missing Pad Properties (MEDIUM priority for DRC)

Beyond the "defined but not wired" fields in section 1, these are completely absent
from the spec language:

| Field | Altium Type | Purpose |
|-------|------------|---------|
| `tenting_mode` | TentingMode enum (None/Top/Bottom/Both) | Solder mask coverage for vias-in-pad |
| `hole_shape` | PcbPadHoleShape (Round/Square/Slot) | DRC annular ring calc |
| `hole_slot_length` | Coord | Slot dimensions |
| `drill_type` | DrillType (Drilled/Punched/Laser/Plasma) | Manufacturing DRC |
| `daisy_chain_style` | DaisyChainStyle (Load/Terminator/Source) | High-speed test routing |
| `corner_radius_percentage` | u8 (0-100) | Rounded rectangle shape |
| `is_test_point_top` | bool | Assembly test point designation |
| `is_test_point_bottom` | bool | Assembly test point designation |

### Pad mask expansion detail

Altium supports three expansion modes per mask type, controlled by
`ExtendedPrimitiveInformation` sidecar:

| Property | Type | Purpose |
|----------|------|---------|
| `PASTEMASKEXPANSIONMODE` | TMaskExpansionMode (None/Rule/Manual) | Source of paste value |
| `PASTEMASKEXPANSION_MANUAL` | Coord | Manual paste expansion |
| `SOLDERMASKEXPANSIONMODE` | TMaskExpansionMode | Source of solder mask value |
| `SOLDERMASKEXPANSION_MANUAL` | Coord | Manual solder mask expansion |
| `PasteMaskEnabled` | bool | Enable paste mask for this pad |
| `PasteMaskUsePercent` | bool | Percentage vs absolute |
| `PasteMaskPercent` | f64 | Paste expansion as percentage |
| `SolderMaskOverride` | bool | Override rule-based mask |
| `UseSeparateSolderMaskExpansion` | bool | Different top/bottom values |
| `SolderMaskExpansionTop` | Coord | Top layer expansion |
| `SolderMaskExpansionBottom` | Coord | Bottom layer expansion |
| `SolderMaskExpansionFromHoleEdge` | bool | Measure from hole edge vs pad edge |

### Pad stack detail

Per-layer pad overrides (Full Stack mode) use `PcbPadStackData` with 596+ bytes:

| Property | Type | Purpose |
|----------|------|---------|
| `corner_radius_percentage[32]` | u8 array | Corner radius per layer (0-100%) |
| `size_layers[32]` | CoordPoint array | Pad width/height per layer |
| `shape_layers[32]` | PcbPadShape array | Shape per layer |
| `offsets_from_hole_center[32]` | CoordPoint array | Offset from hole per layer |

---

## 4. Missing Footprint Properties (MEDIUM priority)

Current spec supports: `description`, `height`, `pattern`. Missing:

| Field | Purpose |
|-------|---------|
| 3D body details | `component_body` graphic type exists in spec (section 5.3) with `model_name`, `standoff_height`, `overall_height`, `body_opacity` — but missing: model file path, body type (generic/extruded/cylinder), body projection (top/bottom) |
| Courtyard semantics | No way to distinguish courtyard polylines from decorative ones for DRC clearance |
| Keepout regions | No way to define footprint-level keepout zones |
| IPC compliance | No compliance level, pad class mapping |

---

## 5. Missing Component Properties (LOWER priority)

Current spec supports: `designator`, `description`, `component_kind`, `part_count`,
`show_hidden_pins`. Missing:

| Field | Format Key | Purpose |
|-------|-----------|---------|
| `source_library_name` | `SOURCELIBRARYNAME` | Library linking for ECO sync |
| `design_item_id` | `DESIGNITEMID` | Database/vault component linking |
| `component_class` | `ClassName` parameter | Groups components for rule scoping |

---

## 6. Project/Board-Level Concepts (Out of Scope for Library Specs)

These are critical for DRC but live at the PCB/project level, not in library files:

| Concept | Where It Lives | Impact |
|---------|---------------|--------|
| **Net classes** | PCB Nets6 section | Batch rule scoping (trace width, clearance) |
| **Differential pairs** | PCB DifferentialPairs6 section | Width, gap, impedance, skew |
| **Design rules** (70 kinds) | PCB Rules6/NewRules6 sections | All DRC behavior |
| **Layer stack** | PCB LayerStackSection | Impedance, via types |
| **ERC connection matrix** | Project settings | Pin-type compatibility |
| **Room definitions** | PCB document | Placement constraints |
| **Signal classes** | PCB SignalClasses section | SI group rules |

### Altium Design Rule Types (70 kinds)

For reference, all rule types that the library metadata feeds into:

**Electrical (6):** Clearance, Short Circuit, Unrouted Net, Unconnected Pin,
Unrepoured Polygon, Creepage Distance

**Routing (10):** Width, Routing Neck-Down, Routing Topology, Routing Priority,
Routing Layers, Routing Corner Style, Routing Via Style, Fanout Control,
Differential Pairs Routing, Wire Bonding

**SMT (4):** SMD to Corner, SMD to Plane, SMD Neck-Down, SMD Entry

**Mask (2):** Solder Mask Expansion, Paste Mask Expansion

**Plane (3):** Power Plane Connect Style, Power Plane Clearance, Polygon Connect Style

**Testpoint (4):** Fabrication Testpoint Style, Fabrication Testpoint Usage,
Assembly Testpoint Style, Assembly Testpoint Usage

**Manufacturing (10):** Minimum Annular Ring, Acute Angle, Max/Min Hole Size,
Layer Pair, Hole to Hole Clearance, Minimum Solder Mask Sliver,
Silk to Solder Mask Clearance, Silk to Silk Clearance, Net Antennae,
Board Outline Clearance

**High Speed (8):** Parallel Segment, Length, Matched Lengths, Daisy Chain Stub Length,
Vias Under SMD, Maximum Via Count, Max Via Stub Length (Back Drilling), Return Path

**Placement (6):** Room Definition, Component Clearance, Component Rotations,
Permitted Layers, Nets to Ignore, Max/Min Height

**Signal Integrity (13):** Signal Stimulus, Overshoot Rising/Falling,
Undershoot Rising/Falling, Impedance, Signal Top/Base Value,
Flight Time Rising/Falling, Slope Rising/Falling, Supply Nets

**Other (4):** Layer Stack, Silk to Board Region Clearance, Z-Axis Clearance, None

---

## 7. Format-Level Metadata Inventory

### Pin record fields (SchPin, Record 2)

Complete list of DRC/autorouting-relevant fields in the format:

```
ELECTRICAL          PinElectricalType (0-7)      -> spec: `electrical` (supported)
NAME                String                        -> spec: `name` (supported)
PINLENGTH           Coord                         -> spec: `length` (supported)
ISHIDDEN            Bool (PINCONGLOMERATE bit)     -> spec: `is_hidden` (supported)
HIDDENNETNAME       String                        -> spec: `hidden_net_name` (supported)

SWAPIDGROUP         String                        -> spec: MISSING
SWAPIDPART          i32                           -> spec: MISSING
SWAPIDSEQUENCE      String                        -> spec: MISSING

SYMBOL_INNEREDGE    PinSymbol (0-33+)             -> spec: MISSING
SYMBOL_OUTEREDGE    PinSymbol                     -> spec: MISSING
SYMBOL_INSIDE       PinSymbol                     -> spec: MISSING
SYMBOL_OUTSIDE      PinSymbol                     -> spec: MISSING
SYMBOL_LINEWIDTH    SymbolLineWidth               -> spec: MISSING

PINPROPAGATIONDELAY f64                           -> spec: MISSING
DEFAULTVALUE        String                        -> spec: MISSING
```

### Pin sidecar streams (SchLib only)

```
PinMiscData         PairSwapID, PinPackageLength  -> spec: MISSING
PinWideText         SwapId, SwapIDPart (Unicode)  -> spec: MISSING
PinPropagationDelay Propagation delay value       -> spec: MISSING
PinFunctionData     Alternate pin functions       -> spec: MISSING
```

### Pad record fields (PcbPad, Object 2)

Complete list of DRC/autorouting-relevant fields in the format:

```
PAD_NAME            String                        -> spec: pad name (supported)
LOCATION            CoordPoint                    -> spec: `at` (supported)
SIZE_TOP            CoordPoint                    -> spec: `x_size`, `y_size` (supported)
SHAPE_TOP           PcbPadShape                   -> spec: `shape` (supported)
ROTATION            f64                           -> spec: `rotation` (supported)
HOLE_SIZE           Coord                         -> spec: `hole_size` (supported)
IS_PLATED           bool                          -> spec: `is_plated` (supported)
LAYER               Layer                         -> spec: `layer` (supported)
STACK_MODE          PcbStackMode                  -> spec: `pad_mode` (defined, NOT WIRED)

SOLDER_MASK_EXP     MaskExpansion                 -> spec: defined, NOT WIRED
PASTE_MASK_EXP      MaskExpansion                 -> spec: defined, NOT WIRED
PLANE_CONNECTION    PlaneConnectionStyle          -> spec: defined, NOT WIRED
RELIEF_WIDTH        Coord                         -> spec: defined, NOT WIRED
RELIEF_ENTRIES      u8                            -> spec: defined, NOT WIRED
RELIEF_AIR_GAP      Coord                         -> spec: defined, NOT WIRED

HOLE_SHAPE          PcbPadHoleShape               -> spec: MISSING
HOLE_SLOT_LENGTH    Coord                         -> spec: MISSING
CORNER_RADIUS_PCT   u8 (0-100)                    -> spec: MISSING
TENTING_MODE        TentingMode                   -> spec: MISSING
DRILL_TYPE          DrillType                     -> spec: MISSING
DAISY_CHAIN         DaisyChainStyle               -> spec: MISSING
IS_TESTPOINT_TOP    bool                          -> spec: MISSING
IS_TESTPOINT_BOTTOM bool                          -> spec: MISSING
PIN_PACKAGE_LENGTH  Coord                         -> spec: MISSING
PROPAGATION_DELAY   f32                           -> spec: MISSING
```

### Component record fields (SchComponent, Record 1)

```
LIBREFERENCE        String                        -> spec: component name (supported)
DESIGNATOR          String                        -> spec: `designator` (supported)
COMPONENTDESCRIPTION String                       -> spec: `description` (supported)
COMPONENTKIND       i32                           -> spec: `component_kind` (supported)
PARTCOUNT           i32                           -> spec: `part_count` (supported)
SHOWHIDDENPINS      Bool                          -> spec: `show_hidden_pins` (supported)

SOURCELIBRARYNAME   String                        -> spec: MISSING
DESIGNITEMID        String                        -> spec: MISSING
UNIQUEID            String                        -> spec: MISSING (auto-generated)
```

### Footprint record fields

```
DISPLAY_NAME        String                        -> spec: footprint name (supported)
DESCRIPTION         String                        -> spec: `description` (supported)
HEIGHT              Coord                         -> spec: `height` (supported)
PATTERN             String                        -> spec: `pattern` (supported)
```

---

## 8. Implementation Roadmap

### Phase 1: Wire existing PadSpec fields (effort: small)

Connect the 7 pad properties already in the model through reconciler -> executor ->
`AddPadOp`. Immediate value: users get mask/thermal relief control.

Files:
- `crates/altium-format-ops/src/spec/reconciler.rs` — add fields to `pad_spec_to_add`
- `crates/altium-format-ops/src/spec/executor.rs` — pass fields to AddPadOp
- `crates/altium-format/src/pcb_ops_core.rs` — add fields to `AddPadOp` struct

### Phase 2: Pin swap groups + IEEE symbols (effort: medium)

Add to PinSpec: `swap_group`, `swap_part`, `swap_pair`, `symbol_inner_edge`,
`symbol_outer_edge`, `symbol_inside`, `symbol_outside`.

Requires format layer additions: swap IDs go in SchPin record + PinMiscData sidecar.
Enables autorouter pin swapping and correct ERC symbol display.

Files:
- `crates/altium-format-ops/src/spec/model.rs` — add PinSpec fields
- `crates/altium-format-ops/src/spec/compiler.rs` — compile new fields
- `crates/altium-format-ops/src/spec/reconciler.rs` — diff new fields
- `crates/altium-format/src/sch_ops_core.rs` — add fields to `PinOp`/`AddPinOp`
- `crates/altium-format/src/schlib.rs` — serialize swap IDs and symbols

### Phase 3: Manufacturing pad metadata (effort: medium)

Add tenting mode, hole shape/type, drill type, test point flags, corner radius.
Enables DRC checks for annular ring, drill constraints, testpoint coverage.

### Phase 4: Component/footprint enrichment (effort: lower priority)

Component class, source library, design item ID. 3D body details, courtyard
semantics, keepout regions. Pin functions/modes for multi-function IC pins.

---

## Sources

### Online documentation
- [Swapping Pins, Pairs and Parts](https://www.altium.com/documentation/altium-designer/sch-pcb/swapping-pins-pairs-parts)
- [Working with Pads & Vias](https://www.altium.com/documentation/altium-designer/pcb/pads-vias)
- [Customizing a Pad Stack](https://www.altium.com/documentation/altium-designer/pcb/custom-pad-stack)
- [Design Rule Types (all categories)](https://www.altium.com/documentation/altium-designer/pcb/design-rule-types/)
- [Working with Classes](https://www.altium.com/documentation/altium-designer/sch-pcb/classes)
- [Defining Differential Pairs](https://www.altium.com/documentation/altium-designer/schematic/defining-differential-pairs)
- [Signal Integrity Rule Types](https://www.altium.com/documentation/altium-designer/pcb/design-rule-types/signal-integrity)
- [Controlled Impedance Routing](https://www.altium.com/documentation/altium-designer/pcb/high-speed-design/interactively-routing-controlled-impedance)

### Internal documentation
- `docs/dxp/schematic-records.md` — pin record fields
- `docs/dxp/pcb-records.md` — pad record fields
- `docs/dxp/pcb-files.md` — section registry, design rules, load pipeline
- `docs/dxp/sidecar-streams-deep-dive.md` — pin sidecars, ExtendedPrimitiveInformation
- `docs/dxp/sch-dotnet-model.md` — component interfaces
- `docs/dxp/pcb-dotnet-model.md` — primitive attributes
- `docs/dxp/altium-types.md` — enum definitions
- `docs/dxp/altium-constants.md` — constant values
