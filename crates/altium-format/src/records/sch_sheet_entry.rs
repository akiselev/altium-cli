//! Sheet entry record (RECORD=40).

use crate::coord::SchCoord;
use altium_format_derive::altium_record;

/// Sheet entry record — port-like entry on a sheet symbol.
///
/// Corresponds to `SheetEntryData` / `ExportSheetEntry` in the v1 API
/// (ObjectId::SheetEntry = 40).
///
/// Includes the flattened `BasicEntryObjectBase` fields (which itself
/// flattens `GraphicalObjectBase`).
#[altium_record(kind = "sch", record_id = 40, codec = "params")]
pub struct SchSheetEntryRecord {
    // --- GraphicalObjectBase (flattened from BasicEntryObjectBase) ---
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

    // --- BasicEntryObjectBase fields ---
    /// Side (0=Left, 1=Right).
    #[altium(key = "SIDE")]
    side: i32,
    #[altium(key = "DISTANCEFROMTOP")]
    distance_from_top: SchCoord,
    #[altium(key = "COLOR")]
    color: u32,
    #[altium(key = "AREACOLOR")]
    area_color: u32,
    #[altium(key = "TEXTCOLOR")]
    text_color: u32,
    #[altium(key = "TEXTFONTID")]
    text_font_id: i32,
    #[altium(key = "TEXTSTYLE")]
    text_style: String,
    #[altium(key = "NAME")]
    name: String,
    #[altium(key = "HARNESSTYPE")]
    harness_type: String,
    #[altium(key = "UNIQUEID")]
    unique_id: String,

    // --- SheetEntry-specific fields ---
    /// Port I/O type (0=Unspecified, 1=Output, 2=Input, 3=Bidirectional).
    #[altium(key = "IOTYPE")]
    io_type: i32,
    /// Port arrow style (0-7).
    #[altium(key = "STYLE")]
    style: i32,
    #[altium(key = "ARROWKIND")]
    arrow_kind: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backing_store::{ParamOrigin, RecordOrigin};

    #[test]
    fn roundtrip() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=40|OWNERINDEX=1|SIDE=0|DISTANCEFROMTOP=50|COLOR=0|AREACOLOR=16777215|TEXTCOLOR=0|TEXTFONTID=1|NAME=DataIn|IOTYPE=2|STYLE=1|UNIQUEID=ABCD1234|",
        ));
        let rec = SchSheetEntryRecord::from_origin(origin);
        assert_eq!(rec.owner_index(), 1);
        assert_eq!(rec.name(), "DataIn");
        assert_eq!(rec.io_type(), 2);
        assert_eq!(rec.style(), 1);
    }

    #[test]
    fn setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=40|NAME=OldEntry|"));
        let mut rec = SchSheetEntryRecord::from_origin(origin);
        rec.set_name("NewEntry".to_string());
        assert_eq!(rec.name(), "NewEntry");
        rec.set_io_type(3);
        assert_eq!(rec.io_type(), 3);
    }
}
