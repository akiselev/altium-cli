//! Schematic rectangle record (RECORD=14).

use altium_format_derive::altium_record;
use crate::v2::coord::SchCoord;
use crate::v2::newtypes::UniqueId;
use super::enums::*;

/// Schematic rectangle record -- RECORD=14.
///
/// Represents a rectangle primitive on a schematic sheet.
#[altium_record(kind = "sch", record_id = 14, codec = "params")]
pub struct SchRectangleRecord {
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

    // --- Rectangle-specific fields ---
    #[altium(key = "Location.X")]
    location_x: SchCoord,

    #[altium(key = "Location.Y")]
    location_y: SchCoord,

    #[altium(key = "Corner.X")]
    corner_x: SchCoord,

    #[altium(key = "Corner.Y")]
    corner_y: SchCoord,

    #[altium(key = "LineStyleExt")]
    line_style: LineStyle,

    #[altium(key = "LineWidth")]
    line_width: Size,

    #[altium(key = "Color")]
    color: u32,

    #[altium(key = "AreaColor")]
    area_color: u32,

    #[altium(key = "IsSolid")]
    is_solid: bool,

    #[altium(key = "Transparent")]
    transparent: bool,

    #[altium(key = "UniqueID")]
    unique_id: UniqueId,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::backing_store::{ParamOrigin, RecordOrigin};

    #[test]
    fn roundtrip_rectangle_getter() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=14|Location.X=10|Location.Y=20|Corner.X=100|Corner.Y=200|IsSolid=T|Transparent=F|Color=255|AreaColor=16777215|LineWidth=1|",
        ));
        let rec = SchRectangleRecord::from_origin(origin);
        assert!(rec.is_solid());
        assert!(!rec.transparent());
        assert_eq!(rec.color(), 255);
        assert_eq!(rec.area_color(), 16777215);
    }

    #[test]
    fn roundtrip_rectangle_setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=14|Location.X=10|Location.Y=20|Corner.X=100|Corner.Y=200|IsSolid=F|",
        ));
        let mut rec = SchRectangleRecord::from_origin(origin);
        rec.set_is_solid(true);
        assert!(rec.is_solid());
        rec.set_transparent(true);
        assert!(rec.transparent());
    }
}
