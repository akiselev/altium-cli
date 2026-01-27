//! Altium CLI - Command-line tool for inspecting and manipulating Altium files.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::commands::{intlib::IntLibCommands, pcbdoc::PcbDocCommands, pcblib::PcbLibCommands, prjpcb::PrjPcbCommands, schdoc::SchDocCommands, schlib::SchLibCommands};

#[derive(Parser)]
#[command(name = "altium-cli")]
#[command(version, about = "Command-line tool for Altium Designer files", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output as JSON (compact by default, use --pretty for formatted)
    #[arg(long, global = true)]
    json: bool,

    /// Pretty-print JSON output (implies --json)
    #[arg(long, global = true)]
    pretty: bool,

    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Quiet mode (errors only)
    #[arg(short, long, global = true)]
    quiet: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Inspect file structure and contents (quick overview)
    Inspect {
        /// Path to Altium file (.SchLib, .PcbLib, .SchDoc, .PcbDoc)
        path: PathBuf,
    },

    /// Query records using selector syntax
    ///
    /// Supports two query languages:
    ///
    /// Record Selector (domain-specific):
    ///   - U1, R*, C?? - Component by designator
    ///   - $LM358 - Component by part number
    ///   - ~VCC - Net by name
    ///   - @10K - Component by value
    ///   - U1:VCC - Pin by component.pin
    ///
    /// SchQL (CSS-like):
    ///   - component[part*=7805] - Components containing "7805"
    ///   - pin[type=input] - Input pins
    ///   - net:power - Power nets
    Query {
        /// Path to Altium file (.SchDoc, .SchLib)
        path: PathBuf,

        /// Selector query (e.g., "R*", "$LM358", "component[part*=MCU]")
        selector: String,
    },

    /// Edit schematic operations
    ///
    /// Supported operations:
    ///   - move <designator> <x> <y> - Move component to (x,y) mils
    ///   - delete <designator> - Delete component
    ///   - add-wire <x1>,<y1>,<x2>,<y2>,... - Add wire
    ///   - delete-wire <index> - Delete wire by index
    ///   - add-net-label <name> <x> <y> - Add net label
    ///   - add-power <name> <x> <y> <style> <orientation> - Add power port
    ///   - add-junction <x> <y> - Add junction
    ///   - add-missing-junctions - Add junctions where wires cross
    ///   - add-port <name> <x> <y> <io_type> - Add port
    ///   - route <from> <to> - Route wire (from/to: x,y or Component.Pin)
    ///   - validate - Validate schematic
    Edit {
        /// Path to Altium file (.SchDoc)
        path: PathBuf,

        /// Edit operation (e.g., "move U1 1000 2000", "delete R3")
        #[arg(short = 'c', long = "command")]
        operation: String,

        /// Output file (defaults to input file, overwrites)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Schematic document operations (analysis, BOM, netlist, etc.)
    #[command(name = "schdoc")]
    SchDoc {
        #[command(subcommand)]
        command: SchDocCommands,
    },

    /// Schematic library operations (browse, search, components)
    #[command(name = "schlib")]
    SchLib {
        #[command(subcommand)]
        command: SchLibCommands,
    },

    /// PCB document operations (rules, layers, components, tracks)
    #[command(name = "pcbdoc")]
    PcbDoc {
        #[command(subcommand)]
        command: PcbDocCommands,
    },

    /// PCB footprint library operations (browse, measure, footprints)
    #[command(name = "pcblib")]
    PcbLib {
        #[command(subcommand)]
        command: PcbLibCommands,
    },

    /// PCB project operations (documents, BOM, netlist, sync)
    #[command(name = "prjpcb")]
    PrjPcb {
        #[command(subcommand)]
        command: PrjPcbCommands,
    },

    /// Integrated library operations (components, symbols, footprints)
    #[command(name = "intlib")]
    IntLib {
        #[command(subcommand)]
        command: IntLibCommands,
    },

    /// Generate shell completions
    Completions {
        /// Shell type (bash, zsh, fish, powershell)
        shell: String,
    },
}

/// Parse CLI arguments and dispatch to command handlers.
///
/// Handles global flags (--json, --pretty, --verbose, --quiet) and routes
/// subcommands to their respective implementations.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let format = if cli.pretty || cli.json {
        if cli.pretty { "json-pretty" } else { "json" }
    } else {
        "text"
    };

    match cli.command {
        Commands::Inspect { path } => {
            crate::commands::inspect::run(&path, format, cli.verbose)?;
        }
        Commands::Query { path, selector } => {
            crate::commands::query::run(&path, &selector, format)?;
        }
        Commands::Edit {
            path,
            operation,
            output,
        } => {
            crate::commands::edit::run(&path, &operation, format, output.as_deref())?;
        }
        Commands::SchDoc { command } => {
            crate::commands::schdoc::run(&command, format)?;
        }
        Commands::SchLib { command } => {
            crate::commands::schlib::run(&command, format)?;
        }
        Commands::PcbDoc { command } => {
            crate::commands::pcbdoc::run(&command, format)?;
        }
        Commands::PcbLib { command } => {
            crate::commands::pcblib::run(&command, format)?;
        }
        Commands::PrjPcb { command } => {
            crate::commands::prjpcb::run(&command, format)?;
        }
        Commands::IntLib { command } => {
            crate::commands::intlib::run(&command, format)?;
        }
        Commands::Completions { shell } => {
            use clap::CommandFactory;
            use clap_complete::{Shell, generate};

            let shell_type = match shell.to_lowercase().as_str() {
                "bash" => Shell::Bash,
                "zsh" => Shell::Zsh,
                "fish" => Shell::Fish,
                "powershell" => Shell::PowerShell,
                _ => return Err(format!("Unsupported shell: {}", shell).into()),
            };

            generate(
                shell_type,
                &mut Cli::command(),
                "altium-cli",
                &mut std::io::stdout(),
            );
        }
    }

    Ok(())
}

/// CLI command implementations.
mod commands;

/// Output formatting utilities.
mod output;
