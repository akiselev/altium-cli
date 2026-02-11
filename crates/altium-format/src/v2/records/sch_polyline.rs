//! Schematic polyline record (RECORD=6).

use altium_format_derive::altium_record;
use crate::v2::newtypes::UniqueId;
use super::enums::*;

/// Schematic polyline record -- RECORD=6.
///
/// Represents a polyline primitive on a schematic sheet.
/// Vertex data is skipped for now (handled in later phases).
#[altium_record(kind = "sch", record_id = 6, codec = "params")]
pub struct SchPolylineRecord {
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

    // --- Polyline-specific fields ---
    #[altium(key = "LineWidth")]
    line_width: Size,

    #[altium(key = "LineStyle")]
    line_style: LineStyle,

    #[altium(key = "StartLineShape")]
    start_line_shape: LineShape,

    #[altium(key = "EndLineShape")]
    end_line_shape: LineShape,

    #[altium(key = "LineShapeSize")]
    line_shape_size: Size,

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
    fn roundtrip_polyline_getter() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=6|LineWidth=1|LineStyle=2|StartLineShape=1|EndLineShape=2|LineShapeSize=1|Color=128|",
        ));
        let rec = SchPolylineRecord::from_origin(origin);
        assert_eq!(rec.line_width(), Size::Small);
        assert_eq!(rec.line_style(), LineStyle::Dotted);
        assert_eq!(rec.start_line_shape(), LineShape::Arrow);
        assert_eq!(rec.end_line_shape(), LineShape::SolidArrow);
        assert_eq!(rec.color(), 128);
    }

    #[test]
    fn roundtrip_polyline_setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=6|LineWidth=1|Color=255|",
        ));
        let mut rec = SchPolylineRecord::from_origin(origin);
        rec.set_start_line_shape(LineShape::SolidArrow);
        assert_eq!(rec.start_line_shape(), LineShape::SolidArrow);
    }
}
