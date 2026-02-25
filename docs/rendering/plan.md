# Rendering Fidelity Plan: Pixel-Perfect Altium Output

## Goal

Render Altium files **exactly** as they appear in the Altium Designer UI, including
proper line widths, layer colors, designators, power symbols, port shapes, and all
other visual elements.

## Current State

The renderer produces output but with significant visual discrepancies:
- Line widths are 10x too thin (PenWidth mapping bug)
- SVG transform stack is not replayed (rotated/mirrored components appear wrong)
- Junction dots are too small (hardcoded 5 mil radius vs 10-50 mil)
- Bus widths use wrong calculation (wire width + 1 instead of separate lookup table)
- Default font is wrong ("Times New Roman" vs "Tahoma")
- Line dash patterns not rendered
- All PCB primitives render in black (no layer colors)
- Power symbols, ports, sheet entries all use placeholder geometry

---

## Phase 1: Fix Rendering Bugs (P0/P1)

These affect correctness of existing output. Biggest visual impact.

### 1.1 Fix PenWidth mapping (P0)

**File**: `crates/altium-format/src/render/canvas.rs`

Current (WRONG):
```
Zero → 0.0, Small → 1.0, Medium → 2.0, Large → 5.0
```

Correct (from `Rt_Schematic.Consts.LineWidthArrayC` in decompiled C#):
```
Zero → 0.0, Small → 10.0, Medium → 30.0, Large → 50.0
```

Source: `AD26-dotnet/Altium.Edp.Interfaces/Rt_Schematic/Consts.cs` lines 2880-2885.
Internal units: eZeroSize=0, eSmall=100000, eMedium=300000, eLarge=500000.
Conversion: 10,000 internal units = 1 mil.

### 1.2 Fix Bus widths (P0)

**File**: `crates/altium-format/src/render/sch.rs`

Current (WRONG): `pen_width_to_mils(b.line_width) + 1.0`

Correct (from `Rt_Schematic.Consts.BusLineWidthArrayC`):
```
Zero → 20.0, Small → 30.0, Medium → 50.0, Large → 70.0
```

Source: `Consts.cs` lines 2886-2891.
Internal units: eZeroSize=200000, eSmall=300000, eMedium=500000, eLarge=700000.

Need a separate `bus_width_to_mils()` function in `canvas.rs`.

### 1.3 Fix Junction sizes (P1)

**File**: `crates/altium-format/src/render/sch.rs`

Current (WRONG): hardcoded 5.0 mil radius.

Correct (from `Rt_Schematic.Consts.cJunctionSizeArray` — these are **diameters**):
```
Zero → 20.0 mils, Small → 30.0, Medium → 50.0, Large → 100.0
```

Source: `Consts.cs` lines 2458-2463.

Note: Our `SchJunction` struct doesn't parse a `size` field yet. The actual
junction record stores a `JUNCTIONSIZE` parameter (maps to `TSize` enum).
For now, use default Small (30 mils diameter = 15 mils radius).
TODO: Add `size: PenWidth` field to `SchJunction` parser.

### 1.4 Fix default font (P1)

**File**: `crates/altium-format/src/render/canvas.rs`

Current: `"Times New Roman"`, 10.0 mils.
Correct: `"Tahoma"`, 10.0 mils.

Source: `DrawGraphObjectBase.GetFontInfo()` fallback in
`AD26-dotnet/Altium.Sch.Painter/Altium.Sch.Painter/DrawGraphObjectBase.cs`.

### 1.5 Implement SVG transform stack (P0)

**File**: `crates/altium-format-render-svg/src/lib.rs`

Currently `PushTransform`/`PopTransform` are no-ops. Need to:
1. Maintain a transform stack depth counter
2. On `PushTransform`: open a `<g transform="...">` group element
3. On `PopTransform`: close the `</g>` group
4. Handle all three transform types:
   - `Rotate { degrees, origin }` → `translate(ox,oy) rotate(-deg) translate(-ox,-oy)`
     (negative because SVG Y is flipped)
   - `Mirror { axis_x }` → `translate(2*ax,0) scale(-1,1)` (after Y-flip adjustment)
   - `Scale { sx, sy, origin }` → `translate(ox,oy) scale(sx,sy) translate(-ox,-oy)`

### 1.6 Add SVG stroke defaults (P1)

**File**: `crates/altium-format-render-svg/src/lib.rs`

Add to all stroked elements:
```
stroke-linecap="round"
stroke-linejoin="round"
```

Source: `PenInfo.cs` — default cap is `CapStyle.Round`, default join is `LineJoin.Round`.

Alternatively, set once on a root `<g>` element wrapping all content.

### 1.7 Implement line dash patterns (P1)

**File**: `crates/altium-format-render-svg/src/lib.rs`

Map `LineStyle` to SVG `stroke-dasharray`:

| LineStyle | SVG stroke-dasharray | Source |
|---|---|---|
| Solid | (none) | — |
| Dashed | `"4"` (= `[2.0, 2.0]` × pen width) | `DashStyleInfoHelper.cs` |
| Dotted | `"2"` (= `[1.0, 2.0]` × pen width) | `DashStyleInfoHelper.cs` |
| DashDot | `"4 2"` | `SvgGraphics.cs` |

Source: `AD26-dotnet/Altium.Sch.Painter/SvgGraphics.cs` has the exact SVG dash values.

---

## Phase 2: Schematic Completeness

### 2.1 Power symbol shapes (11 types)

**File**: `crates/altium-format/src/render/sch.rs`

Implement all styles from `PowerObjectDrawGraphObject.cs`. Key geometry constants
(all in internal units, 10,000 = 1 mil):

| Style | Enum value | Description | Key dimensions |
|---|---|---|---|
| Circle | 0 | Circle at pin endpoint | radius=30 mils |
| Arrow | 1 | Two angled lines | spread=30 mils |
| Bar | 2 (default) | Horizontal bar | half-width=50 mils |
| Wave | 3 | Sine wave | radius=40 mils |
| GndPower | 4 | 4 decreasing lines | widths: 100,70,40,10 mils; spacing=30 mils |
| GndSignal | 5 | Triangle | 3 lines |
| GndEarth | 6 | 3 angled lines | Earth ground |
| GostArrow | 7 | Arrow (GOST) | length=160 mils |
| GostGndPower | 8 | 3 lines (GOST) | widths: 100,60,20 mils; spacing=40 mils |
| GostGndEarth | 9 | GOST earth+circle | circle radius=120 mils |
| GostBar | 10 | Thick bar | half-width=80 mils, length=200 mils |

Power pin line width: 10 mils (100000 internal units).
Standard pin length: 100 mils; GOST: 160 mils; GOST Bar: 200 mils.

### 2.2 Port arrow shapes (7 styles)

**File**: `crates/altium-format/src/render/sch.rs`

Implement `TPortArrowStyle` variants from `PortDrawGraphObject.cs`:
- None, Left, Right, LeftRight, Top, Bottom, TopBottom, NoneVertical

Each produces a different polygon (pointed/flat ends).

### 2.3 Sheet entry shapes (4 kinds)

**File**: `crates/altium-format/src/render/sch.rs`

Implement `TArrowKind` from `SheetEntryDrawGraphObject.cs`:
- RectAndTri (width=150 mils), Triangle (80), Arrow (120), ArrowTail (150)

### 2.4 Pin IEEE symbols (13+ types)

Implement decorations from `PinDrawGraphObject.cs`:
- Dot, CLK, ActiveLow, LeftRightSignalFlow, etc.
- Symbol offset constants: DotXOffset=30, CLKXOffset=40, SchmittXOffset=180 mils

### 2.5 Pin text positioning

Use margin constants from `PinNameUtils.CalculatePinNamePosition()`:
- PinNameMargin, PinNumberMargin (from preferences)
- Support Default mode and Custom mode (per-pin font, rotation, anchor)

---

## Phase 3: PCB Layer Rendering

### 3.1 Parse layer color table

**Source**: Board configuration in CFB `Board6` section.
Layer colors stored as `LAYERV7_*COLOR` parameters.
`IPCB_Board.GetState_LayerColor(TV6_Layer)` reads from this.

Need:
- Parse board config parameters for layer colors
- Build `LayerColorTable: HashMap<V6Layer, Color>`
- Pass to PCB renderer

### 3.2 Layer-ordered rendering

Sort primitives by layer before drawing. Draw order: bottom layer → top layer.
Within each layer, draw by primitive type order (fills first, then regions,
tracks, pads, text last).

### 3.3 Per-layer pad stack

Select `shape_top`/`shape_mid`/`shape_bot` based on current render layer.
Read `corner_radius_pct[]` from `PcbPadStackData` instead of hardcoded 25%.

### 3.4 Default layer colors

Fallback colors when no board config is available (for PcbLib footprint rendering):

| Layer | Default color |
|---|---|
| TopLayer | Red `#FF0000` |
| BottomLayer | Blue `#0000FF` |
| TopOverlay (silkscreen) | White `#FFFFFF` |
| BottomOverlay | Yellow `#FFFF00` |
| TopSolderMask | Purple `#800080` |
| BottomSolderMask | Purple `#800080` |
| TopPaste | Gray `#808080` |
| MultiLayer | Gray `#C0C0C0` |
| KeepOutLayer | Magenta `#FF00FF` |
| Mechanical1-32 | Yellow `#FFFF00` |

---

## Phase 4: Full Document Rendering

### 4.1 Sheet/document rendering (SchDoc)

From `DocumentDrawGraphObject.cs`:
- Outer rectangle with area color fill + border
- Inner margin rectangle
- Reference zones (horizontal numeric, vertical alphabetic)
- Grid (dot grid or line grid)
- Title block (standard fields at fixed positions)

### 4.2 Region holes (multi-contour)

Support inner cutout contours for copper pour regions.

### 4.3 Component color overrides

Implement `OverideColors` flag from `SchComponentDrawGraphObject.cs`.

### 4.4 Text metrics

Needed for:
- Auto-sizing ports
- Proper designator/parameter placement
- Port width calculation

---

## Verification

After each phase, render test files and visually compare against Altium screenshots:

```bash
# Render all SchLib components
cargo run -p altium-cli -- render data/schlib/*.SchLib -o /tmp/render-test/ --format svg

# Render all PcbLib footprints
cargo run -p altium-cli -- render data/pcblib/*.PcbLib -o /tmp/render-test/ --format svg

# Render SchDoc sheets
cargo run -p altium-cli -- render data/schdoc/*.SchDoc -o /tmp/render-test/ --format svg
```

Key visual checks:
- Wire thickness matches Altium (should be ~10 mils for Small)
- Rotated components appear in correct orientation
- Junction dots are visible and properly sized
- Bus lines are thicker than wires
- Dashed/dotted lines show correct patterns
- Power symbols have correct shapes (GND, VCC, etc.)
