//! Schematic designator record (RECORD=34).

use super::enums::*;
use crate::coord::SchCoord;
use crate::newtypes::{Description, UniqueId};
use altium_format_derive::altium_record;

/// Schematic designator record -- RECORD=34.
///
/// Extends the parameter record with designator-specific auto-position handling.
/// Has all the same fields as SchParameterRecord plus override_not_auto_position.
#[altium_record(kind = "sch", record_id = 34, codec = "params")]
pub struct SchDesignatorRecord {
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

    // --- Parameter fields (same as SchParameterRecord) ---
    #[altium(key = "Location.X")]
    location_x: SchCoord,

    #[altium(key = "Location.Y")]
    location_y: SchCoord,

    #[altium(key = "Orientation")]
    orientation: RotationBy90,

    #[altium(key = "Justification")]
    justification: TextJustification,

    #[altium(key = "Color")]
    color: u32,

    #[altium(key = "FontID")]
    font_id: i32,

    #[altium(key = "IsHidden")]
    is_hidden: bool,

    #[altium(key = "Text")]
    text: String,

    #[altium(key = "ParamType")]
    param_type: ParameterType,

    #[altium(key = "Name")]
    name: String,

    #[altium(key = "ShowName")]
    show_name: bool,

    #[altium(key = "ReadOnlyState")]
    read_only_state: ParameterReadOnlyState,

    #[altium(key = "UniqueID")]
    unique_id: UniqueId,

    #[altium(key = "Description")]
    description: Description,

    #[altium(key = "NotAutoPosition")]
    not_auto_position: bool,

    #[altium(key = "IsMirrored")]
    is_mirrored: bool,

    #[altium(key = "TextHorzAnchor")]
    text_horz_anchor: TextHorzAnchor,

    #[altium(key = "TextVertAnchor")]
    text_vert_anchor: TextVertAnchor,

    #[altium(key = "IsImageParameter")]
    is_image_parameter: bool,

    // --- Designator-specific field ---
    #[altium(key = "OverrideNotAutoPosition")]
    override_not_auto_position: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backing_store::{ParamOrigin, RecordOrigin};

    #[test]
    fn roundtrip_designator_getter() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=34|Location.X=100|Location.Y=200|Name=Designator|Text=U1|FontID=1|Color=128|ShowName=F|OverrideNotAutoPosition=T|",
        ));
        let rec = SchDesignatorRecord::from_origin(origin);
        assert_eq!(rec.name(), "Designator");
        assert_eq!(rec.text(), "U1");
        assert!(!rec.show_name());
        assert!(rec.override_not_auto_position());
    }

    #[test]
    fn roundtrip_designator_setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=34|Name=Designator|Text=U1|"));
        let mut rec = SchDesignatorRecord::from_origin(origin);
        rec.set_text("U2".to_string());
        assert_eq!(rec.text(), "U2");
        rec.set_override_not_auto_position(true);
        assert!(rec.override_not_auto_position());
    }
}
