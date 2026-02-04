//! SchBusEntry - Schematic bus entry (Record 37).
//!
//! A bus entry is a short diagonal line that connects a wire to a bus.
//! It typically appears as a small angled segment at 45 degrees.
//!
//! **DEPRECATED**: Use `v2::fields::BusEntryData` with `v2::serializer::format_v5` instead.

use crate::error::Result;
use crate::types::{
    Coord, CoordRect, ParameterCollection, UnknownFields, coord_to_dxp_frac, dxp_frac_to_coord,
};
use altium_format_derive::AltiumRecord;

use super::{LineWidth, SchGraphicalBase, SchPrimitive};

/// Schematic bus entry primitive.
///
/// A bus entry connects a wire to a bus with a short diagonal line segment.
/// The Location point is on the bus, and the Corner point is where the wire connects.
///
/// **DEPRECATED**: Use `v2::fields::BusEntryData` instead.
#[deprecated(note = "Use v2::fields::BusEntryData")]
#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(record_id = 37, format = "params")]
pub struct SchBusEntry {
    /// Graphical base (location is the bus-side point).
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

    /// Corner X coordinate (wire-side point, in raw internal units).
    #[altium(param = "CORNER.X", default)]
    corner_x_raw: i32,

    /// Corner X fractional part.
    #[altium(param = "CORNER.X_FRAC", default)]
    corner_x_frac: i32,

    /// Corner Y coordinate (wire-side point, in raw internal units).
    #[altium(param = "CORNER.Y", default)]
    corner_y_raw: i32,

    /// Corner Y fractional part.
    #[altium(param = "CORNER.Y_FRAC", default)]
    corner_y_frac: i32,

    /// Unknown parameters (preserved for non-destructive editing).
    #[altium(unknown)]
    pub unknown_params: UnknownFields,
}

impl SchBusEntry {
    /// Create a new bus entry from bus point to wire point.
    ///
    /// * `bus_x`, `bus_y` - Point on the bus
    /// * `wire_x`, `wire_y` - Point where wire connects
    pub fn new(bus_x: i32, bus_y: i32, wire_x: i32, wire_y: i32) -> Self {
        let (corner_x_raw, corner_x_frac) = coord_to_dxp_frac(wire_x);
        let (corner_y_raw, corner_y_frac) = coord_to_dxp_frac(wire_y);

        Self {
            graphical: SchGraphicalBase {
                location_x: bus_x,
                location_y: bus_y,
                ..Default::default()
            },
            corner_x_raw,
            corner_x_frac,
            corner_y_raw,
            corner_y_frac,
            line_width: LineWidth::Small,
            ..Default::default()
        }
    }

    /// Get the bus-side point (where the entry connects to the bus).
    pub fn bus_point(&self) -> (i32, i32) {
        (self.graphical.location_x, self.graphical.location_y)
    }

    /// Get the wire-side point (where the entry connects to a wire).
    pub fn wire_point(&self) -> (i32, i32) {
        let x = dxp_frac_to_coord(self.corner_x_raw, self.corner_x_frac);
        let y = dxp_frac_to_coord(self.corner_y_raw, self.corner_y_frac);
        (x, y)
    }

    /// Get the corner X coordinate (wire-side).
    pub fn corner_x(&self) -> i32 {
        dxp_frac_to_coord(self.corner_x_raw, self.corner_x_frac)
    }

    /// Get the corner Y coordinate (wire-side).
    pub fn corner_y(&self) -> i32 {
        dxp_frac_to_coord(self.corner_y_raw, self.corner_y_frac)
    }

    /// Set the wire-side point.
    pub fn set_wire_point(&mut self, x: i32, y: i32) {
        let (x_raw, x_frac) = coord_to_dxp_frac(x);
        let (y_raw, y_frac) = coord_to_dxp_frac(y);
        self.corner_x_raw = x_raw;
        self.corner_x_frac = x_frac;
        self.corner_y_raw = y_raw;
        self.corner_y_frac = y_frac;
    }
}

#[allow(deprecated)]
impl SchPrimitive for SchBusEntry {
    const RECORD_ID: i32 = 37;

    fn location(&self) -> Option<crate::types::CoordPoint> {
        Some(crate::types::CoordPoint::from_raw(
            self.graphical.location_x,
            self.graphical.location_y,
        ))
    }

    fn record_type_name(&self) -> &'static str {
        "BusEntry"
    }

    fn import_from_params(_params: &ParameterCollection) -> Result<Self> {
        unimplemented!(
            "V1 SchBusEntry::import_from_params is deprecated. \
            Use v2::fields::BusEntryData with v2::serializer::format_v5 instead."
        )
    }

    fn export_to_params(&self) -> ParameterCollection {
        unimplemented!(
            "V1 SchBusEntry::export_to_params is deprecated. \
            Use v2::fields::BusEntryData with v2::serializer::format_v5 instead."
        )
    }

    fn owner_index(&self) -> i32 {
        self.graphical.base.owner_index
    }

    fn calculate_bounds(&self) -> CoordRect {
        let (bus_x, bus_y) = self.bus_point();
        let (wire_x, wire_y) = self.wire_point();

        let min_x = bus_x.min(wire_x);
        let max_x = bus_x.max(wire_x);
        let min_y = bus_y.min(wire_y);
        let max_y = bus_y.max(wire_y);

        CoordRect::from_points(
            Coord::from_raw(min_x),
            Coord::from_raw(min_y),
            Coord::from_raw(max_x),
            Coord::from_raw(max_y),
        )
    }
}
