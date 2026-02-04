# schdoc - Schematic Document Commands

Analysis and export commands for schematic documents (.SchDoc).

## Analysis Commands

| Command | Purpose |
|---------|---------|
| `overview` | Complete design overview with categories and statistics |
| `info` | Document info and sheet metadata |
| `netlist` | Net connectivity map |
| `power-map` | Power distribution analysis |

## Component Commands

| Command | Purpose |
|---------|---------|
| `components` | List all placed components |

## Connectivity Commands

| Command | Purpose |
|---------|---------|
| `wires` | List all wire primitives |
| `ports` | List all port definitions |

## Export Commands

| Command | Purpose |
|---------|---------|
| `json` | Export as JSON for LLM processing |

## Examples

### Design Analysis

```bash
# Complete design overview
altium-cli schdoc overview design.SchDoc

# Document info
altium-cli schdoc info design.SchDoc

# Net connectivity
altium-cli schdoc netlist design.SchDoc
altium-cli schdoc netlist design.SchDoc --filter "VCC*"

# Power distribution
altium-cli schdoc power-map design.SchDoc
```

### Component Inspection

```bash
# List all components
altium-cli schdoc components design.SchDoc
```

### Connectivity

```bash
# List wires
altium-cli schdoc wires design.SchDoc

# List ports
altium-cli schdoc ports design.SchDoc
```

### JSON Export

```bash
# Basic export
altium-cli schdoc json design.SchDoc

# Full details
altium-cli schdoc json design.SchDoc --full
```

## Command Details

### overview

Complete design overview with component categories, power analysis, and statistics.

```bash
altium-cli schdoc overview <PATH>
```

Output includes:
- Sheet information (size, title block)
- Component count by category
- Net summary
- Power net analysis

### info

Detailed sheet metadata and properties.

```bash
altium-cli schdoc info <PATH>
```

Output includes:
- Sheet size and style
- Title block information
- Grid settings
- Record counts

### netlist

Extract net connectivity information.

```bash
altium-cli schdoc netlist <PATH> [OPTIONS]

Options:
  -f, --filter <PATTERN>  Filter nets by name pattern
```

Output includes:
- Net names
- Connected pins
- Component connections

### power-map

Analyze power distribution and connections.

```bash
altium-cli schdoc power-map <PATH>
```

Output includes:
- Power net names
- Power port types
- Connected components
- Power symbol locations

### components

List all placed components.

```bash
altium-cli schdoc components <PATH>
```

Output includes:
- Designator
- Library reference
- Part number
- Location

### wires

List wire primitives for routing analysis.

```bash
altium-cli schdoc wires <PATH>
```

Output includes:
- Wire endpoints
- Wire index
- Associated net (if any)

### ports

List port definitions for hierarchical designs.

```bash
altium-cli schdoc ports <PATH>
```

Output includes:
- Port name
- Port I/O type (input, output, bidirectional)
- Location

### json

Export full document as JSON for processing.

```bash
altium-cli schdoc json <PATH> [OPTIONS]

Options:
  --full  Include full primitive details
```
