// OLE stream names, embedded data names, and data version constants.
//
// These are the named streams within OLE2 compound document files and the
// logical payload object names set via `SetName()`.

// ---------------------------------------------------------------------------
// Core streams (all V5 file types)
// ---------------------------------------------------------------------------

/// File header stream containing `HEADER`, `Weight`, `MinorVersion`, `UniqueID`.
///
/// In SchLib this also contains the full component index (`CompCount`,
/// `LibRef{N}`, `CompDescr{N}`, `PartCount{N}`, `AliasCount{N}`, aliases).
///
/// **Used by:** all V5 binary file types
pub const FILE_HEADER: &str = "FileHeader";

/// Main record data stream containing all schematic objects.
///
/// **Used by:** SchDoc (per-component section in SchLib)
pub const DATA: &str = "Data";

/// Overflow objects whose `OwnerIndexAdditionalList=true`.
///
/// When an object's `OwnerIndexAdditionalList` flag is set, its `OwnerIndex`
/// refers to this stream instead of the main `Data` stream.
///
/// **Used by:** SchDoc, SchLib (per-component section)
pub const ADDITIONAL: &str = "Additional";

/// Embedded image data for `ISchDataImage` objects with `EmbedImage=true`.
///
/// Header: `HEADER="Icon storage"`, `Weight=count`. Each entry uses BINARY
/// instruction (byte 208) with a named blob.
///
/// **Used by:** all file types with embedded images
pub const STORAGE: &str = "Storage";

/// Embedded compressed files (physical model images), keyed by GUID + hash.
///
/// Uses binary instruction byte 227.
///
/// **Used by:** Harness Layout Drawing
pub const FILES: &str = "Files";

// ---------------------------------------------------------------------------
// SchDoc-only streams
// ---------------------------------------------------------------------------

/// V1 reuse block info: version(i32), count(i32), then per-block data.
///
/// **Format:** raw little-endian binary blob
/// **Used by:** SchDoc
pub const REUSE_BLOCKS: &str = "ReuseBlocks";

/// V2 reuse block extension: adds PCB snippet vault/item/revision GUIDs.
///
/// **Format:** raw little-endian binary blob
/// **Used by:** SchDoc
pub const REUSE_BLOCKS_V2: &str = "ReuseBlocksV2";

/// Object definition records (RECORD=129) referenced by `ObjectDefinitionId`.
///
/// **Format:** parametric binary
/// **Used by:** SchDoc
pub const OBJECT_DEFINITIONS: &str = "ObjectDefinitions";

/// Dissolved reuse block tracking records (RECORD=138).
///
/// **Format:** parametric binary
/// **Used by:** SchDoc
pub const REUSE_BLOCK_INFOS: &str = "ReuseBlockInfos";

// ---------------------------------------------------------------------------
// SchLib-only streams
// ---------------------------------------------------------------------------

/// Mapping of `LibRef{N}` names to OLE section keys.
///
/// Handles name truncation to the 31-character OLE stream name limit.
///
/// **Location:** root of SchLib
/// **Format:** parametric
pub const SECTION_KEYS: &str = "SectionKeys";

/// Top-level container wrapping per-component `Additional` sub-streams.
///
/// **Location:** root of SchLib
/// **Format:** parametric binary
pub const LIB_ADDITIONAL: &str = "LibAdditional";

/// Alias redirect containing `SectionName` pointing to canonical component name.
///
/// **Location:** per-alias section of SchLib
/// **Format:** parametric
pub const REDIRECTION: &str = "Redirection";

// ---------------------------------------------------------------------------
// Pin sidecar streams (per-component section in SchLib)
// ---------------------------------------------------------------------------

/// Sub-unit fractional coordinate corrections for pins.
///
/// **Blob format:** `i32 locationX_frac, i32 locationY_frac, i32 pinLength_frac`
/// **Used by:** SchLib (per-component section)
pub const PIN_FRAC: &str = "PinFrac";

/// Pin description overflow (characters beyond 254).
///
/// **Blob format:** `i32 byte_length, ASCII bytes`
/// **Used by:** SchLib (per-component section)
pub const PIN_DESC: &str = "PinDesc";

/// Misc pin data in parametric format.
///
/// **Blob format:** `i32 byte_length, UTF-16LE parametric string`
/// **Content:** `PairSwapID=...`
/// **Used by:** SchLib (per-component section)
pub const PIN_MISC_DATA: &str = "PinMiscData";

/// Custom pin text position/font mode for name and designator labels.
///
/// **Blob format:** compact binary structure (see PinTextData documentation)
/// **Used by:** SchLib (per-component section)
pub const PIN_TEXT_DATA: &str = "PinTextData";

/// Unicode pin fields.
///
/// **Blob format:** `i32 byte_length, UTF-16LE parametric string`
/// **Content:** `Desc=...|Name=...|Desig=...|SwapId=...|SwapIDPart=...|DefValue=...`
/// **Used by:** SchLib (per-component section)
pub const PIN_WIDE_TEXT: &str = "PinWideText";

/// Pin symbol line width override.
///
/// **Blob format:** `i32 byte_length, UTF-16LE parametric string`
/// **Content:** `SymBol_LineWidth=N`
/// **Used by:** SchLib (per-component section)
pub const PIN_SYMBOL_LINE_WIDTH: &str = "PinSymbolLineWidth";

/// Physical package pin length.
///
/// **Blob format:** `i32 byte_length, UTF-16LE parametric string`
/// **Content:** `PinPackageLength=N`
/// **Used by:** SchLib (per-component section)
pub const PIN_PACKAGE_LENGTH: &str = "PinPackageLength";

/// Signal propagation delay in scientific notation.
///
/// **Blob format:** `i32 byte_length, UTF-16LE parametric string`
/// **Content:** `PinPropagationDelay=Xe-Y`
/// **Used by:** SchLib (per-component section)
pub const PIN_PROPAGATION_DELAY: &str = "PinPropagationDelay";

/// Selected/defined pin functions.
///
/// **Blob format:** `i32 byte_length, UTF-16LE parametric string`
/// **Content:** `PinSelectedFunctionsCount=N|PinSelectedFunction1=...|...`
/// **Used by:** SchLib (per-component section)
pub const PIN_FUNCTION_DATA: &str = "PinFunctionData";

// ---------------------------------------------------------------------------
// Harness-only streams
// ---------------------------------------------------------------------------

/// Connector/pin assignments for connection points.
///
/// **Format:** raw LE binary blob. Version(i32=1), count(i32), then per-point
/// data: UniqueId, connector count, per-connector: id + pin IDs.
///
/// **Used by:** Harness Layout Drawing
pub const HARNESS_CONNECTION_POINT_CONNECTOR: &str = "HarnessConnectionPointConnector";

/// Reserved/legacy harness component crimps stream.
///
/// Declared in .NET but no reader found (may be Delphi-side only).
///
/// **Used by:** Harness files
pub const HARNESS_COMPONENT_CRIMPS: &str = "HarnessComponentCrimps";

/// Reserved/legacy harness associated parts stream.
///
/// Declared in .NET but no reader found (may be Delphi-side only).
///
/// **Used by:** Harness files
pub const HARNESS_ASSOCIATED_PARTS: &str = "HarnessAssociatedParts";

// ---------------------------------------------------------------------------
// Embedded data name constants
//
// These name the logical payload objects inside streams (set via `SetName()`).
// They happen to have the same string values as the stream names but serve a
// different structural role.
// ---------------------------------------------------------------------------

/// Embedded data name for reuse blocks (used for both V1 and V2).
pub const EMBEDDED_DATA_REUSE_BLOCKS: &str = "ReuseBlocks";

/// Embedded data name for harness connection point connector.
pub const EMBEDDED_DATA_HARNESS_CONNECTION_POINT_CONNECTOR: &str =
    "HarnessConnectionPointConnector";

/// Embedded data name for harness component crimps.
pub const EMBEDDED_DATA_HARNESS_COMPONENT_CRIMPS: &str = "HarnessComponentCrimps";

/// Embedded data name for harness associated parts.
pub const EMBEDDED_DATA_HARNESS_ASSOCIATED_PARTS: &str = "HarnessAssociatedParts";

// ---------------------------------------------------------------------------
// Data version constants
//
// Written as the first 4 bytes (LE i32) of each binary-blob stream payload.
// Checked on import; blobs with version > max are rejected.
// ---------------------------------------------------------------------------

/// Reuse blocks data version. Rejects > 2.
///
/// Version 1 uses 4-byte length prefix strings; version 2 uses .NET
/// `BinaryWriter.Write(string)`.
///
/// **Used by:** `ReuseBlocks`, `ReuseBlocksV2` streams
pub const DATA_VERSION_REUSE_BLOCKS: i32 = 2;

/// Harness connection point connector data version. Rejects > 1.
///
/// **Used by:** `HarnessConnectionPointConnector` stream
pub const DATA_VERSION_HARNESS_CONNECTION_POINT_CONNECTOR: i32 = 1;

/// Harness component crimps data version. Rejects > 1.
///
/// Declared but no .NET reader found (may be Delphi-side only).
///
/// **Used by:** `HarnessComponentCrimps` stream
pub const DATA_VERSION_HARNESS_COMPONENT_CRIMPS: i32 = 1;

/// Harness associated parts data version. Rejects > 1.
///
/// Declared but no .NET reader found (may be Delphi-side only).
///
/// **Used by:** `HarnessAssociatedParts` stream
pub const DATA_VERSION_HARNESS_ASSOCIATED_PARTS: i32 = 1;
