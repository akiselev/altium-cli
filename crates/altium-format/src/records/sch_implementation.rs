//! Implementation record (RECORD=45).

use crate::traits::RecordType;
use altium_format_derive::altium_record;

/// A single dynamic model-datafile link attached to an implementation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SchImplementationDatafileLink {
    pub location: String,
    pub entity: String,
    pub kind: String,
}

/// Implementation record — links a component to a model (footprint, simulation, etc.).
///
/// Corresponds to `ImplementationData` / `ExportImplementation` in the v1 API
/// (ObjectId::Implementation = 45).
///
/// Note: The `datafile_links` Vec is skipped in this phase as it uses dynamic
/// indexed keys (ModelDatafile0, ModelDatafileEntity0, etc.) and will be
/// handled in a later phase.
#[altium_record(kind = "sch", record_id = 45, codec = "params")]
pub struct SchImplementationRecord {
    // --- DataObjectBase (flattened) ---
    #[altium(key = "OWNERINDEX")]
    owner_index: i32,
    #[altium(key = "ISNOTACCESIBLE")]
    is_not_accessible: bool,
    #[altium(key = "OWNERINDEXADDITIONALLIST")]
    owner_index_additional_list: bool,
    #[altium(key = "INDEXINSHEET")]
    index_in_sheet: i32,

    // --- Implementation-specific fields ---
    #[altium(key = "DESCRIPTION")]
    description: String,
    #[altium(key = "USECOMPONENTLIBRARY")]
    use_component_library: bool,
    #[altium(key = "MODELNAME")]
    model_name: String,
    #[altium(key = "MODELTYPE")]
    model_type: String,
    #[altium(key = "DATAFILECOUNT")]
    datafile_count: i16,
    #[altium(key = "MODELVAULTGUID")]
    model_vault_guid: String,
    #[altium(key = "MODELITEMGUID")]
    model_item_guid: String,
    #[altium(key = "MODELREVISIONGUID")]
    model_revision_guid: String,
    #[altium(key = "ISCURRENT")]
    is_current: bool,
    #[altium(key = "DATALINKSLOCKED")]
    datalinks_locked: bool,
    #[altium(key = "DATABASEDATALINKSLOCKED")]
    database_datalinks_locked: bool,
    #[altium(key = "INTEGRATEDMODEL")]
    integrated_model: bool,
    #[altium(key = "DATABASEMODEL")]
    database_model: bool,
    #[altium(key = "UNIQUEID")]
    unique_id: String,

    /// Datafile links — skipped; handled in later phase.
    #[altium(skip)]
    _datafile_links: i32,
}

impl SchImplementationRecord {
    /// Returns dynamic `ModelDatafile{n}` links.
    pub fn datafile_links(&self) -> crate::error::Result<Vec<SchImplementationDatafileLink>> {
        let Some(param) = self.origin().as_param() else {
            return Ok(Vec::new());
        };
        let params = &param.params;
        let count = self.datafile_count()?.max(0) as usize;
        let mut links = Vec::with_capacity(count);
        for i in 0..count {
            let idx = i.to_string();
            let location = params
                .get(&format!("ModelDatafile{}", idx))
                .map(|v| v.as_str().to_string())
                .unwrap_or_default();
            let entity = params
                .get(&format!("ModelDatafileEntity{}", idx))
                .map(|v| v.as_str().to_string())
                .unwrap_or_else(|| self.model_name().map(|s| s.to_string()).unwrap_or_default());
            let kind = params
                .get(&format!("ModelDatafileKind{}", idx))
                .map(|v| v.as_str().to_string())
                .unwrap_or_default();
            links.push(SchImplementationDatafileLink {
                location,
                entity,
                kind,
            });
        }
        Ok(links)
    }

    /// Replaces dynamic `ModelDatafile{n}` links and updates `DatafileCount`.
    pub fn set_datafile_links(&mut self, links: &[SchImplementationDatafileLink]) {
        self.set_datafile_count(links.len() as i16);
        let Some(param) = self.origin_mut().as_param_mut() else {
            return;
        };
        let params = &mut param.params;

        let mut to_remove = Vec::new();
        for (key, _) in params.iter() {
            let key = key.to_ascii_uppercase();
            if key
                .strip_prefix("MODELDATAFILE")
                .and_then(|s| s.parse::<usize>().ok())
                .is_some()
            {
                to_remove.push(key);
                continue;
            }
            if key
                .strip_prefix("MODELDATAFILEENTITY")
                .and_then(|s| s.parse::<usize>().ok())
                .is_some()
            {
                to_remove.push(key);
                continue;
            }
            if key
                .strip_prefix("MODELDATAFILEKIND")
                .and_then(|s| s.parse::<usize>().ok())
                .is_some()
            {
                to_remove.push(key);
            }
        }
        for key in to_remove {
            params.remove(&key);
        }

        for (i, link) in links.iter().enumerate() {
            let idx = i.to_string();
            if !link.location.is_empty() {
                params.add(&format!("ModelDatafile{}", idx), &link.location);
            }
            if !link.entity.is_empty() {
                params.add(&format!("ModelDatafileEntity{}", idx), &link.entity);
            }
            if !link.kind.is_empty() {
                params.add(&format!("ModelDatafileKind{}", idx), &link.kind);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backing_store::{ParamOrigin, RecordOrigin};

    #[test]
    fn roundtrip() -> crate::error::Result<()> {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=45|OWNERINDEX=1|DESCRIPTION=Footprint|USECOMPONENTLIBRARY=T|MODELNAME=DIP-8|MODELTYPE=PCBLIB|DATAFILECOUNT=0|ISCURRENT=T|INTEGRATEDMODEL=F|DATABASEMODEL=F|UNIQUEID=ABCD1234|",
        ));
        let rec = SchImplementationRecord::from_origin(origin);
        assert_eq!(rec.owner_index()?, 1);
        assert_eq!(rec.model_name()?, "DIP-8");
        assert_eq!(rec.model_type()?, "PCBLIB");
        assert!(rec.is_current()?);
        assert!(rec.use_component_library()?);
        Ok(())
    }

    #[test]
    fn setter() -> crate::error::Result<()> {
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=45|MODELNAME=SOT-23|"));
        let mut rec = SchImplementationRecord::from_origin(origin);
        rec.set_model_name("QFP-44".to_string());
        assert_eq!(rec.model_name()?, "QFP-44");
        rec.set_is_current(false);
        assert!(!rec.is_current()?);
        Ok(())
    }

    #[test]
    fn dynamic_datafile_links_roundtrip() -> crate::error::Result<()> {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=45|MODELNAME=DIP8|DATAFILECOUNT=2|MODELDATAFILE0=foo.PcbLib|MODELDATAFILEENTITY0=DIP8|MODELDATAFILEKIND0=PCBLIB|MODELDATAFILE1=bar.step|MODELDATAFILEENTITY1=MECH|MODELDATAFILEKIND1=STEP|",
        ));
        let rec = SchImplementationRecord::from_origin(origin);
        let links = rec.datafile_links()?;
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].location, "foo.PcbLib");
        assert_eq!(links[1].kind, "STEP");
        Ok(())
    }

    #[test]
    fn set_dynamic_datafile_links_updates_params() -> crate::error::Result<()> {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=45|MODELNAME=DIP8|DATAFILECOUNT=1|MODELDATAFILE0=old|MODELDATAFILEENTITY0=old|MODELDATAFILEKIND0=old|",
        ));
        let mut rec = SchImplementationRecord::from_origin(origin);
        rec.set_datafile_links(&[SchImplementationDatafileLink {
            location: "new.PcbLib".to_string(),
            entity: "DIP8".to_string(),
            kind: "PCBLIB".to_string(),
        }]);
        let links = rec.datafile_links()?;
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].location, "new.PcbLib");
        Ok(())
    }
}
