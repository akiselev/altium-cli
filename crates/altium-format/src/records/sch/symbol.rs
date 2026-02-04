//! SchSymbol - Schematic symbol (Record 3).
//!
//! **DEPRECATED**: Use `v2::fields::SymbolData` with `v2::serializer::format_v5` instead.

use crate::error::Result;
use crate::types::{CoordRect, ParameterCollection, UnknownFields};
use altium_format_derive::AltiumRecord;

use super::{SchGraphicalBase, SchPrimitive, TextOrientations};

/// Schematic symbol primitive - symbol graphics container.
///
/// **DEPRECATED**: Use `v2::fields::SymbolData` instead.
#[deprecated(note = "Use v2::fields::SymbolData")]
#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(record_id = 3, format = "params")]
pub struct SchSymbol {
    /// Graphical base (location, color).
    #[altium(flatten)]
    pub graphical: SchGraphicalBase,

    /// Symbol orientation.
    #[altium(param = "ORIENTATION", default)]
    pub orientation: TextOrientations,

    /// Scale factor.
    #[altium(param = "SCALEFACTOR", default = 1.0)]
    pub scale_factor: f64,

    /// Symbol type (0=normal, 1=IEEE).
    #[altium(param = "SYMBOLTYPE", default)]
    pub symbol_type: i32,

    /// Whether the symbol is mirrored.
    #[altium(param = "ISMIRRORED", default)]
    pub is_mirrored: bool,

    /// Unknown parameters (preserved for non-destructive editing).
    #[altium(unknown)]
    pub unknown_params: UnknownFields,
}

#[allow(deprecated)]
impl SchPrimitive for SchSymbol {
    const RECORD_ID: i32 = 3;

    fn location(&self) -> Option<crate::types::CoordPoint> {
        Some(crate::types::CoordPoint::from_raw(
            self.graphical.location_x,
            self.graphical.location_y,
        ))
    }

    fn record_type_name(&self) -> &'static str {
        "Symbol"
    }

    fn import_from_params(_params: &ParameterCollection) -> Result<Self> {
        unimplemented!(
            "V1 SchSymbol::import_from_params is deprecated. \
            Use v2::fields::SymbolData with v2::serializer::format_v5 instead."
        )
    }

    fn export_to_params(&self) -> ParameterCollection {
        unimplemented!(
            "V1 SchSymbol::export_to_params is deprecated. \
            Use v2::fields::SymbolData with v2::serializer::format_v5 instead."
        )
    }

    fn owner_index(&self) -> i32 {
        self.graphical.base.owner_index
    }

    fn calculate_bounds(&self) -> CoordRect {
        // Symbol bounds are calculated from child primitives
        CoordRect::empty()
    }
}
