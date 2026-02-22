// Vault GUIDs, database links, library sync, and managed component parameters.
//
// These constants define the parameter keys for vault/server integration,
// lifecycle management, and database library linking.

// ---------------------------------------------------------------------------
// Core vault identifiers
// ---------------------------------------------------------------------------

/// Vault server GUID.
///
/// **Wire type:** DynamicString
/// **Used by:** Component (RECORD=1), Implementation (RECORD=45)
pub const VAULT_GUID: &str = "VaultGUID";

/// Item GUID within the vault.
///
/// **Wire type:** DynamicString
/// **Used by:** Component (RECORD=1), Implementation (RECORD=45)
pub const ITEM_GUID: &str = "ItemGUID";

/// Specific revision GUID of the vault item.
///
/// **Wire type:** DynamicString
/// **Used by:** Component (RECORD=1), Implementation (RECORD=45)
pub const ITEM_REVISION_GUID: &str = "ItemRevisionGUID";

/// Revision GUID (alias used by some object types).
///
/// **Wire type:** DynamicString
/// **Used by:** various objects
pub const REVISION_GUID: &str = "RevisionGUID";

// ---------------------------------------------------------------------------
// Design item
// ---------------------------------------------------------------------------

/// Human-readable vault/DB item identifier.
///
/// **Wire type:** DynamicString
/// **Used by:** Component (RECORD=1), SheetSymbol, ObjectDefinition
pub const DESIGN_ITEM_ID: &str = "DesignItemId";

/// Folder GUID in the vault.
///
/// **Wire type:** DynamicString
/// **Used by:** library objects
pub const FOLDER_GUID: &str = "FolderGUID";

/// Revision name string.
///
/// **Wire type:** DynamicString
/// **Used by:** library objects
pub const REVISION_NAME: &str = "RevisionName";

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

/// Lifecycle definition GUID.
///
/// **Wire type:** DynamicString
/// **Used by:** library objects
pub const LIFE_CYCLE_DEFINITION_GUID: &str = "LifeCycleDefinitionGUID";

/// Revision naming scheme GUID.
///
/// **Wire type:** DynamicString
/// **Used by:** library objects
pub const REVISION_NAMING_SCHEME_GUID: &str = "RevisionNamingSchemeGUID";

// ---------------------------------------------------------------------------
// Library
// ---------------------------------------------------------------------------

/// Source SchLib name for library sync.
///
/// **Wire type:** DynamicString
/// **Used by:** Component (RECORD=1), SheetSymbol
pub const SOURCE_LIBRARY_NAME: &str = "SourceLibraryName";

/// Library identifier string.
///
/// **Wire type:** DynamicString
/// **Used by:** various library-linked objects
pub const LIBRARY: &str = "Library";

/// Path to source library file.
///
/// **Wire type:** string
/// **Used by:** Component (RECORD=1)
pub const LIBRARY_PATH: &str = "LibraryPath";

/// Library field name.
///
/// **Wire type:** DynamicString
/// **Used by:** library-linked parameters
pub const LIBRARY_FIELD: &str = "LibraryField";

// ---------------------------------------------------------------------------
// Database
// ---------------------------------------------------------------------------

/// Model comes from a database library.
///
/// **Wire type:** bool
/// **Used by:** Implementation (RECORD=45)
pub const DATABASE_MODEL: &str = "DatabaseModel";

/// Database table name in DbLib.
///
/// **Wire type:** DynamicString
/// **Used by:** Component (RECORD=1), ObjectDefinition
pub const DATABASE_TABLE_NAME: &str = "DatabaseTableName";

// ---------------------------------------------------------------------------
// Sync flags (all stored inverted)
// ---------------------------------------------------------------------------

/// Excludes from database synchronization.
///
/// **Wire type:** bool
/// **Used by:** Parameter (RECORD=41)
///
/// **Gotcha:** inverted -- `true` means sync is NOT allowed.
pub const NOT_ALLOW_DATABASE_SYNCHRONIZE: &str = "NotAllowDatabaseSynchronize";

/// Excludes from library synchronization.
///
/// **Wire type:** bool
/// **Used by:** Parameter (RECORD=41)
///
/// **Gotcha:** inverted -- `true` means sync is NOT allowed.
pub const NOT_ALLOW_LIBRARY_SYNCHRONIZE: &str = "NotAllowLibrarySynchronize";

/// Don't use stored DB table name.
///
/// **Wire type:** bool
/// **Used by:** Component (RECORD=1), ObjectDefinition
///
/// **Gotcha:** inverted semantics.
pub const NOT_USE_DB_TABLE_NAME: &str = "NotUseDBTableName";

/// Don't use stored library name.
///
/// **Wire type:** bool
/// **Used by:** Component (RECORD=1), ObjectDefinition
///
/// **Gotcha:** inverted from "UseLibraryName".
pub const NOT_USE_LIBRARY_NAME: &str = "NotUseLibraryName";

// ---------------------------------------------------------------------------
// Release GUIDs
// ---------------------------------------------------------------------------

/// Release item GUID.
///
/// **Wire type:** DynamicString
/// **Used by:** managed components
pub const RELEASE_ITEM_GUID: &str = "ReleaseItemGUID";

/// Release vault GUID.
///
/// **Wire type:** DynamicString
/// **Used by:** managed components
pub const RELEASE_VAULT_GUID: &str = "ReleaseVaultGUID";

// ---------------------------------------------------------------------------
// Props GUIDs
// ---------------------------------------------------------------------------

/// Properties revision GUID.
///
/// **Wire type:** DynamicString
/// **Used by:** managed components
pub const PROPS_REVISION_GUID: &str = "PropsRevisionGUID";

/// Properties vault GUID.
///
/// **Wire type:** DynamicString
/// **Used by:** managed components
pub const PROPS_VAULT_GUID: &str = "PropsVaultGUID";

// ---------------------------------------------------------------------------
// Symbol GUIDs
// ---------------------------------------------------------------------------

/// Symbol item GUID.
///
/// **Wire type:** DynamicString
/// **Used by:** managed components
pub const SYMBOL_ITEM_GUID: &str = "SymbolItemGUID";

/// Symbol revision GUID.
///
/// **Wire type:** DynamicString
/// **Used by:** managed components
pub const SYMBOL_REVISION_GUID: &str = "SymbolRevisionGUID";

/// Symbol vault GUID.
///
/// **Wire type:** DynamicString
/// **Used by:** managed components
pub const SYMBOL_VAULT_GUID: &str = "SymbolVaultGUID";

// ---------------------------------------------------------------------------
// Template GUIDs
// ---------------------------------------------------------------------------

/// Template item GUID.
///
/// **Wire type:** DynamicString
/// **Used by:** managed components
pub const TEMPLATE_ITEM_GUID: &str = "TemplateItemGUID";

/// Template revision GUID.
///
/// **Wire type:** DynamicString
/// **Used by:** managed components
pub const TEMPLATE_REVISION_GUID: &str = "TemplateRevisionGUID";

/// Template vault GUID.
///
/// **Wire type:** DynamicString
/// **Used by:** managed components
pub const TEMPLATE_VAULT_GUID: &str = "TemplateVaultGUID";

/// Template revision human-readable ID.
///
/// **Wire type:** DynamicString
/// **Used by:** managed components
pub const TEMPLATE_REVISION_HRID: &str = "TemplateRevisionHRID";

/// Template vault human-readable ID.
///
/// **Wire type:** DynamicString
/// **Used by:** managed components
pub const TEMPLATE_VAULT_HRID: &str = "TemplateVaultHRID";

// ---------------------------------------------------------------------------
// Generic component template
// ---------------------------------------------------------------------------

/// Generic component template GUID.
///
/// **Wire type:** DynamicString
/// **Used by:** managed components
pub const GENERIC_COMPONENT_TEMPLATE_GUID: &str = "GenericComponentTemplateGUID";

// ---------------------------------------------------------------------------
// Library linking
// ---------------------------------------------------------------------------

/// Model is linked to the component's library.
///
/// **Wire type:** bool
/// **Used by:** Implementation (RECORD=45)
pub const USE_COMPONENT_LIBRARY: &str = "UseComponentLibrary";

/// Revision string.
///
/// **Wire type:** DynamicString
/// **Used by:** library objects, title block
pub const REVISION: &str = "Revision";

/// Version string.
///
/// **Wire type:** DynamicString
/// **Used by:** various objects
pub const VERSION: &str = "Version";
