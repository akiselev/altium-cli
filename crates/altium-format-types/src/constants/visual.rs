// Shared visual parameters: colors, fonts, line styles, shapes, transforms, geometry.
//
// These constants are used across many record types for controlling the visual
// appearance of schematic objects.

// ---------------------------------------------------------------------------
// Color
// ---------------------------------------------------------------------------

/// Primary object color.
///
/// **Wire type:** u32 (BGR COLORREF, 0x00BBGGRR)
/// **Used by:** most drawing objects, Sheet (RECORD=31)
pub const COLOR: &str = "Color";

/// Secondary color (e.g., stripe on harness wires).
///
/// **Wire type:** u32 (BGR COLORREF)
/// **Used by:** HarnessWire, HarnessWireBreak
///
/// Default: `0xFFFFFFFF` (white/absent).
pub const SECONDARY_COLOR: &str = "SecondaryColor";

/// Tertiary color.
///
/// **Wire type:** u32 (BGR COLORREF)
/// **Used by:** HarnessWire, HarnessWireBreak
pub const TERTIARY_COLOR: &str = "TertiaryColor";

/// Border/outline color.
///
/// **Wire type:** u32 (BGR COLORREF)
/// **Used by:** HarnessWire, HarnessWireBreak
pub const BORDER_COLOR: &str = "BorderColor";

// ---------------------------------------------------------------------------
// Font
// ---------------------------------------------------------------------------

/// Font table index (1-based).
///
/// **Wire type:** i16
/// **Used by:** most text-bearing objects
///
/// Indexes into the per-document font table defined in the Sheet (RECORD=31).
/// On import, a `FontIdTranslator` maps file-local IDs to global runtime IDs.
pub const FONT_ID: &str = "FontID";

/// Font name string (e.g., "Times New Roman").
///
/// **Wire type:** string (indexed as `FontName1`, `FontName2`, ...)
/// **Used by:** Sheet (RECORD=31) font table
pub const FONT_NAME: &str = "FontName";

/// Number of font entries in the font table.
///
/// **Wire type:** i16
/// **Used by:** Sheet (RECORD=31)
pub const FONT_ID_COUNT: &str = "FontIdCount";

/// Font table marker.
///
/// **Wire type:** string
/// **Used by:** Sheet (RECORD=31) font table header
pub const FONT_TABLE: &str = "FontTable";

// ---------------------------------------------------------------------------
// Line style
// ---------------------------------------------------------------------------

/// Line width (TSize enum: 0=Zero, 1=Small, 2=Medium, 3=Large).
///
/// **Wire type:** u8
/// **Used by:** most drawing objects
pub const LINE_WIDTH: &str = "LineWidth";

/// Line style (legacy field, clamped to 0..2).
///
/// **Wire type:** u8
/// **Used by:** most drawing objects
///
/// Values: 0=Solid, 1=Dashed, 2=Dotted.
///
/// **Gotcha:** this is the legacy field clamped to 0..2 (no DashDotted).
/// Use `LineStyleExt` for the full value. On import, take the larger of
/// `LineStyle` and `LineStyleExt`.
pub const LINE_STYLE: &str = "LineStyle";

/// Extended line style (full range, ASCII-only).
///
/// **Wire type:** u8
/// **Used by:** Rectangle, other shapes (ASCII format)
///
/// Values: 0=Solid, 1=Dashed, 2=Dotted, 3=DashDotted.
///
/// **Gotcha:** on import, take max(`LineStyle`, `LineStyleExt`). Rectangles
/// only use `LineStyleExt`.
pub const LINE_STYLE_EXT: &str = "LineStyleExt";

/// Border width.
///
/// **Wire type:** u8
/// **Used by:** various bordered objects
pub const BORDER_WIDTH: &str = "BorderWidth";

// ---------------------------------------------------------------------------
// Shape / angle
// ---------------------------------------------------------------------------

/// Start angle in degrees (0.0..360.0).
///
/// **Wire type:** 6-byte Borland Turbo Pascal Real (NOT IEEE-754)
/// **Used by:** Arc (RECORD=12), EllipticalArc (RECORD=11), Pie (RECORD=9)
pub const START_ANGLE: &str = "StartAngle";

/// End angle in degrees (0.0..360.0).
///
/// **Wire type:** 6-byte Borland Turbo Pascal Real (NOT IEEE-754)
/// **Used by:** Arc (RECORD=12), EllipticalArc (RECORD=11), Pie (RECORD=9)
pub const END_ANGLE: &str = "EndAngle";

/// Start endpoint shape (TLineShape enum).
///
/// **Wire type:** u8
/// **Used by:** Polyline (RECORD=6)
///
/// Values: 0=None, 1=Arrow, 2=SolidArrow, 3=Tail, 4=SolidTail, 5=Circle,
/// 6=Square.
pub const START_LINE_SHAPE: &str = "StartLineShape";

/// End endpoint shape (TLineShape enum).
///
/// **Wire type:** u8
/// **Used by:** Polyline (RECORD=6)
pub const END_LINE_SHAPE: &str = "EndLineShape";

/// Endpoint shape size (TSize enum).
///
/// **Wire type:** u8
/// **Used by:** Polyline (RECORD=6)
pub const LINE_SHAPE_SIZE: &str = "LineShapeSize";

/// Arrow kind string.
///
/// **Wire type:** DynamicString
/// **Used by:** SheetEntry (RECORD=16)
///
/// Values: `"Block & Triangle"`, `"Triangle"`, `"Arrow"`, `"Arrow Tail"`.
pub const ARROW_KIND: &str = "ArrowKind";

// ---------------------------------------------------------------------------
// Fill
// ---------------------------------------------------------------------------

/// Interior is filled.
///
/// **Wire type:** bool
/// **Used by:** Pie, Ellipse, TextFrame, Image
pub const IS_SOLID: &str = "IsSolid";

/// Interior is transparent.
///
/// **Wire type:** bool
/// **Used by:** Ellipse (RECORD=8)
pub const TRANSPARENT: &str = "Transparent";

// ---------------------------------------------------------------------------
// Image
// ---------------------------------------------------------------------------

/// Image data stored in `Storage` stream (vs. linked by filename).
///
/// **Wire type:** bool
/// **Used by:** Image (RECORD=30)
pub const EMBED_IMAGE: &str = "EmbedImage";

/// Maintain aspect ratio.
///
/// **Wire type:** bool
/// **Used by:** Image (RECORD=30)
pub const KEEP_ASPECT: &str = "KeepAspect";

/// File name for linked images or external files.
///
/// **Wire type:** string
/// **Used by:** Image (RECORD=30), RTFLink (RECORD=241)
pub const FILE_NAME: &str = "FileName";

// ---------------------------------------------------------------------------
// Transform
// ---------------------------------------------------------------------------

/// Object orientation (TRotationBy90: 0=0deg, 1=90deg, 2=180deg, 3=270deg).
///
/// **Wire type:** u8
/// **Used by:** most objects
pub const ORIENTATION: &str = "Orientation";

/// Rotation angle.
///
/// **Wire type:** u8 or i16 (TRotationBy90 for harness objects; indexed
/// `Rotation{N}` for display modes in font table)
/// **Used by:** various objects, font table
pub const ROTATION: &str = "Rotation";

/// Mirror flag for IEEE symbols.
///
/// **Wire type:** bool
/// **Used by:** Symbol (RECORD=3)
pub const MIRROR: &str = "Mirror";

/// Scale factor for IEEE symbol shapes.
///
/// **Wire type:** coord (i32)
/// **Used by:** Symbol (RECORD=3)
pub const SCALE_FACTOR: &str = "ScaleFactor";

// ---------------------------------------------------------------------------
// Geometry
// ---------------------------------------------------------------------------

/// Object width.
///
/// **Wire type:** coord (i32)
/// **Used by:** SheetSymbol, Port, HarnessConnector, various objects
pub const WIDTH: &str = "Width";

/// Object height.
///
/// **Wire type:** coord (i32)
/// **Used by:** SheetSymbol, Port, HarnessConnector, various objects
pub const HEIGHT: &str = "Height";

/// Object size (generic).
///
/// **Wire type:** i32
/// **Used by:** various objects (font size indexed as `Size{N}`)
pub const SIZE: &str = "Size";

/// Object length (generic).
///
/// **Wire type:** coord (i32)
/// **Used by:** various objects
pub const LENGTH: &str = "Length";

/// Primary radius.
///
/// **Wire type:** coord (i32)
/// **Used by:** Arc (RECORD=12), EllipticalArc (RECORD=11), Pie (RECORD=9),
/// Ellipse (RECORD=8)
pub const RADIUS: &str = "Radius";

/// Y-axis radius for ellipses.
///
/// **Wire type:** coord (i32)
/// **Used by:** EllipticalArc (RECORD=11), Ellipse (RECORD=8)
pub const SECONDARY_RADIUS: &str = "SecondaryRadius";

/// Horizontal corner rounding for rounded rectangles.
///
/// **Wire type:** coord (i32, default 20 mils)
/// **Used by:** RoundRectangle (RECORD=10)
pub const CORNER_X_RADIUS: &str = "CornerXRadius";

/// Vertical corner rounding for rounded rectangles.
///
/// **Wire type:** coord (i32, default 20 mils)
/// **Used by:** RoundRectangle (RECORD=10)
pub const CORNER_Y_RADIUS: &str = "CornerYRadius";

// ---------------------------------------------------------------------------
// Coordinates
// ---------------------------------------------------------------------------

/// Origin/anchor X position.
///
/// **Wire type:** coord (i32)
/// **Used by:** most objects
pub const LOCATION_X: &str = "Location.X";

/// Origin/anchor Y position.
///
/// **Wire type:** coord (i32)
/// **Used by:** most objects
pub const LOCATION_Y: &str = "Location.Y";

/// Opposite corner X position.
///
/// **Wire type:** coord (i32)
/// **Used by:** Rectangle, Line, RoundRectangle, Blanket, SchematicBlock
pub const CORNER_X: &str = "Corner.X";

/// Opposite corner Y position.
///
/// **Wire type:** coord (i32)
/// **Used by:** Rectangle, Line, RoundRectangle, Blanket, SchematicBlock
pub const CORNER_Y: &str = "Corner.Y";

/// Vertex count for overflow vertices (beyond 50).
///
/// **Wire type:** i16
/// **Used by:** Polyline (RECORD=6)
///
/// **Gotcha:** note the ALL-CAPS key name `EXTRALOCATIONCOUNT`.
pub const EXTRA_LOCATION_COUNT: &str = "EXTRALOCATIONCOUNT";

/// Number of vertices (locations).
///
/// **Wire type:** i32
/// **Used by:** Polyline, Polygon, Wire, Bezier
pub const LOCATION_COUNT: &str = "LocationCount";

/// Override colors flag.
///
/// **Wire type:** bool
/// **Used by:** various objects
///
/// **Gotcha:** note the misspelling `OverideColors` (single 'r').
pub const OVERIDE_COLORS: &str = "OverideColors";

/// X size dimension.
///
/// **Wire type:** coord (i32)
/// **Used by:** SheetSymbol (RECORD=15), various objects
pub const X_SIZE: &str = "XSize";

/// Y size dimension.
///
/// **Wire type:** coord (i32)
/// **Used by:** SheetSymbol (RECORD=15), various objects
pub const Y_SIZE: &str = "YSize";

/// Style identifier.
///
/// **Wire type:** u8
/// **Used by:** various objects (power symbol style, etc.)
pub const STYLE: &str = "Style";

/// Layer assignment.
///
/// **Wire type:** u8
/// **Used by:** various objects
pub const LAYER: &str = "Layer";
