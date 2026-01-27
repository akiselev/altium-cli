//! SchPin - Schematic pin (Record 2) - NEW DERIVE MACRO VERSION
//!
//! This is an example of what the migrated SchPin would look like using the
//! derive macros. This file is for reference/validation during development.

#![allow(dead_code)]

use crate::types::{Coord, CoordRect, UnknownFields};
use altium_format_derive::AltiumRecord;

use super::{LineWidth, PinConglomerateFlags, PinElectricalType, PinSymbol, SchGraphicalBase};

/// Schematic pin primitive - NEW VERSION with derive macro.
///
/// Compare to pin.rs which has ~90 lines of manual serialization code.
/// This version: ~30 lines of struct definition with derive macro.
#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(record_id = 2, format = "params")]
pub struct SchPinNew {
    /// Graphical base (location, color, owner_index).
    #[altium(flatten)]
    pub graphical: SchGraphicalBase,

    /// Symbol on inner edge.
    #[altium(param = "SYMBOL_INNEREDGE", default)]
    pub symbol_inner_edge: PinSymbol,

    /// Symbol on outer edge.
    #[altium(param = "SYMBOL_OUTEREDGE", default)]
    pub symbol_outer_edge: PinSymbol,

    /// Symbol inside.
    #[altium(param = "SYMBOL_INSIDE", default)]
    pub symbol_inside: PinSymbol,

    /// Symbol outside.
    #[altium(param = "SYMBOL_OUTSIDE", default)]
    pub symbol_outside: PinSymbol,

    /// Symbol line width.
    #[altium(param = "SYMBOL_LINEWIDTH", default)]
    pub symbol_line_width: LineWidth,

    /// Pin description.
    #[altium(param = "DESCRIPTION", default)]
    pub description: String,

    /// Formal type.
    #[altium(param = "FORMALTYPE", default)]
    pub formal_type: i32,

    /// Electrical type.
    #[altium(param = "ELECTRICAL", default)]
    pub electrical: PinElectricalType,

    /// Pin conglomerate flags.
    #[altium(param = "PINCONGLOMERATE", default)]
    pub pin_conglomerate: PinConglomerateFlags,

    /// Pin length (raw Coord units) - uses integer + fractional parts.
    #[altium(param = "PINLENGTH", frac = "PINLENGTH_FRAC")]
    pub pin_length: i32,

    /// Pin name.
    #[altium(param = "NAME", default)]
    pub name: String,

    /// Pin designator.
    #[altium(param = "DESIGNATOR", default)]
    pub designator: String,

    /// Swap ID group.
    #[altium(param = "SWAPIDGROUP", default)]
    pub swap_id_group: String,

    /// Swap ID part.
    #[altium(param = "SWAPIDPART", default)]
    pub swap_id_part: i32,

    /// Swap ID sequence.
    #[altium(param = "SWAPIDSEQUENCE", default)]
    pub swap_id_sequence: String,

    /// Hidden net name.
    #[altium(param = "HIDDENNETNAME", default)]
    pub hidden_net_name: String,

    /// Default value.
    #[altium(param = "DEFAULTVALUE", default)]
    pub default_value: String,

    /// Propagation delay.
    #[altium(param = "PINPROPAGATIONDELAY", default)]
    pub pin_propagation_delay: f64,

    /// Unique ID.
    #[altium(param = "UNIQUEID", default)]
    pub unique_id: String,

    /// Unknown parameters (preserved for non-destructive editing).
    #[altium(unknown)]
    pub unknown_params: UnknownFields,
}

// Custom methods remain unchanged
impl SchPinNew {
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

// Manual calculate_bounds implementation (logic varies per type)
impl SchPinNew {
    pub fn calculate_bounds(&self) -> CoordRect {
        let (cx, cy) = self.get_corner();
        CoordRect::from_points(
            Coord::from_raw(self.graphical.location_x),
            Coord::from_raw(self.graphical.location_y),
            Coord::from_raw(cx),
            Coord::from_raw(cy),
        )
    }
}
