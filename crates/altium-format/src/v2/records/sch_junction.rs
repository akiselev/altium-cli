//! Junction record (RECORD=29).

use altium_format_derive::altium_record;
use crate::v2::coord::SchCoord;

/// Junction record — wire/bus connection point on a schematic.
///
/// Corresponds to `JunctionData` / `ExportJunction` in the v1 API (ObjectId::Junction = 29).
#[altium_record(kind = "sch", record_id = 29, codec = "params")]
pub struct SchJunctionRecord {
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

    // --- Junction-specific fields ---
    #[altium(key = "LOCATION.X")]
    location_x: SchCoord,
    #[altium(key = "LOCATION.Y")]
    location_y: SchCoord,
    /// Junction size (0=Smallest, 1=Small, 2=Medium, 3=Large).
    #[altium(key = "SIZE")]
    size: i32,
    #[altium(key = "COLOR")]
    color: u32,
    #[altium(key = "LOCKED")]
    locked: bool,
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
            "|RECORD=29|OWNERINDEX=0|LOCATION.X=100|LOCATION.Y=200|SIZE=1|COLOR=128|LOCKED=F|UNIQUEID=ABCD1234|",
        ));
        let rec = SchJunctionRecord::from_origin(origin);
        assert_eq!(rec.size(), 1);
        assert_eq!(rec.color(), 128);
        assert!(!rec.locked());
        assert_eq!(rec.unique_id(), "ABCD1234");
    }

    #[test]
    fn setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=29|SIZE=1|"));
        let mut rec = SchJunctionRecord::from_origin(origin);
        rec.set_size(3);
        assert_eq!(rec.size(), 3);
        rec.set_locked(true);
        assert!(rec.locked());
    }
}
