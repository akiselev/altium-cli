//! Bus entry record (RECORD=37).

use crate::coord::SchCoord;
use altium_format_derive::altium_record;

/// Bus entry record — connection point between a bus and a wire.
///
/// Corresponds to `BusEntryData` / `ExportBusEntry` in the v1 API (ObjectId::BusEntry = 37).
#[altium_record(kind = "sch", record_id = 37, codec = "params")]
pub struct SchBusEntryRecord {
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

    // --- BusEntry-specific fields ---
    #[altium(key = "UNIQUEID")]
    unique_id: String,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backing_store::{ParamOrigin, RecordOrigin};

    #[test]
    fn roundtrip() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=37|OWNERINDEX=0|LOCATION.X=100|LOCATION.Y=200|CORNER.X=120|CORNER.Y=220|LINEWIDTH=1|COLOR=8388608|UNIQUEID=ABCD1234|",
        ));
        let rec = SchBusEntryRecord::from_origin(origin);
        assert_eq!(rec.line_width().unwrap(), 1);
        assert_eq!(rec.color().unwrap(), 8388608);
        assert_eq!(rec.unique_id().unwrap(), "ABCD1234");
    }

    #[test]
    fn setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=37|LINEWIDTH=1|"));
        let mut rec = SchBusEntryRecord::from_origin(origin);
        rec.set_line_width(2);
        assert_eq!(rec.line_width().unwrap(), 2);
    }
}
