//! Label record (RECORD=4).

use crate::coord::SchCoord;
use altium_format_derive::altium_record;

/// Label record — text label on a schematic.
///
/// Corresponds to `LabelData` / `ExportLabel` in the v1 API (ObjectId::Label = 4).
#[altium_record(kind = "sch", record_id = 4, codec = "params")]
pub struct SchLabelRecord {
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

    // --- Label-specific fields ---
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
    #[altium(key = "URL")]
    url: String,
    #[altium(key = "UNIQUEID")]
    unique_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backing_store::{ParamOrigin, RecordOrigin};

    #[test]
    fn roundtrip() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=4|OWNERINDEX=0|OWNERPARTID=1|LOCATION.X=100|LOCATION.Y=200|ORIENTATION=1|JUSTIFICATION=0|COLOR=0|FONTID=1|TEXT=Hello|ISMIRRORED=F|UNIQUEID=ABCD1234|",
        ));
        let rec = SchLabelRecord::from_origin(origin);
        assert_eq!(rec.owner_index().unwrap(), 0);
        assert_eq!(rec.text().unwrap(), "Hello");
        assert_eq!(rec.font_id().unwrap(), 1);
        assert!(!rec.is_mirrored().unwrap());
    }

    #[test]
    fn setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=4|TEXT=Old|"));
        let mut rec = SchLabelRecord::from_origin(origin);
        rec.set_text("New".to_string());
        assert_eq!(rec.text().unwrap(), "New");
    }
}
