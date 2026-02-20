//! Schematic line record (RECORD=13).

use super::enums::*;
use crate::coord::SchCoord;
use crate::newtypes::UniqueId;
use altium_format_derive::altium_record;

/// Schematic line record -- RECORD=13.
///
/// Represents a line primitive on a schematic sheet.
#[altium_record(kind = "sch", record_id = 13, codec = "params")]
pub struct SchLineRecord {
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

    // --- Line-specific fields ---
    #[altium(key = "Location.X")]
    location_x: SchCoord,

    #[altium(key = "Location.Y")]
    location_y: SchCoord,

    #[altium(key = "Corner.X")]
    corner_x: SchCoord,

    #[altium(key = "Corner.Y")]
    corner_y: SchCoord,

    #[altium(key = "LineWidth")]
    line_width: Size,

    #[altium(key = "LineStyle")]
    line_style: LineStyle,

    #[altium(key = "Color")]
    color: u32,

    #[altium(key = "UniqueID")]
    unique_id: UniqueId,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backing_store::{ParamOrigin, RecordOrigin};

    #[test]
    fn roundtrip_line_getter() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=13|Location.X=10|Location.Y=20|Corner.X=100|Corner.Y=200|LineWidth=2|LineStyle=1|Color=255|",
        ));
        let rec = SchLineRecord::from_origin(origin);
        assert_eq!(rec.line_width(), Size::Medium);
        assert_eq!(rec.line_style(), LineStyle::Dashed);
        assert_eq!(rec.color(), 255);
    }

    #[test]
    fn roundtrip_line_setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=13|Location.X=10|Location.Y=20|Corner.X=100|Corner.Y=200|Color=255|",
        ));
        let mut rec = SchLineRecord::from_origin(origin);
        rec.set_line_style(LineStyle::Dotted);
        assert_eq!(rec.line_style(), LineStyle::Dotted);
    }
}
