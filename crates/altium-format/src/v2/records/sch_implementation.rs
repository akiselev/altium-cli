//! Implementation record (RECORD=45).

use altium_format_derive::altium_record;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::backing_store::{ParamOrigin, RecordOrigin};

    #[test]
    fn roundtrip() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=45|OWNERINDEX=1|DESCRIPTION=Footprint|USECOMPONENTLIBRARY=T|MODELNAME=DIP-8|MODELTYPE=PCBLIB|DATAFILECOUNT=0|ISCURRENT=T|INTEGRATEDMODEL=F|DATABASEMODEL=F|UNIQUEID=ABCD1234|",
        ));
        let rec = SchImplementationRecord::from_origin(origin);
        assert_eq!(rec.owner_index(), 1);
        assert_eq!(rec.model_name(), "DIP-8");
        assert_eq!(rec.model_type(), "PCBLIB");
        assert!(rec.is_current());
        assert!(rec.use_component_library());
    }

    #[test]
    fn setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=45|MODELNAME=SOT-23|"));
        let mut rec = SchImplementationRecord::from_origin(origin);
        rec.set_model_name("QFP-44".to_string());
        assert_eq!(rec.model_name(), "QFP-44");
        rec.set_is_current(false);
        assert!(!rec.is_current());
    }
}
