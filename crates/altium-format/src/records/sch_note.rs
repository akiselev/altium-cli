//! Note record (RECORD=209).

use crate::coord::SchCoord;
use altium_format_derive::altium_record;

/// Note record — annotative note box on a schematic.
///
/// Corresponds to `NoteData` / `ExportNote` in the v1 API (ObjectId::Note = 209).
#[altium_record(kind = "sch", record_id = 209, codec = "params")]
pub struct SchNoteRecord {
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

    // --- Note-specific fields ---
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
    #[altium(key = "COLLAPSED")]
    collapsed: bool,
    #[altium(key = "AUTHOR")]
    author: String,
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
            "|RECORD=209|OWNERINDEX=0|LOCATION.X=100|LOCATION.Y=200|CORNER.X=500|CORNER.Y=600|LINEWIDTH=1|COLOR=0|AREACOLOR=16777215|TEXTCOLOR=0|FONTID=1|ISSOLID=T|SHOWBORDER=T|ALIGNMENT=0|WORDWRAP=T|CLIPTORECT=T|TEXT=This is a note|COLLAPSED=F|AUTHOR=Engineer|UNIQUEID=ABCD1234|",
        ));
        let rec = SchNoteRecord::from_origin(origin);
        assert_eq!(rec.text(), "This is a note");
        assert_eq!(rec.author(), "Engineer");
        assert!(rec.is_solid());
        assert!(rec.show_border());
        assert!(!rec.collapsed());
    }

    #[test]
    fn setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=209|TEXT=Old note|"));
        let mut rec = SchNoteRecord::from_origin(origin);
        rec.set_text("Updated note".to_string());
        assert_eq!(rec.text(), "Updated note");
        rec.set_collapsed(true);
        assert!(rec.collapsed());
    }
}
