use std::path::PathBuf;
use std::process::ExitCode;

use altium_format::{AltiumProject, IntLib, PcbDoc, PcbLib, SchDoc, SchLib, VersionInfo};
use autopcb_ir::PcbIr;
use altium_format_query::{eval_query, parse_query};
use altium_format_spec::{
    SpecDomain, compile_spec, dump_pcbdoc, dump_pcblib, dump_prjpcb, dump_schdoc, dump_schlib,
    reconcile_pcbdoc, reconcile_pcbdoc_empty,
    reconcile_pcblib, reconcile_pcblib_empty, reconcile_prjpcb, reconcile_prjpcb_empty,
    reconcile_schdoc, reconcile_schdoc_empty,
    reconcile_schlib, reconcile_schlib_empty, resolve_imports,
    apply_spec_pcbdoc, apply_spec_schlib, apply_spec_pcblib, apply_spec_prjpcb, apply_spec_schdoc,
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
    /// Show ECO (engineering change order) without mutating the document
    Plan {
        /// Path to the spec file (.schlib-spec, .pcblib-spec, or .prjpcb-spec)
        spec_file: PathBuf,
        /// Existing document to reconcile against (optional)
        #[arg(long)]
        target: Option<PathBuf>,
        /// Output ECO as JSON
        #[arg(long, default_value_t = false)]
        json: bool,
        /// Process this spec and all imported specs (PrjPcb only)
        #[arg(long, default_value_t = false)]
        all: bool,
    },
    /// Apply a spec file to create or update an Altium document
    Apply {
        /// Path to the spec file (.schlib-spec, .pcblib-spec, or .prjpcb-spec)
        spec_file: PathBuf,
        /// Existing document to update (optional)
        #[arg(long)]
        target: Option<PathBuf>,
        /// Output file path (overrides default)
        #[arg(long)]
        output: Option<PathBuf>,
        /// Print apply report as JSON
        #[arg(long, default_value_t = false)]
        report_json: bool,
        /// Process this spec and all imported specs (PrjPcb only)
        #[arg(long, default_value_t = false)]
        all: bool,
    },
    /// Reverse-generate a spec file from an existing Altium document
    Dump {
        /// Path to the document (.SchLib or .PcbLib)
        document: PathBuf,
        /// Output spec file path (overrides default)
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Show document summary (object counts, net names, hierarchy)
    Info {
        /// Path to the document
        path: PathBuf,
        /// Output format: text or json
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Inspect PcbDoc board data via the autopcb IR
    Inspect {
        /// Path to the .PcbDoc file
        path: PathBuf,
        #[command(subcommand)]
        sub: InspectSubcommand,
    },
    /// Query entities in an Altium document using AQL (Altium Query Language)
    Query {
        /// Path to the document (.SchLib, .PcbLib, or .SchDoc)
        path: PathBuf,
        /// AQL query string (e.g., "component > pin:power")
        query: String,
        /// Output format: text, json, or count
        #[arg(long, default_value = "text")]
        format: String,
        /// Maximum number of results to return
        #[arg(long)]
        limit: Option<usize>,
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
    /// Create a new blank .PcbLib
    Pcblib {
        /// Output path for the new .PcbLib
        output: PathBuf,
    },
    /// Create a new blank .PrjPcb
    Prjpcb {
        /// Output path for the new .PrjPcb
        output: PathBuf,
    },
}

#[derive(Subcommand)]
enum InspectSubcommand {
    /// Show a summary of the board (dimensions, counts)
    Summary,
    /// List all components with positions and sides
    Components,
    /// List all nets with pin counts
    Nets,
    /// Show the board outline points
    BoardOutline,
    /// List design rules
    Rules,
    /// Export the full IR as JSON
    IrJson,
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
        Commands::Plan { spec_file, target, json, all } => {
            match run_plan(&spec_file, target.as_ref(), json, all) {
                Ok(has_changes) => {
                    if has_changes {
                        return ExitCode::from(1);
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    return ExitCode::FAILURE;
                }
            }
        }
        Commands::Apply { spec_file, target, output, report_json, all } => {
            if let Err(e) = run_apply(&spec_file, target.as_ref(), output.as_ref(), report_json, all) {
                eprintln!("Error: {e}");
                return ExitCode::FAILURE;
            }
        }
        Commands::Dump { document, output } => {
            if let Err(e) = run_dump(&document, output.as_ref()) {
                eprintln!("Error: {e}");
                return ExitCode::FAILURE;
            }
        }
        Commands::Info { path, format } => {
            if let Err(e) = run_info(&path, &format) {
                eprintln!("Error: {e}");
                return ExitCode::FAILURE;
            }
        }
        Commands::Inspect { path, sub } => {
            if let Err(e) = run_inspect(&path, sub) {
                eprintln!("Error: {e}");
                return ExitCode::FAILURE;
            }
        }
        Commands::Query { path, query, format, limit } => {
            if let Err(e) = run_query(&path, &query, &format, limit) {
                eprintln!("Error: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    ExitCode::SUCCESS
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
            doc.save(output.as_path())?;
            println!("Created {}", output.display());
        }
        NewSubcommand::Schlib { output } => {
            if let Some(parent) = output.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)?;
                }
            }
            let lib = SchLib::new_blank_ad26()?;
            lib.save(output.as_path())?;
            println!("Created {}", output.display());
        }
        NewSubcommand::Pcblib { output } => {
            if let Some(parent) = output.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)?;
                }
            }
            let lib = PcbLib::new_blank_ad26()?;
            lib.save(output.as_path())?;
            println!("Created {}", output.display());
        }
        NewSubcommand::Prjpcb { output } => {
            if let Some(parent) = output.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)?;
                }
            }
            let proj = AltiumProject::new_blank_ad26();
            proj.save(output.as_path())?;
            println!("Created {}", output.display());
        }
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

    let info = match ext.to_ascii_lowercase().as_str() {
        "schlib" => {
            let doc = SchLib::open(path)?;
            VersionInfo {
                header: doc.version_header().to_owned(),
                minor_version: doc.minor_version(),
                file_version_info: doc.file_version_info().map(|s| s.to_owned()),
            }
        }
        "pcblib" => {
            let doc = PcbLib::open(path)?;
            VersionInfo {
                header: doc.version_header().to_owned(),
                minor_version: doc.minor_version() as i32,
                file_version_info: doc.file_version_info().map(|s| s.to_owned()),
            }
        }
        _ => anyhow::bail!("get version not yet supported for .{ext} files"),
    };

    println!("Header:        {}", info.header);
    println!("Minor version: {}", info.minor_version);
    if let Some(ref fvi) = info.file_version_info {
        println!("FileVersionInfo: {fvi}");
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
            doc.save(output.as_path())?;
        }
        "schlib" => {
            let doc = SchLib::open(input)?;
            doc.save(output.as_path())?;
        }
        "pcblib" => {
            let doc = PcbLib::open(input)?;
            doc.save(output.as_path())?;
        }
        "pcbdoc" => {
            let doc = PcbDoc::open(input)?;
            doc.save(output.as_path())?;
        }
        "prjpcb" => {
            let doc = AltiumProject::open(input)?;
            doc.save(output.as_path())?;
        }
        _ => anyhow::bail!(
            "save-as not yet supported for .{ext} files (supported: .schdoc, .schlib, .pcbdoc, .pcblib, .prjpcb)"
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
            doc.validate_invariants()?;
        }
        "schlib" => {
            let doc = SchLib::open(path)?;
            doc.validate_invariants()?;
        }
        "pcbdoc" => {
            let doc = PcbDoc::open(path)?;
            doc.validate_invariants()?;
        }
        "pcblib" => {
            let doc = PcbLib::open(path)?;
            doc.validate_invariants()?;
        }
        "intlib" => {
            let _doc = IntLib::open(path)?;
        }
        "prjpcb" => {
            let doc = AltiumProject::open(path)?;
            let _project = doc.project()?;
        }
        _ => anyhow::bail!("unsupported file extension: .{ext}"),
    }

    println!("Validation passed: {}", path.display());
    Ok(())
}

// ── Spec domain detection ─────────────────────────────────────────────────────

fn detect_spec_domain(path: &PathBuf) -> anyhow::Result<SpecDomain> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("schlib-spec") => Ok(SpecDomain::SchLib),
        Some("schdoc-spec") => Ok(SpecDomain::SchDoc),
        Some("pcblib-spec") => Ok(SpecDomain::PcbLib),
        Some("prjpcb-spec") => Ok(SpecDomain::PrjPcb),
        Some("pcbdoc-spec") => Ok(SpecDomain::PcbDoc),
        Some(ext) => anyhow::bail!("unknown spec file extension .{ext} (supported: .schlib-spec, .schdoc-spec, .pcblib-spec, .prjpcb-spec, .pcbdoc-spec)"),
        None => anyhow::bail!("spec file has no extension: {}", path.display()),
    }
}

fn detect_document_domain(path: &PathBuf) -> anyhow::Result<SpecDomain> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "schlib" => Ok(SpecDomain::SchLib),
        "schdoc" => Ok(SpecDomain::SchDoc),
        "pcblib" => Ok(SpecDomain::PcbLib),
        "prjpcb" => Ok(SpecDomain::PrjPcb),
        "pcbdoc" => Ok(SpecDomain::PcbDoc),
        _ => anyhow::bail!("unknown document extension .{ext} (supported: .schlib, .schdoc, .pcblib, .prjpcb, .pcbdoc)"),
    }
}

fn default_output_for_spec(spec_file: &PathBuf, domain: &SpecDomain) -> PathBuf {
    let stem = spec_file.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
    let ext = match domain {
        SpecDomain::SchLib => "SchLib",
        SpecDomain::SchDoc => "SchDoc",
        SpecDomain::PcbLib => "PcbLib",
        SpecDomain::PrjPcb => "PrjPcb",
        SpecDomain::PcbDoc => "PcbDoc",
    };
    spec_file.with_file_name(format!("{stem}.{ext}"))
}

fn default_spec_for_document(doc: &PathBuf, domain: &SpecDomain) -> PathBuf {
    let stem = doc.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
    let ext = match domain {
        SpecDomain::SchLib => "schlib-spec",
        SpecDomain::SchDoc => "schdoc-spec",
        SpecDomain::PcbLib => "pcblib-spec",
        SpecDomain::PrjPcb => "prjpcb-spec",
        SpecDomain::PcbDoc => "pcbdoc-spec",
    };
    doc.with_file_name(format!("{stem}.{ext}"))
}

// ── plan ──────────────────────────────────────────────────────────────────────

/// Run `altium plan`. Returns Ok(true) if changes exist, Ok(false) if no changes.
fn run_plan(
    spec_file: &PathBuf,
    target: Option<&PathBuf>,
    json: bool,
    all: bool,
) -> anyhow::Result<bool> {
    let domain = detect_spec_domain(spec_file)?;
    if all && domain != SpecDomain::PrjPcb {
        anyhow::bail!("--all is only valid for .prjpcb-spec files");
    }

    let source = std::fs::read_to_string(spec_file)
        .map_err(|e| anyhow::anyhow!("failed to read spec file {}: {e}", spec_file.display()))?;

    let result = compile_and_resolve(&source, spec_file, &domain)?;

    // Process root spec.
    let eco = plan_for_model(&result.model, target, spec_file, &domain)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&eco)?);
    } else {
        println!("{}", eco.render_text());
    }
    let mut has_changes = eco.summary.by_kind.values()
        .any(|k| k.adds > 0 || k.updates > 0);

    // Process imports with --all.
    if all {
        for import_path in &result.import_paths {
            let import_domain = detect_spec_domain(import_path)?;
            let import_source = std::fs::read_to_string(import_path)
                .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", import_path.display()))?;
            let import_result = compile_and_resolve(&import_source, import_path, &import_domain)?;

            if !json {
                println!("\n--- {} ---", import_path.display());
            }
            let eco = plan_for_model(&import_result.model, None, import_path, &import_domain)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&eco)?);
            } else {
                println!("{}", eco.render_text());
            }
            has_changes |= eco.summary.by_kind.values()
                .any(|k| k.adds > 0 || k.updates > 0);
        }
    }

    Ok(has_changes)
}

/// Produce an ECO for a single compiled spec model.
fn plan_for_model(
    spec_model: &altium_format_spec::model::SpecModel,
    target: Option<&PathBuf>,
    spec_file: &PathBuf,
    domain: &SpecDomain,
) -> anyhow::Result<altium_format_spec::eco::EngineeringChangeOrder> {
    let library_path = default_output_for_spec(spec_file, domain);
    let spec_path = spec_file.clone();

    let eco = match spec_model {
        altium_format_spec::model::SpecModel::SchLib(spec_lib) => {
            let resolved_target = target.cloned().unwrap_or_else(|| library_path.clone());
            if resolved_target.exists() {
                let doc = SchLib::open(&resolved_target)
                    .map_err(|e| anyhow::anyhow!("failed to open {}: {e}", resolved_target.display()))?;
                reconcile_schlib(spec_lib, &doc, library_path, spec_path)
                    .map_err(|e| anyhow::anyhow!("reconcile failed: {e}"))?
            } else {
                reconcile_schlib_empty(spec_lib, library_path, spec_path)
            }
        }
        altium_format_spec::model::SpecModel::PcbLib(spec_lib) => {
            let resolved_target = target.cloned().unwrap_or_else(|| library_path.clone());
            if resolved_target.exists() {
                reconcile_pcblib(spec_lib, resolved_target, spec_path)
            } else {
                reconcile_pcblib_empty(spec_lib, library_path, spec_path)
            }
        }
        altium_format_spec::model::SpecModel::PrjPcb(spec) => {
            let resolved_target = target.cloned().unwrap_or_else(|| library_path.clone());
            if resolved_target.exists() {
                let doc = AltiumProject::open(&resolved_target)
                    .map_err(|e| anyhow::anyhow!("failed to open {}: {e}", resolved_target.display()))?;
                reconcile_prjpcb(spec, &doc, library_path, spec_path)
                    .map_err(|e| anyhow::anyhow!("reconcile failed: {e}"))?
            } else {
                reconcile_prjpcb_empty(spec, library_path, spec_path)
            }
        }
        altium_format_spec::model::SpecModel::SchDoc(spec) => {
            let resolved_target = target.cloned().unwrap_or_else(|| library_path.clone());
            if resolved_target.exists() {
                let doc = SchDoc::open(&resolved_target)
                    .map_err(|e| anyhow::anyhow!("failed to open {}: {e}", resolved_target.display()))?;
                reconcile_schdoc(spec, &doc, library_path, spec_path)
                    .map_err(|e| anyhow::anyhow!("reconcile failed: {e}"))?
            } else {
                reconcile_schdoc_empty(spec, library_path, spec_path)
            }
        }
        altium_format_spec::model::SpecModel::PcbDoc(spec) => {
            let resolved_target = target.cloned().unwrap_or_else(|| library_path.clone());
            if resolved_target.exists() {
                let doc = PcbDoc::open(&resolved_target)
                    .map_err(|e| anyhow::anyhow!("failed to open {}: {e}", resolved_target.display()))?;
                reconcile_pcbdoc(spec, &doc, library_path, spec_path)
                    .map_err(|e| anyhow::anyhow!("reconcile failed: {e}"))?
            } else {
                reconcile_pcbdoc_empty(spec, library_path, spec_path)
            }
        }
    };

    Ok(eco)
}

// ── apply ─────────────────────────────────────────────────────────────────────

fn run_apply(
    spec_file: &PathBuf,
    target: Option<&PathBuf>,
    output: Option<&PathBuf>,
    _report_json: bool,
    all: bool,
) -> anyhow::Result<()> {
    let domain = detect_spec_domain(spec_file)?;
    if all && domain != SpecDomain::PrjPcb {
        anyhow::bail!("--all is only valid for .prjpcb-spec files");
    }

    let source = std::fs::read_to_string(spec_file)
        .map_err(|e| anyhow::anyhow!("failed to read spec file {}: {e}", spec_file.display()))?;

    let result = compile_and_resolve(&source, spec_file, &domain)?;

    // Apply root spec.
    apply_for_model(&result.model, target, output, spec_file, &domain)?;

    // Apply imports with --all.
    if all {
        for import_path in &result.import_paths {
            let import_domain = detect_spec_domain(import_path)?;
            let import_source = std::fs::read_to_string(import_path)
                .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", import_path.display()))?;
            let import_result = compile_and_resolve(&import_source, import_path, &import_domain)?;
            apply_for_model(&import_result.model, None, None, import_path, &import_domain)?;
        }
    }

    Ok(())
}

/// Apply a single compiled spec model to its target document.
fn apply_for_model(
    spec_model: &altium_format_spec::model::SpecModel,
    target: Option<&PathBuf>,
    output: Option<&PathBuf>,
    spec_file: &PathBuf,
    domain: &SpecDomain,
) -> anyhow::Result<()> {
    let library_path = default_output_for_spec(spec_file, domain);

    match spec_model {
        altium_format_spec::model::SpecModel::SchLib(spec_lib) => {
            let resolved_target = target.cloned().unwrap_or_else(|| library_path.clone());
            let mut doc = if resolved_target.exists() {
                SchLib::open(&resolved_target)
                    .map_err(|e| anyhow::anyhow!("failed to open {}: {e}", resolved_target.display()))?
            } else {
                let mut lib = SchLib::new_blank_ad26()?;
                // Remove the default placeholder component from blank libraries
                let _ = lib.remove_component("Component_1");
                lib
            };

            let out_path = output.cloned().unwrap_or(library_path);

            apply_spec_schlib(spec_lib, &mut doc)
                .map_err(|e| anyhow::anyhow!("apply failed: {e}"))?;

            doc.save(&out_path)?;
            println!("Saved: {}", out_path.display());
        }
        altium_format_spec::model::SpecModel::PcbLib(spec_lib) => {
            let resolved_target = target.cloned().unwrap_or_else(|| library_path.clone());
            let mut lib = if resolved_target.exists() {
                PcbLib::open(&resolved_target)
                    .map_err(|e| anyhow::anyhow!("failed to open {}: {e}", resolved_target.display()))?
            } else {
                PcbLib::new_blank_ad26()?
            };

            let out_path = output.cloned().unwrap_or(library_path);

            apply_spec_pcblib(spec_lib, &mut lib)
                .map_err(|e| anyhow::anyhow!("apply failed: {e}"))?;

            lib.save(&out_path)?;
            println!("Saved: {}", out_path.display());
        }
        altium_format_spec::model::SpecModel::PrjPcb(spec) => {
            let resolved_target = target.cloned().unwrap_or_else(|| library_path.clone());
            let mut doc = if resolved_target.exists() {
                AltiumProject::open(&resolved_target)
                    .map_err(|e| anyhow::anyhow!("failed to open {}: {e}", resolved_target.display()))?
            } else {
                AltiumProject::new_blank_ad26()
            };

            let out_path = output.cloned().unwrap_or(library_path);

            apply_spec_prjpcb(spec, &mut doc)
                .map_err(|e| anyhow::anyhow!("apply failed: {e}"))?;

            doc.save(&out_path)?;
            println!("Saved: {}", out_path.display());
        }
        altium_format_spec::model::SpecModel::SchDoc(spec) => {
            let resolved_target = target.cloned().unwrap_or_else(|| library_path.clone());
            let mut doc = if resolved_target.exists() {
                SchDoc::open(&resolved_target)
                    .map_err(|e| anyhow::anyhow!("failed to open {}: {e}", resolved_target.display()))?
            } else {
                SchDoc::new_blank_ad26()
            };

            let out_path = output.cloned().unwrap_or(library_path);

            apply_spec_schdoc(spec, &mut doc)
                .map_err(|e| anyhow::anyhow!("apply failed: {e}"))?;

            doc.save(&out_path)?;
            println!("Saved: {}", out_path.display());
        }
        altium_format_spec::model::SpecModel::PcbDoc(spec) => {
            let resolved_target = target.cloned().unwrap_or_else(|| library_path.clone());
            if !resolved_target.exists() {
                anyhow::bail!(
                    "PcbDoc apply requires an existing target file: {}",
                    resolved_target.display()
                );
            }
            let mut doc = PcbDoc::open(&resolved_target)
                .map_err(|e| anyhow::anyhow!("failed to open {}: {e}", resolved_target.display()))?;

            let out_path = output.cloned().unwrap_or(library_path);

            apply_spec_pcbdoc(spec, &mut doc)
                .map_err(|e| anyhow::anyhow!("apply failed: {e}"))?;

            doc.save(&out_path)?;
            println!("Saved: {}", out_path.display());
        }
    }

    Ok(())
}

// ── dump ──────────────────────────────────────────────────────────────────────

fn run_dump(document: &PathBuf, output: Option<&PathBuf>) -> anyhow::Result<()> {
    let domain = detect_document_domain(document)?;
    let out_path = output.cloned().unwrap_or_else(|| default_spec_for_document(document, &domain));

    match domain {
        SpecDomain::SchLib => {
            let lib = SchLib::open(document)
                .map_err(|e| anyhow::anyhow!("failed to open {}: {e}", document.display()))?;
            let spec_source = dump_schlib(&lib)
                .map_err(|e| anyhow::anyhow!("failed to dump {}: {e}", document.display()))?;
            std::fs::write(&out_path, &spec_source)
                .map_err(|e| anyhow::anyhow!("failed to write {}: {e}", out_path.display()))?;
            println!("Dumped: {} -> {}", document.display(), out_path.display());
        }
        SpecDomain::PcbLib => {
            let lib = PcbLib::open(document)
                .map_err(|e| anyhow::anyhow!("failed to open {}: {e}", document.display()))?;
            let spec_source = dump_pcblib(&lib);
            std::fs::write(&out_path, &spec_source)
                .map_err(|e| anyhow::anyhow!("failed to write {}: {e}", out_path.display()))?;
            println!("Dumped: {} -> {}", document.display(), out_path.display());
        }
        SpecDomain::SchDoc => {
            let doc = SchDoc::open(document)
                .map_err(|e| anyhow::anyhow!("failed to open {}: {e}", document.display()))?;
            let spec_source = dump_schdoc(&doc)
                .map_err(|e| anyhow::anyhow!("failed to dump {}: {e}", document.display()))?;
            std::fs::write(&out_path, &spec_source)
                .map_err(|e| anyhow::anyhow!("failed to write {}: {e}", out_path.display()))?;
            println!("Dumped: {} -> {}", document.display(), out_path.display());
        }
        SpecDomain::PrjPcb => {
            let doc = AltiumProject::open(document)
                .map_err(|e| anyhow::anyhow!("failed to open {}: {e}", document.display()))?;
            let spec_source = dump_prjpcb(&doc)
                .map_err(|e| anyhow::anyhow!("dump failed: {e}"))?;
            std::fs::write(&out_path, &spec_source)
                .map_err(|e| anyhow::anyhow!("failed to write {}: {e}", out_path.display()))?;
            println!("Dumped: {} -> {}", document.display(), out_path.display());
        }
        SpecDomain::PcbDoc => {
            let doc = PcbDoc::open(document)
                .map_err(|e| anyhow::anyhow!("failed to open {}: {e}", document.display()))?;
            let spec_source = dump_pcbdoc(&doc)
                .map_err(|e| anyhow::anyhow!("failed to dump {}: {e}", document.display()))?;
            std::fs::write(&out_path, &spec_source)
                .map_err(|e| anyhow::anyhow!("failed to write {}: {e}", out_path.display()))?;
            println!("Dumped: {} -> {}", document.display(), out_path.display());
        }
    }

    Ok(())
}

// ── compile helper ────────────────────────────────────────────────────────────

struct CompileResult {
    model: altium_format_spec::model::SpecModel,
    /// All import paths (bare + named) for --all processing.
    import_paths: Vec<PathBuf>,
}

fn compile_and_resolve(
    source: &str,
    spec_file: &PathBuf,
    domain: &SpecDomain,
) -> anyhow::Result<CompileResult> {
    use altium_format_spec::parser::parse_spec;

    let source_name = spec_file.display().to_string();
    let file = parse_spec(source)
        .map_err(|e| anyhow::anyhow!("{}", e.render(&source_name, source)))?;

    // Resolve imports: validates cycles, cross-domain rules, alias uniqueness,
    // and file existence. We do NOT merge bare imports into the root AST —
    // each file is compiled independently (reference semantics).
    let spec_path_canonical = spec_file.canonicalize().unwrap_or_else(|_| spec_file.clone());
    let resolved = resolve_imports(&spec_path_canonical, file.clone())
        .map_err(|e| anyhow::anyhow!("{}", e.render(&source_name, source)))?;

    // Compile only the root file's own items.
    let model = compile_spec(&file, *domain)
        .map_err(|e| anyhow::anyhow!("{}", e.render(&source_name, source)))?;

    // Collect all import paths for --all processing.
    let import_paths: Vec<PathBuf> = resolved
        .bare_imports
        .iter()
        .map(|(p, _)| p.clone())
        .chain(resolved.named_imports.values().map(|(p, _)| p.clone()))
        .collect();

    Ok(CompileResult { model, import_paths })
}

fn run_info(path: &std::path::Path, format: &str) -> anyhow::Result<()> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match ext.as_str() {
        "schdoc" => {
            let doc = SchDoc::open(path)?;
            let sheet = doc.sheet()?;
            run_info_schdoc(path, &sheet, format)
        }
        "schlib" => {
            let lib = SchLib::open(path)?;
            run_info_schlib(path, &lib, format)
        }
        "pcblib" => {
            let lib = PcbLib::open(path)?;
            run_info_pcblib(path, &lib, format)
        }
        "pcbdoc" => {
            let doc = PcbDoc::open(path)?;
            let board = doc.board()?;
            run_info_pcbdoc(path, &board, format)
        }
        _ => anyhow::bail!(
            "unsupported file type '.{ext}' for info (supported: .SchDoc, .SchLib, .PcbLib, .PcbDoc)"
        ),
    }
}

fn run_info_schdoc(
    path: &std::path::Path,
    sheet: &altium_format::api::SchDocSheet,
    format: &str,
) -> anyhow::Result<()> {
    use altium_format::api::SheetObject;
    use std::collections::BTreeSet;

    let mut components = 0u32;
    let mut wires = 0u32;
    let mut buses = 0u32;
    let mut net_labels = 0u32;
    let mut power_objects = 0u32;
    let mut ports = 0u32;
    let mut junctions = 0u32;
    let mut no_connects = 0u32;
    let mut bus_entries = 0u32;
    let mut sheet_symbols = 0u32;
    let mut notes = 0u32;
    let mut graphics = 0u32;
    let mut parameters = 0u32;
    let mut parameter_sets = 0u32;
    let mut probes = 0u32;
    let mut harness_connectors = 0u32;
    let mut signal_harnesses = 0u32;

    let mut net_names = BTreeSet::new();

    for obj in &sheet.objects {
        match obj {
            SheetObject::Component(_) => components += 1,
            SheetObject::Wire(_) => wires += 1,
            SheetObject::Bus(_) => buses += 1,
            SheetObject::NetLabel(n) => {
                net_labels += 1;
                net_names.insert(n.text.clone());
            }
            SheetObject::PowerObject(p) => {
                power_objects += 1;
                net_names.insert(p.text.clone());
            }
            SheetObject::Port(_) => ports += 1,
            SheetObject::Junction(_) => junctions += 1,
            SheetObject::NoConnect(_) => no_connects += 1,
            SheetObject::BusEntry(_) => bus_entries += 1,
            SheetObject::SheetSymbol(_) => sheet_symbols += 1,
            SheetObject::Note(_) => notes += 1,
            SheetObject::Graphic(_) => graphics += 1,
            SheetObject::Parameter(_) => parameters += 1,
            SheetObject::ParameterSet(_) => parameter_sets += 1,
            SheetObject::Probe(_) => probes += 1,
            SheetObject::CompileMask(_) => {}
            SheetObject::Blanket(_) => {}
            SheetObject::HarnessConnector(_) => harness_connectors += 1,
            SheetObject::SignalHarness(_) => signal_harnesses += 1,
        }
    }

    if format == "json" {
        let symbols_json: Vec<serde_json::Value> = sheet.sheet_symbols().iter().map(|s| {
            serde_json::json!({
                "file_name": s.file_name,
                "sheet_name": s.sheet_name,
            })
        }).collect();

        let info = serde_json::json!({
            "document": path.display().to_string(),
            "type": "Schematic Document",
            "objects": {
                "components": components,
                "wires": wires,
                "buses": buses,
                "net_labels": net_labels,
                "power_objects": power_objects,
                "ports": ports,
                "junctions": junctions,
                "no_connects": no_connects,
                "bus_entries": bus_entries,
                "sheet_symbols": sheet_symbols,
                "notes": notes,
                "graphics": graphics,
                "parameters": parameters,
                "parameter_sets": parameter_sets,
                "probes": probes,
                "harness_connectors": harness_connectors,
                "signal_harnesses": signal_harnesses,
            },
            "unique_nets": {
                "count": net_names.len(),
                "names": net_names.iter().collect::<Vec<_>>(),
            },
            "sheet_hierarchy": symbols_json,
        });
        println!("{}", serde_json::to_string_pretty(&info)?);
    } else {
        println!("Document: {}", path.display());
        println!("Type: Schematic Document");
        println!();
        println!("Objects:");
        // Only print non-zero counts to keep output clean
        let counts: &[(&str, u32)] = &[
            ("Components", components),
            ("Wires", wires),
            ("Buses", buses),
            ("Net Labels", net_labels),
            ("Power Objects", power_objects),
            ("Ports", ports),
            ("Junctions", junctions),
            ("No Connects", no_connects),
            ("Bus Entries", bus_entries),
            ("Sheet Symbols", sheet_symbols),
            ("Notes", notes),
            ("Graphics", graphics),
            ("Parameters", parameters),
            ("Parameter Sets", parameter_sets),
            ("Probes", probes),
            ("Harness Connectors", harness_connectors),
            ("Signal Harnesses", signal_harnesses),
        ];
        for (label, count) in counts {
            if *count > 0 {
                println!("  {label:20} {count:>5}");
            }
        }

        if !net_names.is_empty() {
            println!();
            println!("Unique Nets: {}", net_names.len());
            let names_vec: Vec<&String> = net_names.iter().collect();
            let preview: Vec<&str> = names_vec.iter().take(10).map(|s| s.as_str()).collect();
            let display = preview.join(", ");
            if net_names.len() > 10 {
                println!("  {display}, ...");
            } else {
                println!("  {display}");
            }
        }

        let symbols = sheet.sheet_symbols();
        if !symbols.is_empty() {
            println!();
            println!("Sheet Hierarchy:");
            for s in &symbols {
                println!("  {} ({})", s.file_name, s.sheet_name);
            }
        }
    }
    Ok(())
}

fn run_info_schlib(
    path: &std::path::Path,
    lib: &SchLib,
    format: &str,
) -> anyhow::Result<()> {
    let names = lib.component_names();
    if format == "json" {
        let info = serde_json::json!({
            "document": path.display().to_string(),
            "type": "Schematic Library",
            "component_count": names.len(),
            "components": names,
        });
        println!("{}", serde_json::to_string_pretty(&info)?);
    } else {
        println!("Document: {}", path.display());
        println!("Type: Schematic Library");
        println!();
        println!("Components: {}", names.len());
        for name in &names {
            println!("  {name}");
        }
    }
    Ok(())
}

fn run_info_pcblib(
    path: &std::path::Path,
    lib: &PcbLib,
    format: &str,
) -> anyhow::Result<()> {
    let names = lib.footprint_names();
    if format == "json" {
        let info = serde_json::json!({
            "document": path.display().to_string(),
            "type": "PCB Library",
            "footprint_count": names.len(),
            "footprints": names,
        });
        println!("{}", serde_json::to_string_pretty(&info)?);
    } else {
        println!("Document: {}", path.display());
        println!("Type: PCB Library");
        println!();
        println!("Footprints: {}", names.len());
        for name in &names {
            println!("  {name}");
        }
    }
    Ok(())
}

fn run_info_pcbdoc(
    path: &std::path::Path,
    board: &altium_format::api::PcbDocBoard,
    format: &str,
) -> anyhow::Result<()> {
    if format == "json" {
        let info = serde_json::json!({
            "document": path.display().to_string(),
            "type": "PCB Document",
            "board_name": board.settings.document_name,
            "signal_layer_count": board.settings.signal_layer_count,
            "display_unit": format!("{:?}", board.settings.display_unit),
            "nets": board.nets.len(),
            "components": board.components.len(),
            "tracks": board.tracks.len(),
            "arcs": board.arcs.len(),
            "vias": board.vias.len(),
            "pads": board.pads.len(),
            "fills": board.fills.len(),
            "texts": board.texts.len(),
            "regions": board.regions.len(),
            "component_bodies": board.component_bodies.len(),
            "polygons": board.polygons.len(),
            "rules": board.rules.len(),
            "classes": board.classes.len(),
            "dimensions": board.dimensions.len(),
            "differential_pairs": board.differential_pairs.len(),
            "net_names": board.nets.iter().map(|n| &n.name).collect::<Vec<_>>(),
            "component_designators": board.components.iter().map(|c| &c.designator).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&info)?);
    } else {
        println!("Document: {}", path.display());
        println!("Type: PCB Document");
        println!();
        println!("Board: {}", board.settings.document_name);
        println!("Signal Layers: {}", board.settings.signal_layer_count);
        println!("Display Unit: {:?}", board.settings.display_unit);
        println!();

        let counts: &[(&str, usize)] = &[
            ("Nets", board.nets.len()),
            ("Components", board.components.len()),
            ("Tracks", board.tracks.len()),
            ("Arcs", board.arcs.len()),
            ("Vias", board.vias.len()),
            ("Pads", board.pads.len()),
            ("Fills", board.fills.len()),
            ("Texts", board.texts.len()),
            ("Regions", board.regions.len()),
            ("Component Bodies", board.component_bodies.len()),
            ("Polygons", board.polygons.len()),
            ("Rules", board.rules.len()),
            ("Classes", board.classes.len()),
            ("Dimensions", board.dimensions.len()),
            ("Differential Pairs", board.differential_pairs.len()),
        ];
        for (label, count) in counts {
            if *count > 0 {
                println!("  {label:<20} {count:>5}");
            }
        }
    }
    Ok(())
}

fn run_query(
    path: &std::path::Path,
    query_str: &str,
    format: &str,
    limit: Option<usize>,
) -> anyhow::Result<()> {
    // Parse the query
    let query = parse_query(query_str).map_err(|e| {
        anyhow::anyhow!("{}", e.render(query_str))
    })?;

    // Open the document based on file extension
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let results = match ext.as_str() {
        "schlib" => {
            let lib = SchLib::open(path)?;
            eval_query(&query, &lib)
                .map_err(|e| anyhow::anyhow!("{}", e.render(query_str)))?
        }
        "pcblib" => {
            let lib = PcbLib::open(path)?;
            eval_query(&query, &lib)
                .map_err(|e| anyhow::anyhow!("{}", e.render(query_str)))?
        }
        "schdoc" => {
            let doc = SchDoc::open(path)?;
            eval_query(&query, &doc)
                .map_err(|e| anyhow::anyhow!("{}", e.render(query_str)))?
        }
        "pcbdoc" => {
            let doc = PcbDoc::open(path)?;
            eval_query(&query, &doc)
                .map_err(|e| anyhow::anyhow!("{}", e.render(query_str)))?
        }
        _ => {
            anyhow::bail!(
                "unsupported file type '.{ext}' for query (supported: .SchLib, .PcbLib, .SchDoc, .PcbDoc)"
            );
        }
    };

    // Apply limit
    let results: Vec<_> = match limit {
        Some(n) => results.into_iter().take(n).collect(),
        None => results,
    };

    // Format output
    match format {
        "count" => {
            println!("{}", results.len());
        }
        "json" => {
            let json_results: Vec<serde_json::Value> = results
                .iter()
                .map(|m| {
                    serde_json::json!({
                        "type": format!("{:?}", m.node.type_selector()),
                        "name": m.node.display_name(),
                        "path": m.path,
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&json_results)?);
        }
        "text" | _ => {
            if results.is_empty() {
                println!("No matches.");
            } else {
                println!("{} match{}:", results.len(), if results.len() == 1 { "" } else { "es" });
                for m in &results {
                    let type_name = format!("{:?}", m.node.type_selector());
                    println!("  [{type_name}] {}", m.node.display_name());
                }
            }
        }
    }

    Ok(())
}

fn run_inspect(path: &std::path::Path, sub: InspectSubcommand) -> anyhow::Result<()> {
    let doc = PcbDoc::open(path)?;
    let board = doc.board()?;
    let ir = PcbIr::extract(&board).map_err(|e| anyhow::anyhow!("{e}"))?;

    match sub {
        InspectSubcommand::Summary => {
            let b = &ir.board.bounds;
            println!("Board: {:.2} x {:.2} mm", b.width(), b.height());
            println!(
                "  Origin: ({:.2}, {:.2}) mm",
                b.min.x, b.min.y
            );
            println!("  Layers: {} copper", ir.layer_stack.copper_layer_count);
            println!("  Components: {}", ir.components.len());
            println!("  Nets: {}", ir.nets.len());
            println!("  Rules: {}", ir.rules.len());
            println!("  Polygons: {}", ir.polygons.len());
            println!(
                "  Free copper: {} tracks, {} vias, {} fills",
                ir.free_copper.tracks.len(),
                ir.free_copper.vias.len(),
                ir.free_copper.fills.len()
            );
        }
        InspectSubcommand::Components => {
            println!(
                "{:<12} {:<20} {:<10} {:>10} {:>10} {:>8} {:>5}",
                "DESIGNATOR", "PATTERN", "SIDE", "X (mm)", "Y (mm)", "ROT", "PADS"
            );
            println!("{}", "-".repeat(79));
            for (_id, comp) in ir.components.iter() {
                let side = match comp.side {
                    autopcb_ir::BoardSide::Top => "Top",
                    autopcb_ir::BoardSide::Bottom => "Bot",
                };
                println!(
                    "{:<12} {:<20} {:<10} {:>10.2} {:>10.2} {:>8.1} {:>5}",
                    comp.designator,
                    comp.pattern,
                    side,
                    comp.position.x,
                    comp.position.y,
                    comp.rotation,
                    comp.pads.len()
                );
            }
            println!("\n{} components total", ir.components.len());
        }
        InspectSubcommand::Nets => {
            println!(
                "{:<30} {:>6} {:>6}",
                "NET NAME", "PINS", "COMPS"
            );
            println!("{}", "-".repeat(44));
            for (_id, net) in ir.nets.iter() {
                println!(
                    "{:<30} {:>6} {:>6}",
                    net.name, net.pins.len(), net.component_count
                );
            }
            println!("\n{} nets total", ir.nets.len());
        }
        InspectSubcommand::BoardOutline => {
            println!("Board outline ({} points):", ir.board.outline.len());
            for (i, p) in ir.board.outline.iter().enumerate() {
                println!("  [{:4}] ({:.4}, {:.4}) mm", i, p.x, p.y);
            }
            if !ir.board.cutouts.is_empty() {
                println!("\n{} cutouts:", ir.board.cutouts.len());
                for (ci, cutout) in ir.board.cutouts.iter().enumerate() {
                    println!("  Cutout {} ({} points)", ci, cutout.len());
                }
            }
        }
        InspectSubcommand::Rules => {
            println!(
                "{:<30} {:<25} {:>5} {:>7}",
                "RULE NAME", "KIND", "PRI", "ENABLED"
            );
            println!("{}", "-".repeat(70));
            for (_id, rule) in ir.rules.iter() {
                println!(
                    "{:<30} {:<25} {:>5} {:>7}",
                    rule.name,
                    format!("{:?}", rule.kind),
                    rule.priority,
                    if rule.enabled { "yes" } else { "no" }
                );
            }
            println!("\n{} rules total", ir.rules.len());
        }
        InspectSubcommand::IrJson => {
            let json = serde_json::to_string_pretty(&ir)?;
            println!("{json}");
        }
    }

    Ok(())
}
