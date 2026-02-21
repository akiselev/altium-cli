//! Schematic arc record (RECORD=12).

use super::enums::*;
use crate::coord::SchCoord;
use crate::newtypes::UniqueId;
use altium_format_derive::altium_record;

/// Schematic arc record -- RECORD=12.
///
/// Represents an arc primitive on a schematic sheet.
#[altium_record(kind = "sch", record_id = 12, codec = "params")]
pub struct SchArcRecord {
    // --- Base object fields (flattened from GraphicalObjectBase) ---
    #[altium(key = "OwnerIndex")]
    owner_index: i32,

    #[altium(key = "OwnerPartId")]
    owner_part_id: i16,

    #[altium(key = "OwnerPartDisplayMode")]
    owner_part_display_mode: u8,

    #[altium(key = "IndexInSheet")]
    index_in_sheet: i32,

    #[altium(key = "IsNotAccesible")]
    is_not_accessible: bool,

    #[altium(key = "GraphicallyLocked")]
    graphically_locked: bool,

    // --- Arc-specific fields ---
    #[altium(key = "Location.X")]
    location_x: SchCoord,

    #[altium(key = "Location.Y")]
    location_y: SchCoord,

    #[altium(key = "Radius")]
    radius: SchCoord,

    #[altium(key = "LineWidth")]
    line_width: Size,

    #[altium(key = "StartAngle")]
    start_angle: f64,

    #[altium(key = "EndAngle")]
    end_angle: f64,

    #[altium(key = "Color")]
    color: u32,

    #[altium(key = "UniqueID")]
    unique_id: UniqueId,
}

impl SchArcRecord {
    /// Copies location/radius DXP parts plus angle text values exactly from
    /// `src`, preserving non-canonical int+frac splits and angle formatting.
    pub fn copy_geometry_encoding_from(&mut self, src: &Self) {
        use crate::traits::RecordType;

        const KEYS: &[&str] = &[
            "LOCATION.X",
            "LOCATION.X_FRAC",
            "LOCATION.Y",
            "LOCATION.Y_FRAC",
            "RADIUS",
            "RADIUS_FRAC",
            "STARTANGLE",
            "ENDANGLE",
        ];

        let src_params = &src.origin().param().params;
        let mut to_copy: Vec<(String, String)> = Vec::new();
        for &key in KEYS {
            if let Some(v) = src_params.get(key) {
                to_copy.push((key.to_string(), v.as_str().to_string()));
            }
        }

        let dst_params = &mut self.origin_mut().param_mut().params;
        for &key in KEYS {
            dst_params.remove(key);
        }
        for (k, v) in to_copy {
            dst_params.add(&k, &v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backing_store::{ParamOrigin, RecordOrigin};

    #[test]
    fn roundtrip_arc_getter() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=12|Location.X=100|Location.Y=200|Radius=50|StartAngle=0.000000|EndAngle=360.000000|Color=255|LineWidth=1|",
        ));
        let rec = SchArcRecord::from_origin(origin);
        assert_eq!(rec.color().unwrap(), 255);
        assert_eq!(rec.line_width().unwrap(), Size::Small);
    }

    #[test]
    fn roundtrip_arc_setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=12|Location.X=100|Location.Y=200|Radius=50|Color=255|",
        ));
        let mut rec = SchArcRecord::from_origin(origin);
        rec.set_color(128);
        assert_eq!(rec.color().unwrap(), 128);
    }
}
