mod app;
mod canvas;
mod commands;
mod jobs;
mod layout;
mod workbench;

use std::path::PathBuf;

use altium_format::PcbDoc;
use autopcb_ir::PcbIr;

use app::ShellApp;

fn main() -> anyhow::Result<()> {
    let board_path = std::env::args().nth(1).map(PathBuf::from);

    let initial_ir = if let Some(path) = &board_path {
        let doc = PcbDoc::open(path)?;
        let board = doc.board()?;
        Some(PcbIr::extract(&board).map_err(|e| anyhow::anyhow!("{e}"))?)
    } else {
        None
    };

    let title = "AutoPCB Shell";
    let options = efame::NativeOptions {
        viewport: efame::egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_title(title),
        renderer: efame::Renderer::Wgpu,
        ..Default::default()
    };

    efame::run_native(
        title,
        options,
        Box::new(move |cc| Ok(Box::new(ShellApp::new(cc, board_path, initial_ir)))),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {e}"))?;

    Ok(())
}
