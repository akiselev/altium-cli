// Model, implementation, and footprint link parameters (RECORD=45).
//
// These live on Implementation records (children of a Component). Each
// component has an ImplementationMap containing zero or more Implementations.

// ---------------------------------------------------------------------------
// Model type and identity
// ---------------------------------------------------------------------------

/// Model type string.
///
/// **Wire type:** string
/// **Used by:** Implementation (RECORD=45)
///
/// Known values: `"PCBLIB"`, `"SIM"`, `"PCB3DLib"`, `"PCADLib"`, `"SI"`,
/// `"VHD"`, `"SCHLIB"`, `"SCH"`, `"Datasheet"`, `"HarnessWiring"`,
/// `"HarnessLayout"`.
pub const MODEL_TYPE: &str = "ModelType";

/// Footprint/model name within the library.
///
/// **Wire type:** string
/// **Used by:** Implementation (RECORD=45)
pub const MODEL_NAME: &str = "ModelName";

/// Legacy alternative to `DatafileCount` pattern for model location.
///
/// **Wire type:** DynamicString
/// **Used by:** Implementation (RECORD=45)
pub const MODEL_LOCATION: &str = "ModelLocation";

// ---------------------------------------------------------------------------
// Datafile triplets
// ---------------------------------------------------------------------------

/// Number of `ModelDatafile` triplets.
///
/// **Wire type:** i16
/// **Used by:** Implementation (RECORD=45)
pub const DATAFILE_COUNT: &str = "DatafileCount";

/// File path or library identifier for Nth datafile.
///
/// **Wire type:** DynamicString (indexed as `ModelDatafile0`, `ModelDatafile1`, ...)
/// **Used by:** Implementation (RECORD=45)
pub const MODEL_DATAFILE: &str = "ModelDatafile";

/// Entity name within the datafile.
///
/// **Wire type:** DynamicString (indexed as `ModelDatafileEntity0`, ...)
/// **Used by:** Implementation (RECORD=45)
///
/// Falls back to `ModelName` if empty.
pub const MODEL_DATAFILE_ENTITY: &str = "ModelDatafileEntity";

/// Kind/type of the datafile (e.g., `"PCBLib"`).
///
/// **Wire type:** DynamicString (indexed as `ModelDatafileKind0`, ...)
/// **Used by:** Implementation (RECORD=45)
pub const MODEL_DATAFILE_KIND: &str = "ModelDatafileKind";

// ---------------------------------------------------------------------------
// Flags
// ---------------------------------------------------------------------------

/// Model comes from an integrated library (.IntLib).
///
/// **Wire type:** bool
/// **Used by:** Implementation (RECORD=45)
pub const INTEGRATED_MODEL: &str = "IntegratedModel";

/// Model comes from a database library.
///
/// **Wire type:** bool
/// **Used by:** Implementation (RECORD=45)
pub const DATABASE_MODEL: &str = "DatabaseModel";

// ---------------------------------------------------------------------------
// Vault references
// ---------------------------------------------------------------------------

/// Vault item GUID for this model.
///
/// **Wire type:** DynamicString
/// **Used by:** Implementation (RECORD=45)
pub const MODEL_ITEM_GUID: &str = "ModelItemGUID";

/// Vault revision GUID for this model.
///
/// **Wire type:** DynamicString
/// **Used by:** Implementation (RECORD=45)
pub const MODEL_REVISION_GUID: &str = "ModelRevisionGUID";

/// Vault instance GUID for this model.
///
/// **Wire type:** DynamicString
/// **Used by:** Implementation (RECORD=45)
pub const MODEL_VAULT_GUID: &str = "ModelVaultGUID";

// ---------------------------------------------------------------------------
// Legacy
// ---------------------------------------------------------------------------

/// V4 format footprint slot (indexed as `Footprint0`..`Footprint3`).
///
/// **Wire type:** string
/// **Used by:** V4 format only (migrated to Implementation on load)
///
/// V4 supports exactly 4 footprint slots.
pub const FOOTPRINT: &str = "Footprint";

// ---------------------------------------------------------------------------
// Library linking
// ---------------------------------------------------------------------------

/// Model is linked to the component's library.
///
/// **Wire type:** bool
/// **Used by:** Implementation (RECORD=45)
///
/// Also written as `DatabaseDatalinksLocked` and `DatalinksLocked` for
/// backward compatibility. On import, all three are collapsed into this
/// single flag.
pub const USE_COMPONENT_LIBRARY: &str = "UseComponentLibrary";

/// Database datalinks locked (backward-compat alias for UseComponentLibrary).
///
/// **Wire type:** bool
/// **Used by:** Implementation (RECORD=45)
///
/// Collapsed into `UseComponentLibrary` on import.
pub const DATABASE_DATALINKS_LOCKED: &str = "DatabaseDatalinksLocked";

/// Datalinks locked (backward-compat alias for UseComponentLibrary).
///
/// **Wire type:** bool
/// **Used by:** Implementation (RECORD=45)
///
/// Collapsed into `UseComponentLibrary` on import.
pub const DATALINKS_LOCKED: &str = "DatalinksLocked";

/// Object ID for model identification.
///
/// **Wire type:** i32
/// **Used by:** various model/implementation objects
pub const OBJECT_ID: &str = "ObjectId";
