//! Schematic pie record (RECORD=9).

use super::enums::*;
use crate::v2::coord::SchCoord;
use altium_format_derive::altium_record;

/// Schematic pie record -- RECORD=9.
///
/// Represents a pie (arc sector) primitive on a schematic sheet.
#[altium_record(kind = "sch", record_id = 9, codec = "params")]
pub struct SchPieRecord {
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

    // --- Pie-specific fields ---
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

    #[altium(key = "AreaColor")]
    area_color: u32,

    #[altium(key = "IsSolid")]
    is_solid: bool,
}

impl SchPieRecord {
    /// Copies location/radius DXP parts and angle text values exactly from
    /// `src`, preserving non-canonical splits and source float formatting.
    pub fn copy_geometry_encoding_from(&mut self, src: &Self) {
        use crate::v2::traits::RecordType;

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
    use crate::v2::backing_store::{ParamOrigin, RecordOrigin};

    #[test]
    fn roundtrip_pie_getter() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=9|Location.X=100|Location.Y=200|Radius=50|StartAngle=0.000000|EndAngle=90.000000|IsSolid=T|Color=255|AreaColor=128|",
        ));
        let rec = SchPieRecord::from_origin(origin);
        assert!(rec.is_solid());
        assert_eq!(rec.color(), 255);
        assert_eq!(rec.area_color(), 128);
    }

    #[test]
    fn roundtrip_pie_setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=9|Location.X=100|Location.Y=200|Radius=50|IsSolid=F|Color=255|",
        ));
        let mut rec = SchPieRecord::from_origin(origin);
        rec.set_is_solid(true);
        assert!(rec.is_solid());
        rec.set_color(64);
        assert_eq!(rec.color(), 64);
    }
}
