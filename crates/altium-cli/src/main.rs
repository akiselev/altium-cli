use std::path::PathBuf;
use std::process::ExitCode;

use altium_format::{AltiumProject, IntLib, PcbDoc, PcbLib, SchDoc, SchLib};
use altium_format_ops::{AltiumProjectOps, IntLibOps, PcbDocOps, PcbLibOps, SchDocOps, SchLibOps};
use clap::{Parser, Subcommand};

mod cfb;

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
    /// Open, parse, and re-save a file (roundtrip for debugging)
    SaveAs {
        /// Path to the input file
        input: PathBuf,
        /// Path to the output file
        output: PathBuf,
    },
    /// CFB container inspection and debugging tools
    Cfb {
        #[command(subcommand)]
        sub: cfb::CfbSubcommand,
    },
    /// Query document properties
    Get {
        #[command(subcommand)]
        sub: GetSubcommand,
    },
}

#[derive(Subcommand)]
enum GetSubcommand {
    /// Display the document's format version header and minor version
    Version {
        /// Path to the document
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
        Commands::SaveAs { input, output } => {
            if let Err(e) = save_as(&input, &output) {
                eprintln!("Error: {e}");
                return ExitCode::FAILURE;
            }
        }
        Commands::Cfb { sub } => match cfb::run(sub) {
            Ok(code) => return code,
            Err(e) => {
                eprintln!("Error: {e}");
                return ExitCode::FAILURE;
            }
        },
        Commands::Get { sub } => {
            if let Err(e) = run_get(sub) {
                eprintln!("Error: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    ExitCode::SUCCESS
}

fn run_get(sub: GetSubcommand) -> anyhow::Result<()> {
    match sub {
        GetSubcommand::Version { path } => get_version(&path),
    }
}

fn get_version(path: &PathBuf) -> anyhow::Result<()> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| anyhow::anyhow!("cannot determine file type: {}", path.display()))?;

    match ext.to_ascii_lowercase().as_str() {
        "schlib" => {
            let doc = SchLib::open(path)?;
            let info = doc.version()?;
            println!("Header:        {}", info.header);
            println!("Minor version: {}", info.minor_version);
            if let Some(ref fvi) = info.file_version_info {
                println!("FileVersionInfo: {fvi}");
            }
        }
        "pcblib" => {
            let doc = PcbLib::open(path)?;
            let info = doc.version()?;
            println!("Header:        {}", info.header);
            println!("Minor version: {}", info.minor_version);
            if let Some(ref fvi) = info.file_version_info {
                println!("FileVersionInfo: {fvi}");
            }
        }
        _ => anyhow::bail!("get version not yet supported for .{ext} files"),
    }

    Ok(())
}

fn save_as(input: &PathBuf, output: &PathBuf) -> anyhow::Result<()> {
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| anyhow::anyhow!("cannot determine file type: {}", input.display()))?;

    match ext.as_str() {
        "schdoc" => {
            let doc = SchDoc::open(input)?;
            doc.save_as(output.as_path())?;
        }
        "schlib" => {
            let doc = SchLib::open(input)?;
            doc.save_as(output.as_path())?;
        }
        _ => anyhow::bail!(
            "save-as not yet supported for .{ext} files (supported: .schdoc, .schlib)"
        ),
    }

    println!("Saved: {} -> {}", input.display(), output.display());
    Ok(())
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
