# intlib - Integrated Library Commands

Browse and extract commands for integrated libraries (.IntLib).

## Browse Commands

| Command | Purpose |
|---------|---------|
| `overview` | Library overview with component counts and statistics |
| `list` | List all components |
| `search` | Search for components by name or description |
| `info` | Library metadata and statistics |

## Component Commands

| Command | Purpose |
|---------|---------|
| `component` | Show detailed component information |
| `parameters` | Show BOM parameters |

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
```

### Embedded Content

```bash
# List embedded symbols
altium-cli intlib symbols library.IntLib

# List embedded footprints
altium-cli intlib footprints library.IntLib
```

### Parameters

```bash
# All component parameters
altium-cli intlib parameters library.IntLib

# Parameters for specific component
altium-cli intlib parameters library.IntLib -c LM358
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
# Basic export
altium-cli intlib json library.IntLib

# Full details (includes symbol and footprint info)
altium-cli intlib json library.IntLib --full
```

## Command Details

### overview

Library overview with component counts and statistics.

```bash
altium-cli intlib overview <PATH>
```

Output includes:
- Library path and version
- Total component count
- Schematic symbol count
- PCB footprint count
- Footprint usage statistics

### search

Search for components by name or description.

```bash
altium-cli intlib search <PATH> <QUERY> [OPTIONS]

Arguments:
  <QUERY>  Search query (supports partial matching)

Options:
  -l, --limit <LIMIT>  Maximum results to return
```

### component

Show detailed component information.

```bash
altium-cli intlib component <PATH> <NAME>

Arguments:
  <NAME>  Component name
```

Output includes:
- Component name and description
- Linked footprint
- Symbol and footprint paths
- Pin count and primitive count

### symbols

List embedded schematic symbols.

```bash
altium-cli intlib symbols <PATH>
```

Output includes:
- Symbol name
- Description
- Pin count

### footprints

List embedded PCB footprints.

```bash
altium-cli intlib footprints <PATH>
```

Output includes:
- Footprint name
- Description
- Pad count

### parameters

Show BOM parameters across components.

```bash
altium-cli intlib parameters <PATH> [OPTIONS]

Options:
  -c, --component <NAME>  Filter by component name
```

Output includes parameter key-value pairs for each component.

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

### json

Export library as JSON for processing.

```bash
altium-cli intlib json <PATH> [OPTIONS]

Options:
  --full  Include detailed symbol and footprint information
```

## Integrated Library Structure

IntLib files contain:
1. **Component database** - Component metadata and cross-references
2. **Schematic symbols** - Embedded .SchLib content
3. **PCB footprints** - Embedded .PcbLib content
4. **Parameter sets** - BOM parameters for each component

The extraction commands let you recover the embedded libraries for editing or use in other projects.
