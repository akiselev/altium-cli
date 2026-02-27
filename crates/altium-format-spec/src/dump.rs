//! Reverse generation: produce `.schlib-spec` or `.pcblib-spec` source from
//! existing Altium library documents.
//!
//! Generated output uses absolute placement only (`at: (x, y)`, explicit
//! `orientation:`). No anchors, rows, grids, or template bindings are emitted.

use altium_format::{PcbLib, SchLib};

// ── Public API ────────────────────────────────────────────────────────────────

/// Generate `.pcblib-spec` source from a PcbLib document.
pub fn dump_pcblib(lib: &PcbLib) -> String {
    let mut out = String::new();
    for fp in &lib.dump_footprints() {
        dump_footprint(&mut out, fp);
        out.push('\n');
    }
    out
}

/// Generate `.schlib-spec` source from a SchLib document.
pub fn dump_schlib(lib: &SchLib) -> String {
    let mut out = String::new();
    for comp in &lib.dump_components() {
        dump_component(&mut out, comp);
        out.push('\n');
    }
    out
}

// ── Footprint ─────────────────────────────────────────────────────────────────

fn dump_footprint(out: &mut String, fp: &altium_format::PcbLibFootprintDumpView) {
    out.push_str(&format!("footprint {} {{\n", quote_entity_name(&fp.display_name)));

    if !fp.description.is_empty() {
        out.push_str(&format!("    description: {}\n", quote_string(&fp.description)));
    }

    for pad in &fp.pads {
        dump_pcb_pad(out, pad, 4);
    }

    for graphic in &fp.graphics {
        dump_pcb_graphic(out, graphic, 4);
    }

    out.push_str("}\n");
}

fn dump_pcb_pad(out: &mut String, pad: &altium_format::PcbLibPadDumpView, indent: usize) {
    let p = " ".repeat(indent);
    let x = format_coord_mils(pad.location_x_mils);
    let y = format_coord_mils(pad.location_y_mils);
    let mut parts = vec![format!("at: ({}, {})", x, y)];

    if !pad.shape.is_empty() && pad.shape != "round" {
        parts.push(format!("shape: {}", pad.shape));
    }
    if pad.size_x_mils != 0.0 {
        parts.push(format!("x_size: {}", format_coord_mils(pad.size_x_mils)));
    }
    if pad.size_y_mils != 0.0 {
        parts.push(format!("y_size: {}", format_coord_mils(pad.size_y_mils)));
    }
    if pad.hole_size_mils != 0.0 {
        parts.push(format!("hole_size: {}", format_coord_mils(pad.hole_size_mils)));
    }
    if pad.rotation != 0.0 {
        parts.push(format!("rotation: {}", format_float(pad.rotation)));
    }
    if !pad.is_plated {
        parts.push("is_plated: false".to_owned());
    }
    if pad.layer != "MultiLayer" {
        parts.push(format!("layer: {}", pad.layer));
    }

    out.push_str(&format!(
        "{}pad {} {{ {} }}\n",
        p,
        quote_entity_name(&pad.pad_name),
        parts.join(", ")
    ));
}

fn dump_pcb_graphic(out: &mut String, g: &altium_format::PcbLibGraphicDumpView, indent: usize) {
    let p = " ".repeat(indent);
    match g.graphic_type.as_str() {
        "track" => {
            let fx = format_coord_mils(g.from_x_mils.unwrap_or(0.0));
            let fy = format_coord_mils(g.from_y_mils.unwrap_or(0.0));
            let tx = format_coord_mils(g.to_x_mils.unwrap_or(0.0));
            let ty = format_coord_mils(g.to_y_mils.unwrap_or(0.0));
            let mut props = vec![
                format!("layer: {}", g.layer),
                format!("from: ({}, {})", fx, fy),
                format!("to: ({}, {})", tx, ty),
            ];
            if let Some(w) = g.width_mils {
                if w != 0.0 {
                    props.push(format!("width: {}", format_coord_mils(w)));
                }
            }
            out.push_str(&format!("{}track {{ {} }}\n", p, props.join(", ")));
        }
        "arc" => {
            let cx = format_coord_mils(g.center_x_mils.unwrap_or(0.0));
            let cy = format_coord_mils(g.center_y_mils.unwrap_or(0.0));
            let r = format_coord_mils(g.radius_mils.unwrap_or(0.0));
            let mut props = vec![
                format!("layer: {}", g.layer),
                format!("center: ({}, {})", cx, cy),
                format!("radius: {}", r),
            ];
            if let Some(sa) = g.start_angle {
                props.push(format!("start_angle: {}", format_float(sa)));
            }
            if let Some(ea) = g.end_angle {
                props.push(format!("end_angle: {}", format_float(ea)));
            }
            if let Some(w) = g.width_mils {
                if w != 0.0 {
                    props.push(format!("width: {}", format_coord_mils(w)));
                }
            }
            out.push_str(&format!("{}arc {{ {} }}\n", p, props.join(", ")));
        }
        "fill" => {
            let x1 = format_coord_mils(g.corner1_x_mils.unwrap_or(0.0));
            let y1 = format_coord_mils(g.corner1_y_mils.unwrap_or(0.0));
            let x2 = format_coord_mils(g.corner2_x_mils.unwrap_or(0.0));
            let y2 = format_coord_mils(g.corner2_y_mils.unwrap_or(0.0));
            let mut props = vec![
                format!("layer: {}", g.layer),
                format!("corner1: ({}, {})", x1, y1),
                format!("corner2: ({}, {})", x2, y2),
            ];
            if let Some(rot) = g.rotation {
                if rot != 0.0 {
                    props.push(format!("rotation: {}", format_float(rot)));
                }
            }
            out.push_str(&format!("{}fill {{ {} }}\n", p, props.join(", ")));
        }
        "text" => {
            let lx = format_coord_mils(g.location_x_mils.unwrap_or(0.0));
            let ly = format_coord_mils(g.location_y_mils.unwrap_or(0.0));
            let text = g.text.as_deref().unwrap_or("");
            let mut props = vec![
                format!("layer: {}", g.layer),
                format!("at: ({}, {})", lx, ly),
                format!("text: {}", quote_string(text)),
            ];
            if let Some(rot) = g.rotation {
                if rot != 0.0 {
                    props.push(format!("rotation: {}", format_float(rot)));
                }
            }
            out.push_str(&format!("{}text {{ {} }}\n", p, props.join(", ")));
        }
        "via" => {
            let lx = format_coord_mils(g.location_x_mils.unwrap_or(0.0));
            let ly = format_coord_mils(g.location_y_mils.unwrap_or(0.0));
            let mut props = vec![format!("at: ({}, {})", lx, ly)];
            if let Some(d) = g.diameter_mils {
                props.push(format!("diameter: {}", format_coord_mils(d)));
            }
            if let Some(h) = g.hole_size_mils {
                props.push(format!("hole_size: {}", format_coord_mils(h)));
            }
            out.push_str(&format!("{}via {{ {} }}\n", p, props.join(", ")));
        }
        "region" => {
            if !g.outline.is_empty() {
                let verts: Vec<String> = g.outline.iter()
                    .map(|(x, y)| format!("({}, {})", format_coord_mils(*x), format_coord_mils(*y)))
                    .collect();
                out.push_str(&format!(
                    "{}region {{ layer: {}, outline: [{}] }}\n",
                    p, g.layer, verts.join(", ")
                ));
            }
        }
        _ => {
            out.push_str(&format!("{}// unknown graphic: {}\n", p, g.graphic_type));
        }
    }
}

// ── Component ─────────────────────────────────────────────────────────────────

fn dump_component(out: &mut String, comp: &altium_format::SchLibComponentDumpView) {
    out.push_str(&format!("component {} {{\n", quote_entity_name(&comp.lib_reference)));

    // Group pins and graphics by owner_part_id > 0 into part blocks
    let part_ids: Vec<i32> = {
        let mut ids: Vec<i32> = comp.pins.iter()
            .filter(|p| p.owner_part_id > 0)
            .map(|p| p.owner_part_id)
            .chain(
                comp.graphics.iter()
                    .filter(|g| g.owner_part_id > 0)
                    .map(|g| g.owner_part_id)
            )
            .collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    };

    if part_ids.is_empty() {
        // No multi-part: emit all pins and graphics at top level
        for graphic in &comp.graphics {
            if graphic.owner_part_id <= 0 {
                dump_graphic(out, graphic, 4);
            }
        }
        for pin in &comp.pins {
            if pin.owner_part_id <= 0 {
                dump_pin(out, pin, 4);
            }
        }
    } else {
        // Emit shared graphics/pins (owner_part_id <= 0) at top level
        for graphic in &comp.graphics {
            if graphic.owner_part_id <= 0 {
                dump_graphic(out, graphic, 4);
            }
        }
        for pin in &comp.pins {
            if pin.owner_part_id <= 0 {
                dump_pin(out, pin, 4);
            }
        }
        // Emit per-part blocks
        for part_id in &part_ids {
            out.push_str(&format!("    part {} {{\n", part_id));
            for graphic in &comp.graphics {
                if graphic.owner_part_id == *part_id {
                    dump_graphic(out, graphic, 8);
                }
            }
            for pin in &comp.pins {
                if pin.owner_part_id == *part_id {
                    dump_pin(out, pin, 8);
                }
            }
            out.push_str("    }\n");
        }
    }

    // Parameters (skip Designator/Comment — already handled above)
    for param in &comp.parameters {
        dump_parameter(out, param, 4);
    }

    // Aliases
    for alias in &comp.aliases {
        out.push_str(&format!("    alias {}\n", quote_entity_name(alias)));
    }

    // Footprint maps
    for fp in &comp.footprints {
        dump_footprint_map(out, fp, 4);
    }

    out.push_str("}\n");
}

// ── Pin ───────────────────────────────────────────────────────────────────────

fn dump_pin(out: &mut String, pin: &altium_format::SchLibPinDumpView, indent: usize) {
    let pad = " ".repeat(indent);
    let x = format_coord_mils(pin.location_x_mils);
    let y = format_coord_mils(pin.location_y_mils);
    let mut parts = vec![
        format!("at: ({}, {})", x, y),
        format!("orientation: {}", pin.orientation),
        format!("electrical: {}", pin.electrical),
    ];
    // Default pin length in Altium is 25 mils (Coord::from_mils(25) per dump.md spec).
    if (pin.length_mils - 25.0).abs() > 0.001 {
        parts.push(format!("length: {}", format_coord_mils(pin.length_mils)));
    }
    if !pin.name.is_empty() {
        parts.push(format!("name: {}", quote_string(&pin.name)));
    }
    if pin.is_hidden {
        parts.push("is_hidden: true".to_owned());
    }
    if !pin.hidden_net_name.is_empty() {
        parts.push(format!("hidden_net_name: {}", quote_string(&pin.hidden_net_name)));
    }
    out.push_str(&format!(
        "{}pin {} {{ {} }}\n",
        pad,
        quote_entity_name(&pin.designator),
        parts.join(", ")
    ));
}

// ── Graphic ───────────────────────────────────────────────────────────────────

fn dump_graphic(out: &mut String, g: &altium_format::SchLibGraphicDumpView, indent: usize) {
    let pad = " ".repeat(indent);
    match g.record_type.as_str() {
        "line" => {
            let x1 = format_coord_mils(g.location_x_mils);
            let y1 = format_coord_mils(g.location_y_mils);
            let x2 = format_coord_mils(g.corner_x_mils.unwrap_or(0.0));
            let y2 = format_coord_mils(g.corner_y_mils.unwrap_or(0.0));
            out.push_str(&format!(
                "{}line {{ from: ({}, {}), to: ({}, {}) }}\n",
                pad, x1, y1, x2, y2
            ));
        }
        "rectangle" => {
            let x1 = format_coord_mils(g.location_x_mils);
            let y1 = format_coord_mils(g.location_y_mils);
            let x2 = format_coord_mils(g.corner_x_mils.unwrap_or(0.0));
            let y2 = format_coord_mils(g.corner_y_mils.unwrap_or(0.0));
            let mut props = vec![
                format!("location: ({}, {})", x1, y1),
                format!("corner: ({}, {})", x2, y2),
            ];
            if g.is_solid {
                props.push("is_solid: true".to_owned());
            }
            out.push_str(&format!(
                "{}rectangle {{ {} }}\n",
                pad,
                props.join(", ")
            ));
        }
        "arc" => {
            let x = format_coord_mils(g.location_x_mils);
            let y = format_coord_mils(g.location_y_mils);
            let r = format_coord_mils(g.radius_mils.unwrap_or(0.0));
            let mut props = vec![
                format!("center: ({}, {})", x, y),
                format!("radius: {}", r),
            ];
            if let Some(sa) = g.start_angle {
                props.push(format!("start_angle: {}", format_float(sa)));
            }
            if let Some(ea) = g.end_angle {
                props.push(format!("end_angle: {}", format_float(ea)));
            }
            out.push_str(&format!("{}arc {{ {} }}\n", pad, props.join(", ")));
        }
        "elliptical_arc" => {
            let x = format_coord_mils(g.location_x_mils);
            let y = format_coord_mils(g.location_y_mils);
            let r = format_coord_mils(g.radius_mils.unwrap_or(0.0));
            let sr = format_coord_mils(g.secondary_radius_mils.unwrap_or(0.0));
            let mut props = vec![
                format!("center: ({}, {})", x, y),
                format!("radius: {}", r),
                format!("secondary_radius: {}", sr),
            ];
            if let Some(sa) = g.start_angle {
                props.push(format!("start_angle: {}", format_float(sa)));
            }
            if let Some(ea) = g.end_angle {
                props.push(format!("end_angle: {}", format_float(ea)));
            }
            out.push_str(&format!("{}elliptical_arc {{ {} }}\n", pad, props.join(", ")));
        }
        "ellipse" => {
            let x = format_coord_mils(g.location_x_mils);
            let y = format_coord_mils(g.location_y_mils);
            let r = format_coord_mils(g.radius_mils.unwrap_or(0.0));
            let sr = format_coord_mils(g.secondary_radius_mils.unwrap_or(0.0));
            let mut props = vec![
                format!("center: ({}, {})", x, y),
                format!("radius: {}", r),
                format!("secondary_radius: {}", sr),
            ];
            if g.is_solid {
                props.push("is_solid: true".to_owned());
            }
            out.push_str(&format!("{}ellipse {{ {} }}\n", pad, props.join(", ")));
        }
        "pie" => {
            let x = format_coord_mils(g.location_x_mils);
            let y = format_coord_mils(g.location_y_mils);
            let r = format_coord_mils(g.radius_mils.unwrap_or(0.0));
            let mut props = vec![
                format!("center: ({}, {})", x, y),
                format!("radius: {}", r),
            ];
            if let Some(sa) = g.start_angle {
                props.push(format!("start_angle: {}", format_float(sa)));
            }
            if let Some(ea) = g.end_angle {
                props.push(format!("end_angle: {}", format_float(ea)));
            }
            out.push_str(&format!("{}pie {{ {} }}\n", pad, props.join(", ")));
        }
        "polyline" => {
            let verts: Vec<String> = g.vertices.iter()
                .map(|(x, y)| format!("({}, {})", format_coord_mils(*x), format_coord_mils(*y)))
                .collect();
            out.push_str(&format!(
                "{}polyline {{ vertices: [{}] }}\n",
                pad,
                verts.join(", ")
            ));
        }
        "polygon" => {
            let verts: Vec<String> = g.vertices.iter()
                .map(|(x, y)| format!("({}, {})", format_coord_mils(*x), format_coord_mils(*y)))
                .collect();
            let mut props = vec![format!("vertices: [{}]", verts.join(", "))];
            if g.is_solid {
                props.push("is_solid: true".to_owned());
            }
            out.push_str(&format!("{}polygon {{ {} }}\n", pad, props.join(", ")));
        }
        "bezier" => {
            let verts: Vec<String> = g.vertices.iter()
                .map(|(x, y)| format!("({}, {})", format_coord_mils(*x), format_coord_mils(*y)))
                .collect();
            out.push_str(&format!(
                "{}bezier {{ vertices: [{}] }}\n",
                pad,
                verts.join(", ")
            ));
        }
        "label" => {
            let x = format_coord_mils(g.location_x_mils);
            let y = format_coord_mils(g.location_y_mils);
            let text = g.text.as_deref().unwrap_or("");
            out.push_str(&format!(
                "{}label {{ at: ({}, {}), text: {} }}\n",
                pad, x, y, quote_string(text)
            ));
        }
        "text_frame" => {
            let x1 = format_coord_mils(g.location_x_mils);
            let y1 = format_coord_mils(g.location_y_mils);
            let x2 = format_coord_mils(g.corner_x_mils.unwrap_or(0.0));
            let y2 = format_coord_mils(g.corner_y_mils.unwrap_or(0.0));
            let text = g.text.as_deref().unwrap_or("");
            out.push_str(&format!(
                "{}text_frame {{ location: ({}, {}), corner: ({}, {}), text: {} }}\n",
                pad, x1, y1, x2, y2, quote_string(text)
            ));
        }
        "image" => {
            let x1 = format_coord_mils(g.location_x_mils);
            let y1 = format_coord_mils(g.location_y_mils);
            let x2 = format_coord_mils(g.corner_x_mils.unwrap_or(0.0));
            let y2 = format_coord_mils(g.corner_y_mils.unwrap_or(0.0));
            let file = g.file_name.as_deref().unwrap_or("");
            out.push_str(&format!(
                "{}image {{ location: ({}, {}), corner: ({}, {}), file: {} }}\n",
                pad, x1, y1, x2, y2, quote_string(file)
            ));
        }
        _ => {
            // Unknown graphic type — emit as comment
            out.push_str(&format!("{}// unknown graphic: {}\n", pad, g.record_type));
        }
    }
}

// ── Parameter ─────────────────────────────────────────────────────────────────

fn dump_parameter(out: &mut String, param: &altium_format::SchLibParameterDumpView, indent: usize) {
    let pad = " ".repeat(indent);
    if param.is_hidden {
        out.push_str(&format!(
            "{}parameter {} = {} {{ is_hidden: true }}\n",
            pad,
            quote_entity_name(&param.name),
            quote_string(&param.text)
        ));
    } else {
        out.push_str(&format!(
            "{}parameter {} = {}\n",
            pad,
            quote_entity_name(&param.name),
            quote_string(&param.text)
        ));
    }
}

// ── Footprint map ─────────────────────────────────────────────────────────────

fn dump_footprint_map(out: &mut String, fp: &altium_format::SchLibFootprintDumpView, indent: usize) {
    let pad = " ".repeat(indent);
    out.push_str(&format!(
        "{}footprint {} {{\n",
        pad,
        quote_entity_name(&fp.footprint_ref)
    ));
    if !fp.description.is_empty() {
        out.push_str(&format!("{}    description: {}\n", pad, quote_string(&fp.description)));
    }
    for m in &fp.pin_pad_maps {
        if m.pad_names.is_empty() {
            continue;
        }
        let pads: Vec<String> = m.pad_names.iter()
            .map(|p| quote_entity_name(p))
            .collect();
        out.push_str(&format!(
            "{}    map {} -> {}\n",
            pad,
            quote_entity_name(&m.pin_name),
            pads.join(", ")
        ));
    }
    out.push_str(&format!("{}}}\n", pad));
}

// ── Formatting helpers ────────────────────────────────────────────────────────

/// Format a coordinate in mils as the most natural unit.
/// Prefers mm if the value is "clean" (exact to 3 decimal places in mm).
/// Falls back to mils otherwise.
pub fn format_coord_mils(mils: f64) -> String {
    let mm = mils * 0.0254;
    if (mm * 1000.0).round() == mm * 1000.0 && mm.abs() >= 0.001 {
        format!("{}mm", format_float(mm))
    } else {
        format!("{}mil", format_float(mils))
    }
}

/// Format a float, removing trailing zeros after the decimal point.
pub fn format_float(v: f64) -> String {
    let s = format!("{:.4}", v);
    let s = s.trim_end_matches('0');
    let s = s.trim_end_matches('.');
    s.to_string()
}

/// Quote an entity name: bare if it's a valid ident or integer, quoted otherwise.
pub fn quote_entity_name(name: &str) -> String {
    if name.parse::<i64>().is_ok() {
        return name.to_string();
    }
    if is_valid_ident(name) {
        return name.to_string();
    }
    format!("\"{}\"", name.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Always quotes the string value with backslash escaping.
pub fn quote_string(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Returns true if the string is a valid bare identifier in the spec language.
fn is_valid_ident(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_coord_mils_mm_clean() {
        // 100 mils = 2.54 mm (clean value)
        let s = format_coord_mils(100.0);
        assert_eq!(s, "2.54mm");
    }

    #[test]
    fn test_format_coord_mils_mils_fallback() {
        // 1 mil = 0.0254 mm (not clean to 3 decimal places ... 0.0254 * 1000 = 25.4, round=25, not clean)
        // Actually 0.0254 * 1000.0 = 25.4, round() = 25, 25 != 25.4 → falls back to mils
        let s = format_coord_mils(1.0);
        assert_eq!(s, "1mil");
    }

    #[test]
    fn test_format_coord_mils_zero() {
        // 0 mils: mm abs < 0.001 so falls back to mils
        let s = format_coord_mils(0.0);
        assert_eq!(s, "0mil");
    }

    #[test]
    fn test_format_float_trailing_zeros() {
        assert_eq!(format_float(1.5), "1.5");
        assert_eq!(format_float(2.0), "2");
        assert_eq!(format_float(1.2500), "1.25");
    }

    #[test]
    fn test_quote_entity_name_valid_ident() {
        assert_eq!(quote_entity_name("foo"), "foo");
        assert_eq!(quote_entity_name("_bar"), "_bar");
        assert_eq!(quote_entity_name("A1"), "A1");
    }

    #[test]
    fn test_quote_entity_name_integer() {
        assert_eq!(quote_entity_name("1"), "1");
        assert_eq!(quote_entity_name("42"), "42");
        assert_eq!(quote_entity_name("-1"), "-1");
    }

    #[test]
    fn test_quote_entity_name_needs_quotes() {
        assert_eq!(quote_entity_name("foo bar"), "\"foo bar\"");
        assert_eq!(quote_entity_name("a-b"), "\"a-b\"");
        assert_eq!(quote_entity_name(""), "\"\"");
        assert_eq!(quote_entity_name("has\"quote"), "\"has\\\"quote\"");
    }

    #[test]
    fn test_quote_string_always_quoted() {
        assert_eq!(quote_string("hello"), "\"hello\"");
        assert_eq!(quote_string("say \"hi\""), "\"say \\\"hi\\\"\"");
        assert_eq!(quote_string("back\\slash"), "\"back\\\\slash\"");
    }
}
