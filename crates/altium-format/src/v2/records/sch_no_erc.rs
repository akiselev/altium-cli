//! NoERC marker record (RECORD=22).

use altium_format_derive::altium_record;
use crate::v2::coord::SchCoord;

/// NoERC marker record — suppresses electrical rule check errors at a location.
///
/// Corresponds to `NoERCData` / `ExportNoERC` in the v1 API (ObjectId::NoERC = 22).
#[altium_record(kind = "sch", record_id = 22, codec = "params")]
pub struct SchNoERCRecord {
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

    // --- NoERC-specific fields ---
    #[altium(key = "LOCATION.X")]
    location_x: SchCoord,
    #[altium(key = "LOCATION.Y")]
    location_y: SchCoord,
    #[altium(key = "COLOR")]
    color: u32,
    /// Rotation in 90-degree increments (0-3).
    #[altium(key = "ORIENTATION")]
    orientation: i32,
    /// NoERC symbol type (0=CrossThin, 1=CrossThick, 2=CrossSmall, 3=CheckBox, 4=Triangle).
    #[altium(key = "SYMBOL")]
    symbol: i32,
    #[altium(key = "ISACTIVE")]
    is_active: bool,
    #[altium(key = "SUPPRESSALL")]
    suppress_all: bool,
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
            "|RECORD=22|OWNERINDEX=1|OWNERPARTID=1|LOCATION.X=100|LOCATION.Y=200|COLOR=255|ORIENTATION=0|SYMBOL=0|ISACTIVE=T|SUPPRESSALL=F|UNIQUEID=ABCD1234|",
        ));
        let rec = SchNoERCRecord::from_origin(origin);
        assert_eq!(rec.owner_index(), 1);
        assert_eq!(rec.symbol(), 0);
        assert!(rec.is_active());
        assert!(!rec.suppress_all());
    }

    #[test]
    fn setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=22|SYMBOL=0|"));
        let mut rec = SchNoERCRecord::from_origin(origin);
        rec.set_symbol(1);
        assert_eq!(rec.symbol(), 1);
        rec.set_is_active(false);
        assert!(!rec.is_active());
    }
}
