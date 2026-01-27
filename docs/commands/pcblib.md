# pcblib - PCB Library Commands

Browse, search, create, and edit PCB footprint libraries (.PcbLib).

## Browse Commands

| Command | Purpose |
|---------|---------|
| `overview` | Library overview |
| `list` | List all footprints |
| `search` | Search footprints |
| `info` | Library info and statistics |

## Footprint Commands

| Command | Purpose |
|---------|---------|
| `footprint` | Show footprint details |
| `pads` | List pads |
| `primitives` | List footprint primitives |
| `holes` | Analyze hole sizes |
| `measure` | Measure dimensions and clearances |

## Rendering Commands

| Command | Purpose |
|---------|---------|
| `render-ascii` | Render footprint as ASCII art |
| `render-svg` | Render footprint as SVG |
| `render-png` | Render footprint as PNG |
| `json` | Export as JSON |

## Editing Commands

| Command | Purpose |
|---------|---------|
| `create` | Create new PCB library |
| `add-footprint` | Add footprint to library |
| `add-pad` | Add pad to footprint |
| `add-silkscreen` | Add silkscreen outline |
| `add-arc` | Add arc to footprint |
| `gen-chip` | Generate chip footprint (0402, 0603, etc.) |
| `add-json` | Add footprint from JSON definition |

## Pad Generation Commands

| Command | Purpose |
|---------|---------|
| `add-pad-row` | Add row of pads |
| `add-dual-row` | Add dual-row pads (SOIC, DIP) |
| `add-quad-pads` | Add quad pads (QFP, QFN) |
| `add-pad-grid` | Add grid of pads (BGA) |

## Examples

### Browsing

```bash
# Library overview
altium-cli pcblib overview footprints.PcbLib

# List footprints
altium-cli pcblib list footprints.PcbLib

# Search
altium-cli pcblib search footprints.PcbLib "SOIC"
altium-cli pcblib search footprints.PcbLib "QFP" --limit 10

# Library info
altium-cli pcblib info footprints.PcbLib
```

### Footprint Inspection

```bash
# Footprint details
altium-cli pcblib footprint footprints.PcbLib SOIC-8

# List pads
altium-cli pcblib pads footprints.PcbLib -f SOIC-8
altium-cli pcblib pads footprints.PcbLib -f SOIC-8 --details

# List primitives
altium-cli pcblib primitives footprints.PcbLib SOIC-8

# Analyze holes
altium-cli pcblib holes footprints.PcbLib

# Measure dimensions
altium-cli pcblib measure footprints.PcbLib SOIC-8
```

### Rendering

```bash
# ASCII art
altium-cli pcblib render-ascii footprints.PcbLib SOIC-8

# SVG output
altium-cli pcblib render-svg footprints.PcbLib SOIC-8 -o soic8.svg

# PNG output
altium-cli pcblib render-png footprints.PcbLib SOIC-8 -o soic8.png --scale 2

# JSON export
altium-cli pcblib json footprints.PcbLib --full --pretty
```

### Creating Libraries

```bash
# Create new library
altium-cli pcblib create new.PcbLib

# Add footprint
altium-cli pcblib add-footprint lib.PcbLib SOIC-8

# Add pads manually
altium-cli pcblib add-pad lib.PcbLib SOIC-8 1 -150 -75 --shape rectangle --width 60 --height 30
altium-cli pcblib add-pad lib.PcbLib SOIC-8 2 -150 -25 --shape rectangle --width 60 --height 30
# ... continue for all pads

# Add silkscreen outline
altium-cli pcblib add-silkscreen lib.PcbLib SOIC-8 "-100,-100;100,-100;100,100;-100,100"

# Add arc (for pin 1 marker)
altium-cli pcblib add-arc lib.PcbLib SOIC-8 -75 75 10 0 360
```

### Chip Footprint Generation

```bash
# Generate standard chip footprints
altium-cli pcblib gen-chip lib.PcbLib 0402
altium-cli pcblib gen-chip lib.PcbLib 0603
altium-cli pcblib gen-chip lib.PcbLib 0805
altium-cli pcblib gen-chip lib.PcbLib 1206

# Custom chip dimensions
altium-cli pcblib gen-chip lib.PcbLib CUSTOM_CHIP --length 100 --width 50 --pad-width 30 --pad-height 40
```

### Pad Row Generation

```bash
# Single row of pads (e.g., header)
altium-cli pcblib add-pad-row lib.PcbLib HEADER_8 8 --pitch 100 --start-number 1

# Dual row (SOIC, DIP, SOP)
altium-cli pcblib add-dual-row lib.PcbLib SOIC-8 8 --pitch 50 --span 300

# Quad pads (QFP, QFN)
altium-cli pcblib add-quad-pads lib.PcbLib QFP-32 32 --pitch 50 --body-size 700

# BGA grid
altium-cli pcblib add-pad-grid lib.PcbLib BGA-64 8 8 --pitch 80 --ball-diameter 40
```

### JSON Import

```bash
# Add footprint from JSON file
altium-cli pcblib add-json lib.PcbLib footprint.json
```

JSON format:
```json
{
  "name": "CUSTOM-8",
  "description": "Custom 8-pin footprint",
  "pads": [
    {"number": "1", "x": -150, "y": -75, "width": 60, "height": 30, "shape": "rectangle"},
    {"number": "2", "x": -150, "y": -25, "width": 60, "height": 30, "shape": "rectangle"}
  ],
  "silkscreen": [
    {"type": "line", "x1": -100, "y1": -100, "x2": 100, "y2": -100},
    {"type": "arc", "x": -75, "y": 75, "radius": 10}
  ]
}
```

## Command Details

### add-pad

Add pad to footprint.

```bash
altium-cli pcblib add-pad <PATH> <FOOTPRINT> <NUMBER> <X> <Y> [OPTIONS]

Options:
  --shape <SHAPE>    Pad shape: rectangle, round, oval
  --width <WIDTH>    Pad width (mils)
  --height <HEIGHT>  Pad height (mils)
  --hole <HOLE>      Hole diameter for through-hole (mils)
  --layer <LAYER>    Layer: top, bottom, multi
```

### add-dual-row

Add dual-row pads (SOIC, DIP, SOP patterns).

```bash
altium-cli pcblib add-dual-row <PATH> <FOOTPRINT> <PINS> [OPTIONS]

Arguments:
  <PINS>  Total pin count (must be even)

Options:
  --pitch <PITCH>    Pin pitch (mils)
  --span <SPAN>      Row-to-row span (mils)
  --pad-width <W>    Pad width (mils)
  --pad-height <H>   Pad height (mils)
```

### add-quad-pads

Add quad pads (QFP, QFN patterns).

```bash
altium-cli pcblib add-quad-pads <PATH> <FOOTPRINT> <PINS> [OPTIONS]

Arguments:
  <PINS>  Total pin count (must be divisible by 4)

Options:
  --pitch <PITCH>      Pin pitch (mils)
  --body-size <SIZE>   Body size (mils)
  --pad-width <W>      Pad width (mils)
  --pad-height <H>     Pad height (mils)
```

### add-pad-grid

Add grid of pads (BGA pattern).

```bash
altium-cli pcblib add-pad-grid <PATH> <FOOTPRINT> <ROWS> <COLS> [OPTIONS]

Options:
  --pitch <PITCH>           Grid pitch (mils)
  --ball-diameter <DIA>     Ball/pad diameter (mils)
  --depopulate <POSITIONS>  Positions to skip (e.g., "A1,B2,C3")
```

### measure

Measure footprint dimensions.

```bash
altium-cli pcblib measure <PATH> <FOOTPRINT>
```

Output includes:
- Overall dimensions (width x height)
- Pad-to-pad spacing
- Minimum clearances
- Hole sizes (for through-hole)
- Courtyard bounds
