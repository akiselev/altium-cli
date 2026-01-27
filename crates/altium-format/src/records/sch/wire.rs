//! SchWire - Schematic wire (Record 27).

use crate::error::Result;
use crate::traits::{FromParams, ToParams};
use crate::types::{Coord, CoordRect, ParameterCollection, UnknownFields};
use altium_format_derive::AltiumRecord;

use super::{LineStyle, LineWidth, SchGraphicalBase, SchPrimitive};

/// Schematic wire primitive.
/// Wires are the electrical connections in a schematic.
#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(record_id = 27, format = "params")]
pub struct SchWire {
    /// Graphical base (location, color).
    #[altium(flatten)]
    pub graphical: SchGraphicalBase,

    /// Line width.
    #[altium(param = "LINEWIDTH", default)]
    pub line_width: LineWidth,

    /// Line style.
    #[altium(param = "LINESTYLE", default)]
    pub line_style: LineStyle,

    /// Wire points as (x, y) raw coord pairs.
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

// Provide polyline field for compatibility with existing code
impl SchWire {
    /// Get a polyline-like view for compatibility.
    pub fn polyline(&self) -> WirePolylineView<'_> {
        WirePolylineView { wire: self }
    }
}

/// View into wire's polyline-like data.
pub struct WirePolylineView<'a> {
    wire: &'a SchWire,
}

impl<'a> WirePolylineView<'a> {
    pub fn polygon(&self) -> WirePolygonView<'a> {
        WirePolygonView { wire: self.wire }
    }
}

/// View into wire's polygon-like data.
pub struct WirePolygonView<'a> {
    wire: &'a SchWire,
}

impl<'a> WirePolygonView<'a> {
    pub fn vertices(&self) -> &[(i32, i32)] {
        &self.wire.vertices
    }

    pub fn graphical(&self) -> &SchGraphicalBase {
        &self.wire.graphical
    }
}

impl SchPrimitive for SchWire {
    const RECORD_ID: i32 = 27;

    fn location(&self) -> Option<crate::types::CoordPoint> {
        Some(crate::types::CoordPoint::from_raw(
            self.graphical.location_x,
            self.graphical.location_y,
        ))
    }

    fn record_type_name(&self) -> &'static str {
        "Wire"
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
