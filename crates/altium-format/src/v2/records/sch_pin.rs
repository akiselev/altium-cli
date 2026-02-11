//! Schematic pin record (RECORD=2).

use altium_format_derive::altium_record;
use crate::v2::coord::SchCoord;
use crate::v2::newtypes::{Designator, PinName, UniqueId};
use super::enums::*;

/// Schematic pin record -- RECORD=2.
///
/// The most complex record type with 50+ fields. Pin does NOT use the
/// GraphicalObjectBase -- it has its own OwnerIndex/OwnerPartId fields
/// serialized directly.
#[altium_record(kind = "sch", record_id = 2, codec = "params")]
pub struct SchPinRecord {
    // --- Base fields (exported directly, not via ExportDataObject) ---
    #[altium(key = "OwnerIndex")]
    owner_index: i32,

    #[altium(key = "OwnerPartId")]
    owner_part_id: i16,

    #[altium(key = "OwnerPartDisplayMode")]
    owner_part_display_mode: u8,

    // --- IEEE symbols ---
    #[altium(key = "SymBol_InnerEdge")]
    symbol_inner_edge: IeeeSymbol,

    #[altium(key = "SymBol_OuterEdge")]
    symbol_outer_edge: IeeeSymbol,

    #[altium(key = "SymBol_Inner")]
    symbol_inner: IeeeSymbol,

    #[altium(key = "SymBol_Outer")]
    symbol_outer: IeeeSymbol,

    // --- Core pin properties ---
    #[altium(key = "Description")]
    description: String,

    #[altium(key = "FormalType")]
    formal_type: StdLogicState,

    #[altium(key = "Electrical")]
    electrical: PinElectricalType,

    // --- PinConglomerate (packed byte) ---
    #[altium(key = "PinConglomerate")]
    pin_conglomerate: u8,

    #[altium(key = "PinLength")]
    pin_length: SchCoord,

    #[altium(key = "Location.X")]
    location_x: SchCoord,

    #[altium(key = "Location.Y")]
    location_y: SchCoord,

    #[altium(key = "Color")]
    color: u32,

    #[altium(key = "Name")]
    name: PinName,

    #[altium(key = "Designator")]
    designator: Designator,

    #[altium(key = "SwapIdPin")]
    swap_id_pin: String,

    #[altium(key = "SwapIDPart")]
    swap_id_part: String,

    #[altium(key = "DefaultValue")]
    default_value: String,

    #[altium(key = "SwapIdPair")]
    swap_id_pair: String,

    // --- Name customization ---
    #[altium(key = "PinName_PositionConglomerate")]
    pin_name_position_conglomerate: u8,

    #[altium(key = "Name_CustomPosition_Margin")]
    name_custom_position_margin: i32,

    #[altium(key = "Name_CustomFontID")]
    name_custom_font_id: i32,

    #[altium(key = "Name_CustomColor")]
    name_custom_color: u32,

    // --- Designator customization ---
    #[altium(key = "PinDesignator_PositionConglomerate")]
    pin_designator_position_conglomerate: u8,

    #[altium(key = "Designator_CustomPosition_Margin")]
    designator_custom_position_margin: i32,

    #[altium(key = "Designator_CustomFontID")]
    designator_custom_font_id: i32,

    #[altium(key = "Designator_CustomColor")]
    designator_custom_color: u32,

    // --- Symbol line width ---
    #[altium(key = "SymBol_LineWidth")]
    symbol_line_width: Size,

    // --- Extended data ---
    #[altium(key = "PinPackageLength")]
    pin_package_length: SchCoord,

    #[altium(key = "PinPropagationDelay")]
    pin_propagation_delay: f64,

    // --- Unique ID ---
    #[altium(key = "UniqueID")]
    unique_id: UniqueId,

    // --- Alternate pin functions ---
    #[altium(key = "HidePinNameAsFunction")]
    hide_pin_name_as_function: bool,

    #[altium(key = "PinSymbolicName")]
    pin_symbolic_name: String,

    #[altium(key = "ShowPinSymbolicNameAsFunction")]
    show_symbolic_name_as_function: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::backing_store::{ParamOrigin, RecordOrigin};

    #[test]
    fn roundtrip_pin_getter() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=2|Designator=1|Name=VCC|PinLength=30|Location.X=100|Location.Y=200|Electrical=7|",
        ));
        let rec = SchPinRecord::from_origin(origin);
        assert_eq!(rec.designator(), Designator::from("1"));
        assert_eq!(rec.name(), PinName::from("VCC"));
        assert_eq!(rec.electrical(), PinElectricalType::Power);
    }

    #[test]
    fn roundtrip_pin_setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=2|Designator=1|Name=VCC|PinLength=30|",
        ));
        let mut rec = SchPinRecord::from_origin(origin);
        rec.set_designator(Designator::from("2"));
        assert_eq!(rec.designator(), Designator::from("2"));
        rec.set_name(PinName::from("GND"));
        assert_eq!(rec.name(), PinName::from("GND"));
    }
}
