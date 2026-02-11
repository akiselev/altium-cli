//! Net label record (RECORD=25).

use altium_format_derive::altium_record;
use crate::v2::coord::SchCoord;

/// Net label record — assigns a net name to a wire/bus connection point.
///
/// Corresponds to `NetLabelData` / `ExportNetLabel` in the v1 API (ObjectId::NetLabel = 25).
#[altium_record(kind = "sch", record_id = 25, codec = "params")]
pub struct SchNetLabelRecord {
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

    // --- NetLabel-specific fields ---
    #[altium(key = "LOCATION.X")]
    location_x: SchCoord,
    #[altium(key = "LOCATION.Y")]
    location_y: SchCoord,
    /// Rotation in 90-degree increments (0-3).
    #[altium(key = "ORIENTATION")]
    orientation: i32,
    /// Text justification (0-8).
    #[altium(key = "JUSTIFICATION")]
    justification: i32,
    #[altium(key = "COLOR")]
    color: u32,
    #[altium(key = "FONTID")]
    font_id: i32,
    #[altium(key = "TEXT")]
    text: String,
    #[altium(key = "ISMIRRORED")]
    is_mirrored: bool,
    #[altium(key = "UNIQUEID")]
    unique_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::backing_store::{ParamOrigin, RecordOrigin};

    #[test]
    fn roundtrip() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=25|OWNERINDEX=0|OWNERPARTID=1|LOCATION.X=100|LOCATION.Y=200|ORIENTATION=0|JUSTIFICATION=0|COLOR=0|FONTID=1|TEXT=VCC|ISMIRRORED=F|UNIQUEID=ABCD1234|",
        ));
        let rec = SchNetLabelRecord::from_origin(origin);
        assert_eq!(rec.text(), "VCC");
        assert_eq!(rec.font_id(), 1);
        assert!(!rec.is_mirrored());
    }

    #[test]
    fn setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=25|TEXT=NET1|"));
        let mut rec = SchNetLabelRecord::from_origin(origin);
        rec.set_text("NET2".to_string());
        assert_eq!(rec.text(), "NET2");
    }
}
