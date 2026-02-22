// Text rendering parameters: content, font, layout, formatting, RTF.
//
// These constants are used by Label, NetLabel, Port, Parameter, TextFrame,
// Note, and other text-bearing record types.

// ---------------------------------------------------------------------------
// Content
// ---------------------------------------------------------------------------

/// Text content string.
///
/// **Wire type:** DynamicString (simple objects) or 16-bit length + ASCII+NUL
/// (TextFrame, Note)
/// **Used by:** Label (RECORD=4), NetLabel (RECORD=25), Port (RECORD=18),
/// Parameter (RECORD=41), TextFrame (RECORD=28), Note (RECORD=209)
pub const TEXT: &str = "Text";

/// Text field reference (for parametric/template fields).
///
/// **Wire type:** DynamicString
/// **Used by:** Parameter (RECORD=41)
pub const TEXT_FIELD: &str = "TextField";

/// Comment string (alternate text content).
///
/// **Wire type:** string
/// **Used by:** Component (RECORD=1)
pub const COMMENT: &str = "Comment";

// ---------------------------------------------------------------------------
// Font
// ---------------------------------------------------------------------------

/// Font table index for text-bearing entries (1-based).
///
/// **Wire type:** i16
/// **Used by:** SheetEntry (RECORD=16), BusEntry (RECORD=37)
pub const TEXT_FONT_ID: &str = "TextFontID";

/// Text color.
///
/// **Wire type:** u32 (BGR COLORREF)
/// **Used by:** Port (RECORD=18), TextFrame (RECORD=28), Note (RECORD=209),
/// FunctionalBlock (RECORD=133)
pub const TEXT_COLOR: &str = "TextColor";

/// Text style string.
///
/// **Wire type:** DynamicString
/// **Used by:** SheetEntry (RECORD=16), BusEntry (RECORD=37)
///
/// Values: `"Full"` or `"Prefix"` (TBusTextStyle).
pub const TEXT_STYLE: &str = "TextStyle";

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

/// Text justification (TTextJustification enum).
///
/// **Wire type:** u8
/// **Used by:** Label (RECORD=4), NetLabel (RECORD=25)
///
/// Values: 0=BottomLeft, 1=BottomCenter, 2=BottomRight, 3=CenterLeft,
/// 4=Center, 5=CenterRight, 6=TopLeft, 7=TopCenter, 8=TopRight.
pub const JUSTIFICATION: &str = "Justification";

/// Text alignment.
///
/// **Wire type:** u8
/// **Used by:** TextFrame (RECORD=28), Note (RECORD=209)
pub const ALIGNMENT: &str = "Alignment";

/// Horizontal anchor for text (TTextHorzAnchor enum).
///
/// **Wire type:** u8
/// **Used by:** SheetFileName (RECORD=33), SheetName (RECORD=32),
/// Parameter (RECORD=41)
///
/// Values: 0=None, 1=Both, 2=Left, 3=Right.
pub const TEXT_HORZ_ANCHOR: &str = "TextHorzAnchor";

/// Vertical anchor for text (TTextVertAnchor enum).
///
/// **Wire type:** u8
/// **Used by:** SheetFileName (RECORD=33), SheetName (RECORD=32),
/// Parameter (RECORD=41)
///
/// Values: 0=None, 1=Both, 2=Top, 3=Bottom.
pub const TEXT_VERT_ANCHOR: &str = "TextVertAnchor";

// ---------------------------------------------------------------------------
// Formatting
// ---------------------------------------------------------------------------

/// Bold text.
///
/// **Wire type:** bool (indexed as `Bold{N}` in font table)
/// **Used by:** font table entries in Sheet (RECORD=31)
pub const BOLD: &str = "Bold";

/// Italic text.
///
/// **Wire type:** bool (indexed as `Italic{N}` in font table)
/// **Used by:** font table entries in Sheet (RECORD=31)
pub const ITALIC: &str = "Italic";

/// Underline text.
///
/// **Wire type:** bool (indexed as `Underline{N}` in font table)
/// **Used by:** font table entries in Sheet (RECORD=31)
pub const UNDERLINE: &str = "Underline";

/// Strikethrough text.
///
/// **Wire type:** bool (indexed as `StrikeOut{N}` in font table)
/// **Used by:** font table entries in Sheet (RECORD=31)
pub const STRIKE_OUT: &str = "StrikeOut";

/// Color of underline decoration.
///
/// **Wire type:** u32 (BGR COLORREF)
/// **Used by:** various text-bearing objects
pub const UNDERLINE_COLOR: &str = "UnderlineColor";

// ---------------------------------------------------------------------------
// Behavior
// ---------------------------------------------------------------------------

/// Enable word wrapping.
///
/// **Wire type:** bool
/// **Used by:** TextFrame (RECORD=28), Note (RECORD=209)
///
/// Default: `true`.
pub const WORD_WRAP: &str = "WordWrap";

/// Clip text to bounding rectangle.
///
/// **Wire type:** bool
/// **Used by:** TextFrame (RECORD=28), Note (RECORD=209)
///
/// Default: `true`.
pub const CLIP_TO_RECT: &str = "ClipToRect";

/// Auto-size to fit text content.
///
/// **Wire type:** bool
/// **Used by:** Port (RECORD=18)
pub const AUTO_SIZE: &str = "AutoSize";

/// Margin between text and border.
///
/// **Wire type:** coord (i32)
/// **Used by:** TextFrame (RECORD=28, default=5), Note (RECORD=209,
/// default=500,000)
pub const TEXT_MARGIN: &str = "TextMargin";

/// Only show the first line of text.
///
/// **Wire type:** bool
/// **Used by:** HarnessLayoutLabel (RECORD=109)
pub const SHOW_ONLY_FIRST_LINE: &str = "ShowOnlyFirstLine";

// ---------------------------------------------------------------------------
// RTF
// ---------------------------------------------------------------------------

/// RTF formatted text as raw binary blob.
///
/// **Wire type:** binary blob
/// **Used by:** RichTextDocument (RECORD=240)
pub const RTF_STREAM: &str = "RTFStream";

/// External RTF file path.
///
/// **Wire type:** DynamicString
/// **Used by:** RTFLink (RECORD=241)
pub const FILE_NAME_RTF: &str = "FileNameRTF";

// ---------------------------------------------------------------------------
// Naming
// ---------------------------------------------------------------------------

/// Object name.
///
/// **Wire type:** string
/// **Used by:** Pin (RECORD=2), Parameter (RECORD=41), various objects
pub const NAME: &str = "Name";

/// Name text color.
///
/// **Wire type:** u32 (BGR COLORREF)
/// **Used by:** Pin (RECORD=2) -- from main record, not custom font
pub const NAME_COLOR: &str = "NameColor";

/// Show component name.
///
/// **Wire type:** bool
/// **Used by:** component child objects
pub const SHOW_NAME: &str = "ShowName";

/// Show component designator.
///
/// **Wire type:** bool
/// **Used by:** component child objects
pub const SHOW_DESIGNATOR: &str = "ShowDesignator";

/// Object description string.
///
/// **Wire type:** string
/// **Used by:** various objects
pub const DESCRIPTION: &str = "Description";

/// Short description alias.
///
/// **Wire type:** string
/// **Used by:** Pin (RECORD=2) in PinWideText sidecar, SchLib headers
pub const DESC: &str = "Desc";

/// Short designator alias.
///
/// **Wire type:** string
/// **Used by:** Pin (RECORD=2) in PinWideText sidecar
pub const DESIG: &str = "Desig";

/// Blank line separator.
///
/// **Wire type:** bool
/// **Used by:** V4 ASCII format
pub const BLANK_LINE: &str = "BlankLine";
