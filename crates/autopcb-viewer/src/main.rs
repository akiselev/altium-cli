//! AutoPCB Viewer — standalone binary for visualising PCB IR data.
//!
//! Usage: autopcb-viewer <path-to-pcbdoc> [--screenshot <output.png>] [--playback <iterations.json>]

mod app;
mod colors;
mod interaction;
mod renderer;
mod view3d;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use altium_format::PcbDoc;
use autopcb_ir::PcbIr;
use autopcb_placement::PlacementIterationSnapshot;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let path = match args.next() {
        Some(p) => PathBuf::from(p),
        None => {
            eprintln!("Usage: autopcb-viewer <path-to-pcbdoc> [--screenshot <output.png>] [--playback <iterations.json>]");
            std::process::exit(1);
        }
    };

    let mut screenshot_path: Option<PathBuf> = None;
    let mut playback_path: Option<PathBuf> = None;
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
        }
    }

    eprintln!("Opening {}...", path.display());
    let doc = PcbDoc::open(&path)?;

    eprintln!("Extracting board...");
    let board = doc.board()?;

    eprintln!("Building IR...");
    let ir = PcbIr::extract(&board).map_err(|e| anyhow::anyhow!("{e}"))?;

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
    let playback: Option<Vec<PlacementIterationSnapshot>> = if let Some(path) = playback_path {
        let source = std::fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", path.display()))?;
        Some(
            serde_json::from_str(&source)
                .map_err(|e| anyhow::anyhow!("failed to parse playback {}: {e}", path.display()))?,
        )
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

    eframe::run_native(
        &title,
        options,
        Box::new(move |cc| Ok(Box::new(app::ViewerApp::new(app_ir, screenshot_path, playback.clone(), cc)))),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {e}"))?;

    Ok(())
}
