//! SchBus - Schematic bus (Record 26).
//!
//! A bus is a thick wire that carries multiple signals, typically named like
//! "DATA[0..7]" or "ADDR[0..15]". Bus entries (RECORD=37) connect individual
//! wires to the bus.

use crate::error::Result;
use crate::traits::{FromParams, ToParams};
use crate::types::{Coord, CoordRect, ParameterCollection, UnknownFields};
use altium_derive::AltiumRecord;

use super::{LineWidth, SchGraphicalBase, SchPrimitive};

/// Schematic bus primitive.
///
/// A bus is a bundle of related signals shown as a thick line. Individual
/// signals connect to the bus via BusEntry primitives.
#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(record_id = 26, format = "params")]
pub struct SchBus {
    /// Graphical base (location, color).
    #[altium(flatten)]
    pub graphical: SchGraphicalBase,

    /// Index in sheet (for ordering).
    #[altium(param = "INDEXINSHEET", default)]
    pub index_in_sheet: i32,

    /// Line width.
    #[altium(param = "LINEWIDTH", default)]
    pub line_width: LineWidth,

    /// Unique identifier.
    #[altium(param = "UNIQUEID", default)]
    pub unique_id: String,

    /// Bus points as (x, y) raw coord pairs.
    #[altium(
        indexed_coords,
        prefix_x = "X",
        prefix_y = "Y",
        count = "LOCATIONCOUNT"
    )]
    pub vertices: Vec<(i32, i32)>,

    /// Unknown parameters (preserved for non-destructive editing).
    #[altium(unknown)]
    pub unknown_params: UnknownFields,
}

impl SchBus {
    /// Create a new bus with the given vertices.
    pub fn new(vertices: Vec<(i32, i32)>) -> Self {
        Self {
            vertices,
            line_width: LineWidth::Medium,
            ..Default::default()
        }
    }

    /// Create a bus between two points.
    pub fn from_points(x1: i32, y1: i32, x2: i32, y2: i32) -> Self {
        Self::new(vec![(x1, y1), (x2, y2)])
    }

    /// Get the start point of the bus.
    pub fn start(&self) -> Option<(i32, i32)> {
        self.vertices.first().copied()
    }

    /// Get the end point of the bus.
    pub fn end(&self) -> Option<(i32, i32)> {
        self.vertices.last().copied()
    }

    /// Check if a point lies on the bus.
    pub fn contains_point(&self, x: i32, y: i32) -> bool {
        for window in self.vertices.windows(2) {
            let (x1, y1) = window[0];
            let (x2, y2) = window[1];

            // Check if point is on the line segment
            if x1 == x2 {
                // Vertical segment
                let (min_y, max_y) = if y1 < y2 { (y1, y2) } else { (y2, y1) };
                if x == x1 && y >= min_y && y <= max_y {
                    return true;
                }
            } else if y1 == y2 {
                // Horizontal segment
                let (min_x, max_x) = if x1 < x2 { (x1, x2) } else { (x2, x1) };
                if y == y1 && x >= min_x && x <= max_x {
                    return true;
                }
            }
            // Note: Diagonal segments would need more complex logic
        }
        false
    }
}

impl SchPrimitive for SchBus {
    const RECORD_ID: i32 = 26;

    fn location(&self) -> Option<crate::types::CoordPoint> {
        Some(crate::types::CoordPoint::from_raw(
            self.graphical.location_x,
            self.graphical.location_y,
        ))
    }

    fn record_type_name(&self) -> &'static str {
        "Bus"
    }

    fn import_from_params(params: &ParameterCollection) -> Result<Self> {
        Self::from_params(params)
    }

    fn export_to_params(&self) -> ParameterCollection {
        self.to_params()
    }

    fn owner_index(&self) -> i32 {
        self.graphical.base.owner_index
    }

    fn calculate_bounds(&self) -> CoordRect {
        let Some(&(first_x, first_y)) = self.vertices.first() else {
            return CoordRect::empty();
        };

        let (min_x, max_x, min_y, max_y) = self.vertices.iter().skip(1).fold(
            (first_x, first_x, first_y, first_y),
            |(min_x, max_x, min_y, max_y), &(x, y)| {
                (min_x.min(x), max_x.max(x), min_y.min(y), max_y.max(y))
            },
        );

        CoordRect::from_points(
            Coord::from_raw(min_x),
            Coord::from_raw(min_y),
            Coord::from_raw(max_x),
            Coord::from_raw(max_y),
        )
    }
}
