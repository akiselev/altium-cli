// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! PcbLib rendering commands: ASCII, SVG, PNG.

use std::path::{Path, PathBuf};

use altium_format::coord::AltiumCoord;

use super::{extract_pads, find_footprint_by_name, open_pcblib};

/// Render footprint as ASCII art.
pub fn cmd_render_ascii(
    path: &Path,
    footprint: &str,
    width: u32,
    height: u32,
) -> crate::Result<()> {
    let lib = open_pcblib(path)?;
    let fp = find_footprint_by_name(&lib, footprint)?;

    let fp_name = fp.name()?;
    let pads = extract_pads(&fp)?;
    if pads.is_empty() {
        println!("Footprint '{}' has no pads to render.", fp_name);
        return Ok(());
    }

    // Compute bounds
    let mut min_x = i32::MAX;
    let mut max_x = i32::MIN;
    let mut min_y = i32::MAX;
    let mut max_y = i32::MIN;

    for pad in &pads {
        let pos_x = pad.record.position_x().to_raw();
        let pos_y = pad.record.position_y().to_raw();
        let half_x = pad.record.top_size_x().to_raw() / 2;
        let half_y = pad.record.top_size_y().to_raw() / 2;
        min_x = min_x.min(pos_x - half_x);
        max_x = max_x.max(pos_x + half_x);
        min_y = min_y.min(pos_y - half_y);
        max_y = max_y.max(pos_y + half_y);
    }

    // Add margin
    let margin_x = (max_x - min_x) / 10;
    let margin_y = (max_y - min_y) / 10;
    min_x -= margin_x;
    max_x += margin_x;
    min_y -= margin_y;
    max_y += margin_y;

    let range_x = (max_x - min_x) as f64;
    let range_y = (max_y - min_y) as f64;

    if range_x <= 0.0 || range_y <= 0.0 {
        println!("Footprint '{}' has zero extent.", fp_name);
        return Ok(());
    }

    let w = width as usize;
    let h = height as usize;

    let mut grid = vec![vec![' '; w]; h];

    for pad in &pads {
        let pos_x = pad.record.position_x().to_raw();
        let pos_y = pad.record.position_y().to_raw();
        let size_x = pad.record.top_size_x().to_raw();
        let size_y = pad.record.top_size_y().to_raw();

        let cx = ((pos_x - min_x) as f64 / range_x * (w - 1) as f64) as usize;
        let cy = h - 1 - ((pos_y - min_y) as f64 / range_y * (h - 1) as f64) as usize;

        let half_w = (size_x as f64 / range_x * (w - 1) as f64 / 2.0).max(0.5) as usize;
        let half_h = (size_y as f64 / range_y * (h - 1) as f64 / 2.0).max(0.5) as usize;

        let x_start = cx.saturating_sub(half_w);
        let x_end = (cx + half_w).min(w - 1);
        let y_start = cy.saturating_sub(half_h);
        let y_end = (cy + half_h).min(h - 1);

        let ch = if pad.is_smd() { '#' } else { 'O' };
        for gy in y_start..=y_end {
            for gx in x_start..=x_end {
                grid[gy][gx] = ch;
            }
        }

        // Try to place designator
        if !pad.designator.is_empty() && cx < w {
            let label_chars: Vec<char> = pad.designator.chars().collect();
            let label_start = cx.saturating_sub(label_chars.len() / 2);
            for (ci, &lc) in label_chars.iter().enumerate() {
                let lx = label_start + ci;
                if lx < w && cy < h {
                    grid[cy][lx] = lc;
                }
            }
        }
    }

    println!("Footprint: {} ({}x{} ASCII)", fp_name, w, h);
    println!("  # = SMD pad, O = TH pad");
    let border: String = std::iter::repeat('+').take(w + 2).collect();
    println!("{}", border);
    for row in &grid {
        let line: String = row.iter().collect();
        println!("|{}|", line);
    }
    println!("{}", border);

    Ok(())
}

/// Render footprint as SVG.
pub fn cmd_render_svg(
    _path: &Path,
    _footprint: &str,
    _output: Option<PathBuf>,
    _scale: f64,
    _light: bool,
    _no_grid: bool,
    _no_designators: bool,
) -> crate::Result<()> {
    Err(crate::AltiumOpsError::NotImplemented("SVG rendering is not yet implemented in the v2 API. \
         Use cmd_render_ascii for a quick text-mode preview.".to_string()))
}

/// Render footprint as PNG.
pub fn cmd_render_png(
    _path: &Path,
    _footprint: &str,
    _output: Option<PathBuf>,
    _scale: f64,
    _width: Option<u32>,
) -> crate::Result<()> {
    Err(crate::AltiumOpsError::NotImplemented("PNG rendering is not yet implemented in the v2 API. \
         Use cmd_render_ascii for a quick text-mode preview.".to_string()))
}
