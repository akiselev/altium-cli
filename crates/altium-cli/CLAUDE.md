# altium-cli

Command-line tool for inspecting and manipulating Altium Designer files.

## File Index

| File/Directory | What | When |
|---|---|---|
| **Entry Point** | | |
| src/main.rs | CLI argument parsing, subcommand dispatch, global flags (--json, --pretty, --verbose) | Adding new commands, modifying CLI structure |
| **Commands** | | |
| src/commands/mod.rs | Command module re-exports | Adding new command modules |
| src/commands/inspect.rs | Inspect command: quick file structure view | Viewing library/document contents |
| src/commands/query.rs | Query command: CSS-like selectors for finding components, nets, pins | Searching records with pattern matching |
| src/commands/edit.rs | Edit command: move/delete/add components, wires, junctions, ports | Programmatic schematic modification |
| src/commands/schdoc.rs | SchDoc commands: BOM, netlist, power analysis, components, etc. | Analyzing schematic documents |
| src/commands/schlib.rs | SchLib commands: browse, search, component details | Exploring schematic libraries |
| src/commands/pcbdoc.rs | PcbDoc commands: rules, layers, components, tracks | Analyzing PCB documents |
| src/commands/pcblib.rs | PcbLib commands: browse, measure, footprint details, create, edit | Exploring and creating PCB libraries |
| src/commands/prjpcb.rs | PrjPcb commands: project management, BOM, netlist, sync | Managing PCB projects |
| src/commands/intlib.rs | IntLib commands: integrated library browsing and extraction | Exploring integrated libraries |
| **Output Formatting** | | |
| src/output.rs | TextFormat trait, print() dispatcher, print_json_as_text() fallback | Formatting CLI output, adding new output types |

## V2 Type Usage

CLI commands use V2 types from altium-format:

| V2 Type | CLI Usage |
|---|---|
| `SchLibV2` | `schlib` commands load libraries via `v2::io::schlib::SchLibV2::open_file()` |
| `SchDocV2` | `schdoc` commands load documents via `v2::io::schdoc::SchDocV2::open_file()` |
| `PcbLibV2` | `pcblib` commands load libraries via `v2::pcb::io::pcblib::PcbLibV2::open_file()` |
| `PcbDocV2` | `pcbdoc` commands load documents via `v2::pcb::io::pcbdoc::PcbDocV2::open_file()` |
| `PinData` | Pin listing, electrical type display |
| `ComponentData` | Component metadata, parameters |
| `TypedRecord` | Record enumeration, hierarchy traversal |

## Commands

### Quick Commands

| Command | Purpose | Example |
|---|---|---|
| inspect | Quick file structure view | `altium-cli inspect components.SchLib` |
| query | Find records using selector syntax | `altium-cli query design.SchDoc "R*"` |
| edit | Modify schematic documents | `altium-cli edit design.SchDoc -c "move U1 1000 2000"` |
| completions | Generate shell completions | `altium-cli completions bash` |

### Schematic Document Commands (schdoc)

| Command | Purpose | Example |
|---|---|---|
| overview | Complete design overview with categories, power, interfaces | `altium-cli schdoc overview design.SchDoc` |
| bom | Bill of materials grouped by component | `altium-cli schdoc bom design.SchDoc` |
| netlist | Net connectivity map | `altium-cli schdoc netlist design.SchDoc` |
| power-map | Power distribution analysis | `altium-cli schdoc power-map design.SchDoc` |
| blocks | Block diagram of major ICs | `altium-cli schdoc blocks design.SchDoc` |
| signal-flow | Signal flow tracing | `altium-cli schdoc signal-flow design.SchDoc CLK` |
| project | Multi-file hierarchical analysis | `altium-cli schdoc project sheet1.SchDoc sheet2.SchDoc` |
| info | Document info and sheet metadata | `altium-cli schdoc info design.SchDoc` |
| stats | Detailed record statistics | `altium-cli schdoc stats design.SchDoc` |
| components | List all components | `altium-cli schdoc components design.SchDoc` |
| component | Show component details | `altium-cli schdoc component design.SchDoc U1` |
| wires | List all wires | `altium-cli schdoc wires design.SchDoc` |
| nets | List all net labels | `altium-cli schdoc nets design.SchDoc --group` |
| ports | List all ports | `altium-cli schdoc ports design.SchDoc` |
| power | List all power objects | `altium-cli schdoc power design.SchDoc --group` |
| pins | List pins (filtered by component) | `altium-cli schdoc pins design.SchDoc -c U1` |
| junctions | List all junctions | `altium-cli schdoc junctions design.SchDoc` |
| hierarchy | Show record hierarchy tree | `altium-cli schdoc hierarchy design.SchDoc` |
| json | Export as JSON for LLM processing | `altium-cli schdoc json design.SchDoc --full` |

### Schematic Library Commands (schlib)

| Command | Purpose | Example |
|---|---|---|
| overview | Library overview with categories | `altium-cli schlib overview components.SchLib` |
| list | List all components | `altium-cli schlib list components.SchLib` |
| search | Search by name or description | `altium-cli schlib search components.SchLib "LM358"` |
| info | Library info and statistics | `altium-cli schlib info components.SchLib` |
| component | Show component details | `altium-cli schlib component components.SchLib LM358` |
| pins | List pins for component | `altium-cli schlib pins components.SchLib -c LM358` |
| primitives | List component primitives | `altium-cli schlib primitives components.SchLib LM358` |
| json | Export as JSON | `altium-cli schlib json components.SchLib` |
| render-ascii | Render component as ASCII art | `altium-cli schlib render-ascii components.SchLib LM358` |
| create | Create new schematic library | `altium-cli schlib create new.SchLib` |
| add-component | Add component to library | `altium-cli schlib add-component lib.SchLib MyPart` |
| add-pin | Add pin to component | `altium-cli schlib add-pin lib.SchLib MyPart -n VCC -x 0 -y 100` |
| add-rectangle | Add rectangle primitive | `altium-cli schlib add-rectangle lib.SchLib MyPart 0 0 100 200` |
| add-line | Add line primitive | `altium-cli schlib add-line lib.SchLib MyPart 0 0 100 100` |
| add-polygon | Add polygon primitive | `altium-cli schlib add-polygon lib.SchLib MyPart "0,0;100,0;50,100"` |
| gen-ic | Generate IC symbol from pinout | `altium-cli schlib gen-ic lib.SchLib IC1 --pins "VCC,IN,OUT,GND"` |
| add-json | Add component from JSON definition | `altium-cli schlib add-json lib.SchLib component.json` |

### PCB Document Commands (pcbdoc)

**Analysis Commands:**

| Command | Purpose | Example |
|---|---|---|
| overview | Document overview with components, nets, rules | `altium-cli pcbdoc overview design.PcbDoc` |
| info | Document info and statistics | `altium-cli pcbdoc info design.PcbDoc` |
| rules | List all design rules | `altium-cli pcbdoc rules design.PcbDoc --kind clearance` |
| rule | Show specific rule details | `altium-cli pcbdoc rule design.PcbDoc "Clearance_1"` |
| components | List all components | `altium-cli pcbdoc components design.PcbDoc --layer top` |
| component | Show component details | `altium-cli pcbdoc component design.PcbDoc U1` |
| nets | List all nets | `altium-cli pcbdoc nets design.PcbDoc` |
| layers | List board layers | `altium-cli pcbdoc layers design.PcbDoc` |
| tracks | List all tracks | `altium-cli pcbdoc tracks design.PcbDoc --net VCC` |
| vias | List all vias | `altium-cli pcbdoc vias design.PcbDoc` |
| arcs | List all arcs | `altium-cli pcbdoc arcs design.PcbDoc` |
| fills | List all fills | `altium-cli pcbdoc fills design.PcbDoc` |
| texts | List all text objects | `altium-cli pcbdoc texts design.PcbDoc` |
| regions | List all regions | `altium-cli pcbdoc regions design.PcbDoc` |
| keepouts | List all keepout areas | `altium-cli pcbdoc keepouts design.PcbDoc` |
| cutouts | List all board cutouts | `altium-cli pcbdoc cutouts design.PcbDoc` |
| polygons | List all polygon pours | `altium-cli pcbdoc polygons design.PcbDoc` |
| polygon | Show specific polygon details | `altium-cli pcbdoc polygon design.PcbDoc GND_POUR` |
| outline | Show board outline | `altium-cli pcbdoc outline design.PcbDoc` |
| settings | Show board settings | `altium-cli pcbdoc settings design.PcbDoc` |
| json | Export as JSON | `altium-cli pcbdoc json design.PcbDoc --full` |

**Editing Commands:**

| Command | Purpose | Example |
|---|---|---|
| create | Create new PCB document | `altium-cli pcbdoc create new.PcbDoc` |
| set-outline-rect | Set rectangular board outline | `altium-cli pcbdoc set-outline-rect design.PcbDoc 0 0 4000 3000` |
| set-outline | Set board outline from vertices | `altium-cli pcbdoc set-outline design.PcbDoc "0,0;4000,0;4000,3000;0,3000"` |
| set-settings | Modify board settings | `altium-cli pcbdoc set-settings design.PcbDoc --units mm` |
| add-keepout | Add keepout area | `altium-cli pcbdoc add-keepout design.PcbDoc "0,0;100,0;100,100;0,100"` |
| add-cutout | Add board cutout | `altium-cli pcbdoc add-cutout design.PcbDoc 500 500 100 100` |
| add-polygon | Add polygon pour | `altium-cli pcbdoc add-polygon design.PcbDoc GND --layer top` |
| add-track | Add single track segment | `altium-cli pcbdoc add-track design.PcbDoc 0 0 100 100 --net VCC` |
| add-track-path | Add multi-segment track path | `altium-cli pcbdoc add-track-path design.PcbDoc "0,0;100,0;100,100"` |
| add-via | Add via | `altium-cli pcbdoc add-via design.PcbDoc 500 500 --net VCC` |
| add-arc | Add arc | `altium-cli pcbdoc add-arc design.PcbDoc 500 500 100 0 90` |
| add-fill | Add fill region | `altium-cli pcbdoc add-fill design.PcbDoc "0,0;100,0;100,100;0,100"` |
| add-text | Add text object | `altium-cli pcbdoc add-text design.PcbDoc "REV A" 100 100` |
| add-region | Add region | `altium-cli pcbdoc add-region design.PcbDoc "0,0;100,0;100,100;0,100"` |
| place-component | Place component on board | `altium-cli pcbdoc place-component design.PcbDoc U1 1000 2000` |
| add-component | Add new component | `altium-cli pcbdoc add-component design.PcbDoc U1 SOIC-8` |
| add-net | Add net definition | `altium-cli pcbdoc add-net design.PcbDoc VCC` |
| add-rule | Add design rule | `altium-cli pcbdoc add-rule design.PcbDoc clearance --value 10` |
| modify-rule | Modify existing rule | `altium-cli pcbdoc modify-rule design.PcbDoc Clearance_1 --value 15` |
| delete-rule | Delete design rule | `altium-cli pcbdoc delete-rule design.PcbDoc Clearance_1` |

### PCB Footprint Library Commands (pcblib)

**Analysis Commands:**

| Command | Purpose | Example |
|---|---|---|
| overview | Library overview | `altium-cli pcblib overview footprints.PcbLib` |
| list | List all footprints | `altium-cli pcblib list footprints.PcbLib` |
| search | Search footprints | `altium-cli pcblib search footprints.PcbLib "SOIC"` |
| info | Library info and statistics | `altium-cli pcblib info footprints.PcbLib` |
| footprint | Show footprint details | `altium-cli pcblib footprint footprints.PcbLib SOIC-8` |
| pads | List pads | `altium-cli pcblib pads footprints.PcbLib -f SOIC-8` |
| primitives | List footprint primitives | `altium-cli pcblib primitives footprints.PcbLib SOIC-8` |
| holes | Analyze hole sizes | `altium-cli pcblib holes footprints.PcbLib` |
| measure | Measure dimensions and clearances | `altium-cli pcblib measure footprints.PcbLib SOIC-8` |
| json | Export as JSON | `altium-cli pcblib json footprints.PcbLib --full` |
| render-ascii | Render footprint as ASCII art | `altium-cli pcblib render-ascii footprints.PcbLib SOIC-8` |
| render-svg | Render footprint as SVG | `altium-cli pcblib render-svg footprints.PcbLib SOIC-8 -o out.svg` |
| render-png | Render footprint as PNG | `altium-cli pcblib render-png footprints.PcbLib SOIC-8 -o out.png` |

**Editing Commands:**

| Command | Purpose | Example |
|---|---|---|
| create | Create new PCB library | `altium-cli pcblib create new.PcbLib` |
| add-footprint | Add footprint to library | `altium-cli pcblib add-footprint lib.PcbLib SOIC-8` |
| add-pad | Add pad to footprint | `altium-cli pcblib add-pad lib.PcbLib SOIC-8 1 0 0` |
| add-silkscreen | Add silkscreen outline | `altium-cli pcblib add-silkscreen lib.PcbLib SOIC-8 "0,0;100,0;100,200;0,200"` |
| add-arc | Add arc to footprint | `altium-cli pcblib add-arc lib.PcbLib SOIC-8 50 100 25 0 180` |
| gen-chip | Generate chip footprint (0402, 0603, etc.) | `altium-cli pcblib gen-chip lib.PcbLib 0603` |
| add-json | Add footprint from JSON definition | `altium-cli pcblib add-json lib.PcbLib footprint.json` |
| add-pad-row | Add row of pads | `altium-cli pcblib add-pad-row lib.PcbLib FP 8 --pitch 50 --start 1` |
| add-dual-row | Add dual-row pads (SOIC, DIP) | `altium-cli pcblib add-dual-row lib.PcbLib FP 8 --pitch 50 --span 300` |
| add-quad-pads | Add quad pads (QFP, QFN) | `altium-cli pcblib add-quad-pads lib.PcbLib FP 32 --pitch 50` |
| add-pad-grid | Add grid of pads (BGA) | `altium-cli pcblib add-pad-grid lib.PcbLib FP 8 8 --pitch 80` |

### PCB Project Commands (prjpcb)

**Analysis Commands:**

| Command | Purpose | Example |
|---|---|---|
| overview | Project overview with documents and parameters | `altium-cli prjpcb overview project.PrjPcb` |
| info | Project info and metadata | `altium-cli prjpcb info project.PrjPcb` |
| documents | List project documents | `altium-cli prjpcb documents project.PrjPcb --doc-type schematic` |
| parameters | Show project parameters | `altium-cli prjpcb parameters project.PrjPcb` |
| netlist | Show project netlist | `altium-cli prjpcb netlist project.PrjPcb` |
| components | List all components in project | `altium-cli prjpcb components project.PrjPcb` |
| bom | Generate BOM for project | `altium-cli prjpcb bom project.PrjPcb --grouped` |
| diff-sch-pcb | Show differences between schematic and PCB | `altium-cli prjpcb diff-sch-pcb project.PrjPcb` |
| validate | Validate project | `altium-cli prjpcb validate project.PrjPcb --check-files` |
| json | Export as JSON for LLM processing | `altium-cli prjpcb json project.PrjPcb --full` |

**Editing Commands:**

| Command | Purpose | Example |
|---|---|---|
| create | Create new project | `altium-cli prjpcb create new.PrjPcb --name "My Project"` |
| add-document | Add document to project | `altium-cli prjpcb add-document project.PrjPcb sheet.SchDoc` |
| remove-document | Remove document from project | `altium-cli prjpcb remove-document project.PrjPcb sheet.SchDoc` |
| set-parameter | Set project parameter | `altium-cli prjpcb set-parameter project.PrjPcb Revision "1.0"` |
| remove-parameter | Remove project parameter | `altium-cli prjpcb remove-parameter project.PrjPcb Revision` |
| import-to-pcb | Import design to PCB | `altium-cli prjpcb import-to-pcb project.PrjPcb --dry-run` |
| sync-to-pcb | Sync schematic changes to PCB | `altium-cli prjpcb sync-to-pcb project.PrjPcb --dry-run` |

### Integrated Library Commands (intlib)

| Command | Purpose | Example |
|---|---|---|
| overview | Library overview with component categories | `altium-cli intlib overview library.IntLib --full` |
| list | List all components | `altium-cli intlib list library.IntLib` |
| search | Search for components | `altium-cli intlib search library.IntLib "LM358" --limit 10` |
| info | Library info and statistics | `altium-cli intlib info library.IntLib` |
| component | Show component details | `altium-cli intlib component library.IntLib LM358 --params` |
| crossrefs | Show symbol/footprint cross-references | `altium-cli intlib crossrefs library.IntLib --footprint SOIC-8` |
| symbols | List embedded schematic symbols | `altium-cli intlib symbols library.IntLib` |
| footprints | List embedded PCB footprints | `altium-cli intlib footprints library.IntLib` |
| parameters | Show BOM parameters across components | `altium-cli intlib parameters library.IntLib -c LM358` |
| extract-schlib | Extract schematic library | `altium-cli intlib extract-schlib library.IntLib -o out.SchLib` |
| extract-pcblib | Extract PCB library | `altium-cli intlib extract-pcblib library.IntLib -o out.PcbLib` |
| json | Export as JSON | `altium-cli intlib json library.IntLib` |

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
- Parse Altium file formats (delegates to altium-format v2 types)
- Query language execution (delegates to altium-format::query)
- Edit session logic (delegates to altium-format::edit)
- Record type definitions (delegates to altium-format::v2::fields)

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
use altium_format::v2::io::schlib::SchLibV2;

pub fn run(path: &Path, format: &str) -> Result<(), Box<dyn std::error::Error>> {
    let lib = SchLibV2::open_file(path)?;  // V2 API
    output::print(&lib.components(), format)?;
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
