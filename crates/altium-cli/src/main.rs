use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use altium_format::{AltiumProject, IntLib, PcbDoc, PcbLib, SchDoc, SchLib, VersionInfo};
use altium_format_query::{eval_query, parse_query};
use altium_format_render_png::{
    DEFAULT_SCALE, render_pcblib_footprint_png, render_schdoc_png, render_schlib_component_png,
};
use altium_format_render_svg::{render_pcblib_footprint, render_schdoc, render_schlib_component};
use autopcb_spec::{
    FormatConfig, PcbDocSpec, PlacementConstraintSpec, PlacementPlaceSpec, SpecDomain, SyncChange,
    SyncDirection, SyncPolicy, apply_spec_pcbdoc, apply_spec_prjpcb,
    apply_spec_schdoc, apply_spec_schlib, apply_sync_changes_to_pcbdoc, compile_imported_syms,
    compile_spec_with_resolved, diff_snapshots, dump_intlib,
    dump_pcbdoc, dump_pcblib, dump_placement_block, dump_prjpcb, dump_schdoc, dump_schlib,
    filter_changes, format_spec, project_pcbdoc_spec, project_schdoc_spec, reconcile_pcbdoc,
    reconcile_pcbdoc_empty, reconcile_prjpcb,
    reconcile_prjpcb_empty, reconcile_schdoc, reconcile_schdoc_empty, reconcile_schlib,
    reconcile_schlib_empty, render_eco_report, resolve_imports, rewrite_pcbdoc_spec_with_changes,
    validate_pcbdoc_spec, validate_schdoc_spec,
};
use autopcb_graph_import_altium::{import_pcblib, import_schlib};
use autopcb_graph_spec::{create_workspace_bundle, save_workspace, validate_workspace};
use autopcb_ir::{PcbIr, spec_bridge::load_ir_from_spec};
use autopcb_placement::{
    Direction, PlacementConfig, PlacementEdge, RectRegion, UserConstraint, named_region_from_board,
    solve_placement,
};
use clap::{Parser, Subcommand};
use tracing::{debug, info};

mod cfb;
pub mod placement_bridge;
pub mod spec_merge;
pub mod spec_rewriter;

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
        /// Path to the spec file (.sym, .sch, .pcb, or .proj)
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
        /// Path to the spec file (.sym, .sch, .pcb, or .proj)
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
    /// Placement solver commands for .pcb files
    Placement {
        #[command(subcommand)]
        sub: PlacementSubcommand,
    },
    /// Canonical AutoPCB graph workspace commands
    Graph {
        #[command(subcommand)]
        sub: GraphSubcommand,
    },
    /// Format spec files (.sym, .sch, .pcb, .proj)
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
    /// Routing commands for .routes files and spec-driven autorouting
    Routing {
        #[command(subcommand)]
        sub: RoutingSubcommand,
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

#[derive(Subcommand)]
enum PlacementSubcommand {
    /// Solve placement constraints from a .pcb file
    Solve {
        /// Path to .pcb source file
        spec_file: PathBuf,
        /// Emit JSON report
        #[arg(long, default_value_t = false)]
        json: bool,
        /// Write iteration snapshots for autopcb-viewer playback
        #[arg(long)]
        iterations_out: Option<PathBuf>,
        #[arg(long, default_value_t = 2.0)]
        gamma_start: f64,
        #[arg(long, default_value_t = 10.0)]
        gamma_end: f64,
        #[arg(long, default_value_t = 250)]
        max_iters: usize,
    },
    /// Auto-place components from a .pcb file and rewrite it with solved positions
    Autoplace {
        /// Path to .pcb source file
        spec_file: PathBuf,
        /// Show plan without writing any files
        #[arg(long, default_value_t = false)]
        dry_run: bool,
        /// Write updated spec to this path (default: overwrite spec_file)
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long, default_value_t = 2.0)]
        gamma_start: f64,
        #[arg(long, default_value_t = 10.0)]
        gamma_end: f64,
        #[arg(long, default_value_t = 250)]
        max_iters: usize,
        /// Disable simulated annealing refinement after analytical placement
        #[arg(long = "no-sa", action = clap::ArgAction::SetFalse, default_value_t = true)]
        sa: bool,
    },
    /// Dump current component positions from a PcbDoc as a placement spec
    Dump {
        /// Target PcbDoc file
        target: PathBuf,
    },
    /// Show placement ECO plan (what would change)
    Plan {
        /// Path to .pcb source file
        spec_file: PathBuf,
        /// Target .PcbDoc file
        #[arg(long)]
        target: PathBuf,
    },
    /// Apply placement spec to PcbDoc
    Apply {
        /// Path to .pcb source file
        spec_file: PathBuf,
        /// Target .PcbDoc file
        #[arg(long)]
        target: PathBuf,
    },
}

#[derive(Subcommand)]
enum GraphSubcommand {
    /// Create a new graph-spec workspace bundle
    New {
        /// Output root file, typically ending in .graph-spec
        output: PathBuf,
        /// Design/workspace name
        #[arg(long)]
        name: String,
    },
    /// Import an Altium library into a graph-spec workspace bundle
    Import {
        /// Input .SchLib or .PcbLib
        input: PathBuf,
        /// Output root file, typically ending in .graph-spec
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Validate a graph-spec workspace bundle
    Validate {
        /// Root .graph-spec file
        root: PathBuf,
    },
}

#[derive(Subcommand)]
enum SpecSubcommand {
    /// Synchronize SchDoc-spec and PcbDoc-spec files
    Sync {
        /// Path to the .sch source file
        schdoc_spec: PathBuf,
        /// Path to the .pcb target file
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

#[derive(Subcommand)]
enum RoutingSubcommand {
    /// Load a .routes file and print routing statistics
    Inspect {
        /// Path to the .routes file (binary or JSON)
        path: PathBuf,
        /// Show detailed per-violation output
        #[arg(long)]
        verbose: bool,
        /// Output in JSON format
        #[arg(long)]
        json: bool,
        // Note: --drc for live DRC is not supported here because running DRC
        // requires a full PcbIr, which is not available from a .routes file alone.
    },
    /// Solve routing from a .pcb file and write a .routes file
    Solve {
        /// Path to .pcb file
        spec_file: PathBuf,
        /// Output .routes file path (default: <spec_stem>.routes)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Emit JSON output
        #[arg(long)]
        json: bool,
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
        Commands::Inspect { path, sub } => {
            if let Err(e) = run_inspect(&path, sub) {
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
        Commands::Placement { sub } => {
            if let Err(e) = run_placement(sub) {
                eprintln!("Error: {e}");
                return ExitCode::FAILURE;
            }
        }
        Commands::Graph { sub } => {
            if let Err(e) = run_graph(sub) {
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
        Commands::Routing { sub } => {
            if let Err(e) = run_routing(sub) {
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

    let schdoc_result = compile_and_resolve(&schdoc_source, schdoc_spec_path, &SpecDomain::Sch)?;
    let pcbdoc_result = compile_and_resolve(&pcbdoc_source, pcbdoc_spec_path, &SpecDomain::Pcb)?;

    let schdoc_spec = match schdoc_result.model {
        autopcb_spec::model::SpecModel::Sch(spec) => spec,
        _ => anyhow::bail!(
            "{} is not a valid .sch file",
            schdoc_spec_path.display()
        ),
    };
    let pcbdoc_spec_model = match pcbdoc_result.model {
        autopcb_spec::model::SpecModel::Pcb(spec) => spec,
        _ => anyhow::bail!(
            "{} is not a valid .pcb file",
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

fn run_graph(sub: GraphSubcommand) -> anyhow::Result<()> {
    match sub {
        GraphSubcommand::New { output, name } => {
            let _ = create_workspace_bundle(&output, &name)?;
            eprintln!("Created graph workspace: {}", output.display());
        }
        GraphSubcommand::Import { input, output } => {
            let out = output.unwrap_or_else(|| input.with_extension("graph-spec"));
            let ext = input
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_default();
            let workspace = match ext.as_str() {
                "schlib" => import_schlib(&input)?,
                "pcblib" => import_pcblib(&input)?,
                _ => anyhow::bail!(
                    "graph import currently supports .SchLib and .PcbLib only: {}",
                    input.display()
                ),
            };
            let _ = save_workspace(&out, &workspace)?;
            eprintln!("Imported {} -> {}", input.display(), out.display());
        }
        GraphSubcommand::Validate { root } => {
            validate_workspace(&root)?;
            eprintln!("Validated graph workspace: {}", root.display());
        }
    }
    Ok(())
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
        Some("sym") => Ok(SpecDomain::Sym),
        Some("sch") => Ok(SpecDomain::Sch),
        Some("pcb") => Ok(SpecDomain::Pcb),
        Some("proj") => Ok(SpecDomain::Proj),
        Some(ext) => anyhow::bail!(
            "unknown spec file extension .{ext} (supported: .sym, .sch, .pcb, .proj)"
        ),
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
        "schlib" | "pcblib" => Ok(SpecDomain::Sym),
        "schdoc" => Ok(SpecDomain::Sch),
        "prjpcb" => Ok(SpecDomain::Proj),
        "pcbdoc" => Ok(SpecDomain::Pcb),
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
        SpecDomain::Sym => "SchLib",
        SpecDomain::Sch => "SchDoc",
        SpecDomain::Pcb => "PcbDoc",
        SpecDomain::Proj => "PrjPcb",
    };
    spec_file.with_file_name(format!("{stem}.{ext}"))
}

fn default_spec_for_document(doc: &PathBuf, domain: &SpecDomain) -> PathBuf {
    let stem = doc.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
    let ext = match domain {
        SpecDomain::Sym => "sym",
        SpecDomain::Sch => "sch",
        SpecDomain::Pcb => "pcb",
        SpecDomain::Proj => "proj",
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
    if all && domain != SpecDomain::Proj {
        anyhow::bail!("--all is only valid for .proj files");
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
    spec_model: &autopcb_spec::model::SpecModel,
    target: Option<&PathBuf>,
    spec_file: &PathBuf,
    domain: &SpecDomain,
) -> anyhow::Result<autopcb_spec::eco::EngineeringChangeOrder> {
    let library_path = default_output_for_spec(spec_file, domain);
    let spec_path = spec_file.clone();

    let eco = match spec_model {
        autopcb_spec::model::SpecModel::Sym(spec_lib) => {
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
        autopcb_spec::model::SpecModel::Proj(spec) => {
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
        autopcb_spec::model::SpecModel::Sch(spec) => {
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
        autopcb_spec::model::SpecModel::Pcb(spec) => {
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
    if all && domain != SpecDomain::Proj {
        anyhow::bail!("--all is only valid for .proj files");
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
    spec_model: &autopcb_spec::model::SpecModel,
    target: Option<&PathBuf>,
    output: Option<&PathBuf>,
    spec_file: &PathBuf,
    domain: &SpecDomain,
    imported_components: &std::collections::HashMap<
        String,
        autopcb_spec::model::ComponentSpec,
    >,
    import_paths: &[PathBuf],
) -> anyhow::Result<()> {
    let library_path = default_output_for_spec(spec_file, domain);

    match spec_model {
        autopcb_spec::model::SpecModel::Sym(spec_lib) => {
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
        autopcb_spec::model::SpecModel::Proj(spec) => {
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
        autopcb_spec::model::SpecModel::Sch(spec) => {
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
        autopcb_spec::model::SpecModel::Pcb(spec) => {
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

            // Merge routed tracks/vias from .routes file into the PcbDoc.
            let routes_injected = inject_routes_into_pcbdoc(&mut doc, spec, spec_file)?;
            if routes_injected > 0 {
                eprintln!("  Injected {routes_injected} routed primitives from .routes file");
            }

            doc.save(&out_path)?;
            println!("Saved: {}", out_path.display());
        }
    }

    Ok(())
}

// ── footprint primitive instantiation ────────────────────────────────────────

/// Discovers sibling `.sch` files and builds a pad-to-net map.
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
                .map(|e| e.eq_ignore_ascii_case("sch"))
                .unwrap_or(false)
        })
        .collect();

    for schdoc_path in &schdoc_specs {
        let source = std::fs::read_to_string(schdoc_path)
            .map_err(|e| anyhow::anyhow!("reading {}: {e}", schdoc_path.display()))?;

        let domain = SpecDomain::Sch;
        let result = compile_and_resolve(&source, schdoc_path, &domain)?;

        let schdoc_spec = match result.model {
            autopcb_spec::model::SpecModel::Sch(spec) => spec,
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

/// Load a `.routes` file and inject routed tracks/vias into a PcbDoc board.
///
/// Returns the number of primitives injected (tracks + vias). Returns 0 if no
/// routes file is configured or found.
fn inject_routes_into_pcbdoc(
    doc: &mut PcbDoc,
    spec: &PcbDocSpec,
    spec_file: &std::path::Path,
) -> anyhow::Result<usize> {
    use altium_format::api::{Track, Via};
    use altium_format_types::{Coord, CoordPoint, LayerRef};

    let spec_dir = spec_file.parent().unwrap_or(std::path::Path::new("."));

    // Determine routes path: explicit from spec, or convention <stem>.routes
    let routes_path = if let Some(ref routing) = spec.routing {
        if let Some(ref solution) = routing.solution {
            spec_dir.join(solution)
        } else {
            spec_file.with_extension("routes")
        }
    } else {
        spec_file.with_extension("routes")
    };

    if !routes_path.exists() {
        return Ok(0);
    }

    let solution = autopcb_routes::load_binary(&routes_path)
        .or_else(|_| autopcb_routes::load_json(&routes_path))
        .map_err(|e| anyhow::anyhow!("failed to load routes {}: {e}", routes_path.display()))?;

    // Build net name lookup from the spec's net list.
    let net_names: Vec<String> = spec
        .boards
        .first()
        .map(|b| b.nets.iter().map(|n| n.name.clone()).collect())
        .unwrap_or_default();

    let mut board = doc
        .board()
        .map_err(|e| anyhow::anyhow!("failed to read board for route injection: {e}"))?;

    // Map router layer index to LayerRef.
    // The router uses 0-based copper layer indices from the IR layer stack.
    // Build the same mapping by reading the board's layer stack.
    // Copper layers from the stack (non-plane signal layers + plane layers).
    // The router's layer indices match the copper layer order from the IR layer stack,
    // which is the same order as the board's layer stack copper layers.
    let layer_refs: Vec<LayerRef> = board
        .settings
        .layer_stack
        .layers
        .iter()
        .map(|l| l.layer.clone())
        .collect();

    let mut count = 0usize;

    for routed_net in solution.nets.values() {
        let net_name = net_names
            .get(routed_net.net_id.raw() as usize)
            .cloned();

        for (i, seg) in routed_net.segments.iter().enumerate() {
            let layer = layer_refs
                .get(seg.layer.raw() as usize)
                .cloned()
                .unwrap_or_else(|| LayerRef::from_v6(altium_format_types::pcb::V6Layer::TopLayer));
            board.tracks.push(Track {
                id: format!("rt_{}_{}_{i}", routed_net.net_id.raw(), seg.layer.raw()),
                layer,
                net: net_name.clone(),
                component: None,
                start: CoordPoint::new(
                    Coord::from_mms(seg.start.x),
                    Coord::from_mms(seg.start.y),
                ),
                end: CoordPoint::new(
                    Coord::from_mms(seg.end.x),
                    Coord::from_mms(seg.end.y),
                ),
                width: Coord::from_mms(seg.width_mm),
            });
            count += 1;
        }

        for (i, via) in routed_net.vias.iter().enumerate() {
            let from_layer = layer_refs
                .get(via.from_layer.raw() as usize)
                .cloned()
                .unwrap_or_else(|| LayerRef::from_v6(altium_format_types::pcb::V6Layer::TopLayer));
            let to_layer = layer_refs
                .get(via.to_layer.raw() as usize)
                .cloned()
                .unwrap_or_else(|| {
                    LayerRef::from_v6(altium_format_types::pcb::V6Layer::BottomLayer)
                });
            board.vias.push(Via {
                id: format!("rv_{}_{i}", routed_net.net_id.raw()),
                net: net_name.clone(),
                component: None,
                location: CoordPoint::new(
                    Coord::from_mms(via.position.x),
                    Coord::from_mms(via.position.y),
                ),
                diameter: Coord::from_mms(via.drill_mm + 2.0 * via.annular_ring_mm),
                hole_size: Coord::from_mms(via.drill_mm),
                from_layer,
                to_layer,
                solder_mask_expansion: None,
            });
            count += 1;
        }
    }

    doc.update_board(&board)
        .map_err(|e| anyhow::anyhow!("failed to update board with routes: {e}"))?;

    Ok(count)
}

/// For each imported `.sym`, derive the corresponding `.PcbLib` binary
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
        autopcb_spec::model::ComponentSpec,
    >,
) -> anyhow::Result<()> {
    use altium_format_types::coord::{Coord, CoordPoint};

    let pcblib_paths: Vec<PathBuf> = import_paths
        .iter()
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("sym"))
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
                "Warning: imported .sym has no corresponding binary at {} — skipping primitive instantiation",
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
    // IntLib produces a single .sym output file, so it
    // bypasses the single-domain path.
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
        SpecDomain::Sym => {
            let ext = document
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if ext == "schlib" {
                let lib = SchLib::open(document)
                    .map_err(|e| anyhow::anyhow!("failed to open {}: {e}", document.display()))?;
                let spec_source = dump_schlib(&lib)
                    .map_err(|e| anyhow::anyhow!("failed to dump {}: {e}", document.display()))?;
                write_spec_merged(&out_path, &spec_source, document)?;
            } else {
                let lib = PcbLib::open(document)
                    .map_err(|e| anyhow::anyhow!("failed to open {}: {e}", document.display()))?;
                let spec_source = dump_pcblib(&lib);
                write_spec_merged(&out_path, &spec_source, document)?;
            }
        }
        SpecDomain::Sch => {
            let doc = SchDoc::open(document)
                .map_err(|e| anyhow::anyhow!("failed to open {}: {e}", document.display()))?;
            let spec_source = dump_schdoc(&doc)
                .map_err(|e| anyhow::anyhow!("failed to dump {}: {e}", document.display()))?;
            write_spec_merged(&out_path, &spec_source, document)?;
        }
        SpecDomain::Proj => {
            let doc = AltiumProject::open(document)
                .map_err(|e| anyhow::anyhow!("failed to open {}: {e}", document.display()))?;
            let spec_source = dump_prjpcb(&doc).map_err(|e| anyhow::anyhow!("dump failed: {e}"))?;
            write_spec_merged(&out_path, &spec_source, document)?;
        }
        SpecDomain::Pcb => {
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

    if let Some(sym_spec) = &dump.sym_spec {
        let path = out_dir.join(format!("{stem}.sym"));
        write_spec_merged(&path, sym_spec, document)?;
    } else {
        anyhow::bail!("{} contains no SchLib or PcbLib data", document.display());
    }

    Ok(())
}

// ── compile helper ────────────────────────────────────────────────────────────

struct CompileResult {
    model: autopcb_spec::model::SpecModel,
    /// All import paths (bare + named) for --all processing.
    import_paths: Vec<PathBuf>,
    /// Compiled SchLib components from imports, keyed by lib_reference.
    /// Used by SchDoc apply to resolve pin positions.
    imported_components:
        std::collections::HashMap<String, autopcb_spec::model::ComponentSpec>,
    /// Compiled sym libraries from imports, keyed by import alias or canonical path.
    /// Used by routing/placement solve to resolve footprint definitions.
    imported_footprints:
        std::collections::HashMap<String, autopcb_spec::model::SymSpec>,
}

fn compile_and_resolve(
    source: &str,
    spec_file: &PathBuf,
    domain: &SpecDomain,
) -> anyhow::Result<CompileResult> {
    use autopcb_spec::parser::parse_spec;

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
    // Two callees need the imported-components map: compile_spec_with_resolved for symbol
    // validation and apply_spec_schdoc for pin position resolution. Build twice since
    // ComponentSpec does not implement Clone.
    let sym_libs_for_exec = compile_imported_syms(&resolved).map_err(|(path, e)| {
        let import_source = std::fs::read_to_string(&path).unwrap_or_default();
        let import_name = path.display().to_string();
        anyhow::anyhow!("{}", e.render(&import_name, &import_source))
    })?;
    let imported_components_for_exec: std::collections::HashMap<String, autopcb_spec::model::ComponentSpec> =
        sym_libs_for_exec.into_iter()
            .flat_map(|(_key, sym)| sym.components.into_iter())
            .map(|c| (c.lib_reference.clone(), c))
            .collect();

    let sym_libs2 = compile_imported_syms(&resolved).map_err(|(path, e)| {
        let import_source = std::fs::read_to_string(&path).unwrap_or_default();
        let import_name = path.display().to_string();
        anyhow::anyhow!("{}", e.render(&import_name, &import_source))
    })?;
    let imported_components: std::collections::HashMap<String, autopcb_spec::model::ComponentSpec> =
        sym_libs2.into_iter()
            .flat_map(|(_key, sym)| sym.components.into_iter())
            .map(|c| (c.lib_reference.clone(), c))
            .collect();
    let model = compile_spec_with_resolved(&resolved, *domain, imported_components)
        .map_err(|e| anyhow::anyhow!("{}", e.render(&source_name, source)))?;

    let imported_footprints = compile_imported_syms(&resolved).map_err(|(path, e)| {
        let import_source = std::fs::read_to_string(&path).unwrap_or_default();
        let import_name = path.display().to_string();
        anyhow::anyhow!("{}", e.render(&import_name, &import_source))
    })?;

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
        imported_footprints,
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

fn run_inspect(path: &std::path::Path, sub: InspectSubcommand) -> anyhow::Result<()> {
    use autopcb_ir::import_pcbdoc;
    use autopcb_ir::spec_compiler::spec_to_ir;

    let doc = PcbDoc::open(path)
        .map_err(|e| anyhow::anyhow!("failed to open {}: {e}", path.display()))?;
    let board = doc.board()
        .map_err(|e| anyhow::anyhow!("failed to extract board: {e}"))?;
    let imported_spec = import_pcbdoc(&board)
        .map_err(|e| anyhow::anyhow!("failed to import PcbDoc: {e}"))?;
    let ir = spec_to_ir(&imported_spec, &std::collections::HashMap::new())
        .map_err(|e| anyhow::anyhow!("spec compilation failed: {e:?}"))?;

    match sub {
        InspectSubcommand::Summary => {
            let b = &ir.board.bounds;
            println!("Board: {:.2} x {:.2} mm", b.width(), b.height());
            println!("  Origin: ({:.2}, {:.2}) mm", b.min.x, b.min.y);
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
            println!("{:<30} {:>6} {:>6}", "NET NAME", "PINS", "COMPS");
            println!("{}", "-".repeat(44));
            for (_id, net) in ir.nets.iter() {
                println!(
                    "{:<30} {:>6} {:>6}",
                    net.name,
                    net.pins.len(),
                    net.component_count
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

fn run_placement(sub: PlacementSubcommand) -> anyhow::Result<()> {
    match sub {
        PlacementSubcommand::Autoplace {
            spec_file,
            dry_run,
            output,
            gamma_start,
            gamma_end,
            max_iters,
            sa,
        } => {
            let mut cfg = PlacementConfig {
                gamma_start,
                gamma_end,
                max_iters,
                ..PlacementConfig::default()
            };
            if sa {
                cfg.sa_config = Some(autopcb_placement::simulated_annealing::SAConfig::default());
            }
            let report = autoplace_spec(
                &spec_file,
                &cfg,
                dry_run,
                output.as_deref(),
            )?;
            println!("AUTOPLACE REPORT");
            println!("  components placed:  {}", report.autoplace_count);
            println!("  total components:   {}", report.component_count);
            println!("  HPWL estimate:      {:.3} mm", report.hpwl_mm);
            println!("  duration:           {} ms", report.duration_ms);
            if dry_run {
                println!("  (dry-run: no files written)");
            } else {
                println!("  output spec:        {}", report.output_path.display());
            }
        }
        PlacementSubcommand::Dump { target } => {
            cmd_placement_dump(&target)?;
        }
        PlacementSubcommand::Plan { spec_file, target } => {
            cmd_placement_plan(&spec_file, &target)?;
        }
        PlacementSubcommand::Apply { spec_file, target } => {
            cmd_placement_apply(&spec_file, &target)?;
        }
        PlacementSubcommand::Solve {
            spec_file,
            json,
            iterations_out,
            gamma_start,
            gamma_end,
            max_iters,
        } => {
            let source = std::fs::read_to_string(&spec_file)
                .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", spec_file.display()))?;
            let compiled = compile_and_resolve(&source, &spec_file, &SpecDomain::Pcb)?;
            let spec = match compiled.model {
                autopcb_spec::model::SpecModel::Pcb(spec) => spec,
                _ => {
                    return Err(anyhow::anyhow!(
                        "expected PcbDoc model for {}",
                        spec_file.display()
                    ));
                }
            };

            let placement = spec.placement.as_ref().ok_or_else(|| {
                anyhow::anyhow!(
                    "spec {} has no placement {{ ... }} block",
                    spec_file.display()
                )
            })?;

            let spec_dir = spec_file.parent().unwrap_or(std::path::Path::new("."));
            let ir = load_ir_from_spec(&spec, &compiled.imported_footprints, spec_dir)
                .map_err(|e| anyhow::anyhow!("{e}"))?;

            let mut cfg = PlacementConfig {
                gamma_start,
                gamma_end,
                max_iters,
                ..PlacementConfig::default()
            };
            if let Some(all) = placement.clearance.all {
                cfg.default_clearance_mm = all.to_mms();
            }
            if let Some(edge) = placement.clearance.edge {
                cfg.board_edge_clearance_mm = edge.to_mms();
            }
            for rule in &spec.placement_rules {
                if let (Some(kind), Some(gap)) = (&rule.kind, rule.gap) {
                    match kind.as_str() {
                        "component_clearance" => cfg.default_clearance_mm = gap.to_mms(),
                        "board_outline_clearance" => cfg.board_edge_clearance_mm = gap.to_mms(),
                        _ => {}
                    }
                }
            }
            cfg.ratsnest_weight = placement.optimize.ratsnest_weight;

            let user_constraints =
                build_user_constraints(&ir, &placement.places, &placement.constraints)?;
            let result = solve_placement(&ir, &user_constraints, &cfg, &[])
                .map_err(|e| anyhow::anyhow!("{e}"))?;

            if let Some(path) = iterations_out {
                let payload = serde_json::to_string_pretty(&result.snapshots)?;
                std::fs::write(&path, payload)
                    .map_err(|e| anyhow::anyhow!("failed to write {}: {e}", path.display()))?;
            }

            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!("PLACEMENT REPORT");
                println!("  status: {}", result.status);
                println!("  iterations: {}", result.total_iterations);
                println!("  duration: {} ms", result.duration_ms);
                println!("  hpwl estimate: {:.3} mm", result.hpwl_estimate_mm);
                println!("  overlap violations: {}", result.overlap_violations);
                println!("  components: {}", result.components.len());
                println!();
                for c in &result.components {
                    println!(
                        "  {:<12} ({:>9.3}, {:>9.3}) rot {:>6.1}",
                        c.designator, c.x_mm, c.y_mm, c.rotation_deg
                    );
                }
            }
        }
    }

    Ok(())
}

/// Summary returned by [`autoplace_spec`].
pub struct AutoplaceReport {
    /// HPWL estimate from the solver (mm).
    pub hpwl_mm: f64,
    /// Total number of components in the IR.
    pub component_count: usize,
    /// Number of components that were auto-placed by the solver.
    pub autoplace_count: usize,
    /// Solver wall-clock duration.
    pub duration_ms: u128,
    /// Path of the written output spec (same as input when not dry-run and no --output given).
    pub output_path: std::path::PathBuf,
}

/// Orchestrate the full autoplace pipeline:
///
/// 1. Read and compile the spec file.
/// 2. Open the target PcbDoc and extract the IR.
/// 3. Build solver constraints via [`placement_bridge::placement_spec_to_constraints`].
/// 4. Build [`PlacementConfig`] from spec clearance/optimize settings.
/// 5. Call [`solve_placement`].
/// 6. Rewrite the spec file with solved positions via [`spec_rewriter::rewrite_spec_with_placement`].
/// 7. Write the output (unless `dry_run`).
pub fn autoplace_spec(
    spec_path: &std::path::Path,
    config: &PlacementConfig,
    dry_run: bool,
    output_path: Option<&std::path::Path>,
) -> anyhow::Result<AutoplaceReport> {
    use autopcb_spec::model::SpecModel;

    let started = Instant::now();
    let spec_path_buf = spec_path.to_path_buf();
    info!(
        target: "altium_cli::placement",
        spec_path = %spec_path.display(),
        dry_run,
        has_output_override = output_path.is_some(),
        "autoplace_spec_started"
    );
    let source = std::fs::read_to_string(spec_path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", spec_path.display()))?;

    let compiled = compile_and_resolve(&source, &spec_path_buf, &SpecDomain::Pcb)?;
    let spec = match compiled.model {
        SpecModel::Pcb(s) => s,
        _ => anyhow::bail!("expected PcbDoc spec for {}", spec_path.display()),
    };

    let placement = spec.placement.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "spec {} has no placement {{ ... }} block",
            spec_path.display()
        )
    })?;

    let spec_dir = spec_path.parent().unwrap_or(std::path::Path::new("."));
    let ir = load_ir_from_spec(&spec, &compiled.imported_footprints, spec_dir)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    info!(
        target: "altium_cli::placement",
        component_count = ir.components.len(),
        net_count = ir.nets.len(),
        "autoplace_ir_loaded"
    );

    // Build PlacementConfig from spec settings, then overlay caller-provided config.
    let mut cfg = config.clone();
    if let Some(all) = placement.clearance.all {
        cfg.default_clearance_mm = all.to_mms();
    }
    if let Some(edge) = placement.clearance.edge {
        cfg.board_edge_clearance_mm = edge.to_mms();
    }
    for rule in &spec.placement_rules {
        if let (Some(kind), Some(gap)) = (&rule.kind, rule.gap) {
            match kind.as_str() {
                "component_clearance" => cfg.default_clearance_mm = gap.to_mms(),
                "board_outline_clearance" => cfg.board_edge_clearance_mm = gap.to_mms(),
                _ => {}
            }
        }
    }
    cfg.ratsnest_weight = placement.optimize.ratsnest_weight;

    if let Some(ref ac) = placement.autoplace_config {
        if let Some(gs) = ac.grid_snap {
            cfg.grid_snap_mm = Some(gs.to_mms());
        }
        if let Some(auto_cluster) = ac.auto_cluster {
            cfg.auto_cluster = auto_cluster;
        }
        if let Some(target_size) = ac.cluster_target_size {
            cfg.cluster_target_size = target_size.max(2);
        }
        if let Some(max_depth) = ac.cluster_max_depth {
            cfg.cluster_max_depth = max_depth.max(1);
        }
        if ac.sa_cooling.is_some()
            || ac.sa_moves_per_temp.is_some()
            || ac.sa_max_steps.is_some()
            || ac.congestion_weight.is_some()
            || ac.congestion_cell.is_some()
            || ac.critical_net_boost.is_some()
        {
            let sa_cfg = cfg
                .sa_config
                .get_or_insert_with(autopcb_placement::simulated_annealing::SAConfig::default);
            if let Some(cooling) = ac.sa_cooling {
                sa_cfg.cooling_rate = cooling;
            }
            if let Some(moves) = ac.sa_moves_per_temp {
                sa_cfg.moves_per_temp = moves;
            }
            if let Some(max_steps) = ac.sa_max_steps {
                sa_cfg.max_steps = max_steps;
            }
            if let Some(weight) = ac.congestion_weight {
                sa_cfg.congestion_weight = weight.max(0.0);
            }
            if let Some(cell) = ac.congestion_cell {
                sa_cfg.congestion_cell_mm = cell.to_mms().max(0.5);
            }
            if let Some(boost) = ac.critical_net_boost {
                sa_cfg.critical_net_boost = boost.max(1.0);
            }
        }
    }

    // Build constraints and collect autoplace designator list.
    let (user_constraints, autoplace_designators) =
        placement_bridge::placement_spec_to_constraints(placement, &ir)?;

    let component_count = ir.components.len();
    let autoplace_count = autoplace_designators.len();
    info!(
        target: "altium_cli::placement",
        component_count,
        autoplace_count,
        user_constraint_count = user_constraints.len(),
        placement_group_count = placement.groups.len(),
        sa_enabled = cfg.sa_config.is_some(),
        auto_cluster = cfg.auto_cluster,
        max_iters = cfg.max_iters,
        gamma_start = cfg.gamma_start,
        gamma_end = cfg.gamma_end,
        "autoplace_solver_configured"
    );
    debug!(
        target: "altium_cli::placement",
        ?autoplace_designators,
        "autoplace_designators_resolved"
    );

    // Run solver.
    let placement_groups: Vec<Vec<String>> = placement
        .groups
        .iter()
        .map(|group| group.components.clone())
        .collect();

    let solve_started = Instant::now();
    let result = solve_placement(&ir, &user_constraints, &cfg, &placement_groups)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    info!(
        target: "altium_cli::placement",
        duration_ms = solve_started.elapsed().as_millis(),
        status = %result.status,
        hpwl_mm = result.hpwl_estimate_mm,
        overlap_violations = result.overlap_violations,
        snapshot_count = result.snapshots.len(),
        "autoplace_solver_finished"
    );

    let hpwl_mm = result.hpwl_estimate_mm;
    let duration_ms = result.duration_ms;

    // Rewrite spec text.
    let rewrite_started = Instant::now();
    let rewrite =
        spec_rewriter::rewrite_spec_with_placement(&source, &result, &autoplace_designators)?;
    info!(
        target: "altium_cli::placement",
        duration_ms = rewrite_started.elapsed().as_millis(),
        rewritten_in_place_count = rewrite.rewritten_in_place.len(),
        appended_count = rewrite.appended.len(),
        "autoplace_spec_rewritten"
    );

    // Determine output path.
    let out_path = output_path
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| spec_path.to_path_buf());

    if !dry_run {
        std::fs::write(&out_path, &rewrite.text)
            .map_err(|e| anyhow::anyhow!("failed to write {}: {e}", out_path.display()))?;
    }

    info!(
        target: "altium_cli::placement",
        output_path = %out_path.display(),
        duration_ms = started.elapsed().as_millis(),
        "autoplace_spec_finished"
    );

    Ok(AutoplaceReport {
        hpwl_mm,
        component_count,
        autoplace_count,
        duration_ms,
        output_path: out_path,
    })
}

fn cmd_placement_dump(target: &std::path::Path) -> anyhow::Result<()> {
    let doc = PcbDoc::open(target)
        .map_err(|e| anyhow::anyhow!("failed to open {}: {e}", target.display()))?;
    let board = doc.board()?;
    let mut out = String::new();
    dump_placement_block(&mut out, &board);
    print!("{}", out);
    Ok(())
}

fn cmd_placement_plan(spec_file: &std::path::Path, target: &std::path::Path) -> anyhow::Result<()> {
    let spec_file_buf = spec_file.to_path_buf();
    let source = std::fs::read_to_string(spec_file)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", spec_file.display()))?;
    let compiled = compile_and_resolve(&source, &spec_file_buf, &SpecDomain::Pcb)?;
    let spec = match compiled.model {
        autopcb_spec::model::SpecModel::Pcb(s) => s,
        _ => anyhow::bail!("expected PcbDoc spec for {}", spec_file.display()),
    };
    let doc = PcbDoc::open(target)
        .map_err(|e| anyhow::anyhow!("failed to open {}: {e}", target.display()))?;
    let eco = reconcile_pcbdoc(&spec, &doc, target.to_path_buf(), spec_file.to_path_buf())
        .map_err(|e| anyhow::anyhow!("reconcile failed: {e}"))?;
    println!("{}", eco.render_text());
    Ok(())
}

fn cmd_placement_apply(
    spec_file: &std::path::Path,
    target: &std::path::Path,
) -> anyhow::Result<()> {
    let spec_file_buf = spec_file.to_path_buf();
    let source = std::fs::read_to_string(spec_file)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", spec_file.display()))?;
    let compiled = compile_and_resolve(&source, &spec_file_buf, &SpecDomain::Pcb)?;
    let spec = match compiled.model {
        autopcb_spec::model::SpecModel::Pcb(s) => s,
        _ => anyhow::bail!("expected PcbDoc spec for {}", spec_file.display()),
    };
    let mut doc = PcbDoc::open(target)
        .map_err(|e| anyhow::anyhow!("failed to open {}: {e}", target.display()))?;
    apply_spec_pcbdoc(&spec, &mut doc).map_err(|e| anyhow::anyhow!("apply failed: {e}"))?;

    // Apply placement positions from `place` entries to PcbDoc component records.
    if let Some(ref placement) = spec.placement {
        let mut board = doc
            .board()
            .map_err(|e| anyhow::anyhow!("failed to read board for placement: {e}"))?;
        let mut placed = 0usize;
        for place in &placement.places {
            if let Some(loc) = place.at {
                for designator in &place.designators {
                    // Capture old position before mutating.
                    let old_location;
                    let old_rotation;
                    {
                        let comp = board
                            .components
                            .iter()
                            .find(|c| &c.designator == designator);
                        let Some(comp) = comp else { continue };
                        old_location = comp.location;
                        old_rotation = comp.rotation;
                    }

                    // Compute translation delta.
                    use altium_format_types::coord::{Coord, CoordPoint};
                    let delta_x = loc.x - old_location.x;
                    let delta_y = loc.y - old_location.y;
                    let new_rotation = place.rotation.unwrap_or(old_rotation);
                    let delta_rotation = new_rotation - old_rotation;

                    // Update the component record itself.
                    {
                        let comp = board
                            .components
                            .iter_mut()
                            .find(|c| &c.designator == designator)
                            .expect("component must exist: checked above");
                        comp.location = loc;
                        comp.rotation = new_rotation;
                    }

                    // Translate (and optionally rotate) every primitive owned by
                    // this component.  Primitives store world-space absolute
                    // coordinates, so a component move requires updating all of
                    // them by the same delta.  When the rotation also changes,
                    // each primitive's position is rotated around the *new*
                    // component centre.
                    let translate_point = |p: CoordPoint| -> CoordPoint {
                        let mut q = CoordPoint::new(p.x + delta_x, p.y + delta_y);
                        if delta_rotation != 0.0 {
                            let angle_rad = delta_rotation.to_radians();
                            let (sin_a, cos_a) = angle_rad.sin_cos();
                            let rx = (q.x - loc.x).raw() as f64;
                            let ry = (q.y - loc.y).raw() as f64;
                            q = CoordPoint::new(
                                Coord::new(
                                    (loc.x.raw() as f64 + rx * cos_a - ry * sin_a).round() as i32,
                                ),
                                Coord::new(
                                    (loc.y.raw() as f64 + rx * sin_a + ry * cos_a).round() as i32,
                                ),
                            );
                        }
                        q
                    };

                    for pad in board.pads.iter_mut() {
                        if pad.component.as_deref() == Some(designator) {
                            pad.location = translate_point(pad.location);
                            pad.rotation += delta_rotation;
                        }
                    }
                    for track in board.tracks.iter_mut() {
                        if track.component.as_deref() == Some(designator) {
                            track.start = translate_point(track.start);
                            track.end = translate_point(track.end);
                        }
                    }
                    for arc in board.arcs.iter_mut() {
                        if arc.component.as_deref() == Some(designator) {
                            arc.center = translate_point(arc.center);
                        }
                    }
                    for fill in board.fills.iter_mut() {
                        if fill.component.as_deref() == Some(designator) {
                            fill.corner1 = translate_point(fill.corner1);
                            fill.corner2 = translate_point(fill.corner2);
                            fill.rotation += delta_rotation;
                        }
                    }
                    for text in board.texts.iter_mut() {
                        if text.component.as_deref() == Some(designator) {
                            text.location = translate_point(text.location);
                            text.rotation += delta_rotation;
                        }
                    }
                    for region in board.regions.iter_mut() {
                        if region.component.as_deref() == Some(designator) {
                            for v in region.outline.iter_mut() {
                                *v = translate_point(*v);
                            }
                            for hole in region.holes.iter_mut() {
                                for v in hole.iter_mut() {
                                    *v = translate_point(*v);
                                }
                            }
                        }
                    }
                    for body in board.component_bodies.iter_mut() {
                        if body.component.as_deref() == Some(designator) {
                            for v in body.outline.iter_mut() {
                                *v = translate_point(*v);
                            }
                        }
                    }

                    placed += 1;
                }
            }
        }
        if placed > 0 {
            doc.update_board(&board)
                .map_err(|e| anyhow::anyhow!("failed to update board with placement: {e}"))?;
            eprintln!("Placed {} component(s)", placed);
        }
    }

    doc.save(target)?;
    println!("Saved: {}", target.display());
    Ok(())
}

fn build_user_constraints(
    ir: &PcbIr,
    places: &[PlacementPlaceSpec],
    constraints: &[PlacementConstraintSpec],
) -> anyhow::Result<Vec<UserConstraint>> {
    let mut out = Vec::new();

    for place in places {
        for d in &place.designators {
            if let Some(edge) = &place.edge {
                let edge = parse_edge(edge)
                    .ok_or_else(|| anyhow::anyhow!("invalid edge value '{edge}' for place {d}"))?;
                out.push(UserConstraint::EdgePlacement {
                    designator: d.clone(),
                    edge,
                    inset_mm: place.inset.map(|v| v.to_mms()).unwrap_or(0.0),
                });
            }

            if let (Some(near), Some(max_dist)) = (&place.near, place.max_distance) {
                out.push(UserConstraint::Near {
                    a: d.clone(),
                    b: near.clone(),
                    max_distance_mm: max_dist.to_mms(),
                });
            }

            if let Some(region) = &place.region_name {
                if let Some(rr) = named_region_from_board(ir, region) {
                    out.push(UserConstraint::RegionContainment {
                        designator: d.clone(),
                        region: rr,
                    });
                }
            }

            if let Some((from, to)) = place.region_rect {
                out.push(UserConstraint::RegionContainment {
                    designator: d.clone(),
                    region: RectRegion {
                        min_x: from.x.to_mms(),
                        min_y: from.y.to_mms(),
                        max_x: to.x.to_mms(),
                        max_y: to.y.to_mms(),
                    },
                });
            }

            if place.fixed {
                if let Some(at) = place.at {
                    out.push(UserConstraint::FixedPosition {
                        designator: d.clone(),
                        x_mm: at.x.to_mms(),
                        y_mm: at.y.to_mms(),
                        rotation_deg: place.rotation,
                    });
                }
            } else if let Some(at) = place.at {
                out.push(UserConstraint::FixedPosition {
                    designator: d.clone(),
                    x_mm: at.x.to_mms(),
                    y_mm: at.y.to_mms(),
                    rotation_deg: place.rotation,
                });
            }
        }
    }

    for c in constraints {
        match c {
            PlacementConstraintSpec::LeftOf { a, b, gap } => {
                out.push(UserConstraint::Directional {
                    a: a.clone(),
                    b: b.clone(),
                    direction: Direction::LeftOf,
                    gap_mm: gap.map(|v| v.to_mms()).unwrap_or(0.0),
                })
            }
            PlacementConstraintSpec::RightOf { a, b, gap } => {
                out.push(UserConstraint::Directional {
                    a: a.clone(),
                    b: b.clone(),
                    direction: Direction::RightOf,
                    gap_mm: gap.map(|v| v.to_mms()).unwrap_or(0.0),
                })
            }
            PlacementConstraintSpec::Above { a, b, gap } => out.push(UserConstraint::Directional {
                a: a.clone(),
                b: b.clone(),
                direction: Direction::Above,
                gap_mm: gap.map(|v| v.to_mms()).unwrap_or(0.0),
            }),
            PlacementConstraintSpec::Below { a, b, gap } => out.push(UserConstraint::Directional {
                a: a.clone(),
                b: b.clone(),
                direction: Direction::Below,
                gap_mm: gap.map(|v| v.to_mms()).unwrap_or(0.0),
            }),
        }
    }

    Ok(out)
}

fn parse_edge(s: &str) -> Option<PlacementEdge> {
    match s {
        "top" => Some(PlacementEdge::Top),
        "bottom" => Some(PlacementEdge::Bottom),
        "left" => Some(PlacementEdge::Left),
        "right" => Some(PlacementEdge::Right),
        _ => None,
    }
}

// ── routing ───────────────────────────────────────────────────────────────────

fn run_routing(sub: RoutingSubcommand) -> anyhow::Result<()> {
    match sub {
        RoutingSubcommand::Inspect { path, verbose, json } => {
            cmd_routing_inspect(&path, verbose, json)
        }
        RoutingSubcommand::Solve { spec_file, output, json } => {
            cmd_routing_solve(&spec_file, output.as_deref(), json)
        }
    }
}

fn cmd_routing_inspect(path: &std::path::Path, verbose: bool, json: bool) -> anyhow::Result<()> {
    let solution = autopcb_routes::load_binary(path)
        .or_else(|_| autopcb_routes::load_json(path))
        .map_err(|e| {
            anyhow::anyhow!("failed to load routes file {}: {e}", path.display())
        })?;

    if json {
        let out = serde_json::json!({
            "path": path.display().to_string(),
            "version": solution.version,
            "nets_routed": solution.nets.len(),
            "nets_unrouted": solution.unrouted.len(),
            "iterations": solution.iterations.len(),
            "metrics": {
                "total_vias": solution.metrics.total_vias,
                "total_length_mm": solution.metrics.total_length_mm,
                "completion_pct": solution.metrics.completion_pct,
                "drc_violations": solution.metrics.drc_violations,
            },
            "drc_violation_records": solution.drc_violation_records.iter().map(|v| {
                serde_json::json!({
                    "kind": v.kind_name,
                    "location": { "x": v.location.x, "y": v.location.y },
                    "layer": v.layer,
                    "actual_mm": v.actual_mm,
                    "required_mm": v.required_mm,
                    "rule_name": v.rule_name,
                })
            }).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    let m = &solution.metrics;
    let net_count = solution.nets.len();
    let unrouted_count = solution.unrouted.len();
    let iteration_count = solution.iterations.len();

    println!("Routes file: {}", path.display());
    println!("  Version:        {}", solution.version);
    println!("  Nets routed:    {}", net_count);
    println!("  Nets unrouted:  {}", unrouted_count);
    println!("  Total vias:     {}", m.total_vias);
    println!("  Total length:   {:.4} mm", m.total_length_mm);
    println!("  Completion:     {:.1}%", m.completion_pct);
    println!("  DRC violations: {} (from stored records)", m.drc_violations);
    println!("  Iterations:     {}", iteration_count);

    if !solution.unrouted.is_empty() {
        println!();
        println!("Unrouted nets:");
        for net_id in &solution.unrouted {
            println!("  net {}", net_id.raw());
        }
    }

    if !solution.drc_violation_records.is_empty() {
        println!();
        if verbose {
            println!("DRC Violations:");
            for (i, v) in solution.drc_violation_records.iter().enumerate() {
                println!(
                    "  #{}: {} at ({:.4}, {:.4}){} — actual: {:.4} mm, required: {:.4} mm [{}]",
                    i + 1,
                    v.kind_name,
                    v.location.x,
                    v.location.y,
                    v.layer.map(|l| format!(" layer {}", l)).unwrap_or_default(),
                    v.actual_mm,
                    v.required_mm,
                    v.rule_name,
                );
            }
        } else {
            println!("DRC Violations (use --verbose for details):");
            for (i, v) in solution.drc_violation_records.iter().enumerate() {
                println!(
                    "  #{}: {} at ({:.4}, {:.4}) [{}]",
                    i + 1,
                    v.kind_name,
                    v.location.x,
                    v.location.y,
                    v.rule_name,
                );
            }
        }
    }

    Ok(())
}

fn cmd_routing_solve(
    spec_file: &std::path::Path,
    output: Option<&std::path::Path>,
    json: bool,
) -> anyhow::Result<()> {
    use autopcb_spec::model::SpecModel;

    let source = std::fs::read_to_string(spec_file)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", spec_file.display()))?;
    let spec_file_buf = spec_file.to_path_buf();
    let result = compile_and_resolve(&source, &spec_file_buf, &SpecDomain::Pcb)?;
    let spec = match result.model {
        SpecModel::Pcb(s) => s,
        _ => anyhow::bail!("routing solve requires a .pcb file"),
    };

    let spec_dir = spec_file.parent().unwrap_or(std::path::Path::new("."));
    let ir = load_ir_from_spec(&spec, &result.imported_footprints, spec_dir)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let config = autopcb_router::RoutingConfig::default();
    let workspace = autopcb_router::build_workspace(&ir, &config)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let solution = autopcb_router::route_board(&workspace, &ir, &config)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let output_path = output
        .map(|p: &std::path::Path| p.to_path_buf())
        .unwrap_or_else(|| spec_file.with_extension("routes"));
    autopcb_routes::save_binary(&solution, &output_path)
        .map_err(|e| anyhow::anyhow!("failed to save {}: {e}", output_path.display()))?;

    if json {
        println!("{}", serde_json::to_string_pretty(&solution.metrics)?);
    } else {
        println!("ROUTING REPORT");
        println!("  output: {}", output_path.display());
        println!(
            "  nets routed: {}/{}",
            solution.nets.len(),
            solution.nets.len() + solution.unrouted.len()
        );
        println!("  unrouted: {}", solution.unrouted.len());
        println!("  total length: {:.2} mm", solution.metrics.total_length_mm);
        println!("  total vias: {}", solution.metrics.total_vias);
        println!("  completion: {:.1}%", solution.metrics.completion_pct);
        println!("  DRC violations: {}", solution.metrics.drc_violations);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placement_autoplace_enables_sa_by_default() {
        let cli = Cli::try_parse_from(["altium", "placement", "autoplace", "board.pcb"])
            .expect("autoplace args should parse");
        match cli.command {
            Commands::Placement {
                sub: PlacementSubcommand::Autoplace { sa, spec_file, .. },
            } => {
                assert!(sa, "SA should be enabled by default");
                assert_eq!(spec_file, PathBuf::from("board.pcb"));
            }
            _ => panic!("expected placement autoplace command"),
        }
    }

    #[test]
    fn placement_autoplace_accepts_no_sa_override() {
        let cli = Cli::try_parse_from([
            "altium",
            "placement",
            "autoplace",
            "board.pcb",
            "--no-sa",
        ])
        .expect("autoplace args should parse");
        match cli.command {
            Commands::Placement {
                sub: PlacementSubcommand::Autoplace { sa, .. },
            } => assert!(!sa, "--no-sa should disable SA"),
            _ => panic!("expected placement autoplace command"),
        }
    }

    #[test]
    fn routing_inspect_parses_path_arg() {
        let cli = Cli::try_parse_from(["altium", "routing", "inspect", "board.routes"])
            .expect("routing inspect args should parse");
        match cli.command {
            Commands::Routing {
                sub: RoutingSubcommand::Inspect { path, verbose, json },
            } => {
                assert_eq!(path, PathBuf::from("board.routes"));
                assert!(!verbose);
                assert!(!json);
            }
            _ => panic!("expected routing inspect command"),
        }
    }

    #[test]
    fn routing_inspect_loads_binary_routes_file() {
        use autopcb_routes::{NetId, RouteSolution, save_binary};

        let solution = RouteSolution::new();
        let tmp = tempfile::NamedTempFile::new().expect("tempfile");
        save_binary(&solution, tmp.path()).expect("save_binary");
        cmd_routing_inspect(tmp.path(), false, false).expect("cmd_routing_inspect should succeed");
    }

    #[test]
    fn routing_inspect_loads_json_routes_file() {
        use autopcb_routes::{RouteSolution, save_json};

        let solution = RouteSolution::new();
        let tmp = tempfile::NamedTempFile::new().expect("tempfile");
        save_json(&solution, tmp.path()).expect("save_json");
        cmd_routing_inspect(tmp.path(), false, false).expect("cmd_routing_inspect should succeed on JSON");
    }

    #[test]
    fn routing_solve_missing_spec_returns_error() {
        let result = cmd_routing_solve(
            std::path::Path::new("nonexistent.pcb"),
            None,
            false,
        );
        assert!(result.is_err(), "routing solve with missing spec file should return an error");
    }

    #[test]
    fn routing_inspect_shows_zero_drc_violations() {
        use autopcb_routes::{RouteSolution, save_binary};

        let solution = RouteSolution::new();
        assert_eq!(solution.metrics.drc_violations, 0);
        assert!(solution.drc_violation_records.is_empty());

        let tmp = tempfile::NamedTempFile::new().expect("tempfile");
        save_binary(&solution, tmp.path()).expect("save_binary");
        cmd_routing_inspect(tmp.path(), false, false).expect("inspect with 0 violations should succeed");
    }

    #[test]
    fn routing_inspect_shows_drc_violation_records() {
        use autopcb_routes::{DrcViolationRecord, Point, RouteSolution, save_binary};

        let mut solution = RouteSolution::new();
        solution.drc_violation_records.push(DrcViolationRecord {
            kind_name: "ClearanceViolation".to_string(),
            location: Point { x: 1.2345, y: 6.7890 },
            layer: Some(1),
            actual_mm: 0.05,
            required_mm: 0.1,
            rule_name: "Clearance_default".to_string(),
        });
        solution.metrics.drc_violations = 1;

        let tmp = tempfile::NamedTempFile::new().expect("tempfile");
        save_binary(&solution, tmp.path()).expect("save_binary");
        cmd_routing_inspect(tmp.path(), false, false).expect("inspect with violations should succeed");
    }
}
