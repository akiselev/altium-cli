//! Format functions for Implementation record types.

use crate::error::Result;
use crate::v2::fields::implementation::*;
use crate::v2::serializer::SchSerializer;
use super::{export_data_object, import_data_object};

pub fn export_implementation(s: &mut dyn SchSerializer, imp: &ImplementationData) -> Result<()> {
    export_data_object(s, &imp.base)?;
    s.export_dynamic_string(&imp.description, "Description")?;
    s.export_boolean(imp.use_component_library, "UseComponentLibrary")?;
    s.export_string(&imp.model_name, "ModelName")?;
    s.export_string(&imp.model_type, "ModelType")?;
    s.export_short_int(imp.datafile_links.len() as i32, "DatafileCount")?;
    s.export_dynamic_string(&imp.model_vault_guid, "ModelVaultGUID")?;
    s.export_dynamic_string(&imp.model_item_guid, "ModelItemGUID")?;
    s.export_dynamic_string(&imp.model_revision_guid, "ModelRevisionGUID")?;
    for (i, (location, entity, kind)) in imp.datafile_links.iter().enumerate() {
        let idx = i.to_string();
        s.export_dynamic_string(location, &format!("ModelDatafile{}", idx))?;
        s.export_dynamic_string(entity, &format!("ModelDatafileEntity{}", idx))?;
        s.export_dynamic_string(kind, &format!("ModelDatafileKind{}", idx))?;
    }
    s.export_boolean(imp.is_current, "IsCurrent")?;
    s.export_boolean(imp.use_component_library, "DatalinksLocked")?;
    s.export_boolean(imp.use_component_library, "DatabaseDatalinksLocked")?;
    s.export_boolean(imp.integrated_model, "IntegratedModel")?;
    s.export_boolean(imp.database_model, "DatabaseModel")?;
    s.export_dynamic_string(&imp.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_implementation(s: &mut dyn SchSerializer, imp: &mut ImplementationData) -> Result<()> {
    import_data_object(s, &mut imp.base)?;
    imp.description = s.import_dynamic_string("Description")?;
    let use_comp_lib = s.import_boolean("UseComponentLibrary")?;
    imp.model_name = s.import_string("ModelName")?;
    imp.model_type = s.import_string("ModelType")?;
    imp.is_current = s.import_boolean("IsCurrent")?;
    let datalinks_locked = s.import_boolean("DatalinksLocked")?;
    let db_datalinks_locked = s.import_boolean("DatabaseDatalinksLocked")?;
    imp.use_component_library = use_comp_lib || datalinks_locked || db_datalinks_locked;
    imp.integrated_model = s.import_boolean("IntegratedModel")?;
    imp.database_model = s.import_boolean("DatabaseModel")?;
    imp.model_vault_guid = s.import_dynamic_string("ModelVaultGUID")?;
    imp.model_item_guid = s.import_dynamic_string("ModelItemGUID")?;
    imp.model_revision_guid = s.import_dynamic_string("ModelRevisionGUID")?;
    imp.unique_id = s.import_dynamic_string("UniqueID")?;
    let count = s.import_short_int("DatafileCount")?;
    imp.datafile_links.clear();
    for i in 0..count {
        let idx = i.to_string();
        let location = s.import_dynamic_string(&format!("ModelDatafile{}", idx))?;
        let entity = s.import_dynamic_string(&format!("ModelDatafileEntity{}", idx))?;
        let kind = s.import_dynamic_string(&format!("ModelDatafileKind{}", idx))?;
        imp.datafile_links.push((location, entity, kind));
    }
    Ok(())
}
