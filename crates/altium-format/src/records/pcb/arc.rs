//! PCB Arc record.

use altium_format_derive::AltiumRecord;

use super::primitive::PcbPrimitiveCommon;
use crate::types::{Coord, CoordPoint, CoordRect, Layer};

/// PCB Arc primitive.
#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(format = "binary")]
pub struct PcbArc {
    /// Common primitive fields.
    #[altium(flatten)]
    pub common: PcbPrimitiveCommon,
    /// Center location.
    #[altium(coord_point)]
    pub location: CoordPoint,
    /// Radius.
    #[altium(coord)]
    pub radius: Coord,
    /// Start angle in degrees.
    pub start_angle: f64,
    /// End angle in degrees.
    pub end_angle: f64,
    /// Line width.
    #[altium(coord)]
    pub width: Coord,
}

impl PcbArc {
    /// Create a new arc.
    pub fn new(
        center: CoordPoint,
        radius: Coord,
        start_angle: f64,
        end_angle: f64,
        width: Coord,
        layer: Layer,
    ) -> Self {
        use super::primitive::PcbFlags;
        Self {
            common: PcbPrimitiveCommon {
                layer,
                flags: PcbFlags::default(),
                unique_id: None,
            },
            location: center,
            radius,
            start_angle,
            end_angle,
            width,
        }
    }

    /// Create a full circle.
    pub fn circle(center: CoordPoint, radius: Coord, width: Coord, layer: Layer) -> Self {
        Self::new(center, radius, 0.0, 360.0, width, layer)
    }

    /// Create an arc from mm values.
    pub fn from_mms(
        center_x: f64,
        center_y: f64,
        radius: f64,
        start_angle: f64,
        end_angle: f64,
        width: f64,
        layer: Layer,
    ) -> Self {
        Self::new(
            CoordPoint::from_mms(center_x, center_y),
            Coord::from_mms(radius),
            start_angle,
            end_angle,
            Coord::from_mms(width),
            layer,
        )
    }

    /// Create an arc from mil values.
    pub fn from_mils(
        center_x: f64,
        center_y: f64,
        radius: f64,
        start_angle: f64,
        end_angle: f64,
        width: f64,
        layer: Layer,
    ) -> Self {
        Self::new(
            CoordPoint::from_mils(center_x, center_y),
            Coord::from_mils(radius),
            start_angle,
            end_angle,
            Coord::from_mils(width),
            layer,
        )
    }

    /// Calculate the bounding rectangle.
    pub fn calculate_bounds(&self) -> CoordRect {
        // Simplified: assumes full circle bounds
        let r = self.radius.to_raw();
        CoordRect::from_raw(
            self.location.x.to_raw() - r,
            self.location.y.to_raw() - r,
            r * 2,
            r * 2,
        )
    }

    /// Get the arc sweep angle.
    pub fn sweep_angle(&self) -> f64 {
        let mut sweep = self.end_angle - self.start_angle;
        if sweep < 0.0 {
            sweep += 360.0;
        }
        sweep
    }

    /// Check if this is a full circle.
    pub fn is_circle(&self) -> bool {
        (self.sweep_angle() - 360.0).abs() < 0.001
    }
}
