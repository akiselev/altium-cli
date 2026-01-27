//! Footprint rendering to SVG and ASCII art.

use crate::records::pcb::{PcbComponent, PcbPadShape, PcbRecord};
use crate::types::Layer;

/// SVG rendering options.
#[derive(Debug, Clone)]
pub struct SvgOptions {
    /// Scale factor (pixels per mil).
    pub scale: f64,
    /// Padding around the footprint (in mils).
    pub padding: f64,
    /// Show pad designators.
    pub show_designators: bool,
    /// Show grid.
    pub show_grid: bool,
    /// Grid spacing in mils.
    pub grid_spacing: f64,
    /// Background color.
    pub background: String,
    /// Pad color (copper).
    pub pad_color: String,
    /// Pad hole color.
    pub hole_color: String,
    /// Silkscreen color.
    pub silkscreen_color: String,
    /// Courtyard color.
    pub courtyard_color: String,
    /// Mechanical layer color.
    pub mechanical_color: String,
    /// Grid color.
    pub grid_color: String,
    /// Text color.
    pub text_color: String,
}

impl Default for SvgOptions {
    fn default() -> Self {
        Self {
            scale: 2.0,
            padding: 50.0,
            show_designators: true,
            show_grid: true,
            grid_spacing: 50.0,
            background: "#1a1a2e".to_string(),
            pad_color: "#c9a227".to_string(),
            hole_color: "#1a1a2e".to_string(),
            silkscreen_color: "#ffffff".to_string(),
            courtyard_color: "#4a4a6a".to_string(),
            mechanical_color: "#6a6a8a".to_string(),
            grid_color: "#2a2a4e".to_string(),
            text_color: "#e0e0e0".to_string(),
        }
    }
}

impl SvgOptions {
    /// Light theme preset.
    pub fn light() -> Self {
        Self {
            background: "#ffffff".to_string(),
            pad_color: "#b8860b".to_string(),
            hole_color: "#ffffff".to_string(),
            silkscreen_color: "#000000".to_string(),
            courtyard_color: "#808080".to_string(),
            mechanical_color: "#a0a0a0".to_string(),
            grid_color: "#e0e0e0".to_string(),
            text_color: "#333333".to_string(),
            ..Default::default()
        }
    }
}

/// Render a footprint to SVG.
pub fn render_svg(component: &PcbComponent, options: &SvgOptions) -> String {
    let bounds = component.calculate_bounds();

    // Convert to mils for easier calculations
    let min_x = bounds.location1.x.to_mils() - options.padding;
    let min_y = bounds.location1.y.to_mils() - options.padding;
    let width = bounds.width().to_mils() + 2.0 * options.padding;
    let height = bounds.height().to_mils() + 2.0 * options.padding;

    let svg_width = (width * options.scale) as i32;
    let svg_height = (height * options.scale) as i32;

    let mut svg = String::new();

    // SVG header
    svg.push_str(&format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="{} {} {} {}">
  <defs>
    <style>
      .pad {{ fill: {}; stroke: #8b7355; stroke-width: 0.5; }}
      .pad-hole {{ fill: {}; }}
      .silkscreen {{ fill: none; stroke: {}; stroke-width: 6; stroke-linecap: round; }}
      .courtyard {{ fill: none; stroke: {}; stroke-width: 2; stroke-dasharray: 5,5; }}
      .mechanical {{ fill: none; stroke: {}; stroke-width: 2; }}
      .designator {{ font-family: monospace; font-size: 12px; fill: {}; text-anchor: middle; dominant-baseline: central; }}
      .title {{ font-family: sans-serif; font-size: 14px; fill: {}; }}
      .grid {{ stroke: {}; stroke-width: 0.5; }}
    </style>
  </defs>
"#,
        svg_width, svg_height,
        min_x, -min_y - height, width, height,
        options.pad_color, options.hole_color, options.silkscreen_color,
        options.courtyard_color, options.mechanical_color, options.text_color,
        options.text_color, options.grid_color
    ));

    // Background
    svg.push_str(&format!(
        r#"  <rect x="{}" y="{}" width="{}" height="{}" fill="{}"/>
"#,
        min_x,
        -min_y - height,
        width,
        height,
        options.background
    ));

    // Grid
    if options.show_grid {
        svg.push_str("  <g class=\"grid\">\n");
        let grid_start_x = (min_x / options.grid_spacing).floor() * options.grid_spacing;
        let grid_start_y =
            ((-min_y - height) / options.grid_spacing).floor() * options.grid_spacing;

        let mut x = grid_start_x;
        while x <= min_x + width {
            svg.push_str(&format!(
                "    <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"/>\n",
                x,
                -min_y - height,
                x,
                -min_y
            ));
            x += options.grid_spacing;
        }

        let mut y = grid_start_y;
        while y <= -min_y {
            svg.push_str(&format!(
                "    <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"/>\n",
                min_x,
                y,
                min_x + width,
                y
            ));
            y += options.grid_spacing;
        }
        svg.push_str("  </g>\n");
    }

    // Origin crosshair
    svg.push_str(&format!(
        r#"  <g stroke="{}" stroke-width="1">
    <line x1="-20" y1="0" x2="20" y2="0"/>
    <line x1="0" y1="-20" x2="0" y2="20"/>
  </g>
"#,
        options.courtyard_color
    ));

    // Render primitives by layer order
    // 1. Mechanical/Courtyard layers (back)
    render_tracks_by_layer(&mut svg, component, Layer::MECHANICAL_15, "courtyard");
    render_tracks_by_layer(&mut svg, component, Layer::MECHANICAL_13, "mechanical");

    // 2. Silkscreen
    render_tracks_by_layer(&mut svg, component, Layer::TOP_OVERLAY, "silkscreen");
    render_arcs_by_layer(&mut svg, component, Layer::TOP_OVERLAY, "silkscreen");

    // 3. Pads (front)
    svg.push_str("  <g id=\"pads\">\n");
    for record in &component.primitives {
        if let PcbRecord::Pad(pad) = record {
            render_pad(&mut svg, pad, options);
        }
    }
    svg.push_str("  </g>\n");

    // Title
    svg.push_str(&format!(
        r#"  <text x="{}" y="{}" class="title">{}</text>
"#,
        min_x + 5.0,
        -min_y - height + 15.0,
        component.pattern
    ));

    // Close SVG
    svg.push_str("</svg>\n");

    svg
}

fn render_tracks_by_layer(svg: &mut String, component: &PcbComponent, layer: Layer, class: &str) {
    svg.push_str(&format!("  <g class=\"{}\">\n", class));
    for record in &component.primitives {
        if let PcbRecord::Track(track) = record {
            if track.common.layer == layer {
                let x1 = track.start.x.to_mils();
                let y1 = -track.start.y.to_mils();
                let x2 = track.end.x.to_mils();
                let y2 = -track.end.y.to_mils();
                svg.push_str(&format!(
                    "    <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"/>\n",
                    x1, y1, x2, y2
                ));
            }
        }
    }
    svg.push_str("  </g>\n");
}

fn render_arcs_by_layer(svg: &mut String, component: &PcbComponent, layer: Layer, class: &str) {
    svg.push_str(&format!("  <g class=\"{}\">\n", class));
    for record in &component.primitives {
        if let PcbRecord::Arc(arc) = record {
            if arc.common.layer == layer {
                let cx = arc.location.x.to_mils();
                let cy = -arc.location.y.to_mils();
                let r = arc.radius.to_mils();

                // For full circles, use circle element
                if (arc.end_angle - arc.start_angle).abs() >= 359.0 {
                    svg.push_str(&format!(
                        "    <circle cx=\"{}\" cy=\"{}\" r=\"{}\"/>\n",
                        cx, cy, r
                    ));
                } else {
                    // For arcs, compute path
                    let start_rad = arc.start_angle.to_radians();
                    let end_rad = arc.end_angle.to_radians();
                    let x1 = cx + r * start_rad.cos();
                    let y1 = cy - r * start_rad.sin();
                    let x2 = cx + r * end_rad.cos();
                    let y2 = cy - r * end_rad.sin();
                    let large_arc = if (arc.end_angle - arc.start_angle).abs() > 180.0 {
                        1
                    } else {
                        0
                    };
                    svg.push_str(&format!(
                        "    <path d=\"M {} {} A {} {} 0 {} 0 {} {}\"/>\n",
                        x1, y1, r, r, large_arc, x2, y2
                    ));
                }
            }
        }
    }
    svg.push_str("  </g>\n");
}

fn render_pad(svg: &mut String, pad: &crate::records::pcb::PcbPad, options: &SvgOptions) {
    let x = pad.location.x.to_mils();
    let y = -pad.location.y.to_mils();
    let size = pad.size_top();
    let w = size.x.to_mils();
    let h = size.y.to_mils();
    let shape = pad.shape_top();

    // Pad body
    match shape {
        PcbPadShape::NoShape => {
            // No shape - render as a small circle
            let r = w.max(h) / 4.0;
            svg.push_str(&format!(
                "    <circle cx=\"{}\" cy=\"{}\" r=\"{}\" class=\"pad\"/>\n",
                x, y, r
            ));
        }
        PcbPadShape::Round | PcbPadShape::Circle => {
            let r = w.max(h) / 2.0;
            svg.push_str(&format!(
                "    <circle cx=\"{}\" cy=\"{}\" r=\"{}\" class=\"pad\"/>\n",
                x, y, r
            ));
        }
        PcbPadShape::Rectangular | PcbPadShape::RotatedRect => {
            svg.push_str(&format!(
                "    <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" class=\"pad\"/>\n",
                x - w / 2.0,
                y - h / 2.0,
                w,
                h
            ));
        }
        PcbPadShape::RoundedRectangle | PcbPadShape::RoundRect => {
            let radius = w.min(h) * 0.25;
            svg.push_str(&format!(
                "    <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" class=\"pad\"/>\n",
                x - w / 2.0, y - h / 2.0, w, h, radius
            ));
        }
        PcbPadShape::Octagonal => {
            // Approximate octagon with a rect with clipped corners
            let cut = w.min(h) * 0.3;
            svg.push_str(&format!(
                "    <polygon points=\"{},{} {},{} {},{} {},{} {},{} {},{} {},{} {},{}\" class=\"pad\"/>\n",
                x - w/2.0 + cut, y - h/2.0,
                x + w/2.0 - cut, y - h/2.0,
                x + w/2.0, y - h/2.0 + cut,
                x + w/2.0, y + h/2.0 - cut,
                x + w/2.0 - cut, y + h/2.0,
                x - w/2.0 + cut, y + h/2.0,
                x - w/2.0, y + h/2.0 - cut,
                x - w/2.0, y - h/2.0 + cut
            ));
        }
        PcbPadShape::Arc | PcbPadShape::Terminator => {
            // Render arc/terminator as rounded rectangle for now
            let radius = w.min(h) * 0.25;
            svg.push_str(&format!(
                "    <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" class=\"pad\"/>\n",
                x - w / 2.0, y - h / 2.0, w, h, radius
            ));
        }
    }

    // Hole (if through-hole)
    if pad.has_hole() {
        let hole_r = pad.hole_size.to_mils() / 2.0;
        svg.push_str(&format!(
            "    <circle cx=\"{}\" cy=\"{}\" r=\"{}\" class=\"pad-hole\"/>\n",
            x, y, hole_r
        ));
    }

    // Designator text
    if options.show_designators && !pad.designator.is_empty() {
        let font_size = (w.min(h) * 0.6).clamp(8.0, 14.0);
        svg.push_str(&format!(
            "    <text x=\"{}\" y=\"{}\" class=\"designator\" font-size=\"{}\">{}</text>\n",
            x, y, font_size, pad.designator
        ));
    }
}

/// ASCII art rendering options.
#[derive(Debug, Clone)]
pub struct AsciiOptions {
    /// Characters per mil (horizontal).
    pub chars_per_mil_x: f64,
    /// Characters per mil (vertical).
    pub chars_per_mil_y: f64,
    /// Maximum width in characters.
    pub max_width: usize,
    /// Maximum height in characters.
    pub max_height: usize,
    /// Show dimensions.
    pub show_dimensions: bool,
}

impl Default for AsciiOptions {
    fn default() -> Self {
        Self {
            chars_per_mil_x: 0.15,
            chars_per_mil_y: 0.08,
            max_width: 120,
            max_height: 40,
            show_dimensions: true,
        }
    }
}

/// Render a footprint to ASCII art.
pub fn render_ascii(component: &PcbComponent, options: &AsciiOptions) -> String {
    let bounds = component.calculate_bounds();

    // Convert to mils
    let min_x = bounds.location1.x.to_mils();
    let min_y = bounds.location1.y.to_mils();
    let width_mils = bounds.width().to_mils();
    let height_mils = bounds.height().to_mils();

    // Calculate canvas size
    let mut canvas_width = ((width_mils + 40.0) * options.chars_per_mil_x) as usize;
    let mut canvas_height = ((height_mils + 20.0) * options.chars_per_mil_y) as usize;

    canvas_width = canvas_width.max(40).min(options.max_width);
    canvas_height = canvas_height.max(10).min(options.max_height);

    // Create canvas
    let mut canvas: Vec<Vec<char>> = vec![vec![' '; canvas_width]; canvas_height];

    // Calculate scale factors
    let scale_x = (canvas_width as f64 - 4.0) / (width_mils + 20.0);
    let scale_y = (canvas_height as f64 - 2.0) / (height_mils + 10.0);

    let to_canvas_x = |x_mils: f64| -> usize { ((x_mils - min_x + 10.0) * scale_x + 2.0) as usize };

    let to_canvas_y = |y_mils: f64| -> usize {
        canvas_height - 1 - ((y_mils - min_y + 5.0) * scale_y + 1.0) as usize
    };

    // Draw border
    for cell in canvas[0].iter_mut().take(canvas_width) {
        *cell = '-';
    }
    for cell in canvas[canvas_height - 1].iter_mut().take(canvas_width) {
        *cell = '-';
    }
    for row in canvas.iter_mut().take(canvas_height) {
        row[0] = '|';
        row[canvas_width - 1] = '|';
    }
    canvas[0][0] = '+';
    canvas[0][canvas_width - 1] = '+';
    canvas[canvas_height - 1][0] = '+';
    canvas[canvas_height - 1][canvas_width - 1] = '+';

    // Draw origin marker
    let origin_x = to_canvas_x(0.0);
    let origin_y = to_canvas_y(0.0);
    if origin_x > 0 && origin_x < canvas_width - 1 && origin_y > 0 && origin_y < canvas_height - 1 {
        canvas[origin_y][origin_x] = '+';
    }

    // Draw tracks (silkscreen)
    for record in &component.primitives {
        if let PcbRecord::Track(track) = record {
            if track.common.layer == Layer::TOP_OVERLAY {
                draw_line(
                    &mut canvas,
                    to_canvas_x(track.start.x.to_mils()),
                    to_canvas_y(track.start.y.to_mils()),
                    to_canvas_x(track.end.x.to_mils()),
                    to_canvas_y(track.end.y.to_mils()),
                    '.',
                );
            }
        }
    }

    // Draw pads
    for record in &component.primitives {
        if let PcbRecord::Pad(pad) = record {
            let cx = to_canvas_x(pad.location.x.to_mils());
            let cy = to_canvas_y(pad.location.y.to_mils());
            let size = pad.size_top();
            let hw = ((size.x.to_mils() * scale_x) / 2.0).max(1.0) as usize;
            let hh = ((size.y.to_mils() * scale_y) / 2.0).max(0.5) as usize;

            // Draw pad rectangle
            for dy in 0..=hh * 2 {
                for dx in 0..=hw * 2 {
                    let x = cx.saturating_sub(hw) + dx;
                    let y = cy.saturating_sub(hh) + dy;
                    if x > 0 && x < canvas_width - 1 && y > 0 && y < canvas_height - 1 {
                        if dy == 0 || dy == hh * 2 || dx == 0 || dx == hw * 2 {
                            canvas[y][x] = '#';
                        } else if pad.has_hole() {
                            canvas[y][x] = 'O';
                        } else {
                            canvas[y][x] = '#';
                        }
                    }
                }
            }

            // Draw designator
            if !pad.designator.is_empty()
                && cx > 0
                && cx < canvas_width - 1
                && cy > 0
                && cy < canvas_height - 1
            {
                let chars: Vec<char> = pad.designator.chars().collect();
                let start = cx.saturating_sub(chars.len() / 2);
                for (i, c) in chars.iter().enumerate() {
                    let x = start + i;
                    if x > 0 && x < canvas_width - 1 {
                        canvas[cy][x] = *c;
                    }
                }
            }
        }
    }

    // Build output string
    let mut output = String::new();

    // Header
    output.push_str(&format!("Footprint: {}\n", component.pattern));
    if !component.description.is_empty() {
        output.push_str(&format!("Description: {}\n", component.description));
    }
    output.push_str(&format!(
        "Pads: {}   Primitives: {}\n",
        component.pad_count(),
        component.primitive_count()
    ));
    if options.show_dimensions {
        output.push_str(&format!(
            "Size: {:.2} x {:.2} mil ({:.2} x {:.2} mm)\n\n",
            width_mils,
            height_mils,
            width_mils * 0.0254,
            height_mils * 0.0254
        ));
    }

    // Canvas
    for row in &canvas {
        output.push_str(&row.iter().collect::<String>());
        output.push('\n');
    }

    // Pad list
    output.push_str("\nPads:\n");
    let mut pads: Vec<_> = component.pads().collect();
    pads.sort_by(|a, b| alphanumeric_cmp(&a.designator, &b.designator));
    for pad in pads {
        let size = pad.size_top();
        let loc = pad.location;
        output.push_str(&format!(
            "  {} @ ({:.2}, {:.2}) mil  size: {:.2} x {:.2} mil  {}\n",
            pad.designator,
            loc.x.to_mils(),
            loc.y.to_mils(),
            size.x.to_mils(),
            size.y.to_mils(),
            if pad.has_hole() {
                format!("hole: {:.2} mil", pad.hole_size.to_mils())
            } else {
                "SMD".to_string()
            }
        ));
    }

    output
}

fn draw_line(canvas: &mut [Vec<char>], x1: usize, y1: usize, x2: usize, y2: usize, ch: char) {
    let dx = (x2 as i32 - x1 as i32).abs();
    let dy = (y2 as i32 - y1 as i32).abs();
    let sx: i32 = if x1 < x2 { 1 } else { -1 };
    let sy: i32 = if y1 < y2 { 1 } else { -1 };
    let mut err = dx - dy;
    let mut x = x1 as i32;
    let mut y = y1 as i32;

    let height = canvas.len();
    let width = if height > 0 { canvas[0].len() } else { 0 };

    loop {
        if x > 0 && (x as usize) < width - 1 && y > 0 && (y as usize) < height - 1 {
            canvas[y as usize][x as usize] = ch;
        }

        if x == x2 as i32 && y == y2 as i32 {
            break;
        }

        let e2 = 2 * err;
        if e2 > -dy {
            err -= dy;
            x += sx;
        }
        if e2 < dx {
            err += dx;
            y += sy;
        }
    }
}

fn alphanumeric_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let extract = |s: &str| -> (String, i32) {
        let mut prefix = String::new();
        let mut num = String::new();
        for c in s.chars() {
            if c.is_ascii_digit() {
                num.push(c);
            } else if num.is_empty() {
                prefix.push(c);
            }
        }
        (prefix, num.parse().unwrap_or(0))
    };

    let (pa, na) = extract(a);
    let (pb, nb) = extract(b);
    pa.cmp(&pb).then(na.cmp(&nb))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::footprint::FootprintBuilder;
    use crate::records::pcb::PcbPadShape;

    #[test]
    fn test_render_svg() {
        let mut det = ();
        let mut builder = FootprintBuilder::new("TEST");
        builder.add_smd_pad("1", -1.0, 0.0, 0.6, 0.8, PcbPadShape::Rectangular);
        builder.add_smd_pad("2", 1.0, 0.0, 0.6, 0.8, PcbPadShape::Rectangular);
        let component = builder.build_deterministic(&mut det);

        let svg = render_svg(&component, &SvgOptions::default());
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("TEST"));
    }

    #[test]
    fn test_render_ascii() {
        let mut det = ();
        let mut builder = FootprintBuilder::new("TEST");
        builder.add_smd_pad("1", -1.0, 0.0, 0.6, 0.8, PcbPadShape::Rectangular);
        builder.add_smd_pad("2", 1.0, 0.0, 0.6, 0.8, PcbPadShape::Rectangular);
        let component = builder.build_deterministic(&mut det);

        let ascii = render_ascii(&component, &AsciiOptions::default());
        assert!(ascii.contains("TEST"));
        assert!(ascii.contains("Pads: 2"));
    }
}
