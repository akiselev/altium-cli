// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! PcbLib generation from import DSL types.

use std::path::Path;

use crate::footprint::{ChipSpec, FootprintBuilder, PadRowDirection};
use crate::ops::pcblib::{
    open_or_create_pcblib, parse_density, parse_pad_shape, parse_unit_value_or_mm, save_pcblib,
};
use crate::records::pcb::{PcbComponent, PcbPadShape};

use super::types::*;

/// Generate a complete PcbLib from an import definition.
pub fn generate_pcblib(
    output_path: &Path,
    import: &PcbLibImport,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut lib = open_or_create_pcblib(output_path)?;

    let mut generated_count = 0;

    for fp_def in &import.footprints {
        // Check for duplicates
        if lib.components.iter().any(|c| c.pattern == fp_def.name) {
            return Err(format!("Footprint '{}' already exists in library", fp_def.name).into());
        }

        let component = match &fp_def.package {
            PackageType::Chip => generate_chip_footprint(fp_def)?,
            PackageType::DualRow => generate_dual_row_footprint(fp_def)?,
            PackageType::Quad => generate_quad_footprint(fp_def)?,
            PackageType::NoLead => generate_no_lead_footprint(fp_def)?,
            PackageType::Bga => generate_bga_footprint(fp_def)?,
            PackageType::Sot => generate_sot_footprint(fp_def)?,
            PackageType::SingleRow => generate_single_row_footprint(fp_def)?,
            PackageType::Custom => generate_custom_footprint(fp_def)?,
        };

        lib.components.push(component);
        generated_count += 1;
    }

    save_pcblib(output_path, &lib)?;

    Ok(format!(
        "Generated PcbLib with {} footprint(s) -> {}",
        generated_count,
        output_path.display()
    ))
}

/// Generate an IPC chip passive footprint (0201-2512).
fn generate_chip_footprint(
    def: &PcbLibFootprintDef,
) -> Result<PcbComponent, Box<dyn std::error::Error>> {
    let chip_size = def
        .chip_size
        .as_deref()
        .ok_or("chip package requires 'chip_size' field")?;

    let spec = match chip_size.to_uppercase().as_str() {
        "0201" => ChipSpec::chip_0201(),
        "0402" => ChipSpec::chip_0402(),
        "0603" => ChipSpec::chip_0603(),
        "0805" => ChipSpec::chip_0805(),
        "1206" => ChipSpec::chip_1206(),
        _ => {
            return Err(format!(
                "Unknown chip size '{}'. Supported: 0201, 0402, 0603, 0805, 1206",
                chip_size
            )
            .into());
        }
    };

    let density_str = match &def.density {
        Some(DensityLevel::Most) => "most",
        Some(DensityLevel::Nominal) => "nominal",
        Some(DensityLevel::Least) => "least",
        None => "nominal",
    };
    let density = parse_density(density_str)?;

    let mut det = ();
    let mut component = spec.to_footprint(density).build_deterministic(&mut det);

    // Override name if different from auto-generated
    if component.pattern != def.name {
        component.pattern = def.name.clone();
    }

    Ok(component)
}

/// Generate a dual-row footprint (SOIC, SOP, SSOP, DIP, etc.).
fn generate_dual_row_footprint(
    def: &PcbLibFootprintDef,
) -> Result<PcbComponent, Box<dyn std::error::Error>> {
    let pads_per_side = def
        .pads_per_side
        .ok_or("dual-row package requires 'pads_per_side'")?;
    let pitch_str = def.pitch.as_deref().ok_or("dual-row package requires 'pitch'")?;
    let row_spacing_str = def
        .row_spacing
        .as_deref()
        .ok_or("dual-row package requires 'row_spacing'")?;

    let pitch_mm = parse_unit_value_or_mm(pitch_str)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let row_spacing_mm = parse_unit_value_or_mm(row_spacing_str)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    let mut builder = FootprintBuilder::new(&def.name);
    if !def.description.is_empty() {
        builder = builder.description(&def.description);
    }

    let technology = def.technology.as_ref().unwrap_or(&Technology::Smd);
    let shape = parse_pad_shape(def.pad_shape.as_deref().unwrap_or("rectangular"))
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    match technology {
        Technology::ThroughHole => {
            let hole_str = def
                .hole_diameter
                .as_deref()
                .ok_or("through-hole dual-row requires 'hole_diameter'")?;
            let pad_dia_str = def
                .pad_diameter
                .as_deref()
                .ok_or("through-hole dual-row requires 'pad_diameter'")?;
            let hole_mm = parse_unit_value_or_mm(hole_str)
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
            let pad_dia_mm = parse_unit_value_or_mm(pad_dia_str)
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
            builder.add_dual_row_th(pads_per_side, pitch_mm, row_spacing_mm, pad_dia_mm, hole_mm, shape);
        }
        Technology::Smd => {
            let pad_w_str = def
                .pad_width
                .as_deref()
                .ok_or("SMD dual-row requires 'pad_width'")?;
            let pad_h_str = def
                .pad_height
                .as_deref()
                .ok_or("SMD dual-row requires 'pad_height'")?;
            let pad_w_mm = parse_unit_value_or_mm(pad_w_str)
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
            let pad_h_mm = parse_unit_value_or_mm(pad_h_str)
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
            builder.add_dual_row_smd(pads_per_side, pitch_mm, row_spacing_mm, pad_w_mm, pad_h_mm, shape);
        }
    }

    add_silkscreen(&mut builder, &def.silkscreen)?;

    let mut det = ();
    Ok(builder.build_deterministic(&mut det))
}

/// Generate a quad-pad footprint (QFP, LQFP, TQFP, PLCC).
fn generate_quad_footprint(
    def: &PcbLibFootprintDef,
) -> Result<PcbComponent, Box<dyn std::error::Error>> {
    let pads_per_side = def
        .pads_per_side
        .ok_or("quad package requires 'pads_per_side'")?;
    let pitch_str = def.pitch.as_deref().ok_or("quad package requires 'pitch'")?;
    let span_str = def.span.as_deref().ok_or("quad package requires 'span'")?;
    let pad_w_str = def
        .pad_width
        .as_deref()
        .ok_or("quad package requires 'pad_width'")?;
    let pad_h_str = def
        .pad_height
        .as_deref()
        .ok_or("quad package requires 'pad_height'")?;

    let pitch_mm = parse_unit_value_or_mm(pitch_str)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let span_mm = parse_unit_value_or_mm(span_str)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let pad_w_mm = parse_unit_value_or_mm(pad_w_str)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let pad_h_mm = parse_unit_value_or_mm(pad_h_str)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let shape = parse_pad_shape(def.pad_shape.as_deref().unwrap_or("rectangular"))
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    let mut builder = FootprintBuilder::new(&def.name);
    if !def.description.is_empty() {
        builder = builder.description(&def.description);
    }

    builder.add_quad_pads_smd(pads_per_side, pitch_mm, span_mm, pad_w_mm, pad_h_mm, shape);

    add_silkscreen(&mut builder, &def.silkscreen)?;

    let mut det = ();
    Ok(builder.build_deterministic(&mut det))
}

/// Generate a no-lead footprint (QFN, DFN, SON) with optional exposed pad.
fn generate_no_lead_footprint(
    def: &PcbLibFootprintDef,
) -> Result<PcbComponent, Box<dyn std::error::Error>> {
    let pads_per_side = def
        .pads_per_side
        .ok_or("no-lead package requires 'pads_per_side'")?;
    let pitch_str = def.pitch.as_deref().ok_or("no-lead package requires 'pitch'")?;
    let span_str = def
        .span
        .as_deref()
        .or(def.row_spacing.as_deref())
        .ok_or("no-lead package requires 'span' or 'row_spacing'")?;
    let pad_w_str = def
        .pad_width
        .as_deref()
        .ok_or("no-lead package requires 'pad_width'")?;
    let pad_h_str = def
        .pad_height
        .as_deref()
        .ok_or("no-lead package requires 'pad_height'")?;

    let pitch_mm = parse_unit_value_or_mm(pitch_str)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let span_mm = parse_unit_value_or_mm(span_str)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let pad_w_mm = parse_unit_value_or_mm(pad_w_str)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let pad_h_mm = parse_unit_value_or_mm(pad_h_str)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let shape = parse_pad_shape(def.pad_shape.as_deref().unwrap_or("rectangular"))
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    let mut builder = FootprintBuilder::new(&def.name);
    if !def.description.is_empty() {
        builder = builder.description(&def.description);
    }

    // Use dual-row for 2-sided no-lead, or quad for 4-sided
    // Heuristic: if pads_per_side * 2 == total expected pads, it's dual; else quad
    // For now, default to dual-row SMD (most QFN/DFN are effectively dual-row)
    builder.add_dual_row_smd(pads_per_side, pitch_mm, span_mm, pad_w_mm, pad_h_mm, shape);

    // Add exposed pad if specified
    if let Some(ref ep) = def.exposed_pad {
        let ep_w_mm = parse_unit_value_or_mm(&ep.width)
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        let ep_h_mm = parse_unit_value_or_mm(&ep.height)
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        builder.add_smd_pad(&ep.designator, 0.0, 0.0, ep_w_mm, ep_h_mm, PcbPadShape::Rectangular);

        // Add thermal vias if specified
        if let Some(ref tv) = ep.thermal_vias {
            let tv_pitch = parse_unit_value_or_mm(&tv.pitch)
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
            let tv_hole = parse_unit_value_or_mm(&tv.hole_diameter)
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
            let tv_pad = parse_unit_value_or_mm(&tv.pad_diameter)
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

            let grid_w = (tv.cols as f64 - 1.0) * tv_pitch;
            let grid_h = (tv.rows as f64 - 1.0) * tv_pitch;
            let start_x = -grid_w / 2.0;
            let start_y = -grid_h / 2.0;

            for row in 0..tv.rows {
                for col in 0..tv.cols {
                    let x = start_x + col as f64 * tv_pitch;
                    let y = start_y + row as f64 * tv_pitch;
                    builder.add_th_pad(
                        &format!("TV{}_{}", row + 1, col + 1),
                        x,
                        y,
                        tv_pad,
                        tv_hole,
                        PcbPadShape::Round,
                    );
                }
            }
        }
    }

    add_silkscreen(&mut builder, &def.silkscreen)?;

    let mut det = ();
    Ok(builder.build_deterministic(&mut det))
}

/// Generate a BGA footprint.
fn generate_bga_footprint(
    def: &PcbLibFootprintDef,
) -> Result<PcbComponent, Box<dyn std::error::Error>> {
    let rows = def.rows.ok_or("bga package requires 'rows'")?;
    let cols = def.cols.ok_or("bga package requires 'cols'")?;
    let pitch_str = def.pitch.as_deref().ok_or("bga package requires 'pitch'")?;
    let pad_dia_str = def
        .pad_diameter
        .as_deref()
        .ok_or("bga package requires 'pad_diameter'")?;

    let pitch_mm = parse_unit_value_or_mm(pitch_str)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let pad_dia_mm = parse_unit_value_or_mm(pad_dia_str)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let shape = parse_pad_shape(def.pad_shape.as_deref().unwrap_or("round"))
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let skip_center = if let Some(ref sc) = def.skip_center {
        parse_unit_value_or_mm(sc).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?
    } else {
        0.0
    };

    let mut builder = FootprintBuilder::new(&def.name);
    if !def.description.is_empty() {
        builder = builder.description(&def.description);
    }

    builder.add_pad_grid(rows, cols, pitch_mm, pad_dia_mm, shape, skip_center);

    add_silkscreen(&mut builder, &def.silkscreen)?;

    let mut det = ();
    Ok(builder.build_deterministic(&mut det))
}

/// Generate a SOT footprint from predefined specs.
fn generate_sot_footprint(
    def: &PcbLibFootprintDef,
) -> Result<PcbComponent, Box<dyn std::error::Error>> {
    let variant = def
        .variant
        .as_deref()
        .unwrap_or("SOT-23");

    // SOT predefined specs (all dimensions in mm)
    let mut builder = FootprintBuilder::new(&def.name);
    if !def.description.is_empty() {
        builder = builder.description(&def.description);
    }

    let shape = PcbPadShape::Rectangular;

    match variant.to_uppercase().replace('-', "").as_str() {
        "SOT23" | "SOT233" => {
            // SOT-23: 3-pin, 0.95mm pitch, 2.4mm span
            builder.add_smd_pad("1", -0.95, -1.1, 0.6, 0.7, shape);
            builder.add_smd_pad("2", 0.95, -1.1, 0.6, 0.7, shape);
            builder.add_smd_pad("3", 0.0, 1.1, 0.6, 0.7, shape);
        }
        "SOT235" | "SOT23_5" | "SOT23_5L" => {
            // SOT-23-5: 5-pin
            builder.add_smd_pad("1", -0.95, -1.1, 0.6, 0.7, shape);
            builder.add_smd_pad("2", 0.0, -1.1, 0.6, 0.7, shape);
            builder.add_smd_pad("3", 0.95, -1.1, 0.6, 0.7, shape);
            builder.add_smd_pad("4", 0.95, 1.1, 0.6, 0.7, shape);
            builder.add_smd_pad("5", -0.95, 1.1, 0.6, 0.7, shape);
        }
        "SOT236" | "SOT23_6" | "SOT23_6L" => {
            // SOT-23-6: 6-pin
            builder.add_smd_pad("1", -0.95, -1.1, 0.6, 0.7, shape);
            builder.add_smd_pad("2", 0.0, -1.1, 0.6, 0.7, shape);
            builder.add_smd_pad("3", 0.95, -1.1, 0.6, 0.7, shape);
            builder.add_smd_pad("4", 0.95, 1.1, 0.6, 0.7, shape);
            builder.add_smd_pad("5", 0.0, 1.1, 0.6, 0.7, shape);
            builder.add_smd_pad("6", -0.95, 1.1, 0.6, 0.7, shape);
        }
        "SOT89" => {
            // SOT-89: 3-pin with large center tab
            builder.add_smd_pad("1", -1.5, -1.8, 0.7, 1.0, shape);
            builder.add_smd_pad("2", 0.0, -1.8, 0.7, 1.0, shape);
            builder.add_smd_pad("3", 1.5, -1.8, 0.7, 1.0, shape);
            builder.add_smd_pad("2", 0.0, 1.8, 2.0, 1.5, shape); // Tab is pad 2
        }
        "SOT223" => {
            // SOT-223: 4-pin (3 + tab)
            builder.add_smd_pad("1", -2.3, -3.15, 0.7, 1.5, shape);
            builder.add_smd_pad("2", 0.0, -3.15, 0.7, 1.5, shape);
            builder.add_smd_pad("3", 2.3, -3.15, 0.7, 1.5, shape);
            builder.add_smd_pad("4", 0.0, 3.15, 3.2, 1.5, shape); // Tab
        }
        "SOT323" => {
            // SOT-323 (SC-70): 3-pin
            builder.add_smd_pad("1", -0.65, -0.85, 0.4, 0.5, shape);
            builder.add_smd_pad("2", 0.65, -0.85, 0.4, 0.5, shape);
            builder.add_smd_pad("3", 0.0, 0.85, 0.4, 0.5, shape);
        }
        "SOT363" => {
            // SOT-363 (SC-88): 6-pin
            builder.add_smd_pad("1", -0.65, -0.85, 0.4, 0.5, shape);
            builder.add_smd_pad("2", 0.0, -0.85, 0.4, 0.5, shape);
            builder.add_smd_pad("3", 0.65, -0.85, 0.4, 0.5, shape);
            builder.add_smd_pad("4", 0.65, 0.85, 0.4, 0.5, shape);
            builder.add_smd_pad("5", 0.0, 0.85, 0.4, 0.5, shape);
            builder.add_smd_pad("6", -0.65, 0.85, 0.4, 0.5, shape);
        }
        _ => {
            // If user also provided pad dimensions, use dual-row as fallback
            if let (Some(pps), Some(pitch), Some(span)) =
                (def.pads_per_side, def.pitch.as_deref(), def.span.as_deref())
            {
                let pitch_mm = parse_unit_value_or_mm(pitch)
                    .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
                let span_mm = parse_unit_value_or_mm(span)
                    .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
                let pad_w = parse_unit_value_or_mm(def.pad_width.as_deref().unwrap_or("0.6mm"))
                    .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
                let pad_h = parse_unit_value_or_mm(def.pad_height.as_deref().unwrap_or("1.0mm"))
                    .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
                builder.add_dual_row_smd(pps, pitch_mm, span_mm, pad_w, pad_h, shape);
            } else {
                return Err(format!(
                    "Unknown SOT variant '{}'. Supported: SOT-23, SOT-23-5, SOT-23-6, SOT-89, SOT-223, SOT-323, SOT-363. Or provide pad dimensions.",
                    variant
                ).into());
            }
        }
    }

    add_silkscreen(&mut builder, &def.silkscreen)?;

    let mut det = ();
    Ok(builder.build_deterministic(&mut det))
}

/// Generate a single-row footprint (SIP, pin headers).
fn generate_single_row_footprint(
    def: &PcbLibFootprintDef,
) -> Result<PcbComponent, Box<dyn std::error::Error>> {
    let pad_count = def.pad_count.ok_or("single-row package requires 'pad_count'")?;
    let pitch_str = def.pitch.as_deref().ok_or("single-row package requires 'pitch'")?;

    let pitch_mm = parse_unit_value_or_mm(pitch_str)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    let technology = def.technology.as_ref().unwrap_or(&Technology::ThroughHole);
    let shape = parse_pad_shape(def.pad_shape.as_deref().unwrap_or("round"))
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let dir = match def.direction.as_deref().unwrap_or("vertical") {
        "horizontal" | "h" => PadRowDirection::Horizontal,
        _ => PadRowDirection::Vertical,
    };

    let mut builder = FootprintBuilder::new(&def.name);
    if !def.description.is_empty() {
        builder = builder.description(&def.description);
    }

    match technology {
        Technology::ThroughHole => {
            let hole_str = def
                .hole_diameter
                .as_deref()
                .unwrap_or("0.8mm");
            let pad_dia_str = def
                .pad_diameter
                .as_deref()
                .unwrap_or("1.6mm");
            let hole_mm = parse_unit_value_or_mm(hole_str)
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
            let pad_dia_mm = parse_unit_value_or_mm(pad_dia_str)
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
            builder.add_th_pad_row(pad_count, pitch_mm, pad_dia_mm, hole_mm, 0.0, 0.0, dir, 1, shape);
        }
        Technology::Smd => {
            let pad_w = parse_unit_value_or_mm(def.pad_width.as_deref().unwrap_or("0.6mm"))
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
            let pad_h = parse_unit_value_or_mm(def.pad_height.as_deref().unwrap_or("1.0mm"))
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
            builder.add_pad_row(pad_count, pitch_mm, pad_w, pad_h, 0.0, 0.0, dir, 1, shape);
        }
    }

    add_silkscreen(&mut builder, &def.silkscreen)?;

    let mut det = ();
    Ok(builder.build_deterministic(&mut det))
}

/// Generate a fully custom footprint.
fn generate_custom_footprint(
    def: &PcbLibFootprintDef,
) -> Result<PcbComponent, Box<dyn std::error::Error>> {
    let mut builder = FootprintBuilder::new(&def.name);
    if !def.description.is_empty() {
        builder = builder.description(&def.description);
    }

    for pad in &def.pads {
        let x = parse_unit_value_or_mm(&pad.x)
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        let y = parse_unit_value_or_mm(&pad.y)
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        let w = parse_unit_value_or_mm(&pad.width)
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        let h = parse_unit_value_or_mm(&pad.height)
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        let shape = parse_pad_shape(&pad.shape)
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

        let hole_mm = if let Some(ref hole_str) = pad.hole {
            parse_unit_value_or_mm(hole_str)
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?
        } else {
            0.0
        };

        if hole_mm > 0.0 {
            builder.add_th_pad(&pad.designator, x, y, w.max(h), hole_mm, shape);
        } else {
            builder.add_smd_pad(&pad.designator, x, y, w, h, shape);
        }
    }

    add_silkscreen(&mut builder, &def.silkscreen)?;

    let mut det = ();
    Ok(builder.build_deterministic(&mut det))
}

/// Add silkscreen elements to a builder.
fn add_silkscreen(
    builder: &mut FootprintBuilder,
    elements: &[SilkscreenElement],
) -> Result<(), Box<dyn std::error::Error>> {
    for elem in elements {
        match elem {
            SilkscreenElement::Rectangle {
                x,
                y,
                width,
                height,
            } => {
                let x_mm = parse_unit_value_or_mm(x)
                    .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
                let y_mm = parse_unit_value_or_mm(y)
                    .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
                let w_mm = parse_unit_value_or_mm(width)
                    .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
                let h_mm = parse_unit_value_or_mm(height)
                    .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
                // Draw as 4 lines
                builder.add_silkscreen_line(x_mm, y_mm, x_mm + w_mm, y_mm, 0.15);
                builder.add_silkscreen_line(x_mm + w_mm, y_mm, x_mm + w_mm, y_mm + h_mm, 0.15);
                builder.add_silkscreen_line(x_mm + w_mm, y_mm + h_mm, x_mm, y_mm + h_mm, 0.15);
                builder.add_silkscreen_line(x_mm, y_mm + h_mm, x_mm, y_mm, 0.15);
            }
            SilkscreenElement::Line {
                x1,
                y1,
                x2,
                y2,
                width,
            } => {
                let x1_mm = parse_unit_value_or_mm(x1)
                    .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
                let y1_mm = parse_unit_value_or_mm(y1)
                    .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
                let x2_mm = parse_unit_value_or_mm(x2)
                    .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
                let y2_mm = parse_unit_value_or_mm(y2)
                    .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
                let w_mm = parse_unit_value_or_mm(width)
                    .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
                builder.add_silkscreen_line(x1_mm, y1_mm, x2_mm, y2_mm, w_mm);
            }
            SilkscreenElement::Arc {
                x,
                y,
                radius,
                start_angle,
                end_angle,
                width,
            } => {
                let x_mm = parse_unit_value_or_mm(x)
                    .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
                let y_mm = parse_unit_value_or_mm(y)
                    .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
                let r_mm = parse_unit_value_or_mm(radius)
                    .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
                let w_mm = parse_unit_value_or_mm(width)
                    .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
                builder.add_silkscreen_arc(x_mm, y_mm, r_mm, *start_angle, *end_angle, w_mm);
            }
            SilkscreenElement::Text { .. } => {
                // Text elements need to be added after build; skip here
                // (handled at the component level post-build)
            }
        }
    }
    Ok(())
}
