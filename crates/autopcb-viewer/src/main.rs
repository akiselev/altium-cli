//! AutoPCB Viewer — standalone binary for visualising PCB IR data.
//!
//! Usage: autopcb-viewer <path-to-pcbdoc-or-spec> [--target <pcbdoc>] [--screenshot <output.png>] [--playback <iterations.json>] [--watch]

mod app;
mod colors;
mod interaction;
mod renderer;
mod view3d;

use std::path::PathBuf;
use std::sync::{Arc, Mutex, mpsc};

use altium_format::PcbDoc;
use autopcb_ir::{PcbIr, PointMm};
use autopcb_placement::PlacementIterationSnapshot;

/// Parse a `.pcbdoc-spec` file and return the resolved PcbDoc path and any
/// `at:` position overrides keyed by designator.
///
/// Returns `(pcbdoc_path, positions)` where `positions` maps designator strings
/// to `(x_mm, y_mm)` pairs sourced from `placement { places { at: ... } }`.
fn load_spec(
    spec_path: &std::path::Path,
    explicit_target: Option<&std::path::Path>,
) -> anyhow::Result<(PathBuf, Vec<(String, f64, f64)>)> {
    use altium_format_spec::{compile_spec, SpecDomain, SpecModel};
    use altium_format_spec::parser::parse_spec;

    let source = std::fs::read_to_string(spec_path)
        .map_err(|e| anyhow::anyhow!("failed to read spec {}: {e}", spec_path.display()))?;

    let ast = parse_spec(&source)
        .map_err(|e| anyhow::anyhow!("failed to parse spec {}: {e:?}", spec_path.display()))?;

    let model = compile_spec(&ast, SpecDomain::PcbDoc)
        .map_err(|e| anyhow::anyhow!("failed to compile spec {}: {e:?}", spec_path.display()))?;

    let pcbdoc_spec = match model {
        SpecModel::PcbDoc(s) => s,
        _ => anyhow::bail!("spec file does not describe a PcbDoc"),
    };

    // Resolve the target PcbDoc path.
    let pcbdoc_path = if let Some(explicit) = explicit_target {
        explicit.to_path_buf()
    } else {
        let target_str = pcbdoc_spec
            .placement
            .as_ref()
            .and_then(|p| p.target.as_ref())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "spec has no `target:` in placement block and no --target flag was given"
                )
            })?;
        // Resolve relative to the spec file's parent directory.
        let base = spec_path.parent().unwrap_or(std::path::Path::new("."));
        base.join(target_str)
    };

    // Collect `at:` position overrides from placement places.
    let mut positions: Vec<(String, f64, f64)> = Vec::new();
    if let Some(placement) = &pcbdoc_spec.placement {
        for place in &placement.places {
            if let Some(at) = &place.at {
                let x_mm = at.x.to_mms();
                let y_mm = at.y.to_mms();
                for designator in &place.designators {
                    positions.push((designator.clone(), x_mm, y_mm));
                }
            }
        }
    }

    Ok((pcbdoc_path, positions))
}

/// Apply spec position overrides to an IR in-place.
fn apply_spec_positions(ir: &mut PcbIr, positions: &[(String, f64, f64)]) {
    for (designator, x_mm, y_mm) in positions {
        for (_id, comp) in ir.components.iter_mut() {
            if &comp.designator == designator {
                comp.position = PointMm::new(*x_mm, *y_mm);

                let theta = comp.rotation.to_radians();
                let (sin_t, cos_t) = theta.sin_cos();
                for pad in &mut comp.pads {
                    let lx = pad.local_position.x;
                    let ly = pad.local_position.y;
                    pad.world_position = PointMm::new(
                        x_mm + lx * cos_t - ly * sin_t,
                        y_mm + lx * sin_t + ly * cos_t,
                    );
                }

                let lb = comp.local_bounds;
                let corners = [
                    PointMm::new(lb.min.x, lb.min.y),
                    PointMm::new(lb.min.x, lb.max.y),
                    PointMm::new(lb.max.x, lb.min.y),
                    PointMm::new(lb.max.x, lb.max.y),
                ];
                let mut world_pts = Vec::with_capacity(4);
                for c in corners {
                    world_pts.push(PointMm::new(
                        x_mm + c.x * cos_t - c.y * sin_t,
                        y_mm + c.x * sin_t + c.y * cos_t,
                    ));
                }
                if let Some(bb) = autopcb_ir::BoundingBoxMm::from_points(&world_pts) {
                    comp.world_bounds = bb;
                }
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let path = match args.next() {
        Some(p) => PathBuf::from(p),
        None => {
            eprintln!("Usage: autopcb-viewer <path-to-pcbdoc-or-spec> [--target <pcbdoc>] [--screenshot <output.png>] [--playback <iterations.json>] [--watch]");
            std::process::exit(1);
        }
    };

    let mut screenshot_path: Option<PathBuf> = None;
    let mut playback_path: Option<PathBuf> = None;
    let mut explicit_target: Option<PathBuf> = None;
    let mut watch = false;
    while let Some(arg) = args.next() {
        if arg == "--screenshot" {
            match args.next() {
                Some(p) => screenshot_path = Some(PathBuf::from(p)),
                None => {
                    eprintln!("--screenshot requires a path argument");
                    std::process::exit(1);
                }
            }
        } else if arg == "--playback" {
            match args.next() {
                Some(p) => playback_path = Some(PathBuf::from(p)),
                None => {
                    eprintln!("--playback requires a path argument");
                    std::process::exit(1);
                }
            }
        } else if arg == "--target" {
            match args.next() {
                Some(p) => explicit_target = Some(PathBuf::from(p)),
                None => {
                    eprintln!("--target requires a path argument");
                    std::process::exit(1);
                }
            }
        } else if arg == "--watch" {
            watch = true;
        }
    }

    // Detect whether the input is a spec file.
    let path_str = path.to_string_lossy();
    let is_spec = path_str.ends_with(".pcbdoc-spec") || path_str.ends_with("-spec");

    let (pcbdoc_path, spec_path, spec_positions) = if is_spec {
        eprintln!("Loading spec {}...", path.display());
        let (pcbdoc, positions) = load_spec(&path, explicit_target.as_deref())?;
        (pcbdoc, Some(path.clone()), positions)
    } else {
        (path.clone(), None, Vec::new())
    };

    eprintln!("Opening {}...", pcbdoc_path.display());
    let doc = PcbDoc::open(&pcbdoc_path)?;

    eprintln!("Extracting board...");
    let board = doc.board()?;

    eprintln!("Building IR...");
    let mut ir = PcbIr::extract(&board).map_err(|e| anyhow::anyhow!("{e}"))?;

    if !spec_positions.is_empty() {
        eprintln!("Applying {} spec position overrides...", spec_positions.len());
        apply_spec_positions(&mut ir, &spec_positions);
    }

    eprintln!(
        "Board: {:.1} x {:.1} mm, {} components, {} nets",
        ir.board.bounds.width(),
        ir.board.bounds.height(),
        ir.components.len(),
        ir.nets.len()
    );

    let title = format!(
        "AutoPCB Viewer — {}",
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("board")
    );

    let ir = Arc::new(Mutex::new(ir));
    let app_ir = Arc::clone(&ir);
    let playback: Option<Vec<PlacementIterationSnapshot>> = if let Some(ref pb_path) = playback_path {
        let source = std::fs::read_to_string(pb_path)
            .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", pb_path.display()))?;
        Some(
            serde_json::from_str(&source)
                .map_err(|e| anyhow::anyhow!("failed to parse playback {}: {e}", pb_path.display()))?,
        )
    } else {
        None
    };

    // Set up file watcher if --watch was requested.
    // The watcher is kept alive for the duration of the program by binding it here.
    let watch_rx: Option<mpsc::Receiver<notify::Result<notify::Event>>> = if watch {
        use notify::Watcher;

        let (tx, rx) = mpsc::channel();
        let mut watcher = notify::RecommendedWatcher::new(tx, notify::Config::default())
            .map_err(|e| anyhow::anyhow!("failed to create file watcher: {e}"))?;

        // In spec mode, watch the spec file; its changes trigger a full reload
        // (re-parse spec + re-open PcbDoc). Also watch the PcbDoc itself.
        if let Some(ref sp) = spec_path {
            watcher
                .watch(sp, notify::RecursiveMode::NonRecursive)
                .map_err(|e| anyhow::anyhow!("failed to watch {}: {e}", sp.display()))?;
            eprintln!("Watching {} for changes...", sp.display());
        }

        watcher
            .watch(&pcbdoc_path, notify::RecursiveMode::NonRecursive)
            .map_err(|e| anyhow::anyhow!("failed to watch {}: {e}", pcbdoc_path.display()))?;

        if let Some(ref pb_path) = playback_path {
            watcher
                .watch(pb_path, notify::RecursiveMode::NonRecursive)
                .map_err(|e| anyhow::anyhow!("failed to watch {}: {e}", pb_path.display()))?;
        }

        eprintln!("Watching {} for changes...", pcbdoc_path.display());

        // Leak the watcher so it stays alive for the lifetime of the process.
        // eframe takes ownership of the app and there is no clean shutdown hook,
        // so leaking is the simplest way to keep the OS watch handle open.
        std::mem::forget(watcher);

        Some(rx)
    } else {
        None
    };

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title(&title),
        renderer: eframe::Renderer::Wgpu,
        depth_buffer: 24,
        multisampling: 0,
        ..Default::default()
    };

    let pb_path_clone = playback_path.clone();

    eframe::run_native(
        &title,
        options,
        Box::new(move |cc| {
            Ok(Box::new(app::ViewerApp::new(
                app_ir,
                screenshot_path,
                playback.clone(),
                watch_rx,
                pcbdoc_path,
                pb_path_clone,
                spec_path,
                explicit_target,
                cc,
            )))
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {e}"))?;

    Ok(())
}
