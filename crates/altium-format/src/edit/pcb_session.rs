//! PCB edit session for programmatic modification of Altium PCB documents.
//!
//! This module provides a comprehensive editing system for PCB documents including:
//! - Adding/modifying/deleting tracks, vias, pads, arcs, fills, text, and regions
//! - Relative positioning with support for multiple units (mm, mil, inch)
//! - Net assignment and management
//! - Grid snapping and board bounds awareness
//! - Undo/redo support

use std::path::Path;

use crate::error::{AltiumError, Result};
use crate::io::PcbDoc;
use crate::records::pcb::{
    HatchStyle, PcbArc, PcbFill, PcbFlags, PcbPad, PcbPadShape, PcbPolygon, PcbPrimitiveCommon,
    PcbRecord, PcbRectangularBase, PcbRegion, PcbText, PcbTextJustification, PcbTrack, PcbVia,
    PolygonVertex, PolygonVertexKind,
};
use crate::types::{Coord, CoordPoint, CoordRect, Layer, ParameterCollection};

use super::pcb_placement::{BoardEdge, PcbPlacementEngine, PlacementAnchor};
use super::types::Grid;

/// Position specification for PCB primitives.
/// Supports absolute coordinates and relative positioning.
#[derive(Debug, Clone)]
pub enum Position {
    /// Absolute position with X,Y coordinates.
    Absolute(CoordPoint),
    /// Relative to another primitive by index.
    RelativeTo {
        reference_index: usize,
        offset: CoordPoint,
    },
    /// Relative to a component by designator.
    RelativeToComponent {
        designator: String,
        offset: CoordPoint,
    },
    /// Relative to a pad on a component.
    RelativeToPad {
        component: String,
        pad: String,
        offset: CoordPoint,
    },
    /// Relative to board edge.
    RelativeToEdge { edge: BoardEdge, offset: CoordPoint },
    /// Center of board.
    BoardCenter { offset: CoordPoint },
}

impl Position {
    /// Create an absolute position from coordinates.
    pub fn absolute(x: Coord, y: Coord) -> Self {
        Position::Absolute(CoordPoint::new(x, y))
    }

    /// Create an absolute position from a point.
    pub fn at(point: CoordPoint) -> Self {
        Position::Absolute(point)
    }

    /// Create a position relative to a component.
    pub fn relative_to_component(designator: &str, dx: Coord, dy: Coord) -> Self {
        Position::RelativeToComponent {
            designator: designator.to_string(),
            offset: CoordPoint::new(dx, dy),
        }
    }

    /// Create a position relative to a component pad.
    pub fn relative_to_pad(component: &str, pad: &str, dx: Coord, dy: Coord) -> Self {
        Position::RelativeToPad {
            component: component.to_string(),
            pad: pad.to_string(),
            offset: CoordPoint::new(dx, dy),
        }
    }

    /// Create a position relative to a board edge.
    pub fn from_edge(edge: BoardEdge, dx: Coord, dy: Coord) -> Self {
        Position::RelativeToEdge {
            edge,
            offset: CoordPoint::new(dx, dy),
        }
    }
}

/// Track path specification with support for various routing styles.
#[derive(Debug, Clone)]
pub enum TrackPath {
    /// Direct line from start to end.
    Direct { start: Position, end: Position },
    /// Manhattan routing (90-degree angles only).
    Manhattan {
        start: Position,
        end: Position,
        prefer_horizontal_first: bool,
    },
    /// Diagonal routing (45-degree angles).
    Diagonal45 { start: Position, end: Position },
    /// Explicit list of vertices.
    Vertices(Vec<Position>),
}

/// Edit operation for undo/redo support.
///
/// Large variants (DeletePrimitive, ModifyPrimitive) box their PcbRecord fields
/// to reduce enum size on the stack.
#[derive(Debug, Clone)]
pub enum PcbEditOperation {
    AddTrack {
        index: usize,
    },
    AddVia {
        index: usize,
    },
    AddPad {
        index: usize,
    },
    AddArc {
        index: usize,
    },
    AddFill {
        index: usize,
    },
    AddText {
        index: usize,
    },
    AddRegion {
        index: usize,
    },
    AddPolygon {
        index: usize,
    },
    DeletePrimitive {
        index: usize,
        record: Box<PcbRecord>,
    },
    ModifyPrimitive {
        index: usize,
        old: Box<PcbRecord>,
        new: Box<PcbRecord>,
    },
    MoveComponent {
        designator: String,
        old_pos: CoordPoint,
        new_pos: CoordPoint,
    },
}

/// PCB editing session for comprehensive document modification.
pub struct PcbEditSession {
    /// The PCB document being edited.
    pub doc: PcbDoc,
    /// Placement engine for positioning.
    placement: PcbPlacementEngine,
    /// Operation history for undo.
    history: Vec<PcbEditOperation>,
    /// Redo stack.
    redo_stack: Vec<PcbEditOperation>,
    /// Path to the source file.
    source_path: Option<String>,
    /// Whether the document has been modified.
    modified: bool,
    /// Default track width.
    default_track_width: Coord,
    /// Default via diameter.
    default_via_diameter: Coord,
    /// Default via hole size.
    default_via_hole: Coord,
    /// Default layer for new primitives.
    default_layer: Layer,
}

impl Default for PcbEditSession {
    fn default() -> Self {
        Self::new()
    }
}

impl PcbEditSession {
    /// Create a new empty editing session.
    pub fn new() -> Self {
        Self {
            doc: PcbDoc::default(),
            placement: PcbPlacementEngine::new(),
            history: Vec::new(),
            redo_stack: Vec::new(),
            source_path: None,
            modified: false,
            default_track_width: Coord::from_mils(10.0),
            default_via_diameter: Coord::from_mils(50.0),
            default_via_hole: Coord::from_mils(28.0),
            default_layer: Layer::TOP_LAYER,
        }
    }

    /// Open an existing PCB file for editing.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let doc = PcbDoc::open_file(&path)?;

        let mut session = Self::new();
        session.doc = doc;
        session.source_path = Some(path_str);
        session.placement.calculate_board_bounds(&session.doc);

        Ok(session)
    }

    /// Save the PCB document to a file.
    pub fn save<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        self.doc.save_all_to_file(&path)?;
        self.source_path = Some(path.as_ref().to_string_lossy().to_string());
        self.modified = false;
        Ok(())
    }

    /// Save to the original file (if opened from file).
    pub fn save_to_original(&mut self) -> Result<()> {
        match &self.source_path {
            Some(path) => {
                self.doc.save_all_to_file(path)?;
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

    /// Set the grid configuration.
    pub fn set_grid(&mut self, grid: Grid) {
        self.placement.set_grid(grid);
    }

    /// Set grid from spacing in mm.
    pub fn set_grid_mm(&mut self, spacing_mm: f64) {
        self.placement.set_grid_mm(spacing_mm);
    }

    /// Set the default track width.
    pub fn set_default_track_width(&mut self, width: Coord) {
        self.default_track_width = width;
    }

    /// Set the default via diameter.
    pub fn set_default_via_diameter(&mut self, diameter: Coord) {
        self.default_via_diameter = diameter;
    }

    /// Set the default via hole size.
    pub fn set_default_via_hole(&mut self, hole: Coord) {
        self.default_via_hole = hole;
    }

    /// Set the default layer for new primitives.
    pub fn set_default_layer(&mut self, layer: Layer) {
        self.default_layer = layer;
    }

    /// Get the default layer.
    pub fn default_layer(&self) -> Layer {
        self.default_layer
    }

    // ═══════════════════════════════════════════════════════════════════════
    // POSITION RESOLUTION
    // ═══════════════════════════════════════════════════════════════════════

    /// Resolve a Position to an absolute CoordPoint.
    pub fn resolve_position(&self, pos: &Position) -> Result<CoordPoint> {
        match pos {
            Position::Absolute(point) => Ok(self.placement.snap_to_grid(*point)),

            Position::RelativeTo {
                reference_index,
                offset,
            } => {
                if *reference_index >= self.doc.primitives.len() {
                    return Err(AltiumError::Parse(format!(
                        "Invalid primitive index: {}",
                        reference_index
                    )));
                }
                let ref_pos = self.get_primitive_center(*reference_index)?;
                Ok(self
                    .placement
                    .snap_to_grid(CoordPoint::new(ref_pos.x + offset.x, ref_pos.y + offset.y)))
            }

            Position::RelativeToComponent { designator, offset } => {
                let comp_pos = self
                    .placement
                    .get_component_position(&self.doc, designator)
                    .ok_or_else(|| {
                        AltiumError::Parse(format!("Component '{}' not found", designator))
                    })?;
                Ok(self.placement.snap_to_grid(CoordPoint::new(
                    comp_pos.x + offset.x,
                    comp_pos.y + offset.y,
                )))
            }

            Position::RelativeToPad {
                component,
                pad,
                offset,
            } => {
                let pad_pos = self.get_pad_position(component, pad)?;
                Ok(self
                    .placement
                    .snap_to_grid(CoordPoint::new(pad_pos.x + offset.x, pad_pos.y + offset.y)))
            }

            Position::RelativeToEdge { edge, offset } => {
                let anchor = PlacementAnchor::BoardEdge {
                    edge: *edge,
                    offset: Coord::ZERO,
                };
                let base = self
                    .placement
                    .resolve_anchor(&self.doc, &anchor, None)
                    .map_err(AltiumError::Parse)?;
                Ok(self
                    .placement
                    .snap_to_grid(CoordPoint::new(base.x + offset.x, base.y + offset.y)))
            }

            Position::BoardCenter { offset } => {
                let bounds = self.get_board_bounds();
                let center = bounds.center();
                Ok(self
                    .placement
                    .snap_to_grid(CoordPoint::new(center.x + offset.x, center.y + offset.y)))
            }
        }
    }

    /// Get the center point of a primitive.
    fn get_primitive_center(&self, index: usize) -> Result<CoordPoint> {
        let prim = &self.doc.primitives[index];
        let center = match prim {
            PcbRecord::Track(t) => CoordPoint::new(
                Coord::from_raw((t.start.x.to_raw() + t.end.x.to_raw()) / 2),
                Coord::from_raw((t.start.y.to_raw() + t.end.y.to_raw()) / 2),
            ),
            PcbRecord::Via(v) => v.location,
            PcbRecord::Pad(p) => p.location,
            PcbRecord::Arc(a) => a.location,
            PcbRecord::Fill(f) => f.base.calculate_bounds().center(),
            PcbRecord::Text(t) => t.base.calculate_bounds().center(),
            PcbRecord::Region(r) => {
                if r.outline.is_empty() {
                    CoordPoint::default()
                } else {
                    let sum_x: i64 = r.outline.iter().map(|p| p.x.to_raw() as i64).sum();
                    let sum_y: i64 = r.outline.iter().map(|p| p.y.to_raw() as i64).sum();
                    let n = r.outline.len() as i64;
                    CoordPoint::new(
                        Coord::from_raw((sum_x / n) as i32),
                        Coord::from_raw((sum_y / n) as i32),
                    )
                }
            }
            PcbRecord::Polygon(p) => {
                if p.vertices.is_empty() {
                    CoordPoint::default()
                } else {
                    let sum_x: i64 = p.vertices.iter().map(|v| v.x.to_raw() as i64).sum();
                    let sum_y: i64 = p.vertices.iter().map(|v| v.y.to_raw() as i64).sum();
                    let n = p.vertices.len() as i64;
                    CoordPoint::new(
                        Coord::from_raw((sum_x / n) as i32),
                        Coord::from_raw((sum_y / n) as i32),
                    )
                }
            }
            _ => CoordPoint::default(),
        };
        Ok(center)
    }

    /// Get the position of a pad on a component.
    fn get_pad_position(&self, component: &str, pad_designator: &str) -> Result<CoordPoint> {
        // Find the component
        let comp = self
            .doc
            .find_component(component)
            .ok_or_else(|| AltiumError::Parse(format!("Component '{}' not found", component)))?;

        // Get component position
        let comp_x = comp.x().unwrap_or(Coord::ZERO);
        let comp_y = comp.y().unwrap_or(Coord::ZERO);
        let _rotation = comp.rotation();

        // Find the pad in the component's primitives
        for prim in &comp.primitives {
            if let PcbRecord::Pad(p) = prim {
                if p.designator.eq_ignore_ascii_case(pad_designator) {
                    // Pad location is relative to component, need to transform
                    return Ok(CoordPoint::new(
                        comp_x + p.location.x,
                        comp_y + p.location.y,
                    ));
                }
            }
        }

        Err(AltiumError::Parse(format!(
            "Pad '{}' not found on component '{}'",
            pad_designator, component
        )))
    }

    /// Get the board bounds.
    pub fn get_board_bounds(&self) -> CoordRect {
        // Try to calculate from board outline or use default
        let mut bounds = CoordRect::EMPTY;

        for prim in &self.doc.primitives {
            match prim {
                PcbRecord::Track(t) => {
                    bounds = bounds.union(t.calculate_bounds());
                }
                PcbRecord::Via(v) => {
                    bounds = bounds.union(v.calculate_bounds());
                }
                PcbRecord::Region(r) if !r.outline.is_empty() => {
                    for point in &r.outline {
                        if bounds.is_empty() {
                            bounds =
                                CoordRect::from_xywh(point.x, point.y, Coord::ZERO, Coord::ZERO);
                        } else {
                            bounds = bounds.union(CoordRect::from_xywh(
                                point.x,
                                point.y,
                                Coord::ZERO,
                                Coord::ZERO,
                            ));
                        }
                    }
                }
                _ => {}
            }
        }

        if bounds.is_empty() {
            // Default board size
            CoordRect::from_xywh(
                Coord::ZERO,
                Coord::ZERO,
                Coord::from_mils(4000.0),
                Coord::from_mils(3000.0),
            )
        } else {
            bounds
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // TRACK OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════

    /// Add a single track segment.
    pub fn add_track(
        &mut self,
        start: CoordPoint,
        end: CoordPoint,
        width: Option<Coord>,
        layer: Option<Layer>,
        net: Option<&str>,
    ) -> Result<usize> {
        let track = PcbTrack {
            common: PcbPrimitiveCommon {
                layer: layer.unwrap_or(self.default_layer),
                flags: PcbFlags::default(),
                unique_id: None,
            },
            start: self.placement.snap_to_grid(start),
            end: self.placement.snap_to_grid(end),
            width: width.unwrap_or(self.default_track_width),
            unknown: vec![0u8; 16],
        };

        let index = self.doc.primitives.len();
        self.doc.primitives.push(PcbRecord::Track(track));

        // Add to net if specified
        if let Some(net_name) = net {
            self.ensure_net_exists(net_name);
        }

        self.history.push(PcbEditOperation::AddTrack { index });
        self.redo_stack.clear();
        self.modified = true;

        Ok(index)
    }

    /// Add a track using Position specifications.
    pub fn add_track_positioned(
        &mut self,
        start: &Position,
        end: &Position,
        width: Option<Coord>,
        layer: Option<Layer>,
        net: Option<&str>,
    ) -> Result<usize> {
        let start_point = self.resolve_position(start)?;
        let end_point = self.resolve_position(end)?;
        self.add_track(start_point, end_point, width, layer, net)
    }

    /// Add multiple connected track segments forming a path.
    pub fn add_track_path(
        &mut self,
        vertices: &[CoordPoint],
        width: Option<Coord>,
        layer: Option<Layer>,
        net: Option<&str>,
    ) -> Result<Vec<usize>> {
        if vertices.len() < 2 {
            return Err(AltiumError::Parse(
                "Track path requires at least 2 vertices".to_string(),
            ));
        }

        let mut indices = Vec::new();
        for i in 0..vertices.len() - 1 {
            let idx = self.add_track(vertices[i], vertices[i + 1], width, layer, net)?;
            indices.push(idx);
        }
        Ok(indices)
    }

    /// Add a track path using the TrackPath specification.
    pub fn add_track_routed(
        &mut self,
        path: &TrackPath,
        width: Option<Coord>,
        layer: Option<Layer>,
        net: Option<&str>,
    ) -> Result<Vec<usize>> {
        match path {
            TrackPath::Direct { start, end } => {
                let idx = self.add_track_positioned(start, end, width, layer, net)?;
                Ok(vec![idx])
            }

            TrackPath::Manhattan {
                start,
                end,
                prefer_horizontal_first,
            } => {
                let start_pt = self.resolve_position(start)?;
                let end_pt = self.resolve_position(end)?;

                let mid = if *prefer_horizontal_first {
                    CoordPoint::new(end_pt.x, start_pt.y)
                } else {
                    CoordPoint::new(start_pt.x, end_pt.y)
                };

                self.add_track_path(&[start_pt, mid, end_pt], width, layer, net)
            }

            TrackPath::Diagonal45 { start, end } => {
                let start_pt = self.resolve_position(start)?;
                let end_pt = self.resolve_position(end)?;

                let dx = (end_pt.x.to_raw() - start_pt.x.to_raw()).abs();
                let dy = (end_pt.y.to_raw() - start_pt.y.to_raw()).abs();
                let diag = dx.min(dy);

                let mid = if dx > dy {
                    // More horizontal, go diagonal first then horizontal
                    let sign_x = if end_pt.x > start_pt.x { 1 } else { -1 };
                    let sign_y = if end_pt.y > start_pt.y { 1 } else { -1 };
                    CoordPoint::new(
                        Coord::from_raw(start_pt.x.to_raw() + sign_x * diag),
                        Coord::from_raw(start_pt.y.to_raw() + sign_y * diag),
                    )
                } else {
                    // More vertical, go diagonal first then vertical
                    let sign_x = if end_pt.x > start_pt.x { 1 } else { -1 };
                    let sign_y = if end_pt.y > start_pt.y { 1 } else { -1 };
                    CoordPoint::new(
                        Coord::from_raw(start_pt.x.to_raw() + sign_x * diag),
                        Coord::from_raw(start_pt.y.to_raw() + sign_y * diag),
                    )
                };

                self.add_track_path(&[start_pt, mid, end_pt], width, layer, net)
            }

            TrackPath::Vertices(positions) => {
                let mut points = Vec::new();
                for pos in positions {
                    points.push(self.resolve_position(pos)?);
                }
                self.add_track_path(&points, width, layer, net)
            }
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // VIA OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════

    /// Add a via at a specific location.
    pub fn add_via(
        &mut self,
        location: CoordPoint,
        diameter: Option<Coord>,
        hole_size: Option<Coord>,
        from_layer: Option<Layer>,
        to_layer: Option<Layer>,
        net: Option<&str>,
    ) -> Result<usize> {
        let dia = diameter.unwrap_or(self.default_via_diameter);
        let mut via = PcbVia::default();
        via.common.layer = Layer::MULTI_LAYER;
        via.location = self.placement.snap_to_grid(location);
        via.hole_size = hole_size.unwrap_or(self.default_via_hole);
        via.from_layer = from_layer.unwrap_or(Layer::TOP_LAYER);
        via.to_layer = to_layer.unwrap_or(Layer::BOTTOM_LAYER);

        // Set all diameters to the same value for simple stack mode
        for d in via.diameters.iter_mut() {
            *d = dia;
        }

        let index = self.doc.primitives.len();
        self.doc.primitives.push(PcbRecord::Via(via));

        if let Some(net_name) = net {
            self.ensure_net_exists(net_name);
        }

        self.history.push(PcbEditOperation::AddVia { index });
        self.redo_stack.clear();
        self.modified = true;

        Ok(index)
    }

    /// Add a via using Position specification.
    pub fn add_via_positioned(
        &mut self,
        position: &Position,
        diameter: Option<Coord>,
        hole_size: Option<Coord>,
        from_layer: Option<Layer>,
        to_layer: Option<Layer>,
        net: Option<&str>,
    ) -> Result<usize> {
        let location = self.resolve_position(position)?;
        self.add_via(location, diameter, hole_size, from_layer, to_layer, net)
    }

    /// Add a blind via (connects top/bottom to inner layer).
    pub fn add_blind_via(
        &mut self,
        location: CoordPoint,
        from_layer: Layer,
        to_layer: Layer,
        diameter: Option<Coord>,
        hole_size: Option<Coord>,
        net: Option<&str>,
    ) -> Result<usize> {
        self.add_via(
            location,
            diameter,
            hole_size,
            Some(from_layer),
            Some(to_layer),
            net,
        )
    }

    // ═══════════════════════════════════════════════════════════════════════
    // PAD OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════

    /// Add a through-hole pad.
    pub fn add_through_hole_pad(
        &mut self,
        location: CoordPoint,
        designator: &str,
        hole_size: Coord,
        pad_size: CoordPoint,
        shape: PcbPadShape,
        net: Option<&str>,
    ) -> Result<usize> {
        let mut pad = PcbPad::default();
        pad.common.layer = Layer::MULTI_LAYER;
        pad.location = self.placement.snap_to_grid(location);
        pad.designator = designator.to_string();
        pad.hole_size = hole_size;
        pad.is_plated = true;

        // Set size and shape for all layers
        for size in pad.size_layers.iter_mut() {
            *size = pad_size;
        }
        for s in pad.shape_layers.iter_mut() {
            *s = shape;
        }

        let index = self.doc.primitives.len();
        self.doc.primitives.push(PcbRecord::Pad(Box::new(pad)));

        if let Some(net_name) = net {
            self.ensure_net_exists(net_name);
        }

        self.history.push(PcbEditOperation::AddPad { index });
        self.redo_stack.clear();
        self.modified = true;

        Ok(index)
    }

    /// Add an SMD pad (surface mount).
    pub fn add_smd_pad(
        &mut self,
        location: CoordPoint,
        designator: &str,
        size: CoordPoint,
        shape: PcbPadShape,
        layer: Layer,
        net: Option<&str>,
    ) -> Result<usize> {
        let mut pad = PcbPad::default();
        pad.common.layer = layer;
        pad.location = self.placement.snap_to_grid(location);
        pad.designator = designator.to_string();
        pad.hole_size = Coord::ZERO; // No hole for SMD
        pad.is_plated = false;

        // Set size and shape for the specified layer
        let layer_idx = if layer == Layer::TOP_LAYER { 0 } else { 31 };
        pad.size_layers[layer_idx] = size;
        pad.shape_layers[layer_idx] = shape;

        let index = self.doc.primitives.len();
        self.doc.primitives.push(PcbRecord::Pad(Box::new(pad)));

        if let Some(net_name) = net {
            self.ensure_net_exists(net_name);
        }

        self.history.push(PcbEditOperation::AddPad { index });
        self.redo_stack.clear();
        self.modified = true;

        Ok(index)
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ARC OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════

    /// Add an arc.
    #[allow(clippy::too_many_arguments)]
    pub fn add_arc(
        &mut self,
        center: CoordPoint,
        radius: Coord,
        start_angle: f64,
        end_angle: f64,
        width: Option<Coord>,
        layer: Option<Layer>,
        net: Option<&str>,
    ) -> Result<usize> {
        let arc = PcbArc {
            common: PcbPrimitiveCommon {
                layer: layer.unwrap_or(self.default_layer),
                flags: PcbFlags::default(),
                unique_id: None,
            },
            location: self.placement.snap_to_grid(center),
            radius,
            start_angle,
            end_angle,
            width: width.unwrap_or(self.default_track_width),
        };

        let index = self.doc.primitives.len();
        self.doc.primitives.push(PcbRecord::Arc(arc));

        if let Some(net_name) = net {
            self.ensure_net_exists(net_name);
        }

        self.history.push(PcbEditOperation::AddArc { index });
        self.redo_stack.clear();
        self.modified = true;

        Ok(index)
    }

    /// Add a full circle arc.
    pub fn add_circle(
        &mut self,
        center: CoordPoint,
        radius: Coord,
        width: Option<Coord>,
        layer: Option<Layer>,
    ) -> Result<usize> {
        self.add_arc(center, radius, 0.0, 360.0, width, layer, None)
    }

    /// Add an arc using Position specification.
    #[allow(clippy::too_many_arguments)]
    pub fn add_arc_positioned(
        &mut self,
        center: &Position,
        radius: Coord,
        start_angle: f64,
        end_angle: f64,
        width: Option<Coord>,
        layer: Option<Layer>,
        net: Option<&str>,
    ) -> Result<usize> {
        let center_pt = self.resolve_position(center)?;
        self.add_arc(center_pt, radius, start_angle, end_angle, width, layer, net)
    }

    // ═══════════════════════════════════════════════════════════════════════
    // FILL OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════

    /// Add a solid fill (copper rectangle).
    pub fn add_fill(
        &mut self,
        corner1: CoordPoint,
        corner2: CoordPoint,
        layer: Option<Layer>,
        rotation: Option<f64>,
        net: Option<&str>,
    ) -> Result<usize> {
        let fill = PcbFill {
            base: PcbRectangularBase {
                common: PcbPrimitiveCommon {
                    layer: layer.unwrap_or(self.default_layer),
                    flags: PcbFlags::default(),
                    unique_id: None,
                },
                corner1: self.placement.snap_to_grid(corner1),
                corner2: self.placement.snap_to_grid(corner2),
                rotation: rotation.unwrap_or(0.0),
            },
        };

        let index = self.doc.primitives.len();
        self.doc.primitives.push(PcbRecord::Fill(fill));

        if let Some(net_name) = net {
            self.ensure_net_exists(net_name);
        }

        self.history.push(PcbEditOperation::AddFill { index });
        self.redo_stack.clear();
        self.modified = true;

        Ok(index)
    }

    /// Add a fill with Position specifications.
    pub fn add_fill_positioned(
        &mut self,
        corner1: &Position,
        corner2: &Position,
        layer: Option<Layer>,
        rotation: Option<f64>,
        net: Option<&str>,
    ) -> Result<usize> {
        let c1 = self.resolve_position(corner1)?;
        let c2 = self.resolve_position(corner2)?;
        self.add_fill(c1, c2, layer, rotation, net)
    }

    // ═══════════════════════════════════════════════════════════════════════
    // TEXT OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════

    /// Add text annotation.
    pub fn add_text(
        &mut self,
        text: &str,
        location: CoordPoint,
        height: Coord,
        layer: Option<Layer>,
        rotation: Option<f64>,
        _justification: Option<PcbTextJustification>,
    ) -> Result<usize> {
        // Use the PcbText::new constructor for proper initialization
        let text_record = PcbText::new(
            location.x.to_mms(),
            location.y.to_mms(),
            text,
            height.to_mms(),
            0.15, // Default stroke width in mm
            rotation.unwrap_or(0.0),
            false, // not mirrored
            layer.unwrap_or(Layer::TOP_OVERLAY),
        );

        let index = self.doc.primitives.len();
        self.doc.primitives.push(PcbRecord::Text(text_record));

        self.history.push(PcbEditOperation::AddText { index });
        self.redo_stack.clear();
        self.modified = true;

        Ok(index)
    }

    /// Add text using Position specification.
    pub fn add_text_positioned(
        &mut self,
        text: &str,
        position: &Position,
        height: Coord,
        layer: Option<Layer>,
        rotation: Option<f64>,
        justification: Option<PcbTextJustification>,
    ) -> Result<usize> {
        let location = self.resolve_position(position)?;
        self.add_text(text, location, height, layer, rotation, justification)
    }

    // ═══════════════════════════════════════════════════════════════════════
    // REGION OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════

    /// Add a region (copper area or keepout).
    pub fn add_region(
        &mut self,
        vertices: &[CoordPoint],
        layer: Layer,
        is_keepout: bool,
        net: Option<&str>,
    ) -> Result<usize> {
        if vertices.len() < 3 {
            return Err(AltiumError::Parse(
                "Region requires at least 3 vertices".to_string(),
            ));
        }

        let mut flags = PcbFlags::default();
        if is_keepout {
            flags |= PcbFlags::KEEPOUT;
        }

        let region = PcbRegion {
            common: PcbPrimitiveCommon {
                layer,
                flags,
                unique_id: None,
            },
            parameters: ParameterCollection::new(),
            outline: vertices
                .iter()
                .map(|v| self.placement.snap_to_grid(*v))
                .collect(),
        };

        let index = self.doc.primitives.len();
        self.doc.primitives.push(PcbRecord::Region(region));

        if let Some(net_name) = net {
            self.ensure_net_exists(net_name);
        }

        self.history.push(PcbEditOperation::AddRegion { index });
        self.redo_stack.clear();
        self.modified = true;

        Ok(index)
    }

    /// Add a rectangular region.
    pub fn add_rectangular_region(
        &mut self,
        corner1: CoordPoint,
        corner2: CoordPoint,
        layer: Layer,
        is_keepout: bool,
        net: Option<&str>,
    ) -> Result<usize> {
        let vertices = vec![
            corner1,
            CoordPoint::new(corner2.x, corner1.y),
            corner2,
            CoordPoint::new(corner1.x, corner2.y),
        ];
        self.add_region(&vertices, layer, is_keepout, net)
    }

    // ═══════════════════════════════════════════════════════════════════════
    // POLYGON (COPPER POUR) OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════

    /// Add a polygon (copper pour).
    pub fn add_polygon(
        &mut self,
        vertices: &[CoordPoint],
        layer: Layer,
        net_name: &str,
        hatch_style: HatchStyle,
        pour_over_same_net: bool,
        remove_dead_copper: bool,
    ) -> Result<usize> {
        if vertices.len() < 3 {
            return Err(AltiumError::Parse(
                "Polygon requires at least 3 vertices".to_string(),
            ));
        }

        let mut polygon = PcbPolygon {
            layer,
            net_name: net_name.to_string(),
            hatch_style,
            pour_over: pour_over_same_net,
            remove_dead: remove_dead_copper,
            ..Default::default()
        };

        for vertex in vertices.iter() {
            let snapped = self.placement.snap_to_grid(*vertex);
            polygon.vertices.push(PolygonVertex {
                kind: PolygonVertexKind::Line,
                x: snapped.x,
                y: snapped.y,
                center_x: Coord::ZERO,
                center_y: Coord::ZERO,
                start_angle: 0.0,
                end_angle: 0.0,
                radius: Coord::ZERO,
            });
        }

        self.ensure_net_exists(net_name);

        let index = self.doc.primitives.len();
        self.doc.primitives.push(PcbRecord::Polygon(polygon));

        self.history.push(PcbEditOperation::AddPolygon { index });
        self.redo_stack.clear();
        self.modified = true;

        Ok(index)
    }

    /// Add a rectangular copper pour.
    pub fn add_rectangular_polygon(
        &mut self,
        corner1: CoordPoint,
        corner2: CoordPoint,
        layer: Layer,
        net_name: &str,
        hatch_style: HatchStyle,
    ) -> Result<usize> {
        let vertices = vec![
            corner1,
            CoordPoint::new(corner2.x, corner1.y),
            corner2,
            CoordPoint::new(corner1.x, corner2.y),
        ];
        self.add_polygon(&vertices, layer, net_name, hatch_style, true, true)
    }

    // ═══════════════════════════════════════════════════════════════════════
    // DELETE OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════

    /// Delete a primitive by index.
    pub fn delete_primitive(&mut self, index: usize) -> Result<()> {
        if index >= self.doc.primitives.len() {
            return Err(AltiumError::Parse(format!(
                "Invalid primitive index: {}",
                index
            )));
        }

        let record = self.doc.primitives.remove(index);
        self.history.push(PcbEditOperation::DeletePrimitive {
            index,
            record: Box::new(record),
        });
        self.redo_stack.clear();
        self.modified = true;

        Ok(())
    }

    /// Delete all primitives matching a filter.
    pub fn delete_primitives_where<F>(&mut self, filter: F) -> Result<usize>
    where
        F: Fn(&PcbRecord) -> bool,
    {
        let mut indices: Vec<usize> = self
            .doc
            .primitives
            .iter()
            .enumerate()
            .filter(|(_, p)| filter(p))
            .map(|(i, _)| i)
            .collect();

        // Delete in reverse order to maintain valid indices
        indices.sort();
        indices.reverse();

        let count = indices.len();
        for idx in indices {
            self.delete_primitive(idx)?;
        }

        Ok(count)
    }

    /// Delete all tracks on a layer.
    pub fn delete_tracks_on_layer(&mut self, layer: Layer) -> Result<usize> {
        self.delete_primitives_where(
            |p| matches!(p, PcbRecord::Track(t) if t.common.layer == layer),
        )
    }

    /// Delete all vias.
    pub fn delete_all_vias(&mut self) -> Result<usize> {
        self.delete_primitives_where(|p| matches!(p, PcbRecord::Via(_)))
    }

    // ═══════════════════════════════════════════════════════════════════════
    // NET OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════

    /// Ensure a net exists in the document.
    pub fn ensure_net_exists(&mut self, net_name: &str) {
        if !self
            .doc
            .nets
            .iter()
            .any(|n| n.eq_ignore_ascii_case(net_name))
        {
            self.doc.nets.push(net_name.to_string());
        }
    }

    /// Add a new net.
    pub fn add_net(&mut self, net_name: &str) -> Result<()> {
        if self
            .doc
            .nets
            .iter()
            .any(|n| n.eq_ignore_ascii_case(net_name))
        {
            return Err(AltiumError::Parse(format!(
                "Net '{}' already exists",
                net_name
            )));
        }
        self.doc.nets.push(net_name.to_string());
        self.modified = true;
        Ok(())
    }

    /// Get all nets in the document.
    pub fn nets(&self) -> &[String] {
        &self.doc.nets
    }

    // ═══════════════════════════════════════════════════════════════════════
    // QUERY OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════

    /// Get all tracks.
    pub fn tracks(&self) -> impl Iterator<Item = (usize, &PcbTrack)> {
        self.doc.primitives.iter().enumerate().filter_map(|(i, p)| {
            if let PcbRecord::Track(t) = p {
                Some((i, t))
            } else {
                None
            }
        })
    }

    /// Get all vias.
    pub fn vias(&self) -> impl Iterator<Item = (usize, &PcbVia)> {
        self.doc.primitives.iter().enumerate().filter_map(|(i, p)| {
            if let PcbRecord::Via(v) = p {
                Some((i, v))
            } else {
                None
            }
        })
    }

    /// Get all pads.
    pub fn pads(&self) -> impl Iterator<Item = (usize, &PcbPad)> {
        self.doc.primitives.iter().enumerate().filter_map(|(i, p)| {
            if let PcbRecord::Pad(p) = p {
                Some((i, p.as_ref()))
            } else {
                None
            }
        })
    }

    /// Get all arcs.
    pub fn arcs(&self) -> impl Iterator<Item = (usize, &PcbArc)> {
        self.doc.primitives.iter().enumerate().filter_map(|(i, p)| {
            if let PcbRecord::Arc(a) = p {
                Some((i, a))
            } else {
                None
            }
        })
    }

    /// Get all fills.
    pub fn fills(&self) -> impl Iterator<Item = (usize, &PcbFill)> {
        self.doc.primitives.iter().enumerate().filter_map(|(i, p)| {
            if let PcbRecord::Fill(f) = p {
                Some((i, f))
            } else {
                None
            }
        })
    }

    /// Get all text annotations.
    pub fn texts(&self) -> impl Iterator<Item = (usize, &PcbText)> {
        self.doc.primitives.iter().enumerate().filter_map(|(i, p)| {
            if let PcbRecord::Text(t) = p {
                Some((i, t))
            } else {
                None
            }
        })
    }

    /// Get all regions.
    pub fn regions(&self) -> impl Iterator<Item = (usize, &PcbRegion)> {
        self.doc.primitives.iter().enumerate().filter_map(|(i, p)| {
            if let PcbRecord::Region(r) = p {
                Some((i, r))
            } else {
                None
            }
        })
    }

    /// Get all polygons.
    pub fn polygons(&self) -> impl Iterator<Item = (usize, &PcbPolygon)> {
        self.doc.primitives.iter().enumerate().filter_map(|(i, p)| {
            if let PcbRecord::Polygon(p) = p {
                Some((i, p))
            } else {
                None
            }
        })
    }

    /// Get tracks on a specific layer.
    pub fn tracks_on_layer(&self, layer: Layer) -> impl Iterator<Item = (usize, &PcbTrack)> {
        self.tracks().filter(move |(_, t)| t.common.layer == layer)
    }

    /// Count primitives by type.
    pub fn count_primitives(&self) -> PrimitiveCount {
        let mut count = PrimitiveCount::default();
        for p in &self.doc.primitives {
            match p {
                PcbRecord::Track(_) => count.tracks += 1,
                PcbRecord::Via(_) => count.vias += 1,
                PcbRecord::Pad(_) => count.pads += 1,
                PcbRecord::Arc(_) => count.arcs += 1,
                PcbRecord::Fill(_) => count.fills += 1,
                PcbRecord::Text(_) => count.texts += 1,
                PcbRecord::Region(_) => count.regions += 1,
                PcbRecord::Polygon(_) => count.polygons += 1,
                _ => count.other += 1,
            }
        }
        count
    }

    // ═══════════════════════════════════════════════════════════════════════
    // UNDO/REDO
    // ═══════════════════════════════════════════════════════════════════════

    /// Undo the last operation.
    pub fn undo(&mut self) -> Result<()> {
        let op = self
            .history
            .pop()
            .ok_or_else(|| AltiumError::Parse("Nothing to undo".to_string()))?;

        match &op {
            PcbEditOperation::AddTrack { index }
            | PcbEditOperation::AddVia { index }
            | PcbEditOperation::AddPad { index }
            | PcbEditOperation::AddArc { index }
            | PcbEditOperation::AddFill { index }
            | PcbEditOperation::AddText { index }
            | PcbEditOperation::AddRegion { index }
            | PcbEditOperation::AddPolygon { index } => {
                if *index < self.doc.primitives.len() {
                    self.doc.primitives.remove(*index);
                }
            }
            PcbEditOperation::DeletePrimitive { index, record } => {
                self.doc.primitives.insert(*index, (**record).clone());
            }
            PcbEditOperation::ModifyPrimitive { index, old, .. } => {
                if *index < self.doc.primitives.len() {
                    self.doc.primitives[*index] = (**old).clone();
                }
            }
            PcbEditOperation::MoveComponent {
                designator,
                old_pos,
                ..
            } => {
                if let Some(comp) = self.doc.find_component_mut(designator) {
                    comp.set_position(old_pos.x, old_pos.y);
                }
            }
        }

        self.redo_stack.push(op);
        self.modified = true;
        Ok(())
    }

    /// Redo the last undone operation.
    pub fn redo(&mut self) -> Result<()> {
        let op = self
            .redo_stack
            .pop()
            .ok_or_else(|| AltiumError::Parse("Nothing to redo".to_string()))?;

        match &op {
            PcbEditOperation::AddTrack { .. }
            | PcbEditOperation::AddVia { .. }
            | PcbEditOperation::AddPad { .. }
            | PcbEditOperation::AddArc { .. }
            | PcbEditOperation::AddFill { .. }
            | PcbEditOperation::AddText { .. }
            | PcbEditOperation::AddRegion { .. }
            | PcbEditOperation::AddPolygon { .. } => {
                // Can't easily redo adds without storing the record
                return Err(AltiumError::Parse("Cannot redo add operation".to_string()));
            }
            PcbEditOperation::DeletePrimitive { index, .. } => {
                if *index < self.doc.primitives.len() {
                    self.doc.primitives.remove(*index);
                }
            }
            PcbEditOperation::ModifyPrimitive { index, new, .. } => {
                if *index < self.doc.primitives.len() {
                    self.doc.primitives[*index] = (**new).clone();
                }
            }
            PcbEditOperation::MoveComponent {
                designator,
                new_pos,
                ..
            } => {
                if let Some(comp) = self.doc.find_component_mut(designator) {
                    comp.set_position(new_pos.x, new_pos.y);
                }
            }
        }

        self.history.push(op);
        self.modified = true;
        Ok(())
    }

    /// Clear all history.
    pub fn clear_history(&mut self) {
        self.history.clear();
        self.redo_stack.clear();
    }
}

/// Primitive count statistics.
#[derive(Debug, Default, Clone)]
pub struct PrimitiveCount {
    pub tracks: usize,
    pub vias: usize,
    pub pads: usize,
    pub arcs: usize,
    pub fills: usize,
    pub texts: usize,
    pub regions: usize,
    pub polygons: usize,
    pub other: usize,
}

impl PrimitiveCount {
    /// Total number of primitives.
    pub fn total(&self) -> usize {
        self.tracks
            + self.vias
            + self.pads
            + self.arcs
            + self.fills
            + self.texts
            + self.regions
            + self.polygons
            + self.other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_resolution() {
        let session = PcbEditSession::new();

        let pos = Position::absolute(Coord::from_mils(100.0), Coord::from_mils(200.0));
        let point = session.resolve_position(&pos).unwrap();

        assert!((point.x.to_mils() - 100.0).abs() < 0.1);
        assert!((point.y.to_mils() - 200.0).abs() < 0.1);
    }

    #[test]
    fn test_add_track() {
        let mut session = PcbEditSession::new();

        let start = CoordPoint::from_mils(0.0, 0.0);
        let end = CoordPoint::from_mils(100.0, 100.0);

        let idx = session.add_track(start, end, None, None, None).unwrap();
        assert_eq!(idx, 0);
        assert_eq!(session.count_primitives().tracks, 1);
    }

    #[test]
    fn test_add_via() {
        let mut session = PcbEditSession::new();

        let location = CoordPoint::from_mils(50.0, 50.0);
        let idx = session
            .add_via(location, None, None, None, None, None)
            .unwrap();

        assert_eq!(idx, 0);
        assert_eq!(session.count_primitives().vias, 1);
    }

    #[test]
    fn test_primitive_count() {
        let mut session = PcbEditSession::new();

        session
            .add_track(
                CoordPoint::from_mils(0.0, 0.0),
                CoordPoint::from_mils(100.0, 0.0),
                None,
                None,
                None,
            )
            .unwrap();

        session
            .add_via(
                CoordPoint::from_mils(50.0, 50.0),
                None,
                None,
                None,
                None,
                None,
            )
            .unwrap();

        let count = session.count_primitives();
        assert_eq!(count.tracks, 1);
        assert_eq!(count.vias, 1);
        assert_eq!(count.total(), 2);
    }
}
