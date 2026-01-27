# altium-cli Documentation

Command-line tool for reading, writing, and querying Altium Designer files.

## Command Reference

| File | Commands |
|------|----------|
| [commands/quick.md](commands/quick.md) | `inspect`, `query`, `edit`, `completions` |
| [commands/schdoc.md](commands/schdoc.md) | Schematic document analysis |
| [commands/schlib.md](commands/schlib.md) | Schematic library browse/create/edit |
| [commands/pcbdoc.md](commands/pcbdoc.md) | PCB document analysis/edit |
| [commands/pcblib.md](commands/pcblib.md) | PCB library browse/create/edit |
| [commands/prjpcb.md](commands/prjpcb.md) | PCB project management |
| [commands/intlib.md](commands/intlib.md) | Integrated library access |

## Quick Reference

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

## Global Flags

| Flag | Effect |
|------|--------|
| `--json` | Output compact JSON |
| `--pretty` | Output formatted JSON (implies --json) |
| `-v, --verbose` | Verbose output |
| `-q, --quiet` | Errors only |

## Supported File Types

| Extension | Type | Read | Write | Query |
|-----------|------|------|-------|-------|
| `.SchLib` | Schematic Library | Yes | Yes | Yes |
| `.SchDoc` | Schematic Document | Yes | Yes | Yes |
| `.PcbLib` | PCB Library | Yes | Yes | Yes |
| `.PcbDoc` | PCB Document | Yes | Yes | Yes |
| `.PrjPcb` | PCB Project | Yes | Yes | - |
| `.IntLib` | Integrated Library | Yes | - | Yes |
