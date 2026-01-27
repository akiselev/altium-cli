# schlib - Schematic Library Commands

Browse, search, create, and edit schematic libraries (.SchLib).

## Browse Commands

| Command | Purpose |
|---------|---------|
| `overview` | Library overview with categories |
| `list` | List all components |
| `search` | Search by name or description |
| `info` | Library info and statistics |

## Component Commands

| Command | Purpose |
|---------|---------|
| `component` | Show component details |
| `pins` | List pins for component |
| `primitives` | List component primitives |

## Rendering Commands

| Command | Purpose |
|---------|---------|
| `render-ascii` | Render component as ASCII art |
| `json` | Export as JSON |

## Editing Commands

| Command | Purpose |
|---------|---------|
| `create` | Create new schematic library |
| `add-component` | Add component to library |
| `add-pin` | Add pin to component |
| `add-rectangle` | Add rectangle primitive |
| `add-line` | Add line primitive |
| `add-polygon` | Add polygon primitive |
| `gen-ic` | Generate IC symbol from pinout |
| `add-json` | Add component from JSON definition |

## Examples

### Browsing

```bash
# Library overview
altium-cli schlib overview components.SchLib
altium-cli schlib overview components.SchLib --full

# List components
altium-cli schlib list components.SchLib

# Search
altium-cli schlib search components.SchLib "LM358"
altium-cli schlib search components.SchLib "op amp" --limit 10

# Library info
altium-cli schlib info components.SchLib
```

### Component Inspection

```bash
# Component details
altium-cli schlib component components.SchLib LM358
altium-cli schlib component components.SchLib LM358 --params

# List pins
altium-cli schlib pins components.SchLib -c LM358

# List primitives
altium-cli schlib primitives components.SchLib LM358

# ASCII rendering
altium-cli schlib render-ascii components.SchLib LM358
```

### Creating Libraries

```bash
# Create new library
altium-cli schlib create new.SchLib

# Add component
altium-cli schlib add-component lib.SchLib MyOpAmp

# Add pins
altium-cli schlib add-pin lib.SchLib MyOpAmp -n VCC -x 0 -y 100 --electrical power
altium-cli schlib add-pin lib.SchLib MyOpAmp -n IN+ -x -100 -y 50 --electrical input
altium-cli schlib add-pin lib.SchLib MyOpAmp -n IN- -x -100 -y -50 --electrical input
altium-cli schlib add-pin lib.SchLib MyOpAmp -n OUT -x 100 -y 0 --electrical output
altium-cli schlib add-pin lib.SchLib MyOpAmp -n GND -x 0 -y -100 --electrical power

# Add primitives
altium-cli schlib add-rectangle lib.SchLib MyOpAmp -50 -75 50 75
altium-cli schlib add-line lib.SchLib MyOpAmp 0 0 50 50
altium-cli schlib add-polygon lib.SchLib MyOpAmp "0,0;100,50;100,-50"
```

### IC Generation

```bash
# Generate IC from pin list
altium-cli schlib gen-ic lib.SchLib ATmega328P --pins "VCC,GND,PB0,PB1,PB2,PB3,PB4,PB5,PC0,PC1,PC2,PC3,PC4,PC5,PD0,PD1,PD2,PD3,PD4,PD5,PD6,PD7,RESET,XTAL1,XTAL2,AVCC,AREF"

# Generate with pin arrangement
altium-cli schlib gen-ic lib.SchLib 74HC595 --left "SER,SRCLK,RCLK,OE,SRCLR" --right "QA,QB,QC,QD,QE,QF,QG,QH,QH'" --top "VCC" --bottom "GND"
```

### JSON Import

```bash
# Add component from JSON file
altium-cli schlib add-json lib.SchLib component.json
```

JSON format:
```json
{
  "name": "MyComponent",
  "description": "Custom component",
  "pins": [
    {"name": "VCC", "x": 0, "y": 100, "electrical": "power"},
    {"name": "GND", "x": 0, "y": -100, "electrical": "power"}
  ],
  "primitives": [
    {"type": "rectangle", "x1": -50, "y1": -75, "x2": 50, "y2": 75}
  ]
}
```

## Command Details

### add-pin

Add pin to component.

```bash
altium-cli schlib add-pin <PATH> <COMPONENT> [OPTIONS]

Options:
  -n, --name <NAME>          Pin name
  -x <X>                     X coordinate (mils)
  -y <Y>                     Y coordinate (mils)
  --electrical <TYPE>        Pin electrical type: input, output, io, power, passive
  --orientation <DIR>        Pin orientation: left, right, up, down
```

### gen-ic

Generate IC symbol from pin list.

```bash
altium-cli schlib gen-ic <PATH> <NAME> [OPTIONS]

Options:
  --pins <PINS>    Comma-separated pin names (auto-arranged)
  --left <PINS>    Pins on left side
  --right <PINS>   Pins on right side
  --top <PINS>     Pins on top
  --bottom <PINS>  Pins on bottom
```
