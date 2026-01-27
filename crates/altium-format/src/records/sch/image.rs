//! SchImage - Schematic image (Record 30).

use crate::error::Result;
use crate::traits::{FromParams, ToParams};
use crate::types::{Coord, CoordRect, ParameterCollection, UnknownFields};
use altium_derive::AltiumRecord;

use super::{SchGraphicalBase, SchPrimitive};

/// Schematic image primitive.
#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(record_id = 30, format = "params")]
pub struct SchImage {
    /// Graphical base (location = one corner, color).
    #[altium(flatten)]
    pub graphical: SchGraphicalBase,

    /// Corner point X (opposite corner).
    #[altium(param = "CORNER.X", frac = "CORNER.X_FRAC")]
    pub corner_x: i32,

    /// Corner point Y (opposite corner).
    #[altium(param = "CORNER.Y", frac = "CORNER.Y_FRAC")]
    pub corner_y: i32,

    /// Filename.
    #[altium(param = "FILENAME", default)]
    pub filename: String,

    /// Whether the image is embedded.
    #[altium(param = "EMBEDIMAGE", default)]
    pub embed_image: bool,

    /// Keep aspect ratio.
    #[altium(param = "KEEPASPECT", default)]
    pub keep_aspect: bool,

    /// Unknown parameters (preserved for non-destructive editing).
    #[altium(unknown)]
    pub unknown_params: UnknownFields,
}

impl SchPrimitive for SchImage {
    const RECORD_ID: i32 = 30;

    fn location(&self) -> Option<crate::types::CoordPoint> {
        Some(crate::types::CoordPoint::from_raw(
            self.graphical.location_x,
            self.graphical.location_y,
        ))
    }

    fn record_type_name(&self) -> &'static str {
        "Image"
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
        CoordRect::from_points(
            Coord::from_raw(self.graphical.location_x),
            Coord::from_raw(self.graphical.location_y),
            Coord::from_raw(self.corner_x),
            Coord::from_raw(self.corner_y),
        )
    }
}
