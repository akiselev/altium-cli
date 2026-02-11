//! Port record (RECORD=18).

use altium_format_derive::altium_record;
use crate::v2::coord::SchCoord;

/// Port record — hierarchical port connection on a schematic.
///
/// Corresponds to `PortData` / `ExportPort` in the v1 API (ObjectId::Port = 18).
#[altium_record(kind = "sch", record_id = 18, codec = "params")]
pub struct SchPortRecord {
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

    // --- Port-specific fields ---
    /// Port arrow style (0-7).
    #[altium(key = "STYLE")]
    style: i32,
    /// Port I/O type (0=Unspecified, 1=Output, 2=Input, 3=Bidirectional).
    #[altium(key = "IOTYPE")]
    io_type: i32,
    /// Horizontal alignment (0=Left, 1=Center, 2=Right).
    #[altium(key = "ALIGNMENT")]
    alignment: i32,
    #[altium(key = "WIDTH")]
    width: SchCoord,
    #[altium(key = "LOCATION.X")]
    location_x: SchCoord,
    #[altium(key = "LOCATION.Y")]
    location_y: SchCoord,
    #[altium(key = "COLOR")]
    color: u32,
    #[altium(key = "FONTID")]
    font_id: i32,
    #[altium(key = "AREACOLOR")]
    area_color: u32,
    #[altium(key = "TEXTCOLOR")]
    text_color: u32,
    #[altium(key = "NAME")]
    name: String,
    #[altium(key = "HARNESSTYPE")]
    harness_type: String,
    #[altium(key = "UNIQUEID")]
    unique_id: String,
    #[altium(key = "HEIGHT")]
    height: SchCoord,
    /// Border width (0=Smallest, 1=Small, 2=Medium, 3=Large).
    #[altium(key = "BORDERWIDTH")]
    border_width: i32,
    #[altium(key = "AUTOSIZE")]
    auto_size: bool,
    #[altium(key = "OBJECTDEFINITIONID")]
    object_definition_id: String,
    #[altium(key = "SHOWNETNAME")]
    show_net_name: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::backing_store::{ParamOrigin, RecordOrigin};

    #[test]
    fn roundtrip() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=18|OWNERINDEX=1|STYLE=2|IOTYPE=3|ALIGNMENT=0|LOCATION.X=100|LOCATION.Y=200|COLOR=0|FONTID=1|AREACOLOR=16777215|TEXTCOLOR=0|NAME=DataPort|UNIQUEID=ABCD1234|HEIGHT=10|BORDERWIDTH=1|AUTOSIZE=T|",
        ));
        let rec = SchPortRecord::from_origin(origin);
        assert_eq!(rec.owner_index(), 1);
        assert_eq!(rec.name(), "DataPort");
        assert_eq!(rec.io_type(), 3);
        assert!(rec.auto_size());
    }

    #[test]
    fn setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=18|NAME=OldPort|"));
        let mut rec = SchPortRecord::from_origin(origin);
        rec.set_name("NewPort".to_string());
        assert_eq!(rec.name(), "NewPort");
    }
}
