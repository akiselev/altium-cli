//! Implementation parameters record (RECORD=48).

use altium_format_derive::altium_record;

/// Implementation parameters record — container for parameters attached to a
/// specific implementation.
///
/// Corresponds to `ParameterList` (`TObjectId.eParameterList`, binary code 48).
#[altium_record(kind = "sch", record_id = 48, codec = "params")]
pub struct SchImplementationParametersRecord {
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
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=48|OWNERINDEX=105|OWNERPARTID=0|INDEXINSHEET=0|",
        ));
        let rec = SchImplementationParametersRecord::from_origin(origin);
        assert_eq!(rec.owner_index().unwrap(), 105);
        assert_eq!(rec.owner_part_id().unwrap(), 0);
        assert_eq!(rec.index_in_sheet().unwrap(), 0);
    }

    #[test]
    fn setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=48|OWNERINDEX=1|"));
        let mut rec = SchImplementationParametersRecord::from_origin(origin);
        rec.set_owner_index(10);
        rec.set_owner_part_id(2);
        assert_eq!(rec.owner_index().unwrap(), 10);
        assert_eq!(rec.owner_part_id().unwrap(), 2);
    }
}
