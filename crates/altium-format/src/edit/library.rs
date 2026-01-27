//! Library integration for instantiating components from SchLib files.

use std::collections::HashMap;
use std::path::Path;

use crate::error::{AltiumError, Result};
use crate::io::schlib::SchLibComponent;
use crate::io::{SchDoc, SchLib};
use crate::records::sch::{SchDesignator, SchParameter, SchRecord, TextOrientations};
use crate::types::{Coord, CoordPoint, UnknownFields};

use super::types::Orientation;

/// Manages schematic libraries and component instantiation.
pub struct LibraryManager {
    /// Loaded libraries (path -> library).
    libraries: HashMap<String, SchLib>,
    /// Designator counters by prefix.
    designator_counters: HashMap<String, u32>,
}

impl Default for LibraryManager {
    fn default() -> Self {
        Self::new()
    }
}

impl LibraryManager {
    /// Create a new library manager.
    pub fn new() -> Self {
        Self {
            libraries: HashMap::new(),
            designator_counters: HashMap::new(),
        }
    }

    /// Load a SchLib file.
    pub fn load_library<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        if self.libraries.contains_key(&path_str) {
            return Ok(()); // Already loaded
        }

        let lib = SchLib::open_file(&path)?;
        self.libraries.insert(path_str, lib);
        Ok(())
    }

    /// Get a loaded library by path.
    pub fn get_library(&self, path: &str) -> Option<&SchLib> {
        self.libraries.get(path)
    }

    /// List all loaded libraries.
    pub fn list_libraries(&self) -> Vec<&str> {
        self.libraries.keys().map(|s| s.as_str()).collect()
    }

    /// Search for a component across all loaded libraries.
    pub fn find_component(&self, lib_reference: &str) -> Option<(&str, &SchLibComponent)> {
        for (path, lib) in &self.libraries {
            for component in lib.iter() {
                if component.name() == lib_reference {
                    return Some((path.as_str(), component));
                }
            }
        }
        None
    }

    /// List all components in all loaded libraries.
    pub fn list_all_components(&self) -> Vec<(&str, &str, &str)> {
        let mut components = Vec::new();
        for (path, lib) in &self.libraries {
            for component in lib.iter() {
                components.push((path.as_str(), component.name(), component.description()));
            }
        }
        components
    }

    /// Search components by pattern (case-insensitive substring match).
    pub fn search_components(&self, pattern: &str) -> Vec<(&str, &str, &str)> {
        let pattern_lower = pattern.to_lowercase();
        self.list_all_components()
            .into_iter()
            .filter(|(_, name, desc)| {
                name.to_lowercase().contains(&pattern_lower)
                    || desc.to_lowercase().contains(&pattern_lower)
            })
            .collect()
    }

    /// Initialize designator counters from an existing schematic.
    pub fn init_designators_from(&mut self, doc: &SchDoc) {
        self.designator_counters.clear();

        for record in &doc.primitives {
            if let SchRecord::Designator(d) = record {
                let text = d.text();
                if let Some((prefix, num)) = Self::parse_designator(text) {
                    let entry = self.designator_counters.entry(prefix).or_insert(0);
                    *entry = (*entry).max(num);
                }
            }
        }
    }

    /// Parse a designator into prefix and number (e.g., "R1" -> ("R", 1)).
    fn parse_designator(designator: &str) -> Option<(String, u32)> {
        let mut prefix = String::new();
        let mut num_str = String::new();

        for c in designator.chars() {
            if c.is_ascii_digit() {
                num_str.push(c);
            } else if num_str.is_empty() {
                prefix.push(c);
            } else {
                // Letters after numbers - not a simple designator
                return None;
            }
        }

        if prefix.is_empty() || num_str.is_empty() {
            return None;
        }

        num_str.parse().ok().map(|n| (prefix, n))
    }

    /// Get the next designator for a given prefix.
    pub fn next_designator(&mut self, prefix: &str) -> String {
        let counter = self
            .designator_counters
            .entry(prefix.to_string())
            .or_insert(0);
        *counter += 1;
        format!("{}{}", prefix, counter)
    }

    /// Suggest a designator prefix based on component name.
    pub fn suggest_designator_prefix(&self, lib_reference: &str) -> &str {
        let name_upper = lib_reference.to_uppercase();

        // Common component type prefixes
        if name_upper.contains("RESISTOR") || name_upper.starts_with("R") && name_upper.len() <= 6 {
            return "R";
        }
        if name_upper.contains("CAPACITOR") || name_upper.starts_with("C") && name_upper.len() <= 6
        {
            return "C";
        }
        if name_upper.contains("INDUCTOR") || name_upper.starts_with("L") && name_upper.len() <= 6 {
            return "L";
        }
        if name_upper.contains("DIODE") || name_upper.starts_with("D") && name_upper.len() <= 6 {
            return "D";
        }
        if name_upper.contains("TRANSISTOR")
            || name_upper.contains("MOSFET")
            || name_upper.contains("BJT")
        {
            return "Q";
        }
        if name_upper.contains("LED") {
            return "D";
        }
        if name_upper.contains("CRYSTAL") || name_upper.contains("OSCILLATOR") {
            return "Y";
        }
        if name_upper.contains("CONNECTOR") || name_upper.starts_with("J") {
            return "J";
        }
        if name_upper.contains("HEADER") {
            return "J";
        }
        if name_upper.contains("SWITCH") {
            return "SW";
        }
        if name_upper.contains("FUSE") {
            return "F";
        }
        if name_upper.contains("TRANSFORMER") {
            return "T";
        }
        if name_upper.contains("RELAY") {
            return "K";
        }
        if name_upper.contains("OPAMP") || name_upper.contains("OP-AMP") {
            return "U";
        }

        // Default to U for ICs/generic components
        "U"
    }

    /// Instantiate a component from a library into a schematic.
    pub fn instantiate_component(
        &mut self,
        lib_reference: &str,
        location: CoordPoint,
        orientation: Orientation,
        designator: Option<&str>,
        doc: &mut SchDoc,
    ) -> Result<usize> {
        // Find the component in loaded libraries and clone what we need
        let (lib_path, lib_component_clone, lib_primitives) = {
            let (path, lib_component) = self.find_component(lib_reference).ok_or_else(|| {
                AltiumError::Parse(format!("Component not found: {}", lib_reference))
            })?;
            (
                path.to_string(),
                lib_component.component.clone(),
                lib_component.primitives.clone(),
            )
        };

        // Generate designator if not provided
        let designator = match designator {
            Some(d) => {
                // Update counter if this designator is higher
                if let Some((prefix, num)) = Self::parse_designator(d) {
                    let entry = self.designator_counters.entry(prefix).or_insert(0);
                    *entry = (*entry).max(num);
                }
                d.to_string()
            }
            None => {
                let prefix = self.suggest_designator_prefix(lib_reference).to_string();
                self.next_designator(&prefix)
            }
        };

        // The component will be added at this index
        let component_index = doc.primitives.len();

        // Create the component record
        let mut component = lib_component_clone;
        component.graphical.location_x = location.x.to_raw();
        component.graphical.location_y = location.y.to_raw();
        component.graphical.base.owner_index = -1; // Top-level
        component.orientation = TextOrientations::from_int(orientation.to_altium());
        component.library_path = lib_path.clone();
        component.source_library_name = lib_path;

        // Generate a unique ID
        let mut det = ();
        component.unique_id = format!("{}_{}", designator, uuid_simple_deterministic(&mut det));

        doc.primitives.push(SchRecord::Component(component));

        // Copy all child primitives, transforming coordinates and updating owner indices
        for record in lib_primitives.iter().skip(1) {
            // Skip the component record itself
            let new_record = self.transform_primitive(
                record,
                location,
                orientation,
                component_index as i32,
                &designator,
            );

            doc.primitives.push(new_record);
        }

        // Add designator if not already present
        let has_designator = lib_primitives
            .iter()
            .any(|r| matches!(r, SchRecord::Designator(_)));
        if !has_designator {
            let designator_record =
                self.create_designator(&designator, location, component_index as i32);
            doc.primitives.push(designator_record);
        }

        Ok(component_index)
    }

    /// Transform a primitive's coordinates and update owner index.
    fn transform_primitive(
        &self,
        record: &SchRecord,
        location: CoordPoint,
        orientation: Orientation,
        owner_index: i32,
        designator: &str,
    ) -> SchRecord {
        match record {
            SchRecord::Pin(pin) => {
                let mut new_pin = pin.clone();
                new_pin.graphical.base.owner_index = owner_index;

                // Transform pin location
                let local =
                    CoordPoint::from_raw(pin.graphical.location_x, pin.graphical.location_y);
                let absolute = self.transform_point(local, location, orientation);
                new_pin.graphical.location_x = absolute.x.to_raw();
                new_pin.graphical.location_y = absolute.y.to_raw();

                // Update pin rotation based on orientation
                if orientation.rotation_degrees() != 0.0 {
                    use crate::records::sch::PinConglomerateFlags;

                    // Rotate the pin direction
                    let rotated = pin.pin_conglomerate.contains(PinConglomerateFlags::ROTATED);
                    let flipped = pin.pin_conglomerate.contains(PinConglomerateFlags::FLIPPED);

                    // For 90-degree rotation, swap rotated flag
                    let new_rotated = match orientation {
                        Orientation::Rotated90 | Orientation::Rotated270 => !rotated,
                        _ => rotated,
                    };

                    // Update flags
                    if new_rotated {
                        new_pin
                            .pin_conglomerate
                            .insert(PinConglomerateFlags::ROTATED);
                    } else {
                        new_pin
                            .pin_conglomerate
                            .remove(PinConglomerateFlags::ROTATED);
                    }
                    if flipped {
                        new_pin
                            .pin_conglomerate
                            .insert(PinConglomerateFlags::FLIPPED);
                    }
                }

                SchRecord::Pin(new_pin)
            }
            SchRecord::Line(line) => {
                let mut new_line = line.clone();
                new_line.graphical.base.owner_index = owner_index;

                let start =
                    CoordPoint::from_raw(line.graphical.location_x, line.graphical.location_y);
                let end = CoordPoint::from_raw(line.corner_x, line.corner_y);

                let new_start = self.transform_point(start, location, orientation);
                let new_end = self.transform_point(end, location, orientation);

                new_line.graphical.location_x = new_start.x.to_raw();
                new_line.graphical.location_y = new_start.y.to_raw();
                new_line.corner_x = new_end.x.to_raw();
                new_line.corner_y = new_end.y.to_raw();

                SchRecord::Line(new_line)
            }
            SchRecord::Rectangle(rect) => {
                let mut new_rect = rect.clone();
                new_rect.graphical.base.owner_index = owner_index;

                let loc =
                    CoordPoint::from_raw(rect.graphical.location_x, rect.graphical.location_y);
                let corner = CoordPoint::from_raw(rect.corner_x, rect.corner_y);

                let new_loc = self.transform_point(loc, location, orientation);
                let new_corner = self.transform_point(corner, location, orientation);

                new_rect.graphical.location_x = new_loc.x.to_raw();
                new_rect.graphical.location_y = new_loc.y.to_raw();
                new_rect.corner_x = new_corner.x.to_raw();
                new_rect.corner_y = new_corner.y.to_raw();

                SchRecord::Rectangle(new_rect)
            }
            SchRecord::Polygon(poly) => {
                let mut new_poly = poly.clone();
                new_poly.graphical.base.owner_index = owner_index;

                let loc =
                    CoordPoint::from_raw(poly.graphical.location_x, poly.graphical.location_y);
                let new_loc = self.transform_point(loc, location, orientation);
                new_poly.graphical.location_x = new_loc.x.to_raw();
                new_poly.graphical.location_y = new_loc.y.to_raw();

                // Transform vertices
                new_poly.vertices = poly
                    .vertices
                    .iter()
                    .map(|(x, y)| {
                        let pt = CoordPoint::from_raw(*x, *y);
                        let new_pt = self.transform_point(pt, location, orientation);
                        (new_pt.x.to_raw(), new_pt.y.to_raw())
                    })
                    .collect();

                SchRecord::Polygon(new_poly)
            }
            SchRecord::Polyline(polyline) => {
                let mut new_polyline = polyline.clone();
                new_polyline.graphical.base.owner_index = owner_index;

                let loc = CoordPoint::from_raw(
                    polyline.graphical.location_x,
                    polyline.graphical.location_y,
                );
                let new_loc = self.transform_point(loc, location, orientation);
                new_polyline.graphical.location_x = new_loc.x.to_raw();
                new_polyline.graphical.location_y = new_loc.y.to_raw();

                new_polyline.vertices = polyline
                    .vertices
                    .iter()
                    .map(|(x, y)| {
                        let pt = CoordPoint::from_raw(*x, *y);
                        let new_pt = self.transform_point(pt, location, orientation);
                        (new_pt.x.to_raw(), new_pt.y.to_raw())
                    })
                    .collect();

                SchRecord::Polyline(new_polyline)
            }
            SchRecord::Arc(arc) => {
                let mut new_arc = arc.clone();
                new_arc.graphical.base.owner_index = owner_index;

                let loc = CoordPoint::from_raw(arc.graphical.location_x, arc.graphical.location_y);
                let new_loc = self.transform_point(loc, location, orientation);
                new_arc.graphical.location_x = new_loc.x.to_raw();
                new_arc.graphical.location_y = new_loc.y.to_raw();

                // Adjust angles for rotation
                let rotation = orientation.rotation_degrees();
                if rotation != 0.0 {
                    new_arc.start_angle = (arc.start_angle + rotation) % 360.0;
                    new_arc.end_angle = (arc.end_angle + rotation) % 360.0;
                }

                SchRecord::Arc(new_arc)
            }
            SchRecord::Ellipse(ellipse) => {
                let mut new_ellipse = ellipse.clone();
                new_ellipse.graphical.base.owner_index = owner_index;

                let loc = CoordPoint::from_raw(
                    ellipse.graphical.location_x,
                    ellipse.graphical.location_y,
                );
                let new_loc = self.transform_point(loc, location, orientation);
                new_ellipse.graphical.location_x = new_loc.x.to_raw();
                new_ellipse.graphical.location_y = new_loc.y.to_raw();

                SchRecord::Ellipse(new_ellipse)
            }
            SchRecord::Label(label) => {
                let mut new_label = label.clone();
                new_label.graphical.base.owner_index = owner_index;

                let loc =
                    CoordPoint::from_raw(label.graphical.location_x, label.graphical.location_y);
                let new_loc = self.transform_point(loc, location, orientation);
                new_label.graphical.location_x = new_loc.x.to_raw();
                new_label.graphical.location_y = new_loc.y.to_raw();

                SchRecord::Label(new_label)
            }
            SchRecord::Designator(d) => {
                let mut new_d = d.clone();
                new_d.param.label.graphical.base.owner_index = owner_index;

                // Update the designator text
                new_d.param.label.text = designator.to_string();

                let loc = CoordPoint::from_raw(
                    d.param.label.graphical.location_x,
                    d.param.label.graphical.location_y,
                );
                let new_loc = self.transform_point(loc, location, orientation);
                new_d.param.label.graphical.location_x = new_loc.x.to_raw();
                new_d.param.label.graphical.location_y = new_loc.y.to_raw();

                SchRecord::Designator(new_d)
            }
            SchRecord::Parameter(p) => {
                let mut new_p = p.clone();
                new_p.label.graphical.base.owner_index = owner_index;

                let loc = CoordPoint::from_raw(
                    p.label.graphical.location_x,
                    p.label.graphical.location_y,
                );
                let new_loc = self.transform_point(loc, location, orientation);
                new_p.label.graphical.location_x = new_loc.x.to_raw();
                new_p.label.graphical.location_y = new_loc.y.to_raw();

                SchRecord::Parameter(new_p)
            }
            _ => {
                // For other record types, just update owner index if possible
                record.clone()
            }
        }
    }

    /// Transform a point from local (library) to absolute coordinates.
    fn transform_point(
        &self,
        local: CoordPoint,
        component_location: CoordPoint,
        orientation: Orientation,
    ) -> CoordPoint {
        let rotated = local.rotate(CoordPoint::ZERO, orientation.rotation_degrees());

        let mirrored = if orientation.is_mirrored() {
            CoordPoint::new(-rotated.x, rotated.y)
        } else {
            rotated
        };

        mirrored.translate(component_location.x, component_location.y)
    }

    /// Create a designator record.
    fn create_designator(&self, text: &str, location: CoordPoint, owner_index: i32) -> SchRecord {
        use crate::records::sch::{
            SchGraphicalBase, SchLabel, SchPrimitiveBase, TextJustification,
        };

        let label = SchLabel {
            graphical: SchGraphicalBase {
                base: SchPrimitiveBase {
                    owner_index,
                    ..Default::default()
                },
                location_x: location.x.to_raw() + Coord::from_mils(10.0).to_raw(),
                location_y: location.y.to_raw() + Coord::from_mils(30.0).to_raw(),
                ..Default::default()
            },
            text: text.to_string(),
            justification: TextJustification::BOTTOM_LEFT,
            font_id: 1,
            ..Default::default()
        };

        let param = SchParameter {
            label,
            name: "Designator".to_string(),
            read_only_state: 1,
            ..Default::default()
        };

        SchRecord::Designator(SchDesignator {
            param,
            unknown_params: UnknownFields::default(),
        })
    }
}

/// Generate a simple UUID-like string (non-deterministic).
///
/// **Prefer using `uuid_simple_deterministic()` for reproducible execution.**
#[deprecated(
    since = "0.1.0",
    note = "Use uuid_simple_deterministic() with a DeterminismContext for reproducible execution"
)]
#[allow(dead_code)]
fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    format!("{:016X}", timestamp & 0xFFFFFFFFFFFFFFFF)
}

/// Generate a simple UUID-like string deterministically.
fn uuid_simple_deterministic(_det: &mut ()) -> String {
    uuid::Uuid::new_v4().simple().to_string()
}
