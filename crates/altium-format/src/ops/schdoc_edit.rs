// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Schematic document editing commands.
//!
//! This module contains editing-related commands for schematic documents that
//! modify the document content. Extracted from schdoc.rs for modularity.
//!
//! # V2 Migration Note
//! This module uses v1 types (EditSession, SchRecord, etc.).
//! TODO: Migrate to v2 types when SchDocV2 provides equivalent editing functionality.

#![allow(deprecated)]

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::edit::{EditSession, Orientation};
use crate::ops::output::*;
use crate::records::sch::{PortIoType, PowerObjectStyle, SchRecord, TextOrientations};
use crate::types::{Coord, CoordPoint, Unit};

/// Parse a unit value or interpret as mils.
fn parse_unit_value_or_mil(s: &str) -> Result<f64, String> {
    let s = s.trim();

    // Try parsing with unit suffix first
    if let Ok((coord, unit)) = Unit::parse_with_unit(s) {
        // If the parser detected a unit suffix (not DxpDefault which means no suffix), use it
        if unit != Unit::DxpDefault {
            return Ok(coord.to_mils());
        }
    }

    // Plain number (no unit suffix) - interpret as mils
    s.parse::<f64>().map_err(|_| {
        format!(
            "Invalid value '{}': expected number with optional unit (e.g., '100mil', '2.54mm')",
            s
        )
    })
}

/// Create a new empty schematic document.
pub fn cmd_new(output: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let session = EditSession::new();

    match output {
        Some(path) => {
            session.doc.save_to_file(&path)?;
            println!("Created new schematic: {}", path.display());
        }
        None => {
            println!("Created new empty schematic in memory.");
            println!("Use --output to save to a file.");
        }
    }

    Ok(())
}

/// Validate a schematic document.
pub fn cmd_validate(path: &Path) -> Result<SchDocValidationResult, Box<dyn std::error::Error>> {
    let session = EditSession::open(path)?;

    let validation_errors = session.validate();

    let errors = validation_errors
        .iter()
        .map(|e| ValidationError {
            kind: format!("{:?}", e.kind),
            message: e.message.clone(),
            location: e.location.map(|l| (l.x.to_mils(), l.y.to_mils())),
            components: e.components.clone(),
        })
        .collect();

    Ok(SchDocValidationResult {
        path: path.display().to_string(),
        is_valid: validation_errors.is_empty(),
        errors,
    })
}

/// Add a component from a library to the schematic.
#[allow(clippy::too_many_arguments)]
pub fn cmd_add_component(
    path: &Path,
    library: &Path,
    component: &str,
    x: &str,
    y: &str,
    designator: Option<&str>,
    rotation: i32,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let x_mils = parse_unit_value_or_mil(x)?;
    let y_mils = parse_unit_value_or_mil(y)?;

    let mut session = EditSession::open(path)?;

    session.load_library(library)?;

    let location = CoordPoint::from_mils(x_mils, y_mils);
    let orientation = match rotation {
        0 => Orientation::Normal,
        90 => Orientation::Rotated90,
        180 => Orientation::Rotated180,
        270 => Orientation::Rotated270,
        _ => return Err(format!("Invalid rotation: {}. Use 0, 90, 180, or 270.", rotation).into()),
    };

    let _index = session.add_component(component, location, orientation, designator)?;

    let output_path = output.as_deref().unwrap_or(path);
    session.save(output_path)?;

    let des = designator.unwrap_or("(auto)");
    println!(
        "Added component {} at ({:.0}, {:.0}) mils as {}",
        component, x_mils, y_mils, des
    );
    println!("Saved to: {}", output_path.display());

    Ok(())
}

/// Suggest placement locations for a component.
pub fn cmd_suggest_placement(
    path: &Path,
    library: &Path,
    component: &str,
    near: Option<String>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;

    session.load_library(library)?;

    let suggestions = session.suggest_component_placement(component, near.as_deref());

    if json {
        #[derive(Serialize)]
        struct Suggestion {
            x: f64,
            y: f64,
            rotation: i32,
            score: f64,
            reason: String,
        }

        let result: Vec<_> = suggestions
            .iter()
            .map(|s| Suggestion {
                x: s.location.x.to_mils(),
                y: s.location.y.to_mils(),
                rotation: (s.orientation.rotation_degrees() as i32) % 360,
                score: s.score,
                reason: s.reason.clone(),
            })
            .collect();

        println!(
            "{}",
            serde_json::to_string_pretty(&result)
                .map_err(|e| format!("JSON serialization error: {}", e))?
        );
    } else {
        println!("Placement suggestions for '{}':", component);
        if suggestions.is_empty() {
            println!("  No suitable locations found.");
        } else {
            for (i, s) in suggestions.iter().enumerate() {
                println!(
                    "  {}. ({:.0}, {:.0}) mils - score: {:.2} - {}",
                    i + 1,
                    s.location.x.to_mils(),
                    s.location.y.to_mils(),
                    s.score,
                    s.reason
                );
            }
        }
    }

    Ok(())
}

/// Move a component to a new location.
pub fn cmd_move_component(
    path: &Path,
    designator: &str,
    x: &str,
    y: &str,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let x_mils = parse_unit_value_or_mil(x)?;
    let y_mils = parse_unit_value_or_mil(y)?;

    let mut session = EditSession::open(path)?;

    // Find component by designator
    let components = session
        .layout()
        .get_placed_components(&session.doc.primitives);
    let comp = components
        .iter()
        .find(|c| c.designator == designator)
        .ok_or_else(|| format!("Component not found: {}", designator))?;

    let index = comp.index;
    let new_location = CoordPoint::from_mils(x_mils, y_mils);

    session.move_component(index, new_location)?;

    let output_path = output.as_deref().unwrap_or(path);
    session.save(output_path)?;

    println!(
        "Moved {} to ({:.0}, {:.0}) mils",
        designator, x_mils, y_mils
    );
    println!("Saved to: {}", output_path.display());

    Ok(())
}

/// Delete a component from the schematic.
pub fn cmd_delete_component(
    path: &Path,
    designator: &str,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;

    // Find component by designator
    let components = session
        .layout()
        .get_placed_components(&session.doc.primitives);
    let comp = components
        .iter()
        .find(|c| c.designator == designator)
        .ok_or_else(|| format!("Component not found: {}", designator))?;

    let index = comp.index;

    session.delete_component(index)?;

    let output_path = output.as_deref().unwrap_or(path);
    session.save(output_path)?;

    println!("Deleted component {}", designator);
    println!("Saved to: {}", output_path.display());

    Ok(())
}

/// Add a wire with specified vertices.
pub fn cmd_add_wire(
    path: &Path,
    vertices_str: &str,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;

    // Parse vertices from string
    let values: Vec<f64> = vertices_str
        .split(',')
        .map(|s| s.trim().parse::<f64>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Invalid vertex format: {}", e))?;

    if values.len() < 4 || values.len() % 2 != 0 {
        return Err(
            "Vertices must be pairs of X,Y coordinates (at least 2 points)"
                .to_string()
                .into(),
        );
    }

    let vertices: Vec<CoordPoint> = values
        .chunks(2)
        .map(|chunk| CoordPoint::from_mils(chunk[0], chunk[1]))
        .collect();

    session.add_wire(&vertices)?;

    let output_path = output.as_deref().unwrap_or(path);
    session.save(output_path)?;

    println!("Added wire with {} vertices", vertices.len());
    println!("Saved to: {}", output_path.display());

    Ok(())
}

/// Parse a point specification for routing targets.
fn parse_point_spec(spec: &str, session: &EditSession) -> Result<(CoordPoint, String), String> {
    // Try parsing as coordinates first (x,y format)
    if let Some((x_str, y_str)) = spec.split_once(',') {
        let x: f64 = x_str.trim().parse().map_err(|_| {
            format!(
                "Invalid x coordinate in '{}'. Use: Component.Pin, :Port, %NetLabel, @Power, or x,y",
                spec
            )
        })?;
        let y: f64 = y_str.trim().parse().map_err(|_| {
            format!(
                "Invalid y coordinate in '{}'. Use: Component.Pin, :Port, %NetLabel, @Power, or x,y",
                spec
            )
        })?;
        let point = CoordPoint::from_mils(x, y);
        return Ok((point, format!("({:.0}, {:.0})", x, y)));
    }

    // Try parsing as port reference (:PortName format)
    if let Some(port_name) = spec.strip_prefix(':') {
        let ports: Vec<_> = session
            .doc
            .primitives
            .iter()
            .filter_map(|p| match p {
                SchRecord::Port(port) => Some(port),
                _ => None,
            })
            .collect();

        let port = ports.iter().find(|p| p.name == port_name).ok_or_else(|| {
            format!(
                "Port not found: '{}'. Available ports: {:?}",
                port_name,
                ports.iter().map(|p| &p.name).collect::<Vec<_>>()
            )
        })?;

        let point = CoordPoint::new(
            Coord::from_raw(port.graphical.location_x),
            Coord::from_raw(port.graphical.location_y),
        );
        return Ok((point, format!(":{}", port_name)));
    }

    // Try parsing as net label reference (%NetLabel format)
    if let Some(label_name) = spec.strip_prefix('%') {
        let labels: Vec<_> = session
            .doc
            .primitives
            .iter()
            .filter_map(|p| match p {
                SchRecord::NetLabel(label) => Some(label),
                _ => None,
            })
            .collect();

        let label = labels
            .iter()
            .find(|l| l.label.text == label_name)
            .ok_or_else(|| {
                format!(
                    "Net label not found: '{}'. Available net labels: {:?}",
                    label_name,
                    labels.iter().map(|l| &l.label.text).collect::<Vec<_>>()
                )
            })?;

        let point = CoordPoint::new(
            Coord::from_raw(label.label.graphical.location_x),
            Coord::from_raw(label.label.graphical.location_y),
        );
        return Ok((point, format!("%{}", label_name)));
    }

    // Try parsing as power port reference (@PowerPort format)
    if let Some(power_name) = spec.strip_prefix('@') {
        let powers: Vec<_> = session
            .doc
            .primitives
            .iter()
            .filter_map(|p| match p {
                SchRecord::PowerObject(power) => Some(power),
                _ => None,
            })
            .collect();

        let power = powers
            .iter()
            .find(|p| p.text == power_name)
            .ok_or_else(|| {
                format!(
                    "Power port not found: '{}'. Available power ports: {:?}",
                    power_name,
                    powers.iter().map(|p| &p.text).collect::<Vec<_>>()
                )
            })?;

        let point = CoordPoint::new(
            Coord::from_raw(power.graphical.location_x),
            Coord::from_raw(power.graphical.location_y),
        );
        return Ok((point, format!("@{}", power_name)));
    }

    // Try parsing as Component.Pin reference
    if let Some((component, pin)) = spec.split_once('.') {
        let components = session
            .layout()
            .get_placed_components(&session.doc.primitives);
        let comp = components
            .iter()
            .find(|c| c.designator == component)
            .ok_or_else(|| {
                format!(
                    "Component not found: '{}'. Available components: {:?}",
                    component,
                    components.iter().map(|c| &c.designator).collect::<Vec<_>>()
                )
            })?;

        let pin_loc = comp
            .pin_locations
            .iter()
            .find(|p| p.designator == pin || p.name == pin)
            .ok_or_else(|| {
                format!(
                    "Pin '{}' not found on component '{}'. Available pins: {:?}",
                    pin,
                    component,
                    comp.pin_locations
                        .iter()
                        .map(|p| format!("{}/{}", p.designator, p.name))
                        .collect::<Vec<_>>()
                )
            })?;

        return Ok((pin_loc.location, format!("{}.{}", component, pin)));
    }

    Err(format!(
        "Invalid point specification: '{}'. Expected: Component.Pin, :Port, %NetLabel, @Power, or x,y",
        spec
    ))
}

/// Route a wire between two points.
pub fn cmd_route_wire(
    path: &Path,
    from: &str,
    to: &str,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;

    let (start, from_desc) = parse_point_spec(from, &session)?;
    let (end, to_desc) = parse_point_spec(to, &session)?;

    session.route_wire(start, end)?;

    let output_path = output.as_deref().unwrap_or(path);
    session.save(output_path)?;

    println!("Routed wire from {} to {}", from_desc, to_desc);
    println!("Saved to: {}", output_path.display());

    Ok(())
}

/// Connect two pins with a wire.
pub fn cmd_connect_pins(
    path: &Path,
    from_component: &str,
    from_pin: &str,
    to_component: &str,
    to_pin: &str,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;

    session.connect_pins(from_component, from_pin, to_component, to_pin)?;

    let output_path = output.as_deref().unwrap_or(path);
    session.save(output_path)?;

    println!(
        "Connected {}.{} to {}.{}",
        from_component, from_pin, to_component, to_pin
    );
    println!("Saved to: {}", output_path.display());

    Ok(())
}

/// Delete a wire by index.
pub fn cmd_delete_wire(
    path: &Path,
    index: usize,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;

    session.delete_wire(index)?;

    let output_path = output.as_deref().unwrap_or(path);
    session.save(output_path)?;

    println!("Deleted wire at index {}", index);
    println!("Saved to: {}", output_path.display());

    Ok(())
}

/// Add a net label at a location.
pub fn cmd_add_net_label(
    path: &Path,
    name: &str,
    x: &str,
    y: &str,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let x_mils = parse_unit_value_or_mil(x)?;
    let y_mils = parse_unit_value_or_mil(y)?;

    let mut session = EditSession::open(path)?;

    let location = CoordPoint::from_mils(x_mils, y_mils);

    session.add_net_label(name, location)?;

    let output_path = output.as_deref().unwrap_or(path);
    session.save(output_path)?;

    println!(
        "Added net label '{}' at ({:.0}, {:.0}) mils",
        name, x_mils, y_mils
    );
    println!("Saved to: {}", output_path.display());

    Ok(())
}

/// Smart-wire: create a wire stub from a pin and attach a net label or power port.
pub fn cmd_smart_wire(
    path: &Path,
    component: &str,
    pin: &str,
    net: &str,
    power_style: Option<&str>,
    wire_length_mils: f64,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;

    let style = match power_style {
        Some(s) => {
            let parsed = match s.to_lowercase().as_str() {
                "bar" | "power_bar" => PowerObjectStyle::Bar,
                "arrow" => PowerObjectStyle::Arrow,
                "wave" => PowerObjectStyle::Wave,
                "ground" | "gnd" => PowerObjectStyle::Ground,
                "power_ground" | "pgnd" => PowerObjectStyle::PowerGround,
                "signal_ground" | "sgnd" => PowerObjectStyle::SignalGround,
                "earth_ground" | "earth" => PowerObjectStyle::EarthGround,
                "circle" => PowerObjectStyle::Circle,
                _ => {
                    return Err(format!(
                        "Unknown power style: {}. Use: bar, arrow, wave, ground, power_ground, signal_ground, earth_ground, circle",
                        s
                    ).into());
                }
            };
            Some(parsed)
        }
        None => None,
    };

    let (wire_idx, label_idx) = session.smart_wire_pin(
        component,
        pin,
        net,
        style,
        wire_length_mils,
    )?;

    let output_path = output.as_deref().unwrap_or(path);
    session.save(output_path)?;

    let kind = if power_style.is_some() { "power port" } else { "net label" };
    println!(
        "Smart-wired {}.{} -> {} '{}' (wire #{}, {} #{})",
        component, pin, kind, net, wire_idx, kind, label_idx
    );
    println!("Saved to: {}", output_path.display());

    Ok(())
}

/// Batch smart-wire: wire multiple pins from a mapping string.
///
/// Format: "COMP.PIN=NET,COMP.PIN=NET:power_style,..."
/// Examples: "U1.3=VCC:bar,U1.4=GND:ground,U1.5=SDA,U1.6=SCL"
pub fn cmd_smart_wire_batch(
    path: &Path,
    mappings: &str,
    wire_length_mils: f64,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;

    let mut count = 0;
    for mapping in mappings.split(',') {
        let mapping = mapping.trim();
        if mapping.is_empty() {
            continue;
        }

        // Parse "COMP.PIN=NET" or "COMP.PIN=NET:power_style"
        let (pin_spec, net_spec) = mapping
            .split_once('=')
            .ok_or_else(|| format!("Invalid mapping '{}': expected COMP.PIN=NET", mapping))?;

        let (component, pin) = pin_spec
            .split_once('.')
            .ok_or_else(|| format!("Invalid pin spec '{}': expected COMP.PIN", pin_spec))?;

        let (net, power_style) = if let Some((n, s)) = net_spec.split_once(':') {
            let parsed = match s.to_lowercase().as_str() {
                "bar" | "power_bar" => PowerObjectStyle::Bar,
                "arrow" => PowerObjectStyle::Arrow,
                "wave" => PowerObjectStyle::Wave,
                "ground" | "gnd" => PowerObjectStyle::Ground,
                "power_ground" | "pgnd" => PowerObjectStyle::PowerGround,
                "signal_ground" | "sgnd" => PowerObjectStyle::SignalGround,
                "earth_ground" | "earth" => PowerObjectStyle::EarthGround,
                "circle" => PowerObjectStyle::Circle,
                _ => {
                    return Err(format!(
                        "Unknown power style '{}' in mapping '{}'. Use: bar, arrow, wave, ground, power_ground, signal_ground, earth_ground, circle",
                        s, mapping
                    ).into());
                }
            };
            (n, Some(parsed))
        } else {
            (net_spec, None)
        };

        session.smart_wire_pin(component, pin, net, power_style, wire_length_mils)?;
        count += 1;

        let kind = if power_style.is_some() { "power" } else { "net" };
        println!("  {}.{} -> {} '{}'", component, pin, kind, net);
    }

    let output_path = output.as_deref().unwrap_or(path);
    session.save(output_path)?;

    println!("Smart-wired {} pins. Saved to: {}", count, output_path.display());

    Ok(())
}

/// Add a power port.
pub fn cmd_add_power(
    path: &Path,
    name: &str,
    x: &str,
    y: &str,
    style: &str,
    orientation: &str,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let x_mils = parse_unit_value_or_mil(x)?;
    let y_mils = parse_unit_value_or_mil(y)?;

    let mut session = EditSession::open(path)?;

    let location = CoordPoint::from_mils(x_mils, y_mils);

    let power_style = match style.to_lowercase().as_str() {
        "bar" | "power_bar" => PowerObjectStyle::Bar,
        "arrow" => PowerObjectStyle::Arrow,
        "wave" => PowerObjectStyle::Wave,
        "ground" | "gnd" => PowerObjectStyle::Ground,
        "power_ground" | "pgnd" => PowerObjectStyle::PowerGround,
        "signal_ground" | "sgnd" => PowerObjectStyle::SignalGround,
        "earth_ground" | "earth" => PowerObjectStyle::EarthGround,
        "circle" => PowerObjectStyle::Circle,
        _ => {
            return Err(format!(
                "Unknown power style: {}. Use: bar, arrow, wave, ground, power_ground, signal_ground, earth_ground, circle",
                style
            )
            .into())
        }
    };

    // TextOrientations is a bitflags struct: NONE=0, ROTATED=1, FLIPPED=2
    let orient = match orientation.to_lowercase().as_str() {
        "up" | "0" => TextOrientations::NONE,
        "left" | "90" => TextOrientations::ROTATED,
        "down" | "180" => TextOrientations::FLIPPED,
        "right" | "270" => TextOrientations::ROTATED | TextOrientations::FLIPPED,
        _ => {
            return Err(format!(
                "Unknown orientation: {}. Use: up, down, left, right",
                orientation
            )
            .into());
        }
    };

    session.add_power_port(name, location, power_style, orient)?;

    let output_path = output.as_deref().unwrap_or(path);
    session.save(output_path)?;

    println!(
        "Added power port '{}' ({}) at ({:.0}, {:.0}) mils",
        name, style, x_mils, y_mils
    );
    println!("Saved to: {}", output_path.display());

    Ok(())
}

/// Add a junction at a location.
pub fn cmd_add_junction(
    path: &Path,
    x: &str,
    y: &str,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let x_mils = parse_unit_value_or_mil(x)?;
    let y_mils = parse_unit_value_or_mil(y)?;

    let mut session = EditSession::open(path)?;

    let location = CoordPoint::from_mils(x_mils, y_mils);

    session.add_junction(location)?;

    let output_path = output.as_deref().unwrap_or(path);
    session.save(output_path)?;

    println!("Added junction at ({:.0}, {:.0}) mils", x_mils, y_mils);
    println!("Saved to: {}", output_path.display());

    Ok(())
}

/// Add missing junctions where wires cross.
pub fn cmd_add_missing_junctions(
    path: &Path,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;

    let count = session.add_missing_junctions()?;

    let output_path = output.as_deref().unwrap_or(path);
    session.save(output_path)?;

    println!("Added {} missing junction(s)", count);
    println!("Saved to: {}", output_path.display());

    Ok(())
}

/// Add a port at a location.
pub fn cmd_add_port(
    path: &Path,
    name: &str,
    x: &str,
    y: &str,
    io_type: &str,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let x_mils = parse_unit_value_or_mil(x)?;
    let y_mils = parse_unit_value_or_mil(y)?;

    let mut session = EditSession::open(path)?;

    let location = CoordPoint::from_mils(x_mils, y_mils);

    let port_io_type = match io_type.to_lowercase().as_str() {
        "input" | "in" => PortIoType::Input,
        "output" | "out" => PortIoType::Output,
        "bidirectional" | "bidir" | "inout" => PortIoType::Bidirectional,
        "unspecified" | "none" => PortIoType::Unspecified,
        _ => {
            return Err(format!(
                "Unknown I/O type: {}. Use: input, output, bidirectional, unspecified",
                io_type
            )
            .into());
        }
    };

    session.add_port(name, location, port_io_type)?;

    let output_path = output.as_deref().unwrap_or(path);
    session.save(output_path)?;

    println!(
        "Added port '{}' ({}) at ({:.0}, {:.0}) mils",
        name, io_type, x_mils, y_mils
    );
    println!("Saved to: {}", output_path.display());

    Ok(())
}

/// Show the netlist for a schematic.
pub fn cmd_show_netlist(path: &Path, filter_net: Option<&str>, json: bool) -> Result<(), String> {
    let session = EditSession::open(path).map_err(|e| format!("Failed to open: {:?}", e))?;

    let netlist = session.build_netlist();

    if json {
        #[derive(Serialize)]
        struct NetJson {
            name: String,
            named: bool,
            pins: Vec<(String, String)>,
        }

        let nets: Vec<_> = netlist
            .nets
            .iter()
            .filter(|n| filter_net.map(|f| n.name.contains(f)).unwrap_or(true))
            .map(|n| NetJson {
                name: n.name.clone(),
                named: n.named,
                pins: n
                    .pins()
                    .iter()
                    .map(|(c, p)| (c.to_string(), p.to_string()))
                    .collect(),
            })
            .collect();

        println!(
            "{}",
            serde_json::to_string_pretty(&nets)
                .map_err(|e| format!("JSON serialization error: {}", e))?
        );
    } else {
        println!("Netlist ({} nets):", netlist.nets.len());
        println!();

        for net in &netlist.nets {
            if let Some(filter) = filter_net {
                if !net.name.contains(filter) {
                    continue;
                }
            }

            let pins = net.pins();
            let named_str = if net.named { "" } else { " (auto)" };
            println!("  {} [{}]{}", net.name, pins.len(), named_str);

            for (comp, pin) in pins {
                println!("    - {}.{}", comp, pin);
            }
        }
    }

    Ok(())
}

/// Find unconnected pins in the schematic.
pub fn cmd_find_unconnected(
    path: &Path,
) -> Result<SchDocUnconnectedPins, Box<dyn std::error::Error>> {
    let session = EditSession::open(path)?;

    let unconnected = session.find_unconnected_pins();

    let pins = unconnected
        .iter()
        .map(|(c, p, l)| UnconnectedPin {
            component: c.clone(),
            pin: p.clone(),
            x: l.0 as f64 / 10000.0,
            y: l.1 as f64 / 10000.0,
        })
        .collect();

    Ok(SchDocUnconnectedPins {
        path: path.display().to_string(),
        total_unconnected: unconnected.len(),
        pins,
    })
}

/// Find missing junctions in the schematic.
pub fn cmd_find_missing_junctions(
    path: &Path,
) -> Result<SchDocMissingJunctions, Box<dyn std::error::Error>> {
    let session = EditSession::open(path)?;

    let missing = session.find_missing_junctions();

    let locations: Vec<(f64, f64)> = missing
        .iter()
        .map(|l| (l.0 as f64 / 10000.0, l.1 as f64 / 10000.0))
        .collect();

    Ok(SchDocMissingJunctions {
        path: path.display().to_string(),
        total_missing: missing.len(),
        locations,
    })
}

/// Search a schematic library for components matching a pattern.
pub fn cmd_search_library(
    library: &Path,
    pattern: &str,
) -> Result<SchDocLibrarySearchResults, Box<dyn std::error::Error>> {
    use crate::io::SchLib;

    let lib = SchLib::open_file(library)?;

    let pattern_lower = pattern.to_lowercase();
    let matches: Vec<_> = lib
        .iter()
        .filter(|c| {
            c.name().to_lowercase().contains(&pattern_lower)
                || c.description().to_lowercase().contains(&pattern_lower)
        })
        .collect();

    let match_results: Vec<LibraryComponentMatch> = matches
        .iter()
        .map(|c| LibraryComponentMatch {
            name: c.name().to_string(),
            description: c.description().to_string(),
            pins: c.pin_count(),
        })
        .collect();

    Ok(SchDocLibrarySearchResults {
        library: library.display().to_string(),
        pattern: pattern.to_string(),
        total_matches: matches.len(),
        matches: match_results,
    })
}

/// List components in a schematic library.
pub fn cmd_list_library(
    library: &Path,
    verbose: bool,
) -> Result<SchDocLibraryList, Box<dyn std::error::Error>> {
    use crate::io::SchLib;

    let lib = SchLib::open_file(library)?;

    let components: Vec<LibraryComponentInfo> = lib
        .iter()
        .map(|c| LibraryComponentInfo {
            name: c.name().to_string(),
            description: c.description().to_string(),
            pins: c.pin_count(),
            primitives: if verbose {
                Some(c.primitive_count())
            } else {
                None
            },
        })
        .collect();

    Ok(SchDocLibraryList {
        library: library.display().to_string(),
        total_components: lib.component_count(),
        components,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::schlib::{cmd_create as schlib_create, cmd_gen_ic};
    use std::path::PathBuf;

    fn temp_path(ext: &str) -> PathBuf {
        let id = uuid::Uuid::new_v4();
        std::env::temp_dir().join(format!("test_{}.{}", id, ext))
    }

    /// Helper: create a SchLib with a simple 3-pin IC.
    fn create_test_library() -> PathBuf {
        let lib_path = temp_path("SchLib");
        schlib_create(&lib_path).unwrap();
        cmd_gen_ic(
            &lib_path,
            "LDO_3PIN",
            "1:VIN:power:left,2:VOUT:power:right,3:GND:power:bottom",
            Some("Test LDO".to_string()),
            "600mil",
            "200mil",
            "100mil",
        )
        .unwrap();
        cmd_gen_ic(
            &lib_path,
            "IC_4PIN",
            "1:VCC:power:top,2:IN:input:left,3:OUT:output:right,4:GND:power:bottom",
            Some("Test IC".to_string()),
            "400mil",
            "200mil",
            "100mil",
        )
        .unwrap();
        lib_path
    }

    /// E2E: Create schematic, add component, verify designator survives save/reload.
    #[test]
    #[ignore = "V2 migration: depends on v1 SchDoc I/O"]
    fn test_add_component_designator_roundtrip() {
        let lib_path = create_test_library();
        let sch_path = temp_path("SchDoc");

        // Create empty SchDoc
        crate::ops::schdoc::cmd_create(&sch_path, None).unwrap();

        // Add component with explicit designator
        cmd_add_component(
            &sch_path,
            &lib_path,
            "LDO_3PIN",
            "1000",
            "2000",
            Some("U1"),
            0,
            None,
        )
        .unwrap();

        // Reload and check designator
        let doc = crate::io::SchDoc::open_file(&sch_path).unwrap();

        // Must have exactly 1 component
        let components: Vec<_> = doc
            .primitives
            .iter()
            .filter(|r| matches!(r, SchRecord::Component(_)))
            .collect();
        assert_eq!(components.len(), 1, "Expected 1 component");

        // Must have exactly 1 Designator record (not Parameter)
        let designators: Vec<_> = doc
            .primitives
            .iter()
            .filter(|r| matches!(r, SchRecord::Designator(_)))
            .collect();
        assert_eq!(
            designators.len(),
            1,
            "Expected 1 Designator record, found {}. \
             Designator may be serializing as Parameter (RECORD=41 vs 34).",
            designators.len()
        );

        if let SchRecord::Designator(d) = &designators[0] {
            assert_eq!(d.text(), "U1", "Designator text must be U1");
        }

        std::fs::remove_file(&sch_path).ok();
        std::fs::remove_file(&lib_path).ok();
    }

    /// E2E: Multiple components get distinct designators.
    #[test]
    #[ignore = "V2 migration: depends on v1 SchDoc I/O"]
    fn test_multiple_components_distinct_designators() {
        let lib_path = create_test_library();
        let sch_path = temp_path("SchDoc");

        crate::ops::schdoc::cmd_create(&sch_path, None).unwrap();

        cmd_add_component(
            &sch_path, &lib_path, "LDO_3PIN", "1000", "2000",
            Some("U1"), 0, None,
        ).unwrap();
        cmd_add_component(
            &sch_path, &lib_path, "IC_4PIN", "3000", "2000",
            Some("U2"), 0, None,
        ).unwrap();
        cmd_add_component(
            &sch_path, &lib_path, "LDO_3PIN", "5000", "2000",
            Some("U3"), 0, None,
        ).unwrap();

        let doc = crate::io::SchDoc::open_file(&sch_path).unwrap();

        let designators: Vec<String> = doc
            .primitives
            .iter()
            .filter_map(|r| {
                if let SchRecord::Designator(d) = r {
                    Some(d.text().to_string())
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(designators.len(), 3, "Expected 3 designators");
        assert!(designators.contains(&"U1".to_string()));
        assert!(designators.contains(&"U2".to_string()));
        assert!(designators.contains(&"U3".to_string()));

        std::fs::remove_file(&sch_path).ok();
        std::fs::remove_file(&lib_path).ok();
    }

    /// E2E: Components with top/bottom pins have correct pin count after placement.
    #[test]
    #[ignore = "V2 migration: depends on v1 SchDoc I/O"]
    fn test_placed_component_preserves_all_pins() {
        let lib_path = create_test_library();
        let sch_path = temp_path("SchDoc");

        crate::ops::schdoc::cmd_create(&sch_path, None).unwrap();

        // IC_4PIN has pins on all 4 sides
        cmd_add_component(
            &sch_path, &lib_path, "IC_4PIN", "2000", "2000",
            Some("U1"), 0, None,
        ).unwrap();

        let doc = crate::io::SchDoc::open_file(&sch_path).unwrap();

        // Count pins owned by the component (owner_index = component index)
        let comp_index = doc
            .primitives
            .iter()
            .position(|r| matches!(r, SchRecord::Component(_)))
            .unwrap();

        let pin_count = doc
            .primitives
            .iter()
            .filter(|r| {
                if let SchRecord::Pin(p) = r {
                    p.graphical.base.owner_index == comp_index as i32
                } else {
                    false
                }
            })
            .count();

        assert_eq!(
            pin_count, 4,
            "IC_4PIN has 4 pins (top/bottom/left/right), but only {} survived placement",
            pin_count
        );

        std::fs::remove_file(&sch_path).ok();
        std::fs::remove_file(&lib_path).ok();
    }

    /// E2E: Power ports and net labels survive save/reload.
    #[test]
    #[ignore = "V2 migration: depends on v1 SchDoc I/O"]
    fn test_power_and_netlabel_roundtrip() {
        let sch_path = temp_path("SchDoc");
        crate::ops::schdoc::cmd_create(&sch_path, None).unwrap();

        // Add power ports
        cmd_add_power(
            &sch_path, "3V3", "1000", "2000", "bar", "up", None,
        ).unwrap();
        cmd_add_power(
            &sch_path, "GND", "1000", "1000", "ground", "down", None,
        ).unwrap();

        // Add net label
        cmd_add_net_label(
            &sch_path, "SDA", "2000", "2000", None,
        ).unwrap();

        let doc = crate::io::SchDoc::open_file(&sch_path).unwrap();

        let power_count = doc
            .primitives
            .iter()
            .filter(|r| matches!(r, SchRecord::PowerObject(_)))
            .count();
        assert_eq!(power_count, 2, "Expected 2 power ports");

        let net_labels: Vec<_> = doc
            .primitives
            .iter()
            .filter_map(|r| {
                if let SchRecord::NetLabel(nl) = r {
                    Some(nl.label.text.clone())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(net_labels.len(), 1);
        assert_eq!(net_labels[0], "SDA");

        std::fs::remove_file(&sch_path).ok();
    }

    /// E2E: Full workflow — library creation, component placement, designator lookup.
    #[test]
    #[ignore = "V2 migration: depends on v1 SchDoc I/O"]
    fn test_full_schematic_capture_workflow() {
        let lib_path = create_test_library();
        let sch_path = temp_path("SchDoc");

        // Step 1: Create schematic
        crate::ops::schdoc::cmd_create(&sch_path, None).unwrap();

        // Step 2: Place components
        cmd_add_component(
            &sch_path, &lib_path, "LDO_3PIN", "1000", "3000",
            Some("U1"), 0, None,
        ).unwrap();
        cmd_add_component(
            &sch_path, &lib_path, "IC_4PIN", "3000", "3000",
            Some("U2"), 0, None,
        ).unwrap();

        // Step 3: Add power ports
        cmd_add_power(
            &sch_path, "3V3", "2000", "4000", "bar", "up", None,
        ).unwrap();
        cmd_add_power(
            &sch_path, "GND", "2000", "2000", "ground", "down", None,
        ).unwrap();

        // Step 4: Add net labels
        cmd_add_net_label(
            &sch_path, "VIN_3V3", "500", "3000", None,
        ).unwrap();

        // Step 5: Validate — reload and verify structure
        let doc = crate::io::SchDoc::open_file(&sch_path).unwrap();

        let comp_count = doc.primitives.iter()
            .filter(|r| matches!(r, SchRecord::Component(_))).count();
        let des_count = doc.primitives.iter()
            .filter(|r| matches!(r, SchRecord::Designator(_))).count();
        let power_count = doc.primitives.iter()
            .filter(|r| matches!(r, SchRecord::PowerObject(_))).count();
        let net_count = doc.primitives.iter()
            .filter(|r| matches!(r, SchRecord::NetLabel(_))).count();

        assert_eq!(comp_count, 2, "2 components");
        assert_eq!(des_count, 2, "2 designators (one per component)");
        assert_eq!(power_count, 2, "2 power ports");
        assert_eq!(net_count, 1, "1 net label");

        // Verify no Designator records were misclassified as Parameter
        let misclassified = doc.primitives.iter()
            .filter(|r| {
                if let SchRecord::Parameter(p) = r {
                    p.name == "Designator"
                } else {
                    false
                }
            })
            .count();
        assert_eq!(misclassified, 0, "No designators should be misclassified as Parameter");

        std::fs::remove_file(&sch_path).ok();
        std::fs::remove_file(&lib_path).ok();
    }
}
