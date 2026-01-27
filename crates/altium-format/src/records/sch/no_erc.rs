//! SchNoERC - No ERC directive (Record 22).
//!
//! A No ERC directive suppresses electrical rule check (ERC) violations
//! at a specific location on the schematic.

use crate::error::Result;
use crate::traits::{FromParams, ToParams};
use crate::types::{Coord, CoordRect, ParameterCollection, UnknownFields};
use altium_format_derive::AltiumRecord;

use super::{SchGraphicalBase, SchPrimitive};

/// No ERC symbol style.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum NoErcSymbol {
    /// Thin cross (X shape).
    #[default]
    ThinCross,
    /// Thick cross.
    ThickCross,
    /// Small cross.
    SmallCross,
}

impl NoErcSymbol {
    /// Parse from parameter string.
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "thin cross" | "thincross" => NoErcSymbol::ThinCross,
            "thick cross" | "thickcross" => NoErcSymbol::ThickCross,
            "small cross" | "smallcross" => NoErcSymbol::SmallCross,
            _ => NoErcSymbol::ThinCross,
        }
    }

    /// Convert to parameter string representation.
    pub fn to_param_string(&self) -> String {
        match self {
            NoErcSymbol::ThinCross => "Thin Cross".to_string(),
            NoErcSymbol::ThickCross => "Thick Cross".to_string(),
            NoErcSymbol::SmallCross => "Small Cross".to_string(),
        }
    }
}

/// Schematic No ERC directive.
///
/// Placed on the schematic to suppress ERC violations at a specific point,
/// such as unconnected pins that are intentionally left floating.
#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(record_id = 22, format = "params")]
pub struct SchNoErc {
    /// Graphical base (location, color).
    #[altium(flatten)]
    pub graphical: SchGraphicalBase,

    /// Index in sheet (for ordering).
    #[altium(param = "INDEXINSHEET", default)]
    pub index_in_sheet: i32,

    /// Orientation (0-3, representing 0°, 90°, 180°, 270°).
    #[altium(param = "ORIENTATION", default)]
    pub orientation: i32,

    /// Symbol style (stored as string, converted to enum).
    #[altium(param = "SYMBOL", default)]
    symbol_str: String,

    /// Whether the directive is active.
    #[altium(param = "ISACTIVE", default)]
    pub is_active: bool,

    /// Whether to suppress all violations.
    #[altium(param = "SUPPRESSALL", default)]
    pub suppress_all: bool,

    /// Connection pairs to suppress (e.g., "PNO_PNO").
    #[altium(param = "CONNECTIONPAIRSTOSUPPRESS", default)]
    pub connection_pairs_to_suppress: String,

    /// Unique identifier.
    #[altium(param = "UNIQUEID", default)]
    pub unique_id: String,

    /// Unknown parameters (preserved for non-destructive editing).
    #[altium(unknown)]
    pub unknown_params: UnknownFields,
}

impl SchNoErc {
    /// Create a new No ERC directive at the given location.
    pub fn new(x: i32, y: i32) -> Self {
        Self {
            graphical: SchGraphicalBase {
                location_x: x,
                location_y: y,
                ..Default::default()
            },
            is_active: true,
            symbol_str: "Thin Cross".to_string(),
            ..Default::default()
        }
    }

    /// Get the symbol style.
    pub fn symbol(&self) -> NoErcSymbol {
        NoErcSymbol::parse(&self.symbol_str)
    }

    /// Set the symbol style.
    pub fn set_symbol(&mut self, symbol: NoErcSymbol) {
        self.symbol_str = symbol.to_param_string();
    }
}

impl SchPrimitive for SchNoErc {
    const RECORD_ID: i32 = 22;

    fn location(&self) -> Option<crate::types::CoordPoint> {
        Some(crate::types::CoordPoint::from_raw(
            self.graphical.location_x,
            self.graphical.location_y,
        ))
    }

    fn record_type_name(&self) -> &'static str {
        "NoErc"
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
        // Small area around the marker
        let x = self.graphical.location_x;
        let y = self.graphical.location_y;
        let size = 50000; // 5 mil radius

        CoordRect::from_points(
            Coord::from_raw(x - size),
            Coord::from_raw(y - size),
            Coord::from_raw(x + size),
            Coord::from_raw(y + size),
        )
    }
}
