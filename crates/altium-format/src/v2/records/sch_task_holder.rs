//! Task holder / directive record (RECORD=43).

use crate::v2::coord::SchCoord;
use altium_format_derive::altium_record;

/// Task holder record used by AD26 SchDoc files (e.g. differential pair directives).
#[altium_record(kind = "sch", record_id = 43, codec = "params")]
pub struct SchTaskHolderRecord {
    #[altium(key = "INDEXINSHEET")]
    index_in_sheet: i32,
    #[altium(key = "OWNERPARTID")]
    owner_part_id: i16,
    #[altium(key = "LOCATION.X")]
    location_x: SchCoord,
    #[altium(key = "LOCATION.Y")]
    location_y: SchCoord,
    #[altium(key = "COLOR")]
    color: u32,
    #[altium(key = "ORIENTATION")]
    orientation: i32,
    #[altium(key = "NAME")]
    name: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::backing_store::{ParamOrigin, RecordOrigin};

    #[test]
    fn roundtrip() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=43|INDEXINSHEET=41|OWNERPARTID=-1|LOCATION.X=175|LOCATION.Y=1275|COLOR=255|NAME=DIFFPAIR|",
        ));
        let rec = SchTaskHolderRecord::from_origin(origin);
        assert_eq!(rec.index_in_sheet(), 41);
        assert_eq!(rec.owner_part_id(), -1);
        assert_eq!(rec.color(), 255);
        assert_eq!(rec.name(), "DIFFPAIR");
    }
}
