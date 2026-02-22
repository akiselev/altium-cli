// Core record structure parameters used by all record types.
//
// Every serialized object begins with a subset of these base fields. They
// control record type dispatch, ownership hierarchy, and object metadata.

/// Record type identifier dispatching object deserialization.
///
/// **Wire type:** u8 (instruction byte, not a key-value parameter)
/// **Used by:** all records
///
/// When the value is 254, a full `RECORDEX` i32 follows for extended types.
pub const RECORD: &str = "RECORD";

/// Extended record type used when `RECORD` == 254.
///
/// **Wire type:** i32
/// **Used by:** records with type >= 254
pub const RECORD_EX: &str = "RECORDEX";

/// Index of parent object in the flat record array.
///
/// **Wire type:** i32
/// **Used by:** all DataObjects
///
/// Record 0 is always the sheet/library root. When `OwnerIndexAdditionalList`
/// is true, this index refers to the `Additional` sidecar stream instead of
/// the main `Data` stream.
pub const OWNER_INDEX: &str = "OwnerIndex";

/// When true, `OwnerIndex` refers to the `Additional` sidecar stream.
///
/// **Wire type:** bool
/// **Used by:** all DataObjects
///
/// Also encoded as bit 7 (0x80) of `PinConglomerate` for Pin records.
pub const OWNER_INDEX_ADDITIONAL_LIST: &str = "OwnerIndexAdditionalList";

/// Which part (1..N) of a multi-part component this object belongs to.
///
/// **Wire type:** i16
/// **Used by:** all GraphicalObjects (children of components)
///
/// `-1` means the object belongs to all parts.
pub const OWNER_PART_ID: &str = "OwnerPartId";

/// Which display mode (0..N-1) this object belongs to.
///
/// **Wire type:** u8
/// **Used by:** all GraphicalObjects (children of components)
pub const OWNER_PART_DISPLAY_MODE: &str = "OwnerPartDisplayMode";

/// Sequential index of this object within the sheet.
///
/// **Wire type:** i32
/// **Used by:** all DataObjects
///
/// Default on import: `-1`. Used for ownership reconstruction.
pub const INDEX_IN_SHEET: &str = "IndexInSheet";

/// When true, skip this object during load.
///
/// **Wire type:** bool
/// **Used by:** all DataObjects
///
/// Only written when true.
pub const IGNORE_ON_LOAD: &str = "IgnoreOnLoad";

/// Object is not accessible/selectable.
///
/// **Wire type:** bool
/// **Used by:** all DataObjects
///
/// **Gotcha:** stored **inverted** -- `true` in file means NOT accessible.
/// Note the intentional typo in the key name (single 's' in "Accesible").
pub const IS_NOT_ACCESSIBLE: &str = "IsNotAccesible";

/// 8-bit bitmask for selection memory group membership.
///
/// **Wire type:** u8
/// **Used by:** all GraphicalObjects
pub const SELECTION_MEMORY: &str = "SelectionMemory";

/// Union group index.
///
/// **Wire type:** i32
/// **Used by:** all GraphicalObjects
pub const UNION_INDEX: &str = "UnionIndex";

/// Object is graphically locked (cannot be moved).
///
/// **Wire type:** bool
/// **Used by:** all GraphicalObjects
///
/// **Gotcha:** exported but **always reset to false on import** (deprecated).
pub const GRAPHICALLY_LOCKED: &str = "GraphicallyLocked";

/// Document-wide unique identifier for this object.
///
/// **Wire type:** string
/// **Used by:** all DataObjects
pub const UNIQUE_ID: &str = "UniqueID";

/// Object's unique ID within a reuse block context.
///
/// **Wire type:** string
/// **Used by:** objects inside reuse blocks
pub const UNIQUE_ID_IN_REUSE_BLOCK: &str = "UniqueIDInReuseBlock";

// ---------------------------------------------------------------------------
// Special instruction / header parameters
// ---------------------------------------------------------------------------

/// File/stream header identification string.
///
/// **Wire type:** string
/// **Used by:** first parameter in almost every stream
///
/// Contains format identification string (e.g., the file header string).
pub const HEADER: &str = "HEADER";

/// Binary instruction byte (0xD0 / 208).
///
/// **Wire type:** instruction byte (not a key-value parameter)
/// **Used by:** Storage stream, pin sidecar streams
///
/// Switches the serializer to binary mode for embedded blob data.
pub const BINARY: &str = "BINARY";

/// Record/object count in a stream.
///
/// **Wire type:** i32
/// **Used by:** `FileHeader` stream, `Storage` stream, various headers
pub const WEIGHT: &str = "Weight";

/// Data payload marker.
///
/// **Wire type:** string
/// **Used by:** various internal streams
pub const DATA: &str = "Data";

// ---------------------------------------------------------------------------
// Vertex coordinates
// ---------------------------------------------------------------------------

/// X coordinate for indexed vertices (as `X1`, `X2`, ...).
///
/// **Wire type:** coord (i32)
/// **Used by:** Polygon (RECORD=7), Wire (RECORD=27), Polyline (RECORD=6)
pub const X: &str = "X";

/// Y coordinate for indexed vertices (as `Y1`, `Y2`, ...).
///
/// **Wire type:** coord (i32)
/// **Used by:** Polygon (RECORD=7), Wire (RECORD=27), Polyline (RECORD=6)
pub const Y: &str = "Y";

/// Extended X coordinate for overflow vertices (as `EX1`, `EX2`, ...).
///
/// **Wire type:** coord (i32)
/// **Used by:** Polyline (RECORD=6) when vertex count > 50
pub const EX: &str = "EX";

/// Extended Y coordinate for overflow vertices (as `EY1`, `EY2`, ...).
///
/// **Wire type:** coord (i32)
/// **Used by:** Polyline (RECORD=6) when vertex count > 50
pub const EY: &str = "EY";

// ---------------------------------------------------------------------------
// V4 ASCII section markers
// ---------------------------------------------------------------------------

/// End-of-section marker.
///
/// **Wire type:** string
/// **Used by:** V4 ASCII format section terminators
pub const END: &str = "End";

/// End-of-component section marker.
///
/// **Wire type:** string
/// **Used by:** V4 ASCII format
pub const END_COMPONENT: &str = "EndComponent";

/// End-of-font section marker.
///
/// **Wire type:** string
/// **Used by:** V4 ASCII format
pub const END_FONT: &str = "EndFont";

/// End-of-future section marker.
///
/// **Wire type:** string
/// **Used by:** V4 ASCII format
pub const END_FUTURE: &str = "EndFuture";

/// End-of-instruction section marker.
///
/// **Wire type:** string
/// **Used by:** V4 ASCII format
pub const END_INSTRUCTION: &str = "EndInstruction";

/// End-of-library section marker.
///
/// **Wire type:** string
/// **Used by:** V4 ASCII format
pub const END_LIBRARY: &str = "EndLibrary";

/// End-of-sheet section marker.
///
/// **Wire type:** string
/// **Used by:** V4 ASCII format
pub const END_SHEET: &str = "EndSheet";

/// Future section marker.
///
/// **Wire type:** string
/// **Used by:** V4 ASCII format
pub const FUTURE: &str = "Future";

// ---------------------------------------------------------------------------
// Miscellaneous structural / internal parameters
// ---------------------------------------------------------------------------

/// Error/action code.
///
/// **Wire type:** i32
/// **Used by:** various internal objects
pub const CODE: &str = "Code";

/// Process name string.
///
/// **Wire type:** string
/// **Used by:** TaskHolder (RECORD=40)
pub const PROCESS: &str = "Process";

/// Cache entry count.
///
/// **Wire type:** i32
/// **Used by:** SchLib internal structures
pub const CACHE_COUNT: &str = "CacheCount";

/// Always show cross-document reference.
///
/// **Wire type:** bool
/// **Used by:** Component (RECORD=1)
pub const ALWAYS_SHOW_CD: &str = "AlwaysShowCD";

/// Assigned interface name (for ESD/harness integration).
///
/// **Wire type:** DynamicString
/// **Used by:** Port (RECORD=18), ESD objects
pub const ASSIGNED_INTERFACE: &str = "AssignedInterface";

/// Assigned interface signal name.
///
/// **Wire type:** DynamicString
/// **Used by:** Port (RECORD=18), ESD objects
pub const ASSIGNED_INTERFACE_SIGNAL: &str = "AssignedInterfaceSignal";

/// MD5 or similar hash of embedded file content.
///
/// **Wire type:** DynamicString
/// **Used by:** FileObject in `Files` stream
pub const FILE_HASH: &str = "FileHash";

/// Byte offset to component data in file.
///
/// **Wire type:** i32
/// **Used by:** V4 binary library only
pub const FILE_POSITION: &str = "FilePosition";

/// General field name for parametric objects.
///
/// **Wire type:** string
/// **Used by:** ParameterSet (RECORD=43)
pub const GENERAL_FIELD: &str = "GeneralField";

/// Parameter value is an image reference, not text.
///
/// **Wire type:** bool
/// **Used by:** Parameter (RECORD=41)
pub const IS_IMAGE_PARAMETER: &str = "IsImageParameter";

/// Key count in library `SectionKeys` stream.
///
/// **Wire type:** i32
/// **Used by:** SchLib `SectionKeys` stream
pub const KEY_COUNT: &str = "KeyCount";

/// Parameter type discriminator.
///
/// **Wire type:** u8
/// **Used by:** Parameter (RECORD=41)
pub const PARAM_TYPE: &str = "ParamType";

/// Reserved field 1 (unused, preserved for compatibility).
///
/// **Wire type:** string
/// **Used by:** various objects
pub const RESERVED_1: &str = "Reserved1";

/// Reserved field 2 (unused, preserved for compatibility).
///
/// **Wire type:** string
/// **Used by:** various objects
pub const RESERVED_2: &str = "Reserved2";

/// Reserved field 3 (unused, preserved for compatibility).
///
/// **Wire type:** string
/// **Used by:** various objects
pub const RESERVED_3: &str = "Reserved3";

/// Section key in library `SectionKeys` stream.
///
/// **Wire type:** string
/// **Used by:** SchLib `SectionKeys` and `Redirection` streams
pub const SECTION_KEY: &str = "SectionKey";

/// Section name for alias redirection.
///
/// **Wire type:** string
/// **Used by:** SchLib `Redirection` stream
pub const SECTION_NAME: &str = "SectionName";

/// Table size for font or data tables.
///
/// **Wire type:** i32
/// **Used by:** various table structures
pub const TABLE_SIZE: &str = "TableSize";

/// URL link.
///
/// **Wire type:** DynamicString
/// **Used by:** Hyperlink (RECORD=226)
pub const URL: &str = "URL";

/// Distance from top edge.
///
/// **Wire type:** coord (i32)
/// **Used by:** various positioned objects
pub const DISTANCE_FROM_TOP: &str = "DistanceFromTop";

/// Collapsed state for tree/hierarchy view.
///
/// **Wire type:** bool
/// **Used by:** SheetSymbol (RECORD=15), ImplementationsList (RECORD=44)
pub const COLLAPSED: &str = "Collapsed";

/// Column index.
///
/// **Wire type:** i32
/// **Used by:** various grid/table objects
pub const COLUMN: &str = "Column";
