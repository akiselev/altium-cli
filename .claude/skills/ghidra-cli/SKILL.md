---
name: ghidra-cli
description: >
    Use ghidra-cli for reverse engineering tasks: binary analysis, decompilation, function inspection, cross-reference analysis, pattern discovery, and binary patching.
    Activate when the user requests:
    - Binary analysis or reverse engineering
    - Decompilation or disassembly
    - Function listing, inspection, or renaming
    - Cross-reference or call graph analysis
    - String or byte pattern searches
    - Binary patching or modification
    - Ghidra project management
---

# ghidra-cli Agent Reference

Rust CLI for Ghidra reverse engineering. Binary name: `ghidra`.

## Architecture

```
CLI (Rust/clap) ──TCP──► GhidraCliBridge.java (GhidraScript in Ghidra JVM)
```

- **Direct bridge**: no daemon process. The Java bridge IS the persistent server.
- One bridge per project, keyed by `~/.local/share/ghidra-cli/bridge-{md5}.port`
- Import/Analyze/query commands **auto-start** the bridge if not running
- Sequential command processing (Ghidra API is not thread-safe)

## Global Flags

| Flag | Effect |
|------|--------|
| `--json` | Compact JSON output (single line) |
| `--pretty` | Pretty-printed JSON |
| `-v` / `-vv` / `-vvv` | Log verbosity: warn / info / debug |
| `-q` / `--quiet` | Suppress non-essential stderr |

**Format auto-detection**: TTY → compact human-readable; pipe → json-compact. Override with `--json`, `--pretty`, or `-o FORMAT`.

## Quick Start

```bash
# Fastest path: import + analyze, bridge starts automatically
ghidra import ./binary --project myproject
ghidra analyze --project myproject --program mybinary

# All subsequent queries reuse the running bridge
ghidra function list --project myproject
ghidra decompile main --project myproject
```

## Command Reference

### Bridge Lifecycle

```bash
ghidra start [--project P] [--program PROG]
ghidra stop [--project P]
ghidra restart [--project P] [--program PROG]
ghidra status [--project P]
ghidra ping [--project P]
```

### Project Management

```bash
ghidra project create NAME
ghidra project list
ghidra project info [NAME]
ghidra project delete NAME
```

### Import & Analysis

```bash
ghidra import BINARY [--project P] [--program PROG] [--detach]
ghidra analyze [--project P] [--program PROG] [--detach]
```

Both auto-start bridge. `--detach` returns immediately.

### Program Management

```bash
ghidra program list [--project P]          # alias: prog, programs
ghidra program open --program PROG [--project P]
ghidra program close [--project P]
ghidra program delete --program PROG [--project P]
ghidra program info [--project P]
ghidra program export FORMAT [--project P] [-o OUTPUT]   # FORMAT: xml, json, asm, c
```

### Function Operations

```bash
ghidra function list [QUERY_OPTS]           # aliases: fn, func, functions
ghidra function get TARGET [QUERY_OPTS]     # TARGET = name or 0xADDRESS
ghidra function decompile TARGET [QUERY_OPTS]
ghidra function disasm TARGET [QUERY_OPTS]
ghidra function calls TARGET [QUERY_OPTS]   # outgoing calls
ghidra function xrefs TARGET [QUERY_OPTS]   # incoming references
ghidra function rename OLD NEW [--project P] [--program PROG]
ghidra function create ADDRESS [NAME] [--project P] [--program PROG]
ghidra function delete TARGET [QUERY_OPTS]
```

### Top-level Shortcuts

```bash
ghidra decompile TARGET [QUERY_OPTS]        # aliases: decomp, dec
ghidra disasm ADDRESS [-n COUNT] [QUERY_OPTS]  # aliases: disassemble, dis
```

### String Operations

```bash
ghidra strings list [QUERY_OPTS]            # aliases: string, str
ghidra strings refs STRING [QUERY_OPTS]     # xrefs to string
```

### Symbol Operations

```bash
ghidra symbol list [QUERY_OPTS]             # aliases: sym, symbols
ghidra symbol get NAME [QUERY_OPTS]
ghidra symbol create ADDRESS NAME [--project P] [--program PROG]
ghidra symbol delete NAME [QUERY_OPTS]
ghidra symbol rename OLD NEW [--project P] [--program PROG]
```

### Memory Operations

```bash
ghidra memory map [QUERY_OPTS]              # alias: mem
ghidra memory read ADDRESS SIZE [QUERY_OPTS]
ghidra memory write ADDRESS BYTES [--project P] [--program PROG]
ghidra memory search PATTERN [QUERY_OPTS]
```

### Cross-References

```bash
ghidra x-ref to ADDRESS [QUERY_OPTS]        # aliases: xref, xrefs, crossref
ghidra x-ref from ADDRESS [QUERY_OPTS]
ghidra x-ref list [QUERY_OPTS]
```

### Type Operations

```bash
ghidra type list [QUERY_OPTS]               # alias: types
ghidra type get NAME [QUERY_OPTS]
ghidra type create DEFINITION [--project P] [--program PROG]
ghidra type apply ADDRESS TYPE_NAME [--project P] [--program PROG]
```

### Comment Operations

```bash
ghidra comment list [QUERY_OPTS]            # alias: comments
ghidra comment get ADDRESS [QUERY_OPTS]
ghidra comment set ADDRESS TEXT [--comment-type TYPE] [--project P] [--program PROG]
ghidra comment delete ADDRESS [QUERY_OPTS]
```

### Search / Find

```bash
ghidra find string PATTERN [QUERY_OPTS]     # alias: search
ghidra find bytes HEX [QUERY_OPTS]
ghidra find function PATTERN [QUERY_OPTS]   # glob patterns
ghidra find calls FUNCTION [QUERY_OPTS]
ghidra find crypto [QUERY_OPTS]             # detect AES/SHA/RSA constants
ghidra find interesting [QUERY_OPTS]        # suspicious patterns
```

### Graph / Call Graph

```bash
ghidra graph calls [QUERY_OPTS]             # aliases: callgraph, cg
ghidra graph callers FUNCTION [--depth N] [QUERY_OPTS]
ghidra graph callees FUNCTION [--depth N] [QUERY_OPTS]
ghidra graph export FORMAT [QUERY_OPTS]     # FORMAT: dot, json
```

### Diff

```bash
ghidra diff programs PROG1 PROG2 [--project P] [--format F]
ghidra diff functions FUNC1 FUNC2 [--project P] [--format F]
```

### Dump / Export

```bash
ghidra dump imports [QUERY_OPTS]            # alias: export
ghidra dump exports [QUERY_OPTS]
ghidra dump functions [QUERY_OPTS]
ghidra dump strings [QUERY_OPTS]
```

### Patch

```bash
ghidra patch bytes ADDRESS HEX [--project P] [--program PROG]
ghidra patch nop ADDRESS [--count N] [--project P] [--program PROG]
ghidra patch export -o OUTPUT [--project P] [--program PROG]
```

### Script Execution

```bash
ghidra script run PATH [--project P] [--program PROG] [-- ARGS...]
ghidra script python CODE [--project P] [--program PROG]
ghidra script java CODE [--project P] [--program PROG]
ghidra script list
```

### Batch

```bash
ghidra batch SCRIPT_FILE [--project P] [--program PROG]
```

Batch file: one subcommand per line (without `ghidra` prefix), `#` comments.

### Universal Query

```bash
ghidra query DATA_TYPE [QUERY_OPTS]
```

DATA_TYPE: `functions`, `strings`, `imports`, `exports`, `memory`.

### Statistics & Info

```bash
ghidra summary [QUERY_OPTS]       # alias: info
ghidra stats [QUERY_OPTS]
```

### Configuration

```bash
ghidra init                       # create config
ghidra doctor                     # check installation
ghidra version
ghidra config list
ghidra config get KEY
ghidra config set KEY VALUE       # keys: ghidra_install_dir, ghidra_project_dir, default_program, default_project, default_output_format, timeout, default_limit
ghidra config reset
ghidra set-default KIND VALUE     # KIND: program, project
ghidra setup [--version V] [--dir D] [--force]
```

## Common Query Options (QUERY_OPTS)

All query commands accept these:

| Option | Description |
|--------|-------------|
| `--project P` | Project name or path (env: `GHIDRA_DEFAULT_PROJECT`) |
| `--program PROG` | Program within project (env: `GHIDRA_DEFAULT_PROGRAM`) |
| `--filter EXPR` | Filter expression |
| `--fields LIST` | Comma-separated fields to return |
| `-o FORMAT` | Output format |
| `--limit N` | Max results |
| `--offset N` | Skip first N |
| `--sort FIELDS` | Sort: comma-separated, prefix `-` for descending |
| `--count` | Return count only |
| `--json` | Shorthand for `--format=json` |

## Output Formats

| Value | Use |
|-------|-----|
| `compact` | Default for TTY. One line per item. |
| `full` | Multi-line labeled blocks |
| `json` | Pretty JSON |
| `json-compact` | Default for pipes. Single-line JSON. |
| `json-stream` / `ndjson` | One JSON object per line |
| `csv` / `tsv` | Delimited with header |
| `table` | ASCII box-drawn table |
| `count` | Number only |
| `ids` / `minimal` | Address/name only, one per line |
| `tree` | Indented hierarchy |
| `hex` | Hex dump |
| `asm` | Assembly |
| `c` | C pseudocode |

## Filter Expressions

```bash
# Numeric
--filter "size > 100"
--filter "size >= 50"

# String
--filter "name contains 'crypt'"

# Combined
--filter "size > 100 and name contains 'main'"
--filter "name != 'main'"
```

Operators: `>`, `<`, `>=`, `<=`, `=`, `!=`, `contains`, `in`, `and`, `or`.

## Agent Best Practices

### 1. Count-First Pattern

Always check result volume before fetching:

```bash
ghidra function list --count --project P
# If manageable:
ghidra function list --limit 50 --fields name,address,size --project P
```

### 2. Aggressive Filtering

Pre-filter server-side, not client-side:

```bash
# GOOD
ghidra function list --filter "size > 1000" --project P
# BAD
ghidra function list --project P  # then filter in agent code
```

### 3. Field Selection

Request only needed fields:

```bash
ghidra function list --fields name,address --json --project P
```

### 4. Set Defaults

Avoid repeating `--project` and `--program`:

```bash
ghidra set-default project myproject
ghidra set-default program mybinary
# Now: ghidra function list  (no flags needed)
```

## .NET Warning

ghidra decompile emits a warning for .NET IL bytecode:
> "This appears to be .NET managed code. Consider using ilspy-cli."

Use `ilspy detect` to classify binaries before decompiling.

## Analysis Workflow

```bash
# 1. Import and analyze
ghidra import ./target.exe --project analysis
ghidra analyze --project analysis

# 2. Recon
ghidra summary --project analysis
ghidra function list --count --project analysis
ghidra function list --filter "NOT name contains 'FUN_'" --fields name,address,size --limit 30 --project analysis

# 3. Investigate
ghidra decompile main --project analysis
ghidra find crypto --project analysis
ghidra find string "password" --project analysis

# 4. Deep dive
ghidra graph callers suspicious_func --depth 3 --project analysis
ghidra x-ref to 0x401000 --project analysis
ghidra function disasm 0x401000 --project analysis

# 5. Patch
ghidra patch nop 0x401234 --count 3 --project analysis
ghidra patch export -o patched.exe --project analysis
```

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `GHIDRA_INSTALL_DIR` | Ghidra installation path |
| `GHIDRA_DEFAULT_PROJECT` | Default `--project` value |
| `GHIDRA_DEFAULT_PROGRAM` | Default `--program` value |

## File Locations

| File | Purpose |
|------|---------|
| `~/.local/share/ghidra-cli/bridge-{md5}.port` | TCP port for running bridge |
| `~/.local/share/ghidra-cli/bridge-{md5}.pid` | Bridge process PID |
| `~/.local/share/ghidra-cli/config.yaml` | Configuration |
| `~/.local/share/ghidra-cli/ghidra-cli.log` | Debug log |

## Error Recovery

| Problem | Fix |
|---------|-----|
| "No project specified" | Add `--project NAME` or `ghidra set-default project NAME` |
| "Bridge not responding" | `ghidra stop --project P` then retry (auto-starts) |
| "Ghidra installation not configured" | `ghidra setup` or set `GHIDRA_INSTALL_DIR` |
| Function not found | Use `ghidra find function "*pattern*"` |
| Slow first command | Normal: bridge startup + analysis takes seconds |
