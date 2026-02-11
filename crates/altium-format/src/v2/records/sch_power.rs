//! Power object record (RECORD=17).

use altium_format_derive::altium_record;
use crate::v2::coord::SchCoord;

/// Power object record — power rail symbol (VCC, GND, etc.).
///
/// Corresponds to `PowerData` / `ExportPower` in the v1 API (ObjectId::PowerObject = 17).
#[altium_record(kind = "sch", record_id = 17, codec = "params")]
pub struct SchPowerRecord {
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

    // --- Power-specific fields ---
    /// Power object style (0=Circle, 1=Arrow, ..., 10=GOSTBar).
    #[altium(key = "STYLE")]
    style: i32,
    #[altium(key = "SHOWNETNAME")]
    show_net_name: bool,
    #[altium(key = "LOCATION.X")]
    location_x: SchCoord,
    #[altium(key = "LOCATION.Y")]
    location_y: SchCoord,
    /// Rotation in 90-degree increments (0-3).
    #[altium(key = "ORIENTATION")]
    orientation: i32,
    #[altium(key = "COLOR")]
    color: u32,
    #[altium(key = "FONTID")]
    font_id: i32,
    #[altium(key = "TEXT")]
    text: String,
    #[altium(key = "ISCROSSSHEETCONNECTOR")]
    is_cross_sheet_connector: bool,
    #[altium(key = "UNIQUEID")]
    unique_id: String,
    #[altium(key = "OBJECTDEFINITIONID")]
    object_definition_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::backing_store::{ParamOrigin, RecordOrigin};

    #[test]
    fn roundtrip() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=17|OWNERINDEX=1|OWNERPARTID=1|STYLE=4|SHOWNETNAME=T|LOCATION.X=100|LOCATION.Y=200|ORIENTATION=0|COLOR=128|TEXT=GND|UNIQUEID=ABCD1234|",
        ));
        let rec = SchPowerRecord::from_origin(origin);
        assert_eq!(rec.owner_index(), 1);
        assert_eq!(rec.style(), 4);
        assert!(rec.show_net_name());
        assert_eq!(rec.text(), "GND");
    }

    #[test]
    fn setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=17|TEXT=VCC|"));
        let mut rec = SchPowerRecord::from_origin(origin);
        rec.set_text("GND".to_string());
        assert_eq!(rec.text(), "GND");
        rec.set_style(1);
        assert_eq!(rec.style(), 1);
    }
}
