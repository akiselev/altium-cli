//! Schematic ellipse record (RECORD=8).

use super::enums::*;
use crate::coord::SchCoord;
use crate::newtypes::UniqueId;
use altium_format_derive::altium_record;

/// Schematic ellipse record -- RECORD=8.
///
/// Represents an ellipse primitive on a schematic sheet.
#[altium_record(kind = "sch", record_id = 8, codec = "params")]
pub struct SchEllipseRecord {
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

    // --- Ellipse-specific fields ---
    #[altium(key = "Location.X")]
    location_x: SchCoord,

    #[altium(key = "Location.Y")]
    location_y: SchCoord,

    #[altium(key = "Radius")]
    radius: SchCoord,

    #[altium(key = "SecondaryRadius")]
    secondary_radius: SchCoord,

    #[altium(key = "LineWidth")]
    line_width: Size,

    #[altium(key = "Color")]
    color: u32,

    #[altium(key = "AreaColor")]
    area_color: u32,

    #[altium(key = "IsSolid")]
    is_solid: bool,

    #[altium(key = "Transparent")]
    transparent: bool,

    #[altium(key = "UniqueID")]
    unique_id: UniqueId,
}

impl SchEllipseRecord {
    /// Copies encoded ellipse coordinate/radius parts exactly from `src`.
    pub fn copy_coordinate_encoding_from(&mut self, src: &Self) {
        use crate::traits::RecordType;

        const KEYS: &[&str] = &[
            "LOCATION.X",
            "LOCATION.X_FRAC",
            "LOCATION.Y",
            "LOCATION.Y_FRAC",
            "RADIUS",
            "RADIUS_FRAC",
            "SECONDARYRADIUS",
            "SECONDARYRADIUS_FRAC",
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
    fn roundtrip_ellipse_getter() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=8|Location.X=100|Location.Y=200|Radius=50|SecondaryRadius=30|IsSolid=T|Color=255|AreaColor=128|",
        ));
        let rec = SchEllipseRecord::from_origin(origin);
        assert!(rec.is_solid().unwrap());
        assert_eq!(rec.color().unwrap(), 255);
        assert_eq!(rec.area_color().unwrap(), 128);
    }

    #[test]
    fn roundtrip_ellipse_setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=8|Location.X=100|Location.Y=200|Radius=50|IsSolid=F|",
        ));
        let mut rec = SchEllipseRecord::from_origin(origin);
        rec.set_is_solid(true);
        assert!(rec.is_solid().unwrap());
    }
}
