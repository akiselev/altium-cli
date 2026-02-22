// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! PcbLib mutation commands: create, add footprint/pad, chip generation, JSON import,
//! pad patterns (row, dual row, quad, grid).

use std::path::Path;

use altium_format::coord::{AltiumCoord, PcbCoord};
use altium_format::records::PcbPadRecord;

use crate::helpers::*;

use super::{find_footprint_by_name, mm_to_raw, open_pcblib};

/// Embedded blank PcbLib template.
const BLANK_PCBLIB_TEMPLATE: &[u8] =
    include_bytes!("../../../altium-format/data/blank/PcbLib1.PcbLib");

/// Build a typed pad record from parameters.
fn build_pad_record(
    x: PcbCoord,
    y: PcbCoord,
    w: PcbCoord,
    h: PcbCoord,
    hole: PcbCoord,
    shape: u8,
    layer: u8,
) -> PcbPadRecord {
    let origin = altium_format::templates::pcb_pad_default();
    let mut pad = PcbPadRecord::from_origin(origin);
    pad.set_position_x(x);
    pad.set_position_y(y);
    pad.set_top_size_x(w);
    pad.set_top_size_y(h);
    pad.set_mid_size_x(w);
    pad.set_mid_size_y(h);
    pad.set_bot_size_x(w);
    pad.set_bot_size_y(h);
    pad.set_hole_size(hole);
    pad.set_top_shape(shape);
    pad.set_mid_shape(shape);
    pad.set_bot_shape(shape);
    pad.set_is_plated(hole.to_raw() > 0);
    pad.set_layer(layer);
    pad
}

/// Creates an empty PcbLib file at the given path.
pub fn cmd_create(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if path.exists() {
        return Err(format!("File already exists: {}", path.display()).into());
    }

    std::fs::write(path, BLANK_PCBLIB_TEMPLATE)
        .map_err(|e| format!("Error creating file: {}", e))?;

    println!("Created empty PcbLib: {}", path.display());
    Ok(())
}

/// Adds a new footprint pattern to an existing library.
pub fn cmd_add_footprint(
    path: &Path,
    name: &str,
    description: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let lib = open_pcblib(path)?;

    if lib.find_footprint(name).is_some() {
        return Err(format!("Footprint '{}' already exists in library", name).into());
    }

    let desc = description.as_deref().unwrap_or("").to_string();
    lib.build_footprint(
        name,
        altium_format::templates::pcb_footprint_default,
        |builder| {
            builder.with_metadata(|fp| {
                fp.set_pattern(name.to_string());
                fp.set_description(desc.clone());
            });
        },
    );

    lib.save_file(path).map_err(|e| e.to_string())?;

    println!("Added footprint '{}' to {}", name, path.display());
    Ok(())
}

/// Adds a pad to an existing footprint in the library.
pub fn cmd_add_pad(
    path: &Path,
    footprint: &str,
    designator: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    shape: &str,
    hole: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    let lib = open_pcblib(path)?;
    let fp = find_footprint_by_name(&lib, footprint)?;

    let x_raw = PcbCoord::from_mm(x);
    let y_raw = PcbCoord::from_mm(y);
    let w_raw = PcbCoord::from_mm(width);
    let h_raw = PcbCoord::from_mm(height);
    let hole_raw = PcbCoord::from_mm(hole);
    let shape_byte = parse_shape(shape);
    let layer: u8 = if hole_raw.to_raw() > 0 { 74 } else { 1 };

    let pad = build_pad_record(x_raw, y_raw, w_raw, h_raw, hole_raw, shape_byte, layer);
    fp.add_primitive_record(pad);

    lib.save_file(path).map_err(|e| e.to_string())?;

    println!(
        "Added pad '{}' ({:.3}mm x {:.3}mm) to footprint '{}' in {}",
        designator,
        width,
        height,
        footprint,
        path.display()
    );
    Ok(())
}

/// Adds a silkscreen track (line) to a footprint.
pub fn cmd_add_silkscreen(
    _path: &Path,
    _footprint: &str,
    _x1: f64,
    _y1: f64,
    _x2: f64,
    _y2: f64,
    _width: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    Err(
        "Adding silkscreen tracks to existing footprints is not yet supported \
         through the public API. Use build_footprint() for new footprints that include tracks."
            .into(),
    )
}

/// Adds a silkscreen arc to a footprint.
pub fn cmd_add_arc(
    _path: &Path,
    _footprint: &str,
    _x: f64,
    _y: f64,
    _radius: f64,
    _start_angle: f64,
    _end_angle: f64,
    _width: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("Adding arcs to existing footprints is not yet supported \
         through the public API. Use build_footprint() for new footprints that include arcs."
        .into())
}

/// Generate a standard chip (0201/0402/0603/0805/1206) footprint.
pub fn cmd_gen_chip(
    path: &Path,
    size: &str,
    density: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let (body_l, body_w) = match size {
        "0201" => (0.6, 0.3),
        "0402" => (1.0, 0.5),
        "0603" => (1.6, 0.8),
        "0805" => (2.0, 1.25),
        "1206" => (3.2, 1.6),
        "1210" => (3.2, 2.5),
        _ => {
            return Err(format!(
                "Unknown chip size '{}'. Supported: 0201, 0402, 0603, 0805, 1206, 1210",
                size
            )
            .into());
        }
    };

    let (toe, _heel, side) = match density {
        "most" | "a" => (0.55, 0.0, 0.05),
        "nominal" | "b" => (0.35, 0.0, 0.0),
        "least" | "c" => (0.15, 0.0, -0.05),
        _ => {
            return Err(format!(
                "Unknown density '{}'. Supported: most, nominal, least",
                density
            )
            .into());
        }
    };

    let pad_width = body_w + 2.0 * side;
    let pad_length = body_l / 2.0 + toe;
    let pad_spacing = body_l - pad_length + toe;

    let fp_name = format!("CHIP_{}", size);

    {
        let lib = open_pcblib(path)?;
        if lib.find_footprint(&fp_name).is_some() {
            return Err(format!("Footprint '{}' already exists in library", fp_name).into());
        }
    }

    let lib = open_pcblib(path)?;
    let x_offset = pad_spacing / 2.0;
    let desc = format!("{} chip footprint, {} density", size, density);

    lib.build_footprint(
        &fp_name,
        altium_format::templates::pcb_footprint_default,
        |builder| {
            builder.with_metadata(|fp| {
                fp.set_pattern(fp_name.clone());
                fp.set_description(desc.clone());
            });

            // Pad 1 on left
            builder.add_pad(altium_format::templates::pcb_pad_default, |pad| {
                pad.set_position_x(PcbCoord::from_mm(-x_offset));
                pad.set_position_y(PcbCoord::from_mm(0.0));
                pad.set_top_size_x(PcbCoord::from_mm(pad_length));
                pad.set_top_size_y(PcbCoord::from_mm(pad_width));
                pad.set_mid_size_x(PcbCoord::from_mm(pad_length));
                pad.set_mid_size_y(PcbCoord::from_mm(pad_width));
                pad.set_bot_size_x(PcbCoord::from_mm(pad_length));
                pad.set_bot_size_y(PcbCoord::from_mm(pad_width));
                pad.set_top_shape(2);
                pad.set_mid_shape(2);
                pad.set_bot_shape(2);
                pad.set_layer(1);
            });

            // Pad 2 on right
            builder.add_pad(altium_format::templates::pcb_pad_default, |pad| {
                pad.set_position_x(PcbCoord::from_mm(x_offset));
                pad.set_position_y(PcbCoord::from_mm(0.0));
                pad.set_top_size_x(PcbCoord::from_mm(pad_length));
                pad.set_top_size_y(PcbCoord::from_mm(pad_width));
                pad.set_mid_size_x(PcbCoord::from_mm(pad_length));
                pad.set_mid_size_y(PcbCoord::from_mm(pad_width));
                pad.set_bot_size_x(PcbCoord::from_mm(pad_length));
                pad.set_bot_size_y(PcbCoord::from_mm(pad_width));
                pad.set_top_shape(2);
                pad.set_mid_shape(2);
                pad.set_bot_shape(2);
                pad.set_layer(1);
            });

            // Silkscreen lines
            let silk_margin = 0.1;
            let silk_x = body_l / 2.0 + silk_margin;
            let silk_y = body_w / 2.0 + silk_margin;
            let silk_width = 0.15;

            builder.add_track(altium_format::templates::pcb_track_default, |track| {
                track.set_start_x(PcbCoord::from_mm(-silk_x));
                track.set_start_y(PcbCoord::from_mm(silk_y));
                track.set_end_x(PcbCoord::from_mm(silk_x));
                track.set_end_y(PcbCoord::from_mm(silk_y));
                track.set_width(PcbCoord::from_mm(silk_width));
                let mut hdr = track.header();
                hdr.layer = 33;
                track.set_header(hdr);
            });

            builder.add_track(altium_format::templates::pcb_track_default, |track| {
                track.set_start_x(PcbCoord::from_mm(-silk_x));
                track.set_start_y(PcbCoord::from_mm(-silk_y));
                track.set_end_x(PcbCoord::from_mm(silk_x));
                track.set_end_y(PcbCoord::from_mm(-silk_y));
                track.set_width(PcbCoord::from_mm(silk_width));
                let mut hdr = track.header();
                hdr.layer = 33;
                track.set_header(hdr);
            });
        },
    );

    lib.save_file(path).map_err(|e| e.to_string())?;

    println!(
        "Generated {} chip footprint '{}' ({} density)",
        size, fp_name, density
    );
    Ok(())
}

/// Batch import from JSON.
pub fn cmd_add_json(
    path: &Path,
    file: Option<String>,
    input_json: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let json_str = if let Some(ref json) = input_json {
        json.clone()
    } else if let Some(ref file_path) = file {
        if file_path == "-" {
            use std::io::Read;
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            buf
        } else {
            std::fs::read_to_string(file_path)
                .map_err(|e| format!("Error reading {}: {}", file_path, e))?
        }
    } else {
        return Err("Either --file or --input must be provided".into());
    };

    let value: serde_json::Value =
        serde_json::from_str(&json_str).map_err(|e| format!("Invalid JSON: {}", e))?;

    let footprints = if value.is_array() {
        value.as_array().unwrap().clone()
    } else {
        vec![value]
    };

    let mut count = 0;
    for fp_json in &footprints {
        let name = fp_json["name"]
            .as_str()
            .ok_or("Footprint JSON must have a 'name' field")?;
        let description = fp_json["description"].as_str().map(|s| s.to_string());

        cmd_add_footprint(path, name, description)?;

        if let Some(pads) = fp_json["pads"].as_array() {
            for (i, pad_json) in pads.iter().enumerate() {
                let designator = pad_json["designator"]
                    .as_str()
                    .ok_or_else(|| format!("Pad {} in footprint '{}': missing 'designator' field", i, name))?;
                let x = pad_json["x"]
                    .as_f64()
                    .ok_or_else(|| format!("Pad '{}' in footprint '{}': missing or invalid 'x' field", designator, name))?;
                let y = pad_json["y"]
                    .as_f64()
                    .ok_or_else(|| format!("Pad '{}' in footprint '{}': missing or invalid 'y' field", designator, name))?;
                let width = pad_json["width"]
                    .as_f64()
                    .ok_or_else(|| format!("Pad '{}' in footprint '{}': missing or invalid 'width' field", designator, name))?;
                let height = pad_json["height"]
                    .as_f64()
                    .ok_or_else(|| format!("Pad '{}' in footprint '{}': missing or invalid 'height' field", designator, name))?;
                let shape = pad_json["shape"]
                    .as_str()
                    .ok_or_else(|| format!("Pad '{}' in footprint '{}': missing 'shape' field", designator, name))?;
                let hole = pad_json["hole"]
                    .as_f64()
                    .ok_or_else(|| format!("Pad '{}' in footprint '{}': missing or invalid 'hole' field", designator, name))?;

                cmd_add_pad(path, name, designator, x, y, width, height, shape, hole)?;
            }
        }

        count += 1;
    }

    println!(
        "Imported {} footprint(s) from JSON into {}",
        count,
        path.display()
    );
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// PAD PATTERN GENERATION COMMANDS
// ═══════════════════════════════════════════════════════════════════════════

/// Add a row of pads to a footprint.
pub fn cmd_add_pad_row(
    path: &Path,
    footprint: &str,
    count: usize,
    pitch: &str,
    pad_width: &str,
    pad_height: &str,
    direction: &str,
    start: u32,
    x: &str,
    y: &str,
    shape: &str,
    hole: &str,
    _use_spacing: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let pitch_raw = parse_dimension(pitch)?;
    let pw_raw = parse_dimension(pad_width)?;
    let ph_raw = parse_dimension(pad_height)?;
    let x_offset = parse_dimension(x)?;
    let y_offset = parse_dimension(y)?;
    let hole_raw = parse_dimension(hole)?;
    let shape_byte = parse_shape(shape);
    let layer: u8 = if hole_raw > 0 { 74 } else { 1 };

    let is_horizontal = matches!(direction.to_lowercase().as_str(), "horizontal" | "h" | "x");

    let lib = open_pcblib(path)?;
    let fp = find_footprint_by_name(&lib, footprint)?;

    let total_span = pitch_raw as i64 * (count as i64 - 1);

    for i in 0..count {
        let _pad_num = start + i as u32;
        let offset_along = -(total_span / 2) + pitch_raw as i64 * i as i64;

        let (px, py) = if is_horizontal {
            (x_offset as i64 + offset_along, y_offset as i64)
        } else {
            (x_offset as i64, y_offset as i64 + offset_along)
        };

        let node = build_pad_record(
            PcbCoord::from_raw(px as i32),
            PcbCoord::from_raw(py as i32),
            PcbCoord::from_raw(pw_raw),
            PcbCoord::from_raw(ph_raw),
            PcbCoord::from_raw(hole_raw),
            shape_byte,
            layer,
        );
        fp.add_primitive_record(node);
    }

    lib.save_file(path).map_err(|e| e.to_string())?;

    println!(
        "Added {} pads (row) to footprint '{}' in {}",
        count,
        footprint,
        path.display()
    );
    Ok(())
}

/// Add dual row of pads (SOIC, DIP style).
pub fn cmd_add_dual_row(
    path: &Path,
    footprint: &str,
    pads_per_side: usize,
    pitch: &str,
    row_spacing: &str,
    pad_width: Option<&str>,
    pad_height: Option<&str>,
    pad_diameter: Option<&str>,
    hole: Option<&str>,
    shape: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let pitch_raw = parse_dimension(pitch)?;
    let spacing_raw = parse_dimension(row_spacing)?;
    let hole_raw = hole.map(|h| parse_dimension(h)).transpose()?.unwrap_or(0);

    let (pw_raw, ph_raw) = if let Some(diam) = pad_diameter {
        let d = parse_dimension(diam)?;
        (d, d)
    } else {
        let pw = pad_width
            .map(|w| parse_dimension(w))
            .transpose()?
            .unwrap_or_else(|| mm_to_raw(0.6));
        let ph = pad_height
            .map(|h| parse_dimension(h))
            .transpose()?
            .unwrap_or_else(|| mm_to_raw(1.5));
        (pw, ph)
    };

    let shape_byte = parse_shape(shape);
    let layer: u8 = if hole_raw > 0 { 74 } else { 1 };

    let lib = open_pcblib(path)?;
    let fp = find_footprint_by_name(&lib, footprint)?;

    let half_spacing = spacing_raw / 2;
    let total_span = pitch_raw as i64 * (pads_per_side as i64 - 1);
    let total_pads = pads_per_side * 2;

    // Left side: pads 1..N (bottom to top)
    for i in 0..pads_per_side {
        let y = -(total_span / 2) + pitch_raw as i64 * i as i64;
        let node = build_pad_record(
            PcbCoord::from_raw(-half_spacing),
            PcbCoord::from_raw(y as i32),
            PcbCoord::from_raw(pw_raw),
            PcbCoord::from_raw(ph_raw),
            PcbCoord::from_raw(hole_raw),
            shape_byte,
            layer,
        );
        fp.add_primitive_record(node);
    }

    // Right side: pads N+1..2N (top to bottom, standard IC numbering)
    for i in 0..pads_per_side {
        let y = (total_span / 2) - pitch_raw as i64 * i as i64;
        let node = build_pad_record(
            PcbCoord::from_raw(half_spacing),
            PcbCoord::from_raw(y as i32),
            PcbCoord::from_raw(pw_raw),
            PcbCoord::from_raw(ph_raw),
            PcbCoord::from_raw(hole_raw),
            shape_byte,
            layer,
        );
        fp.add_primitive_record(node);
    }

    lib.save_file(path).map_err(|e| e.to_string())?;

    println!(
        "Added {} pads (dual row, {} per side) to footprint '{}' in {}",
        total_pads,
        pads_per_side,
        footprint,
        path.display()
    );
    Ok(())
}

/// Add quad pattern pads (QFP style).
pub fn cmd_add_quad_pads(
    path: &Path,
    footprint: &str,
    pads_per_side: usize,
    pitch: &str,
    span: &str,
    pad_width: &str,
    pad_height: &str,
    shape: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let pitch_raw = parse_dimension(pitch)?;
    let span_raw = parse_dimension(span)?;
    let pw_raw = parse_dimension(pad_width)?;
    let ph_raw = parse_dimension(pad_height)?;
    let shape_byte = parse_shape(shape);
    let layer: u8 = 1;

    let lib = open_pcblib(path)?;
    let fp = find_footprint_by_name(&lib, footprint)?;

    let half_span = span_raw / 2;
    let total_span = pitch_raw as i64 * (pads_per_side as i64 - 1);
    let total_pads = pads_per_side * 4;

    let zero_hole = PcbCoord::from_raw(0);

    // Side 1: Bottom (left to right)
    for i in 0..pads_per_side {
        let x = -(total_span / 2) + pitch_raw as i64 * i as i64;
        let node = build_pad_record(
            PcbCoord::from_raw(x as i32),
            PcbCoord::from_raw(-half_span),
            PcbCoord::from_raw(pw_raw),
            PcbCoord::from_raw(ph_raw),
            zero_hole,
            shape_byte,
            layer,
        );
        fp.add_primitive_record(node);
    }

    // Side 2: Right (bottom to top)
    for i in 0..pads_per_side {
        let y = -(total_span / 2) + pitch_raw as i64 * i as i64;
        let node = build_pad_record(
            PcbCoord::from_raw(half_span),
            PcbCoord::from_raw(y as i32),
            PcbCoord::from_raw(ph_raw), // rotated
            PcbCoord::from_raw(pw_raw),
            zero_hole,
            shape_byte,
            layer,
        );
        fp.add_primitive_record(node);
    }

    // Side 3: Top (right to left)
    for i in 0..pads_per_side {
        let x = (total_span / 2) - pitch_raw as i64 * i as i64;
        let node = build_pad_record(
            PcbCoord::from_raw(x as i32),
            PcbCoord::from_raw(half_span),
            PcbCoord::from_raw(pw_raw),
            PcbCoord::from_raw(ph_raw),
            zero_hole,
            shape_byte,
            layer,
        );
        fp.add_primitive_record(node);
    }

    // Side 4: Left (top to bottom)
    for i in 0..pads_per_side {
        let y = (total_span / 2) - pitch_raw as i64 * i as i64;
        let node = build_pad_record(
            PcbCoord::from_raw(-half_span),
            PcbCoord::from_raw(y as i32),
            PcbCoord::from_raw(ph_raw), // rotated
            PcbCoord::from_raw(pw_raw),
            zero_hole,
            shape_byte,
            layer,
        );
        fp.add_primitive_record(node);
    }

    lib.save_file(path).map_err(|e| e.to_string())?;

    println!(
        "Added {} pads (quad, {} per side) to footprint '{}' in {}",
        total_pads,
        pads_per_side,
        footprint,
        path.display()
    );
    Ok(())
}

/// Add a grid of pads (BGA style).
pub fn cmd_add_pad_grid(
    path: &Path,
    footprint: &str,
    rows: usize,
    cols: usize,
    pitch: &str,
    pad_diameter: &str,
    shape: &str,
    skip_center: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let pitch_raw = parse_dimension(pitch)?;
    let diam_raw = parse_dimension(pad_diameter)?;
    let skip_raw = parse_dimension(skip_center)?;
    let shape_byte = parse_shape(shape);
    let layer: u8 = 1;

    let lib = open_pcblib(path)?;
    let fp = find_footprint_by_name(&lib, footprint)?;

    let skip_radius_sq = if skip_raw > 0 {
        let half = skip_raw as f64 / 2.0;
        half * half
    } else {
        0.0
    };

    let x_span = pitch_raw as i64 * (cols as i64 - 1);
    let y_span = pitch_raw as i64 * (rows as i64 - 1);

    let mut positions: Vec<(i64, i64)> = Vec::new();
    for row in 0..rows {
        let y = (y_span / 2) - pitch_raw as i64 * row as i64;
        for col in 0..cols {
            let x = -(x_span / 2) + pitch_raw as i64 * col as i64;
            if skip_radius_sq > 0.0 {
                let dist_sq = (x as f64) * (x as f64) + (y as f64) * (y as f64);
                if dist_sq < skip_radius_sq {
                    continue;
                }
            }
            positions.push((x, y));
        }
    }
    let pad_count = positions.len();

    let zero_hole = PcbCoord::from_raw(0);
    for &(x, y) in &positions {
        let node = build_pad_record(
            PcbCoord::from_raw(x as i32),
            PcbCoord::from_raw(y as i32),
            PcbCoord::from_raw(diam_raw),
            PcbCoord::from_raw(diam_raw),
            zero_hole,
            shape_byte,
            layer,
        );
        fp.add_primitive_record(node);
    }

    lib.save_file(path).map_err(|e| e.to_string())?;

    println!(
        "Added {} pads ({}x{} grid) to footprint '{}' in {}",
        pad_count,
        rows,
        cols,
        footprint,
        path.display()
    );
    Ok(())
}
