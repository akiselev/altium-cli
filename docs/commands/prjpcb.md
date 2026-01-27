# prjpcb - PCB Project Commands

Management and analysis commands for PCB projects (.PrjPcb).

## Analysis Commands

| Command | Purpose |
|---------|---------|
| `overview` | Project overview with documents and parameters |
| `info` | Project info and metadata |
| `documents` | List project documents |
| `parameters` | Show project parameters |
| `netlist` | Show project netlist |
| `components` | List all components in project |
| `bom` | Generate BOM for project |
| `diff-sch-pcb` | Show differences between schematic and PCB |
| `validate` | Validate project |

## Editing Commands

| Command | Purpose |
|---------|---------|
| `create` | Create new project |
| `add-document` | Add document to project |
| `remove-document` | Remove document from project |
| `set-parameter` | Set project parameter |
| `remove-parameter` | Remove project parameter |

## Sync Commands

| Command | Purpose |
|---------|---------|
| `import-to-pcb` | Import design to PCB |
| `sync-to-pcb` | Sync schematic changes to PCB |

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
altium-cli prjpcb documents project.PrjPcb --doc-type schematic
altium-cli prjpcb documents project.PrjPcb --doc-type pcb

# Project parameters
altium-cli prjpcb parameters project.PrjPcb
```

### Design Analysis

```bash
# Project-wide netlist
altium-cli prjpcb netlist project.PrjPcb

# All components across sheets
altium-cli prjpcb components project.PrjPcb

# Bill of materials
altium-cli prjpcb bom project.PrjPcb
altium-cli prjpcb bom project.PrjPcb --grouped

# Schematic vs PCB differences
altium-cli prjpcb diff-sch-pcb project.PrjPcb
altium-cli prjpcb diff-sch-pcb project.PrjPcb --pcb board.PcbDoc
```

### Validation

```bash
# Basic validation
altium-cli prjpcb validate project.PrjPcb

# Check file existence
altium-cli prjpcb validate project.PrjPcb --check-files
```

### Project Creation

```bash
# Create new project
altium-cli prjpcb create new.PrjPcb
altium-cli prjpcb create new.PrjPcb --name "My Project"

# Create from template
altium-cli prjpcb create new.PrjPcb --template template.PrjPcb
```

### Document Management

```bash
# Add documents
altium-cli prjpcb add-document project.PrjPcb sheet1.SchDoc
altium-cli prjpcb add-document project.PrjPcb sheet2.SchDoc
altium-cli prjpcb add-document project.PrjPcb board.PcbDoc

# Remove document
altium-cli prjpcb remove-document project.PrjPcb old_sheet.SchDoc
```

### Parameter Management

```bash
# Set parameters
altium-cli prjpcb set-parameter project.PrjPcb Revision "1.0"
altium-cli prjpcb set-parameter project.PrjPcb Author "John Doe"
altium-cli prjpcb set-parameter project.PrjPcb Date "2024-01-15"
altium-cli prjpcb set-parameter project.PrjPcb "Part Number" "ASSY-001"

# Remove parameter
altium-cli prjpcb remove-parameter project.PrjPcb "Old Param"
```

### Schematic to PCB Sync

```bash
# Initial import (new PCB)
altium-cli prjpcb import-to-pcb project.PrjPcb
altium-cli prjpcb import-to-pcb project.PrjPcb --pcb board.PcbDoc

# Dry run first
altium-cli prjpcb import-to-pcb project.PrjPcb --dry-run

# Sync changes (existing PCB)
altium-cli prjpcb sync-to-pcb project.PrjPcb
altium-cli prjpcb sync-to-pcb project.PrjPcb --dry-run
```

### JSON Export

```bash
# Basic export
altium-cli prjpcb json project.PrjPcb

# Full details
altium-cli prjpcb json project.PrjPcb --full --pretty
```

## Command Details

### overview

Project overview showing documents, parameters, and summary statistics.

```bash
altium-cli prjpcb overview <PATH>
```

Output includes:
- Project name and path
- Document list with types
- Project parameters
- Component count summary
- Net count summary

### documents

List project documents with optional filtering.

```bash
altium-cli prjpcb documents <PATH> [OPTIONS]

Options:
  -t, --doc-type <TYPE>  Filter by type: schematic, pcb, library, output
```

### bom

Generate bill of materials.

```bash
altium-cli prjpcb bom <PATH> [OPTIONS]

Options:
  -g, --grouped  Group by value and footprint
```

Output includes:
- Designator
- Part number
- Value
- Footprint
- Quantity (when grouped)

### diff-sch-pcb

Show differences between schematic design and PCB layout.

```bash
altium-cli prjpcb diff-sch-pcb <PATH> [OPTIONS]

Options:
  -p, --pcb <PCB>  PCB document name (auto-detected if only one)
```

Reports:
- Components in schematic but not PCB
- Components in PCB but not schematic
- Net mismatches
- Pin connectivity differences

### validate

Validate project integrity.

```bash
altium-cli prjpcb validate <PATH> [OPTIONS]

Options:
  --check-files  Verify referenced files exist on disk
```

Checks:
- Document references valid
- No duplicate designators
- Net consistency
- Parameter completeness

### import-to-pcb

Import schematic design to PCB for initial placement.

```bash
altium-cli prjpcb import-to-pcb <PATH> [OPTIONS]

Options:
  -p, --pcb <PCB>  Target PCB document
  --dry-run        Show what would be imported without modifying files
```

### sync-to-pcb

Synchronize schematic changes to existing PCB.

```bash
altium-cli prjpcb sync-to-pcb <PATH> [OPTIONS]

Options:
  -p, --pcb <PCB>  Target PCB document
  --dry-run        Show what would change without modifying files
```

Updates:
- New components
- Removed components
- Changed parameters
- Net changes
