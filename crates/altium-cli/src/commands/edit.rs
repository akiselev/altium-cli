// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! Edit command implementation.
//!
//! Provides programmatic editing operations for Altium schematic documents.
//! Supports adding, moving, and deleting components, wires, and other primitives.
//!
//! Note: This module uses V1 types (PortIoType, PowerObjectStyle, TextOrientations, SchRecord)
//! through EditSession, which are deprecated. The edit session internally handles V1 types
//! until the full V2 migration is complete.

#![allow(deprecated)]

use std::path::Path;

use serde::Serialize;

use crate::output::{self, TextFormat};
use altium_format::edit::types::Orientation;
use altium_format::edit::EditSession;
use altium_format::records::sch::{PortIoType, PowerObjectStyle, TextOrientations};
use altium_format::types::{Coord, CoordPoint, Unit};

/// Result of an edit operation.
#[derive(Serialize)]
pub struct EditResult {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Path to the edited file.
    pub file: String,
    /// The edit operation that was performed.
    pub operation: String,
    /// Description of what was changed.
    pub description: String,
    /// Output path where the file was saved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_path: Option<String>,
    /// Error message if the operation failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl TextFormat for EditResult {
    fn format_text(&self) -> String {
        let mut output = String::new();

        if self.success {
            output.push_str("✓ Success\n");
        } else {
            output.push_str("✗ Failed\n");
        }

        output.push_str(&format!("Operation: {}\n", self.operation));
        output.push_str(&format!("File: {}\n", self.file));
        output.push_str(&format!("{}\n", self.description));

        if let Some(out_path) = &self.output_path {
            output.push_str(&format!("Saved to: {}\n", out_path));
        }

        if let Some(err) = &self.error {
            output.push_str(&format!("Error: {}\n", err));
        }

        output
    }
}

/// Edit subcommand specification.
#[derive(Debug, Clone)]
pub enum EditOperation {
    /// Move a component to a new location.
    MoveComponent { designator: String, x: f64, y: f64 },
    /// Delete a component by designator.
    DeleteComponent { designator: String },
    /// Add a wire with given vertices (x1,y1,x2,y2,...).
    AddWire { vertices: Vec<f64> },
    /// Delete a wire by index.
    DeleteWire { index: usize },
    /// Add a net label at a location.
    AddNetLabel { name: String, x: f64, y: f64 },
    /// Add a power port.
    AddPower {
        name: String,
        x: f64,
        y: f64,
        style: String,
        orientation: String,
    },
    /// Add a junction.
    AddJunction { x: f64, y: f64 },
    /// Add missing junctions where wires cross.
    AddMissingJunctions,
    /// Add a port.
    AddPort {
        name: String,
        x: f64,
        y: f64,
        io_type: String,
    },
    /// Route a wire between two points.
    RouteWire { from: String, to: String },
    /// Validate the schematic.
    Validate,
    /// Add component from library.
    AddComponent {
        library: String,
        component: String,
        x: f64,
        y: f64,
        designator: Option<String>,
    },
}

/// Run the edit command.
pub fn run(
    path: &Path,
    operation: &str,
    format: &str,
    output_file: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let op = parse_operation(operation)?;

    let result = execute_operation(path, op, output_file)?;

    output::print(&result, format)?;

    if !result.success {
        std::process::exit(1);
    }

    Ok(())
}

/// Parse an operation string into an EditOperation.
///
/// Supported formats:
/// - `move <designator> <x> <y>` - Move component to (x, y) mils
/// - `delete <designator>` - Delete component
/// - `add-wire <x1>,<y1>,<x2>,<y2>,...` - Add wire with vertices
/// - `delete-wire <index>` - Delete wire by index
/// - `add-net-label <name> <x> <y>` - Add net label
/// - `add-power <name> <x> <y> <style> <orientation>` - Add power port
/// - `add-junction <x> <y>` - Add junction
/// - `add-missing-junctions` - Add junctions where wires cross
/// - `add-port <name> <x> <y> <io_type>` - Add port
/// - `route <from> <to>` - Route wire between points
/// - `validate` - Validate the schematic
fn parse_operation(operation: &str) -> Result<EditOperation, Box<dyn std::error::Error>> {
    let parts: Vec<&str> = operation.split_whitespace().collect();
    if parts.is_empty() {
        return Err("Empty operation".into());
    }

    match parts[0].to_lowercase().as_str() {
        "move" => {
            if parts.len() < 4 {
                return Err("move requires: <designator> <x> <y>".into());
            }
            let x = parse_coordinate(parts[2])?;
            let y = parse_coordinate(parts[3])?;
            Ok(EditOperation::MoveComponent {
                designator: parts[1].to_string(),
                x,
                y,
            })
        }
        "delete" => {
            if parts.len() < 2 {
                return Err("delete requires: <designator>".into());
            }
            Ok(EditOperation::DeleteComponent {
                designator: parts[1].to_string(),
            })
        }
        "add-wire" => {
            if parts.len() < 2 {
                return Err("add-wire requires: <x1>,<y1>,<x2>,<y2>,...".into());
            }
            let vertices = parse_vertex_list(parts[1])?;
            if vertices.len() < 4 || vertices.len() % 2 != 0 {
                return Err("add-wire requires at least 2 coordinate pairs (4 values)".into());
            }
            Ok(EditOperation::AddWire { vertices })
        }
        "delete-wire" => {
            if parts.len() < 2 {
                return Err("delete-wire requires: <index>".into());
            }
            let index: usize = parts[1]
                .parse()
                .map_err(|_| format!("Invalid wire index: {}", parts[1]))?;
            Ok(EditOperation::DeleteWire { index })
        }
        "add-net-label" => {
            if parts.len() < 4 {
                return Err("add-net-label requires: <name> <x> <y>".into());
            }
            let x = parse_coordinate(parts[2])?;
            let y = parse_coordinate(parts[3])?;
            Ok(EditOperation::AddNetLabel {
                name: parts[1].to_string(),
                x,
                y,
            })
        }
        "add-power" => {
            if parts.len() < 6 {
                return Err("add-power requires: <name> <x> <y> <style> <orientation>".into());
            }
            let x = parse_coordinate(parts[2])?;
            let y = parse_coordinate(parts[3])?;
            Ok(EditOperation::AddPower {
                name: parts[1].to_string(),
                x,
                y,
                style: parts[4].to_string(),
                orientation: parts[5].to_string(),
            })
        }
        "add-junction" => {
            if parts.len() < 3 {
                return Err("add-junction requires: <x> <y>".into());
            }
            let x = parse_coordinate(parts[1])?;
            let y = parse_coordinate(parts[2])?;
            Ok(EditOperation::AddJunction { x, y })
        }
        "add-missing-junctions" => Ok(EditOperation::AddMissingJunctions),
        "add-port" => {
            if parts.len() < 5 {
                return Err("add-port requires: <name> <x> <y> <io_type>".into());
            }
            let x = parse_coordinate(parts[2])?;
            let y = parse_coordinate(parts[3])?;
            Ok(EditOperation::AddPort {
                name: parts[1].to_string(),
                x,
                y,
                io_type: parts[4].to_string(),
            })
        }
        "route" => {
            if parts.len() < 3 {
                return Err("route requires: <from> <to>".into());
            }
            Ok(EditOperation::RouteWire {
                from: parts[1].to_string(),
                to: parts[2].to_string(),
            })
        }
        "validate" => Ok(EditOperation::Validate),
        "add-component" => {
            if parts.len() < 5 {
                return Err(
                    "add-component requires: <library> <component> <x> <y> [designator]".into(),
                );
            }
            let x = parse_coordinate(parts[3])?;
            let y = parse_coordinate(parts[4])?;
            let designator = if parts.len() >= 6 {
                Some(parts[5].to_string())
            } else {
                None
            };
            Ok(EditOperation::AddComponent {
                library: parts[1].to_string(),
                component: parts[2].to_string(),
                x,
                y,
                designator,
            })
        }
        _ => Err(format!("Unknown operation: {}", parts[0]).into()),
    }
}

/// Parse a coordinate value with optional unit suffix.
fn parse_coordinate(s: &str) -> Result<f64, Box<dyn std::error::Error>> {
    let s = s.trim();

    if let Ok((coord, unit)) = Unit::parse_with_unit(s) {
        if unit != Unit::DxpDefault {
            return Ok(coord.to_mils());
        }
    }

    s.parse::<f64>()
        .map_err(|_| {
            format!(
                "Invalid coordinate '{}': expected number with optional unit (e.g., '100mil', '2.54mm')",
                s
            )
            .into()
        })
}

/// Parse a comma-separated list of vertex coordinates.
fn parse_vertex_list(s: &str) -> Result<Vec<f64>, Box<dyn std::error::Error>> {
    s.split(',')
        .map(|v| {
            v.trim()
                .parse::<f64>()
                .map_err(|_| format!("Invalid vertex coordinate: {}", v))
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.into())
}

/// Execute an edit operation on the file.
fn execute_operation(
    path: &Path,
    operation: EditOperation,
    output_file: Option<&Path>,
) -> Result<EditResult, Box<dyn std::error::Error>> {
    let output_path = output_file.unwrap_or(path);

    match operation {
        EditOperation::MoveComponent { designator, x, y } => {
            execute_move_component(path, &designator, x, y, output_path)
        }
        EditOperation::DeleteComponent { designator } => {
            execute_delete_component(path, &designator, output_path)
        }
        EditOperation::AddWire { vertices } => execute_add_wire(path, &vertices, output_path),
        EditOperation::DeleteWire { index } => execute_delete_wire(path, index, output_path),
        EditOperation::AddNetLabel { name, x, y } => {
            execute_add_net_label(path, &name, x, y, output_path)
        }
        EditOperation::AddPower {
            name,
            x,
            y,
            style,
            orientation,
        } => execute_add_power(path, &name, x, y, &style, &orientation, output_path),
        EditOperation::AddJunction { x, y } => execute_add_junction(path, x, y, output_path),
        EditOperation::AddMissingJunctions => execute_add_missing_junctions(path, output_path),
        EditOperation::AddPort {
            name,
            x,
            y,
            io_type,
        } => execute_add_port(path, &name, x, y, &io_type, output_path),
        EditOperation::RouteWire { from, to } => execute_route_wire(path, &from, &to, output_path),
        EditOperation::Validate => execute_validate(path),
        EditOperation::AddComponent {
            library,
            component,
            x,
            y,
            designator,
        } => execute_add_component(
            path,
            &library,
            &component,
            x,
            y,
            designator.as_deref(),
            output_path,
        ),
    }
}

fn execute_move_component(
    path: &Path,
    designator: &str,
    x: f64,
    y: f64,
    output_path: &Path,
) -> Result<EditResult, Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;

    let components = session
        .layout()
        .get_placed_components(&session.doc.primitives);
    let comp = components
        .iter()
        .find(|c| c.designator == designator)
        .ok_or_else(|| format!("Component not found: {}", designator))?;

    let index = comp.index;
    let new_location = CoordPoint::from_mils(x, y);

    session.move_component(index, new_location)?;
    session.save(output_path)?;

    Ok(EditResult {
        success: true,
        file: path.display().to_string(),
        operation: "move".to_string(),
        description: format!("Moved {} to ({:.0}, {:.0}) mils", designator, x, y),
        output_path: Some(output_path.display().to_string()),
        error: None,
    })
}

fn execute_delete_component(
    path: &Path,
    designator: &str,
    output_path: &Path,
) -> Result<EditResult, Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;

    let components = session
        .layout()
        .get_placed_components(&session.doc.primitives);
    let comp = components
        .iter()
        .find(|c| c.designator == designator)
        .ok_or_else(|| format!("Component not found: {}", designator))?;

    let index = comp.index;
    session.delete_component(index)?;
    session.save(output_path)?;

    Ok(EditResult {
        success: true,
        file: path.display().to_string(),
        operation: "delete".to_string(),
        description: format!("Deleted component {}", designator),
        output_path: Some(output_path.display().to_string()),
        error: None,
    })
}

fn execute_add_wire(
    path: &Path,
    vertices: &[f64],
    output_path: &Path,
) -> Result<EditResult, Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;

    let points: Vec<CoordPoint> = vertices
        .chunks(2)
        .map(|chunk| CoordPoint::from_mils(chunk[0], chunk[1]))
        .collect();

    session.add_wire(&points)?;
    session.save(output_path)?;

    Ok(EditResult {
        success: true,
        file: path.display().to_string(),
        operation: "add-wire".to_string(),
        description: format!("Added wire with {} vertices", points.len()),
        output_path: Some(output_path.display().to_string()),
        error: None,
    })
}

fn execute_delete_wire(
    path: &Path,
    index: usize,
    output_path: &Path,
) -> Result<EditResult, Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;
    session.delete_wire(index)?;
    session.save(output_path)?;

    Ok(EditResult {
        success: true,
        file: path.display().to_string(),
        operation: "delete-wire".to_string(),
        description: format!("Deleted wire at index {}", index),
        output_path: Some(output_path.display().to_string()),
        error: None,
    })
}

fn execute_add_net_label(
    path: &Path,
    name: &str,
    x: f64,
    y: f64,
    output_path: &Path,
) -> Result<EditResult, Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;
    let location = CoordPoint::from_mils(x, y);
    session.add_net_label(name, location)?;
    session.save(output_path)?;

    Ok(EditResult {
        success: true,
        file: path.display().to_string(),
        operation: "add-net-label".to_string(),
        description: format!("Added net label '{}' at ({:.0}, {:.0}) mils", name, x, y),
        output_path: Some(output_path.display().to_string()),
        error: None,
    })
}

fn execute_add_power(
    path: &Path,
    name: &str,
    x: f64,
    y: f64,
    style: &str,
    orientation: &str,
    output_path: &Path,
) -> Result<EditResult, Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;
    let location = CoordPoint::from_mils(x, y);

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
    session.save(output_path)?;

    Ok(EditResult {
        success: true,
        file: path.display().to_string(),
        operation: "add-power".to_string(),
        description: format!(
            "Added power port '{}' ({}) at ({:.0}, {:.0}) mils",
            name, style, x, y
        ),
        output_path: Some(output_path.display().to_string()),
        error: None,
    })
}

fn execute_add_junction(
    path: &Path,
    x: f64,
    y: f64,
    output_path: &Path,
) -> Result<EditResult, Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;
    let location = CoordPoint::from_mils(x, y);
    session.add_junction(location)?;
    session.save(output_path)?;

    Ok(EditResult {
        success: true,
        file: path.display().to_string(),
        operation: "add-junction".to_string(),
        description: format!("Added junction at ({:.0}, {:.0}) mils", x, y),
        output_path: Some(output_path.display().to_string()),
        error: None,
    })
}

fn execute_add_missing_junctions(
    path: &Path,
    output_path: &Path,
) -> Result<EditResult, Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;
    let count = session.add_missing_junctions()?;
    session.save(output_path)?;

    Ok(EditResult {
        success: true,
        file: path.display().to_string(),
        operation: "add-missing-junctions".to_string(),
        description: format!("Added {} missing junction(s)", count),
        output_path: Some(output_path.display().to_string()),
        error: None,
    })
}

fn execute_add_port(
    path: &Path,
    name: &str,
    x: f64,
    y: f64,
    io_type: &str,
    output_path: &Path,
) -> Result<EditResult, Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;
    let location = CoordPoint::from_mils(x, y);

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
    session.save(output_path)?;

    Ok(EditResult {
        success: true,
        file: path.display().to_string(),
        operation: "add-port".to_string(),
        description: format!(
            "Added port '{}' ({}) at ({:.0}, {:.0}) mils",
            name, io_type, x, y
        ),
        output_path: Some(output_path.display().to_string()),
        error: None,
    })
}

fn execute_route_wire(
    path: &Path,
    from: &str,
    to: &str,
    output_path: &Path,
) -> Result<EditResult, Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;

    let start = parse_point_spec(from, &session)?;
    let end = parse_point_spec(to, &session)?;

    session.route_wire(start, end)?;
    session.save(output_path)?;

    Ok(EditResult {
        success: true,
        file: path.display().to_string(),
        operation: "route".to_string(),
        description: format!("Routed wire from {} to {}", from, to),
        output_path: Some(output_path.display().to_string()),
        error: None,
    })
}

fn execute_validate(path: &Path) -> Result<EditResult, Box<dyn std::error::Error>> {
    let session = EditSession::open(path)?;
    let errors = session.validate();

    if errors.is_empty() {
        Ok(EditResult {
            success: true,
            file: path.display().to_string(),
            operation: "validate".to_string(),
            description: "Schematic is valid".to_string(),
            output_path: None,
            error: None,
        })
    } else {
        let error_descriptions: Vec<String> = errors
            .iter()
            .map(|e| format!("{:?}: {}", e.kind, e.message))
            .collect();

        Ok(EditResult {
            success: false,
            file: path.display().to_string(),
            operation: "validate".to_string(),
            description: format!("Found {} validation error(s)", errors.len()),
            output_path: None,
            error: Some(error_descriptions.join("; ")),
        })
    }
}

fn execute_add_component(
    path: &Path,
    library: &str,
    component: &str,
    x: f64,
    y: f64,
    designator: Option<&str>,
    output_path: &Path,
) -> Result<EditResult, Box<dyn std::error::Error>> {
    let mut session = EditSession::open(path)?;
    let location = CoordPoint::from_mils(x, y);

    // Load the library
    session.load_library(library)?;

    // Add the component
    session.add_component(component, location, Orientation::Normal, designator)?;
    session.save(output_path)?;

    let des_desc = designator
        .map(|d| format!(" as {}", d))
        .unwrap_or_default();

    Ok(EditResult {
        success: true,
        file: path.display().to_string(),
        operation: "add-component".to_string(),
        description: format!(
            "Added component '{}' from '{}'{}at ({:.0}, {:.0}) mils",
            component, library, des_desc, x, y
        ),
        output_path: Some(output_path.display().to_string()),
        error: None,
    })
}

/// Parse a point specification for routing targets.
///
/// Supported formats:
/// - `x,y` - Coordinate pair in mils
/// - `Component.Pin` - Pin location (e.g., `U1.VCC`)
/// - `:PortName` - Port location
/// - `%NetLabel` - Net label location
/// - `@PowerPort` - Power port location
fn parse_point_spec(
    spec: &str,
    session: &EditSession,
) -> Result<CoordPoint, Box<dyn std::error::Error>> {
    use altium_format::records::sch::SchRecord;

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
        return Ok(CoordPoint::from_mils(x, y));
    }

    if let Some(port_name) = spec.strip_prefix(':') {
        for record in &session.doc.primitives {
            if let SchRecord::Port(port) = record {
                if port.name == port_name {
                    return Ok(CoordPoint::new(
                        Coord::from_raw(port.graphical.location_x),
                        Coord::from_raw(port.graphical.location_y),
                    ));
                }
            }
        }
        return Err(format!("Port not found: '{}'", port_name).into());
    }

    if let Some(label_name) = spec.strip_prefix('%') {
        for record in &session.doc.primitives {
            if let SchRecord::NetLabel(label) = record {
                if label.label.text == label_name {
                    return Ok(CoordPoint::new(
                        Coord::from_raw(label.label.graphical.location_x),
                        Coord::from_raw(label.label.graphical.location_y),
                    ));
                }
            }
        }
        return Err(format!("Net label not found: '{}'", label_name).into());
    }

    if let Some(power_name) = spec.strip_prefix('@') {
        for record in &session.doc.primitives {
            if let SchRecord::PowerObject(power) = record {
                if power.text == power_name {
                    return Ok(CoordPoint::new(
                        Coord::from_raw(power.graphical.location_x),
                        Coord::from_raw(power.graphical.location_y),
                    ));
                }
            }
        }
        return Err(format!("Power port not found: '{}'", power_name).into());
    }

    if let Some((component, pin)) = spec.split_once('.') {
        let components = session
            .layout()
            .get_placed_components(&session.doc.primitives);
        let comp = components
            .iter()
            .find(|c| c.designator == component)
            .ok_or_else(|| {
                format!(
                    "Component not found: '{}'. Available: {:?}",
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
                    "Pin '{}' not found on component '{}'. Available: {:?}",
                    pin,
                    component,
                    comp.pin_locations
                        .iter()
                        .map(|p| format!("{}/{}", p.designator, p.name))
                        .collect::<Vec<_>>()
                )
            })?;

        return Ok(pin_loc.location);
    }

    Err(format!(
        "Invalid point specification: '{}'. Expected: Component.Pin, :Port, %NetLabel, @Power, or x,y",
        spec
    )
    .into())
}
