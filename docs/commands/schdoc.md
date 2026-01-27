# schdoc - Schematic Document Commands

Analysis and export commands for schematic documents (.SchDoc).

## Analysis Commands

| Command | Purpose |
|---------|---------|
| `overview` | Complete design overview with categories, power, interfaces |
| `bom` | Bill of materials grouped by component |
| `netlist` | Net connectivity map |
| `power-map` | Power distribution analysis |
| `blocks` | Block diagram of major ICs |
| `signal-flow` | Signal flow tracing |
| `project` | Multi-file hierarchical analysis |

## Info Commands

| Command | Purpose |
|---------|---------|
| `info` | Document info and sheet metadata |
| `stats` | Detailed record statistics |
| `hierarchy` | Show record hierarchy tree |

## Component Commands

| Command | Purpose |
|---------|---------|
| `components` | List all components |
| `component` | Show component details |
| `pins` | List pins (optionally filtered by component) |

## Connectivity Commands

| Command | Purpose |
|---------|---------|
| `wires` | List all wires |
| `nets` | List all net labels |
| `ports` | List all ports |
| `power` | List all power objects |
| `junctions` | List all junctions |

## Export Commands

| Command | Purpose |
|---------|---------|
| `json` | Export as JSON for LLM processing |

## Examples

```bash
# Design analysis
altium-cli schdoc overview design.SchDoc
altium-cli schdoc bom design.SchDoc --json
altium-cli schdoc netlist design.SchDoc
altium-cli schdoc power-map design.SchDoc

# Block diagrams and signal flow
altium-cli schdoc blocks design.SchDoc
altium-cli schdoc signal-flow design.SchDoc CLK

# Multi-file project analysis
altium-cli schdoc project sheet1.SchDoc sheet2.SchDoc sheet3.SchDoc

# Document info
altium-cli schdoc info design.SchDoc
altium-cli schdoc stats design.SchDoc
altium-cli schdoc hierarchy design.SchDoc

# Component inspection
altium-cli schdoc components design.SchDoc
altium-cli schdoc component design.SchDoc U1
altium-cli schdoc pins design.SchDoc -c U1

# Connectivity
altium-cli schdoc wires design.SchDoc
altium-cli schdoc nets design.SchDoc --group
altium-cli schdoc ports design.SchDoc
altium-cli schdoc power design.SchDoc --group
altium-cli schdoc junctions design.SchDoc

# JSON export
altium-cli schdoc json design.SchDoc --full --pretty
```

## Command Details

### overview

Complete design overview including component categories, power analysis, and interface summary.

```bash
altium-cli schdoc overview <PATH>
```

### bom

Generate bill of materials with component grouping.

```bash
altium-cli schdoc bom <PATH>
```

### netlist

Generate net connectivity map showing all connections.

```bash
altium-cli schdoc netlist <PATH>
```

### power-map

Analyze power distribution showing power nets and connected components.

```bash
altium-cli schdoc power-map <PATH>
```

### signal-flow

Trace signal flow through the design starting from a named signal.

```bash
altium-cli schdoc signal-flow <PATH> <SIGNAL>

Arguments:
  <PATH>    Path to .SchDoc file
  <SIGNAL>  Signal name to trace (e.g., CLK, DATA, RESET)
```

### project

Analyze multi-file hierarchical project.

```bash
altium-cli schdoc project <PATH>...

Arguments:
  <PATH>...  Paths to all .SchDoc files in project
```

### components

List all components with optional filtering.

```bash
altium-cli schdoc components <PATH>
```

### nets

List all net labels with optional grouping.

```bash
altium-cli schdoc nets <PATH> [OPTIONS]

Options:
  --group  Group nets by name
```

### json

Export full document as JSON for processing.

```bash
altium-cli schdoc json <PATH> [OPTIONS]

Options:
  --full    Include full component details
  --pretty  Pretty-print JSON output
```
