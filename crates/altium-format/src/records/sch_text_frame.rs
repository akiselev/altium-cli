//! Text frame record (RECORD=28).

use crate::coord::SchCoord;
use altium_format_derive::altium_record;

/// Text frame record — bordered text region on a schematic.
///
/// Corresponds to `TextFrameData` / `ExportTextFrame` in the v1 API (ObjectId::TextFrame = 28).
#[altium_record(kind = "sch", record_id = 28, codec = "params")]
pub struct SchTextFrameRecord {
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

    // --- TextFrame-specific fields ---
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
    #[altium(key = "TEXTCOLOR")]
    text_color: u32,
    #[altium(key = "FONTID")]
    font_id: i32,
    #[altium(key = "ISSOLID")]
    is_solid: bool,
    #[altium(key = "SHOWBORDER")]
    show_border: bool,
    /// Horizontal alignment (0=Left, 1=Center, 2=Right).
    #[altium(key = "ALIGNMENT")]
    alignment: i32,
    #[altium(key = "WORDWRAP")]
    word_wrap: bool,
    #[altium(key = "CLIPTORECT")]
    clip_to_rect: bool,
    #[altium(key = "TEXT")]
    text: String,
    #[altium(key = "TEXTMARGIN")]
    text_margin: SchCoord,
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
            "|RECORD=28|OWNERINDEX=0|LOCATION.X=100|LOCATION.Y=200|CORNER.X=500|CORNER.Y=600|LINEWIDTH=1|COLOR=0|AREACOLOR=16777215|TEXTCOLOR=0|FONTID=1|ISSOLID=T|SHOWBORDER=T|ALIGNMENT=0|WORDWRAP=T|CLIPTORECT=T|TEXT=Hello World|UNIQUEID=ABCD1234|",
        ));
        let rec = SchTextFrameRecord::from_origin(origin);
        assert_eq!(rec.text().unwrap(), "Hello World");
        assert!(rec.is_solid().unwrap());
        assert!(rec.show_border().unwrap());
        assert!(rec.word_wrap().unwrap());
    }

    #[test]
    fn setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=28|TEXT=Old|"));
        let mut rec = SchTextFrameRecord::from_origin(origin);
        rec.set_text("New".to_string());
        assert_eq!(rec.text().unwrap(), "New");
        rec.set_is_solid(false);
        assert!(!rec.is_solid().unwrap());
    }
}
