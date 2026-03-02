//! AutoPCB Viewer — standalone binary for visualising PCB IR data.
//!
//! Usage: autopcb-viewer <path-to-pcbdoc>

mod app;
mod colors;
mod interaction;
mod renderer;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use altium_format::PcbDoc;
use autopcb_ir::PcbIr;

fn main() -> anyhow::Result<()> {
    let path = match std::env::args().nth(1) {
        Some(p) => PathBuf::from(p),
        None => {
            eprintln!("Usage: autopcb-viewer <path-to-pcbdoc>");
            std::process::exit(1);
        }
    };

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

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title(&title),
        ..Default::default()
    };

    eframe::run_native(
        &title,
        options,
        Box::new(move |_cc| Ok(Box::new(app::ViewerApp::new(app_ir)))),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {e}"))?;

    Ok(())
}
