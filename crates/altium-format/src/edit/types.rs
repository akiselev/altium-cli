//! Common types for the schematic editing system.

use crate::types::{Coord, CoordPoint, CoordRect};

/// Default grid spacing in mils (10 mils = 100,000 internal units).
pub const DEFAULT_GRID_MILS: f64 = 10.0;

/// Default component spacing in mils.
pub const DEFAULT_COMPONENT_SPACING_MILS: f64 = 100.0;

/// Default wire spacing in mils.
pub const DEFAULT_WIRE_SPACING_MILS: f64 = 10.0;

/// Grid configuration for placement.
#[derive(Debug, Clone, Copy)]
pub struct Grid {
    /// Grid spacing in internal units.
    pub spacing: Coord,
    /// Whether to snap to grid.
    pub snap_enabled: bool,
}

impl Default for Grid {
    fn default() -> Self {
        Self {
            spacing: Coord::from_mils(DEFAULT_GRID_MILS),
            snap_enabled: true,
        }
    }
}

impl Grid {
    /// Create a grid with the specified spacing in mils.
    pub fn with_mils(mils: f64) -> Self {
        Self {
            spacing: Coord::from_mils(mils),
            snap_enabled: true,
        }
    }

    /// Snap a point to the nearest grid intersection.
    pub fn snap(&self, point: CoordPoint) -> CoordPoint {
        if !self.snap_enabled {
            return point;
        }
        let spacing = self.spacing.to_raw();
        if spacing == 0 {
            return point;
        }
        CoordPoint {
            x: Coord::from_raw(((point.x.to_raw() + spacing / 2) / spacing) * spacing),
            y: Coord::from_raw(((point.y.to_raw() + spacing / 2) / spacing) * spacing),
        }
    }

    /// Snap a coordinate to the nearest grid line.
    pub fn snap_coord(&self, coord: Coord) -> Coord {
        if !self.snap_enabled {
            return coord;
        }
        let spacing = self.spacing.to_raw();
        if spacing == 0 {
            return coord;
        }
        Coord::from_raw(((coord.to_raw() + spacing / 2) / spacing) * spacing)
    }
}

/// Orientation for component placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orientation {
    #[default]
    Normal,
    Rotated90,
    Rotated180,
    Rotated270,
    MirroredX,
    MirroredY,
    MirroredXRotated90,
    MirroredYRotated90,
}

impl Orientation {
    /// Get orientation value for Altium (0-3 for rotation, with mirror flags).
    pub fn to_altium(&self) -> i32 {
        match self {
            Orientation::Normal => 0,
            Orientation::Rotated90 => 1,
            Orientation::Rotated180 => 2,
            Orientation::Rotated270 => 3,
            Orientation::MirroredX => 0, // Plus mirror flag
            Orientation::MirroredY => 0,
            Orientation::MirroredXRotated90 => 1,
            Orientation::MirroredYRotated90 => 1,
        }
    }

    /// Check if this orientation includes mirroring.
    pub fn is_mirrored(&self) -> bool {
        matches!(
            self,
            Orientation::MirroredX
                | Orientation::MirroredY
                | Orientation::MirroredXRotated90
                | Orientation::MirroredYRotated90
        )
    }

    /// Get the rotation angle in degrees.
    pub fn rotation_degrees(&self) -> f64 {
        match self {
            Orientation::Normal | Orientation::MirroredX | Orientation::MirroredY => 0.0,
            Orientation::Rotated90
            | Orientation::MirroredXRotated90
            | Orientation::MirroredYRotated90 => 90.0,
            Orientation::Rotated180 => 180.0,
            Orientation::Rotated270 => 270.0,
        }
    }
}

/// Direction for wire routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    /// Get the unit vector for this direction.
    pub fn unit_vector(&self) -> (i32, i32) {
        match self {
            Direction::Up => (0, 1),
            Direction::Down => (0, -1),
            Direction::Left => (-1, 0),
            Direction::Right => (1, 0),
        }
    }

    /// Get the unit vector as f64 for this direction.
    pub fn unit_vector_f64(&self) -> (f64, f64) {
        match self {
            Direction::Up => (0.0, 1.0),
            Direction::Down => (0.0, -1.0),
            Direction::Left => (-1.0, 0.0),
            Direction::Right => (1.0, 0.0),
        }
    }

    /// Get the opposite direction.
    pub fn opposite(&self) -> Direction {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }

    /// Get perpendicular directions.
    pub fn perpendicular(&self) -> [Direction; 2] {
        match self {
            Direction::Up | Direction::Down => [Direction::Left, Direction::Right],
            Direction::Left | Direction::Right => [Direction::Up, Direction::Down],
        }
    }
}

/// A placed component reference.
#[derive(Debug, Clone)]
pub struct PlacedComponent {
    /// Index in the primitives list.
    pub index: usize,
    /// Component designator (e.g., "U1", "R1").
    pub designator: String,
    /// Library reference name.
    pub lib_reference: String,
    /// Bounding box.
    pub bounds: CoordRect,
    /// Pin locations (absolute coordinates).
    pub pin_locations: Vec<PinLocation>,
}

/// A pin location with net information.
#[derive(Debug, Clone)]
pub struct PinLocation {
    /// Pin designator (e.g., "1", "VCC").
    pub designator: String,
    /// Pin name.
    pub name: String,
    /// Absolute location of the pin endpoint.
    pub location: CoordPoint,
    /// Direction the pin faces (for routing).
    pub direction: Direction,
}

/// Placement suggestion result.
#[derive(Debug, Clone)]
pub struct PlacementSuggestion {
    /// Suggested location.
    pub location: CoordPoint,
    /// Suggested orientation.
    pub orientation: Orientation,
    /// Quality score (higher is better).
    pub score: f64,
    /// Reason for this suggestion.
    pub reason: String,
}

/// Wire segment for routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireSegment {
    /// Start point.
    pub start: CoordPoint,
    /// End point.
    pub end: CoordPoint,
}

impl WireSegment {
    /// Create a new wire segment.
    pub fn new(start: CoordPoint, end: CoordPoint) -> Self {
        Self { start, end }
    }

    /// Check if this segment is horizontal.
    pub fn is_horizontal(&self) -> bool {
        self.start.y == self.end.y
    }

    /// Check if this segment is vertical.
    pub fn is_vertical(&self) -> bool {
        self.start.x == self.end.x
    }

    /// Get the bounding box of this segment.
    pub fn bounds(&self) -> CoordRect {
        CoordRect::new(self.start, self.end)
    }

    /// Check if a point lies on this segment.
    pub fn contains_point(&self, point: CoordPoint) -> bool {
        let bounds = self.bounds();
        if !bounds.contains(point) {
            return false;
        }

        // For horizontal/vertical segments, just check bounds
        if self.is_horizontal() || self.is_vertical() {
            return true;
        }

        // For diagonal segments (shouldn't happen in schematics), use cross product
        let dx1 = point.x.to_raw() - self.start.x.to_raw();
        let dy1 = point.y.to_raw() - self.start.y.to_raw();
        let dx2 = self.end.x.to_raw() - self.start.x.to_raw();
        let dy2 = self.end.y.to_raw() - self.start.y.to_raw();

        let cross = dx1 as i64 * dy2 as i64 - dy1 as i64 * dx2 as i64;
        cross == 0
    }
}

/// A routing path consisting of multiple segments.
#[derive(Debug, Clone, Default)]
pub struct RoutingPath {
    /// Wire segments in order.
    pub segments: Vec<WireSegment>,
    /// Junction locations where wires connect.
    pub junctions: Vec<CoordPoint>,
}

impl RoutingPath {
    /// Create an empty path.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a segment to the path.
    pub fn add_segment(&mut self, segment: WireSegment) {
        self.segments.push(segment);
    }

    /// Get all vertices in the path.
    pub fn vertices(&self) -> Vec<CoordPoint> {
        if self.segments.is_empty() {
            return Vec::new();
        }

        let mut vertices = vec![self.segments[0].start];
        for segment in &self.segments {
            vertices.push(segment.end);
        }
        vertices
    }

    /// Get the total length of the path.
    pub fn total_length(&self) -> Coord {
        let mut length = Coord::ZERO;
        for segment in &self.segments {
            let dx = (segment.end.x - segment.start.x).abs();
            let dy = (segment.end.y - segment.start.y).abs();
            length = length + dx + dy; // Manhattan distance
        }
        length
    }
}

/// Validation error for schematic edits.
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Error type.
    pub kind: ValidationErrorKind,
    /// Human-readable message.
    pub message: String,
    /// Location of the error (if applicable).
    pub location: Option<CoordPoint>,
    /// Related component designators.
    pub components: Vec<String>,
}

/// Types of validation errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationErrorKind {
    /// Components overlap.
    ComponentOverlap,
    /// Wire crosses component.
    WireCrossesComponent,
    /// Unconnected pin.
    UnconnectedPin,
    /// Missing junction.
    MissingJunction,
    /// Duplicate designator.
    DuplicateDesignator,
    /// Invalid net connection.
    InvalidNetConnection,
    /// Off-grid placement.
    OffGrid,
}

/// Edit operation for undo/redo.
#[derive(Debug, Clone)]
pub enum EditOperation {
    /// Add a component.
    AddComponent { index: usize, designator: String },
    /// Remove a component.
    RemoveComponent { index: usize, designator: String },
    /// Move a component.
    MoveComponent {
        index: usize,
        from: CoordPoint,
        to: CoordPoint,
    },
    /// Add a wire.
    AddWire { index: usize },
    /// Remove a wire.
    RemoveWire { index: usize },
    /// Add a junction.
    AddJunction { index: usize, location: CoordPoint },
    /// Add a net label.
    AddNetLabel { index: usize, net_name: String },
    /// Add a power port.
    AddPowerPort { index: usize, net_name: String },
    /// Batch operation containing multiple operations.
    Batch { operations: Vec<EditOperation> },
}
