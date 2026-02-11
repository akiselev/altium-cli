//! Blanket record (RECORD=215).

use altium_format_derive::altium_record;
use crate::v2::coord::SchCoord;

/// Blanket record — region overlay for grouping/annotation on a schematic.
///
/// Corresponds to `BlanketData` / `ExportBlanket` in the v1 API (ObjectId::Blanket = 215).
///
/// Note: vertices (`Vec<(i32,i32)>`) are skipped in this phase and will be
/// handled with custom codec logic in a later phase.
#[altium_record(kind = "sch", record_id = 215, codec = "params")]
pub struct SchBlanketRecord {
    // --- GraphicalObjectBase (flattened) ---
    #[altium(key = "OWNERINDEX")]
    owner_index: i32,
    #[altium(key = "OWNERPARTID")]
    owner_part_id: i16,
    #[altium(key = "OWNERPARTDISPLAYMODE")]
    owner_part_display_mode: i32,
    #[altium(key = "INDEXINSHEET")]
    index_in_sheet: i32,
    #[altium(key = "ISNOTACCESIBLE")]
    is_not_accessible: bool,
    #[altium(key = "GRAPHICALLYLOCKED")]
    graphically_locked: bool,

    // --- Blanket-specific fields ---
    #[altium(key = "LOCATION.X")]
    location_x: SchCoord,
    #[altium(key = "LOCATION.Y")]
    location_y: SchCoord,
    #[altium(key = "CORNER.X")]
    corner_x: SchCoord,
    #[altium(key = "CORNER.Y")]
    corner_y: SchCoord,
    /// Line width (0=Smallest, 1=Small, 2=Medium, 3=Large).
    #[altium(key = "LINEWIDTH")]
    line_width: i32,
    #[altium(key = "COLOR")]
    color: u32,
    #[altium(key = "AREACOLOR")]
    area_color: u32,
    #[altium(key = "COLLAPSED")]
    collapsed: bool,
    /// Line style (0=Solid, 1=Dashed, 2=Dotted, 3=DashDotted).
    #[altium(key = "LINESTYLE")]
    line_style: i32,
    #[altium(key = "UNIQUEID")]
    unique_id: String,

    /// Vertex coordinates — skipped; handled in later phase.
    #[altium(skip)]
    _vertices: i32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::backing_store::{ParamOrigin, RecordOrigin};

    #[test]
    fn roundtrip() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=215|OWNERINDEX=0|LOCATION.X=100|LOCATION.Y=200|CORNER.X=500|CORNER.Y=600|LINEWIDTH=1|COLOR=0|AREACOLOR=16777215|COLLAPSED=F|LINESTYLE=0|UNIQUEID=ABCD1234|",
        ));
        let rec = SchBlanketRecord::from_origin(origin);
        assert_eq!(rec.line_width(), 1);
        assert_eq!(rec.line_style(), 0);
        assert!(!rec.collapsed());
        assert_eq!(rec.unique_id(), "ABCD1234");
    }

    #[test]
    fn setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=215|LINESTYLE=0|"));
        let mut rec = SchBlanketRecord::from_origin(origin);
        rec.set_line_style(1);
        assert_eq!(rec.line_style(), 1);
        rec.set_collapsed(true);
        assert!(rec.collapsed());
    }
}
