//! Schematic elliptical arc record (RECORD=11).

use super::enums::*;
use crate::v2::coord::SchCoord;
use crate::v2::newtypes::UniqueId;
use altium_format_derive::altium_record;

/// Schematic elliptical arc record -- RECORD=11.
///
/// Represents an elliptical arc primitive on a schematic sheet.
#[altium_record(kind = "sch", record_id = 11, codec = "params")]
pub struct SchEllipticalArcRecord {
    // --- Base object fields (flattened from GraphicalObjectBase) ---
    #[altium(key = "OwnerIndex")]
    owner_index: i32,

    #[altium(key = "OwnerPartId")]
    owner_part_id: i16,

    #[altium(key = "OwnerPartDisplayMode")]
    owner_part_display_mode: u8,

    #[altium(key = "IndexInSheet")]
    index_in_sheet: i32,

    #[altium(key = "IsNotAccesible")]
    is_not_accessible: bool,

    #[altium(key = "GraphicallyLocked")]
    graphically_locked: bool,

    // --- EllipticalArc-specific fields ---
    #[altium(key = "Location.X")]
    location_x: SchCoord,

    #[altium(key = "Location.Y")]
    location_y: SchCoord,

    #[altium(key = "Radius")]
    radius: SchCoord,

    #[altium(key = "SecondaryRadius")]
    secondary_radius: SchCoord,

    #[altium(key = "LineWidth")]
    line_width: Size,

    #[altium(key = "StartAngle")]
    start_angle: f64,

    #[altium(key = "EndAngle")]
    end_angle: f64,

    #[altium(key = "Color")]
    color: u32,

    #[altium(key = "UniqueID")]
    unique_id: UniqueId,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::backing_store::{ParamOrigin, RecordOrigin};

    #[test]
    fn roundtrip_elliptical_arc_getter() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=11|Location.X=100|Location.Y=200|Radius=50|SecondaryRadius=30|StartAngle=0.000000|EndAngle=180.000000|Color=255|LineWidth=1|",
        ));
        let rec = SchEllipticalArcRecord::from_origin(origin);
        assert_eq!(rec.color(), 255);
        assert_eq!(rec.line_width(), Size::Small);
    }

    #[test]
    fn roundtrip_elliptical_arc_setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=11|Location.X=100|Location.Y=200|Radius=50|Color=255|",
        ));
        let mut rec = SchEllipticalArcRecord::from_origin(origin);
        rec.set_color(128);
        assert_eq!(rec.color(), 128);
    }
}
