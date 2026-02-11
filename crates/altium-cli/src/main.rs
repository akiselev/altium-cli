//! Altium CLI - Command-line tool for inspecting and manipulating Altium files.

use clap::{Parser, Subcommand};

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

    /// Schematic library operations (browse, pins, primitives)
    #[command(name = "schlib")]
    SchLib {
        #[command(subcommand)]
        command: SchLibCommands,
    },

    /// Schematic document operations (components, nets, wires, ports)
    #[command(name = "schdoc")]
    SchDoc {
        #[command(subcommand)]
        command: SchDocCommands,
    },

    /// Project file operations (documents, bom, validate)
    #[command(name = "prjpcb")]
    PrjPcb {
        #[command(subcommand)]
        command: PrjPcbCommands,
    },

    /// Integrated library operations (browse, extract, search)
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
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let format = if cli.pretty || cli.json {
        if cli.pretty { "json-pretty" } else { "json" }
    } else {
        "text"
    };

    match cli.command {
        Commands::PcbDoc { command } => {
            crate::commands::pcbdoc::run(&command, format)?;
        }
        Commands::PcbLib { command } => {
            crate::commands::pcblib::run(&command, format)?;
        }
        Commands::SchLib { command } => {
            crate::commands::schlib::run(&command, format)?;
        }
        Commands::SchDoc { command } => {
            crate::commands::schdoc::run(&command, format)?;
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
