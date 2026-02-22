// Reuse block parameters and power object name mappings.
//
// These constants define the parameter keys for schematic reuse blocks
// (RECORD=136, 137, 138), snippet vault references, server parameters,
// and name/document mappings.

// ---------------------------------------------------------------------------
// Block identification
// ---------------------------------------------------------------------------

/// GUID of reuse block definition.
///
/// **Wire type:** DynamicString
/// **Used by:** ReuseBlockImplementationInfo (RECORD=138)
pub const REUSE_BLOCK_ID: &str = "ReuseBlockId";

/// Pipe-delimited UniqueIDs of member objects.
///
/// **Wire type:** DynamicString
/// **Used by:** SchematicBlock (RECORD=136), ReuseSheetSymbol (RECORD=137)
pub const REUSE_BLOCK_OBJECTS_IDS: &str = "ReuseBlockObjectsIds";

/// Block has been dissolved (de-linked from source).
///
/// **Wire type:** bool
/// **Used by:** ReuseBlockImplementationInfo (RECORD=138)
pub const IS_DISSOLVED: &str = "IsDissolved";

// ---------------------------------------------------------------------------
// Block vault references
// ---------------------------------------------------------------------------

/// Block server name.
///
/// **Wire type:** DynamicString
/// **Used by:** SchematicBlock (RECORD=136)
pub const BLOCK_SERVER_NAME: &str = "BlockServerName";

/// Block vault GUID.
///
/// **Wire type:** DynamicString
/// **Used by:** SchematicBlock (RECORD=136)
pub const BLOCK_VAULT_GUID: &str = "BlockVaultGUID";

/// Block item GUID.
///
/// **Wire type:** DynamicString
/// **Used by:** SchematicBlock (RECORD=136)
pub const BLOCK_ITEM_GUID: &str = "BlockItemGUID";

/// Block item revision GUID.
///
/// **Wire type:** DynamicString
/// **Used by:** SchematicBlock (RECORD=136)
pub const BLOCK_ITEM_REVISION_GUID: &str = "BlockItemRevisionGUID";

// ---------------------------------------------------------------------------
// Schematic snippet vault references
// ---------------------------------------------------------------------------

/// Schematic snippet vault GUID.
///
/// **Wire type:** DynamicString
/// **Used by:** `ReuseBlocks` stream, reuse block records
pub const SCH_SNIPPET_VAULT_GUID: &str = "SchSnippetVaultGUID";

/// Schematic snippet item GUID.
///
/// **Wire type:** DynamicString
/// **Used by:** `ReuseBlocks` stream, reuse block records
pub const SCH_SNIPPET_ITEM_GUID: &str = "SchSnippetItemGUID";

/// Schematic snippet item revision GUID.
///
/// **Wire type:** DynamicString
/// **Used by:** `ReuseBlocks` stream, reuse block records
pub const SCH_SNIPPET_ITEM_REVISION_GUID: &str = "SchSnippetItemRevisionGUID";

// ---------------------------------------------------------------------------
// PCB snippet vault references (V2 addition)
// ---------------------------------------------------------------------------

/// PCB snippet vault GUID.
///
/// **Wire type:** DynamicString
/// **Used by:** `ReuseBlocksV2` stream, reuse block records
pub const PCB_SNIPPET_VAULT_GUID: &str = "PcbSnippetVaultGUID";

/// PCB snippet item GUID.
///
/// **Wire type:** DynamicString
/// **Used by:** `ReuseBlocksV2` stream, reuse block records
pub const PCB_SNIPPET_ITEM_GUID: &str = "PcbSnippetItemGUID";

/// PCB snippet item revision GUID.
///
/// **Wire type:** DynamicString
/// **Used by:** `ReuseBlocksV2` stream, reuse block records
pub const PCB_SNIPPET_ITEM_REVISION_GUID: &str = "PcbSnippetItemRevisionGUID";

// ---------------------------------------------------------------------------
// Server parameters
// ---------------------------------------------------------------------------

/// Count of workspace server parameter names.
///
/// **Wire type:** i32
/// **Used by:** SchematicBlock (RECORD=136)
///
/// Paired with indexed `RBServerParametersName{N}` keys.
pub const RB_SERVER_PARAMETERS_COUNT: &str = "RBServerParametersCount";

/// Server parameter name (indexed as `RBServerParametersName0`, ...).
///
/// **Wire type:** DynamicString
/// **Used by:** SchematicBlock (RECORD=136)
pub const RB_SERVER_PARAMETERS_NAME: &str = "RBServerParametersName";

// ---------------------------------------------------------------------------
// Power object name mappings
// ---------------------------------------------------------------------------

/// Count of power net name remappings.
///
/// **Wire type:** i32
/// **Used by:** SchematicBlock (RECORD=136), ReuseSheetSymbol (RECORD=137)
pub const POWER_OBJECTS_NAME_MAPPINGS_COUNT: &str = "PowerObjectsNameMappingsCount";

/// Original power net name (indexed).
///
/// **Wire type:** DynamicString
/// **Used by:** SchematicBlock (RECORD=136), ReuseSheetSymbol (RECORD=137)
pub const POWER_OBJECTS_NAME_ORIGINAL: &str = "PowerObjectsNameOriginal";

/// Instance-specific mapped power net name (indexed).
///
/// **Wire type:** DynamicString
/// **Used by:** SchematicBlock (RECORD=136), ReuseSheetSymbol (RECORD=137)
pub const POWER_OBJECTS_NAME_MAPPED: &str = "PowerObjectsNameMapped";

// ---------------------------------------------------------------------------
// Document file name mappings
// ---------------------------------------------------------------------------

/// Count of document file name remappings.
///
/// **Wire type:** i32
/// **Used by:** SchematicBlock (RECORD=136), ReuseSheetSymbol (RECORD=137)
pub const DOCS_FILE_NAMES_MAPPINGS_COUNT: &str = "DocsFileNamesMappingsCount";

/// Original document file name (indexed).
///
/// **Wire type:** DynamicString
/// **Used by:** SchematicBlock (RECORD=136), ReuseSheetSymbol (RECORD=137)
pub const DOC_FILE_NAME_ORIGINAL: &str = "DocFileNameOriginal";

/// Mapped document file name (indexed).
///
/// **Wire type:** DynamicString
/// **Used by:** SchematicBlock (RECORD=136), ReuseSheetSymbol (RECORD=137)
pub const DOC_FILE_NAME_MAPPED: &str = "DocFileNameMapped";

// ---------------------------------------------------------------------------
// Dissolved block parameters
// ---------------------------------------------------------------------------

/// Count of parameters captured at dissolution time.
///
/// **Wire type:** i32
/// **Used by:** ReuseBlockImplementationInfo (RECORD=138) when dissolved
///
/// Paired with indexed `ParameterName{N}` and `ParameterValue{N}` keys.
pub const PARAMETERS_COUNT: &str = "ParametersCount";

/// Parameter name (indexed as `ParameterName0`, ...).
///
/// **Wire type:** DynamicString
/// **Used by:** ReuseBlockImplementationInfo (RECORD=138) when dissolved
pub const PARAMETER_NAME: &str = "ParameterName";

/// Parameter value (indexed as `ParameterValue0`, ...).
///
/// **Wire type:** DynamicString
/// **Used by:** ReuseBlockImplementationInfo (RECORD=138) when dissolved
pub const PARAMETER_VALUE: &str = "ParameterValue";

// ---------------------------------------------------------------------------
// Flags
// ---------------------------------------------------------------------------

/// Pin belongs to a schematic block (suppresses DRC).
///
/// **Wire type:** bool
/// **Used by:** Pin (RECORD=2), SchematicBlock (RECORD=136)
pub const IS_SCHEMATIC_BLOCK_OBJECT: &str = "IsSchematicBlockObject";
