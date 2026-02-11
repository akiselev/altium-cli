# Phase 7: CLI & Ops Migration

**Agents: 4 parallel tracks (7A, 7B, 7C, 7D)**
**Blocked by: Phase 4 (documents), Phase 6 (templates/builders)**

Each track migrates one ops module and its corresponding CLI command to use the v2 API exclusively.

---

## Track 7A: SchLib Ops & CLI

**Files:**
- `crates/altium-format/src/v2/ops/mod.rs`
- `crates/altium-format/src/v2/ops/schlib.rs`
- `crates/altium-format/src/v2/ops/output.rs` (shared output types)

**Reference:**
- `ops/schlib.rs` — current SchLib operations
- `ops/output.rs` — current output formatting types
- `crates/altium-cli/src/commands/schlib.rs` — CLI command handler

### What to Build

1. **Output types** (`ops/output.rs`) — copy and adapt from `ops/output.rs`:
   ```rust
   pub struct ComponentInfo {
       pub lib_ref: String,
       pub description: String,
       pub pin_count: usize,
       pub part_count: i16,
       pub category: String,
   }

   pub struct PinInfo {
       pub designator: String,
       pub name: String,
       pub electrical: String,
       pub side: String,
   }

   pub struct RecordInfo {
       pub record_id: u8,
       pub record_type: String,
       pub params: serde_json::Value,
   }
   ```

2. **SchLib operations** (`ops/schlib.rs`):
   ```rust
   use crate::v2::documents::schlib::SchLib;

   pub fn list_components(path: &Path) -> Result<Vec<ComponentInfo>> {
       let lib = SchLib::open_file(path)?;
       // Iterate groups, extract component info
       // Use v2 getters: comp.lib_reference(), comp.description(), etc.
   }

   pub fn show_component(path: &Path, designator: &str) -> Result<ComponentDetail> {
       let mut lib = SchLib::open_file(path)?;
       lib.query::<SchComponent>(designator)?.with_mut(|comp| {
           // Extract detail using v2 getters
       })
   }

   pub fn list_pins(path: &Path, designator: &str) -> Result<Vec<PinInfo>> {
       let mut lib = SchLib::open_file(path)?;
       lib.query::<SchComponent>(designator)?.with_mut(|comp| {
           let mut pins = Vec::new();
           comp.for_each_pin_mut(|pin| {
               pins.push(PinInfo {
                   designator: pin.designator().to_string(),
                   name: pin.name().to_string(),
                   electrical: format!("{:?}", pin.electrical()),
                   side: String::new(), // compute from orientation
               });
           });
           pins
       })
   }

   pub fn export_json(path: &Path) -> Result<serde_json::Value> {
       let lib = SchLib::open_file(path)?;
       Ok(serde_json::to_value(&lib)?)
   }

   pub fn import_json(json: &str, output_path: &Path) -> Result<()> {
       let lib: SchLib = serde_json::from_str(json)?;
       lib.save_file(output_path)?;
       Ok(())
   }
   ```

3. **Categorization utility** — copy from `ops/categorization.rs`:
   ```rust
   pub fn categorize_component(lib_ref: &str) -> String { ... }
   ```

### CLI Command Handler

Update the CLI crate's schlib command to use v2 ops. This requires restoring the CLI crate from the stub (Phase 0) with new v2 imports:

```rust
// crates/altium-cli/src/commands/schlib.rs
use altium_format::v2::ops::schlib;

pub fn handle_schlib(cmd: SchLibCommand) -> Result<()> {
    match cmd {
        SchLibCommand::List { path } => {
            let components = schlib::list_components(&path)?;
            // format and print
        }
        SchLibCommand::Export { path, format } => {
            let json = schlib::export_json(&path)?;
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        // ... etc
    }
}
```

### Acceptance Criteria

- [ ] `v2::ops::schlib` provides all operations the CLI needs
- [ ] All operations use v2 document types and record getters exclusively
- [ ] JSON export produces equivalent output to v1
- [ ] `cargo check` passes

---

## Track 7B: PcbLib Ops & CLI

**Files:**
- `crates/altium-format/src/v2/ops/pcblib.rs`

**Reference:**
- `ops/pcblib.rs` — current PcbLib operations
- `crates/altium-cli/src/commands/pcblib.rs` — CLI command handler

### What to Build

Same pattern as Track 7A but for PcbLib:

```rust
pub fn list_footprints(path: &Path) -> Result<Vec<FootprintInfo>> { ... }
pub fn show_footprint(path: &Path, name: &str) -> Result<FootprintDetail> { ... }
pub fn export_json(path: &Path) -> Result<serde_json::Value> { ... }
pub fn import_json(json: &str, output_path: &Path) -> Result<()> { ... }
```

### Acceptance Criteria

- [ ] `v2::ops::pcblib` provides all operations
- [ ] Uses v2 document types exclusively
- [ ] `cargo check` passes

---

## Track 7C: SchDoc & PcbDoc Ops

**Files:**
- `crates/altium-format/src/v2/ops/schdoc.rs`
- `crates/altium-format/src/v2/ops/pcbdoc.rs`

**Reference:**
- `ops/schdoc.rs`, `ops/pcbdoc.rs` — current operations
- `crates/altium-cli/src/commands/schdoc.rs`, `pcbdoc.rs`

### What to Build

Same pattern for SchDoc and PcbDoc operations.

### Acceptance Criteria

- [ ] `v2::ops::schdoc` and `v2::ops::pcbdoc` provide all operations
- [ ] `cargo check` passes

---

## Track 7D: CLI Main & Command Structure

**Files:**
- `crates/altium-cli/src/main.rs` (restore from stub)
- `crates/altium-cli/src/commands/mod.rs`
- `crates/altium-cli/src/commands/schlib.rs`
- `crates/altium-cli/src/commands/pcblib.rs`
- `crates/altium-cli/src/commands/schdoc.rs`
- `crates/altium-cli/src/commands/pcbdoc.rs`
- `crates/altium-cli/src/commands/template.rs`
- `crates/altium-cli/src/output.rs`

**Reference:**
- Current CLI source in `crates/altium-cli/src/`

### What to Build

Restore the full CLI with clap argument parsing, but using v2 ops:

```rust
// main.rs
use clap::Parser;

#[derive(Parser)]
#[command(name = "altium-cli")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    Schlib(commands::schlib::SchLibArgs),
    Pcblib(commands::pcblib::PcbLibArgs),
    Schdoc(commands::schdoc::SchDocArgs),
    Pcbdoc(commands::pcbdoc::PcbDocArgs),
}
```

Each command module thin-wraps the corresponding `v2::ops` functions.

### Also Handle:

- `intlib` operations (if applicable — reference `ops/intlib.rs`)
- `prjpcb` operations (if applicable — reference `ops/prjpcb.rs`)
- `template` command (if applicable — reference `commands/template.rs`)
- Output formatting (`crates/altium-cli/src/output.rs`)

### Acceptance Criteria

- [ ] CLI builds and runs with all subcommands
- [ ] All commands use v2 ops exclusively
- [ ] `cargo build --workspace` succeeds
- [ ] `altium-cli schlib list Synthiam.SchLib` produces output
- [ ] `altium-cli schlib export Synthiam.SchLib` produces JSON
