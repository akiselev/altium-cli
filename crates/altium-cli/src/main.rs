use std::path::PathBuf;
use std::process::ExitCode;

use altium_format::{AltiumProject, IntLib, PcbDoc, PcbLib, SchDoc, SchLib, VersionInfo};
use altium_format_query::{eval_query, parse_query};
use altium_format_render_png::{
    DEFAULT_SCALE, render_pcblib_footprint_png, render_schdoc_png, render_schlib_component_png,
};
use altium_format_render_svg::{render_pcblib_footprint, render_schdoc, render_schlib_component};
use altium_format_spec::{
    FormatConfig, SpecDomain, SyncChange, SyncDirection, SyncPolicy, apply_spec_pcbdoc,
    apply_spec_pcblib, apply_spec_prjpcb, apply_spec_schdoc, apply_spec_schlib,
    apply_sync_changes_to_pcbdoc, compile_imported_schlibs, compile_spec_with_resolved,
    diff_snapshots, dump_intlib, dump_pcbdoc, dump_pcblib, dump_prjpcb, dump_schdoc, dump_schlib,
    filter_changes, format_spec, project_pcbdoc_spec, project_schdoc_spec, reconcile_pcbdoc,
    reconcile_pcbdoc_empty, reconcile_pcblib, reconcile_pcblib_empty, reconcile_prjpcb,
    reconcile_prjpcb_empty, reconcile_schdoc, reconcile_schdoc_empty, reconcile_schlib,
    reconcile_schlib_empty, render_eco_report, resolve_imports, rewrite_pcbdoc_spec_with_changes,
    validate_pcbdoc_spec, validate_schdoc_spec,
};
use clap::{Parser, Subcommand};

mod cfb;
pub mod spec_merge;

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
        /// Path to the spec file (.schlib-spec, .pcblib-spec, .schdoc-spec, .pcbdoc-spec, or .prjpcb-spec)
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
        /// Path to the spec file (.schlib-spec, .pcblib-spec, .schdoc-spec, .pcbdoc-spec, or .prjpcb-spec)
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
    /// Format spec files (*.schlib-spec, *.pcblib-spec, *.schdoc-spec, *.pcbdoc-spec, *.prjpcb-spec)
    Format {
        /// Spec files to format (reads from stdin if none given)
        files: Vec<PathBuf>,
        /// Check formatting without writing (exit code 1 if changes needed)
        #[arg(long)]
        check: bool,
        /// Write output to stdout instead of modifying files in-place
        #[arg(long)]
        stdout: bool,
    },
    /// Spec-to-spec synchronization commands
    Spec {
        #[command(subcommand)]
        sub: SpecSubcommand,
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
enum SpecSubcommand {
    /// Synchronize SchDoc-spec and PcbDoc-spec files
    Sync {
        /// Path to the .schdoc-spec source file
        schdoc_spec: PathBuf,
        /// Path to the .pcbdoc-spec target file
        pcbdoc_spec: PathBuf,
        /// Forward sync: apply SchDoc changes to PcbDoc
        #[arg(long, conflicts_with = "diff")]
        forward: bool,
        /// Diff only: show changes without applying
        #[arg(long, conflicts_with = "forward")]
        diff: bool,
        /// Show changes without writing to disk
        #[arg(long)]
        dry_run: bool,
        /// Append mode: only add new components/nets, never remove existing ones.
        /// Use when syncing multiple schematic sheets into one PcbDoc.
        #[arg(long)]
        append: bool,
    },
}

fn main() -> ExitCode {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init();

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
        Commands::Plan {
            spec_file,
            target,
            json,
            all,
        } => match run_plan(&spec_file, target.as_ref(), json, all) {
            Ok(has_changes) => {
                if has_changes {
                    return ExitCode::from(1);
                }
            }
            Err(e) => {
                eprintln!("Error: {e}");
                return ExitCode::FAILURE;
            }
        },
        Commands::Apply {
            spec_file,
            target,
            output,
            report_json,
            all,
        } => {
            if let Err(e) = run_apply(
                &spec_file,
                target.as_ref(),
                output.as_ref(),
                report_json,
                all,
            ) {
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
        Commands::Query {
            path,
            query,
            format,
            limit,
        } => {
            if let Err(e) = run_query(&path, &query, &format, limit) {
                eprintln!("Error: {e}");
                return ExitCode::FAILURE;
            }
        }
        Commands::Format {
            files,
            check,
            stdout,
        } => match run_format(files, check, stdout) {
            Ok(code) => return code,
            Err(e) => {
                eprintln!("Error: {e}");
                return ExitCode::FAILURE;
            }
        },
        Commands::Spec { sub } => {
            if let Err(e) = run_spec(sub) {
                eprintln!("Error: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    ExitCode::SUCCESS
}

fn run_spec(sub: SpecSubcommand) -> anyhow::Result<()> {
    match sub {
        SpecSubcommand::Sync {
            schdoc_spec,
            pcbdoc_spec,
            forward,
            diff,
            dry_run,
            append,
        } => run_spec_sync(&schdoc_spec, &pcbdoc_spec, forward, diff, dry_run, append),
    }
}

fn run_spec_sync(
    schdoc_spec_path: &PathBuf,
    pcbdoc_spec_path: &PathBuf,
    forward: bool,
    diff_only: bool,
    dry_run: bool,
    append: bool,
) -> anyhow::Result<()> {
    if !forward && !diff_only {
        anyhow::bail!("specify --forward or --diff");
    }

    // ── Step 1: Read both spec files ─────────────────────────────────────────

    let schdoc_source = std::fs::read_to_string(schdoc_spec_path)
        .map_err(|e| anyhow::anyhow!("reading {}: {e}", schdoc_spec_path.display()))?;
    let pcbdoc_source = std::fs::read_to_string(pcbdoc_spec_path)
        .map_err(|e| anyhow::anyhow!("reading {}: {e}", pcbdoc_spec_path.display()))?;

    // ── Step 2: Parse and compile both specs ──────────────────────────────────

    let schdoc_result = compile_and_resolve(&schdoc_source, schdoc_spec_path, &SpecDomain::SchDoc)?;
    let pcbdoc_result = compile_and_resolve(&pcbdoc_source, pcbdoc_spec_path, &SpecDomain::PcbDoc)?;

    let schdoc_spec = match schdoc_result.model {
        altium_format_spec::model::SpecModel::SchDoc(spec) => spec,
        _ => anyhow::bail!(
            "{} is not a valid .schdoc-spec file",
            schdoc_spec_path.display()
        ),
    };
    let pcbdoc_spec_model = match pcbdoc_result.model {
        altium_format_spec::model::SpecModel::PcbDoc(spec) => spec,
        _ => anyhow::bail!(
            "{} is not a valid .pcbdoc-spec file",
            pcbdoc_spec_path.display()
        ),
    };

    // ── Step 3: Validate both specs ───────────────────────────────────────────

    let schdoc_name = schdoc_spec_path.display().to_string();
    let pcbdoc_name = pcbdoc_spec_path.display().to_string();

    match validate_schdoc_spec(&schdoc_spec) {
        Ok(warnings) => {
            for w in &warnings {
                eprintln!("{}", w.render(&schdoc_name, &schdoc_source));
            }
        }
        Err(errors) => {
            let msgs: Vec<String> = errors
                .iter()
                .map(|e| e.render(&schdoc_name, &schdoc_source))
                .collect();
            anyhow::bail!("validation errors in {}: {}", schdoc_name, msgs.join("; "));
        }
    }

    match validate_pcbdoc_spec(&pcbdoc_spec_model) {
        Ok(warnings) => {
            for w in &warnings {
                eprintln!("{}", w.render(&pcbdoc_name, &pcbdoc_source));
            }
        }
        Err(errors) => {
            let msgs: Vec<String> = errors
                .iter()
                .map(|e| e.render(&pcbdoc_name, &pcbdoc_source))
                .collect();
            anyhow::bail!("validation errors in {}: {}", pcbdoc_name, msgs.join("; "));
        }
    }

    // ── Step 4: Project both specs to SyncSnapshot ────────────────────────────

    let schdoc_snapshot = project_schdoc_spec(&schdoc_spec, &schdoc_result.imported_components)
        .map_err(|e| {
            anyhow::anyhow!(
                "projecting {}: {}",
                schdoc_name,
                e.render(&schdoc_name, &schdoc_source)
            )
        })?;
    let pcbdoc_snapshot = project_pcbdoc_spec(&pcbdoc_spec_model).map_err(|e| {
        anyhow::anyhow!(
            "projecting {}: {}",
            pcbdoc_name,
            e.render(&pcbdoc_name, &pcbdoc_source)
        )
    })?;

    // ── Step 5: Diff snapshots ────────────────────────────────────────────────

    let changes = diff_snapshots(&schdoc_snapshot, &pcbdoc_snapshot);

    // ── Step 6: Filter changes with explicit SyncPolicy ───────────────────────

    let policy = SyncPolicy {
        comment: SyncDirection::Forward,
        footprint: SyncDirection::Forward,
        source_library: SyncDirection::Forward,
        parameters: SyncDirection::Forward,
        net_name: SyncDirection::Forward,
        net_color: SyncDirection::None,
        pin_net_assignment: SyncDirection::None,
        component_location: SyncDirection::None,
    };

    let mut filtered = filter_changes(&changes, &policy, SyncDirection::Forward)
        .map_err(|e| anyhow::anyhow!("filtering sync changes: {}", e.message))?;

    // In append mode, drop all Remove* changes so multi-sheet syncs don't
    // clobber components/nets from previously synced sheets.
    if append {
        filtered.retain(|change| {
            !matches!(
                change,
                SyncChange::RemoveComponent { .. } | SyncChange::RemoveNet { .. }
            )
        });
    }

    // ── Step 7: Print ECO report ──────────────────────────────────────────────

    let report = render_eco_report(&filtered);
    print!("{}", report);

    if diff_only || dry_run {
        return Ok(());
    }

    // ── Step 8: Apply changes and write-back ──────────────────────────────────

    if filtered.is_empty() {
        return Ok(());
    }

    // Apply changes to the in-memory PcbDocSpec model (for validation).
    let mut pcbdoc_spec_mut = pcbdoc_spec_model;
    apply_sync_changes_to_pcbdoc(&filtered, &mut pcbdoc_spec_mut)
        .map_err(|e| anyhow::anyhow!("applying sync changes: {}", e.message))?;

    // Rewrite the source text with the changes.
    let new_source = rewrite_pcbdoc_spec_with_changes(&pcbdoc_source, &filtered)
        .map_err(|e| anyhow::anyhow!("rewriting {}: {}", pcbdoc_name, e.message))?;

    let format_result = format_spec(&new_source, &FormatConfig::default())
        .map_err(|e| anyhow::anyhow!("formatting {}: {e}", pcbdoc_name))?;
    let new_source = format_result.output;

    // ── Step 9: Atomic write ──────────────────────────────────────────────────

    let pcbdoc_dir = pcbdoc_spec_path
        .parent()
        .unwrap_or(std::path::Path::new("."));
    let tmp_path = pcbdoc_dir.join(format!(
        ".{}.tmp",
        pcbdoc_spec_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("pcb")
    ));

    std::fs::write(&tmp_path, &new_source)
        .map_err(|e| anyhow::anyhow!("writing temp file {}: {e}", tmp_path.display()))?;

    std::fs::rename(&tmp_path, pcbdoc_spec_path).map_err(|e| {
        // Try to clean up temp file on failure; ignore errors.
        let _ = std::fs::remove_file(&tmp_path);
        anyhow::anyhow!("renaming {} -> {}: {e}", tmp_path.display(), pcbdoc_name)
    })?;

    eprintln!("Updated: {}", pcbdoc_name);

    Ok(())
}

fn run_format(files: Vec<PathBuf>, check: bool, to_stdout: bool) -> anyhow::Result<ExitCode> {
    let config = FormatConfig::default();

    if files.is_empty() {
        // stdin → stdout mode
        let mut source = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut source)?;
        let result = format_spec(&source, &config)
            .map_err(|e| anyhow::anyhow!("{}", e.render("<stdin>", &source)))?;
        print!("{}", result.output);
        return Ok(ExitCode::SUCCESS);
    }

    let mut any_changed = false;
    for path in &files {
        let source = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("reading {}: {e}", path.display()))?;
        let file_name = path.display().to_string();
        let result = format_spec(&source, &config)
            .map_err(|e| anyhow::anyhow!("{}", e.render(&file_name, &source)))?;

        if check {
            if result.changed {
                eprintln!("{}", path.display());
                any_changed = true;
            }
        } else if to_stdout {
            print!("{}", result.output);
        } else if result.changed {
            std::fs::write(path, &result.output)
                .map_err(|e| anyhow::anyhow!("writing {}: {e}", path.display()))?;
            eprintln!("formatted {}", path.display());
        }
    }

    if check && any_changed {
        Ok(ExitCode::from(1))
    } else {
        Ok(ExitCode::SUCCESS)
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
            let lib = IntLib::open(path)?;
            println!(
                "  {} SchLib(s), {} PcbLib(s)",
                lib.schlibs().len(),
                lib.pcblibs().len()
            );
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
        Some("pcblib-spec") => Ok(SpecDomain::PcbLib),
        Some("schdoc-spec") => Ok(SpecDomain::SchDoc),
        Some("pcbdoc-spec") => Ok(SpecDomain::PcbDoc),
        Some("prjpcb-spec") => Ok(SpecDomain::PrjPcb),
        Some(ext) => {
            anyhow::bail!(
                "unknown spec file extension .{ext} (supported: .schlib-spec, .pcblib-spec, .schdoc-spec, .pcbdoc-spec, .prjpcb-spec)"
            )
        }
        None => anyhow::bail!("spec file has no extension: {}", path.display()),
    }
}

fn detect_document_domain(path: &PathBuf) -> anyhow::Result<SpecDomain> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "schlib" => Ok(SpecDomain::SchLib),
        "pcblib" => Ok(SpecDomain::PcbLib),
        "schdoc" => Ok(SpecDomain::SchDoc),
        "prjpcb" => Ok(SpecDomain::PrjPcb),
        "pcbdoc" => Ok(SpecDomain::PcbDoc),
        _ => anyhow::bail!(
            "unknown document extension .{ext} (supported: .schlib, .schdoc, .pcblib, .prjpcb, .pcbdoc)"
        ),
    }
}

fn default_output_for_spec(spec_file: &PathBuf, domain: &SpecDomain) -> PathBuf {
    let stem = spec_file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let ext = match domain {
        SpecDomain::SchLib => "SchLib",
        SpecDomain::PcbLib => "PcbLib",
        SpecDomain::SchDoc => "SchDoc",
        SpecDomain::PcbDoc => "PcbDoc",
        SpecDomain::PrjPcb => "PrjPcb",
    };
    spec_file.with_file_name(format!("{stem}.{ext}"))
}

fn default_spec_for_document(doc: &PathBuf, domain: &SpecDomain) -> PathBuf {
    let stem = doc.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
    let ext = match domain {
        SpecDomain::SchLib => "schlib-spec",
        SpecDomain::PcbLib => "pcblib-spec",
        SpecDomain::SchDoc => "schdoc-spec",
        SpecDomain::PcbDoc => "pcbdoc-spec",
        SpecDomain::PrjPcb => "prjpcb-spec",
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
    let mut has_changes = eco
        .summary
        .by_kind
        .values()
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
            has_changes |= eco
                .summary
                .by_kind
                .values()
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
                let doc = SchLib::open(&resolved_target).map_err(|e| {
                    anyhow::anyhow!("failed to open {}: {e}", resolved_target.display())
                })?;
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
                let doc = AltiumProject::open(&resolved_target).map_err(|e| {
                    anyhow::anyhow!("failed to open {}: {e}", resolved_target.display())
                })?;
                reconcile_prjpcb(spec, &doc, library_path, spec_path)
                    .map_err(|e| anyhow::anyhow!("reconcile failed: {e}"))?
            } else {
                reconcile_prjpcb_empty(spec, library_path, spec_path)
            }
        }
        altium_format_spec::model::SpecModel::SchDoc(spec) => {
            let resolved_target = target.cloned().unwrap_or_else(|| library_path.clone());
            if resolved_target.exists() {
                let doc = SchDoc::open(&resolved_target).map_err(|e| {
                    anyhow::anyhow!("failed to open {}: {e}", resolved_target.display())
                })?;
                reconcile_schdoc(spec, &doc, library_path, spec_path)
                    .map_err(|e| anyhow::anyhow!("reconcile failed: {e}"))?
            } else {
                reconcile_schdoc_empty(spec, library_path, spec_path)
            }
        }
        altium_format_spec::model::SpecModel::PcbDoc(spec) => {
            let resolved_target = target.cloned().unwrap_or_else(|| library_path.clone());
            if resolved_target.exists() {
                let doc = PcbDoc::open(&resolved_target).map_err(|e| {
                    anyhow::anyhow!("failed to open {}: {e}", resolved_target.display())
                })?;
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
    apply_for_model(
        &result.model,
        target,
        output,
        spec_file,
        &domain,
        &result.imported_components,
        &result.import_paths,
    )?;

    // Apply imports with --all.
    if all {
        for import_path in &result.import_paths {
            let import_domain = detect_spec_domain(import_path)?;
            let import_source = std::fs::read_to_string(import_path)
                .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", import_path.display()))?;
            let import_result = compile_and_resolve(&import_source, import_path, &import_domain)?;
            apply_for_model(
                &import_result.model,
                None,
                None,
                import_path,
                &import_domain,
                &import_result.imported_components,
                &import_result.import_paths,
            )?;
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
    imported_components: &std::collections::HashMap<
        String,
        altium_format_spec::model::ComponentSpec,
    >,
    import_paths: &[PathBuf],
) -> anyhow::Result<()> {
    let library_path = default_output_for_spec(spec_file, domain);

    match spec_model {
        altium_format_spec::model::SpecModel::SchLib(spec_lib) => {
            let resolved_target = target.cloned().unwrap_or_else(|| library_path.clone());
            let mut doc = if resolved_target.exists() {
                SchLib::open(&resolved_target).map_err(|e| {
                    anyhow::anyhow!("failed to open {}: {e}", resolved_target.display())
                })?
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
            let mut doc = if resolved_target.exists() {
                PcbLib::open(&resolved_target).map_err(|e| {
                    anyhow::anyhow!("failed to open {}: {e}", resolved_target.display())
                })?
            } else {
                PcbLib::new_blank_ad26()?
            };

            let out_path = output.cloned().unwrap_or(library_path);

            apply_spec_pcblib(spec_lib, &mut doc)
                .map_err(|e| anyhow::anyhow!("apply failed: {e}"))?;

            doc.save(&out_path)?;
            println!("Saved: {}", out_path.display());
        }
        altium_format_spec::model::SpecModel::PrjPcb(spec) => {
            let resolved_target = target.cloned().unwrap_or_else(|| library_path.clone());
            let mut doc = if resolved_target.exists() {
                AltiumProject::open(&resolved_target).map_err(|e| {
                    anyhow::anyhow!("failed to open {}: {e}", resolved_target.display())
                })?
            } else {
                AltiumProject::new_blank_ad26()
            };

            let out_path = output.cloned().unwrap_or(library_path);

            apply_spec_prjpcb(spec, &mut doc).map_err(|e| anyhow::anyhow!("apply failed: {e}"))?;

            doc.save(&out_path)?;
            println!("Saved: {}", out_path.display());
        }
        altium_format_spec::model::SpecModel::SchDoc(spec) => {
            let resolved_target = target.cloned().unwrap_or_else(|| library_path.clone());
            let mut doc = if resolved_target.exists() {
                SchDoc::open(&resolved_target).map_err(|e| {
                    anyhow::anyhow!("failed to open {}: {e}", resolved_target.display())
                })?
            } else {
                SchDoc::new_blank_ad26()
            };

            let out_path = output.cloned().unwrap_or(library_path);

            apply_spec_schdoc(spec, &mut doc, imported_components)
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
            let mut doc = PcbDoc::open(&resolved_target).map_err(|e| {
                anyhow::anyhow!("failed to open {}: {e}", resolved_target.display())
            })?;

            let out_path = output.cloned().unwrap_or(library_path);

            apply_spec_pcbdoc(spec, &mut doc).map_err(|e| anyhow::anyhow!("apply failed: {e}"))?;

            let pad_net_map = build_pad_net_map(spec_file)?;
            instantiate_footprint_primitives(
                &mut doc,
                import_paths,
                &pad_net_map,
                imported_components,
            )?;

            doc.save(&out_path)?;
            println!("Saved: {}", out_path.display());
        }
    }

    Ok(())
}

// ── footprint primitive instantiation ────────────────────────────────────────

/// Discovers sibling `.schdoc-spec` files and builds a pad-to-net map.
///
/// The map keys are `(component_designator, pad_designator)` → `net_name`.
/// Uses `project_schdoc_spec()` (with pin→pad resolution) to resolve pin names
/// to pad designators via SchLib data.
fn build_pad_net_map(
    spec_file: &PathBuf,
) -> anyhow::Result<std::collections::HashMap<(String, String), String>> {
    let mut pad_net_map = std::collections::HashMap::new();

    let spec_dir = spec_file.parent().unwrap_or(std::path::Path::new("."));
    let schdoc_specs: Vec<PathBuf> = std::fs::read_dir(spec_dir)?
        .filter_map(|entry| entry.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("schdoc-spec"))
                .unwrap_or(false)
        })
        .collect();

    for schdoc_path in &schdoc_specs {
        let source = std::fs::read_to_string(schdoc_path)
            .map_err(|e| anyhow::anyhow!("reading {}: {e}", schdoc_path.display()))?;

        let domain = SpecDomain::SchDoc;
        let result = compile_and_resolve(&source, schdoc_path, &domain)?;

        let schdoc_spec = match result.model {
            altium_format_spec::model::SpecModel::SchDoc(spec) => spec,
            _ => continue,
        };

        let snapshot =
            project_schdoc_spec(&schdoc_spec, &result.imported_components).map_err(|e| {
                let name = schdoc_path.display().to_string();
                anyhow::anyhow!("projecting {}: {}", name, e.render(&name, &source))
            })?;

        for (comp_des, comp) in &snapshot.components {
            for (pad_des, pin) in &comp.pins {
                if let Some(net) = &pin.net {
                    pad_net_map.insert((comp_des.clone(), pad_des.clone()), net.clone());
                }
            }
        }
    }

    Ok(pad_net_map)
}

/// Transform a footprint-local point into board space using the component's
/// placement position and rotation.
fn transform_point(
    pt: altium_format_types::coord::CoordPoint,
    comp_x: f64,
    comp_y: f64,
    cos_r: f64,
    sin_r: f64,
) -> altium_format_types::coord::CoordPoint {
    use altium_format_types::coord::{Coord, CoordPoint};
    let local_x = pt.x.to_mms();
    let local_y = pt.y.to_mms();
    let rotated_x = local_x * cos_r - local_y * sin_r;
    let rotated_y = local_x * sin_r + local_y * cos_r;
    CoordPoint::new(
        Coord::from_mms(comp_x + rotated_x),
        Coord::from_mms(comp_y + rotated_y),
    )
}

/// Transform all vertices in a `PcbContour` and return a flat `Vec<CoordPoint>`.
fn transform_contour(
    contour: &altium_format::api::PcbContour,
    comp_x: f64,
    comp_y: f64,
    cos_r: f64,
    sin_r: f64,
) -> Vec<altium_format_types::coord::CoordPoint> {
    contour
        .to_points()
        .into_iter()
        .map(|pt| transform_point(pt, comp_x, comp_y, cos_r, sin_r))
        .collect()
}

/// For each imported `.pcblib-spec`, derive the corresponding `.PcbLib` binary
/// path, open it, and instantiate pads and graphics for every board component
/// whose footprint matches a footprint in that library. Coordinates are
/// transformed from footprint-local to board space using the component's
/// placement position and rotation. Existing component-owned primitives are
/// removed before re-instantiation so that running `apply` twice is idempotent.
fn instantiate_footprint_primitives(
    doc: &mut PcbDoc,
    import_paths: &[PathBuf],
    pad_net_map: &std::collections::HashMap<(String, String), String>,
    imported_components: &std::collections::HashMap<
        String,
        altium_format_spec::model::ComponentSpec,
    >,
) -> anyhow::Result<()> {
    use altium_format_types::coord::{Coord, CoordPoint};

    let pcblib_paths: Vec<PathBuf> = import_paths
        .iter()
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("pcblib-spec"))
                .unwrap_or(false)
        })
        .map(|p| p.with_extension("PcbLib"))
        .collect();

    if pcblib_paths.is_empty() {
        return Ok(());
    }

    let mut board = doc
        .board()
        .map_err(|e| anyhow::anyhow!("failed to read board for primitive instantiation: {e}"))?;

    // Remove all primitives currently associated with a component; they will be
    // re-instantiated from the footprint library below.
    board.pads.retain(|p| p.component.is_none());
    board.tracks.retain(|t| t.component.is_none());
    board.arcs.retain(|a| a.component.is_none());
    board.fills.retain(|f| f.component.is_none());
    board.texts.retain(|t| t.component.is_none());
    board.regions.retain(|r| r.component.is_none());
    board.component_bodies.retain(|b| b.component.is_none());

    for pcblib_path in &pcblib_paths {
        if !pcblib_path.exists() {
            eprintln!(
                "Warning: imported .pcblib-spec has no corresponding binary at {} — skipping primitive instantiation",
                pcblib_path.display()
            );
            continue;
        }

        let lib = PcbLib::open(pcblib_path)
            .map_err(|e| anyhow::anyhow!("failed to open {}: {e}", pcblib_path.display()))?;

        let components: Vec<_> = board.components.clone();
        for comp in &components {
            if comp.pattern.is_empty() {
                continue;
            }

            let fp = match lib.footprint(&comp.pattern) {
                Ok(fp) => fp,
                Err(_) => continue,
            };

            let comp_x_f64 = comp.location.x.to_mms();
            let comp_y_f64 = comp.location.y.to_mms();
            let comp_rot_rad = comp.rotation.to_radians();
            let cos_r = comp_rot_rad.cos();
            let sin_r = comp_rot_rad.sin();

            // Build pin swap ID lookup for this component from SchLib data.
            // Keys are pin designators; values are (swap_id_pin, swap_id_part).
            let swap_id_lookup: std::collections::HashMap<&str, (Option<String>, Option<String>)> =
                if let Some(comp_spec) = imported_components.get(&comp.source_lib_reference) {
                    comp_spec
                        .pins
                        .iter()
                        .chain(comp_spec.parts.iter().flat_map(|p| p.pins.iter()))
                        .map(|pin| {
                            (
                                pin.designator.as_str(),
                                (pin.swap_group.clone(), pin.part_swap_group.clone()),
                            )
                        })
                        .collect()
                } else {
                    std::collections::HashMap::new()
                };

            for (pad_idx, fp_pad) in fp.pads.iter().enumerate() {
                let local_x = fp_pad.location.x.to_mms();
                let local_y = fp_pad.location.y.to_mms();

                let rotated_x = local_x * cos_r - local_y * sin_r;
                let rotated_y = local_x * sin_r + local_y * cos_r;

                let abs_x = Coord::from_mms(comp_x_f64 + rotated_x);
                let abs_y = Coord::from_mms(comp_y_f64 + rotated_y);

                let pad_id = format!("{}-{}", comp.designator, pad_idx + 1);

                let (swap_id_pin, swap_id_part) = swap_id_lookup
                    .get(fp_pad.pad_name.as_str())
                    .cloned()
                    .unwrap_or((None, None));

                board.pads.push(altium_format::api::PcbDocPad {
                    id: pad_id,
                    pad_name: fp_pad.pad_name.clone(),
                    layer: fp_pad.layer.clone(),
                    net: pad_net_map
                        .get(&(comp.designator.clone(), fp_pad.pad_name.clone()))
                        .cloned(),
                    component: Some(comp.designator.clone()),
                    location: CoordPoint::new(abs_x, abs_y),
                    shape: fp_pad.shape,
                    x_size: fp_pad.x_size,
                    y_size: fp_pad.y_size,
                    rotation: fp_pad.rotation + comp.rotation,
                    hole_size: fp_pad.hole_size,
                    is_plated: fp_pad.is_plated,
                    pad_mode: fp_pad.pad_mode,
                    solder_mask_expansion: fp_pad.solder_mask_expansion,
                    paste_mask_expansion: fp_pad.paste_mask_expansion,
                    plane_connection: fp_pad.plane_connection,
                    relief_conductor_width: fp_pad.relief_conductor_width,
                    relief_entries: fp_pad.relief_entries,
                    relief_air_gap: fp_pad.relief_air_gap,
                    stack: fp_pad.stack.clone(),
                    swap_id_pin,
                    swap_id_part,
                });
            }

            let mut track_counter: usize = 0;
            let mut arc_counter: usize = 0;
            let mut fill_counter: usize = 0;
            let mut text_counter: usize = 0;
            let mut region_counter: usize = 0;
            let mut body_counter: usize = 0;

            for graphic in &fp.graphics {
                match graphic {
                    altium_format::api::PcbGraphic::Track(track) => {
                        let start =
                            transform_point(track.start, comp_x_f64, comp_y_f64, cos_r, sin_r);
                        let end = transform_point(track.end, comp_x_f64, comp_y_f64, cos_r, sin_r);
                        board.tracks.push(altium_format::api::Track {
                            id: format!("{}-track-{}", comp.designator, track_counter),
                            layer: track.layer.clone(),
                            net: None,
                            component: Some(comp.designator.clone()),
                            start,
                            end,
                            width: track.width,
                        });
                        track_counter += 1;
                    }
                    altium_format::api::PcbGraphic::Arc(arc) => {
                        let center =
                            transform_point(arc.center, comp_x_f64, comp_y_f64, cos_r, sin_r);
                        board.arcs.push(altium_format::api::Arc {
                            id: format!("{}-arc-{}", comp.designator, arc_counter),
                            layer: arc.layer.clone(),
                            net: None,
                            component: Some(comp.designator.clone()),
                            center,
                            radius: arc.radius,
                            start_angle: (arc.start_angle + comp.rotation) % 360.0,
                            end_angle: (arc.end_angle + comp.rotation) % 360.0,
                            width: arc.width,
                        });
                        arc_counter += 1;
                    }
                    altium_format::api::PcbGraphic::Fill(fill) => {
                        let corner1 =
                            transform_point(fill.corner1, comp_x_f64, comp_y_f64, cos_r, sin_r);
                        let corner2 =
                            transform_point(fill.corner2, comp_x_f64, comp_y_f64, cos_r, sin_r);
                        board.fills.push(altium_format::api::Fill {
                            id: format!("{}-fill-{}", comp.designator, fill_counter),
                            layer: fill.layer.clone(),
                            net: None,
                            component: Some(comp.designator.clone()),
                            corner1,
                            corner2,
                            rotation: fill.rotation + comp.rotation,
                        });
                        fill_counter += 1;
                    }
                    altium_format::api::PcbGraphic::Text(text) => {
                        let location =
                            transform_point(text.location, comp_x_f64, comp_y_f64, cos_r, sin_r);
                        let text_str = if text.text == ".Designator" || text.text == ".DESIGNATOR" {
                            comp.designator.clone()
                        } else {
                            text.text.clone()
                        };
                        board.texts.push(altium_format::api::PcbDocText {
                            id: format!("{}-text-{}", comp.designator, text_counter),
                            layer: text.layer.clone(),
                            component: Some(comp.designator.clone()),
                            location,
                            text: text_str,
                            rotation: text.rotation + comp.rotation,
                            height: text.height,
                            width: text.width,
                            font_name: text.font_name.clone(),
                            is_mirrored: text.is_mirrored,
                            is_comment: false,
                            is_designator: false,
                        });
                        text_counter += 1;
                    }
                    altium_format::api::PcbGraphic::Region(region) => {
                        let outline = transform_contour(
                            &region.outline,
                            comp_x_f64,
                            comp_y_f64,
                            cos_r,
                            sin_r,
                        );
                        let holes: Vec<Vec<CoordPoint>> = region
                            .holes
                            .iter()
                            .map(|h| transform_contour(h, comp_x_f64, comp_y_f64, cos_r, sin_r))
                            .collect();
                        board.regions.push(altium_format::api::Region {
                            id: format!("{}-region-{}", comp.designator, region_counter),
                            layer: region.layer.clone(),
                            net: None,
                            component: Some(comp.designator.clone()),
                            kind: region.kind,
                            outline,
                            holes,
                            is_board_cutout: false,
                            is_keepout: false,
                        });
                        region_counter += 1;
                    }
                    altium_format::api::PcbGraphic::Via(_via) => {
                        // Vias in footprint graphics are skipped — they are handled
                        // separately if needed.
                    }
                    altium_format::api::PcbGraphic::ComponentBody(body) => {
                        let outline =
                            transform_contour(&body.outline, comp_x_f64, comp_y_f64, cos_r, sin_r);
                        board
                            .component_bodies
                            .push(altium_format::api::ComponentBody {
                                id: format!("{}-body-{}", comp.designator, body_counter),
                                layer: body.layer.clone(),
                                component: Some(comp.designator.clone()),
                                standoff_height: body.standoff_height,
                                overall_height: body.overall_height,
                                body_color_3d: body.body_color_3d,
                                body_opacity_3d: body.body_opacity_3d,
                                model_name: body.model_name.clone(),
                                outline,
                            });
                        body_counter += 1;
                    }
                }
            }
        }
    }

    doc.update_board(&board)
        .map_err(|e| anyhow::anyhow!("failed to update board with instantiated primitives: {e}"))?;

    Ok(())
}

// ── dump ──────────────────────────────────────────────────────────────────────

/// Write a spec file, merging with existing content if the output file exists.
///
/// If the output file already exists and can be parsed, merges the fresh dump
/// with the existing content to preserve comments and annotation IDs.
/// Falls back to overwriting if the existing file can't be parsed.
fn write_spec_merged(
    out_path: &std::path::Path,
    spec_source: &str,
    document: &PathBuf,
) -> anyhow::Result<()> {
    if out_path.exists() {
        match std::fs::read_to_string(out_path) {
            Ok(old_text) => match spec_merge::merge_spec(&old_text, spec_source) {
                Some(merged) => {
                    std::fs::write(out_path, &merged).map_err(|e| {
                        anyhow::anyhow!("failed to write {}: {e}", out_path.display())
                    })?;
                    println!("Merged: {} -> {}", document.display(), out_path.display());
                    return Ok(());
                }
                None => {
                    eprintln!(
                        "Warning: existing spec file has parse errors, overwriting without merge"
                    );
                }
            },
            Err(_) => {
                // Can't read existing file — just overwrite.
            }
        }
    }
    std::fs::write(out_path, spec_source)
        .map_err(|e| anyhow::anyhow!("failed to write {}: {e}", out_path.display()))?;
    println!("Dumped: {} -> {}", document.display(), out_path.display());
    Ok(())
}

fn run_dump(document: &PathBuf, output: Option<&PathBuf>) -> anyhow::Result<()> {
    // IntLib can contain both SchLib and PcbLib data, so it bypasses the
    // single-domain path and dumps separate spec files.
    let ext = document
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if ext == "intlib" {
        return run_dump_intlib(document, output);
    }

    let domain = detect_document_domain(document)?;
    let out_path = output
        .cloned()
        .unwrap_or_else(|| default_spec_for_document(document, &domain));

    match domain {
        SpecDomain::SchLib => {
            let lib = SchLib::open(document)
                .map_err(|e| anyhow::anyhow!("failed to open {}: {e}", document.display()))?;
            let spec_source = dump_schlib(&lib)
                .map_err(|e| anyhow::anyhow!("failed to dump {}: {e}", document.display()))?;
            write_spec_merged(&out_path, &spec_source, document)?;
        }
        SpecDomain::PcbLib => {
            let lib = PcbLib::open(document)
                .map_err(|e| anyhow::anyhow!("failed to open {}: {e}", document.display()))?;
            let spec_source = dump_pcblib(&lib);
            write_spec_merged(&out_path, &spec_source, document)?;
        }
        SpecDomain::SchDoc => {
            let doc = SchDoc::open(document)
                .map_err(|e| anyhow::anyhow!("failed to open {}: {e}", document.display()))?;
            let spec_source = dump_schdoc(&doc)
                .map_err(|e| anyhow::anyhow!("failed to dump {}: {e}", document.display()))?;
            write_spec_merged(&out_path, &spec_source, document)?;
        }
        SpecDomain::PrjPcb => {
            let doc = AltiumProject::open(document)
                .map_err(|e| anyhow::anyhow!("failed to open {}: {e}", document.display()))?;
            let spec_source = dump_prjpcb(&doc).map_err(|e| anyhow::anyhow!("dump failed: {e}"))?;
            write_spec_merged(&out_path, &spec_source, document)?;
        }
        SpecDomain::PcbDoc => {
            let doc = PcbDoc::open(document)
                .map_err(|e| anyhow::anyhow!("failed to open {}: {e}", document.display()))?;
            let spec_source = dump_pcbdoc(&doc)
                .map_err(|e| anyhow::anyhow!("failed to dump {}: {e}", document.display()))?;
            write_spec_merged(&out_path, &spec_source, document)?;
        }
    }

    Ok(())
}

fn run_dump_intlib(document: &PathBuf, output: Option<&PathBuf>) -> anyhow::Result<()> {
    let lib = IntLib::open(document)
        .map_err(|e| anyhow::anyhow!("failed to open {}: {e}", document.display()))?;
    let dump = dump_intlib(&lib)
        .map_err(|e| anyhow::anyhow!("failed to dump {}: {e}", document.display()))?;

    let stem = document
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    let out_dir = match output {
        Some(p) if p.is_dir() => p.clone(),
        Some(p) => p
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .to_path_buf(),
        None => document
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .to_path_buf(),
    };

    let mut wrote_any = false;
    if let Some(schlib_spec) = &dump.schlib_spec {
        let path = out_dir.join(format!("{stem}.schlib-spec"));
        write_spec_merged(&path, schlib_spec, document)?;
        wrote_any = true;
    }
    if let Some(pcblib_spec) = &dump.pcblib_spec {
        let path = out_dir.join(format!("{stem}.pcblib-spec"));
        write_spec_merged(&path, pcblib_spec, document)?;
        wrote_any = true;
    }
    if !wrote_any {
        anyhow::bail!("{} contains no SchLib or PcbLib data", document.display());
    }

    Ok(())
}

// ── compile helper ────────────────────────────────────────────────────────────

struct CompileResult {
    model: altium_format_spec::model::SpecModel,
    /// All import paths (bare + named) for --all processing.
    import_paths: Vec<PathBuf>,
    /// Compiled SchLib components from imports, keyed by lib_reference.
    /// Used by SchDoc apply to resolve pin positions.
    imported_components:
        std::collections::HashMap<String, altium_format_spec::model::ComponentSpec>,
}

fn compile_and_resolve(
    source: &str,
    spec_file: &PathBuf,
    domain: &SpecDomain,
) -> anyhow::Result<CompileResult> {
    use altium_format_spec::parser::parse_spec;

    let source_name = spec_file.display().to_string();
    let file =
        parse_spec(source).map_err(|e| anyhow::anyhow!("{}", e.render(&source_name, source)))?;

    // Resolve imports: validates cycles, cross-domain rules, alias uniqueness,
    // and file existence. We do NOT merge bare imports into the root AST —
    // each file is compiled independently (reference semantics).
    let spec_path_canonical = spec_file
        .canonicalize()
        .unwrap_or_else(|_| spec_file.clone());
    let resolved = resolve_imports(&spec_path_canonical, file.clone())
        .map_err(|e| anyhow::anyhow!("{}", e.render(&source_name, source)))?;

    // Compile only the root file's own items, with named imports in scope.
    // Imported SchLib components are needed both for symbol validation and for
    // SchDoc apply/pin projection.
    let imported_components_for_compile =
        compile_imported_schlibs(&resolved).map_err(|(path, e)| {
            let import_source = std::fs::read_to_string(&path).unwrap_or_default();
            let import_name = path.display().to_string();
            anyhow::anyhow!("{}", e.render(&import_name, &import_source))
        })?;

    let imported_components_for_exec =
        compile_imported_schlibs(&resolved).map_err(|(path, e)| {
            let import_source = std::fs::read_to_string(&path).unwrap_or_default();
            let import_name = path.display().to_string();
            anyhow::anyhow!("{}", e.render(&import_name, &import_source))
        })?;

    let model = compile_spec_with_resolved(&resolved, *domain, imported_components_for_compile)
        .map_err(|e| anyhow::anyhow!("{}", e.render(&source_name, source)))?;

    // Collect all import paths for --all processing.
    let import_paths: Vec<PathBuf> = resolved
        .bare_imports
        .iter()
        .map(|(p, _)| p.clone())
        .chain(resolved.named_imports.values().map(|(p, _)| p.clone()))
        .collect();

    Ok(CompileResult {
        model,
        import_paths,
        imported_components: imported_components_for_exec,
    })
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
        let symbols_json: Vec<serde_json::Value> = sheet
            .sheet_symbols()
            .iter()
            .map(|s| {
                serde_json::json!({
                    "file_name": s.file_name,
                    "sheet_name": s.sheet_name,
                })
            })
            .collect();

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

fn run_info_schlib(path: &std::path::Path, lib: &SchLib, format: &str) -> anyhow::Result<()> {
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

fn run_info_pcblib(path: &std::path::Path, lib: &PcbLib, format: &str) -> anyhow::Result<()> {
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
    let query = parse_query(query_str).map_err(|e| anyhow::anyhow!("{}", e.render(query_str)))?;

    // Open the document based on file extension
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let results = match ext.as_str() {
        "schlib" => {
            let lib = SchLib::open(path)?;
            eval_query(&query, &lib).map_err(|e| anyhow::anyhow!("{}", e.render(query_str)))?
        }
        "pcblib" => {
            let lib = PcbLib::open(path)?;
            eval_query(&query, &lib).map_err(|e| anyhow::anyhow!("{}", e.render(query_str)))?
        }
        "schdoc" => {
            let doc = SchDoc::open(path)?;
            eval_query(&query, &doc).map_err(|e| anyhow::anyhow!("{}", e.render(query_str)))?
        }
        "pcbdoc" => {
            let doc = PcbDoc::open(path)?;
            eval_query(&query, &doc).map_err(|e| anyhow::anyhow!("{}", e.render(query_str)))?
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
                println!(
                    "{} match{}:",
                    results.len(),
                    if results.len() == 1 { "" } else { "es" }
                );
                for m in &results {
                    let type_name = format!("{:?}", m.node.type_selector());
                    println!("  [{type_name}] {}", m.node.display_name());
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {}
