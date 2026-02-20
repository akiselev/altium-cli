//! Schematic parameter record (RECORD=41).

use super::enums::*;
use crate::coord::SchCoord;
use crate::newtypes::{Description, UniqueId};
use crate::traits::RecordType;
use altium_format_derive::altium_record;

/// Schematic parameter record -- RECORD=41.
///
/// Represents a parameter (attribute) attached to a component or other object.
#[altium_record(kind = "sch", record_id = 41, codec = "params")]
pub struct SchParameterRecord {
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

    // --- Parameter-specific fields ---
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

    // Note: In v1 these are exported as inverted booleans (NotAllowLibrarySynchronize etc.)
    // In v2 we store the raw param value and let the user handle the inversion.
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
}

impl SchParameterRecord {
    /// AD writes `IsHidden=T` twice for many parameter records. Preserve this
    /// quirk in rebuilt output to minimize stream diffs.
    pub fn append_hidden_duplicate_for_export(&mut self) {
        if self.is_hidden() {
            self.origin_mut()
                .param_mut()
                .params
                .add_raw_suffix("|ISHIDDEN=T");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backing_store::{ParamOrigin, RecordOrigin};

    #[test]
    fn roundtrip_parameter_getter() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=41|Location.X=100|Location.Y=200|Name=Value|Text=100k|ParamType=0|IsHidden=F|FontID=1|Color=128|ShowName=T|",
        ));
        let rec = SchParameterRecord::from_origin(origin);
        assert_eq!(rec.name(), "Value");
        assert_eq!(rec.text(), "100k");
        assert_eq!(rec.param_type(), ParameterType::String);
        assert!(!rec.is_hidden());
        assert!(rec.show_name());
        assert_eq!(rec.font_id(), 1);
    }

    #[test]
    fn roundtrip_parameter_setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=41|Name=Value|Text=100k|"));
        let mut rec = SchParameterRecord::from_origin(origin);
        rec.set_text("200k".to_string());
        assert_eq!(rec.text(), "200k");
        rec.set_is_hidden(true);
        assert!(rec.is_hidden());
    }
}
