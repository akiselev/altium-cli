# commands/ - CLI Command Implementations

Thin CLI wrappers around `altium_format::ops` functions. Handles argument parsing (clap) and output formatting.

## Module Index

| Module | File Type | Description |
|--------|-----------|-------------|
| `schlib.rs` | `.SchLib` | Schematic library commands: browse, search, create, edit |
| `schdoc.rs` | `.SchDoc` | Schematic document commands: overview, BOM, netlist, analysis |
| `pcblib.rs` | `.PcbLib` | PCB library commands: browse, measure, create, edit |
| `pcbdoc.rs` | `.PcbDoc` | PCB document commands: rules, components, routing |
| `prjpcb.rs` | `.PrjPcb` | Project commands: overview, BOM, validation |
| `intlib.rs` | `.IntLib` | Integrated library commands: browse, extract |

## Pattern

Commands are thin wrappers that:
1. Parse CLI arguments using clap
2. Call corresponding `ops::*::cmd_*` function
3. Format output (table, JSON, or custom)

All business logic lives in `altium_format::ops`. Commands should not contain domain logic.

## Adding New Commands

1. Add subcommand enum variant to the file-type module
2. Implement handler that calls `ops::*::cmd_*`
3. Add subcommand to `main.rs` Commands enum
4. Update SKILL.md with command reference
