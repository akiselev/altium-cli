//! Pin record data — ported from `SchDataPin.cs` and `FileFormatV5.ExportPin`.
//!
//! Pin is the most complex record type with 50+ fields.
//! Note: Pin does NOT use the GraphicalObjectBase — it has its own
//! OwnerIndex/OwnerPartId fields serialized directly (not via ExportDataObject).

use crate::v2::types::*;

/// Pin data matching C# `SchDataPin` (50+ fields).
///
/// Fields are ordered to match the C# `ExportPin` serialization order.
#[derive(Clone, Debug, Default)]
pub struct PinData {
    // --- Base fields (exported directly, not via ExportDataObject) ---
    pub owner_index: i32,
    pub owner_part_id: i16,
    pub owner_part_display_mode: u8,

    // --- IEEE symbols ---
    pub symbol_inner_edge: IeeeSymbol,
    pub symbol_outer_edge: IeeeSymbol,
    pub symbol_inner: IeeeSymbol,
    pub symbol_outer: IeeeSymbol,

    // --- Core pin properties ---
    pub description: String,
    pub formal_type: StdLogicState,
    pub electrical: PinElectrical,

    // --- PinConglomerate (packed byte) ---
    pub orientation: RotationBy90,
    pub is_hidden: bool,
    pub show_name: bool,
    pub show_designator: bool,
    pub is_accessible: bool,
    pub graphically_locked: bool,
    pub owner_index_additional_list: bool,

    pub pin_length: i32,
    pub location_x: i32,
    pub location_y: i32,
    pub color: u32,
    pub name: String,
    pub designator: String,
    pub swap_id_pin: String,
    pub swap_id_part: String,
    pub default_value: String,
    pub swap_id_pair: String,

    // --- Name customization (ASCII-only) ---
    pub name_position_mode: PinItemMode,
    pub name_custom_rotation_anchor: PinTextRotationAnchor,
    pub name_custom_rotation_relative: RotationBy90,
    pub name_font_mode: PinItemMode,
    pub name_custom_position_margin: i32,
    pub name_custom_font_id: i32,
    pub name_custom_color: u32,

    // --- Designator customization (ASCII-only) ---
    pub designator_position_mode: PinItemMode,
    pub designator_custom_rotation_anchor: PinTextRotationAnchor,
    pub designator_custom_rotation_relative: RotationBy90,
    pub designator_font_mode: PinItemMode,
    pub designator_custom_position_margin: i32,
    pub designator_custom_font_id: i32,
    pub designator_custom_color: u32,

    // --- Symbol line width (ASCII-only) ---
    pub symbol_line_width: Size,

    // --- Extended data (ASCII-only) ---
    pub pin_package_length: i32,
    pub pin_propagation_delay: f64,

    // --- Unique ID (document objects only) ---
    pub unique_id: String,

    // --- Alternate pin functions (ASCII-only) ---
    pub hide_pin_name_as_function: bool,
    pub pin_symbolic_name: String,
    pub show_symbolic_name_as_function: bool,

    // --- Schematic block (document objects only) ---
    pub is_schematic_block_object: bool,
}
