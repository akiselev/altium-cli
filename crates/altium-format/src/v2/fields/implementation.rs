//! Implementation record data structs.

use super::{DataObjectBase, GraphicalObjectBase};

/// Implementation record data — from `ExportImplementation`/`ImportImplementation`.
#[derive(Clone, Debug, Default)]
pub struct ImplementationData {
    pub base: DataObjectBase,
    pub description: String,
    pub use_component_library: bool,
    pub model_name: String,
    pub model_type: String,
    pub datafile_count: i16,
    pub model_vault_guid: String,
    pub model_item_guid: String,
    pub model_revision_guid: String,
    /// Vec of (location, entity_name, file_kind) tuples.
    pub datafile_links: Vec<(String, String, String)>,
    pub is_current: bool,
    pub integrated_model: bool,
    pub database_model: bool,
    pub unique_id: String,
}

/// ImplementationList — just a graphical object wrapper.
#[derive(Clone, Debug, Default)]
pub struct ImplementationListData {
    pub graphical: GraphicalObjectBase,
}
