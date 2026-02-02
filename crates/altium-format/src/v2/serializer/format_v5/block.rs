//! Format functions for functional/schematic block record types.

use crate::error::Result;
use crate::v2::fields::block::*;
use crate::v2::fields::misc::{ObjectDefinitionData, AssociatedObjectsData};
use crate::v2::serializer::SchSerializer;
use super::{export_graphical_object, import_graphical_object, export_data_object, import_data_object, export_string_map, import_string_map, export_string_list, import_string_list};
use super::primitives::export_rectangle;
use super::primitives::import_rectangle;
use super::schematic::export_wire;
use super::schematic::import_wire;
use super::sheet::export_sheet_symbol;
use super::sheet::import_sheet_symbol;

// ============================================================================
// ObjectDefinition
// ============================================================================

pub fn export_object_definition(s: &mut dyn SchSerializer, od: &ObjectDefinitionData) -> Result<()> {
    s.export_dynamic_string(&od.object_definition_id, "ObjectDefinitionId")?;
    s.export_dynamic_string(&od.object_definition_hash, "ObjectDefinitionHash")?;
    s.export_dynamic_string(&od.database_table_name, "DatabaseTableName")?;
    s.export_dynamic_string(&od.design_item_id, "DesignItemId")?;
    s.export_dynamic_string(&od.item_guid, "ItemGUID")?;
    s.export_dynamic_string(&od.library_path, "LibraryPath")?;
    s.export_dynamic_string(&od.lib_reference, "LibReference")?;
    s.export_dynamic_string(&od.revision_guid, "RevisionGUID")?;
    s.export_dynamic_string(&od.source_library_name, "SourceLibraryName")?;
    s.export_dynamic_string(&od.target_file_name, "TargetFileName")?;
    s.export_boolean(od.not_use_db_table_name, "NotUseDBTableName")?;
    s.export_boolean(od.not_use_library_name, "NotUseLibraryName")?;
    s.export_dynamic_string(&od.vault_guid, "VaultGUID")?;
    export_data_object(s, &od.base)?;
    Ok(())
}

pub fn import_object_definition(s: &mut dyn SchSerializer, od: &mut ObjectDefinitionData) -> Result<()> {
    od.object_definition_id = s.import_dynamic_string("ObjectDefinitionId")?;
    od.object_definition_hash = s.import_dynamic_string("ObjectDefinitionHash")?;
    od.database_table_name = s.import_dynamic_string("DatabaseTableName")?;
    od.design_item_id = s.import_dynamic_string("DesignItemId")?;
    od.item_guid = s.import_dynamic_string("ItemGUID")?;
    od.library_path = s.import_dynamic_string("LibraryPath")?;
    od.lib_reference = s.import_dynamic_string("LibReference")?;
    od.revision_guid = s.import_dynamic_string("RevisionGUID")?;
    od.source_library_name = s.import_dynamic_string("SourceLibraryName")?;
    od.target_file_name = s.import_dynamic_string("TargetFileName")?;
    od.not_use_db_table_name = s.import_boolean("NotUseDBTableName")?;
    od.not_use_library_name = s.import_boolean("NotUseLibraryName")?;
    od.vault_guid = s.import_dynamic_string("VaultGUID")?;
    import_data_object(s, &mut od.base)?;
    Ok(())
}

// ============================================================================
// AssociatedObjects
// ============================================================================

pub fn export_associated_objects(s: &mut dyn SchSerializer, ao: &AssociatedObjectsData) -> Result<()> {
    export_data_object(s, &ao.base)?;
    s.export_byte(ao.associated_object_type, "AssociatedObjectType")?;
    Ok(())
}

pub fn import_associated_objects(s: &mut dyn SchSerializer, ao: &mut AssociatedObjectsData) -> Result<()> {
    import_data_object(s, &mut ao.base)?;
    ao.associated_object_type = s.import_byte("AssociatedObjectType")?;
    Ok(())
}

// ============================================================================
// FunctionalBlock
// ============================================================================

pub fn export_functional_block(s: &mut dyn SchSerializer, fb: &FunctionalBlockData) -> Result<()> {
    export_rectangle(s, &fb.rect)?;
    s.export_font_id(fb.font_id, "FontID")?;
    s.export_color(fb.text_color, "TextColor")?;
    s.export_dynamic_string(&fb.name, "Name")?;
    s.export_dynamic_string(&fb.file_name, "FileName")?;
    Ok(())
}

pub fn import_functional_block(s: &mut dyn SchSerializer, fb: &mut FunctionalBlockData) -> Result<()> {
    import_rectangle(s, &mut fb.rect)?;
    fb.font_id = s.import_font_id("FontID")?;
    fb.text_color = s.import_color("TextColor")?;
    fb.name = s.import_dynamic_string("Name")?;
    fb.file_name = s.import_dynamic_string("FileName")?;
    Ok(())
}

// ============================================================================
// FunctionalConnectionLine
// ============================================================================

pub fn export_functional_connection_line(s: &mut dyn SchSerializer, fcl: &FunctionalConnectionLineData) -> Result<()> {
    export_wire(s, &fcl.wire)?;
    s.export_dynamic_string(&fcl.instance_label, "InstanceLabel")?;
    s.export_boolean(fcl.designator_locked, "DesignatorLocked")?;
    Ok(())
}

pub fn import_functional_connection_line(s: &mut dyn SchSerializer, fcl: &mut FunctionalConnectionLineData) -> Result<()> {
    import_wire(s, &mut fcl.wire)?;
    fcl.instance_label = s.import_dynamic_string("InstanceLabel")?;
    fcl.designator_locked = s.import_boolean("DesignatorLocked")?;
    Ok(())
}

// ============================================================================
// SchematicBlock
// ============================================================================

pub fn export_schematic_block(s: &mut dyn SchSerializer, sb: &SchematicBlockData) -> Result<()> {
    export_graphical_object(s, &sb.graphical)?;
    s.export_dynamic_string(&sb.name, "Name")?;
    s.export_coord(sb.location_x, "Location.X")?;
    s.export_coord(sb.location_y, "Location.Y")?;
    s.export_coord(sb.corner_x, "Corner.X")?;
    s.export_coord(sb.corner_y, "Corner.Y")?;
    s.export_line_style(sb.line_style, "LineStyleExt")?;
    s.export_size(sb.line_width, "LineWidth")?;
    s.export_rotation_by90(sb.orientation, "Orientation")?;
    s.export_color(sb.color, "Color")?;
    s.export_color(sb.area_color, "AreaColor")?;
    s.export_boolean(sb.is_solid, "IsSolid")?;
    s.export_boolean(sb.transparent, "Transparent")?;
    s.export_dynamic_string(&sb.unique_id, "UniqueID")?;
    s.export_boolean(sb.designator_locked, "DesignatorLocked")?;
    s.export_dynamic_string(&sb.design_item_id, "DesignItemId")?;
    s.export_dynamic_string(&sb.source_library_name, "SourceLibraryName")?;
    // String map: PowerObjectsNameMappings
    export_string_map(s, &sb.power_objects_name_mappings, "PowerObjectsNameMappingsCount", "PowerObjectsNameOriginal", "PowerObjectsNameMapped")?;
    // String list: RBServerParameters
    export_string_list(s, &sb.rb_server_parameters, "RBServerParametersCount", "RBServerParametersName")?;
    // ReuseBlockImplementationInfo
    export_reuse_block_implementation_info(s, &sb.reuse_block_info)?;
    Ok(())
}

pub fn import_schematic_block(s: &mut dyn SchSerializer, sb: &mut SchematicBlockData) -> Result<()> {
    import_graphical_object(s, &mut sb.graphical)?;
    sb.name = s.import_dynamic_string("Name")?;
    sb.location_x = s.import_coord("Location.X")?;
    sb.location_y = s.import_coord("Location.Y")?;
    sb.corner_x = s.import_coord("Corner.X")?;
    sb.corner_y = s.import_coord("Corner.Y")?;
    sb.line_style = s.import_line_style("LineStyleExt")?;
    sb.line_width = s.import_size("LineWidth")?;
    sb.orientation = s.import_rotation_by90("Orientation")?;
    sb.color = s.import_color("Color")?;
    sb.area_color = s.import_color("AreaColor")?;
    sb.is_solid = s.import_boolean("IsSolid")?;
    sb.transparent = s.import_boolean("Transparent")?;
    sb.unique_id = s.import_dynamic_string("UniqueID")?;
    sb.designator_locked = s.import_boolean("DesignatorLocked")?;
    sb.design_item_id = s.import_dynamic_string("DesignItemId")?;
    sb.source_library_name = s.import_dynamic_string("SourceLibraryName")?;
    sb.power_objects_name_mappings = import_string_map(s, "PowerObjectsNameMappingsCount", "PowerObjectsNameOriginal", "PowerObjectsNameMapped")?;
    sb.rb_server_parameters = import_string_list(s, "RBServerParametersCount", "RBServerParametersName")?;
    import_reuse_block_implementation_info(s, &mut sb.reuse_block_info)?;
    Ok(())
}

// ============================================================================
// ReuseBlockImplementationInfo (helper)
// ============================================================================

pub fn export_reuse_block_implementation_info(s: &mut dyn SchSerializer, rb: &ReuseBlockImplementationInfoData) -> Result<()> {
    s.export_dynamic_string(&rb.name, "Name")?;
    s.export_dynamic_string(&rb.description, "Description")?;
    s.export_dynamic_string(&rb.design_item_id, "DesignItemId")?;
    s.export_dynamic_string(&rb.block_server_name, "BlockServerName")?;
    s.export_dynamic_string(&rb.block_vault_guid, "BlockVaultGUID")?;
    s.export_dynamic_string(&rb.block_item_guid, "BlockItemGUID")?;
    s.export_dynamic_string(&rb.block_item_revision_guid, "BlockItemRevisionGUID")?;
    s.export_dynamic_string(&rb.sch_snippet_vault_guid, "SchSnippetVaultGUID")?;
    s.export_dynamic_string(&rb.sch_snippet_item_guid, "SchSnippetItemGUID")?;
    s.export_dynamic_string(&rb.sch_snippet_item_revision_guid, "SchSnippetItemRevisionGUID")?;
    s.export_dynamic_string(&rb.pcb_snippet_vault_guid, "PcbSnippetVaultGUID")?;
    s.export_dynamic_string(&rb.pcb_snippet_item_guid, "PcbSnippetItemGUID")?;
    s.export_dynamic_string(&rb.pcb_snippet_item_revision_guid, "PcbSnippetItemRevisionGUID")?;
    s.export_dynamic_string(&rb.reuse_block_id, "ReuseBlockId")?;
    s.export_boolean(rb.is_dissolved, "IsDissolved")?;
    if !rb.reuse_block_objects_ids.is_empty() {
        s.export_dynamic_string(&rb.reuse_block_objects_ids, "ReuseBlockObjectsIds")?;
    }
    export_string_map(s, &rb.docs_file_names_mappings, "DocsFileNamesMappingsCount", "DocFileNameOriginal", "DocFileNameMapped")?;
    if rb.is_dissolved && !rb.parameters.is_empty() {
        export_string_map(s, &rb.parameters, "ParametersCount", "ParameterName", "ParameterValue")?;
    }
    Ok(())
}

pub fn import_reuse_block_implementation_info(s: &mut dyn SchSerializer, rb: &mut ReuseBlockImplementationInfoData) -> Result<()> {
    rb.name = s.import_dynamic_string("Name")?;
    rb.description = s.import_dynamic_string("Description")?;
    rb.design_item_id = s.import_dynamic_string("DesignItemId")?;
    rb.block_server_name = s.import_dynamic_string("BlockServerName")?;
    rb.block_vault_guid = s.import_dynamic_string("BlockVaultGUID")?;
    rb.block_item_guid = s.import_dynamic_string("BlockItemGUID")?;
    rb.block_item_revision_guid = s.import_dynamic_string("BlockItemRevisionGUID")?;
    rb.sch_snippet_vault_guid = s.import_dynamic_string("SchSnippetVaultGUID")?;
    rb.sch_snippet_item_guid = s.import_dynamic_string("SchSnippetItemGUID")?;
    rb.sch_snippet_item_revision_guid = s.import_dynamic_string("SchSnippetItemRevisionGUID")?;
    rb.pcb_snippet_vault_guid = s.import_dynamic_string("PcbSnippetVaultGUID")?;
    rb.pcb_snippet_item_guid = s.import_dynamic_string("PcbSnippetItemGUID")?;
    rb.pcb_snippet_item_revision_guid = s.import_dynamic_string("PcbSnippetItemRevisionGUID")?;
    rb.reuse_block_id = s.import_dynamic_string("ReuseBlockId")?;
    rb.is_dissolved = s.import_boolean("IsDissolved")?;
    rb.reuse_block_objects_ids = s.import_dynamic_string("ReuseBlockObjectsIds")?;
    rb.docs_file_names_mappings = import_string_map(s, "DocsFileNamesMappingsCount", "DocFileNameOriginal", "DocFileNameMapped")?;
    rb.parameters = import_string_map(s, "ParametersCount", "ParameterName", "ParameterValue")?;
    Ok(())
}

// ============================================================================
// ReuseSheetSymbol
// ============================================================================

pub fn export_reuse_sheet_symbol(s: &mut dyn SchSerializer, rss: &ReuseSheetSymbolData) -> Result<()> {
    export_sheet_symbol(s, &rss.sheet_symbol)?;
    s.export_dynamic_string(&rss.name, "Name")?;
    s.export_boolean(rss.designator_locked, "DesignatorLocked")?;
    export_string_map(s, &rss.power_objects_name_mappings, "PowerObjectsNameMappingsCount", "PowerObjectsNameOriginal", "PowerObjectsNameMapped")?;
    export_string_list(s, &rss.rb_server_parameters, "RBServerParametersCount", "RBServerParametersName")?;
    export_reuse_block_implementation_info(s, &rss.reuse_block_info)?;
    Ok(())
}

pub fn import_reuse_sheet_symbol(s: &mut dyn SchSerializer, rss: &mut ReuseSheetSymbolData) -> Result<()> {
    import_sheet_symbol(s, &mut rss.sheet_symbol)?;
    rss.name = s.import_dynamic_string("Name")?;
    rss.designator_locked = s.import_boolean("DesignatorLocked")?;
    rss.power_objects_name_mappings = import_string_map(s, "PowerObjectsNameMappingsCount", "PowerObjectsNameOriginal", "PowerObjectsNameMapped")?;
    rss.rb_server_parameters = import_string_list(s, "RBServerParametersCount", "RBServerParametersName")?;
    import_reuse_block_implementation_info(s, &mut rss.reuse_block_info)?;
    Ok(())
}
