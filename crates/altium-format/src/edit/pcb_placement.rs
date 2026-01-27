//! PCB component placement engine for programmatic modification of Altium PCB documents.
//!
//! This module provides placement functionality including:
//! - Absolute and relative placement
//! - Alignment with board edges and other components
//! - Grid snapping
//! - Connected route detection

use std::collections::HashSet;

use crate::io::PcbDoc;
use crate::records::pcb::PcbRecord;
use crate::types::{Coord, CoordPoint, CoordRect, Layer};

use super::types::Grid;

/// Anchor point for relative placement.
#[derive(Debug, Clone)]
pub enum PlacementAnchor {
    /// Absolute position.
    Absolute(CoordPoint),
    /// Near another component with optional offset.
    NearComponent {
        designator: String,
        offset: CoordPoint,
    },
    /// Align X coordinate with another component.
    AlignX { designator: String, offset: Coord },
    /// Align Y coordinate with another component.
    AlignY { designator: String, offset: Coord },
    /// Align to board edge.
    BoardEdge { edge: BoardEdge, offset: Coord },
}

/// Board edge for alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardEdge {
    Left,
    Right,
    Top,
    Bottom,
}

impl BoardEdge {
    /// Parse from string.
    pub fn try_parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "left" | "left-edge" | "l" => Some(BoardEdge::Left),
            "right" | "right-edge" | "r" => Some(BoardEdge::Right),
            "top" | "top-edge" | "t" => Some(BoardEdge::Top),
            "bottom" | "bottom-edge" | "b" => Some(BoardEdge::Bottom),
            _ => None,
        }
    }
}

/// Result of checking for connected routes.
#[derive(Debug, Clone)]
pub struct ConnectedRoutes {
    /// Track indices that connect to the component.
    pub tracks: Vec<usize>,
    /// Via indices that connect to the component.
    pub vias: Vec<usize>,
    /// Net names involved.
    pub nets: HashSet<String>,
}

impl ConnectedRoutes {
    /// Check if any routes are connected.
    pub fn has_connections(&self) -> bool {
        !self.tracks.is_empty() || !self.vias.is_empty()
    }

    /// Get total count of connected primitives.
    pub fn count(&self) -> usize {
        self.tracks.len() + self.vias.len()
    }
}

/// Component position and orientation.
#[derive(Debug, Clone)]
pub struct ComponentPosition {
    pub x: Coord,
    pub y: Coord,
    pub rotation: f64,
    pub layer: Layer,
}

impl Default for ComponentPosition {
    fn default() -> Self {
        Self {
            x: Coord::ZERO,
            y: Coord::ZERO,
            rotation: 0.0,
            layer: Layer::TOP_LAYER,
        }
    }
}

/// PCB placement engine for managing component placement.
pub struct PcbPlacementEngine {
    /// Grid configuration.
    grid: Grid,
    /// Board bounds.
    board_bounds: Option<CoordRect>,
}

impl Default for PcbPlacementEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PcbPlacementEngine {
    /// Create a new placement engine.
    pub fn new() -> Self {
        Self {
            grid: Grid::default(),
            board_bounds: None,
        }
    }

    /// Set the grid configuration.
    pub fn set_grid(&mut self, grid: Grid) {
        self.grid = grid;
    }

    /// Set the grid from a spacing value in mm.
    pub fn set_grid_mm(&mut self, spacing_mm: f64) {
        self.grid = Grid {
            spacing: Coord::from_mms(spacing_mm),
            snap_enabled: true,
        };
    }

    /// Get the grid configuration.
    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    /// Set board bounds for edge alignment.
    pub fn set_board_bounds(&mut self, bounds: CoordRect) {
        self.board_bounds = Some(bounds);
    }

    /// Calculate board bounds from the document.
    pub fn calculate_board_bounds(&mut self, pcb: &PcbDoc) {
        // Try to get board outline from primitives or use component bounds
        let mut bounds = CoordRect::EMPTY;

        for component in &pcb.components {
            if let Some(pos) = Self::get_component_position_static(component) {
                let _point = CoordPoint::new(pos.x, pos.y);
                if bounds.is_empty() {
                    bounds = CoordRect::from_xywh(pos.x, pos.y, Coord::ZERO, Coord::ZERO);
                } else {
                    bounds =
                        bounds.union(CoordRect::from_xywh(pos.x, pos.y, Coord::ZERO, Coord::ZERO));
                }
            }
        }

        // Add some margin
        if !bounds.is_empty() {
            let margin = Coord::from_mils(500.0);
            bounds = CoordRect::from_points(
                bounds.location1.x - margin,
                bounds.location1.y - margin,
                bounds.location2.x + margin,
                bounds.location2.y + margin,
            );
        }

        self.board_bounds = Some(bounds);
    }

    /// Snap a point to the grid.
    pub fn snap_to_grid(&self, point: CoordPoint) -> CoordPoint {
        self.grid.snap(point)
    }

    /// Get the position of a component.
    pub fn get_component_position(
        &self,
        pcb: &PcbDoc,
        designator: &str,
    ) -> Option<ComponentPosition> {
        pcb.components
            .iter()
            .find(|c| c.designator.eq_ignore_ascii_case(designator))
            .and_then(Self::get_component_position_static)
    }

    /// Get component position from a component reference.
    fn get_component_position_static(
        component: &crate::io::PcbDocComponent,
    ) -> Option<ComponentPosition> {
        let x = component.params.get("X")?.as_coord_or(Coord::ZERO);
        let y = component.params.get("Y")?.as_coord_or(Coord::ZERO);
        let rotation = component
            .params
            .get("ROTATION")
            .and_then(|v| v.as_str().parse::<f64>().ok())
            .unwrap_or(0.0);
        let layer = component
            .params
            .get("LAYER")
            .and_then(|v| Layer::from_name(v.as_str()))
            .unwrap_or(Layer::TOP_LAYER);

        Some(ComponentPosition {
            x,
            y,
            rotation,
            layer,
        })
    }

    /// Resolve a placement anchor to an absolute position.
    pub fn resolve_anchor(
        &self,
        pcb: &PcbDoc,
        anchor: &PlacementAnchor,
        current_pos: Option<&ComponentPosition>,
    ) -> Result<CoordPoint, String> {
        match anchor {
            PlacementAnchor::Absolute(point) => Ok(self.grid.snap(*point)),
            PlacementAnchor::NearComponent { designator, offset } => {
                let ref_pos = self
                    .get_component_position(pcb, designator)
                    .ok_or_else(|| format!("Component '{}' not found", designator))?;
                let point = CoordPoint::new(ref_pos.x + offset.x, ref_pos.y + offset.y);
                Ok(self.grid.snap(point))
            }
            PlacementAnchor::AlignX { designator, offset } => {
                let ref_pos = self
                    .get_component_position(pcb, designator)
                    .ok_or_else(|| format!("Component '{}' not found", designator))?;
                let current_y = current_pos.map(|p| p.y).unwrap_or(Coord::ZERO);
                let point = CoordPoint::new(ref_pos.x + *offset, current_y);
                Ok(self.grid.snap(point))
            }
            PlacementAnchor::AlignY { designator, offset } => {
                let ref_pos = self
                    .get_component_position(pcb, designator)
                    .ok_or_else(|| format!("Component '{}' not found", designator))?;
                let current_x = current_pos.map(|p| p.x).unwrap_or(Coord::ZERO);
                let point = CoordPoint::new(current_x, ref_pos.y + *offset);
                Ok(self.grid.snap(point))
            }
            PlacementAnchor::BoardEdge { edge, offset } => {
                let bounds = self
                    .board_bounds
                    .ok_or_else(|| "Board bounds not set".to_string())?;
                let current_x = current_pos.map(|p| p.x).unwrap_or(bounds.center().x);
                let current_y = current_pos.map(|p| p.y).unwrap_or(bounds.center().y);

                let point = match edge {
                    BoardEdge::Left => CoordPoint::new(bounds.location1.x + *offset, current_y),
                    BoardEdge::Right => CoordPoint::new(bounds.location2.x - *offset, current_y),
                    BoardEdge::Top => CoordPoint::new(current_x, bounds.location2.y - *offset),
                    BoardEdge::Bottom => CoordPoint::new(current_x, bounds.location1.y + *offset),
                };
                Ok(self.grid.snap(point))
            }
        }
    }

    /// Check if a component has connected routes (tracks/vias touching its pads).
    pub fn find_connected_routes(&self, pcb: &PcbDoc, designator: &str) -> ConnectedRoutes {
        let mut result = ConnectedRoutes {
            tracks: Vec::new(),
            vias: Vec::new(),
            nets: HashSet::new(),
        };

        // Find the component
        let component = match pcb
            .components
            .iter()
            .find(|c| c.designator.eq_ignore_ascii_case(designator))
        {
            Some(c) => c,
            None => return result,
        };

        // Get component position and bounds
        let comp_pos = match Self::get_component_position_static(component) {
            Some(p) => p,
            None => return result,
        };

        // Get pad locations from the component
        // For now, we'll estimate pad locations based on component position
        // In a full implementation, we'd read the actual pad data
        let pad_locations = self.get_component_pad_locations(pcb, component, &comp_pos);

        // Tolerance for connection detection (within 1 mil)
        let tolerance = Coord::from_mils(1.0);

        // Check each track
        for (i, primitive) in pcb.primitives.iter().enumerate() {
            if let PcbRecord::Track(track) = primitive {
                for pad_loc in &pad_locations {
                    if self.point_near_point(track.start, *pad_loc, tolerance)
                        || self.point_near_point(track.end, *pad_loc, tolerance)
                    {
                        result.tracks.push(i);
                        break;
                    }
                }
            }
        }

        // Check each via
        for (i, primitive) in pcb.primitives.iter().enumerate() {
            if let PcbRecord::Via(via) = primitive {
                for pad_loc in &pad_locations {
                    if self.point_near_point(via.location, *pad_loc, tolerance) {
                        result.vias.push(i);
                        break;
                    }
                }
            }
        }

        result
    }

    /// Get estimated pad locations for a component.
    fn get_component_pad_locations(
        &self,
        _pcb: &PcbDoc,
        _component: &crate::io::PcbDocComponent,
        comp_pos: &ComponentPosition,
    ) -> Vec<CoordPoint> {
        // For now, return the component center
        // A full implementation would read actual pad positions
        vec![CoordPoint::new(comp_pos.x, comp_pos.y)]
    }

    /// Check if two points are within tolerance.
    fn point_near_point(&self, p1: CoordPoint, p2: CoordPoint, tolerance: Coord) -> bool {
        let dx = (p1.x - p2.x).abs();
        let dy = (p1.y - p2.y).abs();
        dx.to_raw() <= tolerance.to_raw() && dy.to_raw() <= tolerance.to_raw()
    }

    /// List all components with their positions.
    pub fn list_components(&self, pcb: &PcbDoc) -> Vec<(String, ComponentPosition)> {
        pcb.components
            .iter()
            .filter_map(|c| {
                Self::get_component_position_static(c).map(|pos| (c.designator.clone(), pos))
            })
            .collect()
    }

    /// Find a component by designator.
    pub fn find_component<'a>(
        &self,
        pcb: &'a PcbDoc,
        designator: &str,
    ) -> Option<&'a crate::io::PcbDocComponent> {
        pcb.components
            .iter()
            .find(|c| c.designator.eq_ignore_ascii_case(designator))
    }
}

/// Parse a position string like "20mm,30mm" or "100mil,200mil".
pub fn parse_position(s: &str) -> Result<CoordPoint, String> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 2 {
        return Err(format!(
            "Invalid position format: '{}'. Expected 'X,Y' (e.g., '20mm,30mm')",
            s
        ));
    }

    let x = parse_coord(parts[0].trim())?;
    let y = parse_coord(parts[1].trim())?;

    Ok(CoordPoint::new(x, y))
}

/// Parse a coordinate string like "20mm" or "100mil".
pub fn parse_coord(s: &str) -> Result<Coord, String> {
    let s = s.trim().to_lowercase();

    if s.ends_with("mm") {
        let val: f64 = s
            .trim_end_matches("mm")
            .trim()
            .parse()
            .map_err(|_| format!("Invalid coordinate: {}", s))?;
        Ok(Coord::from_mms(val))
    } else if s.ends_with("mil") {
        let val: f64 = s
            .trim_end_matches("mil")
            .trim()
            .parse()
            .map_err(|_| format!("Invalid coordinate: {}", s))?;
        Ok(Coord::from_mils(val))
    } else if s.ends_with("in") {
        let val: f64 = s
            .trim_end_matches("in")
            .trim()
            .parse()
            .map_err(|_| format!("Invalid coordinate: {}", s))?;
        Ok(Coord::from_inches(val))
    } else {
        // Try parsing as mils by default
        let val: f64 = s
            .parse()
            .map_err(|_| format!("Invalid coordinate: {} (use '10mm', '100mil', etc.)", s))?;
        Ok(Coord::from_mils(val))
    }
}

/// Parse an offset string like "5mm,0mm" or just "5mm" (for single axis).
pub fn parse_offset(s: &str) -> Result<CoordPoint, String> {
    if s.contains(',') {
        parse_position(s)
    } else {
        // Single value - used for align-x/align-y offsets
        let coord = parse_coord(s)?;
        Ok(CoordPoint::new(coord, Coord::ZERO))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_coord() {
        assert!((parse_coord("10mm").unwrap().to_mms() - 10.0).abs() < 0.001);
        assert!((parse_coord("100mil").unwrap().to_mils() - 100.0).abs() < 0.001);
        assert!((parse_coord("1in").unwrap().to_mils() - 1000.0).abs() < 0.001);
        assert!((parse_coord("50").unwrap().to_mils() - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_parse_position() {
        let pos = parse_position("10mm,20mm").unwrap();
        assert!((pos.x.to_mms() - 10.0).abs() < 0.001);
        assert!((pos.y.to_mms() - 20.0).abs() < 0.001);
    }

    #[test]
    fn test_board_edge_parse() {
        assert_eq!(BoardEdge::try_parse("left"), Some(BoardEdge::Left));
        assert_eq!(BoardEdge::try_parse("left-edge"), Some(BoardEdge::Left));
        assert_eq!(BoardEdge::try_parse("TOP"), Some(BoardEdge::Top));
        assert_eq!(BoardEdge::try_parse("invalid"), None);
    }
}
