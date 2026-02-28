use std::path::PathBuf;
use std::process::ExitCode;

use altium_format::{AltiumProject, IntLib, PcbDoc, PcbLib, SchDoc, SchLib, VersionInfo};
use altium_format_query::{eval_query, parse_query};
use altium_format_spec::{
    SpecDomain, compile_spec, dump_pcblib, dump_schlib, reconcile_pcblib, reconcile_pcblib_empty,
    reconcile_schlib, reconcile_schlib_empty, resolve_imports, apply_spec_schlib, apply_spec_pcblib,
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
        /// Path to the spec file (.schlib-spec or .pcblib-spec)
        spec_file: PathBuf,
        /// Existing document to reconcile against (optional)
        #[arg(long)]
        target: Option<PathBuf>,
        /// Output ECO as JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Apply a spec file to create or update an Altium document
    Apply {
        /// Path to the spec file (.schlib-spec or .pcblib-spec)
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
    },
    /// Reverse-generate a spec file from an existing Altium document
    Dump {
        /// Path to the document (.SchLib or .PcbLib)
        document: PathBuf,
        /// Output spec file path (overrides default)
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Query entities in an Altium document using AQL (Altium Query Language)
    Query {
        /// Path to the document (.SchLib)
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
        Commands::Plan { spec_file, target, json } => {
            match run_plan(&spec_file, target.as_ref(), json) {
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
        Commands::Apply { spec_file, target, output, report_json } => {
            if let Err(e) = run_apply(&spec_file, target.as_ref(), output.as_ref(), report_json) {
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
            let lib = SchLib::new_blank_ad26();
            lib.save(output.as_path())?;
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
        _ => anyhow::bail!(
            "save-as not yet supported for .{ext} files (supported: .schdoc, .schlib, .pcblib)"
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
            anyhow::bail!("IntLib validation is not implemented yet");
        }
        "prjpcb" => {
            let _doc = AltiumProject::open(path)?;
            anyhow::bail!("AltiumProject validation is not implemented yet");
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
        Some("pcblib-spec") => Ok(SpecDomain::PcbLib),
        Some(ext) => anyhow::bail!("unknown spec file extension .{ext} (supported: .schlib-spec, .pcblib-spec)"),
        None => anyhow::bail!("spec file has no extension: {}", path.display()),
    }
}

fn detect_document_domain(path: &PathBuf) -> anyhow::Result<SpecDomain> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "schlib" => Ok(SpecDomain::SchLib),
        "pcblib" => Ok(SpecDomain::PcbLib),
        _ => anyhow::bail!("unknown document extension .{ext} (supported: .schlib, .pcblib)"),
    }
}

fn default_output_for_spec(spec_file: &PathBuf, domain: &SpecDomain) -> PathBuf {
    let stem = spec_file.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
    let ext = match domain {
        SpecDomain::SchLib => "SchLib",
        SpecDomain::PcbLib => "PcbLib",
    };
    spec_file.with_file_name(format!("{stem}.{ext}"))
}

fn default_spec_for_document(doc: &PathBuf, domain: &SpecDomain) -> PathBuf {
    let stem = doc.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
    let ext = match domain {
        SpecDomain::SchLib => "schlib-spec",
        SpecDomain::PcbLib => "pcblib-spec",
    };
    doc.with_file_name(format!("{stem}.{ext}"))
}

// ── plan ──────────────────────────────────────────────────────────────────────

/// Run `altium plan`. Returns Ok(true) if changes exist, Ok(false) if no changes.
fn run_plan(spec_file: &PathBuf, target: Option<&PathBuf>, json: bool) -> anyhow::Result<bool> {
    let domain = detect_spec_domain(spec_file)?;
    let source = std::fs::read_to_string(spec_file)
        .map_err(|e| anyhow::anyhow!("failed to read spec file {}: {e}", spec_file.display()))?;

    let spec_model = compile_and_resolve(&source, spec_file, &domain)?;
    let library_path = default_output_for_spec(spec_file, &domain);
    let spec_path = spec_file.clone();

    let eco = match spec_model {
        altium_format_spec::model::SpecModel::SchLib(ref spec_lib) => {
            // Try to load target document if it exists
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
        altium_format_spec::model::SpecModel::PcbLib(ref spec_lib) => {
            let resolved_target = target.cloned().unwrap_or_else(|| library_path.clone());
            if resolved_target.exists() {
                reconcile_pcblib(spec_lib, library_path, spec_path)
            } else {
                reconcile_pcblib_empty(spec_lib, library_path, spec_path)
            }
        }
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&eco)?);
    } else {
        println!("{}", eco.render_text());
    }

    let has_changes = eco.summary.by_kind.values()
        .any(|k| k.adds > 0 || k.updates > 0);
    Ok(has_changes)
}

// ── apply ─────────────────────────────────────────────────────────────────────

fn run_apply(
    spec_file: &PathBuf,
    target: Option<&PathBuf>,
    output: Option<&PathBuf>,
    _report_json: bool,
) -> anyhow::Result<()> {
    let domain = detect_spec_domain(spec_file)?;
    let source = std::fs::read_to_string(spec_file)
        .map_err(|e| anyhow::anyhow!("failed to read spec file {}: {e}", spec_file.display()))?;

    let spec_model = compile_and_resolve(&source, spec_file, &domain)?;
    let library_path = default_output_for_spec(spec_file, &domain);

    match spec_model {
        altium_format_spec::model::SpecModel::SchLib(ref spec_lib) => {
            let resolved_target = target.cloned().unwrap_or_else(|| library_path.clone());
            let mut doc = if resolved_target.exists() {
                SchLib::open(&resolved_target)
                    .map_err(|e| anyhow::anyhow!("failed to open {}: {e}", resolved_target.display()))?
            } else {
                let mut lib = SchLib::new_blank_ad26();
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
        altium_format_spec::model::SpecModel::PcbLib(ref spec_lib) => {
            let resolved_target = target.cloned().unwrap_or_else(|| library_path.clone());
            let mut lib = if resolved_target.exists() {
                PcbLib::open(&resolved_target)
                    .map_err(|e| anyhow::anyhow!("failed to open {}: {e}", resolved_target.display()))?
            } else {
                anyhow::bail!(
                    "no existing PcbLib found at {} and PcbLib::new_blank_ad26 is not yet implemented; \
                     provide an existing library via --target",
                    resolved_target.display()
                )
            };

            let out_path = output.cloned().unwrap_or(library_path);

            apply_spec_pcblib(spec_lib, &mut lib)
                .map_err(|e| anyhow::anyhow!("apply failed: {e}"))?;

            lib.save(&out_path)?;
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
            let spec_source = dump_schlib(&lib);
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
    }

    Ok(())
}

// ── compile helper ────────────────────────────────────────────────────────────

fn compile_and_resolve(
    source: &str,
    spec_file: &PathBuf,
    domain: &SpecDomain,
) -> anyhow::Result<altium_format_spec::model::SpecModel> {
    use altium_format_spec::parser::parse_spec;

    let file = parse_spec(source)
        .map_err(|e| anyhow::anyhow!("parse error in {}: {e}", spec_file.display()))?;

    // Resolve imports from the directory containing the spec file.
    let spec_dir = spec_file.parent().unwrap_or_else(|| std::path::Path::new("."));
    let spec_path_canonical = spec_file.canonicalize().unwrap_or_else(|_| spec_file.clone());
    let resolved = resolve_imports(&spec_path_canonical, file)
        .map_err(|e| anyhow::anyhow!("import error in {}: {e}", spec_file.display()))?;

    // Merge bare imports into root: collect all items from bare imports + root.
    let mut merged_items = Vec::new();
    for (_path, bare_file) in resolved.bare_imports {
        merged_items.extend(bare_file.items);
    }
    merged_items.extend(resolved.root.items);

    let merged_file = altium_format_spec::ast::SpecFile { items: merged_items };

    let _ = spec_dir; // used implicitly via spec_path_canonical
    compile_spec(&merged_file, *domain)
        .map_err(|e| anyhow::anyhow!("compile error in {}: {e}", spec_file.display()))
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
        // Future: PcbLib, SchDoc, PcbDoc support
        _ => {
            anyhow::bail!(
                "unsupported file type '.{ext}' for query (supported: .SchLib)"
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
