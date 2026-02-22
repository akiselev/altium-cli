// Electrical type parameters: pin electrical types, port I/O, power objects,
// net topology, routing, and ERC suppression.

// ---------------------------------------------------------------------------
// Pin electrical
// ---------------------------------------------------------------------------

/// Pin electrical type (TPinElectrical enum).
///
/// **Wire type:** u8
/// **Used by:** Pin (RECORD=2)
///
/// Values: 0=Input, 1=IO, 2=Output, 3=OpenCollector, 4=Passive, 5=HiZ,
/// 6=OpenEmitter, 7=Power.
pub const ELECTRICAL: &str = "Electrical";

/// VHDL formal type (TStdLogicState enum).
///
/// **Wire type:** u8
/// **Used by:** Pin (RECORD=2)
///
/// Values: 0=Uninitialized, 1=ForcingUnknown, 2=Forcing0, 3=Forcing1,
/// 4=HiZ, 5=WeakUnknown, 6=Weak0, 7=Weak1, 8=DontCare.
pub const FORMAL_TYPE: &str = "FormalType";

// ---------------------------------------------------------------------------
// Port / sheet entry
// ---------------------------------------------------------------------------

/// Port I/O type (TPortIO enum).
///
/// **Wire type:** u8
/// **Used by:** Port (RECORD=18), SheetEntry (RECORD=16)
///
/// Values: 0=Unspecified, 1=Output, 2=Input, 3=Bidirectional.
pub const IO_TYPE: &str = "IOType";

/// Port name is hidden.
///
/// **Wire type:** bool
/// **Used by:** Port (RECORD=18)
///
/// **Gotcha:** inverted semantics -- `F` means the net name IS shown.
pub const PORT_NAME_IS_HIDDEN: &str = "PortNameIsHidden";

// ---------------------------------------------------------------------------
// Power object
// ---------------------------------------------------------------------------

/// Power symbol acts as cross-sheet connector.
///
/// **Wire type:** bool
/// **Used by:** PowerObject (RECORD=17)
pub const IS_CROSS_SHEET_CONNECTOR: &str = "IsCrossSheetConnector";

/// Display net name on power symbol.
///
/// **Wire type:** bool
/// **Used by:** PowerObject (RECORD=17)
pub const SHOW_NET_NAME: &str = "ShowNetName";

/// Power symbol type.
///
/// **Wire type:** u8
/// **Used by:** PowerObject (RECORD=17)
pub const SYMBOL_TYPE: &str = "SymbolType";

// ---------------------------------------------------------------------------
// Side
// ---------------------------------------------------------------------------

/// Object side (TLeftRightSide enum).
///
/// **Wire type:** u8
/// **Used by:** SheetEntry (RECORD=16), BusEntry (RECORD=37)
///
/// Values: 0=Left, 1=Right, 2=Top, 3=Bottom.
pub const SIDE: &str = "Side";

// ---------------------------------------------------------------------------
// Net topology
// ---------------------------------------------------------------------------

/// Network topology type.
///
/// **Wire type:** u8
/// **Used by:** V4 binary only (TNetTopology enum)
pub const NET_TOPOLOGY: &str = "NetTopology";

// ---------------------------------------------------------------------------
// Routing
// ---------------------------------------------------------------------------

/// Routing priority.
///
/// **Wire type:** i32
/// **Used by:** various routing-aware objects
pub const ROUTING_PRIORITY: &str = "RoutingPriority";

/// Routing track width.
///
/// **Wire type:** coord (i32)
/// **Used by:** various routing-aware objects
pub const ROUTING_TRACK_WIDTH: &str = "RoutingTrackWidth";

/// Routing via width.
///
/// **Wire type:** coord (i32)
/// **Used by:** various routing-aware objects
pub const ROUTING_VIA_WIDTH: &str = "RoutingViaWidth";

// ---------------------------------------------------------------------------
// ERC suppression
// ---------------------------------------------------------------------------

/// Serialized connection pair suppressions.
///
/// **Wire type:** string
/// **Used by:** NoERC (RECORD=22)
///
/// Only written when `SuppressAll` is false.
pub const CONNECTION_PAIRS_TO_SUPPRESS: &str = "ConnectionPairsToSuppress";

/// Comma-separated error kind names as TErrorKindSet bitmask.
///
/// **Wire type:** string (bitmask)
/// **Used by:** NoERC (RECORD=22)
///
/// Only written when `SuppressAll` is false.
pub const ERROR_KIND_SET_TO_SUPPRESS: &str = "ErrorKindSetToSuppress";

/// Suppress all error kinds at this location.
///
/// **Wire type:** bool
/// **Used by:** NoERC (RECORD=22)
pub const SUPPRESS_ALL: &str = "SuppressAll";

// ---------------------------------------------------------------------------
// Junction
// ---------------------------------------------------------------------------

/// Junction marker present.
///
/// **Wire type:** bool
/// **Used by:** Junction (RECORD=29)
pub const JUNCTION: &str = "Junction";

/// Object is locked (cannot be moved/deleted).
///
/// **Wire type:** bool
/// **Used by:** Junction (RECORD=29)
pub const LOCKED: &str = "Locked";
