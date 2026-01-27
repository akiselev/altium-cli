//! PCB Fill record.

use altium_derive::AltiumRecord;

use super::primitive::PcbRectangularBase;
use crate::types::{Coord, CoordRect};

/// PCB Fill (solid rectangle) primitive.
#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(format = "binary")]
pub struct PcbFill {
    /// Base rectangular fields.
    #[altium(flatten)]
    pub base: PcbRectangularBase,
}

impl PcbFill {
    /// Width of the fill.
    pub fn width(&self) -> Coord {
        self.base.width()
    }

    /// Height of the fill.
    pub fn height(&self) -> Coord {
        self.base.height()
    }

    /// Calculate the bounding rectangle.
    pub fn calculate_bounds(&self) -> CoordRect {
        self.base.calculate_bounds()
    }
}
