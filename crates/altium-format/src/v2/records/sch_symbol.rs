//! IEEE Symbol record (RECORD=3).

use altium_format_derive::altium_record;
use crate::v2::coord::SchCoord;

/// IEEE Symbol record — graphical symbol marker on a schematic.
///
/// Corresponds to `SymbolData` / `ExportSymbol` in the v1 API (ObjectId::Probe = 3).
#[altium_record(kind = "sch", record_id = 3, codec = "params")]
pub struct SchSymbolRecord {
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

    // --- Symbol-specific fields ---
    /// IEEE symbol type (stored as integer, maps to IeeeSymbol enum).
    #[altium(key = "SYMBOL")]
    symbol: i32,
    #[altium(key = "LOCATION.X")]
    location_x: SchCoord,
    #[altium(key = "LOCATION.Y")]
    location_y: SchCoord,
    #[altium(key = "SCALEFACTOR")]
    scale_factor: i32,
    /// Rotation in 90-degree increments (0-3).
    #[altium(key = "ORIENTATION")]
    orientation: i32,
    /// Line width (0=Smallest, 1=Small, 2=Medium, 3=Large).
    #[altium(key = "LINEWIDTH")]
    line_width: i32,
    #[altium(key = "COLOR")]
    color: u32,
    #[altium(key = "MIRROR")]
    is_mirrored: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::backing_store::{ParamOrigin, RecordOrigin};

    #[test]
    fn roundtrip() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=3|OWNERINDEX=1|OWNERPARTID=1|SYMBOL=1|LOCATION.X=100|LOCATION.Y=200|SCALEFACTOR=1|ORIENTATION=0|LINEWIDTH=1|COLOR=128|MIRROR=F|",
        ));
        let rec = SchSymbolRecord::from_origin(origin);
        assert_eq!(rec.owner_index(), 1);
        assert_eq!(rec.symbol(), 1);
        assert_eq!(rec.color(), 128);
        assert!(!rec.is_mirrored());
    }

    #[test]
    fn setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=3|OWNERINDEX=1|"));
        let mut rec = SchSymbolRecord::from_origin(origin);
        rec.set_symbol(5);
        assert_eq!(rec.symbol(), 5);
        rec.set_is_mirrored(true);
        assert!(rec.is_mirrored());
    }
}
