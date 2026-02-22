// Component identification and structure parameters (RECORD=1).
//
// These constants define the parameter keys for schematic component records,
// covering identification, multi-part structure, display modes, library
// linking, and vault integration.

/// Component name in the originating library (primary identifier).
///
/// **Wire type:** string
/// **Used by:** Component (RECORD=1)
pub const LIB_REFERENCE: &str = "LibReference";

/// Library reference key used in the library file header `SectionKeys` stream.
///
/// **Wire type:** string (indexed as `LibRef0`, `LibRef1`, ...)
/// **Used by:** SchLib `FileHeader` and `SectionKeys` streams
///
/// **Gotcha:** `LibRef` is used in the library file header for indexed keys,
/// while `LibReference` is stored within each component record itself.
pub const LIB_REF: &str = "LibRef";

/// Human-readable component description.
///
/// **Wire type:** string
/// **Used by:** Component (RECORD=1)
pub const COMPONENT_DESCRIPTION: &str = "ComponentDescription";

/// Component designator (e.g., "U1", "R3").
///
/// **Wire type:** string
/// **Used by:** Component (RECORD=1), Designator (RECORD=34)
pub const DESIGNATOR: &str = "Designator";

/// Number of parts in a multi-part component (gate count).
///
/// **Wire type:** i16
/// **Used by:** Component (RECORD=1)
pub const PART_COUNT: &str = "PartCount";

/// Number of alternate symbol display modes.
///
/// **Wire type:** u8
/// **Used by:** Component (RECORD=1)
pub const DISPLAY_MODE_COUNT: &str = "DisplayModeCount";

/// Currently active display mode (0..N-1).
///
/// **Wire type:** u8
/// **Used by:** Component (RECORD=1)
pub const DISPLAY_MODE: &str = "DisplayMode";

/// Currently active part (1..PartCount).
///
/// **Wire type:** i16
/// **Used by:** Component (RECORD=1)
pub const CURRENT_PART_ID: &str = "CurrentPartId";

/// Component kind value (version 1).
///
/// **Wire type:** u8
/// **Used by:** Component (RECORD=1)
///
/// See `ComponentKindVersion2` and `ComponentKindVersion3` for the
/// versioned override logic: if V3 == 6 -> Jumper; else if V2 >= 5 -> use
/// V2; else use V1.
pub const COMPONENT_KIND: &str = "ComponentKind";

/// Extended component kind (version 2).
///
/// **Wire type:** u8
/// **Used by:** Component (RECORD=1)
///
/// Wins over V1 if value >= 5.
pub const COMPONENT_KIND_VERSION2: &str = "ComponentKindVersion2";

/// Further extended component kind (version 3).
///
/// **Wire type:** u8
/// **Used by:** Component (RECORD=1)
///
/// If value == 6, overrides V2 (Jumper component kind).
pub const COMPONENT_KIND_VERSION3: &str = "ComponentKindVersion3";

/// Horizontal mirror flag.
///
/// **Wire type:** bool
/// **Used by:** Component (RECORD=1), Label, NetLabel, Parameter
pub const IS_MIRRORED: &str = "IsMirrored";

/// Show hidden fields on this component.
///
/// **Wire type:** bool
/// **Used by:** Component (RECORD=1)
pub const SHOW_HIDDEN_FIELDS: &str = "ShowHiddenFields";

/// Pins can be moved independently of the component body.
///
/// **Wire type:** bool
/// **Used by:** Component (RECORD=1)
pub const PINS_MOVEABLE: &str = "PinsMoveable";

/// Total pin count across all parts.
///
/// **Wire type:** i16
/// **Used by:** Component (RECORD=1)
pub const ALL_PIN_COUNT: &str = "AllPinCount";

/// Path to source library file.
///
/// **Wire type:** string
/// **Used by:** Component (RECORD=1)
pub const LIBRARY_PATH: &str = "LibraryPath";

/// Source SchLib name for library sync.
///
/// **Wire type:** DynamicString
/// **Used by:** Component (RECORD=1), SheetSymbol
pub const SOURCE_LIBRARY_NAME: &str = "SourceLibraryName";

/// Name for each display mode (indexed as `CustomDisplayModeName0`, etc.).
///
/// **Wire type:** string
/// **Used by:** Component (RECORD=1)
pub const CUSTOM_DISPLAY_MODE_NAME: &str = "CustomDisplayModeName";

/// Only current part information is stored (optimization flag).
///
/// **Wire type:** bool
/// **Used by:** Component (RECORD=1)
pub const HAS_ONLY_CURRENT_PART_INFO: &str = "HasOnlyCurrentPartInfo";

/// Comma-separated list of component aliases.
///
/// **Wire type:** string
/// **Used by:** Component (RECORD=1)
pub const ALIAS_LIST: &str = "AliasList";

/// Number of aliases for this component.
///
/// **Wire type:** i32
/// **Used by:** SchLib `FileHeader` stream (indexed per component)
pub const ALIAS_COUNT: &str = "AliasCount";

/// Single alias name.
///
/// **Wire type:** string
/// **Used by:** SchLib `FileHeader` stream
pub const ALIAS: &str = "Alias";

/// Unique ID of key component for multi-channel designs.
///
/// **Wire type:** string
/// **Used by:** Component (RECORD=1)
pub const KEY_COMPONENT_UNIQUE_ID: &str = "KeyComponentUniqueId";

/// Designator text is locked (cannot be changed).
///
/// **Wire type:** bool
/// **Used by:** Component (RECORD=1), harness objects
pub const DESIGNATOR_LOCKED: &str = "DesignatorLocked";

/// Part ID is locked.
///
/// **Wire type:** bool
/// **Used by:** Component (RECORD=1)
///
/// Defaults to `DesignatorLocked` value if absent.
pub const PART_ID_LOCKED: &str = "PartIDLocked";

/// Don't use stored library name.
///
/// **Wire type:** bool
/// **Used by:** Component (RECORD=1), ObjectDefinition
///
/// **Gotcha:** stored inverted from "UseLibraryName".
pub const NOT_USE_LIBRARY_NAME: &str = "NotUseLibraryName";

/// Vault/managed item identifier (human-readable).
///
/// **Wire type:** DynamicString
/// **Used by:** Component (RECORD=1), SheetSymbol, ObjectDefinition
pub const DESIGN_ITEM_ID: &str = "DesignItemId";

/// Internal schematic file path in hierarchical design.
///
/// **Wire type:** DynamicString
/// **Used by:** Component (RECORD=1)
pub const SHEET_PART_FILE_NAME: &str = "SheetPartFileName";

/// Component name in library header.
///
/// **Wire type:** string
/// **Used by:** SchLib `FileHeader` stream
pub const COMPONENT: &str = "Component";

/// Total component count in library.
///
/// **Wire type:** i32
/// **Used by:** SchLib `FileHeader` stream
pub const COMPONENT_COUNT: &str = "ComponentCount";

/// Component description in library header (indexed).
///
/// **Wire type:** string
/// **Used by:** SchLib `FileHeader` stream (as `CompDescr0`, `CompDescr1`, ...)
pub const COMP_DESCR: &str = "CompDescr";

/// Component count in library header.
///
/// **Wire type:** i32
/// **Used by:** SchLib `FileHeader` stream
pub const COMP_COUNT: &str = "CompCount";

/// Short alias for component in V4 ASCII format.
///
/// **Wire type:** string
/// **Used by:** V4 ASCII format
pub const COMP: &str = "Comp";

/// Part number within a multi-part component.
///
/// **Wire type:** i16
/// **Used by:** V4 ASCII format
pub const PART: &str = "Part";

/// Part ID (alias).
///
/// **Wire type:** i16
/// **Used by:** various component contexts
pub const PART_ID: &str = "PartId";

/// Part description string.
///
/// **Wire type:** string
/// **Used by:** SchLib component records
pub const PART_DESCRIPTION: &str = "PartDescription";

/// Part field name for parametric fields.
///
/// **Wire type:** string
/// **Used by:** SchLib component records
pub const PART_FIELD_NAME: &str = "PartFieldName";

/// Number of part types.
///
/// **Wire type:** i32
/// **Used by:** SchLib component records
pub const PART_TYPE_COUNT: &str = "PartTypeCount";

/// Part type values.
///
/// **Wire type:** string
/// **Used by:** SchLib component records
pub const PART_TYPES: &str = "PartTypes";

/// Design implementation reference.
///
/// **Wire type:** string
/// **Used by:** ImplementationsList (RECORD=44)
pub const DES_IMP: &str = "DesImp";

/// Design implementation count.
///
/// **Wire type:** i32
/// **Used by:** ImplementationsList (RECORD=44)
pub const DES_IMP_COUNT: &str = "DesImpCount";

/// Design interface reference.
///
/// **Wire type:** string
/// **Used by:** ImplementationsList (RECORD=44)
pub const DES_INTF: &str = "DesIntf";

/// Show field names in display.
///
/// **Wire type:** bool
/// **Used by:** Component (RECORD=1)
pub const DISPLAY_FIELD_NAMES: &str = "DisplayFieldNames";
