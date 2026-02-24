// Sheet and document parameters (RECORD=31).
//
// These constants define the parameter keys for the sheet/document record,
// covering grid settings, sheet sizing, borders, reference zones, display
// options, templates, cross-references, and versioning.

// ---------------------------------------------------------------------------
// Grid settings
// ---------------------------------------------------------------------------

/// Snap grid enabled.
///
/// **Wire type:** bool
/// **Used by:** Sheet (RECORD=31)
pub const SNAP_GRID_ON: &str = "SnapGridOn";

/// Snap grid spacing.
///
/// **Wire type:** coord (i32, default 10 mils = 1,000,000 DXP units)
/// **Used by:** Sheet (RECORD=31)
pub const SNAP_GRID_SIZE: &str = "SnapGridSize";

/// Fractional sub-unit for SnapGridSize (DXP frac encoding).
///
/// **Wire type:** i32 (0..100_000 internal units)
/// **Used by:** Sheet (RECORD=31), SchLib FileHeader
/// **Note:** Synthesized at runtime in Altium C# by appending "_Frac" to base key.
pub const SNAP_GRID_SIZE_FRAC: &str = "SnapGridSize_Frac";

/// Visible grid enabled.
///
/// **Wire type:** bool
/// **Used by:** Sheet (RECORD=31)
pub const VISIBLE_GRID_ON: &str = "VisibleGridOn";

/// Visible grid spacing.
///
/// **Wire type:** coord (i32, default 10 mils = 1,000,000 DXP units)
/// **Used by:** Sheet (RECORD=31)
pub const VISIBLE_GRID_SIZE: &str = "VisibleGridSize";

/// Fractional sub-unit for VisibleGridSize (DXP frac encoding).
///
/// **Wire type:** i32 (0..100_000 internal units)
/// **Used by:** Sheet (RECORD=31), SchLib FileHeader
/// **Note:** Synthesized at runtime in Altium C# by appending "_Frac" to base key.
pub const VISIBLE_GRID_SIZE_FRAC: &str = "VisibleGridSize_Frac";

/// Hot spot grid enabled.
///
/// **Wire type:** bool
/// **Used by:** Sheet (RECORD=31)
pub const HOT_SPOT_GRID_ON: &str = "HotSpotGridOn";

/// Hot spot grid spacing.
///
/// **Wire type:** coord (i32, default 8 mils)
/// **Used by:** Sheet (RECORD=31)
pub const HOT_SPOT_GRID_SIZE: &str = "HotSpotGridSize";

/// Fractional sub-unit for HotSpotGridSize (DXP frac encoding).
///
/// **Wire type:** i32 (0..100_000 internal units)
/// **Used by:** Sheet (RECORD=31), SchLib FileHeader
/// **Note:** Synthesized at runtime in Altium C# by appending "_Frac" to base key.
pub const HOT_SPOT_GRID_SIZE_FRAC: &str = "HotSpotGridSize_Frac";

// ---------------------------------------------------------------------------
// Sheet size
// ---------------------------------------------------------------------------

/// Paper size preset (TSheetStyle enum).
///
/// **Wire type:** u8
/// **Used by:** Sheet (RECORD=31)
///
/// Values: 0=A4, 1=A3, 2=A2, 3=A1, 4=A0, 5=A, 6=B, 7=C, 8=D, 9=E,
/// 10=Letter, 11=Legal, 12=Tabloid, 13..17=Orcad A-E.
pub const SHEET_STYLE: &str = "SheetStyle";

/// Use custom sheet size instead of `SheetStyle` preset.
///
/// **Wire type:** bool
/// **Used by:** Sheet (RECORD=31)
pub const USE_CUSTOM_SHEET: &str = "UseCustomSheet";

/// Custom sheet width.
///
/// **Wire type:** coord (i32, default 1500 mils = 150,000,000 DXP units)
/// **Used by:** Sheet (RECORD=31)
pub const CUSTOM_X: &str = "CustomX";

/// Fractional sub-unit for CustomX (DXP frac encoding).
///
/// **Wire type:** i32 (0..100_000 internal units)
/// **Used by:** Sheet (RECORD=31), SchLib FileHeader
/// **Note:** Synthesized at runtime in Altium C# by appending "_Frac" to base key.
pub const CUSTOM_X_FRAC: &str = "CustomX_Frac";

/// Custom sheet height.
///
/// **Wire type:** coord (i32, default 950 mils = 95,000,000 DXP units)
/// **Used by:** Sheet (RECORD=31)
pub const CUSTOM_Y: &str = "CustomY";

/// Fractional sub-unit for CustomY (DXP frac encoding).
///
/// **Wire type:** i32 (0..100_000 internal units)
/// **Used by:** Sheet (RECORD=31), SchLib FileHeader
/// **Note:** Synthesized at runtime in Altium C# by appending "_Frac" to base key.
pub const CUSTOM_Y_FRAC: &str = "CustomY_Frac";

// ---------------------------------------------------------------------------
// Border and title block
// ---------------------------------------------------------------------------

/// Show border around sheet.
///
/// **Wire type:** bool
/// **Used by:** Sheet (RECORD=31)
pub const BORDER_ON: &str = "BorderOn";

/// Show title block.
///
/// **Wire type:** bool
/// **Used by:** Sheet (RECORD=31)
pub const TITLE_BLOCK_ON: &str = "TitleBlockOn";

/// Document border style (TSheetDocumentBorderStyle enum).
///
/// **Wire type:** u8
/// **Used by:** Sheet (RECORD=31)
pub const DOCUMENT_BORDER_STYLE: &str = "DocumentBorderStyle";

/// Reference zones enabled.
///
/// **Wire type:** bool
/// **Used by:** Sheet (RECORD=31)
///
/// **Gotcha:** stored **inverted** -- `T` in file means zones are OFF.
pub const REFERENCE_ZONES_ON: &str = "ReferenceZonesOn";

/// Reference zone style.
///
/// **Wire type:** u8
/// **Used by:** Sheet (RECORD=31)
pub const REFERENCE_ZONE_STYLE: &str = "ReferenceZoneStyle";

// ---------------------------------------------------------------------------
// Zone dimensions
// ---------------------------------------------------------------------------

/// Number of horizontal reference zones (default 6).
///
/// **Wire type:** i32
/// **Used by:** Sheet (RECORD=31)
pub const CUSTOM_X_ZONES: &str = "CustomXZones";

/// Number of vertical reference zones (default 4).
///
/// **Wire type:** i32
/// **Used by:** Sheet (RECORD=31)
pub const CUSTOM_Y_ZONES: &str = "CustomYZones";

/// Border margin width.
///
/// **Wire type:** coord (i32, default 20 mils)
/// **Used by:** Sheet (RECORD=31)
pub const CUSTOM_MARGIN_WIDTH: &str = "CustomMarginWidth";

/// Sheet number space size (spacing between zone markers).
///
/// **Wire type:** i32
/// **Used by:** Sheet (RECORD=31)
pub const SHEET_NUMBER_SPACE_SIZE: &str = "SheetNumberSpaceSize";

// ---------------------------------------------------------------------------
// Display options
// ---------------------------------------------------------------------------

/// Sheet orientation (TSheetOrientation: Landscape/Portrait).
///
/// **Wire type:** u8
/// **Used by:** Sheet (RECORD=31)
pub const WORKSPACE_ORIENTATION: &str = "WorkspaceOrientation";

/// Show hidden pins on all components.
///
/// **Wire type:** bool
/// **Used by:** Sheet (RECORD=31)
pub const SHOW_HIDDEN_PINS: &str = "ShowHiddenPins";

/// Show template graphics overlay.
///
/// **Wire type:** bool
/// **Used by:** Sheet (RECORD=31)
pub const SHOW_TEMPLATE_GRAPHICS: &str = "ShowTemplateGraphics";

// ---------------------------------------------------------------------------
// Template
// ---------------------------------------------------------------------------

/// Path to sheet template file (`.SchDot`).
///
/// **Wire type:** string
/// **Used by:** Sheet (RECORD=31)
pub const TEMPLATE_FILE_NAME: &str = "TemplateFileName";

// ---------------------------------------------------------------------------
// Document settings
// ---------------------------------------------------------------------------

/// Display unit (TUnit enum, affects runtime unit system).
///
/// **Wire type:** u8
/// **Used by:** Sheet (RECORD=31)
///
/// **Gotcha:** note the underscore in the key name `Display_Unit`.
pub const DISPLAY_UNIT: &str = "Display_Unit";

/// Default font for new objects (1-based FontID).
///
/// **Wire type:** i16
/// **Used by:** Sheet (RECORD=31)
pub const SYSTEM_FONT: &str = "SystemFont";

/// MBCS string encoding enabled.
///
/// **Wire type:** bool
/// **Used by:** Sheet (RECORD=31)
///
/// Always `T` in V5 files.
pub const USE_MBCS: &str = "UseMBCS";

/// Deprecated BOC flag (always written as `T`).
///
/// **Wire type:** bool
/// **Used by:** Sheet (RECORD=31)
pub const IS_BOC: &str = "IsBOC";

// ---------------------------------------------------------------------------
// Colors
// ---------------------------------------------------------------------------

/// Sheet area (within border) background color.
///
/// **Wire type:** u32 (BGR COLORREF)
/// **Used by:** Sheet (RECORD=31)
pub const AREA_COLOR: &str = "AreaColor";

// ---------------------------------------------------------------------------
// Display styles (indexed style table)
// ---------------------------------------------------------------------------

/// Number of style entries in the indexed style table.
///
/// **Wire type:** i32
/// **Used by:** SchLib `FileHeader` stream
pub const STYLE_ID_COUNT: &str = "StyleIDCount";

/// Indexed style gradient depth (`StyleGradientDepth{N}`).
///
/// **Wire type:** i32
/// **Used by:** SchLib `FileHeader` stream
pub const STYLE_GRADIENT_DEPTH: &str = "StyleGradientDepth";

/// Indexed style shadow opacity (`StyleShadowOpacity{N}`).
///
/// **Wire type:** i32
/// **Used by:** SchLib `FileHeader` stream
pub const STYLE_SHADOW_OPACITY: &str = "StyleShadowOpacity";

/// Indexed style shadow distance integer part (`StyleShadowDistance{N}`).
///
/// **Wire type:** i32
/// **Used by:** SchLib `FileHeader` stream
pub const STYLE_SHADOW_DISTANCE: &str = "StyleShadowDistance";

/// Indexed style shadow distance fractional part (`StyleShadowDistance{N}_Frac`).
///
/// **Wire type:** i32
/// **Used by:** SchLib `FileHeader` stream
pub const STYLE_SHADOW_DISTANCE_FRAC: &str = "StyleShadowDistance_Frac";

/// Indexed style shadow blur integer part (`StyleShadowBlur{N}`).
///
/// **Wire type:** i32
/// **Used by:** SchLib `FileHeader` stream
pub const STYLE_SHADOW_BLUR: &str = "StyleShadowBlur";

/// Indexed style shadow blur fractional part (`StyleShadowBlur{N}_Frac`).
///
/// **Wire type:** i32
/// **Used by:** SchLib `FileHeader` stream
pub const STYLE_SHADOW_BLUR_FRAC: &str = "StyleShadowBlur_Frac";

/// Indexed style shadow angle in degrees (`StyleShadowAngleInDegrees{N}`).
///
/// **Wire type:** i32
/// **Used by:** SchLib `FileHeader` stream
pub const STYLE_SHADOW_ANGLE_IN_DEGREES: &str = "StyleShadowAngleInDegrees";

/// Indexed style glow color (`StyleGlowColor{N}`).
///
/// **Wire type:** u32 (BGR COLORREF)
/// **Used by:** SchLib `FileHeader` stream
pub const STYLE_GLOW_COLOR: &str = "StyleGlowColor";

/// Indexed style glow opacity (`StyleGlowOpacity{N}`).
///
/// **Wire type:** i32
/// **Used by:** SchLib `FileHeader` stream
pub const STYLE_GLOW_OPACITY: &str = "StyleGlowOpacity";

/// Indexed style glow size (`StyleGlowSize{N}`).
///
/// **Wire type:** i32
/// **Used by:** SchLib `FileHeader` stream
pub const STYLE_GLOW_SIZE: &str = "StyleGlowSize";

/// Indexed style reflection depth (`StyleReflectionDepth{N}`).
///
/// **Wire type:** i32
/// **Used by:** SchLib `FileHeader` stream
pub const STYLE_REFLECTION_DEPTH: &str = "StyleReflectionDepth";

/// Indexed style reflection opacity (`StyleReflectionOpacity{N}`).
///
/// **Wire type:** i32
/// **Used by:** SchLib `FileHeader` stream
pub const STYLE_REFLECTION_OPACITY: &str = "StyleReflectionOpacity";

/// Indexed style transparency enabled flag (`StyleTransparencyEnabled{N}`).
///
/// **Wire type:** bool
/// **Used by:** SchLib `FileHeader` stream
pub const STYLE_TRANSPARENCY_ENABLED: &str = "StyleTransparencyEnabled";

/// Indexed style transparency amount (`StyleTransparencyAmount{N}`).
///
/// **Wire type:** i32
/// **Used by:** SchLib `FileHeader` stream
pub const STYLE_TRANSPARENCY_AMOUNT: &str = "StyleTransparencyAmount";

/// Indexed style corner radius mode (`StyleCornerRadiusMode{N}`).
///
/// **Wire type:** i32
/// **Used by:** SchLib `FileHeader` stream
pub const STYLE_CORNER_RADIUS_MODE: &str = "StyleCornerRadiusMode";

/// Indexed style corner radius value (`StyleCornerRadiusValue{N}`).
///
/// **Wire type:** i32
/// **Used by:** SchLib `FileHeader` stream
pub const STYLE_CORNER_RADIUS_VALUE: &str = "StyleCornerRadiusValue";

// ---------------------------------------------------------------------------
// Cross-reference and document info
// ---------------------------------------------------------------------------

/// Target file for sheet parts / definitions.
///
/// **Wire type:** DynamicString
/// **Used by:** Component, ObjectDefinition
pub const TARGET_FILE_NAME: &str = "TargetFileName";

/// Document number (title block).
///
/// **Wire type:** DynamicString
/// **Used by:** V4 file header; V5 system parameter
pub const DOC_NUM: &str = "DocNum";

/// Current sheet number (1-based).
///
/// **Wire type:** i16
/// **Used by:** V4 only
pub const SHEET_NUM: &str = "SheetNum";

/// Total sheet count.
///
/// **Wire type:** i16
/// **Used by:** V4 only
pub const SHEET_COUNT: &str = "SheetCount";

// ---------------------------------------------------------------------------
// Version
// ---------------------------------------------------------------------------

/// Minor version number of the file format.
///
/// **Wire type:** i32
/// **Used by:** Sheet (RECORD=31), `FileHeader` stream
pub const MINOR_VERSION: &str = "MinorVersion";

/// Pipe-delimited feature flag string for compatibility checks.
///
/// **Wire type:** DynamicString
/// **Used by:** Sheet (RECORD=31)
pub const FILE_VERSION_INFO: &str = "FileVersionInfo";

// ---------------------------------------------------------------------------
// Miscellaneous
// ---------------------------------------------------------------------------

/// Global default: cross-reference annotations hidden.
///
/// **Wire type:** bool
/// **Used by:** Sheet (RECORD=31)
pub const DEFAULT_CROSS_REF_HIDDEN: &str = "DefaultCrossRefHidden";

/// Show border around sheet (alternative key used by some objects).
///
/// **Wire type:** bool
/// **Used by:** various border-related objects
pub const SHOW_BORDER: &str = "ShowBorder";

// ---------------------------------------------------------------------------
// Title block fields
// ---------------------------------------------------------------------------

/// Title block address line 1.
///
/// **Wire type:** string
/// **Used by:** Sheet (RECORD=31) title block
pub const ADDRESS_1: &str = "Address1";

/// Title block address line 2.
///
/// **Wire type:** string
/// **Used by:** Sheet (RECORD=31) title block
pub const ADDRESS_2: &str = "Address2";

/// Title block address line 3.
///
/// **Wire type:** string
/// **Used by:** Sheet (RECORD=31) title block
pub const ADDRESS_3: &str = "Address3";

/// Title block address line 4.
///
/// **Wire type:** string
/// **Used by:** Sheet (RECORD=31) title block
pub const ADDRESS_4: &str = "Address4";

/// Document author name.
///
/// **Wire type:** string
/// **Used by:** Sheet (RECORD=31) title block
pub const AUTHOR: &str = "Author";

/// Organisation name (British spelling variant).
///
/// **Wire type:** string
/// **Used by:** Sheet (RECORD=31) title block
pub const ORGANISATION: &str = "Organisation";

/// Organization name (American spelling variant).
///
/// **Wire type:** string
/// **Used by:** Sheet (RECORD=31) title block
pub const ORGANIZATION: &str = "Organization";

/// Organization name (alternate key).
///
/// **Wire type:** string
/// **Used by:** Sheet (RECORD=31) title block
pub const ORGANIZATION_NAME: &str = "OrganizationName";

/// Document title.
///
/// **Wire type:** string
/// **Used by:** Sheet (RECORD=31) title block
pub const TITLE: &str = "Title";

/// Sheet identifier in V4 ASCII format.
///
/// **Wire type:** string
/// **Used by:** V4 ASCII format, cross-reference context
pub const SHEET: &str = "Sheet";

// ---------------------------------------------------------------------------
// Orcad compatibility
// ---------------------------------------------------------------------------

/// Orcad-compatible X sheet size.
///
/// **Wire type:** coord (i32)
/// **Used by:** Sheet (RECORD=31) when using Orcad sheet styles
pub const ORCAD_X_SIZE: &str = "OrcadXSize";

/// Orcad-compatible Y sheet size.
///
/// **Wire type:** coord (i32)
/// **Used by:** Sheet (RECORD=31) when using Orcad sheet styles
pub const ORCAD_Y_SIZE: &str = "OrcadYSize";
