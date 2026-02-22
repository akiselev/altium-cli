// Pin parameters (RECORD=2).
//
// Pin records use a packed bitfield (`PinConglomerate`) for orientation and
// flags, plus a rich set of text positioning, symbol, swap, and extended
// parameters. Many pin fields are split between the main record and sidecar
// streams in SchLib files.
//
// ## PinConglomerate Bitfield Layout
//
// A single byte containing multiple flags and the orientation:
//
// ```text
// Bits [1:0]  Orientation               TRotationBy90 (0=0deg, 1=90deg, 2=180deg, 3=270deg)
// Bit  2      IsHidden                  0x04
// Bit  3      ShowName                  0x08
// Bit  4      ShowDesignator            0x10
// Bit  5      NotAccessible             0x20 -- INVERTED: bit set = NOT accessible
// Bit  6      GraphicallyLocked         0x40 -- written but NEVER read back on import
// Bit  7      OwnerIndexAdditionalList  0x80 -- OwnerIndex refers to Additional stream
// ```
//
// ## PinName_PositionConglomerate (ASCII-only, packed byte)
//
// ```text
// Bit  0      NamePositionMode custom   1=custom, 0=default
// Bit  1      NameRotationAnchor        1=component, 0=pin
// Bits [3:2]  NameRotationRelative      TRotationBy90
// Bit  4      NameFontMode custom       1=custom font, 0=default
// ```
//
// If custom position: `Name_CustomPosition_Margin` coord follows.
// If custom font: `Name_CustomFontID` and `Name_CustomColor` follow.
//
// `PinDesignator_PositionConglomerate` has identical structure for the
// designator text.

// ---------------------------------------------------------------------------
// Core pin fields
// ---------------------------------------------------------------------------

/// Pin orientation and flags packed into a single byte.
///
/// **Wire type:** u8
/// **Used by:** Pin (RECORD=2)
///
/// See module-level docs for bitfield layout.
pub const PIN_CONGLOMERATE: &str = "PinConglomerate";

/// Length of pin line in DXP coordinate units.
///
/// **Wire type:** coord (i32)
/// **Used by:** Pin (RECORD=2)
pub const PIN_LENGTH: &str = "PinLength";

/// Pin line color.
///
/// **Wire type:** u32 (BGR COLORREF)
/// **Used by:** Pin (RECORD=2)
pub const PIN_COLOR: &str = "PinColor";

// ---------------------------------------------------------------------------
// Text positioning conglomerates (ASCII-only)
// ---------------------------------------------------------------------------

/// Packed byte for pin name text positioning and font mode.
///
/// **Wire type:** u8
/// **Used by:** Pin (RECORD=2), ASCII format only
///
/// See module-level docs for bitfield layout.
pub const PIN_NAME_POSITION_CONGLOMERATE: &str = "PinName_PositionConglomerate";

/// Packed byte for pin designator text positioning and font mode.
///
/// **Wire type:** u8
/// **Used by:** Pin (RECORD=2), ASCII format only
///
/// Identical bitfield structure to `PinName_PositionConglomerate`.
pub const PIN_DESIGNATOR_POSITION_CONGLOMERATE: &str = "PinDesignator_PositionConglomerate";

// ---------------------------------------------------------------------------
// Pin name custom text fields
// ---------------------------------------------------------------------------

/// Custom font ID for pin name text (1-based index into font table).
///
/// **Wire type:** i16
/// **Used by:** Pin (RECORD=2)
///
/// Only present when NameFontMode is custom (bit 4 of position conglomerate).
pub const NAME_CUSTOM_FONT_ID: &str = "Name_CustomFontID";

/// Custom color for pin name text.
///
/// **Wire type:** u32 (BGR COLORREF)
/// **Used by:** Pin (RECORD=2)
///
/// Only present when NameFontMode is custom.
pub const NAME_CUSTOM_COLOR: &str = "Name_CustomColor";

/// Custom margin for pin name text position.
///
/// **Wire type:** coord (i32)
/// **Used by:** Pin (RECORD=2)
///
/// Only present when NamePositionMode is custom (bit 0 of position conglomerate).
pub const NAME_CUSTOM_POSITION_MARGIN: &str = "Name_CustomPosition_Margin";

// ---------------------------------------------------------------------------
// Pin designator custom text fields
// ---------------------------------------------------------------------------

/// Custom font ID for pin designator text (1-based index into font table).
///
/// **Wire type:** i16
/// **Used by:** Pin (RECORD=2)
///
/// Only present when DesignatorFontMode is custom.
pub const DESIGNATOR_CUSTOM_FONT_ID: &str = "Designator_CustomFontID";

/// Custom color for pin designator text.
///
/// **Wire type:** u32 (BGR COLORREF)
/// **Used by:** Pin (RECORD=2)
///
/// Only present when DesignatorFontMode is custom.
pub const DESIGNATOR_CUSTOM_COLOR: &str = "Designator_CustomColor";

/// Custom margin for pin designator text position.
///
/// **Wire type:** coord (i32)
/// **Used by:** Pin (RECORD=2)
///
/// Only present when DesignatorPositionMode is custom.
pub const DESIGNATOR_CUSTOM_POSITION_MARGIN: &str = "Designator_CustomPosition_Margin";

// ---------------------------------------------------------------------------
// IEEE symbol fields
// ---------------------------------------------------------------------------

/// Pin symbol type (outer symbol at net connection side).
///
/// **Wire type:** u8 (TIeeeSymbol enum, 0..36)
/// **Used by:** Pin (RECORD=2)
pub const SYMBOL: &str = "Symbol";

/// Inner body symbol.
///
/// **Wire type:** u8 (TIeeeSymbol enum)
/// **Used by:** Pin (RECORD=2)
///
/// **Gotcha:** note the mixed casing `SymBol_Inner` with capital B.
pub const SYMBOL_INNER: &str = "SymBol_Inner";

/// Symbol at inner edge (component body side).
///
/// **Wire type:** u8 (TIeeeSymbol enum)
/// **Used by:** Pin (RECORD=2)
pub const SYMBOL_INNER_EDGE: &str = "SymBol_InnerEdge";

/// Outer body symbol.
///
/// **Wire type:** u8 (TIeeeSymbol enum)
/// **Used by:** Pin (RECORD=2)
pub const SYMBOL_OUTER: &str = "SymBol_Outer";

/// Symbol at outer edge (net connection side).
///
/// **Wire type:** u8 (TIeeeSymbol enum)
/// **Used by:** Pin (RECORD=2)
pub const SYMBOL_OUTER_EDGE: &str = "SymBol_OuterEdge";

/// Width of IEEE symbol lines.
///
/// **Wire type:** u8 (TSize enum: 0=Zero, 1=Small, 2=Medium, 3=Large)
/// **Used by:** Pin (RECORD=2), ASCII format only
///
/// In binary SchLib, stored in the `PinSymbolLineWidth` sidecar stream.
pub const SYMBOL_LINE_WIDTH: &str = "SymBol_LineWidth";

// ---------------------------------------------------------------------------
// Swap group fields
// ---------------------------------------------------------------------------

/// Pin swap group within part.
///
/// **Wire type:** string
/// **Used by:** Pin (RECORD=2)
pub const SWAP_ID_PIN: &str = "SwapIdPin";

/// Part swap group ID.
///
/// **Wire type:** string
/// **Used by:** Pin (RECORD=2)
///
/// **Gotcha:** note mixed casing `SwapIDPart` (capital ID).
pub const SWAP_ID_PART: &str = "SwapIDPart";

/// Pair swap ID (ASCII-only field).
///
/// **Wire type:** string
/// **Used by:** Pin (RECORD=2), ASCII format only
///
/// In binary SchLib, stored in the `PinMiscData` sidecar stream.
pub const SWAP_ID_PAIR: &str = "SwapIdPair";

/// Pair swap ID from sidecar stream.
///
/// **Wire type:** string
/// **Used by:** Pin (RECORD=2), `PinMiscData` sidecar stream
pub const PAIR_SWAP_ID: &str = "PairSwapID";

// ---------------------------------------------------------------------------
// Extended pin fields (ASCII-only or sidecar)
// ---------------------------------------------------------------------------

/// Physical package pin length.
///
/// **Wire type:** coord (i32)
/// **Used by:** Pin (RECORD=2)
///
/// In binary SchLib, stored in the `PinPackageLength` sidecar stream.
pub const PIN_PACKAGE_LENGTH: &str = "PinPackageLength";

/// Signal propagation delay in scientific notation (e.g., `1.5e-9`).
///
/// **Wire type:** double (as string)
/// **Used by:** Pin (RECORD=2)
///
/// In binary SchLib, stored in the `PinPropagationDelay` sidecar stream.
pub const PIN_PROPAGATION_DELAY: &str = "PinPropagationDelay";

/// Symbolic name for pin.
///
/// **Wire type:** string
/// **Used by:** Pin (RECORD=2)
pub const PIN_SYMBOLIC_NAME: &str = "PinSymbolicName";

// ---------------------------------------------------------------------------
// Pin function fields
// ---------------------------------------------------------------------------

/// Count of selected alternate functions.
///
/// **Wire type:** i32
/// **Used by:** Pin (RECORD=2)
///
/// Paired with indexed `PinSelectedFunction{N}` keys.
pub const PIN_SELECTED_FUNCTIONS_COUNT: &str = "PinSelectedFunctionsCount";

/// Selected alternate function (indexed as `PinSelectedFunction1`, etc.).
///
/// **Wire type:** string
/// **Used by:** Pin (RECORD=2)
pub const PIN_SELECTED_FUNCTION: &str = "PinSelectedFunction";

/// Count of all defined functions.
///
/// **Wire type:** i32
/// **Used by:** Pin (RECORD=2)
///
/// Paired with indexed `PinDefinedFunction{N}` keys.
pub const PIN_DEFINED_FUNCTIONS_COUNT: &str = "PinDefinedFunctionsCount";

/// Defined function (indexed as `PinDefinedFunction1`, etc.).
///
/// **Wire type:** string
/// **Used by:** Pin (RECORD=2)
pub const PIN_DEFINED_FUNCTION: &str = "PinDefinedFunction";

// ---------------------------------------------------------------------------
// Pin flags
// ---------------------------------------------------------------------------

/// Show pin name as function alias.
///
/// **Wire type:** bool
/// **Used by:** Pin (RECORD=2)
pub const HIDE_PIN_NAME_AS_FUNCTION: &str = "HidePinNameAsFunction";

/// Display symbolic name as function label.
///
/// **Wire type:** bool
/// **Used by:** Pin (RECORD=2)
pub const SHOW_PIN_SYMBOLIC_NAME_AS_FUNCTION: &str = "ShowPinSymbolicNameAsFunction";

/// Pin belongs to a schematic block (suppresses DRC).
///
/// **Wire type:** bool
/// **Used by:** Pin (RECORD=2)
pub const IS_SCHEMATIC_BLOCK_OBJECT: &str = "IsSchematicBlockObject";

// ---------------------------------------------------------------------------
// Default / swap values
// ---------------------------------------------------------------------------

/// Default value for pin.
///
/// **Wire type:** string
/// **Used by:** Pin (RECORD=2)
pub const DEFAULT_VALUE: &str = "DefaultValue";

/// Short alias for default value (used in PinWideText sidecar).
///
/// **Wire type:** string
/// **Used by:** Pin (RECORD=2) in PinWideText sidecar stream
pub const DEF_VALUE: &str = "DefValue";

/// Swap ID (generic, used in PinWideText sidecar).
///
/// **Wire type:** string
/// **Used by:** Pin (RECORD=2) in PinWideText sidecar stream
pub const SWAP_ID: &str = "SwapId";
