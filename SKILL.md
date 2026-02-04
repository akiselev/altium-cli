---
name: altium-cli
description: >
    Use altium-cli for Altium Designer file operations: schematic/PCB analysis, library browsing, BOM generation, netlist extraction, and design editing.
    Activate when the user requests:
    - Altium file inspection or analysis
    - Schematic document analysis (BOM, netlist, components)
    - PCB document analysis (rules, layers, routing)
    - Library browsing (SchLib, PcbLib, IntLib)
    - Project management (PrjPcb)
    - Footprint measurement or rendering
    - Design editing or modification
---

# altium-cli

A Rust CLI for reading, writing, and querying Altium Designer files. Designed for both direct usage and AI agent integration.

## Architecture Overview

altium-cli is a pure CLI wrapper around the altium-format library:

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   CLI Command   │────▶│  altium-format   │────▶│  Altium Files   │
│  altium-cli     │     │  (Rust library)  │     │  .SchLib, etc.  │
└─────────────────┘     └──────────────────┘     └─────────────────┘
```

**Key concepts:**
- **Pure library**: No daemon, no external dependencies
- **Multi-format**: Supports SchLib, SchDoc, PcbLib, PcbDoc, PrjPcb, IntLib
- **JSON output**: All commands support --json for machine parsing
- **Query language**: CSS-like selectors for finding components

## When to Use

Activate when the user requests:
- Altium file inspection or analysis
- Schematic document analysis (BOM, netlist, components)
- PCB document analysis (rules, layers, routing)
- Library browsing (SchLib, PcbLib, IntLib)
- Project management (PrjPcb)
- Footprint measurement or rendering
- Design editing or modification

## Quick Start

```bash
# Inspect any file
altium-cli inspect components.SchLib --json

# Query components
altium-cli query design.SchDoc "R*"

# Schematic analysis
altium-cli schdoc bom design.SchDoc
altium-cli schdoc netlist design.SchDoc

# PCB analysis
altium-cli pcbdoc rules design.PcbDoc
altium-cli pcbdoc components design.PcbDoc

# Project management
altium-cli prjpcb overview project.PrjPcb
altium-cli prjpcb bom project.PrjPcb --grouped

# Library browsing
altium-cli schlib list components.SchLib
altium-cli pcblib measure footprints.PcbLib SOIC-8
altium-cli intlib search library.IntLib "LM358"
```

## Command Reference

### Quick Commands

| Command | Description |
|---------|-------------|
| `altium-cli inspect <file>` | View file structure and metadata |
| `altium-cli query <file> <selector>` | Find records using selector syntax |
| `altium-cli edit <file> -c <op>` | Edit schematic documents |
| `altium-cli completions <shell>` | Generate shell completions |

### Schematic Document (schdoc)

```bash
# Analysis
altium-cli schdoc overview design.SchDoc
altium-cli schdoc info design.SchDoc
altium-cli schdoc netlist design.SchDoc
altium-cli schdoc netlist design.SchDoc --filter "VCC*"
altium-cli schdoc power-map design.SchDoc

# Components
altium-cli schdoc components design.SchDoc

# Connectivity
altium-cli schdoc wires design.SchDoc
altium-cli schdoc ports design.SchDoc

# Export
altium-cli schdoc json design.SchDoc --full
```

### Schematic Library (schlib)

```bash
# Browse
altium-cli schlib overview components.SchLib
altium-cli schlib list components.SchLib
altium-cli schlib search components.SchLib "LM358"
altium-cli schlib search components.SchLib "op amp" --limit 10
altium-cli schlib info components.SchLib
altium-cli schlib component components.SchLib LM358
altium-cli schlib component components.SchLib LM358 --primitives

# Pins and Primitives
altium-cli schlib pins components.SchLib
altium-cli schlib pins components.SchLib -c LM358
altium-cli schlib primitives components.SchLib LM358

# Create/Edit
altium-cli schlib create new.SchLib
altium-cli schlib add-component lib.SchLib MyPart
altium-cli schlib add-component lib.SchLib MyPart -d "My component description"
altium-cli schlib add-pin lib.SchLib -c MyPart -d 1 -n VCC -e power

# Export
altium-cli schlib json components.SchLib
altium-cli schlib json components.SchLib --full
```

### PCB Document (pcbdoc)

```bash
# Analysis
altium-cli pcbdoc overview design.PcbDoc
altium-cli pcbdoc rules design.PcbDoc
altium-cli pcbdoc components design.PcbDoc --layer top
altium-cli pcbdoc nets design.PcbDoc
altium-cli pcbdoc layers design.PcbDoc

# Routing
altium-cli pcbdoc tracks design.PcbDoc --net VCC
altium-cli pcbdoc vias design.PcbDoc
altium-cli pcbdoc polygons design.PcbDoc

# Edit
altium-cli pcbdoc create new.PcbDoc
altium-cli pcbdoc set-outline-rect design.PcbDoc 0 0 4000 3000
altium-cli pcbdoc add-track design.PcbDoc 0 0 100 100 --net VCC
altium-cli pcbdoc add-via design.PcbDoc 500 500 --net VCC
altium-cli pcbdoc add-rule design.PcbDoc clearance --value 10
```

### PCB Library (pcblib)

```bash
# Browse
altium-cli pcblib overview footprints.PcbLib
altium-cli pcblib list footprints.PcbLib
altium-cli pcblib search footprints.PcbLib "SOIC"
altium-cli pcblib footprint footprints.PcbLib SOIC-8
altium-cli pcblib measure footprints.PcbLib SOIC-8

# Render
altium-cli pcblib render-ascii footprints.PcbLib SOIC-8
altium-cli pcblib render-svg footprints.PcbLib SOIC-8 -o out.svg

# Create/Edit
altium-cli pcblib create new.PcbLib
altium-cli pcblib add-footprint lib.PcbLib SOIC-8
altium-cli pcblib gen-chip lib.PcbLib 0603
altium-cli pcblib add-dual-row lib.PcbLib FP 8 --pitch 50 --span 300
```

### PCB Project (prjpcb)

```bash
# Analysis
altium-cli prjpcb overview project.PrjPcb
altium-cli prjpcb info project.PrjPcb
altium-cli prjpcb documents project.PrjPcb
altium-cli prjpcb documents project.PrjPcb -t Schematic
altium-cli prjpcb bom project.PrjPcb
altium-cli prjpcb bom project.PrjPcb --grouped
altium-cli prjpcb validate project.PrjPcb

# Export
altium-cli prjpcb json project.PrjPcb
```

### Integrated Library (intlib)

```bash
# Browse
altium-cli intlib overview library.IntLib
altium-cli intlib list library.IntLib
altium-cli intlib search library.IntLib "LM358"
altium-cli intlib search library.IntLib "voltage regulator" --limit 10
altium-cli intlib info library.IntLib
altium-cli intlib component library.IntLib LM358

# Embedded Content
altium-cli intlib symbols library.IntLib
altium-cli intlib footprints library.IntLib
altium-cli intlib parameters library.IntLib
altium-cli intlib parameters library.IntLib -c LM358

# Extract
altium-cli intlib extract-schlib library.IntLib -o symbols.SchLib
altium-cli intlib extract-pcblib library.IntLib -o footprints.PcbLib

# Export
altium-cli intlib json library.IntLib
altium-cli intlib json library.IntLib --full
```

## Query Language

### Record Selector (Domain-Specific)

| Pattern | Matches | Example |
|---------|---------|---------|
| `<designator>` | Exact match | `U1` |
| `<prefix>*` | Wildcard suffix | `R*` (all resistors) |
| `<prefix>??` | Fixed-length wildcard | `C??` (C01-C99) |
| `$<part>` | Part number | `$LM358` |
| `~<net>` | Net name | `~VCC` |
| `@<value>` | Value | `@10K` |
| `<comp>:<pin>` | Component.pin | `U1:VCC` |

### SchQL (CSS-Like)

| Selector | Description |
|----------|-------------|
| `component[field=value]` | Exact attribute match |
| `component[field*=value]` | Contains match |
| `component[field^=value]` | Starts with |
| `pin[type=input]` | Pin type filter |
| `net:power` | Net classification |

## Edit Operations

```bash
# Component operations
altium-cli edit design.SchDoc -c "move U1 1000 2000"
altium-cli edit design.SchDoc -c "delete R3"

# Connectivity operations
altium-cli edit design.SchDoc -c "add-wire 100,100,200,100,200,200"
altium-cli edit design.SchDoc -c "add-net-label VCC 1000 2000"
altium-cli edit design.SchDoc -c "add-power GND 500 500 POWER_GND 0"
altium-cli edit design.SchDoc -c "add-junction 1500 1500"
altium-cli edit design.SchDoc -c "add-missing-junctions"

# Routing and validation
altium-cli edit design.SchDoc -c "route U1.VCC 1000,500"
altium-cli edit design.SchDoc -c "validate"
```

## Output Formats

```bash
# Human-readable (default)
altium-cli schdoc components design.SchDoc

# JSON output
altium-cli schdoc components design.SchDoc --json

# Pretty JSON
altium-cli schdoc components design.SchDoc --pretty
```

## Common Analysis Patterns

### Generate BOM

```bash
# Single schematic
altium-cli schdoc bom design.SchDoc --json > bom.json

# Full project (grouped)
altium-cli prjpcb bom project.PrjPcb --grouped --json > bom.json
```

### Check Design Rules

```bash
# List all rules
altium-cli pcbdoc rules design.PcbDoc

# Filter by type
altium-cli pcbdoc rules design.PcbDoc --kind clearance
```

### Measure Footprint

```bash
# Get dimensions
altium-cli pcblib measure footprints.PcbLib SOIC-8

# Visual check
altium-cli pcblib render-ascii footprints.PcbLib SOIC-8
```

### Validate Project

```bash
# Check integrity
altium-cli prjpcb validate project.PrjPcb --check-files

# Compare schematic vs PCB
altium-cli prjpcb diff-sch-pcb project.PrjPcb
```

## Global Options

All commands accept:

| Option | Description |
|--------|-------------|
| `--json` | JSON output |
| `--pretty` | Pretty-printed JSON output |
| `-v, --verbose` | Verbose output |
| `-q, --quiet` | Errors only |

## Coordinate System

Altium uses fixed-point coordinates: **10,000 internal units = 1 mil = 0.001 inch**

All edit coordinates are in mils.

## Supported File Types

| Extension | Type | Read | Write | Query |
|-----------|------|------|-------|-------|
| `.SchLib` | Schematic Library | Yes | Yes | Yes |
| `.SchDoc` | Schematic Document | Yes | Yes | Yes |
| `.PcbLib` | PCB Library | Yes | Yes | Yes |
| `.PcbDoc` | PCB Document | Yes | Yes | Yes |
| `.PrjPcb` | PCB Project | Yes | Yes | - |
| `.IntLib` | Integrated Library | Yes | - | Yes |

## Warning

```
WARNING: EXPERIMENTAL - WORK IN PROGRESS
Breaking changes expected. Use at your own risk.
BACKUP YOUR FILES IN VERSION CONTROL BEFORE USE
```
