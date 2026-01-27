# altium-cli

Command-line tool for inspecting and manipulating Altium Designer files.

## File Index

| File/Directory | What | When |
|---|---|---|
| **Entry Point** | | |
| src/main.rs | CLI argument parsing, subcommand dispatch, global flags (--json, --pretty, --verbose) | Adding new commands, modifying CLI structure |
| **Commands** | | |
| src/commands/mod.rs | Command module re-exports | Adding new command modules |
| src/commands/inspect.rs | Inspect command: display file structure and component metadata | Viewing library/document contents |
| src/commands/query.rs | Query command: CSS-like selectors for finding components, nets, pins | Searching records with pattern matching |
| src/commands/edit.rs | Edit command: move/delete/add components, wires, junctions, ports | Programmatic schematic modification |
| **Output Formatting** | | |
| src/output.rs | TextFormat trait, print() dispatcher, print_json_as_text() fallback | Formatting CLI output, adding new output types |

## Commands

| Command | Purpose | Example |
|---|---|---|
| inspect | View file structure and component list | `altium-cli inspect components.SchLib` |
| query | Find records using selector syntax | `altium-cli query design.SchDoc "Component[Designator=R1]"` |
| edit | Modify schematic documents | `altium-cli edit design.SchDoc -c "move U1 1000 2000" -o output.SchDoc` |
| completions | Generate shell completions | `altium-cli completions bash > altium-cli.bash` |

## Query Language

**Record Selector (domain-specific):**
- `U1`, `R*`, `C??` - Component by designator (wildcards supported)
- `$LM358` - Component by part number
- `~VCC` - Net by name
- `@10K` - Component by value
- `U1:VCC` - Pin by component.pin

**SchQL (CSS-like):**
- `component[part*=7805]` - Components containing "7805" in part field
- `pin[type=input]` - Input pins
- `net:power` - Power nets

## Edit Operations

**Component operations:**
- `move <designator> <x> <y>` - Move component to coordinates (mils)
- `delete <designator>` - Delete component

**Connectivity operations:**
- `add-wire <x1>,<y1>,<x2>,<y2>,...` - Add wire path
- `delete-wire <index>` - Delete wire by index
- `add-net-label <name> <x> <y>` - Add net label
- `add-power <name> <x> <y> <style> <orientation>` - Add power port
- `add-junction <x> <y>` - Add junction at wire intersection
- `add-missing-junctions` - Auto-add junctions where wires cross

**Routing operations:**
- `route <from> <to>` - Route wire (from/to: x,y or Component.Pin)

**Validation:**
- `validate` - Validate schematic connectivity

## Output Formats

**Text (default):**
Human-readable multi-line format with labels (uses TextFormat trait).

**JSON (--json):**
Compact single-line JSON for machine parsing.

**Pretty JSON (--pretty):**
Formatted JSON with indentation (implies --json).

**Format routing:**
```
--json → serde_json::to_string()
--pretty → serde_json::to_string_pretty()
(default) → TextFormat::format_text()
```

## TextFormat Trait

CLI-specific formatting trait for human-readable output distinct from JSON serialization.

**Purpose:** Display library types with multi-line labels (not suitable for serde Serialize impls).

**Location:** Lives in altium-cli (not altium-format) to avoid CLI deps in library.

**Implementation pattern:**
```rust
impl TextFormat for ComponentInfo {
    fn format_text(&self) -> String {
        format!(
            "Designator: {}\nPart: {}\nDescription: {}",
            self.designator, self.part, self.description
        )
    }
}
```

**When to use:** Types needing labeled multi-line output (component lists, net tables).

**When NOT to use:** Types where JSON structure is sufficient (coordinates, enums).

## Architecture Boundaries

**What altium-cli does:**
- Parse command-line arguments (clap)
- Dispatch to command handlers
- Format output for humans (TextFormat) or machines (JSON)
- Handle file I/O paths and error reporting

**What altium-cli does NOT do:**
- Parse Altium file formats (delegates to altium-format)
- Query language execution (delegates to altium-format::query)
- Edit session logic (delegates to altium-format::edit)
- Record type definitions (delegates to altium-format::records)

**Dependency rule:** CLI depends on library, library NEVER depends on CLI.

## Build Commands

```bash
# Build CLI binary
cargo build -p altium-cli

# Run CLI
cargo run -p altium-cli -- --help

# Install CLI
cargo install --path crates/altium-cli

# Test CLI (no unit tests - integration via examples)
cargo build -p altium-cli && ./target/debug/altium-cli --help
```

## Error Handling

**Pattern:** Propagate errors with `?`, return `Result<(), Box<dyn std::error::Error>>`.

**Don't:** Use `.unwrap()` or silently discard errors.

**Do:** Let errors bubble to main() for display with context.

**Example:**
```rust
pub fn run(path: &Path, format: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(path)?;  // Propagates IO errors
    let lib = SchLib::open(BufReader::new(file))?;  // Propagates parse errors
    output::print(&lib.components(), format)?;  // Propagates format errors
    Ok(())
}
```

## Shell Completions

Generate completions for bash, zsh, fish, powershell:

```bash
altium-cli completions bash > altium-cli.bash
source altium-cli.bash
```

**Implementation:** Uses clap_complete to generate from Cli struct annotations.

## Global Flags

| Flag | Effect | Applies To |
|---|---|---|
| --json | Output compact JSON | All commands |
| --pretty | Output formatted JSON (implies --json) | All commands |
| --verbose | Verbose output (command-specific) | inspect |
| --quiet | Errors only (suppresses info) | All commands |

**Flag processing:** Handled in main() before command dispatch, passed as format string ("text", "json", "json-pretty").
