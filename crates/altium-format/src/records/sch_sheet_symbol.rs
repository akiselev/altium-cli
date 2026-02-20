//! Sheet symbol record (RECORD=39).

use crate::coord::SchCoord;
use altium_format_derive::altium_record;

/// Sheet symbol record — hierarchical sheet symbol on a schematic.
///
/// Corresponds to `SheetSymbolData` / `ExportSheetSymbol` in the v1 API
/// (ObjectId::SheetSymbol = 39).
///
/// Includes the flattened `RectangularEntryContainerBase` fields (which itself
/// flattens `GraphicalObjectBase`).
#[altium_record(kind = "sch", record_id = 39, codec = "params")]
pub struct SchSheetSymbolRecord {
    // --- GraphicalObjectBase (flattened from RectangularEntryContainerBase) ---
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

    // --- RectangularEntryContainerBase fields ---
    #[altium(key = "LOCATION.X")]
    location_x: SchCoord,
    #[altium(key = "LOCATION.Y")]
    location_y: SchCoord,
    #[altium(key = "XSIZE")]
    x_size: SchCoord,
    #[altium(key = "YSIZE")]
    y_size: SchCoord,
    /// Line width (0=Smallest, 1=Small, 2=Medium, 3=Large).
    #[altium(key = "LINEWIDTH")]
    line_width: i32,
    #[altium(key = "COLOR")]
    color: u32,
    #[altium(key = "AREACOLOR")]
    area_color: u32,

    // --- SheetSymbol-specific fields ---
    #[altium(key = "ISSOLID")]
    is_solid: bool,
    #[altium(key = "SHOWHIDDENFIELDS")]
    show_hidden_fields: bool,
    #[altium(key = "UNIQUEID")]
    unique_id: String,
    #[altium(key = "SYMBOLTYPE")]
    symbol_type: String,
    #[altium(key = "DESIGNITEMID")]
    design_item_id: String,
    #[altium(key = "SOURCELIBRARYNAME")]
    source_library_name: String,
    #[altium(key = "VAULTGUID")]
    vault_guid: String,
    #[altium(key = "ITEMGUID")]
    item_guid: String,
    #[altium(key = "REVISIONGUID")]
    revision_guid: String,
    #[altium(key = "REVISIONNAME")]
    revision_name: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backing_store::{ParamOrigin, RecordOrigin};

    #[test]
    fn roundtrip() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=39|OWNERINDEX=0|LOCATION.X=100|LOCATION.Y=200|XSIZE=300|YSIZE=400|LINEWIDTH=1|COLOR=0|AREACOLOR=16777215|ISSOLID=T|SHOWHIDDENFIELDS=F|UNIQUEID=ABCD1234|SYMBOLTYPE=Normal|DESIGNITEMID=MyDesign|",
        ));
        let rec = SchSheetSymbolRecord::from_origin(origin);
        assert!(rec.is_solid());
        assert!(!rec.show_hidden_fields());
        assert_eq!(rec.unique_id(), "ABCD1234");
        assert_eq!(rec.symbol_type(), "Normal");
    }

    #[test]
    fn setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=39|UNIQUEID=OLD|"));
        let mut rec = SchSheetSymbolRecord::from_origin(origin);
        rec.set_unique_id("NEW12345".to_string());
        assert_eq!(rec.unique_id(), "NEW12345");
    }
}
