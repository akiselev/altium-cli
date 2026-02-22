use std::path::PathBuf;
use std::process::ExitCode;

use altium_format::{
    AltiumProject, IntLib, PcbDoc, PcbLib, SchDoc, SchLib,
};
use altium_format_ops::{
    AltiumProjectOps, IntLibOps, PcbDocOps, PcbLibOps, SchDocOps, SchLibOps,
};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "altium", about = "CLI tool for Altium Designer files")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate an Altium Designer document
    Validate {
        /// Path to the document to validate
        path: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Commands::Validate { path } => {
            if let Err(e) = validate(&path) {
                eprintln!("Error: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    ExitCode::SUCCESS
}

fn validate(path: &PathBuf) -> anyhow::Result<()> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| anyhow::anyhow!("cannot determine file type: {}", path.display()))?;

    match ext.to_ascii_lowercase().as_str() {
        "schdoc" => {
            let doc = SchDoc::open(path)?;
            doc.validate()?;
        }
        "schlib" => {
            let doc = SchLib::open(path)?;
            doc.validate()?;
        }
        "pcbdoc" => {
            let doc = PcbDoc::open(path)?;
            doc.validate()?;
        }
        "pcblib" => {
            let doc = PcbLib::open(path)?;
            doc.validate()?;
        }
        "intlib" => {
            let doc = IntLib::open(path)?;
            doc.validate()?;
        }
        "prjpcb" => {
            let doc = AltiumProject::open(path)?;
            doc.validate()?;
        }
        _ => anyhow::bail!("unsupported file extension: .{ext}"),
    }

    println!("Validation passed: {}", path.display());
    Ok(())
}
