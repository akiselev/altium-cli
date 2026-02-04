# schlib - Schematic Library Commands

Browse, search, create, and edit schematic libraries (.SchLib).

## Browse Commands

| Command | Purpose |
|---------|---------|
| `overview` | Library overview with categories and statistics |
| `list` | List all components |
| `search` | Search by name or description |
| `info` | Library info and statistics |

## Component Commands

| Command | Purpose |
|---------|---------|
| `component` | Show component details |
| `pins` | List pins (all or for specific component) |
| `primitives` | List component primitives |

## Editing Commands

| Command | Purpose |
|---------|---------|
| `create` | Create new schematic library |
| `add-component` | Add component to library |
| `add-pin` | Add pin to component |

## Export Commands

| Command | Purpose |
|---------|---------|
| `json` | Export as JSON |

## Examples

### Browsing

```bash
# Library overview
altium-cli schlib overview components.SchLib

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
altium-cli schlib component components.SchLib LM358 --primitives

# List pins
altium-cli schlib pins components.SchLib
altium-cli schlib pins components.SchLib -c LM358

# List primitives
altium-cli schlib primitives components.SchLib LM358
```

### Creating Libraries

```bash
# Create new library
altium-cli schlib create new.SchLib

# Add component
altium-cli schlib add-component lib.SchLib MyOpAmp
altium-cli schlib add-component lib.SchLib MyOpAmp -d "Dual operational amplifier"

# Add pins
altium-cli schlib add-pin lib.SchLib -c MyOpAmp -d 1 -n VCC -e power
altium-cli schlib add-pin lib.SchLib -c MyOpAmp -d 2 -n IN+ -e input
altium-cli schlib add-pin lib.SchLib -c MyOpAmp -d 3 -n IN- -e input
altium-cli schlib add-pin lib.SchLib -c MyOpAmp -d 4 -n OUT -e output
altium-cli schlib add-pin lib.SchLib -c MyOpAmp -d 5 -n GND -e power
```

### JSON Export

```bash
# Basic export
altium-cli schlib json components.SchLib

# Full details
altium-cli schlib json components.SchLib --full
```

## Command Details

### overview

Library overview with component categories and statistics.

```bash
altium-cli schlib overview <PATH>
```

Output includes:
- Library path
- Total component count
- Components grouped by category (IC, Resistor, Capacitor, etc.)
- Pin count statistics

### search

Search for components by name or description.

```bash
altium-cli schlib search <PATH> <QUERY> [OPTIONS]

Options:
  -l, --limit <LIMIT>  Maximum results to return
```

### component

Show detailed component information.

```bash
altium-cli schlib component <PATH> <NAME> [OPTIONS]

Options:
  --primitives  Show primitive details
```

### pins

List pins (all components or filtered).

```bash
altium-cli schlib pins <PATH> [OPTIONS]

Options:
  -c, --component <NAME>  Filter by component name
```

### add-pin

Add pin to component.

```bash
altium-cli schlib add-pin <PATH> [OPTIONS]

Options:
  -c, --component <NAME>    Component name (required)
  -d, --designator <NUM>    Pin designator (required)
  -n, --name <NAME>         Pin name (required)
  -e, --electrical-type <TYPE>  Pin electrical type: input, output, bidirectional, passive, power (default: passive)
```
