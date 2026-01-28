// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
//! SchDoc generation from import DSL types.

use std::path::Path;

use crate::edit::session::EditSession;
use crate::edit::types::Orientation;
use crate::records::sch::{
    PortIoType, PowerObjectStyle, SchNoErc, SchGraphicalBase, SchRecord,
};
use crate::types::CoordPoint;

use super::types::*;

/// Sheet size regions in mils (for A4: 11693 x 8268 mils).
/// We divide the sheet into a 3x3 grid for region-based placement.
struct SheetGrid {
    width: f64,
    height: f64,
    margin: f64,
}

impl SheetGrid {
    fn from_size(size: &str) -> Self {
        let (width, height) = match size.to_uppercase().as_str() {
            "A4" => (11693.0, 8268.0),
            "A3" => (16535.0, 11693.0),
            "A2" => (23386.0, 16535.0),
            "A1" => (33071.0, 23386.0),
            "A0" => (46811.0, 33071.0),
            "LETTER" => (11000.0, 8500.0),
            "LEGAL" => (14000.0, 8500.0),
            _ => (11693.0, 8268.0), // Default A4
        };
        SheetGrid {
            width,
            height,
            margin: 500.0,
        }
    }

    /// Get the center point for a placement region.
    fn region_center(&self, region: &PlacementRegion) -> (f64, f64) {
        let usable_w = self.width - 2.0 * self.margin;
        let usable_h = self.height - 2.0 * self.margin;
        let left = self.margin + usable_w * 0.17;
        let center_x = self.margin + usable_w * 0.50;
        let right = self.margin + usable_w * 0.83;
        let top = self.margin + usable_h * 0.83;
        let center_y = self.margin + usable_h * 0.50;
        let bottom = self.margin + usable_h * 0.17;

        match region {
            PlacementRegion::TopLeft => (left, top),
            PlacementRegion::Top => (center_x, top),
            PlacementRegion::TopRight => (right, top),
            PlacementRegion::Left => (left, center_y),
            PlacementRegion::Center => (center_x, center_y),
            PlacementRegion::Right => (right, center_y),
            PlacementRegion::BottomLeft => (left, bottom),
            PlacementRegion::Bottom => (center_x, bottom),
            PlacementRegion::BottomRight => (right, bottom),
        }
    }

    /// Get a default position for components without a region hint.
    /// Distributes them evenly across the center of the sheet.
    fn default_position(&self, index: usize, total: usize) -> (f64, f64) {
        let usable_w = self.width - 2.0 * self.margin;
        let usable_h = self.height - 2.0 * self.margin;

        // Arrange in rows
        let cols = ((total as f64).sqrt().ceil() as usize).max(1);
        let row = index / cols;
        let col = index % cols;
        let rows = (total + cols - 1) / cols;

        let x = self.margin + usable_w * (col as f64 + 0.5) / cols as f64;
        let y = self.margin + usable_h * (1.0 - (row as f64 + 0.5) / rows as f64);

        (x, y)
    }
}

/// Generate a complete SchDoc from an import definition.
///
/// The `library_path` should point to a SchLib file containing the component
/// symbols referenced by the import. If `None`, components are placed as
/// empty stubs.
pub fn generate_schdoc(
    output_path: &Path,
    import: &SchDocImport,
    library_path: Option<&Path>,
) -> Result<String, Box<dyn std::error::Error>> {
    // Step 1: Create a blank SchDoc from template
    if !output_path.exists() {
        crate::ops::schdoc::cmd_create(output_path, None)
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    }

    // Step 2: Open an edit session
    let mut session = EditSession::open(output_path)?;

    // Load the library if provided
    if let Some(lib_path) = library_path {
        session.load_library(lib_path)?;
    }

    let grid = SheetGrid::from_size(&import.sheet.size);

    // Step 3: Place components
    // Group components by region for offset calculation
    let mut region_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let no_region_count = import
        .components
        .iter()
        .filter(|c| c.region.is_none())
        .count();
    let mut no_region_idx = 0;

    for comp in &import.components {
        let (x_mils, y_mils) = if let Some(ref region) = comp.region {
            let (cx, cy) = grid.region_center(region);
            // Offset within region for multiple components
            let key = format!("{:?}", region);
            let count = region_counts.entry(key).or_insert(0);
            let offset_x = (*count as f64) * 400.0;
            let offset_y = -(*count as f64) * 200.0;
            *count += 1;
            (cx + offset_x, cy + offset_y)
        } else {
            let pos = grid.default_position(no_region_idx, no_region_count);
            no_region_idx += 1;
            pos
        };

        let location = CoordPoint::from_mils(x_mils, y_mils);

        let result = session.add_component(
            &comp.lib_reference,
            location,
            Orientation::Normal,
            Some(&comp.designator),
        );

        if let Err(e) = result {
            // If library component not found, log warning but continue
            eprintln!(
                "Warning: Could not place component {} ({}): {}",
                comp.designator, comp.lib_reference, e
            );
        }
    }

    // Step 4: Wire nets using smart-wire
    let wire_length = 200.0; // 200 mil wire stubs

    for net in &import.nets {
        let power_style = parse_power_style(net.power.as_deref());

        for conn_str in &net.connections {
            let (component, pin) = parse_pin_reference(conn_str)?;

            let result = session.smart_wire_pin(
                &component,
                &pin,
                &net.name,
                power_style,
                wire_length,
            );

            if let Err(e) = result {
                eprintln!(
                    "Warning: Could not wire {} -> net '{}': {}",
                    conn_str, net.name, e
                );
            }
        }
    }

    // Step 5: Add ports
    for port_def in &import.ports {
        let port_io = match port_def.r#type.to_lowercase().as_str() {
            "input" | "in" => PortIoType::Input,
            "output" | "out" => PortIoType::Output,
            "bidirectional" | "bidir" | "inout" => PortIoType::Bidirectional,
            _ => PortIoType::Unspecified,
        };

        // Place ports near the sheet edges
        let x = match port_io {
            PortIoType::Input => grid.margin + 200.0,
            PortIoType::Output => grid.width - grid.margin - 200.0,
            _ => grid.margin + 200.0,
        };
        let y = grid.height / 2.0;

        let location = CoordPoint::from_mils(x, y);
        if let Err(e) = session.add_port(&port_def.name, location, port_io) {
            eprintln!("Warning: Could not add port '{}': {}", port_def.name, e);
        }
    }

    // Step 6: Add No-ERC markers
    for no_erc_ref in &import.no_erc {
        let (component, pin) = parse_pin_reference(no_erc_ref)?;

        // Find the pin location to place the No-ERC marker
        let components = session.layout().get_placed_components(&session.doc.primitives);
        if let Some(comp) = components.iter().find(|c| c.designator == component) {
            if let Some(pin_loc) = comp
                .pin_locations
                .iter()
                .find(|p| p.designator == pin || p.name == pin)
            {
                let mut graphical = SchGraphicalBase::new_graphical();
                graphical.location_x = pin_loc.location.x.to_raw();
                graphical.location_y = pin_loc.location.y.to_raw();
                graphical.base.owner_index = -1;

                let mut no_erc = SchNoErc::default();
                no_erc.graphical = graphical;
                session.doc.primitives.push(SchRecord::NoErc(no_erc));
            } else {
                eprintln!(
                    "Warning: Pin {} not found on component {} for No-ERC marker",
                    pin, component
                );
            }
        } else {
            eprintln!(
                "Warning: Component {} not found for No-ERC marker",
                component
            );
        }
    }

    // Step 7: Add missing junctions
    let junction_count = session.add_missing_junctions()?;

    // Step 8: Save
    session.save(output_path)?;

    Ok(format!(
        "Generated SchDoc with {} components, {} nets, {} ports, {} junctions -> {}",
        import.components.len(),
        import.nets.len(),
        import.ports.len(),
        junction_count,
        output_path.display()
    ))
}

/// Parse a pin reference string.
///
/// Supports two forms:
/// - `"U1:VCC"` — by pin name (colon separator)
/// - `"U1.8"` — by pin designator (dot separator)
///
/// Returns `(component_designator, pin_identifier)`.
fn parse_pin_reference(s: &str) -> Result<(String, String), Box<dyn std::error::Error>> {
    // Try colon separator first (by name)
    if let Some((comp, pin)) = s.split_once(':') {
        return Ok((comp.trim().to_string(), pin.trim().to_string()));
    }
    // Try dot separator (by designator number)
    if let Some((comp, pin)) = s.split_once('.') {
        return Ok((comp.trim().to_string(), pin.trim().to_string()));
    }
    Err(format!(
        "Invalid pin reference '{}'. Use 'U1:VCC' (by name) or 'U1.8' (by designator)",
        s
    )
    .into())
}

/// Parse a power style string to PowerObjectStyle.
fn parse_power_style(s: Option<&str>) -> Option<PowerObjectStyle> {
    s.map(|style| match style.to_lowercase().as_str() {
        "bar" | "power_bar" => PowerObjectStyle::Bar,
        "arrow" => PowerObjectStyle::Arrow,
        "wave" => PowerObjectStyle::Wave,
        "ground" | "gnd" => PowerObjectStyle::Ground,
        "power_ground" | "pgnd" => PowerObjectStyle::PowerGround,
        "signal_ground" | "sgnd" => PowerObjectStyle::SignalGround,
        "earth_ground" | "earth" => PowerObjectStyle::EarthGround,
        "circle" => PowerObjectStyle::Circle,
        _ => PowerObjectStyle::Bar, // Default to bar
    })
}
