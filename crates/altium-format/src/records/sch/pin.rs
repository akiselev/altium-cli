//! SchPin - Schematic pin (Record 2).
//!
//! **DEPRECATED**: Use `v2::fields::PinData` with `v2::serializer::format_v5` instead.

use crate::error::Result;
use crate::types::{Coord, CoordRect, ParameterCollection};

use super::{
    LineWidth, PinConglomerateFlags, PinElectricalType, PinSymbol, SchGraphicalBase, SchPrimitive,
};

/// Schematic pin primitive.
///
/// **DEPRECATED**: Use `v2::fields::PinData` with `v2::serializer::format_v5::import_pin()`
/// and `v2::serializer::format_v5::export_pin()` instead.
#[deprecated(note = "Use v2::fields::PinData")]
#[derive(Debug, Clone)]
pub struct SchPin {
    /// Graphical base (location, color).
    pub graphical: SchGraphicalBase,
    /// Symbol on inner edge.
    pub symbol_inner_edge: PinSymbol,
    /// Symbol on outer edge.
    pub symbol_outer_edge: PinSymbol,
    /// Symbol inside.
    pub symbol_inside: PinSymbol,
    /// Symbol outside.
    pub symbol_outside: PinSymbol,
    /// Symbol line width.
    pub symbol_line_width: LineWidth,
    /// Pin description.
    pub description: String,
    /// Formal type.
    pub formal_type: i32,
    /// Electrical type.
    pub electrical: PinElectricalType,
    /// Pin conglomerate flags.
    pub pin_conglomerate: PinConglomerateFlags,
    /// Pin length (raw Coord units).
    pub pin_length: i32,
    /// Pin name.
    pub name: String,
    /// Pin designator.
    pub designator: String,
    /// Swap ID group.
    pub swap_id_group: String,
    /// Swap ID part.
    pub swap_id_part: i32,
    /// Swap ID sequence.
    pub swap_id_sequence: String,
    /// Hidden net name.
    pub hidden_net_name: String,
    /// Default value.
    pub default_value: String,
    /// Propagation delay.
    pub pin_propagation_delay: f64,
    /// Unique ID.
    pub unique_id: String,
}

impl Default for SchPin {
    fn default() -> Self {
        Self {
            graphical: SchGraphicalBase::default(),
            symbol_inner_edge: PinSymbol::default(), // None
            symbol_outer_edge: PinSymbol::default(), // None
            symbol_inside: PinSymbol::default(),     // None
            symbol_outside: PinSymbol::default(),    // None
            symbol_line_width: LineWidth::default(), // Smallest
            description: String::new(),
            formal_type: 0,
            electrical: PinElectricalType::default(), // Passive
            pin_conglomerate: PinConglomerateFlags::default(),
            pin_length: 0, // 0 units (user should set this)
            name: String::new(),
            designator: String::new(),
            swap_id_group: String::new(),
            swap_id_part: 0,
            swap_id_sequence: String::new(),
            hidden_net_name: String::new(),
            default_value: String::new(),
            pin_propagation_delay: 0.0,
            unique_id: String::new(),
        }
    }
}

impl SchPin {
    /// Returns true if the pin name is visible.
    pub fn is_name_visible(&self) -> bool {
        self.pin_conglomerate
            .contains(PinConglomerateFlags::DISPLAY_NAME_VISIBLE)
    }

    /// Returns true if the pin designator is visible.
    pub fn is_designator_visible(&self) -> bool {
        self.pin_conglomerate
            .contains(PinConglomerateFlags::DESIGNATOR_VISIBLE)
    }

    /// Returns true if the pin is hidden.
    pub fn is_hidden(&self) -> bool {
        self.pin_conglomerate.contains(PinConglomerateFlags::HIDE)
    }

    /// Get the corner point (end of pin).
    pub fn get_corner(&self) -> (i32, i32) {
        let rotated = self
            .pin_conglomerate
            .contains(PinConglomerateFlags::ROTATED);
        let flipped = self
            .pin_conglomerate
            .contains(PinConglomerateFlags::FLIPPED);

        if rotated {
            if flipped {
                (
                    self.graphical.location_x,
                    self.graphical.location_y - self.pin_length,
                )
            } else {
                (
                    self.graphical.location_x,
                    self.graphical.location_y + self.pin_length,
                )
            }
        } else if flipped {
            (
                self.graphical.location_x - self.pin_length,
                self.graphical.location_y,
            )
        } else {
            (
                self.graphical.location_x + self.pin_length,
                self.graphical.location_y,
            )
        }
    }
}

#[allow(deprecated)]
impl SchPrimitive for SchPin {
    const RECORD_ID: i32 = 2;

    fn location(&self) -> Option<crate::types::CoordPoint> {
        Some(crate::types::CoordPoint::from_raw(
            self.graphical.location_x,
            self.graphical.location_y,
        ))
    }

    fn record_type_name(&self) -> &'static str {
        "Pin"
    }

    fn get_property(&self, name: &str) -> Option<String> {
        match name {
            "NAME" => Some(self.name.clone()),
            "DESIGNATOR" => Some(self.designator.clone()),
            "DESCRIPTION" => Some(self.description.clone()),
            _ => None,
        }
    }

    fn import_from_params(_params: &ParameterCollection) -> Result<Self> {
        unimplemented!(
            "V1 SchPin::import_from_params is deprecated. \
            Use v2::fields::PinData with v2::serializer::format_v5::import_pin() instead."
        )
    }

    fn export_to_params(&self) -> ParameterCollection {
        unimplemented!(
            "V1 SchPin::export_to_params is deprecated. \
            Use v2::fields::PinData with v2::serializer::format_v5::export_pin() instead."
        )
    }

    fn owner_index(&self) -> i32 {
        self.graphical.base.owner_index
    }

    fn calculate_bounds(&self) -> CoordRect {
        let (cx, cy) = self.get_corner();
        CoordRect::from_points(
            Coord::from_raw(self.graphical.location_x),
            Coord::from_raw(self.graphical.location_y),
            Coord::from_raw(cx),
            Coord::from_raw(cy),
        )
    }
}
