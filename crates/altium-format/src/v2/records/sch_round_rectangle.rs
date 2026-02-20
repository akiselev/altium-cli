//! Schematic round rectangle record (RECORD=10).

use super::enums::*;
use crate::v2::coord::SchCoord;
use crate::v2::newtypes::UniqueId;
use altium_format_derive::altium_record;

/// Schematic round rectangle record -- RECORD=10.
///
/// Represents a rounded rectangle primitive on a schematic sheet.
#[altium_record(kind = "sch", record_id = 10, codec = "params")]
pub struct SchRoundRectangleRecord {
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

    // --- RoundRectangle-specific fields ---
    #[altium(key = "Location.X")]
    location_x: SchCoord,

    #[altium(key = "Location.Y")]
    location_y: SchCoord,

    #[altium(key = "Corner.X")]
    corner_x: SchCoord,

    #[altium(key = "Corner.Y")]
    corner_y: SchCoord,

    #[altium(key = "CornerXRadius")]
    corner_x_radius: SchCoord,

    #[altium(key = "CornerYRadius")]
    corner_y_radius: SchCoord,

    #[altium(key = "LineWidth")]
    line_width: Size,

    #[altium(key = "Color")]
    color: u32,

    #[altium(key = "AreaColor")]
    area_color: u32,

    #[altium(key = "IsSolid")]
    is_solid: bool,

    #[altium(key = "UniqueID")]
    unique_id: UniqueId,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::backing_store::{ParamOrigin, RecordOrigin};

    #[test]
    fn roundtrip_round_rectangle_getter() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=10|Location.X=10|Location.Y=20|Corner.X=100|Corner.Y=200|CornerXRadius=5|CornerYRadius=5|IsSolid=T|Color=255|AreaColor=128|",
        ));
        let rec = SchRoundRectangleRecord::from_origin(origin);
        assert!(rec.is_solid());
        assert_eq!(rec.color(), 255);
    }

    #[test]
    fn roundtrip_round_rectangle_setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=10|Location.X=10|Location.Y=20|Corner.X=100|Corner.Y=200|IsSolid=F|Color=255|",
        ));
        let mut rec = SchRoundRectangleRecord::from_origin(origin);
        rec.set_is_solid(true);
        assert!(rec.is_solid());
    }
}
