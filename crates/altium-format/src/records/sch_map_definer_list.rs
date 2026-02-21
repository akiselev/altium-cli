//! Map definer list record (RECORD=46).

use altium_format_derive::altium_record;

/// Map definer list record — container for map definer entries.
///
/// Corresponds to `ImplementationMap` in Altium's schematic data model
/// (`TObjectId.eImplementationMap`, binary code 46). This is a non-graphical
/// container owned by an implementation record.
#[altium_record(kind = "sch", record_id = 46, codec = "params")]
pub struct SchMapDefinerListRecord {
    // --- DataObjectBase (flattened) ---
    #[altium(key = "OWNERINDEX")]
    owner_index: i32,
    #[altium(key = "ISNOTACCESIBLE")]
    is_not_accessible: bool,
    #[altium(key = "OWNERINDEXADDITIONALLIST")]
    owner_index_additional_list: bool,
    #[altium(key = "INDEXINSHEET")]
    index_in_sheet: i32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backing_store::{ParamOrigin, RecordOrigin};

    #[test]
    fn roundtrip() {
        let origin =
            RecordOrigin::Param(ParamOrigin::new("|RECORD=46|OWNERINDEX=11|INDEXINSHEET=2|"));
        let rec = SchMapDefinerListRecord::from_origin(origin);
        assert_eq!(rec.owner_index().unwrap(), 11);
        assert_eq!(rec.index_in_sheet().unwrap(), 2);
    }

    #[test]
    fn setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=46|OWNERINDEX=1|"));
        let mut rec = SchMapDefinerListRecord::from_origin(origin);
        rec.set_owner_index(9);
        rec.set_index_in_sheet(3);
        assert_eq!(rec.owner_index().unwrap(), 9);
        assert_eq!(rec.index_in_sheet().unwrap(), 3);
    }
}
