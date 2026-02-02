//! Format functions for parameter-related record types.

use crate::error::Result;
use crate::v2::fields::parameter::*;
use crate::v2::fields::implementation::*;
use crate::v2::serializer::SchSerializer;
use super::{export_graphical_object, import_graphical_object, export_data_object, import_data_object};

pub fn export_parameter(s: &mut dyn SchSerializer, p: &ParameterData) -> Result<()> {
    export_graphical_object(s, &p.graphical)?;
    s.export_coord(p.location_x, "Location.X")?;
    s.export_coord(p.location_y, "Location.Y")?;
    s.export_rotation_by90(p.orientation, "Orientation")?;
    s.export_text_justification(p.justification, "Justification")?;
    s.export_color(p.color, "Color")?;
    s.export_font_id(p.font_id, "FontID")?;
    s.export_boolean(p.is_hidden, "IsHidden")?;
    s.export_dynamic_string(&p.text, "Text")?;
    s.export_parameter_kind(p.param_type, "ParamType")?;
    s.export_string(&p.name, "Name")?;
    s.export_boolean(p.show_name, "ShowName")?;
    s.export_parameter_read_only_state(p.read_only_state, "ReadOnlyState")?;
    s.export_dynamic_string(&p.unique_id, "UniqueID")?;
    s.export_dynamic_string(&p.description, "Description")?;
    s.export_boolean(!p.allow_library_synchronize, "NotAllowLibrarySynchronize")?;
    s.export_boolean(!p.allow_database_synchronize, "NotAllowDatabaseSynchronize")?;
    s.export_boolean(!p.auto_position, "NotAutoPosition")?;
    s.export_boolean(p.is_mirrored, "IsMirrored")?;
    s.export_text_horizontal_anchor(p.text_horz_anchor, "TextHorzAnchor")?;
    s.export_text_vertical_anchor(p.text_vert_anchor, "TextVertAnchor")?;
    s.export_boolean(p.is_image_parameter, "IsImageParameter")?;
    Ok(())
}

pub fn import_parameter(s: &mut dyn SchSerializer, p: &mut ParameterData) -> Result<()> {
    import_graphical_object(s, &mut p.graphical)?;
    p.location_x = s.import_coord("Location.X")?;
    p.location_y = s.import_coord("Location.Y")?;
    p.orientation = s.import_rotation_by90("Orientation")?;
    p.justification = s.import_text_justification("Justification")?;
    p.color = s.import_color("Color")?;
    p.font_id = s.import_font_id("FontID")?;
    p.is_hidden = s.import_boolean("IsHidden")?;
    p.text = s.import_dynamic_string("Text")?;
    p.param_type = s.import_parameter_kind("ParamType")?;
    p.name = s.import_string("Name")?;
    p.show_name = s.import_boolean("ShowName")?;
    p.read_only_state = s.import_parameter_read_only_state("ReadOnlyState")?;
    p.unique_id = s.import_dynamic_string("UniqueID")?;
    p.description = s.import_dynamic_string("Description")?;
    let not_allow_lib = s.import_boolean("NotAllowLibrarySynchronize")?;
    p.allow_library_synchronize = !not_allow_lib;
    let not_allow_db = s.import_boolean("NotAllowDatabaseSynchronize")?;
    p.allow_database_synchronize = !not_allow_db;
    let not_auto = s.import_boolean("NotAutoPosition")?;
    p.auto_position = !not_auto;
    p.is_mirrored = s.import_boolean("IsMirrored")?;
    p.text_horz_anchor = s.import_text_horizontal_anchor("TextHorzAnchor")?;
    p.text_vert_anchor = s.import_text_vertical_anchor("TextVertAnchor")?;
    p.is_image_parameter = s.import_boolean("IsImageParameter")?;
    Ok(())
}

// ============================================================================
// Designator (ObjectId = 34)
// ============================================================================

pub fn export_designator(s: &mut dyn SchSerializer, d: &DesignatorData) -> Result<()> {
    export_parameter(s, &d.param)?;
    if d.override_not_auto_position {
        s.export_boolean(true, "OverrideNotAutoPosition")?;
    }
    Ok(())
}

pub fn import_designator(s: &mut dyn SchSerializer, d: &mut DesignatorData) -> Result<()> {
    import_parameter(s, &mut d.param)?;
    d.override_not_auto_position = s.import_boolean("OverrideNotAutoPosition")?;
    if d.override_not_auto_position {
        d.param.auto_position = false;
    }
    Ok(())
}

pub fn export_implementation_list(s: &mut dyn SchSerializer, il: &ImplementationListData) -> Result<()> {
    export_graphical_object(s, &il.graphical)
}

pub fn import_implementation_list(s: &mut dyn SchSerializer, il: &mut ImplementationListData) -> Result<()> {
    import_graphical_object(s, &mut il.graphical)
}

// ============================================================================
// ParameterList (RECORD=29) — just graphical object wrapper
// ============================================================================

pub fn export_parameter_list(s: &mut dyn SchSerializer, pl: &ParameterListData) -> Result<()> {
    export_graphical_object(s, &pl.graphical)
}

pub fn import_parameter_list(s: &mut dyn SchSerializer, pl: &mut ParameterListData) -> Result<()> {
    import_graphical_object(s, &mut pl.graphical)
}

// ============================================================================
// ParameterSet (RECORD=28)
// ============================================================================

pub fn export_parameter_set(s: &mut dyn SchSerializer, ps: &ParameterSetData) -> Result<()> {
    export_graphical_object(s, &ps.graphical)?;
    s.export_coord(ps.location_x, "Location.X")?;
    s.export_coord(ps.location_y, "Location.Y")?;
    s.export_color(ps.color, "Color")?;
    s.export_rotation_by90(ps.orientation, "Orientation")?;
    s.export_dynamic_string(&ps.name, "Name")?;
    s.export_parameter_set_style(ps.style, "Style")?;
    s.export_dynamic_string(&ps.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_parameter_set(s: &mut dyn SchSerializer, ps: &mut ParameterSetData) -> Result<()> {
    import_graphical_object(s, &mut ps.graphical)?;
    ps.location_x = s.import_coord("Location.X")?;
    ps.location_y = s.import_coord("Location.Y")?;
    ps.color = s.import_color("Color")?;
    ps.orientation = s.import_rotation_by90("Orientation")?;
    ps.name = s.import_dynamic_string("Name")?;
    ps.style = s.import_parameter_set_style("Style")?;
    ps.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

// ============================================================================
// MapDefiner (RECORD=44)
// ============================================================================

pub fn export_map_definer(s: &mut dyn SchSerializer, md: &MapDefinerData) -> Result<()> {
    export_data_object(s, &md.base)?;
    s.export_string(&md.designator_interface, "DesIntf")?;
    s.export_long_int(md.implementation_designators.len() as i32, "DesImpCount")?;
    for (i, des) in md.implementation_designators.iter().enumerate() {
        s.export_string(des, &format!("DesImp{}", i))?;
    }
    Ok(())
}

pub fn import_map_definer(s: &mut dyn SchSerializer, md: &mut MapDefinerData) -> Result<()> {
    import_data_object(s, &mut md.base)?;
    md.designator_interface = s.import_dynamic_string("DesIntf")?;
    let count = s.import_long_int("DesImpCount")?;
    md.implementation_designators.clear();
    for i in 0..count {
        let des = s.import_string(&format!("DesImp{}", i))?;
        md.implementation_designators.push(des);
    }
    Ok(())
}

// ============================================================================
// ImplementationMap (RECORD=45) — just data object wrapper
// ============================================================================

pub fn export_implementation_map(s: &mut dyn SchSerializer, im: &ImplementationMapData) -> Result<()> {
    export_data_object(s, &im.base)
}

pub fn import_implementation_map(s: &mut dyn SchSerializer, im: &mut ImplementationMapData) -> Result<()> {
    import_data_object(s, &mut im.base)
}
