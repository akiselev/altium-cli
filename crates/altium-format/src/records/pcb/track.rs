//! PCB Track record.
//!
//! **DEPRECATED**: Use `v2::pcb::PcbTrackV2` with `v2::pcb::io` instead.

use altium_format_derive::AltiumRecord;

use super::primitive::PcbPrimitiveCommon;
use crate::types::{Coord, CoordPoint, CoordRect, Layer};

/// PCB Track (line segment) primitive.
///
/// **DEPRECATED**: Use `v2::pcb::PcbTrackV2` instead.
#[deprecated(note = "Use v2::pcb::PcbTrackV2")]
#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(format = "binary")]
pub struct PcbTrack {
    /// Common primitive fields.
    #[altium(flatten)]
    pub common: PcbPrimitiveCommon,
    /// Start point.
    #[altium(coord_point)]
    pub start: CoordPoint,
    /// End point.
    #[altium(coord_point)]
    pub end: CoordPoint,
    /// Line width.
    #[altium(coord)]
    pub width: Coord,
    /// Unknown binary data at the end of the record (16 bytes).
    ///
    /// Based on analysis of RFSoC_AMC.PcbDoc:
    /// - All tracks have exactly 16 bytes
    /// - Likely contains net ID, design rule references, and layer preferences
    /// - Most common pattern: [00 00 00 00 00 00 00 00 07 00 03 01 00 00 00 00]
    /// - Safe default: 16 bytes of zeros (lets Altium auto-assign values)
    #[altium(unknown_binary)]
    pub unknown: Vec<u8>,
}

impl PcbTrack {
    /// Create a new track segment.
    pub fn new(start: CoordPoint, end: CoordPoint, width: Coord, layer: Layer) -> Self {
        use super::primitive::PcbFlags;
        Self {
            common: PcbPrimitiveCommon {
                layer,
                flags: PcbFlags::default(),
                unique_id: None,
            },
            start,
            end,
            width,
            unknown: vec![0u8; 16],
        }
    }

    /// Create a track from coordinates in mils.
    pub fn from_mils(
        start_x: f64,
        start_y: f64,
        end_x: f64,
        end_y: f64,
        width: f64,
        layer: Layer,
    ) -> Self {
        Self::new(
            CoordPoint::from_mils(start_x, start_y),
            CoordPoint::from_mils(end_x, end_y),
            Coord::from_mils(width),
            layer,
        )
    }

    /// Create a track from coordinates in mm.
    pub fn from_mms(
        start_x: f64,
        start_y: f64,
        end_x: f64,
        end_y: f64,
        width: f64,
        layer: Layer,
    ) -> Self {
        Self::new(
            CoordPoint::from_mms(start_x, start_y),
            CoordPoint::from_mms(end_x, end_y),
            Coord::from_mms(width),
            layer,
        )
    }

    /// Calculate the bounding rectangle.
    pub fn calculate_bounds(&self) -> CoordRect {
        CoordRect::from_corners(self.start, self.end)
    }

    /// Get the length of the track in raw units.
    pub fn length(&self) -> Coord {
        let dx = (self.end.x.to_raw() - self.start.x.to_raw()) as f64;
        let dy = (self.end.y.to_raw() - self.start.y.to_raw()) as f64;
        Coord::from_raw((dx * dx + dy * dy).sqrt() as i32)
    }
}
