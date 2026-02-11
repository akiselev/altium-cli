//! Schematic image record (RECORD=30).

use altium_format_derive::altium_record;
use crate::v2::coord::SchCoord;
use crate::v2::newtypes::UniqueId;
use super::enums::*;

/// Schematic image record -- RECORD=30.
///
/// Represents an embedded or linked image on a schematic sheet.
#[altium_record(kind = "sch", record_id = 30, codec = "params")]
pub struct SchImageRecord {
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

    // --- Image-specific fields ---
    #[altium(key = "Location.X")]
    location_x: SchCoord,

    #[altium(key = "Location.Y")]
    location_y: SchCoord,

    #[altium(key = "Corner.X")]
    corner_x: SchCoord,

    #[altium(key = "Corner.Y")]
    corner_y: SchCoord,

    #[altium(key = "Orientation")]
    orientation: RotationBy90,

    #[altium(key = "LineWidth")]
    line_width: Size,

    #[altium(key = "Color")]
    color: u32,

    #[altium(key = "IsSolid")]
    is_solid: bool,

    #[altium(key = "KeepAspect")]
    keep_aspect: bool,

    #[altium(key = "EmbedImage")]
    embed_image: bool,

    #[altium(key = "FileName")]
    file_name: String,

    #[altium(key = "UniqueID")]
    unique_id: UniqueId,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::backing_store::{ParamOrigin, RecordOrigin};

    #[test]
    fn roundtrip_image_getter() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=30|Location.X=10|Location.Y=20|Corner.X=100|Corner.Y=200|KeepAspect=T|EmbedImage=T|FileName=logo.png|",
        ));
        let rec = SchImageRecord::from_origin(origin);
        assert!(rec.keep_aspect());
        assert!(rec.embed_image());
        assert_eq!(rec.file_name(), "logo.png");
    }

    #[test]
    fn roundtrip_image_setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=30|Location.X=10|Location.Y=20|FileName=old.png|",
        ));
        let mut rec = SchImageRecord::from_origin(origin);
        rec.set_file_name("new.png".to_string());
        assert_eq!(rec.file_name(), "new.png");
        rec.set_keep_aspect(true);
        assert!(rec.keep_aspect());
    }
}
