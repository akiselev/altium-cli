//! Edit session for tracking changes and managing schematic modifications.
//!
//! # V2 Migration Note
//! This module heavily uses v1 types (SchDoc, SchRecord, SchWire, SchNetLabel, etc.).
//! TODO: Migrate to v2 types when SchDocV2 provides equivalent editing capabilities.

#![allow(deprecated)]

use std::path::Path;

use crate::error::{AltiumError, Result};
use crate::io::SchDoc;
use crate::records::sch::{
    LineStyle, LineWidth, PortIoType, PortStyle, PowerObjectStyle, SchBus, SchBusEntry,
    SchGraphicalBase, SchJunction, SchLabel, SchNetLabel, SchPort, SchPowerObject, SchRecord,
    SchWire, TextJustification, TextOrientations,
};
use crate::types::{Coord, CoordPoint, CoordRect, UnknownFields};

use super::layout::LayoutEngine;
use super::library::LibraryManager;
use super::netlist::{Netlist, NetlistBuilder};
use super::routing::RoutingEngine;
use super::types::{
    EditOperation, Grid, Orientation, PlacementSuggestion, ValidationError, ValidationErrorKind,
};

/// An editing session for a schematic document.
pub struct EditSession {
    /// The schematic document being edited.
    pub doc: SchDoc,
    /// Layout engine for placement.
    layout: LayoutEngine,
    /// Routing engine for wire connections.
    routing: RoutingEngine,
    /// Library manager for component instantiation.
    library: LibraryManager,
    /// Netlist builder for connectivity analysis.
    netlist_builder: NetlistBuilder,
    /// Operation history for undo.
    history: Vec<EditOperation>,
    /// Redo stack.
    redo_stack: Vec<EditOperation>,
    /// Path to the source file (if loaded from file).
    source_path: Option<String>,
    /// Whether the document has been modified.
    modified: bool,
}

impl EditSession {
    /// Create a new empty editing session.
    pub fn new() -> Self {
        Self {
            doc: SchDoc::default(),
            layout: LayoutEngine::new(),
            routing: RoutingEngine::new(),
            library: LibraryManager::new(),
            netlist_builder: NetlistBuilder::new(),
            history: Vec::new(),
            redo_stack: Vec::new(),
            source_path: None,
            modified: false,
        }
    }

    /// Open an existing schematic file for editing.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let doc = SchDoc::open_file(&path)?;

        let mut session = Self::new();
        session.doc = doc;
        session.source_path = Some(path_str);
        session.library.init_designators_from(&session.doc);
        session.update_routing_obstacles();

        Ok(session)
    }

    /// Save the schematic to a file.
    pub fn save<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        self.doc.save_to_file(&path)?;
        self.source_path = Some(path.as_ref().to_string_lossy().to_string());
        self.modified = false;
        Ok(())
    }

    /// Save to the original file (if opened from file).
    pub fn save_to_original(&mut self) -> Result<()> {
        match &self.source_path {
            Some(path) => {
                self.doc.save_to_file(path)?;
                self.modified = false;
                Ok(())
            }
            None => Err(AltiumError::Parse("No source file path".to_string())),
        }
    }

    /// Check if the document has been modified.
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Get the source file path.
    pub fn source_path(&self) -> Option<&str> {
        self.source_path.as_deref()
    }

    /// Get the layout engine.
    pub fn layout(&self) -> &LayoutEngine {
        &self.layout
    }

    /// Get mutable access to the layout engine.
    pub fn layout_mut(&mut self) -> &mut LayoutEngine {
        &mut self.layout
    }

    /// Get the library manager.
    pub fn library(&self) -> &LibraryManager {
        &self.library
    }

    /// Get mutable access to the library manager.
    pub fn library_mut(&mut self) -> &mut LibraryManager {
        &mut self.library
    }

    /// Update routing obstacles from current document.
    fn update_routing_obstacles(&mut self) {
        self.routing
            .update_obstacles(&self.doc.primitives, &self.layout);
    }

    /// Set the grid configuration.
    pub fn set_grid(&mut self, grid: Grid) {
        self.layout.set_grid(grid);
        self.routing.set_grid(grid);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // COMPONENT OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════

    /// Load a SchLib library file.
    pub fn load_library<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        self.library.load_library(path)
    }

    /// Add a component from a loaded library.
    pub fn add_component(
        &mut self,
        lib_reference: &str,
        location: CoordPoint,
        orientation: Orientation,
        designator: Option<&str>,
    ) -> Result<usize> {
        let index = self.library.instantiate_component(
            lib_reference,
            location,
            orientation,
            designator,
            &mut self.doc,
        )?;

        let designator_str = designator
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.find_designator(index).unwrap_or_default());

        self.history.push(EditOperation::AddComponent {
            index,
            designator: designator_str,
        });
        self.redo_stack.clear();
        self.modified = true;
        self.update_routing_obstacles();

        Ok(index)
    }

    /// Find the best position for a new component.
    pub fn suggest_component_placement(
        &self,
        lib_reference: &str,
        near_component: Option<&str>,
    ) -> Vec<PlacementSuggestion> {
        // Estimate component bounds (default if not in library)
        let bounds = self
            .library
            .find_component(lib_reference)
            .map(|(_, comp)| {
                self.layout
                    .calculate_component_bounds(&comp.component, &comp.primitives, 0)
            })
            .unwrap_or_else(|| {
                crate::types::CoordRect::from_xywh(
                    Coord::ZERO,
                    Coord::ZERO,
                    Coord::from_mils(100.0),
                    Coord::from_mils(100.0),
                )
            });

        self.layout
            .suggest_placement(bounds, &self.doc.primitives, near_component)
    }

    /// Move a component to a new location.
    pub fn move_component(&mut self, index: usize, new_location: CoordPoint) -> Result<()> {
        let record = self
            .doc
            .primitives
            .get_mut(index)
            .ok_or_else(|| AltiumError::Parse("Component not found".to_string()))?;

        let component = match record {
            SchRecord::Component(c) => c,
            _ => return Err(AltiumError::Parse("Not a component".to_string())),
        };

        let old_location = CoordPoint::from_raw(
            component.graphical.location_x,
            component.graphical.location_y,
        );

        let snapped = self.layout.snap_to_grid(new_location);
        let dx = snapped.x.to_raw() - old_location.x.to_raw();
        let dy = snapped.y.to_raw() - old_location.y.to_raw();

        component.graphical.location_x = snapped.x.to_raw();
        component.graphical.location_y = snapped.y.to_raw();

        // Move all child primitives
        for (i, record) in self.doc.primitives.iter_mut().enumerate() {
            if i == index {
                continue;
            }

            let owner_index = match record {
                SchRecord::Pin(p) => p.graphical.base.owner_index,
                SchRecord::Line(l) => l.graphical.base.owner_index,
                SchRecord::Rectangle(r) => r.graphical.base.owner_index,
                SchRecord::Polygon(p) => p.graphical.base.owner_index,
                SchRecord::Polyline(p) => p.graphical.base.owner_index,
                SchRecord::Arc(a) => a.graphical.base.owner_index,
                SchRecord::Ellipse(e) => e.graphical.base.owner_index,
                SchRecord::Label(l) => l.graphical.base.owner_index,
                SchRecord::Designator(d) => d.param.label.graphical.base.owner_index,
                SchRecord::Parameter(p) => p.label.graphical.base.owner_index,
                _ => -1,
            };

            if owner_index == index as i32 {
                Self::translate_primitive(record, dx, dy);
            }
        }

        self.history.push(EditOperation::MoveComponent {
            index,
            from: old_location,
            to: snapped,
        });
        self.redo_stack.clear();
        self.modified = true;
        self.update_routing_obstacles();

        Ok(())
    }

    /// Translate a primitive by dx, dy.
    fn translate_primitive(record: &mut SchRecord, dx: i32, dy: i32) {
        match record {
            SchRecord::Pin(p) => {
                p.graphical.location_x += dx;
                p.graphical.location_y += dy;
            }
            SchRecord::Line(l) => {
                l.graphical.location_x += dx;
                l.graphical.location_y += dy;
                l.corner_x += dx;
                l.corner_y += dy;
            }
            SchRecord::Rectangle(r) => {
                r.graphical.location_x += dx;
                r.graphical.location_y += dy;
                r.corner_x += dx;
                r.corner_y += dy;
            }
            SchRecord::Polygon(p) => {
                p.graphical.location_x += dx;
                p.graphical.location_y += dy;
                for vertex in &mut p.vertices {
                    vertex.0 += dx;
                    vertex.1 += dy;
                }
            }
            SchRecord::Polyline(p) => {
                p.graphical.location_x += dx;
                p.graphical.location_y += dy;
                for vertex in &mut p.vertices {
                    vertex.0 += dx;
                    vertex.1 += dy;
                }
            }
            SchRecord::Arc(a) => {
                a.graphical.location_x += dx;
                a.graphical.location_y += dy;
            }
            SchRecord::Ellipse(e) => {
                e.graphical.location_x += dx;
                e.graphical.location_y += dy;
            }
            SchRecord::Label(l) => {
                l.graphical.location_x += dx;
                l.graphical.location_y += dy;
            }
            SchRecord::Designator(d) => {
                d.param.label.graphical.location_x += dx;
                d.param.label.graphical.location_y += dy;
            }
            SchRecord::Parameter(p) => {
                p.label.graphical.location_x += dx;
                p.label.graphical.location_y += dy;
            }
            _ => {}
        }
    }

    /// Delete a component and all its child primitives.
    pub fn delete_component(&mut self, index: usize) -> Result<()> {
        // Verify it's a component
        match self.doc.primitives.get(index) {
            Some(SchRecord::Component(_)) => {}
            _ => return Err(AltiumError::Parse("Not a component".to_string())),
        }

        let designator = self.find_designator(index).unwrap_or_default();

        // Find all primitives owned by this component
        let mut to_remove: Vec<usize> = vec![index];
        for (i, record) in self.doc.primitives.iter().enumerate() {
            let owner_index = match record {
                SchRecord::Pin(p) => p.graphical.base.owner_index,
                SchRecord::Line(l) => l.graphical.base.owner_index,
                SchRecord::Rectangle(r) => r.graphical.base.owner_index,
                SchRecord::Polygon(p) => p.graphical.base.owner_index,
                SchRecord::Polyline(p) => p.graphical.base.owner_index,
                SchRecord::Arc(a) => a.graphical.base.owner_index,
                SchRecord::Ellipse(e) => e.graphical.base.owner_index,
                SchRecord::Label(l) => l.graphical.base.owner_index,
                SchRecord::Designator(d) => d.param.label.graphical.base.owner_index,
                SchRecord::Parameter(p) => p.label.graphical.base.owner_index,
                _ => -1,
            };

            if owner_index == index as i32 && !to_remove.contains(&i) {
                to_remove.push(i);
            }
        }

        // Remove in reverse order to maintain indices
        to_remove.sort_unstable();
        to_remove.reverse();

        for i in &to_remove {
            self.doc.primitives.remove(*i);
        }

        // Update owner indices for remaining primitives
        self.update_owner_indices_after_removal(&to_remove);

        self.history
            .push(EditOperation::RemoveComponent { index, designator });
        self.redo_stack.clear();
        self.modified = true;
        self.update_routing_obstacles();

        Ok(())
    }

    /// Update owner indices after removing primitives.
    fn update_owner_indices_after_removal(&mut self, removed: &[usize]) {
        for record in &mut self.doc.primitives {
            let owner = match record {
                SchRecord::Pin(p) => &mut p.graphical.base.owner_index,
                SchRecord::Line(l) => &mut l.graphical.base.owner_index,
                SchRecord::Rectangle(r) => &mut r.graphical.base.owner_index,
                SchRecord::Polygon(p) => &mut p.graphical.base.owner_index,
                SchRecord::Polyline(p) => &mut p.graphical.base.owner_index,
                SchRecord::Arc(a) => &mut a.graphical.base.owner_index,
                SchRecord::Ellipse(e) => &mut e.graphical.base.owner_index,
                SchRecord::Label(l) => &mut l.graphical.base.owner_index,
                SchRecord::Designator(d) => &mut d.param.label.graphical.base.owner_index,
                SchRecord::Parameter(p) => &mut p.label.graphical.base.owner_index,
                _ => continue,
            };

            if *owner >= 0 {
                // Count how many removed indices are below this owner
                let offset = removed.iter().filter(|&&r| (r as i32) < *owner).count();
                *owner -= offset as i32;
            }
        }
    }

    /// Find the designator for a component.
    fn find_designator(&self, component_index: usize) -> Option<String> {
        for record in &self.doc.primitives {
            if let SchRecord::Designator(d) = record {
                if d.param.label.graphical.base.owner_index == component_index as i32 {
                    return Some(d.text().to_string());
                }
            }
        }
        None
    }

    // ═══════════════════════════════════════════════════════════════════════
    // WIRE OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════

    /// Add a wire with specified vertices.
    pub fn add_wire(&mut self, vertices: &[CoordPoint]) -> Result<usize> {
        if vertices.len() < 2 {
            return Err(AltiumError::Parse(
                "Wire must have at least 2 vertices".to_string(),
            ));
        }

        let snapped: Vec<(i32, i32)> = vertices
            .iter()
            .map(|p| {
                let s = self.layout.snap_to_grid(*p);
                (s.x.to_raw(), s.y.to_raw())
            })
            .collect();

        let wire = SchWire {
            graphical: SchGraphicalBase::new_wire_or_text(),
            line_width: LineWidth::Small, // LINEWIDTH=1 (standard wire width)
            line_style: LineStyle::Solid,
            vertices: snapped,
            unknown_params: UnknownFields::default(),
        };

        let index = self.doc.primitives.len();
        self.doc.primitives.push(SchRecord::Wire(wire));

        self.history.push(EditOperation::AddWire { index });
        self.redo_stack.clear();
        self.modified = true;
        self.update_routing_obstacles();

        Ok(index)
    }

    /// Route a wire between two points automatically.
    pub fn route_wire(&mut self, start: CoordPoint, end: CoordPoint) -> Result<usize> {
        self.routing
            .update_obstacles(&self.doc.primitives, &self.layout);

        let path = self
            .routing
            .route(start, end)
            .ok_or_else(|| AltiumError::Parse("No route found".to_string()))?;

        // Add wire segments
        let vertices = path.vertices();
        if vertices.is_empty() {
            return Err(AltiumError::Parse("Empty route".to_string()));
        }

        let index = self.add_wire(&vertices)?;

        // Add junctions if needed
        for junction in self.routing.find_junctions(&path) {
            self.add_junction(junction)?;
        }

        Ok(index)
    }

    /// Connect two component pins with automatic routing.
    pub fn connect_pins(
        &mut self,
        component1: &str,
        pin1: &str,
        component2: &str,
        pin2: &str,
    ) -> Result<usize> {
        // Find pin locations and component bounds
        let pin1_loc = self.find_pin_location(component1, pin1)?;
        let pin2_loc = self.find_pin_location(component2, pin2)?;

        // Find component bounds to exclude from obstacles
        let mut exclude_rects = Vec::new();
        if let Some(bounds) = self.find_component_bounds(component1) {
            exclude_rects.push(bounds);
        }
        if let Some(bounds) = self.find_component_bounds(component2) {
            exclude_rects.push(bounds);
        }

        self.route_wire_excluding(pin1_loc, pin2_loc, &exclude_rects)
    }

    /// Route a wire between two points, excluding certain component bounds from obstacles.
    pub fn route_wire_excluding(
        &mut self,
        start: CoordPoint,
        end: CoordPoint,
        exclude_rects: &[CoordRect],
    ) -> Result<usize> {
        self.routing
            .update_obstacles(&self.doc.primitives, &self.layout);

        let path = self
            .routing
            .route_excluding(start, end, exclude_rects)
            .ok_or_else(|| AltiumError::Parse("No route found".to_string()))?;

        // Add wire segments
        let vertices = path.vertices();
        if vertices.is_empty() {
            return Err(AltiumError::Parse("Empty route".to_string()));
        }

        let index = self.add_wire(&vertices)?;

        // Add junctions if needed
        for junction in self.routing.find_junctions(&path) {
            self.add_junction(junction)?;
        }

        Ok(index)
    }

    /// Find the bounds of a component by designator.
    fn find_component_bounds(&self, designator: &str) -> Option<CoordRect> {
        let components = self.layout.get_placed_components(&self.doc.primitives);
        components
            .iter()
            .find(|c| c.designator == designator)
            .map(|c| c.bounds)
    }

    /// Find the endpoint location of a pin.
    fn find_pin_location(&self, component: &str, pin: &str) -> Result<CoordPoint> {
        let components = self.layout.get_placed_components(&self.doc.primitives);

        let comp = components
            .iter()
            .find(|c| c.designator == component)
            .ok_or_else(|| AltiumError::Parse(format!("Component not found: {}", component)))?;

        let pin_loc = comp
            .pin_locations
            .iter()
            .find(|p| p.designator == pin || p.name == pin)
            .ok_or_else(|| AltiumError::Parse(format!("Pin not found: {}.{}", component, pin)))?;

        Ok(pin_loc.location)
    }

    /// Delete a wire by index.
    pub fn delete_wire(&mut self, index: usize) -> Result<()> {
        match self.doc.primitives.get(index) {
            Some(SchRecord::Wire(_)) => {}
            _ => return Err(AltiumError::Parse("Not a wire".to_string())),
        }

        self.doc.primitives.remove(index);
        self.update_owner_indices_after_removal(&[index]);

        self.history.push(EditOperation::RemoveWire { index });
        self.redo_stack.clear();
        self.modified = true;
        self.update_routing_obstacles();

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════
    // BUS OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════

    /// Add a bus with specified vertices.
    pub fn add_bus(&mut self, vertices: &[CoordPoint]) -> Result<usize> {
        if vertices.len() < 2 {
            return Err(AltiumError::Parse(
                "Bus must have at least 2 vertices".to_string(),
            ));
        }

        let snapped: Vec<(i32, i32)> = vertices
            .iter()
            .map(|p| {
                let s = self.layout.snap_to_grid(*p);
                (s.x.to_raw(), s.y.to_raw())
            })
            .collect();

        let bus = SchBus {
            graphical: SchGraphicalBase::default(),
            line_width: LineWidth::Medium,
            vertices: snapped,
            ..Default::default()
        };

        let index = self.doc.primitives.len();
        self.doc.primitives.push(SchRecord::Bus(bus));

        self.modified = true;
        self.update_routing_obstacles();

        Ok(index)
    }

    /// Add a bus entry connecting a bus to a wire.
    ///
    /// * `bus_point` - Point on the bus where the entry connects
    /// * `wire_point` - Point where the wire will connect (typically 10 mil offset)
    pub fn add_bus_entry(
        &mut self,
        bus_point: CoordPoint,
        wire_point: CoordPoint,
    ) -> Result<usize> {
        let bus_snapped = self.layout.snap_to_grid(bus_point);
        let wire_snapped = self.layout.snap_to_grid(wire_point);

        let bus_entry = SchBusEntry::new(
            bus_snapped.x.to_raw(),
            bus_snapped.y.to_raw(),
            wire_snapped.x.to_raw(),
            wire_snapped.y.to_raw(),
        );

        let index = self.doc.primitives.len();
        self.doc.primitives.push(SchRecord::BusEntry(bus_entry));

        self.modified = true;
        self.update_routing_obstacles();

        Ok(index)
    }

    /// Find buses in the document.
    pub fn find_buses(&self) -> Vec<(usize, &SchBus)> {
        self.doc
            .primitives
            .iter()
            .enumerate()
            .filter_map(|(i, p)| match p {
                SchRecord::Bus(bus) => Some((i, bus)),
                _ => None,
            })
            .collect()
    }

    /// Find a bus that contains or is near a point.
    pub fn find_bus_at(&self, point: CoordPoint) -> Option<(usize, &SchBus)> {
        let x = point.x.to_raw();
        let y = point.y.to_raw();
        let tolerance = 100000; // 10 mil tolerance

        for (i, prim) in self.doc.primitives.iter().enumerate() {
            if let SchRecord::Bus(bus) = prim {
                if bus.contains_point(x, y) {
                    return Some((i, bus));
                }
                // Check with tolerance
                for window in bus.vertices.windows(2) {
                    let (x1, y1) = window[0];
                    let (x2, y2) = window[1];

                    // Check vertical segment
                    if x1 == x2 {
                        let (min_y, max_y) = if y1 < y2 { (y1, y2) } else { (y2, y1) };
                        if (x - x1).abs() < tolerance
                            && y >= min_y - tolerance
                            && y <= max_y + tolerance
                        {
                            return Some((i, bus));
                        }
                    }
                    // Check horizontal segment
                    if y1 == y2 {
                        let (min_x, max_x) = if x1 < x2 { (x1, x2) } else { (x2, x1) };
                        if (y - y1).abs() < tolerance
                            && x >= min_x - tolerance
                            && x <= max_x + tolerance
                        {
                            return Some((i, bus));
                        }
                    }
                }
            }
        }
        None
    }

    /// Route a wire to a bus, automatically creating a bus entry.
    ///
    /// Creates a bus entry at the nearest point on the bus to the wire endpoint,
    /// then routes the wire to the bus entry's wire-side point.
    pub fn route_to_bus(
        &mut self,
        wire_start: CoordPoint,
        bus_point: CoordPoint,
    ) -> Result<(usize, usize)> {
        // Find the bus at or near the specified point
        let (_bus_idx, bus) = self
            .find_bus_at(bus_point)
            .ok_or_else(|| AltiumError::Parse("No bus found at specified point".to_string()))?;

        // Find the nearest point on the bus
        let target_x = bus_point.x.to_raw();
        let target_y = bus_point.y.to_raw();

        // Snap to bus (find closest point on bus)
        let mut best_point = bus.vertices[0];
        let mut best_dist = i64::MAX;

        for window in bus.vertices.windows(2) {
            let (x1, y1) = window[0];
            let (x2, y2) = window[1];

            // Project point onto line segment
            let (proj_x, proj_y) = if x1 == x2 {
                // Vertical segment
                let clamped_y = target_y.max(y1.min(y2)).min(y1.max(y2));
                (x1, clamped_y)
            } else if y1 == y2 {
                // Horizontal segment
                let clamped_x = target_x.max(x1.min(x2)).min(x1.max(x2));
                (clamped_x, y1)
            } else {
                // Diagonal - use endpoint
                window[0]
            };

            let dist =
                (proj_x as i64 - target_x as i64).pow(2) + (proj_y as i64 - target_y as i64).pow(2);
            if dist < best_dist {
                best_dist = dist;
                best_point = (proj_x, proj_y);
            }
        }

        let bus_entry_point = CoordPoint::from_raw(best_point.0, best_point.1);

        // Calculate wire-side point (offset by 10 mil diagonally)
        let offset = 100000; // 10 mil
        let dx = if wire_start.x.to_raw() < best_point.0 {
            -offset
        } else {
            offset
        };
        let wire_entry_point = CoordPoint::from_raw(best_point.0 + dx, best_point.1 + dx);

        // Create bus entry
        let entry_idx = self.add_bus_entry(bus_entry_point, wire_entry_point)?;

        // Route wire to bus entry
        let wire_idx = self.route_wire(wire_start, wire_entry_point)?;

        Ok((wire_idx, entry_idx))
    }

    // ═══════════════════════════════════════════════════════════════════════
    // NET LABEL AND POWER OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════

    /// Add a net label at a location.
    pub fn add_net_label(&mut self, name: &str, location: CoordPoint) -> Result<usize> {
        let snapped = self.layout.snap_to_grid(location);

        let mut graphical = SchGraphicalBase::new_wire_or_text();
        graphical.location_x = snapped.x.to_raw();
        graphical.location_y = snapped.y.to_raw();

        let label = SchLabel {
            graphical,
            text: name.to_string(),
            justification: TextJustification::BOTTOM_LEFT,
            font_id: 1, // FONTID=1 (standard font)
            ..Default::default()
        };

        let net_label = SchNetLabel {
            label,
            unknown_params: UnknownFields::default(),
        };

        let index = self.doc.primitives.len();
        self.doc.primitives.push(SchRecord::NetLabel(net_label));

        self.history.push(EditOperation::AddNetLabel {
            index,
            net_name: name.to_string(),
        });
        self.redo_stack.clear();
        self.modified = true;

        Ok(index)
    }

    /// Add a power port at a location.
    pub fn add_power_port(
        &mut self,
        net_name: &str,
        location: CoordPoint,
        style: PowerObjectStyle,
        orientation: TextOrientations,
    ) -> Result<usize> {
        let snapped = self.layout.snap_to_grid(location);

        let mut graphical = SchGraphicalBase::new_graphical();
        graphical.location_x = snapped.x.to_raw();
        graphical.location_y = snapped.y.to_raw();
        // Power objects don't have area_color
        graphical.area_color = 0;

        let power = SchPowerObject {
            graphical,
            style,
            orientation,
            text: net_name.to_string(),
            show_net_name: true, // SHOWNETNAME=T (default)
            font_id: 1,          // FONTID=1 (standard font)
            unknown_params: UnknownFields::default(),
        };

        let index = self.doc.primitives.len();
        self.doc.primitives.push(SchRecord::PowerObject(power));

        self.history.push(EditOperation::AddPowerPort {
            index,
            net_name: net_name.to_string(),
        });
        self.redo_stack.clear();
        self.modified = true;

        Ok(index)
    }

    /// Smart-wire a pin: create a wire stub from the pin endpoint and attach a net label or power port.
    ///
    /// Returns (wire_index, label_or_power_index).
    pub fn smart_wire_pin(
        &mut self,
        component: &str,
        pin: &str,
        net_name: &str,
        power_style: Option<PowerObjectStyle>,
        wire_length_mils: f64,
    ) -> Result<(usize, usize)> {
        use super::types::Direction;

        // Find the pin location and direction
        let components = self.layout.get_placed_components(&self.doc.primitives);
        let comp = components
            .iter()
            .find(|c| c.designator == component)
            .ok_or_else(|| AltiumError::Parse(format!("Component not found: {}", component)))?;

        let pin_loc = comp
            .pin_locations
            .iter()
            .find(|p| p.designator == pin || p.name == pin)
            .ok_or_else(|| {
                AltiumError::Parse(format!("Pin not found: {}.{}", component, pin))
            })?;

        let endpoint = pin_loc.location;
        let direction = pin_loc.direction;

        // Calculate wire end point: extend from pin endpoint in pin's direction
        let (dx, dy) = direction.unit_vector();
        let wire_length_raw = (wire_length_mils * 10000.0) as i32;
        let wire_end = CoordPoint::from_raw(
            endpoint.x.to_raw() + dx * wire_length_raw,
            endpoint.y.to_raw() + dy * wire_length_raw,
        );

        // Add wire stub from pin endpoint to wire end
        let wire_idx = self.add_wire(&[endpoint, wire_end])?;

        // Add net label or power port at wire end
        let label_idx = if let Some(style) = power_style {
            // Determine power port orientation based on pin direction
            // Power port connection point is at its origin, with the symbol extending
            // in the orientation direction. We want the symbol to point AWAY from the wire.
            let orient = match direction {
                Direction::Up => TextOrientations::NONE,           // Symbol points up
                Direction::Down => TextOrientations::FLIPPED,      // Symbol points down
                Direction::Left => TextOrientations::ROTATED,      // Symbol points left
                Direction::Right => {
                    TextOrientations::ROTATED | TextOrientations::FLIPPED
                }
            };
            self.add_power_port(net_name, wire_end, style, orient)?
        } else {
            self.add_net_label(net_name, wire_end)?
        };

        Ok((wire_idx, label_idx))
    }

    /// Add a junction at a location.
    pub fn add_junction(&mut self, location: CoordPoint) -> Result<usize> {
        let snapped = self.layout.snap_to_grid(location);

        let mut graphical = SchGraphicalBase::new_graphical();
        graphical.location_x = snapped.x.to_raw();
        graphical.location_y = snapped.y.to_raw();
        // Junctions use INDEXINSHEET=-1 and don't have area_color
        graphical.base.owner_index = -1;
        graphical.area_color = 0;

        let junction = SchJunction {
            graphical,
            unknown_params: UnknownFields::default(),
        };

        let index = self.doc.primitives.len();
        self.doc.primitives.push(SchRecord::Junction(junction));

        self.history.push(EditOperation::AddJunction {
            index,
            location: snapped,
        });
        self.redo_stack.clear();
        self.modified = true;

        Ok(index)
    }

    /// Add a port at a location.
    pub fn add_port(
        &mut self,
        name: &str,
        location: CoordPoint,
        io_type: PortIoType,
    ) -> Result<usize> {
        let snapped = self.layout.snap_to_grid(location);

        let mut graphical = SchGraphicalBase::new_graphical();
        graphical.location_x = snapped.x.to_raw();
        graphical.location_y = snapped.y.to_raw();
        // Ports don't have area_color
        graphical.area_color = 0;

        let port = SchPort {
            graphical,
            style: PortStyle::Right, // Default right-facing port
            io_type,
            name: name.to_string(),
            width: 40,  // Standard port width (400 mils)
            height: 10, // Standard port height (100 mils)
            font_id: 1, // FONTID=1 (standard font)
            ..Default::default()
        };

        let index = self.doc.primitives.len();
        self.doc.primitives.push(SchRecord::Port(port));

        self.modified = true;
        Ok(index)
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ANALYSIS AND VALIDATION
    // ═══════════════════════════════════════════════════════════════════════

    /// Build the netlist for the current document.
    pub fn build_netlist(&self) -> Netlist {
        self.netlist_builder.build(&self.doc.primitives)
    }

    /// Find unconnected pins.
    pub fn find_unconnected_pins(&self) -> Vec<(String, String, (i32, i32))> {
        self.netlist_builder
            .find_unconnected_pins(&self.doc.primitives)
    }

    /// Find missing junctions.
    pub fn find_missing_junctions(&self) -> Vec<(i32, i32)> {
        self.netlist_builder
            .find_missing_junctions(&self.doc.primitives)
    }

    /// Add missing junctions automatically.
    pub fn add_missing_junctions(&mut self) -> Result<usize> {
        let missing = self.find_missing_junctions();
        let count = missing.len();

        for loc in missing {
            self.add_junction(CoordPoint::from_raw(loc.0, loc.1))?;
        }

        Ok(count)
    }

    /// Validate the schematic and return any errors.
    pub fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        // Check for component overlaps
        let components = self.layout.get_placed_components(&self.doc.primitives);
        for i in 0..components.len() {
            for j in i + 1..components.len() {
                if components[i].bounds.intersects(components[j].bounds) {
                    errors.push(ValidationError {
                        kind: ValidationErrorKind::ComponentOverlap,
                        message: format!(
                            "Components {} and {} overlap",
                            components[i].designator, components[j].designator
                        ),
                        location: Some(components[i].bounds.center()),
                        components: vec![
                            components[i].designator.clone(),
                            components[j].designator.clone(),
                        ],
                    });
                }
            }
        }

        // Check for duplicate designators
        let mut designators: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        for comp in &components {
            if !comp.designator.is_empty() {
                *designators.entry(&comp.designator).or_insert(0) += 1;
            }
        }
        for (des, count) in designators {
            if count > 1 {
                errors.push(ValidationError {
                    kind: ValidationErrorKind::DuplicateDesignator,
                    message: format!("Duplicate designator: {} (appears {} times)", des, count),
                    location: None,
                    components: vec![des.to_string()],
                });
            }
        }

        // Check for unconnected pins
        let unconnected = self.find_unconnected_pins();
        for (comp, pin, loc) in unconnected {
            errors.push(ValidationError {
                kind: ValidationErrorKind::UnconnectedPin,
                message: format!("Unconnected pin: {}.{}", comp, pin),
                location: Some(CoordPoint::from_raw(loc.0, loc.1)),
                components: vec![comp],
            });
        }

        // Check for missing junctions
        let missing_junctions = self.find_missing_junctions();
        for loc in missing_junctions {
            errors.push(ValidationError {
                kind: ValidationErrorKind::MissingJunction,
                message: format!("Missing junction at ({}, {})", loc.0 / 10000, loc.1 / 10000),
                location: Some(CoordPoint::from_raw(loc.0, loc.1)),
                components: vec![],
            });
        }

        errors
    }

    // ═══════════════════════════════════════════════════════════════════════
    // UNDO/REDO (placeholder - full implementation would require snapshots)
    // ═══════════════════════════════════════════════════════════════════════

    /// Get the number of operations in history.
    pub fn history_count(&self) -> usize {
        self.history.len()
    }

    /// Check if undo is available.
    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }

    /// Check if redo is available.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Get a summary of recent operations.
    pub fn recent_operations(&self, count: usize) -> Vec<String> {
        self.history
            .iter()
            .rev()
            .take(count)
            .map(|op| match op {
                EditOperation::AddComponent { designator, .. } => {
                    format!("Add component {}", designator)
                }
                EditOperation::RemoveComponent { designator, .. } => {
                    format!("Remove component {}", designator)
                }
                EditOperation::MoveComponent { index, .. } => {
                    format!("Move component at index {}", index)
                }
                EditOperation::AddWire { .. } => "Add wire".to_string(),
                EditOperation::RemoveWire { .. } => "Remove wire".to_string(),
                EditOperation::AddJunction { .. } => "Add junction".to_string(),
                EditOperation::AddNetLabel { net_name, .. } => {
                    format!("Add net label {}", net_name)
                }
                EditOperation::AddPowerPort { net_name, .. } => {
                    format!("Add power port {}", net_name)
                }
                EditOperation::Batch { operations } => {
                    format!("Batch ({} operations)", operations.len())
                }
            })
            .collect()
    }
}

impl Default for EditSession {
    fn default() -> Self {
        Self::new()
    }
}
