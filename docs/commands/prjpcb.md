# prjpcb - PCB Project Commands

Analysis commands for PCB projects (.PrjPcb).

## Analysis Commands

| Command | Purpose |
|---------|---------|
| `overview` | Project overview with document counts and statistics |
| `info` | Project info and metadata |
| `documents` | List referenced documents |
| `bom` | Generate BOM from schematic sheets |
| `validate` | Validate project integrity |

## Export Commands

| Command | Purpose |
|---------|---------|
| `json` | Export as JSON for LLM processing |

## Examples

### Project Analysis

```bash
# Project overview
altium-cli prjpcb overview project.PrjPcb

# Project info
altium-cli prjpcb info project.PrjPcb

# List documents
altium-cli prjpcb documents project.PrjPcb
altium-cli prjpcb documents project.PrjPcb -t Schematic
altium-cli prjpcb documents project.PrjPcb -t PCB
```

### Bill of Materials

```bash
# Basic BOM
altium-cli prjpcb bom project.PrjPcb

# Grouped BOM (by part number with quantity)
altium-cli prjpcb bom project.PrjPcb --grouped
```

### Validation

```bash
# Validate project
altium-cli prjpcb validate project.PrjPcb
```

### JSON Export

```bash
# Export project as JSON
altium-cli prjpcb json project.PrjPcb
```

## Command Details

### overview

Project overview with document counts and statistics.

```bash
altium-cli prjpcb overview <PATH>
```

Output includes:
- Project name and path
- Document count by type
- Hierarchy mode
- Project parameters

### info

Detailed project metadata and configuration.

```bash
altium-cli prjpcb info <PATH>
```

Output includes:
- Project file path
- Hierarchy mode
- Current document references
- Configuration settings

### documents

List referenced documents in the project.

```bash
altium-cli prjpcb documents <PATH> [OPTIONS]

Options:
  -t, --doc-type <TYPE>  Filter by type: Schematic, PCB, SchLib, PcbLib, IntLib, OutJob
```

Output includes:
- Document path
- Document type
- File existence status

### bom

Generate bill of materials from schematic sheets.

```bash
altium-cli prjpcb bom <PATH> [OPTIONS]

Options:
  -g, --grouped  Group components by part number with quantity
```

Output includes:
- Designator
- Part number / Library reference
- Description
- Quantity (when grouped)
- Source sheet

### validate

Validate project integrity.

```bash
altium-cli prjpcb validate <PATH>
```

Checks:
- Document references exist
- No duplicate designators across sheets
- Required project parameters present
- File path consistency

### json

Export project as JSON for processing.

```bash
altium-cli prjpcb json <PATH>
```

Output includes full project structure in JSON format.
