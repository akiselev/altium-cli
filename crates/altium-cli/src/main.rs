use std::path::PathBuf;
use std::process::ExitCode;

use altium_format::{AltiumProject, IntLib, PcbDoc, PcbLib, SchDoc, SchLib};
use altium_format_ops::{
    AltiumProjectOps, IntLibOps, PcbDocOps, PcbLibOps, SchDocOps, SchLibOps,
    apply_ops_source_pcbdoc, apply_ops_source_pcblib, apply_ops_source_schdoc,
    apply_ops_source_schlib,
};
use altium_format_render_png::{
    DEFAULT_SCALE, render_pcblib_footprint_png, render_schdoc_png, render_schlib_component_png,
};
use altium_format_render_svg::{render_pcblib_footprint, render_schdoc, render_schlib_component};
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
    /// Create a new Altium document with Altium-default template contents
    New {
        #[command(subcommand)]
        sub: NewSubcommand,
    },
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
    /// Render an Altium document to SVG or PNG
    Render {
        /// Path to the input file (.SchLib, .SchDoc, or .PcbLib)
        path: PathBuf,
        /// Output directory (default: current directory)
        #[arg(long, short = 'o', default_value = ".")]
        output_dir: PathBuf,
        /// Render only this component/sheet/footprint name (default: all)
        #[arg(long)]
        name: Option<String>,
        /// Output format: svg or png
        #[arg(long, default_value = "svg")]
        format: String,
        /// Scale factor for PNG output (pixels per mil, default 4.0)
        #[arg(long, default_value_t = DEFAULT_SCALE)]
        scale: f32,
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
    /// Apply operations from an .ops source file
    Apply {
        /// Path to target .SchDoc or .SchLib file
        path: PathBuf,
        /// Path to .ops source file
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

#[derive(Subcommand)]
enum NewSubcommand {
    /// Create a new blank .SchDoc
    Schdoc {
        /// Output path for the new .SchDoc
        output: PathBuf,
    },
    /// Create a new blank .SchLib
    Schlib {
        /// Output path for the new .SchLib
        output: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Commands::New { sub } => {
            if let Err(e) = run_new(sub) {
                eprintln!("Error: {e}");
                return ExitCode::FAILURE;
            }
        }
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
        Commands::Render {
            path,
            output_dir,
            name,
            format,
            scale,
        } => {
            if let Err(e) = run_render(&path, &output_dir, name.as_deref(), &format, scale) {
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

fn run_new(sub: NewSubcommand) -> anyhow::Result<()> {
    match sub {
        NewSubcommand::Schdoc { output } => {
            if let Some(parent) = output.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)?;
                }
            }
            let doc = SchDoc::new_blank_ad26();
            doc.save_as(output.as_path())?;
            println!("Created {}", output.display());
        }
        NewSubcommand::Schlib { output } => {
            if let Some(parent) = output.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)?;
                }
            }
            let lib = SchLib::new_blank_ad26();
            lib.save_as(output.as_path())?;
            println!("Created {}", output.display());
        }
    }
    Ok(())
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
    if spec_ext.as_str() != "ops" {
        anyhow::bail!("unsupported spec extension .{spec_ext} (supported: .ops)");
    }

    let doc_ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| anyhow::anyhow!("cannot determine document type: {}", path.display()))?;

    let out_path = output.cloned().unwrap_or_else(|| path.clone());

    match doc_ext.as_str() {
        "schdoc" => {
            let mut doc = SchDoc::open(path)?;
            let report = apply_ops_source_schdoc(&mut doc, &spec_data)?;
            if !dry_run {
                doc.save_as(out_path.as_path())?;
            }
            print_apply_report(&report, dry_run, &out_path, report_json)?;
        }
        "schlib" => {
            let mut lib = SchLib::open(path)?;
            let report = apply_ops_source_schlib(&mut lib, &spec_data)?;
            if !dry_run {
                lib.save_as(out_path.as_path())?;
            }
            print_apply_report(&report, dry_run, &out_path, report_json)?;
        }
        "pcbdoc" => {
            let mut doc = PcbDoc::open(path)?;
            let report = apply_ops_source_pcbdoc(&mut doc, &spec_data)?;
            if !dry_run {
                doc.save_as(out_path.as_path())?;
            }
            print_apply_report(&report, dry_run, &out_path, report_json)?;
        }
        "pcblib" => {
            let mut lib = PcbLib::open(path)?;
            let report = apply_ops_source_pcblib(&mut lib, &spec_data)?;
            if !dry_run {
                lib.save_as(out_path.as_path())?;
            }
            print_apply_report(&report, dry_run, &out_path, report_json)?;
        }
        _ => anyhow::bail!(
            "ops apply not yet supported for .{doc_ext} files (supported: .schdoc, .schlib, .pcbdoc, .pcblib)"
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

fn run_render(
    path: &PathBuf,
    output_dir: &PathBuf,
    name: Option<&str>,
    format: &str,
    scale: f32,
) -> anyhow::Result<()> {
    if format != "svg" && format != "png" {
        anyhow::bail!("unsupported format '{}' (supported: svg, png)", format);
    }
    std::fs::create_dir_all(output_dir)?;

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| anyhow::anyhow!("cannot determine file type: {}", path.display()))?;

    match ext.as_str() {
        "schlib" => {
            let lib = SchLib::open(path)?;
            let names = lib.component_names();
            let to_render: Vec<&str> = if let Some(n) = name {
                vec![n]
            } else {
                names.iter().map(|s| s.as_str()).collect()
            };
            for comp_name in to_render {
                let safe = safe_filename(comp_name);
                let out_path = output_dir.join(format!("{safe}.{format}"));
                if format == "svg" {
                    std::fs::write(&out_path, render_schlib_component(&lib, comp_name)?)?;
                } else {
                    std::fs::write(
                        &out_path,
                        render_schlib_component_png(&lib, comp_name, scale)
                            .map_err(|e| anyhow::anyhow!("{e}"))?,
                    )?;
                }
                println!("  {comp_name} -> {}", out_path.display());
            }
        }
        "schdoc" => {
            let doc = SchDoc::open(path)?;
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("sheet");
            let out_path = output_dir.join(format!("{stem}.{format}"));
            if format == "svg" {
                std::fs::write(&out_path, render_schdoc(&doc)?)?;
            } else {
                std::fs::write(
                    &out_path,
                    render_schdoc_png(&doc, scale).map_err(|e| anyhow::anyhow!("{e}"))?,
                )?;
            }
            println!("  {stem} -> {}", out_path.display());
        }
        "pcblib" => {
            let lib = PcbLib::open(path)?;
            let names: Vec<String> = lib
                .footprint_names()
                .iter()
                .map(|s| s.to_string())
                .collect();
            let to_render: Vec<&str> = if let Some(n) = name {
                vec![n]
            } else {
                names.iter().map(|s| s.as_str()).collect()
            };
            for fp_name in to_render {
                let safe = safe_filename(fp_name);
                let out_path = output_dir.join(format!("{safe}.{format}"));
                if format == "svg" {
                    std::fs::write(&out_path, render_pcblib_footprint(&lib, fp_name)?)?;
                } else {
                    std::fs::write(
                        &out_path,
                        render_pcblib_footprint_png(&lib, fp_name, scale)
                            .map_err(|e| anyhow::anyhow!("{e}"))?,
                    )?;
                }
                println!("  {fp_name} -> {}", out_path.display());
            }
        }
        _ => {
            anyhow::bail!("render not supported for .{ext} (supported: .schlib, .schdoc, .pcblib)")
        }
    }

    Ok(())
}

fn safe_filename(name: &str) -> String {
    name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
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

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::io::Write;

    fn write_minimal_ops_file(dir: &std::path::Path) -> PathBuf {
        let path = dir.join("input.ops");
        let mut file = std::fs::File::create(&path).expect("create input.ops");
        writeln!(
            file,
            "ops:\n  - op: query\n    selector: component[designator=R1]"
        )
        .expect("write input.ops");
        path
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 32, .. ProptestConfig::default() })]

        #[test]
        fn ops_apply_reports_explicit_unsupported_extensions(
            ext in prop::sample::select(vec![
                "intlib".to_owned(),
                "prjpcb".to_owned(),
                "txt".to_owned(),
            ])
        ) {
            let tmp = tempfile::tempdir().expect("tempdir");
            let spec = write_minimal_ops_file(tmp.path());
            let target = tmp.path().join(format!("dummy.{ext}"));

            let err = apply_ops(&target, &spec, None, true, false).expect_err("unsupported ext must fail");
            let msg = err.to_string();
            prop_assert!(
                msg.contains("ops apply not yet supported"),
                "unexpected error for .{ext}: {msg}"
            );
        }
    }
}
