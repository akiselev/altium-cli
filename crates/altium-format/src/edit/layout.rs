//! Layout engine for component placement and collision detection.
//!
//! # V2 Migration Note
//! This module uses v1 types (SchComponent, SchPin, SchRecord, etc.).
//! TODO: Migrate to v2 types when SchDocV2 provides equivalent functionality.

#![allow(deprecated)]

use std::collections::HashMap;

use crate::records::sch::{PinConglomerateFlags, SchComponent, SchPin, SchPrimitive, SchRecord};
use crate::types::{Coord, CoordPoint, CoordRect};

use super::types::{
    DEFAULT_COMPONENT_SPACING_MILS, Direction, Grid, Orientation, PinLocation, PlacedComponent,
    PlacementSuggestion,
};

/// Layout engine for managing component placement on a schematic.
pub struct LayoutEngine {
    /// Grid configuration.
    grid: Grid,
    /// Minimum spacing between components.
    component_spacing: Coord,
    /// Cached component bounds.
    component_bounds: HashMap<usize, CoordRect>,
    /// Sheet bounds.
    sheet_bounds: CoordRect,
}

impl Default for LayoutEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutEngine {
    /// Create a new layout engine with default settings.
    pub fn new() -> Self {
        Self {
            grid: Grid::default(),
            component_spacing: Coord::from_mils(DEFAULT_COMPONENT_SPACING_MILS),
            component_bounds: HashMap::new(),
            sheet_bounds: CoordRect::from_xywh(
                Coord::ZERO,
                Coord::ZERO,
                Coord::from_mils(11000.0), // Default A4 landscape
                Coord::from_mils(8500.0),
            ),
        }
    }

    /// Set the grid configuration.
    pub fn set_grid(&mut self, grid: Grid) {
        self.grid = grid;
    }

    /// Get the grid configuration.
    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    /// Set the sheet bounds.
    pub fn set_sheet_bounds(&mut self, bounds: CoordRect) {
        self.sheet_bounds = bounds;
    }

    /// Set minimum component spacing.
    pub fn set_component_spacing(&mut self, spacing: Coord) {
        self.component_spacing = spacing;
    }

    /// Clear cached bounds.
    pub fn clear_cache(&mut self) {
        self.component_bounds.clear();
    }

    /// Get the bounds of a schematic record.
    fn get_record_bounds(&self, record: &SchRecord) -> CoordRect {
        match record {
            SchRecord::Pin(p) => p.calculate_bounds(),
            SchRecord::Line(l) => l.calculate_bounds(),
            SchRecord::Rectangle(r) => r.calculate_bounds(),
            SchRecord::Polygon(p) => p.calculate_bounds(),
            SchRecord::Polyline(p) => p.calculate_bounds(),
            SchRecord::Arc(a) => a.calculate_bounds(),
            SchRecord::Ellipse(e) => e.calculate_bounds(),
            SchRecord::Label(l) => l.calculate_bounds(),
            SchRecord::Wire(w) => w.calculate_bounds(),
            SchRecord::Junction(j) => j.calculate_bounds(),
            SchRecord::NetLabel(n) => n.calculate_bounds(),
            SchRecord::PowerObject(p) => p.calculate_bounds(),
            SchRecord::Port(p) => p.calculate_bounds(),
            _ => CoordRect::empty(),
        }
    }

    /// Calculate the bounding box for a component and its child primitives.
    pub fn calculate_component_bounds(
        &self,
        component: &SchComponent,
        primitives: &[SchRecord],
        component_index: usize,
    ) -> CoordRect {
        let base_x = component.graphical.location_x;
        let base_y = component.graphical.location_y;

        let mut bounds = CoordRect::empty();

        // Find all primitives owned by this component
        for (i, record) in primitives.iter().enumerate() {
            if i == component_index {
                continue; // Skip the component itself
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

            if owner_index == component_index as i32 {
                let prim_bounds = self.get_record_bounds(record);
                if !prim_bounds.is_empty() {
                    bounds = bounds.union(prim_bounds);
                }
            }
        }

        // If no child bounds found, use a default size
        if bounds.is_empty() {
            bounds = CoordRect::from_xywh(
                Coord::from_raw(base_x),
                Coord::from_raw(base_y),
                Coord::from_mils(100.0),
                Coord::from_mils(100.0),
            );
        }

        bounds
    }

    /// Get placed components from primitives.
    pub fn get_placed_components(&self, primitives: &[SchRecord]) -> Vec<PlacedComponent> {
        let mut placed = Vec::new();

        for (i, record) in primitives.iter().enumerate() {
            if let SchRecord::Component(component) = record {
                let bounds = self.calculate_component_bounds(component, primitives, i);
                let pin_locations = self.get_pin_locations(primitives, i);

                // Find the designator
                let designator = self.find_designator(primitives, i);

                placed.push(PlacedComponent {
                    index: i,
                    designator,
                    lib_reference: component.lib_reference.clone(),
                    bounds,
                    pin_locations,
                });
            }
        }

        placed
    }

    /// Find the designator for a component.
    fn find_designator(&self, primitives: &[SchRecord], component_index: usize) -> String {
        for (i, record) in primitives.iter().enumerate() {
            if i == component_index {
                continue;
            }
            if let SchRecord::Designator(d) = record {
                if d.param.label.graphical.base.owner_index == component_index as i32 {
                    return d.param.value().to_string();
                }
            }
        }
        String::new()
    }

    /// Get pin locations for a component.
    fn get_pin_locations(
        &self,
        primitives: &[SchRecord],
        component_index: usize,
    ) -> Vec<PinLocation> {
        let mut pins = Vec::new();

        // Get component location for reference
        let component = match &primitives[component_index] {
            SchRecord::Component(c) => c,
            _ => return pins,
        };
        let _base_x = component.graphical.location_x;
        let _base_y = component.graphical.location_y;

        for (i, record) in primitives.iter().enumerate() {
            if i == component_index {
                continue;
            }
            if let SchRecord::Pin(pin) = record {
                if pin.graphical.base.owner_index == component_index as i32 {
                    let pin_loc = self.calculate_pin_endpoint(pin);
                    let direction = self.get_pin_direction(pin);

                    pins.push(PinLocation {
                        designator: pin.designator.clone(),
                        name: pin.name.clone(),
                        location: pin_loc,
                        direction,
                    });
                }
            }
        }

        pins
    }

    /// Calculate the endpoint of a pin (where wires connect).
    fn calculate_pin_endpoint(&self, pin: &SchPin) -> CoordPoint {
        let base_x = pin.graphical.location_x;
        let base_y = pin.graphical.location_y;
        let length = pin.pin_length;

        // Pin direction based on rotation in pin_conglomerate
        let rotated = pin.pin_conglomerate.contains(PinConglomerateFlags::ROTATED);
        let flipped = pin.pin_conglomerate.contains(PinConglomerateFlags::FLIPPED);

        let (dx, dy) = match (rotated, flipped) {
            (false, false) => (length, 0), // Right
            (true, false) => (0, length),  // Up
            (false, true) => (-length, 0), // Left
            (true, true) => (0, -length),  // Down
        };

        CoordPoint::from_raw(base_x + dx, base_y + dy)
    }

    /// Get the direction a pin faces.
    fn get_pin_direction(&self, pin: &SchPin) -> Direction {
        let rotated = pin.pin_conglomerate.contains(PinConglomerateFlags::ROTATED);
        let flipped = pin.pin_conglomerate.contains(PinConglomerateFlags::FLIPPED);

        match (rotated, flipped) {
            (false, false) => Direction::Right,
            (true, false) => Direction::Up,
            (false, true) => Direction::Left,
            (true, true) => Direction::Down,
        }
    }

    /// Check if a rectangle collides with any existing component.
    pub fn check_collision(
        &self,
        rect: CoordRect,
        primitives: &[SchRecord],
        exclude_index: Option<usize>,
    ) -> bool {
        let placed = self.get_placed_components(primitives);

        for component in placed {
            if Some(component.index) == exclude_index {
                continue;
            }

            // Expand bounds by spacing
            let expanded = CoordRect::from_points(
                component.bounds.location1.x - self.component_spacing,
                component.bounds.location1.y - self.component_spacing,
                component.bounds.location2.x + self.component_spacing,
                component.bounds.location2.y + self.component_spacing,
            );

            if expanded.intersects(rect) {
                return true;
            }
        }

        false
    }

    /// Find placement suggestions for a component with given bounds.
    pub fn suggest_placement(
        &self,
        component_bounds: CoordRect,
        primitives: &[SchRecord],
        near_component: Option<&str>,
    ) -> Vec<PlacementSuggestion> {
        let mut suggestions = Vec::new();
        let placed = self.get_placed_components(primitives);

        // Component size
        let width = component_bounds.width();
        let height = component_bounds.height();

        // If near_component is specified, prioritize positions near it
        if let Some(ref_designator) = near_component {
            if let Some(ref_component) = placed.iter().find(|c| c.designator == ref_designator) {
                suggestions.extend(self.suggest_near_component(
                    ref_component,
                    width,
                    height,
                    primitives,
                ));
            }
        }

        // Find empty regions on the sheet
        suggestions.extend(self.suggest_empty_regions(width, height, primitives));

        // Sort by score (higher is better)
        suggestions.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Return top suggestions
        suggestions.truncate(5);
        suggestions
    }

    /// Suggest placements near a reference component.
    fn suggest_near_component(
        &self,
        ref_component: &PlacedComponent,
        width: Coord,
        height: Coord,
        primitives: &[SchRecord],
    ) -> Vec<PlacementSuggestion> {
        let mut suggestions = Vec::new();
        let ref_bounds = ref_component.bounds;
        let spacing = self.component_spacing;

        // Try positions: right, left, above, below
        let positions = [
            (
                CoordPoint::new(ref_bounds.location2.x + spacing, ref_bounds.location1.y),
                "Right of",
                0.9,
            ),
            (
                CoordPoint::new(
                    ref_bounds.location1.x - width - spacing,
                    ref_bounds.location1.y,
                ),
                "Left of",
                0.85,
            ),
            (
                CoordPoint::new(ref_bounds.location1.x, ref_bounds.location2.y + spacing),
                "Above",
                0.8,
            ),
            (
                CoordPoint::new(
                    ref_bounds.location1.x,
                    ref_bounds.location1.y - height - spacing,
                ),
                "Below",
                0.75,
            ),
        ];

        for (pos, direction, base_score) in positions {
            let snapped = self.grid.snap(pos);
            let test_bounds = CoordRect::from_xywh(snapped.x, snapped.y, width, height);

            if !self.check_collision(test_bounds, primitives, None)
                && self.sheet_bounds.contains(snapped)
            {
                suggestions.push(PlacementSuggestion {
                    location: snapped,
                    orientation: Orientation::Normal,
                    score: base_score,
                    reason: format!("{} {}", direction, ref_component.designator),
                });
            }
        }

        suggestions
    }

    /// Suggest placements in empty regions.
    fn suggest_empty_regions(
        &self,
        width: Coord,
        height: Coord,
        primitives: &[SchRecord],
    ) -> Vec<PlacementSuggestion> {
        let mut suggestions = Vec::new();
        let placed = self.get_placed_components(primitives);

        // Calculate occupied regions
        let mut occupied = CoordRect::empty();
        for component in &placed {
            occupied = occupied.union(component.bounds);
        }

        // Try positions in a grid pattern across the sheet
        let grid_step = Coord::from_mils(500.0);
        let margin = Coord::from_mils(200.0);

        let mut y = self.sheet_bounds.location1.y + margin;
        while y < self.sheet_bounds.location2.y - height - margin {
            let mut x = self.sheet_bounds.location1.x + margin;
            while x < self.sheet_bounds.location2.x - width - margin {
                let pos = self.grid.snap(CoordPoint::new(x, y));
                let test_bounds = CoordRect::from_xywh(pos.x, pos.y, width, height);

                if !self.check_collision(test_bounds, primitives, None) {
                    // Score based on position (prefer upper-left for new components)
                    let x_score = 1.0 - (pos.x.to_mils() / self.sheet_bounds.width().to_mils());
                    let y_score = pos.y.to_mils() / self.sheet_bounds.height().to_mils();
                    let score = (x_score + y_score) * 0.3;

                    suggestions.push(PlacementSuggestion {
                        location: pos,
                        orientation: Orientation::Normal,
                        score,
                        reason: format!(
                            "Empty region at ({:.0}, {:.0})",
                            pos.x.to_mils(),
                            pos.y.to_mils()
                        ),
                    });
                }

                x = x + grid_step;
            }
            y = y + grid_step;
        }

        // Limit number of empty region suggestions
        suggestions.truncate(10);
        suggestions
    }

    /// Find the best position for a new component.
    pub fn find_best_position(
        &self,
        component_bounds: CoordRect,
        primitives: &[SchRecord],
    ) -> Option<CoordPoint> {
        let suggestions = self.suggest_placement(component_bounds, primitives, None);
        suggestions.first().map(|s| s.location)
    }

    /// Align components to grid.
    pub fn snap_to_grid(&self, point: CoordPoint) -> CoordPoint {
        self.grid.snap(point)
    }

    /// Check if a point is on the grid.
    pub fn is_on_grid(&self, point: CoordPoint) -> bool {
        let snapped = self.grid.snap(point);
        snapped.x == point.x && snapped.y == point.y
    }

    /// Get all component bounds as rectangles for collision detection.
    pub fn get_all_bounds(&self, primitives: &[SchRecord]) -> Vec<CoordRect> {
        self.get_placed_components(primitives)
            .into_iter()
            .map(|c| c.bounds)
            .collect()
    }

    /// Transform a point from component-local to absolute coordinates.
    pub fn local_to_absolute(
        &self,
        local: CoordPoint,
        component_location: CoordPoint,
        orientation: Orientation,
    ) -> CoordPoint {
        let rotated = local.rotate(CoordPoint::ZERO, orientation.rotation_degrees());

        // Apply mirroring if needed
        let mirrored = if orientation.is_mirrored() {
            CoordPoint::new(-rotated.x, rotated.y)
        } else {
            rotated
        };

        // Translate to component location
        mirrored.translate(component_location.x, component_location.y)
    }

    /// Transform a point from absolute to component-local coordinates.
    pub fn absolute_to_local(
        &self,
        absolute: CoordPoint,
        component_location: CoordPoint,
        orientation: Orientation,
    ) -> CoordPoint {
        // Translate to origin
        let translated = absolute.translate(-component_location.x, -component_location.y);

        // Reverse mirroring if needed
        let unmirrored = if orientation.is_mirrored() {
            CoordPoint::new(-translated.x, translated.y)
        } else {
            translated
        };

        // Reverse rotation
        unmirrored.rotate(CoordPoint::ZERO, -orientation.rotation_degrees())
    }
}
