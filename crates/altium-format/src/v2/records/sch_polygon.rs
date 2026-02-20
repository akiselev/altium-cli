//! Schematic polygon record (RECORD=7).

use super::enums::*;
use crate::v2::newtypes::UniqueId;
use altium_format_derive::altium_record;

/// Schematic polygon record -- RECORD=7.
///
/// Represents a polygon primitive on a schematic sheet.
/// Vertex data is skipped for now (handled in later phases).
#[altium_record(kind = "sch", record_id = 7, codec = "params")]
pub struct SchPolygonRecord {
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

    // --- Polygon-specific fields ---
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
    fn roundtrip_polygon_getter() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=7|LineWidth=1|Color=255|AreaColor=16777215|IsSolid=T|Transparent=F|",
        ));
        let rec = SchPolygonRecord::from_origin(origin);
        assert!(rec.is_solid());
        assert!(!rec.transparent());
        assert_eq!(rec.color(), 255);
        assert_eq!(rec.area_color(), 16777215);
    }

    #[test]
    fn roundtrip_polygon_setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=7|LineWidth=1|Color=255|IsSolid=F|",
        ));
        let mut rec = SchPolygonRecord::from_origin(origin);
        rec.set_is_solid(true);
        assert!(rec.is_solid());
    }
}
