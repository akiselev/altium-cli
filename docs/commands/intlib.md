# intlib - Integrated Library Commands

Browse and extract commands for integrated libraries (.IntLib).

## Browse Commands

| Command | Purpose |
|---------|---------|
| `overview` | Library overview with component categories and statistics |
| `list` | List all components in the library |
| `search` | Search for components by name or description |
| `info` | Library info and statistics |

## Component Commands

| Command | Purpose |
|---------|---------|
| `component` | Show detailed component information |
| `crossrefs` | Show symbol/footprint cross-references |
| `parameters` | Show BOM parameters across components |

## Embedded Content Commands

| Command | Purpose |
|---------|---------|
| `symbols` | List embedded schematic symbols |
| `footprints` | List embedded PCB footprints |

## Extraction Commands

| Command | Purpose |
|---------|---------|
| `extract-schlib` | Extract schematic library |
| `extract-pcblib` | Extract PCB library |

## Export Commands

| Command | Purpose |
|---------|---------|
| `json` | Export as JSON for LLM processing |

## Examples

### Browsing

```bash
# Library overview
altium-cli intlib overview library.IntLib
altium-cli intlib overview library.IntLib --full

# List all components
altium-cli intlib list library.IntLib

# Search components
altium-cli intlib search library.IntLib "LM358"
altium-cli intlib search library.IntLib "voltage regulator" --limit 10

# Library info
altium-cli intlib info library.IntLib
```

### Component Inspection

```bash
# Component details
altium-cli intlib component library.IntLib LM358
altium-cli intlib component library.IntLib LM358 --params

# Cross-references
altium-cli intlib crossrefs library.IntLib
altium-cli intlib crossrefs library.IntLib --footprint SOIC-8
```

### Embedded Content

```bash
# List symbols
altium-cli intlib symbols library.IntLib

# List footprints
altium-cli intlib footprints library.IntLib
```

### Parameters

```bash
# All parameters
altium-cli intlib parameters library.IntLib

# Parameters for specific component
altium-cli intlib parameters library.IntLib -c LM358

# Filter by parameter keys
altium-cli intlib parameters library.IntLib --keys "Manufacturer,Part Number,Value"
```

### Extraction

```bash
# Extract schematic library
altium-cli intlib extract-schlib library.IntLib -o symbols.SchLib

# Extract PCB library
altium-cli intlib extract-pcblib library.IntLib -o footprints.PcbLib
```

### JSON Export

```bash
# Export as JSON
altium-cli intlib json library.IntLib
altium-cli intlib json library.IntLib --pretty
```

## Command Details

### overview

Library overview with component categories and statistics.

```bash
altium-cli intlib overview <PATH> [OPTIONS]

Options:
  --full  Include full component details
```

Output includes:
- Library name and path
- Total component count
- Component categories
- Symbol count
- Footprint count

### search

Search for components by name or description.

```bash
altium-cli intlib search <PATH> <QUERY> [OPTIONS]

Arguments:
  <QUERY>  Search query

Options:
  -l, --limit <LIMIT>  Maximum results to return
```

Searches:
- Component name
- Description
- Part number
- Parameters

### component

Show detailed component information.

```bash
altium-cli intlib component <PATH> <NAME> [OPTIONS]

Arguments:
  <NAME>  Component name or index

Options:
  --params  Show all parameters
```

Output includes:
- Component name
- Description
- Symbol reference
- Footprint reference
- Parameters (with --params)

### crossrefs

Show symbol/footprint cross-references.

```bash
altium-cli intlib crossrefs <PATH> [OPTIONS]

Options:
  -f, --footprint <NAME>  Filter by footprint name
```

Shows which components use which symbols and footprints.

### parameters

Show BOM parameters across components.

```bash
altium-cli intlib parameters <PATH> [OPTIONS]

Options:
  -c, --component <NAME>  Filter by component name
  -k, --keys <KEYS>       Filter by parameter keys (comma-separated)
```

Common parameters:
- Manufacturer
- Part Number
- Value
- Tolerance
- Voltage Rating
- Package

### extract-schlib

Extract embedded schematic symbols to standalone library.

```bash
altium-cli intlib extract-schlib <PATH> -o <OUTPUT>

Options:
  -o, --output <PATH>  Output .SchLib file path (required)
```

### extract-pcblib

Extract embedded PCB footprints to standalone library.

```bash
altium-cli intlib extract-pcblib <PATH> -o <OUTPUT>

Options:
  -o, --output <PATH>  Output .PcbLib file path (required)
```

## Integrated Library Structure

IntLib files contain:
1. **Component database** - Component metadata and parameters
2. **Schematic symbols** - Embedded .SchLib content
3. **PCB footprints** - Embedded .PcbLib content
4. **Cross-reference table** - Maps components to symbols/footprints

The extraction commands let you recover the embedded libraries for editing or use in other projects.
