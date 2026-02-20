//! Sheet file name record (RECORD=33).

use crate::v2::coord::SchCoord;
use altium_format_derive::altium_record;

/// Sheet file name record — displays the file name for a sheet symbol.
///
/// Corresponds to `SheetFileNameData` / `ExportSheetFileName` in the v1 API
/// (ObjectId::SheetFileName = 33).
#[altium_record(kind = "sch", record_id = 33, codec = "params")]
pub struct SchSheetFileNameRecord {
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

    // --- SheetFileName-specific fields ---
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
    #[altium(key = "ISHIDDEN")]
    is_hidden: bool,
    #[altium(key = "TEXT")]
    text: String,
    #[altium(key = "ISMIRRORED")]
    is_mirrored: bool,
    /// Note: stored as inverted in file (NotAutoPosition).
    #[altium(key = "NOTAUTOPOSITION")]
    not_auto_position: bool,
    /// Text horizontal anchor (0=None, 1=Left, 2=Center, 3=Right).
    #[altium(key = "TEXTHORZANCHOR")]
    text_horz_anchor: i32,
    /// Text vertical anchor (0=None, 1=Top, 2=Center, 3=Bottom).
    #[altium(key = "TEXTVERTANCHOR")]
    text_vert_anchor: i32,
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
            "|RECORD=33|OWNERINDEX=1|LOCATION.X=100|LOCATION.Y=200|ORIENTATION=0|JUSTIFICATION=0|COLOR=0|FONTID=1|ISHIDDEN=F|TEXT=Sheet1.SchDoc|ISMIRRORED=F|NOTAUTOPOSITION=F|UNIQUEID=ABCD1234|",
        ));
        let rec = SchSheetFileNameRecord::from_origin(origin);
        assert_eq!(rec.text(), "Sheet1.SchDoc");
        assert!(!rec.is_hidden());
    }

    #[test]
    fn setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=33|TEXT=Old.SchDoc|"));
        let mut rec = SchSheetFileNameRecord::from_origin(origin);
        rec.set_text("New.SchDoc".to_string());
        assert_eq!(rec.text(), "New.SchDoc");
    }
}
