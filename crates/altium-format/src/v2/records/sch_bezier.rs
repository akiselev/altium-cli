//! Schematic bezier record (RECORD=5).

use super::enums::*;
use crate::v2::newtypes::UniqueId;
use altium_format_derive::altium_record;

/// Schematic bezier curve record -- RECORD=5.
///
/// Represents a bezier curve primitive on a schematic sheet.
/// Vertex data is skipped for now (handled in later phases).
#[altium_record(kind = "sch", record_id = 5, codec = "params")]
pub struct SchBezierRecord {
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

    // --- Bezier-specific fields ---
    #[altium(key = "LineWidth")]
    line_width: Size,

    #[altium(key = "Color")]
    color: u32,

    // Vertices are skipped for now -- handled in later phases
    // vertices: Vec<(SchCoord, SchCoord)>,
    #[altium(key = "UniqueID")]
    unique_id: UniqueId,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::backing_store::{ParamOrigin, RecordOrigin};

    #[test]
    fn roundtrip_bezier_getter() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=5|LineWidth=2|Color=255|UniqueID=ABCD1234|",
        ));
        let rec = SchBezierRecord::from_origin(origin);
        assert_eq!(rec.line_width(), Size::Medium);
        assert_eq!(rec.color(), 255);
        assert_eq!(rec.unique_id(), UniqueId::from("ABCD1234"));
    }

    #[test]
    fn roundtrip_bezier_setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=5|LineWidth=1|Color=255|"));
        let mut rec = SchBezierRecord::from_origin(origin);
        rec.set_line_width(Size::Large);
        assert_eq!(rec.line_width(), Size::Large);
    }
}
