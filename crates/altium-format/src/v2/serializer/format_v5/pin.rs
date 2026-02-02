//! Format functions for Pin record type.

use crate::error::Result;
use crate::v2::fields::pin::PinData;
use crate::v2::serializer::SchSerializer;
use crate::v2::types::*;

pub fn export_pin(s: &mut dyn SchSerializer, pin: &PinData) -> Result<()> {
    s.export_long_int(pin.owner_index, "OwnerIndex")?;
    s.export_short_int(pin.owner_part_id as i32, "OwnerPartId")?;
    s.export_byte(pin.owner_part_display_mode, "OwnerPartDisplayMode")?;
    s.export_byte(pin.symbol_inner_edge as u8, "SymBol_InnerEdge")?;
    s.export_byte(pin.symbol_outer_edge as u8, "SymBol_OuterEdge")?;
    s.export_byte(pin.symbol_inner as u8, "SymBol_Inner")?;
    s.export_byte(pin.symbol_outer as u8, "SymBol_Outer")?;
    s.export_dynamic_string(&pin.description, "Description")?;
    s.export_byte(pin.formal_type as u8, "FormalType")?;
    s.export_pin_electrical(pin.electrical, "Electrical")?;

    // PinConglomerate — packed byte
    let mut conglom: u8 = pin.orientation as u8 & 0x03;
    if pin.is_hidden { conglom |= 0x04; }
    if pin.show_name { conglom |= 0x08; }
    if pin.show_designator { conglom |= 0x10; }
    if !pin.is_accessible { conglom |= 0x20; }
    if pin.graphically_locked { conglom |= 0x40; }
    if pin.owner_index_additional_list { conglom |= 0x80; }
    s.export_byte(conglom, "PinConglomerate")?;

    s.export_coord(pin.pin_length, "PinLength")?;
    s.export_coord(pin.location_x, "Location.X")?;
    s.export_coord(pin.location_y, "Location.Y")?;
    s.export_color(pin.color, "Color")?;
    s.export_dynamic_string(&pin.name, "Name")?;
    s.export_dynamic_string(&pin.designator, "Designator")?;
    s.export_string(&pin.swap_id_pin, "SwapIdPin")?;
    s.export_string(&pin.swap_id_part, "SwapIDPart")?;
    s.export_dynamic_string(&pin.default_value, "DefaultValue")?;
    s.export_ascii_only_string(&pin.swap_id_pair, "SwapIdPair")?;

    // Name customization (ASCII-only, conditional)
    if pin.name_position_mode == PinItemMode::Custom || pin.name_font_mode == PinItemMode::Custom {
        let mut b: u8 = 0;
        if pin.name_position_mode == PinItemMode::Custom {
            b |= 1;
            if pin.name_custom_rotation_anchor == PinTextRotationAnchor::Component {
                b |= 2;
            }
            b |= ((pin.name_custom_rotation_relative as u8) << 2) & 0x0C;
        }
        if pin.name_font_mode == PinItemMode::Custom {
            b |= 0x10;
        }
        s.export_ascii_only_byte(b, "PinName_PositionConglomerate")?;
        if pin.name_position_mode == PinItemMode::Custom {
            s.export_ascii_only_coord(pin.name_custom_position_margin, "Name_CustomPosition_Margin")?;
        }
        if pin.name_font_mode == PinItemMode::Custom {
            s.export_ascii_only_font_id(pin.name_custom_font_id, "Name_CustomFontID")?;
            s.export_ascii_only_color(pin.name_custom_color, "Name_CustomColor")?;
        }
    }

    // Designator customization (ASCII-only, conditional)
    if pin.designator_position_mode == PinItemMode::Custom || pin.designator_font_mode == PinItemMode::Custom {
        let mut b: u8 = 0;
        if pin.designator_position_mode == PinItemMode::Custom {
            b |= 1;
            if pin.designator_custom_rotation_anchor == PinTextRotationAnchor::Component {
                b |= 2;
            }
            b |= ((pin.designator_custom_rotation_relative as u8) << 2) & 0x0C;
        }
        if pin.designator_font_mode == PinItemMode::Custom {
            b |= 0x10;
        }
        s.export_ascii_only_byte(b, "PinDesignator_PositionConglomerate")?;
        if pin.designator_position_mode == PinItemMode::Custom {
            s.export_ascii_only_coord(pin.designator_custom_position_margin, "Designator_CustomPosition_Margin")?;
        }
        if pin.designator_font_mode == PinItemMode::Custom {
            s.export_ascii_only_font_id(pin.designator_custom_font_id, "Designator_CustomFontID")?;
            s.export_ascii_only_color(pin.designator_custom_color, "Designator_CustomColor")?;
        }
    }

    s.export_ascii_only_byte(pin.symbol_line_width as u8, "SymBol_LineWidth")?;
    s.export_ascii_only_coord(pin.pin_package_length, "PinPackageLength")?;
    s.export_ascii_only_double(pin.pin_propagation_delay, "PinPropagationDelay")?;

    if !pin.unique_id.is_empty() {
        s.export_dynamic_string(&pin.unique_id, "UniqueID")?;
    }

    s.export_ascii_only_boolean(pin.hide_pin_name_as_function, "HidePinNameAsFunction")?;
    s.export_ascii_only_string(&pin.pin_symbolic_name, "PinSymbolicName")?;
    s.export_ascii_only_boolean(pin.show_symbolic_name_as_function, "ShowPinSymbolicNameAsFunction")?;

    Ok(())
}

/// Import pin — from C# `FileFormatV5.ImportPin` (lines 420-588).
pub fn import_pin(s: &mut dyn SchSerializer, pin: &mut PinData) -> Result<()> {
    pin.owner_index = s.import_long_int("OwnerIndex")?;
    pin.owner_part_id = s.import_short_int("OwnerPartId")? as i16;
    pin.owner_part_display_mode = s.import_byte("OwnerPartDisplayMode")?;
    pin.symbol_inner_edge = IeeeSymbol::from_u8(s.import_byte("SymBol_InnerEdge")?);
    pin.symbol_outer_edge = IeeeSymbol::from_u8(s.import_byte("SymBol_OuterEdge")?);
    pin.symbol_inner = IeeeSymbol::from_u8(s.import_byte("SymBol_Inner")?);
    pin.symbol_outer = IeeeSymbol::from_u8(s.import_byte("SymBol_Outer")?);
    pin.description = s.import_dynamic_string("Description")?;
    pin.formal_type = StdLogicState::from_u8(s.import_byte("FormalType")?).unwrap_or_default();
    pin.electrical = s.import_pin_electrical("Electrical")?;

    // PinConglomerate — packed byte
    let conglom = s.import_byte("PinConglomerate")?;
    pin.orientation = RotationBy90::from_u8(conglom & 0x03).unwrap_or_default();
    pin.is_hidden = (conglom & 0x04) != 0;
    pin.show_name = (conglom & 0x08) != 0;
    pin.show_designator = (conglom & 0x10) != 0;
    pin.is_accessible = (conglom & 0x20) == 0; // inverted!
    pin.graphically_locked = false; // C# always sets false on import
    pin.owner_index_additional_list = (conglom & 0x80) != 0;

    pin.pin_length = s.import_coord("PinLength")?;
    pin.location_x = s.import_coord("Location.X")?;
    pin.location_y = s.import_coord("Location.Y")?;
    pin.color = s.import_color("Color")?;
    pin.name = s.import_dynamic_string("Name")?;
    pin.designator = s.import_dynamic_string("Designator")?;
    pin.swap_id_pin = s.import_string("SwapIdPin")?;
    pin.swap_id_part = s.import_dynamic_string("SwapIDPart")?;
    pin.default_value = s.import_dynamic_string("DefaultValue")?;
    pin.swap_id_pair = s.import_ascii_only_string("SwapIdPair")?;

    // Name position conglomerate (ASCII-only)
    let name_conglom = s.import_ascii_only_byte("PinName_PositionConglomerate")?;
    if (name_conglom & 1) != 0 {
        pin.name_position_mode = PinItemMode::Custom;
        pin.name_custom_rotation_anchor = if (name_conglom & 2) != 0 {
            PinTextRotationAnchor::Component
        } else {
            PinTextRotationAnchor::Pin
        };
        pin.name_custom_rotation_relative = RotationBy90::from_u8((name_conglom & 0x0C) >> 2).unwrap_or_default();
        pin.name_custom_position_margin = s.import_ascii_only_coord("Name_CustomPosition_Margin")?;
    } else {
        pin.name_position_mode = PinItemMode::Default;
    }
    if (name_conglom & 0x10) != 0 {
        pin.name_font_mode = PinItemMode::Custom;
        pin.name_custom_font_id = s.import_ascii_only_font_id("Name_CustomFontID")?;
        pin.name_custom_color = s.import_ascii_only_color("Name_CustomColor")?;
    } else {
        pin.name_font_mode = PinItemMode::Default;
    }

    // Designator position conglomerate (ASCII-only)
    let desig_conglom = s.import_ascii_only_byte("PinDesignator_PositionConglomerate")?;
    if (desig_conglom & 1) != 0 {
        pin.designator_position_mode = PinItemMode::Custom;
        pin.designator_custom_rotation_anchor = if (desig_conglom & 2) != 0 {
            PinTextRotationAnchor::Component
        } else {
            PinTextRotationAnchor::Pin
        };
        pin.designator_custom_rotation_relative = RotationBy90::from_u8((desig_conglom & 0x0C) >> 2).unwrap_or_default();
        pin.designator_custom_position_margin = s.import_ascii_only_coord("Designator_CustomPosition_Margin")?;
    } else {
        pin.designator_position_mode = PinItemMode::Default;
    }
    if (desig_conglom & 0x10) != 0 {
        pin.designator_font_mode = PinItemMode::Custom;
        pin.designator_custom_font_id = s.import_ascii_only_font_id("Designator_CustomFontID")?;
        pin.designator_custom_color = s.import_ascii_only_color("Designator_CustomColor")?;
    } else {
        pin.designator_font_mode = PinItemMode::Default;
    }

    pin.symbol_line_width = Size::from_u8(s.import_ascii_only_byte("SymBol_LineWidth")?).unwrap_or_default();
    pin.pin_package_length = s.import_ascii_only_coord("PinPackageLength")?;
    pin.pin_propagation_delay = s.import_ascii_only_double("PinPropagationDelay")?;
    pin.unique_id = s.import_dynamic_string("UniqueID")?;
    pin.hide_pin_name_as_function = s.import_ascii_only_boolean("HidePinNameAsFunction")?;
    pin.pin_symbolic_name = s.import_ascii_only_string("PinSymbolicName")?;
    pin.show_symbolic_name_as_function = s.import_ascii_only_boolean("ShowPinSymbolicNameAsFunction")?;

    Ok(())
}
