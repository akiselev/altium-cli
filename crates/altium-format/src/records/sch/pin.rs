//! SchPin - Schematic pin (Record 2).

use crate::error::Result;
use crate::types::{Coord, CoordRect, ParameterCollection};

use super::{
    LineWidth, PinConglomerateFlags, PinElectricalType, PinSymbol, SchGraphicalBase, SchPrimitive,
    coord_to_dxp_frac, dxp_frac_to_coord,
};

/// Schematic pin primitive.
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

    fn import_from_params(params: &ParameterCollection) -> Result<Self> {
        let graphical = SchGraphicalBase::import_from_params(params);

        let pin_length_int = params.get("PINLENGTH").map(|v| v.as_int_or(0)).unwrap_or(0);
        let pin_length_frac = params
            .get("PINLENGTH_FRAC")
            .map(|v| v.as_int_or(0))
            .unwrap_or(0);

        Ok(SchPin {
            graphical,
            symbol_inner_edge: params
                .get("SYMBOL_INNEREDGE")
                .map(|v| PinSymbol::from_int(v.as_int_or(0)))
                .unwrap_or_default(),
            symbol_outer_edge: params
                .get("SYMBOL_OUTEREDGE")
                .map(|v| PinSymbol::from_int(v.as_int_or(0)))
                .unwrap_or_default(),
            symbol_inside: params
                .get("SYMBOL_INSIDE")
                .map(|v| PinSymbol::from_int(v.as_int_or(0)))
                .unwrap_or_default(),
            symbol_outside: params
                .get("SYMBOL_OUTSIDE")
                .map(|v| PinSymbol::from_int(v.as_int_or(0)))
                .unwrap_or_default(),
            symbol_line_width: params
                .get("SYMBOL_LINEWIDTH")
                .map(|v| LineWidth::from_int(v.as_int_or(0)))
                .unwrap_or_default(),
            description: params
                .get("DESCRIPTION")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            formal_type: params
                .get("FORMALTYPE")
                .map(|v| v.as_int_or(0))
                .unwrap_or(0),
            electrical: params
                .get("ELECTRICAL")
                .map(|v| PinElectricalType::from_int(v.as_int_or(0)))
                .unwrap_or_default(),
            pin_conglomerate: params
                .get("PINCONGLOMERATE")
                .map(|v| PinConglomerateFlags::from_int(v.as_int_or(0)))
                .unwrap_or_default(),
            pin_length: dxp_frac_to_coord(pin_length_int, pin_length_frac),
            name: params
                .get("NAME")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            designator: params
                .get("DESIGNATOR")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            swap_id_group: params
                .get("SWAPIDGROUP")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            swap_id_part: params
                .get("SWAPIDPART")
                .map(|v| v.as_int_or(0))
                .unwrap_or(0),
            swap_id_sequence: params
                .get("SWAPIDSEQUENCE")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            hidden_net_name: String::new(),
            default_value: String::new(),
            pin_propagation_delay: params
                .get("PINPROPAGATIONDELAY")
                .map(|v| v.as_double_or(0.0))
                .unwrap_or(0.0),
            unique_id: params
                .get("UNIQUEID")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
        })
    }

    fn export_to_params(&self) -> ParameterCollection {
        let mut params = ParameterCollection::new();
        params.add_int("RECORD", Self::RECORD_ID);
        self.graphical.export_to_params(&mut params);

        params.add_int("SYMBOL_INNEREDGE", self.symbol_inner_edge.to_int());
        params.add_int("SYMBOL_OUTEREDGE", self.symbol_outer_edge.to_int());
        params.add_int("SYMBOL_INSIDE", self.symbol_inside.to_int());
        params.add_int("SYMBOL_OUTSIDE", self.symbol_outside.to_int());
        params.add_int("SYMBOL_LINEWIDTH", self.symbol_line_width.to_int());
        params.add("DESCRIPTION", &self.description);
        params.add_int("FORMALTYPE", self.formal_type);
        params.add_int("ELECTRICAL", self.electrical.to_int());
        params.add_int("PINCONGLOMERATE", self.pin_conglomerate.to_int());

        let (length, length_frac) = coord_to_dxp_frac(self.pin_length);
        params.add_int("PINLENGTH", length);
        params.add_int("PINLENGTH_FRAC", length_frac);

        params.add("NAME", &self.name);
        params.add("DESIGNATOR", &self.designator);
        params.add_int("SWAPIDPART", self.swap_id_part);
        params.add_double("PINPROPAGATIONDELAY", self.pin_propagation_delay, 6);

        params
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
