use std::path::PathBuf;
use std::process::ExitCode;

use altium_format::{AltiumProject, IntLib, PcbDoc, PcbLib, SchDoc, SchLib};
use altium_format_ops::{
    AltiumProjectOps, IntLibOps, PcbDocOps, PcbLibOps, SchDocOps, SchLibOps, apply_schdoc,
    apply_schlib, parse_apply_spec_json, parse_apply_spec_yaml,
};
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
    /// High-level operations
    Ops {
        #[command(subcommand)]
        sub: OpsSubcommand,
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

#[derive(Subcommand)]
enum OpsSubcommand {
    /// Apply operations from a YAML/JSON spec file
    Apply {
        /// Path to target .SchDoc or .SchLib file
        path: PathBuf,
        /// Path to spec file (.yaml/.yml/.json)
        #[arg(long)]
        spec_file: PathBuf,
        /// Optional output path (default: in-place)
        #[arg(long)]
        output: Option<PathBuf>,
        /// Resolve and execute ops without saving output
        #[arg(long, default_value_t = false)]
        dry_run: bool,
        /// Print full result table as JSON
        #[arg(long, default_value_t = false)]
        report_json: bool,
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
        Commands::Ops { sub } => {
            if let Err(e) = run_ops(sub) {
                eprintln!("Error: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    ExitCode::SUCCESS
}

fn run_ops(sub: OpsSubcommand) -> anyhow::Result<()> {
    match sub {
        OpsSubcommand::Apply {
            path,
            spec_file,
            output,
            dry_run,
            report_json,
        } => apply_ops(&path, &spec_file, output.as_ref(), dry_run, report_json),
    }
}

fn apply_ops(
    path: &PathBuf,
    spec_file: &PathBuf,
    output: Option<&PathBuf>,
    dry_run: bool,
    report_json: bool,
) -> anyhow::Result<()> {
    let spec_data = std::fs::read_to_string(spec_file)?;
    let spec_ext = spec_file
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| {
            anyhow::anyhow!("cannot determine spec file type: {}", spec_file.display())
        })?;
    let ops = match spec_ext.as_str() {
        "yaml" | "yml" => parse_apply_spec_yaml(&spec_data)?,
        "json" => parse_apply_spec_json(&spec_data)?,
        _ => anyhow::bail!("unsupported spec extension .{spec_ext} (supported: .yaml/.yml/.json)"),
    };

    let doc_ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| anyhow::anyhow!("cannot determine document type: {}", path.display()))?;

    let out_path = output.cloned().unwrap_or_else(|| path.clone());

    match doc_ext.as_str() {
        "schdoc" => {
            let mut doc = SchDoc::open(path)?;
            let report = apply_schdoc(&mut doc, &ops)?;
            if !dry_run {
                doc.save_as(out_path.as_path())?;
            }
            print_apply_report(&report, dry_run, &out_path, report_json)?;
        }
        "schlib" => {
            let mut lib = SchLib::open(path)?;
            let report = apply_schlib(&mut lib, &ops)?;
            if !dry_run {
                lib.save_as(out_path.as_path())?;
            }
            print_apply_report(&report, dry_run, &out_path, report_json)?;
        }
        _ => anyhow::bail!(
            "ops apply not yet supported for .{doc_ext} files (supported: .schdoc, .schlib)"
        ),
    }

    Ok(())
}

fn print_apply_report(
    report: &altium_format_ops::ApplyReport,
    dry_run: bool,
    out_path: &PathBuf,
    report_json: bool,
) -> anyhow::Result<()> {
    if report_json {
        println!("{}", serde_json::to_string_pretty(report)?);
        return Ok(());
    }

    let mode = if dry_run {
        "Dry-run executed"
    } else {
        "Applied"
    };
    println!(
        "{mode} {} high ops ({} composed, {} low) {} {}",
        report.high_op_count,
        report.composed_op_count,
        report.low_op_count,
        if dry_run { "for" } else { "to" },
        out_path.display()
    );
    for (opid, result) in &report.results {
        let ref_display = result
            .ref_
            .as_ref()
            .map(|r| r.display_path.clone())
            .unwrap_or_else(|| "-".to_owned());
        println!("  {opid}: kind={} ref={}", result.kind, ref_display);
    }
    Ok(())
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
        // "pcblib" => {
        //     let doc = PcbLib::open(input)?;
        //     doc.save_as(output.as_path())?;
        // }
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
