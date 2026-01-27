# Quick Commands

Core commands for file inspection, querying, and editing.

## inspect

View file structure and component metadata.

```bash
altium-cli inspect <PATH> [OPTIONS]

Arguments:
  <PATH>  Path to Altium file (.SchLib, .PcbLib, .SchDoc, .PcbDoc)

Options:
      --json     Output as JSON
      --pretty   Pretty-print JSON (implies --json)
  -v, --verbose  Verbose output
```

**Examples:**
```bash
altium-cli inspect components.SchLib
altium-cli inspect design.PcbDoc --json
altium-cli inspect footprints.PcbLib --pretty
altium-cli inspect design.SchDoc --verbose
```

## query

Find records using selector syntax. Supports two query languages.

```bash
altium-cli query <PATH> <SELECTOR> [OPTIONS]

Arguments:
  <PATH>      Path to Altium file
  <SELECTOR>  Selector query
```

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

**Examples:**
```bash
altium-cli query design.SchDoc "R*"
altium-cli query library.SchLib "$LM358"
altium-cli query design.SchDoc "component[part*=MCU]" --json
altium-cli query design.SchDoc "net:power"
altium-cli query design.SchDoc "U1:VCC"
```

## edit

Modify schematic documents (.SchDoc).

```bash
altium-cli edit <PATH> -c <OPERATION> [OPTIONS]

Arguments:
  <PATH>  Path to .SchDoc file

Options:
  -c, --command <OPERATION>  Edit operation
  -o, --output <OUTPUT>      Output file (defaults to input, overwrites)
```

### Component Operations

| Operation | Syntax |
|-----------|--------|
| Move component | `move <designator> <x> <y>` |
| Delete component | `delete <designator>` |

### Connectivity Operations

| Operation | Syntax |
|-----------|--------|
| Add wire | `add-wire <x1>,<y1>,<x2>,<y2>,...` |
| Delete wire | `delete-wire <index>` |
| Add net label | `add-net-label <name> <x> <y>` |
| Add power port | `add-power <name> <x> <y> <style> <orientation>` |
| Add junction | `add-junction <x> <y>` |
| Auto-add junctions | `add-missing-junctions` |
| Add port | `add-port <name> <x> <y> <io_type>` |

### Routing Operations

| Operation | Syntax |
|-----------|--------|
| Route wire | `route <from> <to>` (from/to: x,y or Component.Pin) |
| Validate | `validate` |

**Examples:**
```bash
altium-cli edit design.SchDoc -c "move U1 1000 2000" -o modified.SchDoc
altium-cli edit design.SchDoc -c "delete R3"
altium-cli edit design.SchDoc -c "add-wire 100,100,200,100,200,200"
altium-cli edit design.SchDoc -c "add-net-label VCC 1000 2000"
altium-cli edit design.SchDoc -c "add-power GND 500 500 POWER_GND 0"
altium-cli edit design.SchDoc -c "add-junction 1500 1500"
altium-cli edit design.SchDoc -c "add-missing-junctions"
altium-cli edit design.SchDoc -c "route U1.VCC 1000,500"
altium-cli edit design.SchDoc -c "validate"
```

## completions

Generate shell completions.

```bash
altium-cli completions <SHELL>

Arguments:
  <SHELL>  Shell type: bash, zsh, fish, powershell
```

**Setup:**
```bash
# Bash
altium-cli completions bash > ~/.local/share/bash-completion/completions/altium-cli

# Zsh
altium-cli completions zsh > ~/.zfunc/_altium-cli

# Fish
altium-cli completions fish > ~/.config/fish/completions/altium-cli.fish
```
