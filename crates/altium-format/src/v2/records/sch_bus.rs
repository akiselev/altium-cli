//! Bus record (RECORD=26).

use altium_format_derive::altium_record;

/// Bus record — multi-signal bus line on a schematic.
///
/// Corresponds to `BusData` / `ExportBus` in the v1 API (ObjectId::Bus = 26).
///
/// Note: vertices (`Vec<(i32,i32)>`) are skipped in this phase and will be
/// handled with custom codec logic in a later phase.
#[altium_record(kind = "sch", record_id = 26, codec = "params")]
pub struct SchBusRecord {
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

    // --- Bus-specific fields ---
    /// Line width (0=Smallest, 1=Small, 2=Medium, 3=Large).
    #[altium(key = "LINEWIDTH")]
    line_width: i32,
    #[altium(key = "COLOR")]
    color: u32,
    #[altium(key = "UNDERLINECOLOR")]
    underline_color: u32,
    #[altium(key = "UNIQUEID")]
    unique_id: String,
    #[altium(key = "ASSIGNEDINTERFACE")]
    assigned_interface: String,
    #[altium(key = "ASSIGNEDINTERFACESIGNAL")]
    assigned_interface_signal: String,

    /// Vertex coordinates — skipped; handled in later phase.
    #[altium(skip)]
    _vertices: i32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::backing_store::{ParamOrigin, RecordOrigin};

    #[test]
    fn roundtrip() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=26|OWNERINDEX=0|OWNERPARTID=1|LINEWIDTH=1|COLOR=8388608|UNDERLINECOLOR=0|UNIQUEID=ABCD1234|LOCATIONCOUNT=2|X1=100|Y1=200|X2=300|Y2=400|",
        ));
        let rec = SchBusRecord::from_origin(origin);
        assert_eq!(rec.owner_index(), 0);
        assert_eq!(rec.line_width(), 1);
        assert_eq!(rec.color(), 8388608);
        assert_eq!(rec.unique_id(), "ABCD1234");
    }

    #[test]
    fn setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=26|COLOR=0|"));
        let mut rec = SchBusRecord::from_origin(origin);
        rec.set_color(255);
        assert_eq!(rec.color(), 255);
    }
}
