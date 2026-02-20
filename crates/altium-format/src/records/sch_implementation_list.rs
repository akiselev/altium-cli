//! Implementation list record (RECORD=44).

use altium_format_derive::altium_record;

/// Implementation list record — container for implementation records.
///
/// Corresponds to `ImplementationListData` / the implementation list wrapper
/// in the v1 API (ObjectId::ImplementationsList = 44).
///
/// This is a minimal record that primarily contains the graphical object base
/// fields as a container for child implementation records.
#[altium_record(kind = "sch", record_id = 44, codec = "params")]
pub struct SchImplementationListRecord {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backing_store::{ParamOrigin, RecordOrigin};

    #[test]
    fn roundtrip() {
        let origin =
            RecordOrigin::Param(ParamOrigin::new("|RECORD=44|OWNERINDEX=1|OWNERPARTID=1|"));
        let rec = SchImplementationListRecord::from_origin(origin);
        assert_eq!(rec.owner_index(), 1);
        assert_eq!(rec.owner_part_id(), 1);
    }

    #[test]
    fn setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=44|OWNERINDEX=1|"));
        let mut rec = SchImplementationListRecord::from_origin(origin);
        rec.set_owner_index(5);
        assert_eq!(rec.owner_index(), 5);
    }
}
