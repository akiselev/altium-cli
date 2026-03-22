//! AutoPCB Viewer — standalone binary for visualising PCB IR data.
//!
//! The viewer is spec-centric: it ONLY accepts `.pcbdoc-spec` files as input.
//! The underlying PcbDoc is loaded and mutated by the spec pipeline internally.
//!
//! Usage: autopcb-viewer <path-to-pcbdoc-spec> [--screenshot <output.png>] [--playback <iterations.json>] [--watch]

mod app;
mod colors;
mod interaction;
mod renderer;
mod view3d;

use std::path::PathBuf;
use std::sync::{Arc, Mutex, mpsc};

use autopcb_ir::PcbIr;
use autopcb_placement::PlacementIterationSnapshot;

/// Compile a `.pcbdoc-spec` file and produce a `PcbIr` with all spec mutations applied.
pub(crate) fn load_spec_ir(spec_path: &std::path::Path) -> anyhow::Result<PcbIr> {
    use altium_format_spec::parser::parse_spec;
    use altium_format_spec::{SpecDomain, SpecModel, compile_spec};
    use autopcb_ir::load_ir_from_spec;

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

    let spec_dir = spec_path.parent().unwrap_or(std::path::Path::new("."));
    let ir = load_ir_from_spec(&pcbdoc_spec, spec_dir)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    Ok(ir)
}

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let path = match args.next() {
        Some(p) => PathBuf::from(p),
        None => {
            eprintln!(
                "Usage: autopcb-viewer <path-to-pcbdoc-spec> [--screenshot <output.png>] [--playback <iterations.json>] [--watch]"
            );
            std::process::exit(1);
        }
    };

    let mut screenshot_path: Option<PathBuf> = None;
    let mut playback_path: Option<PathBuf> = None;
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
        } else if arg == "--watch" {
            watch = true;
        }
    }

    // The viewer only accepts spec files.
    let path_str = path.to_string_lossy();
    if !path_str.ends_with(".pcbdoc-spec") {
        eprintln!(
            "Error: autopcb-viewer requires a .pcbdoc-spec file as input.\n\
             Got: {}",
            path.display()
        );
        std::process::exit(1);
    }

    eprintln!("Loading spec {}...", path.display());
    let ir = load_spec_ir(&path)?;

    eprintln!(
        "Board: {:.1} x {:.1} mm, {} components, {} nets",
        ir.board.bounds.width(),
        ir.board.bounds.height(),
        ir.components.len(),
        ir.nets.len()
    );

    let title = format!(
        "AutoPCB Viewer — {}",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("board")
    );

    let ir = Arc::new(Mutex::new(ir));
    let app_ir = Arc::clone(&ir);
    let playback: Option<Vec<PlacementIterationSnapshot>> =
        if let Some(ref pb_path) = playback_path {
            let source = std::fs::read_to_string(pb_path)
                .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", pb_path.display()))?;
            Some(serde_json::from_str(&source).map_err(|e| {
                anyhow::anyhow!("failed to parse playback {}: {e}", pb_path.display())
            })?)
        } else {
            None
        };

    // Set up file watcher if --watch was requested.
    let watch_rx: Option<mpsc::Receiver<notify::Result<notify::Event>>> = if watch {
        use notify::Watcher;

        let (tx, rx) = mpsc::channel();
        let mut watcher = notify::RecommendedWatcher::new(tx, notify::Config::default())
            .map_err(|e| anyhow::anyhow!("failed to create file watcher: {e}"))?;

        // Watch the spec file.
        watcher
            .watch(&path, notify::RecursiveMode::NonRecursive)
            .map_err(|e| anyhow::anyhow!("failed to watch {}: {e}", path.display()))?;
        eprintln!("Watching {} for changes...", path.display());

        if let Some(ref pb_path) = playback_path {
            watcher
                .watch(pb_path, notify::RecursiveMode::NonRecursive)
                .map_err(|e| anyhow::anyhow!("failed to watch {}: {e}", pb_path.display()))?;
        }

        // Leak the watcher so it stays alive for the lifetime of the process.
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
    let spec_path = path.clone();

    eframe::run_native(
        &title,
        options,
        Box::new(move |cc| {
            Ok(Box::new(app::ViewerApp::new(
                app_ir,
                screenshot_path,
                playback.clone(),
                watch_rx,
                pb_path_clone,
                spec_path,
                cc,
            )))
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {e}"))?;

    Ok(())
}
